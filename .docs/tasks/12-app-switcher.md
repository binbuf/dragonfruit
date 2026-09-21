# T-12 — App Switcher (Cmd-Tab Overlay)

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `compositor/` (keybind + state) + `shell/` (overlay) |
| **Depends on** | [T-03](03-input-keymaps-shortcuts.md) · [T-05](05-spaces-model.md) (Space activation, minimized restore) · [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-11](11-mission-control-workspace-ux.md) (Space-switch progress pipeline) · [T-23](23-app-index.md) (identity; stub until it lands) |
| **Blocks** | Phase-2 exit (core loop) |
| **Estimate** | M |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [04-shell.md](../design/04-shell.md) |

## Summary

The Cmd-Tab-style app switcher: **compositor-driven and shell-rendered**,
app-level (not window-level) switching with modifier-cycling windows within
the selected app, animated in and out, workspace-aware.

## Background

The compositor owns the global keybind, the window list, and workspace
awareness; the shell draws the overlay. Per-window selection stays with the
Dock window chooser and Mission Control — the switcher stays app-first, the
macOS mental model ([04-shell.md](../design/04-shell.md)).

## Scope

### In scope

1. **Compositor side**: global keybind (Super/Mod4+Tab per the Cmd=Super
   mapping in [02-compositor.md](../design/02-compositor.md)); recency
   tracking; app-level grouping via app identity — keyed by raw
   `app_id` (Wayland) / `WM_CLASS` (Xwayland) until [T-23](23-app-index.md)
   lands, so this ticket has **no forward dependency on Phase 4** (same
   rule as T-05); app-switcher state exposure over the private protocol
   (T-07 already provisions this interface); activation requests on
   release.
2. **Shell side**: the overlay — running apps by recency with icons,
   selection highlight, animated entry/exit per design-system motion
   tokens, reduced-motion variant.
3. **Interaction model** (from
   [04-shell.md](../design/04-shell.md)):
   ```text
   hold switch key   → overlay lists running apps by recency
   cycle             → apps; modifier cycles windows within the selected app
   release           → focus the selection, animated out of the overlay
   ```
4. **Workspace awareness**: switching to an app whose windows are on
   another Space activates that Space first (compositor round-trip with
   T-05); apps with only minimized windows restore one.

### Out of scope

- Window-level switching UI (Dock chooser T-10, Mission Control T-11).
- Settings for switcher keys (beyond defaults) — later with T-16 Keyboard
  pane.

## Requirements

- FR-1: Hold/cycle/release works with keyboard repeat and multiple
  monitors (overlay on the focused output).
- FR-2: The list is recency-ordered apps, grouped windows; the modifier
  (e.g. Backtick/`~` role) cycles windows within the selected app.
- FR-3: Release activates the selected app's most recent window, switching
  Spaces if needed, minimizing nothing, restoring minimized windows when
  that's all the app has.
- FR-4: Overlay animation in/out follows `motion.*` tokens with
  reduced-motion variants; interaction-latency budget: overlay appears
  within one frame of key press.
- FR-5: The overlay itself never takes keyboard focus away from the
  mechanism (exclusive keyboard mode via chrome protocol, or compositor
  input routing).

## Acceptance criteria

- [ ] Scripted walkthrough: hold→cycle→within-app cycle→release across
      apps on multiple Spaces, including minimized-only apps.
- [ ] Overlay within frame budget; zero dropped frames during
      enter/exit animation.
- [ ] "app switch" step of the 30-second loop passes (Phase-2 exit).

## Test plan

- Headless: recency ordering, grouping via app-index identities.
- Nested: visual + timing tests.

## Risks / open questions

- Key repeat during hold — ensure holding the key doesn't cycle; only
  distinct presses cycle (test with synthetic input).
- Recency across Xwayland apps with weak identity — degrade gracefully
  via app-index heuristics.
