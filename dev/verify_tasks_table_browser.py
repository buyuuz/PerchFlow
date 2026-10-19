"""Real Edge checks for PerchFlow scheduled-task management table.
Only http://127.0.0.1:9011 mock mode; never touches actual Perch process API.
"""
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
SCREEN = ROOT / "dev" / "screenshots"
SCREEN.mkdir(exist_ok=True)
BASE = "http://127.0.0.1:9011"
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"


def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, executable_path=EDGE)
        context = browser.new_context(viewport={"width": 1440,"height": 900},
                                      permissions=["clipboard-read","clipboard-write"])
        page = context.new_page()
        errors = []
        page.on("pageerror", lambda exc: errors.append(str(exc)))
        state = page.request.get(BASE + "/_dev/status").json()
        assert state["app"] == "perchflow-dev-preview" and state["mode"] == "mock", state

        for width in (1648, 1440, 390):
            page.set_viewport_size({"width": width, "height": 900})
            page.goto(BASE + "/tasks",wait_until="domcontentloaded")
            page.locator("#task-rows tr[data-task-name]").first.wait_for()
            page.wait_for_function("() => document.querySelectorAll('#task-rows tr[data-task-name]').length === 4")
            size = page.evaluate("""() => {
              const table=document.querySelector('.pf-task-table');
              const wrapper=document.querySelector('.pf-process-table-scroll');
              const first=document.querySelector('#task-rows tr[data-task-name]');
              const title=document.querySelector('.pf-management-page-title');
              return {
                width:innerWidth, pageWidth:document.documentElement.scrollWidth,
                tableWidth:table.getBoundingClientRect().width,
                wrapperWidth:wrapper.getBoundingClientRect().width,
                rowHeight:first.getBoundingClientRect().height,
                cols:first.children.length,
                scrollable:wrapper.scrollWidth-wrapper.clientWidth>3,
                titleFont:parseFloat(getComputedStyle(title).fontSize)
              };
            }""")
            print("LAYOUT",width,size,flush=True)
            assert size["cols"] == 8, size
            assert size["pageWidth"] <= width + 3, size
            assert abs(size["rowHeight"] - (68 if width < 700 else 73)) < 1, size
            assert size["titleFont"] == 17, size
            assert size["scrollable"] == (width < 1100), size
            page.screenshot(path=str(SCREEN / f"tasks-table-{width}.png"),full_page=True)

        page.set_viewport_size({"width": 1440, "height": 900})
        page.goto(BASE + "/tasks")
        rows=page.locator("#task-rows tr[data-task-name]")
        rows.first.wait_for()
        assert rows.count() == 4
        assert page.locator("#task-count").inner_text().startswith("共 4 项")
        assert page.locator("th").count() == 8
        assert page.locator(".pf-task-result--success").count() == 2
        assert page.locator(".pf-task-result--failed").count() == 1

        search=page.locator("#task-search")
        search.fill("Weekly")
        assert rows.count() == 1
        assert "Weekly Report" in rows.first.inner_text()
        search.fill("no-such-cron-job")
        assert rows.count() == 0
        assert "没有匹配" in page.locator("#task-rows").inner_text()
        search.fill("")
        assert rows.count() == 4

        page.locator('[data-task-sort="name"]').click()
        names=page.locator("tr[data-task-name] .pf-process-name").all_text_contents()
        assert names == sorted(names), names
        page.locator('[data-task-sort="name"]').click()
        names=page.locator("tr[data-task-name] .pf-process-name").all_text_contents()
        assert names == sorted(names,reverse=True), names

        page.locator("#task-select-all").check()
        assert page.locator(".pf-task-select:checked").count() == 4
        assert page.locator("#task-selection-toolbar").is_visible()
        page.locator("#task-clear-selection").click()
        assert page.locator(".pf-task-select:checked").count() == 0

        row=page.locator('tr[data-task-name="Morning Check (DEMO)"]')
        row.locator('[data-task-action="copy"]').click()
        assert page.evaluate("navigator.clipboard.readText()") == "python daily_check.py"

        row.locator('[data-task-action="toggle"]').click()
        page.wait_for_function("""() => {
            const r=document.querySelector('tr[data-task-name="Morning Check (DEMO)"]');
            return r && r.querySelector('.pf-task-status--disabled');
        }""")
        assert page.locator("#task-count").inner_text().endswith("2 项")
        row.locator('[data-task-action="toggle"]').click()
        page.wait_for_function("""() => {
            const r=document.querySelector('tr[data-task-name="Morning Check (DEMO)"]');
            return r && r.querySelector('.pf-task-status--enabled');
        }""")

        row.locator('[data-task-action="edit"]').click()
        assert page.locator("#task-modal").is_visible()
        assert page.locator("#task-name").input_value() == "Morning Check (DEMO)"
        assert page.locator("#task-schedule").input_value() == "0 0 9 * * *"
        page.locator("#task-schedule").fill("0 0 8 * * *")
        page.locator('button[onclick="submitTaskModal()"]').click()
        page.wait_for_function("""() => {
           const row=document.querySelector('tr[data-task-name="Morning Check (DEMO)"]');
           return row && row.querySelector('.pf-task-cron').textContent === '0 0 8 * * *';
        }""")

        # API run endpoint should write mock run result (never launch any process).
        page.locator('tr[data-task-name="Weekly Report (DEMO)"] [data-task-action="run"]').click()
        page.wait_for_function("""() => {
          const r=document.querySelector('tr[data-task-name="Weekly Report (DEMO)"]');
          return r && r.querySelector('.pf-task-result--success');
        }""")

        page.locator('button[onclick="openTaskModal()"]').click()
        page.locator("#task-name").fill("QA Cron Demo")
        page.locator("#task-cmd").fill("python")
        page.locator("#task-args").fill("qa_task.py")
        page.locator("#task-schedule").fill("0 0 7 * * *")
        page.locator('button[onclick="submitTaskModal()"]').click()
        page.locator('tr[data-task-name="QA Cron Demo"]').wait_for()
        assert page.locator("tr[data-task-name]").count() == 5

        page.once("dialog",lambda dialog: dialog.accept())
        page.locator('tr[data-task-name="QA Cron Demo"] [data-task-action="delete"]').click()
        page.wait_for_function("() => !document.querySelector('tr[data-task-name=\"QA Cron Demo\"]')")
        assert page.locator("tr[data-task-name]").count() == 4

        page.locator("select[data-lang-select]").select_option("en")
        assert page.locator('[data-task-label="name"]').inner_text() == "Task name"
        assert "Total:" in page.locator("#task-count").inner_text()
        page.locator("select[data-lang-select]").select_option("zh")
        assert page.locator('[data-task-label="name"]').inner_text() == "任务名称"

        page.locator("#task-search").fill("backup")
        page.locator('[data-task-sort="name"]').click()
        page.wait_for_timeout(3300)
        assert rows.count() == 1
        assert page.locator("th[aria-sort='ascending']").count() == 1

        page.goto(BASE + "/processes")
        page.locator('tr[data-process-name]').first.wait_for()
        page.locator('nav a[href="/tasks"]').first.click()
        page.wait_for_url("**/tasks")
        page.locator("tr[data-task-name]").first.wait_for()
        assert not errors, errors
        print("TASK_TABLE_BROWSER=PASS: search/sort/selection/copy/toggle/edit/run/add/delete/lang/polling/navigation",flush=True)
        browser.close()


if __name__ == "__main__":
    main()
