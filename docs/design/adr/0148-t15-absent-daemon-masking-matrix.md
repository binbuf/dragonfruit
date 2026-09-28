# 0148 — The T-15 absent-daemon masking matrix and its two absence families

## Status

accepted

## Context

T-15 landed sixteen Wave-2/3 Settings panes and their Control Center tiles
(eighteen shipped panes total with Wave 1). Each pane ticket documented its
own provider's absent behavior, and each adapter enforces the three-state
projection (`available` / `present:false` / `Error`). No single place stated
the contract for the **whole** shipped surface, so T-16 (soak/packaging) and
T-17 (premium gate) had to reconstruct it pane by pane. The Wave-2/3 panes also
mix two kinds of backing — durable settingsd keys and live adapter reads — in
one pane, which the Wave-1 matrix ([T-09.6b](../08-settings.md)) did not cover.

## Decision

- **Every shipped pane satisfies one documented matrix.** The normative table
  is [The absent-daemon masking matrix (T-15.16)](../08-settings.md#the-absent-daemon-masking-matrix-t-1516)
  in `docs/design/08-settings.md`; the reviewed state matrix with the live/VM
  half is `docs/captures/t15-absence-matrix.md`.
- **Two absence families, mixed freely within a pane.**
  - *settingsd-backed controls* stay live on the schema defaults with no
    daemon; a write lands in memory and is mirrored on return ([0030](0030-settingsd-schema-and-dbus-surface.md)).
    There is no "settings unavailable" banner — that would be a half-pane.
  - *adapter-read halves* project through the `system-status` bridge host
    ([0029](0029-system-status-bridge-host.md)). Absence replaces the half with
    a one-line note and refuses the write; the matching Control Center tile
    hides by the same rule.
- **Absence is not error.** A present-but-unreadable daemon is `Error`
  (visible, inert); a polkit refusal is a read-only degradation (list visible,
  writes disabled); an all-off bus is `available` + `present:false` (second
  hide rule). None of these is absence.
- **The headless gate is authoritative in CI.** `tst_settings_absence.qml`
  runs on a private bus with no settingsd, portal, or bridge host, asserts all
  22 shipped bodies, and sweeps every shipped pane for "mounts and stays
  interactive". `scripts/t15-absence-matrix.sh` (`make t15-absence-matrix`)
  reproduces it. The real-daemon VM check is recorded by hand, not automated.

## Consequences

- T-16/T-17 consume the matrix instead of re-deriving absence per pane; a new
  pane must add its row and a case to the absence suite before it ships.
- The absence suite's routing sweep becomes the structural guard against a
  pane that throws or disables wholesale when its subsystem is gone; it is
  cheap and catches the class of bug the per-pane suites miss.
- "No bridge host" and "daemon masked" are the same consumer state, so the
  nested no-host capture (`t15-breadth.png`) is valid live evidence without a
  VM.
- This consolidates, and does not change, the per-adapter decisions in ADRs
  0117 (Bluetooth), 0119 (storage), 0121 (audio routing), 0126 (overview),
  0128 (power), 0130 (notifications), 0132 (lock), 0134 (menu bar), 0136
  (general/updates), 0138 (accounts), 0140 (printers), 0142 (privacy), 0144
  (accessibility), and 0146 (VPN).