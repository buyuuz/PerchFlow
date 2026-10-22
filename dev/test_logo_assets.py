"""Checks branding sources without requiring Pillow in GitHub Actions."""
import json
import re
import struct
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FRONTEND = ROOT / "frontend"
ASSETS = FRONTEND / "assets"


class BrandAssetsTest(unittest.TestCase):
    def test_png_assets_are_rgba(self):
        for name, size in (
            ("logo.png", 256),
            ("favicon-96x96.png", 96),
            ("apple-touch-icon.png", 180),
            ("web-app-manifest-192x192.png", 192),
            ("web-app-manifest-512x512.png", 512),
        ):
            with self.subTest(name=name):
                data = (ASSETS / name).read_bytes()
                self.assertTrue(data.startswith(b"\x89PNG\r\n\x1a\n"))
                width, height, depth, color_type = struct.unpack(">IIBB", data[16:26])
                self.assertEqual((width, height), (size, size))
                self.assertEqual(depth, 8)
                self.assertEqual(color_type, 6)  # True RGBA, not white-backed RGB

    def test_browser_icons_have_correct_magic(self):
        for name in ("favicon.ico", "logo.ico"):
            self.assertTrue((ASSETS / name).read_bytes().startswith(b"\x00\x00\x01\x00"))
        svg = (ASSETS / "favicon.svg").read_text(encoding="utf-8")
        self.assertIn("<svg", svg)
        self.assertIn("data:image/png;base64,", svg)

    def test_pages_use_new_brand_everywhere(self):
        for name in ("login.html", "processes.html", "process-detail.html", "tasks.html"):
            with self.subTest(page=name):
                text = (FRONTEND / name).read_text(encoding="utf-8")
                self.assertIn("PerchFlow -", text)
                if name != "process-detail.html":
                    self.assertIn('alt="PerchFlow"', text)
                self.assertIn("/assets/logo.png?v=brand2", text)
                self.assertIn("/assets/favicon.svg?v=brand2", text)
                self.assertIn("/assets/favicon.ico?v=brand2", text)
                self.assertIn('content="PerchFlow"', text)
                self.assertNotIn("https://github.com/tanzibiao/perch", text)
        login = (FRONTEND / "login.html").read_text(encoding="utf-8")
        self.assertIn('>Perch<span class="text-primary">Flow</span></h1>', login)
        for name in ("processes.html", "tasks.html"):
            text = (FRONTEND / name).read_text(encoding="utf-8")
            self.assertIn('Perch<span class="text-on-surface">Flow</span>', text)

    def test_branding_has_explicit_fixed_icon_dimensions(self):
        # The prebuilt Tailwind bundle does not guarantee the newly added w-7
        # and h-7 utilities; a missing rule previously made the logo huge.
        css = (ASSETS / "branding.css").read_text(encoding="utf-8")
        self.assertIn(".pf-header-brand img.pf-brand-icon", css)
        self.assertIn("width: 28px;", css)
        self.assertIn("height: 28px;", css)
        self.assertIn("max-width: 28px;", css)
        self.assertIn("max-height: 28px;", css)
        self.assertIn(".pf-login-brand img.pf-login-icon", css)
        self.assertIn("width: 80px;", css)
        self.assertIn("height: 80px;", css)
        for page_name in ("processes.html", "tasks.html", "login.html"):
            html = (FRONTEND / page_name).read_text(encoding="utf-8")
            version = "brand3" if page_name == "login.html" else "brand4"
            self.assertIn(f"/assets/branding.css?v={version}", html)
            if page_name == "login.html":
                self.assertIn('class="pf-login-icon"', html)
            else:
                self.assertIn('class="pf-brand-icon"', html)
                self.assertIn('width="28" height="28"', html)

    def test_task_page_heading_matches_process_page_typography(self):
        css = (ASSETS / "branding.css").read_text(encoding="utf-8")
        html = (FRONTEND / "tasks.html").read_text(encoding="utf-8")
        self.assertIn(".pf-management-page-title", css)
        self.assertIn("font-size: 17px;", css)
        self.assertIn(".pf-management-page-subtitle", css)
        self.assertIn("font-size: 12px;", css)
        self.assertIn('class="pf-management-page-title text-on-surface"', html)
        self.assertIn('class="pf-management-page-subtitle text-on-surface-variant"', html)

    def test_desktop_nav_styles_keep_active_labels_stationary(self):
        css = (ASSETS / "branding.css").read_text(encoding="utf-8")
        self.assertIn(".pf-main-nav-link {", css)
        self.assertIn(".pf-main-nav-link::after", css)
        self.assertIn('.pf-main-nav-link[aria-current="page"]', css)
        self.assertIn("position: absolute;", css)
        self.assertIn("font-weight: 600;", css)
        for route, active in (("processes.html", "/processes"),
                              ("tasks.html", "/tasks")):
            html = (FRONTEND / route).read_text(encoding="utf-8")
            self.assertEqual(html.count('class="pf-main-nav-link"'), 4)
            self.assertIn(f'href="{active}" aria-current="page"', html)
            self.assertIn('/assets/branding.css?v=brand4', html)
            self.assertNotIn('border-b-2 border-primary pb-1 font-bold" data-i18n="nav.', html)
            self.assertNotIn('rounded-t-sm mt-1 mb-1" data-i18n="nav.', html)

    def test_manifest_icons_resolve_under_assets(self):
        data = json.loads((ASSETS / "site.webmanifest").read_text(encoding="utf-8"))
        self.assertEqual(data["name"], "PerchFlow")
        self.assertEqual(data["short_name"], "PerchFlow")
        for entry in data["icons"]:
            self.assertTrue(entry["src"].startswith("/assets/"))
            self.assertTrue((FRONTEND / entry["src"].lstrip("/")).is_file())


if __name__ == "__main__":
    unittest.main()
