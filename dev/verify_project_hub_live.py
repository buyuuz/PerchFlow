"""Check live-run UI with a fake advancing task and mock-only API.

Requires disposable GitHub-hosted Windows Edge. NEVER invokes TikTok or user's PC.
"""
import json
import os
import sys
import threading
import time
from http.server import ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote, urlsplit
from playwright.sync_api import sync_playwright, expect

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verify_project_hub_browser as fixture

RUN_ID = "live-888-1"
HISTORY_READS = []
LOG_READS = []
STARTED = time.monotonic()

class LiveHandler(fixture.Handler):
    def do_GET(self):
        path = unquote(urlsplit(self.path).path)
        if path in ("/api/tasks/mock_job/history",
                    f"/api/tasks/mock_job/log/{RUN_ID}",
                    f"/api/tasks/mock_job/files/{RUN_ID}"):
            if not self.authenticate():
                return
            elapsed = time.monotonic() - STARTED
            running = elapsed < 6.0
            if path.endswith("/history"):
                HISTORY_READS.append(elapsed)
                run = {"run_id":RUN_ID,"started_at":"2026-10-21T01:00:00Z",
                       "status":"running" if running else "success",
                       "event":"started" if running else "finished",
                       "files_recorded":not running}
                if not running:
                    run.update({"finished_at":"2026-10-21T01:00:06Z",
                        "duration_ms":6000,"exit_code":0,
                        "output_summary":{"created":1,"modified":0,"removed":0,
                                          "incomplete_scan":False}})
                return self.api(200,{"ok":True,"runs":[run, {
                    "run_id":"123-1-1","started_at":"2026-10-20T00:00:00Z",
                    "status":"success","duration_ms":1234,"files_recorded":True,
                    "output_summary":{"created":1,"modified":0,"removed":0}}]})
            if "/log/" in path:
                LOG_READS.append(elapsed)
                content = "LIVE_LINE_1"
                if elapsed >= 3.0: content += "\nLIVE_LINE_2"
                if not running: content += "\nLIVE_DONE"
                return self.api(200,{"ok":True,"content":content,"truncated":False})
            if running:
                return self.api(200,{"ok":False,"error":"record not yet available"})
            record=dict(fixture.CHANGES)
            record["run_id"]=RUN_ID
            return self.api(200,{"ok":True,"record":record})
        return super().do_GET()

def check_ci():
    assert os.name == "nt"
    assert os.environ.get("GITHUB_ACTIONS") == "true"
    assert os.environ.get("RUNNER_OS") == "Windows"
    assert os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"

def main():
    check_ci()
    global STARTED
    server=ThreadingHTTPServer(("127.0.0.1",0),LiveHandler)
    runner=threading.Thread(target=server.serve_forever,daemon=True)
    runner.start()
    folder=Path("dev/reports"); folder.mkdir(parents=True,exist_ok=True)
    checks={}
    try:
        with sync_playwright() as playwright:
            browser=playwright.chromium.launch(channel="msedge",headless=True)
            page=browser.new_page(viewport={"width":1280,"height":800})
            page.add_init_script(
                "localStorage.setItem('perch_token',%s);localStorage.setItem('perch-lang','zh');"
                % json.dumps(fixture.TOKEN))
            page.goto(f"http://127.0.0.1:{server.server_port}/projects",wait_until="networkidle")
            STARTED = time.monotonic()
            page.get_by_role("button",name="执行详情").click()
            expect(page.locator("#project-run-summary")).to_contain_text("执行状态：运行中")
            expect(page.locator("#project-run-changes")).to_contain_text("任务正在运行")
            assert not LOG_READS, "log must not be accessed before user opt-in"
            checks["log_opt_in_required"]="PASS"

            page.get_by_role("button",name="按需查看该次日志").click()
            expect(page.locator("#project-run-log")).to_contain_text("LIVE_LINE_1")
            expect(page.locator("#project-run-log")).to_contain_text("LIVE_LINE_2",timeout=7500)
            checks["live_stdout_refresh_without_button"]="PASS"
            expect(page.locator("#project-run-summary")).to_contain_text("执行状态：成功",timeout=12000)
            expect(page.locator("#project-run-summary")).to_contain_text("新增 1")
            expect(page.locator("#project-run-changes")).to_contain_text("generated.xlsx")
            expect(page.locator("#project-run-log")).to_contain_text("LIVE_DONE",timeout=5000)
            checks["run_completion_and_file_changes"]="PASS"
            assert len(HISTORY_READS)>=2, "history was not polled in the background"
            assert len(LOG_READS)>=2, "opted-in log was not polled while running"
            checks["live_polling_active"]="PASS"

            # Selecting an older run must not jump back to the newest run.
            page.locator(".pf-project-run-item").nth(1).click()
            expect(page.locator("#project-run-summary")).to_contain_text("123-1-1")
            page.wait_for_timeout(2200)
            expect(page.locator("#project-run-summary")).to_contain_text("123-1-1")
            checks["manual_history_selection_persists"]="PASS"

            page.locator("#project-runs-close").click()
            count=len(HISTORY_READS)
            page.wait_for_timeout(2400)
            assert count==len(HISTORY_READS), "closed panel kept polling run details"
            checks["close_stops_polling"]="PASS"

            page.screenshot(path=str(folder/"project-hub-live-details.png"),full_page=True)
            browser.close()
        (folder/"project-hub-live.json").write_text(
            json.dumps({"checks":checks,"production_called":False},indent=2),
            encoding="utf-8")
        print("PASS opt-in live log and 2s task details",json.dumps(checks))
    finally:
        server.shutdown()
        server.server_close()

if __name__=="__main__":
    main()
