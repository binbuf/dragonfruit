# T-15 — System Services and Settings Waves 2–3

> **Track, not a single slice.** This file is the design reference. It is executed as 16 one-session units: [T-15.1](units/072-t-15.1-bluetooth.md) · [T-15.2](units/073-t-15.2-storage-and-removable-media.md) · [T-15.3](units/074-t-15.3-sound-and-routing.md) · [T-15.4](units/075-t-15.4-keyboard-mouse-and-trackpad.md) · [T-15.5](units/076-t-15.5-mission-control-and-hot-corners.md) · [T-15.6](units/077-t-15.6-battery-and-power-profiles.md) · [T-15.7](units/078-t-15.7-notifications-and-focus.md) · [T-15.8](units/079-t-15.8-lock-screen-policy.md) · [T-15.9](units/080-t-15.9-menu-bar-configuration.md) · [T-15.10](units/081-t-15.10-general-about-and-updates.md) · [T-15.11](units/082-t-15.11-users-and-groups.md) · [T-15.12](units/083-t-15.12-printers-and-scanners.md) · [T-15.13](units/084-t-15.13-privacy-and-security.md) · [T-15.14](units/085-t-15.14-accessibility.md) · [T-15.15](units/086-t-15.15-network-advanced-vpn.md) · [T-15.16](units/087-t-15.16-absent-daemon-matrix-and-breadth-capture.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

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
