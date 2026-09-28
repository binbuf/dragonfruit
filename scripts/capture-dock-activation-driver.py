#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T-14.7x Dock activation / launch capture driver.

Driven by ``scripts/capture-dock-activation.sh`` (run that, not this). Requires
a host Wayland session, ``spectacle`` and Pillow.

The T-14.7g seam (``DF_DOCK_ACTIVATION_FIXTURE``) retired: this driver now
performs a *real* stationary click through the compositor's synthetic pointer
(same path a mouse takes: ``onDockPointerMoved`` + ``onDockPointerButton`` into
the offscreen Dock window). It finds the pinned Settings entry by scanning the
left half of the Dock for the launched window (``query identity``), so the
capture does not depend on hard-coded entry coordinates.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

# The bottom Dock band and the resting artwork y, matching the sibling Dock
# capture drivers (T-14.7b). Clicks are normalized to the nested output, so no
# host-window rect is needed for input; only the screenshot crop needs Pillow.
DOCK_Y = 1155
SCAN_LEFT = 770
SCAN_RIGHT = 990
SCAN_STEP = 10


def log(message):
    print(f"capture-dock-activation: {message}", flush=True)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-dock-activation-{os.getpid()}.sock"
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

    def identities(self):
        """{window_id: app_id} from ``query identity``."""
        out = {}
        for line in self.query("query identity").splitlines():
            parts = line.split(" ", 3)
            if len(parts) >= 3 and parts[0] == "identity":
                out[int(parts[1])] = parts[2]
        return out

    def click(self, x, y):
        self.send(f"motion-abs {x / 1920:.5f} {y / 1200:.5f}")
        time.sleep(0.05)
        self.send("button 272 down")
        self.send("button 272 up")
        time.sleep(0.4)


def screenshot(dest):
    subprocess.run(
        ["spectacle", "-b", "-n", "-a", "-o", dest],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def scan_for_app(synth, needle):
    """Click across the pinned half of the Dock until an app_id matches
    `needle`; returns the x that launched it (or None)."""
    before = synth.identities()
    for x in range(SCAN_LEFT, SCAN_RIGHT + 1, SCAN_STEP):
        synth.click(x, DOCK_Y)
        after = synth.identities()
        for win, app_id in after.items():
            if win not in before and needle in app_id.lower():
                return x
    return None


def run(args):
    synth = Synthetic(args.synth)
    if args.mode == "launch":
        x = scan_for_app(synth, "settings")
        if x is None:
            log("no Settings window launched by the Dock scan")
            return 1
        log(f"launched Settings from the pinned entry at x={x}")
        with open(args.entry_x, "w") as handle:
            handle.write(str(x))
        screenshot(args.raw)
        log(f"saved {args.raw}")
        return 0
    if args.mode == "click":
        x = args.x
        if x <= 0:
            try:
                with open(args.entry_x) as handle:
                    x = int(handle.read().strip())
            except (FileNotFoundError, ValueError):
                log("no entry x available for the click")
                return 1
        synth.click(x, DOCK_Y)
        # Let the activation/focus settle before the still.
        time.sleep(0.8)
        screenshot(args.raw)
        log(f"clicked the pinned entry at x={x} and saved {args.raw}")
        return 0
    log(f"unknown mode {args.mode!r}")
    return 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--mode", required=True, choices=("launch", "click"))
    parser.add_argument("--raw", required=True)
    parser.add_argument("--entry-x", default="")
    parser.add_argument("--x", type=int, default=-1)
    args = parser.parse_args()
    try:
        return run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1


if __name__ == "__main__":
    sys.exit(main())