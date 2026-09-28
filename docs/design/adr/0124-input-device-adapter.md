# 0124 — The input device adapter is a read-only libinput inventory

## Status

accepted

## Context

T-15.4 splits the macOS Keyboard / Mouse / Trackpad surface into an adapter
(T-15.4a) and a Settings pane plus Control Center tile (T-15.4b). The task's
host stack is **libinput**, but the design's principle 3 (and the legacy
adapter roster's FR-4) says keyboard settings go through the **compositor**
API, never a system daemon: the compositor owns the physical devices, and
`settingsd` already owns `input.repeatDelay`/`input.repeatRate` and forwards
them over the private `df_toplevel_manager.set_input_policy` bridge
([0034](0034-compositor-policy-via-shell-bridge.md)). libinput itself keeps no
persisted configuration and ships no setter tool. There is therefore no daemon
to read the *user's* settings from; the adapter's honest job is the device
inventory.

## Decision

- **A new crate, `dragonfruit-input` (`services/input`).** It implements the
  T-07 adapter contract (`Adapter`, `AdapterState`, `Subscription`) behind its
  own `InputSource` seam, exactly like `dragonfruit-bluetooth` and
  `dragonfruit-storage`. `AdapterId::INPUT` joins the shared ids.
- **The live source reuses libinput's shipped control tool.** `CommandLibinput`
  runs `libinput list-devices` once per `InputAdapter::refresh` and parses the
  record-per-device output into `InputData` (`services/input/src/libinput.rs`).
  This mirrors the audio adapter's `pw-dump` source
  ([0028](0028-audio-adapter-over-wireplumber-cli.md)): the CLI's human-oriented
  format is pinned in one parser and tested against a fixture, so the project
  links no libinput and the compositor remains the only holder of device
  handles. The parser is total (unknown keys are ignored, blank records
  skipped), so a tool that grows a field keeps working.
- **The model classifies, the adapter does not.** `InputSnapshot` groups devices
  into `DeviceKind::{Keyboard, Mouse, Touchpad, Touchscreen, Tablet, Switch,
  Other}` from the capability tokens (a pointer with `gesture`/tapping is a
  touchpad, any other pointer a mouse), orders keyboards then pointers, and
  derives the glyph/label. A pure `device_changes(previous)` diff reports
  added/removed devices as the event half of the adapter.
- **It is read-only, by design.** libinput offers no write, and the user's
  settings stay where the design already put them: `settingsd` keys applied by
  the compositor over the shell bridge. The adapter answers "which devices does
  the session have, and what can they do?"; it is not a second settings owner.
  The pointer rows the pane will add (`Tracking speed`, `Tap to click`,
  scrolling, …) are therefore **settingsd keys consumed by the compositor**,
  added by T-15.4b when the pane that writes them lands — the same split T-15.3b
  used for the Sound Effects/Balance keys.
- **Absence has two normal forms.** libinput missing or unable to reach a seat
  is `AdapterState::Unavailable` (hidden); a running stack with no recognized
  device answers `Available` with `InputSnapshot::present()` false (also
  hidden), exactly like the battery item's no-battery case. `MockInput`'s
  `kill()`/`restart()` drive the re-subscribe lifecycle in CI.

Rejected: extending the compositor's `InputSettings` into a services adapter
(the compositor is a Wayland server with no bus and no consumer path for it,
and [0034](0034-compositor-policy-via-shell-bridge.md) already made the shell
its sole forwarder); reading `/dev/input` or udev directly (that reimplements
libinput, which the task forbids); a new `set-*` protocol now (no pane exists
to drive it; T-15.4b owns the pointer keys and their compositor wiring).

## Consequences

- `dragonfruit-input` is a workspace member with no external dependency beyond
  the dependency-free adapter contract. CI drives it with `MockInput`; the live
  tool needs no hardware to build.
- Tests: 26 unit tests in-crate (parser, model, adapter) plus
  `services/input/tests/read_path.rs` (5 integration tests: inventory,
  lifecycle, device-change diff, absence at every seam, free construction) over
  the captured fixture `tests/fixtures/libinput-list-devices.txt`. `make e2e`
  runs `cargo test -p dragonfruit-input`.
- T-15.4b adds the pane and tile: it reads the settings values through
  `settingsd`/the bridge and may project this inventory through the status
  bridge host, following the `*Host`/`*Client` precedent. The pointer settings
  keys it adds are settingsd-owned and compositor-consumed.
- The parser depends on a tool whose man page says its output "may change at
  any time"; the fixture is the tripwire, and a format change is a one-file
  fix behind the source seam.