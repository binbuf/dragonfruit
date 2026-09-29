#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
#
# T-03.2 DRM seat probe.
#
# Answers one question for `scripts/drm-bringup.sh`: can this process take
# DRM master on a card node? On a workstation already running a desktop the
# host compositor holds master, so every card reports busy and the T-03.2
# unit is marked OPEN. The probe never leaves a card in master state: if
# `drmSetMaster` succeeds it immediately calls `drmDropMaster`.
#
# Prints a JSON report to stdout. Exit 0 when at least one card accepted
# (and released) master, 1 otherwise, 2 when no card node exists.

import ctypes
import glob
import json
import os
import sys

DRM_IOCTL = None  # the probe only uses the exported drmSet/drmDrop/drmIsMaster


def probe(card):
    lib = ctypes.CDLL("libdrm.so.2", use_errno=True)
    lib.drmIsMaster.argtypes = [ctypes.c_int]
    lib.drmIsMaster.restype = ctypes.c_int
    lib.drmSetMaster.argtypes = [ctypes.c_int]
    lib.drmSetMaster.restype = ctypes.c_int
    lib.drmDropMaster.argtypes = [ctypes.c_int]
    lib.drmDropMaster.restype = ctypes.c_int

    report = {"path": card, "opened": False, "is_master": False,
              "can_set_master": False, "errno": 0, "error": ""}
    try:
        fd = os.open(card, os.O_RDWR | os.O_CLOEXEC)
    except OSError as exc:
        report["errno"] = exc.errno
        report["error"] = exc.strerror
        return report
    report["opened"] = True
    try:
        report["is_master"] = bool(lib.drmIsMaster(fd))
        ctypes.set_errno(0)
        if lib.drmSetMaster(fd) == 0:
            report["can_set_master"] = True
            lib.drmDropMaster(fd)
        else:
            report["errno"] = ctypes.get_errno()
            report["error"] = os.strerror(report["errno"])
    finally:
        os.close(fd)
    return report


def main():
    cards = sorted(glob.glob("/dev/dri/card*"))
    report = {
        "cards": [probe(card) for card in cards],
        "free_card": None,
    }
    for card in report["cards"]:
        if card["can_set_master"]:
            report["free_card"] = card["path"]
            break
    print(json.dumps(report, indent=2))
    if not cards:
        return 2
    return 0 if report["free_card"] else 1


if __name__ == "__main__":
    sys.exit(main())