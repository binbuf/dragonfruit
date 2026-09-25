# 0026 — Concrete system adapters live in their own crates behind the dependency-free contract

## Status

accepted

## Context

T-07.1a/T-07.1b put the one adapter contract (`Adapter`, `AdapterState`,
`StatusSlot`, `Subscription`, `AdapterEvent`) in a deliberately
dependency-free crate, `services/system-adapters`, so concrete adapters can
depend on it without leaking their daemon stack
([0024](0024-system-adapter-contract.md)). T-07.2a is the first concrete
adapter: NetworkManager read over its D-Bus API, no `libnm` link. It needs a
D-Bus client (`zbus`) — a stack the contract crate must not carry — and a
headless test path, because CI has no bus and no daemon. ADR 0024 said
"concrete adapters bring their own dependencies behind the `Adapter` trait"
but left the directory layout open.

## Decision

- **Each concrete adapter is its own crate.** `dragonfruit-networkmanager`
  (`services/networkmanager`) depends on `dragonfruit-system-adapters` and
  owns its daemon stack: `zbus` for the D-Bus client, `serde` for the fixture
  shape. T-07.3 and T-07.4 add `services/audio` and `services/power` the same
  way. The contract crate stays dependency-free.
- **A source trait is the transport seam.** `NetworkManagerSource::read`
  returns the raw `NetworkManagerData` (`Ok(Some)`), absence (`Ok(None)`), or a
  read failure (`Err`). The real source is `DbusNetworkManager`; tests and CI
  use `MockNetworkManager`, which serves a fixture with no bus. The adapter
  (`NetworkManagerAdapter<S>`) turns one read into the typed `WifiSnapshot`
  and drives the shared `Subscription`.
- **The state owns the degradation, as before.** Absence maps to
  `AdapterState::Unavailable` (hidden), a failure to `AdapterState::Error`
  (visible, inert), data to `Available`. No consumer re-derives it.

Rejected: adding `zbus` to `services/system-adapters` (would drag a D-Bus
stack into the contract and every consumer/test, contradicting 0024); putting
the adapter as a module in the contract crate (same leak); making the D-Bus
path a `#[cfg(test)]`-only or feature-gated stub (the real read path must
compile and ship).

## Consequences

- T-07.2b extends `dragonfruit-networkmanager` with the write path (activate/
  join and polkit degradation); T-07.3/T-07.4 follow the crate-per-adapter
  layout.
- The host that bridges adapters to the C++ shell (T-07.5) depends on the
  concrete adapter crates and sees only `Adapter`/`StatusSlot`; it must not
  add a poll loop.
- `make e2e` runs each concrete adapter crate's tests, so `services/
  networkmanager` is in the gate.
- The live D-Bus source is compile-checked but not exercised in CI (no bus, no
  daemon); the tested contract is the fixture-driven seam, mirroring T-07.1a.