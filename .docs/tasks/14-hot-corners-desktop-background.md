# T-14 — Hot Corners Configuration and Desktop Background

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `compositor/` (wallpaper, reveal transition) + `shell/` (corner config UX) |
| **Depends on** | [T-03](03-input-keymaps-shortcuts.md) (hot-corner dispatch) · [T-05](05-spaces-model.md) (per-Space wallpaper) · [T-07](07-private-shell-protocols.md) · [T-09](09-menu-bar.md) · [T-11](11-mission-control-workspace-ux.md) (reveal shares the overview pipeline) · [T-15](15-settingsd-settings-model.md) (persistence keys; interim persistence below) |
| **Blocks** | [T-16](16-settings-app.md) (Desktop & Dock / wallpaper panes) · [T-19](19-desktop-icons.md) (reveal must expose icons later) |
| **Estimate** | M |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [03-workspaces.md](../design/03-workspaces.md) · [08-settings.md](../design/08-settings.md) |

## Summary

User-facing configuration of hot corners, the Desktop Reveal transition, and
the wallpaper-management model that backs the Settings Wallpaper pane. The
MVP ships plain wallpaper (per Space, compositor-rendered); desktop icons
arrive later in Files (T-19) and must keep working with Desktop Reveal.

## Background

Hot corners are configurable triggers (Mission Control, notification
center, desktop reveal, lock screen) detected in the compositor input path
and dispatched to the shell, identical for pointer/gesture/keyboard
([04-shell.md](../design/04-shell.md)). The desktop background is
**compositor-drawn**, part of each Space's scene; the shell draws chrome
only. Desktop Reveal "moves windows aside to expose the background."

## Scope

### In scope

1. **Hot-corner configuration**:
   - Assignment of the four corners (each: none / Mission Control /
      notification center / desktop reveal / lock screen) persisted via
      `settingsd` (T-15) and applied live in the compositor. T-15 lands
      in Phase 3; until then persist under
      `$XDG_CONFIG_HOME/dragonfruit/` in the eventual settingsd key shape
      (the T-10 interim-persistence pattern) so T-15 adopts it without
      migration.
   - Configuration UX (menu-bar/Settings surface, with the Desktop & Dock
     pane in T-16 as the final home).
2. **Desktop Reveal transition**:
   - A progress-based, interruptible window-aside animation sharing the
     overview pipeline machinery (T-11) — windows slide to screen edges
     exposing the background; repeat trigger or Escape restores.
   - Works regardless of whether desktop icons exist
     ([04-shell.md](../design/04-shell.md)); when T-19 lands, the reveal
     exposes icons without changes here.
3. **Wallpaper management model**:
   - Per-Space wallpaper sources (fills/scale modes, solid colors as
     fallback) — rendering lives in T-05; this ticket adds selection state,
     persistence keys (via settingsd), and the surface Settings will bind
     to (Wallpaper pane, T-16).
   - Original artwork only; we ship our own default wallpapers
     ([14-risks.md](../design/14-risks.md) — never Apple's).
4. **Notification-center corner behavior** wiring (opens T-25's center) and
   **lock-screen trigger** wiring (asks compositor to lock via T-26).
   Both arrive in Phase 5; until then these actions are routable against
   stub handlers — absence is a normal state, not an error
   ([07-system-integration.md](../design/07-system-integration.md)).

### Out of scope

- Mission Control itself (T-11), the notification center UI (T-25), the
  lock screen (T-26) — this ticket routes triggers.
- Desktop icons (T-19).
- Wallpaper *picker UI* polish (T-16) — this ticket defines the model.

## Requirements

- FR-1: Corner assignments persist and apply live (no restart) to the
  compositor's hot-corner detection (T-03 path); triggers behave
  identically for pointer, gesture, and keyboard
  ([04-shell.md](../design/04-shell.md)).
- FR-2: All four named actions are routable; unknown/unassigned corners do
  nothing (no accidental triggers).
- FR-3: Desktop Reveal is progress-based, interruptible, reversible;
  reduced-motion variant = windows fade states without translation.
- FR-4: Wallpaper per Space independent; switching Spaces slides each
  background with its Space (T-05 FR-4 verified here for the config
  matrix: image × fill modes + solid color).
- FR-5: Default session ships with tasteful original wallpapers assigned
  to the three vertical-slice Spaces.

## Acceptance criteria

- [ ] Corner config round-trips through settingsd and live-applies.
- [ ] Reveal walkthrough: trigger → aside → restore (Escape, re-trigger,
      and timeout paths) at 60 Hz.
- [ ] Wallpaper per-Space switching verified visually in nested mode.

## Test plan

- UI tests for assignment surface; integration test settingsd→compositor
  apply path.
- Frame-budget test for the reveal transition (shared instrumentation with
  T-11).

## Risks / open questions

- Reveal vs. Mission Control input contention if both trigger near-
  simultaneously — the single overview-state-machine rule (T-11) should
  govern; make reveal a state in the same machine and document.
- Multi-monitor corners: which output's corners? (Answer: each output's own
  corners; document.)
