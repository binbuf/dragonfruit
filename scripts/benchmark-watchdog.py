#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Watch a benchmark compositor and capture diagnostics if it stops responding.

The nested compositor can stall when the host stops presenting its window: the
main loop never returns from the backend callback, so calloop never services
the synthetic-input socket or the SIGUSR1 signal. A plain timeout then produces
a silent hang. This watchdog detects the starvation and writes the evidence
that distinguishes it from a deadlock or a GPU wait:

* the synthetic socket's receive queue (queued datagrams = the loop is not
  reading input),
* every thread's state, wchan, syscall, and kernel stack,
* userspace backtraces (`eu-stack` and `gdb thread apply all bt`),
* the process fd table and the host Wayland/socket connections,
* the tail of the compositor log.

On trigger it SIGKILLs the compositor so the caller fails fast instead of
hanging, and exits 1.

Usage:
  benchmark-watchdog.py --pid PID --synth PATH --log LOG --out HANG.txt
"""

import argparse
import os
import shutil
import signal
import subprocess
import sys
import time


def run(cmd, timeout=10):
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            errors="replace",
        )
        return result.stdout + result.stderr
    except Exception as exc:  # noqa: BLE001 - diagnostics must never throw
        return f"<{cmd[0]} failed: {exc}>\n"


def thread_dump(pid):
    lines = []
    task_dir = f"/proc/{pid}/task"
    try:
        tids = sorted(os.listdir(task_dir), key=int)
    except OSError as exc:
        return f"<cannot list {task_dir}: {exc}>\n"
    for tid in tids:
        base = f"{task_dir}/{tid}"
        comm = read(os.path.join(base, "comm")).strip()
        stat = read(os.path.join(base, "stat"))
        state = ""
        fields = stat.rsplit(")", 1)
        if len(fields) == 2:
            parts = fields[1].split()
            if parts:
                state = parts[0]
        wchan = read(os.path.join(base, "wchan")).strip()
        syscall = read(os.path.join(base, "syscall")).strip()
        stack = read(os.path.join(base, "stack")).strip().replace("\n", " | ")
        lines.append(
            f"tid={tid} comm={comm} state={state} wchan={wchan}\n"
            f"  syscall: {syscall}\n"
            f"  stack:   {stack}\n"
        )
    return "".join(lines)


def read(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError:
        return ""


def synth_queue(synth):
    """Return the synthetic socket's Recv-Q, or None if it cannot be read."""
    if not synth:
        return None
    out = run(["ss", "-x", "-a"], timeout=5)
    for line in out.splitlines():
        if synth in line:
            parts = line.split()
            # u_dgr UNCONN <Recv-Q> <Send-Q> <Local> <Peer>
            for index, token in enumerate(parts):
                if token == "UNCONN" or token == "ESTAB":
                    if index + 1 < len(parts):
                        try:
                            return int(parts[index + 1])
                        except ValueError:
                            return None
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--synth", default="")
    parser.add_argument("--log", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument(
        "--timeout",
        type=float,
        default=15.0,
        help="seconds with a non-empty synthetic queue before declaring a hang",
    )
    parser.add_argument("--interval", type=float, default=1.0)
    args = parser.parse_args()

    stuck_since = None
    while True:
        if not os.path.exists(f"/proc/{args.pid}"):
            return 0
        queue = synth_queue(args.synth)
        # A few queued datagrams are normal between dispatch ticks. A queue
        # that never drains while the process is alive is starvation.
        if queue is not None and queue > 0:
            if stuck_since is None:
                stuck_since = time.monotonic()
            elif time.monotonic() - stuck_since >= args.timeout:
                write_hang(args, queue)
                try:
                    os.kill(args.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                return 1
        else:
            stuck_since = None
        time.sleep(args.interval)


def write_hang(args, queue):
    log_tail = read(args.log).splitlines()[-80:]
    sections = [
        "=== benchmark watchdog: compositor stopped servicing input ===",
        f"time: {time.strftime('%Y-%m-%d %H:%M:%S')}",
        f"pid: {args.pid}",
        f"synthetic socket: {args.synth}",
        f"synthetic Recv-Q: {queue} (datagrams queued, never read)",
        "",
        "=== /proc/<pid>/status ===",
        "\n".join(
            line
            for line in read(f"/proc/{args.pid}/status").splitlines()
            if line.startswith(("Name:", "State:", "Threads:", "Sig", "VmRSS:"))
        ),
        "",
        "=== threads ===",
        thread_dump(args.pid),
        "=== fd table ===",
        run(["ls", "-l", f"/proc/{args.pid}/fd"], timeout=5),
        "=== host unix sockets ===",
        run(["ss", "-x", "-p"], timeout=5),
        "=== /proc/net/unix (filtered) ===",
        "\n".join(
            line
            for line in read("/proc/net/unix").splitlines()
            if "dragonfruit" in line or "wayland" in line
        ),
        "=== eu-stack ===",
        run(["eu-stack", "-p", str(args.pid)], timeout=15),
        "=== gdb thread backtraces ===",
        run(
            [
                "gdb",
                "-q",
                "-p",
                str(args.pid),
                "-batch",
                "-ex",
                "set pagination off",
                "-ex",
                "thread apply all bt",
            ],
            timeout=30,
        ),
        "=== compositor log tail ===",
        "\n".join(log_tail),
    ]
    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(sections))
        handle.write("\n")


if __name__ == "__main__":
    sys.exit(main())
