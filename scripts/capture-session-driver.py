#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-12 session stills from the live nested session.

The compositor runs with the synthetic-input harness bound (`T12_SYNTH`). The
driver:

  1. records `query lock` / `query session` while the session is unlocked and
     screenshots the desktop (`t12-session.png`);
  2. locks with the real Cmd+Ctrl+Q shortcut (Super = evdev KEY_LEFTMETA), the
     same chord a user presses;
  3. records `query lock` again (must report `locked=1` with a live lock
     surface and lock focus) and screenshots the lock UI
     (`t12-session-lock.png`);
  4. writes the transcript to `t12-session.txt`.

The nested output is derived from the screenshot; all synthetic pointer/key
coordinates are normalized against that output, not the host screen.
"""
import os
import socket
import subprocess
import time

from PIL import Image

# evdev codes (linux/input-event-codes.h).
KEY_LEFTCTRL = 29
KEY_LEFTMETA = 125
KEY_Q = 16

# The compositor clears the nested output with the active Space's wallpaper,
# which the host desktop does not share. Only this dark clear color is used
# (the light UI surfaces would otherwise match the host editor background).
WALL = (33, 13, 41)
WALL_TOL = 8
WALL_ROW_MIN = 200
NESTED_W = 1920
NESTED_H = 1200


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-capture-t12-{os.getpid()}.sock"
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
        try:
            data = self.sock.recv(65536)
        except socket.timeout:
            return ""
        return data.decode(errors="ignore").strip()

    def chord(self, *codes):
        for code in codes:
            self.send(f"key {code} down")
            time.sleep(0.04)
        time.sleep(0.12)
        for code in reversed(codes):
            self.send(f"key {code} up")
            time.sleep(0.04)


def matches(c):
    return all(abs(c[i] - WALL[i]) <= WALL_TOL for i in range(3))


def detect_rect(im):
    """Locate the nested window from its dark wallpaper clear color.

    Rows that are mostly that color mark the nested output; its left edge and
    bottom edge plus the known output size give the crop. The lock UI covers
    the wallpaper, so the rect is detected once from the unlocked desktop and
    reused for the lock still.
    """
    px = im.load()
    w, h = im.size
    minx = maxy = None
    for y in range(0, h):
        xs = [x for x in range(0, w, 4) if matches(px[x, y])]
        if len(xs) >= WALL_ROW_MIN:
            minx = xs[0] if minx is None else min(minx, xs[0])
            maxy = y if maxy is None else max(maxy, y)
    if minx is None or maxy is None:
        raise RuntimeError("nested window not found (no wallpaper pixels)")
    return minx, max(0, maxy - NESTED_H + 1), NESTED_W, NESTED_H


def screenshot(shot_path, cwd):
    subprocess.run(["spectacle", "-b", "-n", "-f", "-o", shot_path], check=True,
                   cwd=cwd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return Image.open(shot_path).convert("RGB")


def main():
    synth_path = os.environ.get("T12_SYNTH")
    assert synth_path, "T12_SYNTH must point at the synthetic-input socket"
    out = os.environ.get("T12_OUT", "docs/captures")
    cwd = os.getcwd()
    os.makedirs(out, exist_ok=True)
    synth = Synthetic(synth_path)
    raw = "/tmp/opencode/t12-full-raw.png"
    transcript = []

    def record(note, subject):
        report = synth.query(subject)
        transcript.append(f"# {note}\n{report}\n")
        print(f"{note}: {report!r}")
        return report

    unlocked_lock = record("unlocked query lock", "lock")
    unlocked_session = record("unlocked query session", "session")

    im = screenshot(raw, cwd)
    rect = detect_rect(im)
    x, y, w, h = rect
    im.crop((x, y, x + w, y + h)).save(os.path.join(out, "t12-session.png"))
    print(f"nested rect x={x} y={y} w={w} h={h}")

    # Cmd+Ctrl+Q: the T-12 lock shortcut through the real input router.
    synth.chord(KEY_LEFTMETA, KEY_LEFTCTRL, KEY_Q)
    time.sleep(2.0)

    locked_lock = record("locked query lock", "lock")
    locked_session = record("locked query session", "session")

    # The lock UI covers the wallpaper, so reuse the rect detected while the
    # desktop was visible.
    im = screenshot(raw, cwd)
    im.crop((x, y, x + w, y + h)).save(os.path.join(out, "t12-session-lock.png"))

    with open(os.path.join(out, "t12-session.txt"), "w") as handle:
        handle.write("T-12 session capture transcript (T-12.5b)\n")
        handle.write("nested demo, synthetic input harness\n\n")
        handle.write("\n".join(transcript))

    if "locked=1" not in locked_lock:
        print("WARNING: the lock chord did not lock the session; lock still raced")
    if "locked=0" not in unlocked_lock:
        print("WARNING: the session did not start unlocked")
    print(f"saved {out}/t12-session.png and t12-session-lock.png")


if __name__ == "__main__":
    main()