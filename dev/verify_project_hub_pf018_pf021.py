"""PF018–PF021 isolated Edge regressions (fake tasks/files; no real execution)."""
import json
import threading
from http.server import ThreadingHTTPServer
from urllib.parse import unquote,urlsplit
from playwright.sync_api import sync_playwright, expect

import verify_project_hub_pf001_pf004 as fixture
import verify_project_hub_browser as fake

class Handler(fixture.Handler):
    open_requests=[]
    def do_POST(self):
        path=unquote(urlsplit(self.path).path)
        if path==f"/api/projects/{fake.PROJECT_NAME}/tasks/mock_job/open-file":
            if not self.authenticate():return
            body=json.loads(self.rfile.read(int(self.headers.get("Content-Length",0))))
            Handler.open_requests.append(body)
            return self.api(200,{"ok":True,"requested":True})
        return super().do_POST()
    def do_GET(self):
        path=unquote(urlsplit(self.path).path)
        if path==f"/api/projects/{fake.PROJECT_NAME}/tasks/mock_job/files":
            if not self.authenticate():return
            return self.api(200,{"ok":True,"files":[
                {"path":"accounts.txt","resolved_path":r"C:\CI\MOCK_PROJECT\accounts.txt",
                 "source":"registered_input","exists":True,"size_bytes":150,"inferred":False},
                {"path":"data\\result.xlsx","resolved_path":r"c:\ci\mock_project\data\result.xlsx",
                 "source":"registered_output","exists":True,"size_bytes":350,"inferred":False},
                {"path":r"C:\MOCK\Doc\tasks.xlsx","resolved_path":r"C:\MOCK\Doc\tasks.xlsx",
                 "source":"code_candidate","exists":True,"size_bytes":111,"inferred":True}
            ],"partial":True})
        return super().do_GET()

def inspect_modal(page,height):
    page.locator("#project-new").click()
    expect(page.locator("#project-modal")).to_be_visible()
    modal=page.locator("#project-form")
    r=modal.bounding_box()
    assert r and r["y"]>=9 and r["y"]+r["height"]<=height-9,("modal clipped",height,r)
    assert page.locator("#project-form-title").is_visible()
    assert page.locator("#project-close").is_visible()
    z=page.evaluate("""() => ({
      overlay:Number(getComputedStyle(document.querySelector('#project-modal')).zIndex),
      nav:Number(getComputedStyle(document.querySelector('.pf-process-topnav')).zIndex)
    })""")
    assert z["overlay"]>z["nav"],("overlay below navbar",z)
    scroll=modal.evaluate("(e)=>({room:e.scrollHeight-e.clientHeight,top:e.scrollTop})")
    assert scroll["top"]==0,("modal did not begin at heading",scroll)
    if scroll["room"]>0:
        modal.evaluate("(e)=>{e.scrollTop=e.scrollHeight}")
        assert page.locator("#project-save").is_visible(),("footer hidden",height)
        modal.evaluate("(e)=>{e.scrollTop=0}")
    page.locator("#project-close").click()
    expect(page.locator("#project-modal")).to_be_hidden()
    return r,scroll

def run():
    fake.TASKS[0].update({
        "cmd":r"C:\Python311\python.exe",
        "args":[r"C:\CI\MOCK_PROJECT\mock_job.py"],
        "cwd":fake.PROJECT["root"],"status":"success","enabled":False
    })
    fixture.STATUS["running"]=False
    server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
    thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    failures=[]
    try:
        with sync_playwright() as p:
            browser=p.chromium.launch(channel="msedge",headless=True)
            for width,height in ((1920,1080),(1600,900),(1366,768),(1280,720),(1000,768),(1366,650)):
                page=browser.new_page(viewport={"width":width,"height":height})
                page.on("pageerror",lambda e:failures.append(str(e)))
                page.add_init_script('localStorage.setItem("perch_token",%s);localStorage.setItem("perch-lang","zh");'%json.dumps(fake.TOKEN))
                base=f"http://127.0.0.1:{server.server_port}"
                page.goto(base+"/projects",wait_until="networkidle")
                expect(page.locator(".pf-project-task-item")).to_have_count(1)
                expect(page.locator(".pf-project-task-title")).to_have_text("mock_job")
                expect(page.locator(".pf-project-task-filename")).to_have_text("mock_job.py")
                assert r"C:\CI\MOCK_PROJECT" not in page.locator(".pf-project-task-link").inner_text()
                box,scroll=inspect_modal(page,height)
                print("MODAL",width,height,box,scroll)
                page.locator(".pf-project-detail-link").click()
                expect(page.locator("#task-name")).to_have_text("mock_job")
                expect(page.locator("#task-breadcrumb-name")).to_have_text("mock_job")
                expect(page.locator("#task-path")).to_have_text("mock_job.py")
                assert r"C:\CI\MOCK_PROJECT" not in page.locator(".pf-taskdetail-title").inner_text()
                expect(page.locator(".pf-taskdetail-file-name")).to_have_count(3,timeout=6000)
                labels=page.locator(".pf-taskdetail-file-name").all_inner_texts()
                assert labels==["accounts.txt",r"data\result.xlsx",r"C:\MOCK\Doc\tasks.xlsx"],labels
                # A relative label must never replace the absolute path sent to the backend.
                page.locator(".pf-taskdetail-file").first.get_by_role("button",name="打开文件").click()
                page.wait_for_timeout(150)
                assert Handler.open_requests[-1]=={"path":r"C:\CI\MOCK_PROJECT\accounts.txt","confirmed":True}
                assert page.locator("#task-settings-subtitle").count()==0
                heights=page.evaluate("""() => {
                  const p=['.pf-taskdetail-history','.pf-taskdetail-files-pane','.pf-taskdetail-info-pane'];
                  const h=p.map(s=>document.querySelector(s).getBoundingClientRect().height);
                  return {h,ratios:h.map(x=>x/h.reduce((a,b)=>a+b,0)),
                    bottom:document.querySelector('.pf-taskdetail-info-pane').getBoundingClientRect().bottom,
                    pageOverflow:document.documentElement.scrollHeight-document.documentElement.clientHeight}
                }""")
                if width>980 and height>615:
                    assert heights["pageOverflow"]<=1,("viewport overflow",width,height,heights)
                    assert heights["bottom"]<height,("sidebar clipped",width,height,heights)
                    if height>=700:
                        ratio=heights["ratios"]
                        assert abs(ratio[0]-.32)<.065 and abs(ratio[1]-.44)<.065 and abs(ratio[2]-.24)<.065,(width,height,heights)
                print("PANEL",width,height,heights)
                page.close()
            assert not failures,failures
            browser.close()
    finally:
        server.shutdown()
    print("PF018-PF021 EDGE PASS")

if __name__=="__main__":
    run()

