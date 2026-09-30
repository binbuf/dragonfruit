#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested session and capture the T-20.3 chrome-material stills.

Machine half of the T-20.3 live visual check: on the running nested session it
sends synthetic-input commands (the T-03 harness installed when
``DRAGONFRUIT_SYNTHETIC_INPUT`` is set) through the compositor's real input
path, then screenshots the nested window with Spectacle.

``--steps`` is a small sequence language so the shell script owns the scenario:

    sleep:2.0            wait
    click:X:Y            absolute pointer left click in output pixels
    rclick:X:Y           absolute pointer right click in output pixels
    move:X:Y             absolute pointer motion
    key:CODE:down|up     a raw keyboard key down/up (evdev code)
    chord:C,C,C          press then release keys in order (Ctrl+Alt+C opens
                         the Control Center)
    drag:X0:Y0:X1:Y1     pointer drag (presents the OSD from a slider)
    menu:Y:X0:X1:STEP:NAME
                         probe left-to-right clicks at bar row Y until one opens
                         a menu (the bar below changes), then save the frame as
                         ``<outdir>/<name>.png``. The status row is
                         right-anchored, so its x shifts with the clock width; a
                         fixed coordinate is not stable across runs.
    cmd:<raw command>    send a raw synthetic-input command
    shot:<name>          screenshot to ``<outdir>/<name>.png``

The screenshot is the active nested window (Spectacle ``-a``): it is
alpha-cropped to the window and then cropped center-x / bottom to the nested
1920x1200 output, matching the T-17/T-19/T-20.2 capture harness. Driven by
``scripts/capture-t20-chrome-material.sh``.

Requires a host Wayland session, ``spectacle``, python3 with Pillow, and the
built tree.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

NESTED_W, NESTED_H = 1920, 1200
BTN_LEFT = 0x110
BTN_RIGHT = 0x111


def log(message):
    print(f"capture-t20: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/dragonfruit-capture-t20-chrome-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def move(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")
        time.sleep(0.2)

    def button(self, code, x, y):
        self.move(x, y)
        self.send(f"button {code} down")
        time.sleep(0.12)
        self.send(f"button {code} up")
        time.sleep(0.6)

    def click(self, x, y):
        self.button(BTN_LEFT, x, y)

    def rclick(self, x, y):
        self.button(BTN_RIGHT, x, y)

    def key(self, code, down):
        self.send(f"key {code} {'down' if down else 'up'}")

    def chord(self, codes):
        for code in codes:
            self.key(code, True)
        time.sleep(0.15)
        for code in reversed(codes):
            self.key(code, False)
        time.sleep(1.0)

    def drag(self, x0, y0, x1, y1):
        self.move(x0, y0)
        self.send(f"button {BTN_LEFT} down")
        time.sleep(0.1)
        self.move(x1, y1)
        time.sleep(0.1)
        self.send(f"button {BTN_LEFT} up")
        time.sleep(0.4)


def crop_image(raw):
    """Alpha-crop the nested window and crop to the 1920x1200 output."""
    from PIL import Image

    im = Image.open(raw)
    if im.mode == "RGBA":
        alpha = im.getchannel("A")
        w, h = alpha.size
        px = alpha.load()
        minx, miny, maxx, maxy = w, h, -1, -1
        for y in range(h):
            for x in range(w):
                if px[x, y] == 255:
                    minx = min(minx, x)
                    maxx = max(maxx, x)
                    miny = min(miny, y)
                    maxy = max(maxy, y)
        if maxx > minx:
            im = im.crop((minx, miny, maxx + 1, maxy + 1))
    im = im.convert("RGB")
    w, h = im.size
    if w >= NESTED_W and h >= NESTED_H:
        x0 = (w - NESTED_W) // 2
        y0 = h - NESTED_H
        im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
    return im


def capture(outdir):
    """Screenshot the active nested window and return the cropped output."""
    raw = os.path.join(outdir, ".capture-raw.png")
    subprocess.run(
        ["spectacle", "-b", "-n", "-a", "-o", raw],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    im = crop_image(raw)
    try:
        os.unlink(raw)
    except FileNotFoundError:
        pass
    return im


def shot(outdir, name):
    im = capture(outdir)
    dest = os.path.join(outdir, f"{name}.png")
    im.save(dest)
    log(f"saved {dest}")


def probe_menu(synth, outdir, y, x0, x1, step, name):
    """Click along the bar row `y` until a menu opens; save that frame.

    The status row is right-anchored, so the item center drifts with the clock
    width. A click that opens a menu changes the full-width region below the
    bar, which is the detection signal.
    """
    from PIL import ImageChops

    base = capture(outdir)
    box = (0, 30, NESTED_W, NESTED_H)
    base_crop = base.crop(box)
    for x in range(x0, x1 + 1, step):
        synth.click(x, y)
        im = capture(outdir)
        diff = ImageChops.difference(im.crop(box), base_crop)
        pixels = list(diff.getdata())
        mad = sum(sum(p) for p in pixels) / (len(pixels) * 3)
        log(f"menu probe x={x} mad={mad:.1f}")
        if mad > 20.0:
            dest = os.path.join(outdir, f"{name}.png")
            im.save(dest)
            log(f"saved {dest} (menu at x={x})")
            return True
    return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--steps", required=True)
    args = parser.parse_args()

    os.makedirs(args.outdir, exist_ok=True)
    synth = Synthetic(args.synth)

    for step in args.steps.split(";"):
        step = step.strip()
        if not step:
            continue
        verb, _, rest = step.partition(":")
        if verb == "sleep":
            time.sleep(float(rest))
        elif verb == "click":
            x, y = (float(v) for v in rest.split(":"))
            log(f"click ({x:.0f}, {y:.0f})")
            synth.click(x, y)
        elif verb == "rclick":
            x, y = (float(v) for v in rest.split(":"))
            log(f"rclick ({x:.0f}, {y:.0f})")
            synth.rclick(x, y)
        elif verb == "move":
            x, y = (float(v) for v in rest.split(":"))
            synth.move(x, y)
        elif verb == "key":
            code, _, direction = rest.partition(":")
            synth.key(int(code), direction == "down")
            time.sleep(0.1)
        elif verb == "chord":
            codes = [int(c) for c in rest.split(",")]
            log(f"chord {codes}")
            synth.chord(codes)
        elif verb == "drag":
            x0, y0, x1, y1 = (float(v) for v in rest.split(":"))
            log(f"drag ({x0:.0f}, {y0:.0f}) -> ({x1:.0f}, {y1:.0f})")
            synth.drag(x0, y0, x1, y1)
        elif verb == "menu":
            y, x0, x1, step, name = rest.split(":")
            log(f"menu probe y={y} x={x0}..{x1} step={step}")
            if not probe_menu(synth, args.outdir, int(y), int(x0), int(x1),
                              int(step), name):
                log(f"menu probe found no menu for {name}")
                return 1
        elif verb == "cmd":
            log(f"cmd {rest}")
            synth.send(rest)
            time.sleep(0.5)
        elif verb == "shot":
            shot(args.outdir, rest)
        else:
            log(f"unknown step {step!r}")
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())