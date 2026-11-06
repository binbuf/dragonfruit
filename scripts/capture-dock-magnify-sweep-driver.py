#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-14.7y Dock magnification slow-sweep strip.

Driven by ``scripts/capture-dock-magnify-sweep.sh`` (run that, not this).
Requires a host Wayland session, ``spectacle``, and Pillow.

Two modes:

* ``--capture`` sweeps the pointer slowly across the Dock for one colour
  scheme and writes one vertical filmstrip of Dock crops
  (``sweep-row-<scheme>-<label>.png`` in ``--outdir``).
* ``--compose`` stacks the captured rows into the committed
  ``t14-dock-magnify-sweep-<scheme>.png`` strips, labelling the pre-fix row
  "before" and the fixed row "after" when both are present.

The before row is captured separately from the pre-fix tree (the T110y note in
PROGRESS.md records the procedure); a settled crop cannot show the temporal
ringing, so the regression is pinned by ``tst_dock.qml`` and the strip is the
visual pass over the fixed tracking.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

from PIL import Image, ImageDraw

NESTED_W, NESTED_H = 1920, 1200
# The virtual host's desktop is pure black; the nested window (including its
# titlebar) is the one large non-black rectangle. Do not key on the wallpaper
# colour: the shipped default is a photograph, and the host may supply its own.
BLACK_SUM = 12
DETECT_ROW_MIN = 200

DOCK_Y = 1155
DOCK_LEFT_X = 785
DOCK_RIGHT_X = 1135
DOCK_BAND_X = (560, 1360)  # x, x2 relative to the nested content
DOCK_BAND_H = 120          # bottom-relative crop height (Dock + a little air)
FRAMES = 6


def log(message):
    print(f"capture-dock-magnify-sweep: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-sweep-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(2.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def query(self, command):
        self.send(command)
        data, _ = self.sock.recvfrom(65536)
        return data.decode()

    def motion(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")


def detect_rect(path):
    im = Image.open(path).convert("RGB")
    px = im.load()
    w, h = im.size

    rows = []
    for y in range(0, h, 2):
        xs = [x for x in range(0, w, 4) if sum(px[x, y]) > BLACK_SUM]
        if len(xs) >= DETECT_ROW_MIN:
            rows.append((y, xs[0], xs[-1]))
    if not rows:
        raise RuntimeError("nested window not found (no non-black pixels)")
    top = rows[0][0]
    bottom = rows[-1][0]
    left = min(row[1] for row in rows)
    right = max(row[2] for row in rows)
    return (left, top, right - left, bottom - top)


def full_shot(dest):
    subprocess.run(
        ["spectacle", "-b", "-n", "-f", "-o", dest],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return dest


def crop_band(full_path, rect, dest):
    rx, _ry, rw, rh = rect
    im = Image.open(full_path).convert("RGB")
    x0, x1 = DOCK_BAND_X
    # Crop from the nested window's bottom edge, not its top: the host may
    # decorate the nested window, and the Dock is anchored to the content
    # bottom in every case.
    x0 = max(0, min(rw, x0))
    x1 = max(0, min(rw, x1))
    y1 = rh
    y0 = max(0, rh - DOCK_BAND_H)
    crop = im.crop((rx + x0, _ry + y0, rx + x1, _ry + y1))
    crop.save(dest)
    return crop


def capture(args):
    synth = Synthetic(args.synth)
    scratch = args.scratch
    os.makedirs(scratch, exist_ok=True)
    synth.send(f"set color-scheme {args.scheme}")
    time.sleep(0.5)

    # The loop demo opens Settings (and the X11 probe) over the Dock. Close
    # every client window so the sweep crops the live Dock and its wallpaper,
    # not an app pane behind the glass.
    try:
        report = synth.query("query decorations")
        for line in report.splitlines():
            fields = line.split()
            if len(fields) >= 2 and fields[0] == "decoration":
                synth.send(f"close {fields[1]}")
        time.sleep(0.8)
    except (TimeoutError, socket.timeout):
        log("no decoration reply; sweeping with the demo windows present")

    init = full_shot(os.path.join(scratch, "init.png"))
    rect = detect_rect(init)
    log(f"nested window rect {rect}")

    rows = []
    positions = [DOCK_LEFT_X + (DOCK_RIGHT_X - DOCK_LEFT_X) * i / (args.frames - 1)
                 for i in range(args.frames)]
    for index, x in enumerate(positions):
        synth.motion(x, DOCK_Y)
        # Let the short tracker settle so the crop is a real Dock state; the
        # temporal ringing itself is measured by the QML tests.
        time.sleep(0.25)
        full = full_shot(os.path.join(scratch, f"{args.label}-{args.scheme}-{index}.png"))
        rows.append(crop_band(full, rect, os.path.join(
            scratch, f"{args.label}-{args.scheme}-{index}-crop.png")))

    strip = Image.new("RGB", (rows[0].width, rows[0].height * len(rows)))
    for index, row in enumerate(rows):
        strip.paste(row, (0, index * rows[0].height))
    out = os.path.join(args.outdir, f"sweep-row-{args.scheme}-{args.label}.png")
    strip.save(out)
    log(f"saved {out} ({strip.width}x{strip.height})")


def label_bar(width, text, home):
    bar = Image.new("RGB", (width, home), (18, 18, 22))
    draw = ImageDraw.Draw(bar)
    draw.text((8, max(0, home // 2 - 6)), text, fill=(230, 230, 235))
    return bar


def compose(args):
    home = 20
    for scheme in ("light", "dark"):
        rows = []
        for label in ("before", "after"):
            path = os.path.join(args.outdir, f"sweep-row-{scheme}-{label}.png")
            if not os.path.exists(path):
                continue
            image = Image.open(path).convert("RGB")
            rows.append(label_bar(image.width, f"{label}  ({scheme})", home))
            rows.append(image)
        if not rows:
            log(f"no rows for {scheme}; skipping")
            continue
        width = max(image.width for image in rows)
        height = sum(image.height for image in rows)
        strip = Image.new("RGB", (width, height))
        y = 0
        for image in rows:
            strip.paste(image, (0, y))
            y += image.height
        out = os.path.join(args.outdir, f"t14-dock-magnify-sweep-{scheme}.png")
        strip.save(out)
        log(f"saved {out} ({strip.width}x{strip.height})")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth")
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch")
    parser.add_argument("--scheme", choices=("light", "dark"))
    parser.add_argument("--label", default="after")
    parser.add_argument("--frames", type=int, default=FRAMES)
    parser.add_argument("--capture", action="store_true")
    parser.add_argument("--compose", action="store_true")
    args = parser.parse_args()
    try:
        if args.compose:
            compose(args)
        else:
            capture(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())