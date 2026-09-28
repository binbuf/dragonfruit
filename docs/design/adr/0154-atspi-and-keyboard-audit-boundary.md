# 0154 — The AT-SPI audit boundary: apps live, shell chrome per component

## Status

accepted

## Context

T-16.6a is the accessibility audit the T-11.4b OSD work deferred: "the live
AT-SPI tree walkthrough (cross-process dump, every flow keyboard-only)"
([0063](0063-osd-keyboard-atspi.md)). The design reference
([10-design-system.md](../10-design-system.md)) already fixes the shape —
"keyboard navigation and AT-SPI roles are verified **per component**, not
re-proven per application" — but the track asks for a *live* dump, which is a
different claim: a real assistive client (an AT-SPI consumer on the session)
must see the running desktop.

What is actually true in this architecture:

- The shell is a **Wayland client of our own compositor** through the private
  protocol, and it renders QML **offscreen**
  (`QT_QPA_PLATFORM=offscreen`, `shell/src/main.cpp`) into shared-memory chrome
  surfaces it paints itself. Qt's Linux AT-SPI bridge registers an application
  on the accessibility bus from its shown `QWindow`s through the platform
  plugin; the **offscreen platform exposes no accessibility backend**, so the
  shell never appears on the bus (verified live: the shell's windows are absent
  from `pyatspi`'s desktop while `dragonfruit-settings` is present).
- First-party apps (Settings, Files) are ordinary Wayland toplevels. They do
  register, and their whole tree — roles, names, focusable/selected/checked
  states — is readable live.
- Every **global** flow (Mission Control, Desktop Reveal, app switcher,
  workspace switch, Control Center, notification center, Dock focus/auto-hide,
  screenshot, lock, and the Ctrl+1..9 Space jumps) is owned by the
  compositor's one shortcut engine and dispatched from the keyboard. It can be
  proven keyboard-only without a screen reader.

## Decision

- **The live cross-process AT-SPI audit covers the first-party apps.**
  `scripts/t16-a11y-audit.sh` (`make t16-a11y-audit`) launches the nested demo
  with `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`, dumps the Settings and Files
  accessibility trees, checks that every interactive node has an accessible
  name, and drives a keyboard-only walkthrough (Tab focus traversal; select a
  Settings pane from the sidebar with the arrow keys + Return) over the
  compositor's synthetic-input harness, reading each move back from AT-SPI.
  The transcript is committed as `docs/captures/t16-a11y-atspi.txt`.
- **Every compositor-owned system binding is proven keyboard-only headlessly.**
  `compositor/tests/shell_protocol_conformance.rs::keyboard_only_walkthrough_dispatches_every_system_binding`
  sends each default system chord through the synthetic (libinput-equivalent)
  path and asserts the shared outbox records the matching action with a
  `keyboard` trigger. This is the CI gate; it needs no session bus, display
  server, or assistive client.
- **Shell chrome accessible semantics stay per component.** The menu bar, Dock,
  Control Center, OSD, dialogs, and overlays keep their `Accessible.*` roles
  and names asserted in `make qml-test` (the design-system contract). The shell
  is **not** exported to the accessibility bus, and this is a documented
  boundary of the offscreen render architecture, not a missing test.
- The live script is a host-session capture (like the other `*-capture`
  targets), not part of `make e2e`.

## Consequences

- A live screen reader can announce and navigate the first-party applications.
  It does **not** see the shell's menu bar, Dock, or panels; those are reachable
  only through the shell's own keyboard routing. Exposing the shell chrome on
  the accessibility bus would require a compositor-side a11y bridge (a new
  track), not a fix to a component.
- T-11.4b's OSD alert contract ([0063](0063-osd-keyboard-atspi.md)) is verified
  per component, and the OSD's Escape dismissal is part of the composer/shell
  keyboard path; the live dump does not cover it for the same offscreen reason.
- T-16.6b (magnifier and reduced-motion sweep) builds on this boundary: it
  sweeps compositor/shell animations, which are not AT-SPI surfaces either.
- `make t16-a11y-audit` is host-session-only; the headless keyboard test is the
  always-on regression gate.