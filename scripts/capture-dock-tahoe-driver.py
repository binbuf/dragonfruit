#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-14.7j Dock Tahoe visual language.

Driven by ``scripts/capture-dock-tahoe.sh`` (run that, not this). Requires a
host Wayland session, ``spectacle``, and Pillow.

For the light and dark schemes it parks the pointer off the Dock (resting),
over the Trash (a hover wash + name label, no magnification), over an app icon
(magnification + name label), and holds the left button down on an app icon
(pressed). The four crops are stacked into one still per scheme. A third still
shows the `Minimal` degrade tier (blur off) and reduced motion, so the plate
reads as a clean capsule with no glass claim.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

from PIL import Image, ImageDraw

NESTED_W, NESTED_H = 1920, 1200
# The active Space wallpaper the compositor clears the nested output with.
WALL = (33, 13, 41)
WALL_TOL = 3
WALL_ROW_MIN = 200

DOCK_Y = 1155
DOCK_CENTER_X = 960
# A little inside the right end, over the Trash (which does not magnify).
DOCK_TRASH_X = 1265
# An app icon left of centre, so magnification and its neighbour read.
DOCK_APP_X = 870
# Pointer parked well above the Dock for the resting still.
AWAY_X, AWAY_Y = 960, 160

DOCK_BAND = (560, 1000, 1360, 1200)  # x, y, x2, y2
BTN_LEFT = 272


def log(message):
    print(f"capture-dock-tahoe: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-tahoe-{os.getpid()}.sock"
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
        chunks = []
        while True:
            try:
                data, _ = self.sock.recvfrom(65536)
            except socket.timeout:
                break
            text = data.decode()
            chunks.append(text)
            if "end\n" in text:
                break
        return "".join(chunks)

    def motion(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")

    def button(self, down):
        self.send(f"button {BTN_LEFT} {'down' if down else 'up'}")


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
        for y in range(0, h):
            xs = [x for x in range(0, w, 4) if matches(px[x, y])]
            if len(xs) >= WALL_ROW_MIN:
                rows.append((y, xs[0], xs[-1]))
        if not rows:
            raise RuntimeError("nested window not found (no wallpaper pixels)")
        top = rows[0][0]
        left = min(row[1] for row in rows)
        self.rect = (left, top, NESTED_W, NESTED_H)
        return self.rect

    def dock_crop(self, name):
        full = self.full(name)
        rx, ry, _rw, _rh = self.rect
        im = Image.open(full).convert("RGB")
        x0, y0, x1, y1 = DOCK_BAND
        return im.crop((rx + x0, ry + y0, rx + x1, ry + y1))


def stack(panels, labels, dest, label_h=22, gap=8, bg=(24, 24, 28)):
    w = max(p.width for p in panels)
    h = sum(p.height for p in panels) + gap * (len(panels) - 1) + label_h * len(panels)
    out = Image.new("RGB", (w, h), bg)
    draw = ImageDraw.Draw(out)
    y = 0
    for panel, label in zip(panels, labels):
        draw.text((6, y + 4), label, fill=(220, 220, 224))
        y += label_h
        out.paste(panel, ((w - panel.width) // 2, y))
        y += panel.height + gap
    out.save(dest)
    log(f"saved {dest} ({out.width}x{out.height})")


def capture_scheme(synth, cap, scheme):
    synth.send(f"set color-scheme {scheme}")
    time.sleep(0.5)

    synth.motion(AWAY_X, AWAY_Y)
    time.sleep(0.8)
    resting = cap.dock_crop(f"resting-{scheme}")

    synth.motion(DOCK_TRASH_X, DOCK_Y)
    time.sleep(0.9)
    hovered = cap.dock_crop(f"hovered-{scheme}")

    synth.motion(DOCK_APP_X, DOCK_Y)
    time.sleep(0.9)
    magnified = cap.dock_crop(f"magnified-{scheme}")

    synth.button(True)
    time.sleep(0.25)
    pressed = cap.dock_crop(f"pressed-{scheme}")
    synth.button(False)
    synth.motion(AWAY_X, AWAY_Y)
    time.sleep(0.4)

    dest = os.path.join(cap.outdir, f"t14-dock-tahoe-{scheme}.png")
    stack([resting, hovered, magnified, pressed],
          ["resting", "hovered (Trash)", "magnified", "pressed"], dest)


def capture_reduced(synth, cap):
    synth.send("set color-scheme dark")
    synth.send("set reduced-motion on")
    synth.send("set degrade-tier minimal")
    time.sleep(0.5)

    synth.motion(AWAY_X, AWAY_Y)
    time.sleep(0.8)
    resting = cap.dock_crop("resting-reduced")

    synth.motion(DOCK_APP_X, DOCK_Y)
    synth.button(True)
    time.sleep(0.5)
    pressed = cap.dock_crop("pressed-reduced")
    synth.button(False)
    synth.motion(AWAY_X, AWAY_Y)

    dest = os.path.join(cap.outdir, "t14-dock-tahoe-reduced.png")
    stack([resting, pressed],
          ["minimal tier / reduced motion — resting (no glass claim)",
           "minimal tier / reduced motion — pressed"], dest)


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

    capture_scheme(synth, cap, "dark")
    capture_scheme(synth, cap, "light")
    capture_reduced(synth, cap)
    log("done")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())