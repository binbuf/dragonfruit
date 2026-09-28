#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested navigation loop and capture each step (T-17.1b).

This is the machine half of the T-17.1b verification unit
(docs/tasks/153-t-17.1b-workspace-overview-switcher-verification.md): it
performs the workspace-switch, Mission Control, and app-switch navigation on
the *running* nested session, by **pointer and keyboard**, through the real
compositor input path (the T-03 synthetic-input harness, installed on the
nested backend only when ``DRAGONFRUIT_SYNTHETIC_INPUT`` is set). Each path is
observed through the read-only introspection the compositor already exposes:
``query spaces`` (the active Space and its windows), ``query grid`` (the
Mission Control live-surface transform), ``query wallpaper`` (the in-flight
Space slide), and ``query switcher`` (the app-switcher entries, selection and
focus).

It is driven by ``scripts/capture-t17-navigation.sh``; run that, not this.
Requires a host Wayland session, ``spectacle`` (screenshot tool), Pillow, and
(optionally) the KWin scripting D-Bus used to raise the nested window for the
active-window capture.
"""
import argparse
import json
import os
import socket
import subprocess
import sys
import time

from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
BTN_LEFT = 272

# evdev codes (the backend adds xkb's +8, exactly like libinput).
KEY_LEFTCTRL = 29
KEY_LEFTMETA = 125
KEY_TAB = 15
KEY_UP = 103
KEY_LEFT = 105
KEY_RIGHT = 106

# Mission Control workspace-strip geometry, reproduced from the shared design
# tokens (docs/design/10-design-system.md): menu bar 28 + strip margin 16, cards
# 132x84 with a 12 px gap, centered horizontally on the 1920 px output.
STRIP_TOP = 28 + 16
CARD_WIDTH, CARD_HEIGHT, CARD_GAP = 132, 84, 12


def log(message):
    print(f"t17-nav: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-t17-nav-{os.getpid()}.sock"
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

    def spaces(self):
        rows = []
        for line in self.query("query spaces").splitlines():
            p = line.split()
            if p and p[0] == "space":
                rows.append(
                    {
                        "output": p[1],
                        "index": int(p[2]),
                        "id": int(p[3]),
                        "active": "active=1" in p[4],
                        "windows": int(p[5].split("=")[1]),
                    }
                )
        return rows

    def active_space(self, output="NESTED-1"):
        for row in self.spaces():
            if row["output"] == output and row["active"]:
                return row["index"]
        return None

    def grid(self):
        report = self.query("query grid")
        entry = {"active": False, "progress": 0.0, "windows": [], "outputs": []}
        for line in report.splitlines():
            p = line.split()
            if not p or p[0] != "grid":
                continue
            if p[1] == "none":
                return entry
            if p[1].startswith("progress="):
                entry["active"] = True
                entry["progress"] = float(p[1].split("=")[1])
            elif p[1] == "output":
                entry["outputs"].append({"name": p[2], "columns": int(p[3].split("=")[1])})
            elif p[1] == "window":
                values = [int(v) for v in p[3:12]]
                entry["windows"].append({"window": int(p[2]), "target": tuple(values[4:8])})
        return entry

    def wallpaper(self):
        report = self.query("query wallpaper")
        entry = {"active": None, "direction": 0, "progress": 0.0, "slots": 0}
        for line in report.splitlines():
            p = line.split()
            if not p or p[0] != "wallpaper":
                continue
            if p[1] == "output":
                for token in p[3:]:
                    if token.startswith("direction="):
                        entry["direction"] = int(token.split("=")[1])
                    if token.startswith("progress="):
                        entry["progress"] = float(token.split("=")[1])
            elif p[1] == "slot":
                entry["slots"] += 1
        return entry

    def switcher(self):
        report = self.query("query switcher")
        entry = {
            "active": False,
            "selected": -1,
            "focus": -1,
            "entries": [],
            "previews": [],
        }
        for line in report.splitlines():
            p = line.split()
            if not p or p[0] != "switcher":
                continue
            if p[1] == "app":
                entry["entries"].append({"window": int(p[4]), "app": p[3]})
            elif p[1] == "preview":
                entry["previews"].append(
                    {
                        "window": int(p[2]),
                        "x": int(p[3]),
                        "y": int(p[4]),
                        "w": int(p[5]),
                        "h": int(p[6]),
                    }
                )
            else:
                for token in p[1:]:
                    if token.startswith("active="):
                        entry["active"] = token == "active=1"
                    elif token.startswith("selected="):
                        entry["selected"] = int(token.split("=")[1])
                    elif token.startswith("focus="):
                        entry["focus"] = int(token.split("=")[1])
        return entry

    def wait_spaces(self, predicate, timeout=6.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            rows = self.spaces()
            if predicate(rows):
                return rows
            time.sleep(0.1)
        raise RuntimeError(f"spaces never matched: {self.spaces()}")

    def wait_grid(self, predicate, timeout=6.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            report = self.grid()
            if predicate(report):
                return report
            time.sleep(0.1)
        raise RuntimeError(f"grid never matched: {self.grid()}")

    def wait_switcher(self, predicate, timeout=6.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            report = self.switcher()
            if predicate(report):
                return report
            time.sleep(0.1)
        raise RuntimeError(f"switcher never matched: {self.switcher()}")

    def motion(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")

    def click(self, x, y, button=BTN_LEFT):
        self.motion(x, y)
        time.sleep(0.15)
        self.send(f"button {button} down")
        time.sleep(0.1)
        self.send(f"button {button} up")
        time.sleep(0.3)

    def chord(self, *keys):
        script = "".join(f"key {key} down\n" for key in keys)
        script += "".join(f"key {key} up\n" for key in reversed(keys))
        self.send(script.strip())

    def swipe(self, fingers, dx, dy, steps=2):
        self.send(f"swipe-begin {fingers}")
        for _ in range(steps):
            self.send(f"swipe-update {dx} {dy}")
            time.sleep(0.1)

    def swipe_end(self):
        self.send("swipe-end")


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

    def __init__(self, synth, outdir, scratch):
        self.synth = synth
        self.outdir = outdir
        self.scratch = scratch
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
            subprocess.run(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1.0)

    def _nudge(self):
        for x in (0.25, 0.75, 0.5):
            self.synth.motion(NESTED_W * x, NESTED_H * 0.6)
            time.sleep(0.15)
        time.sleep(0.6)

    def _active_raw(self, name):
        dest = os.path.join(self.scratch, name + "-raw.png")
        try:
            self._raise()
            self._nudge()
        except Exception as err:  # noqa: BLE001 - raising is best-effort
            log(f"raise/nudge skipped: {err}")
        subprocess.run(
            ["spectacle", "-b", "-n", "-a", "-o", dest],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return dest

    def full(self, name):
        """Capture the nested output and return an RGB 1920x1200 image."""
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
        dest = os.path.join(self.outdir, name)
        self.full(name).save(dest)
        log(f"saved {dest}")
        return dest


def strip_card_center(index, count):
    """The Mission Control workspace-strip card center (T-05 strip geometry)."""
    total = count * CARD_WIDTH + (count - 1) * CARD_GAP
    start_x = (NESTED_W - total) // 2
    x = start_x + index * (CARD_WIDTH + CARD_GAP) + CARD_WIDTH // 2
    y = STRIP_TOP + CARD_HEIGHT // 2
    return x, y


def run(args):
    synth = Synthetic(args.synth)
    cap = Capture(synth, args.outdir, args.scratch)
    prefix = args.prefix
    steps = []
    transcript = []

    def snap(label, name):
        cap.still(name + ".png")
        steps.append((label, name))
        transcript.append(
            {
                "step": label,
                "spaces": synth.spaces(),
                "grid": synth.grid(),
                "switcher": synth.switcher(),
            }
        )
        return name

    synth.send("set color-scheme dark")
    time.sleep(0.4)

    # Wait until the demo's clients are mapped and the Spaces exist.
    for _ in range(200):
        if synth.spaces() and synth.query("query decorations").count("decoration") >= 2:
            break
        time.sleep(0.2)
    while synth.active_space() not in (None, 0):
        synth.chord(KEY_LEFTCTRL, KEY_LEFT)
        time.sleep(0.4)
    initial = synth.spaces()
    log(f"initial spaces: {initial}")
    assert initial, "no Spaces reported"

    # --- 0. the navigation desktop -----------------------------------------
    snap("initial: the nested desktop on Space 0 with the demo clients", prefix)

    # --- 1. workspace switch: keyboard (Ctrl+Right / Ctrl+Left) ------------
    synth.chord(KEY_LEFTCTRL, KEY_RIGHT)
    synth.wait_spaces(lambda rows: any(r["active"] and r["index"] == 1 for r in rows))
    time.sleep(0.4)
    snap("workspace switch (keyboard): Ctrl+Right moved to Space 1", prefix + "-workspace-keyboard")
    synth.chord(KEY_LEFTCTRL, KEY_LEFT)
    synth.wait_spaces(lambda rows: any(r["active"] and r["index"] == 0 for r in rows))
    time.sleep(0.3)

    # --- 2. workspace switch: trackpad gesture (three-finger swipe) --------
    # The gesture is caught mid-flight, when both wallpapers are sliding and no
    # Space is active yet, then commits to Space 1.
    synth.swipe(3, -100, 0, steps=2)
    mid = None
    deadline = time.time() + 3.0
    while time.time() < deadline:
        w = synth.wallpaper()
        if w["direction"] == 1 and 0.0 < w["progress"] < 1.0:
            mid = w
            break
        time.sleep(0.05)
    assert mid is not None, f"no in-flight workspace slide was observed: {synth.wallpaper()}"
    transcript.append({"step": "workspace switch (gesture) mid-flight", "wallpaper": mid})
    snap("workspace switch (gesture): three-finger swipe caught mid-slide", prefix + "-workspace-gesture")
    synth.swipe_end()
    synth.wait_spaces(lambda rows: any(r["active"] and r["index"] == 1 for r in rows))
    time.sleep(0.3)
    # Return to Space 0 with the mirrored swipe.
    synth.swipe(3, 100, 0, steps=2)
    synth.swipe_end()
    synth.wait_spaces(lambda rows: any(r["active"] and r["index"] == 0 for r in rows))
    time.sleep(0.3)

    # --- 3. Mission Control: keyboard (Ctrl+Up) ----------------------------
    synth.chord(KEY_LEFTCTRL, KEY_UP)
    grid = synth.wait_grid(
        lambda g: g["active"] and g["progress"] > 0.99 and len(g["windows"]) >= 2
    )
    transcript.append({"step": "mission control (keyboard)", "grid": grid})
    snap("mission control (keyboard): Ctrl+Up laid the live surfaces into the grid",
         prefix + "-mission-control-keyboard")
    synth.chord(KEY_LEFTCTRL, KEY_UP)
    synth.wait_grid(lambda g: not g["active"])

    # --- 4. Mission Control: pointer (hot-corner dwell) --------------------
    synth.motion(2, 2)
    grid = synth.wait_grid(
        lambda g: g["active"] and g["progress"] > 0.99 and len(g["windows"]) >= 2,
        timeout=8.0,
    )
    transcript.append({"step": "mission control (pointer hot corner)", "grid": grid})
    snap("mission control (pointer): top-left hot-corner dwell opened the overview",
         prefix + "-mission-control-pointer")

    # --- 5. workspace switch: pointer (Mission Control Space card) ---------
    count = len(synth.spaces())
    card = strip_card_center(1, count)
    log(f"clicking Mission Control Space card 1 at {card}")
    synth.click(*card)
    synth.wait_spaces(lambda rows: any(r["active"] and r["index"] == 1 for r in rows))
    time.sleep(0.4)
    snap("workspace switch (pointer): clicking the Space 1 strip card switched Spaces",
         prefix + "-workspace-pointer")
    # Leave the overview and return to Space 0.
    synth.chord(KEY_LEFTCTRL, KEY_UP)
    synth.wait_grid(lambda g: not g["active"])
    synth.chord(KEY_LEFTCTRL, KEY_LEFT)
    synth.wait_spaces(lambda rows: any(r["active"] and r["index"] == 0 for r in rows))
    time.sleep(0.3)

    # --- 6. app switch: keyboard (Cmd+Tab, release to commit) --------------
    # The switcher opens on the *next* app (the focused one is skipped), so the
    # modifier release alone commits the switch.
    focus_before = synth.switcher()["focus"]
    synth.send(f"key {KEY_LEFTMETA} down\nkey {KEY_TAB} down")
    opened = synth.wait_switcher(lambda s: s["active"] and len(s["previews"]) >= 2)
    time.sleep(0.3)
    snap("app switch (keyboard): Cmd+Tab opened the switcher on the live previews",
         prefix + "-app-switch-keyboard")
    synth.send(f"key {KEY_TAB} up")
    before = synth.switcher()
    synth.send(f"key {KEY_LEFTMETA} up")
    committed = synth.wait_switcher(lambda s: not s["active"])
    assert committed["focus"] != focus_before, (
        f"the keyboard app switch did not change focus: {focus_before} -> {committed}"
    )
    transcript.append(
        {"step": "app switch (keyboard) commit", "before": focus_before, "after": committed}
    )
    time.sleep(0.4)

    # --- 7. app switch: pointer (click a live preview) ---------------------
    synth.send(f"key {KEY_LEFTMETA} down\nkey {KEY_TAB} down")
    opened = synth.wait_switcher(lambda s: s["active"] and len(s["previews"]) >= 2)
    synth.send(f"key {KEY_TAB} up")
    target = next(
        (p for p in opened["previews"] if p["window"] != opened["focus"]),
        opened["previews"][0],
    )
    transcript.append({"step": "app switch (pointer) target", "target": target})
    px = target["x"] + target["w"] // 2
    py = target["y"] + target["h"] // 2
    log(f"clicking switcher preview for window {target['window']} at ({px},{py})")
    snap("app switch (pointer): the live preview under the pointer", prefix + "-app-switch-pointer")
    synth.click(px, py)
    committed = synth.wait_switcher(lambda s: not s["active"])
    synth.send(f"key {KEY_LEFTMETA} up")
    assert committed["focus"] == target["window"], (
        f"the pointer commit focused {committed['focus']}, expected {target['window']}"
    )
    snap("app switch (pointer): the clicked app's window took focus",
         prefix + "-app-switch-pointer-committed")

    if args.leave_open:
        return steps, transcript

    with open(os.path.join(cap.scratch, "walkthrough.txt"), "w") as handle:
        for label, path in steps:
            handle.write(f"{label}\n  {path}.png\n")
    with open(os.path.join(cap.scratch, "transcript.txt"), "w") as handle:
        for entry in transcript:
            handle.write(json.dumps(entry, default=list) + "\n")
    log("navigation loop complete")
    return steps, transcript


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--prefix", default="t17-navigation")
    parser.add_argument("--leave-open", action="store_true")
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())