# ADR 0275 — Appearance groups, provider-backed rows, and the reference accent

| | |
|---|---|
| **Status** | accepted; the card layout and Preview group are superseded by [ADR 0296](0296-appearance-pane-adaptations.md) (T290), keys/owners unchanged |
| **Date** | 2026-11-15 |
| **Task** | T274 |
| **Supersedes** | — |
| **Companions** | [24-system-settings.md](../24-system-settings.md) §4.7, [08-settings.md](../08-settings.md), [settings-business-logic-map.md](../settings-business-logic-map.md), ADR 0164, ADR 0225, ADR 0236 |

## Context

The T-22 review found the Appearance pane was seven one-row cards: the macOS
groups were not legible, the icon/widget style, folder color, and scroll-bar
click rows were missing, and the transparency slider stayed active while
Reduce Transparency owned the rendered result. The token accent was a
brand magenta, so the pane read as an unrelated brand rather than macOS
reference.

The pane also had no honest answer for Linux-only appearance policy. Icon
themes, cursor themes/sizes, window-control layout, material rollout, and
accent-from-wallpaper have different owners and some have no live consumer
yet. A row that writes a key nothing reads is a fake control.

## Decision

1. **Six groups, not a card per row.** Appearance, Color, Icon & widget
   style, Windows and scroll bars, Desktop integration, and Preview.
2. **The default accent is the reviewed reference system blue** (`#007aff`
   light / `#0a84ff` dark). The empty `appearance.accent` value is the
   Multicolor choice; a custom `#rrggbb` still overrides the accent family.
3. **One key, one owner, one consumer set.** `appearance.scrollBarClick`
   feeds `Theme.scrollBarClick`, the default for every `ScrollView`.
   `appearance.folderColor` feeds `Theme.folderColor`; the design-system
   `Icon` replaces the generic folder place artwork with a tinted folder
   glyph in Files and the Dock. `appearance.iconStyle`/`iconTheme` feed the
   Settings process's installed-theme provider; an explicit theme wins over
   the style mapping. Cursor rows use the same provider shape and are
   applied by the compositor at the next session start.
4. **Provider absence is a visible disabled row.** The icon/cursor lists are
   the host's real theme directories. A style with no matching installed
   theme is disabled in the segmented control and rejected with its reason;
   with no provider at all the row is disabled with the reason. No silent
   omission and no fake toggle.
5. **Reduce Transparency wins.** The slider is disabled, the row explains
   the override, and a drag, commit, or queued write is discarded rather
   than written. The Glass resolver remains the one owner of the result.
6. **The preview is the rendered resolver.** It reads `Theme` colours and
   the same `Glass.resolve("menuAttached", …)` output as the live shell; it
   never re-derives scheme, accent, highlight, folder, or material.
7. **Rows without a live consumer are disabled with the owner named.**
   `appearance.windowControls`, `appearance.material`, and
   `appearance.accentFromWallpaper` are reserved schema keys for the
   decoration, Glass-budget, and wallpaper-sampler owners.

## Consequences

- The token accent change touches every accent consumer and the gallery
  goldens; regenerate them with `make gallery-snapshot` rather than
  hand-editing generated tokens.
- The icon-theme consumer is the Settings process's `QIcon` provider. The
  shell/app-index icon lookup is named in the key contract but follows when
  it adopts the same provider; until then the pane says so rather than
  claiming a desktop-wide live change.
- Cursor rows persist immediately and apply at the next session start; the
  row description states that instead of pretending to be a live cursor
  swap.
- `SettingsKeyRow` stays the only row anatomy for keys, so a rejected write
  reverts the control and shows the owner's message inline.
- The three scroll modes remain a `ScrollView` contract (T266); T274 adds
  the pane row and the click policy, not a second scroll model.
