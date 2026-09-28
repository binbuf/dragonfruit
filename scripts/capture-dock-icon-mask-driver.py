#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-14.7w Dock icon squircle-mask stills.

Driven by ``scripts/capture-dock-icon-mask.sh`` (run that, not this).
Requires a host Wayland session, ``spectacle``, and Pillow.

The scratch settingsd pins three apps whose themed icons are a full-bleed
square (red), a padded square (green), and a circle (blue). For each colour
scheme this parks the pointer off the Dock, detects the resting plate in a full
screenshot, and writes a zoomed crop of the plate so the three tiles can be
compared: the square must read as a rounded tile with transparent corners, the
circle must reach the tile edge (no double inset), and the padded square keeps
its own padding.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
# The active Space wallpaper the compositor clears the nested output with
# (under the dark scheme the capture starts in).
WALL = (33, 13, 41)
WALL_TOL = 3
WALL_ROW_MIN = 200

# The resting Dock is bottom-centred in the nested output, so the search box is
# a fixed band around it (nested-local) rather than a threshold detection,
# which the wallpaper defeats. The three capture icons are then located by
# their distinctive colours so the still frames only them.
CROP_HALF_W = 500
CROP_BOTTOM = 5
CROP_HEIGHT = 200
# The three scratch icon fill colours.
COLORS = {
    "square": (226, 59, 59),
    "padded": (23, 163, 74),
    "round": (43, 108, 255),
}
ICON_MARGIN = 8
SCALE = 4


def log(message):
    print(f"capture-dock-icon-mask: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-iconmask-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(2.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def motion(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")


def full(outdir, name):
    dest = os.path.join(outdir, name + ".png")
    subprocess.run(
        ["spectacle", "-b", "-n", "-f", "-o", dest],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return dest


def detect_rect(path):
    im = Image.open(path).convert("RGB")
    px = im.load()
    w, h = im.size

    def matches(c):
        return all(abs(c[i] - WALL[i]) <= WALL_TOL for i in range(3))

    rows = []
    for y in range(0, h, 2):
        xs = [x for x in range(0, w, 4) if matches(px[x, y])]
        if len(xs) >= WALL_ROW_MIN:
            rows.append((y, xs[0], xs[-1]))
    if not rows:
        raise RuntimeError("nested window not found (no wallpaper pixels)")
    return (min(row[1] for row in rows), rows[0][0])


def plate_box(rect):
    """The fixed resting-Dock search box in full-screenshot coordinates."""
    rx, ry = rect
    cx = rx + NESTED_W // 2
    return (cx - CROP_HALF_W, ry + NESTED_H - CROP_HEIGHT,
            cx + CROP_HALF_W, ry + NESTED_H - CROP_BOTTOM)


def color_bbox(im, rgb, x0=0, x1=None, tol=30):
    px = im.load()
    if x1 is None:
        x1 = im.width
    xs = []
    ys = []
    for y in range(im.height):
        for x in range(x0, x1):
            if all(abs(px[x, y][i] - rgb[i]) <= tol for i in range(3)):
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return (min(xs), min(ys), max(xs), max(ys))


def icon_span(im):
    """The union bbox of the three adjacent test icons, or fail.

    The square, padded, and round apps are pinned first, so each is found in
    the narrow x-window just to the right of the previous one. That keeps a
    later same-coloured icon (e.g. a blue folder) out of the frame.
    """
    span = 90  # one tile + gap, with slack
    boxes = []
    red = color_bbox(im, COLORS["square"])
    if red is None:
        raise RuntimeError("the square test icon is missing from the Dock")
    boxes.append(red)
    green = color_bbox(im, COLORS["padded"], x0=red[2], x1=red[2] + span)
    if green is None:
        raise RuntimeError("the padded test icon is missing from the Dock")
    boxes.append(green)
    blue = color_bbox(im, COLORS["round"], x0=green[2], x1=green[2] + span)
    if blue is None:
        raise RuntimeError("the round test icon is missing from the Dock")
    boxes.append(blue)
    left = min(b[0] for b in boxes)
    top = min(b[1] for b in boxes)
    right = max(b[2] for b in boxes)
    bottom = max(b[3] for b in boxes)
    return (left, top, right, bottom)


def run(args):
    synth = Synthetic(args.synth)
    synth.send("set color-scheme dark")
    time.sleep(0.3)

    init = full(args.scratch, "initial")
    rect = None
    for attempt in range(40):
        try:
            rect = detect_rect(init)
            break
        except RuntimeError:
            if attempt == 39:
                raise
            time.sleep(0.5)
            init = full(args.scratch, "initial")
    log(f"nested window rect {rect}")

    for scheme in ("dark", "light"):
        synth.send(f"set color-scheme {scheme}")
        time.sleep(0.5)
        # Pointer off the Dock so the plate is at its baseline.
        synth.motion(NESTED_W // 2, 20)
        time.sleep(0.9)
        still = full(args.scratch, f"rest-{scheme}")
        search = Image.open(still).convert("RGB").crop(plate_box(rect))
        search.save(os.path.join(args.scratch, f"plate-{scheme}.png"))
        left, top, right, bottom = icon_span(search)
        log(f"{scheme}: icons at {left},{top}..{right},{bottom}")
        crop = search.crop((max(0, left - ICON_MARGIN), max(0, top - ICON_MARGIN),
                            min(search.width, right + ICON_MARGIN),
                            min(search.height, bottom + ICON_MARGIN)))
        crop = crop.resize((crop.width * SCALE, crop.height * SCALE), Image.NEAREST)
        dest = os.path.join(args.outdir, f"t14-dock-icon-mask-{scheme}.png")
        os.makedirs(args.outdir, exist_ok=True)
        crop.save(dest)
        log(f"saved {dest} ({crop.width}x{crop.height})")

    log("done")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--demo-log", required=True)
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        if args.demo_log and os.path.exists(args.demo_log):
            with open(args.demo_log) as handle:
                tail = handle.readlines()[-30:]
            log("demo log tail:\n" + "".join(tail))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())