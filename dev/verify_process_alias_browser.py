"""End-to-end browser verification for bilingual process display and editing.

Runs ONLY against port 9011 when it reports mode=mock. Never sends process
commands to the real PerchFlow server on port 9002.
"""
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]
EDGE=r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
BASE="http://127.0.0.1:9011"
OUT=ROOT/"dev"/"screenshots"
OUT.mkdir(exist_ok=True)


def main():
    with sync_playwright() as p:
        browser=p.chromium.launch(headless=True,executable_path=EDGE)
        context=browser.new_context(
            viewport={"width":1440,"height":900},
            permissions=["clipboard-read","clipboard-write"],
        )
        page=context.new_page()
        errors=[]
        page.on("pageerror",lambda err:errors.append(str(err)))
        status=page.request.get(BASE+"/_dev/status").json()
        assert status["app"]=="perchflow-dev-preview" and status["mode"]=="mock",status
        page.goto(BASE+"/processes",wait_until="domcontentloaded")
        mail=page.locator('tr[data-process-name="MailHub (DEMO)"]')
        mail.wait_for()
        assert page.locator("tr[data-process-name]").count()==4
        assert mail.locator(".pf-process-name").inner_text()=="MailHub (DEMO)"
        assert mail.locator(".pf-process-alias").inner_text()=="邮箱管理中心"
        sizes=mail.evaluate("""node=>({
          en:parseFloat(getComputedStyle(node.querySelector('.pf-process-name')).fontSize),
          zh:parseFloat(getComputedStyle(node.querySelector('.pf-process-alias')).fontSize),
          height:node.getBoundingClientRect().height,
          documentWidth:document.documentElement.scrollWidth
        })""")
        print("DESKTOP",sizes,flush=True)
        assert sizes["en"]>sizes["zh"] and abs(sizes["height"]-73)<1,sizes
        page.screenshot(path=str(OUT/"process-alias-list-1440.png"),full_page=True)

        # Switching between 中文 and English changes UI labels, NOT name order.
        page.locator(".pf-lang-trigger").click()
        page.locator('[data-lang-value="en"]').click()
        assert mail.locator(".pf-process-name").inner_text()=="MailHub (DEMO)"
        assert mail.locator(".pf-process-alias").inner_text()=="邮箱管理中心"
        page.locator('tr[data-process-name="Daily_Check (DEMO)"] .pf-process-alias').wait_for()
        page.locator("#proc-search").fill("每日巡检")
        assert page.locator("tr[data-process-name]").count()==1
        assert page.locator("tr[data-process-name]").first.get_attribute("data-process-name")=="Daily_Check (DEMO)"
        page.locator("#proc-search").fill("")

        # Edit running process ONLY in its alias: no restart or PID loss.
        before=page.request.get(BASE+"/api/processes/MailHub%20(DEMO)").json()
        assert before["status"]=="running" and before["pid"]==12340
        sent=[]
        page.on("request", lambda request:sent.append((request.method,request.url))
                if request.method in ("PUT","POST") else None)
        mail.locator('[data-action="edit"]').click()
        assert page.locator("#add-name").input_value()=="MailHub (DEMO)"
        assert page.locator("#add-name").is_disabled()
        assert page.locator("#add-alias-zh").input_value()=="邮箱管理中心"
        page.locator("#add-alias-zh").fill("邮件运营中心")
        page.locator('button[onclick="submitAddModal()"]').click()
        page.wait_for_function("""() => {
          let row=document.querySelector('tr[data-process-name="MailHub (DEMO)"]');
          return row && row.querySelector('.pf-process-alias')?.textContent==='邮件运营中心';
        }""")
        after=page.request.get(BASE+"/api/processes/MailHub%20(DEMO)").json()
        assert (after["status"],after["pid"])==("running",12340)
        assert after["alias_zh"]=="邮件运营中心"
        assert any(method=="PUT" and url.endswith("/api/processes/MailHub%20(DEMO)/alias")
                   for method,url in sent),sent
        print("ALIAS_ONLY_EDIT_NO_RESTART=PASS",flush=True)

        # On detail page, English remains main heading and alias stays smaller.
        mail.locator(".pf-process-name").click()
        page.wait_for_url("**/process-detail?name=MailHub%20(DEMO)")
        page.locator("#proc-alias-zh").filter(has_text="邮件运营中心").wait_for()
        assert page.locator("#proc-name").inner_text()=="MailHub (DEMO)"
        details=page.evaluate("""() => ({
          main:parseFloat(getComputedStyle(document.querySelector('#proc-name')).fontSize),
          sub:parseFloat(getComputedStyle(document.querySelector('#proc-alias-zh')).fontSize),
          aliasHidden:document.querySelector('#proc-alias-zh').hidden
        })""")
        assert details["main"]>details["sub"] and not details["aliasHidden"],details
        page.locator(".pf-detail-back").click()

        # Clearing optional alias hides only the subtitle.
        mail=page.locator('tr[data-process-name="MailHub (DEMO)"]')
        mail.locator('[data-action="edit"]').click()
        page.locator("#add-alias-zh").fill("")
        page.locator('button[onclick="submitAddModal()"]').click()
        page.wait_for_function("""() => {
          const row=document.querySelector('tr[data-process-name="MailHub (DEMO)"]');
          return row && !row.querySelector('.pf-process-alias');
        }""")
        restored=page.request.get(BASE+"/api/processes/MailHub%20(DEMO)").json()
        assert restored["alias_zh"] is None and restored["pid"]==12340

        # Fresh create, persist and later edit both names independently.
        page.locator('button[onclick="openAddModal()"]').click()
        page.locator("#add-name").fill("Notes Worker QA")
        page.locator("#add-alias-zh").fill("笔记整理")
        page.locator("#add-cmd").fill("python")
        page.locator("#add-args").fill("notes.py")
        page.locator('button[onclick="submitAddModal()"]').click()
        new=page.locator('tr[data-process-name="Notes Worker QA"]')
        new.wait_for()
        assert new.locator(".pf-process-alias").inner_text()=="笔记整理"
        fresh=page.request.get(BASE+"/api/processes/Notes%20Worker%20QA").json()
        assert fresh["name"]=="Notes Worker QA" and fresh["alias_zh"]=="笔记整理"
        assert fresh["status"]=="stopped"

        # Full PUT changes both command and Chinese subtitle.
        new.locator('[data-action="edit"]').click()
        assert page.locator("#add-alias-zh").input_value()=="笔记整理"
        page.locator("#add-alias-zh").fill("内容归档")
        page.locator("#add-args").fill("archive.py")
        page.locator('button[onclick="submitAddModal()"]').click()
        page.wait_for_function("""() => {
          const row=document.querySelector('tr[data-process-name="Notes Worker QA"]');
          return row && row.querySelector('.pf-process-alias')?.textContent==='内容归档';
        }""")
        newState=page.request.get(BASE+"/api/processes/Notes%20Worker%20QA").json()
        assert newState["alias_zh"]=="内容归档" and newState["args"]==["archive.py"]
        new=page.locator('tr[data-process-name="Notes Worker QA"]')
        new.locator(".pf-process-name").click()
        page.locator("#proc-alias-zh").filter(has_text="内容归档").wait_for()
        page.locator(".pf-detail-back").click()

        # Legacy process without alias retains single-line title.
        older=page.locator('tr[data-process-name="Data Collector (DEMO)"]')
        assert older.count()==1 and older.locator(".pf-process-alias").count()==0
        older.locator(".pf-process-name").click()
        assert page.locator("#proc-alias-zh").is_hidden()
        page.locator(".pf-detail-back").click()

        # Mobile layout and edit modal scrolling.
        page.set_viewport_size({"width":390,"height":700})
        page.goto(BASE+"/processes")
        page.locator('tr[data-process-name="Notes Worker QA"]').wait_for()
        mobile=page.evaluate("""() => ({
          w:document.documentElement.scrollWidth,
          h:document.querySelector('tr[data-process-name="Notes Worker QA"]').getBoundingClientRect().height
        })""")
        assert mobile["w"]<=392 and abs(mobile["h"]-68)<1,mobile
        page.locator('button[onclick="openAddModal()"]').click()
        assert page.locator("#add-alias-zh").is_visible()
        modal=page.locator("#add-modal .glass-card").bounding_box()
        assert modal["height"]<=700,modal
        page.screenshot(path=str(OUT/"process-alias-modal-390.png"),full_page=False)
        page.locator('button[onclick="closeAddModal()"]').click()

        # Delete QA (only Mock).
        page.set_viewport_size({"width":1440,"height":900})
        page.once("dialog",lambda d:d.accept())
        page.locator('tr[data-process-name="Notes Worker QA"] [data-action="delete"]').click()
        page.wait_for_function("() => !document.querySelector('tr[data-process-name=\"Notes Worker QA\"]')")
        assert not errors,errors
        print("PROCESS_ALIAS_BROWSER=PASS: dual label, optional, filter, CRUD, no restart, details, lang, mobile",flush=True)
        browser.close()

if __name__=="__main__":
    main()
