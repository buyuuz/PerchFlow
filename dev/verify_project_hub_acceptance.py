"""Visual and interaction acceptance checks for Project Hub V1-V5.
GitHub-hosted Windows + mock API ONLY; never runs the actual PerchFlow daemon.
"""
import json
import os
import sys
import threading
import time
from pathlib import Path
from http.server import ThreadingHTTPServer
from urllib.parse import unquote, urlsplit

from playwright.sync_api import sync_playwright, expect

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verify_project_hub_browser as mock

OUT = Path("dev/reports")
OUT.mkdir(parents=True, exist_ok=True)
SECOND = {
    "name": "Second Mock Project", "root": r"C:\CI\SECOND_MOCK",
    "tasks": ["second_mock"], "inputs": [], "outputs": [],
    "output_dirs": [], "preflight_inputs": False,
}
SECOND_TASK = {"name":"second_mock","cmd":"cmd","args":["/c","echo MOCK"],
    "cwd":SECOND["root"],"schedule":"0 0 0 * * *","enabled":False,
    "status":"idle","project":SECOND["name"]}
mode = "normal"
MANY = [{
    "name":f"Mock Project {i:02d}", "root":fr"C:\CI\PROJECT_{i:02d}",
    "tasks":[], "inputs":[], "outputs":[], "output_dirs":[], "preflight_inputs":False
} for i in range(1,41)]

def check_ci():
    assert os.name == "nt"
    assert os.environ.get("GITHUB_ACTIONS") == "true"
    assert os.environ.get("RUNNER_OS") == "Windows"
    assert os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"

class AcceptanceHandler(mock.Handler):
    def do_GET(self):
        global mode
        path = unquote(urlsplit(self.path).path)
        if path.startswith("/api/") and mode != "normal":
            if not self.authenticate():
                return
            if mode == "empty":
                if path == "/api/projects":
                    return self.api(200, {"projects":[]})
                if path == "/api/tasks":
                    return self.api(200, {"tasks":[]})
            if mode == "many":
                if path == "/api/projects":
                    return self.api(200, {"projects":[mock.PROJECT]+MANY})
                if path == "/api/tasks":
                    return self.api(200, {"tasks":mock.TASKS})
                for item in MANY:
                    if path == "/api/projects/" + item["name"]:
                        return self.api(200, {"ok":True,"project":item,"files":[]})
                    if path == "/api/projects/" + item["name"] + "/preflight":
                        return self.api(200, {"ok":True,"preflight":{
                            "enabled":False,"ready":True,"checked":0,"unavailable":[],"reason":None}})
            if mode == "error" and path == "/api/projects":
                return self.api(503, {"ok":False,"error":"MOCK_SERVER_UNAVAILABLE"})
            if mode == "switch":
                if path == "/api/projects":
                    return self.api(200, {"projects":[mock.PROJECT,SECOND]})
                if path == "/api/tasks":
                    return self.api(200, {"tasks":mock.TASKS+[SECOND_TASK]})
                if path == "/api/projects/Second Mock Project":
                    return self.api(200, {"ok":True,"project":SECOND,"files":[]})
                if path == "/api/projects/Second Mock Project/preflight":
                    return self.api(200, {"ok":True,"preflight":{
                        "enabled":False,"ready":True,"checked":0,"unavailable":[],"reason":None}})
                if path == "/api/tasks/mock_job/history":
                    time.sleep(1.2)  # deliberately stale response
        return super().do_GET()

def prep(page, lang):
    page.add_init_script(
        "localStorage.setItem('perch_token',%s);localStorage.setItem('perch-lang',%s);" %
        (json.dumps(mock.TOKEN),json.dumps(lang))
    )

def assert_no_horizontal_scroll(page, viewport):
    widths=page.evaluate("({body:document.body.scrollWidth,doc:document.documentElement.scrollWidth,w:innerWidth})")
    assert widths["doc"] <= viewport+1, f"horizontal overflow at {viewport}: {widths}"

def main():
    global mode
    check_ci()
    server=ThreadingHTTPServer(("127.0.0.1",0),AcceptanceHandler)
    thread=threading.Thread(target=server.serve_forever,daemon=True)
    thread.start()
    url=f"http://127.0.0.1:{server.server_port}/projects"
    checks={}
    try:
        with sync_playwright() as pw:
            browser=pw.chromium.launch(channel="msedge",headless=True)

            # Chinese desktop: core controls, project counts, no surprise file content
            page=browser.new_page(viewport={"width":1280,"height":800})
            prep(page,"zh")
            page.goto(url,wait_until="networkidle")
            expect(page.locator("h1")).to_have_text("项目管理")
            expect(page.locator(".pf-project-column-title h2")).to_have_text("我的项目")
            expect(page.locator("#project-job-count")).to_have_text("1")
            assert_no_horizontal_scroll(page,1280)
            page.screenshot(path=str(OUT/"acceptance-desktop-zh.png"),full_page=True)
            checks["desktop_zh"]="PASS"

            # Full bilingual switch must not leave visible Chinese in English view.
            page.locator("#project-language").select_option("en")
            expect(page.locator("h1")).to_have_text("Projects")
            expect(page.locator(".pf-project-column-title h2")).to_have_text("My projects")
            expect(page.get_by_role("button",name="Run once")).to_be_visible()
            expect(page.get_by_role("button",name="Enable Cron")).to_be_visible()
            expect(page.get_by_role("button",name="Run details")).to_be_visible()
            page.locator("#project-edit").click()
            expect(page.locator("#project-form-title")).to_have_text("Edit project")
            expect(page.locator("#project-form label").first).to_contain_text("Project name")
            page.locator("#project-cancel").click()
            page.screenshot(path=str(OUT/"acceptance-desktop-en.png"),full_page=True)
            checks["english_ui"]="PASS"

            # Dismissing confirmations must not trigger a task or change Cron.
            baseline=(len(mock.MANUAL_RUNS),len(mock.SCHEDULE_TOGGLES),len(mock.MANUAL_STOPS))
            page.once("dialog", lambda dialog: dialog.dismiss())
            page.get_by_role("button",name="Run once").click()
            page.once("dialog", lambda dialog: dialog.dismiss())
            page.get_by_role("button",name="Enable Cron").click()
            assert baseline==(len(mock.MANUAL_RUNS),len(mock.SCHEDULE_TOGGLES),len(mock.MANUAL_STOPS))
            checks["confirmation_cancel"]="PASS"

            # English run detail, summary, and on-demand logs.
            page.get_by_role("button",name="Run details").click()
            expect(page.locator("#project-run-summary")).to_contain_text("Output changes: created 1")
            expect(page.locator("#project-run-changes")).to_contain_text("Discovered")
            assert not mock.LOG_READS
            page.get_by_role("button",name="View this run’s log").click()
            expect(page.locator("#project-run-log")).to_contain_text("MOCK_LOG_ONLY")
            checks["history_and_log"]="PASS"
            page.evaluate("window.scrollTo(0,0)")
            page.screenshot(path=str(OUT/"acceptance-run-details.png"),full_page=True)

            # Compact form: mobile should not show 360px of empty list space.
            mode="normal"
            for width in (360,390,768):
                mobile=browser.new_page(viewport={"width":width,"height":844})
                prep(mobile,"zh")
                mobile.goto(url,wait_until="networkidle")
                expect(mobile.locator("#project-current-name")).to_have_text(mock.PROJECT_NAME)
                assert_no_horizontal_scroll(mobile,width)
                list_box=mobile.locator(".pf-project-list-column").bounding_box()
                assert list_box and list_box["height"] < 210, f"excess blank list height at {width}: {list_box}"
                if width==390:
                    mobile.screenshot(path=str(OUT/"acceptance-mobile-390.png"),full_page=True)
                mobile.close()
            checks["responsive_360_390_768"]="PASS"

            # A typical dozens-of-projects setup must scroll within its list
            # instead of growing the whole page or losing access to the last entry.
            mode="many"
            bulk=browser.new_page(viewport={"width":390,"height":844})
            prep(bulk,"zh")
            bulk.goto(url,wait_until="networkidle")
            expect(bulk.locator("#project-count")).to_have_text("41")
            sizes=bulk.locator("#project-list").evaluate("(el)=>({scroll:el.scrollHeight,view:el.clientHeight})")
            assert sizes["scroll"]>sizes["view"],f"41 projects must have an internal scroll: {sizes}"
            assert_no_horizontal_scroll(bulk,390)
            bulk.get_by_role("button",name="查看 Mock Project 40").click()
            expect(bulk.locator("#project-current-name")).to_have_text("Mock Project 40")
            expect(bulk.locator("#project-job-count")).to_have_text("0")
            checks["bulk_41_projects"]="PASS"
            bulk.screenshot(path=str(OUT/"acceptance-many-projects.png"),full_page=True)
            bulk.close()

            # Empty state: neither stale details nor broken controls remain.
            mode="empty"
            empty=browser.new_page(viewport={"width":390,"height":844})
            prep(empty,"zh")
            empty.goto(url,wait_until="networkidle")
            expect(empty.locator("#project-count")).to_have_text("0")
            expect(empty.locator("#project-empty")).to_be_visible()
            expect(empty.locator("#project-detail")).to_be_hidden()
            empty.screenshot(path=str(OUT/"acceptance-empty.png"),full_page=True)
            checks["empty_state"]="PASS"
            empty.close()

            # An HTTP 503 should be surfaced without pretending the state is empty.
            mode="error"
            error=browser.new_page(viewport={"width":1280,"height":800})
            prep(error,"zh")
            error.goto(url,wait_until="networkidle")
            expect(error.locator("#project-notice")).to_be_visible()
            expect(error.locator("#project-notice")).to_contain_text("MOCK_SERVER_UNAVAILABLE")
            checks["api_error_visible"]="PASS"
            error.close()

            # Slow A history must never render inside B's panel after switching.
            mode="switch"
            switching=browser.new_page(viewport={"width":1280,"height":800})
            prep(switching,"zh")
            switching.goto(url,wait_until="networkidle")
            expect(switching.locator("#project-current-name")).to_have_text(mock.PROJECT_NAME)
            switching.get_by_role("button",name="执行详情").click()
            switching.get_by_role("button",name="查看 Second Mock Project").click()
            expect(switching.locator("#project-current-name")).to_have_text("Second Mock Project")
            switching.wait_for_timeout(1600)
            expect(switching.locator("#project-runs")).to_be_hidden()
            assert "mock_job" not in (switching.locator("#project-runs-title").inner_text() if switching.locator("#project-runs").is_visible() else "")
            checks["stale_history_ignored"]="PASS"
            switching.close()
            browser.close()

        (OUT/"project-hub-acceptance.json").write_text(
            json.dumps({"tests":checks,"production_invoked":False,
                        "fake_data_only":True},indent=2,ensure_ascii=False),encoding="utf-8")
        print("PASS Project Hub acceptance",json.dumps(checks))
    finally:
        server.shutdown()
        server.server_close()

if __name__=="__main__":
    main()
