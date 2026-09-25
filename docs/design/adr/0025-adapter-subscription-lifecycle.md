# 0025 — Adapters own one subscription lifecycle; events carry no snapshot

## Status

accepted

## Context

FR-1 requires every system adapter to support event subscription with no
polling above the adapter, and FR-6 requires adapters to re-subscribe and
re-sync when their daemon restarts. T-07.1a fixed the state contract
(`AdapterState`, `AdapterId`, `StatusSlot`) but deliberately left the
subscription seam open ([0024](0024-system-adapter-contract.md)). T-07.1b must
fix that seam before T-07.2–T-07.4 implement NetworkManager, PipeWire/
WirePlumber, and UPower adapters, so all three handle a daemon dying and
returning the same way and the absence/error split is decided once.

## Decision

- **One subscription lifecycle in the same dependency-free crate.**
  `services/system-adapters/src/subscription.rs` defines `ConnectionState`
  (`Absent`/`Subscribed`), `Subscription` (the per-adapter bookkeeping and
  bounded event outbox), and `AdapterEvent`. Concrete adapters own a
  `Subscription` and drive it from their transport's name-owner/signal
  handling; the mock drives it directly.
- **`Adapter` gains `connection()` and `drain_events()`.** `AdapterState`
  stays the stable base and the `slot()` projection is unchanged. The host's
  event loop drains events and re-reads `Adapter::state`; nothing above the
  adapter ever queries the daemon.
- **Events carry no snapshot** (`Subscribed { resubscribe }`, `Disconnected`,
  `Changed`). The adapter state is the single source of the payload, so a
  stream entry and the state cannot disagree.
- **A restart is `absent()` then `subscribed()`**, emitting `Disconnected`
  then `Subscribed { resubscribe: true }` and counting a second subscribe.
  The first subscribe is `resubscribe: false`. Redundant lifecycle calls are
  no-ops so a busy loop cannot inflate the restart count.
- **Absence is not an error.** `SubscribeError::Absent` projects to
  `Unavailable` (slot hidden) and `SubscribeError::Failed` to `Error` (slot
  visible, inert); neither blocks startup.

Rejected: an `EventSource`/transport trait in this crate (there is no D-Bus
stack here and every concrete adapter has a different transport); delivering
the snapshot in the event (two sources of truth); a `poll_events` name (reads
as the poll loop the track forbids); a separate `Subscribed` trait (every
adapter is required to have a subscription, so it belongs on `Adapter`).

## Consequences

- T-07.2–T-07.4 implement `Adapter::connection`/`drain_events` with a
  `Subscription` field and classify their connect failures as `Absent` vs.
  `Failed`; the mock is the CI path for all three.
- T-07.5 (the host that bridges the adapters to the shell) drains
  `AdapterEvent`s and reads `StatusSlot`; it must not add a poll loop.
- `make e2e` already runs the crate's tests, so the lifecycle cannot regress
  silently; the acceptance test is `tests/subscription.rs`.