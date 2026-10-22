"""Windows-only real PerchFlow EXE regression: disable Cron mid-run and repeat start/delete races.

Runs only on ephemeral GitHub-hosted Windows CI, with a copied EXE, temporary
TOML, randomized loopback port, and synthetic Python scripts.
No production directories, scheduling config, or real scripts are touched.
"""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import secrets
import shutil
import sys
import tempfile
import threading
import time

from simulate_task_runtime_binary import (
    api, ensure, free_loopback_port, launch, stop_process, wait_for,
)

RUN_ROUNDS = 24


def check_environment(exe):
    ensure(os.name == "nt", "Windows runner required")
    ensure(os.environ.get("GITHUB_ACTIONS") == "true", "GitHub Actions only")
    ensure(os.environ.get("RUNNER_OS") == "Windows", "Windows runner only")
    ensure(os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted",
           "Refusing execution outside disposable GitHub-hosted runner")
    ensure(exe.is_file() and exe.name.lower() == "perch.exe", "built EXE required")


def new_task(base, token, name, script, root, project=None):
    payload = {
        "name": name, "cmd": str(Path(sys.executable).resolve()),
        "args": [str(script)], "cwd": str(root),
        "schedule": "", "enabled": False, "timeout_secs": 25,
    }
    response = api(base, token, "/api/tasks", "POST", payload)
    ensure(response.get("ok"), f"create {name}: {response}")


def view(base, token, name):
    return api(base, token, "/api/tasks/" + name)


def main(exe, report_file):
    check_environment(exe)
    result = {
        "environment": "ephemeral github-hosted windows runner",
        "source_files_deleted": False,
        "real_business_called": False,
        "tests": {},
        "race_rounds": RUN_ROUNDS,
        "outcomes": {"run_first": 0, "delete_first": 0},
    }
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="perch-cron-race-isolated-") as wd:
            root = Path(wd).resolve()
            binexe = root / "perch.exe"
            shutil.copy2(exe, binexe)
            work = root / "mock_sources"
            work.mkdir()
            cron_script = work / "cron_dummy.py"
            cron_script.write_text(
                "from pathlib import Path\n"
                "import time\n"
                "p=Path('cron_starts.txt')\n"
                "with p.open('a',encoding='utf-8') as f:\n"
                " f.write('start\\n')\n"
                "print('DUMMY_CRON_STARTED',flush=True)\n"
                "time.sleep(3.6)\n"
                "Path('cron_complete.txt').write_text('completed')\n"
                "print('DUMMY_CRON_FINISHED',flush=True)\n",
                encoding="utf-8",
            )
            race_script = work / "race_dummy.py"
            race_script.write_text(
                "import time\n"
                "print('DUMMY_RACE_STARTED',flush=True)\n"
                "time.sleep(0.85)\n"
                "print('DUMMY_RACE_COMPLETE',flush=True)\n",
                encoding="utf-8",
            )
            marker = work / "important_input.txt"
            marker.write_text("PRESERVE", encoding="utf-8")
            port = free_loopback_port()
            token = secrets.token_urlsafe(28)
            config = (
                "[server]\n"
                f'bind = "127.0.0.1:{port}"\n'
                f"token = {json.dumps(token)}\n"
                'lang = "zh"\n'
            )
            (root / "perch.toml").write_text(config, encoding="utf-8")
            base = f"http://127.0.0.1:{port}"
            try:
                process = launch(binexe, root)
                wait_for(lambda: api(base, token, "/api/tasks"),
                         "sandbox EXE ready", seconds=25)

                # IMPORTANT: starts due to actual Cron, not the Run button.
                new_task(base, token, "cron_dummy", cron_script, work)
                plan = api(base, token, "/api/tasks/cron_dummy/schedule", "PUT",
                           {"schedule": "*/2 * * * * *", "enabled": True})
                ensure(plan.get("ok") and plan.get("enabled"), f"enable cron: {plan}")
                running = wait_for(
                    lambda: ((v := view(base, token, "cron_dummy"))
                             if v.get("status") == "running" and v.get("pid") else None),
                    "Cron-triggered dummy process", seconds=12, interval=0.15,
                )
                original_run_id = running["run_id"]
                ensure(original_run_id, f"missing run id for Cron: {running}")
                disabled = api(base, token, "/api/tasks/cron_dummy/enabled", "PUT",
                               {"enabled": False})
                ensure(disabled.get("ok") and disabled.get("enabled") is False,
                       f"disable during run: {disabled}")
                still = view(base, token, "cron_dummy")
                ensure(still["run_id"] == original_run_id and still["status"] == "running",
                       f"disabling Cron interrupted running job: {still}")
                finish = wait_for(
                    lambda: ((v := view(base, token, "cron_dummy"))
                             if v.get("status") == "success"
                             and v.get("run_id") == original_run_id else None),
                    "in-progress Cron child completes successfully", seconds=12, interval=0.18,
                )
                ensure((work / "cron_complete.txt").read_text() == "completed",
                       "Cron child did not finish normally")
                result["tests"]["disable_cron_preserves_running_child"] = {
                    "status": "PASS", "run_id": original_run_id,
                    "exit_code": finish["last_exit_code"],
                }
                # A tick every 2s; check for >2 intervals after prior run ends.
                time.sleep(5.3)
                after = view(base, token, "cron_dummy")
                starts = (work / "cron_starts.txt").read_text().splitlines()
                history = api(base, token, "/api/tasks/cron_dummy/history")["runs"]
                ensure(len(starts) == 1 and len(history) == 1
                       and after["run_id"] == original_run_id
                       and after["enabled"] is False,
                       f"additional cron execution after disabling: starts={starts},"
                       f" history={history}, view={after}")
                result["tests"]["disabled_cron_has_no_followup_ticks"] = {
                    "status": "PASS", "observed_seconds": 5.3,
                    "start_count": len(starts), "history_count": len(history),
                }

                # 24 distinct registrations, running actual Python script,
                # racing two real authenticated API requests each iteration.
                for i in range(RUN_ROUNDS):
                    name = f"race_{i:02d}"
                    new_task(base, token, name, race_script, work)
                    gate = threading.Barrier(2)

                    def request_together(action):
                        gate.wait(timeout=8)
                        if action == "run":
                            return api(base, token, f"/api/tasks/{name}/run", "POST")
                        return api(base, token, f"/api/tasks/{name}", "DELETE")

                    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                        fr = pool.submit(request_together, "run")
                        fd = pool.submit(request_together, "delete")
                        run_response = fr.result(timeout=15)
                        delete_response = fd.result(timeout=15)
                    run_ok = run_response.get("ok") is True
                    delete_ok = delete_response.get("ok") is True
                    ensure(run_ok != delete_ok,
                           f"round {i}: both succeeded or both failed: "
                           f"{run_response} / {delete_response}")
                    if run_ok:
                        ensure("running" in str(delete_response.get("error", "")),
                               f"round {i} delete did not refuse claimed run: {delete_response}")
                        wait_for(
                            lambda: ((v := view(base, token, name))
                                     if v.get("status") == "success" else None),
                            f"round {i} run completion", seconds=10, interval=0.1,
                        )
                        deleted = api(base, token, f"/api/tasks/{name}", "DELETE")
                        ensure(deleted.get("ok"), f"round {i} cleanup deletion: {deleted}")
                        result["outcomes"]["run_first"] += 1
                    else:
                        ensure(delete_ok and not run_ok,
                               f"round {i}: unexpected winner: {run_response}/{delete_response}")
                        result["outcomes"]["delete_first"] += 1
                    tasks = api(base, token, "/api/tasks")["tasks"]
                    ensure(name not in {task["name"] for task in tasks},
                           f"round {i}: ghost registration persisted")
                    ensure(race_script.exists() and cron_script.exists()
                           and marker.read_text() == "PRESERVE",
                           f"round {i}: source/marker unexpectedly modified")
                result["tests"]["repeated_start_delete_races"] = {
                    "status": "PASS", "rounds": RUN_ROUNDS,
                    "outcomes": result["outcomes"],
                    "preserved_sources": True,
                }
                ensure(all(test["status"] == "PASS" for test in result["tests"].values()),
                       "incomplete verification results")
            finally:
                stop_process(process)
                process = None
    except Exception as exc:
        result["failure"] = str(exc)
        raise
    finally:
        report_file.parent.mkdir(parents=True, exist_ok=True)
        report_file.write_text(json.dumps(result, ensure_ascii=False, indent=2),
                               encoding="utf-8")
    print("CRON_RACE_VERIFICATION_PASS", json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--exe", type=Path, required=True)
    p.add_argument("--report", type=Path,
                   default=Path("dev/reports/cron-race-verification.json"))
    args = p.parse_args()
    main(args.exe.resolve(), args.report.resolve())
