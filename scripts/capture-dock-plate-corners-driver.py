#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-14.7z Dock plate corner and frosting-alignment stills.

Driven by ``scripts/capture-dock-plate-corners.sh`` (run that, not this).
Requires a host Wayland session, ``spectacle``, and Pillow.

For each colour scheme it parks the pointer off the Dock, detects the resting
plate's top rim in a full screenshot, and writes a 6x nearest-neighbour close-up
of the top-left and top-right corners side by side. It then parks the pointer
over the centre of the plate and writes a 2x magnified Dock strip so the plate
and the compositor frost can be compared mid-magnification.
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
# (under the dark scheme the capture starts in). The host desktop is a
# different colour, so the nested window is the large region matching this.
WALL = (33, 13, 41)
WALL_TOL = 3
WALL_ROW_MIN = 200

# Broad dock band inside the nested output (nested-local x, y, x2, y2). It
# contains the resting plate and enough room for the magnified row.
DOCK_BAND = (560, 950, 1360, 1200)
# Absolute luma threshold that isolates the bright top rim in each scheme.
RIM_THRESHOLD = {"dark": 430, "light": 560}
# The close-up window around a detected corner.
CORNER_HALF_W = 46
CORNER_TOP = 16
CORNER_BOTTOM = 56
CORNER_SCALE = 6


def log(message):
    print(f"capture-dock-plate-corners: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-corners-{os.getpid()}.sock"
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


class Capturer:
    def __init__(self, outdir, scratch):
        self.outdir = outdir
        self.scratch = scratch
        os.makedirs(outdir, exist_ok=True)
        os.makedirs(scratch, exist_ok=True)
        self.rect = None

    def full(self, name):
        dest = os.path.join(self.scratch, name + ".png")
        subprocess.run(
            ["spectacle", "-b", "-n", "-f", "-o", dest],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return dest

    def detect_rect(self, path):
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
        top = rows[0][0]
        left = min(row[1] for row in rows)
        self.rect = (left, top)
        return self.rect


def detect_plate(full_path, rect, threshold):
    """Locate the resting plate's top rim and horizontal extent.

    Returns ``(top_y, left_x, right_x)`` in nested-local logical coordinates.
    """
    rx, ry = rect
    x0, y0, x1, y1 = DOCK_BAND
    im = Image.open(full_path).convert("RGB")
    sub = im.crop((rx + x0, ry + y0, rx + x1, ry + y1))
    px = sub.load()
    w, h = sub.size
    best = (0, [])
    for y in range(4, min(h, 200)):
        bright = [x for x in range(w) if sum(px[x, y]) > threshold]
        if len(bright) > len(best[1]):
            best = (y, bright)
    ytop, run = best
    if len(run) < 80:
        raise RuntimeError("plate top rim not found")
    lo, hi = run[0] - 60, run[-1] + 60
    xs = []
    for y in range(ytop, min(h, ytop + 80)):
        xs += [x for x in range(max(0, lo), min(w, hi))
               if sum(px[x, y]) > threshold]
    if not xs:
        raise RuntimeError("plate extent not found")
    return (y0 + ytop, x0 + min(xs), x0 + max(xs))


def corner_strip(full_path, rect, plate, scheme, outdir):
    """A 6x close-up of the top-left and top-right corners, side by side."""
    rx, ry = rect
    top, left, right = plate
    im = Image.open(full_path).convert("RGB")
    windows = []
    for cx in (left, right):
        box = (rx + cx - CORNER_HALF_W, ry + top - CORNER_TOP,
               rx + cx + CORNER_HALF_W, ry + top + CORNER_BOTTOM)
        crop = im.crop(box).resize(
            ((CORNER_HALF_W * 2) * CORNER_SCALE,
             (CORNER_TOP + CORNER_BOTTOM) * CORNER_SCALE),
            Image.NEAREST,
        )
        windows.append(crop)
    gap = 12
    out = Image.new("RGB",
                    (windows[0].width * 2 + gap, windows[0].height),
                    (24, 24, 24))
    out.paste(windows[0], (0, 0))
    out.paste(windows[1], (windows[0].width + gap, 0))
    dest = os.path.join(outdir, f"t14-dock-plate-corners-{scheme}.png")
    os.makedirs(outdir, exist_ok=True)
    out.save(dest)
    log(f"saved {dest} ({out.width}x{out.height})")
    return dest


def magnified_strip(full_path, rect, plate, scheme, outdir):
    """A 2x strip of the plate mid-magnification."""
    rx, ry = rect
    top, left, right = plate
    im = Image.open(full_path).convert("RGB")
    x0 = max(0, left - 60)
    x1 = min(NESTED_W, right + 60)
    y0 = max(0, top - 70)
    crop = im.crop((rx + x0, ry + y0, rx + x1, ry + DOCK_BAND[3]))
    crop = crop.resize((crop.width * 2, crop.height * 2), Image.NEAREST)
    dest = os.path.join(outdir, f"t14-dock-plate-magnified-{scheme}.png")
    crop.save(dest)
    log(f"saved {dest} ({crop.width}x{crop.height})")
    return dest


def run(args):
    synth = Synthetic(args.synth)
    cap = Capturer(args.outdir, args.scratch)
    synth.send("set color-scheme dark")
    time.sleep(0.3)

    init = cap.full("initial")
    for attempt in range(40):
        try:
            cap.detect_rect(init)
            break
        except RuntimeError:
            if attempt == 39:
                raise
            time.sleep(0.5)
            init = cap.full("initial")
    log(f"nested window rect {cap.rect}")

    for scheme in ("dark", "light"):
        synth.send(f"set color-scheme {scheme}")
        time.sleep(0.5)

        # Resting: pointer off the Dock so the plate is at its baseline.
        synth.motion(NESTED_W // 2, 20)
        time.sleep(0.9)
        full = cap.full(f"rest-{scheme}")
        plate = detect_plate(full, cap.rect, RIM_THRESHOLD[scheme])
        log(f"{scheme}: resting plate top={plate[0]} left={plate[1]} right={plate[2]}")
        corner_strip(full, cap.rect, plate, scheme, args.outdir)

        # Magnified: pointer over the plate centre, on the artwork band.
        cx = (plate[1] + plate[2]) // 2
        cy = plate[0] + 40
        synth.motion(cx, cy)
        time.sleep(1.0)
        full = cap.full(f"magnified-{scheme}")
        magnified_strip(full, cap.rect, plate, scheme, args.outdir)

    log("done")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--demo-log", required=True)
    parser.add_argument("--compositor-pid", type=int, default=0)
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())