# T-03 — Input Stack, Keymap Conventions, Global Shortcuts, Hot Corners

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `compositor/` (input) |
| **Depends on** | [T-02](02-compositor-core.md) |
| **Blocks** | [T-11](11-mission-control-workspace-ux.md) · [T-12](12-app-switcher.md) · [T-14](14-hot-corners-desktop-background.md) (hot-corner dispatch) · [T-16](16-settings-app.md) (input panes) · [T-28](28-screenshot-recording-ui.md) (keybind) |
| **Estimate** | L |
| **Design docs** | [02-compositor.md](../../design/02-compositor.md) · [03-workspaces.md](../../design/03-workspaces.md) · [04-shell.md](../../design/04-shell.md) |

## Summary

Everything input: libinput handling with touch/tablet as first-class citizens,
xkbcommon keymaps with the Cmd/Option mapping chosen once, the global
shortcut engine, gesture recognition feeding shared progress pipelines, and
hot-corner detection in the compositor's input path.

## Background

The compositor is the only process that sees raw input
([01-architecture.md](../../design/01-architecture.md)). System shortcuts,
gestures, and hot corners must all drive **the same** animation pipelines so
behavior is identical regardless of trigger
([03-workspaces.md](../../design/03-workspaces.md)).

## Scope

### In scope

1. **libinput plumbing**: pointer, keyboard, touch, and tablet events —
   touch and tablet first-class from the start, not a later port.
2. **Gesture recognition**: swipe and pinch recognition lives **in the
   compositor** and feeds the same progress pipelines as keyboard and
   hot-corner triggers — no gesture-only code path.
3. **Keymaps**: xkbcommon; the mapping is fixed **once** here and shared by
   system shortcuts and first-party accelerators:
   - macOS-Cmd role → **Super/Mod4**
   - Option role → **Alt**
   Applications cannot drift from the shell.
4. **Global shortcut engine** (compositor-owned):
   - Owns system shortcuts: workspace switching, Mission Control, app
     switcher, screenshots.
   - Application accelerators admitted **only** through the menu-broker while
     the owning window is focused (feeds
     [T-22](22-global-menu-broker.md)).
   - Sandboxed applications register shortcuts via the portal's
     GlobalShortcuts interface (feeds [T-27](27-portal-backend.md)).
   - **No client grabs keys directly** — the compositor is the sole arbiter.
   - Conflict resolution: system shortcuts take precedence over application
     accelerators; the focused window's menu wins among app accelerators.
5. **Hot corners**: screen-corner triggers (Mission Control, notification
   center, desktop reveal, lock screen) detected in the compositor input
   path and **dispatched to the shell**; identical behavior for pointer,
   gesture, or keyboard triggering.
6. **Configurable input settings surfaced through Settings panes** (data
   model now, UI in T-16): keyboard repeat, per-device pointer acceleration,
   scroll configuration.

### Out of scope

- The shell-side hot-corner *configuration UI* (Settings, T-16).
- Gesture-driven *animations* themselves (T-11) — this ticket delivers the
  progress events they consume.
- Accessibility magnification (compositor screen zoom) — related but
  separable; scheduled with [T-31](31-polish-hardening.md) accessibility
  work per [02-compositor.md](../../design/02-compositor.md).

## Requirements

- FR-1: All libinput device classes deliver events; touch and tablet pass
  basic interaction tests on the DRM backend (multitouch + pen pressure).
- FR-2: Cmd/Super and Option/Alt mapping is defined in exactly one place and
  consumed by shortcut engine + keymap; changing it once changes it
  everywhere.
- FR-3: System shortcuts work: workspace switch (per-display lockstep with
  T-05), Mission Control, app switcher (T-12), screenshot (T-28).
- FR-4: Gesture progress events (0→1, clamped, with velocity) are emitted on
  the same internal path as keyboard/hot-corner triggers — verifiable by one
  test observing identical progress curves from all trigger types.
- FR-5: No Wayland client can install a raw key grab; clients requesting
  grabs outside the sanctioned mechanisms are refused.
- FR-6: Hot-corner dispatch reaches the shell over the private protocol
  (with T-07) with pointer/gesture/keyboard producing one event stream.
- FR-7: Keyboard repeat, pointer acceleration, and scroll settings are
  read/write via the compositor's settings surface with live effect.

## Acceptance criteria

- [ ] A scripted matrix: each system shortcut, gesture, and hot corner fires
      the same compositor event regardless of trigger type.
- [ ] Grab attempts by non-sanctioned clients are logged + refused (test
      with a malicious client).
- [ ] Keymap decision documented in-repo; first-party apps and shell agree
      (verified in T-22/T-18 integration tests).

## Test plan

- Unit: gesture math (clamp, rubber-band, velocity thresholds — shared with
  T-11 commit rules).
- Integration (headless): synthetic libinput events drive shortcuts.
- Hardware: real trackpad gesture quality (three/four-finger swipe, pinch),
  tablet, and multi-key layouts (test at least one non-US layout).

## Risks / open questions

- Gesture thresholds and rubber-banding constants need on-device tuning
  against the commit rules in
  [03-workspaces.md](../../design/03-workspaces.md); parameterize, don't
  hardcode.
- Some devices emit quirky gesture events; keep a device quirk table.
