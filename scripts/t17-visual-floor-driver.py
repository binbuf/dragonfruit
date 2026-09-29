#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Capture the T-17.3 visual floor under dark, light, and reduced motion.

This is the machine half of the T-17.3 sign-off unit
(docs/tasks/155-t-17.3-visual-floor-and-reduced-motion-sign-off.md). It runs
against the *live* nested session and captures the same chrome in one
continuous compositor state:

* dark, light, and dark+reduced-motion variants, so the comparison cannot
  drift between demo launches;
* the whole desktop plus the menu-bar band, the compositor SSD titlebar, and
  the Dock band;
* the live ``query material`` / ``query degrade`` tones the compositor resolved
  for each variant, written to ``visual-floor.txt``.

The reduced-motion capture proves the chrome still renders with animation
disabled; the reduced-motion *behaviour* is pinned by
``compositor/tests/reduced_motion_sweep.rs`` and the gallery ``--strict`` gate.

It is driven by ``scripts/capture-t17-visual-floor.sh``; run that, not this.
It reuses the T-17.1a active-window capturer (KWin raise + ``spectacle -a``)
from ``t17-window-loop-driver.py``, which is the proven way to capture the
nested output on this host. Requires a host Wayland session, ``spectacle``,
Pillow, and the nested demo running with the synthetic-input harness.
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
NESTED_W, NESTED_H = _loop.NESTED_W, _loop.NESTED_H
# component.menuBar.height (T-17.3 menu-bar material band).
MENUBAR_H = 28


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
    print(f"t17-floor: {message}", flush=True)


def run(args):
    synth = Synthetic(args.synth)
    cap = Capture(synth, args.outdir, args.scratch)

    # The compositor SSD window is the one whose titlebar, shadow, and backdrop
    # the material pass draws; first-party CSD windows have a zero titlebar and
    # no compositor material of their own.
    rows = [r for r in synth.decorations() if r["ssd"] and r["tb"][2] > 0]
    if not rows:
        raise RuntimeError("no compositor SSD window to capture the material from")
    ssd = rows[0]
    tb = ssd["tb"]
    titlebar_box = (tb[0] - 10, tb[1] - 6, tb[2] + 20, tb[3] + 12)
    dock_band = (700, NESTED_H - 140, 560, 140)
    log(f"SSD window {ssd['window']} titlebar={tb}")

    variants = [
        ("dark", "dark", False),
        ("light", "light", False),
        ("reduced", "dark", True),
    ]
    transcript = []
    for name, scheme, reduced in variants:
        synth.send(f"set reduced-motion {'on' if reduced else 'off'}")
        synth.send(f"set color-scheme {scheme}")
        time.sleep(0.6)
        log(f"variant {name}: scheme={scheme} reduced={int(reduced)}")
        transcript.append(f"variant name={name} scheme={scheme} reduced={int(reduced)}")
        transcript.append("  " + synth.query("query material").strip().replace("\n", " | "))
        transcript.append("  " + synth.query("query degrade").strip().replace("\n", " | "))

        full = timed(30, cap.full, f"floor-{name}")
        cap.still(full, f"{args.prefix}-{name}.png")
        cap.still(full, f"{args.prefix}-{name}-menubar.png", box=(0, 0, NESTED_W, MENUBAR_H))
        cap.still(full, f"{args.prefix}-{name}-titlebar.png", box=titlebar_box)
        cap.still(full, f"{args.prefix}-{name}-dock.png", box=dock_band)

    with open(os.path.join(cap.scratch, "visual-floor.txt"), "w") as handle:
        handle.write("\n".join(transcript) + "\n")
    log("visual-floor capture complete")
    return transcript


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--prefix", default="t17-visual-floor")
    args = parser.parse_args()
    try:
        run(args)
    except Exception as err:  # noqa: BLE001 - surface the reason to the shell
        log(f"FAILED: {err}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())