"""Guard the scheduled-task table layout and keep its API contract intact.

These source-level tests require only Python stdlib and run in GitHub Actions.
End-to-end Edge interaction checks: python dev/verify_tasks_table_browser.py
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "frontend"
TASK_PAGE = (WEB / "tasks.html").read_text(encoding="utf-8")
TASK_JS = (WEB / "assets" / "tasks.js").read_text(encoding="utf-8")
TASK_CSS = (WEB / "assets" / "tasks-table.css").read_text(encoding="utf-8")
SHARED_CSS = (WEB / "assets" / "process-table.css").read_text(encoding="utf-8")


class TasksTableSourceTests(unittest.TestCase):
    def test_semantic_table_columns(self):
        self.assertIn('class="pf-process-table pf-task-table"', TASK_PAGE)
        self.assertIn('id="task-rows"', TASK_PAGE)
        self.assertIn('id="task-search"', TASK_PAGE)
        self.assertIn('href="/assets/tasks-table.css?v=tasks2"', TASK_PAGE)
        self.assertIn('href="/assets/process-table.css?v=table2"', TASK_PAGE)
        self.assertNotIn("hidden md:grid grid-cols-[auto_1.5fr", TASK_PAGE)
        for key in ("name", "enabled", "schedule", "command",
                    "last_run", "last_exit_code", "actions"):
            self.assertIn(f'data-task-label="{key}"', TASK_PAGE)

    def test_task_actions_preserve_api(self):
        for name in ("runNow", "deleteTask", "toggleEnabled", "openTaskModal",
                     "editTask", "submitTaskModal", "loadTasks", "closeTaskModal"):
            self.assertIn(f"function {name}(", TASK_JS)
        for action in ("run", "toggle", "edit", "delete", "copy"):
            self.assertIn(f"'{action}'", TASK_JS)
        self.assertIn("/api/tasks", TASK_JS)
        self.assertIn("setInterval(loadTasks, 3000)", TASK_JS)
        self.assertIn("data-task-action", TASK_JS)
        self.assertIn("data-name=", TASK_JS)

    def test_search_sort_lang_and_selection_present(self):
        for token in (
            "visibleTasks()", "renderTaskTable()", "taskSearch",
            "taskSortKey", "selectedTasks", "task-select-all",
            "task-clear-selection", "task-selection-toolbar",
            "data-task-sort", "data-task-label", "getLang()",
            "taskText('count'",
        ):
            self.assertIn(token, TASK_JS + TASK_PAGE)

    def test_no_nonexistent_next_run_field(self):
        self.assertNotIn('next_run', TASK_PAGE + TASK_JS)
        self.assertIn("last_exit_code", TASK_JS)
        self.assertIn("last_run", TASK_JS)
        self.assertIn("schedule", TASK_JS)

    def test_visuals_match_processes_and_mobile(self):
        self.assertIn(".pf-process-table tbody tr", SHARED_CSS)
        self.assertIn(".pf-task-table tbody tr { height: 73px; }", TASK_CSS)
        self.assertIn(".pf-task-table tbody tr { height: 68px; }", TASK_CSS)
        self.assertIn(".pf-task-table", TASK_CSS)
        self.assertIn(".pf-task-cron", TASK_CSS)
        self.assertIn(".pf-task-status--enabled", TASK_CSS)
        self.assertIn(".pf-task-status--disabled", TASK_CSS)
        self.assertIn(".pf-task-result--success", TASK_CSS)
        self.assertIn(".pf-task-result--failed", TASK_CSS)
        self.assertIn("overflow-x: auto;", SHARED_CSS)
        self.assertIn('class="pf-main-nav-link"', TASK_PAGE)


if __name__ == "__main__":
    unittest.main()
