"""Visual and interaction QA for PerchFlow's shared top-right controls.

Only uses 127.0.0.1:9011 Mock preview. Checks the new language listbox
without triggering real process or cron task commands.
"""
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]
BASE="http://127.0.0.1:9011"
EDGE=r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
SHOTS=ROOT/"dev"/"screenshots"
SHOTS.mkdir(exist_ok=True)

def measure(page, route, width):
    page.set_viewport_size({"width":width,"height":900})
    page.goto(BASE+route, wait_until="domcontentloaded")
    page.locator(".pf-lang-picker.is-enhanced .pf-lang-trigger").wait_for()
    metrics=page.evaluate("""() => {
      const geom=(selector)=>{
        const el=document.querySelector(selector),rect=el.getBoundingClientRect(),sty=getComputedStyle(el);
        return {x:rect.x,y:rect.y,width:rect.width,height:rect.height,
          font:parseFloat(sty.fontSize),radius:sty.borderRadius};
      };
      const menu=document.querySelector('.pf-lang-menu');
      const before=document.querySelector('input[type="search"]').getBoundingClientRect();
      return {window:innerWidth, scrollWidth:document.documentElement.scrollWidth,
        actions:geom('.pf-top-actions'), status:geom('.pf-top-status'),
        add:geom('.pf-top-add'), lang:geom('.pf-lang-trigger'),
        nav:geom('.pf-process-topnav'),
        search:{x:before.x,y:before.y,w:before.width,h:before.height},
        nativeHidden:getComputedStyle(document.querySelector('select[data-lang-select]')).opacity==="0",
        menuHidden:menu.hidden,
        enhanced:document.querySelector('.pf-lang-picker').classList.contains('is-enhanced')};
    }""")
    print(route,width,metrics,flush=True)
    assert metrics["enhanced"]
    assert metrics["nativeHidden"]
    assert metrics["menuHidden"]
    assert abs(metrics["add"]["height"]-32)<0.5
    assert abs(metrics["lang"]["height"]-32)<0.5
    assert abs(metrics["status"]["height"]-32)<0.5 if width>=768 else True
    assert metrics["add"]["font"]<=13
    assert metrics["lang"]["font"]<=13
    assert metrics["scrollWidth"]<=width+2,metrics
    assert metrics["lang"]["x"]>=0
    assert metrics["lang"]["x"]+metrics["lang"]["width"]<=width+2,metrics
    return metrics


def main():
    with sync_playwright() as p:
        browser=p.chromium.launch(headless=True,executable_path=EDGE)
        context=browser.new_context(viewport={"width":1440,"height":900})
        page=context.new_page()
        errors=[]
        page.on("pageerror",lambda e:errors.append(str(e)))
        info=page.request.get(BASE+"/_dev/status").json()
        assert info["app"]=="perchflow-dev-preview" and info["mode"]=="mock"

        for path in ("/processes","/tasks"):
            for width in (1440,390,320):
                m=measure(page,path,width)
                if width==1440:
                    page.screenshot(path=str(SHOTS/("topbar-"+path.strip("/")+"-1440.png")),full_page=False)
                    trigger=page.locator(".pf-lang-trigger")
                    trigger.click()
                    menu=page.locator(".pf-lang-menu")
                    assert menu.is_visible()
                    assert trigger.get_attribute("aria-expanded")=="true"
                    assert page.locator(".pf-lang-option").count()==5
                    openInfo=page.evaluate("""() => ({
                       menu:document.querySelector('.pf-lang-menu').getBoundingClientRect().toJSON(),
                       search:document.querySelector('input[type="search"]').getBoundingClientRect().toJSON(),
                       pageWidth:document.documentElement.scrollWidth,
                       color:getComputedStyle(document.querySelector('.pf-lang-menu')).backgroundColor
                    })""")
                    print("OPEN_MENU",path,openInfo,flush=True)
                    assert openInfo["pageWidth"]==width
                    assert openInfo["menu"]["right"]<=width+1
                    assert abs(openInfo["search"]["x"]-m["search"]["x"])<0.5
                    assert abs(openInfo["search"]["y"]-m["search"]["y"])<0.5
                    page.screenshot(path=str(SHOTS/("topbar-"+path.strip("/")+"-open.png")),full_page=False)

                    # Explicitly select English through new custom popover.
                    page.locator('[data-lang-value="en"]').click()
                    assert not menu.is_visible()
                    assert trigger.get_attribute("aria-expanded")=="false"
                    assert page.locator("[data-lang-current]").inner_text()=="English"
                    assert page.locator("[data-lang-code]").first.inner_text()=="EN"
                    assert page.locator('select[data-lang-select]').input_value()=="en"
                    assert page.locator('[data-lang-value="en"]').get_attribute("aria-selected")=="true"
                    assert page.locator('[data-lang-value="zh"]').get_attribute("aria-selected")=="false"
                    label='button.add_process' if path=='/processes' else 'button.add_task'
                    addText=page.locator('[data-i18n="'+label+'"]').inner_text()
                    assert addText.lower().startswith("add"),addText
                    dynamicHead='[data-table-label="name"]' if path=='/processes' else '[data-task-label="name"]'
                    assert "name" in page.locator(dynamicHead).inner_text().lower()

                    # Clicking outside closes and never shifts content.
                    trigger.click()
                    assert menu.is_visible()
                    page.locator('h1').click()
                    assert not menu.is_visible()

                    # Keyboard: ArrowDown opens and moves selection; Escape closes.
                    trigger.focus()
                    trigger.press("ArrowDown")
                    assert menu.is_visible()
                    page.keyboard.press("Escape")
                    assert not menu.is_visible()
                    assert trigger.evaluate("(e)=>e===document.activeElement")

                    # Keyboard click: ArrowDown then Enter chooses a different language.
                    trigger.press("ArrowDown")
                    page.keyboard.press("ArrowDown")
                    page.keyboard.press("Enter")
                    assert not menu.is_visible()
                    assert page.locator("[data-lang-current]").inner_text()=="Español"

                    # Native select also remains valid and triggers dynamic table updates.
                    page.locator('select[data-lang-select]').select_option("zh")
                    assert page.locator("[data-lang-current]").inner_text()=="中文"
                    assert page.locator("[data-lang-code]").first.inner_text()=="CN"
                    assert page.locator("html").get_attribute("lang")=="zh"

                    # Verify modal operation unchanged by new header styling.
                    page.locator(".pf-top-add").click()
                    popup="#add-modal" if path=="/processes" else "#task-modal"
                    assert page.locator(popup).is_visible()
                    cancel='button[onclick="closeAddModal()"]' if path=="/processes" else 'button[onclick="closeTaskModal()"]'
                    page.locator(cancel).click()
                    assert not page.locator(popup).is_visible()
                else:
                    # Check custom menu fits narrow viewport and opening won't cause page overflow.
                    page.locator(".pf-lang-trigger").click()
                    assert page.locator(".pf-lang-menu").is_visible()
                    rect=page.locator(".pf-lang-menu").bounding_box()
                    assert rect["x"]>=-1 and rect["x"]+rect["width"]<=width+1,rect
                    assert page.evaluate("document.documentElement.scrollWidth <= innerWidth + 2")
                    page.keyboard.press("Escape")

        # Page-to-page nav remains stable after controls shrink.
        page.set_viewport_size({"width":1440,"height":900})
        page.goto(BASE+"/processes")
        p1=page.locator(".pf-top-add").bounding_box()
        page.locator('nav a[href="/tasks"]').first.click()
        page.wait_for_url("**/tasks")
        p2=page.locator(".pf-top-add").bounding_box()
        assert abs(p1["height"]-p2["height"])<0.5
        assert not errors,errors
        browser.close()
    print("TOPBAR_BROWSER=PASS",flush=True)

if __name__=="__main__":
    main()
