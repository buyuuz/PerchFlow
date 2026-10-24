"""Regression checks: revert excess action-table widths while retaining the Web icon."""
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
def content(path): return (ROOT/path).read_text(encoding="utf-8")

class ActionLayoutTests(unittest.TestCase):
    def test_page_margin_stays_aligned(self):
        html=content("frontend/processes.html")
        css=content("frontend/assets/page-spacing.css")
        self.assertIn('class="pf-docker-aligned pf-process-main"',html)
        self.assertIn('page-spacing.css?v=2',html)
        for token in ("max-width:1500px","padding-left:28px","padding-left:15px"):
            self.assertIn(token,css)

    def test_sixth_web_button_is_always_rendered_and_disabled_safely(self):
        js=content("frontend/assets/processes.js")
        self.assertIn("tableText('web'), !running || !webpage, 'pf-action--terminal'",js)
        self.assertNotIn("webpage ? actionButton(p, 'web'",js)
        self.assertNotIn("（未识别网页地址）",js)
        self.assertIn("else if (action === 'web')",js)
        css=content("frontend/assets/process-table.css")
        self.assertIn(".pf-action--terminal:disabled",css)
        self.assertIn(".pf-action:disabled",css)

    def test_exact_original_column_widths_and_alignments(self):
        html=content("frontend/processes.html")
        tasks=content("frontend/tasks.html")
        css=content("frontend/assets/process-table.css")
        task_css=content("frontend/assets/tasks-table.css")
        for part in ('<col style="width:17%">','<col style="width:8%">',
                     '<col style="width:16.3%">','<col style="width:18.9%">'):
            self.assertIn(part,html)
        for part in ('<col style="width:18%">','<col style="width:14%">',
                     '<col style="width:14.5%">'):
            self.assertIn(part,tasks)
        self.assertNotIn('<col style="width:254px">',html+tasks)
        self.assertNotIn('.pf-process-table th:last-child { text-align: right; }',css)
        self.assertIn('gap: 3px;',css)
        self.assertIn('.pf-task-table .pf-actions { gap: 4px; }',task_css)
        self.assertIn('.pf-task-table .pf-action { flex-basis: 32px; }',task_css)

if __name__=="__main__": unittest.main()
