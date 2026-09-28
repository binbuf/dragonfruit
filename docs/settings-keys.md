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
| `dock.pinnedFolders` | as | `[]` | | shell/Dock | shell/Dock | Ordered absolute folder paths pinned to the Dock as stacks; empty seeds the Downloads default. |
| `dock.chooserOnHover` | b | false | | shell/Dock | shell/Dock | Open a grouped app's window chooser on hover dwell, and retarget it along the Dock. |
| `dock.minimizeReaction` | b | false | | shell/Dock | shell/Dock | Bounce an app's Dock tile once when one of its windows minimizes; off by default. |
| `workspaces.count` | x | 3 | 1–16 | settingsd | compositor/workspace model, shell | Number of Spaces every output starts with. |
| `gestures.enabled` | b | true | | settingsd | compositor/input | Master switch for trackpad gesture recognition. |
| `gestures.spaceSwitch` | b | true | | settingsd | compositor/input | Horizontal swipe switches Spaces. |
| `gestures.missionControl` | b | true | | settingsd | compositor/input | Vertical swipe opens Mission Control. |
| `appearance.colorScheme` | s | `auto` | `light`/`dark`/`auto` | settingsd | compositor/window decoration, shell/design-system Theme | Light/dark scheme; auto follows the host style hint. The Control Center dark-mode tile writes `dark`/`light` only (T-11.3b). |
| `appearance.accent` | s | `` (empty) | `#rrggbb` or empty | settingsd | shell/design-system Theme | Accent color override (`#rrggbb`); empty uses the token default. |
| `wallpaper.source` | s | `` (empty) | | apps/settings | shell/wallpaper forwarder, compositor/workspace model | Image path for the selected wallpaper; empty keeps the solid color. |
| `wallpaper.fit` | s | `fill` | `fill`/`fit`/`stretch`/`center` | apps/settings | shell/wallpaper forwarder, compositor/workspace model | How the wallpaper image maps onto the output. |
| `wallpaper.showOnAllSpaces` | b | true | | apps/settings | shell/wallpaper forwarder, compositor/workspace model | Apply the selection to every Space, or only the active one. |
| `wallpaper.provider` | s | `wikimedia` | | wallpaperd | services/wallpaperd | Active online wallpaper content provider; `wikimedia` today. |
| `wallpaper.providerAutoFetch` | b | true | | wallpaperd | services/wallpaperd | Let the provider refresh its Featured catalogue in the background. |
| `wallpaper.providerLastFetch` | x | 0 | 0– | wallpaperd | services/wallpaperd | Unix seconds of the provider's last successful catalogue fetch; 0 = never. |
| `wallpaper.providerSource` | s | `` (empty) | | wallpaperd | shell/wallpaper forwarder | Fetched Featured default/fallback image path; empty until a catalogue exists. |
| `wallpaper.builtinDefault` | s | `` (empty) | | wallpaperd | shell/wallpaper forwarder, apps/settings | Resolved path of the shipped original Default.jpg; the out-of-box background. |
| `display.scale` | d | 1.0 | 0.5–2.0 | apps/settings | shell/display forwarder, compositor/output | Output scale / scaled-resolution factor; 1.0 is the native mode. |
| `display.rotation` | s | `normal` | `normal`/`90`/`180`/`270` | apps/settings | shell/display forwarder, compositor/output | Output rotation as clock-wise degrees. |
| `display.brightness` | d | 1.0 | 0.0–1.0 | shell/control-center | shell/display forwarder, compositor/output | Output brightness level; 1.0 is full brightness. |
| `accessibility.reduceMotion` | b | false | | settingsd | shell/design-system Theme, compositor/window motion | Global animation policy: collapse motion to instant transitions. |
| `input.repeatDelay` | x | 200 | 0–5000 ms | settingsd | compositor/input keyboard repeat | Milliseconds before a held key begins repeating. |
| `input.repeatRate` | x | 25 | 0–200 Hz | settingsd | compositor/input keyboard repeat | Key repeat rate in keys per second; 0 disables repeat. |
| `input.pointerSpeed` | d | 0.0 | -1.0–1.0 | apps/settings | compositor/input pointer acceleration | Pointer tracking speed, -1.0 (slow) to 1.0 (fast); 0 is neutral. |
| `input.naturalScroll` | b | true | | apps/settings | compositor/input pointer scrolling | Natural (content follows finger) scrolling for pointers. |
| `input.tapToClick` | b | true | | apps/settings | compositor/input pointer tapping | Tap the trackpad to click. |
| `input.leftHanded` | b | false | | apps/settings | compositor/input pointer handedness | Swap the primary and secondary pointer buttons. |
| `input.scrollMethod` | s | `two-finger` | `two-finger`/`edge`/`button` | apps/settings | compositor/input pointer scrolling | How a trackpad scrolls: two-finger, edge, or button. |
| `input.keyboardBrightness` | d | 0.5 | 0.0–1.0 | apps/settings | compositor/keyboard backlight (hardware bridge deferred) | Keyboard backlight level, 0.0 (off) to 1.0 (bright). |
| `input.adjustBrightnessLowLight` | b | true | | apps/settings | compositor/keyboard backlight (hardware bridge deferred) | Adjust the keyboard backlight automatically in low light. |
| `input.backlightOffAfter` | x | 0 | 0–3600 s | apps/settings | compositor/keyboard backlight (hardware bridge deferred) | Seconds of inactivity before the keyboard backlight turns off; 0 keeps it on. |
| `input.keyboardNavigation` | b | false | | apps/settings | compositor/input keyboard navigation | Move focus between controls with Tab and Shift+Tab. |
| `input.emojiKeyAction` | s | `emoji` | `emoji`/`none` | apps/settings | compositor/input (emoji panel bridge deferred) | Action when the Compose/Super key is pressed: show the emoji panel or nothing. |
| `idle.dim` | x | 150 | 0–86400 s | apps/settings | session/idle engine | Seconds of inactivity before the screen dims; 0 disables the stage. |
| `idle.blank` | x | 300 | 0–86400 s | apps/settings | session/idle engine | Seconds of inactivity before the screen blanks; 0 disables the stage. |
| `idle.lock` | x | 600 | 0–86400 s | apps/settings | session/idle engine | Seconds of inactivity before the session locks; 0 disables the stage. |
| `idle.suspend` | x | 0 | 0–86400 s | apps/settings | session/idle engine, session/suspend | Seconds of inactivity before the session suspends; 0 disables the stage. |
| `menu.global` | b | true | | apps/settings | shell/MenuBar, services/menu-broker | Show the focused app's menus in the global menu bar; off restores local app menus. |
| `sound.alertSound` | s | `Chime` | `Chime`/`Marimba`/`Pulse`/`Woodblock`/`Breeze` | apps/settings | apps/settings (alert playback engine deferred) | Alert sound name selected in the Sound pane. |
| `sound.playEffectsThrough` | s | `output` | `output`/`alerts` | apps/settings | apps/settings (alert playback engine deferred) | Which device sound effects play through: the selected output or the alerts device. |
| `sound.alertVolume` | d | 0.8 | 0.0–1.0 | apps/settings | apps/settings (alert playback engine deferred) | Alert sound volume, 0.0 to 1.0, set by the Sound pane's Alert volume slider. |
| `sound.playOnStartup` | b | true | | apps/settings | apps/settings (startup chime engine deferred) | Play the startup sound when the session begins. |
| `sound.uiEffects` | b | true | | apps/settings | apps/settings (UI sound engine deferred) | Play user-interface sound effects. |
| `sound.volumeFeedback` | b | false | | apps/settings | shell/control-center, apps/settings (UI sound engine deferred) | Play feedback when the output volume is changed. |
| `sound.balance` | d | 0.5 | 0.0–1.0 | apps/settings | apps/settings (balance write deferred) | Output balance from Left (0.0) to Right (1.0); 0.5 is centered. |
| `overview.hotCornerTopLeft` | s | `mission-control` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen` | apps/settings | compositor/input hot corners (apply deferred, ADR 0126) | Action assigned to the top-left hot corner; `none` disables it. |
| `overview.hotCornerTopRight` | s | `notification-center` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen` | apps/settings | compositor/input hot corners (apply deferred, ADR 0126) | Action assigned to the top-right hot corner; `none` disables it. |
| `overview.hotCornerBottomLeft` | s | `desktop-reveal` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen` | apps/settings | compositor/input hot corners (apply deferred, ADR 0126) | Action assigned to the bottom-left hot corner; `none` disables it. |
| `overview.hotCornerBottomRight` | s | `lock-screen` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen` | apps/settings | compositor/input hot corners (apply deferred, ADR 0126) | Action assigned to the bottom-right hot corner; `none` disables it. |
| `notifications.showPreviews` | s | `when-unlocked` | `always`/`when-unlocked`/`never` | apps/settings | apps/settings (stored policy) | When notification previews show: always, when unlocked, or never. |
| `notifications.showWhenSleeping` | b | false | | apps/settings | apps/settings (stored policy) | Show notification banners while the display is sleeping. |
| `notifications.showWhenLocked` | b | true | | apps/settings | apps/settings (stored policy) | Show notification banners while the screen is locked. |
| `notifications.showWhenMirroring` | b | false | | apps/settings | apps/settings (stored policy) | Show notification banners while mirroring or sharing the display. |
| `lock.showUserNameAndPhoto` | b | true | | apps/settings | shell/LockScreen (stored policy) | Show the user name and photo on the lock screen (T-15.8b). |
| `lock.showPasswordHints` | b | false | | apps/settings | shell/LockScreen (stored policy) | Show the password hint on the lock screen (T-15.8b). |
| `lock.showMessageWhenLocked` | b | false | | apps/settings | shell/LockScreen (stored policy) | Show a custom message on the lock screen (T-15.8b). |
| `lock.message` | s | `` (empty) | | apps/settings | shell/LockScreen (stored policy) | The custom lock-screen message set by the `Set...` editor (T-15.8b). |
| `lock.showPowerButtons` | b | true | | apps/settings | shell/LockScreen (stored policy) | Show the Sleep, Restart, and Shut Down buttons on the lock screen (T-15.8b). |

## Consumer map

| Consumer | Keys it reads |
|---|---|
| `shell/Dock` (`libs/settings-client/settingsclient.*`, `shell/src/dockmodel.cpp`) | every `dock.*` key |
| `apps/settings` (`apps/settings/SettingsBridge.*`, the QML `Settings` singleton) | every key (Wave-1 panes write through it; T-09.1b) |
| `shell/design-system Theme` (`shell/src/themebinding.*`) | `appearance.colorScheme`, `appearance.accent`, `accessibility.reduceMotion` |
| `apps/settings/design-system Theme` (`apps/settings/SettingsShell.qml` bindings, T-09.2) | `appearance.colorScheme`, `appearance.accent`, `accessibility.reduceMotion` (the app is a separate process, so it mirrors the same keys onto its own `Theme`) |
| compositor motion/input (over `df_toplevel_manager` v5, ADR [0034](design/adr/0034-compositor-policy-via-shell-bridge.md)) | `dock.titlebarDoubleClick`, `dock.minimizedAnimation`, `gestures.*`, `accessibility.reduceMotion`, `appearance.colorScheme`, `input.repeatDelay`, `input.repeatRate` |
| compositor pointer (T-15.4b; the `PointerSettings` half of `compositor/src/input/settings.rs`) | `input.pointerSpeed`, `input.naturalScroll`, `input.tapToClick`, `input.leftHanded`, `input.scrollMethod` — `settingsd`-owned preferences the compositor applies live; the keyboard-backlight and emoji-panel bridges remain follow-ups |
| shell/wallpaper forwarder (`shell/src/wallpaperpolicy.*`, `shell/src/shellcontroller.cpp`) | `wallpaper.source`, `wallpaper.fit`, `wallpaper.showOnAllSpaces`, `wallpaper.builtinDefault`, `wallpaper.providerSource` — the effective source (user choice, then shipped default, then fetched fallback, then solid color) is forwarded to the compositor as `df_workspace.set_wallpaper` |
| shell/display forwarder (`shell/src/displayspolicy.*`, `shell/src/shellcontroller.cpp`) | `display.scale`, `display.rotation`, `display.brightness` — forwarded to the compositor as `df_output.set_scale` / `df_output.set_transform` / `df_output.set_brightness` |
| compositor workspace model | `workspaces.count` (no live owner yet; follow-up) |
| `session/idle engine` (`services/session/src/idle.rs`, ADR [0070](design/adr/0070-idle-timer-engine-and-policy.md)) | `idle.dim`, `idle.blank`, `idle.lock`, `idle.suspend` via `IdlePolicy::from_keys` (the production reader is the future idle service; T-12.5b registers the keys and freezes the contract) |
| `shell/MenuBar` (`shell/src/shellcontroller.cpp`, the `menu.global` toggle, T-14.2b) | `menu.global` — off suppresses the focused app's exported menus in the bar |
| Notifications/Focus panes (`apps/settings/NotificationsPane.qml`, `apps/settings/FocusPane.qml`, T-15.7b) | `notifications.showPreviews`, `notifications.showWhenSleeping`, `notifications.showWhenLocked`, `notifications.showWhenMirroring` — the stored presentation policy the Notifications pane writes; the Focus mode and per-app allow list are the notification adapter's state, not settingsd keys |
| Mission Control / hot corners (T-15.5b; `compositor/input/hot_corners.rs`) | `overview.hotCornerTopLeft`, `overview.hotCornerTopRight`, `overview.hotCornerBottomLeft`, `overview.hotCornerBottomRight` — the durable corner assignments the Mission Control pane writes, plus the revision-1 `gestures.enabled`/`gestures.spaceSwitch`/`gestures.missionControl` trio the compositor already applies live via `set_input_policy`. Applying an assignment needs the append-only compositor request ADR [0126](design/adr/0126-mission-control-hot-corners-adapter.md) names (deferred). |
| Lock Screen pane (`apps/settings/LockScreenPane.qml`, T-15.8b) | `lock.showUserNameAndPhoto`, `lock.showPasswordHints`, `lock.showMessageWhenLocked`, `lock.message`, `lock.showPowerButtons` — the stored display policy the pane writes; the pane reuses the existing `idle.blank`/`idle.lock` timing keys (session idle engine) and the compositor lock stays compositor-owned (ADR [0132](design/adr/0132-lock-screen-policy-adapter.md)). The lock-screen renderer that consumes the `lock.*` keys is a follow-up. |

`dock.minimizeIntoTileIcon` is Dock entry visibility, not a compositor
key — it is not forwarded over the private protocol. `appearance.accent`
is consumed since T-09.2: the design-system `Theme` gained a writable
`accentOverride` (generated from `design-system/tokens/tokens.json`) and the
shell's `ThemeBinding` and the Settings app's local bindings write it, so
every `Theme.color.accent*` consumer follows live (ADR
[0037](design/adr/0037-accent-override-and-app-local-theme-sync.md)).

The `wallpaper.provider*` keys and `wallpaper.builtinDefault` are the additive
T-18 provider group (revision 10, owner `wallpaperd`): `wallpaper.source` stays
the user override and is never written by the provider. The shell's effective
source is `wallpaper.source` when non-empty, otherwise `wallpaper.builtinDefault`
(the shipped `Default.jpg`), otherwise `wallpaper.providerSource` (the fetched
Featured fallback), otherwise the Space's solid color. The shell also reads the
provider's live `org.dragonfruit.Wallpaper1` `BuiltinDefaultSource`/`DefaultSource`
properties, so the shipped default still reaches the compositor when `settingsd`
is absent; with the provider absent it resolves the shipped asset itself. See
ADR [0094](design/adr/0094-bundled-default-wallpaper-and-lazy-cache.md).

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