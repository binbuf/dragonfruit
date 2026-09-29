#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
#
# T-03.4 multi-GPU import/fallback probe.
#
# Prints the multi-GPU decision that `scripts/multi-gpu-validation.sh` records.
# The classification mirrors `compositor/src/multi_gpu.rs`
# (`MultiGpuOutcome::classify`): every non-primary card with its own render node
# is imported across devices; a card without a render node falls back to the
# primary renderer and may only import linear buffers. Keep the two in sync.
#
# The inventory comes from sysfs: `/sys/class/drm/cardN/device/drm/` lists the
# card and render nodes that belong to the same physical device. The primary
# device is the first card (the backend orders it first too).

import argparse
import datetime
import glob
import os
import platform
import sys


def driver_for(card):
    link = f"/sys/class/drm/{card}/device/driver"
    if os.path.islink(link):
        return os.path.basename(os.path.realpath(link))
    return "unknown"


def render_node_for(card):
    base = f"/sys/class/drm/{card}/device/drm"
    nodes = sorted(glob.glob(f"{base}/renderD*"))
    return os.path.basename(nodes[0]) if nodes else None


def read_gpus():
    gpus = []
    for path in sorted(glob.glob("/dev/dri/card*")):
        card = os.path.basename(path)
        gpus.append(
            {
                "card": card,
                "render_node": render_node_for(card),
                "driver": driver_for(card),
            }
        )
    return gpus


def classify(gpus):
    if not gpus:
        state = "NONE"
        summary = "Multi-GPU: NONE (no DRM device on the seat)"
    elif len(gpus) == 1:
        state = "SINGLE"
        summary = f"Multi-GPU: SINGLE device={gpus[0]['card']}"
    else:
        secondaries = gpus[1:]
        fallback = [g for g in secondaries if not g["render_node"]]
        if not fallback:
            state = "IMPORTED"
            summary = (
                f"Multi-GPU: IMPORTED devices={len(gpus)} "
                f"secondaries={len(secondaries)} (per-device render nodes)"
            )
        else:
            state = "FALLBACK"
            summary = (
                f"Multi-GPU: FALLBACK devices={len(gpus)} "
                f"secondaries={len(secondaries)} fallback={len(fallback)} "
                f"(primary renderer, linear import)"
            )
    return state, summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seat-owned", type=int, choices=(0, 1), required=True)
    parser.add_argument("--seat-detail", default="")
    args = parser.parse_args()

    gpus = read_gpus()
    state, summary = classify(gpus)

    recorded = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    host = platform.uname()

    lines = []
    lines.append(f"T-03.4 — Multi-GPU import/fallback: {state}")
    lines.append("=" * (len(lines[0]) - 1))
    lines.append("")
    lines.append(f"recorded:  {recorded}")
    lines.append(f"host:      {host.system} {host.release} {host.machine}")
    lines.append(f"result:    {summary}")
    lines.append("")
    lines.append("Seat ownership:")
    lines.append(f"  - {args.seat_detail}")
    lines.append(
        "  - the compositor opens DRM devices through the seat; on this host a"
    )
    lines.append(
        "    free seat is a precondition for any card to be composited on."
    )
    lines.append("")
    lines.append("GPU inventory (/sys/class/drm):")
    if gpus:
        width = max(len(g["card"]) for g in gpus)
        for gpu in gpus:
            render = gpu["render_node"] or "no render node (primary-renderer fallback)"
            lines.append(f"  - {gpu['card'].ljust(width)}  -> {render}  [driver={gpu['driver']}]")
    else:
        lines.append("  (no /dev/dri/card* node)")
    lines.append("")
    lines.append("Decision (compositor/src/multi_gpu.rs; this probe mirrors it):")
    if state == "SINGLE":
        lines.append("  - one device: no cross-device import is required")
    elif state == "IMPORTED":
        lines.append(
            "  - every secondary has a render node: buffers are imported across devices"
        )
    elif state == "FALLBACK":
        lines.append(
            "  - a secondary has no render node: it renders on the primary GPU and"
        )
        lines.append("    may only import linear buffers (other modifiers are filtered)")
    else:
        lines.append("  - no card: nothing to classify")
    lines.append("")
    lines.append("Tests that pin this without hardware (make e2e):")
    lines.append("  - compositor/tests/multi_gpu.rs")
    lines.append("  - compositor/src/multi_gpu.rs unit tests")
    lines.append("")
    lines.append("Remaining on hardware (T-03.4 runbook / T-16 driver matrix):")
    if len(gpus) > 1:
        lines.append("  - run a client on each output and confirm no import errors;")
        lines.append(
            "  - if `fallback=` is non-zero, confirm the fallback output still scans out."
        )
    else:
        lines.append(
            "  - this seat has a single GPU; attach a second GPU and re-run"
        )
        lines.append("    `make multi-gpu-validation` to exercise import/fallback.")
    lines.append("")
    lines.append("Reproduce:")
    lines.append("  make multi-gpu-validation")

    print("\n".join(lines))

    # Exit 0 only when the hardware can actually exercise the multi-GPU paths
    # (more than one card). The shell script turns a 0 into the validated
    # artifact and a 1 into the explicit open.
    return 0 if len(gpus) > 1 else 1


if __name__ == "__main__":
    sys.exit(main())