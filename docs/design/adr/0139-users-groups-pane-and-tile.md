# 0139 — The Users & Groups pane and tile ride the bridge host

- **Status:** Accepted (T-15.11b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  ADR [0138](0138-users-and-groups-adapter.md),
  ADR [0137](0137-general-about-updates-pane-and-tile.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.11a landed `dragonfruit-account-adapter` (AccountsService users plus a
distro group provider seam). T-15.11b must ship the Settings pane and the
Control Center tile as one functional unit, with every control applying live.
Account and group management are privileged, explicit actions over the host
stack; the pane's fields are reads. As with General/About/Updates (ADR 0137),
the C++/QML side never links the Rust adapter: it decodes the bridge host's
JSON view.

The Control Center panel already held thirteen tiles at the ceiling of its
fixed 360×1160 surface; a fourteenth had to fit without clipping.

## Decision

- **One new bridge interface, no new daemon.** `services/system-status` serves
  `org.dragonfruit.SystemStatus1.Accounts` at the shared object path. Its
  `AccountsHost<HostAccounts>` wraps the T-15.11a adapter and projects one flat
  `accounts` view (users, groups, counts, automatic login) plus the eight
  explicit writes (`CreateUser`, `DeleteUser`, `SetAccountType`, `SetLocked`,
  `SetAutomaticLogin`, `CreateGroup`, `DeleteGroup`, `SetGroupMembers`). Each
  write returns the `applied`/`denied`/`absent`/`failed` report and the host
  re-reads; no snapshot is invented.
- **No settingsd keys.** Users and groups are host state, not durable
  presentation preferences. The pane's controls are reads plus explicit
  actions, and the tile is read-only.
- **Absence is layered and visible.** The view is `unavailable` only when
  neither AccountsService nor the group provider is reachable; a host with no
  group provider is `available` with `groupsAvailable: false`, so only the
  group controls disable and the user list stays live. The pane shows a
  one-line absence note; the tile hides on `unavailable`.
- **The Control Center tile is a read-only summary.** It carries the `users`
  glyph, the live user-count label, and a `Users & Groups Settings…` link. It
  has no write; account/group writes live in the pane.
- **The panel compacts rather than scrolls.** To fit the fourteenth tile, every
  Control Center tile's vertical padding moves from the `xs` to the `xxs` token
  and the panel gap stays `xs`. A scrolling panel was rejected: the compositor
  forwards no pointer-axis events (`ShellProtocol::onPointerAxis` is a no-op),
  so an in-panel scroll would be unreachable in a real session.
- **Reference deviations, once.** The Apple Account/iCloud sign-in, `iCloud` /
  `Internet Accounts`, `Touch ID & Password` rows, and the FileVault helper
  text are dropped (Apple-only or no honest Linux provider); `Network account
  server` is dropped (no enterprise directory provider, and a disabled
  `Edit…` would be a dead control); the trailing `?` help is omitted
  project-wide. `Add User…` / `Add Group…` open dialogs that perform the
  explicit writes; the per-user info dialog edits account type, disabled state,
  and automatic login and can delete the account; the per-group info dialog
  edits membership and can delete the group.

Rejected: adding a second account/group implementation in the shell (the
adapter owns it); adding settingsd presentation keys (nothing durable is
preference); hiding the whole item on a group-provider absence (the users half
is independent); shipping a disabled `Network account server` row (dead
control).

## Consequences

- `AccountsInterface` joins `interface_names()` (now nine); `main.rs` gains
  `--print-accounts`; `services/system-status` depends on
  `dragonfruit-account-adapter`.
- `apps/settings/AccountsClient` is the pane's seam (`DF_ACCOUNTS_FIXTURE`
  selects the in-process mock); `shell/src/systemstatusclient` and
  `systemstatusmodel` gain the read-only accounts path.
- The Control Center panel stays a non-scrolling fixed surface at 360×1160;
  a fifteenth tile needs a taller nested output (the compositor has no
  pointer-axis forwarding to make a scroll reachable).
- AccountsService authorizes through polkit; a refusal surfaces as `denied`
  without degrading the read state, and the pane keeps working on the last
  read.