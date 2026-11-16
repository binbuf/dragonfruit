# ADR 0282 — Linux-specific settings are one audited map layer

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-16 |
| **Task** | T281 |
| **Companions** | [24-system-settings.md](../24-system-settings.md) §5, [08-settings.md](../08-settings.md), `apps/settings/SettingsMap.qml` |

## Context

System Settings is a macOS-familiar information architecture with deliberate
Linux additions: per-Space wallpaper, tiling, icon/cursor themes, Night Light,
HDR/VRR, firewall/VPN tiers, idle stages, IME, clipboard history, and so on.
The first pane pass shipped many of these rows, but there was no single
contract that proved every Linux-only row had an owner, a consumer or an
explicit `stored-only` mark, an absence path, and a home in a macOS pane.
Without one, a row can drift into a fake live toggle or silently disappear.

## Decision

- `SettingsMap.linuxAdditions` is the executable audit of
  `24-system-settings.md` §5. Each entry names the area, setting, pane, row
  ids, and the task that owns the backend.
- Each referenced row records its owner, key/API, absence kind, and
  **consumer** in the same map. `SettingsMap.validateLinuxAdditions()` fails
  when an entry names a missing row, a row loses its owner/key/absence, or a
  live row claims `stored-only` without being marked (and vice versa).
  `tst_settings_map` runs the validator.
- A Linux-only row lives inside the relevant macOS pane. No separate “Linux”
  pane is allowed.
- A preference with no runtime consumer is `stored-only`: the schema
  `consumer` starts with `stored-only:` and the pane note says what is not
  applied yet. A setting with no usable owner is `disabled-with-reason` and
  names the missing service or tool.
- Per-workspace wallpaper is the inverse of the existing
  `wallpaper.showOnAllSpaces` selection, not a second `wallpaper.perWorkspace`
  key.
- T281's new rows: `wallpaper.slideshow` and `search.clipboardHistory`
  (revision 38, stored-only), `windows.rules` (disabled compositor hand-off),
  `input.method`/`input.composeKey` (disabled IBus/Fcitx5/XKB hand-offs), and
  the live `idle.dim`/`idle.suspend` stages in Lock Screen.

## Consequences

- A later backend task that makes a stored-only row live must change the
  schema consumer, the map's `consumer`, `storedOnly`, and the pane note in
  one change; the audit test then proves the transition.
- The audit is not a second registry: panes still render their own rows and
  the map still owns the row metadata. The audit only checks the Linux
  subset against the existing map.
- Firewall and VPN rows keep T272a's tiers: no first-party nftables/iptables
  rule editor, native WireGuard is a NetworkManager connection type, and an
  unreadable backend is delegated with its exact command.
- A macOS row that Linux cannot own is recorded in the §5.3 review table
  (or the pane's `omit` rows); it is never implied by a dead Linux row.
