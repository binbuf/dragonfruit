#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the live nested menu bar and capture the T-07.5a status menus.

Machine half of the T-07.5a capture: on the running nested session it clicks
the Wi-Fi and volume status items through the real compositor input path (the
T-03 synthetic-input harness, installed when ``DRAGONFRUIT_SYNTHETIC_INPUT``
is set) and screenshots each open popover with Spectacle. Driven by
``scripts/capture-status-menus.sh``.

The status row is right-anchored, so the item centers are a fixed number of
pixels from the right edge of the output (measured from the headless QML test
at 1280 wide: Wi-Fi 199 px, volume 170 px, bar centre y = 14 px).

Requires a host Wayland session, ``spectacle``, and the built tree.
"""
import argparse
import os
import socket
import subprocess
import sys
import time

NESTED_W, NESTED_H = 1920, 1200
BTN_LEFT = 0x110

# Pixels from the output's right edge to the centre of each status item; the
# bar's centre row is at y = 14 (output pixels). Measured from the live shell
# (the placeholder demo shows Wi-Fi + Bluetooth + volume + battery + clock +
# Control Center + Mission Control): Wi-Fi centre x = 1615 px and volume
# x = 1673 px on a 1920-wide output.
WIFI_FROM_RIGHT = 305
VOLUME_FROM_RIGHT = 247
BAR_Y = 14


def log(message):
    print(f"capture: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/dragonfruit-capture-status-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(2.0)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def click(self, x, y):
        self.send(f"motion-abs {x / NESTED_W} {y / NESTED_H}")
        time.sleep(0.15)
        self.send(f"button {BTN_LEFT} down")
        self.send(f"button {BTN_LEFT} up")
        time.sleep(0.1)


def shot(dest):
    subprocess.run(
        ["spectacle", "-b", "-n", "-f", "-o", dest],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--settle", type=float, default=8.0)
    args = parser.parse_args()

    os.makedirs(args.outdir, exist_ok=True)
    for tool in ("spectacle",):
        if subprocess.run(["which", tool], stdout=subprocess.DEVNULL).returncode != 0:
            log(f"{tool} not found — needs a host Wayland session")
            return 1

    time.sleep(args.settle)
    synth = Synthetic(args.synth)

    wifi_x = NESTED_W - WIFI_FROM_RIGHT
    volume_x = NESTED_W - VOLUME_FROM_RIGHT

    log(f"clicking Wi-Fi status item at ({wifi_x}, {BAR_Y})")
    synth.click(wifi_x, BAR_Y)
    time.sleep(0.8)
    wifi_shot = os.path.join(args.outdir, "t07.5a-wifi.png")
    shot(wifi_shot)
    log(f"saved {wifi_shot}")

    log(f"clicking volume status item at ({volume_x}, {BAR_Y})")
    synth.click(volume_x, BAR_Y)
    time.sleep(0.8)
    volume_shot = os.path.join(args.outdir, "t07.5a-volume.png")
    shot(volume_shot)
    log(f"saved {volume_shot}")
    return 0


if __name__ == "__main__":
    sys.exit(main())