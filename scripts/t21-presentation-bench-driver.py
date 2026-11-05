#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the T-21.4b presentation benchmark on a running nested session.

Machine half of ``scripts/t21-presentation-bench.sh``. It talks to the
compositor's synthetic-input harness (``DRAGONFRUIT_SYNTHETIC_INPUT``), runs
the benchmark scenarios, and brackets each one with a ``SIGUSR1`` snapshot on
both the shell and the compositor. The shell script turns the numbered
``SIGUSR1-<n>`` dumps into per-scenario deltas in the committed report.

Modes:

* ``full`` — warm-up, idle (default 60 s), Dock sweep (default 10 s),
  menu-bar open/close, Dock popover open/close, one app launch, Mission
  Control with 10+ windows, and a fullscreen client. The optional live
  visual stills are taken after the measured scenarios.
* ``sweep`` — one Dock sweep at the requested material tier. Used by the
  refresh matrix and by the material-tier rows.
* ``tiers`` — one session that runs a Dock sweep at Full, Reduced, and
  Minimal, with a baseline snapshot before each sweep so the shell's
  interval-windowed timing rings describe that tier only.

Coordinates are normalized (``motion-abs``) so the same driver works at the
nested window's actual size. X11 fixture windows use small fixed pixel
geometry that fits both the 1280x800 and 1920x1200 nested outputs.

Requires a host Wayland session and the built tree.
"""
import argparse
import glob
import os
import shutil
import signal
import socket
import subprocess
import sys
import time

BTN_LEFT = 272
KEY_ESCAPE = 1
KEY_LEFTCTRL = 29
KEY_LEFTSHIFT = 42
KEY_UP = 103
KEY_LEFTMETA = 125
KEY_3 = 4

# The Dock plate sits at the bottom of the output; the sweep stays inside the
# resting plate so it never turns into an app launch.
DOCK_Y = 0.96
DOCK_LEFT_X = 0.38
DOCK_RIGHT_X = 0.63
MENUBAR_X = 0.012
MENUBAR_Y = 0.012
DESKTOP_X = 0.05
DESKTOP_Y = 0.50


def log(message):
    print(f"t21-presentation-bench: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        self.client = f"{runtime}/dragonfruit-t21-bench-{os.getpid()}.sock"
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
        self.send(f"motion-abs {x:.6f} {y:.6f}")

    def click(self, x, y, button=BTN_LEFT):
        self.motion(x, y)
        time.sleep(0.15)
        self.send(f"button {button} down")
        self.send(f"button {button} up")
        time.sleep(0.25)

    def key(self, code):
        self.send(f"key {code} down")
        self.send(f"key {code} up")
        time.sleep(0.15)

    def key_combo(self, *codes):
        for code in codes:
            self.send(f"key {code} down")
            time.sleep(0.08)
        for code in reversed(codes):
            self.send(f"key {code} up")
            time.sleep(0.08)

    def identities(self):
        out = {}
        for line in self.query("query identity").splitlines():
            parts = line.split(" ", 3)
            if len(parts) >= 3 and parts[0] == "identity":
                out[int(parts[1])] = parts[2]
        return out

    def grid(self):
        report = {"progress": 0.0, "windows": 0}
        for line in self.query("query grid").splitlines():
            parts = line.split()
            if not parts or parts[0] != "grid":
                continue
            if len(parts) > 1 and "=" in parts[1]:
                for field in parts[1:]:
                    key, _, value = field.partition("=")
                    if key == "progress":
                        report["progress"] = float(value)
            elif len(parts) > 1 and parts[1] == "window":
                report["windows"] += 1
        return report

    def decorations(self):
        windows = []
        for line in self.query("query decorations").splitlines():
            parts = line.split()
            if parts and parts[0] == "decoration" and len(parts) > 1:
                try:
                    windows.append(int(parts[1]))
                except ValueError:
                    pass
        return windows

    def close(self):
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass


class Windows:
    """Spawn real X11 windows on the nested Xwayland display."""

    def __init__(self, display):
        self.display = display
        self.children = []

    def spawn(self, index):
        env = dict(os.environ)
        env["DISPLAY"] = self.display
        # Four columns by three rows fits 1280x800 and 1920x1200; the overview
        # lays the windows out itself, so the exact position is not evidence.
        x = 40 + (index % 4) * 280
        y = 120 + (index // 4) * 180
        title = f"T21 bench window {index}"
        child = subprocess.Popen(
            [
                "xmessage",
                "-geometry",
                f"240x120+{x}+{y}",
                "-title",
                title,
                title,
            ],
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        self.children.append(child)
        return child

    def close(self):
        for child in self.children:
            try:
                child.terminate()
            except ProcessLookupError:
                pass
        for child in self.children:
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                child.kill()


def wait_for(predicate, timeout, description):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if predicate():
            return True
        time.sleep(0.1)
    log(f"timed out waiting for {description}")
    return False


def newest_screenshot(directories):
    candidates = []
    for directory in directories:
        candidates.extend(glob.glob(os.path.join(directory, "Screenshot_*.png")))
    if not candidates:
        return None
    return max(candidates, key=os.path.getmtime)


def capture_still(synth, screenshot_dir, dest, click_x, click_y):
    """Capture the nested output through the trusted compositor path."""
    os.makedirs(screenshot_dir, exist_ok=True)
    before = newest_screenshot(
        [screenshot_dir, os.path.expanduser("~/Pictures/Screenshots")]
    )
    synth.key_combo(KEY_LEFTMETA, KEY_LEFTSHIFT, KEY_3)
    time.sleep(1.5)
    synth.click(click_x, click_y)
    # The shell writes the delivered PNG after the compositor's capture; wait
    # for the size to settle before moving it, or the move can catch a
    # half-written file.
    time.sleep(1.5)
    saved = None
    for _ in range(80):
        candidate = newest_screenshot(
            [screenshot_dir, os.path.expanduser("~/Pictures/Screenshots")]
        )
        if candidate and candidate != before:
            try:
                first_size = os.path.getsize(candidate)
                time.sleep(0.2)
                second_size = os.path.getsize(candidate)
            except OSError:
                time.sleep(0.1)
                continue
            if first_size > 0 and first_size == second_size:
                saved = candidate
                break
        time.sleep(0.1)
    if not saved:
        log(f"no screenshot appeared for {dest}")
        return None
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    shutil.move(saved, dest)
    log(f"saved {dest}")
    return dest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--socket", required=True)
    parser.add_argument("--shell-pid", type=int, required=True)
    parser.add_argument("--compositor-pid", type=int, required=True)
    parser.add_argument("--screenshot-dir", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("--shell-log", required=True)
    parser.add_argument("--outdir", default="docs/captures")
    parser.add_argument("--mode", choices=("full", "sweep", "tiers"), default="full")
    parser.add_argument("--idle-s", type=float, default=60.0)
    parser.add_argument("--sweep-s", type=float, default=10.0)
    parser.add_argument("--tier", default="full")
    parser.add_argument("--skip-visual", action="store_true")
    args = parser.parse_args()

    synth = Synthetic(args.synth)
    runtime = os.environ.get("XDG_RUNTIME_DIR", "/run/user")
    display_file = os.path.join(runtime, f"{args.socket}.x11-display")
    display = ""
    if os.path.exists(display_file):
        with open(display_file, encoding="utf-8") as handle:
            display = handle.read().strip()
    windows = Windows(display) if display else None
    if args.mode == "full" and windows is None:
        log("no nested Xwayland DISPLAY hand-off; app-launch scenarios will fail")
        synth.close()
        return 1

    sequence = 0
    last_snapshot = time.time()
    failures = 0

    def snapshot(name, required=True):
        nonlocal sequence, last_snapshot, failures
        sequence += 1
        label = f"SIGUSR1-{sequence}"
        for pid in (args.shell_pid, args.compositor_pid):
            try:
                os.kill(pid, signal.SIGUSR1)
            except ProcessLookupError:
                log(f"pid {pid} disappeared before snapshot {name}")
                failures += 1
                return label
        # Wait until both dumps have landed before the next scenario, so two
        # signals can never coalesce into one self-pipe/notifier wakeup.
        needle_shell = f"dragonfruit-shell: chrome stats ({label})"
        needle_comp = f"dragonfruit-compositor: chrome stats ({label})"
        for _ in range(200):
            try:
                with open(args.shell_log, encoding="utf-8", errors="replace") as handle:
                    shell_text = handle.read()
            except OSError:
                shell_text = ""
            try:
                with open(args.log, encoding="utf-8", errors="replace") as handle:
                    comp_text = handle.read()
            except OSError:
                comp_text = ""
            if needle_shell in shell_text and needle_comp in comp_text:
                break
            time.sleep(0.1)
        else:
            log(f"snapshot {label} did not produce both dumps")
            if required:
                failures += 1
        time.sleep(0.2)
        now = time.time()
        log(f"scenario {sequence} {name} elapsed={now - last_snapshot:.3f}")
        last_snapshot = now
        return label

    def sweep(duration_s):
        steps = max(20, int(duration_s * 20))
        for index in range(steps):
            x = DOCK_LEFT_X + (DOCK_RIGHT_X - DOCK_LEFT_X) * index / (steps - 1)
            synth.motion(x, DOCK_Y)
            time.sleep(duration_s / steps)

    def set_tier(tier):
        synth.send(f"set degrade-tier {tier}")
        time.sleep(0.5)
        material = synth.query("query material")
        degrade = synth.query("query degrade")
        log(f"tier {tier}")
        for line in material.strip().splitlines():
            log(line)
        for line in degrade.strip().splitlines():
            log(line)

    try:
        # Let the shell finish its startup commits and the first GPU producer
        # warm-up. The first snapshot is the baseline and clears the shell's
        # interval-windowed timing rings.
        time.sleep(2.0)
        time.sleep(1.0)

        if args.mode == "sweep":
            set_tier(args.tier)
            # Discard the startup window: the shell's commit/frame-callback
            # rings include the first render and the GPU producer warm-up.
            snapshot("warmup", required=False)
            snapshot("baseline")
            sweep(args.sweep_s)
            time.sleep(0.4)
            snapshot("sweep")
        elif args.mode == "tiers":
            snapshot("warmup", required=False)
            for tier in ("full", "reduced", "minimal"):
                set_tier(tier)
                snapshot(f"{tier}-baseline")
                sweep(args.sweep_s)
                time.sleep(0.4)
                snapshot(f"{tier}-sweep")
        else:
            set_tier(args.tier)
            snapshot("warmup", required=False)
            snapshot("baseline")

            # 1. Idle: the requested window with no synthetic input.
            time.sleep(args.idle_s)
            snapshot("idle")

            # 2. Dock sweep.
            sweep(args.sweep_s)
            time.sleep(0.4)
            snapshot("dock-sweep")

            # 2b. Settled idle: the Dock spring has stopped and the chrome is
            # back at rest. This is the parent spec's zero-wakeup window,
            # distinct from the pre-animation idle above.
            time.sleep(args.idle_s)
            snapshot("idle-settled")

            # 3. Menu-bar dropdown open/close.
            synth.click(MENUBAR_X, MENUBAR_Y)
            time.sleep(0.8)
            snapshot("menubar-open")
            synth.click(DESKTOP_X, DESKTOP_Y)
            time.sleep(0.8)
            snapshot("menubar-close")

            # 4. Dock popover open/close. The chooser fixture opens the real
            # window chooser once a grouped app exists; two X11 windows are
            # enough. Click-away on empty desktop space dismisses it.
            windows.spawn(0)
            windows.spawn(1)
            wait_for(lambda: len(synth.decorations()) >= 2, 5.0, "two X11 windows")
            time.sleep(1.5)
            snapshot("dock-popover-open")
            synth.click(DESKTOP_X, DESKTOP_Y)
            time.sleep(0.8)
            snapshot("dock-popover-close")

            # 5. One app launch (a third X11 window maps).
            before = len(synth.decorations())
            windows.spawn(2)
            wait_for(lambda: len(synth.decorations()) > before, 8.0, "the launched window")
            time.sleep(1.0)
            snapshot("app-launch")

            # 6. Mission Control with 10+ windows.
            for index in range(3, 10):
                windows.spawn(index)
            wait_for(lambda: len(synth.decorations()) >= 10, 10.0, "ten X11 windows")
            time.sleep(1.0)
            synth.key_combo(KEY_LEFTCTRL, KEY_UP)
            wait_for(lambda: synth.grid()["progress"] >= 0.999, 8.0, "Mission Control")
            time.sleep(0.5)
            grid = synth.grid()
            log(f"mission-control windows={grid['windows']}")
            snapshot("mission-control")
            synth.key_combo(KEY_LEFTCTRL, KEY_UP)
            wait_for(lambda: synth.grid()["progress"] <= 0.001, 8.0, "Mission Control close")
            time.sleep(0.5)
            snapshot("mission-control-close")

            # 7. Fullscreen client. The persistent chrome hides and the
            # sampled material pass stops; this is the direct-scanout policy
            # scenario, not a scanout counter claim on the nested backend.
            window_ids = synth.decorations()
            if window_ids:
                synth.send(f"fullscreen {window_ids[0]}")
                time.sleep(1.5)
                snapshot("fullscreen")
                synth.send(f"unfullscreen {window_ids[0]}")
                time.sleep(1.0)
                snapshot("fullscreen-exit")
            else:
                log("no window to fullscreen; fullscreen scenario skipped")
                failures += 1

            # 8. Live visual stills, after the measured scenarios. The capture
            # itself goes through the compositor's trusted screenshot path.
            if not args.skip_visual:
                synth.click(DESKTOP_X, DESKTOP_Y)
                time.sleep(0.4)
                synth.motion(0.5, DOCK_Y)
                time.sleep(0.8)
                for index in range(24):
                    x = DOCK_LEFT_X + (DOCK_RIGHT_X - DOCK_LEFT_X) * index / 23
                    synth.motion(x, DOCK_Y)
                    time.sleep(0.05)
                time.sleep(0.3)
                capture_still(
                    synth,
                    args.screenshot_dir,
                    os.path.join(args.outdir, "t21-presentation-bench-dock.png"),
                    0.5,
                    0.975,
                )
                synth.key_combo(KEY_LEFTCTRL, KEY_UP)
                wait_for(
                    lambda: synth.grid()["progress"] >= 0.999,
                    8.0,
                    "Mission Control for the still",
                )
                time.sleep(0.5)
                capture_still(
                    synth,
                    args.screenshot_dir,
                    os.path.join(
                        args.outdir, "t21-presentation-bench-mission-control.png"
                    ),
                    0.5,
                    0.5,
                )
                synth.key_combo(KEY_LEFTCTRL, KEY_UP)
                time.sleep(0.5)
    finally:
        if windows is not None:
            windows.close()
        synth.close()

    if failures:
        log(f"{failures} scenario(s) failed to snapshot")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
