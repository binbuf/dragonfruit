# 0296 — Appearance pane adaptations

## Status

Accepted 2026-11-19. Completes the T290a–T290g rewrite of the Appearance pane.
It refines [ADR 0275](0275-appearance-groups-provider-rows-and-tahoe-accent.md)
(the T274 six-card layout, now history) and keeps every key, owner, and
consumer that ADR recorded. The target is
[25-system-settings-visual-parity.md](../25-system-settings-visual-parity.md)
§5.4.

## Context

T274 grouped Appearance into six cards but still carried a separate Preview
card, a segmented scheme control, text selects for the two colour popups, and
a Linux-only "Desktop integration" card in the main flow. The 2026-11-17
mockup moves Appearance to six cards plus a footer: three visual mode tiles,
one Glass preview, one Theme card, one Windows card, and two scroll-bar radio
cards, with the Linux-only rows behind an Advanced Appearance sheet.

The pane already had the real keys and consumers. The open questions were
presentation boundaries that later tasks could accidentally break: what the
Glass slider actually tints, which icon control owns the installed theme, and
where a colour popup may accept a custom value.

## Decision

- **Three mode tiles, not a segmented control.** Light, Dark, and Auto are
  `SettingsModeTile`s over `appearance.colorScheme`. The tiles are one radio
  group, derive their width from the row, and keep the existing key and the
  compositor/settingsd owner.
- **`appearance.transparency` is the attached-menu Glass tint.** It feeds the
  `menuAttached` resolver preset only. It does not re-tint the Dock, the menu
  bar, or windows, and the row says so. A later global glass-tint key must add
  its own consumer and resolver input; this key does not become one silently.
- **The four icon-style tiles are `appearance.iconStyle`.** Default, Dark,
  Clear, and Tinted are widget/icon styles mapped by the installed-theme
  provider. The installed icon theme itself stays in the Advanced Appearance
  sheet as `appearance.iconTheme`, and an explicit theme still wins over the
  style mapping.
- **The highlight and folder-color popups use the design-system palette.** They
  are single swatch popups (`ColorChoicePopup`) over `appearance.highlight` and
  `appearance.folderColor`. The custom `#rrggbb` entry point is accent-only;
  a custom hex set elsewhere is shown as its hex label and never cleared.
- **The six Linux-only rows keep their owners in the Advanced Appearance
  sheet.** Icon theme, cursor theme, cursor size, window control layout,
  material effects, and accent from wallpaper keep their keys, owners, order,
  enabled state, and disabled-with-reason text. They are reachable through
  the footer's **Advanced Appearance…** action and deep links; the sheet is an
  organization change, not a new owner.
- **The footer opens Accessibility → Display.** **More in Accessibility
  Settings…** requests `accessibility/display` through the shell's section
  hand-off. The pane does not grow a second copy of an Accessibility row.
- **The Windows card and the scroll radios follow the mockup order.** Sidebar
  icon size and wallpaper tint are one card; scroll-bar visibility and
  click behavior are separate cards, each a stacked `SettingsRadioGroup` over
  the existing keys.

## Consequences

- The Glass preview renders `Glass.resolve("menuAttached", …)`; it is still
  the one resolver output, never a second alpha formula. Reduce Transparency
  keeps winning, and the slider is disabled with the reason.
- The Advanced Appearance sheet is a row surface: every sheet row has a
  `SettingsMap` entry and `rowSurface()` answers `"dialog"`, so search, deep
  links, and the row audit still land on it.
- A future global material/glass preference is a new key with a new consumer;
  extending `appearance.transparency` beyond attached menus would contradict
  this record.
- `make t290-settings-appearance-capture` is the repeatable visual evidence;
  the pane's 1×/1.5×/2× fit assertions live in `SettingsScaleCase.qml`.
