# 0171 — Hardware input validation is a typed matrix, and a non-US layout is a headless input class

## Status

accepted

## Context

T-03.3 validates the input stack on **real devices**: mouse, keyboard,
touchpad gestures, hot corners, tablet/pen pressure, and one non-US layout
([03-real-session-bringup-perf.md](../tracks/03-real-session-bringup-perf.md)).
Like T-03.2's DRM bring-up, real-device validation needs the compositor to own
a libinput seat, and this host has no free seat (a KDE Wayland session owns
`seat0`). The track's rule is explicit: if hardware is unavailable the slice is
*marked open, not silently skipped*, and swept on the hardware rail.

Several of the classes are already exercised without hardware: the synthetic
input harness is libinput-equivalent and feeds the one
`process_input_event` router, and
`compositor/tests/shell_protocol_conformance.rs::synthetic_input_drives_shortcuts_hot_corners_and_gestures`
drives shortcuts, hot corners, and a four-finger swipe. The pen is the
exception — the synthetic wire format has no pressure axis — and no test
validated a non-US layout at all.

## Decision

- **One pure classification owns the matrix.**
  `compositor/src/input_validation.rs` defines `InputClass` (six classes, with
  stable ids) and classifies each into `Coverage::Device`,
  `Coverage::Headless`, or `Coverage::Gap(NoDevice | NoSeat)` from a
  `seat_owned` bit and a `SeatInventory`. It is exposed through the
  `dragonfruit-compositor` library so `compositor/tests/input_validation.rs`
  (in `make e2e`) pins it without hardware.
- **A class is `Device` only when the seat is owned *and* the device is
  present.** Otherwise a headless-covered class is `Headless` (the router path
  is exercised, real-device confirmation is still due); the pen, with no
  synthetic route, is always a `Gap` naming `no-device` or `no-seat`. The
  matrix never lets a synthetic pass stand in for a real pen.
- **The non-US layout is validated headlessly.** `layout_keysym_for_evdev`
  compiles the compositor's pinned RMLVO path (`df_ipc::keymap` rules, model,
  options) with an explicit layout and returns the keysym an evdev key
  produces. The test asserts German (QWERTZ) swaps the physical y/z keys.
- **A script records the open.** `scripts/input-validation.sh`
  (`make input-validation`) derives seat ownership from the T-03.2
  `libdrm` master probe, reads the inventory from `libinput list-devices` (or
  `/proc/bus/input/devices` when libinput cannot open the event nodes), and
  writes `docs/captures/t03-input-matrix.open.txt` with no free seat, or
  `t03-input-matrix.txt` with one. It always exits 0: a no-seat host must not
  fail the pipeline.

## Consequences

- The T-03.3 unit is **open** on this host (`1 gap: tablet-pen`; the other
  classes are headless-covered); T-03.4's runbook completes the real-device run
  and replaces the open artifact.
- The six `InputClass::ALL` ids are the canonical class list;
  `scripts/input-validation-probe.py` must stay in sync with them.
- T-16 owns per-device acceleration tuning and the driver matrix; this unit
  records coverage, it does not tune.