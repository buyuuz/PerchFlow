#!/usr/bin/env python3
"""Regenerate PerchFlow browser branding from the approved transparent emblem.

Requires Pillow on a development machine. Generated files are committed to Git,
so the final Rust/Windows build does not require Python or Pillow.
"""
from __future__ import annotations

import base64
import io
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "frontend" / "assets"
SOURCE = ASSETS / "perchflow-emblem.webp"


def main() -> None:
    source = Image.open(SOURCE).convert("RGBA")
    if source.getchannel("A").getextrema() != (0, 255):
        raise ValueError("The approved emblem must contain true alpha transparency")

    sizes = {
        "logo.png": 256,
        "favicon-96x96.png": 96,
        "apple-touch-icon.png": 180,
        "web-app-manifest-192x192.png": 192,
        "web-app-manifest-512x512.png": 512,
    }
    for filename, size in sizes.items():
        icon = source.resize((size, size), Image.Resampling.LANCZOS)
        icon.save(ASSETS / filename, "PNG", optimize=True)
        print(f"Generated {filename}: {size}x{size} RGBA")

    for filename in ("logo.ico", "favicon.ico"):
        source.resize((256, 256), Image.Resampling.LANCZOS).save(
            ASSETS / filename, format="ICO",
            sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
        print(f"Generated {filename}: multi-size ICO")

    thumb = source.resize((128, 128), Image.Resampling.LANCZOS)
    buffer = io.BytesIO()
    thumb.save(buffer, "PNG", optimize=True)
    png_data = base64.b64encode(buffer.getvalue()).decode("ascii")
    favicon_svg = (
        '<svg xmlns="http://www.w3.org/2000/svg" '
        'xmlns:xlink="http://www.w3.org/1999/xlink" '
        'viewBox="0 0 128 128" width="128" height="128">'
        '<image width="128" height="128" '
        'href="data:image/png;base64,' + png_data + '" />'
        '</svg>'
    )
    (ASSETS / "favicon.svg").write_text(favicon_svg, encoding="utf-8")
    print("Generated favicon.svg with embedded transparent PNG")


if __name__ == "__main__":
    main()
