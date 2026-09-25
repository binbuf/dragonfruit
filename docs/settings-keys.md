# Desktop-settings keys (`org.dragonfruit.Settings1`)

This is the in-repo key schema for the T-08 desktop-settings provider:
every key `dragonfruit-settingsd` owns, its D-Bus type, default,
constraints, and the **owner** (the component allowed to write it) and
**consumer** (the components that read it and react to its `Changed`
signal). The authoritative declaration is `services/settingsd/src/schema.rs`
(`KEYS`, `SCHEMA_VERSION`); this table is kept in lockstep with it by
`services/settingsd/tests/schema_doc.rs`. `dragonfruit-settingsd
--print-keys` renders the same owner/consumer pair for the terminal.

Values are typed D-Bus variants (`b`, `d`, `x`, `s`, `as`), never JSON
strings. The persisted file is
`$XDG_CONFIG_HOME/dragonfruit/settings.json` (fallback
`$HOME/.config/dragonfruit/settings.json`), written only by `settingsd`
(ADR [0031](design/adr/0031-settingsd-persistence-format-and-atomic-writes.md)).

The schema is **additive-only** within the `1` series: a key may be added
(bump `SCHEMA_VERSION`, set its `since`), never renamed or removed. A test
freezes the v1 key set.

| Key | Type | Default | Constraints | Owner | Consumer | Summary |
|---|---|---|---|---|---|---|
| `dock.size` | d | 0.5 | 0.0–1.0 | shell/Dock | shell/Dock | Dock icon scale, 0 (small) to 1 (large). |
| `dock.magnification` | d | 0.5 | 0.0–1.0 | shell/Dock | shell/Dock | Magnification strength under the pointer; 0 disables it. |
| `dock.position` | s | `bottom` | `bottom`/`left`/`right` | shell/Dock | shell/Dock | Screen edge the Dock sits on. |
| `dock.autohide` | b | false | | shell/Dock | shell/Dock | Hide the Dock off-edge until the pointer reaches it. |
| `dock.animateOpening` | b | true | | shell/Dock | shell/Dock | Bounce a Dock tile when it launches an app. |
| `dock.showIndicators` | b | true | | shell/Dock | shell/Dock | Show the running-application dot under a tile. |
| `dock.minimizeIntoTileIcon` | b | false | | shell/Dock | shell/Dock, compositor/window motion | Minimize into the app's tile instead of a separate entry. |
| `dock.minimizedAnimation` | s | `scale` | `genie`/`scale`/`none` | shell/Dock | compositor/window motion | Minimize/restore animation style. |
| `dock.titlebarDoubleClick` | s | `zoom` | `zoom`/`minimize`/`none` | settingsd | compositor/window decoration (SSD) | Action on a titlebar double-click. |
| `dock.showRecentApps` | b | false | | shell/Dock | shell/Dock | Show recent/suggested apps in the Dock. |
| `dock.pinned` | as | `[]` | | shell/Dock | shell/Dock | Ordered desktop ids pinned to the Dock; empty seeds defaults. |
| `workspaces.count` | x | 3 | 1–16 | settingsd | compositor/workspace model, shell | Number of Spaces every output starts with. |
| `gestures.enabled` | b | true | | settingsd | compositor/input | Master switch for trackpad gesture recognition. |
| `gestures.spaceSwitch` | b | true | | settingsd | compositor/input | Horizontal swipe switches Spaces. |
| `gestures.missionControl` | b | true | | settingsd | compositor/input | Vertical swipe opens Mission Control. |
| `appearance.colorScheme` | s | `auto` | `light`/`dark`/`auto` | settingsd | compositor/window decoration, shell/design-system Theme | Light/dark scheme; auto follows the host style hint. |
| `appearance.accent` | s | `` (empty) | `#rrggbb` or empty | settingsd | shell/design-system Theme | Accent color override (`#rrggbb`); empty uses the token default. |
| `wallpaper.source` | s | `` (empty) | | apps/settings | shell/wallpaper forwarder, compositor/workspace model | Image path for the selected wallpaper; empty keeps the solid color. |
| `wallpaper.fit` | s | `fill` | `fill`/`fit`/`stretch`/`center` | apps/settings | shell/wallpaper forwarder, compositor/workspace model | How the wallpaper image maps onto the output. |
| `wallpaper.showOnAllSpaces` | b | true | | apps/settings | shell/wallpaper forwarder, compositor/workspace model | Apply the selection to every Space, or only the active one. |
| `display.scale` | d | 1.0 | 0.5–2.0 | apps/settings | shell/display forwarder, compositor/output | Output scale / scaled-resolution factor; 1.0 is the native mode. |
| `display.rotation` | s | `normal` | `normal`/`90`/`180`/`270` | apps/settings | shell/display forwarder, compositor/output | Output rotation as clock-wise degrees. |
| `display.brightness` | d | 1.0 | 0.0–1.0 | shell/control-center | shell/display forwarder, compositor/output | Output brightness level; 1.0 is full brightness. |
| `accessibility.reduceMotion` | b | false | | settingsd | shell/design-system Theme, compositor/window motion | Global animation policy: collapse motion to instant transitions. |
| `input.repeatDelay` | x | 200 | 0–5000 ms | settingsd | compositor/input keyboard repeat | Milliseconds before a held key begins repeating. |
| `input.repeatRate` | x | 25 | 0–200 Hz | settingsd | compositor/input keyboard repeat | Key repeat rate in keys per second; 0 disables repeat. |

## Consumer map

| Consumer | Keys it reads |
|---|---|
| `shell/Dock` (`libs/settings-client/settingsclient.*`, `shell/src/dockmodel.cpp`) | every `dock.*` key |
| `apps/settings` (`apps/settings/SettingsBridge.*`, the QML `Settings` singleton) | every key (Wave-1 panes write through it; T-09.1b) |
| `shell/design-system Theme` (`shell/src/themebinding.*`) | `appearance.colorScheme`, `appearance.accent`, `accessibility.reduceMotion` |
| `apps/settings/design-system Theme` (`apps/settings/SettingsShell.qml` bindings, T-09.2) | `appearance.colorScheme`, `appearance.accent`, `accessibility.reduceMotion` (the app is a separate process, so it mirrors the same keys onto its own `Theme`) |
| compositor motion/input (over `df_toplevel_manager` v5, ADR [0034](design/adr/0034-compositor-policy-via-shell-bridge.md)) | `dock.titlebarDoubleClick`, `dock.minimizedAnimation`, `gestures.*`, `accessibility.reduceMotion`, `appearance.colorScheme`, `input.repeatDelay`, `input.repeatRate` |
| shell/wallpaper forwarder (`shell/src/wallpaperpolicy.*`, `shell/src/shellcontroller.cpp`) | `wallpaper.source`, `wallpaper.fit`, `wallpaper.showOnAllSpaces` — forwarded to the compositor as `df_workspace.set_wallpaper` |
| shell/display forwarder (`shell/src/displayspolicy.*`, `shell/src/shellcontroller.cpp`) | `display.scale`, `display.rotation`, `display.brightness` — forwarded to the compositor as `df_output.set_scale` / `df_output.set_transform` / `df_output.set_brightness` |
| compositor workspace model | `workspaces.count` (no live owner yet; follow-up) |

`dock.minimizeIntoTileIcon` is Dock entry visibility, not a compositor
key — it is not forwarded over the private protocol. `appearance.accent`
is consumed since T-09.2: the design-system `Theme` gained a writable
`accentOverride` (generated from `design-system/tokens/tokens.json`) and the
shell's `ThemeBinding` and the Settings app's local bindings write it, so
every `Theme.color.accent*` consumer follows live (ADR
[0037](design/adr/0037-accent-override-and-app-local-theme-sync.md)).

## Restart and resync

`settingsd` is independently restartable. `Set` persists **before** it
signals `Changed`, so a consumer reacting to the signal can rely on the
durable file already holding the new value (ADR
[0031](design/adr/0031-settingsd-persistence-format-and-atomic-writes.md)).
Clients never poll: `DbusSettingsClient` subscribes to `Changed` and, when
the well-known name reappears after a daemon restart, calls `GetAll()` to
re-sync; until the snapshot arrives it keeps its last-known values so the
desktop is not visibly reset (T-08 restart behavior, ADR
[0032](design/adr/0032-shell-settings-client.md)). A write attempted while
the daemon is down is local and in-memory only; the next re-sync replaces
it with the daemon's durable state, so an unpersisted write is not silently
kept.