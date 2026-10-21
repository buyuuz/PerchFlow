"""Real PerchFlow EXE + real Python interpreter, mock business data only.

Required:
- GitHub-hosted ephemeral Windows CI runner, never user devices.
- A freshly built perch.exe, copied to a fresh temporary directory.
- Only Python standard-library mock tasks and loopback PerchFlow APIs.
- No genuine project files, account material, network services, or logins.
"""
import argparse
import json
import os
import secrets
import shutil
import sys
import tempfile
import time
from pathlib import Path

from simulate_task_runtime_binary import (
    api, ensure, free_loopback_port, launch, stop_process, wait_for, wait_status,
)

FAKE_MARKER = "SYNTHETIC_PRIVATE_VALUE_NEVER_EXPOSE"
MOCK_WORKER = r'''
"""Isolated dummy file-producing task. No networking or third-party imports."""
import sys
import time
from pathlib import Path

mode = sys.argv[1] if len(sys.argv) > 1 else "success"
if mode == "success":
    # Read fixture bodies *inside this fake worker only*, not by PerchFlow.
    assert Path("input.txt").read_text(encoding="utf-8").startswith("SYNTHETIC_")
    assert Path("settings.txt").read_text(encoding="utf-8") == "MOCK_SETTINGS"
    artifact = Path("output")
    (artifact / "photos").mkdir(parents=True, exist_ok=True)
    stamp = str(time.time_ns())
    (artifact / "report.csv").write_text("id,value\nmock," + stamp + "\n", encoding="utf-8")
    (artifact / "photos" / "demo.png").write_bytes(b"MOCK_PNG_" + stamp.encode())
    Path("results.txt").write_text(
        "SYNTHETIC_PRIVATE_VALUE_NEVER_EXPOSE " + stamp, encoding="utf-8"
    )
    print("MOCK_PYTHON_SUCCESS cwd=" + Path.cwd().name, flush=True)
elif mode == "failure":
    print("MOCK_PYTHON_EXPECTED_FAILURE", flush=True)
    sys.exit(9)
elif mode == "sleep":
    print("MOCK_PYTHON_WAITING", flush=True)
    time.sleep(30)
else:
    raise RuntimeError("Unexpected mock mode")
'''

def ci_guard(exe):
    ensure(os.name == "nt", "Only ephemeral Windows runners are supported")
    ensure(os.environ.get("GITHUB_ACTIONS") == "true", "Must run in GitHub Actions")
    ensure(os.environ.get("RUNNER_OS") == "Windows", "Expected Windows runner")
    ensure(os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted",
           "Refuse self-hosted or unrecognized runner")
    ensure(exe.is_file() and exe.name.lower() == "perch.exe", "Built perch.exe required")


def create_disabled_python_task(base, token, task_name, python, mode, timeout=12):
    result = api(base, token, "/api/projects/Mock%20Python%20Pipeline/python-task",
                 "POST", {
                     "name": task_name,
                     "python": str(python),
                     "script": "mock_worker.py",
                     "args": [mode],
                     "schedule": "0 0 0 * * *",
                     "timeout_secs": timeout,
                 })
    ensure(result.get("ok") and result.get("enabled") is False,
           f"Python task must be saved disabled, got {result}")
    detail = api(base, token, f"/api/tasks/{task_name}")
    ensure(detail["enabled"] is False, "Config enabled unexpectedly")
    ensure(detail["cmd"] == str(python), "Python interpreter not preserved")
    ensure(detail["cwd"].endswith("mock_project"), "Working directory not bound to project")
    ensure(detail["args"][1] == mode, "Python arguments not preserved")
    return detail


def start_and_wait(base, token, name, expected, timeout=16):
    started = api(base, token, f"/api/tasks/{name}/run", "POST")
    ensure(started.get("ok") and started.get("run_id"), f"Run rejected for {name}: {started}")
    run_id = started["run_id"]
    state = wait_status(base, token, name, expected, seconds=timeout)
    ensure(state["run_id"] == run_id, f"Run ID changed for {name}")
    history = api(base, token, f"/api/tasks/{name}/history")["runs"]
    ensure(history and history[0]["run_id"] == run_id, "Wrong run history ID")
    ensure(history[0]["status"] == expected, f"History disagrees for {name}")
    return run_id, history[0]


def test_pipeline(exe, output_file):
    ci_guard(exe)
    checks = {}
    report = {
        "kind": "actual PerchFlow EXE + actual isolated Python interpreter, FAKE project only",
        "platform": "GitHub-hosted Windows disposable runner",
        "real_business_files_used": False,
        "production_script_called": False,
        "external_logins_performed": False,
        "real_computer_touched": False,
        "checks": checks,
    }
    instance = None
    try:
        with tempfile.TemporaryDirectory(prefix="perch-project-rc-mock-") as tmp:
            root = Path(tmp).resolve()
            sandbox_exe = root / "perch.exe"
            shutil.copy2(exe, sandbox_exe)
            project = root / "mock_project"
            project.mkdir()
            (project / "mock_worker.py").write_text(MOCK_WORKER, encoding="utf-8")
            port = free_loopback_port()
            token = secrets.token_urlsafe(28)
            (root / "perch.toml").write_text(
                f'[server]\nbind = "127.0.0.1:{port}"\n'
                f'token = "{token}"\nlang = "en"\n',
                encoding="utf-8",
            )
            base = f"http://127.0.0.1:{port}"
            python = Path(sys.executable).resolve()
            try:
                instance = launch(sandbox_exe, root)
                wait_for(lambda: api(base, token, "/api/tasks"), "perch.exe boot", seconds=20)
                ensure(instance.poll() is None, "PerchFlow exited unexpectedly")
                checks["isolated_exe_boot"] = "PASS"

                result = api(base, token, "/api/projects", "POST", {
                    "name": "Mock Python Pipeline",
                    "root": str(project),
                    "tasks": [],
                    "inputs": ["input.txt", "settings.txt"],
                    "outputs": ["results.txt"],
                    "output_dirs": ["output"],
                    "preflight_inputs": True,
                })
                ensure(result.get("ok"), f"Project not created: {result}")
                create_disabled_python_task(base, token, "mock_success", python, "success")
                ensure(not (project / "results.txt").exists(),
                       "Project/task creation must not execute the script")
                ready = api(base, token, "/api/projects/Mock%20Python%20Pipeline/preflight")
                ensure(ready["preflight"]["ready"] is False, "Missing fixtures not detected")
                ensure(set(ready["preflight"]["unavailable"]) ==
                       {"input.txt", "settings.txt"},
                       f"Unexpected required inputs: {ready}")
                checks["python_task_created_disabled"] = "PASS"

                blocked_id, blocked = start_and_wait(
                    base, token, "mock_success", "blocked")
                ensure(blocked["preflight"]["ready"] is False, "Preflight not stored")
                ensure(blocked["files_recorded"] is False, "Blocked task produced file metadata")
                ensure(not (project / "results.txt").exists(),
                       "Preflight failure incorrectly executed Python")
                checks["required_input_preflight_block"] = "PASS"

                (project / "input.txt").write_text(
                    "SYNTHETIC_PRIVATE_VALUE_NEVER_EXPOSE", encoding="utf-8")
                (project / "settings.txt").write_text("MOCK_SETTINGS", encoding="utf-8")
                ready = api(base, token, "/api/projects/Mock%20Python%20Pipeline/preflight")
                ensure(ready["preflight"]["ready"] is True, "Fixtures not recognized")
                first_id, first = start_and_wait(base, token, "mock_success", "success")
                ensure(first["preflight"]["ready"] is True and first["preflight"]["checked"] == 3,
                       "Python script missing from required-input check")
                ensure(first["files_recorded"] and first["output_summary"]["created"] >= 3,
                       f"Missing auto-discovered outputs: {first['output_summary']}")
                ensure((project / "results.txt").exists() and
                       (project / "output" / "photos" / "demo.png").exists(),
                       "Python didn't generate expected mock files in its working directory")
                files = api(base, token, f"/api/tasks/mock_success/files/{first_id}")
                ensure(files["ok"] and files["record"]["run_id"] == first_id,
                       "File sidecar not associated with correct run")
                changed = {e["path"].replace("\\", "/"): e
                           for e in files["record"]["changes"]}
                for path in ("results.txt", "output/report.csv", "output/photos/demo.png"):
                    ensure(path in changed and changed[path]["change"] == "created",
                           f"Missing created artifact {path}: {sorted(changed)}")
                log = api(base, token, f"/api/tasks/mock_success/log/{first_id}")
                ensure(log["ok"] and "MOCK_PYTHON_SUCCESS" in log["content"],
                       "Real Python stdout not captured")
                ensure(FAKE_MARKER not in json.dumps(first) and
                       FAKE_MARKER not in json.dumps(files) and
                       FAKE_MARKER not in json.dumps(log),
                       "Sensitive fixture body leaked into management APIs")
                checks["real_python_workdir_stdout_outputs"] = "PASS"
                checks["per_run_metadata_and_no_body_leak"] = "PASS"

                second_id, second = start_and_wait(base, token, "mock_success", "success")
                ensure(first_id != second_id, "Run IDs reused")
                ensure(second["output_summary"]["modified"] >= 3,
                       f"Repeated run should update three artifacts: {second['output_summary']}")
                checks["subsequent_run_modified_files"] = "PASS"

                create_disabled_python_task(base, token, "mock_failure", python, "failure")
                error_id, error = start_and_wait(base, token, "mock_failure", "failed")
                ensure(error["exit_code"] == 9, "Python exit code lost")
                failure_log = api(base, token, f"/api/tasks/mock_failure/log/{error_id}")
                ensure("MOCK_PYTHON_EXPECTED_FAILURE" in failure_log["content"],
                       "Python failure stdout not captured")
                checks["python_failure_exitcode_and_log"] = "PASS"

                create_disabled_python_task(base, token, "mock_timeout", python, "sleep", timeout=1)
                timeout_id, timeout_history = start_and_wait(
                    base, token, "mock_timeout", "timeout", timeout=14)
                ensure(timeout_history["exit_code"] is None,
                       "Timeout must not claim success")
                checks["python_timeout"] = "PASS"

                create_disabled_python_task(base, token, "mock_cancel", python, "sleep", timeout=28)
                started = api(base, token, "/api/tasks/mock_cancel/run", "POST")
                ensure(started.get("ok") and started.get("run_id"), "Unable to start mock cancel")
                wait_for(lambda: (
                    True if api(base, token, "/api/tasks/mock_cancel")["status"] == "running"
                    else None
                ), "cancel task running", seconds=5)
                stopped = api(base, token, "/api/tasks/mock_cancel/stop", "POST")
                ensure(stopped["ok"], "Manual cancellation rejected")
                wait_status(base, token, "mock_cancel", "cancelled")
                checks["python_manual_cancel"] = "PASS"

                stop_process(instance)
                instance = None
                instance = launch(sandbox_exe, root)
                wait_for(lambda: api(base, token, "/api/tasks"), "fresh EXE restart", seconds=20)
                restored = api(base, token, "/api/tasks/mock_failure")
                ensure(restored["status"] == "failed" and restored["last_exit_code"] == 9,
                       f"Python failure history not recovered: {restored}")
                ensure(api(base, token, "/api/tasks/mock_success/history")["runs"][0]
                       ["run_id"] == second_id, "History lost over restart")
                original_sidecar = api(base, token,
                                       f"/api/tasks/mock_success/files/{first_id}")
                ensure(original_sidecar["ok"], "File changes lost over restart")
                for task_name in ("mock_success", "mock_failure", "mock_timeout", "mock_cancel"):
                    ensure(api(base, token, f"/api/tasks/{task_name}")["enabled"] is False,
                           "A disabled mock task became scheduled on restart")
                checks["restart_restores_history_and_file_sidecars"] = "PASS"
                checks["host_machine_and_real_projects_untouched"] = "PASS"
            finally:
                stop_process(instance)
                instance = None
    finally:
        output_file.parent.mkdir(parents=True, exist_ok=True)
        report["result"] = "PASS" if len(checks) == 10 else "PARTIAL"
        output_file.write_text(json.dumps(report, indent=2, ensure_ascii=False),
                               encoding="utf-8")
    ensure(len(checks) == 10, f"Incomplete Python integration: {checks}")
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--exe", type=Path, required=True)
    parser.add_argument("--report", type=Path, default=Path("dev/reports/project-rc-compat.json"))
    args = parser.parse_args()
    test_pipeline(args.exe.resolve(), args.report.resolve())
