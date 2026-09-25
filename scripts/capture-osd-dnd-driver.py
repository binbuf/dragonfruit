#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-11.4b OSD and DND stills from the live nested session.

The shell runs with `DF_STATUS_FIXTURE=1 DF_NOTIFY_FIXTURE=1
DF_FOCUS_FIXTURE=dnd`, so the Focus/DND crescent renders in the status row
deterministically. The driver:

  1. crops the right of the menu bar to `t11-dnd.png` (the accent-tinted
     crescent left of the clock);
  2. opens the Control Center through the real Control-Option-C shortcut and
     drags the Sound volume slider (a real user gesture that presents the OSD),
     polling the shell log for the OSD scene-graph commit, then crops the
     centered card to `t11-osd.png` and keeps the whole nested window as
     `t11-osd-context.png`.

The nested output is 1920x1200 and the panel/surface coordinates are derived
from the shell's geometry constants (the same values the T-11.3a/b driver
uses).
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
PANEL_TOP = BAR_HEIGHT + 8
OUTPUT_W = 1920
OUTPUT_H = 1200
LOG = "/tmp/opencode/t11-demo.log"

KEY_LEFTCTRL, KEY_LEFTALT, KEY_C = 29, 56, 46
BTN_LEFT = 272


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-capture-t11-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(1.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def move(self, x, y):
        self.send(f"motion-abs {x / OUTPUT_W:.4f} {y / OUTPUT_H:.4f}")
        time.sleep(0.06)

    def click(self, x, y):
        self.move(x, y)
        self.send(f"button {BTN_LEFT} down")
        time.sleep(0.08)
        self.send(f"button {BTN_LEFT} up")
        time.sleep(0.4)

    def drag(self, x0, y0, x1, y1):
        self.move(x0, y0)
        self.send(f"button {BTN_LEFT} down")
        time.sleep(0.1)
        self.move(x1, y1)
        time.sleep(0.1)
        self.send(f"button {BTN_LEFT} up")


def matches(c, wall):
    return all(abs(c[i] - wall[i]) <= WALL_TOL for i in range(3))


def detect_rect(im):
    px = im.load()
    w, h = im.size
    minx = miny = 10**9
    maxx = maxy = -1
    for y in range(0, h, 2):
        xs = [x for x in range(0, w, 2)
              if any(matches(px[x, y], wall) for wall in WALLS)]
        if len(xs) >= WALL_ROW_MIN:
            minx = min(minx, xs[0])
            maxx = max(maxx, xs[-1])
            miny = min(miny, y)
            maxy = max(maxy, y)
    if maxx < 0 or (maxx - minx) > 2400 or miny < 0:
        return (w - OUTPUT_W) // 2, (h - OUTPUT_H) // 2, OUTPUT_W, OUTPUT_H
    return minx, max(0, miny - BAR_HEIGHT), maxx - minx, maxy - max(0, miny - BAR_HEIGHT)


def screenshot(raw):
    subprocess.run(["spectacle", "-b", "-n", "-f", "-o", raw], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return Image.open(raw).convert("RGB")


def osd_committed():
    if not os.path.exists(LOG):
        return False
    with open(LOG, errors="ignore") as handle:
        return "OSD scene-graph commit path active" in handle.read()


def main():
    synth_path = os.environ.get("T77_SYNTH")
    assert synth_path, "T77_SYNTH must point at the synthetic-input socket"
    out = os.environ.get("T77_OUT", "docs/captures")
    os.makedirs(out, exist_ok=True)
    synth = Synthetic(synth_path)
    raw = "/tmp/opencode/t11-full-raw.png"

    # Dismiss the notification fixture banner so it does not occlude the panel
    # (the fixture mock seeds one even in DND; the real service suppresses).
    synth.click(OUTPUT_W - 190, PANEL_TOP + 44)
    time.sleep(0.5)

    # The DND still: the accent-tinted crescent sits immediately left of the
    # clock in the status row. Capture before opening the panel.
    im = screenshot(raw)
    x, y, w, _ = detect_rect(im)
    print(f"nested rect x={x} y={y} w={w}")
    im.crop((x + int(w * 0.62), y, x + w, y + BAR_HEIGHT)).save(
        os.path.join(out, "t11-dnd.png"))

    # Open the Control Center through the real Control-Option-C shortcut.
    for code in (KEY_LEFTCTRL, KEY_LEFTALT):
        synth.send(f"key {code} down")
    synth.send(f"key {KEY_C} down")
    time.sleep(0.15)
    synth.send(f"key {KEY_C} up")
    for code in (KEY_LEFTALT, KEY_LEFTCTRL):
        synth.send(f"key {code} up")
    time.sleep(1.2)

    # Synthetic pointer input is normalized against the nested *output*, not the
    # host screen; the panel is anchored to the output's top-right.
    panel_left = OUTPUT_W - PANEL_W
    # Probe candidate Sound slider rows; stop once the shell logs the OSD.
    for rel_y in (200, 215, 230, 245, 260, 275, 185, 290):
        synth.drag(panel_left + 50, PANEL_TOP + rel_y,
                   panel_left + 300, PANEL_TOP + rel_y)
        time.sleep(0.25)
        if osd_committed():
            print(f"OSD committed after drag at panel-relative y={rel_y}")
            break
    else:
        print("no OSD commit observed")

    time.sleep(0.2)
    im = screenshot(raw)
    x, y, w, h = detect_rect(im)
    if h > 1300 or w > 2000:
        x, y, w, h = ((im.size[0] - OUTPUT_W) // 2, (im.size[1] - OUTPUT_H) // 2,
                      OUTPUT_W, OUTPUT_H)
    cx, cy = x + w // 2, y + h // 2
    im.crop((cx - 150, cy - 150, cx + 150, cy + 150)).save(
        os.path.join(out, "t11-osd.png"))
    im.crop((x, y, x + w, y + h)).save(os.path.join(out, "t11-osd-context.png"))
    print(f"saved {out}/t11-osd.png and t11-dnd.png")

    if os.path.exists(LOG):
        with open(LOG, errors="ignore") as handle:
            for line in handle.readlines()[-6:]:
                print("LOG:", line.rstrip())


if __name__ == "__main__":
    main()