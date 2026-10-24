"""Isolated visual layout acceptance for the Process and Task action columns.
Run against a separate mock preview; NEVER target production.
"""
import os
from playwright.sync_api import sync_playwright

BASE=os.getenv("PERCHFLOW_PREVIEW_URL","http://127.0.0.1:9019")
with sync_playwright() as playwright:
    browser=playwright.chromium.launch(channel="msedge",headless=True)
    context=browser.new_context(viewport={"width":1600,"height":900})
    marker=context.request.get(BASE+"/_dev/status",timeout=5000).json()
    assert marker.get("app")=="perchflow-dev-preview" and marker.get("mode")=="mock",marker
    page=context.new_page()
    page.on("pageerror",lambda e: print("PAGE_ERROR",e))
    for width in (1600,1440,1000,390):
        page.set_viewport_size({"width":width,"height":900})
        for route,tbody in (("/processes","#proc-table-body"),("/tasks","#task-rows")):
            page.goto(BASE+route,wait_until="domcontentloaded",timeout=12000)
            page.locator(tbody+" tr").first.wait_for(timeout=12000)
            # Simulate the task's sixth, conditional Stop button.
            if route=="/tasks":
                page.evaluate("""() => {
                  const item={name:'Simulated Running Task',cmd:'python',args:[],schedule:'0 0 12 * * *',enabled:true,status:'running',last_run:null,last_exit_code:null};
                  document.querySelector('#task-rows').insertAdjacentHTML('beforeend',renderTaskRow(item));
                }""")
            info=page.evaluate("""(selector) => {
              const table=document.querySelector('.pf-process-table');
              const wrapper=document.querySelector('.pf-process-table-scroll');
              const rows=[...document.querySelectorAll(selector+' tr')].filter(r=>r.querySelector('.pf-actions'));
              const box=e=>{const b=e.getBoundingClientRect();return {left:b.left,right:b.right,width:b.width}};
              return {
                frame:box(document.querySelector('main')),
                header:box(table.querySelector('thead th:last-child')),
                pageWidth:document.documentElement.scrollWidth,
                viewportWidth:window.innerWidth,
                scrollWidth:wrapper.scrollWidth,clientWidth:wrapper.clientWidth,
                rows:rows.map(r=>{
                  const td=r.querySelector('td:last-child');
                  const group=td.querySelector('.pf-actions');
                  const buttons=[...group.querySelectorAll('button')];
                  return {name:r.dataset.processName||r.dataset.taskName,
                          cell:box(td),group:box(group),buttonCount:buttons.length,
                          actions:buttons.map(b=>({act:b.dataset.action||b.dataset.taskAction,disabled:b.disabled,rect:box(b)})),
                          justify:getComputedStyle(group).justifyContent,gap:getComputedStyle(group).gap};
                }),
                headerAlign:getComputedStyle(table.querySelector('thead th:last-child')).textAlign,
              }
            }""",tbody)
            assert info["pageWidth"]<=width+3,(width,route,"page overflow",info["pageWidth"])
            assert info["headerAlign"]=="right",(width,route,info["headerAlign"])
            if route=="/processes":
                assert abs(info["frame"]["width"]-min(1500,width))<=1,(width,route,"frame",info["frame"])
                for r in info["rows"]:
                    assert r["buttonCount"]==6,(width,r)
                    web=r["actions"][-1]
                    assert web["act"]=="web",(width,r)
                    if r["name"].startswith(("Daily_Check","Video Publisher")):
                        assert web["disabled"],(width,r)
            else:
                for r in info["rows"]:
                    assert r["buttonCount"] in (5,6),(width,r)
                assert any(r["buttonCount"]==6 for r in info["rows"]),(width,info)
            for row in info["rows"]:
                assert row["justify"]=="flex-end",row
                assert row["gap"]=="4px",row
                assert row["group"]["left"]>=row["cell"]["left"]-1,(width,route,"left overflow",row)
                for b in row["actions"]:
                    assert b["rect"]["right"]<=row["cell"]["right"]-14,(width,route,"last button clipped",row)
                    assert b["rect"]["width"]>=31.5,(width,route,"button squeezed",row)
            print(f'PASS {route} width={width} table-scroll={info["scrollWidth"]>info["clientWidth"]} rows={len(info["rows"])} last-col={info["rows"][0]["cell"]["width"]:.1f}px count={info["rows"][0]["buttonCount"]}')
    context.close()
    browser.close()
print("ACTION_LAYOUT_BROWSER_PASS")
