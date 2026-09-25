# 0024 — System-service adapters share one dependency-free contract with three states

## Status

accepted

## Context

T-07 makes the menu bar live (Wi-Fi, volume, battery) over host daemons
(NetworkManager, PipeWire/WirePlumber, UPower). The design requires adapters
that are thin, mockable, and daemon-agnostic above the adapter, and FR-1
requires every adapter to expose available / unavailable / error states with a
mock for tests. Before any concrete adapter lands, T-07.1a must fix the
contract so T-07.2–T-07.4 (network, audio, power) and T-07.1b (subscription)
build on one shape. The shell already has status-item slots with an
`available` flag (`shell/menubar/StatusItem.qml`); the contract must project
onto them without the UI re-deriving degradation.

## Decision

- **One dependency-free crate owns the contract.** `services/system-adapters`
  (`dragonfruit-system-adapters`) defines the `Adapter` trait,
  `AdapterState<T>`, `AdapterId`, `StatusSlot`, and `MockAdapter`. It has no
  dependencies: every concrete adapter depends on it and brings its own
  D-Bus/daemon stack behind the trait, so no daemon-specific type leaks above
  the adapter.
- **Three states, owned by the state enum.** `AdapterState<T>` is
  `Available(T)` (the daemon answered), `Unavailable` (the daemon is absent —
  a normal state, not an error), or `Error(AdapterError)` (the daemon is
  present but unreadable). A new mock defaults to `Unavailable`, the safe
  "daemon absent" path.
- **The state owns the degradation policy.** Consumers project with
  `AdapterState::slot` / `status_slots`: `Unavailable` hides the slot, `Error`
  shows it visible-but-inert with the message, `Available` shows it live. The
  `AdapterId` constants are chosen to match the shell status-item ids
  (`wifi`, `bluetooth`, `volume`, `battery`).
- **No polling seam.** An adapter exposes the last state pushed by its daemon;
  consumers read that. Subscription and re-subscribe on restart are T-07.1b
  and will extend this crate.

Rejected: putting the contract in `df-ipc` (it is an internal, daemon-agnostic
adapter API, not a cross-process constant set); a per-adapter trait with no
shared state enum (each adopter would re-invent degradation and could disagree
with the shell); a polling `tick()` on the trait (would invite poll loops above
the adapter, which the track forbids).

## Consequences

- T-07.1b adds the subscription/event API to this crate; `AdapterState` stays
  the stable base and the `slot` projection is unchanged.
- T-07.2–T-07.4 implement the trait and add their snapshots; the mock is the
  CI path so the adapters' tests run with no daemon on the bus.
- Whatever host wires the adapters to the shell (T-07.5) consumes `StatusSlot`
  only; hidden/inert/live is decided once, here.
- `make e2e` runs `cargo test -p dragonfruit-system-adapters` so the contract
  cannot regress silently.