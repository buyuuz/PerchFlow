"""Real Edge QA for the redesigned PerchFlow process-detail page.

Runs only against the local Mock preview at 127.0.0.1:9011; checks the
preview marker before sending any process controls. Never runs on production.
"""
from __future__ import annotations
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "dev" / "screenshots"
OUT.mkdir(exist_ok=True)
BASE = "http://127.0.0.1:9011"
TARGET = "/process-detail?name=MailHub%20(DEMO)"
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

def snapshot(page, width, height, expected_pid):
    page.set_viewport_size({"width": width, "height": height})
    page.goto(BASE + TARGET, wait_until="domcontentloaded")
    page.locator("#proc-pid").filter(has_text=str(expected_pid)).wait_for(timeout=6000)
    page.wait_for_function("() => document.querySelectorAll('#log-content .pf-log-line').length === 3",timeout=6000)
    dimensions = page.evaluate("""() => {
      const rect=selector=>{
        const r=document.querySelector(selector).getBoundingClientRect();
        return {x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom};
      };
      return {
        viewport:{w:innerWidth,h:innerHeight},
        document:{width:document.documentElement.scrollWidth,height:document.documentElement.scrollHeight},
        main:rect('.pf-detail-main'),
        header:rect('.pf-detail-header'),
        controls:rect('.pf-detail-header-actions'),
        lang:rect('.pf-lang-picker'),
        title:rect('#proc-name'),
        summary:rect('.pf-detail-overview'),
        stats:[...document.querySelectorAll('.pf-metric')].map(el=>{
          const r=el.getBoundingClientRect();
          return {x:r.x,y:r.y,w:r.width,h:r.height};
        }),
        logToolbar:rect('.pf-log-toolbar'),
        logTitle:rect('.pf-log-title-line'),
        logHeading:rect('.pf-log-heading'),
        logTools:rect('.pf-log-tools'),
        terminal:rect('.pf-terminal-window'),
        logArea:rect('#log-content'),
        footer:rect('.pf-terminal-footer'),
        cssFont:getComputedStyle(document.querySelector('#log-content')).fontSize,
      };
    }""")
    print("DETAIL_LAYOUT",width,dimensions,flush=True)
    assert dimensions["document"]["width"] <= width + 2, dimensions
    assert dimensions["summary"]["bottom"] < dimensions["logToolbar"]["y"] + 1, dimensions
    assert dimensions["terminal"]["y"] >= dimensions["logToolbar"]["y"], dimensions
    assert dimensions["controls"]["x"] >= -1 and dimensions["controls"]["right"] <= width + 2, dimensions
    assert dimensions["logArea"]["height"] >= 220, dimensions
    assert len(dimensions["stats"]) == 4
    assert page.locator(".pf-detail-action").count() == 3
    assert page.locator(".pf-lang-option").count() == 2
    assert page.locator("#lines-select option").count() == 3
    assert page.locator("#log-line-count").inner_text().strip().startswith("3") or "3" in page.locator("#log-line-count").inner_text()
    if width >= 900:
        assert dimensions["document"]["height"] <= height + 3, dimensions
        assert all(abs(x["y"]-dimensions["stats"][0]["y"])<1 for x in dimensions["stats"])
    if width <= 620:
        assert len(set(round(x["y"]) for x in dimensions["stats"])) == 2
        assert dimensions["document"]["height"] >= height
        assert dimensions["logTitle"]["height"] < 29, dimensions
        assert dimensions["logTools"]["y"] >= dimensions["logHeading"]["bottom"] - 1, dimensions
    page.screenshot(path=str(OUT / f"process-detail-{width}.png"),full_page=width < 620)
    return dimensions

def main():
    with sync_playwright() as engine:
        browser = engine.chromium.launch(executable_path=EDGE, headless=True)
        ctx = browser.new_context(viewport={"width":1440,"height":900}, permissions=["clipboard-read","clipboard-write"])
        page = ctx.new_page()
        errors = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        mark = page.request.get(BASE + "/_dev/status").json()
        assert mark["app"] == "perchflow-dev-preview" and mark["mode"] == "mock", mark
        current = page.request.get(BASE + "/api/processes/MailHub%20(DEMO)").json()
        if current["status"] != "running":
            assert page.request.post(BASE + "/api/processes/MailHub%20(DEMO)/start").ok
        expected = page.request.get(BASE + "/api/processes/MailHub%20(DEMO)").json()
        assert expected["pid"] is not None, expected
        for width,height in ((1440,900),(1280,800),(768,850),(390,844),(320,750)):
            snapshot(page,width,height,expected["pid"])

        page.set_viewport_size({"width":1440,"height":900})
        page.goto(BASE+TARGET)
        page.locator("#proc-pid").filter(has_text=str(expected["pid"])).wait_for()
        assert page.locator("#proc-status-badge").get_attribute("data-status")=="running"
        assert page.locator("#detail-start").is_disabled()
        assert page.locator("#detail-stop").is_enabled()
        assert page.locator("#proc-cmd").inner_text()=="python launcher.py"
        assert f'{round(expected["mem_bytes"] / 1048576)} MB' in page.locator("#proc-mem").inner_text()

        # Copy command.
        page.locator("#detail-copy").click()
        assert page.evaluate("navigator.clipboard.readText()")=="python launcher.py"
        assert page.locator("#detail-copy .material-symbols-outlined").inner_text()=="check"

        # Two-language dropdown and translated details.
        picker=page.locator(".pf-lang-trigger")
        picker.click()
        assert page.locator(".pf-lang-option").count()==2
        page.locator('[data-lang-value="en"]').click()
        assert page.locator('[data-i18n="detail.label"]').inner_text()=="Process details"
        assert page.locator('[data-i18n="log.output"]').inner_text()=="Log Output"
        page.locator("#proc-status").filter(has_text="Running").wait_for()
        picker.click()
        page.locator('[data-lang-value="zh"]').click()
        page.locator('#proc-status').filter(has_text='运行中').wait_for()

        # Selector refresh and clear only the DISPLAYED logs.
        page.locator("#lines-select").select_option("100")
        page.wait_for_function("() => document.querySelectorAll('#log-content .pf-log-line').length === 3")
        page.locator('.pf-log-clear').click()
        assert page.locator('#log-content .pf-log-line').count() == 0
        assert "0" in page.locator("#log-line-count").inner_text()
        assert page.locator("#log-content").get_attribute("data-empty")

        # Re-fetch on selector change, still readable.
        page.locator("#lines-select").select_option("500")
        page.wait_for_function("() => document.querySelectorAll('#log-content .pf-log-line').length === 3")

        # Live is a real accessible switch.
        toggle=page.locator("#live-toggle")
        assert toggle.get_attribute("aria-checked")=="true"
        toggle.click()
        assert toggle.get_attribute("aria-checked")=="false"
        assert page.locator("#log-view-mode").get_attribute("data-live")=="false"
        toggle.press("Space")
        assert toggle.get_attribute("aria-checked")=="true"

        # Process controls only mutate the Mock preview; status colors and button
        # enable/disable state must follow the real API response.
        page.locator("#detail-stop").click()
        page.locator("#proc-status-badge[data-status='stopped']").wait_for()
        assert page.locator("#detail-stop").is_disabled()
        assert page.locator("#detail-start").is_enabled()
        page.locator("#detail-start").click()
        page.locator("#proc-status-badge[data-status='running']").wait_for()
        assert page.locator("#detail-start").is_disabled()
        page.locator("#detail-restart").click()
        page.locator("#proc-status-badge[data-status='running']").wait_for()

        # Back navigation returns to the chosen long-row process list.
        page.locator(".pf-detail-back").click()
        page.wait_for_url("**/processes")
        page.locator("tr[data-process-name='MailHub (DEMO)']").wait_for()
        page.goto(BASE + "/process-detail?name=Video%20Publisher%20(DEMO)")
        page.locator("#proc-status-badge[data-status='crashed']").wait_for()
        assert page.locator("#detail-start").is_enabled()
        assert page.locator("#detail-stop").is_disabled()
        assert not errors,errors
        print("DETAIL_INTERACTIONS=PASS: copy/lang/log/clear/select/live/stop/start/restart/back",flush=True)
        browser.close()

if __name__ == "__main__":
    main()
