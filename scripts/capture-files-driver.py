#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture one Files still from the live nested demo (T-10.7).

Raises the nested Dragonfruit window over the KWin host, nudges the frame loop
through the synthetic-input harness so the last edit/selection repaints, then
screenshots the active window with Spectacle and trims it to the nested
1920x1200 output rectangle. Driven by ``scripts/capture-files.sh``; run that,
not this. Requires a host Wayland session, ``spectacle``, ``gdbus``, and
Pillow.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

from PIL import Image

NESTED_W, NESTED_H = 1920, 1200
RAISE_JS = """
function raiseDragonfruit() {
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


def log(message):
    print(f"capture-files: {message}", flush=True)


def raise_dragonfruit(scratch):
    script = os.path.join(scratch, "raise-files.js")
    with open(script, "w") as handle:
        handle.write(RAISE_JS)
    # df-allow-desktop-name: the KWin host compositor's scripting D-Bus, used
    # only to raise the nested window for the active-window capture.
    subprocess.run(["gdbus", "call", "--session", "--dest", "org.kde.KWin",
                    "--object-path", "/Scripting", "--method",
                    "org.kde.kwin.Scripting.loadScript", script,
                    "df_capture_files_raise"], stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL)
    subprocess.run(["gdbus", "call", "--session", "--dest", "org.kde.KWin",
                    "--object-path", "/Scripting", "--method",
                    "org.kde.kwin.Scripting.start"], stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL)
    time.sleep(1.0)


def trim(im):
    if im.mode == "RGBA":
        bbox = im.getchannel("A").getbbox()
        if bbox:
            im = im.crop(bbox)
    im = im.convert("RGB")
    if im.width >= NESTED_W and im.height >= NESTED_H:
        x0 = (im.width - NESTED_W) // 2
        y0 = im.height - NESTED_H
        im = im.crop((x0, y0, x0 + NESTED_W, y0 + NESTED_H))
    return im


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--dest", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--scroll", action="store_true",
                        help="scroll the list after the nudge (large-directory trace)")
    parser.add_argument("--dock-crop", help="also save the Dock band to this path")
    args = parser.parse_args()

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    client = f"/tmp/opencode-capture-files-{os.getpid()}.sock"
    try:
        os.unlink(client)
    except FileNotFoundError:
        pass
    sock.bind(client)

    # The compositor clears the nested output with the active Space wallpaper;
    # the dark scheme is the tone the host desktop does not share, so the
    # nested window identifies cleanly after the alpha trim.
    sock.sendto(b"set color-scheme dark", args.synth)
    time.sleep(0.5)
    raise_dragonfruit(args.scratch)
    for step in range(16):
        x = 0.5 + 0.002 * (step % 4)
        sock.sendto(f"motion-abs {x} 0.5".encode(), args.synth)
        time.sleep(0.04)

    if args.scroll:
        sock.sendto(b"motion-abs 0.62 0.60", args.synth)
        time.sleep(0.3)
        for _ in range(60):
            sock.sendto(b"axis 0 10", args.synth)
            time.sleep(0.02)
        time.sleep(0.8)

    time.sleep(0.8)
    raw = os.path.join(args.scratch, os.path.basename(args.dest) + "-raw.png")
    subprocess.run(["spectacle", "-b", "-n", "-a", "-o", raw],
                   check=True, stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL)
    im = trim(Image.open(raw))
    im.save(args.dest)
    log(f"saved {args.dest} ({im.width}x{im.height})")
    if args.dock_crop:
        # The Dock sits at the bottom centre of the nested output.
        dock = im.crop((int(im.width * 0.29), im.height - 200,
                        int(im.width * 0.71), im.height))
        dock.save(args.dock_crop)
        log(f"saved {args.dock_crop} ({dock.width}x{dock.height})")


if __name__ == "__main__":
    main()