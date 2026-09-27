#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-14.7c Dock motion trace.

Driven by ``scripts/capture-dock-motion.sh`` (run that, not this). Requires a
host Wayland session, ``spectacle``, ``gdbus`` and Pillow.

It brackets a scripted Dock motion sequence with compositor ``SIGUSR1``
render-stat dumps and reports the frames-rendered delta and the degrade/frame
budget lines: a magnification sweep, a context menu open/close, and a
``dock.size`` change. A trailing idle window proves the Dock contributes no
frames once the motion settles.
"""
import argparse
import os
import signal
import socket
import subprocess
import sys
import time

from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
WALL = (33, 13, 41)
WALL_TOL = 3
WALL_ROW_MIN = 200

DOCK_Y = 1155
DOCK_LEFT_X = 785
DOCK_CENTER_X = 960
DOCK_RIGHT_X = 1135
DOCK_BAND = (560, 1000, 1360, 1200)

BTN_LEFT = 272
BTN_RIGHT = 273
KEY_ESC = 1


def log(message):
    print(f"capture-dock-motion: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-motion-{os.getpid()}.sock"
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

    def button(self, code, down):
        self.send(f"button {code} {'down' if down else 'up'}")

    def key(self, code, down):
        self.send(f"key {code} {'down' if down else 'up'}")


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

    def dock_still(self, full_path, name):
        rx, ry, _rw, _rh = self.rect
        im = Image.open(full_path).convert("RGB")
        x0, y0, x1, y1 = DOCK_BAND
        crop = im.crop((rx + x0, ry + y0, rx + x1, ry + y1))
        dest = os.path.join(self.outdir, name)
        crop.save(dest)
        log(f"saved {dest} ({crop.width}x{crop.height})")
        return dest


def latest_lines(log_path, needle):
    found = None
    try:
        with open(log_path) as handle:
            for line in handle:
                if needle in line:
                    found = line.strip()
    except FileNotFoundError:
        return None
    return found


def parse_render_stats(line):
    out = {}
    for token in line.split(": ", 1)[-1].split():
        if "=" in token:
            key, value = token.split("=", 1)
            try:
                out[key] = int(value)
            except ValueError:
                out[key] = value
    return out


def gdbus_set(dest, path, iface, key, value):
    subprocess.run(
        ["gdbus", "call", "--session", "--dest", dest, "--object-path", path,
         "--method", f"{iface}.Set", key, value],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def dump(pid):
    os.kill(pid, signal.SIGUSR1)
    time.sleep(0.5)


def snapshot(pid, log_path):
    dump(pid)
    render = parse_render_stats(latest_lines(log_path, "render stats ("))
    degrade = parse_render_stats(latest_lines(log_path, "degrade stats ("))
    return render, degrade


def phase_line(name, elapsed, before, after):
    before_r, before_d = before
    after_r, after_d = after
    if not before_r or not after_r:
        return f"{name}: could not read render stats"
    frames = after_r["frames_rendered"] - before_r["frames_rendered"]
    skipped = after_r["frames_skipped_no_damage"] - before_r["frames_skipped_no_damage"]
    stepped = after_r["animation_frames_stepped"] - before_r["animation_frames_stepped"]
    over = after_d.get("over_budget", 0) - before_d.get("over_budget", 0) \
        if before_d and after_d else -1
    fps = frames / elapsed if elapsed > 0 else 0.0
    return (f"{name} {elapsed:.2f}s: frames_rendered=+{frames} "
            f"frames_skipped_no_damage=+{skipped} "
            f"animation_frames_stepped=+{stepped} over_budget=+{over} "
            f"~{fps:.0f} fps")


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

    if not args.compositor_pid:
        log("no compositor pid; cannot record the trace")
        return 1

    phases = []

    # --- Magnification sweep under the spring ------------------------------
    before = snapshot(args.compositor_pid, args.demo_log)
    start = time.monotonic()
    for _ in range(2):
        for x in range(DOCK_LEFT_X, DOCK_RIGHT_X + 1, 20):
            synth.motion(x, DOCK_Y)
            time.sleep(0.02)
        for x in range(DOCK_RIGHT_X, DOCK_LEFT_X - 1, -20):
            synth.motion(x, DOCK_Y)
            time.sleep(0.02)
    elapsed = time.monotonic() - start
    after = snapshot(args.compositor_pid, args.demo_log)
    phases.append(phase_line("sweep", elapsed, before, after))

    # --- A context menu opens and closes over the centre entry -------------
    before = snapshot(args.compositor_pid, args.demo_log)
    start = time.monotonic()
    synth.motion(DOCK_CENTER_X, DOCK_Y)
    time.sleep(0.2)
    synth.button(BTN_RIGHT, True)
    synth.button(BTN_RIGHT, False)
    time.sleep(0.35)
    menu_full = cap.full("dock-motion-menu-dark")
    cap.dock_still(menu_full, "t14-dock-motion-menu-dark.png")
    synth.key(KEY_ESC, True)
    synth.key(KEY_ESC, False)
    time.sleep(0.3)
    elapsed = time.monotonic() - start
    after = snapshot(args.compositor_pid, args.demo_log)
    phases.append(phase_line("popover", elapsed, before, after))

    # --- A dock.size change animates the icon size then settles ------------
    before = snapshot(args.compositor_pid, args.demo_log)
    start = time.monotonic()
    gdbus_set(args.dbus_dest, args.dbus_path, args.dbus_iface, "dock.size", "<0.85>")
    time.sleep(0.7)
    resized_full = cap.full("dock-motion-resized-dark")
    cap.dock_still(resized_full, "t14-dock-motion-resized-dark.png")
    gdbus_set(args.dbus_dest, args.dbus_path, args.dbus_iface, "dock.size", "<0.5>")
    time.sleep(0.5)
    synth.motion(20, 20)
    time.sleep(0.3)
    elapsed = time.monotonic() - start
    after = snapshot(args.compositor_pid, args.demo_log)
    phases.append(phase_line("size-change", elapsed, before, after))

    # --- Idle window: the settled Dock must contribute no frames -----------
    before = snapshot(args.compositor_pid, args.demo_log)
    idle_start = time.monotonic()
    time.sleep(1.5)
    idle_elapsed = time.monotonic() - idle_start
    after = snapshot(args.compositor_pid, args.demo_log)
    phases.append(phase_line("idle", idle_elapsed, before, after))

    lines = ["T-14.7c Dock motion trace",
             "magnification sweep, context menu open/close, "
             "dock.size 0.5 -> 0.85 -> 0.5, then idle"]
    lines.extend(phases)
    lines.append(f"render stats (final): {latest_lines(args.demo_log, 'render stats (')}")
    lines.append(f"degrade stats (final): {latest_lines(args.demo_log, 'degrade stats (')}")
    lines.append(f"frame timing (final): {latest_lines(args.demo_log, 'frame timing (')}")

    dest = os.path.join(args.outdir, "t14-dock-motion-trace.txt")
    with open(dest, "w") as handle:
        handle.write("\n".join(lines) + "\n")
    log(f"wrote {dest}")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--demo-log", required=True)
    parser.add_argument("--compositor-pid", type=int, default=0)
    parser.add_argument("--dbus-dest", default="org.dragonfruit.Settings1")
    parser.add_argument("--dbus-path", default="/org/dragonfruit/Settings1")
    parser.add_argument("--dbus-iface", default="org.dragonfruit.Settings1")
    args = parser.parse_args()
    try:
        return run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1


if __name__ == "__main__":
    sys.exit(main())