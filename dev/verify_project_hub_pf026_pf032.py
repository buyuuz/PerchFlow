"""PF026–PF032: isolated Edge tests; mock HTTP server, no real scripts or files deleted."""
import json, threading
from http.server import ThreadingHTTPServer
from urllib.parse import unquote,urlsplit
from playwright.sync_api import sync_playwright,expect
import verify_project_hub_pf001_pf004 as fixture
import verify_project_hub_browser as fake

class Handler(fixture.Handler):
    calls=[]
    def do_GET(self):
        path=unquote(urlsplit(self.path).path)
        if path=="/tasks":
            self.path="/tasks.html";return super().do_GET()
        if path=="/api/tasks":
            if not self.authenticate():return
            query=urlsplit(self.path).query
            tasks=[dict(t) for t in fake.TASKS if (not query or
                ('scope=scheduled' in query and t.get('schedule')) or
                ('scope=unscheduled' in query and not t.get('schedule')))]
            return self.api(200,{"ok":True,"tasks":tasks})
        return super().do_GET()
    def do_POST(self):
        path=unquote(urlsplit(self.path).path)
        if path=="/api/tasks/mock_job/run":
            if not self.authenticate():return
            Handler.calls.append(("run",None))
            return self.api(200,{"ok":True,"run_id":"mock-run"})
        return super().do_POST()
    def do_PUT(self):
        path=unquote(urlsplit(self.path).path)
        if path=="/api/tasks/mock_job/schedule":
            if not self.authenticate():return
            body=json.loads(self.rfile.read(int(self.headers.get("Content-Length",0))))
            Handler.calls.append(("schedule",body))
            fake.TASKS[0]["schedule"]=body["schedule"]
            fake.TASKS[0]["enabled"]=body.get("enabled",True)
            return self.api(200,{"ok":True,"enabled":fake.TASKS[0]["enabled"]})
        return super().do_PUT()
    def do_DELETE(self):
        path=unquote(urlsplit(self.path).path)
        if path==f"/api/projects/{fake.PROJECT_NAME}/tasks/mock_job":
            if not self.authenticate():return
            Handler.calls.append(("delete",None))
            fake.PROJECT["tasks"].clear()
            fake.TASKS.clear()
            return self.api(200,{"ok":True,"source_files_deleted":False})
        return self.api(404,{"ok":False,"error":"unsupported"})

def run():
    fake.PROJECT["tasks"][:]=["mock_job"]
    fake.TASKS[:]=[{
        "name":"mock_job","cmd":"python",
        "args":[r"C:\CI\MOCK_PROJECT\mock_job.py"],
        "cwd":fake.PROJECT["root"],"schedule":"",
        "enabled":False,"status":"idle","project":fake.PROJECT_NAME
    }]
    Handler.calls.clear();fixture.STATUS["running"]=False
    server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    errors=[];native=[]
    try:
        with sync_playwright() as p:
            browser=p.chromium.launch(channel="msedge",headless=True)
            page=browser.new_page(viewport={"width":1366,"height":768})
            page.on("pageerror",lambda e:errors.append(str(e)))
            page.on("dialog",lambda d:native.append(d.message) or d.dismiss())
            page.add_init_script('localStorage.setItem("perch_token",%s);localStorage.setItem("perch-lang","zh");'%json.dumps(fake.TOKEN))
            base=f"http://127.0.0.1:{server.server_port}"
            page.goto(base+"/projects",wait_until="networkidle")
            expect(page.locator(".pf-project-delete-script")).to_be_visible()
            page.locator(".pf-project-delete-script").click()
            expect(page.locator(".pf-dialog-panel")).to_be_visible()
            assert ".py" in page.locator(".pf-dialog-panel").inner_text()
            page.locator(".pf-dialog-footer button").first.click()
            assert not any(x[0]=="delete" for x in Handler.calls)
            page.locator('[data-task-action="run"]').first.click()
            page.wait_for_function("() => document.querySelector('.pf-project-task-item') !== null")
            assert sum(x[0]=="run" for x in Handler.calls)==1,Handler.calls
            page.goto(base+"/task-detail?project=Mock%20Project&task=mock_job",wait_until="networkidle")
            expect(page.locator("#task-name")).to_have_text("mock_job")
            page.locator("#task-schedule-add").click()
            expect(page.locator(".pf-dialog-panel")).to_be_visible()
            page.locator(".pf-dialog-input").fill("bad")
            page.get_by_role("button",name="保存计划").click()
            expect(page.locator(".pf-dialog-error")).to_be_visible()
            page.locator(".pf-dialog-input").fill("0 20 9 * * *")
            page.get_by_role("button",name="保存计划").click()
            expect(page.locator(".pf-dialog-panel")).to_have_count(0)
            assert Handler.calls[-1]==("schedule",{"schedule":"0 20 9 * * *","enabled":True}),Handler.calls[-1]
            fake.TASKS[0]["enabled"]=False
            page.reload(wait_until="networkidle")
            page.locator("#task-schedule-add").click()
            page.locator(".pf-dialog-input").fill("0 30 9 * * *")
            page.get_by_role("button",name="保存计划").click()
            expect(page.locator(".pf-dialog-panel")).to_have_count(0)
            assert Handler.calls[-1]==("schedule",{"schedule":"0 30 9 * * *","enabled":False}),Handler.calls[-1]
            # PF030: search/select the script then configure schedule.
            fake.TASKS[0].update({"schedule":"","enabled":False})
            page.goto(base+"/tasks",wait_until="networkidle")
            page.locator("#task-bind-existing").click()
            expect(page.locator(".pf-dialog-choice")).to_have_count(1)
            page.locator(".pf-dialog-input").fill("no-match")
            expect(page.locator(".pf-dialog-choice")).to_have_count(0)
            page.locator(".pf-dialog-input").fill("mock_job.py")
            page.locator(".pf-dialog-choice").click()
            page.get_by_role("button",name="下一步：配置计划").click()
            expect(page.locator(".pf-dialog-panel")).to_be_visible()
            page.locator(".pf-dialog-input").fill("0 0 10 * * *")
            page.get_by_role("button",name="保存计划").click()
            expect(page.locator(".pf-dialog-panel")).to_have_count(0)
            assert Handler.calls[-1]==("schedule",{"schedule":"0 0 10 * * *","enabled":True}),Handler.calls[-1]
            expect(page.locator('tr[data-task-name="mock_job"]')).to_have_count(1)
            page.locator('[data-task-action="history"]').click()
            expect(page.locator("#task-history-modal")).to_be_visible()
            expect(page.locator("#task-run-log")).to_contain_text("FAKE_START")
            rect=page.locator("#task-history-modal>.glass-card").bounding_box()
            assert rect and rect["y"]>=0 and rect["y"]+rect["height"]<=768,rect
            assert "2026" in page.locator(".pf-task-history-record").first.inner_text()
            layout=page.evaluate("""()=>{
                const parent=document.querySelector('#task-history-modal>.glass-card'),
                  list=document.querySelector('#task-history-list'),
                  log=document.querySelector('#task-run-log');
                log.textContent=Array(200).fill('[INFO] sample').join('\\n');
                return {overflow:parent.scrollHeight-parent.clientHeight,
                  logScroll:log.scrollHeight-log.clientHeight,logHeight:log.clientHeight,
                  listHeight:list.clientHeight};
            }""")
            assert layout["overflow"]<=2 and layout["logScroll"]>0 and layout["logHeight"]>=100,layout
            page.get_by_role("button",name="关闭历史记录").click()
            expect(page.locator("#task-history-modal")).to_be_hidden()
            # Scope-verified delete from Project page removes registration without file deletion.
            page.goto(base+"/projects",wait_until="networkidle")
            page.locator(".pf-project-delete-script").click()
            page.locator(".pf-dialog-primary").click()
            expect(page.locator(".pf-project-task-item")).to_have_count(0)
            assert Handler.calls[-1][0]=="delete",Handler.calls
            assert len(fake.FILES)==3, "mock file references must be preserved"
            assert not native, native
            assert not errors, errors
            print("PF026-PF032 EDGE MOCK PASS",Handler.calls,layout)
            browser.close()
    finally:
        server.shutdown()

if __name__=="__main__":run()
