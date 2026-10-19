"""Check nav-tab geometry across active/inactive states in Edge.

The selected nav link must never change the position of either label.
Usage:
    python dev/verify_nav_stability.py --baseline
    python dev/verify_nav_stability.py
"""
from __future__ import annotations

import argparse
from pathlib import Path
from playwright.sync_api import sync_playwright

BASE = "http://127.0.0.1:9011"
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
ROOT = Path(__file__).resolve().parents[1]


def measure(page):
    return page.evaluate("""() => {
      const tabs = [...document.querySelectorAll('nav')].find(n => n.querySelector('a[href="/processes"]') && n.querySelector('a[href="/tasks"]') && n.getBoundingClientRect().top < 100);
      if (!tabs) throw new Error('Desktop nav not found');
      const result={};
      for (const path of ['/processes','/tasks']) {
        const link = tabs.querySelector('a[href="'+path+'"]');
        const linkRect = link.getBoundingClientRect();
        const selection = document.createRange();
        selection.selectNodeContents(link);
        const textRect = selection.getBoundingClientRect();
        const style=getComputedStyle(link);
        result[path]={
          box:[linkRect.x,linkRect.y,linkRect.width,linkRect.height],
          text:[textRect.x,textRect.y,textRect.width,textRect.height],
          weight:style.fontWeight,
          padding:style.padding,
          margin:style.margin,
          current:link.getAttribute('aria-current'),
        };
      }
      return result;
    }""")


def run(baseline: bool):
    with sync_playwright() as pw:
        browser = pw.chromium.launch(headless=True, executable_path=EDGE)
        page = browser.new_page(viewport={"width": 1440, "height": 900}, device_scale_factor=1)
        measurements = {}
        for route in ("/processes", "/tasks"):
            page.goto(BASE + route, wait_until="load")
            page.locator('nav a[href="/processes"]').first.wait_for()
            measurements[route] = measure(page)
        for route, values in measurements.items():
            print(route, values, flush=True)
        if not baseline:
            for label in ("/processes", "/tasks"):
                a = measurements["/processes"][label]
                b = measurements["/tasks"][label]
                for k in ("box", "text"):
                    differences = [abs(x-y) for x, y in zip(a[k], b[k])]
                    assert max(differences) < 0.65, f"{label} {k} MOVED {differences}"
                assert a["weight"] == b["weight"], f"{label}: font weight causes visual displacement"
            assert measurements["/processes"]["/processes"]["current"] == "page"
            assert measurements["/tasks"]["/tasks"]["current"] == "page"
            assert measurements["/processes"]["/tasks"]["current"] is None
            assert measurements["/tasks"]["/processes"]["current"] is None
            print("NAV_POSITION_STABLE=PASS")
            page.goto(BASE + "/processes")
            page.locator('nav a[href="/tasks"]').first.click()
            page.wait_for_url("**/tasks")
            page.locator('nav a[href="/processes"]').first.click()
            page.wait_for_url("**/processes")
            print("NAV_CLICK_SWITCH=PASS")
            for width in (390, 768, 1440):
                page.set_viewport_size({"width": width, "height": 850})
                page.goto(BASE + "/processes")
                assert page.evaluate("document.documentElement.scrollWidth <= window.innerWidth+2")
                if width >= 768:
                    assert page.locator('nav .pf-main-nav-link').first.is_visible()
                print(f"VIEWPORT={width}=PASS")
        else:
            print("BASELINE_RECORD_COMPLETE")
        browser.close()


if __name__ == "__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--baseline",action="store_true")
    run(parser.parse_args().baseline)
