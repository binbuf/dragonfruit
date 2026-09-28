# 0135 — The Menu Bar pane and tile are shell-native plus settingsd keys

- **Status:** Accepted (T-15.9b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settingsd-live-settings.md](../tracks/08-settingsd-live-settings.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md),
  ADR [0133](0133-lock-screen-pane-and-tile.md),
  ADR [0134](0134-menu-bar-configuration-adapter.md)

## Context

T-15.9a shipped `dragonfruit-menubar-adapter`, a read-only projection over the
shell menu bar and the menu-broker. The menu bar is **shell-native**: the
shell's `MenuBar` (`shell/menubar/MenuBar.qml`) owns the chrome, the status row,
and the clock, the menu-broker (`services/menu-broker`) resolves the focused
app's menus, and `settingsd` owns the durable preferences both read. There is
no external daemon to wrap and no services-layer host that can read a Wayland
client's private bridge, exactly as Mission Control (T-15.5b) and the Lock
Screen (T-15.8b) found.

The durable home is principle 3's existing split: `settingsd` owns the
preference and the shell applies it live. The existing revision-6 `menu.global`
key already drives the global application-menu toggle, so the pane's new keys
join it in the `Menu` group.

## Decision

The pane and the Control Center tile are one functional unit, and both are
shell-native where the runtime is concerned.

1. **Settingsd keys, revision 16.** The `menu` group gains `menu.autoHide`
   (`never`/`always`/`full-screen`, mirroring `MenuBarAutoHide::id()`),
   `menu.showBackground`, `menu.recentItems` (0–50), `menu.clock.showDate`,
   `menu.clock.showSeconds`, and the six `menu.control.<id>` toggles whose
   suffixes are the T-15.9a adapter's `MenuBarControl::id()` spellings (`wifi`,
   `bluetooth`, `battery`, `volume`, `focus`, `accessibility`). The clock
   defaults mirror `ClockOptions::default()` and the visibility defaults are
   "shown". `docs/settings-keys.md` documents every row and the schema-doc test
   keeps the table honest.
2. **The pane writes keys, never the adapter.** `apps/settings/MenuBarPane.qml`
   has the auto-hide popup, the background toggle, the recent-items popup, the
   `Clock Options...` sheet, and the per-control toggles. Every row applies
   live and persists where the shell consumes it; the rest stays stored policy.
3. **The shell applies what has a runtime.** `applyMenuBarPolicy` sets the
   clock options (`showDate`/`showSeconds`) and the background material on the
   bar live. `applyStatusItems` gates each status item on its
   `menu.control.<id>` key, additive with the adapter's own availability: a
   control shows only when its daemon is present *and* the user wants it. The
   auto-hide mode and the recent-items count have no consumer yet and are stored
   policy (see Consequences).
4. **The tile is projected by the shell.** No services-layer host is added.
   `shell/src/controlcenterpolicy.cpp::menuBarView` maps `menu.autoHide` onto
   the tile view (`Never` / `Always` / `In Full Screen Only`) and the
   `menu-bar` glyph, mirroring `MenuBarSnapshot::label()`. It is pure and
   unit-tested by `tst_controlcenterpolicy`; the QML tile renders it and raises
   `menuBarSettingsRequested`.
5. **Absence is a missing settings daemon.** With no daemon the pane's rows
   stay live on the schema defaults and it shows a one-line note; there is no
   second `present` hide rule because the shell path has no adapter view.
6. **New design-system glyph.** `menu-bar` joins `Icon.qml`, used by the Menu
   Bar sidebar row and the tile.

Rejected: a `dragonfruit-system-status` menu-bar host (no honest live source);
a shell→D-Bus push bridge (inverts ownership); applying auto-hide by mutating
the pinned chrome surface without a compositor input-region contract (a larger
change, deferred).

## Consequences

- The Control Center panel grows from 360×1120 to 360×1200 to fit the new tile;
  the fit test and the T-15.9b capture assert it.
- `menu.autoHide` and `menu.recentItems` persist but do not yet change the bar:
  the shell chrome surface stays pinned and no recent-items consumer exists.
  A later task applies auto-hide (the compositor input-region/hover path) and
  reads the count where recents are listed. The preference is the single source
  of truth until then, as the Lock Screen display keys were.
- The pane omits the Apple-only controls (`AirDrop`, `Screen Mirroring`,
  `Display`) and `Spotlight` (our deferred Search pane), the dead-end
  `Add Controls...` / `Battery Options...` buttons, and the per-control
  `Show When Active` popups (no Linux provider yet), plus the project-wide
  Apple Account/sidebar row and `?` help
  (ADR [0122](0122-tahoe-interface-language-across-chrome.md)). The recent-items
  stepper is drawn as a popup because the design system has no stepper.