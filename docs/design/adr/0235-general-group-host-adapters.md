# 0235 — General group host adapters and the in-place sub-pane stack

- **Status:** Accepted (T226)
- **Date:** 2026-11-08

## Context

The General group is not one daemon: About/Software Update read the host stack
and the distribution update provider (ADR 0136), while Date & Time, Language &
Region, Login Items, Sharing, Backup, Storage usage, and Startup Disk each have
a different owner. T-15.10b shipped only About and Software Update and omitted
the rest rather than ship dead rows. T226 must land the reference information
architecture with real host reads, explicit writes, and a per-owner absence
path.

## Decision

1. **One dependency-free adapter crate.** `services/general-adapter`
   (`dragonfruit-general-adapter`) owns every General sub-pane read and write
   behind one `GeneralSource` seam. `HostGeneral` is the live implementation;
   `MockGeneral` is the deterministic fixture. Each sub-pane is an `Option` in
   `GeneralSnapshot`, so one missing tool disables only its own pane and records
   its reason in `errors`.
2. **One bridge-host interface.** `org.dragonfruit.SystemStatus1.General`
   (`services/system-status/src/general.rs`) projects the typed snapshot to the
   flat JSON view and exposes one explicit write per row. The Settings app
   reads it through `apps/settings/GeneralClient.*`; QML never shells out.
3. **The update provider is native, not a second package manager.** T226
   attaches `CommandUpdateProvider` to the existing `Updates` interface:
   `dnf`, `apt-get`, `pacman`, or PackageKit (`pkcon`) is detected at startup,
   check/install run on a worker thread, and the phase is the progress. The
   durable `updates.auto`/`updates.autoInstall` keys default to `false`; the
   pane forwards them to the running provider, and nothing silently enables
   them.
4. **The General list is the only sidebar pane; sub-panes open in place.**
   About/Software Update stay dialogs, Storage keeps its existing top-level
   pane and the row navigates there, and the remaining rows open a sub-pane
   with a `‹ General` back control. A row's description is the same live state
   the sub-pane renders.
5. **Reset is one settingsd method.** `org.dragonfruit.Settings1.Reset`
   reseeds every key to its schema default, persists once, and emits one
   `Changed` per changed key. It is not a factory reset and never touches
   personal files or host configuration.

## Consequences

- The schema gains revision 20 with `clock.use24Hour`, `updates.auto`,
  `updates.autoInstall`, the six `locale.*` presentation keys, and the four
  `backup.*` keys. All defaults are the conservative/off choice.
- The shell's menu-bar clock consumes `clock.use24Hour`; the Date & Time pane
  therefore changes a visible surface, not just a stored value.
- The backup pane delegates the engine: restic/Pika/Timeshift do the work, and
  an absent backend disables only the Backup sub-pane.
- The sharing pane hides services whose units/binaries are not installed; it
  never renders a toggle that cannot be applied.
- A future packaging task can replace the native provider behind the same
  `UpdateProvider` seam without touching the pane or the view.
