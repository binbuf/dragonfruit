#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested T-06 app switcher with synthetic input and capture stills.

This is the machine half of the T-06.2a capture (docs/design/tracks/06-loop-v4-
app-switcher.md): it opens the compositor-owned Cmd-Tab switcher on the
*running* nested session through the real input path (the T-03 synthetic-input
harness, installed on the nested backend only when
``DRAGONFRUIT_SYNTHETIC_INPUT`` is set), cycles the selection, and screenshots
each state, plus the reduced-motion variant.

The compositor renders the live preview surfaces through the T-04 scene
transform; the shell draws the centered app cards. Both are visible in the
still, so the capture is the whole overlay, not a thumbnail substitution.

It is driven by ``scripts/capture-switcher.sh``; run that, not this. Requires a
host Wayland session, ``spectacle``, and Pillow.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
# The active Space wallpaper the compositor clears the nested output with; the
# host centers the nested 1920x1200 window, so `detect_rect` validates that.
WALL = (41, 36, 52)
WALL_TOL = 8

KEY_LEFTMETA, KEY_TAB, KEY_ESC, KEY_GRAVE = 125, 15, 1, 41


def log(message):
    print(f"capture: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-capture-switcher-{os.getpid()}.sock"
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

    def switcher(self):
        """The `query switcher` report as a dict (summary plus entries)."""
        report = {"active": False, "entries": [], "previews": []}
        for line in self.query("query switcher").splitlines():
            p = line.split()
            if not p or p[0] != "switcher":
                continue
            if p[1] == "app":
                report["entries"].append(p[3])
            elif p[1] == "preview":
                report["previews"].append(int(p[2]))
            elif "=" in p[1]:
                for field in p[1:]:
                    key, _, value = field.partition("=")
                    report[key] = value
                report["active"] = report.get("active") == "1"
        return report


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
        """Locate the nested 1920x1200 output in the host screenshot."""
        im = Image.open(path).convert("RGB")
        px = im.load()
        w, h = im.size

        def matches(c):
            return all(abs(c[i] - WALL[i]) <= WALL_TOL for i in range(3))

        if w >= NESTED_W and h >= NESTED_H:
            self.rect = ((w - NESTED_W) // 2, (h - NESTED_H) // 2, NESTED_W, NESTED_H)
            return self.rect

        best = (0, 0)
        for y in range(0, h, 2):
            run_start = None
            for x in range(0, w):
                if matches(px[x, y]):
                    if run_start is None:
                        run_start = x
                else:
                    if run_start is not None:
                        if x - run_start > best[0]:
                            best = (x - run_start, run_start)
                        run_start = None
            if run_start is not None and w - run_start > best[0]:
                best = (w - run_start, run_start)

        if best[0] >= NESTED_W // 2:
            run_w, run_left = best
            scale = run_w / NESTED_W
            out_w = max(1, int(NESTED_W * scale))
            out_h = max(1, int(NESTED_H * scale))
            left = run_left - (out_w - run_w) // 2
            top = max(0, (h - out_h) // 2)
            self.rect = (left, top, out_w, out_h)
            return self.rect

        raise RuntimeError("nested window not found (no wallpaper pixels)")

    def still(self, full_path, name):
        rx, ry, rw, rh = self.rect
        im = Image.open(full_path).convert("RGB")
        crop = im.crop((rx, ry, rx + rw, ry + rh))
        dest = os.path.join(self.outdir, name)
        crop.save(dest)
        log(f"saved {dest} ({crop.width}x{crop.height})")
        return dest


def run(args):
    synth = Synthetic(args.synth)
    cap = Capturer(args.outdir, args.scratch)
    prefix = args.prefix

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

    if args.reduced:
        synth.send("set reduced-motion on")
        time.sleep(0.3)
        log("reduced motion enabled")

    def wait_for(pred, timeout=5.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if pred():
                return True
            time.sleep(0.05)
        return False

    # --- 1. open the switcher: hold Cmd, press Tab ------------------------
    synth.send(f"key {KEY_LEFTMETA} down")
    synth.send(f"key {KEY_TAB} down")
    if not wait_for(lambda: synth.switcher()["active"]):
        raise RuntimeError("the switcher never opened")
    time.sleep(0.4)
    opened = synth.switcher()
    log(f"switcher open apps={opened['entries']} previews={len(opened['previews'])}")
    still = cap.full("switcher")
    cap.still(still, f"{prefix}.png")

    # --- 2. cycle the selection forward -----------------------------------
    synth.send(f"key {KEY_TAB} up")
    synth.send(f"key {KEY_TAB} down")
    time.sleep(0.4)
    cycled = synth.switcher()
    log(f"switcher cycled selected={cycled.get('selected')} app={cycled.get('app')}")
    still = cap.full("switcher-cycled")
    cap.still(still, f"{prefix}-cycled.png")
    synth.send(f"key {KEY_TAB} up")

    # --- 3. cycle windows within the selected app (Cmd+`, T-06.2b) --------
    # The compositor owns the chord and the one machine; the live preview
    # follows the window cursor. With a single window for the app this is a
    # no-op, but the stills still capture the chord's settled frame.
    synth.send(f"key {KEY_GRAVE} down")
    synth.send(f"key {KEY_GRAVE} up")
    time.sleep(0.4)
    window_cycled = synth.switcher()
    log(
        "switcher window-cycled selected="
        f"{window_cycled.get('selected')} window={window_cycled.get('window')}"
    )
    still = cap.full("switcher-window-cycled")
    cap.still(still, f"{prefix}-window-cycled.png")

    # --- 4. release Cmd: commit and close ---------------------------------
    synth.send(f"key {KEY_LEFTMETA} up")
    wait_for(lambda: not synth.switcher()["active"])
    time.sleep(0.3)
    log("switcher committed and closed")

    log("switcher capture complete")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument(
        "--prefix",
        default="t06-app-switcher",
        help="output still name prefix (default: t06-app-switcher)",
    )
    parser.add_argument(
        "--reduced",
        action="store_true",
        help="drive the walkthrough with accessibility.reduceMotion on",
    )
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())