#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Measure a macOS System Settings reference screenshot deterministically.

The art-direction captures under ``docs/reference/macos/`` are local-only and
git-ignored, but the layout numbers we clone from them (window size, sidebar
width, toolbar height, content margins, card padding, row height, colors)
should not be eyeballed. This script derives them from the pixels so the
design-system tokens and the Settings app can be checked against the source.

Only Pillow is required (no numpy).

Usage::

    python3 scripts/measure-settings-reference.py "docs/reference/macos/Screenshot ... PM.png"
    python3 scripts/measure-settings-reference.py 9.21.21 --json

Absolute point values need a display scale, which a capture does not record.
The script therefore reports distances in *traffic-light units* (px / red
button diameter) and converts them with our 12 px
``component.trafficLights.diameter`` token, which is what the layout actually
clones. Ratios and traffic-light units are the scale-independent numbers.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import statistics
import sys
from collections import Counter

from PIL import Image, ImageFilter

# Our design-system traffic-light diameter; macOS's close button is 12 pt, so
# one "unit" is the same physical size in both.
UNIT_PX = 12.0


def find_reference(pattern: str) -> str:
    if os.path.isfile(pattern):
        return pattern
    candidates = glob.glob("docs/reference/macos/*.png")
    matches = [p for p in candidates if pattern in p or pattern in p.replace("\u202f", " ")]
    if len(matches) != 1:
        raise SystemExit(f"reference {pattern!r} matched {len(matches)} files")
    return matches[0]


def window_bounds(px, w: int, h: int):
    def bright(x, y):
        r, g, b = px[x, y]
        return (r + g + b) / 3

    xs = [x for x in range(w) if bright(x, h // 2) > 60]
    ys = [y for y in range(h) if bright(w // 2, y) > 60]
    if not xs or not ys:
        raise SystemExit("could not find the window")
    return xs[0], ys[0], xs[-1], ys[-1]


def traffic_light(px, x0, y0, x1):
    xs, ys = [], []
    for y in range(y0, y0 + 140):
        for x in range(x0, min(x0 + 200, x1)):
            r, g, b = px[x, y]
            if r > 180 and g < 170 and b < 170 and (r - max(g, b)) > 40:
                xs.append(x)
                ys.append(y)
    if not xs:
        return None, None
    return (min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1), max(xs) - min(xs) + 1


def edge_profile(im: Image.Image, box, axis: str):
    region = im.crop(box).convert("L")
    edges = region.filter(ImageFilter.FIND_EDGES)
    w, h = edges.size
    data = edges.tobytes()
    profile = []
    if axis == "x":
        for x in range(w):
            profile.append(sum(1 for y in range(h) if data[y * w + x] > 40))
    else:
        for y in range(h):
            profile.append(sum(1 for v in data[y * w:(y + 1) * w] if v > 40))
    return profile


def peak(profile, lo: int, hi: int, min_v: int = 0):
    lo, hi = max(0, lo), min(len(profile), hi)
    best_i, best_v = -1, min_v
    for i in range(lo, hi):
        if profile[i] > best_v:
            best_i, best_v = i, profile[i]
    return best_i, best_v


def dominant(px, x0, y0, x1, y1):
    counts = Counter()
    for y in range(y0, y1):
        for x in range(x0, x1):
            counts[px[x, y]] += 1
    color, n = counts.most_common(1)[0]
    total = max(1, (x1 - x0) * (y1 - y0))
    return color, round(n / total, 3)


def near(a, b, tol=4):
    return abs(a[0] - b[0]) <= tol and abs(a[1] - b[1]) <= tol and abs(a[2] - b[2]) <= tol


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("reference", help="path or unique substring of a capture")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    path = find_reference(args.reference)
    im = Image.open(path).convert("RGB")
    w, h = im.size
    px = im.load()

    x0, y0, x1, y1 = window_bounds(px, w, h)
    win_w, win_h = x1 - x0 + 1, y1 - y0 + 1
    red, diameter = traffic_light(px, x0, y0, x1)
    unit = (diameter or 24) / UNIT_PX  # px per one 12-unit (≈2.3 at Retina)

    def units(px_len):
        return round(px_len / unit, 1)

    # Sidebar/content separator: strongest interior column edge, skipping the
    # window border and the right half.
    col_edges = edge_profile(im, (x0, y0, x1 + 1, y1 + 1), "x")
    sidebar_px, _ = peak(col_edges, int(win_w * 0.18), int(win_w * 0.45))
    sidebar_w = units(sidebar_px)

    # Toolbar bottom: strongest interior row edge in the top band of the
    # window, below the traffic-light row.
    row_edges = edge_profile(im, (x0, y0, x1 + 1, y1 + 1), "y")
    toolbar_px, _ = peak(row_edges, int(win_h * 0.035), int(win_h * 0.13))
    toolbar_h = units(toolbar_px)

    content_x0 = x0 + sidebar_px

    # Card geometry: sample a patch inside the first card, then measure the
    # run of that color across a row and the separators down a column.
    card_y = y0 + toolbar_px + int((win_h - toolbar_px) * 0.12)
    card_color, card_share = dominant(px, content_x0 + int(win_w * 0.30), card_y,
                                      content_x0 + int(win_w * 0.34), card_y + 8)
    row = [px[x, card_y] for x in range(content_x0 + 2, x1)]
    runs, start = [], None
    for i, c in enumerate(row):
        match = near(c, card_color)
        if match and start is None:
            start = i
        if not match and start is not None:
            if i - start > 30:
                runs.append((start, i - 1))
            start = None
    if start is not None and len(row) - start > 30:
        runs.append((start, len(row) - 1))
    card_run = max(runs, key=lambda r: r[1] - r[0]) if runs else None
    card = None
    if card_run:
        left = content_x0 + 2 + card_run[0]
        right = content_x0 + 2 + card_run[1]
        card = {
            "left_margin_px": left - content_x0,
            "right_margin_px": x1 - right,
            "left_margin_units": units(left - content_x0),
            "right_margin_units": units(x1 - right),
            "width_units": units(right - left + 1),
        }

    # Row height: separators are rows inside the card that differ from the card
    # fill along the card's horizontal centre.
    rows = None
    if card_run:
        cx = (content_x0 + 2 + card_run[0] + content_x0 + 2 + card_run[1]) // 2
        col = [px[cx, y] for y in range(y0 + toolbar_px, y1)]
        sep = [i for i, c in enumerate(col) if not near(c, card_color, 3)
               and sum(c) > 3 * 180]
        if len(sep) >= 2:
            gaps = [b - a for a, b in zip(sep, sep[1:]) if 20 < b - a < 200]
            if gaps:
                rows = {
                    "row_height_px_median": statistics.median(gaps),
                    "row_height_units": units(statistics.median(gaps)),
                }

    colors = {
        "sidebar_bg": list(dominant(px, x0 + 8, y0 + int(win_h * 0.5),
                                    x0 + sidebar_px - 8, y1 - 8)[0]),
        "content_bg": list(dominant(px, x1 - int(win_w * 0.08), y0 + int(win_h * 0.5),
                                    x1 - 8, y1 - 8)[0]),
        "toolbar_band": list(dominant(px, content_x0 + 20, y0 + 4, x1 - 20,
                                      y0 + max(8, toolbar_px - 4))[0]),
        "card_bg": list(card_color),
    }

    report = {
        "file": os.path.basename(path),
        "image_px": [w, h],
        "window_px": {"w": win_w, "h": win_h},
        "traffic_light": {
            "bbox_px": red, "diameter_px": diameter,
            "px_per_unit": round(unit, 3),
            "implied_display_scale": round(unit / 1.0, 2),
        },
        "window_units": {"w": units(win_w), "h": units(win_h)},
        "sidebar_units": sidebar_w,
        "toolbar_units": toolbar_h,
        "card": card,
        "row": rows,
        "colors": colors,
        "ratio": {
            "sidebar_over_window": round(sidebar_px / win_w, 3),
            "toolbar_over_window": round(toolbar_px / win_h, 3),
        },
    }

    if args.json:
        print(json.dumps(report, indent=2))
        return 0

    print(f"# {report['file']}  ({w}x{h} px)")
    print(f"window        {win_w}x{win_h} px  = {units(win_w)}x{units(win_h)} units")
    print(f"traffic light {diameter}px -> {unit:.2f} px/unit (12-unit token)")
    print(f"sidebar       {sidebar_px} px = {sidebar_w} units "
          f"({report['ratio']['sidebar_over_window']*100:.0f}% of window)")
    print(f"toolbar       {toolbar_px} px = {toolbar_h} units "
          f"({report['ratio']['toolbar_over_window']*100:.0f}% of window)")
    if card:
        print(f"content card  left {card['left_margin_units']} / right "
              f"{card['right_margin_units']} / width {card['width_units']} units")
    if rows:
        print(f"row height    {rows['row_height_px_median']:.0f} px = "
              f"{rows['row_height_units']} units")
    print(f"colors        sidebar {colors['sidebar_bg']}  content "
          f"{colors['content_bg']}  card {colors['card_bg']}  toolbar "
          f"{colors['toolbar_band']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())