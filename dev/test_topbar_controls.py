"""Regression checks for shared Process/Task top-right controls.

Standard-library-only; GitHub Actions can run these without a browser.
For actual geometry, keyboard and CRUD checks use verify_topbar_controls_browser.py.
"""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[1]
WEB=ROOT/"frontend"
CSS=(WEB/"assets"/"topbar-controls.css").read_text(encoding="utf-8")
JS=(WEB/"assets"/"topbar-controls.js").read_text(encoding="utf-8")
I18N=(WEB/"assets"/"i18n.js").read_text(encoding="utf-8")


class TopbarControlsSourceTests(unittest.TestCase):
    def test_both_pages_share_the_same_controls(self):
        for name in ("processes.html","tasks.html"):
            with self.subTest(page=name):
                html=(WEB/name).read_text(encoding="utf-8")
                self.assertIn('class="pf-process-nav-right pf-top-actions"',html)
                self.assertIn('class="pf-top-status" role="status"',html)
                self.assertIn('class="pf-process-add pf-top-add"',html)
                self.assertIn('class="pf-lang-picker" data-pf-language-picker',html)
                self.assertIn('data-lang-trigger',html)
                self.assertIn('data-lang-menu role="listbox"',html)
                self.assertIn('data-lang-select',html)
                self.assertIn('/assets/topbar-controls.css?v=top2',html)
                self.assertIn('/assets/topbar-controls.js?v=top2',html)
                for lang in ("zh","en"):
                    self.assertIn(f'data-lang-value="{lang}"',html)
                for lang in ("es","ja","pt"):
                    self.assertNotIn(f'data-lang-value="{lang}"',html)
                    self.assertNotIn(f'<option value="{lang}">',html)
                self.assertIn('class="pf-top-add-icon"',html)
                self.assertIn('class="pf-top-add-text"',html)

    def test_only_two_languages_across_application(self):
        self.assertIn("const LANGS = ['zh', 'en']", I18N)
        self.assertIn("LANGS.includes(stored)", I18N)
        self.assertIn("if (!LANGS.includes(lang)) return", I18N)
        for page_name in ("processes.html", "tasks.html", "login.html", "process-detail.html"):
            page=(WEB/page_name).read_text(encoding="utf-8")
            self.assertIn('<option value="zh">', page)
            self.assertIn('<option value="en">', page)
            for obsolete in ("es", "ja", "pt"):
                self.assertNotIn(f'<option value="{obsolete}">', page)

    def test_sizes_and_open_menu_stay_consistent(self):
        self.assertIn(".pf-top-actions", CSS)
        self.assertIn(".pf-top-status", CSS)
        self.assertIn(".pf-process-topnav .pf-top-add", CSS)
        self.assertIn(".pf-lang-trigger", CSS)
        self.assertIn(".pf-lang-menu", CSS)
        self.assertIn("height: 32px;", CSS)
        self.assertIn("font-size: 12px;", CSS)
        self.assertIn("border-radius: 8px;", CSS)
        self.assertIn(".pf-top-add-icon::before", CSS)
        self.assertIn(".pf-top-add-icon::after", CSS)
        self.assertIn("translate(-50%, -50%)", CSS)
        self.assertIn(".pf-lang-menu[hidden]", CSS)
        self.assertIn("position: absolute;", CSS)
        self.assertIn("@media (max-width: 767px)", CSS)

    def test_language_picker_keeps_select_event_contract(self):
        self.assertIn("picker.classList.add('is-enhanced')", JS)
        self.assertIn("native.dispatchEvent(new Event('change', {bubbles: true}))",JS)
        self.assertIn("perchflow:language-changed",JS)
        self.assertIn("perchflow:language-changed",I18N)
        self.assertIn("aria-selected",JS)
        self.assertIn("aria-expanded",JS)
        self.assertIn("event.key === 'Escape'",JS)
        self.assertIn("event.key === 'ArrowDown'",JS)
        self.assertIn("document.addEventListener('pointerdown'",JS)


if __name__=="__main__":
    unittest.main()
