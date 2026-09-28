# 0138 — The Users and Groups adapter projects the host stack

- **Status:** Accepted (T-15.11a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  ADR [0128](0128-battery-power-profiles-adapter.md),
  ADR [0136](0136-general-about-updates-adapter.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.11 splits the macOS Users & Groups surface into an adapter (T-15.11a) and
a Settings pane plus Control Center tile (T-15.11b). [08-settings.md] routes the
pane to "accountsservice + distro provider". AccountsService
(`org.freedesktop.Accounts`) owns the user list over the system bus; it has **no
group API**, so "Users **and** Groups" needs a second source. The task requires
state, events, and absent-daemon behavior, unit-tested against a mock, and
forbids reimplementing account management.

## Decision

- **A new crate, `dragonfruit-account-adapter` (`services/account-adapter`).**
  It implements the T-07 adapter contract (`Adapter`, `AdapterState`,
  `Subscription`) behind its own `AccountSource` seam, exactly like the other
  T-15 adapters. `AdapterId::ACCOUNTS` (id `accounts`) joins the shared ids.
- **Two halves, one snapshot.** `AccountsData` carries the AccountsService
  user list (`AccountData`:` uid`, login/real name, account type, password mode,
  home, shell, email, language, avatar, locked, system account, automatic
  login, login time, session) and an optional group list (`GroupData`) from the
  distribution provider. `AccountsSnapshot` types both, orders human users
  before system accounts and user groups before system ones, and derives the
  labels/helpers the pane draws.
- **The live user read is a real AccountsService client.** `HostAccounts` is
  the concrete `AccountSource`: `ListCachedUsers` at
  `/org/freedesktop/Accounts`, then
  `org.freedesktop.DBus.Properties.GetAll("org.freedesktop.Accounts.User")` on
  each user object. `account_from_props` is pure, so a fixture map drives it in
  tests. The writes are single method calls: `CreateUser`/`DeleteUser` on the
  manager, `SetAccountType`/`SetLocked`/`SetAutomaticLogin` on the user object.
- **Groups are a distro provider seam, not an implementation.** `GroupProvider`
  (`status`, `create_group`, `delete_group`, `set_members`) is the half
  AccountsService cannot supply; the concrete provider is distro-specific and
  belongs with packaging, so `HostAccounts::new` runs with none and reports
  groups as absent until one is attached. This mirrors T-15.10a's
  `UpdateProvider`.
- **Absence is layered and normal.** The adapter is `Unavailable` only when
  neither AccountsService nor the provider is reachable. AccountsService
  answering with no cached user is `Available` with an empty snapshot
  (`AccountsSnapshot::present()` false); a running host with no group provider
  is `Available` with `groups: None`, so only the group controls disable. A
  present stack that cannot be read is `Error`, visible and inert with the
  message.
- **Explicit writes, no invented snapshots.** The account and group writes
  forward one request each and answer `Applied`/`Denied`/`Absent`/`Failed`; the
  daemon or provider publishes the result and the host re-reads. A write never
  changes the adapter state — a group write that is absent because the provider
  is missing must not hide the live user list.
- **One event stream.** `AccountsSnapshot::changes(previous)` is a pure diff —
  users added/removed/edited, the automatic-login user, the provider's
  presence, and groups added/removed/edited — queued by the adapter and drained
  through `drain_changes`.

Rejected: reading groups by parsing `/etc/group` (it reimplements the stack and
misses NSS/LDAP); putting account management in the shell (it is host state,
and AccountsService authorizes through polkit); treating a missing group
provider as an error (it is a normal, layered absence); hiding the whole item
on a group-write absence (the users half is independent).

## Consequences

- `dragonfruit-account-adapter` is a workspace member whose dependencies are
  the adapter contract, `zbus`, and `serde`. CI drives it with `MockAccounts`;
  no AccountsService, provider, shell, or hardware is involved. `make e2e` runs
  `cargo test -p dragonfruit-account-adapter`.
- `HostAccounts` is the live `AccountSource`: its user read is a real
  AccountsService client, its group half is absent until a `GroupProvider` is
  attached, which is the honest state until packaging supplies one.
- T-15.11b adds the Settings pane and Control Center tile. It declares any
  settingsd presentation preferences and wires the pane/tile; it must not add a
  second account or group implementation.