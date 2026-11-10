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

    def identities(self):
        out = {}
        for line in self.query("query identity").splitlines():
            parts = line.split(" ", 3)
            if len(parts) >= 3 and parts[0] == "identity":
                out[int(parts[1])] = parts[2]
        return out

    def find_settings_window(self):
        """The demo's existing Settings window, without launching more apps.

        The nested dev session starts Settings as its Qt app. Clicking a Dock
        entry for an already-running app only activates it, and scanning the
        Dock would launch unrelated pinned apps, so the capture uses the live
        window and its completed appear record.
        """
        for win, app_id in self.identities().items():
            if "settings" in app_id.lower():
                return win
        raise RuntimeError("the demo has no Settings window")

    def motions(self):
        rows = []
        for line in self.query("query motion").splitlines():
            parts = line.split()
            if not parts or parts[0] != "motion" or len(parts) < 14:
                continue
            rows.append(
                {
                    "window": int(parts[1]),
                    "kind": parts[2],
                    "active": parts[3] == "1",
                    "completed": parts[4] == "1",
                    "frames": int(parts[5]),
                    "origin": tuple(int(v) for v in parts[6:10]),
                    "target": tuple(int(v) for v in parts[10:14]),
                }
            )
        return rows


class Capturer:
    def __init__(self, outdir, scratch, synth=None):
        self.outdir = outdir
        self.scratch = scratch
        self.synth = synth
        os.makedirs(outdir, exist_ok=True)
        os.makedirs(scratch, exist_ok=True)
        self.rect = None

    def full(self, name):
        dest = os.path.join(self.scratch, name + ".png")
        # The compositor's own capture path works on this host where the
        # host screenshot tools return 1 for a nested DRM session.
        if self.synth is not None:
            try:
                os.unlink(dest)
            except FileNotFoundError:
                pass
            self.synth.send(f"capture {dest}")
            for _ in range(80):
                if os.path.exists(dest) and os.path.getsize(dest) > 0:
                    return dest
                time.sleep(0.05)
            raise RuntimeError(f"compositor capture never wrote {dest}")
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
        # The compositor capture is already the nested output, with no host
        # window frame or wallpaper margin.
        if (w, h) == (NESTED_W, NESTED_H):
            self.rect = (0, 0, NESTED_W, NESTED_H)
            return self.rect

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
    cap = Capturer(args.outdir, args.scratch, synth=synth)
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

    # --- T237: launch, minimize/restore, and auto-hide ---------------------
    before = snapshot(args.compositor_pid, args.demo_log)
    start = time.monotonic()
    window_id = synth.find_settings_window()
    appear = [m for m in synth.motions() if m["kind"] == "appear"
              and m["window"] == window_id]
    launch_origin = appear[-1]["origin"] if appear else None
    time.sleep(0.3)
    launch_full = cap.full("t237-dock-launch")
    cap.dock_still(launch_full, "t237-dock-launch.png")

    # Minimize: with `dock.minimizeIntoTileIcon` off the target is the Dock's
    # minimized region, not the app icon.
    synth.send(f"minimize {window_id}")
    time.sleep(0.08)
    minimize = [m for m in synth.motions() if m["kind"] == "minimize"
                and m["window"] == window_id]
    minimize_origin = minimize[-1]["origin"] if minimize else None
    time.sleep(0.55)
    minimized_full = cap.full("t237-dock-minimized")
    cap.dock_still(minimized_full, "t237-dock-minimized.png")

    # Restore: the same window grows back out of the target.
    synth.send(f"restore {window_id}")
    time.sleep(0.08)
    restore = [m for m in synth.motions() if m["kind"] == "restore"
               and m["window"] == window_id]
    restore_origin = restore[-1]["origin"] if restore else None
    time.sleep(0.6)
    elapsed = time.monotonic() - start
    after = snapshot(args.compositor_pid, args.demo_log)
    phases.append(phase_line("launch-minimize-restore", elapsed, before, after))
    phases.append(
        "motion-records: launch_origin=%s minimize_origin=%s restore_origin=%s "
        "appear_frames=%s minimize_frames=%s restore_frames=%s"
        % (
            launch_origin,
            minimize_origin,
            restore_origin,
            appear[-1]["frames"] if appear else None,
            minimize[-1]["frames"] if minimize else None,
            restore[-1]["frames"] if restore else None,
        )
    )
    if launch_origin and minimize_origin:
        # The minimized region sits to the right of the pinned app entry, so a
        # `minimizeIntoTileIcon=off` target is not the launch tile.
        log(f"launch origin {launch_origin}, minimize origin {minimize_origin}")
        if minimize_origin == launch_origin:
            log("WARNING: minimize target still equals the app tile")

    # Auto-hide: hide, then reveal from the edge; the reserved zone is zero in
    # both states (the QML/unit tests pin the value, the stills show the slide).
    before = snapshot(args.compositor_pid, args.demo_log)
    start = time.monotonic()
    gdbus_set(args.dbus_dest, args.dbus_path, args.dbus_iface, "dock.autohide", "<true>")
    time.sleep(0.9)
    hidden_full = cap.full("t237-dock-autohide-hidden")
    cap.dock_still(hidden_full, "t237-dock-autohide-hidden.png")
    synth.motion(DOCK_CENTER_X, NESTED_H - 2)
    time.sleep(0.7)
    revealed_full = cap.full("t237-dock-autohide-revealed")
    cap.dock_still(revealed_full, "t237-dock-autohide-revealed.png")
    gdbus_set(args.dbus_dest, args.dbus_path, args.dbus_iface, "dock.autohide", "<false>")
    synth.motion(20, 20)
    time.sleep(0.5)
    elapsed = time.monotonic() - start
    after = snapshot(args.compositor_pid, args.demo_log)
    phases.append(phase_line("auto-hide", elapsed, before, after))

    # --- Idle window: the settled Dock must contribute no frames -----------
    before = snapshot(args.compositor_pid, args.demo_log)
    idle_start = time.monotonic()
    time.sleep(1.5)
    idle_elapsed = time.monotonic() - idle_start
    after = snapshot(args.compositor_pid, args.demo_log)
    phases.append(phase_line("idle", idle_elapsed, before, after))

    lines = ["T-14.7c/T237 Dock motion trace",
             "magnification sweep, context menu open/close, "
             "dock.size 0.5 -> 0.85 -> 0.5, launch/minimize/restore, "
             "auto-hide hidden/revealed, then idle"]
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