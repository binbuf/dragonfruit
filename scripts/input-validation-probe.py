#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
#
# T-03.3 hardware input validation probe.
#
# Prints the input-class matrix that `scripts/input-validation.sh` records. The
# required classes and their ids are the canonical list in
# `compositor/src/input_validation.rs` (`InputClass::ALL`); keep the CLASSES
# table below in sync with it.
#
# Inventory source, in order:
#   1. `libinput list-devices` when it lists at least one device (accurate
#      capability tokens);
#   2. `/proc/bus/input/devices` otherwise (names and handlers only; the
#      permission error that libinput hit is recorded in the header).
# No process is run when libinput is absent.

import argparse
import datetime
import platform
import shutil
import subprocess
import sys

# (id, needs-inventory-flag, headless-covered, detail-when-present)
CLASSES = [
    ("mouse", "pointer", True),
    ("keyboard", "keyboard", True),
    ("touchpad-gestures", "touchpad", True),
    ("hot-corners", "pointer", True),
    ("tablet-pen", "tablet", False),
    ("non-us-layout", "keyboard", True),
]

# `kbd`-handler devices that are not keyboards.
BUTTON_NAMES = (
    "power button",
    "sleep button",
    "video bus",
    "pc speaker",
    "lid switch",
    "hdmi",
    "gpio",
)

TABLET_KEYWORDS = ("tablet", "pen", "stylus", "eraser", "wacom")
TOUCHPAD_KEYWORDS = ("touchpad", "trackpad")


def parse_libinput(output):
    """Parse `libinput list-devices` into (name, kind) records."""
    devices = []
    name = None
    capabilities = []
    for raw in output.splitlines():
        line = raw.rstrip()
        if not line.strip():
            if name is not None:
                devices.append((name, kind_for_libinput(capabilities, name)))
                name, capabilities = None, []
            continue
        if line.startswith("Device:"):
            if name is not None:
                devices.append((name, kind_for_libinput(capabilities, name)))
            name = line.split(":", 1)[1].strip()
            capabilities = []
        elif line.startswith("Capabilities:"):
            capabilities = line.split(":", 1)[1].split()
    if name is not None:
        devices.append((name, kind_for_libinput(capabilities, name)))
    return devices


def kind_for_libinput(capabilities, name):
    caps = set(capabilities)
    lower = name.lower()
    if "keyboard" in caps:
        return "keyboard"
    if "tablet" in caps:
        return "tablet"
    if "touch" in caps:
        return "touchscreen"
    if "gesture" in caps or any(word in lower for word in TOUCHPAD_KEYWORDS):
        return "touchpad"
    if "pointer" in caps:
        return "pointer"
    return "other"


def parse_proc(output):
    """Parse /proc/bus/input/devices into (name, kind) records."""
    devices = []
    name = None
    handlers = []
    for raw in output.splitlines():
        line = raw.rstrip()
        if not line:
            if name is not None:
                devices.append((name, kind_for_proc(name, handlers)))
                name, handlers = None, []
            continue
        if line.startswith("N: Name="):
            name = line[len("N: Name=") :].strip().strip('"')
        elif line.startswith("H: Handlers="):
            handlers = line[len("H: Handlers=") :].split()
    if name is not None:
        devices.append((name, kind_for_proc(name, handlers)))
    return [d for d in devices if d[1] != "other"]


def kind_for_proc(name, handlers):
    lower = name.lower()
    if any(button in lower for button in BUTTON_NAMES):
        return "other"
    if any(word in lower for word in TABLET_KEYWORDS):
        return "tablet"
    if any(word in lower for word in TOUCHPAD_KEYWORDS):
        return "touchpad"
    if any(handler.startswith("mouse") for handler in handlers):
        return "pointer"
    if any(handler.startswith("kbd") for handler in handlers):
        return "keyboard"
    return "other"


def read_inventory():
    """Return (source-note, records, inventory-flags)."""
    libinput = shutil.which("libinput")
    if libinput:
        try:
            proc = subprocess.run(
                [libinput, "list-devices"],
                capture_output=True,
                text=True,
                timeout=10,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            proc = None
            libinput_error = str(exc)
        else:
            libinput_error = (proc.stderr or "").strip().splitlines()
            libinput_error = (
                "; ".join(libinput_error) if libinput_error else ""
            )
            devices = parse_libinput(proc.stdout)
            if devices:
                return (
                    "libinput list-devices",
                    devices,
                    flags(devices),
                    "",
                )
        if libinput_error:
            libinput_error = f"libinput list-devices unavailable: {libinput_error}"
    else:
        libinput_error = "libinput not installed"

    try:
        with open("/proc/bus/input/devices", encoding="utf-8") as handle:
            devices = parse_proc(handle.read())
        source = "/proc/bus/input/devices"
    except OSError as exc:
        devices = []
        source = f"/proc/bus/input/devices unreadable ({exc.strerror})"
    return source, devices, flags(devices), libinput_error


def flags(devices):
    kinds = {kind for _, kind in devices}
    return {
        "pointer": "pointer" in kinds or "touchpad" in kinds,
        "keyboard": "keyboard" in kinds,
        "touchpad": "touchpad" in kinds,
        "tablet": "tablet" in kinds,
    }


def first_name(devices, kinds):
    for name, kind in devices:
        if kind in kinds:
            return name
    return None


def status_for(present, headless, seat_owned):
    if seat_owned and present:
        return "device"
    if headless:
        return "headless"
    return "gap (no-seat)" if present else "gap (no-device)"


def detail_for(class_id, present, seat_owned, devices):
    if class_id == "mouse":
        if present and seat_owned:
            return f"device: {first_name(devices, {'pointer'})}"
        if present:
            return f"pointer present ({first_name(devices, {'pointer'})}); headless path"
        return "no pointer on the seat"
    if class_id == "keyboard":
        if present and seat_owned:
            return f"device: {first_name(devices, {'keyboard'})}"
        if present:
            return f"device present ({first_name(devices, {'keyboard'})}); headless path"
        return "no keyboard on the seat"
    if class_id == "touchpad-gestures":
        if present:
            return f"touchpad present ({first_name(devices, {'touchpad'})}); headless path"
        return "no touchpad on the seat"
    if class_id == "hot-corners":
        return "compositor dwell detection; headless path (pointer present)" if present else "needs a pointer; headless path only"
    if class_id == "tablet-pen":
        if present and seat_owned:
            return f"device: {first_name(devices, {'tablet'})}"
        return "no tablet/pen on the seat (no synthetic pressure route)"
    if class_id == "non-us-layout":
        return "xkb layout mechanism tested headlessly; real layout pending"
    return ""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seat-owned", type=int, choices=(0, 1), required=True)
    parser.add_argument("--seat-detail", default="")
    args = parser.parse_args()

    source, devices, inventory, libinput_error = read_inventory()
    seat_owned = bool(args.seat_owned)

    rows = []
    gaps = []
    for class_id, flag, headless in CLASSES:
        present = inventory.get(flag, False)
        status = status_for(present, headless, seat_owned)
        if status.startswith("gap"):
            gaps.append(class_id)
        rows.append((class_id, status, detail_for(class_id, present, seat_owned, devices)))

    state = "READY" if not gaps else "OPEN"
    if not gaps:
        summary = "Input validation: READY (6/6 classes)"
    else:
        word = "gap" if len(gaps) == 1 else "gaps"
        summary = f"Input validation: OPEN ({len(gaps)} {word}: {', '.join(gaps)})"

    recorded = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    host = platform.uname()

    lines = []
    lines.append(f"T-03.3 — Hardware input validation: {state}")
    lines.append("=" * (len(lines[0]) - 1))
    lines.append("")
    lines.append(f"recorded:  {recorded}")
    lines.append(f"host:      {host.system} {host.release} {host.machine}")
    lines.append(f"result:    {summary}")
    lines.append("")
    lines.append("Seat ownership:")
    lines.append(f"  - {args.seat_detail}")
    lines.append(
        "  - real-device input validation needs the compositor to own a libinput"
    )
    lines.append(
        "    seat (T-03.2 owns the DRM-seat knowledge and the T-03.4 runbook)."
    )
    lines.append("")
    lines.append(f"Device inventory ({source}):")
    if libinput_error:
        lines.append(f"  note: {libinput_error}")
    if devices:
        width = max(len(name) for name, _ in devices)
        for name, kind in devices:
            lines.append(f"  - {name.ljust(width)}  [{kind}]")
    else:
        lines.append("  (no recognized input device)")
    lines.append("")
    lines.append("Input-class matrix:")
    width = max(len(class_id) for class_id, _, _ in rows)
    for class_id, status, detail in rows:
        lines.append(f"  {class_id.ljust(width)}  {status.ljust(16)}  {detail}")
    lines.append("")
    lines.append(
        "Headless coverage (compositor input router, libinput-equivalent synthetic events):"
    )
    lines.append("  - mouse, keyboard, touchpad-gestures, hot-corners:")
    lines.append(
        "      compositor/tests/shell_protocol_conformance.rs::"
        "synthetic_input_drives_shortcuts_hot_corners_and_gestures"
    )
    lines.append("  - non-us-layout:")
    lines.append(
        "      compositor/tests/input_validation.rs::"
        "a_non_us_layout_compiles_and_remaps_keys"
    )
    lines.append("")
    lines.append("Remaining on hardware (T-03.4 runbook / T-16 driver matrix):")
    lines.append(
        "  - run the T-01 loop on a free seat and physically use the mouse, keyboard,"
    )
    lines.append("    a touchpad (gestures), and a hot corner;")
    lines.append(
        "  - attach a pen tablet and confirm pressure reaches a client;"
    )
    lines.append(
        "  - set XKB_DEFAULT_LAYOUT to a non-US layout (e.g. de) and confirm key output."
    )
    lines.append("")
    lines.append("Reproduce:")
    lines.append("  make input-validation")

    print("\n".join(lines))


if __name__ == "__main__":
    sys.exit(main())