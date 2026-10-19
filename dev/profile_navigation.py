"""Measure full navigation costs between processes and tasks in local preview."""
from __future__ import annotations
import statistics
import time
from pathlib import Path
from playwright.sync_api import sync_playwright

BASE="http://127.0.0.1:9011"
EDGE=r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path=EDGE)
    page=browser.new_page(viewport={"width":1440,"height":900})
    reqs=[]
    def record(response):
        try:
            url=response.url.replace(BASE,"")
            if not url.startswith("/assets/"):
                return
            reqs.append({"path":url.split("?")[0],"status":response.status,
                         "cache":response.headers.get("cache-control"),
                         "length":int(response.headers.get("content-length","0") or 0)})
        except Exception:
            pass
    page.on("response",record)
    page.goto(BASE+"/processes",wait_until="load")
    print("FIRST_PAGE_COUNT",len(reqs),"BYTES",sum(x["length"] for x in reqs))
    for index,target in enumerate(["tasks","processes","tasks","processes","tasks","processes"]):
        reqs.clear()
        start=time.perf_counter()
        page.locator(f'nav:first-of-type a[href="/{target}"]').click()
        page.wait_for_url("**/"+target,wait_until="load")
        # page navigates fully; mock API check once ready
        selector="#proc-table-body tr[data-process-name]" if target=="processes" else "#task-rows > *"
        try: page.locator(selector).first.wait_for(timeout=1800)
        except: pass
        elapsed=(time.perf_counter()-start)*1000
        navigation=page.evaluate("""() => {
            const n=performance.getEntriesByType('navigation')[0];
            return n ? {domReady:n.domContentLoadedEventEnd,load:n.loadEventEnd,
                        responseEnd:n.responseEnd,transfer:n.transferSize} : {};
        }""")
        # response headers still advertise the file's full Content-Length
        # even on a cache hit; transferSize distinguishes cached font loads.
        actual_transfers=page.evaluate("""() => {
            const entries=performance.getEntriesByType('resource');
            const font=entries.filter(e=>e.name.endsWith('.woff2'));
            const icon=font.filter(e=>e.name.includes('material-symbols-outlined'));
            return {
                iconRequests:icon.length,
                iconTransferBytes:icon.reduce((n,e)=>n+e.transferSize,0),
                fontsTransferBytes:font.reduce((n,e)=>n+e.transferSize,0),
                resourceCount:entries.length
            };
        }""")
        summary={}
        for rr in reqs:
            name=rr["path"]
            if "material-symbols" in name:cat="icon-font"
            elif name.endswith(".woff2"):cat="other-font"
            elif name.endswith(".css"):cat="css"
            elif name.endswith(".js"):cat="js"
            else:cat="other"
            summary.setdefault(cat,{"requests":0,"bytes":0,"statuses":[]})
            summary[cat]["requests"]+=1
            summary[cat]["bytes"]+=rr["length"]
            summary[cat]["statuses"].append(rr["status"])
        print(f"STEP={index+1},target={target},wallMs={elapsed:.0f},nav={navigation},resourceTransfers={actual_transfers},assets={summary}",flush=True)
    browser.close()
