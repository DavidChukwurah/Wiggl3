#!/usr/bin/env python3
"""Builds every wiggl3 icon from the Material Symbols glyph "pest_control_rodent"
(assets/pest_control_rodent.svg, Apache License 2.0, Google).

Needs:  pip install cairosvg pillow
Run from the source/ folder:  python3 tools/make_icon.py

There is no hue anywhere: every icon is pale, near-black or grey.

  icon_512.png        app icon: pale glyph on a dark rounded tile (macOS .icns, Linux)
  icon_128.rgba       window icon (raw RGBA, 128 x 128)
  icon.ico            Windows exe icon
  tray_<kind>_<state>_64.rgba   tray / menu bar icons (raw RGBA, 64 x 64), glyph only, transparent background
      kind  light    pale glyph, for dark panels (Windows dark taskbar)
            dark     near-black glyph, for light panels (Windows light taskbar)
            mid      pale glyph with a thin dark outline, reads on any panel (Linux, panel colour unknown)
            template black glyph, alpha only (macOS recolours it for light / dark menu bars)
      state on       solid glyph
            off      faded glyph with a slash through it
"""
import io
import os
import re

import cairosvg
from PIL import Image, ImageChops, ImageDraw, ImageFilter

here = os.path.dirname(os.path.abspath(__file__))
assets = os.path.join(here, "..", "assets")
SVG = open(os.path.join(assets, "pest_control_rodent.svg"), encoding="utf-8").read()
SVG_FILL = open(os.path.join(assets, "pest_control_rodent-fill.svg"), encoding="utf-8").read()

PALE, INK = "#F5F5F7", "#1D1D1F"
TILE = (30, 30, 32, 255)
# Tight bounds of the drawn glyph inside the 960 x 960 box: x, y, w, h (measured; see check at the bottom).
VB = (80, -840, 800, 720)
VB_FILL = VB  # the filled variant has the same bounds


def glyph(color, width_px, src=None, vb=None):
    """The glyph alone, cropped tight to its shape, on a transparent background."""
    x, y, w, h = vb or VB
    svg = re.sub(r'viewBox="[^"]*"', f'viewBox="{x} {y} {w} {h}"', src or SVG)
    svg = re.sub(r'\s(width|height)="\d+"', "", svg)
    svg = svg.replace("<path ", f'<path fill="{color}" ')
    png = cairosvg.svg2png(bytestring=svg.encode(), output_width=width_px, output_height=round(width_px * h / w))
    return Image.open(io.BytesIO(png)).convert("RGBA")


def app_icon(size=2048):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    tile = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(tile).rounded_rectangle([0, 0, size - 1, size - 1], radius=int(size * 0.225), fill=TILE)
    img.alpha_composite(tile)
    g = glyph(PALE, int(size * 0.60))
    img.alpha_composite(g, ((size - g.width) // 2, (size - g.height) // 2))
    return img


def fit(g, size, margin):
    box = size - 2 * margin
    scale = min(box / g.width, box / g.height)
    return g.resize((max(1, round(g.width * scale)), max(1, round(g.height * scale))), Image.LANCZOS)


def tray(color, state, size=64, margin=3, outline=None):
    """Glyph only. 'on' = solid. 'off' = faded, with a diagonal slash cut through it."""
    big = 8  # draw at 8x then shrink, for smooth edges
    s = size * big
    base = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    g = fit(glyph(color, s, SVG_FILL, VB_FILL), s, margin * big)
    pos = ((s - g.width) // 2, (s - g.height) // 2)
    layer = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    layer.alpha_composite(g, pos)
    if outline:
        alpha = layer.getchannel("A").filter(ImageFilter.MaxFilter(2 * big + 1))
        ring = Image.new("RGBA", (s, s), outline)
        ring.putalpha(alpha)
        base.alpha_composite(ring)
    base.alpha_composite(layer)
    if state == "off":
        r, gc, b, a = base.split()
        base = Image.merge("RGBA", (r, gc, b, a.point(lambda v: int(v * 0.5))))
        a0, a1 = (s * 0.12, s * 0.88), (s * 0.88, s * 0.12)
        cut = Image.new("L", (s, s), 255)
        ImageDraw.Draw(cut).line([a0, a1], fill=0, width=int(s * 0.13))
        base.putalpha(ImageChops.multiply(base.getchannel("A"), cut))
        line = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        stroke = (29, 29, 31, 255) if outline else Image.new("RGBA", (1, 1), color).getpixel((0, 0))
        ImageDraw.Draw(line).line([a0, a1], fill=stroke, width=int(s * 0.055))
        base.alpha_composite(line)
    return base.resize((size, size), Image.LANCZOS)


def write(name, img):
    with open(os.path.join(assets, name), "wb") as f:
        f.write(img.tobytes())


def main():
    app = app_icon()
    app.resize((512, 512), Image.LANCZOS).save(os.path.join(assets, "icon_512.png"))
    write("icon_128.rgba", app.resize((128, 128), Image.LANCZOS))
    app.resize((256, 256), Image.LANCZOS).save(
        os.path.join(assets, "icon.ico"),
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    for state in ("on", "off"):
        write(f"tray_light_{state}_64.rgba", tray(PALE, state))
        write(f"tray_dark_{state}_64.rgba", tray(INK, state))
        write(f"tray_mid_{state}_64.rgba", tray(PALE, state, outline=(29, 29, 31, 235)))
        write(f"tray_template_{state}_64.rgba", tray("#000000", state))
    print("icons written")


if __name__ == "__main__":
    main()
