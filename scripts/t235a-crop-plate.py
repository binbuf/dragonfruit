#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Crop a T235a Dock plate capture into an edge strip and corner close-ups.

The T235a capture is the nested compositor's own render, so the plate is
somewhere inside a full-screen still. The previous version cropped the first
and last 360 px of the *strip*, which is usually wallpaper, not the plate's
rounded corners. This helper finds the plate's specular rim (a long bright run
that does not span the whole output, unlike a wallpaper grid line) and writes
a close-up of the actual corners.

Usage:
    t235a-crop-plate.py --capture raw.png --position bottom --scale 1 \
        --out-strip strip.png --out-corners corners.png

Exit status 2 means the rim was not found; the strip is still written so the
report can record the corner row as OPEN rather than inventing evidence.
"""
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image

STRIP = 200
CORNER = 120
CORNER_PAD = 18
STRIP_TARGET = 900
CORNER_TARGET = 720
BRIGHT = 110
MIN_RUN_FRACTION = 0.15
MAX_RUN_FRACTION = 0.80


def luma(pixel: tuple[int, int, int]) -> float:
    return (pixel[0] + pixel[1] + pixel[2]) / 3.0


def rim_threshold(im: Image.Image) -> float:
    """A brightness threshold that isolates the specular rim on any backdrop.

    A fixed threshold fails on a mid-gray field: the whole wallpaper row is
    already "bright", so the plate's inset rim cannot be told apart from the
    full-width background. The rim is a specular line above the local field,
    so the threshold follows the image median with a margin, capped below
    white. A dark backdrop keeps the original floor.
    """
    histogram = im.convert("L").histogram()
    total = sum(histogram)
    cumulative = 0
    median = 0
    for level, count in enumerate(histogram):
        cumulative += count
        if cumulative * 2 >= total:
            median = level
            break
    return float(max(BRIGHT, min(235, median + 24)))


def longest_run(values: list[float], threshold: float) -> tuple[int, int]:
    """Return the (start, end) of the longest run at or above threshold."""
    best = (0, 0)
    start = None
    for index, value in enumerate(values):
        if value >= threshold:
            if start is None:
                start = index
        elif start is not None:
            if index - start > best[1] - best[0]:
                best = (start, index - 1)
            start = None
    if start is not None and len(values) - start > best[1] - best[0]:
        best = (start, len(values) - 1)
    return best


def find_plate(im: Image.Image, position: str, scale: float) -> tuple[int, int, int, int] | None:
    """Return the plate rect (left, top, right, bottom) from its bright rim.

    A bottom Dock's rim is a long horizontal bright run; a left/right Dock's
    rim is a long vertical bright run. Wallpaper grid/stripe lines can also be
    bright, so a run that spans almost the whole output is rejected: the plate
    is inset from the screen edge and never covers the entire width/height.
    """
    px = im.load()
    width, height = im.size
    threshold = rim_threshold(im)
    if position == "bottom":
        min_run = max(40, int(width * MIN_RUN_FRACTION))
        max_run = int(width * MAX_RUN_FRACTION)
        rows = []
        # The plate is at the bottom edge; the menu bar and most wallpaper
        # lines are above this band. A horizontal grid line still spans more
        # than `MAX_RUN_FRACTION` of the output and is rejected below.
        for y in range(height // 2, height):
            row = [luma(px[x, y]) for x in range(width)]
            start, end = longest_run(row, threshold)
            length = end - start + 1
            # A wallpaper grid/stripe line runs to the output edge; the plate
            # is inset from the left and right edges, so its rim never does.
            if start <= 2 or end >= width - 3:
                continue
            if min_run <= length <= max_run:
                rows.append((y, start, end))
        if not rows:
            return None
        return (
            min(row[1] for row in rows),
            min(row[0] for row in rows),
            max(row[2] for row in rows),
            max(row[0] for row in rows),
        )
    min_run = max(40, int(height * MIN_RUN_FRACTION))
    max_run = int(height * MAX_RUN_FRACTION)
    if position == "left":
        x_start, x_end = 0, max(1, width // 4)
    else:
        x_start, x_end = min(width - 1, width * 3 // 4), width
    columns = []
    for x in range(x_start, x_end):
        column = [luma(px[x, y]) for y in range(height)]
        start, end = longest_run(column, threshold)
        length = end - start + 1
        # A wallpaper grid/stripe line runs to the output edge; the plate is
        # inset from the top and bottom edges, so its rim never does.
        if start <= 2 or end >= height - 3:
            continue
        if min_run <= length <= max_run:
            columns.append((x, start, end))
    if not columns:
        return None
    return (
        min(column[0] for column in columns),
        min(column[1] for column in columns),
        max(column[0] for column in columns),
        max(column[2] for column in columns),
    )


def upscale(crop: Image.Image, target: int) -> Image.Image:
    factor = max(1, target // max(1, min(crop.size)))
    return crop.resize((crop.width * factor, crop.height * factor), Image.NEAREST)


def corner_windows(plate: tuple[int, int, int, int], position: str) -> list[tuple[int, int, int, int]]:
    left, top, right, bottom = plate
    pad = CORNER_PAD
    if position == "bottom":
        # The screen-edge corners are clipped by the output; the visible
        # rounded corners are the top-left and top-right pair.
        return [
            (left - pad, top - pad, left + CORNER, top + CORNER),
            (right - CORNER, top - pad, right + pad, top + CORNER),
        ]
    # A vertical plate's visible rounded corners are its top and bottom ends.
    return [
        (left - pad, top - pad, left + CORNER, top + CORNER),
        (left - pad, bottom - CORNER, left + CORNER, bottom + pad),
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--capture", required=True)
    parser.add_argument("--position", choices=("bottom", "left", "right"), required=True)
    parser.add_argument("--scale", type=float, default=1.0)
    parser.add_argument("--out-strip", required=True)
    parser.add_argument("--out-corners", required=True)
    args = parser.parse_args()

    capture = Path(args.capture)
    im = Image.open(capture).convert("RGB")
    width, height = im.size

    strip_size = min(width, height, int(STRIP * args.scale))
    if args.position == "bottom":
        strip_box = (0, max(0, height - strip_size), width, height)
    elif args.position == "left":
        strip_box = (0, 0, strip_size, height)
    else:
        strip_box = (max(0, width - strip_size), 0, width, height)
    strip = upscale(im.crop(strip_box), STRIP_TARGET)
    strip.save(args.out_strip)

    plate = find_plate(im, args.position, args.scale)
    if plate is None:
        print("capture-t235a: plate rim not found; corner crop OPEN", flush=True)
        return 2

    windows = corner_windows(plate, args.position)
    crops = []
    for box in windows:
        box = (max(0, box[0]), max(0, box[1]),
               min(width, box[2]), min(height, box[3]))
        crops.append(im.crop(box))
    if args.position == "bottom":
        gap = 12
        out = Image.new("RGB", (sum(crop.width for crop in crops) + gap,
                                max(crop.height for crop in crops)), (24, 24, 24))
        x = 0
        for crop in crops:
            out.paste(crop, (x, 0))
            x += crop.width + gap
    else:
        gap = 12
        out = Image.new("RGB", (max(crop.width for crop in crops),
                                sum(crop.height for crop in crops) + gap), (24, 24, 24))
        y = 0
        for crop in crops:
            out.paste(crop, (0, y))
            y += crop.height + gap
    out = upscale(out, CORNER_TARGET)
    out.save(args.out_corners)
    print(
        "capture-t235a: plate "
        f"({plate[0]},{plate[1]})-({plate[2]},{plate[3]}) "
        f"corners {out.size[0]}x{out.size[1]}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
