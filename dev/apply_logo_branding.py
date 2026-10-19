#!/usr/bin/env python3
"""Apply approved transparent PerchFlow emblem across the existing HTML pages."""
from __future__ import annotations
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "frontend"
PAGES = ["login.html", "processes.html", "process-detail.html", "tasks.html"]

FAVICON_ASSETS = (
    "logo.ico",
    "favicon-96x96.png",
    "favicon.svg",
    "favicon.ico",
    "apple-touch-icon.png",
    "site.webmanifest",
)
VERSION_QUERY = "brand2"


def update_page(path: Path) -> None:
    html = path.read_text(encoding="utf-8")
    html = html.replace("Perch -", "PerchFlow -")
    html = html.replace('content="Perch -', 'content="PerchFlow -')
    html = html.replace('content="Perch"', 'content="PerchFlow"')
    html = html.replace('alt="Perch"', 'alt="PerchFlow"')
    html = html.replace(
        'content="https://github.com/tanzibiao/perch"',
        'content="https://github.com/buyuuz/PerchFlow"')

    html = html.replace('src="/assets/logo.png"',
                        'src="/assets/logo.png?v=' + VERSION_QUERY + '"')
    html = html.replace('content="/assets/logo.png"',
                        'content="/assets/logo.png?v=' + VERSION_QUERY + '"')
    for name in FAVICON_ASSETS:
        html = html.replace('href="/assets/' + name + '"',
                            'href="/assets/' + name + '?v=' + VERSION_QUERY + '"')

    if path.name == "login.html":
        html = html.replace(
            'class="w-12 h-12 rounded-full border border-primary/30 mb-4 object-cover bg-primary/10"',
            'class="w-20 h-20 mb-4 object-contain"')
        html = html.replace(
            '>Perch</h1>',
            '>Perch<span class="text-primary">Flow</span></h1>')
    elif path.name in ("processes.html", "tasks.html"):
        html = html.replace(
            'class="w-6 h-6 rounded-md object-cover"',
            'class="w-7 h-7 object-contain"')
        html = html.replace(
            '\n                Perch\n',
            '\n                Perch<span class="text-on-surface">Flow</span>\n')
    path.write_text(html, encoding="utf-8", newline="\n")
    print("Updated", path.name)


def update_manifest() -> None:
    path = WEB / "assets" / "site.webmanifest"
    manifest = json.loads(path.read_text(encoding="utf-8"))
    manifest["name"] = "PerchFlow"
    manifest["short_name"] = "PerchFlow"
    manifest["theme_color"] = "#0c1424"
    manifest["background_color"] = "#0c1424"
    for icon in manifest["icons"]:
        icon["src"] = "/assets/" + Path(icon["src"]).name
        icon["purpose"] = "any"
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("Updated", path.name)


if __name__ == "__main__":
    for name in PAGES:
        update_page(WEB / name)
    update_manifest()
