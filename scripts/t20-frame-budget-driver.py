#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the T-20.4 frame-budget trace on a running nested session.

Machine half of the T-20.4 frame-budget capture: it talks to the compositor's
synthetic-input harness (``DRAGONFRUIT_SYNTHETIC_INPUT``), pins the material to
``Full`` so the sampled blur/liquid-glass pass runs, drives a burst of animation
frames through the real animation clock, and reads the material/degrade state
back over ``query material`` and ``query degrade``. The shell script
(``scripts/t20-frame-budget.sh``) starts the demo, calls this, and then cleanly
stops the compositor so the ``exit`` ``dump_stats`` block lands in the log.

Requires a host Wayland session and the built tree.
"""
import argparse
import os
import socket
import sys
import time


def log(message):
    print(f"t20-frame-budget: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        self.client = f"{runtime}/dragonfruit-t20-budget-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(3.0)

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

    def close(self):
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--animate-ms", type=int, default=1200)
    parser.add_argument("--settle", type=float, default=2.0)
    args = parser.parse_args()

    synth = Synthetic(args.synth)
    try:
        # Pin Full so the trace measures the sampled material, not the fallback.
        synth.send("set degrade-tier full")
        time.sleep(0.5)
        material = synth.query("query material")
        log("material pinned to Full")
        for line in material.strip().splitlines():
            log(line)

        # Drive real frames through the animation clock, so the frame-timing
        # samples in the exit dump are compositor work, not idle.
        synth.send(f"animate-dummy {args.animate_ms}")
        time.sleep(args.settle)

        degrade = synth.query("query degrade")
        log("degrade after the Full burst")
        for line in degrade.strip().splitlines():
            log(line)
    finally:
        synth.close()


if __name__ == "__main__":
    sys.exit(main())