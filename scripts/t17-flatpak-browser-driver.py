#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live Flatpak browser portal flows on the nested session (T-17.1c).

This is the machine half of the T-17.1c verification unit
(docs/tasks/154-t-17.1c-flatpak-browser-verification.md). It runs a real
Flatpak browser (`org.mozilla.firefox`) against the **live nested Dragonfruit
session** and its real `xdg-desktop-portal` backend, and completes each of the
three flows in the **live shell presenter** by synthetic input through the
compositor's input path (the T-03 harness, installed only when
``DRAGONFRUIT_SYNTHETIC_INPUT`` is set):

* file-choose — the browser's ``FileChooser.OpenFile`` raises the shell's
  centred picker; the driver clicks a row and presses Return.
* screenshot — the browser's interactive ``Screenshot.Screenshot`` raises the
  shell's full-output selection overlay; the driver drags a region and releases.
* screen-share — the browser's ScreenCast ``CreateSession``/``SelectSources``
  raises the shell's source picker; the driver clicks the monitor row and
  presses Return, then the browser's ``Start`` negotiates a stream.

Each flow is observed end to end: the Flatpak client inside the sandbox prints
its portal response (the file URI, the screenshot URI, the stream list) and
``FLOW: RESULT: PASS``. A flow whose response does not arrive fails the run.

It is started by ``scripts/capture-t17-flatpak-browser.sh``; run that, not this.
The nested stills are captured with Spectacle (active-window, trimmed to the
1920x1200 nested output, the T-17.1a/b pattern).
"""
import argparse
import json
import os
import queue
import signal
import socket
import subprocess
import sys
import threading
import time

from PIL import Image, ImageChops

NESTED_W, NESTED_H = 1920, 1200
BTN_LEFT = 272
KEY_RETURN = 28

# The shell's centred portal surfaces (shell/src/shellcontroller.cpp):
# `kChooserWidth x kChooserHeight` and `kScreenCastWidth x kScreenCastHeight`,
# centred by the compositor on the 1920x1200 output. Card coordinates below are
# surface-local; tokens are the design-system spacing/controls (docs/design).
CHOOSER_W, CHOOSER_H = 640, 440
CHOOSER_ORIGIN = ((NESTED_W - CHOOSER_W) // 2, (NESTED_H - CHOOSER_H) // 2)
SCREENCAST_W, SCREENCAST_H = 560, 460
SCREENCAST_ORIGIN = ((NESTED_W - SCREENCAST_W) // 2, (NESTED_H - SCREENCAST_H) // 2)


def log(message):
    print(f"t17-flatpak: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-t17-flatpak-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(1.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def motion(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")

    def click(self, x, y, button=BTN_LEFT):
        self.motion(x, y)
        time.sleep(0.25)
        self.send(f"button {button} down")
        time.sleep(0.12)
        self.send(f"button {button} up")
        time.sleep(0.4)

    def drag(self, x0, y0, x1, y1, button=BTN_LEFT):
        self.motion(x0, y0)
        time.sleep(0.25)
        self.send(f"button {button} down")
        time.sleep(0.15)
        for i in range(1, 6):
            self.motion(x0 + (x1 - x0) * i / 5, y0 + (y1 - y0) * i / 5)
            time.sleep(0.08)
        self.send(f"button {button} up")
        time.sleep(0.4)

    def return_key(self):
        self.send(f"key {KEY_RETURN} down")
        time.sleep(0.1)
        self.send(f"key {KEY_RETURN} up")
        time.sleep(0.6)


class Capture:
    """Active-window nested stills, cropped to the 1920x1200 nested output."""

    RAISE_JS = """function raiseDragonfruit() {
    var wins = workspace.windowList();
    for (var i = 0; i < wins.length; ++i) {
        var w = wins[i];
        var cap = (w.caption || "").toString();
        var cls = (w.resourceClass || "").toString().toLowerCase();
        if (cap.indexOf("Dragonfruit") >= 0 || cls.indexOf("dragonfruit") >= 0) {
            w.minimized = false;
            if (workspace.activateWindow) { workspace.activateWindow(w); }
            else { workspace.activeWindow = w; }
        }
    }
}
raiseDragonfruit();
"""

    def __init__(self, synth, outdir, scratch, host_bus=None):
        self.synth = synth
        self.outdir = outdir
        self.scratch = scratch
        # KWin and Spectacle live on the *host* session bus; the nested demo
        # runs on the private portal bus (the capture script's ambient
        # DBUS_SESSION_BUS_ADDRESS), so pin the host bus for both tools.
        self.host_env = dict(os.environ)
        if host_bus:
            self.host_env["DBUS_SESSION_BUS_ADDRESS"] = host_bus
        os.makedirs(outdir, exist_ok=True)
        os.makedirs(scratch, exist_ok=True)
        self.script = os.path.join(scratch, "raise-dragonfruit.js")
        with open(self.script, "w") as handle:
            handle.write(self.RAISE_JS)

    def _raise(self):
        for method in ("loadScript", "start"):
            args = ["gdbus", "call", "--session", "--dest", "org.kde.KWin",
                    "--object-path", "/Scripting",
                    "--method", f"org.kde.kwin.Scripting.{method}"]
            if method == "loadScript":
                args += [self.script, "df_capture_raise"]
            subprocess.run(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                           env=self.host_env)
        time.sleep(1.0)

    def _nudge(self):
        for x in (0.25, 0.75, 0.5):
            self.synth.motion(NESTED_W * x, NESTED_H * 0.6)
            time.sleep(0.15)
        time.sleep(0.6)

    def _active_raw(self, name):
        dest = os.path.join(self.scratch, name + "-raw.png")
        last = ""
        for _ in range(4):
            try:
                self._raise()
                self._nudge()
            except Exception as err:  # noqa: BLE001 - raising is best-effort
                log(f"raise skipped: {err}")
            proc = subprocess.run(
                ["spectacle", "-b", "-n", "-a", "-o", dest],
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
                env=self.host_env,
            )
            if proc.returncode == 0 and os.path.exists(dest):
                return dest
            last = (proc.stderr or "").strip()
            log(f"active-window capture retry ({proc.returncode}): {last}")
            time.sleep(1.0)
        raise RuntimeError(f"spectacle active-window capture failed: {last}")

    def full(self, name):
        im = Image.open(self._active_raw(name))
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
        if im.width >= NESTED_W and im.height >= NESTED_H:
            x0 = (im.width - NESTED_W) // 2
            y0 = im.height - NESTED_H
            im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
        return im.convert("RGB")

    def still(self, name):
        dest = os.path.join(self.outdir, name + ".png")
        self.full(name).save(dest)
        log(f"saved {dest}")
        return dest

    def raw_image(self, name):
        """A 1920x1200 image of the nested output, not saved to `outdir`."""
        return self.full(name)

    def still_from(self, image, name):
        dest = os.path.join(self.outdir, name + ".png")
        image.save(dest)
        log(f"saved {dest}")
        return dest


def changed_bounds(before, after, threshold=40):
    """The bounding box of strong pixel changes between two stills.

    A portal overlay surface is the only thing that changed between the
    baseline and the mapped-picker still, so the box is the surface rect. This
    calibrates the driver's clicks against the live layout instead of a
    hard-coded card origin.
    """
    diff = ImageChops.difference(before, after).convert("L")
    mask = diff.point(lambda p: 255 if p > threshold else 0)
    box = mask.getbbox()
    if box is None:
        return None
    left, top, right, bottom = box
    return (left, top, right - left, bottom - top)


class FlatpakFlow:
    """One sandboxed portal flow, run and observed by the host driver."""

    def __init__(self, app, root, folder, scratch, driver):
        self.app = app
        self.root = root
        self.folder = folder
        self.scratch = scratch
        self.driver = driver
        self.proc = None
        self.lines = []
        self.queue = queue.Queue()
        self.reader = None

    def _command(self, flow):
        return [
            "flatpak", "run",
            f"--filesystem={self.root}/scripts",
            f"--filesystem={self.folder}",
            f"--filesystem={self.scratch}",
            "--command=python3", self.app, self.driver, flow,
        ]

    def _read(self):
        for line in self.proc.stdout:
            line = line.rstrip("\n")
            self.lines.append(line)
            log(f"client: {line}")
            self.queue.put(line)
        self.queue.put(None)

    def start(self, flow):
        env = dict(os.environ)
        env["DF_T154_FOLDER"] = self.folder
        env["PYTHONUNBUFFERED"] = "1"
        # Own process group so a stuck sandbox can be reaped wholesale (flatpak
        # run spawns bwrap, which otherwise keeps the stdout pipe open).
        self.proc = subprocess.Popen(
            self._command(flow), stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, bufsize=1, env=env, start_new_session=True,
        )
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        return self.proc

    def _drain(self, timeout):
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                line = self.queue.get(timeout=0.25)
            except queue.Empty:
                if self.proc.poll() is not None and self.queue.empty():
                    break
                continue
            if line is None:
                return
        return

    def wait_for(self, needle, timeout=25.0):
        """Read the client's output until `needle` appears (or time out)."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                line = self.queue.get(timeout=0.25)
            except queue.Empty:
                if self.proc.poll() is not None:
                    return False
                continue
            if line is None:
                return False
            if needle in line:
                return True
        return False

    def finish(self, timeout=45.0):
        """Wait for the flow to settle, then reap the whole sandbox group."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.proc.poll() is not None:
                self._drain(2.0)
                return self.proc.returncode
            try:
                line = self.queue.get(timeout=0.25)
            except queue.Empty:
                continue
            if line is None:
                return self.proc.poll()
        try:
            os.killpg(os.getpgid(self.proc.pid), signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
        self.proc.wait(timeout=5)
        return self.proc.returncode


def run(args):
    synth = Synthetic(args.synth)
    cap = Capture(synth, args.outdir, args.scratch, host_bus=args.host_bus)
    root = os.path.abspath(args.root)
    driver = os.path.join(root, "scripts", "t17-flatpak-flow.py")
    folder = os.path.abspath(args.folder)
    os.makedirs(folder, exist_ok=True)
    with open(os.path.join(folder, "picked.txt"), "w") as handle:
        handle.write("dragonfruit T-17.1c picked file\n")

    transcript = []

    # Let the demo's clients map and the Spaces settle before the first flow.
    time.sleep(2.0)
    cap.still(args.prefix)

    # --- 1. file-choose ----------------------------------------------------
    baseline = cap.raw_image("baseline-file")
    flow = FlatpakFlow(args.app, root, folder, args.scratch, driver)
    flow.start("file")
    if not flow.wait_for("FileChooser.OpenFile"):
        raise RuntimeError("the Flatpak FileChooser.OpenFile never reached the shell")
    time.sleep(2.5)
    choose_img = cap.raw_image("file-choose")
    cap.still_from(choose_img, args.prefix + "-file-choose")
    rect = changed_bounds(baseline, choose_img)
    log(f"file-choose: picker surface rect {rect} (expected ~{CHOOSER_W}x{CHOOSER_H})")
    if rect is None:
        raise RuntimeError("the file-choose picker surface never appeared")
    rx, ry, rw, rh = rect
    local = (rx + rw // 2, ry + 108)
    log(f"file-choose: clicking the first row at {local}")
    synth.click(*local)
    synth.return_key()
    status = flow.finish()
    transcript.append({"flow": "file-choose", "returncode": status, "log": flow.lines})
    if status != 0 or not any("FLOW: RESULT: PASS" in line for line in flow.lines):
        raise RuntimeError("the Flatpak file-choose flow did not pass")
    cap.still(args.prefix + "-file-choose-accepted")
    time.sleep(1.0)

    # --- 2. screenshot -----------------------------------------------------
    baseline = cap.raw_image("baseline-shot")
    flow = FlatpakFlow(args.app, root, folder, args.scratch, driver)
    flow.start("shot")
    if not flow.wait_for("Screenshot.Screenshot"):
        raise RuntimeError("the Flatpak Screenshot request never reached the shell")
    time.sleep(2.5)
    shot_img = cap.raw_image("screenshot")
    cap.still_from(shot_img, args.prefix + "-screenshot")
    log("screenshot: dragging a region in the live selection overlay")
    synth.drag(500, 350, 1300, 850)
    status = flow.finish()
    transcript.append({"flow": "screenshot", "returncode": status, "log": flow.lines})
    if status != 0 or not any("FLOW: RESULT: PASS" in line for line in flow.lines):
        raise RuntimeError("the Flatpak screenshot flow did not pass")
    cap.still(args.prefix + "-screenshot-accepted")
    time.sleep(1.0)

    # --- 3. screen-share ---------------------------------------------------
    baseline = cap.raw_image("baseline-cast")
    flow = FlatpakFlow(args.app, root, folder, args.scratch, driver)
    flow.start("cast")
    if not flow.wait_for("ScreenCast.SelectSources"):
        raise RuntimeError("the Flatpak ScreenCast.SelectSources never reached the shell")
    time.sleep(2.5)
    cast_img = cap.raw_image("screen-share")
    cap.still_from(cast_img, args.prefix + "-screen-share")
    rect = changed_bounds(baseline, cast_img)
    log(f"screen-share: picker surface rect {rect} (expected ~{SCREENCAST_W}x{SCREENCAST_H})")
    if rect is None:
        raise RuntimeError("the screen-share picker surface never appeared")
    rx, ry, rw, rh = rect
    local = (rx + rw // 2, ry + 170)
    log(f"screen-share: clicking the first source row at {local}")
    synth.click(*local)
    synth.return_key()
    status = flow.finish()
    transcript.append({"flow": "screen-share", "returncode": status, "log": flow.lines})
    if status != 0 or not any("FLOW: RESULT: PASS" in line for line in flow.lines):
        raise RuntimeError("the Flatpak screen-share flow did not pass")
    cap.still(args.prefix + "-screen-share-accepted")

    with open(os.path.join(args.scratch, "transcript.txt"), "w") as handle:
        for entry in transcript:
            handle.write(json.dumps(entry) + "\n")
    with open(os.path.join(args.scratch, "walkthrough.txt"), "w") as handle:
        handle.write("Flatpak browser walkthrough (live shell presenter)\n")
        handle.write("file-choose:   opened, clicked, accepted\n")
        handle.write("screenshot:    interactive region drag, accepted\n")
        handle.write("screen-share:  source picked, Share\n")
    log("Flatpak browser walkthrough complete")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--prefix", default="t17-flatpak-browser")
    parser.add_argument("--app", default="org.mozilla.firefox")
    parser.add_argument("--root", default=".")
    parser.add_argument("--folder", required=True)
    parser.add_argument("--host-bus", default=None,
                        help="host session bus address for KWin/Spectacle "
                             "(the demo runs on a private portal bus)")
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())