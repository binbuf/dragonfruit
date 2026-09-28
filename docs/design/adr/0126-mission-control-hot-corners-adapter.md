# 0126 — The Mission Control and hot corners adapter projects the compositor

- **Status:** Accepted (T-15.5a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [03-workspaces.md](../03-workspaces.md), [04-shell.md](../04-shell.md),
  ADR [0124](0124-input-device-adapter.md),
  ADR [0034](0034-compositor-policy-via-shell-bridge.md)

## Context

T-15.5 splits the Mission Control / hot corners surface into an adapter
(T-15.5a) and a Settings pane plus Control Center tile (T-15.5b). Unlike
Bluetooth, storage, audio, and input, this subsystem has **no external daemon**:
the compositor detects hot corners in its input path
(`compositor/src/input/hot_corners.rs`) and drives the one overview state
machine (`compositor/src/overview/mod.rs`); the shell mirrors both over the
private `df_toplevel_manager` bridge (`hot_corner`, `overview_changed`). The
durable trigger *configuration* is where principle 3 already put keyboard and
pointer policy: `settingsd` owns the preference and the compositor applies it
live, as the gesture trio already does over `set_input_policy`
([0034](0034-compositor-policy-via-shell-bridge.md)).

The task requires an adapter with state, events, and absent-daemon behavior,
unit-tested against a mock. The risk is re-implementing a corner detector or an
overview transition in a service, which the task explicitly forbids ("reuse,
never reimplement") and which would fork the one-machine rule.

## Decision

- **A new crate, `dragonfruit-overview` (`services/overview`).** It implements
  the T-07 adapter contract (`Adapter`, `AdapterState`, `Subscription`) behind
  its own `MissionControlSource` seam, exactly like `dragonfruit-input`.
  `AdapterId::MISSION_CONTROL` joins the shared ids.
- **It projects, it does not detect.** The raw read is
  `MissionControlData`: the four corner assignments, dwell/inset, the
  gesture-gating trio, and the runtime overview state (active, selection,
  Space/window counts). The typed `MissionControlSnapshot` names corners and
  actions and owns the `HotCornerAction` vocabulary (`none`,
  `mission-control`, `notification-center`, `desktop-reveal`, `lock-screen`)
  with stable ids shared by the settings key and the wire. No detection and no
  transition logic lives here.
- **Two event streams.** `MissionControlSnapshot::changes(previous)` is a pure
  diff (corner reassignment, gesture gating, overview open/close, selection,
  counts). A hot-corner trigger is instantaneous, so it is not a snapshot
  field: the source queues it and `MissionControlAdapter::drain_triggers`
  returns `HotCornerTrigger { corner, action }`.
- **Configuration stays settingsd-owned.** The adapter has no write method.
  The pane writes settings keys; the compositor applies them; the bridge
  publishes the result; the adapter reports it. This is the same split the
  input pointer keys and the Sound Effects keys use.
- **Absence is a normal state.** A missing bridge (no `df_toplevel_manager`
  global, or an unreachable compositor) is `AdapterState::Unavailable` and
  hides the item; a present bridge that cannot be read is `Error`, visible and
  inert with the message. Neither blocks session startup. `MockMissionControl`
  drives the states in CI, with `kill`/`restart` for the re-subscribe lifecycle
  and `trigger` for the event stream.

Rejected: placing the adapter in the compositor crate (the compositor is a
Wayland server and the T-15 track is the services layer; the shell bridge is
the only consumer path, per [0034](0034-compositor-policy-via-shell-bridge.md));
re-implementing corner detection or an overview state machine in a service
(forbidden by the task and a fork of the one-machine rule); depending on the
compositor crate (it drags the whole Smithay backend stack into a
dependency-light adapter).

## Consequences

- `dragonfruit-overview` is a workspace member whose only dependency is the
  dependency-free adapter contract. CI drives it with `MockMissionControl`; no
  compositor, Wayland socket, or hardware is involved.
- Tests: 23 unit tests in-crate (source lifecycle and triggers, model
  classification/diff/labels, adapter lifecycle) plus
  `services/overview/tests/read_path.rs` (7 integration tests: default map,
  re-subscribe, configuration and runtime changes, triggers, absence at every
  seam, free construction). `make e2e` runs
  `cargo test -p dragonfruit-overview`.
- The concrete bridge source is the shell's `df_toplevel_manager` client, which
  is C++: T-15.5b wires it through the status bridge host (following the
  `*Host`/`*Client` precedent) and adds the hot-corner assignment settings keys
  the pane writes. Until then the adapter's live path is the seam the bridge
  fills; the mock is the tested path.
- A later task that wants to *apply* an assignment must extend the compositor
  policy protocol (an append-only request), not add a write to this adapter.