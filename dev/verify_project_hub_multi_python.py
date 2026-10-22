"""PF001...PF004: actual PerchFlow binary with TWO synthetic Python scripts.

Disposable GitHub-hosted Windows only. No TikTok, browsers, real user files
or native document-opening calls are used by this test.
"""
import argparse
import hashlib
import json
import os
import secrets
import shutil
import sys
import tempfile
import time
from pathlib import Path
from simulate_task_runtime_binary import (
    api, ensure, free_loopback_port, launch, stop_process, wait_for, wait_status
)

def guard(exe):
    ensure(os.name == "nt", "Windows required")
    ensure(os.environ.get("GITHUB_ACTIONS")=="true", "GitHub Actions required")
    ensure(os.environ.get("RUNNER_OS")=="Windows", "Windows runner required")
    ensure(os.environ.get("RUNNER_ENVIRONMENT")=="github-hosted", "Only disposable hosted runner")
    ensure(exe.is_file() and exe.name.lower()=="perch.exe", "real perch.exe is required")

def task_def(name,exe,script,project):
    return (
      "[[task]]\n"
      f"name = {json.dumps(name)}\n"
      f"cmd = {json.dumps(str(exe))}\n"
      f"args = [{json.dumps(str(script))}]\n"
      f"cwd = {json.dumps(str(project))}\n"
      'schedule = "0 0 0 * * *"\n'
      "enabled = false\n"
      "timeout_secs = 20\n\n"
    )

def run(base, token, name, seconds=20):
    started=api(base,token,f"/api/tasks/{name}/run","POST")
    ensure(started.get("ok"),f"Unable to start test task {name}: {started}")
    rid=started["run_id"]
    status=wait_status(base,token,name,"success",seconds=seconds)
    ensure(status["run_id"]==rid, "run id mismatch")
    return rid

def main(exe,report_file):
    guard(exe)
    checks={}
    process=None
    try:
        with tempfile.TemporaryDirectory(prefix="perch-pf001-004-mock-") as root_str:
            root=Path(root_str).resolve()
            shutil.copy2(exe,root/"perch.exe")
            folder=root/"fake_python"
            folder.mkdir()
            a=folder/"mock_publisher.py"
            b=folder/"mock_sync.py"
            (folder/"report.xlsx").write_text("MOCK XLSX NAME ONLY", encoding="utf-8")
            (folder/"settings.txt").write_text("MOCK TXT CONTENT",encoding="utf-8")
            a.write_text(
                'from pathlib import Path\n'
                'import time\n'
                'path = Path("report.xlsx")\n'
                'print("MOCK_A_STARTED", flush=True)\n'
                'time.sleep(2.0)\n'
                'Path("generated_a.txt").write_text(str(time.time_ns()))\n'
                'print("MOCK_A_DONE", flush=True)\n',
                encoding="utf-8")
            b.write_text(
                'from pathlib import Path\n'
                'path = Path("settings.txt")\n'
                'print("MOCK_B_ONLY", flush=True)\n'
                'Path("generated_b.txt").write_text("mock")\n',
                encoding="utf-8")
            port=free_loopback_port()
            token=secrets.token_urlsafe(28)
            cfg=(
                "[server]\n"
                f'bind = "127.0.0.1:{port}"\n'
                f"token = {json.dumps(token)}\n"
                'lang = "zh"\n\n'
                +task_def("fake_A",Path(sys.executable).resolve(),a,folder)
                +task_def("fake_B",Path(sys.executable).resolve(),b,folder)
                +"[[project]]\n"
                'name = "Fake Python Project"\n'
                f"root = {json.dumps(str(folder))}\n"
                'tasks = ["fake_A", "fake_B"]\n'
                'inputs = []\noutputs = ["generated_a.txt", "generated_b.txt"]\n'
                'preflight_inputs = false\n'
            )
            (root/"perch.toml").write_text(cfg,encoding="utf-8")
            base=f"http://127.0.0.1:{port}"
            try:
                process=launch(root/"perch.exe",root)
                wait_for(lambda: api(base,token,"/api/tasks"),"real EXE startup",seconds=25)
                task_list=api(base,token,"/api/tasks")["tasks"]
                ensure(len(task_list)==2 and all(not task["enabled"] for task in task_list),
                    "Both fake Python entries must be independent and disabled")
                checks["two_distinct_disabled_python_tasks"]="PASS"

                rid_b=run(base,token,"fake_B")
                ensure("MOCK_B_ONLY" in api(base,token,f"/api/tasks/fake_B/log/{rid_b}")["content"],
                    "B script log missing")
                checks["second_script_distinct_stdout"]="PASS"

                ids=[]
                for i in range(4):
                    started=api(base,token,"/api/tasks/fake_A/run","POST")
                    ensure(started.get("ok"),f"start fake A run {i}: {started}")
                    rid=started["run_id"]
                    if i == 0:
                        running=wait_for(lambda: (
                            view if (view:=api(base,token,"/api/tasks/fake_A")).get("pid") is not None
                            and view.get("status")=="running" else None
                        ),"task child PID",seconds=8)
                        ensure(isinstance(running["pid"],int) and running["pid"]>0,
                            f"PID not exposed: {running}")
                        ensure(running["uptime_secs"]>=0, "No runtime data")
                        checks["actual_child_pid_and_runtime"]="PASS"
                        rejected=api(base,token,
                            "/api/projects/Fake%20Python%20Project/tasks/fake_A","DELETE")
                        ensure(rejected.get("ok") is False and "running" in rejected.get("error",""),
                            f"Cannot delete a running task: {rejected}")
                        checks["pf032_running_project_script_cannot_be_deleted"]="PASS"

                        log=wait_for(lambda: (
                            entry if (view:=api(base,token,"/api/tasks/fake_A")).get("status")=="running"
                            and "MOCK_A_STARTED" in (entry:=api(base,token,f"/api/tasks/fake_A/log/{rid}")).get("content","")
                            else None
                        ), "Python first line visible before child exit", seconds=1.7)
                        ensure("MOCK_A_STARTED" in log["content"], "stdout not visible while running")
                        checks["unbuffered_live_log_before_completion"]="PASS"
                    finish=wait_status(base,token,"fake_A","success",seconds=12)
                    ensure(finish["run_id"]==rid, "A run id mismatch")
                    ids.append(rid)
                checks["four_independent_reruns"]="PASS"
                history=api(base,token,"/api/tasks/fake_A/history")["runs"]
                by_id={h["run_id"]:h for h in history}
                ensure(all(rid in by_id for rid in ids), "Persistent history should retain all runs")
                ensure(by_id[ids[0]]["log_available"] is False, "old log incorrectly retained")
                ensure(all(by_id[rid]["log_available"] is True for rid in ids[1:]),
                       "newer three run logs not available")
                checks["log_retention_three_and_history_preserved"]="PASS"
                def task_log_dir(task_name):
                    return root/"task-logs"/hashlib.sha256(task_name.encode("utf-8")).hexdigest()
                log_files=list(task_log_dir("fake_A").glob("*.log"))
                ensure(len(log_files)==3, f"expected 3 A logs, got {log_files}")
                ensure((task_log_dir("fake_B")/f"{rid_b}.log").is_file(),
                       "A retention affected other task B")
                ensure(not (root/"task-logs"/"v2").exists(),
                       "new task runs must not create v2 subdirectory")
                checks["retention_does_not_cross_task_boundaries"]="PASS"
                stop=api(base,token,"/api/tasks/fake_A/stop","POST")
                ensure(stop.get("already_finished") and stop.get("status")=="success",
                       f"Already-finished stop should not be a server error: {stop}")
                checks["stop_after_completion_is_idempotent"]="PASS"

                candidates=api(base,token,"/api/projects/Fake%20Python%20Project/tasks/fake_A/files")["files"]
                ensure(any(f["path"]=="report.xlsx" and f["exists"] for f in candidates),
                       f"Code path candidate missing: {candidates}")
                ensure(not any("settings.txt"==f["path"] for f in candidates),
                       "Unrelated B-only file incorrectly discovered in A")
                checks["bounded_per_script_file_candidate_discovery"]="PASS"
                # Never call confirmed:true or open a document in CI.
                denied=api(base,token,"/api/projects/Fake%20Python%20Project/tasks/fake_A/open-file",
                           "POST",{"path":str(folder/"report.xlsx"),"confirmed":False})
                ensure(denied.get("ok") is False, "Missing confirmation must fail")
                checks["document_open_requires_explicit_user_confirmation"]="PASS"
                # New Cron defaults enabled when attached to an unscheduled project script.
                cleared=api(base,token,"/api/tasks/fake_B/schedule","DELETE")
                ensure(cleared.get("ok") and not cleared["enabled"],f"clear cron: {cleared}")
                created=api(base,token,"/api/tasks/fake_B/schedule","PUT",
                    {"schedule":"0 0 3 * * *"})
                ensure(created.get("ok") and created["enabled"],f"new cron should enable: {created}")
                paused=api(base,token,
                    "/api/projects/Fake%20Python%20Project/tasks/fake_B/schedule","PUT",
                    {"enabled":False})
                ensure(paused.get("ok"),f"pause cron failed: {paused}")
                edited=api(base,token,"/api/tasks/fake_B/schedule","PUT",
                    {"schedule":"0 0 4 * * *"})
                ensure(edited.get("ok") and edited["enabled"] is False,
                    f"editing Cron must preserve disabled state: {edited}")
                checks["pf027_cron_creation_enables_and_edit_preserves_status"]="PASS"

                # PF-032: real Windows EXE backend deletes task identity/association
                # transactionally, but leaves all disk files and run-log history intact.
                removed=api(base,token,
                    "/api/projects/Fake%20Python%20Project/tasks/fake_A","DELETE")
                ensure(removed.get("ok") and removed["source_files_deleted"] is False,
                    f"project-scoped task deletion failed: {removed}")
                after=api(base,token,"/api/projects/Fake%20Python%20Project")["project"]
                ensure("fake_A" not in after["tasks"] and "fake_B" in after["tasks"],
                    f"project association incorrect after delete: {after}")
                remaining=api(base,token,"/api/tasks")["tasks"]
                ensure({item["name"] for item in remaining}=={"fake_B"},
                    f"unrelated task lost: {remaining}")
                ensure(a.is_file() and b.is_file() and
                    (folder/"report.xlsx").is_file() and (folder/"settings.txt").is_file(),
                    "PF032 must not delete Python source or registered documents")
                ensure(len(list(task_log_dir("fake_A").glob("*.log")))==3,
                    "PF032 must not remove retained execution logs")
                checks["pf032_project_delete_deregisters_only_and_preserves_files"]="PASS"
                # PF038: a retired task name can be reused, but its old
                # log files and run history must not show as the new task.
                resurrected=api(base,token,"/api/tasks","POST",{
                    "name":"fake_A","cmd":str(Path(sys.executable).resolve()),
                    "args":[str(a)],"cwd":str(folder),"schedule":"","enabled":False,
                    "timeout_secs":20
                })
                ensure(resurrected.get("ok"),f"recreate fake_A: {resurrected}")
                previous=api(base,token,"/api/tasks/fake_A/history")["runs"]
                ensure(not previous,f"new task inherited old run history: {previous}")
                old_log=api(base,token,f"/api/tasks/fake_A/log/{ids[-1]}")
                ensure(old_log.get("ok") is False,
                    f"new task read a predecessor's log: {old_log}")
                new_rid=run(base,token,"fake_A")
                new_history=api(base,token,"/api/tasks/fake_A/history")["runs"]
                ensure(new_history and new_history[0]["run_id"]==new_rid
                    and all(row["run_id"] not in ids for row in new_history),
                    f"recreated task's history not isolated: {new_history}")
                checks["pf038_recreated_name_has_independent_history_and_logs"]="PASS"
                # PF039: clean only the auto-linked Python input when deleting
                # its final owning script, with the source file still on disk.
                registered=api(base,token,"/api/projects/Fake%20Python%20Project/python-task",
                    "POST",{"name":"fake_C","python":str(Path(sys.executable).resolve()),
                             "script":"mock_publisher.py"})
                ensure(registered.get("ok"),f"register fake_C: {registered}")
                before=api(base,token,"/api/projects/Fake%20Python%20Project")["project"]
                ensure("mock_publisher.py" in before["inputs"],
                    f"expected automatic Python input: {before}")
                deleted=api(base,token,
                    "/api/projects/Fake%20Python%20Project/tasks/fake_C","DELETE")
                ensure(deleted.get("ok"),f"delete fake_C: {deleted}")
                after=api(base,token,"/api/projects/Fake%20Python%20Project")["project"]
                ensure("mock_publisher.py" not in after["inputs"]
                    and "fake_C" not in after["tasks"] and a.is_file(),
                    f"stale input retained or real file removed: {after}")
                checks["pf039_remove_exclusive_python_input_without_deleting_file"]="PASS"
                # PF041: start and delete race on one disposable Python script.
                # The lifecycle mutex must make these mutually exclusive.
                created=api(base,token,
                    "/api/projects/Fake%20Python%20Project/python-task","POST",
                    {"name":"fake_D","python":str(Path(sys.executable).resolve()),
                     "script":"mock_publisher.py"})
                ensure(created.get("ok"),f"register fake_D: {created}")
                import threading
                from concurrent.futures import ThreadPoolExecutor
                gate=threading.Barrier(2)
                def raced(path,method):
                    gate.wait(timeout=7)
                    return api(base,token,path,method)
                with ThreadPoolExecutor(max_workers=2) as pool:
                    fr=pool.submit(raced,"/api/tasks/fake_D/run","POST")
                    fd=pool.submit(raced,
                        "/api/projects/Fake%20Python%20Project/tasks/fake_D","DELETE")
                    start_response=fr.result(timeout=12)
                    delete_response=fd.result(timeout=12)
                ensure(not (start_response.get("ok") and delete_response.get("ok")),
                    f"delete/start both succeeded; stale running task: {start_response}/{delete_response}")
                if start_response.get("ok"):
                    ensure(delete_response.get("ok") is False,
                        "cannot delete after run was claimed")
                    wait_status(base,token,"fake_D","success",seconds=12)
                    deleted=api(base,token,
                        "/api/projects/Fake%20Python%20Project/tasks/fake_D","DELETE")
                    ensure(deleted.get("ok"),f"cleanup fake_D: {deleted}")
                else:
                    ensure(delete_response.get("ok"),
                        f"start and delete both failed: {start_response}/{delete_response}")
                state=api(base,token,"/api/projects/Fake%20Python%20Project")["project"]
                ensure("fake_D" not in state["tasks"] and a.is_file(),
                    f"race left ghost task or deleted source: {state}")
                checks["pf041_run_delete_race_does_not_orphan_task"]="PASS"



            finally:
                stop_process(process)
                process=None
    finally:
        report_file.parent.mkdir(parents=True,exist_ok=True)
        report_file.write_text(json.dumps({
            "checks":checks,"real_business_called":False,"real_document_opened":False,
            "environment":"disposable GitHub Windows runner"
        },indent=2),encoding="utf-8")
    ensure(len(checks)==16, f"Incomplete checks: {checks}")
    print("PASS PF001-PF004 actual EXE with fake scripts",json.dumps(checks,indent=2))

if __name__=="__main__":
    p=argparse.ArgumentParser()
    p.add_argument("--exe",type=Path,required=True)
    p.add_argument("--report",type=Path,default=Path("dev/reports/pf001-pf004-runtime.json"))
    a=p.parse_args()
    main(a.exe.resolve(),a.report.resolve())
