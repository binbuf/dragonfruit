#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Print the normalized centre of the first window's SSD titlebar.

Used by the decoration matrix to hover the real titlebar at every output
scale. `query scale` supplies the logical output size that absolute synthetic
pointer coordinates are normalized against; `query decorations` supplies the
titlebar rect.

Usage: t238-hover-point.py <synthetic-socket>
"""

import os
import re
import socket
import sys


def query(sock, synth, command):
    sock.sendto((command + "\n").encode(), synth)
    lines = []
    try:
        while True:
            data, _ = sock.recvfrom(65535)
            for line in data.decode(errors="replace").splitlines():
                if line == "end":
                    return lines
                lines.append(line)
    except OSError:
        return lines


def main():
    synth = sys.argv[1]
    local = synth + ".client"
    try:
        os.unlink(local)
    except OSError:
        pass
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    sock.bind(local)
    sock.settimeout(1.5)

    logical = (1920, 1200)
    for line in query(sock, synth, "query scale"):
        match = re.search(r"logical=(\d+)x(\d+)", line)
        if match:
            logical = (int(match.group(1)), int(match.group(2)))

    titlebar = None
    for line in query(sock, synth, "query decorations"):
        parts = line.split()
        if len(parts) >= 7 and parts[0] == "decoration":
            titlebar = tuple(int(value) for value in parts[3:7])
            break

    if titlebar is None:
        print("0.5 0.5")
        return 0
    x, y, width, height = titlebar
    # `titlebar_hover_at` requires the pointer over the traffic-light
    # *cluster*, not the whole titlebar. The first light is inset by the
    # chrome's traffic inset plus half a button.
    cx = x + 16
    cy = y + 16
    print(f"{cx / logical[0]:.6f} {cy / logical[1]:.6f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
