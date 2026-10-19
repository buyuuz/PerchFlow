"""Source-level regression checks for the redesigned process-detail interface.

The Edge end-to-end counterpart is dev/verify_process_detail_browser.py.
No Pillow, Playwright or Rust toolchain needed for unit tests.
"""
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
WEB=ROOT/"frontend"
PAGE=(WEB/"process-detail.html").read_text(encoding="utf-8")
STYLE=(WEB/"assets"/"process-detail.css").read_text(encoding="utf-8")
JS=(WEB/"assets"/"process-detail.js").read_text(encoding="utf-8")
I18N=(WEB/"assets"/"i18n.js").read_text(encoding="utf-8")

class ProcessDetailDesignTests(unittest.TestCase):
    def test_page_uses_shared_brand_language_and_actions(self):
        self.assertIn('class="pf-detail-main"',PAGE)
        self.assertIn('class="pf-detail-header"',PAGE)
        self.assertIn('class="pf-detail-header-actions"',PAGE)
        self.assertIn('/assets/branding.css?v=brand4',PAGE)
        self.assertIn('/assets/topbar-controls.css?v=top2',PAGE)
        self.assertIn('/assets/topbar-controls.js?v=top2',PAGE)
        self.assertIn('/assets/process-detail.css?v=detail1',PAGE)
        self.assertIn('data-pf-language-picker',PAGE)
        for lang in ("zh","en"):
            self.assertIn(f'data-lang-value="{lang}"',PAGE)
        for obsolete in ("es","ja","pt"):
            self.assertNotIn(f'data-lang-value="{obsolete}"',PAGE)

    def test_process_fields_and_api_actions_are_unchanged(self):
        for field in ("proc-name","proc-status","proc-pid","proc-cpu","proc-mem",
                      "proc-uptime","proc-cmd","log-content","lines-select","live-toggle"):
            self.assertIn(f'id="{field}"',PAGE)
        for operation in ("start","restart","stop"):
            self.assertIn(f'onclick="act(\'{operation}\')"',PAGE)
        self.assertIn("/api/processes/",JS)
        self.assertIn("/api/logs/stream/",JS)
        self.assertIn("setInterval(loadProcess, 3000)",JS)
        self.assertIn("function startStream()",JS)
        self.assertIn("function stopStream()",JS)
        self.assertIn("function renderLineSeq(item)",JS)

    def test_log_controls_and_accessibility(self):
        self.assertIn('onclick="copyProcessCommand()"',PAGE)
        self.assertIn('onclick="clearLogs()"',PAGE)
        self.assertIn('onchange="loadRecentLogs()"',PAGE)
        self.assertIn('onclick="toggleLive()"',PAGE)
        self.assertIn('role="switch" aria-checked="true"',PAGE)
        self.assertIn('role="log"',PAGE)
        self.assertIn('tabindex="0"',PAGE)
        self.assertIn("toggle.setAttribute('aria-checked', String(liveOn))",JS)
        self.assertIn("eventSource.close()",JS)
        self.assertIn("escapeHtml(line)",JS)
        self.assertIn("updateLogCount()",JS)

    def test_layout_preserves_one_screen_desktop_and_mobile_scroll(self):
        self.assertIn("height: 100dvh;",STYLE)
        self.assertIn("overflow: hidden;",STYLE)
        self.assertIn(".pf-detail-metrics",STYLE)
        self.assertIn("grid-template-columns: repeat(4, minmax(0, 1fr))",STYLE)
        self.assertIn(".pf-terminal-window",STYLE)
        self.assertIn(".pf-terminal-content",STYLE)
        self.assertIn("@media (max-width: 620px)",STYLE)
        self.assertIn("flex-direction: column; align-items: stretch; gap: 12px;",STYLE)
        self.assertIn("grid-template-columns: repeat(2, minmax(0,1fr))",STYLE)
        self.assertIn("overflow-y: auto; height: auto;",STYLE)

    def test_both_languages_cover_detail_text(self):
        keys=("back","label","copy_command","copied","copy_failed","log_subtitle",
              "log_footer","log_count","no_logs","live_status","paused_status",
              "switch_live")
        for key in keys:
            self.assertGreaterEqual(I18N.count(f"'detail.{key}'"),2,key)
        self.assertIn("document.addEventListener('perchflow:language-changed'",JS)
        self.assertIn("renderProcess(lastProcess)",JS)

if __name__=="__main__":
    unittest.main()
