# T-15 — System Services and Settings Waves 2–3

> **Track, not a single slice.** This file is the design reference. It is executed as 31 one-session units: [T-15.1a](units/114-t-15.1a-bluetooth-adapter.md) · [T-15.1b](units/115-t-15.1b-bluetooth-pane-and-tile.md) · [T-15.2a](units/116-t-15.2a-storage-and-removable-media-adapter.md) · [T-15.2b](units/117-t-15.2b-storage-and-removable-media-pane-and-tile.md) · [T-15.3a](units/118-t-15.3a-sound-and-routing-adapter.md) · [T-15.3b](units/119-t-15.3b-sound-and-routing-pane-and-tile.md) · [T-15.4a](units/120-t-15.4a-keyboard-mouse-and-trackpad-adapter.md) · [T-15.4b](units/121-t-15.4b-keyboard-mouse-and-trackpad-pane-and-tile.md) · [T-15.5a](units/122-t-15.5a-mission-control-and-hot-corners-adapter.md) · [T-15.5b](units/123-t-15.5b-mission-control-and-hot-corners-pane-and-tile.md) · [T-15.6a](units/124-t-15.6a-battery-and-power-profiles-adapter.md) · [T-15.6b](units/125-t-15.6b-battery-and-power-profiles-pane-and-tile.md) · [T-15.7a](units/126-t-15.7a-notifications-and-focus-adapter.md) · [T-15.7b](units/127-t-15.7b-notifications-and-focus-pane-and-tile.md) · [T-15.8a](units/128-t-15.8a-lock-screen-policy-adapter.md) · [T-15.8b](units/129-t-15.8b-lock-screen-policy-pane-and-tile.md) · [T-15.9a](units/130-t-15.9a-menu-bar-configuration-adapter.md) · [T-15.9b](units/131-t-15.9b-menu-bar-configuration-pane-and-tile.md) · [T-15.10a](units/132-t-15.10a-general-about-and-updates-adapter.md) · [T-15.10b](units/133-t-15.10b-general-about-and-updates-pane-and-tile.md) · [T-15.11a](units/134-t-15.11a-users-and-groups-adapter.md) · [T-15.11b](units/135-t-15.11b-users-and-groups-pane-and-tile.md) · [T-15.12a](units/136-t-15.12a-printers-and-scanners-adapter.md) · [T-15.12b](units/137-t-15.12b-printers-and-scanners-pane-and-tile.md) · [T-15.13a](units/138-t-15.13a-privacy-and-security-adapter.md) · [T-15.13b](units/139-t-15.13b-privacy-and-security-pane-and-tile.md) · [T-15.14a](units/140-t-15.14a-accessibility-adapter.md) · [T-15.14b](units/141-t-15.14b-accessibility-pane-and-tile.md) · [T-15.15a](units/142-t-15.15a-network-advanced-vpn-adapter.md) · [T-15.15b](units/143-t-15.15b-network-advanced-vpn-pane-and-tile.md) · [T-15.16](units/144-t-15.16-absent-daemon-matrix-and-breadth-capture.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 15 of 17 — daily-driver breadth |
| **Area** | remaining `services/` adapters · `apps/settings/` panes · Control Center tiles |
| **Depends on** | T-11, T-13 |
| **Blocks** | T-16, T-17 |
| **Legacy detail** | [legacy/20-system-service-adapters.md](legacy/20-system-service-adapters.md) (remaining adapters) · [legacy/16-settings-app.md](legacy/16-settings-app.md) (Waves 2–3) · [legacy/21-control-center.md](legacy/21-control-center.md) · [07-system-integration.md](../design/07-system-integration.md) |

## Demo

```
Bluetooth: discover, pair, connect a device
Storage: mount/eject a USB drive; it appears in Files and the sidebar
Sound: switch output device, set default routing
Keyboard/Mouse/Trackpad: layout, repeat, acceleration, scroll — applied live
Mission Control / hot corners: configure triggers
Users & Groups, Date & Time, Privacy & Security, Updates: the distro path
Printers: add a printer and see queue state (where CUPS exists)
every pane degrades cleanly when its daemon is absent
```

Capture: `docs/captures/t15-breadth.*` (one walkthrough per subsystem, or a
linked set).

## Why now

This is the long tail of "daily driver": the subsystems that are not on the
30-second loop but are required for real use. It lands after the core loop,
materials, session, and portals because it is breadth, not affordance — and
it must not be allowed to precede them (the roadmap's explicit guardrail).

## Inherited and reused

- The adapter contract, mock pattern, and absent-daemon discipline from T-07.
- The Settings app shell from T-09 (panes plug in; no shell rework).
- The Control Center panel from T-11 (tiles plug in).
- T-08's settingsd host-services provider for the keys that belong there.
- Design-system components; the no-half-panes rule.

## Scope

### In

1. **Remaining adapters**: A2 Bluetooth (BlueZ), A5 power profiles, A6 storage
   (UDisks2/GIO), A7 printing/scanning (CUPS/SANE, minimal IPP first), A8
   secrets (Secret Service — reuse only), A9 date/time, A10 user accounts,
   A12 polkit (already partially in T-13).
2. **Settings Waves 2–3**: Keyboard/Mouse/Trackpad, Mission Control/hot
   corners, Bluetooth, Sound, Battery, Notifications/Focus, Lock Screen
   policy, Menu Bar config, General/About/Updates, Users & Groups, Printers &
   Scanners, Privacy & Security, Accessibility, Storage, Network advanced
   (VPN).
3. **Control Center tiles** for the new adapters.
4. **Live apply everywhere**; no pane requires a restart.
5. **Absent-daemon matrix** for every shipped pane, with the VM masking
   matrix documented.

### Out / explicitly deferred

- Search/Spotlight pane, Internet Accounts (GOA), Screen Time/AI (post-gate
  backlog or not planned).
- Distro-specific package management beyond the provider interface.

## Acceptance

- [ ] The demo runs and the capture set is committed.
- [ ] Every shipped pane fully configures its subsystem (no dead controls, no
      TODO toggles, no external GNOME/KDE dialog escapes).
- [ ] The absent-daemon matrix passes for every shipped pane.
- [ ] Live-apply verified for each pane within one interaction beat.
- [ ] `make e2e` and `make soak` stay green; previous demos still pass.

## Test plan

- Per-adapter contract tests (mock + real-in-VM).
- Per-pane routing-table tests and the masking matrix.
- Nested/session walkthrough captures.

## Risks

- **Scope creep is the named project risk.** The wave order is the guardrail:
  no pane ships half-done, and breadth never delays a loop slice.
- **WirePlumber/CUPS/SANE surface area**; pin and isolate.
- **Privilege boundaries**: everything privileged goes through polkit and the
  T-13 agent.

## Hand-off

- T-16 soaks and packages the finished feature set.
- T-17 reviews the whole surface.
