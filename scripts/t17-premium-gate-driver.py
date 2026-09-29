#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the live nested desktop for the T-17 premium-gate sign-off (T-17.6).

This is the machine half of the T-17.6 sign-off report
(docs/tasks/158-t-17.6-unfamiliar-user-test-and-sign-off-report.md). It runs
against the *live* nested session, raises the compositor window through the
host compositor's scripting D-Bus, and captures the whole 1920x1200 nested
output (menu bar, Dock, wallpaper, first-party CSD window, X11 window) to
``docs/captures/t17-premium-gate.png``. It also records the live
``query material`` / ``query degrade`` / ``query spaces`` / ``query grid``
state into a transcript the shell driver folds into
``docs/captures/t17-premium-gate.txt``.

The report is the reviewed companion: ``docs/captures/t17-premium-gate.md``.
The task owns no new surface, so this capture confirms the assembled desktop
still renders; the per-behaviour evidence is the T-17.1a/1b/1c, T-17.3,
T-17.5a, and T-17.5b captures it indexes.

It is driven by ``scripts/capture-t17-premium-gate.sh``; run that, not this.
It reuses the T-17.1a active-window capturer (KWin raise + ``spectacle -a``)
from ``t17-window-loop-driver.py``. Requires a host Wayland session,
``spectacle``, Pillow, and the nested demo running with the synthetic-input
harness.
"""
import argparse
import importlib.util
import os
import signal
import sys
import time

_HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "t17_window_loop_driver", os.path.join(_HERE, "t17-window-loop-driver.py")
)
_loop = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_loop)

Synthetic = _loop.Synthetic
Capture = _loop.Capture


class CaptureTimeout(Exception):
    pass


def _alarm(signum, frame):
    raise CaptureTimeout("capture step exceeded its deadline")


signal.signal(signal.SIGALRM, _alarm)


def timed(seconds, fn, *args, **kwargs):
    signal.alarm(seconds)
    try:
        return fn(*args, **kwargs)
    finally:
        signal.alarm(0)


def log(message):
    print(f"t17-gate: {message}", flush=True)


def run(args):
    synth = Synthetic(args.synth)
    cap = Capture(synth, args.outdir, args.scratch)

    rows = synth.decorations()
    if not rows:
        raise RuntimeError("no windows were announced by the nested session")
    ids = synth.identities()
    log("windows: " + ", ".join(
        f"{r['window']}({ids.get(r['window'], ('', ''))[0]}, ssd={int(r['ssd'])})"
        for r in rows
    ))

    transcript = []
    transcript.append("state: live nested session (assembled desktop)")
    for label, command in (
        ("spaces", "query spaces"),
        ("grid", "query grid"),
        ("material", "query material"),
        ("degrade", "query degrade"),
    ):
        transcript.append(f"  {label}: " + synth.query(command).strip().replace("\n", " | "))
    transcript.append("  windows: " + ", ".join(
        f"{r['window']}:{ids.get(r['window'], ('', ''))[0]} ssd={int(r['ssd'])}"
        for r in rows
    ))

    full = timed(30, cap.full, "premium-gate")
    dest = cap.still(full, "t17-premium-gate.png")
    transcript.append(f"  capture: {os.path.basename(dest)} ({full.width}x{full.height})")

    with open(os.path.join(cap.scratch, "premium-gate.txt"), "w") as handle:
        handle.write("\n".join(transcript) + "\n")
    log("premium-gate capture complete")
    return transcript


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--prefix", default="t17-premium-gate")
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())