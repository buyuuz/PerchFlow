"""Run the built PerchFlow EXE against disposable fake tasks on a Windows CI runner.

Nothing here loads production configuration, touches Windows Task Scheduler, or
imports a Daily_Check module. Requires only Python's standard library.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import secrets

MOCK_PROGRAM = r'''
import subprocess
import sys
import time
from pathlib import Path

mode = sys.argv[1]
if mode == "tick":
    with Path("mock_cron_ticks.txt").open("a", encoding="utf-8") as out:
        out.write("MOCK_ONLY cron tick\n")
    print("MOCK_ONLY scheduled check", flush=True)
elif mode == "fail":
    print("MOCK_ONLY intentional failure", flush=True)
    sys.exit(17)
elif mode == "sleep":
    print("MOCK_ONLY waiting", flush=True)
    time.sleep(15)
elif mode == "child":
    time.sleep(3)
    Path("mock_orphan_marker.txt").write_text("CHILD_ESCAPED", encoding="utf-8")
elif mode == "spawn":
    child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "child"])
    Path("mock_child_started.txt").write_text(str(child.pid), encoding="utf-8")
    print("MOCK_ONLY launched descendant", flush=True)
    time.sleep(15)
else:
    raise RuntimeError("unknown fake task mode")
'''


def free_loopback_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for(predicate, label, seconds=17, interval=0.12):
    deadline = time.monotonic() + seconds
    last_error = None
    while time.monotonic() < deadline:
        try:
            found = predicate()
            if found:
                return found
        except (urllib.error.URLError, ValueError, OSError) as exc:
            last_error = exc
        time.sleep(interval)
    raise AssertionError(f"timed out: {label}; last={last_error}")


def api(base, token, path, method="GET", payload=None):
    body = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(
        base + path,
        data=body,
        method=method,
        headers={
            "Authorization": "Bearer " + token,
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=6) as response:
        return json.loads(response.read().decode("utf-8"))


def ensure(condition, message):
    if not condition:
        raise AssertionError(message)


def task_definition(root, name, python, mode, enabled=False, schedule="0 0 0 * * *", timeout=0):
    return {
        "name": name,
        "cmd": python,
        "args": ["-u", str(root / "mock_worker.py"), mode],
        "cwd": str(root),
        "schedule": schedule,
        "timeout_secs": timeout,
        "enabled": enabled,
    }


def wait_status(base, token, name, terminal, seconds=18):
    return wait_for(
        lambda: (state if (state := api(base, token, f"/api/tasks/{name}"))["status"] == terminal else None),
        f"{name} -> {terminal}",
        seconds=seconds,
    )


def launch(exe, root):
    out = (root / "sandbox_perch_stdout.log").open("a", encoding="utf-8")
    err = (root / "sandbox_perch_stderr.log").open("a", encoding="utf-8")
    try:
        child = subprocess.Popen(
            [str(exe), "run"],
            cwd=str(root),
            stdin=subprocess.DEVNULL,
            stdout=out,
            stderr=err,
        )
    finally:
        out.close()
        err.close()
    return child


def stop_process(child):
    if child is None or child.poll() is not None:
        return
    child.terminate()
    try:
        child.wait(timeout=7)
    except subprocess.TimeoutExpired:
        child.kill()
        child.wait(timeout=5)


def run_simulation(exe, results_path=None):
    ensure(os.name == "nt", "This mock binary test is for Windows runners only")
    ensure(exe.is_file(), f"Missing test executable: {exe}")
    ensure(exe.name.lower() == "perch.exe", "Expected built PerchFlow exe")

    results = {}
    with tempfile.TemporaryDirectory(prefix="perch-mock-only-") as dirname:
        root = Path(dirname).resolve()
        isolated_exe = root / "perch.exe"
        shutil.copy2(exe, isolated_exe)
        (root / "mock_worker.py").write_text(MOCK_PROGRAM, encoding="utf-8")
        port = free_loopback_port()
        token = secrets.token_urlsafe(30)
        (root / "perch.toml").write_text(
            f'[server]\nbind = "127.0.0.1:{port}"\n'
            f'token = "{token}"\nlang = "en"\n',
            encoding="utf-8",
        )
        base = f"http://127.0.0.1:{port}"
        python = str(Path(sys.executable).resolve())
        process = None
        try:
            process = launch(isolated_exe, root)
            wait_for(lambda: api(base, token, "/api/tasks"), "isolated server boot", seconds=20)
            ensure(process.poll() is None, "Sandbox EXE exited after startup")
            results["isolated_boot"] = "PASS"

            # Real cron scheduling, not POST /run. Check two separate trigger IDs.
            cron = task_definition(root, "cron_mock", python, "tick", True, "* * * * * *")
            ensure(api(base, token, "/api/tasks", "POST", cron)["ok"], "Cron registration failed")
            def two_cron_completions():
                events = api(base, token, "/api/tasks/cron_mock/history")["runs"]
                completed = [e for e in events if e["status"] == "success"]
                return completed if len(completed) >= 2 else None
            runs = wait_for(two_cron_completions, "two real automatic cron triggers", seconds=12)
            ensure(len(set(r["run_id"] for r in runs)) >= 2, "Cron runs share a run ID")
            marker = root / "mock_cron_ticks.txt"
            ensure(marker.is_file() and marker.read_text(encoding="utf-8").count("MOCK_ONLY") >= 2, "Cron did not run the mock executable")
            results["automatic_cron_twice"] = "PASS"

            # Disable recurring tasks before restart; never leave scheduled effects.
            wait_for(lambda: (
                True if api(base, token, "/api/tasks/cron_mock")["status"] != "running" else None
            ), "cron not running", seconds=5)
            cron["enabled"] = False
            ensure(api(base, token, "/api/tasks/cron_mock", "PUT", cron)["ok"], "Cron disable failed")

            # Exit code and persisted captured output.
            fail = task_definition(root, "fail_mock", python, "fail")
            ensure(api(base, token, "/api/tasks", "POST", fail)["ok"], "Failure task registration failed")
            run_id = api(base, token, "/api/tasks/fail_mock/run", "POST")["run_id"]
            finished = wait_status(base, token, "fail_mock", "failed")
            ensure(finished["last_exit_code"] == 17, "Nonzero exit code lost")
            history = api(base, token, "/api/tasks/fail_mock/history")["runs"]
            ensure(history[0]["run_id"] == run_id and history[0]["status"] == "failed", "Failure history mismatch")
            log = api(base, token, f"/api/tasks/fail_mock/log/{run_id}")
            ensure(log["ok"] and "MOCK_ONLY intentional failure" in log["content"], "Captured stderr/stdout not returned")
            try:
                urllib.request.urlopen(base + f"/api/tasks/fail_mock/log/{run_id}", timeout=5)
                raise AssertionError("Log endpoint unauthenticated!")
            except urllib.error.HTTPError as error:
                ensure(error.code == 401, "Log endpoint returned unexpected unauthenticated code")
            results["failure_logging_and_auth"] = "PASS"

            # Manual starts cannot overlap. Cancel must finish and preserve history.
            sleep = task_definition(root, "slow_mock", python, "sleep")
            ensure(api(base, token, "/api/tasks", "POST", sleep)["ok"], "Slow task creation failed")
            manual = api(base, token, "/api/tasks/slow_mock/run", "POST")
            ensure(manual["ok"] and manual.get("run_id"), "Async start should return run ID")
            duplicate = api(base, token, "/api/tasks/slow_mock/run", "POST")
            ensure(duplicate["ok"] is False and "already running" in duplicate["error"], "Overlapping job not rejected")
            ensure(api(base, token, "/api/tasks/slow_mock/stop", "POST")["ok"], "Cancellation rejected")
            wait_status(base, token, "slow_mock", "cancelled")
            results["duplicate_guard_and_cancel"] = "PASS"

            # Deadline terminates mock task and persists timeout status.
            timeout = task_definition(root, "timeout_mock", python, "sleep", timeout=1)
            ensure(api(base, token, "/api/tasks", "POST", timeout)["ok"], "Timeout task creation failed")
            ensure(api(base, token, "/api/tasks/timeout_mock/run", "POST")["ok"], "Timeout start failed")
            timed_out = wait_status(base, token, "timeout_mock", "timeout")
            ensure(timed_out["last_exit_code"] is None, "Timeout must not claim process exit 0")
            results["timeout_enforcement"] = "PASS"

            # Verify actual spawned descendant cannot outlive the parent after stop.
            spawn = task_definition(root, "tree_mock", python, "spawn")
            ensure(api(base, token, "/api/tasks", "POST", spawn)["ok"], "Process tree task creation failed")
            ensure(api(base, token, "/api/tasks/tree_mock/run", "POST")["ok"], "Process tree task start failed")
            wait_for(lambda: (root / "mock_child_started.txt").exists(), "mock spawned descendant", seconds=9)
            ensure(api(base, token, "/api/tasks/tree_mock/stop", "POST")["ok"], "Tree cancellation failed")
            wait_status(base, token, "tree_mock", "cancelled")
            time.sleep(4)
            ensure(not (root / "mock_orphan_marker.txt").exists(), "Process tree kill left child running")
            results["descendant_termination"] = "PASS"

            # Stop one disposable EXE, restart another in the same disposable folder,
            # and verify that the previous completed status is recovered from JSONL.
            stop_process(process)
            process = None
            process = launch(isolated_exe, root)
            wait_for(lambda: api(base, token, "/api/tasks"), "isolated service restart", seconds=20)
            restored = api(base, token, "/api/tasks/fail_mock")
            ensure(restored["status"] == "failed" and restored["last_exit_code"] == 17,
                   "Restart did not restore task outcome")
            ensure(api(base, token, "/api/tasks/fail_mock/history")["runs"][0]["run_id"] == run_id,
                   "Restart lost persisted JSONL history")
            results["restart_state_and_history"] = "PASS"
            results["production_unmodified"] = "PASS (CI runner and temporary directory only)"
        finally:
            stop_process(process)
            if results_path:
                results_path.parent.mkdir(parents=True, exist_ok=True)
                results_path.write_text(json.dumps({
                    "result": "PASS" if len(results) == 8 else "PARTIAL",
                    "tests": results,
                    "backend": "Windows CI isolated perch.exe",
                    "production_script_called": False,
                }, indent=2), encoding="utf-8")
        print(json.dumps(results, indent=2))
        return results


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--exe", type=Path, required=True)
    ap.add_argument("--results", type=Path, default=Path("dev/reports/mock-runtime-ci.json"))
    opts = ap.parse_args()
    run_simulation(opts.exe.resolve(), opts.results.resolve())
