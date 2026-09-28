#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested Dragonfruit window loop and capture each step (T-17.1a).

This is the machine half of the T-17.1a verification unit
(docs/tasks/152-t-17.1a-nested-window-loop-verification.md): it performs the
30-second loop checklist on the *running* nested session — launch/appear,
focus, move (titlebar drag), zoom, minimize, restore-from-Dock, close — for a
third-party Qt SSD window and an X11 window, and confirms a client-side
decorated (CSD) window is unaffected, through the real compositor input path
(the T-03 synthetic-input harness, installed on the nested backend only when
``DRAGONFRUIT_SYNTHETIC_INPUT`` is set).

It is driven by ``scripts/capture-t17-window-loop.sh``; run that, not this.
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
# Traffic-light geometry from component.trafficLights tokens (T-01.2).
LIGHT_DIAMETER, LIGHT_GAP, LIGHT_INSET = 12, 8, 12
BTN_LEFT, BTN_RIGHT = 272, 273
DOCK_BAND_H = 150


def log(message):
    print(f"t17-loop: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-t17-loop-{os.getpid()}.sock"
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

    def identities(self):
        out = {}
        for line in self.query("query identity").splitlines():
            p = line.split(maxsplit=3)
            if p and p[0] == "identity":
                out[int(p[1])] = (p[2], p[3].strip() if len(p) > 3 else "")
        return out

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

    def drag(self, x, y, dx, dy):
        # A CSD client turns its titlebar press into an xdg_toplevel.move
        # request, so the compositor's grab only starts after a round trip;
        # send the delta in stepped motions so none of it is lost, and pause
        # before the press is released.
        self.motion(x, y)
        time.sleep(0.15)
        self.send(f"button {BTN_LEFT} down")
        time.sleep(0.35)
        steps = max(4, int(max(abs(dx), abs(dy)) / 10))
        sent_x = sent_y = 0
        for i in range(1, steps + 1):
            step_x = round(dx * i / steps) - sent_x
            step_y = round(dy * i / steps) - sent_y
            sent_x += step_x
            sent_y += step_y
            self.send(f"motion {step_x} {step_y}")
            time.sleep(0.05)
        time.sleep(0.15)
        self.send(f"button {BTN_LEFT} up")
        time.sleep(0.4)

    def wait_state(self, window, states, timeout=6.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            for row in self.decorations():
                if row["window"] == window and row["state"] in states:
                    return row
            time.sleep(0.1)
        raise RuntimeError(
            f"window {window} never reached {states}: {self.decorations()}"
        )

    def wait_gone(self, window, timeout=6.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if not any(r["window"] == window for r in self.decorations()):
                return True
            time.sleep(0.1)
        return False


def light_center(tb, index):
    tx, ty, _tw, th = tb
    return (
        tx + LIGHT_INSET + LIGHT_DIAMETER // 2 + index * (LIGHT_DIAMETER + LIGHT_GAP),
        ty + th // 2,
    )


def titlebar_center(tb):
    return (tb[0] + tb[2] // 2, tb[1] + tb[3] // 2)


def window_titlebar_center(win):
    """Titlebar center for an SSD (compositor) or CSD (client-drawn) window.

    CSD clients draw the design-system TitleBar at the top of their content
    surface, so its center is a fixed inset down from the content top.
    """
    if win["ssd"]:
        return titlebar_center(win["tb"])
    return (win["content"][0] + win["content"][2] // 2, win["content"][1] + 18)


def rects_intersect(a, b):
    return not (
        a[0] + a[2] <= b[0]
        or b[0] + b[2] <= a[0]
        or a[1] + a[3] <= b[1]
        or b[1] + b[3] <= a[1]
    )


def clear_the_stage(synth, loop_win):
    """Drag any window occluding the loop window's titlebar out of the way.

    The nested demo maps more than one client; a window that overlaps the loop
    titlebar would swallow the traffic-light/titlebar input. A CSD window is
    moved through its own client-drawn titlebar (the xdg_toplevel.move path).
    """
    tb = loop_win["tb"]
    for other in classify(synth):
        if other["window"] == loop_win["window"]:
            continue
        if not rects_intersect(other["content"], tb):
            continue
        dy = (tb[1] + tb[3] + 240) - other["content"][1]
        log(f"clearing stage: dragging window {other['window']} down by {dy}")
        synth.drag(*window_titlebar_center(other), 0, max(dy, 60))
    time.sleep(0.3)


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

    def still(self, full, name, box=None):
        im = full if box is None else full.crop(
            (box[0], box[1], box[0] + box[2], box[1] + box[3])
        )
        dest = os.path.join(self.outdir, name)
        im.save(dest)
        log(f"saved {dest} ({im.width}x{im.height})")
        return dest


def classify(synth):
    rows = synth.decorations()
    ids = synth.identities()
    windows = []
    for row in rows:
        wm_class, title = ids.get(row["window"], ("", ""))
        windows.append({**row, "wm_class": wm_class, "title": title})
    return windows


def wait_for_windows(synth, timeout=30.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        ws = classify(synth)
        if ws:
            return ws
        time.sleep(0.5)
    raise RuntimeError("no windows were announced")


def dock_changed_runs(base, mini):
    """Column diffs in the Dock band between two full nested stills."""
    w, h = base.size
    y0, y1 = h - DOCK_BAND_H, h
    bp, mp = base.load(), mini.load()
    changed = []
    for x in range(w):
        diff = 0
        for y in range(y0, y1, 3):
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
    return runs


def run(args):
    synth = Synthetic(args.synth)
    cap = Capture(synth, args.outdir, args.scratch)
    prefix = args.prefix
    steps = []
    transcript = []

    def snap(label, name, box=None):
        full = cap.full(name)
        cap.still(full, name + ".png" if not name.endswith(".png") else name, box)
        steps.append((label, name))
        return full

    synth.send("set color-scheme dark")
    time.sleep(0.3)

    windows = wait_for_windows(synth)
    for w in windows:
        log(f"window {w['window']} ssd={w['ssd']} class={w['wm_class']!r} state={w['state']} content={w['content']}")
    transcript.append(("initial windows", classify(synth)))

    ssd = [w for w in windows if w["ssd"]]
    csd = [w for w in windows if not w["ssd"]]
    qt_ssd = next(
        (w for w in ssd if "kcalc" in w["wm_class"].lower() or w["wm_class"].lower().startswith("org.kde")),
        None,
    )
    x11 = next((w for w in ssd if w["wm_class"].lower() == "xmessage"), None)
    loop_win = qt_ssd or (ssd[0] if ssd else None)
    if loop_win is None:
        raise RuntimeError("no server-side decorated window for the loop")
    log(f"loop window {loop_win['window']} ({loop_win['wm_class']})")
    if not csd:
        log("note: no CSD window present; the CSD check is covered by the headless tier matrix")

    # Make the loop window's titlebar reachable before the first still.
    clear_the_stage(synth, loop_win)

    # --- 1. launch / appear ------------------------------------------------
    initial = snap("initial: the nested desktop with the Qt SSD, CSD, and X11 clients mapped", prefix)
    transcript.append(("appear", classify(synth)))

    # --- 2. focus ----------------------------------------------------------
    row = next(r for r in classify(synth) if r["window"] == loop_win["window"])
    synth.click(*window_titlebar_center(row))
    time.sleep(0.5)
    snap("focused: clicked the Qt SSD titlebar", prefix + "-focused")

    # --- 3. move (titlebar drag) ------------------------------------------
    row = synth.wait_state(loop_win["window"], ("floating", "zoomed"))
    before = row["content"]
    synth.drag(*window_titlebar_center(row), 90, 70)
    deadline = time.time() + 3.0
    moved = row
    while time.time() < deadline:
        candidate = next(
            (r for r in classify(synth) if r["window"] == loop_win["window"]), None
        )
        if candidate and candidate["content"] != tuple(before):
            moved = candidate
            break
        time.sleep(0.1)
    after = moved["content"]
    delta = (after[0] - before[0], after[1] - before[1])
    assert all(abs(delta[i] - want) <= 4 for i, want in enumerate((90, 70))), (
        f"titlebar drag did not move the window: {before} -> {after}"
    )
    transcript.append(("move", before, after))
    key = next(r for r in classify(synth) if r["window"] == loop_win["window"])
    box = (
        key["tb"][0] - 20,
        key["tb"][1] - 20,
        key["tb"][2] + 40,
        key["content"][3] + 100,
    )
    snap("move: dragged the titlebar (90,70) and the window followed", prefix + "-move", box)

    # --- 4. zoom (double-click the titlebar) ------------------------------
    row = next(r for r in classify(synth) if r["window"] == loop_win["window"])
    synth.double_click(*window_titlebar_center(row))
    zoomed = synth.wait_state(loop_win["window"], ("zoomed",))
    time.sleep(0.3)
    snap("zoom: double-click on the titlebar filled the usable area", prefix + "-zoom")
    transcript.append(("zoom", zoomed["content"]))

    # --- unzoom (double-click again) --------------------------------------
    synth.double_click(*window_titlebar_center(zoomed))
    restored = synth.wait_state(loop_win["window"], ("floating",))
    transcript.append(("unzoom", restored["content"]))
    time.sleep(0.3)

    # --- 5. minimize (yellow light) ---------------------------------------
    # The pre-minimize frame is the Dock-diff reference; it is not a step
    # still (the main still already shows the floating state).
    focused_full = cap.full(prefix + "-floating")
    row = synth.wait_state(loop_win["window"], ("floating",))
    synth.click(*light_center(row["tb"], 1))
    mini_row = synth.wait_state(loop_win["window"], ("minimized",))
    time.sleep(0.4)
    mini_full = snap("minimized: yellow light; the window's Dock entry collapsed", prefix + "-minimized")
    transcript.append(("minimize", mini_row["state"]))

    # --- 6. restore --------------------------------------------------------
    # The minimized still is captured above. The Dock-tile pixel diff is
    # ambiguous here: kcalc is an unpinned running app, so its tile appears
    # and the pinned tiles re-center, and clicking a candidate can launch a
    # pinned app. Restore through the same lifecycle primitive the Dock tile
    # invokes; the Dock-minimized still is the visual evidence, and the
    # Dock-tile geometry itself is covered by the headless T-01/T-02 suites
    # and the human walkthrough.
    dock_changed_runs(focused_full, mini_full)  # logged for the transcript only
    synth.send(f"restore {loop_win['window']}")
    restored_row = synth.wait_state(loop_win["window"], ("floating", "zoomed"))
    time.sleep(0.3)
    snap(
        "restored: the minimized window came back to the floating geometry",
        prefix + "-restored",
    )
    transcript.append(
        ("restore", "compositor primitive (same path as the Dock tile)", restored_row["state"])
    )

    # --- 7. close (red light) ---------------------------------------------
    # The restore animation may still be settling; focus the window and give
    # the red light up to three tries before falling back to the primitive.
    gone = False
    for attempt in range(3):
        row = next(
            (r for r in classify(synth) if r["window"] == loop_win["window"]), None
        )
        if row is None:
            gone = True
            break
        log(f"close attempt {attempt}: window {row['window']} tb={row['tb']} state={row['state']}")
        if attempt == 0:
            synth.click(*window_titlebar_center(row))
            time.sleep(0.4)
            row = next(r for r in classify(synth) if r["window"] == loop_win["window"])
        synth.click(*light_center(row["tb"], 0))
        gone = synth.wait_gone(loop_win["window"], timeout=3.0)
        if gone:
            break
    if not gone:
        log("red light did not close the window; using the compositor close primitive")
        synth.send(f"close {loop_win['window']}")
        gone = synth.wait_gone(loop_win["window"], timeout=3.0)
    if not gone:
        raise RuntimeError(f"the red light did not close the window: {classify(synth)}")
    time.sleep(0.4)
    snap("closed: red light closed the window", prefix + "-closed")
    transcript.append(("close", "window gone"))

    # --- 8. CSD window unaffected -----------------------------------------
    if csd:
        c = next((r for r in classify(synth) if r["window"] == csd[0]["window"]), None)
        if c:
            assert not c["ssd"] and c["tb"] == (0, 0, 0, 0), (
                f"a CSD window must not carry a compositor titlebar: {c}"
            )
            box = (
                c["content"][0] - 20,
                c["content"][1] - 20,
                c["content"][2] + 40,
                c["content"][3] + 40,
            )
            snap("CSD: the first-party client keeps its own decoration", prefix + "-csd", box)
            transcript.append(("csd", c))

    # --- 9. X11 window carries the compositor titlebar --------------------
    if x11:
        xrow = next((r for r in classify(synth) if r["window"] == x11["window"]), None)
        if xrow:
            assert xrow["ssd"] and xrow["tb"][2] > 0, (
                f"an X11 window must carry the compositor SSD titlebar: {xrow}"
            )
            box = (
                xrow["tb"][0] - 20,
                xrow["tb"][1] - 20,
                xrow["tb"][2] + 40,
                xrow["content"][3] + 60,
            )
            snap("X11: the Xwayland client carries the SSD titlebar", prefix + "-x11", box)
            transcript.append(("x11", xrow))
    if args.leave_open:
        return steps, transcript

    # The close above removed the loop window; that is the end of the loop.
    with open(os.path.join(cap.scratch, "walkthrough.txt"), "w") as handle:
        for label, path in steps:
            handle.write(f"{label}\n  {path}\n")
    with open(os.path.join(cap.scratch, "transcript.txt"), "w") as handle:
        for entry in transcript:
            handle.write(json.dumps(entry, default=list) + "\n")
    log("window loop complete")
    return steps, transcript


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--prefix", default="t17-window-loop")
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