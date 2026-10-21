"""PF-005..PF-009: isolated Edge layout and interaction regression.

Starts only an in-process mock HTTP server, never launches real scripts or
opens real documents. Windows runner / C2 may run it without privileged ops.
"""
import json
import sys
import threading
from http.server import ThreadingHTTPServer
from pathlib import Path
from playwright.sync_api import sync_playwright, expect
import verify_project_hub_pf001_pf004 as mock_detail
import verify_project_hub_browser as mock
BASE_DIR=Path(__file__).resolve().parents[1]
DEST=BASE_DIR / "dev" / "ui-smoke-shots"
def read_box(page, selector):
    return page.locator(selector).first.bounding_box()
def inspect_view(page):
    return page.evaluate("""() => ({
      width:innerWidth,height:innerHeight,
      pageScroll:document.documentElement.scrollHeight-document.documentElement.clientHeight,
      bodyScroll:document.body.scrollHeight-document.body.clientHeight,
      headerFont:getComputedStyle(document.querySelector('.pf-main-nav-link')).fontFamily,
      pageFont:getComputedStyle(document.querySelector('main')).fontFamily,
      titleFont:getComputedStyle(document.querySelector('#task-name')).fontSize,
      cols:document.querySelector('.pf-taskdetail-columns')?.getBoundingClientRect().height,
      sidebarBottom:document.querySelector('.pf-taskdetail-sidebar')?.getBoundingClientRect().bottom,
      paneBottom:document.querySelector('.pf-taskdetail-info-pane')?.getBoundingClientRect().bottom,
      logScroll:document.querySelector('#task-log-content')?.scrollHeight-
        document.querySelector('#task-log-content')?.clientHeight
    })""")
def main():
    DEST.mkdir(parents=True,exist_ok=True)
    class Handler(mock_detail.Handler):
        def do_GET(self):
            if self.path == "/tasks":
                self.path = "/tasks.html"
            return super().do_GET()
    mock_detail.STATUS['running']=False
    server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
    thread=threading.Thread(target=server.serve_forever,daemon=True)
    thread.start()
    failures=[]
    try:
      with sync_playwright() as playwright:
        browser=playwright.chromium.launch(channel="msedge",headless=True)
        for width,height in ((1600,900),(1366,768),(1280,720),(1000,768)):
          page=browser.new_page(viewport={"width":width,"height":height})
          page.on("pageerror",lambda err:failures.append(str(err)))
          page.add_init_script('localStorage.setItem("perch_token",%s);localStorage.setItem("perch-lang","zh");' % json.dumps(mock.TOKEN))
          base=f"http://127.0.0.1:{server.server_port}"
          page.goto(base+"/tasks",wait_until="networkidle")
          nav_font=page.locator(".pf-main-nav-link").first.evaluate("(el)=>getComputedStyle(el).fontFamily")
          page.goto(base+"/projects",wait_until="networkidle")
          expect(page.locator("#project-current-name")).to_have_text(mock.PROJECT_NAME)
          left=read_box(page,".pf-project-list-column")
          right=read_box(page,".pf-project-detail-column")
          project_font=page.locator(".pf-project-main").evaluate("(el)=>getComputedStyle(el).fontFamily")
          assert nav_font==project_font,("font mismatch",nav_font,project_font)
          if width>850:
            assert left["x"]+left["width"] < right["x"],("not side-by-side",width,left,right)
            assert abs(left["y"]-right["y"])<=1,(left,right)
          if width==1600:
            page.screenshot(path=str(DEST/"project-overview-1600.png"),full_page=True)
          # Large project counts stay inside the left navigator, not a wide top strip.
          if width>850:
            long_list=page.evaluate("""() => {
              const holder=document.querySelector('#project-list');
              const row=holder.querySelector('.pf-project-entry');
              for(let i=0;i<40;i++)holder.append(row.cloneNode(true));
              return {scroll:holder.scrollHeight-holder.clientHeight,
                layout:getComputedStyle(document.querySelector('.pf-project-layout')).gridTemplateColumns};
            }""")
            assert long_list["scroll"]>100,("project menu does not scroll",long_list)
          page.locator(".pf-project-detail-link").first.click()
          expect(page.locator("#task-name")).to_have_text("mock_job")
          page.wait_for_timeout(550)
          details=inspect_view(page)
          if width>980 and height>615:
            assert details["pageScroll"]<=1,("page overflows",details)
            assert details["bodyScroll"]<=1,("body overflows",details)
            assert details["paneBottom"] <=height-2,("script info clipped",details)
          assert page.locator(".pf-taskdetail-sidebar .pf-taskdetail-pane").count()==3
          print("LAYOUT",width,height,details,"projects",left["x"],right["x"])
          if width==1600 and height==900:
            page.screenshot(path=str(DEST/"project-detail-1600.png"),full_page=True)
          if width==1366 and height==768:
            page.screenshot(path=str(DEST/"project-detail-1366.png"),full_page=True)
          # Only explicit user interaction may read an output log.
          previously_read=len(mock_detail.LOG_READS)
          page.locator("#task-log-show").click()
          expect(page.locator("#task-log-content")).to_contain_text("FAKE_START",timeout=5000)
          assert len(mock_detail.LOG_READS)>previously_read
          page.locator("#task-log-clear").click()
          expect(page.locator("#task-log-count")).to_have_text("0 行")
          page.locator("#task-more").click()
          assert page.locator("#task-refresh").is_visible()
          page.locator("#task-more").click()
          if width>980 and height>615:
            scroll=page.evaluate("""() => {
              const pre=document.querySelector('#task-log-content');
              pre.textContent=Array(250).fill('SIMULATED_LOG_LINE').join(String.fromCharCode(10));
              for(const [sel,item] of [['#task-history','.pf-taskdetail-history-item'],
                  ['#task-linked-files','.pf-taskdetail-file']]) {
                const list=document.querySelector(sel);
                const el=list.querySelector(item);
                if(el)for(let i=0;i<30;i++)list.append(el.cloneNode(true));
              }
              const log=document.querySelector('#task-log-content');
              const hist=document.querySelector('#task-history');
              const files=document.querySelector('#task-linked-files');
              return {
                outer:document.documentElement.scrollHeight-document.documentElement.clientHeight,
                log:log.scrollHeight-log.clientHeight,
                history:hist.scrollHeight-hist.clientHeight,
                files:files.scrollHeight-files.clientHeight,
                sidebarBottom:document.querySelector('.pf-taskdetail-info-pane').getBoundingClientRect().bottom
              };
            }""")
            assert scroll["outer"]<=1,("long data causes page scroll",scroll)
            assert scroll["log"]>0 and scroll["history"]>0 and scroll["files"]>0,("panes do not scroll independently",scroll)
            assert scroll["sidebarBottom"]<height,scroll
            print("INNER-SCROLL",width,height,scroll)
          page.close()
        if failures:raise RuntimeError(failures)
        browser.close()
    finally:
      server.shutdown()
    print("PF005-PF009 EDGE UI PASS")
if __name__=="__main__":main()
