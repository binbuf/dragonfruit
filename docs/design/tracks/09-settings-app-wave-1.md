# T-09 — Settings Wave 1: The App and Its Core Panes

> **Track, not a single slice.** This file is the design reference. It is executed as 8 one-session tasks: [T-09.1a](../../tasks/048-t-09.1a-settings-app-shell.md) · [T-09.1b](../../tasks/049-t-09.1b-settings-live-apply.md) · [T-09.2](../../tasks/050-t-09.2-appearance-pane.md) · [T-09.3](../../tasks/051-t-09.3-wallpaper-pane.md) · [T-09.4](../../tasks/052-t-09.4-desktop-dock-pane.md) · [T-09.5](../../tasks/053-t-09.5-displays-basic-pane.md) · [T-09.6a](../../tasks/054-t-09.6a-menu-model-publication.md) · [T-09.6b](../../tasks/055-t-09.6b-absence-matrix-and-wave-captures.md). Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 9 of 17 — the first flagship app |
| **Area** | `apps/settings/` · `shell/` (app chrome) |
| **Depends on** | T-08 |
| **Blocks** | T-14 (menu model), T-17 |
| **Legacy detail** | [legacy/16-settings-app.md](../../tasks/legacy/16-settings-app.md) (Wave 1) · [legacy/15-settingsd-settings-model.md](../../tasks/legacy/15-settingsd-settings-model.md) · [08-settings.md](../08-settings.md) · [10-design-system.md](../10-design-system.md) |

## Demo

```
open Settings from the Dock / the system menu
→ the sidebar, search, and window with our traffic lights and titlebar
→ Appearance: flip dark/light and accent; the whole desktop updates live
→ Wallpaper: pick a per-Space image; the Space background changes live
→ Desktop & Dock: auto-hide, magnification, size, position; the Dock updates
  live
→ Displays-basic: resolution and scale; the output reconfigures
→ every control is live (no Apply, no restart), keyboard-navigable, and
  degrades cleanly when a provider is absent
```

Capture: `docs/captures/t09-settings-wave-1.*`, plus a dark/light and
reduced-motion still.

## Why now

Settings is the first flagship app and the highest-value consumer of
settingsd. Wave 1 is deliberately the four panes that touch the core
experience (appearance, wallpaper, Dock, displays) — the panes that make the
desktop *yours*. It also proves the no-half-panes rule and the design-system
"no hand-rolled chrome" contract before Files repeats the pattern.

## Inherited and reused

- Design-system `Sidebar`, `SettingsRow`, `SettingsGroup`, `SegmentedControl`,
  `Toggle`, `SearchField`, `Toolbar`, `AppWindow`, `TitleBar`, `TrafficLights`.
- T-01's SSD titlebar (Settings is a first-party Tier-1 app: its own titlebar
  must match the compositor SSD exactly, per the legacy T-08 FR-3 reference
  test).
- settingsd (T-08) for Appearance/Wallpaper/Dock; the output API from the
  private protocol for Displays.
- The `--placeholders` Settings window is replaced.

## Scope

### In

1. **App shell**: sidebar, pane search (local), window chrome with traffic
   lights, live-apply everywhere, keyboard navigation, AT-SPI roles.
2. **Appearance**: dark/light, accent, (reduced motion lives here or in
   Accessibility — pick and document).
3. **Wallpaper**: per-Space image selection and fit; binds the T-05 wallpaper
   model.
4. **Desktop & Dock**: auto-hide, magnification, size, position,
   minimize-into-icon, indicators, recents.
5. **Displays-basic**: resolution, scale, rotation. Advanced color/night
   light/VRR are T-15/T-16.
6. **Menu model**: publish the app's menu model (consumed by T-14; until then
   the fixed app menu is enough).
7. **Absent-provider states** and the no-half-panes rule: a pane ships only
   when every control on it is functional.

### Out / explicitly deferred

- Waves 2–3 panes (T-15).
- Global menu integration toggle (T-14).
- Displays advanced (color management, night light, VRR) (T-15/T-16).

## Shell implementation (T-09.1a)

`apps/settings` is the reusable `Dragonfruit.Settings` QML module plus a thin
executable. One ordered catalog singleton, `SettingsPanes`, holds every
reference pane as `{ id, title, icon, description, shipped }`; the sidebar and
local search list only the `shipped` subset, so a row can never appear before
its pane works (the no-half-panes rule). The shell owns the frameless Tier-1
window's design-system `TitleBar`/traffic lights, the `Sidebar` + `SearchField`
navigation, back/forward history, and the header card; a pane unit adds its
body, wires settingsd, and flips its catalog entry to `shipped: true`. See
[ADR 0035](../adr/0035-settings-shell-and-pane-catalog.md).

## Live-apply plumbing (T-09.1b)

The app's panes never touch D-Bus. `apps/settings/SettingsBridge.{h,cpp}` is a
C++ QML singleton registered as **`Settings`** in the `Dragonfruit.Settings`
module, over the shared `dragonfruit-settings-client` library
([ADR 0036](../adr/0036-shared-settings-client-and-qml-singleton.md)) — the
same client the shell's Dock/Theme/compositor-policy controllers link:

- `Settings.values` is a reactive map of every `org.dragonfruit.Settings1`
  key; a binding such as `Settings.values["dock.size"]` re-evaluates on the
  daemon's `Changed` signal, so there is no poll and no restart.
- `Settings.value(key, fallback)`, `Settings.set(key, value)`, `refresh()`, and
  `keys()` are the rest of the surface. `set` applies locally on the same
  event-loop turn and mirrors to settingsd; the daemon's echo is
  de-duplicated.
- A bound control writes on interaction and re-binds to `Settings.values`, so
  a user edit and an external edit converge:

  ```qml
  Toggle {
      onToggled: (checked) => Settings.set("dock.autohide", checked)
      Binding {
          target: dockAutohide
          property: "checked"
          value: Settings.values["dock.autohide"] === true
      }
  }
  ```

- With no daemon on the bus the live client serves the schema defaults and
  keeps writes in memory (the absent-provider state); `DF_SETTINGS_FIXTURE`
  forces the deterministic mock for headless QML tests and captures.
- `--placeholders` (the old shell Settings stand-in) is gone; this shell is
  the app.

**Verification.** `apps/settings/tests/tst_settings_live.cpp` binds a stock
design-system `Toggle` to `accessibility.reduceMotion` and round-trips it
through the fixture, a fake `org.dragonfruit.Settings1` service, and the real
`dragonfruit-settingsd` binary over a private session bus.

## Appearance pane (T-09.2)

`apps/settings/AppearancePane.qml` is the first real pane body. It ships the
two controls the schema owns:

- **Appearance** — a `SegmentedControl` (Light / Dark / Auto) bound to
  `appearance.colorScheme`.
- **Accent color** — a swatch row (Default plus our palette) and a `Custom…`
  popup accepting any `#rrggbb`, bound to `appearance.accent`.

Both use the T-09.1b write-on-interaction / bind-to-`Settings.values` pattern;
`SettingsShell.paneBody` loads the body when its pane id is registered (the
`paneComponent(id)` registry). The pane deliberately omits the reference
pane's Highlight color, Sidebar icon size, wallpaper tinting, and scroll-bar
rows: they have no provider yet (T-15.x), and the no-half-panes rule says a
row appears only when its control works. Reduced motion stays the
Accessibility item (T-15.14).

The design-system `Theme` gained a writable `accentOverride` (ADR
[0037](../adr/0037-accent-override-and-app-local-theme-sync.md)); the shell's
`ThemeBinding` writes it, and the app mirrors the same keys onto its own
`Theme` with `Binding`s in `SettingsShell.qml`, since the shell's writer is in
another process.

**Verification.** `apps/settings/tests/tst_settings_appearance.qml` runs
headless against the `DF_SETTINGS_FIXTURE` mock and asserts each control
changes both the settings key and the app-local `Theme` on the same
event-loop turn.

## Wallpaper pane (T-09.3)

`apps/settings/WallpaperPane.qml` is the second real pane body. It ships:

- **Current wallpaper** — a hero preview (the exact image the compositor
  decodes), the name, the **Show on all Spaces** toggle
  (`wallpaper.showOnAllSpaces`), and a **Fit** segmented control
  (fill/fit/stretch/center, `wallpaper.fit`).
- **Built-in collections** — our own gradients (`Dragonfruit`, `Landscape`),
  rendered once to stable PNGs by `SettingsBridge` and exposed as
  `Settings.wallpaperPresets`; picking a tile writes `wallpaper.source`.
- **Your Photos** — **Add Photo…** through the xdg-desktop-portal
  FileChooser, disabled cleanly when the portal is absent.

The app never touches the compositor: the keys persist in settingsd and the
shell forwards them per Space as `df_workspace.set_wallpaper` (the
`CompositorPolicy` pattern). The compositor keeps each Space's solid color
when only the image changes, so a NULL source returns the Space to its
default (ADR [0038](../adr/0038-wallpaper-pane-settingsd-and-shell-forwarder.md)).

**Verification.** `apps/settings/tests/tst_settings_wallpaper.qml` runs
headless against the `DF_SETTINGS_FIXTURE` mock: the six presets load, each
tile selection, the all-Spaces toggle, and the fit control apply live to the
key, and an external `Settings.set` converges into the preview and selection;
`shell/tests/tst_wallpaperpolicy.cpp` unit-tests the pure key → wire mapping.

## Desktop & Dock pane (T-09.4)

`apps/settings/DesktopDockPane.qml` is the third real pane body. It ships the
`Dock` group for every `dock.*` key from T-08:

- **Size** and **Magnification** — `Slider` rows (captions `Small`/`Large` and
  `Off`/`Small`/`Large`) over `dock.size` and `dock.magnification`.
- **Dock position on screen**, **Minimized window animation**, and **Window
  title bar double-click action** — `Select` rows over `dock.position`,
  `dock.minimizedAnimation`, `dock.titlebarDoubleClick`.
- **Minimize windows into application icon**, **Automatically hide and show
  the Dock**, **Animate opening applications**, **Show indicators for open
  applications**, and **Show suggested and recent apps in Dock** — `Toggle`
  rows.

Every control uses the T-09.1b write-on-interaction / bind-to-`Settings.values`
pattern. There is no new applier: `ShellController::applyDockSettings` already
reacts to a settingsd `Changed` and re-lays-out the Dock, so the pane only
writes keys (the shell side was landed by T-08.2a). `dock.pinned` has no
reference row (macOS reorders by drag) and is absent.

The reference pane's **Desktop & Stage Manager** group is omitted: `Show
items` / `Click wallpaper to show desktop` map to Desktop Reveal and hot
corners, which have no settings schema key yet, so the no-half-panes rule
keeps them off the pane (the provider lands with T-15.5).

`Slider` and `Select` are new design-system components (ADR
[0039](../adr/0039-slider-and-select-design-system-components.md)); each has a
gallery page and per-component tests.

**Verification.** `apps/settings/tests/tst_settings_desktop_dock.qml` runs
headless against the `DF_SETTINGS_FIXTURE` mock: the pane opens with all ten
wired controls, each slider/select/toggle applies live to its key, an external
`Settings.set` converges back into the control, and every row's control is
right-aligned; `design-system/tests/tst_design_system.qml` covers the new
components' keyboard, roles, and token colors.

## Acceptance

- [ ] The demo runs and the captures are committed.
- [ ] Every Wave-1 control applies live within one interaction beat.
- [ ] The Settings titlebar renders identically to the compositor SSD
      reference (the legacy FR-3 diff test stays green).
- [ ] Keyboard walkthrough + AT-SPI roles for the shell and each pane.
- [ ] Absent-provider matrix passes for the shipped panes.
- [ ] `make e2e` and `make soak` stay green; previous demos still pass.

## Test plan

- QML tests per pane against the settingsd mock; routing-table test.
- Nested: live-apply capture; restart persistence.
- Visual: gallery/golden comparison and the SSD/titlebar diff.
- Regression: the T-08 D-Bus flip demo.

## Risks

- **Scope creep** is the named project risk; Wave 1 only, no half-panes.
- **Titlebar drift** between the app QML and the compositor SSD; the shared
  token reference test is the guardrail.
- **Search** is local pane search only; the app-index Spotlight equivalent is
  post-gate.

## Hand-off

- T-14 consumes the published menu model.
- T-15 adds the remaining panes without reworking the shell.
- T-17's loop uses Settings as the first-party app.
