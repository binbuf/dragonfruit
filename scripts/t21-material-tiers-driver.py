#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-21.2c forced-tier stills.

Driven by ``scripts/capture-t21-material-tiers.sh`` (run that, not this).
Requires a host Wayland session and Pillow. The stills are the compositor's
own render (``df_toplevel_manager.capture_screenshot``), so no host screenshot
tool is needed.

For one colour scheme and each forced degrade tier (Full, Reduced, Minimal)
the driver sends the compositor's synthetic ``set degrade-tier`` command,
picks the newest capture the shell saved after the state was applied, and crops
the requested surface (the menu bar and Dock for ``persistent``, one overlay
panel otherwise). The three tiers are also stacked into one review sheet per
surface. ``query degrade`` is recorded next to every shot so the forced state
is not inferred from the pixels alone.
"""
import argparse
import os
import socket
import sys
import time

from PIL import Image, ImageDraw

TIERS = ("full", "reduced", "minimal")
SURFACES = (
    "persistent", "dockpopover", "menupopover", "controlcenter",
    "notification", "osd", "drawer",
)


def log(message):
    print(f"capture-t21-material: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/dragonfruit-capture-t21-material-{os.getpid()}.sock"
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


def newest_capture(directory, since):
    candidates = []
    for name in os.listdir(directory):
        if not name.endswith(".png"):
            continue
        path = os.path.join(directory, name)
        try:
            mtime = os.stat(path).st_mtime
        except FileNotFoundError:
            continue
        if mtime < since or os.path.getsize(path) == 0:
            continue
        # A capture can be observed while the compositor is still writing it;
        # only accept a complete PNG.
        try:
            with Image.open(path) as image:
                image.load()
        except (OSError, ValueError):
            continue
        candidates.append((mtime, path))
    return max(candidates)[1] if candidates else None


def wait_for_capture(directory, since, timeout=12.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        path = newest_capture(directory, since)
        if path:
            return path
        time.sleep(0.2)
    return None


def bands(width, height):
    """The menu-bar and Dock bands, scaled from the 1920x1200 reference."""
    menu = (0, 0, width, max(1, round(height * 44 / 1200)))
    dock = (round(width * 560 / 1920), round(height * 1000 / 1200),
            round(width * 1360 / 1920), height)
    return menu, dock


def crop_rect(surface, width, height):
    """The visible panel region for one overlay surface.

    Fractions are scaled from the nested 1280x800 demo; the capture fixture
    always writes the primary output's logical geometry, so a different output
    size keeps the same relative framing.
    """
    if surface == "dockpopover":
        return (round(width * 0.20), round(height * 0.42),
                round(width * 0.80), height)
    if surface == "menupopover":
        return (0, 0, round(width * 0.44), round(height * 0.58))
    if surface == "controlcenter":
        return (round(width * 0.58), 0, width, round(height * 0.96))
    if surface == "notification":
        return (round(width * 0.58), 0, width, round(height * 0.24))
    if surface == "osd":
        return (round(width * 0.30), round(height * 0.30),
                round(width * 0.70), round(height * 0.70))
    if surface == "drawer":
        return (round(width * 0.10), round(height * 0.10),
                round(width * 0.90), round(height * 0.90))
    raise ValueError(f"no crop for surface {surface!r}")


def stack(panels, labels, dest, label_h=22, gap=8, bg=(24, 24, 28)):
    w = max(p.width for p in panels)
    h = sum(p.height for p in panels) + gap * (len(panels) - 1) + label_h * len(panels)
    out = Image.new("RGB", (w, h), bg)
    draw = ImageDraw.Draw(out)
    y = 0
    for panel, label in zip(panels, labels):
        draw.text((6, y + 4), label, fill=(220, 220, 224))
        y += label_h
        out.paste(panel, ((w - panel.width) // 2, y))
        y += panel.height + gap
    out.save(dest)
    log(f"saved {dest} ({out.width}x{out.height})")


def run(args):
    os.makedirs(args.outdir, exist_ok=True)
    os.makedirs(args.scratch, exist_ok=True)
    synth = Synthetic(args.synth)
    trace = []

    # The compositor's own scheme follows the synthetic command; the shell
    # QML Theme follows settingsd, which the caller sets to the same scheme.
    synth.send(f"set color-scheme {args.scheme}")
    time.sleep(0.5)

    for tier in TIERS:
        synth.send(f"set degrade-tier {tier}")
        started = time.time()
        # Let the role event reach the shell and the next capture land after it.
        time.sleep(1.8)
        shot = wait_for_capture(args.shots, started + 1.0)
        if not shot:
            raise RuntimeError(f"no compositor capture appeared for {args.scheme}/{tier}")
        state = synth.query("query degrade").strip()
        log(f"{args.surface}/{args.scheme}/{tier}: {state}")
        trace.append(
            f"surface={args.surface} scheme={args.scheme} forced={tier}\n{state}\n")

        image = Image.open(shot).convert("RGB")
        if args.surface == "persistent":
            menu_band, dock_band = bands(image.width, image.height)
            panels = (
                ("menubar", image.crop(menu_band)),
                ("dock", image.crop(dock_band)),
            )
        else:
            panels = ((args.surface,
                       image.crop(crop_rect(args.surface, image.width,
                                            image.height))),)
        for surface, panel in panels:
            path = os.path.join(
                args.outdir,
                f"t21-material-{surface}-{tier}-{args.scheme}.png")
            panel.save(path)
            log(f"saved {path}")

    if args.compose:
        surfaces = ("menubar", "dock") if args.surface == "persistent" \
            else (args.surface,)
        for surface in surfaces:
            panels = []
            for tier in TIERS:
                path = os.path.join(
                    args.outdir,
                    f"t21-material-{surface}-{tier}-{args.scheme}.png")
                panels.append(Image.open(path).convert("RGB"))
            dest = os.path.join(
                args.outdir,
                f"t21-material-{surface}-tiers-{args.scheme}.png")
            stack(panels, [f"{tier}" for tier in TIERS], dest)

    with open(os.path.join(args.outdir, "t21-material-tiers.txt"), "a") as handle:
        handle.write("\n".join(trace))
        handle.write("\n")
    log("done")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--shots", required=True)
    parser.add_argument("--scheme", required=True, choices=("light", "dark"))
    parser.add_argument("--surface", default="persistent", choices=SURFACES)
    parser.add_argument("--compose", action="store_true")
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
