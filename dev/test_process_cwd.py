"""Source and host-filesystem regression tests for the process working directory.

These tests do not start a real managed process or touch the production server.
The preview mock has an isolated, explicit cwd validator matching Rust semantics.
"""
from pathlib import Path
import tempfile
import unittest

from preview_server import cwd_error_on_host

ROOT = Path(__file__).resolve().parents[1]
FRONT = ROOT / "frontend"
WEB = ROOT / "src" / "web"
PROCESS_HTML = (FRONT / "processes.html").read_text(encoding="utf-8")
PROCESS_JS = (FRONT / "assets" / "processes.js").read_text(encoding="utf-8")
CWD_JS = (FRONT / "assets" / "process-cwd.js").read_text(encoding="utf-8")
PROCESS_CSS = (FRONT / "assets" / "process-table.css").read_text(encoding="utf-8")
I18N = (FRONT / "assets" / "i18n.js").read_text(encoding="utf-8")
API = (WEB / "api_process.rs").read_text(encoding="utf-8")
BACKEND = (ROOT / "src/core/process.rs").read_text(encoding="utf-8")


class WorkingDirectoryTests(unittest.TestCase):
    def test_real_host_directory_validation(self):
        with tempfile.TemporaryDirectory(prefix="PerchFlow cwd unittest ") as directory:
            folder = Path(directory)
            data = folder / "config.json"
            data.write_text("{}", encoding="utf-8")
            self.assertIsNone(cwd_error_on_host(directory))
            self.assertIsNone(cwd_error_on_host(None))
            self.assertIsNone(cwd_error_on_host(""))
            self.assertEqual(cwd_error_on_host(str(data)), "cwd_not_directory")
            self.assertEqual(cwd_error_on_host(str(folder / "missing")), "cwd_not_found")
        self.assertEqual(cwd_error_on_host("relative\\folder"), "cwd_not_absolute")
        self.assertEqual(cwd_error_on_host("D:" + "x"*1030), "cwd_too_long")

    def test_add_edit_expose_optional_working_directory(self):
        self.assertIn('id="add-cwd"', PROCESS_HTML)
        self.assertIn('id="cwd-suggestion"', PROCESS_HTML)
        self.assertIn('id="cwd-use-suggestion"', PROCESS_HTML)
        self.assertIn('aria-describedby="cwd-help cwd-validation"', PROCESS_HTML)
        self.assertIn('/assets/process-cwd.js?v=cwd1', PROCESS_HTML)
        self.assertIn("resetWorkingDirectoryField();", PROCESS_JS)
        self.assertIn("resetWorkingDirectoryField(p.cwd || '');", PROCESS_JS)
        self.assertIn("const cwd = getWorkingDirectoryInput();", PROCESS_JS)
        self.assertIn("cwd: cwd || null", PROCESS_JS)
        self.assertIn(".pf-cwd-suggestion", PROCESS_CSS)
        self.assertIn(".pf-cwd-validation", PROCESS_CSS)

    def test_exe_suggestion_requires_acceptance(self):
        self.assertIn("function directorySuggestedByExe(command)", CWD_JS)
        self.assertIn("return '';", CWD_JS)
        self.assertIn("cwdSuggestedDirectory = directorySuggestedByExe(cmdInput.value)", CWD_JS)
        self.assertIn("function applySuggestedCwd()", CWD_JS)
        self.assertIn("cwdInput.value = cwdSuggestedDirectory", CWD_JS)
        # No modification in the input handler; only the explicit button fills cwd.
        on_cmd = CWD_JS.split("cmd?.addEventListener('input'", 1)[1].split(";", 1)[0]
        self.assertNotIn("cwdInput.value =", on_cmd)
        self.assertIn("cwd?.addEventListener('input'", CWD_JS)

    def test_host_check_is_not_a_browser_filesystem_guess(self):
        self.assertIn('/api/processes/validate-cwd', CWD_JS)
        self.assertIn("method: 'POST'", CWD_JS)
        self.assertIn('verify_working_directory(value: &str)', API)
        self.assertIn('std::fs::metadata(path)', API)
        self.assertIn('path.is_absolute()', API)
        self.assertIn('cwd_not_found', API)
        self.assertIn('cwd_not_directory', API)
        self.assertIn('cwd_not_absolute', API)
        self.assertIn('Path::new(cwd).is_dir()', BACKEND)
        self.assertIn('/processes/:name/cwd', API)

    def test_cwd_only_update_preserves_hidden_settings(self):
        self.assertIn("base + '/cwd'", PROCESS_JS)
        self.assertIn("has_env = payload.get(\"env\").is_some()", API)
        self.assertIn("if !has_env { def.env = p.env.clone(); }", API)
        self.assertIn("if !has_restart { def.restart = p.restart.clone(); }", API)
        self.assertIn("Some(p) => {", API)
        self.assertIn("let changed = p.cwd != cwd;", API)
        self.assertIn('ctx.supervisor.restart(&name).await', API)

    def test_zh_and_english_labels_are_present(self):
        for key in ("label.cwd", "label.cwd_hint", "placeholder.cwd",
                    "button.use_cwd_suggestion", "cwd.checking", "cwd.valid",
                    "error.cwd_not_absolute", "error.cwd_not_found",
                    "error.cwd_not_directory", "error.cwd_inaccessible",
                    "error.cwd_check_failed"):
            self.assertEqual(I18N.count("'" + key + "'"), 2, key)


if __name__ == "__main__":
    unittest.main()
