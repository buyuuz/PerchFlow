"""Regression coverage for six PerchFlow user-facing improvements."""
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
def src(path): return (ROOT/path).read_text(encoding="utf-8")

class SixImprovementsTests(unittest.TestCase):
    def test_auto_web_entry_and_advanced_settings(self):
        js=src("frontend/assets/processes.js")
        html=src("frontend/processes.html")
        rust=src("src/web/api_process.rs")
        model=src("src/config/model.rs")
        self.assertIn("function discoverWebProcess(",js)
        self.assertIn("window.open(url,'_blank','noopener,noreferrer')",js)
        self.assertIn("web_url: Option<String>",model)
        self.assertIn('"/processes/:name/web"',rust)
        self.assertIn('"/processes/:name/settings"',rust)
        for field in ("add-web-mode","add-web-url","add-advanced",
                      "add-env-vars","add-restart-policy","add-restart-backoff"):
            self.assertIn('id="'+field+'"',html)
        self.assertIn("document.getElementById('add-advanced').open=false",js)

    def test_duplicate_project_scripts_are_internal_keyed(self):
        model=src("src/config/model.rs")
        api=src("src/web/api_project.rs")
        ui=src("frontend/assets/projects.js")
        self.assertIn("display_name: Option<String>",model)
        self.assertIn("internal_name",api)
        self.assertIn("same visible label",api)
        self.assertIn("task?.display_name || name",ui)
        self.assertIn("task.display_name||task.name",src("frontend/assets/task-detail.js"))
        self.assertIn("display_name:t.display_name.clone()",src("src/web/api_task.rs"))
        self.assertIn("name: internal_name.clone()",api)

    def test_visual_alignment_and_short_schedule_label(self):
        self.assertIn(">添加定时</button>",src("frontend/tasks.html"))
        for page in ("tasks.html","projects.html","task-detail.html"):
            html=src("frontend/"+page)
            self.assertIn('pf-docker-aligned',html)
            self.assertIn("page-spacing.css",html)
        css=src("frontend/assets/page-spacing.css")
        self.assertIn("max-width:1500px",css)
        self.assertIn("padding-left:28px",css)
        self.assertIn("padding-left:15px",css)

    def test_docker_disconnected_does_not_poll_every_five_seconds(self):
        js=src("frontend/assets/docker.js")
        self.assertIn("data.error==='cli_not_found'?0:30000",js)
        self.assertIn("let pollDelay=5000",js)
        self.assertIn("function nextPoll()",js)
        self.assertNotIn("setInterval(()=>{",js)

if __name__=="__main__":
    unittest.main()
