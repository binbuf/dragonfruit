#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the T-21.1a chrome transport baseline on a nested session.

Machine half of ``scripts/t21-chrome-baseline.sh``. It talks to the
compositor's synthetic-input harness (``DRAGONFRUIT_SYNTHETIC_INPUT``), runs
the six baseline scenarios, and brackets each one with a SIGUSR1 snapshot on
both the shell and the compositor. The shell script turns the numbered
``SIGUSR1-<n>`` dumps into per-scenario deltas in the committed report.

Scenarios (each snapshot is taken after the scenario has settled):

* ``idle`` — five seconds with nothing animating.
* ``dock-sweep`` — a slow ``motion-abs`` sweep across the resting Dock.
* ``menubar-open`` / ``menubar-close`` — the system dropdown.
* ``dock-popover-open`` / ``dock-popover-close`` — the Dock window chooser,
  opened by the shell's capture-only ``DF_DOCK_CHOOSER_FIXTURE`` seam after
  two X11 windows exist (a real grouped entry).
* ``app-launch`` — one new X11 window maps.
* ``mission-control`` / ``mission-control-close`` — Ctrl+Up with 10+ windows.

The last block takes the live-visual stills through the trusted compositor
capture path (Cmd+Shift+3), so the artifact does not depend on a screencopy
client. Requires a host Wayland session and the built tree.
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

NESTED_W, NESTED_H = 1920, 1200
DOCK_Y = 1155
DOCK_LEFT_X = 730
DOCK_RIGHT_X = 1210
BTN_LEFT = 272
KEY_ESCAPE = 1
KEY_RETURN = 28
KEY_LEFTSHIFT = 42
KEY_LEFTCTRL = 29
KEY_UP = 103
KEY_LEFTMETA = 125
KEY_3 = 4


def log(message):
    print(f"t21-chrome-baseline: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        self.client = f"{runtime}/dragonfruit-t21-baseline-{os.getpid()}.sock"
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
        self.send(f"motion-abs {x / NESTED_W:.6f} {y / NESTED_H:.6f}")

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
        x = 80 + (index % 5) * 330
        y = 120 + (index // 5) * 240
        title = f"T21 baseline window {index}"
        child = subprocess.Popen(
            [
                "xmessage",
                "-geometry",
                f"300x150+{x}+{y}",
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
    before = newest_screenshot([screenshot_dir, os.path.expanduser("~/Pictures/Screenshots")])
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
    parser.add_argument("--skip-visual", action="store_true")
    args = parser.parse_args()

    synth = Synthetic(args.synth)
    runtime = os.environ.get("XDG_RUNTIME_DIR", "/run/user")
    display_file = os.path.join(runtime, f"{args.socket}.x11-display")
    display = ""
    if os.path.exists(display_file):
        with open(display_file, encoding="utf-8") as handle:
            display = handle.read().strip()
    if not display:
        log("no nested Xwayland DISPLAY hand-off; app-launch scenarios will fail")
        return 1
    windows = Windows(display)

    sequence = 0
    last_snapshot = time.time()

    def snapshot(name):
        nonlocal sequence, last_snapshot
        sequence += 1
        label = f"SIGUSR1-{sequence}"
        for pid in (args.shell_pid, args.compositor_pid):
            try:
                os.kill(pid, signal.SIGUSR1)
            except ProcessLookupError:
                log(f"pid {pid} disappeared before snapshot {name}")
                return
        # Wait until both dumps have landed before the next scenario, so two
        # signals can never coalesce into one self-pipe/notifier wakeup.
        needle_shell = f"dragonfruit-shell: chrome stats ({label})"
        needle_comp = f"dragonfruit-compositor: chrome stats ({label})"
        for _ in range(120):
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
        time.sleep(0.2)
        now = time.time()
        log(f"scenario {sequence} {name} elapsed={now - last_snapshot:.3f}")
        last_snapshot = now

    try:
        # Let the shell finish its startup commits before the baseline dump.
        # The extra second absorbs a host-driven focus/redraw (the nested
        # window being mapped or maximized), so the idle interval below starts
        # from a quiescent compositor rather than counting that one-off wake.
        time.sleep(1.5)
        time.sleep(1.0)
        snapshot("baseline")

        # 1. Idle: five seconds with no synthetic input.
        time.sleep(5.0)
        snapshot("idle")

        # 2. Slow Dock sweep: the same motion-abs path a real pointer takes.
        steps = 40
        for index in range(steps):
            x = DOCK_LEFT_X + (DOCK_RIGHT_X - DOCK_LEFT_X) * index / (steps - 1)
            synth.motion(x, DOCK_Y)
            time.sleep(0.05)
        time.sleep(0.4)
        snapshot("dock-sweep")

        # 3. Menu-bar dropdown open/close.
        synth.click(20, 14)
        time.sleep(0.8)
        snapshot("menubar-open")
        synth.click(960, 600)
        time.sleep(0.8)
        snapshot("menubar-close")

        # 4. Dock popover open/close. The chooser fixture opens the real
        # window chooser once a grouped app exists; two X11 windows are enough.
        windows.spawn(0)
        windows.spawn(1)
        wait_for(lambda: len(synth.identities()) >= 4, 8.0, "two X11 windows")
        time.sleep(1.5)
        snapshot("dock-popover-open")
        # Click-away on empty desktop space dismisses the popover (T-10
        # section 13). Keep the click outside the Dock plate so a widened or
        # reordered Dock cannot turn the dismissal into an app launch.
        synth.click(100, 600)
        time.sleep(0.8)
        snapshot("dock-popover-close")

        # 5. One app launch (a third X11 window maps).
        before = len(synth.identities())
        windows.spawn(2)
        wait_for(lambda: len(synth.identities()) > before, 8.0, "the launched window")
        time.sleep(1.0)
        snapshot("app-launch")

        # 6. Mission Control with 10+ windows.
        for index in range(3, 10):
            windows.spawn(index)
        wait_for(lambda: len(synth.identities()) >= 12, 15.0, "ten X11 windows")
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

        # 7. Live-visual stills, after the measured scenarios. The capture
        # itself goes through the compositor's trusted screenshot path.
        if not args.skip_visual:
            # Focus the desktop/chrome, not whatever window happens to sit at
            # the output centre, before the screenshot chord.
            synth.click(100, 600)
            time.sleep(0.4)
            # Dock visible at rest, then a second slow sweep.
            synth.motion(960, DOCK_Y)
            time.sleep(0.8)
            for index in range(24):
                x = DOCK_LEFT_X + (DOCK_RIGHT_X - DOCK_LEFT_X) * index / 23
                synth.motion(x, DOCK_Y)
                time.sleep(0.05)
            time.sleep(0.3)
            capture_still(synth, args.screenshot_dir,
                          os.path.join(args.outdir, "t21-chrome-baseline-dock.png"),
                          960, 1170)
            # Menu bar dropdown open.
            synth.click(20, 14)
            time.sleep(0.8)
            capture_still(synth, args.screenshot_dir,
                          os.path.join(args.outdir, "t21-chrome-baseline-menu.png"),
                          960, 600)
            synth.key(KEY_ESCAPE)
            time.sleep(0.3)
            # Mission Control with the full window set.
            synth.key_combo(KEY_LEFTCTRL, KEY_UP)
            wait_for(lambda: synth.grid()["progress"] >= 0.999, 8.0,
                     "Mission Control for the still")
            time.sleep(0.5)
            capture_still(synth, args.screenshot_dir,
                          os.path.join(args.outdir,
                                       "t21-chrome-baseline-mission-control.png"),
                          960, 600)
            synth.key_combo(KEY_LEFTCTRL, KEY_UP)
            time.sleep(0.5)
    finally:
        windows.close()
        synth.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
