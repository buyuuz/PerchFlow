"""PF033–041 browser regressions: isolated fake server and no business scripts."""
import json,threading,sys
from http.server import ThreadingHTTPServer
from urllib.parse import unquote,urlsplit
from pathlib import Path
from playwright.sync_api import sync_playwright,expect
import verify_project_hub_browser as fake
from verify_project_hub_pf026_pf032 import Handler as Base

class Handler(Base):
    events=[]
    def do_PUT(self):
        path=unquote(urlsplit(self.path).path)
        if path=="/api/tasks/mock_job/enabled":
            if not self.authenticate():return
            data=json.loads(self.rfile.read(int(self.headers.get("Content-Length",0))))
            Handler.events.append(("enabled",data))
            fake.TASKS[0]["enabled"]=data["enabled"]
            return self.api(200,{"ok":True,"enabled":data["enabled"]})
        return super().do_PUT()
    def do_DELETE(self):
        path=unquote(urlsplit(self.path).path)
        if path=="/api/tasks/mock_job/schedule":
            if not self.authenticate():return
            Handler.events.append(("unschedule",None))
            fake.TASKS[0]["schedule"]=""
            fake.TASKS[0]["enabled"]=False
            return self.api(200,{"ok":True,"schedule":"","enabled":False})
        return super().do_DELETE()

def run():
    fake.PROJECT["tasks"][:]=["mock_job"]
    fake.TASKS[:]=[{
        "name":"mock_job","cmd":"python",
        "args":[r"C:\CI\MOCK_PROJECT\mock_job.py"],
        "cwd":fake.PROJECT["root"],"schedule":"0 0 9 * * *",
        "enabled":True,"status":"running","project":fake.PROJECT_NAME
    }]
    server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    try:
      with sync_playwright() as p:
        browser=p.chromium.launch(channel="msedge",headless=True)
        page=browser.new_page(viewport={"width":1366,"height":768})
        errors=[]
        page.on("pageerror",lambda e:errors.append(str(e)))
        page.on("dialog",lambda e:errors.append("Unexpected native dialog: "+e.message) or e.dismiss())
        root=Path(__file__).resolve().parents[1]
        page.goto("about:blank")
        page.add_script_tag(path=str(root/"frontend/assets/pf-dialog.js"))
        page.add_style_tag(path=str(root/"frontend/assets/pf-dialog.css"))
        page.evaluate("""() => {
          window.chosenTask='none';
          window.PerchDialog.chooseScript([
            {name:'Job A',project:'Fire',args:['C:\\\\Code\\\\a.py']},
            {name:'Job B',project:'Fire',args:['C:\\\\Code\\\\b.py']}
          ]).then(t=>window.chosenTask=t?.name||'none');
        }""")
        page.get_by_role("button",name="Job A").click()
        page.get_by_role("textbox").fill("Job B")
        assert not page.get_by_role("button",name="下一步：配置计划").is_enabled(), "hidden selection still active"
        page.get_by_role("button",name="Job B").click()
        page.get_by_role("button",name="下一步：配置计划").click()
        assert page.evaluate("window.chosenTask")=="Job B"
        page.evaluate("""() => {
          window._saveFinished=false;
          window._cronResult='pending';
          window.PerchDialog.cron({title:'Test',initial:'0 0 9 * * *',
            onSave:()=>new Promise(resolve=>{
              window._finishSave=()=>{window._saveFinished=true;resolve();};
            })
          }).then(v=>window._cronResult=v);
        }""")
        page.get_by_role("button",name="保存计划").click()
        page.keyboard.press("Escape")
        expect(page.locator(".pf-dialog-panel")).to_be_visible()
        assert page.evaluate("window._cronResult")=="pending"
        assert page.locator(".pf-dialog-close").is_disabled()
        page.evaluate("window._finishSave()")
        expect(page.locator(".pf-dialog-panel")).to_have_count(0)
        assert page.evaluate("window._cronResult") is True, "save should settle only after persistence"
        page.close()
        # One-shot scheduled task UI actions against mock API.
        page=browser.new_page(viewport={"width":1366,"height":768})
        page.on("pageerror",lambda e:errors.append(str(e)))
        page.add_init_script('localStorage.setItem("perch_token",%s);localStorage.setItem("perch-lang","zh");'%json.dumps(fake.TOKEN))
        base=f"http://127.0.0.1:{server.server_port}"
        page.goto(base+"/tasks",wait_until="networkidle")
        row=page.locator('tr[data-task-name="mock_job"]')
        expect(row).to_be_visible()
        row.locator('[data-task-action="toggle"]').click()
        page.wait_for_function("() => !!document.querySelector('.pf-task-status--disabled')")
        assert Handler.events[-1]==("enabled",{"enabled":False}),Handler.events
        # Delete means remove Cron for project-linked tasks, not task registration.
        row=page.locator('tr[data-task-name="mock_job"]')
        row.locator('[data-task-action="delete"]').click()
        expect(page.locator(".pf-dialog-panel")).to_be_visible()
        assert "保留项目脚本" in page.locator(".pf-dialog-panel").inner_text()
        page.locator(".pf-dialog-primary").click()
        expect(row).to_have_count(0)
        assert Handler.events[-1][0]=="unschedule",Handler.events
        assert fake.PROJECT["tasks"]==["mock_job"]
        for width,height in ((390,600),(390,500),(1366,650),(1280,720)):
            page.set_viewport_size({"width":width,"height":height})
            page.goto(base+"/tasks",wait_until="networkidle")
            page.locator('button[onclick="openTaskModal()"]').click()
            dims=page.locator("#task-modal>.glass-card").bounding_box()
            assert dims["y"]>=7 and dims["y"]+dims["height"]<=height-7,(width,height,dims)
            assert page.locator("#task-modal>.glass-card").evaluate(
                "(e)=>getComputedStyle(e).overflowY")=="auto"
            page.locator('button[onclick="closeTaskModal()"]').click()
        assert not errors,errors
        print("PF033-PF041 EDGE PASS",Handler.events)
        browser.close()
    finally:
        server.shutdown()

if __name__=="__main__":run()

