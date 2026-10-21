"""PF012-PF017 browser regressions using mock APIs; never launches real files."""
import json
import threading
from http.server import ThreadingHTTPServer
from urllib.parse import urlsplit,unquote
from playwright.sync_api import sync_playwright,expect
import verify_project_hub_pf001_pf004 as fixture
import verify_project_hub_browser as seed

class Handler(fixture.Handler):
    empty_history=False
    settings_updates=[]
    def do_GET(self):
        path=unquote(urlsplit(self.path).path)
        if path=="/api/tasks/mock_job/history" and Handler.empty_history:
            if not self.authenticate():return
            return self.api(200,{"ok":True,"runs":[]})
        if path.startswith("/api/tasks/mock_job/log/"):
            if not self.authenticate():return
            run=path.split("/")[-1]
            fixture.LOG_READS.append(path)
            return self.api(200,{"ok":True,"content":"[INFO] SCRIPT_RUN_"+run,"truncated":False})
        return super().do_GET()
    def do_PUT(self):
        path=unquote(urlsplit(self.path).path)
        if path==f"/api/projects/{seed.PROJECT_NAME}/tasks/mock_job/settings":
            if not self.authenticate():return
            req=json.loads(self.rfile.read(int(self.headers.get("Content-Length",0))))
            assert set(req)=={"cmd","script","cwd","args","timeout_secs"}
            Handler.settings_updates.append(req)
            seed.TASKS[0].update({"cmd":req["cmd"],"args":[req["script"]]+req["args"],
                "cwd":req["cwd"],"timeout_secs":req["timeout_secs"]})
            return self.api(200,{"ok":True})
        return self.api(404,{"ok":False,"error":"not found"})

def run():
    seed.TASKS[0].update({"cmd":r"C:\Python311\python.exe",
        "args":[r"C:\CI\MOCK_PROJECT\mock_job.py","--old"],
        "cwd":r"C:\CI\MOCK_PROJECT","timeout_secs":90})
    fixture.STATUS["running"]=False
    fixture.LOG_READS.clear();fixture.FILE_REQUESTS.clear()
    http=ThreadingHTTPServer(("127.0.0.1",0),Handler)
    thread=threading.Thread(target=http.serve_forever,daemon=True);thread.start()
    js_errors=[]
    try:
        with sync_playwright() as p:
            browser=p.chromium.launch(channel="msedge",headless=True)
            page=browser.new_page(viewport={"width":1366,"height":768})
            page.on("pageerror",lambda e:js_errors.append(str(e)))
            page.on("dialog",lambda d:js_errors.append("unexpected dialog: "+d.message) or d.dismiss())
            page.add_init_script('localStorage.setItem("perch_token",%s);localStorage.setItem("perch-lang","zh");'%json.dumps(seed.TOKEN))
            url=f"http://127.0.0.1:{http.server_port}"
            page.goto(url+"/projects",wait_until="networkidle")
            expect(page.locator(".pf-project-task-item")).to_have_count(1)
            assert page.locator(".pf-project-row-more").count()==0, "PF016"
            page.locator(".pf-project-detail-link").click()
            expect(page.locator("#task-name")).to_have_text("mock_job.py")
            # PF013 default latest and immediate historical log switching.
            expect(page.locator("#task-log-content")).to_contain_text("SCRIPT_RUN_"+fixture.RUN_ID)
            assert page.locator("#task-log-show").count()==0
            assert page.locator("#task-history .pf-taskdetail-history-item[aria-current=true]").count()==1
            page.locator("#task-history .pf-taskdetail-history-item").nth(1).click()
            expect(page.locator("#task-log-content")).to_contain_text("SCRIPT_RUN_1771700000000-11-3")
            assert page.locator("#task-history .pf-taskdetail-history-item").nth(1).get_attribute("aria-current")=="true"
            # PF014 + PF017
            for item in ("task-exit","task-cpu","task-memory"):
                assert page.locator("#"+item).count()==0
            assert page.locator(".pf-taskdetail-info-pane a[data-pf-en=Edit]").count()==0
            # PF012 file opener is direct, but backend still requires confirmed=true.
            expect(page.get_by_role("button",name="打开文件")).to_be_enabled()
            page.get_by_role("button",name="打开文件").click()
            page.wait_for_timeout(250)
            assert len(fixture.FILE_REQUESTS)==1, "file opener should submit exactly one request"
            assert page.locator("#task-message").is_hidden(), "successful file open should not display misleading warning"
            # PF015 modal editing retains Cron state through a dedicated endpoint.
            page.locator("#task-settings").click()
            expect(page.locator("#task-settings-dialog")).to_be_visible()
            assert "/tasks" not in page.url
            expect(page.locator("#task-settings-python")).to_have_value(r"C:\Python311\python.exe")
            expect(page.locator("#task-settings-args")).to_have_value("--old")
            page.locator("#task-settings-args").fill("--mode\nsafe mode")
            page.locator("#task-settings-timeout").fill("160")
            page.locator("#task-settings-save").click()
            expect(page.locator("#task-settings-dialog")).to_be_hidden()
            assert Handler.settings_updates[-1]["args"]==["--mode","safe mode"],Handler.settings_updates[-1]
            assert seed.TASKS[0]["schedule"]=="0 0 0 * * *"
            assert seed.TASKS[0]["enabled"]==False
            expect(page.locator("#task-args")).to_have_text("--mode safe mode")
            assert not js_errors,js_errors
            page.close()
            # No run history should yield an empty log without API log fetch.
            Handler.empty_history=True
            log_count=len(fixture.LOG_READS)
            fresh=browser.new_page()
            fresh.on("pageerror",lambda e:js_errors.append(str(e)))
            fresh.add_init_script('localStorage.setItem("perch_token",%s);localStorage.setItem("perch-lang","zh");'%json.dumps(seed.TOKEN))
            fresh.goto(url+"/task-detail?project="+seed.PROJECT_NAME.replace(" ","%20")+"&task=mock_job",wait_until="networkidle")
            expect(fresh.locator("#task-history")).to_contain_text("没有执行记录")
            assert len(fixture.LOG_READS)==log_count,(log_count,len(fixture.LOG_READS))
            assert not js_errors,js_errors
            fresh.close()
            browser.close()
    finally:
        http.shutdown()
    print("PF012-PF017 MOCKED EDGE PASS")
if __name__=="__main__":run()
