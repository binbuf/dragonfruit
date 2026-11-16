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
| `gestures.pages` | s | `two-finger` | `off`/`two-finger`/`three-finger` | apps/settings | stored-only: compositor/input gesture recognizer does not read this family yet | Finger count for the page-swipe gesture; off disables it (T280). |
| `gestures.fullscreen` | s | `three-finger` | `off`/`three-finger`/`four-finger` | apps/settings | stored-only: compositor/input gesture recognizer does not read this family yet | Finger count for the full-screen-app swipe; off disables it (T280). |
| `gestures.notifications` | b | false | | apps/settings | stored-only: compositor/input gesture recognizer does not read this family yet | Two-finger swipe from the right edge opens Notification Center (T280). |
| `gestures.showDesktop` | b | true | | apps/settings | stored-only: compositor/input gesture recognizer does not read this family yet | Spread the thumb and three fingers to reveal the desktop (T280). |
| `gestures.appWindows` | b | true | | apps/settings | stored-only: compositor/input gesture recognizer does not read this family yet | Swipe down with three fingers to show the app's windows (T280). |
| `appearance.colorScheme` | s | `auto` | `light`/`dark`/`auto` | settingsd | compositor/window decoration, shell/design-system Theme | Light/dark scheme; auto follows the host style hint. The Control Center dark-mode tile writes `dark`/`light` only (T-11.3b). |
| `appearance.accent` | s | `` (empty) | `#rrggbb` or empty | settingsd | shell/design-system Theme | Accent color override (`#rrggbb`); empty uses the token default. |
| `appearance.transparency` | d | 0.5255 | 0.0–1.0 | apps/settings | shell/design-system Glass, compositor/material | Attached-menu glass transparency, 0 (opaque) to 1 (clearer); the measured midpoint is 0.5255 and is the default for the shipped replacement glass (ADR 0234). |
| `wallpaper.source` | s | `` (empty) | | apps/settings | shell/wallpaper forwarder, compositor/workspace model | Image path for the selected wallpaper; empty falls back to the shipped default (ADR 0094). |
| `wallpaper.fit` | s | `fill` | `fill`/`fit`/`stretch`/`center` | apps/settings | shell/wallpaper forwarder, compositor/workspace model | How the wallpaper image maps onto the output. |
| `wallpaper.showOnAllSpaces` | b | true | | apps/settings | shell/wallpaper forwarder, compositor/workspace model | Apply the selection to every Space, or only the active one. |
| `wallpaper.slideshow` | x | 0 | 0–1440 min | apps/settings | stored-only: the shell has no wallpaper-rotation timer yet | Minutes between automatic wallpaper changes; 0 keeps the selection fixed (T281). |
| `wallpaper.provider` | s | `wikimedia` | | wallpaperd | services/wallpaperd | Primary online wallpaper content provider; `wikimedia` plus the additive NASA Space source (ADR 0240). |
| `wallpaper.providerAutoFetch` | b | true | | wallpaperd | services/wallpaperd | Let the provider refresh its Featured catalogue in the background. |
| `wallpaper.providerLastFetch` | x | 0 | 0– | wallpaperd | services/wallpaperd | Unix seconds of the provider's last successful catalogue fetch; 0 = never. |
| `wallpaper.providerSource` | s | `` (empty) | | wallpaperd | shell/wallpaper forwarder | Fetched Featured default/fallback image path; empty until a catalogue exists. |
| `wallpaper.builtinDefault` | s | `` (empty) | | wallpaperd | shell/wallpaper forwarder, apps/settings | Resolved path of the shipped original Default.jpg; the out-of-box background. |
| `display.scale` | d | 1.0 | 0.5–2.0 | apps/settings | shell/display forwarder, compositor/output | Output scale / scaled-resolution factor; 1.0 is the native mode. |
| `display.rotation` | s | `normal` | `normal`/`90`/`180`/`270` | apps/settings | shell/display forwarder, compositor/output | Output rotation as clock-wise degrees. |
| `display.brightness` | d | 1.0 | 0.0–1.0 | shell/control-center | shell/display forwarder, compositor/output | Output brightness level; 1.0 is full brightness. |
| `accessibility.reduceMotion` | b | false | | settingsd | shell/design-system Theme, compositor/window motion | Global animation policy: collapse motion to instant transitions. |
| `accessibility.reduceTransparency` | b | false | | apps/settings | shell/design-system Glass, compositor/material | Replace sampled glass with a scheme-appropriate opaque plate; wins over the transparency slider. |
| `accessibility.increaseContrast` | b | false | | apps/settings | shell/design-system Glass, compositor/material | Strengthen glass edges and foreground contrast, including when Reduce Transparency is active. |
| `accessibility.textSize` | s | `default` | `small`/`default`/`large`/`extra-large` | apps/settings | shell/design-system Theme | Interface text size: one design-system type scale (small, default, large, extra-large). |
| `accessibility.differentiateWithoutColor` | b | false | | apps/settings | shell/design-system Theme/Toggle | Add a non-colour state cue to controls (the checked toggle carries a check glyph). |
| `accessibility.dimFlashingLights` | b | false | | apps/settings | shell/Dock attention bounce | Reduce the amplitude of attention-seeking flashes and bounces. |
| `accessibility.magnifier` | b | false | | apps/settings | shell/compositor policy, compositor/magnifier | Enable the compositor's whole-output screen magnifier. |
| `accessibility.magnifierZoom` | d | 2.0 | 1.0–8.0 | apps/settings | compositor/magnifier | Magnifier zoom factor, 1.0 (off) to 8.0. |
| `accessibility.magnifierMode` | s | `follow-focus` | `follow-focus`/`follow-caret` | apps/settings | compositor/magnifier | What the magnifier keeps in view: the focused window or its caret. |
| `accessibility.shortcut` | s | `` (empty) | | apps/settings | shell/compositor shortcut engine | Recorded chord that opens the Accessibility pane; empty disables it. |
| `privacy.locationServices` | b | true | | apps/settings | apps/settings Privacy pane (gates per-app location writes) | Desktop-level Location Services master; off disables every per-app location write. |
| `privacy.analytics` | b | false | | apps/settings | apps/settings Privacy pane (stored-only; no collector in this build) | Opt-in analytics preference; off by default and stored only. |
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
| `input.secondaryClick` | s | `right` | `right`/`left` | apps/settings | compositor/input pointer buttons | Which button a secondary click uses: right (default) or left. |
| `input.scrollSpeed` | d | 0.0 | -1.0–1.0 | apps/settings | compositor/input pointer scrolling | Pointer scroll speed, -1.0 (slow) to 1.0 (fast); 0 is neutral. |
| `input.doubleClickSpeed` | x | 400 | 100–900 ms | apps/settings | compositor/input pointer double-click interval | Maximum milliseconds between two clicks for a double-click. |
| `input.haptics` | b | true | | apps/settings | compositor/input trackpad haptics (hardware bridge deferred) | Haptic feedback for a trackpad that supports it. |
| `input.lookUp` | s | `force-click` | `force-click`/`three-finger`/`off` | apps/settings | compositor/input gesture mapping | How a trackpad look-up gesture is triggered; off disables it. |
| `input.sources` | as | `[]` | | apps/settings | shell/compositor keymap | Ordered keyboard layout ids; the first is the active source. Empty follows the host keymap. |
| `input.replacements` | as | `[]` | | apps/settings | input method (T253) | Text replacements as `from\tto` entries for the session input method. |
| `input.dictation` | b | false | | apps/settings | speech-to-text delegate (when installed) | Enable dictation when a speech-to-text service is installed. |
| `input.dictationLanguage` | s | `` (empty) | | apps/settings | speech-to-text delegate (T253) | BCP-47 language id for dictation; empty follows the session locale (T280). |
| `input.dictationMicrophone` | s | `` (empty) | | apps/settings | speech-to-text delegate (T253) | Input device name for dictation; empty uses the default source (T280). |
| `input.dictationShortcut` | s | `` (empty) | | apps/settings | shortcut engine (T245a) | Keyboard shortcut that starts dictation; empty uses the delegate default (T280). |
| `input.dictationAutoPunctuation` | b | false | | apps/settings | speech-to-text delegate (T253) | Let the dictation delegate insert punctuation automatically (T280). |
| `input.mouse.pointerSpeed` | d | 0.0 | -1.0–1.0 | apps/settings | stored-only: compositor/input mouse pointer acceleration bridge pending | Mouse tracking speed, -1.0 (slow) to 1.0 (fast); 0 is neutral (T280). |
| `input.mouse.naturalScroll` | b | true | | apps/settings | stored-only: compositor/input mouse scrolling bridge pending | Natural scrolling for the mouse wheel (T280). |
| `input.mouse.secondaryClick` | s | `right` | `right`/`left` | apps/settings | stored-only: compositor/input mouse buttons bridge pending | Which mouse button a secondary click uses (T280). |
| `input.mouse.smartZoom` | b | true | | apps/settings | stored-only: compositor/input mouse gestures bridge pending | Double-click with one finger to smart-zoom the content under the pointer (T280). |
| `input.mouse.pointerAcceleration` | b | true | | apps/settings | stored-only: compositor/input mouse acceleration bridge pending | Apply pointer acceleration to mouse motion; off uses a flat curve (T280). |
| `input.mouse.leftHanded` | b | false | | apps/settings | stored-only: compositor/input mouse buttons bridge pending | Swap the mouse's primary and secondary buttons (T280). |
| `input.mouse.scrollSpeed` | d | 0.0 | -1.0–1.0 | apps/settings | stored-only: compositor/input mouse scrolling bridge pending | Mouse wheel scroll speed, -1.0 (slow) to 1.0 (fast) (T280). |
| `input.mouse.scrollMethod` | s | `button` | `two-finger`/`edge`/`button` | apps/settings | stored-only: compositor/input mouse scrolling bridge pending | How the mouse scrolls; a wheel reports button scrolling (T280). |
| `input.mouse.doubleClickSpeed` | d | 400.0 | 100–900 ms | apps/settings | stored-only: compositor/input mouse double-click interval bridge pending | Maximum milliseconds between two mouse clicks for a double-click (T280). |
| `input.trackpad.pointerSpeed` | d | 0.0 | -1.0–1.0 | apps/settings | stored-only: compositor/input trackpad pointer acceleration bridge pending | Trackpad tracking speed, -1.0 (slow) to 1.0 (fast); 0 is neutral (T280). |
| `input.trackpad.naturalScroll` | b | true | | apps/settings | stored-only: compositor/input trackpad scrolling bridge pending | Natural scrolling for the trackpad (T280). |
| `input.trackpad.secondaryClick` | s | `right` | `right`/`left` | apps/settings | stored-only: compositor/input trackpad buttons bridge pending | Which button a trackpad secondary click uses (T280). |
| `input.trackpad.clickPressure` | d | 0.5 | 0.0–1.0 | apps/settings | stored-only: compositor/input trackpad pressure bridge pending | Trackpad click pressure, 0.0 (light) to 1.0 (firm) (T280). |
| `input.trackpad.smartZoom` | b | true | | apps/settings | stored-only: compositor/input trackpad gestures bridge pending | Double-tap with two fingers to smart-zoom the content under the pointer (T280). |
| `input.trackpad.pinchZoom` | b | true | | apps/settings | stored-only: compositor/input trackpad gestures bridge pending | Pinch with two fingers to zoom (T280). |
| `input.trackpad.rotate` | b | true | | apps/settings | stored-only: compositor/input trackpad gestures bridge pending | Rotate with two fingers to rotate the content (T280). |
| `power.showPercentage` | b | false | | apps/settings | shell/MenuBar battery item | Show the battery percentage beside the menu-bar glyph. |
| `power.historyRange` | x | 0 | 0–1 | apps/settings | apps/settings Battery pane | Battery usage range: 0 = last 24 hours, 1 = last 10 days. |
| `power.dimOnBattery` | b | false | | apps/settings | shell/display forwarder | Reduce the display brightness while the machine runs on battery. |
| `power.preventSleep` | b | false | | apps/settings | session/idle engine | Keep the session awake by disabling the automatic suspend stage. |
| `idle.dim` | x | 150 | 0–86400 s | apps/settings | session/idle engine | Seconds of inactivity before the screen dims; 0 disables the stage. |
| `idle.blank` | x | 300 | 0–86400 s | apps/settings | session/idle engine | Seconds of inactivity before the screen blanks; 0 disables the stage. |
| `idle.lock` | x | 600 | 0–86400 s | apps/settings | session/idle engine | Seconds of inactivity before the session locks; 0 disables the stage. |
| `idle.suspend` | x | 0 | 0–86400 s | apps/settings | session/idle engine, session/suspend | Seconds of inactivity before the session suspends; 0 disables the stage. |
| `menu.global` | b | true | | apps/settings | shell/MenuBar, services/menu-broker | Show the focused app's menus in the global menu bar; off restores local app menus. |
| `menu.autoHide` | s | `full-screen` | `never`/`always`/`full-screen` | apps/settings | shell/MenuBar, compositor/menu-bar visibility | When the menu bar auto-hides: never, always, or only in full screen (T-15.9b; applied per output by the compositor since T201). |
| `menu.showBackground` | b | true | | apps/settings | shell/MenuBar, compositor/menu-bar veil | Draw the menu bar's background material behind its items; off clears both the QML fill and the compositor veil/panel (T-15.9b/T201). |
| `menu.recentItems` | x | 10 | 0–50 | apps/settings | apps/settings, shell/MenuBar system menu | How many recent applications the system menu's Recent Items submenu remembers (T-15.9b; consumed by T230). |
| `menu.osGlyph` | s | `dragonfruit` | `dragonfruit`/`tux` | apps/settings | shell/MenuBar system menu | The OS menu glyph at the far left of the menu bar: the Dragonfruit mark or the Tux mark (revision 26). |
| `menu.clock.showDate` | b | true | | apps/settings | shell/MenuBar clock | Show the date beside the time in the menu-bar clock (T-15.9b). |
| `menu.clock.showSeconds` | b | false | | apps/settings | shell/MenuBar clock | Display the menu-bar clock with seconds (T-15.9b). |
| `menu.control.wifi` | b | true | | apps/settings | shell/MenuBar status row | Show the Wi-Fi control in the menu bar when its adapter is present (T-15.9b). |
| `menu.control.bluetooth` | b | true | | apps/settings | shell/MenuBar status row | Show the Bluetooth control and its power/device status menu in the menu bar when its adapter is present (T-15.9b/T202). |
| `menu.control.battery` | b | true | | apps/settings | shell/MenuBar status row | Show the Battery control in the menu bar when its adapter is present (T-15.9b). |
| `menu.control.volume` | b | true | | apps/settings | shell/MenuBar status row | Show the Sound control in the menu bar when its adapter is present (T-15.9b). |
| `menu.control.focus` | b | true | | apps/settings | shell/MenuBar status row | Show the Focus control in the menu bar when its service is present (T-15.9b). |
| `menu.control.accessibility` | b | true | | apps/settings | shell/MenuBar status row | Show the Accessibility control in the menu bar when it is available (T-15.9b). |
| `menu.control.clock` | b | true | | apps/settings | shell/MenuBar status row | Show the Date & Time (clock) item in the menu bar (T231). |
| `menu.control.notifications` | b | true | | apps/settings | shell/MenuBar status row | Show the Notifications item in the menu bar when the notification service is present (T231). |
| `menu.control.controlCenter` | b | true | | apps/settings | shell/MenuBar status row | Show the Quick Settings item in the menu bar (T231). |
| `menu.control.focusAlways` | b | false | | apps/settings | shell/MenuBar status row | Show the Focus mark even when no Focus mode is active (T277). |
| `menu.control.volumeWhenActive` | b | false | | apps/settings | shell/MenuBar status row | Hide the Sound mark while the default output is muted (T277). |
| `menu.control.workspaces` | b | true | | apps/settings | shell/MenuBar status row | Show the active workspace indicator in the menu bar (T277). |
| `menu.overflow` | s | `automatic` | `automatic`/`always` | apps/settings | shell/MenuBar status row | When the status row overflows: only when the measured bar is full, or always (T277). |
| `menu.tray.hidden` | as | `[]` | | apps/settings | shell/MenuBar status row | Third-party StatusNotifier item names the user has removed from the menu bar (T277). |
| `sound.alertSound` | s | `Chime` | `Chime`/`Marimba`/`Pulse`/`Woodblock`/`Breeze` | apps/settings | apps/settings (alert playback engine deferred) | Alert sound name selected in the Sound pane. |
| `sound.playEffectsThrough` | s | `output` | `output`/`alerts` | apps/settings | apps/settings (alert playback engine deferred) | Which device sound effects play through: the selected output or the alerts device. |
| `sound.alertVolume` | d | 0.8 | 0.0–1.0 | apps/settings | apps/settings (alert playback engine deferred) | Alert sound volume, 0.0 to 1.0, set by the Sound pane's Alert volume slider. |
| `sound.playOnStartup` | b | true | | apps/settings | apps/settings (startup chime engine deferred) | Play the startup sound when the session begins. |
| `sound.uiEffects` | b | true | | apps/settings | apps/settings (UI sound engine deferred) | Play user-interface sound effects. |
| `sound.volumeFeedback` | b | false | | apps/settings | shell/control-center, apps/settings (UI sound engine deferred) | Play feedback when the output volume is changed. |
| `sound.balance` | d | 0.5 | 0.0–1.0 | apps/settings | apps/settings (balance write deferred) | Output balance from Left (0.0) to Right (1.0); 0.5 is centered. |
| `overview.hotCornerTopLeft` | s | `mission-control` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen`/`launchpad`/`control-center`/`show-dock`/`menu-bar`/`focus-dock` | apps/settings | compositor/input hot corners | Action assigned to the top-left hot corner; `none` disables it. |
| `overview.hotCornerTopRight` | s | `notification-center` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen`/`launchpad`/`control-center`/`show-dock`/`menu-bar`/`focus-dock` | apps/settings | compositor/input hot corners | Action assigned to the top-right hot corner; `none` disables it. |
| `overview.hotCornerBottomLeft` | s | `desktop-reveal` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen`/`launchpad`/`control-center`/`show-dock`/`menu-bar`/`focus-dock` | apps/settings | compositor/input hot corners | Action assigned to the bottom-left hot corner; `none` disables it. |
| `overview.hotCornerBottomRight` | s | `lock-screen` | `none`/`mission-control`/`notification-center`/`desktop-reveal`/`lock-screen`/`launchpad`/`control-center`/`show-dock`/`menu-bar`/`focus-dock` | apps/settings | compositor/input hot corners | Action assigned to the bottom-right hot corner; `none` disables it. |
| `overview.hotCornerTopLeftModifier` | s | `none` | `none`/`command`/`shift`/`option`/`control` | apps/settings | compositor/input hot corners | Modifier held while the pointer enters the top-left corner; `none` needs no modifier. |
| `overview.hotCornerTopRightModifier` | s | `none` | `none`/`command`/`shift`/`option`/`control` | apps/settings | compositor/input hot corners | Modifier held while the pointer enters the top-right corner; `none` needs no modifier. |
| `overview.hotCornerBottomLeftModifier` | s | `none` | `none`/`command`/`shift`/`option`/`control` | apps/settings | compositor/input hot corners | Modifier held while the pointer enters the bottom-left corner; `none` needs no modifier. |
| `overview.hotCornerBottomRightModifier` | s | `none` | `none`/`command`/`shift`/`option`/`control` | apps/settings | compositor/input hot corners | Modifier held while the pointer enters the bottom-right corner; `none` needs no modifier. |
| `overview.switchToAppSpace` | b | true | | apps/settings | compositor/workspace activation | Activating an app window on another Space switches to that Space. |
| `notifications.showPreviews` | s | `when-unlocked` | `always`/`when-unlocked`/`never` | apps/settings | apps/settings (stored policy) | When notification previews show: always, when unlocked, or never. |
| `notifications.showWhenSleeping` | b | false | | apps/settings | apps/settings (stored policy) | Show notification banners while the display is sleeping. |
| `notifications.showWhenLocked` | b | true | | apps/settings | apps/settings (stored policy) | Show notification banners while the screen is locked. |
| `notifications.showWhenMirroring` | b | false | | apps/settings | apps/settings (stored policy) | Show notification banners while mirroring or sharing the display. |
| `notifications.appOptions` | s | `` (empty) | | apps/settings | apps/settings (stored policy) | Per-app notification options the service does not own yet (JSON, keyed by app): lock screen, Notification Center count, previews, grouping (T278). |
| `focus.schedule.enabled` | b | false | | apps/settings | shell/FocusSchedule | Enable the daily Focus schedule (T229a). |
| `focus.schedule.start` | s | `22:00` | | apps/settings | shell/FocusSchedule | Focus schedule start time, 24-hour HH:MM (T229a). |
| `focus.schedule.end` | s | `07:00` | | apps/settings | shell/FocusSchedule | Focus schedule end time, 24-hour HH:MM; an earlier end crosses midnight (T229a). |
| `focus.schedule.days` | as | `[mon, tue, wed, thu, fri, sat, sun]` | | apps/settings | shell/FocusSchedule | Weekdays the Focus schedule runs, `mon`…`sun`; empty means every day (T229a). |
| `focus.shortcut` | s | `` (empty) | | apps/settings | shell shortcuts (registration deferred) | Global shortcut that toggles Focus; empty disables it (T229a). |
| `focus.customProfiles` | as | `[]` | | apps/settings | apps/settings Focus pane | User-added Focus names layered over the service's custom profile (T278). |
| `focus.customProfile` | s | `` (empty) | | apps/settings | apps/settings Focus pane | The active user-added Focus name; empty means the built-in Custom profile (T278). |
| `lock.showUserNameAndPhoto` | b | true | | apps/settings | shell/LockScreen (stored policy) | Show the user name and photo on the lock screen (T-15.8b). |
| `lock.showPasswordHints` | b | false | | apps/settings | shell/LockScreen (stored policy) | Show the password hint on the lock screen (T-15.8b). |
| `lock.showMessageWhenLocked` | b | false | | apps/settings | shell/LockScreen (stored policy) | Show a custom message on the lock screen (T-15.8b). |
| `lock.message` | s | `` (empty) | | apps/settings | shell/LockScreen (stored policy) | The custom lock-screen message set by the `Set...` editor (T-15.8b). |
| `lock.showPowerButtons` | b | true | | apps/settings | shell/LockScreen (stored policy) | Show the Sleep, Restart, and Shut Down buttons on the lock screen (T-15.8b). |
| `lock.passwordHint` | s | `` (empty) | | apps/settings | shell/LockScreen | The hint shown under the password field when password hints are on (T229a). |
| `login.windowStyle` | s | `list` | `list`/`name-and-password` | apps/settings | display manager adapter (not exposed; row disabled) | Login-window style: a user list or name-and-password fields (T229a). |
| `lock.requirePasswordAfter` | x | 300 | -1–86400 | apps/settings | apps/settings Lock Screen pane (derives idle.lock) | Seconds after display-off before the password is required; -1 never, 0 immediately (T278). |
| `printers.defaultPaperSize` | s | `us-letter` | `us-letter`/`us-legal`/`a3`/`a4`/`a5` | apps/settings | apps/settings (stored preference) | The default paper size new print jobs assume (T-15.12b). CUPS and SANE own the queue and scanner state. |
| `network.wifi.askToJoin` | s | `notify` | `off`/`notify`/`ask`/`new` | apps/settings | stored-only: no shell Wi-Fi prompt consumer yet | When to prompt before joining a nearby Wi-Fi network (T225, T271). NetworkManager still owns autoconnect. |
| `network.wifi.askToJoinHotspots` | s | `ask` | `off`/`ask`/`automatic` | apps/settings | stored-only: no shell Wi-Fi prompt consumer yet | When to prompt before joining a captive-portal hotspot (T225, T271). |
| `clock.use24Hour` | b | false | | apps/settings | shell/MenuBar clock | Force the menu-bar clock to 24-hour time instead of the locale format (T226). |
| `updates.auto` | b | false | | apps/settings | dragonfruit-system-status update provider | Check for software updates automatically; never enabled by default (T226). |
| `updates.autoInstall` | b | false | | apps/settings | dragonfruit-system-status update provider | Install the updates a check finds automatically; never enabled by default (T226). |
| `defaults.passwordManager` | s | `` (empty) | | apps/settings | apps/settings Autofill & Passwords provider selection | The installed password manager Autofill delegates to; empty follows the active provider. No credential or secret is stored (T269). |
| `defaults.browser` | s | `` (empty) | | apps/settings | apps/settings default-apps delegate (xdg-settings; settings portal T258) | Desktop id of the default web browser; empty follows the host default. The delegate writes the freedesktop default-apps entry. |
| `locale.calendar` | s | `gregorian` | `gregorian`/`iso`/`buddhist`/`japanese` | apps/settings | apps/settings (date preview) | The calendar system the Date & Time and Language previews use (T226). |
| `locale.temperature` | s | `celsius` | `celsius`/`fahrenheit` | apps/settings | apps/settings (weather and date previews) | The temperature unit the desktop shows (T226). |
| `locale.measurement` | s | `metric` | `metric`/`imperial` | apps/settings | apps/settings (measurement previews) | The measurement system the desktop shows (T226). |
| `locale.dateFormat` | s | `medium` | `short`/`medium`/`long` | apps/settings | apps/settings (date preview) | The date-format length the Language & Region preview uses (T226). |
| `locale.numberFormat` | s | `locale` | `locale`/`decimal`/`comma` | apps/settings | apps/settings (number preview) | How numbers group and separate their decimals (T226). |
| `locale.firstDay` | s | `locale` | `locale`/`sunday`/`monday`/`saturday` | apps/settings | apps/settings (calendar preview) | The first day of the week the desktop calendar uses (T226). |
| `locale.languages` | as | `[]` | | apps/settings | apps/settings (preferred-language order and preview) | The ordered preferred languages the Language & Region pane keeps; the first entry is the system locale the host is asked to apply (T270). |
| `backup.auto` | b | false | | apps/settings | apps/settings Backup pane / backup backend | Run the selected backup backend on its schedule; off by default (T226). |
| `backup.backend` | s | `auto` | `auto`/`restic`/`pika`/`timeshift` | apps/settings | apps/settings Backup pane | The backup backend to prefer; auto detects restic, Pika, or Timeshift (T226). |
| `backup.destination` | s | `` (empty) | | apps/settings | apps/settings Backup pane / backup backend | The destination the selected backup backend writes to (T226). |
| `backup.schedule` | s | `off` | `off`/`hourly`/`daily`/`weekly`/`monthly` | apps/settings | apps/settings Backup pane / backup backend | How often the selected backup backend runs (T226). |
| `settings.lastPane` | s | `` (empty) | | apps/settings | apps/settings shell (last pane) | The last visited System Settings pane id; empty opens General (T263). |
| `settings.window.width` | x | 740 | 720–10000 | apps/settings | apps/settings window | Remembered System Settings window width in points (T263). |
| `settings.window.height` | x | 820 | 480–10000 | apps/settings | apps/settings window | Remembered System Settings window height in points (T263). |
| `settings.window.x` | x | 0 | | apps/settings | apps/settings window | Remembered window x position; used only when `settings.window.positioned` is true (T263). |
| `settings.window.y` | x | 0 | | apps/settings | apps/settings window | Remembered window y position; used only when `settings.window.positioned` is true (T263). |
| `settings.window.positioned` | b | false | | apps/settings | apps/settings window | Whether the remembered x/y position has been set by the user (T263). |
| `appearance.highlight` | s | `` (empty) | | apps/settings | shell/design-system Theme, apps/settings Theme | Selection/highlight colour override (`#rrggbb`); empty uses the token selection (T227). |
| `appearance.sidebarIconSize` | s | `medium` | `small`/`medium`/`large` | apps/settings | apps/settings Sidebar | Sidebar icon size (T227). |
| `appearance.wallpaperTint` | b | true | | apps/settings | shell/design-system Glass, apps/settings Glass | Let glass surfaces take their tint from the wallpaper; off uses the neutral plate (T227). |
| `appearance.scrollBars` | s | `automatic` | `automatic`/`when-scrolling`/`always` | apps/settings | shell/design-system ScrollView, apps/settings ScrollView | When scroll bars show (T227). |
| `appearance.scrollBarClick` | s | `jump-to-next-page` | `jump-to-spot`/`jump-to-next-page` | apps/settings | shell/design-system ScrollView, apps/settings ScrollView | Scroll-bar track click: jump to the clicked spot or the next page (T274). |
| `appearance.folderColor` | s | `` (empty) | | apps/settings | shell/design-system Theme, apps/files Theme | Folder glyph colour override (`#rrggbb`); empty keeps the shipped artwork (T274). |
| `appearance.iconStyle` | s | `default` | `default`/`dark`/`clear`/`tinted` | apps/settings | apps/settings icon-theme provider | Icon and widget style, mapped to an installed icon theme (T274). |
| `appearance.iconTheme` | s | `` (empty) | | apps/settings | apps/settings icon-theme provider, shell/app-index icon lookup | Explicit installed icon theme; empty follows the icon style mapping (T274). |
| `appearance.cursorTheme` | s | `` (empty) | | apps/settings | compositor/cursor theme (next session) | Cursor theme name; the compositor reads it at the next session start (T274). |
| `appearance.cursorSize` | x | 24 | 16–64 | apps/settings | compositor/cursor theme (next session) | Cursor size in logical pixels, 16 to 64; applied at the next session start (T274). |
| `appearance.windowControls` | s | `left` | `left`/`right` | apps/settings | compositor/window decoration, session GTK button-layout | Window control side; the compositor keeps traffic lights on the left today (T274). |
| `appearance.material` | s | `auto` | `auto`/`full`/`reduced`/`minimal` | apps/settings | shell/design-system Glass (degrade budget owns the effective tier) | Material effects preference; auto follows the GPU budget and rollout policy (T274). |
| `appearance.accentFromWallpaper` | b | false | | apps/settings | shell/design-system Theme (wallpaper color sampler) | Derive the accent from the wallpaper when no explicit accent is set (T274). |
| `quickSettings.modules` | as | `[wifi, bluetooth, now-playing, airdrop, stage-manager, screen-mirroring, dark, capture-screen, focus, brightness, volume, airplay-audio, edit-controls]` | | apps/settings | shell/control-center | Ordered module ids on the first Control Center page; empty means the measured default inventory (T235). |
| `quickSettings.menuBarOnly` | as | `[]` | | apps/settings | shell/control-center, shell/MenuBar | Module ids that stay in the menu bar and are hidden from the Control Center panel (T227). |
| `quickSettings.page2` | as | `[]` | | apps/settings | shell/control-center | Ordered module ids on the second Control Center page (T235). |
| `quickSettings.page3` | as | `[]` | | apps/settings | shell/control-center | Ordered module ids on the third Control Center page (T235). |
| `quickSettings.pageCount` | x | 1 | 1–3 | apps/settings | shell/control-center | How many Control Center pages exist (T235). |
| `quickSettings.activePage` | x | 0 | 0–2 | apps/settings | shell/control-center | The Control Center page shown when the panel opens (T235). |
| `quickSettings.perDisplay` | b | false | | apps/settings | shell/control-center | Show Control Center on every display instead of the active display only (T235). |
| `quickSettings.spans` | as | `[]` | | apps/settings | shell/control-center | Per-module measured span overrides, encoded as `id=<span>` where span is 1x1, 2x1, 2x2 or 4x1 (T235). |
| `quickSettings.pageNames` | as | `[]` | | apps/settings | apps/settings Quick Settings pane | User-facing names for the Control Center pages; empty entries fall back to Page N (T277). |
| `search.categories` | as | `[applications, files, folders, actions, calculations, definitions, web]` | | apps/settings | search service (T246) | Result categories the desktop search includes (T227). |
| `search.indexLocations` | as | `[home]` | | apps/settings | search service (T246) | Folders the file index includes; `home` is the user's home directory (T227). |
| `search.shortcut` | s | `Super+Space` | | apps/settings | search service (T246), shell shortcuts | Global keyboard shortcut that opens Search (T227). |
| `search.web` | b | true | | apps/settings | search service (T246) | Offer a web-search result for the query using the default browser (T227). |
| `search.clipboardHistory` | b | true | | apps/settings | stored-only: shell clipboard history always observes; no preference consumer yet | Keep the desktop clipboard history available in Quick Settings (T281). |
| `menu.control.search` | b | true | | apps/settings | shell/MenuBar status row | Show the Search item in the menu bar; activation opens the Applications drawer's search field (T227; consumed by T231). |
| `screenSaver.style` | s | `clock` | `clock`/`none` | apps/settings | shell/LockScreen | Screen-saver style: the lock-screen clock, or no screen saver (T227). |
| `screenSaver.clock` | b | true | | apps/settings | shell/LockScreen | Show the clock on the screen-saver/lock surface (T227). |
| `screenSaver.showDate` | b | true | | apps/settings | shell/LockScreen | Show the date under the screen-saver clock (T227). |
| `screenSaver.showSeconds` | b | false | | apps/settings | shell/LockScreen | Show seconds in the screen-saver clock (T227). |
| `desktop.showItems` | s | `on-desktop` | `on-desktop`/`in-stage` | apps/settings | apps/files desktop process | Show desktop items on the desktop or leave them to Stage Manager (T227). |

## Consumer map

| Consumer | Keys it reads |
|---|---|
| `shell/Dock` (`libs/settings-client/settingsclient.*`, `shell/src/dockmodel.cpp`) | every `dock.*` key |
| `apps/settings` (`apps/settings/SettingsBridge.*`, the QML `Settings` singleton) | every key (Wave-1 panes write through it; T-09.1b) |
| `shell/design-system Theme` (`shell/src/themebinding.*`) | `appearance.colorScheme`, `appearance.accent`, `appearance.highlight`, `appearance.scrollBars`, `accessibility.reduceMotion` |
| `shell/design-system Glass` (`shell/src/themebinding.*`) | `appearance.transparency`, `appearance.wallpaperTint`, `accessibility.reduceTransparency`, `accessibility.increaseContrast` — the same writer keeps the QML resolver in step with the forwarded compositor snapshot (T196/T227) |
| `apps/settings/design-system Theme` (`apps/settings/SettingsShell.qml` bindings, T-09.2) | `appearance.colorScheme`, `appearance.accent`, `appearance.highlight`, `appearance.scrollBars`, `accessibility.reduceMotion` (the app is a separate process, so it mirrors the same keys onto its own `Theme`) |
| `apps/settings/design-system Glass` (`apps/settings/SettingsShell.qml` bindings, T196/T227) | `appearance.transparency`, `appearance.wallpaperTint`, `accessibility.reduceTransparency`, `accessibility.increaseContrast` — the app-local resolver drives both pane previews |
| `apps/settings Sidebar` (`apps/settings/SettingsShell.qml`, T227) | `appearance.sidebarIconSize` — mapped onto the design-system Sidebar icon/category slots |
| `shell/control-center` (`shell/control-center/ControlCenter.qml`, T227/T235/T277) | `quickSettings.modules`, `quickSettings.menuBarOnly`, `quickSettings.page2`, `quickSettings.page3`, `quickSettings.pageNames`, `quickSettings.pageCount`, `quickSettings.activePage`, `quickSettings.perDisplay`, `quickSettings.spans` — the live panel renders the measured grid and the Edit Controls catalogue writes the same keys; menu-bar-only items keep their `menu.control.*` mark. An explicit empty `quickSettings.modules` renders only the Edit Controls footer (T277) |
| `apps/files desktop` (`apps/files/DesktopSurface.qml`, T227) | `desktop.showItems` — `in-stage` hides the Files-owned desktop icon surface |
| `shell/LockScreen` (`shell/lock/LockScreen.qml`, T227) | `screenSaver.style`, `screenSaver.clock`, `screenSaver.showDate`, `screenSaver.showSeconds` — the lock scene's clock presentation; `idle.blank` is the Start After delay |
| `search service (T246)` | `search.categories`, `search.indexLocations`, `search.shortcut`, `search.web` — durable preferences the T246 service reads; the pane disables indexing status/rebuild until it exists |
| compositor motion/input (over `df_toplevel_manager` v5, ADR [0034](design/adr/0034-compositor-policy-via-shell-bridge.md)) | `dock.titlebarDoubleClick`, `dock.minimizedAnimation`, `gestures.enabled`, `gestures.spaceSwitch`, `gestures.missionControl`, `accessibility.reduceMotion`, `appearance.colorScheme`, `input.repeatDelay`, `input.repeatRate` (the other `gestures.*` families are stored-only until the recognizer reads them) |
| compositor appearance/glass (over `df_toplevel_manager` v9 `set_appearance_policy`, T196) | `appearance.colorScheme` (resolved), `appearance.transparency`, `accessibility.reduceTransparency`, `accessibility.increaseContrast` |
| compositor pointer (T-15.4b; the `PointerSettings` half of `compositor/src/input/settings.rs`) | `input.pointerSpeed`, `input.naturalScroll`, `input.tapToClick`, `input.leftHanded`, `input.scrollMethod` — the revision-1 shared fallbacks the compositor still applies; the per-device `input.mouse.*`/`input.trackpad.*` families and the remaining `gestures.*` families are stored-only until the per-device/gesture bridge lands (T280/T282). The keyboard-backlight and emoji-panel bridges remain follow-ups |
| shell/wallpaper forwarder (`shell/src/wallpaperpolicy.*`, `shell/src/shellcontroller.cpp`) | `wallpaper.source`, `wallpaper.fit`, `wallpaper.showOnAllSpaces`, `wallpaper.builtinDefault`, `wallpaper.providerSource` — the effective source (user choice, then shipped default, then fetched fallback, then solid color) is forwarded to the compositor as `df_workspace.set_wallpaper`; `wallpaper.slideshow` is stored-only until a rotation timer lands (T281) |
| shell clipboard history (`shell/src/clipboardhistory.*`) | no settings key yet; `search.clipboardHistory` is marked stored-only until the store exposes an enable/clear policy (T281) |
| shell/display forwarder (`shell/src/displayspolicy.*`, `shell/src/shellcontroller.cpp`) | `display.scale`, `display.rotation`, `display.brightness` — forwarded to the compositor as `df_output.set_scale` / `df_output.set_transform` / `df_output.set_brightness` |
| compositor workspace model | `workspaces.count` (no live owner yet; follow-up) |
| `session/idle engine` (`services/session/src/idle.rs`, ADR [0070](design/adr/0070-idle-timer-engine-and-policy.md)) | `idle.dim`, `idle.blank`, `idle.lock`, `idle.suspend` via `IdlePolicy::from_keys` (the production reader is the future idle service; T-12.5b registers the keys and freezes the contract) |
| `shell/MenuBar` (`shell/src/shellcontroller.cpp`, the `menu.global` toggle, T-14.2b) | `menu.global` — off suppresses the focused app's exported menus in the bar |
| Menu Bar pane (`apps/settings/MenuBarPane.qml`, T-15.9b) | `menu.autoHide`, `menu.showBackground`, `menu.recentItems`, `menu.osGlyph`, `menu.clock.showDate`, `menu.clock.showSeconds`, and the `menu.control.*` toggles (including the T227 `menu.control.search`) — the stored configuration the pane writes; the shell's `applyMenuBarPolicy` applies the clock options, the background, and the OS menu glyph live and forwards `menu.autoHide` to the compositor over `df_layer_surface.set_auto_hide`, `applyStatusItems` gates each status item on its `menu.control.*` key (additive with the adapter's availability), and the recent-items count feeds the system menu's Recent Items submenu (T230; ADR [0135](design/adr/0135-menu-bar-pane-and-tile.md), [0220](design/adr/0220-menu-bar-auto-hide-visibility.md), [0240](design/adr/0240-menu-bar-focus-dbusmenu-and-hide-verbs.md)). T231 adds the registry keys `menu.control.clock`, `menu.control.notifications` and `menu.control.controlCenter` (revision 24) and makes `menu.control.search` live; the pane rows for the new keys landed with T233 and now list Wi-Fi, Bluetooth, Battery, Sound, Search, Notifications, Focus, Date & Time, Accessibility and Quick Settings; the pane also explains that items which do not fit move into the overflow menu rather than being clipped (ADR [0248](design/adr/0248-status-item-registry-and-clock.md), [0250](design/adr/0250-menubar-reveal-motion-and-measured-overflow.md)). T277 adds the per-control options (`menu.control.focusAlways`, `menu.control.volumeWhenActive`, `power.showPercentage`), the live third-party allowance list (`menu.tray.hidden`), the Linux-only `menu.overflow` and `menu.control.workspaces` rows, and Reset; the shell reads them in `applyStatusItems` and `applyMenuBarPolicy` (ADR [0278](design/adr/0278-quick-settings-and-menu-bar-management.md)). |
| compositor/menu-bar visibility (`compositor/src/shell/menubar.rs`, T201) | `menu.autoHide` via `df_layer_surface.set_auto_hide` — owns the per-output conceal/reveal, reservation, input hit area, and veil eligibility; `menu.showBackground` reaches it indirectly through the shell's empty/veil panel region. |
| Notifications/Focus panes (`apps/settings/NotificationsPane.qml`, `apps/settings/FocusPane.qml`, T-15.7b) | `notifications.showPreviews`, `notifications.showWhenSleeping`, `notifications.showWhenLocked`, `notifications.showWhenMirroring` — the stored presentation policy the Notifications pane writes; the Focus mode and per-app allow list are the notification adapter's state, not settingsd keys |
| Mission Control / hot corners (T-15.5b; `compositor/input/hot_corners.rs`) | `overview.hotCornerTopLeft`, `overview.hotCornerTopRight`, `overview.hotCornerBottomLeft`, `overview.hotCornerBottomRight` — the durable corner assignments the Mission Control pane writes, plus the revision-1 `gestures.enabled`/`gestures.spaceSwitch`/`gestures.missionControl` trio the compositor already applies live via `set_input_policy`. Applying an assignment needs the append-only compositor request ADR [0126](design/adr/0126-mission-control-hot-corners-adapter.md) names (deferred). |
| Lock Screen pane (`apps/settings/LockScreenPane.qml`, T-15.8b) | `lock.showUserNameAndPhoto`, `lock.showPasswordHints`, `lock.showMessageWhenLocked`, `lock.message`, `lock.showPowerButtons` — the stored display policy the pane writes; the pane reuses the existing `idle.blank`/`idle.lock` timing keys (session idle engine) and the compositor lock stays compositor-owned (ADR [0132](design/adr/0132-lock-screen-policy-adapter.md)). The lock-screen renderer that consumes the `lock.*` keys is a follow-up. |
| Date & Time pane (`apps/settings/DateTimePane.qml`, T226) | `clock.use24Hour`, `menu.clock.showSeconds`, `menu.clock.showDate` — the durable clock-format keys the pane writes; the shell's `applyMenuBarPolicy` forwards them to the menu-bar clock, and the host adapter owns NTP/time-zone/manual time through `timedatectl`. |
| Software Update sub-pane (`apps/settings/SoftwareUpdatePane.qml`, T269) | `updates.auto`, `updates.autoInstall` — the pane forwards both to the running `dragonfruit-system-status` update provider; both default to false and are never silently enabled. |
| Autofill & Passwords sub-pane (`apps/settings/PasswordsPane.qml`, T269) | `defaults.passwordManager` — the chosen provider id for the delegate; empty means the active provider. Settings never reads or stores a credential. |
| Language & Region pane (`apps/settings/LanguageRegionPane.qml`, T226; T270) | `locale.languages`, `locale.calendar`, `locale.temperature`, `locale.measurement`, `locale.dateFormat`, `locale.numberFormat`, `locale.firstDay` — the ordered preferred languages, the pane's live preview, and the stored format policy; the system locale itself is `localectl`'s. |
| Backup pane (`apps/settings/BackupPane.qml`, T226) | `backup.auto`, `backup.backend`, `backup.destination`, `backup.schedule` — the durable policy the pane writes; the host adapter detects and runs restic/Pika/Timeshift, and `backup.auto` defaults off. |
| System Settings shell/window (`apps/settings/SettingsShell.qml`, `SettingsWindow.qml`, T263) | `settings.lastPane`, `settings.window.width`, `settings.window.height`, `settings.window.x`, `settings.window.y`, `settings.window.positioned` — the shell restores/writes the last pane and the host window restores/writes its normal geometry; a deep link wins over the restored pane. |

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
Featured fallback), otherwise the Space's solid color. The provider's catalogue
now spans Wikimedia Commons plus the NASA Space source (ADR 0240); NASA
thumbnails are cached first and the full original is fetched on demand through
`org.dragonfruit.Wallpaper1.FetchItem`. The shell also reads the provider's live
`BuiltinDefaultSource`/`DefaultSource` properties, so the shipped default still
reaches the compositor when `settingsd` is absent; with the provider absent it
resolves the shipped asset itself. See ADR
[0094](design/adr/0094-bundled-default-wallpaper-and-lazy-cache.md).

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