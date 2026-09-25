#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-11.3b Control Center Focus/DND and dark-mode tiles.

Opens the panel through the real Control-Option-C shortcut, captures the
expanded five-tile panel, clicks the Focus switch and the dark-mode switch
through the real synthetic-pointer path, and writes the before/after stills:

  docs/captures/t11-control-center-focus-dark.png   (after Focus -> off)
  docs/captures/t11-control-center-dark-toggle.png  (after Dark Mode toggle)

The nested output is 1920x1200 and the panel is anchored top-right; the
switch coordinates are derived from the panel geometry constants.
"""
import os
import socket
import subprocess
import time

from PIL import Image

WALLS = [(33, 13, 41), (41, 36, 52), (243, 243, 245), (250, 250, 252)]
WALL_TOL = 6
WALL_ROW_MIN = 100
BAR_HEIGHT = 28
PANEL_W = 360
PANEL_H = 520
PANEL_TOP = BAR_HEIGHT + 8
OUTPUT_W = 1920
OUTPUT_H = 1200
TOGGLE_RIGHT = 320  # switch centre x, from the panel's left edge

KEY_LEFTCTRL, KEY_LEFTALT, KEY_C = 29, 56, 46
BTN_LEFT = 272


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-capture-t75-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(1.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def click(self, x, y):
        self.send(f"motion-abs {x / OUTPUT_W:.4f} {y / OUTPUT_H:.4f}")
        time.sleep(0.1)
        self.send(f"button {BTN_LEFT} down")
        time.sleep(0.08)
        self.send(f"button {BTN_LEFT} up")
        time.sleep(0.5)


def matches(c):
    return any(all(abs(c[i] - w[i]) <= WALL_TOL for i in range(3)) for w in WALLS)


def detect_rect(im):
    px = im.load()
    w, h = im.size
    minx = miny = 10**9
    maxx = maxy = -1
    for y in range(0, h, 2):
        xs = [x for x in range(0, w, 2) if matches(px[x, y])]
        if len(xs) >= WALL_ROW_MIN:
            minx = min(minx, xs[0])
            maxx = max(maxx, xs[-1])
            miny = min(miny, y)
            maxy = max(maxy, y)
    if maxx < 0 or (maxx - minx) > 2400 or miny < 0:
        return (w - OUTPUT_W) // 2, (h - OUTPUT_H) // 2, OUTPUT_W, (h + OUTPUT_H) // 2
    return minx, max(0, miny - BAR_HEIGHT), maxx - minx, maxy


def clip(im, rect, out_dir, name):
    x, y, w, _ = rect
    panel_x = x + w - PANEL_W
    panel_y = y + PANEL_TOP
    im.crop((panel_x, panel_y, panel_x + PANEL_W, panel_y + PANEL_H)).save(
        os.path.join(out_dir, name))
    return os.path.join(out_dir, name)


def screenshot():
    raw = "/tmp/opencode/t75-full-raw.png"
    subprocess.run(["spectacle", "-b", "-n", "-f", "-o", raw], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return Image.open(raw).convert("RGB")


def main():
    synth_path = os.environ.get("T75_SYNTH")
    assert synth_path, "T75_SYNTH must point at the synthetic-input socket"
    synth = Synthetic(synth_path)
    out = os.environ.get("T75_OUT", "docs/captures")
    os.makedirs(out, exist_ok=True)

    # The notification fixture seeds a banner at the same top-right corner as
    # the panel; click its body to dismiss it so the panel is not occluded.
    synth.click(OUTPUT_W - 190, PANEL_TOP + 44)
    time.sleep(0.5)

    # Control-Option-C opens the Control Center.
    for code in (KEY_LEFTCTRL, KEY_LEFTALT):
        synth.send(f"key {code} down")
    synth.send(f"key {KEY_C} down")
    time.sleep(0.15)
    synth.send(f"key {KEY_C} up")
    for code in (KEY_LEFTALT, KEY_LEFTCTRL):
        synth.send(f"key {code} up")
    time.sleep(1.5)

    im = screenshot()
    rect = detect_rect(im)
    print(f"nested rect x={rect[0]} y={rect[1]} w={rect[2]}")

    # Canonical five-tile panel (Focus active from the fixture), and the whole
    # nested window for context. The T-11.3a stills are refreshed in place.
    clip(im, rect, out, "t11-control-center.png")
    x, y, w, _ = rect
    im.crop((x, y, x + w, y + 900)).save(
        os.path.join(out, "t11-control-center-context.png"))

    # Switch centres in panel-relative pixels, taken from the QML layout
    # (`tst_controlcenter` dumps them). Synthetic pointer input is normalized
    # against the nested output, so the panel origin is the output's top-right.
    panel_left = OUTPUT_W - PANEL_W
    focus_y = PANEL_TOP + 131   # focusToggle 300,119 40x24
    dark_y = PANEL_TOP + 419    # darkToggle 300,407 40x24

    # Focus switch off (the fixture starts it in DND); capture the live apply.
    synth.click(panel_left + TOGGLE_RIGHT, focus_y)
    time.sleep(0.4)
    im = screenshot()
    clip(im, rect, out, "t11-control-center-focus-dark.png")
    print("saved focus-dark", focus_y)

    # Dark-mode switch: capture the scheme after the flip.
    synth.click(panel_left + TOGGLE_RIGHT, dark_y)
    time.sleep(0.6)
    im = screenshot()
    clip(im, rect, out, "t11-control-center-dark-toggle.png")
    print("saved dark-toggle", dark_y)

    log = "/tmp/opencode/t75-demo.log"
    if os.path.exists(log):
        with open(log) as handle:
            for line in handle.readlines()[-8:]:
                print("LOG:", line.rstrip())


if __name__ == "__main__":
    main()