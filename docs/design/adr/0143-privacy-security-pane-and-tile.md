# 0143 — The Privacy & Security pane and tile ride the bridge host

- **Status:** Accepted (T-15.13b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  ADR [0142](0142-privacy-and-security-adapter.md),
  ADR [0141](0141-printers-scanners-pane-and-tile.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.13a landed `dragonfruit-privacy-adapter` (the portal PermissionStore
projection). T-15.13b must ship the Settings pane and the Control Center tile as
one functional unit, with every control applying live. Granting or revoking an
application's access is an explicit action over the store; there is no separate
Linux daemon behind the macOS rows. As with Printers & Scanners (ADR 0141), the
C++/QML side never links the Rust adapter: it decodes the bridge host's JSON
view.

The Control Center panel already held fifteen tiles at the ceiling of its fixed
360×1160 surface; a sixteenth had to fit without clipping and without a
scrolling panel (the compositor forwards no pointer-axis events).

## Decision

- **One new bridge interface, no new daemon.** `services/system-status` serves
  `org.dragonfruit.SystemStatus1.Privacy` at the shared object path. Its
  `PrivacyHost<HostPrivacy>` wraps the T-15.13a adapter and projects one flat
  `privacy` view (every portal category with its summary, the flattened app
  permissions, and the counts) plus the two explicit writes
  (`SetPermission(table, id, app, permission)` and
  `DeletePermission(table, id, app)`). Each write returns the
  `applied`/`denied`/`absent`/`failed` report and the host re-reads; no snapshot
  is invented. The `permission` argument is the stable tristate id
  (`allowed`/`denied`/`ask`); an unknown id is `unset`, whose stored permission
  list is empty.
- **No settingsd key.** The portal PermissionStore is the state: every category
  and every application permission is a host read plus an explicit action, not
  a durable preference. `settingsd` declares nothing for this pane (contrast
  Printers, whose paper size has no CUPS client-tool equivalent).
- **Absence is single-layered.** The view is `unavailable` only when there is
  no session bus or no PermissionStore owner; a store that answers with no
  entry is `available` with `present: false` and every category an honest empty
  row (`None`). The pane shows a one-line absence note when the host is absent;
  the tile hides on `unavailable` or `present: false`.
- **The pane is a flat list of disclosure rows.** Per ADR 0122, Privacy &
  Security is one of the flat-list panes, so each portal category is a row
  (icon, label, secondary summary, trailing chevron) without an inset card.
  Opening a row shows its applications; each application's permission is a
  four-way selector (`Allowed`/`Denied`/`Ask`/`Not Set`) that maps to
  `SetPermission` or `DeletePermission`.
- **The Control Center tile is a read-only summary.** It carries the `privacy`
  glyph, the live app-count label, and a `Privacy & Security Settings…` link.
- **The panel compacts rather than scrolls.** To fit the sixteenth tile the
  tiles' internal gap is compacted from `sm` to `xs`; the fixed surface holds
  every tile. A scrolling panel was rejected (no pointer-axis forwarding).
- **The design system gains one glyph.** `Icon` grows an original `privacy`
  shield-with-check mark for the sidebar row, the pane header, the category
  rows, and the tile.

Rejected: a second permission implementation in the shell (the adapter owns
it); hiding the whole item when one table is empty (an empty category is a
normal row); inventing macOS-only rows (Calendars, Photos, Full Disk Access)
with no Linux host owner; a settingsd preference with no durable meaning.

## Consequences

- `PrivacyInterface` joins `interface_names()` (now eleven); `main.rs` gains
  `--print-privacy`; `services/system-status` depends on
  `dragonfruit-privacy-adapter`.
- `apps/settings/PrivacyClient` is the pane's seam (`DF_PRIVACY_FIXTURE`
  selects the in-process mock); `shell/src/systemstatusclient` and
  `systemstatusmodel` gain the read-only privacy path.
- The Control Center panel stays a non-scrolling fixed surface at 360×1160.
- The dialog captures its row list when it opens and re-reads the live state
  through Bindings, so a permission write converges without rebuilding the
  `Select` that is mid-interaction.
- The Secret Service (passkeys) and polkit (App Management) host services the
  reference also names remain out of scope; a later task may add a read-only
  Secret Service projection as its own row.