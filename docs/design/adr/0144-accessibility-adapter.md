# 0144 — The Accessibility adapter projects the AT-SPI accessibility bus

- **Status:** Accepted (T-15.14a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  [15-system-services-breadth.md](../tracks/15-system-services-breadth.md),
  ADR [0124](0124-input-device-adapter.md)
- **Supersedes:** —

## Context

T-15.14 splits the macOS Accessibility surface into an adapter (T-15.14a) and a
Settings pane plus Control Center tile (T-15.14b). [07-system-integration.md]
and [08-settings.md] route the pane to
"`settingsd` → compositor magnification + toolkit/AT-SPI settings": the
compositor owns magnification, `settingsd` owns the durable presentation
preferences, and the **AT-SPI accessibility bus** owns the live assistive
bridge every toolkit and screen reader already speaks. T-15.14a must ship the
backend half — state, events, and absent-daemon behavior — unit-tested against
a mock, and must reuse the host stack rather than reimplement it.

## Decision

- **A new crate, `dragonfruit-accessibility-adapter`
  (`services/accessibility-adapter`).** It implements the T-07 adapter contract
  (`Adapter`, `AdapterState`, `Subscription`) behind its own
  `AccessibilitySource` seam, exactly like the other T-15 adapters.
  `AdapterId::ACCESSIBILITY` (id `accessibility`) joins the shared ids and
  matches the shell's `accessibility` status-item id.
- **The host stack is the AT-SPI accessibility bus.** The live source,
  `HostAccessibility`, uses the session bus and reads the standard
  `org.a11y.Status` properties at `/org/a11y/bus` on the well-known name
  `org.a11y.Bus`: `IsEnabled` (the toolkit accessibility bridge is on) and
  `ScreenReaderEnabled` (a screen reader is enabled). One read is one
  `org.freedesktop.DBus.Properties.GetAll`. This is the same reuse the audio
  adapter makes of `pw-dump`/`wpctl` and the input adapter of
  `libinput list-devices`: the host stack owns the value, and the project
  reimplements no screen reader, toolkit, or magnifier.
- **The adapter is read-only.** `org.a11y.Status` offers no setter. The user's
  durable accessibility preferences are `settingsd`'s and magnification is
  compositor-owned ([07-system-integration.md], principle 3), exactly as input
  settings are. This adapter is the live bridge projection — the two flags and
  their derived labels — and has no writes.
- **Absence is normal and single-layered.** The adapter is `Unavailable` only
  when the session bus is unreachable or no `org.a11y.Bus` owns its name. A bus
  that answers with every feature off is `Available` with
  `AccessibilitySnapshot::present()` false; the tile/pane's second hide rule is
  `present()` (any feature on), exactly as the input tile hides on an empty
  inventory. A bus that owns its name but cannot be read is `Error`, visible
  and inert with the message.
- **One event stream.** The shared subscription lifecycle (`AdapterEvent`)
  plus a pure `AccessibilitySnapshot::changes(previous)` diff —
  `Enabled`/`ScreenReader` flag moves — queued by the adapter and drained
  through `drain_changes`.

Rejected: reimplementing a screen reader, a toolkit magnifier, or an
accessibility settings store; adding an `org.a11y` write that does not exist;
treating an all-off bus as absence (off is a value, not a missing daemon);
reading a desktop-specific settings schema (the project names no desktop's
GSettings).

## Consequences

- `dragonfruit-accessibility-adapter` is a workspace member whose dependencies
  are the adapter contract and `zbus`. CI drives it with `MockAccessibility`;
  no AT-SPI, bus, shell, or hardware is involved. `make e2e` runs
  `cargo test -p dragonfruit-accessibility-adapter`.
- `HostAccessibility` is the live `AccessibilitySource`; the mock is the
  contract's CI path. The two property names and their derived labels are
  pinned by tests, and a live read on a machine with an accessibility bus is
  exercised where one exists.
- T-15.14b adds the pane and tile. It projects this adapter through the bridge
  host like the other T-15 adapters and must not add a second accessibility
  implementation. The durable preferences and compositor magnification the
  reference rows need are T-15.14b's to declare.