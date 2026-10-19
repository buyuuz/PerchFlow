"""Source-level regression checks for the compact process management table.

No browser dependency: runs in Windows Actions as part of dev unittest suite.
The interactive Playwright checks are in verify_process_table_browser.py.
"""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HTML = (ROOT / "frontend" / "processes.html").read_text(encoding="utf-8")
CSS = (ROOT / "frontend" / "assets" / "process-table.css").read_text(encoding="utf-8")
JS = (ROOT / "frontend" / "assets" / "processes.js").read_text(encoding="utf-8")


class ProcessTableSourceTests(unittest.TestCase):
    def test_table_has_all_fields(self):
        self.assertIn('class="pf-process-table"', HTML)
        self.assertIn('id="proc-table-body"', HTML)
        self.assertNotIn('id="proc-grid"', HTML)
        self.assertIn('id="proc-search"', HTML)
        self.assertIn('/assets/process-table.css?v=table1', HTML)
        for field in (
            'name', 'status', 'pid', 'cpu', 'mem', 'uptime', 'restarts', 'cmd', 'actions',
        ):
            self.assertIn(f'data-table-label="{field}"', HTML)

    def test_actions_wired_to_existing_api(self):
        for action in ('start', 'restart', 'stop', 'edit', 'delete', 'logs', 'copy'):
            self.assertIn(f"'{action}'", JS)
        self.assertIn("function renderProcessRow(p)", JS)
        self.assertIn("function renderProcessTable()", JS)
        self.assertIn("function editProcess(name)", JS)
        self.assertIn("async function deleteProcess(name)", JS)
        self.assertIn("function openAddModal()", JS)
        self.assertIn("async function submitAddModal()", JS)
        self.assertIn("async function loadProcesses()", JS)
        self.assertIn("setInterval(loadProcesses, 3000)", JS)
        self.assertIn("data-action=", JS)

    def test_search_sort_and_selection_are_live(self):
        self.assertIn("search.addEventListener('input'", JS)
        self.assertIn("data-sort-key", JS)
        self.assertIn("sortAscending", JS)
        self.assertIn("selectedProcesses", JS)
        self.assertIn("proc-select-all", JS)
        self.assertIn("proc-clear-selection", JS)
        self.assertIn("getVisibleProcesses()", JS)

    def test_layout_is_constrained_and_scrolls_only_inside_list(self):
        self.assertIn("min-width: 1220px", CSS)
        self.assertIn("overflow-x: auto", CSS)
        self.assertIn(".pf-process-table tbody tr", CSS)
        self.assertIn(".pf-process-icon", CSS)
        self.assertIn(".pf-status--running", CSS)
        self.assertIn(".pf-status--crashed", CSS)
        self.assertIn(".pf-command-text", CSS)
        self.assertIn(".pf-action--terminal", CSS)
        self.assertIn("@media (max-width: 700px)", CSS)


if __name__ == "__main__":
    unittest.main()
