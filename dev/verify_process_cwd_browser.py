"""Windows Edge E2E: optional working directory in PerchFlow process forms.

Only mutates the 9011 MOCK API after verifying /_dev/status. Never touches
the production 9002 server nor any real managed process.
"""
import tempfile
from pathlib import Path

from playwright.sync_api import sync_playwright

BASE = "http://127.0.0.1:9011"
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
OUT = Path(__file__).resolve().parent / "screenshots"
OUT.mkdir(exist_ok=True)


def main():
    with tempfile.TemporaryDirectory(prefix="PerchFlow CWD QA ") as directory:
        workspace = Path(directory)
        nested = workspace / "another project"
        nested.mkdir()
        config_file = workspace / "config.json"
        config_file.write_text("{}", encoding="utf-8")

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(executable_path=EDGE, headless=True)
            context = browser.new_context(viewport={"width": 1440, "height": 900})
            page = context.new_page()
            errors = []
            page.on("pageerror", lambda exc: errors.append(str(exc)))
            status = page.request.get(BASE + "/_dev/status").json()
            assert status["app"] == "perchflow-dev-preview" and status["mode"] == "mock", status
            page.goto(BASE + "/processes")
            page.locator('tr[data-process-name="MailHub (DEMO)"]').wait_for()
            assert page.locator("#add-cwd").count() == 1

            # Add an EXE: suggest directory, but never silently assign it.
            page.locator('button[onclick="openAddModal()"]').click()
            assert page.locator("#add-cwd").input_value() == ""
            assert page.locator("#cwd-suggestion").is_hidden()
            exe_path = r"D:\Projects\wb2api-panel-v1.13.0\wb2api.exe"
            exe_dir = r"D:\Projects\wb2api-panel-v1.13.0"
            page.locator("#add-cmd").fill(exe_path)
            page.locator("#cwd-suggestion").wait_for(state="visible")
            assert page.locator("#cwd-suggest-value").inner_text() == exe_dir
            assert page.locator("#add-cwd").input_value() == ""
            page.locator("#cwd-use-suggestion").click()
            assert page.locator("#add-cwd").input_value() == exe_dir
            assert page.locator("#cwd-suggestion").is_hidden()

            # User overrides suggestion; changing the EXE must never overwrite cwd.
            page.locator("#add-cwd").fill(str(workspace))
            page.locator("#add-cmd").fill(r"D:\Tools\Another Program\second.exe")
            assert page.locator("#add-cwd").input_value() == str(workspace)
            assert page.locator("#cwd-suggestion").is_hidden()
            # Bare commands and interpreters never auto-fill cwd.
            page.locator("#add-cmd").fill("python")
            page.locator("#add-cwd").fill("")
            assert page.locator("#cwd-suggestion").is_hidden()
            page.locator("#add-cmd").fill(r'"D:\Scoop\apps\python312\current\python.exe"')
            assert page.locator("#cwd-suggestion").is_visible()
            assert page.locator("#add-cwd").input_value() == ""

            # Client-side path syntax rejection happens without a network check.
            calls = []
            page.on("request", lambda request: calls.append(request.url)
                    if request.method == "POST" and "/validate-cwd" in request.url else None)
            page.locator("#add-name").fill("Working Directory QA")
            page.locator("#add-cmd").fill(exe_path)
            page.locator("#add-args").fill("-config config.json")
            page.locator("#add-alias-zh").fill("工作目录测试")
            page.locator("#add-cwd").fill(r".\relative-folder")
            page.locator('button[onclick="submitAddModal()"]').click()
            page.locator('#cwd-validation[data-tone="error"]').wait_for()
            assert not calls, calls
            assert page.locator("#add-modal").is_visible()
            assert page.request.get(BASE + "/api/processes/Working%20Directory%20QA").status == 404

            # Missing folder and existing file are rejected by the HOST, not guesses.
            missing = workspace / "not-created"
            page.locator("#add-cwd").fill(str(missing))
            page.locator('button[onclick="submitAddModal()"]').click()
            page.wait_for_function("""() => document.querySelector('#cwd-validation')?.dataset.statusKey === 'error.cwd_not_found'""")
            assert page.locator("#add-modal").is_visible()
            page.locator("#add-cwd").fill(str(config_file))
            page.locator('button[onclick="submitAddModal()"]').click()
            page.wait_for_function("""() => document.querySelector('#cwd-validation')?.dataset.statusKey === 'error.cwd_not_directory'""")
            assert page.locator("#add-modal").is_visible()
            assert len(calls) >= 2, calls
            print("SUGGESTION_AND_VALIDATION=PASS", flush=True)

            # Valid existing directory saved to API cwd, independent of EXE path.
            page.locator("#add-cwd").fill(str(workspace))
            page.locator('button[onclick="submitAddModal()"]').click()
            new_row = page.locator('tr[data-process-name="Working Directory QA"]')
            new_row.wait_for()
            result = page.request.get(BASE + "/api/processes/Working%20Directory%20QA").json()
            assert result["cwd"] == str(workspace), result
            assert result["cmd_raw"] == exe_path
            assert result["args"] == ["-config", "config.json"]
            assert result["alias_zh"] == "工作目录测试"
            new_row.locator('[data-action="edit"]').click()
            assert page.locator("#add-cwd").input_value() == str(workspace)
            assert page.locator("#cwd-suggestion").is_hidden()
            print("CREATE_AND_EDIT_PREFILL=PASS", flush=True)

            # Edit only cwd: uses dedicated route, does not overwrite cmd/alias.
            calls2 = []
            page.on("request", lambda request: calls2.append((request.method, request.url))
                    if request.method in ("PUT", "POST") else None)
            page.locator("#add-cwd").fill(str(nested))
            page.locator('button[onclick="submitAddModal()"]').click()
            page.wait_for_function("""() => document.querySelector('#add-modal')?.classList.contains('hidden')""")
            updated = page.request.get(BASE + "/api/processes/Working%20Directory%20QA").json()
            assert updated["cwd"] == str(nested)
            assert updated["alias_zh"] == "工作目录测试"
            assert updated["cmd_raw"] == exe_path
            assert any(method == "PUT" and url.endswith("/api/processes/Working%20Directory%20QA/cwd") for method, url in calls2), calls2
            print("CWD_ONLY_CONFIG_SAFE=PASS", flush=True)

            # Clear cwd returns to null/default behavior.
            new_row = page.locator('tr[data-process-name="Working Directory QA"]')
            new_row.locator('[data-action="edit"]').click()
            page.locator("#add-cwd").fill("")
            page.locator('button[onclick="submitAddModal()"]').click()
            page.wait_for_function("""() => document.querySelector('#add-modal')?.classList.contains('hidden')""")
            assert page.request.get(BASE + "/api/processes/Working%20Directory%20QA").json()["cwd"] is None

            # Running process: new directory takes effect only after restart.
            running_row = page.locator('tr[data-process-name="MailHub (DEMO)"]')
            before = page.request.get(BASE + "/api/processes/MailHub%20(DEMO)").json()
            assert before["status"] == "running" and before["pid"] == 12340
            running_row.locator('[data-action="edit"]').click()
            page.locator("#add-cwd").fill(str(workspace))
            page.locator('button[onclick="submitAddModal()"]').click()
            page.wait_for_function("""() => document.querySelector('#add-modal')?.classList.contains('hidden')""")
            after = page.request.get(BASE + "/api/processes/MailHub%20(DEMO)").json()
            assert after["cwd"] == str(workspace) and after["pid"] != before["pid"], after
            assert after["status"] == "running"
            running_row = page.locator('tr[data-process-name="MailHub (DEMO)"]')
            running_row.locator('[data-action="edit"]').click()
            page.locator("#add-alias-zh").fill("邮件运营中心")
            page.locator('button[onclick="submitAddModal()"]').click()
            page.wait_for_function("""() => document.querySelector('#add-modal')?.classList.contains('hidden')""")
            alias_after = page.request.get(BASE + "/api/processes/MailHub%20(DEMO)").json()
            assert alias_after["pid"] == after["pid"] and alias_after["cwd"] == str(workspace)
            print("RUNNING_CWD_RESTART_AND_ALIAS_NO_RESTART=PASS", flush=True)

            # Legacy no cwd remains editable; language toggle and small modal.
            page.locator(".pf-lang-trigger").click()
            page.locator('[data-lang-value="zh"]').click()
            page.locator('tr[data-process-name="Data Collector (DEMO)"] [data-action="edit"]').click()
            assert page.locator("#add-cwd").input_value() == ""
            assert "工作目录" in page.locator('label[for="add-cwd"]').inner_text()
            page.locator('button[onclick="closeAddModal()"]').click()

            for width, height in ((1440, 900), (390, 700), (320, 720)):
                page.set_viewport_size({"width": width, "height": height})
                page.goto(BASE + "/processes")
                page.locator("tr[data-process-name]").first.wait_for()
                page.locator('button[onclick="openAddModal()"]').click()
                page.locator("#add-cmd").fill(exe_path)
                page.locator("#cwd-suggestion").wait_for(state="visible")
                assert page.evaluate("document.documentElement.scrollWidth") <= width + 2
                modal = page.locator("#add-modal .glass-card").bounding_box()
                assert modal["height"] <= height + 1, (width, modal)
                page.screenshot(path=str(OUT / f"process-cwd-modal-{width}.png"))
                page.locator('button[onclick="closeAddModal()"]').click()
            assert not errors, errors
            print("MOBILE_LAYOUT_AND_NO_JS_ERRORS=PASS", flush=True)

            # Delete only a QA entry on the mock instance.
            page.set_viewport_size({"width": 1440, "height": 900})
            page.goto(BASE + "/processes")
            page.once("dialog", lambda dialog: dialog.accept())
            page.locator('tr[data-process-name="Working Directory QA"] [data-action="delete"]').click()
            page.wait_for_function('''() => !document.querySelector('tr[data-process-name="Working Directory QA"]')''')
            print("WORKING_DIRECTORY_BROWSER_QA=PASS", flush=True)
            browser.close()


if __name__ == "__main__":
    main()
