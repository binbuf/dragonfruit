#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T-21.1c close-out live checks on a nested session.

Runs against an already-started `make demo` nested session with the
synthetic-input harness bound. It performs the T185a visual check, the shell
kill/restart check, and the lock-surface smoke test:

* visual: Dock sweep, menu-bar open/close, Dock popover open/close, and a
  still captured while the Dock is still moving;
* restart: `SIGKILL` the shell, wait for the compositor to reissue its launch
  token, relaunch the shell with the original environment plus the new token,
  and confirm the chrome surfaces map again;
* lock: the real Cmd+Ctrl+Q chord, `query lock` before/while locked, one typed
  character (password dots), and a lock still.

The stills go through the trusted compositor capture path (Cmd+Shift+3), not
a screencopy client. Requires a host Wayland session and the built tree.
"""
import argparse
import glob
import os
import shutil
import signal
import socket
import subprocess
import time

# Defaults for a 1920x1200 nested output; `apply_output_geometry()` replaces
# them from the compositor's `query scale` report so the same driver works on
# a 1280x800 host session or a maximized virtual output.
NESTED_W, NESTED_H = 1920, 1200
DOCK_Y = 1155
DOCK_LEFT_X = 730
DOCK_RIGHT_X = 1210
BTN_LEFT = 272
KEY_ESCAPE = 1
KEY_LEFTSHIFT = 42
KEY_LEFTCTRL = 29
KEY_UP = 103
KEY_LEFTMETA = 125
KEY_Q = 16
KEY_A = 30
KEY_3 = 4


def log(message):
    print(f"t21-closeout: {message}", flush=True)


def apply_output_geometry(report):
    """Take the output size and Dock plate rect from `query scale`."""
    global NESTED_W, NESTED_H, DOCK_Y, DOCK_LEFT_X, DOCK_RIGHT_X
    width, height = NESTED_W, NESTED_H
    dock = None
    for line in report.splitlines():
        parts = line.split()
        if len(parts) < 3 or parts[0] != "scale":
            continue
        if parts[1] == "output":
            for field in parts[2:]:
                key, sep, value = field.partition("=")
                if sep and key == "logical":
                    try:
                        w, h = value.split("x", 1)
                        width, height = int(w), int(h)
                    except ValueError:
                        pass
        elif parts[1] == "chrome" and parts[2] == "dock":
            for field in parts[3:]:
                key, sep, value = field.partition("=")
                if sep and key == "logical":
                    try:
                        dock = tuple(int(v) for v in value.split(","))
                    except ValueError:
                        pass
    NESTED_W, NESTED_H = width, height
    if dock and len(dock) == 4 and dock[2] > 0 and dock[3] > 0:
        x, y, w, h = dock
        DOCK_LEFT_X = x + min(24, w // 4)
        DOCK_RIGHT_X = x + w - min(24, w // 4)
        DOCK_Y = y + h // 2
    else:
        DOCK_LEFT_X = int(width * 0.38)
        DOCK_RIGHT_X = int(width * 0.62)
        DOCK_Y = height - 45
    log(f"output geometry {NESTED_W}x{NESTED_H}, Dock x={DOCK_LEFT_X}.."
        f"{DOCK_RIGHT_X} y={DOCK_Y}")


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        self.client = f"{runtime}/dragonfruit-t21-closeout-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(2.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def query(self, subject):
        self.send(f"query {subject}")
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
        title = f"T21 close-out window {index}"
        child = subprocess.Popen(
            ["xmessage", "-geometry", f"300x150+{x}+{y}", "-title", title, title],
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


def newest_screenshot(directories):
    candidates = []
    for directory in directories:
        candidates.extend(glob.glob(os.path.join(directory, "Screenshot_*.png")))
    if not candidates:
        return None
    return max(candidates, key=os.path.getmtime)


def capture_still(synth, screenshot_dir, dest, click_x, click_y, pre_click=1.5):
    """Capture the nested output through the trusted compositor path."""
    os.makedirs(screenshot_dir, exist_ok=True)
    before = newest_screenshot([screenshot_dir, os.path.expanduser("~/Pictures/Screenshots")])
    synth.key_combo(KEY_LEFTMETA, KEY_LEFTSHIFT, KEY_3)
    time.sleep(pre_click)
    synth.click(click_x, click_y)
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


def wait_for(predicate, timeout, description):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if predicate():
            return True
        time.sleep(0.1)
    log(f"timed out waiting for {description}")
    return False


def read_file(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError:
        return ""


def shell_processes(socket_name):
    out = subprocess.run(
        ["pgrep", "-f", f"dragonfruit-shell --socket-name {socket_name}"],
        capture_output=True,
        text=True,
        check=False,
    )
    return [int(line) for line in out.stdout.split() if line.strip()]


def read_shell_launch(old_pid):
    """Capture the old shell's argv and environment before killing it."""
    cmdline = [part for part in read_file(f"/proc/{old_pid}/cmdline").split("\0") if part]
    env = {}
    for entry in read_file(f"/proc/{old_pid}/environ").split("\0"):
        key, sep, value = entry.partition("=")
        if sep:
            env[key] = value
    return cmdline, env


def relaunch_shell(socket_name, cmdline, env, log_path):
    """Relaunch the shell with the captured env and the reissued token."""
    if not cmdline:
        log("could not read the old shell cmdline")
        return None
    token_path = os.path.join(os.environ["XDG_RUNTIME_DIR"], f"{socket_name}.launch-token")
    token = read_file(token_path).strip()
    if not token:
        log(f"no reissued token at {token_path}")
        return None
    env["DRAGONFRUIT_LAUNCH_TOKEN"] = token
    handle = open(log_path, "w", encoding="utf-8")
    log(f"relaunching shell: {' '.join(cmdline)}")
    return subprocess.Popen(
        cmdline,
        env=env,
        stdout=handle,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )


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
    parser.add_argument("--skip-restart", action="store_true")
    parser.add_argument("--skip-lock", action="store_true")
    args = parser.parse_args()

    synth = Synthetic(args.synth)
    apply_output_geometry(synth.query("scale"))
    runtime = os.environ.get("XDG_RUNTIME_DIR", "/run/user")
    display_file = os.path.join(runtime, f"{args.socket}.x11-display")
    display = read_file(display_file).strip()
    windows = Windows(display) if display else None
    new_shell = None
    transcript = []

    def record(note, subject):
        report = synth.query(subject)
        transcript.append(f"# {note}\n{report}\n")
        log(f"{note}: {report.strip()!r}")
        return report

    try:
        # -- visual --------------------------------------------------------
        if not args.skip_visual:
            # Dock at rest.
            synth.motion(NESTED_W // 2, DOCK_Y)
            time.sleep(1.0)
            capture_still(synth, args.screenshot_dir,
                          os.path.join(args.outdir, "t21-chrome-dock-rest.png"),
                          NESTED_W // 2, DOCK_Y)

            # A frame while the Dock is still magnifying. Start the capture
            # chord at the end of a fast sweep, then accept as soon as the
            # overlay appears; the spring is still moving when the compositor
            # renders the capture.
            for index in range(10):
                x = DOCK_LEFT_X + (DOCK_RIGHT_X - DOCK_LEFT_X) * index / 9
                synth.motion(x, DOCK_Y)
                time.sleep(0.03)
            capture_still(synth, args.screenshot_dir,
                          os.path.join(args.outdir, "t21-chrome-motion.png"),
                          NESTED_W // 2, DOCK_Y, pre_click=0.35)

            # Menu-bar dropdown open/close.
            synth.click(20, 14)
            time.sleep(0.8)
            capture_still(synth, args.screenshot_dir,
                          os.path.join(args.outdir, "t21-chrome-menubar-open.png"),
                          NESTED_W // 2, NESTED_H // 2)
            synth.click(NESTED_W // 2, NESTED_H // 2)
            time.sleep(0.5)

            # Dock popover open/close. The chooser fixture opens the real
            # window chooser once a grouped app exists.
            if windows:
                windows.spawn(0)
                windows.spawn(1)
                wait_for(lambda: len(synth.query("identity").splitlines()) >= 4,
                         10.0, "two X11 windows")
                time.sleep(1.5)
                capture_still(synth, args.screenshot_dir,
                              os.path.join(args.outdir, "t21-chrome-dock-popover.png"),
                              NESTED_W // 2, NESTED_H // 2)
                synth.click(max(20, DOCK_LEFT_X - 40), DOCK_Y)
                time.sleep(0.8)
                capture_still(synth, args.screenshot_dir,
                              os.path.join(args.outdir, "t21-chrome-dock-popover-closed.png"),
                              NESTED_W // 2, NESTED_H // 2)

        # -- shell kill/restart -------------------------------------------
        if not args.skip_restart:
            old_shells = shell_processes(args.socket)
            if not old_shells:
                log("no shell process found before restart")
            else:
                old_pid = old_shells[0]
                cmdline, env = read_shell_launch(old_pid)
                token_path = os.path.join(runtime, f"{args.socket}.launch-token")
                old_token = read_file(token_path).strip()
                log(f"killing shell {old_pid}")
                os.kill(old_pid, signal.SIGKILL)
                wait_for(
                    lambda: read_file(token_path).strip() not in ("", old_token),
                    10.0,
                    "the compositor to reissue the launch token",
                )
                reissued = read_file(token_path).strip()
                transcript.append(f"# shell restart\nold_pid={old_pid}\n"
                                  f"token_reissued={reissued not in ('', old_token)}\n")
                restart_log = os.path.join(os.path.dirname(args.shell_log),
                                           "shell-restart.log")
                new_shell = relaunch_shell(args.socket, cmdline, env, restart_log)
                if new_shell:
                    ok = wait_for(
                        lambda: "Dock scene-graph commit path active"
                        in read_file(restart_log),
                        30.0,
                        "the restarted shell's chrome path",
                    )
                    transcript.append(f"# restarted shell\npid={new_shell.pid}\n"
                                      f"chrome_path={ok}\n")
                    if ok:
                        record("scale after shell restart", "scale")
                        time.sleep(1.0)
                        capture_still(synth, args.screenshot_dir,
                                      os.path.join(args.outdir,
                                                   "t21-chrome-after-restart.png"),
                                      NESTED_W // 2, NESTED_H // 2)

        # -- lock surface --------------------------------------------------
        if not args.skip_lock:
            unlocked = record("unlocked query lock", "lock")
            synth.key_combo(KEY_LEFTMETA, KEY_LEFTCTRL, KEY_Q)
            time.sleep(2.0)
            locked = record("locked query lock", "lock")
            # One typed character must reach the lock surface's password
            # buffer. Return is deliberately not pressed: PAM is out of scope.
            synth.key(KEY_A)
            time.sleep(0.8)
            record("lock after one typed key", "lock")
            capture_still(synth, args.screenshot_dir,
                          os.path.join(args.outdir, "t21-chrome-lock.png"),
                          NESTED_W // 2, NESTED_H // 2)
            if "locked=1" not in locked:
                log("WARNING: the lock chord did not lock the session")
            if "locked=0" not in unlocked:
                log("WARNING: the session did not start unlocked")
    finally:
        if windows:
            windows.close()
        if new_shell is not None:
            try:
                os.killpg(new_shell.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                new_shell.wait(timeout=3)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(new_shell.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        synth.close()

    with open(os.path.join(args.outdir, "t21-chrome-closeout.txt"), "w",
              encoding="utf-8") as handle:
        handle.write("T-21.1c close-out live checks\n")
        handle.write("nested demo, synthetic input harness\n\n")
        handle.write("\n".join(transcript))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
