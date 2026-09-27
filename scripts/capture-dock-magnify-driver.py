#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-14.7b Dock magnification sweep and its frame budget.

Driven by ``scripts/capture-dock-magnify.sh`` (run that, not this). Requires a
host Wayland session, ``spectacle``, and Pillow.

For each colour scheme it parks the pointer over the left end, the centre, and
the right end of the Dock, lets the `motion.dockMagnify` spring settle, and
saves a crop of the Dock band. It also brackets one timed pointer sweep with
compositor ``SIGUSR1`` render-stat dumps and reports the frames-rendered delta
against the elapsed time (the T-03 frame budget).
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
# The active Space wallpaper the compositor clears the nested output with
# (under the dark scheme the capture starts in). The host desktop is a
# different colour, so the nested window is the large region matching this.
WALL = (33, 13, 41)
WALL_TOL = 3
WALL_ROW_MIN = 200

# Dock geometry inside the nested output: the plate is centred on the bottom
# edge. The sweeps sit a little inside each end and at the centre; the pointer
# y is over the resting artwork.
DOCK_Y = 1155
DOCK_LEFT_X = 785
DOCK_CENTER_X = 960
DOCK_RIGHT_X = 1135
# The crop that shows the plate, its magnified artwork, and the gap to the
# screen edge.
DOCK_BAND = (560, 1000, 1360, 1200)  # x, y, x2, y2


def log(message):
    print(f"capture-dock-magnify: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-magnify-{os.getpid()}.sock"
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
        right = max(row[2] for row in rows)
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


def latest_render_stats(log_path):
    """The last render-stats line the compositor dumped into the demo log."""
    stats = None
    try:
        with open(log_path) as handle:
            for line in handle:
                if "render stats (" in line:
                    stats = line.strip()
    except FileNotFoundError:
        return None
    return stats


def parse_render_stats(line):
    out = {}
    for token in line.split(": ", 1)[-1].split():
        if "=" in token:
            key, value = token.split("=", 1)
            out[key] = int(value)
    return out


def frame_budget(synth, capturer, pid, log_path, outdir):
    """Bracket a timed sweep with SIGUSR1 dumps and report the delta."""
    if not pid:
        log("no compositor pid; skipping the frame-budget probe")
        return
    os.kill(pid, signal.SIGUSR1)
    time.sleep(0.5)
    before_line = latest_render_stats(log_path)
    synth.send("set color-scheme dark")
    start = time.monotonic()
    # Sweep left -> right -> left in small steps so the shell's spring runs for
    # the whole window.
    for _ in range(3):
        for x in range(DOCK_LEFT_X, DOCK_RIGHT_X + 1, 20):
            synth.motion(x, DOCK_Y)
            time.sleep(0.02)
        for x in range(DOCK_RIGHT_X, DOCK_LEFT_X - 1, -20):
            synth.motion(x, DOCK_Y)
            time.sleep(0.02)
    elapsed = time.monotonic() - start
    time.sleep(0.4)
    os.kill(pid, signal.SIGUSR1)
    time.sleep(0.5)
    after_line = latest_render_stats(log_path)
    if not before_line or not after_line:
        log(f"frame budget: could not read render stats ({before_line!r} / {after_line!r})")
        return
    before = parse_render_stats(before_line)
    after = parse_render_stats(after_line)
    frames = after["frames_rendered"] - before["frames_rendered"]
    skipped = after["frames_skipped_no_damage"] - before["frames_skipped_no_damage"]
    stepped = after["animation_frames_stepped"] - before["animation_frames_stepped"]
    fps = frames / elapsed if elapsed > 0 else 0.0
    summary = (
        f"sweep {elapsed:.2f}s: frames_rendered=+{frames} "
        f"frames_skipped_no_damage=+{skipped} "
        f"animation_frames_stepped=+{stepped} ~{fps:.0f} fps"
    )
    log(summary)
    with open(os.path.join(outdir, "t14-dock-magnify-frame-budget.txt"), "w") as handle:
        handle.write(summary + "\n")
        handle.write(f"before: {before_line}\n")
        handle.write(f"after:  {after_line}\n")


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

    for scheme in ("dark", "light"):
        synth.send(f"set color-scheme {scheme}")
        time.sleep(0.5)
        for name, x in (("left", DOCK_LEFT_X), ("center", DOCK_CENTER_X),
                        ("right", DOCK_RIGHT_X)):
            synth.motion(x, DOCK_Y)
            # Let the spring settle at the new anchor before the still.
            time.sleep(0.6)
            full = cap.full(f"dock-magnify-{name}-{scheme}")
            cap.dock_still(full, f"t14-dock-magnify-{name}-{scheme}.png")

    frame_budget(synth, cap, args.compositor_pid, args.demo_log, args.outdir)
    log("done")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--demo-log", required=True)
    parser.add_argument("--compositor-pid", type=int, default=0)
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())