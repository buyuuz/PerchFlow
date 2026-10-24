"""Guard the process editor modal from topbar overlap and native select drift."""
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
def read(p): return (ROOT/p).read_text(encoding="utf-8")

class ProcessModalStyleTest(unittest.TestCase):
    def test_modal_stays_above_header(self):
        html=read("frontend/processes.html")
        css=read("frontend/assets/process-advanced.css")
        self.assertIn('pf-process-modal fixed inset-0',html)
        self.assertIn("z-index:70",css)
        self.assertIn("max-height:min(calc(100dvh - 32px), 840px)",css)
        self.assertIn("overflow-y:auto",css)
        self.assertIn(".pf-process-edit-dialog:hover",css)
        self.assertIn("background:#151b20",css)
    def test_web_mode_and_restart_mode_use_styled_native_select(self):
        html=read("frontend/processes.html")
        css=read("frontend/assets/process-advanced.css")
        for name in ("add-web-mode","add-restart-policy"):
            self.assertIn('id="'+name+'" class="pf-process-field"',html)
        self.assertIn("select.pf-process-field",css)
        self.assertIn("appearance:none",css)
        self.assertIn("background-position:right 12px center",css)
        self.assertIn(".pf-process-field:focus-visible",css)
        self.assertIn("/assets/process-advanced.css?v=modal2",html)

if __name__=="__main__":unittest.main()
