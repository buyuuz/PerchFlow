"""CI-only Windows SCM end-to-end tests for a *disposable* PerchFlow EXE.

No access to the user's machine, Daily_Check, existing services, or Task Scheduler.
Run only on GitHub-hosted Windows runner with explicit workflow opt-in.
"""
import ctypes
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

SERVICE = "Perch"  # hard-coded by PerchFlow's Windows service dispatcher
WORKER = r'''
import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import time

mode = sys.argv[1]
if mode == "session":
    session_id = ctypes.c_ulong(0xFFFFFFFF)
    ok = ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(), ctypes.byref(session_id))
    if not ok:
        raise RuntimeError("ProcessIdToSessionId failed")
    Path("mock_session.json").write_text(
        json.dumps({"label": "MOCK_ONLY", "id": session_id.value}), encoding="utf-8"
    )
    print("MOCK_ONLY session", session_id.value, flush=True)
elif mode == "cron":
    with Path("mock_ticks.txt").open("a", encoding="utf-8") as log:
        log.write("MOCK_ONLY cron\n")
    print("MOCK_ONLY scheduled", flush=True)
elif mode == "child":
    time.sleep(4)
    Path("mock_orphan.txt").write_text("ESCAPED", encoding="utf-8")
elif mode == "parent":
    p = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "child"])
    Path("mock_child_pid.txt").write_text(str(p.pid), encoding="utf-8")
    print("MOCK_ONLY spawned", flush=True)
    time.sleep(30)
else:
    raise RuntimeError("not a known fake task")
'''


def require_ci():
    assert os.name == "nt", "Windows-only"
    assert os.environ.get("GITHUB_ACTIONS") == "true", "GitHub Actions only"
    assert os.environ.get("RUNNER_OS") == "Windows", "Windows runner only"
    assert os.environ.get("CI", "").lower() == "true", "CI only"
    assert os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted", "Refusing self-hosted runner"
    assert os.environ.get("PERCH_SCM_MOCK_OPT_IN") == "1", "Explicit workflow opt-in required"


def run(*args, check=True, timeout=20):
    p = subprocess.run(args, capture_output=True, text=True, errors="replace", timeout=timeout)
    if check and p.returncode != 0:
        raise RuntimeError(f"{args[:2]} failed {p.returncode}: {p.stdout} {p.stderr}")
    return p


def scm_state():
    p = run("sc.exe", "queryex", SERVICE, check=False)
    if p.returncode:
        if "1060" in p.stdout + p.stderr or "1072" in p.stdout + p.stderr:
            return "ABSENT", 0
        raise RuntimeError(f"Cannot safely inspect service: {p.stdout} {p.stderr}")
    m = re.search(r"STATE\s*:\s*\d+\s+(\w+)", p.stdout)
    n = re.search(r"PID\s*:\s*(\d+)", p.stdout)
    return m.group(1) if m else "UNKNOWN", int(n.group(1)) if n else 0


def wait(predicate, label, limit=20):
    end = time.monotonic() + limit
    last = None
    while time.monotonic() < end:
        try:
            answer = predicate()
            if answer:
                return answer
        except (OSError, ValueError, urllib.error.URLError) as exc:
            last = str(exc)
        time.sleep(0.13)
    raise AssertionError(f"Timeout {label}, last={last}, SCM={scm_state()}")


def api(base, token, endpoint, method="GET", payload=None):
    req = urllib.request.Request(
        base + endpoint,
        data=json.dumps(payload).encode() if payload is not None else None,
        method=method,
        headers={"Authorization": "Bearer " + token, "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=5) as reply:
        return json.loads(reply.read().decode("utf-8"))


def task(root, name, mode, enabled=False, cron="0 0 0 * * *"):
    return dict(
        name=name, cmd=str(Path(sys.executable).resolve()),
        args=["-u", str(root / "mock_worker.py"), mode],
        cwd=str(root), schedule=cron, timeout_secs=0, enabled=enabled,
    )


def await_status(base, token, name, wanted):
    return wait(lambda: (v if (v := api(base, token, "/api/tasks/" + name))["status"] == wanted else None),
                name + "=" + wanted)


def service_exe(pid):
    # Never signal a process without confirming the exact expected temporary binary.
    p = run("powershell.exe", "-NoProfile", "-Command",
            f"(Get-CimInstance Win32_Process -Filter 'ProcessId={pid}').ExecutablePath")
    return p.stdout.strip()


def start(base, token, exe):
    result = run("sc.exe", "start", SERVICE, check=False)
    assert result.returncode == 0, result.stdout + result.stderr
    pid = wait(lambda: (info[1] if (info := scm_state())[0] == "RUNNING" and info[1] else None),
               "Windows service running")
    assert service_exe(pid).casefold() == str(exe).casefold(), "Unexpected SCM process! Refuse action."
    wait(lambda: api(base, token, "/api/tasks"), "service API online")
    return pid


def stop():
    p = run("sc.exe", "stop", SERVICE, check=False)
    assert p.returncode == 0 or scm_state()[0] == "STOPPED", p.stdout + p.stderr
    wait(lambda: scm_state()[0] == "STOPPED", "service stopped", limit=28)


def kill_owned(pid, exe):
    assert service_exe(pid).casefold() == str(exe).casefold(), "Unexpected PID, refuse kill"
    run("taskkill.exe", "/T", "/F", "/PID", str(pid))
    wait(lambda: scm_state()[0] == "STOPPED", "SCM noticed crash", limit=27)


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def test(exe, report):
    require_ci()
    assert exe.name.lower() == "perch.exe" and exe.is_file()
    assert scm_state()[0] == "ABSENT", "Service Perch exists, refusing changes"
    checks = {}
    registered = False

    with tempfile.TemporaryDirectory(prefix="perch-scm-disposable-") as folder:
        root = Path(folder).resolve()
        isolated = root / "perch.exe"
        shutil.copy2(exe, isolated)
        (root / "mock_worker.py").write_text(WORKER, encoding="utf-8")
        token = secrets.token_urlsafe(32)
        base = f"http://127.0.0.1:{free_port()}"
        (root / "perch.toml").write_text(
            '[server]\nbind = "' + base.removeprefix("http://") + '"\n'
            'token = "' + token + '"\nlang = "en"\n', encoding="utf-8"
        )
        try:
            # This only registers a service in the disposable GitHub-hosted VM.
            cmdline = '"' + str(isolated) + '" service'
            run("sc.exe", "create", SERVICE, "binPath=", cmdline, "type=", "own",
                "start=", "auto")
            registered = True
            assert "AUTO_START" in run("sc.exe", "qc", SERVICE).stdout
            checks["scm_autostart_registration"] = "PASS"

            pid = start(base, token, isolated)
            checks["scm_start_and_loopback_api"] = "PASS"

            session = task(root, "session_mock", "session")
            assert api(base, token, "/api/tasks", "POST", session)["ok"]
            assert api(base, token, "/api/tasks/session_mock/run", "POST")["ok"]
            await_status(base, token, "session_mock", "success")
            data = json.loads((root / "mock_session.json").read_text(encoding="utf-8"))
            assert data == {"label": "MOCK_ONLY", "id": 0}, data
            checks["real_session_zero_task"] = "PASS"

            cron = task(root, "cron_mock", "cron", True, "* * * * * *")
            assert api(base, token, "/api/tasks", "POST", cron)["ok"]
            def twice():
                runs = api(base, token, "/api/tasks/cron_mock/history")["runs"]
                passed = [r for r in runs if r["status"] == "success"]
                return passed if len(passed) >= 2 else None
            runs = wait(twice, "two scheduled triggers", limit=15)
            assert len({r["run_id"] for r in runs}) >= 2
            assert (root / "mock_ticks.txt").read_text(encoding="utf-8").count("MOCK_ONLY") >= 2
            # Disable recurring job before testing service restart.
            wait(lambda: api(base, token, "/api/tasks/cron_mock")["status"] != "running",
                 "cron idle")
            cron["enabled"] = False
            assert api(base, token, "/api/tasks/cron_mock", "PUT", cron)["ok"]
            checks["scm_real_cron_twice"] = "PASS"

            parent = task(root, "parent_mock", "parent")
            assert api(base, token, "/api/tasks", "POST", parent)["ok"]
            assert api(base, token, "/api/tasks/parent_mock/run", "POST")["ok"]
            wait(lambda: (root / "mock_child_pid.txt").exists(), "mock spawned descendant")
            stop()
            time.sleep(5)
            assert not (root / "mock_orphan.txt").exists(), "SCM stop orphaned mock descendant"
            checks["scm_stop_kills_mock_descendants"] = "PASS"

            pid = start(base, token, isolated)
            old = api(base, token, "/api/tasks/parent_mock")
            assert old["status"] in ("cancelled", "interrupted"), old
            checks["scm_restart_recovers_state"] = "PASS"

            # Crash only the verified disposable service process in CI.
            (root / "mock_child_pid.txt").unlink(missing_ok=True)
            assert api(base, token, "/api/tasks/parent_mock/run", "POST")["ok"]
            wait(lambda: (root / "mock_child_pid.txt").exists(), "mock process before crash")
            kill_owned(pid, isolated)
            pid = start(base, token, isolated)
            old = api(base, token, "/api/tasks/parent_mock")
            assert old["status"] == "interrupted", old
            checks["crash_then_manual_restart"] = "PASS"

            bad = task(root, "missing_mock", "session")
            bad["cmd"] = str(root / "fake-nonexistent.exe")
            assert api(base, token, "/api/tasks", "POST", bad)["ok"]
            assert api(base, token, "/api/tasks/missing_mock/run", "POST")["ok"]
            assert await_status(base, token, "missing_mock", "failed")["last_exit_code"] is None
            assert scm_state()[0] == "RUNNING"
            checks["missing_mock_worker_isolated"] = "PASS"
        finally:
            if registered:
                try:
                    if scm_state()[0] != "STOPPED":
                        run("sc.exe", "stop", SERVICE, check=False)
                        wait(lambda: scm_state()[0] == "STOPPED", "cleanup stop", limit=28)
                finally:
                    run("sc.exe", "delete", SERVICE, check=False)
                    wait(lambda: scm_state()[0] == "ABSENT", "service removed", limit=28)
            report.parent.mkdir(parents=True, exist_ok=True)
            report.write_text(json.dumps({
                "checks": checks,
                "passed": len(checks),
                "expected": 8,
                "scope": "GitHub-hosted ephemeral Windows SCM only",
                "Daily_Check_called": False,
                "real_reboot_tested": False,
                "browser_interactivity_tested": False,
            }, indent=2), encoding="utf-8")
    print(json.dumps(checks, indent=2))
    assert len(checks) == 8, checks


if __name__ == "__main__":
    require_ci()
    assert len(sys.argv) == 3, "usage: python script.py perch.exe report.json"
    test(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
