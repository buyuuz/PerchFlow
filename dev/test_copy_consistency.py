"""Prevent misleading add/edit/delete/schedule interface wording regressions."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "frontend"

def source(name):
    return (ROOT / name).read_text(encoding="utf-8")

class CopyConsistencyTests(unittest.TestCase):
    def test_project_script_controls_describe_real_effect(self):
        html = source("projects.html")
        js = source("assets/projects.js")
        for label in ("添加脚本", "添加 Python 脚本", "脚本名称", "管理定时计划"):
            self.assertIn(label, html)
        self.assertNotIn("保存停用任务", html)
        self.assertIn("移除脚本", js)
        self.assertIn("不会删除磁盘上的 .py", js)

    def test_schedule_controls_do_not_imply_manual_run_is_disabled(self):
        self.assertIn("添加定时", source("tasks.html"))
        self.assertIn("启用定时执行", source("tasks.html"))
        js = source("assets/tasks.js")
        self.assertIn("定时已暂停", js)
        self.assertIn("暂停定时", js)
        self.assertIn("button.save_changes", js)

    def test_process_and_docker_copy_follow_action(self):
        self.assertIn("button.save_changes", source("assets/processes.js"))
        js = source("assets/docker.js")
        self.assertIn("确定停止容器", js)
        self.assertIn("确定重启容器", js)
        self.assertNotIn("Are you sure you want to ')+verb", js)

if __name__ == "__main__":
    unittest.main()
