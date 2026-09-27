#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-14.7l Dock launch-origin tile hand-off.

Driven by ``scripts/capture-dock-launch-origin.sh`` (run that, not this).
Requires a host Wayland session, ``spectacle`` and Pillow.

The demo runs with ``DF_DOCK_ACTIVATION_FIXTURE=launch`` so the Dock launches a
pinned app through the real click tree. This driver reads the compositor's
``query motion`` record for the launched window and proves the appear started at
the Dock entry's icon (bottom band), not the centered fallback, then annotates
the still with the recorded origin.
"""
import argparse
import os
import socket
import subprocess
import sys

from PIL import Image, ImageDraw

NESTED_W, NESTED_H = 1920, 1200
# The bottom Dock band: an icon origin must fall near this edge, far below a
# centered fallback for a normal window.
DOCK_ORIGIN_MIN_Y = 1080


def log(message):
    print(f"capture-dock-launch-origin: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-launch-origin-{os.getpid()}.sock"
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


def parse_motion(report):
    rows = []
    for line in report.splitlines():
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


def nested_window(raw_path):
    """Crop the host still to the nested window via the alpha bbox."""
    im = Image.open(raw_path)
    if im.mode == "RGBA":
        alpha = im.getchannel("A")
        bbox = alpha.point(lambda a: 255 if a == 255 else 0).getbbox()
        if bbox:
            im = im.crop(bbox)
    im = im.convert("RGB")
    w, h = im.size
    if w >= NESTED_W and h >= NESTED_H:
        x0 = (w - NESTED_W) // 2
        y0 = h - NESTED_H
        im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
    return im


def run(args):
    synth = Synthetic(args.synth)
    report = synth.query("query motion")
    motions = parse_motion(report)
    appears = [m for m in motions if m["kind"] == "appear"]
    if not appears:
        log("no appear motion recorded; the launch did not happen")
        log(report)
        return 1
    appear = appears[-1]
    ox, oy, ow, oh = appear["origin"]
    tx, ty, tw, th = appear["target"]
    log(f"appear origin={appear['origin']} target={appear['target']}")
    if oy + oh < DOCK_ORIGIN_MIN_Y:
        log(f"origin is not in the Dock band (need y+h >= {DOCK_ORIGIN_MIN_Y})")
        return 1

    raw = os.path.join(args.scratch, "launch-origin-raw.png")
    subprocess.run(
        ["spectacle", "-b", "-n", "-a", "-o", raw],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    nested = nested_window(raw)
    draw = ImageDraw.Draw(nested)
    # The recorded appear origin (the Dock tile) and the settled window.
    draw.rectangle([ox, oy, ox + ow, oy + oh], outline=(255, 64, 64), width=4)
    draw.rectangle([tx, ty, tx + tw, ty + th], outline=(64, 160, 255), width=3)
    draw.text((ox, max(0, oy - 22)), "appear origin = entry icon", fill=(255, 90, 90))
    dest = os.path.join(args.outdir, "t14-dock-launch-origin.png")
    nested.resize((960, 600), Image.LANCZOS).save(dest)
    log(f"saved {dest}")
    with open(os.path.join(args.outdir, "t14-dock-launch-origin-trace.txt"), "w") as handle:
        handle.write(f"appear origin={appear['origin']}\n")
        handle.write(f"appear target={appear['target']}\n")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    args = parser.parse_args()
    try:
        return run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1


if __name__ == "__main__":
    sys.exit(main())