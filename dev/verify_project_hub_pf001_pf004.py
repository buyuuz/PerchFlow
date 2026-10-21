"""PF-001...PF-004 opt-in UI regression: fake Windows task data only.

No real Python scripts, network services or Windows file opening are used.
"""
import json
import os
import re
import threading
import time
from http.server import ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote, urlsplit

from playwright.sync_api import sync_playwright, expect
import verify_project_hub_browser as mock

TOKEN = mock.TOKEN
FILE_REQUESTS = []
LOG_READS = []
STATUS = {"running": True, "stopped_at": None}
RUN_ID = "1771700000000-11-4"

class Handler(mock.Handler):
    def do_GET(self):
        path = unquote(urlsplit(self.path).path)
        if path == "/task-detail":
            self.path = "/task-detail.html"
            return super().do_GET()
        if path == "/api/tasks":
            if not self.authenticate(): return
            result=dict(mock.TASKS[0])
            result.update({"status":"running" if STATUS["running"] else "success",
                           "pid":4444 if STATUS["running"] else None,
                           "run_id":RUN_ID})
            return self.api(200,{"tasks":[result]})
        if path == "/api/tasks/mock_job":
            if not self.authenticate(): return
            result = dict(mock.TASKS[0])
            result.update({"status":"running" if STATUS["running"] else "success",
                           "pid": 4444 if STATUS["running"] else None,
                           "cpu_percent": 2.5, "mem_bytes": 10485760, "uptime_secs": 68,
                           "last_exit_code":0,"run_id":RUN_ID})
            return self.api(200,result)
        if path == "/api/tasks/mock_job/history":
            if not self.authenticate(): return
            runs=[]
            for i in range(4,0,-1):
                runs.append({"run_id":f"1771700000000-11-{i}",
                             "started_at":f"2026-10-21T00:00:0{i}Z",
                             "status":"running" if i==4 and STATUS["running"] else "success",
                             "duration_ms":None if i==4 and STATUS["running"] else 800,
                             "log_available":i>=2})
            return self.api(200,{"ok":True,"runs":runs})
        if path.startswith("/api/tasks/mock_job/log/"):
            if not self.authenticate(): return
            LOG_READS.append(path)
            return self.api(200,{"ok":True,"content":"FAKE_START\nFAKE_SECOND_LINE"
                             if len(LOG_READS)>=2 else "FAKE_START","truncated":False})
        if path == f"/api/projects/{mock.PROJECT_NAME}/tasks/mock_job/files":
            if not self.authenticate(): return
            return self.api(200,{"ok":True,"files":[{
                "path":"tasks.xlsx","resolved_path":r"C:\MOCK\Doc\tasks.xlsx",
                "source":"code_candidate","exists":True,"size_bytes":111,
                "inferred":True}],"partial":True})
        return super().do_GET()

    def do_POST(self):
        path = unquote(urlsplit(self.path).path)
        if path == f"/api/projects/{mock.PROJECT_NAME}/tasks/mock_job/open-file":
            if not self.authenticate(): return
            length=int(self.headers.get("Content-Length",0))
            data=json.loads(self.rfile.read(length))
            assert data=={"path":r"C:\MOCK\Doc\tasks.xlsx","confirmed":True}
            FILE_REQUESTS.append(data)
            return self.api(200,{"ok":True,"requested":True})
        return super().do_POST()

def main():
    assert os.name == "nt"
    assert os.environ.get("GITHUB_ACTIONS") == "true"
    assert os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"
    server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
    thread=threading.Thread(target=server.serve_forever,daemon=True)
    thread.start()
    checks={}
    try:
        with sync_playwright() as playwright:
            browser=playwright.chromium.launch(channel="msedge",headless=True)
            page=browser.new_page(viewport={"width":1366,"height":860})
            page.add_init_script('localStorage.setItem("perch_token",%s);localStorage.setItem("perch-lang","zh");' % json.dumps(TOKEN))
            page.goto(f"http://127.0.0.1:{server.server_port}/projects",wait_until="networkidle")
            expect(page.locator("#project-current-name")).to_have_text(mock.PROJECT_NAME)
            expect(page.locator(".pf-project-task-item")).to_contain_text("PID")
            expect(page.locator(".pf-project-task-item")).to_contain_text("运行中")
            expect(page.locator('[data-task-action="stop"]')).to_be_enabled()
            STATUS["running"]=False
            expect(page.locator(".pf-project-task-item")).to_contain_text("成功",timeout=6500)
            expect(page.locator('[data-task-action="stop"]')).to_be_disabled()
            checks["pf001_status_sync_without_manual_refresh"]="PASS"

            page.locator(".pf-project-detail-link").click()
            expect(page).to_have_url(re.compile(r"/task-detail\?"))
            expect(page.locator("#task-name")).to_contain_text("mock_job")
            expect(page.locator("#task-pid")).to_have_text("—")
            expect(page.locator("#task-python")).to_contain_text("cmd")
            checks["pf003_dedicated_task_detail"]="PASS"
            expect(page.locator("#task-history .pf-taskdetail-history-item")).to_have_count(4)
            expect(page.locator("#task-log-count")).to_have_text("0 行")
            assert len(LOG_READS)==0
            checks["pf004_new_log_starts_zero_and_opt_in"]="PASS"

            page.locator("#task-log-show").click()
            expect(page.locator("#task-log-content")).to_contain_text("FAKE_START")
            expect(page.locator("#task-log-content")).to_contain_text("FAKE_SECOND_LINE",timeout=6500)
            checks["pf004_live_independent_log_updates"]="PASS"

            expect(page.locator("#task-linked-files")).to_contain_text("tasks.xlsx")
            assert not FILE_REQUESTS
            page.once("dialog",lambda d:d.dismiss())
            page.get_by_role("button",name="打开文件").click()
            assert not FILE_REQUESTS
            page.once("dialog",lambda d:d.accept())
            page.get_by_role("button",name="打开文件").click()
            expect(page.locator("#task-message")).to_be_visible()
            assert len(FILE_REQUESTS)==1
            checks["pf003_file_open_confirmation"]="PASS"
            page.screenshot(path="dev/reports/pf001-pf004-task-detail.png",full_page=True)
            browser.close()
        Path("dev/reports/pf001-pf004.json").write_text(
            json.dumps({"checks":checks,"real_business_executed":False,
                        "real_file_opened":False},indent=2),encoding="utf-8")
        print("PASS PF001...PF004 mock UI",json.dumps(checks))
    finally:
        server.shutdown()
        server.server_close()

if __name__ == "__main__":
    Path("dev/reports").mkdir(parents=True,exist_ok=True)
    main()
