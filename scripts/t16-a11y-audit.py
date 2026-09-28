#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
#
# T-16.6a — live AT-SPI dump and keyboard-only walkthrough (one first-party
# app). Driven by `scripts/t16-a11y-audit.sh` against a nested `make demo`
# session with `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`:
#
#   * dumps the whole accessibility tree of the target app, and
#   * drives a keyboard-only walkthrough over the compositor's synthetic-input
#     harness (Tab through the window; select a Settings pane from the sidebar
#     with the arrow keys + Return), reading each focus move back from AT-SPI.
#
# It is the live, cross-process half of the audit. The CI half — every
# compositor-owned system binding reachable from the keyboard — is
# `compositor/tests/shell_protocol_conformance.rs::
# keyboard_only_walkthrough_dispatches_every_system_binding`.
#
# Usage: t16-a11y-audit.py <synthetic-input-path> <atspi-app-name> [<out.txt>]
# Exit code is non-zero when the tree is not walkable or an interactive node is
# unnamed.

import os
import socket
import sys
import time

import pyatspi

TAB = 15       # evdev KEY_TAB
DOWN = 108     # evdev KEY_DOWN
RETURN = 28    # evdev KEY_RETURN
SHIFT = 42     # evdev KEY_LEFTSHIFT

# Roles whose accessible name is required: an assistive client announces them
# and a keyboard user can act on them.
INTERACTIVE_HINTS = (
    "button", "check box", "radio", "slider", "combo", "entry",
    "list item", "menu item", "link", "tab", "switch",
)


def is_interactive(role_name):
    role = role_name.lower()
    return any(hint in role for hint in INTERACTIVE_HINTS)


class Synthetic:
    def __init__(self, path):
        self.path = path
        self.reply = path + ".reply"
        try:
            os.unlink(self.reply)
        except FileNotFoundError:
            pass
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.sock.bind(self.reply)
        self.sock.settimeout(3)

    def send(self, command):
        self.sock.sendto(command.encode(), self.path)

    def chord(self, keys):
        script = "".join(f"key {k} down\n" for k in keys)
        script += "".join(f"key {k} up\n" for k in reversed(keys))
        self.send(script.strip())

    def close(self):
        try:
            os.unlink(self.reply)
        except FileNotFoundError:
            pass


def find_app(name):
    desktop = pyatspi.Registry.getDesktop(0)
    for i in range(desktop.childCount):
        app = desktop.getChildAtIndex(i)
        if (app.name or "").lower() == name.lower():
            return app
    return None


def flatten(node, depth=0, out=None):
    if out is None:
        out = []
    out.append((depth, node))
    for i in range(node.childCount):
        flatten(node.getChildAtIndex(i), depth + 1, out)
    return out


def focused(node):
    if node.getState().contains(pyatspi.STATE_FOCUSED):
        return node
    for i in range(node.childCount):
        found = focused(node.getChildAtIndex(i))
        if found is not None:
            return found
    return None


def describe(node):
    if node is None:
        return "<none>"
    return f"{node.getRoleName()} name={node.name!r}"


def toolbar_name(app):
    for _, node in flatten(app):
        if node.getRoleName().lower() == "tool bar":
            return node.name
    return None


def main():
    if len(sys.argv) < 3:
        print("usage: t16-a11y-audit.py <synth-path> <app-name> [out]", file=sys.stderr)
        return 2
    synth_path, app_name = sys.argv[1], sys.argv[2]
    out = open(sys.argv[3], "w") if len(sys.argv) > 3 else sys.stdout
    timeout = float(os.environ.get("T16_A11Y_TIMEOUT", "45"))
    synthetic = Synthetic(synth_path)

    def emit(*args):
        print(*args, file=out)

    try:
        emit(f"=== AT-SPI audit: {app_name} ===")
        deadline = time.time() + timeout
        app = None
        while time.time() < deadline:
            app = find_app(app_name)
            if app is not None:
                break
            time.sleep(0.4)
        if app is None:
            emit(f"FAIL: {app_name} never registered on the AT-SPI accessibility bus")
            return 1
        emit(f"registered: {app_name} ({app.getRoleName()})")

        nodes = flatten(app)
        emit("")
        emit(f"--- accessibility tree ({len(nodes)} nodes) ---")
        for depth, node in nodes:
            states = []
            state = node.getState()
            for flag, label in (
                (pyatspi.STATE_FOCUSABLE, "focusable"),
                (pyatspi.STATE_FOCUSED, "focused"),
                (pyatspi.STATE_ENABLED, "enabled"),
                (pyatspi.STATE_CHECKED, "checked"),
                (pyatspi.STATE_SELECTED, "selected"),
                (pyatspi.STATE_PRESSED, "pressed"),
            ):
                if state.contains(flag):
                    states.append(label)
            suffix = f" [{','.join(states)}]" if states else ""
            emit(f"{'  ' * depth}- {node.getRoleName()} name={node.name!r}{suffix}")

        interactive = [
            n for _, n in nodes if is_interactive(n.getRoleName())
        ]
        unnamed = [n for n in interactive if not (n.name or "").strip()]
        emit("")
        emit(f"interactive nodes: {len(interactive)}")
        emit(f"unnamed interactive nodes: {len(unnamed)}")
        for node in unnamed:
            emit(f"  UNNAMED {node.getRoleName()}")
        tree_ok = len(nodes) > 20 and len(interactive) > 5 and not unnamed

        emit("")
        emit("--- keyboard-only walkthrough (synthetic input, focus read from AT-SPI) ---")

        # Focus traversal: a run of Tabs from a fresh window must move the
        # AT-SPI focus through distinct interactive nodes.
        visited = []
        for _ in range(6):
            synthetic.chord((TAB,))
            time.sleep(0.4)
            node = focused(app)
            visited.append(describe(node))
        emit("Tab focus sequence:")
        for step in visited:
            emit(f"  {step}")
        distinct_focus = len({step for step in visited if step != "<none>"}) >= 3

        # Select Settings panes purely from the keyboard. After the six Tabs
        # focus sits on the content scroll view; one Shift+Tab returns it to
        # the sidebar, then Down moves the selection and Return activates it.
        # The pane change is read back from AT-SPI, not assumed.
        pane_ok = True
        panes = []
        if app_name == "dragonfruit-settings":
            synthetic.chord((SHIFT, TAB))
            time.sleep(0.4)
            panes.append(toolbar_name(app))
            for _ in range(4):
                synthetic.chord((DOWN,))
                time.sleep(0.25)
                synthetic.chord((RETURN,))
                time.sleep(0.6)
                panes.append(toolbar_name(app))
            emit("Sidebar pane sequence (Shift+Tab -> sidebar, Down, Return):")
            for pane in panes:
                emit(f"  {pane}")
            pane_ok = len({p for p in panes if p}) >= 2

        emit("")
        emit("--- verdict ---")
        emit(f"tree walkable: {'PASS' if tree_ok else 'FAIL'}")
        emit(f"keyboard focus traversal: {'PASS' if distinct_focus else 'FAIL'}")
        if app_name == "dragonfruit-settings":
            emit(f"keyboard pane selection: {'PASS' if pane_ok else 'FAIL'}")
        passed = tree_ok and distinct_focus and pane_ok
        emit(f"RESULT: {'PASS' if passed else 'FAIL'}")
        return 0 if passed else 1
    finally:
        synthetic.close()
        if out is not sys.stdout:
            out.close()


if __name__ == "__main__":
    sys.exit(main())