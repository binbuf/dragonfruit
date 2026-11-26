#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""T347 reference-hardware daily-driver matrix.

This is the executable half of the T347 hardware gate
(``docs/tasks/stability/347-hardware-matrix.md`` and
``docs/design/27-stability-and-daily-driver.md`` §5.2). It:

1. captures the host hardware profile and probes for a free DRM seat;
2. runs the headless conformance checks that prove the state-machine half of
   each matrix row;
3. writes a complete PASS/FAIL/OPEN report to
   ``docs/captures/t347-daily-driver-matrix.md`` plus a machine-readable
   ``.json`` companion.

A host without a free DRM seat gets an ``OPEN`` row for every native check
with the exact rerun command, never a skipped row. A failed headless check is
a ``FAIL`` and blocks the gate; it is not downgraded to ``OPEN``.

Native PASS/FAIL evidence is recorded in the local, gitignored intake file
``docs/captures/t347-native-evidence.json``::

    {
      "profile": "reference-A",
      "recorded": "2026-11-25T10:00:00Z",
      "rows": {
        "cold-login": {
          "verdict": "PASS",
          "note": "4.2 s from login to usable desktop",
          "artifact": "docs/captures/t347-native/cold-login.txt"
        }
      }
    }

A PASS or FAIL intake row must name both a note and an artifact path. A
headless failure always wins over an intake PASS (fail closed).

Run it through ``make t347-daily-driver-matrix``. Overrides: ``OUT_MD``,
``OUT_JSON``, ``LOG_DIR``, ``T347_NATIVE_EVIDENCE``, ``T347_SOAK_CYCLES``,
``T347_DEV_BIN``, ``KEEP_SCRATCH``.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Iterable

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUT_MD = ROOT / "docs" / "captures" / "t347-daily-driver-matrix.md"
DEFAULT_OUT_JSON = ROOT / "docs" / "captures" / "t347-daily-driver-matrix.json"
DEFAULT_LOG_DIR = ROOT / "docs" / "captures" / "t347" / "logs"
DEFAULT_INTAKE = ROOT / "docs" / "captures" / "t347-native-evidence.json"
DEFAULT_SOAK_CYCLES = 3
INTAKE_PATH_LABEL = "docs/captures/t347-native-evidence.json"

PASS = "PASS"
FAIL = "FAIL"
OPEN = "OPEN"
VERDICTS = {PASS, FAIL, OPEN}

COMPOSITOR_ARTIFACT_SUFFIXES = (
    ".lock",
    ".launch-token",
    ".desktop-launch-token",
    ".portal-launch-token",
    ".x11-display",
)


@dataclass(frozen=True)
class RowDef:
    id: str
    title: str
    severity: str
    owner: str
    requirement: str
    checks: tuple[str, ...]
    rerun: str
    native_artifact: str


@dataclass
class HeadlessResult:
    id: str
    title: str
    command: str
    verdict: str
    detail: str
    artifact: str


@dataclass
class RowResult:
    id: str
    title: str
    severity: str
    owner: str
    requirement: str
    verdict: str
    reason: str
    command: str
    artifact: str
    checks: tuple[str, ...]


@dataclass
class SeatState:
    available: bool
    reason: str
    probe: dict = field(default_factory=dict)


@dataclass
class Intake:
    present: bool
    path: Path
    profile: str | None = None
    recorded: str | None = None
    rows: dict[str, dict] = field(default_factory=dict)
    warnings: list[str] = field(default_factory=list)


# The native rows are the design 27 §5.2 checks, with the launch, data, and
# post-resume sub-checks called out because the task scope names them. The
# extra media-key and latency rows are the hardware-open references from
# ADR 0331 and §7/G-03.
ROW_DEFS: tuple[RowDef, ...] = (
    RowDef(
        "cold-login",
        "Cold login",
        "P0",
        "T347 / T348",
        "Usable desktop in < 5 s; no error dialog; chrome and wallpaper present.",
        ("startup",),
        "make native  # time login -> usable desktop, then make native-capture",
        "docs/captures/t347-native/cold-login/",
    ),
    RowDef(
        "workday",
        "8-hour workday",
        "P0",
        "T347 / T348",
        "8 hours of normal use; no shell/compositor restart; no stuck focus; no leaked sockets.",
        ("startup", "shell-kill", "idle-trace"),
        "make native  # use for 8 h, then make native-capture and make native-status",
        "docs/captures/t347-native/workday/",
    ),
    RowDef(
        "idle-soak",
        "24-hour idle soak (dim/blank/lock/suspend)",
        "P0",
        "T347 / T348",
        "24 h; dim -> blank -> lock -> suspend at the configured stages; input wakes; no runaway CPU.",
        ("idle-policy", "idle-trace"),
        "make native; make idle-trace; make native-capture  # 24 h with the idle.* keys set",
        "docs/captures/t347-native/idle-soak/",
    ),
    RowDef(
        "suspend-resume",
        "20 suspend/resume cycles",
        "P0",
        "T347 / T348",
        "20 cycles; session survives; lock is shown on wake; Wi-Fi/Bluetooth/audio recover.",
        ("suspend", "adapters"),
        "make native; make t16-suspend-resume-soak  # 20 real cycles on reference A",
        "docs/captures/t347-native/suspend-resume/",
    ),
    RowDef(
        "lid-close",
        "20 lid-close cycles",
        "P0",
        "T347 / T348",
        "20 cycles; locks before sleep; external display resumes.",
        ("suspend",),
        "make native  # close/open the lid 20x, then make native-capture",
        "docs/captures/t347-native/lid-close/",
    ),
    RowDef(
        "lock-unlock",
        "50 lock/unlock cycles",
        "P0",
        "T347 / T348",
        "50 cycles; PAM; no key leak to a client; a shell restart while locked ends the session safely.",
        ("lock",),
        "make native; make t17-leak-lock-soak  # 50 real loginctl lock/unlock cycles",
        "docs/captures/t347-native/lock-unlock/",
    ),
    RowDef(
        "shell-kill",
        "20 shell kills (locked and unlocked)",
        "P0",
        "T347 / T348",
        "20 unlocked kills -> chrome back < 2 s; 20 locked kills -> session ends safely, never frozen; a blocked GUI thread is watchdog-killed.",
        ("shell-kill", "lock"),
        "make native  # pkill -9 -f dragonfruit-shell 20x unlocked and 20x locked",
        "docs/captures/t347-native/shell-kill/",
    ),
    RowDef(
        "hotplug",
        "20 output hotplug cycles",
        "P0",
        "T347 / T348",
        "20 plug/unplug cycles; one output each; no phantom; popups/touch on the right output.",
        ("hotplug",),
        "make native  # plug/unplug the external display 20x, then make native-capture",
        "docs/captures/t347-native/hotplug/",
    ),
    RowDef(
        "launch-x11",
        "X11 launch",
        "P0",
        "T347 / T321 / T320",
        "An X11 app receives DISPLAY and maps with SSD and working decorations.",
        ("launch",),
        "make native  # launch xmessage or xclock; make t17-window-loop-capture",
        "docs/captures/t347-native/launch-x11/",
    ),
    RowDef(
        "launch-wayland",
        "Wayland launch",
        "P1",
        "T347 / T321",
        "A Wayland app receives WAYLAND_DISPLAY and opens with the session environment.",
        ("launch",),
        "make native  # launch a Qt Wayland app; make t17-window-loop-capture",
        "docs/captures/t347-native/launch-wayland/",
    ),
    RowDef(
        "launch-flatpak",
        "Flatpak launch",
        "P1",
        "T347 / T335a / T335b",
        "A Flatpak browser launches, can file-choose, screenshot, and screen-share.",
        ("launch", "portals"),
        "make native  # run make t17-flatpak-browser-capture against reference A",
        "docs/captures/t347-native/launch-flatpak/",
    ),
    RowDef(
        "file-dialog-screenshot",
        "File dialog and screenshot",
        "P1",
        "T347 / T335a / T335b",
        "The FileChooser and Screenshot portals complete through the shell presenter; screen share falls back honestly.",
        ("portals",),
        "make native  # run make portals-capture and make t17-flatpak-browser-capture",
        "docs/captures/t347-native/file-dialog-screenshot/",
    ),
    RowDef(
        "data",
        "Cross-filesystem move",
        "P0",
        "T347 / T312 / T323",
        "An interrupted cross-filesystem move leaves the source intact; a retry completes cleanly.",
        ("data",),
        "make native  # interrupt a Files move across filesystems, then retry",
        "docs/captures/t347-native/data/",
    ),
    RowDef(
        "battery",
        "Battery drain",
        "P1",
        "T347 / T348",
        "Drain measurement on the reference laptop; no idle wakeups above the existing budget.",
        (),
        "make native; make native-perf  # upower -d before and after a 30 min idle window",
        "docs/captures/t347-native/battery/",
    ),
    RowDef(
        "journal",
        "Journal",
        "P0",
        "T347 / T348",
        "No panics, no GPU errors, no repeated service restarts, no leaked processes.",
        ("journal",),
        "journalctl --user -b -p err --no-pager; make native-capture",
        "docs/captures/t347-native/journal/",
    ),
    RowDef(
        "media-keys",
        "Reference keyboard and media keys",
        "P1",
        "T347 / T321",
        "Volume/mute/brightness keys step 5%, show the OSD, and leak no key to the focused client.",
        ("media-keys",),
        "wev -f wl_keyboard:key -f wl_keyboard:modifiers  # press XF86AudioRaiseVolume, XF86AudioLowerVolume, XF86AudioMute, XF86MonBrightnessUp, XF86MonBrightnessDown",
        "docs/captures/t347-native/media-keys/",
    ),
    RowDef(
        "latency-scanout",
        "Latency and scanout",
        "P2",
        "T347 / T238 / G-03",
        "Input-to-present is within one frame; scanout counters are observed on the reference GPU.",
        ("latency",),
        "make latency-trace; make t238-native-acceptance; make t21-scanout-counters",
        "docs/captures/t347-native/latency-scanout/",
    ),
)

ROW_BY_ID = {row.id: row for row in ROW_DEFS}


# ---------------------------------------------------------------------------
# Small process/profile helpers
# ---------------------------------------------------------------------------


def _read_text(path: str | Path, default: str = "") -> str:
    try:
        return Path(path).read_text(errors="replace").strip()
    except OSError:
        return default


def _run(
    args: Iterable[str],
    *,
    timeout: int = 10,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str] | None:
    try:
        return subprocess.run(
            list(args),
            capture_output=True,
            text=True,
            errors="replace",
            timeout=timeout,
            env=env,
            cwd=ROOT,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None


def _shell_join(args: Iterable[str]) -> str:
    return " ".join(args)


def _rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def _os_release() -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in _read_text("/etc/os-release").splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        fields[key] = value.strip().strip('"')
    return fields


def _cpu_model() -> str:
    for line in _read_text("/proc/cpuinfo").splitlines():
        if line.lower().startswith("model name"):
            return line.split(":", 1)[1].strip()
    return platform.processor() or "unknown"


def _memory_gib() -> str:
    for line in _read_text("/proc/meminfo").splitlines():
        if line.startswith("MemTotal:"):
            kib = int(re.search(r"\d+", line).group(0))
            return f"{kib / 1024 / 1024:.1f} GiB"
    return "unknown"


def _gpu_lines() -> list[str]:
    lines: list[str] = []
    proc = _run(["lspci", "-nn"], timeout=5)
    if proc is not None and proc.returncode == 0:
        for line in proc.stdout.splitlines():
            if re.search(r"(VGA compatible|3D controller|Display controller)", line):
                lines.append(line.strip())
    if lines:
        return lines
    seen: set[str] = set()
    for uevent in sorted(Path("/sys/class/drm").glob("card*/device/uevent")):
        fields = {}
        for line in _read_text(uevent).splitlines():
            if "=" in line:
                key, value = line.split("=", 1)
                fields[key] = value
        key = fields.get("PCI_ID", fields.get("DRIVER", str(uevent)))
        if key in seen:
            continue
        seen.add(key)
        driver = fields.get("DRIVER", "unknown driver")
        pci = fields.get("PCI_ID", "unknown id")
        lines.append(f"{driver} {pci} ({uevent.parent.parent.name})")
    return lines or ["no DRM GPU node"]


def _drm_outputs() -> list[str]:
    outputs: list[str] = []
    for connector in sorted(Path("/sys/class/drm").glob("card*-*")):
        status = _read_text(connector / "status", "unknown")
        if status not in {"connected", "disconnected", "unknown"}:
            continue
        modes = _read_text(connector / "modes").splitlines()
        mode = modes[0] if modes else "no mode"
        outputs.append(f"{connector.name} {status} {mode}")
    return outputs or ["no DRM connectors"]


def _input_devices() -> list[str]:
    names: list[str] = []
    block: list[str] = []
    for line in _read_text("/proc/bus/input/devices").splitlines() + [""]:
        if not line.strip():
            name = next(
                (part.split("=", 1)[1].strip('"') for part in block if part.startswith("N: Name=")),
                "",
            )
            if name and not re.search(
                r"(Video Bus|Power Button|Sleep Button|PC Speaker|HD-Audio|HDA|HDMI/DP)",
                name,
            ):
                names.append(name)
            block = []
        else:
            block.append(line)
    # Deduplicate while preserving order (a keyboard and its consumer-control
    # node often share a name).
    return list(dict.fromkeys(names)) or ["no input devices reported"]


def _battery_lines() -> list[str]:
    lines: list[str] = []
    for supply in sorted(Path("/sys/class/power_supply").glob("BAT*")):
        capacity = _read_text(supply / "capacity", "?")
        status = _read_text(supply / "status", "?")
        lines.append(f"{supply.name} {capacity}% {status}")
    return lines or ["no battery (desktop or VM)"]


def _session_summary() -> str:
    parts = []
    for key in ("XDG_SESSION_TYPE", "XDG_CURRENT_DESKTOP", "WAYLAND_DISPLAY", "DISPLAY"):
        value = os.environ.get(key)
        if value:
            parts.append(f"{key}={value}")
    return "; ".join(parts) or "no session environment"


def _logind_sessions() -> str:
    proc = _run(["loginctl", "list-sessions", "--no-legend"], timeout=5)
    if proc is None or proc.returncode != 0:
        return "loginctl unavailable"
    rows = [line for line in proc.stdout.splitlines() if line.strip()]
    return f"{len(rows)} session(s)" if rows else "no sessions"


def _seat_state(probe_path: Path) -> SeatState:
    proc = _run([sys.executable, "scripts/drm-seat-probe.py"], timeout=20)
    if proc is None:
        return SeatState(False, "DRM seat probe did not run", {})
    try:
        probe_path.write_text(proc.stdout or proc.stderr)
    except OSError:
        pass
    try:
        report = json.loads(proc.stdout) if proc.stdout.strip() else {}
    except json.JSONDecodeError:
        report = {"raw": (proc.stdout or proc.stderr).strip()}
    rc = proc.returncode
    if rc == 0 and report.get("free_card"):
        return SeatState(True, f"free DRM master on {report['free_card']}", report)
    if rc == 2:
        return SeatState(False, "no DRM card node (/dev/dri/card*)", report)
    states = []
    for card in report.get("cards", []):
        if card.get("can_set_master"):
            continue
        if card.get("opened"):
            states.append(f"{card.get('path')} busy ({card.get('error') or 'master held'})")
        else:
            states.append(f"{card.get('path')} unopenable ({card.get('error')})")
    detail = ", ".join(states) or (proc.stderr or proc.stdout or "probe failed").strip()
    return SeatState(False, f"no free DRM seat ({detail})", report)


def _hardware_profile(seat: SeatState, intake: Intake) -> dict[str, str]:
    osr = _os_release()
    return {
        "host": f"{platform.system()} {platform.release()} {platform.machine()}",
        "distro": osr.get("PRETTY_NAME", osr.get("NAME", "unknown")),
        "cpu": _cpu_model(),
        "memory": _memory_gib(),
        "gpu": "; ".join(_gpu_lines()),
        "drm cards": ", ".join(
            sorted(str(path) for path in Path("/dev/dri").glob("card*"))
        )
        or "none",
        "outputs": "; ".join(_drm_outputs()),
        "input": "; ".join(_input_devices()),
        "battery": "; ".join(_battery_lines()),
        "session": _session_summary(),
        "logind": _logind_sessions(),
        "seat": seat.reason,
        "evidence profile": (
            f"{intake.profile or 'intake'} ({INTAKE_PATH_LABEL})"
            if intake.present
            else f"host only (no native intake; {INTAKE_PATH_LABEL} absent)"
        ),
    }


# ---------------------------------------------------------------------------
# Headless checks
# ---------------------------------------------------------------------------


def _toolchain_env() -> dict[str, str]:
    env = os.environ.copy()
    toolchain = Path.home() / ".local" / "df-toolchain" / "usr" / "lib64"
    if toolchain.is_dir():
        existing = env.get("LD_LIBRARY_PATH")
        env["LD_LIBRARY_PATH"] = (
            f"{toolchain}:{existing}" if existing else str(toolchain)
        )
    env.setdefault("CARGO_TERM_COLOR", "never")
    return env


def _leaked_runtime_files(runtime_dir: Path) -> list[str]:
    leaks: list[str] = []
    try:
        entries = list(runtime_dir.iterdir())
    except OSError:
        return leaks
    for entry in entries:
        if entry.is_dir():
            continue
        try:
            is_socket = entry.is_socket()
        except OSError:
            is_socket = False
        if is_socket or entry.name.endswith(COMPOSITOR_ARTIFACT_SUFFIXES):
            leaks.append(entry.name)
    return leaks


def _cargo_summary(output: str, returncode: int) -> str:
    results = [
        line.strip() for line in output.splitlines() if "test result:" in line
    ]
    if results:
        return " | ".join(results)
    for line in reversed(output.splitlines()):
        if line.strip():
            return line.strip()
    return f"exit {returncode}"


def _run_cargo(
    commands: list[list[str]],
    *,
    timeout: int = 1200,
    strict_runtime: bool = False,
) -> tuple[str, str, str]:
    """Run one or more cargo test commands in an isolated runtime dir."""
    env = _toolchain_env()
    runtime_dir = Path(tempfile.mkdtemp(prefix="dragonfruit-t347-cargo-"))
    env["XDG_RUNTIME_DIR"] = str(runtime_dir)
    output_parts: list[str] = []
    failed = False
    summaries: list[str] = []
    leaks: list[str] = []
    try:
        for args in commands:
            command = _shell_join(args)
            output_parts.append(f"$ XDG_RUNTIME_DIR=<scratch> {command}")
            proc = _run(args, timeout=timeout, env=env)
            if proc is None:
                output_parts.append("(did not run: missing executable or timeout)")
                failed = True
                summaries.append(f"{command}: did not run")
                continue
            output = (proc.stdout or "") + (proc.stderr or "")
            output_parts.append(output)
            summaries.append(f"{command}: {_cargo_summary(output, proc.returncode)}")
            if proc.returncode != 0:
                failed = True
        leaks = _leaked_runtime_files(runtime_dir)
    finally:
        if os.environ.get("KEEP_SCRATCH") != "1":
            shutil.rmtree(runtime_dir, ignore_errors=True)
    if leaks:
        if strict_runtime:
            failed = True
            summaries.append(f"leaked runtime artifacts: {', '.join(leaks)}")
        else:
            summaries.append(
                "runtime leftovers from the test harness (not a product leak): "
                + ", ".join(leaks)
            )
    verdict = FAIL if failed else PASS
    detail = " | ".join(summaries)
    command_text = " && ".join(_shell_join(args) for args in commands)
    return verdict, detail, command_text + "\n\n" + "\n".join(output_parts)


def _run_startup(cycles: int) -> tuple[str, str, str]:
    env = _toolchain_env()
    runtime_dir = Path(tempfile.mkdtemp(prefix="dragonfruit-t347-start-"))
    env["XDG_RUNTIME_DIR"] = str(runtime_dir)
    dev_bin = Path(os.environ.get("T347_DEV_BIN", "target/debug/dragonfruit"))
    if not dev_bin.is_absolute():
        dev_bin = ROOT / dev_bin
    command = [
        str(dev_bin),
        "dev",
        "--soak",
        str(cycles),
    ]
    command_text = (
        f"XDG_RUNTIME_DIR=<scratch> "
        f"{_rel(dev_bin)} "
        f"dev --soak {cycles}"
    )
    output = f"$ {command_text}\n"
    started = dt.datetime.now(dt.timezone.utc)
    proc = _run(command, timeout=600, env=env)
    elapsed = (dt.datetime.now(dt.timezone.utc) - started).total_seconds()
    if proc is None:
        output += "(did not run: missing binary or timeout)\n"
        verdict = FAIL
        detail = "did not run"
    else:
        output += (proc.stdout or "") + (proc.stderr or "")
        leaks = sorted(entry.name for entry in runtime_dir.iterdir()) if runtime_dir.exists() else []
        ok = proc.returncode == 0 and "soak passed" in output and not leaks
        verdict = PASS if ok else FAIL
        detail = f"{cycles} headless cycles in {elapsed:.2f}s"
        if leaks:
            detail += f"; leaked {', '.join(leaks)}"
        if not ok and proc.returncode == 0:
            detail += "; soak did not report a clean pass"
    if os.environ.get("KEEP_SCRATCH") != "1":
        shutil.rmtree(runtime_dir, ignore_errors=True)
    return verdict, detail, command_text + "\n\n" + output


def _run_media_keys() -> tuple[str, str, str]:
    build_dir = Path(os.environ.get("BUILD_DIR", str(ROOT / "build")))
    if not build_dir.is_absolute():
        build_dir = ROOT / build_dir
    test_bin = build_dir / "shell" / "tests" / "tst_mediakeys"
    if not test_bin.is_file():
        return (
            OPEN,
            f"shell test binary {_rel(test_bin)} is not built",
            "make build  # then rerun the T347 matrix",
        )
    proc = _run([str(test_bin)], timeout=300, env=_toolchain_env())
    output = (proc.stdout or "") + (proc.stderr or "") if proc is not None else ""
    verdict = PASS if proc is not None and proc.returncode == 0 else FAIL
    detail = _cargo_summary(output, proc.returncode if proc is not None else -1)
    return verdict, detail, f"{_rel(test_bin)}\n\n" + output


def _run_journal(log_dir: Path) -> tuple[str, str, str]:
    findings: list[str] = []
    pattern = re.compile(r"panicked at|SIGSEGV|GPU hang|device lost", re.IGNORECASE)
    for log in sorted(log_dir.glob("*.log")):
        if log.name == "journal.log":
            continue
        for number, line in enumerate(_read_text(log).splitlines(), start=1):
            if pattern.search(line):
                findings.append(f"{log.name}:{number}: {line.strip()}")
    if findings:
        return FAIL, f"{len(findings)} panic/GPU line(s) in the headless logs", "\n".join(findings)
    return PASS, f"{len(list(log_dir.glob('*.log')))} headless log(s) clean", ""


def _check_defs(cycles: int) -> list[tuple[str, str, str, Callable[[], tuple[str, str, str]]]]:
    return [
        (
            "startup",
            "Session startup and teardown (headless proxy)",
            f"XDG_RUNTIME_DIR=<scratch> target/debug/dragonfruit dev --soak {cycles}",
            lambda: _run_startup(cycles),
        ),
        (
            "lock",
            "Lock/unlock conformance",
            "cargo test -p dragonfruit-compositor --test session_lock_conformance",
            lambda: _run_cargo(
                [["cargo", "test", "-p", "dragonfruit-compositor", "--test", "session_lock_conformance"]],
                strict_runtime=True,
            ),
        ),
        (
            "suspend",
            "Suspend/resume conformance",
            "cargo test -p dragonfruit-compositor --test suspend_resume_conformance",
            lambda: _run_cargo(
                [["cargo", "test", "-p", "dragonfruit-compositor", "--test", "suspend_resume_conformance"]],
                strict_runtime=True,
            ),
        ),
        (
            "idle-policy",
            "Idle policy chain",
            "cargo test -p dragonfruit-compositor --test idle_policy_conformance",
            lambda: _run_cargo(
                [["cargo", "test", "-p", "dragonfruit-compositor", "--test", "idle_policy_conformance"]],
            ),
        ),
        (
            "idle-trace",
            "Idle trace (zero frames, zero shell wakeups)",
            "cargo test -p dragonfruit-compositor --test idle_trace --test shell_idle_trace",
            lambda: _run_cargo(
                [
                    ["cargo", "test", "-p", "dragonfruit-compositor", "--test", "idle_trace"],
                    ["cargo", "test", "-p", "dragonfruit-compositor", "--test", "shell_idle_trace"],
                ],
            ),
        ),
        (
            "shell-kill",
            "Shell kill and restart policy",
            "cargo test -p dragonfruit-session --test kill_matrix --test restart_policy_matrix",
            lambda: _run_cargo(
                [
                    ["cargo", "test", "-p", "dragonfruit-session", "--test", "kill_matrix"],
                    ["cargo", "test", "-p", "dragonfruit-session", "--test", "restart_policy_matrix"],
                ],
            ),
        ),
        (
            "hotplug",
            "Output and input hotplug",
            "cargo test -p dragonfruit-compositor --test shell_protocol_conformance hotplug && cargo test -p dragonfruit-compositor --test input_settings hotplug",
            lambda: _run_cargo(
                [
                    ["cargo", "test", "-p", "dragonfruit-compositor", "--test", "shell_protocol_conformance", "hotplug"],
                    ["cargo", "test", "-p", "dragonfruit-compositor", "--test", "input_settings", "hotplug"],
                ],
            ),
        ),
        (
            "launch",
            "X11/Wayland/Flatpak launch environment",
            "cargo test -p dragonfruit-compositor --test xwayland_conformance && cargo test -p dragonfruit-session --test environment",
            lambda: _run_cargo(
                [
                    ["cargo", "test", "-p", "dragonfruit-compositor", "--test", "xwayland_conformance"],
                    ["cargo", "test", "-p", "dragonfruit-session", "--test", "environment"],
                ],
            ),
        ),
        (
            "portals",
            "FileChooser, Screenshot, and ScreenCast portals",
            "cargo test -p dragonfruit-portal --test filechooser --test screenshot --test screencast -- --test-threads=1",
            lambda: _run_cargo(
                [
                    [
                        "cargo",
                        "test",
                        "-p",
                        "dragonfruit-portal",
                        "--test",
                        "filechooser",
                        "--test",
                        "screenshot",
                        "--test",
                        "screencast",
                        "--",
                        "--test-threads=1",
                    ],
                ],
            ),
        ),
        (
            "data",
            "Files operations and cross-filesystem move",
            "cargo test -p dragonfruit-files-core --test operations",
            lambda: _run_cargo(
                [["cargo", "test", "-p", "dragonfruit-files-core", "--test", "operations"]],
            ),
        ),
        (
            "adapters",
            "Audio, Wi-Fi, and Bluetooth adapters",
            "cargo test -p dragonfruit-audio --tests && cargo test -p dragonfruit-networkmanager --tests && cargo test -p dragonfruit-bluetooth --tests",
            lambda: _run_cargo(
                [
                    ["cargo", "test", "-p", "dragonfruit-audio", "--tests"],
                    ["cargo", "test", "-p", "dragonfruit-networkmanager", "--tests"],
                    ["cargo", "test", "-p", "dragonfruit-bluetooth", "--tests"],
                ],
            ),
        ),
        (
            "media-keys",
            "Media keys (shell policy test)",
            "build/shell/tests/tst_mediakeys",
            _run_media_keys,
        ),
        (
            "latency",
            "Latency instrument and scanout template",
            "cargo test -p dragonfruit-compositor --test latency_trace",
            lambda: _run_cargo(
                [["cargo", "test", "-p", "dragonfruit-compositor", "--test", "latency_trace"]],
            ),
        ),
    ]


# ---------------------------------------------------------------------------
# Intake and verdict resolution
# ---------------------------------------------------------------------------


def load_intake(path: Path) -> Intake:
    intake = Intake(present=path.is_file(), path=path)
    if not intake.present:
        return intake
    try:
        data = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"{path}: {exc}") from exc
    if not isinstance(data, dict):
        raise ValueError(f"{path}: top level must be an object")
    intake.profile = data.get("profile")
    intake.recorded = data.get("recorded")
    rows = data.get("rows", {})
    if not isinstance(rows, dict):
        raise ValueError(f"{path}: rows must be an object")
    for row_id, entry in rows.items():
        if row_id not in ROW_BY_ID:
            intake.warnings.append(f"unknown row id {row_id!r} ignored")
            continue
        if not isinstance(entry, dict):
            raise ValueError(f"{path}: row {row_id!r} must be an object")
        verdict = entry.get("verdict")
        if verdict not in VERDICTS:
            raise ValueError(
                f"{path}: row {row_id!r} verdict must be one of {sorted(VERDICTS)}"
            )
        intake.rows[row_id] = entry
    return intake


def _check_summary(check_ids: Iterable[str], checks: dict[str, HeadlessResult]) -> str:
    parts = []
    for check_id in check_ids:
        result = checks.get(check_id)
        if result is None:
            parts.append(f"{check_id}: missing")
        else:
            parts.append(f"{check_id}: {result.verdict}")
    return ", ".join(parts)


def resolve_rows(
    seat: SeatState,
    checks: dict[str, HeadlessResult],
    intake: Intake,
) -> list[RowResult]:
    results: list[RowResult] = []
    for row in ROW_DEFS:
        failed = [
            check_id
            for check_id in row.checks
            if check_id in checks and checks[check_id].verdict == FAIL
        ]
        headless = _check_summary(row.checks, checks)
        entry = intake.rows.get(row.id)
        artifact = row.native_artifact
        reason = ""

        if failed:
            verdict = FAIL
            reason = (
                f"Headless proof failed: {', '.join(failed)}. "
                f"See docs/captures/t347/logs/{failed[0]}.log."
            )
            artifact = f"docs/captures/t347/logs/{failed[0]}.log"
        elif entry is not None:
            recorded = entry.get("verdict")
            note = (entry.get("note") or "").strip()
            recorded_artifact = (entry.get("artifact") or "").strip()
            if recorded == FAIL:
                verdict = FAIL
                reason = note or "Native intake recorded FAIL."
                artifact = recorded_artifact or row.native_artifact
            elif recorded == PASS:
                if not note or not recorded_artifact:
                    verdict = OPEN
                    reason = (
                        "Native intake recorded PASS without a note and artifact; "
                        "record both before the gate can accept it."
                    )
                else:
                    verdict = PASS
                    reason = note
                    artifact = recorded_artifact
            else:
                verdict = OPEN
                reason = note or "Native intake recorded OPEN."
                artifact = recorded_artifact or row.native_artifact
        elif seat.available:
            verdict = OPEN
            reason = (
                f"A free DRM seat is available ({seat.reason}), but no native "
                f"evidence is recorded. Run the scenario and record it in "
                f"{INTAKE_PATH_LABEL}."
            )
        else:
            verdict = OPEN
            headless_note = f" Headless half: {headless}." if row.checks else ""
            reason = (
                f"{seat.reason[:1].upper() + seat.reason[1:]}."
                f"{headless_note} Run on the reference laptop and record it in "
                f"{INTAKE_PATH_LABEL}."
            )

        if row.checks:
            if verdict == FAIL:
                pass
            elif all(
                check_id in checks and checks[check_id].verdict == PASS
                for check_id in row.checks
            ):
                if "Headless half" not in reason:
                    if not reason.endswith((".", "!", "?")):
                        reason += "."
                    reason += f" Headless half PASS ({headless})."
            else:
                open_reasons = [
                    f"{check_id}: {checks[check_id].detail}"
                    for check_id in row.checks
                    if check_id in checks and checks[check_id].verdict == OPEN
                ]
                suffix = f" Headless half: {headless}."
                if open_reasons:
                    suffix += " " + "; ".join(open_reasons) + "."
                if "Headless half" not in reason:
                    if not reason.endswith((".", "!", "?")):
                        reason += "."
                    reason += suffix

        results.append(
            RowResult(
                id=row.id,
                title=row.title,
                severity=row.severity,
                owner=row.owner,
                requirement=row.requirement,
                verdict=verdict,
                reason=reason.strip(),
                command=row.rerun,
                artifact=artifact,
                checks=row.checks,
            )
        )
    return results


def gate_verdict(rows: Iterable[RowResult]) -> str:
    rows = list(rows)
    if any(row.verdict == FAIL for row in rows):
        return FAIL
    if any(row.verdict == OPEN for row in rows):
        return OPEN
    return PASS


def counts(rows: Iterable[RowResult]) -> dict[str, int]:
    result = {PASS: 0, FAIL: 0, OPEN: 0}
    for row in rows:
        result[row.verdict] += 1
    return result


# ---------------------------------------------------------------------------
# Report rendering
# ---------------------------------------------------------------------------


def _cell(value: object) -> str:
    text = str(value).replace("\r", " ").replace("\n", "<br>")
    return text.replace("|", "\\|")


def _code(value: str) -> str:
    text = value.replace("\n", " ")
    if "`" in text:
        return text
    return f"`{text}`"


def render_report(
    *,
    generated: str,
    hardware: dict[str, str],
    seat: SeatState,
    checks: list[HeadlessResult],
    rows: list[RowResult],
    intake: Intake,
    report_path: Path,
) -> str:
    verdict = gate_verdict(rows)
    tally = counts(rows)
    lines: list[str] = [
        "# T347 — Reference-hardware daily-driver matrix",
        "",
        f"Generated: `{generated}`  ",
        f"Runner: `make t347-daily-driver-matrix` (`scripts/t347-daily-driver-matrix.py`)  ",
        f"Report: `{_rel(report_path)}`",
        "",
        "> A headless pass is not a hardware pass. A row is `PASS` only with",
        "> native evidence from the reference laptop; a host without a free DRM",
        "> seat gets `OPEN` rows with the exact rerun command, never a skipped",
        "> row. A failed headless check is `FAIL` and blocks the gate.",
        "",
        "## Verdict",
        "",
        "| Gate | PASS | FAIL | OPEN |",
        "|---|---|---|---|",
        f"| **{verdict}** | {tally[PASS]} | {tally[FAIL]} | {tally[OPEN]} |",
        "",
    ]
    if verdict == PASS:
        lines.append("All rows have native PASS evidence.")
    elif verdict == FAIL:
        lines.append(
            "The gate is **FAIL**. Fix the failed row or file a follow-up task; "
            "a P0 failure is never waived."
        )
    else:
        lines.append(
            "The gate is **OPEN**: every row has a verdict, but the native "
            "evidence is not complete. Run the rerun command on the reference "
            "laptop and record the result in the intake file."
        )
    lines.extend(["", "## Hardware profile", "", "| Field | Value |", "|---|---|"])
    for key, value in hardware.items():
        lines.append(f"| {_cell(key)} | {_cell(value)} |")
    lines.extend(
        [
            "",
            f"Native evidence intake: "
            + (
                f"`{INTAKE_PATH_LABEL}` (profile `{intake.profile or 'unstated'}`, "
                f"recorded `{intake.recorded or 'unstated'}`)"
                if intake.present
                else f"absent (`{INTAKE_PATH_LABEL}`); native rows are OPEN"
            ),
            "",
        ]
    )
    if intake.warnings:
        lines.append("Intake warnings: " + "; ".join(intake.warnings))
        lines.append("")

    lines.extend(
        [
            "## Headless supporting checks",
            "",
            "| Check | Verdict | Command | Artifact | Detail |",
            "|---|---|---|---|---|",
        ]
    )
    for check in checks:
        lines.append(
            f"| {_cell(check.title)} | **{check.verdict}** | "
            f"{_cell(_code(check.command))} | `{check.artifact}` | "
            f"{_cell(check.detail)} |"
        )
    lines.extend(
        [
            "",
            "## Daily-driver matrix",
            "",
            "| # | Row | Severity | Verdict | Requirement | Reason / evidence | Rerun command | Artifact | Owner |",
            "|---|---|---|---|---|---|---|---|---|",
        ]
    )
    for index, row in enumerate(rows, start=1):
        lines.append(
            f"| {index} | {_cell(row.title)} | {row.severity} | **{row.verdict}** | "
            f"{_cell(row.requirement)} | {_cell(row.reason)} | "
            f"{_cell(_code(row.command))} | `{row.artifact}` | {_cell(row.owner)} |"
        )

    failures = [row for row in rows if row.verdict == FAIL]
    lines.extend(["", "## Failures and follow-ups", ""])
    if not failures:
        lines.append("None. No row failed.")
    else:
        lines.append(
            "Every failure below needs a follow-up task before T348; a P0 row "
            "is not waived."
        )
        lines.append("")
        for row in failures:
            lines.append(
                f"- **{row.severity} `{row.id}`** — {row.reason} "
                f"Follow-up: file the next free stability task with the "
                f"artifact `{row.artifact}`. Owner: {row.owner}."
            )

    lines.extend(
        [
            "",
            "## Rerun",
            "",
            "```bash",
            "make t347-daily-driver-matrix",
            "# On the reference laptop, record each native row in",
            "# docs/captures/t347-native-evidence.json with verdict, note, and artifact.",
            "```",
            "",
        ]
    )
    return "\n".join(lines)


def build_json(
    *,
    generated: str,
    hardware: dict[str, str],
    seat: SeatState,
    checks: list[HeadlessResult],
    rows: list[RowResult],
    intake: Intake,
    report_path: Path,
) -> dict:
    return {
        "generated": generated,
        "runner": "scripts/t347-daily-driver-matrix.py",
        "report": _rel(report_path),
        "gate": gate_verdict(rows),
        "counts": counts(rows),
        "hardware": hardware,
        "seat": {
            "available": seat.available,
            "reason": seat.reason,
            "probe": seat.probe,
        },
        "intake": {
            "present": intake.present,
            "path": _rel(intake.path),
            "profile": intake.profile,
            "recorded": intake.recorded,
            "warnings": intake.warnings,
        },
        "headless_checks": [
            {
                "id": check.id,
                "title": check.title,
                "verdict": check.verdict,
                "command": check.command,
                "artifact": check.artifact,
                "detail": check.detail,
            }
            for check in checks
        ],
        "rows": [
            {
                "id": row.id,
                "title": row.title,
                "severity": row.severity,
                "owner": row.owner,
                "verdict": row.verdict,
                "requirement": row.requirement,
                "reason": row.reason,
                "command": row.command,
                "artifact": row.artifact,
                "headless_checks": list(row.checks),
            }
            for row in rows
        ],
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def run_matrix(args: argparse.Namespace) -> int:
    out_md = Path(args.out_md).resolve()
    out_json = Path(args.out_json).resolve()
    log_dir = Path(args.log_dir).resolve()
    intake_path = Path(args.intake).resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_json.parent.mkdir(parents=True, exist_ok=True)
    # The journal row scans the log directory; drop last run's logs so a stale
    # failure cannot be attributed to this run.
    for stale in log_dir.glob("*.log"):
        try:
            stale.unlink()
        except OSError:
            pass

    generated = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    if args.dev_bin:
        os.environ["T347_DEV_BIN"] = args.dev_bin

    try:
        intake = load_intake(intake_path)
    except ValueError as exc:
        print(f"t347: invalid intake: {exc}", file=sys.stderr)
        return 1

    seat = _seat_state(log_dir / "seat-probe.json")
    hardware = _hardware_profile(seat, intake)

    checks: list[HeadlessResult] = []
    for check_id, title, command, runner in _check_defs(args.soak_cycles):
        print(f"t347: {check_id} ...", flush=True)
        try:
            verdict, detail, log_text = runner()
        except Exception as exc:  # noqa: BLE001 - one check must not hide the report
            verdict = FAIL
            detail = f"{type(exc).__name__}: {exc}"
            log_text = f"{detail}\n"
        artifact = log_dir / f"{check_id}.log"
        try:
            artifact.write_text(
                f"# T347 headless check: {check_id}\n"
                f"# generated: {generated}\n"
                f"# command: {command}\n"
                f"# verdict: {verdict}\n"
                f"# detail: {detail}\n\n{log_text}\n"
            )
        except OSError as exc:
            print(f"t347: could not write {artifact}: {exc}", file=sys.stderr)
        checks.append(
            HeadlessResult(
                id=check_id,
                title=title,
                command=command,
                verdict=verdict,
                detail=detail,
                artifact=_rel(artifact),
            )
        )

    journal_verdict, journal_detail, journal_text = _run_journal(log_dir)
    journal_artifact = log_dir / "journal.log"
    journal_artifact.write_text(
        f"# T347 headless check: journal\n"
        f"# generated: {generated}\n"
        f"# verdict: {journal_verdict}\n"
        f"# detail: {journal_detail}\n\n{journal_text}\n"
    )
    checks.append(
        HeadlessResult(
            id="journal",
            title="Journal scan of the headless logs",
            command="grep -E 'panicked at|SIGSEGV|GPU hang|device lost' docs/captures/t347/logs/*.log",
            verdict=journal_verdict,
            detail=journal_detail,
            artifact=_rel(journal_artifact),
        )
    )

    checks_by_id = {check.id: check for check in checks}
    rows = resolve_rows(seat, checks_by_id, intake)
    report = render_report(
        generated=generated,
        hardware=hardware,
        seat=seat,
        checks=checks,
        rows=rows,
        intake=intake,
        report_path=out_md,
    )
    out_md.write_text(report)
    out_json.write_text(
        json.dumps(
            build_json(
                generated=generated,
                hardware=hardware,
                seat=seat,
                checks=checks,
                rows=rows,
                intake=intake,
                report_path=out_md,
            ),
            indent=2,
        )
        + "\n"
    )

    tally = counts(rows)
    verdict = gate_verdict(rows)
    print(
        f"t347: wrote {_rel(out_md)} and "
        f"{_rel(out_json)} "
        f"(gate {verdict}: {tally[PASS]} PASS, {tally[FAIL]} FAIL, "
        f"{tally[OPEN]} OPEN)"
    )
    if verdict == FAIL:
        print("t347: FAILED; see the report and logs", file=sys.stderr)
        return 1
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out-md", default=os.environ.get("OUT_MD", str(DEFAULT_OUT_MD)))
    parser.add_argument("--out-json", default=os.environ.get("OUT_JSON", str(DEFAULT_OUT_JSON)))
    parser.add_argument("--log-dir", default=os.environ.get("LOG_DIR", str(DEFAULT_LOG_DIR)))
    parser.add_argument(
        "--intake",
        default=os.environ.get("T347_NATIVE_EVIDENCE", str(DEFAULT_INTAKE)),
    )
    parser.add_argument(
        "--soak-cycles",
        type=int,
        default=int(os.environ.get("T347_SOAK_CYCLES", DEFAULT_SOAK_CYCLES)),
    )
    parser.add_argument(
        "--dev-bin",
        default=os.environ.get("T347_DEV_BIN"),
        help="dragonfruit dev binary (default: target/debug/dragonfruit)",
    )
    args = parser.parse_args(argv)
    if args.soak_cycles < 1:
        parser.error("--soak-cycles must be at least 1")
    return run_matrix(args)


if __name__ == "__main__":
    sys.exit(main())
