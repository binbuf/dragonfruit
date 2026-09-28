# 0136 — The General, About, and Updates adapter projects the host stack

- **Status:** Accepted (T-15.10a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  [12-packaging.md](../12-packaging.md),
  ADR [0128](0128-battery-power-profiles-adapter.md),
  ADR [0130](0130-notifications-focus-adapter.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.10 splits the General surface into an adapter (T-15.10a) and a Settings
pane plus Control Center tile (T-15.10b). The pane's rows are the macOS General
list: `About` (the device/release/spec block) and `Software Update` (the distro
path), plus settingsd-owned defaults. `System_Preferences.md` routes the pane
to "settingsd + the distro update provider".

There is no single daemon to wrap. The host stack is two independent halves:
the **host identity** (`/etc/os-release`, `/proc`, the DMI sysfs nodes) that
`About` renders, and the **distribution update provider** — the
`SystemProvider` of [08-settings.md], which [12-packaging.md] deliberately puts
below a distro-agnostic interface so `FedoraProvider`/`DebianProvider` can
differ. The task requires state, events, and absent-daemon behavior,
unit-tested against a mock, and forbids reimplementing package management.

## Decision

- **A new crate, `dragonfruit-update-adapter` (`services/update-adapter`).**
  It implements the T-07 adapter contract (`Adapter`, `AdapterState`,
  `Subscription`) behind its own `SystemSource` seam, exactly like the other
  T-15 adapters. `AdapterId::UPDATES` (id `updates`) joins the shared ids.
- **Two halves, one snapshot.** `SystemData` carries the always-read
  `SystemIdentity` (host name, `PRETTY_NAME`/`VERSION_ID`/`ID`, kernel,
  architecture, DMI model/serial, processor, memory) and an optional
  `UpdateData` (phase, the offered list, last-check time, message).
  `SystemSnapshot` types both and derives the tile label/glyph and the pane
  rows.
- **Absence is layered, like the battery adapter's.** The adapter is
  `Unavailable` only when neither half is reachable. A host that answers but
  runs no update provider is `Available` with `updates: None`: the About rows
  stay live and only the update controls disable. A present provider that
  cannot be read is `Error`, visible and inert with the message.
- **The provider is a seam, not an implementation.** `UpdateProvider`
  (`status`, `start_check`, `start_install`, `start_reboot`) is the
  distro-specific half. This crate ships the trait and `MockUpdateProvider`;
  the concrete provider belongs with packaging (T-16.9/T-16.10), where the
  `SystemProvider` interface already lives. The adapter never parses
  package-manager output or resolves dependencies.
- **Explicit writes, no invented snapshots.** `UpdateAdapter::check`/`install`/
  `reboot` forward one request each and return `Applied`/`Absent`/`Failed`;
  the provider publishes the result and the host re-reads.
- **One event stream.** `SystemSnapshot::changes(previous)` is a pure diff —
  the identity, the provider's presence, the phase, the list size, the
  last-check time, and the message — queued by the adapter and drained through
  `drain_changes`.

Rejected: hardcoding a package manager in this crate (it would duplicate the
packaging provider and defeat portability); an async provider (the adapter
contract is synchronous and event-driven; the provider owns any concurrency);
putting the identity read in the shell (About is system state, not chrome);
treating a missing provider as an error (it is a normal, layered absence).

## Consequences

- `dragonfruit-update-adapter` is a workspace member whose only dependency is
  the adapter contract. CI drives it with `MockSystem`; no package manager,
  provider, shell, or hardware is involved. `make e2e` runs
  `cargo test -p dragonfruit-update-adapter`.
- `HostSystem` is the live `SystemSource`: its identity read is
  root-configurable and unit-tested against a fixture tree; its update half is
  absent until a `UpdateProvider` is attached, which is the honest state until
  packaging supplies one.
- T-15.10b adds the Settings pane and Control Center tile. It declares any
  settingsd presentation preferences and wires the pane/tile; it must not add a
  second update implementation.
- General's "defaults" (language/region, login items, sharing, startup disk)
  remain other providers or future tasks; this adapter covers About and
  Updates, which is what the T-15.10a acceptance names.