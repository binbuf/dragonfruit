#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Open About through the nested system menu and exercise More Info."""
import argparse
import os
import socket
import time

OUTPUT_W, OUTPUT_H = 1920, 1200
LEFT_BUTTON = 0x110


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"{os.environ['XDG_RUNTIME_DIR']}/t230c-about-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(1)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def click(self, x, y):
        self.send(f"motion-abs {x / OUTPUT_W:.6f} {y / OUTPUT_H:.6f}")
        time.sleep(0.15)
        self.send(f"button {LEFT_BUTTON} down")
        self.send(f"button {LEFT_BUTTON} up")

    def windows(self):
        self.send("query decorations")
        lines = []
        while True:
            try:
                chunk = self.sock.recv(65536).decode()
            except socket.timeout:
                break
            lines.extend(chunk.splitlines())
            if "end\n" in chunk:
                break
        result = []
        for line in lines:
            parts = line.split()
            if parts and parts[0] == "decoration" and len(parts) > 10:
                result.append(tuple(int(v) for v in parts[7:11]))
        return result

    def close(self):
        self.sock.close()
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass


def wait_window(synth, width, height, timeout=12):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for rect in synth.windows():
            if rect[2:] == (width, height):
                return rect
        time.sleep(0.25)
    raise RuntimeError(f"{width}x{height} About window did not map; windows={synth.windows()}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--mode", choices=("panel", "page"), required=True)
    args = parser.parse_args()
    synth = Synthetic(args.synth)
    try:
        # System mark, then the first row of its dropdown (About System).
        synth.click(20, 14)
        time.sleep(0.5)
        synth.click(82, 48)
        panel = wait_window(synth, 280, 503)
        print(f"t230c-live: system menu opened 280x503 panel at {panel}", flush=True)
        if args.mode == "page":
            synth.click(panel[0] + 140, panel[1] + 417)
            page = wait_window(synth, 740, 820)
            print(f"t230c-live: More Info opened 740x820 page at {page}", flush=True)
        time.sleep(1)
    finally:
        synth.close()


if __name__ == "__main__":
    main()
