# T-16 — Settings Application

| | |
|---|---|
| **Phase** | 3 · Flagship apps |
| **Area** | `apps/settings/` |
| **Depends on** | [T-08](08-design-system.md) · [T-15](15-settingsd-settings-model.md) · [T-20](20-system-service-adapters.md) · [T-07](07-private-shell-protocols.md) (Displays/output API) |
| **Blocks** | Phase-3 exit (Settings fully configures every pane it ships) |
| **Estimate** | XL |
| **Design docs** | [08-settings.md](../design/08-settings.md) · [10-design-system.md](../design/10-design-system.md) · [13-roadmap.md](../design/13-roadmap.md) |

## Summary

The Settings app: one macOS-style information-architecture application
**built entirely in our design system** — not wrappers around GNOME/KDE
dialogs — with every pane's routing explicit, panes shipping in priority
order (core interaction first, complete System-Preferences clone never
allowed to precede the 30-second loop).

## Background

Settings is one of the two flagship apps. The pane list mirrors the macOS
IA (see `../reference/System_Preferences.md`), each pane's routing made
explicit — no pane is "a wrapper around a GNOME dialog"
([08-settings.md](../design/08-settings.md)). The roadmap's guardrail: we
deliberately do **not** spend months cloning every page before the desktop
feels good ([13-roadmap.md](../design/13-roadmap.md)).

## Scope

### In scope

1. **App shell**: SettingsRow/SettingsGroup/Sidebar design-system
   components; search over panes; window with traffic lights (Tier 1
   perfect decoration); publishes a menu model (`MenuBarMenu`) to the
   menu-broker; app preferences (Cmd+,) minimal.
2. **Pane build-out order** (core → breadth):
   - **Wave 1 (vertical slice):** Wi-Fi (list/join via NM adapter),
     volume/battery as part of the core loop's menu-bar surface, Desktop &
     Dock basics (auto-hide, magnification), Displays basic (resolution,
     scale), Appearance (dark/light, accent), Wallpaper (per-Space, binds
     T-14 model).
   - **Wave 2:** Keyboard/Mouse/Trackpad (compositor input API + xkbcommon:
     layouts, repeat, acceleration, scroll), Mission Control/hot corners
     (T-14 config), Bluetooth, Sound (devices, default routing), Battery
     (UPower + power-profiles-daemon where present), Notifications/Focus,
     Lock Screen policy, Menu Bar config.
   - **Wave 3:** General/About/Updates (distro provider), Users & Groups
     (accountsservice + distro provider), Printers & Scanners (CUPS/SANE),
     Privacy & Security (Secret Service, polkit), Accessibility (compositor
     magnification + toolkit/AT-SPI), Storage, Network advanced (VPN),
     Biometrics & Password (fprintd, Secret Service where present).
   - **Deferred (design but don't build):** Search pane (app-index
     Spotlight-equivalent — roadmap "later"); Internet Accounts (GOA) —
     "deferred, not core"; Screen Time/AI — **not planned**.
3. **Pane routing** exactly per
   [08-settings.md](../design/08-settings.md):
   | Pane | Routed to |
   |---|---|
   | Wi-Fi / Network | NetworkManager |
   | Bluetooth | BlueZ |
   | Battery | UPower + power-profiles-daemon |
   | General | settingsd + distro provider |
   | Appearance | settingsd (design-system themes, dark/light, accent) |
   | Accessibility | settingsd → compositor magnification + toolkit/AT-SPI |
   | Desktop & Dock | settingsd → shell + compositor |
   | Displays | Compositor output API (resolution, scaling, rotation, color, night light, VRR) |
   | Wallpaper | settingsd + compositor (per-Space) |
   | Menu Bar | settingsd → shell |
   | Search | app-index (later) |
   | Notifications, Focus | Notification service |
   | Lock Screen | settingsd + compositor lock/idle policy |
   | Privacy & Security | settingsd + host (Secret Service, polkit) |
   | Users & Groups | accountsservice + distro provider |
   | Keyboard, Mouse, Trackpad | Compositor input API + xkbcommon |
   | Sound | PipeWire / WirePlumber |
   | Printers & Scanners | CUPS + SANE |
4. **Empty/absent states**: every pane hides or disables cleanly when its
   daemon is missing (VM-masked verification).
5. **Global menu integration**: menu model publication with live
   enable/disable, and the **Global application menu toggle** from
   [06-global-menu.md](../design/06-global-menu.md) (On → compatible apps
   show `File Edit View Window Help`; non-compatible show app name only;
   Off → first-party apps restore local menus immediately).

### Out of scope

- settingsd itself (T-15), adapters (T-20), compositor input/output APIs
  (T-03/T-02/T-07) — this ticket is the UI and its pane wiring.

## Requirements

- FR-1: Phase-3 exit — **every pane Settings ships, fully configures**: no
  dead controls, no "TODO" toggles, no external-launch escapes to GNOME
  dialogs.
- FR-2: Traffic lights and menu-bar behavior identical to Files (both
  publish menu models and render identical traffic lights — Phase-3 exit
  criterion).
- FR-3: Each pane binds its documented route; a routing table test asserts
  pane↔provider wiring.
- FR-4: Absent-daemon matrix passes for every shipped pane.
- FR-5: Live-apply everywhere: no pane requires restart; changes land via
  settingsd signals/compositor requests within one interaction beat.
- FR-6: Search finds panes and settings by name (local only; the Search
  pane itself is deferred).
- FR-7: Accessibility: full keyboard navigation, AT-SPI roles, respect
  reduced-motion.

## Acceptance criteria

- [ ] Wave-1 panes land with the vertical slice; each later wave ships
      complete panes only (no half-panes).
- [ ] Phase-3 exit review: pane list, routing table, and functionality
      audit passes.
- [ ] Global-menu toggle demonstration recorded (Settings itself proving
      the first-party path).

## Test plan

- Per-pane integration tests against providers (real where possible,
  mocked in CI).
- Absent-daemon matrix per pane.
- UI tests in nested session (restart persistence of window state).

## Risks / open questions

- Scope creep is the named project risk ([14-risks.md](../design/14-risks.md)):
  the wave order is the guardrail; a complete printer page never precedes
  the 30-second loop.
- Displays pane breadth (color management/HDR staging protocol) depends on
  compositor coverage from T-02 — gate per-wave.
