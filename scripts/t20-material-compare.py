#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Tolerance-based comparison for T-20 material captures.

GL blur output is driver-dependent, so the T-20 captures are **not** byte
goldens. Instead a capture is reproducible if a re-run with the same static
fixture backdrop agrees within a tolerance. The comparison fails when the mean
absolute per-channel difference exceeds ``--mean`` (default 8/255), or when more
than ``--outlier-frac`` of the pixels exceed ``--max`` (default 64/255). The
outlier fraction absorbs a handful of unrelated pixels — the shell clock, a
cursor, text anti-aliasing — that are not part of the material.

Usage:
    scripts/t20-material-compare.py before.png after.png [--mean 8] [--max 64]

The static-backdrop choice (the apps-drawer and status fixtures, which pin the
bar/card content) is what keeps the *structure* identical; the tolerance covers
the driver's blur/refraction arithmetic. See docs/captures/README.md.
"""
import argparse
import sys


def load(path):
    from PIL import Image

    return Image.open(path).convert("RGBA")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("--mean", type=float, default=8.0)
    parser.add_argument("--max", type=float, default=64.0)
    parser.add_argument("--outlier-frac", type=float, default=0.005)
    args = parser.parse_args()

    a = load(args.before)
    b = load(args.after)
    if a.size != b.size:
        print(f"t20-material-compare: size mismatch {a.size} != {b.size}", file=sys.stderr)
        return 2

    pa = a.tobytes()
    pb = b.tobytes()
    total = 0
    worst = 0
    outliers = 0
    for x, y in zip(pa, pb):
        diff = abs(x - y)
        total += diff
        if diff > worst:
            worst = diff
        if diff > args.max:
            outliers += 1
    mean = total / len(pa)
    outlier_frac = outliers / len(pa)
    print(
        f"t20-material-compare: mean={mean:.2f} max={worst} "
        f"outliers={outlier_frac:.4%} "
        f"(tolerance mean<={args.mean} outlier_frac<={args.outlier_frac:.2%})"
    )
    if mean > args.mean:
        print("t20-material-compare: FAIL — mean difference exceeds tolerance", file=sys.stderr)
        return 1
    if outlier_frac > args.outlier_frac:
        print(
            "t20-material-compare: FAIL — too many pixels exceed the max tolerance",
            file=sys.stderr,
        )
        return 1
    print("t20-material-compare: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())