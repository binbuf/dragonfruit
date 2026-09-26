# 0088 — app-index subscription and signal coalescing

## Status

accepted

## Context

`app-index` owns application identity, a live `.desktop` corpus, and a launch
registry (ADR [0086](0086-app-index-identity-ownership.md), ADR
[0087](0087-app-index-events-launch-registry-recency.md)). T-14.1b emitted
`AppRunning`, `AppExited`, and `IndexChanged` as broadcast signals, but no
consumer could declare what it cared about and a burst of changes (an install
storm, a window appearing and then taking focus) produced one signal per
change. Consumers either re-queried the index or would have to poll. T-14.1c
adds the subscription API.

## Decision

- **Subscription is per consumer, by unique bus name.** `Subscribe(interests)`
  records the caller's unique name and the categories it wants; `Unsubscribe`
  drops it; `SubscriberCount` is diagnostics. A consumer's identity is the
  sender header, so a reconnect re-subscribes by calling `Subscribe` again and
  there is no token to persist.
- **Interests are `identity` / `recency` / `icons`.** Identity covers
  install/update/uninstall, recency covers app running/exited/focused, and icons
  covers the set of themed icons that resolve. The spec is a comma-separated
  set or `all`; an empty or unrecognized spec means `all`, so an older consumer
  never goes silent.
- **Signals are coalesced and directed.** A change notes its kind; the first
  change opens a `COALESCE_WINDOW_MS` (100 ms) window that later changes fold
  into, and the window closes into **one** directed `Changed(interests)` signal
  per subscriber carrying the union of changed kinds. The window is not extended
  by later changes, so a continuous stream still delivers at a bounded rate.
  `Flush` delivers pending notices immediately (the deterministic test hook).
- **The bookkeeping is pure.** `subscription::Subscriptions` owns the
  subscriber table, the pending sets, and the deadline; the D-Bus layer only
  forwards notices and starts a condvar-woken coalescer thread that sleeps until
  `next_deadline_ms`, so an idle service does no work.
- **Granular signals stay.** `AppRunning`/`AppExited`/`IndexChanged` remain for
  observability; `Changed` is the coalesced consumer path.

## Consequences

- Consumers stop polling and stop re-querying: a burst is one wake-up, and the
  payload says which categories changed so the consumer can re-read only what it
  needs.
- The pure model is unit-tested with an injected clock; the wire contract is
  covered against a private `dbus-daemon`
  (`services/app-index/tests/session_bus.rs`).
- The shell does not subscribe yet — it still loads the index once at startup.
  Wiring the shell's `Changed` subscription (and the window-activity forwarder
  from T-14.1b) is deferred; see the task hand-off.