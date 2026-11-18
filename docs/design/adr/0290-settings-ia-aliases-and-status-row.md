# 0290 — Settings aliases are executable, and the update row is not a pane

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-17 |
| **Task** | T285 |
| **Supersedes** | — |

## Context

T285 replaces the 30-pane System Settings catalog with the mockup's 27-entry
sidebar. Three ids leave the top-level catalog (`biometrics`,
`mission-control`, `control-center`), two ids move under an existing pane
(Overview → Desktop & Dock → Workspace Overview, Quick Settings → Menu Bar →
Menu Bar Controls), and the macOS *Internet Accounts* label is kept as the
generic **Online Accounts** title with an `internet-accounts` alias. The
mockup also puts a provider-backed Software Update status row at the top of the
sidebar.

A stale deep link, a search result, or an old internal navigation link must
not open a pane that no longer exists. The removal must also be explainable,
not merely invisible.

## Decision

1. `SettingsPanes.aliases` is the single alias table; `SettingsMap.aliases`
   mirrors it one-for-one. An alias entry carries `{ paneId, subPaneId,
   anchor }`, so a moved pane lands on the exact section or sub-pane its new
   home names.
2. `SettingsPanes.removed`/`SettingsMap.removed` list the three ids that left
   the catalog with a reason and a `refused` flag. `biometrics` has no alias,
   so `resolve()` returns null and `SettingsShell.deepLink()` returns `false`
   after setting `lastDeepLinkError`. It is a rejection, never a silent
   no-op. The debug view shows the same reasons.
3. The Software Update row is `SettingsPanes.sidebarStatus`, a sidebar entry
   with an id (`general/software-update`), a badge, and a resolve target. It
   is not a catalog pane, is not indexed as a pane, and the shell prepends it
   to the first group. T286 renders it from the update provider.
4. The sidebar groups keep empty painted titles and an `accessibleTitle` for
   AT-SPI (`Connectivity`, `Desktop`, `Input`); `spacerAfter` is the only
   spacer marker. Spacers are non-selectable sidebar entries, so the keyboard
   path skips them.
5. VPN becomes always visible as part of the 27-entry IA. T288 keeps the pane
   body's empty state and Add VPN Configuration work; the optional-profile
   gate is removed here because the IA cannot be stable without it.

## Consequences

- A later pane task that needs an old id must add it to the alias table, not
  a second navigation list or a hidden catalog entry.
- Removing a pane means adding a `removed` reason and, when it should still
  land, an alias. “Hidden” is reserved for a real pane body that stays
  reachable through General (Storage).
- Search and the View menu derive from `SettingsMap.panes`/`shippedPanes`, so
  they cannot list a removed pane without a catalog change.
- T292 and T293 add the Workspace Overview section and Menu Bar Controls
  sub-pane; until then the aliases land on the owning pane and the target is
  asserted in the alias table and tests.
