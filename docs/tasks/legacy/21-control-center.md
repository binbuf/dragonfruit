# T-21 — Control Center

| | |
|---|---|
| **Phase** | 4 · System integration |
| **Area** | `shell/control-center/` |
| **Depends on** | [T-09](09-menu-bar.md) · [T-20](20-system-service-adapters.md) · [T-08](08-design-system.md) · [T-25](25-notifications-and-osd.md) (Focus/DND source) |
| **Blocks** | Daily-driver bar (system controls) |
| **Estimate** | L |
| **Design docs** | [04-shell.md](../../design/04-shell.md) · [07-system-integration.md](../../design/07-system-integration.md) |

## Summary

One highly curated panel opened from the menu bar: Wi-Fi, Bluetooth,
Sound, Battery, Displays, Brightness, Focus/DND, Keyboard, Accessibility —
internally reusing the Linux adapters, never talking to hardware directly.

## Background

The user sees one highly curated panel; internally it is sensibly reusing
Linux components ([04-shell.md](../../design/04-shell.md)). The shell is
crashable and restartable; system state arrives via adapters; the shell
never talks to hardware directly.

## Scope

The panel tree (from
[04-shell.md](../../design/04-shell.md)):

```text
Control Center
│
├── Wi-Fi ───────────── NetworkManager adapter (A1)
├── Bluetooth ───────── BlueZ adapter (A2)
├── Sound ───────────── PipeWire / WirePlumber adapter (A3)
├── Battery ─────────── UPower adapter (A4) (+ power profiles A5)
├── Displays ────────── our compositor output API (T-07/T-02)
├── Brightness ──────── compositor / kernel interfaces
├── Focus / DND ─────── our notification service (T-25)
├── Keyboard ────────── compositor / xkbcommon (T-03)
└── Accessibility ───── shell + toolkit services
```

1. **Panel shell**: popover-style chrome surface from the menu-bar entry
   (T-09), design-system components, animated open/close per motion
   tokens, reduced-motion variant.
2. **Tiles per branch** above, each bound to its adapter/compositor
   source:
   - Wi-Fi: toggle, current network, join flow (list + password sheet),
     quick switch of known networks.
   - Bluetooth: toggle, device list, connect/disconnect, battery levels
     where exposed.
   - Sound: volume slider, mute, input/output device switching.
   - Battery: level, charging state, power-profile segment (where
     power-profiles-daemon exists).
   - Displays: quick brightness; deeper display controls live in Settings
     (T-16).
   - Focus/DND: on/off + mode, sourced from the notification service (one
     source of truth shared with the menu bar).
   - Keyboard: backlight where present, input-source quick switch.
   - Accessibility: zoom (compositor magnification), reduced-motion
     toggle, sticky keys quick access — routed per T-16's Accessibility
     pane rules.
3. **Graceful degradation per tile**: adapter "unavailable" → tile hides
   or disables; the panel never shows broken controls.

### Out of scope

- Settings-depth configuration (join-advanced-network dialogs, etc. —
  link out to the corresponding Settings pane; deep links are in scope).
- The menu-bar status items themselves (T-09 renders them; this panel is
  their expanded form).

## Requirements

- FR-1: Every tile reflects its source within one adapter event; no
  polling (idle-desktop budget applies).
- FR-2: Toggling Focus/DND in the panel and menu bar stays consistent —
  one source of truth (notification service), verified by a dual-render
  test.
- FR-3: Wi-Fi join flow works end-to-end through NetworkManager (real
  hardware or NM-in-VM), including failure states.
- FR-4: Absent-daemon matrix passes per tile (A1–A5 masking).
- FR-5: Panel opens within one frame budget of menu-bar click; animation
  interruptible (dismiss mid-open).
- FR-6: All controls operable by keyboard; AT-SPI roles correct.

## Acceptance criteria

- [ ] Full tree ships with its nine branches bound to live sources.
- [ ] Phase-4 contribution: tile-level degradation verified by masking.
- [ ] No Control Center control writes to hardware except via adapters or
      the compositor API (code review gate).

## Test plan

- Mock-adapter UI tests for the full tree; real-daemon matrix in VM.
- Keyboard walkthrough; restart-recovery test (panel state rebuilt from
  sources).

## Risks / open questions

- Tile density vs. small screens — reuse the menu-bar overflow decision
  from T-09.
- Displays-vs-Settings overlap (brightness quick control here, everything
  else there) — keep the split documented to avoid drift.
