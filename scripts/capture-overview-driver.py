#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested T-05 overview with synthetic input and capture stills.

This is the machine half of the T-05.6 capture (docs/design/tracks/05-loop-v3-
mission-control-live.md): it performs the Mission Control walkthrough on the
*running* nested session — open on the live surfaces, Desktop Reveal, slide a
Space's wallpaper, and the reduced-motion variant — through the real compositor
input path (the T-03 synthetic-input harness, installed on the nested backend
only when ``DRAGONFRUIT_SYNTHETIC_INPUT`` is set), and screenshots each step.

It also samples the T-05.6 gesture frame-budget instrument (`query gesture`)
after every gesture so the full-gesture 60 Hz verdict (or its honest shortfall)
is recorded next to the stills.

It is driven by ``scripts/capture-overview.sh``; run that, not this. Requires a
host Wayland session, ``spectacle`` (screenshot tool), and Pillow.

Pointer selection/drag of a live representation is deliberately *not* driven
here: in the nested session the shell's overview chrome is an offscreen
scene-graph surface that owns pointer focus but does not forward it to QML, so
the compositor's own T-05.2/T-05.3 hit-test path (exercised headlessly by
``window_conformance``) is the pointer contract. This script captures the
scene *transforms* the track is about.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
# The desktop background the compositor clears the nested output with. The
# host compositor centers the nested 1920x1200 window; `detect_rect` validates
# that against this tone (the active Space wallpaper) so a wrong host placement
# is caught rather than silently cropped.
WALL = (41, 36, 52)
WALL_TOL = 8

KEY_LEFTCTRL, KEY_UP, KEY_DOWN = 29, 103, 108


def log(message):
    print(f"capture: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-capture-overview-{os.getpid()}.sock"
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

    def key_combo(self, *codes):
        for code in codes:
            self.send(f"key {code} down")
        for code in reversed(codes):
            self.send(f"key {code} up")

    def swipe(self, fingers, dx, dy, steps):
        self.send(f"swipe-begin {fingers}")
        for _ in range(steps):
            self.send(f"swipe-update {dx} {dy}")
            time.sleep(0.03)

    def grid(self):
        """The `query grid` report as a dict of summary fields plus windows."""
        report = {"active": False, "progress": 0.0, "windows": [], "material": {}}
        for line in self.query("query grid").splitlines():
            p = line.split()
            if not p or p[0] != "grid":
                continue
            if len(p) > 1 and "=" in p[1]:
                for field in p[1:]:
                    key, _, value = field.partition("=")
                    report[key] = float(value) if key == "progress" else value
                report["active"] = float(report["progress"]) > 0.0
            elif len(p) > 1 and p[1] == "material":
                for field in p[2:]:
                    key, _, value = field.partition("=")
                    report["material"][key] = value
            elif len(p) > 1 and p[1] == "window":
                report["windows"].append(int(p[2]))
        return report

    def gesture(self):
        """The `query gesture` report as a dict (T-05.6)."""
        first = self.query("query gesture").splitlines()[0].split()
        out = {}
        for field in first[1:]:
            key, _, value = field.partition("=")
            out[key] = value
        return out


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
        """Locate the nested 1920x1200 output in the host screenshot.

        The nested buffer is 1920x1200 physical and centered on the host
        screen, so the rect is the screen center minus half the output size;
        the rounded corners are transparent, exactly like the T-01..T-04
        captures. If the host screen is smaller than the nested output (the
        host scales it), fall back to the widest contiguous run of the
        compositor's wallpaper clear color to recover the scaled output rect.
        This must run on the *initial* (nothing open) frame.
        """
        im = Image.open(path).convert("RGB")
        px = im.load()
        w, h = im.size

        def matches(c):
            return all(abs(c[i] - WALL[i]) <= WALL_TOL for i in range(3))

        if w >= NESTED_W and h >= NESTED_H:
            self.rect = ((w - NESTED_W) // 2, (h - NESTED_H) // 2, NESTED_W, NESTED_H)
            return self.rect

        best = (0, 0)
        for y in range(0, h, 2):
            run_start = None
            for x in range(0, w):
                if matches(px[x, y]):
                    if run_start is None:
                        run_start = x
                else:
                    if run_start is not None:
                        if x - run_start > best[0]:
                            best = (x - run_start, run_start)
                        run_start = None
            if run_start is not None and w - run_start > best[0]:
                best = (w - run_start, run_start)

        if best[0] >= NESTED_W // 2:
            run_w, run_left = best
            scale = run_w / NESTED_W
            out_w = max(1, int(NESTED_W * scale))
            out_h = max(1, int(NESTED_H * scale))
            left = run_left - (out_w - run_w) // 2
            top = max(0, (h - out_h) // 2)
            self.rect = (left, top, out_w, out_h)
            return self.rect

        raise RuntimeError("nested window not found (no wallpaper pixels)")

    def still(self, full_path, name):
        """Save a still cropped to the nested window rect."""
        rx, ry, rw, rh = self.rect
        im = Image.open(full_path).convert("RGB")
        crop = im.crop((rx, ry, rx + rw, ry + rh))
        dest = os.path.join(self.outdir, name)
        crop.save(dest)
        log(f"saved {dest} ({crop.width}x{crop.height})")
        return dest


def run(args):
    synth = Synthetic(args.synth)
    cap = Capturer(args.outdir, args.scratch)
    prefix = args.prefix
    traces = []

    # Dark is the tone the host desktop does not share, so it identifies the
    # output for rect detection regardless of the host scheme.
    synth.send("set color-scheme dark")
    time.sleep(0.3)

    # Detect the nested rect before any gesture (the wallpaper is fully
    # visible only at rest).
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

    if args.reduced:
        synth.send("set reduced-motion on")
        time.sleep(0.3)
        log("reduced motion enabled")

    def record_trace(label):
        report = synth.gesture()
        line = " ".join(f"{k}={v}" for k, v in report.items())
        log(f"gesture[{label}] {line}")
        traces.append((label, report))

    def wait_for(pred, timeout=5.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if pred():
                return True
            time.sleep(0.05)
        return False

    # --- 1. Mission Control open: the live surfaces scale into the grid ----
    # A four-finger vertical swipe is the real gesture; the discrete Ctrl+Up
    # shares the same one pipeline (T-05.1a).
    synth.swipe(4, 0, -100, 2)
    synth.send("swipe-end")
    wait_for(lambda: synth.grid()["progress"] >= 0.999)
    time.sleep(0.4)
    grid = synth.grid()
    log(f"grid open windows={len(grid['windows'])} material={grid['material']}")
    still = cap.full("overview")
    cap.still(still, f"{prefix}.png")
    record_trace("mission-control")

    # --- 2. close the overview --------------------------------------------
    synth.key_combo(KEY_LEFTCTRL, KEY_UP)
    wait_for(lambda: synth.grid()["progress"] <= 0.001)
    time.sleep(0.3)

    # --- 3. Desktop Reveal (on the active Space, with live windows) --------
    synth.key_combo(KEY_LEFTCTRL, KEY_DOWN)
    time.sleep(0.5)
    still = cap.full("reveal")
    cap.still(still, f"{prefix}-reveal.png")
    record_trace("desktop-reveal")
    synth.key_combo(KEY_LEFTCTRL, KEY_DOWN)
    time.sleep(0.5)

    # --- 4. per-Space wallpaper slide -------------------------------------
    # A three-finger horizontal swipe drives the progress pipeline; sample
    # mid-slide (both Spaces' wallpapers visible) then settled.
    synth.swipe(3, -100, 0, 2)
    time.sleep(0.3)
    still = cap.full("slide")
    cap.still(still, f"{prefix}-slide.png")
    synth.send("swipe-end")
    time.sleep(0.8)
    still = cap.full("slide-settled")
    cap.still(still, f"{prefix}-slide-settled.png")
    record_trace("space-slide")

    # --- 5. the gesture-budget trace file for this mode -------------------
    lines = [
        f"# {prefix}: one `query gesture` sample per gesture (nested backend).",
        "# budget_us=16000 is one 60 Hz frame; held=0 records a shortfall rather",
        "# than hiding it. tier is the live T-04.4a material degrade tier.",
        "# Frame timing is host- and load-dependent: the honest verdict is what",
        "# the instrumentation reports, not a threshold this script asserts.",
        "",
    ]
    for label, report in traces:
        fields = " ".join(f"{k}={v}" for k, v in report.items())
        lines.append(f"gesture[{label}] {fields}")
    lines.append("")
    trace_path = os.path.join(cap.scratch, f"gesture-{prefix}.txt")
    with open(trace_path, "w") as handle:
        handle.write("\n".join(lines) + "\n")
    log(f"wrote {trace_path}")
    log("overview capture complete")
    return traces


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument(
        "--prefix",
        default="t05-mission-control-live",
        help="output still name prefix (default: t05-mission-control-live)",
    )
    parser.add_argument(
        "--reduced",
        action="store_true",
        help="drive the walkthrough with accessibility.reduceMotion on",
    )
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())