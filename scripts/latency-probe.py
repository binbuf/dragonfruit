#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Record input-to-photon latency from a running Dragonfruit compositor.

Machine half of the T-03.1b latency capture. It talks to the compositor's
synthetic-input harness (``DRAGONFRUIT_SYNTHETIC_INPUT``), injects real input
through the normal input router, and reads the compositor's latency instrument
back over the ``query latency`` reply. One sample is recorded per input that
actually caused a presented frame (the instrument's ``samples`` counter
advances); inputs that produce no damage are skipped, not fabricated.

Driven by ``scripts/latency-trace.sh``; run that, not this.
"""
import argparse
import os
import socket
import statistics
import sys
import time

# evdev codes (the backend adds the xkb +8 offset, like libinput).
KEY_LEFTCTRL = 29
KEY_UP = 103
KEY_ESCAPE = 1


def log(message):
    print(f"latency-probe: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        self.client = f"{runtime}/dragonfruit-latency-probe-{os.getpid()}.sock"
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

    def latency(self):
        """Return (last_us, samples, dropped) from the instrument."""
        fields = {}
        for line in self.query("query latency").splitlines():
            if not line.startswith("latency "):
                continue
            for token in line.split()[1:]:
                key, _, value = token.partition("=")
                fields[key] = value
        return (
            int(fields.get("last_us", 0)),
            int(fields.get("samples", 0)),
            int(fields.get("dropped", 0)),
        )

    def open_overview(self):
        self.send(f"key {KEY_LEFTCTRL} down")
        self.send(f"key {KEY_UP} down")
        time.sleep(0.03)
        self.send(f"key {KEY_UP} up")
        self.send(f"key {KEY_LEFTCTRL} up")

    def close_overview(self):
        self.send(f"key {KEY_ESCAPE} down")
        self.send(f"key {KEY_ESCAPE} up")


def run(args):
    synth = Synthetic(args.synth)
    samples = []
    skipped = 0
    dropped = 0

    # Enter Mission Control, then alternate close/open so every injection
    # changes the scene and produces at least one presented frame.
    synth.open_overview()
    time.sleep(args.interval * 3)

    for index in range(args.samples):
        _, before, _ = synth.latency()
        if index % 2 == 0:
            synth.open_overview()
        else:
            synth.close_overview()
        time.sleep(args.interval)
        last_us, after, dropped_now = synth.latency()
        dropped = max(dropped, dropped_now)
        if after > before:
            samples.append(last_us)
        else:
            skipped += 1

    if not samples:
        log("no latency samples recorded (is a display session present?)")
        return 1

    samples.sort()
    count = len(samples)
    median = int(statistics.median(samples))
    p95 = samples[min(count - 1, int(count * 0.95))]
    frame_us = 1_000_000 // args.refresh_hz
    passed = samples[-1] <= frame_us

    lines = [
        "# T-03.1b input-to-photon latency (nested)",
        f"# backend=nested refresh_hz={args.refresh_hz} frame_us={frame_us}",
        f"# samples={count} skipped(no-damage)={skipped} dropped(stale)={dropped}",
        "last_us min_us median_us p95_us max_us",
    ]
    # Expensive to keep every raw sample positionally paired; the summary and
    # the full sorted tail are the evidence.
    lines.append(
        f"{samples[-1]} {samples[0]} {median} {p95} {samples[-1]}"
    )
    lines.append("# sorted_samples_us:")
    lines.append(" ".join(str(value) for value in samples))
    lines.append(
        f"# verdict: {'pass' if passed else 'fail'} "
        f"(max {samples[-1]} us <= one frame {frame_us} us)"
    )
    report = "\n".join(lines) + "\n"

    if args.out:
        os.makedirs(os.path.dirname(args.out), exist_ok=True)
        with open(args.out, "w") as handle:
            handle.write(report)
        log(f"wrote {args.out}")
    else:
        sys.stdout.write(report)

    log(
        f"nested latency: n={count} min={samples[0]}us median={median}us "
        f"p95={p95}us max={samples[-1]}us frame={frame_us}us "
        f"verdict={'pass' if passed else 'fail'} "
        f"(skipped={skipped} dropped={dropped})"
    )
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True, help="synthetic-input socket path")
    parser.add_argument("--samples", type=int, default=50)
    parser.add_argument("--interval", type=float, default=0.12)
    parser.add_argument("--refresh-hz", type=int, default=60)
    parser.add_argument("--out", default=None, help="report path (default stdout)")
    args = parser.parse_args()
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
