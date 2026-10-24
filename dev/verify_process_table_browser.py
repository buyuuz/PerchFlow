"""Interactive Edge verification for the PerchFlow process table, preview only.

Optional local developer check (requires Playwright and Edge). Never points at
production: the preview marker is verified before every mutation.
Run: python dev/verify_process_table_browser.py
"""
from __future__ import annotations

from pathlib import Path
import os
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "dev" / "screenshots"
OUTPUT.mkdir(exist_ok=True)
BASE = os.getenv("PERCHFLOW_PREVIEW_URL", "http://127.0.0.1:9011")
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"


def inspect_table(page, width: int, min_rows: int = 4):
    page.set_viewport_size({"width": width, "height": 900})
    page.goto(BASE + "/processes", wait_until="domcontentloaded")
    page.locator("#proc-table-body tr").first.wait_for()
    page.wait_for_function("(count) => document.querySelectorAll('#proc-table-body tr[data-process-name]').length >= count", arg=min_rows)
    row_count = page.locator("#proc-table-body tr[data-process-name]").count()
    size = page.evaluate("""() => {
      const box = r => {const x=r.getBoundingClientRect();return {x:x.x,y:x.y,w:x.width,h:x.height,right:x.right};};
      const table=document.querySelector('.pf-process-table');
      const wrapper=document.querySelector('.pf-process-table-scroll');
      const nav=document.querySelector('nav');
      const first=document.querySelector('#proc-table-body tr');
      return {window:innerWidth, document:document.documentElement.scrollWidth,
              header:box(document.querySelector('.pf-process-table thead')),
              table:box(table), viewport:box(wrapper), first:box(first),
              firstCols:[...first.children].map(box),
              actualRows:[...document.querySelectorAll('#proc-table-body tr')].map(box),
              icon:box(document.querySelector('.pf-brand-icon')), nav:box(nav),
              titlePx:parseFloat(getComputedStyle(document.querySelector('.pf-process-page-title')).fontSize),
              subtitlePx:parseFloat(getComputedStyle(document.querySelector('.pf-process-page-subtitle')).fontSize),
              horizontalScroll:wrapper.scrollWidth-wrapper.clientWidth>3, scrollWidth:wrapper.scrollWidth, clientWidth:wrapper.clientWidth, overflowCandidates:[...document.querySelectorAll('.pf-process-table *')].filter(e=>e.getBoundingClientRect().right > wrapper.getBoundingClientRect().right+3).slice(0,12).map(e=>({tag:e.tagName, cls:e.className?.baseVal || e.className, text:e.textContent?.slice(0,25),right:e.getBoundingClientRect().right}))};
    }""")
    assert row_count >= min_rows, row_count
    assert page.locator("th").count() == 10, "Expected 10 aligned table columns"
    expected_row_height = 68 if width < 700 else 73
    assert abs(size["first"]["h"] - expected_row_height) <= 1, size
    assert size["titlePx"] == 17, size
    assert size["subtitlePx"] == 12, size
    assert size["icon"]["h"] < size["nav"]["h"], size
    assert size["document"] <= width + 3, f"Unexpected horizontal PAGE overflow: {size}"
    assert size["header"]["w"] == size["table"]["w"], size
    if width < 1100:
        assert size["horizontalScroll"], size
    else:
        assert not size["horizontalScroll"], size
    page.screenshot(path=str(OUTPUT / f"process-table-{width}.png"), full_page=True)
    print(f"VIEWPORT={width}: rows={row_count}, rowHeight={size['first']['h']}, title={size['titlePx']}px, subtitle={size['subtitlePx']}px, pageWidth={size['document']}, innerScroll={size['horizontalScroll']}")
    return size


def run():
    with sync_playwright() as engine:
        browser = engine.chromium.launch(executable_path=EDGE, headless=True)
        context = browser.new_context(viewport={"width": 1648, "height": 900}, permissions=["clipboard-read", "clipboard-write"])
        page = context.new_page()
        errors = []
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on("console", lambda msg: errors.append(msg.text) if msg.type == "error" else None)
        marker = page.request.get(BASE + "/_dev/status").json()
        assert marker["app"] == "perchflow-dev-preview" and marker["mode"] == "mock", marker

        inspect_table(page, 1648)
        inspect_table(page, 1440)
        inspect_table(page, 390)

        page.set_viewport_size({"width": 1440, "height": 900})
        page.goto(BASE + "/processes")
        page.locator("#proc-table-body tr[data-process-name]").first.wait_for()
        rows = page.locator("#proc-table-body tr[data-process-name]")
        assert rows.count() == 4

        # Real search and empty state.
        search = page.locator("#proc-search")
        search.fill("mail")
        assert rows.count() == 1
        assert "MailHub" in rows.first.inner_text()
        search.fill("not-an-existing-process")
        assert rows.count() == 0
        assert "没有匹配" in page.locator("#proc-table-body").inner_text() or "No matching" in page.locator("#proc-table-body").inner_text()
        search.fill("")
        assert rows.count() == 4

        # Sorting numeric and name columns.
        page.locator('[data-sort-key="mem"]').click()
        values = page.locator("#proc-table-body td:nth-child(6)").all_text_contents()
        nums = [int(x.replace(" MB", "").strip()) for x in values]
        assert nums == sorted(nums), nums
        page.locator('[data-sort-key="mem"]').click()
        values = page.locator("#proc-table-body td:nth-child(6)").all_text_contents()
        nums = [int(x.replace(" MB", "").strip()) for x in values]
        assert nums == sorted(nums, reverse=True), nums

        # Selection and clear.
        page.locator("#proc-select-all").check()
        assert page.locator(".pf-select-process:checked").count() == 4
        assert page.locator("#proc-selection-toolbar").is_visible()
        page.locator("#proc-clear-selection").click()
        assert page.locator(".pf-select-process:checked").count() == 0
        assert not page.locator("#proc-selection-toolbar").is_visible()

        # Clipboard (copy complete command) and non-mutating edit modal.
        row = page.locator('tr[data-process-name="Daily_Check (DEMO)"]')
        row.locator('[data-action="copy"]').click()
        clip = page.evaluate("navigator.clipboard.readText()")
        assert clip == "python daily_check.py", clip
        row.locator('[data-action="edit"]').click()
        assert page.locator("#add-modal").is_visible()
        assert page.locator("#add-name").input_value() == "Daily_Check (DEMO)"
        page.locator("#add-modal button").filter(has_text="取消").click()
        assert not page.locator("#add-modal").is_visible()

        # End-to-end simulated lifecycle; only mock API is permitted.
        row.locator('[data-action="start"]').click()
        page.wait_for_function("""() => {
            const r=document.querySelector('tr[data-process-name="Daily_Check (DEMO)"]');
            return r && r.querySelector('.pf-status').textContent === 'RUNNING';
        }""")
        row = page.locator('tr[data-process-name="Daily_Check (DEMO)"]')
        assert row.locator('[data-action="start"]').is_disabled()
        row.locator('[data-action="stop"]').click()
        page.wait_for_function("""() => {
            const r=document.querySelector('tr[data-process-name="Daily_Check (DEMO)"]');
            return r && r.querySelector('.pf-status').textContent === 'STOPPED';
        }""")

        # The process name still opens logs/detail; the far-right button
        # now opens the detected web app in a separate tab.
        page.evaluate("window.__opened=null; window.open=(url)=>{window.__opened=url;return null;}")
        page.locator('tr[data-process-name="MailHub (DEMO)"] [data-action="web"]').click()
        assert page.evaluate("window.__opened") == "http://127.0.0.1:9877"
        page.locator('tr[data-process-name="MailHub (DEMO)"] .pf-process-name').click()
        page.wait_for_url("**/process-detail?name=MailHub%20(DEMO)")
        assert page.url.endswith("MailHub%20(DEMO)"), page.url

        # Switching languages should update dynamic table headers.
        page.goto(BASE + "/processes")
        page.locator("#proc-table-body tr[data-process-name]").first.wait_for()
        page.locator("select[data-lang-select]").select_option("en")
        assert page.locator("[data-table-label='name']").inner_text() == "Process name"
        page.locator("select[data-lang-select]").select_option("zh")
        assert page.locator("[data-table-label='name']").inner_text() == "进程名称"
        # Edit uses a separate authenticated settings request, not the list
        page.locator('tr[data-process-name="MailHub (DEMO)"] [data-action="edit"]').click()
        page.locator("#add-advanced").wait_for()
        assert not page.locator("#add-advanced").evaluate("(el)=>el.open")
        page.locator("#add-advanced summary").click()
        assert page.locator("#add-restart-policy").input_value() == "on-failure"
        assert page.locator("#add-web-mode").input_value() == "auto"
        page.locator('button[onclick="closeAddModal()"]').click()

        # Add/restart/delete exercise original real UI callbacks, on MOCK only.
        page.locator('button[onclick="openAddModal()"]').click()
        page.locator("#add-name").fill("Automation QA (DEMO)")
        page.locator("#add-cmd").fill("python")
        page.locator("#add-args").fill("qa_mock.py")
        page.locator('button[onclick="submitAddModal()"]').click()
        added = page.locator('tr[data-process-name="Automation QA (DEMO)"]')
        added.wait_for()
        assert "STOPPED" in added.inner_text()
        added.locator('[data-action="restart"]').click()
        page.wait_for_function("""() => {
            const r=document.querySelector('tr[data-process-name="Automation QA (DEMO)"]');
            return r && r.querySelector('.pf-status').textContent === 'RUNNING';
        }""")
        added.locator('[data-action="stop"]').click()
        page.wait_for_function("""() => {
            const r=document.querySelector('tr[data-process-name="Automation QA (DEMO)"]');
            return r && r.querySelector('.pf-status').textContent === 'STOPPED';
        }""")
        added.locator('[data-action="delete"]').click()
        page.locator('.pf-dialog-overlay .pf-dialog-primary').click()
        page.wait_for_function("""() => !document.querySelector('tr[data-process-name="Automation QA (DEMO)"]')""")

        # Refresh interval should not destroy search and sort state.
        page.locator("#proc-search").fill("mail")
        page.locator('[data-sort-key="name"]').click()
        page.wait_for_timeout(3200)
        assert page.locator("tr[data-process-name]").count() == 1
        assert page.locator("th[aria-sort='ascending']").count() == 1
        assert not errors, errors
        print("INTERACTIONS: search, empty, sorting, selection, clipboard, edit/add/delete modal, start/restart/stop, details, language, auto refresh PASS")
        browser.close()


if __name__ == "__main__":
    run()
