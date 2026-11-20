#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested session and capture the T-20.2 liquid-glass stills.

Machine half of the T-20.2 live visual check: on the running nested session it
sends synthetic-input commands (the T-03 harness installed when
``DRAGONFRUIT_SYNTHETIC_INPUT`` is set) through the compositor's real input
path, then screenshots the nested window with Spectacle.

``--steps`` is a small sequence language so the shell script owns the scenario:

    sleep:2.0            wait
    click:X:Y            absolute pointer click in output pixels
    move:X:Y             absolute pointer motion
    cmd:<raw command>    send a raw synthetic-input command
    shot:<name>          screenshot to ``<outdir>/<name>.png``

The screenshot is the active nested window (Spectacle ``-a``): it is
alpha-cropped to the window and then cropped center-x / bottom to the nested
1920x1200 output, matching the T-17/T-19 capture harness. Driven by
``scripts/capture-t20-liquid-glass.sh``.

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


def log(message):
    print(f"capture-t20: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/dragonfruit-capture-t20-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def click(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")
        time.sleep(0.3)
        self.send(f"button {BTN_LEFT} down")
        time.sleep(0.12)
        self.send(f"button {BTN_LEFT} up")
        time.sleep(0.6)


def crop_nested(raw, dest):
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
    im.save(dest)
    log(f"saved {dest}")


def shot(outdir, name):
    raw = os.path.join(outdir, f".{name}-raw.png")
    dest = os.path.join(outdir, f"{name}.png")
    subprocess.run(
        ["spectacle", "-b", "-n", "-a", "-o", raw],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    crop_nested(raw, dest)
    try:
        os.unlink(raw)
    except FileNotFoundError:
        pass


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
        elif verb == "move":
            x, y = (float(v) for v in rest.split(":"))
            synth.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")
            time.sleep(0.2)
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