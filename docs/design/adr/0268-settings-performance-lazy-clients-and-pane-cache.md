# 0268 — Settings performance is lazy clients, per-key state, and a cached pane

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-15 |
| **Task** | T268 |

## Context

System Settings constructed all fifteen adapter clients in the
`SettingsBridge` constructor. Every client's constructor performs a host read,
so a cold open made ~20 D-Bus calls before the first pane painted. The QML
`Settings.values` property returned a fresh `QVariantMap` and notified every
binding on every settingsd change; `writeErrors`/`writeStates` did the same.
Every `Settings.schemaFor()` rebuilt the 142-key defaults table and scanned the
constraint table linearly. Sliders wrote on every `moved()` tick, and the
wallpaper provider was read with four blocking `Get` calls. Finally, the shell's
`Loader` destroyed each pane body on switch, so returning to a pane re-ran its
`Component.onCompleted` refreshes.

## Decision

1. Adapter clients are created on first use by the pane that owns them. The
   bridge exposes `adapterClientCount` for diagnostics; it is zero after
   construction. Fixture selection stays per client, and a mock publishes its
   initial view once its signals are connected.
2. `Settings.values`, `Settings.writeErrors`, and `Settings.writeStates` are
   `QQmlPropertyMap` instances. The bracket syntax (`Settings.values["key"]`)
   is unchanged, but a key notification only re-evaluates bindings on that key.
   A rejected write re-publishes the stored value through an invalid transient
   so a control that already moved snaps back.
3. `settingsSchemaDefaults()` returns one immutable process-wide table;
   `settingsSchemaInfo()` serves a precomputed key→info table, and constraints
   are a keyed hash.
4. Sliders write on `committed`, not `moved`. The Appearance transparency
   slider keeps its bounded debounce for live preview; every other slider
   commits once on release.
5. The wallpaper provider is read with one asynchronous `Properties.GetAll`;
   concurrent requests coalesce into one follow-up read. No provider call
   blocks the UI thread.
6. `PaneLoader` keeps each instantiated pane alive in a hidden holder and
   reparents it on return. A pane that needs fresh host state asks for it
   explicitly through its own `refresh*()` entry point.

## Consequences

- Cold open paints the first pane without waiting for unrelated host services.
- Returning to a pane does not restart adapter reads or re-apply policy.
  Opening General no longer forwards the schema default update policy.
- Cached panes keep their bindings and timers alive while hidden; a pane with
  an unbounded live subscription must scope it to visibility if that becomes a
  cost. The current adapters are event-driven and cheap when idle.
- The performance test (`make t268-settings-performance`) records cold open and
  first/cached pane-switch p50/p95/p99. Its offscreen budgets are wider than
  the reference 33.3 ms pane-switch target; they are a regression floor, not a
  hardware claim.
- Adapter reads that cannot be made asynchronous remain named `OPEN` rows in
  the design doc rather than being hidden behind a blank first paint.
