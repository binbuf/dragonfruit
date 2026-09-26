# 0087 — app-index events, launch registry, and recency

## Status

accepted

## Context

`app-index` owns application identity (ADR
[0086](0086-app-index-identity-ownership.md)) but its T-14.1a surface was
queries only: it scanned the `.desktop` corpus once and never changed, and it
knew nothing about which apps were on screen. The design calls for a live
index (install/uninstall/update reflected without a restart) and a launch
registry with recency so the Dock, menu bar, and app switcher stop inferring
app usage. The compositor owns windows and, per ADR
[0034](0034-compositor-policy-via-shell-bridge.md), opens no D-Bus; the shell
is the sole forwarder. The subscription API (coalesced change signals) is
T-14.1c.

## Decision

- **The index is live and diff-based.** `AppIndex` remembers the directories it
  was built from; `refresh()` re-scans them and diffs against the current
  corpus. A new desktop id is `installed`, a changed record `updated`, a
  vanished id `uninstalled`. The service watches the directories with inotify
  (a recursive `DirectoryWatcher`, debounced 150 ms; no polling at idle) and
  calls the same path. The diff is returned and retained (bounded, drained by
  `IndexEvents`). A refresh never touches the resolution counters or the miss
  set: maintenance is not a query.
- **The launch registry is pure and activity-fed.** `LaunchRegistry` tracks the
  running apps and their window counts and a most-recent-first recency order
  (bounded at 32) keyed by resolved desktop id, or the canonical raw identity
  (`WM_CLASS` class, else instance, else `app_id`) on a miss. Recency is
  ordered by the registry's insertion order, not by timestamps, so events in
  the same millisecond still order correctly. An exited app leaves the running
  set but stays in recency; a focus change reorders without changing the
  running set.
- **The bus surface is additive.** `WindowOpened`/`WindowClosed`/`NoteActivity`
  feed the registry; `Running`/`Recent` read it; `Refresh`/`IndexEvents` and
  `ActivityEvents` expose the logs. `AppRunning`, `AppExited`, and
  `IndexChanged` are emitted as signals now so T-14.1c only adds the
  subscription bookkeeping. Flat JSON stays the payload format.
- **The shell remains the forwarder.** Window map/unmap and focus reach the
  service through these methods; the compositor stays D-Bus-free.

## Consequences

- A `.desktop` install/uninstall/update reaches consumers without a restart,
  and only on a monitor event — idle cost stays zero.
- The registry is std-only and deterministic, so it is unit-tested directly;
  the bus surface is covered against a private `dbus-daemon`
  (`services/app-index/tests/session_bus.rs`).
- Recency is per-session in memory; persistence (across restarts) and the
  subscription/coalescing API are future work (T-14.1c and later).
- The shell does not yet call the activity methods; wiring the window
  lifecycle forwarder is deferred and recorded in the task hand-off.