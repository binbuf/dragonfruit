#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested T-01 demo with synthetic input and capture stills.

This is the machine half of the T-01.6b walkthrough
(docs/design/tracks/01-loop-v0-window-controls.md): it performs the demo
checklist on the *running* nested session — focus, window menu, zoom,
minimize, restore-from-Dock, close — through the real compositor input path
(the T-03 synthetic-input harness, installed on the nested backend only when
``DRAGONFRUIT_SYNTHETIC_INPUT`` is set), and screenshots each step.

It is driven by ``scripts/capture-demo.sh``; run that, not this. Requires a
host Wayland session, ``spectacle`` (screenshot tool), and Pillow.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
# Space 0's default wallpaper (compositor/src/workspace/mod.rs), used to find
# the nested window on the host screen.
WALL = (33, 13, 41)
# Traffic-light geometry from component.trafficLights tokens (T-01.2).
LIGHT_DIAMETER, LIGHT_GAP, LIGHT_INSET = 12, 8, 12
BTN_LEFT, BTN_RIGHT = 272, 273
KEY_ESCAPE = 1


def log(message):
    print(f"capture: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-capture-{os.getpid()}.sock"
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

    def decorations(self):
        rows = []
        for line in self.query("query decorations").splitlines():
            p = line.split()
            if p and p[0] == "decoration":
                rows.append(
                    {
                        "window": int(p[1]),
                        "ssd": p[2] == "1",
                        "tb": tuple(int(v) for v in p[3:7]),
                        "content": tuple(int(v) for v in p[7:11]),
                        "state": p[11],
                        "space": int(p[12]),
                    }
                )
        return rows

    def window_menu_open(self):
        first = self.query("query window-menu").splitlines()[0].split()
        return first[1] == "1" if len(first) > 1 else False

    def motion(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")

    def click(self, x, y, button=BTN_LEFT):
        self.motion(x, y)
        self.send(f"button {button} down")
        self.send(f"button {button} up")
        time.sleep(0.25)

    def double_click(self, x, y):
        self.motion(x, y)
        for _ in range(2):
            self.send(f"button {BTN_LEFT} down")
            self.send(f"button {BTN_LEFT} up")
            time.sleep(0.08)
        time.sleep(0.25)

    def key(self, code):
        self.send(f"key {code} down")
        self.send(f"key {code} up")
        time.sleep(0.2)

    def wait_state(self, window, states, timeout=5.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            for row in self.decorations():
                if row["window"] == window and row["state"] in states:
                    return row
            time.sleep(0.1)
        raise RuntimeError(f"window {window} never reached {states}: {self.decorations()}")


def light_center(tb, index):
    tx, ty, _tw, th = tb
    return (
        tx + LIGHT_INSET + LIGHT_DIAMETER // 2 + index * (LIGHT_DIAMETER + LIGHT_GAP),
        ty + th // 2,
    )


def titlebar_center(tb):
    return (tb[0] + tb[2] // 2, tb[1] + tb[3] // 2)


class Capturer:
    def __init__(self, synth, outdir, scratch):
        self.synth = synth
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
        minx, miny = w, h
        for y in range(0, h, 2):
            for x in range(0, w, 2):
                c = px[x, y]
                if (
                    abs(c[0] - WALL[0]) <= 3
                    and abs(c[1] - WALL[1]) <= 3
                    and abs(c[2] - WALL[2]) <= 3
                ):
                    minx = min(minx, x)
                    miny = min(miny, y)
        if miny >= h:
            raise RuntimeError("nested window not found (no wallpaper pixels)")
        # The wallpaper starts below the 28px menu bar; the window top is ~30px
        # above the first wallpaper pixel.
        self.rect = (minx, miny - 30, NESTED_W, NESTED_H)
        return self.rect

    def still(self, full_path, name, box=None):
        """Save a still: the nested window view by default, or `box` (nested
        logical coords) cropped out of the full screen."""
        rx, ry, _rw, _rh = self.rect
        im = Image.open(full_path).convert("RGB")
        if box is None:
            crop = im.crop((rx, ry, rx + NESTED_W, ry + NESTED_H))
        else:
            x, y, w, h = box
            crop = im.crop((rx + x, ry + y, rx + x + w, ry + y + h))
        dest = os.path.join(self.outdir, name)
        crop.save(dest)
        log(f"saved {dest} ({crop.width}x{crop.height})")
        return dest


def run(args):
    synth = Synthetic(args.synth)
    cap = Capturer(synth, args.outdir, args.scratch)
    steps = []

    # --- 1. the mapped loop ------------------------------------------------
    init = cap.full("initial")
    cap.detect_rect(init)
    log(f"nested window rect {cap.rect}")
    rows = synth.decorations()
    settings = max(rows, key=lambda r: r["content"][2] * r["content"][3])
    x11 = min(rows, key=lambda r: r["content"][2] * r["content"][3])
    log(f"Wayland/SSD window {settings['window']} tb={settings['tb']}")
    log(f"X11 window {x11['window']} tb={x11['tb']}")
    steps.append(("initial: both windows mapped, Dock running indicators", init))
    cap.still(init, "t01-loop-v0.png")

    tc = titlebar_center(settings["tb"])

    # --- 2. focus: menu-bar app name updates -------------------------------
    synth.click(*tc)
    time.sleep(0.5)
    focused = cap.full("focused")
    steps.append(("focused: clicked the Wayland titlebar; menu bar shows its app", focused))
    cap.still(focused, "t01-loop-v0-wayland.png", box=(
        settings["content"][0] - 8,
        settings["content"][1] - 40 - 8,
        settings["content"][2] + 16,
        settings["content"][3] + 40 + 16,
    ))
    cap.still(focused, "t01-loop-v0-x11.png", box=(
        x11["content"][0] - 8,
        x11["content"][1] - 40 - 8,
        x11["content"][2] + 16,
        x11["content"][3] + 40 + 16,
    ))
    dock_band = (700, NESTED_H - 140, 560, 140)
    cap.still(focused, "t01-loop-v0-dock.png", box=dock_band)
    titlebar_box = (
        settings["tb"][0] - 10,
        settings["tb"][1] - 6,
        settings["tb"][2] + 20,
        settings["tb"][3] + 12,
    )
    cap.still(focused, "t01-loop-v0-titlebar.png", box=titlebar_box)

    # --- 3. window menu ----------------------------------------------------
    synth.click(*tc, button=BTN_RIGHT)
    if not synth.window_menu_open():
        raise RuntimeError("right-click on the titlebar did not open the window menu")
    menu = cap.full("menu")
    steps.append(("window menu: right-click on the titlebar opens it", menu))
    cap.still(menu, "t01-loop-v0-menu.png", box=(900, 240, 360, 220))
    synth.key(KEY_ESCAPE)
    time.sleep(0.3)

    # --- 4. zoom (double-click) -------------------------------------------
    synth.double_click(*tc)
    zoomed = synth.wait_state(settings["window"], ("zoomed",))
    time.sleep(0.3)
    zfull = cap.full("zoomed")
    steps.append(("zoomed: double-click on the titlebar fills the usable area", zfull))
    cap.still(zfull, "t01-loop-v0-zoomed.png")
    synth.double_click(*titlebar_center(zoomed["tb"]))
    synth.wait_state(settings["window"], ("floating",))
    time.sleep(0.3)

    # --- 5. minimize: the Dock gains a minimized entry ---------------------
    settings = synth.wait_state(settings["window"], ("floating",))
    synth.click(*light_center(settings["tb"], 1))
    synth.wait_state(settings["window"], ("minimized",))
    time.sleep(0.4)
    mini = cap.full("minimized")
    steps.append(("minimized: yellow light; the window's Dock entry collapses", mini))
    cap.still(mini, "t01-loop-v0-minimized.png")
    cap.still(mini, "t01-loop-v0-dock-minimized.png", box=dock_band)

    # --- 6. restore from the Dock -----------------------------------------
    # The minimized entry appears to the right of the app's running tile, and
    # the centre-anchored Dock shifts the entries after it. Probe the changed
    # columns in the Dock band and click the first one that restores.
    base = Image.open(os.path.join(cap.scratch, "focused.png")).convert("RGB")
    mini_im = Image.open(mini).convert("RGB")
    rx, ry, _rw, _rh = cap.rect
    dy0, dy1 = ry + NESTED_H - 150, ry + NESTED_H
    b = base.crop((rx, dy0, rx + NESTED_W, dy1))
    m = mini_im.crop((rx, dy0, rx + NESTED_W, dy1))
    bp, mp = b.load(), m.load()
    changed = []
    for x in range(NESTED_W):
        diff = 0
        for y in range(0, dy1 - dy0, 3):
            c1, c2 = bp[x, y], mp[x, y]
            diff += abs(c1[0] - c2[0]) + abs(c1[1] - c2[1]) + abs(c1[2] - c2[2])
        changed.append(diff)
    runs, start = [], None
    for x, d in enumerate(changed + [0]):
        if d > 60 and start is None:
            start = x
        elif d <= 60 and start is not None:
            runs.append((x - start, start, x))
            start = None
    runs.sort(reverse=True)
    log(f"dock changed runs {runs[:3]}")
    restored = None
    for _width, cstart, cend in runs[:2]:
        for frac in (0.25, 0.5, 0.75):
            x = cstart + int((cend - cstart) * frac)
            synth.click(x, NESTED_H - 60)
            time.sleep(0.5)
            row = next(
                (r for r in synth.decorations() if r["window"] == settings["window"]),
                None,
            )
            if row and row["state"] != "minimized":
                restored = row
                break
        if restored:
            break
    if restored is None:
        raise RuntimeError("restore-from-Dock did not take")
    time.sleep(0.3)
    rest = cap.full("restored")
    steps.append(("restored: the Dock click brings the window back and focuses it", rest))
    cap.still(rest, "t01-loop-v0-restored.png")

    # --- 7. close: the Dock entry resolves ---------------------------------
    row = next(r for r in synth.decorations() if r["window"] == settings["window"])
    synth.click(*light_center(row["tb"], 0))
    deadline = time.time() + 5
    while time.time() < deadline:
        if not any(r["window"] == settings["window"] for r in synth.decorations()):
            break
        time.sleep(0.1)
    time.sleep(0.4)
    closed = cap.full("closed")
    remaining = [r["window"] for r in synth.decorations()]
    steps.append(("closed: red light closes the window; its Dock entry is gone", closed))
    cap.still(closed, "t01-loop-v0-closed.png")
    log(f"windows remaining after close: {remaining}")

    with open(os.path.join(cap.scratch, "walkthrough.txt"), "w") as handle:
        for label, path in steps:
            handle.write(f"{label}\n  {path}\n")
    log("walkthrough complete")
    return steps


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
