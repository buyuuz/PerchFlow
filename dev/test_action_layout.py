"""Keep fixed right-side actions consistent across the Processes and Tasks tables."""
import unittest
from pathlib import Path

R=Path(__file__).resolve().parents[1]
def content(relative):
    return (R/relative).read_text(encoding="utf-8")
class ActionLayoutTests(unittest.TestCase):
    def test_process_frame_matches_docker(self):
        html=content("frontend/processes.html")
        css=content("frontend/assets/page-spacing.css")
        self.assertIn('class="pf-docker-aligned pf-process-main"',html)
        self.assertIn('page-spacing.css?v=2',html)
        self.assertIn("max-width:1500px",css)
        self.assertIn("padding-left:28px",css)
        self.assertIn("padding-left:15px",css)
    def test_sixth_web_button_is_always_rendered(self):
        js=content("frontend/assets/processes.js")
        self.assertIn("!running || !webpage, 'pf-action--terminal'",js)
        self.assertNotIn("webpage ? actionButton(p, 'web'",js)
        self.assertIn("else if (action === 'web')",js)
    def test_action_columns_have_fixed_capacity_and_shared_gap(self):
        process=content("frontend/processes.html")
        tasks=content("frontend/tasks.html")
        css=content("frontend/assets/process-table.css")
        task_css=content("frontend/assets/tasks-table.css")
        for html in (process,tasks):
            self.assertIn('<col style="width:254px">',html)
            self.assertIn('process-table.css?v=table2',html)
        self.assertIn('gap: 4px;',css)
        self.assertIn('.pf-process-table th:last-child { text-align: right; }',css)
        self.assertNotIn('.pf-task-table .pf-actions { gap:',task_css)

if __name__=="__main__": unittest.main()
