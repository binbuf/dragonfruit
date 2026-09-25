#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-11.3a Control Center panel from the live nested session.

Opens the panel through the real input path (the Control-Option-C keyboard
shortcut, which the compositor routes as a `control-center` input action),
screenshots the whole host screen, locates the nested window, and crops the
top-right panel into `docs/captures/t11-control-center.png`.
"""
import os
import socket
import subprocess
import time

from PIL import Image

# Candidate wallpaper colors (dark and light schemes) the nested output is
# cleared with; the host desktop does not share them.
WALLS = [(33, 13, 41), (41, 36, 52), (243, 243, 245), (250, 250, 252)]
WALL_TOL = 6
WALL_ROW_MIN = 100
BAR_HEIGHT = 28
PANEL_W = 360
PANEL_H = 520
PANEL_TOP = BAR_HEIGHT + 8  # the panel's top margin below the menu bar

KEY_LEFTCTRL, KEY_LEFTALT, KEY_C = 29, 56, 46


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-capture-t74-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(1.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)


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
    # The host desktop can share a candidate color; fall back to the known
    # centered nested window (1920x1200) when the match spans the whole screen.
    if maxx < 0 or (maxx - minx) > 2400 or miny < 0:
        return (w - 1920) // 2, (h - 1200) // 2, 1920, (h + 1200) // 2
    return minx, max(0, miny - BAR_HEIGHT), maxx - minx, maxy


def main():
    synth_path = os.environ.get("T74_SYNTH")
    assert synth_path, "T74_SYNTH must point at the synthetic-input socket"
    synth = Synthetic(synth_path)
    # Control-Option-C opens the Control Center.
    for code in (KEY_LEFTCTRL, KEY_LEFTALT):
        synth.send(f"key {code} down")
    synth.send(f"key {KEY_C} down")
    time.sleep(0.15)
    synth.send(f"key {KEY_C} up")
    for code in (KEY_LEFTALT, KEY_LEFTCTRL):
        synth.send(f"key {code} up")
    time.sleep(1.5)

    raw = "/tmp/opencode/t74-full-raw.png"
    subprocess.run(["spectacle", "-b", "-n", "-f", "-o", raw], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    im = Image.open(raw).convert("RGB")
    x, y, w, maxy = detect_rect(im)
    print(f"nested rect x={x} y={y} w={w} bottom={maxy}")
    out = os.environ.get("T74_OUT", "docs/captures")
    os.makedirs(out, exist_ok=True)
    # The panel is anchored to the nested output's top-right corner.
    panel_x = x + w - PANEL_W
    panel_y = y + PANEL_TOP
    panel = im.crop((panel_x, panel_y, panel_x + PANEL_W, panel_y + PANEL_H))
    panel.save(os.path.join(out, "t11-control-center.png"))
    # Also keep the whole nested window for context.
    im.crop((x, y, x + w, y + 900)).save(os.path.join(out, "t11-control-center-context.png"))
    print("saved panel", panel.size)
    log = "/tmp/opencode/t74-demo.log"
    if os.path.exists(log):
        with open(log) as handle:
            for line in handle.readlines()[-6:]:
                print("LOG:", line.rstrip())


if __name__ == "__main__":
    main()