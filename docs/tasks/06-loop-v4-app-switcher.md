# T-06 — Loop v4: The App Switcher

> **Track, not a single slice.** This file is the design reference. It is executed as 3 one-session units: [T-06.1](units/032-t-06.1-app-switcher-state-machine.md) · [T-06.2a](units/033-t-06.2a-switcher-overlay-and-previews.md) · [T-06.2b](units/034-t-06.2b-switcher-commit-and-cycling.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 6 of 17 — completes the 30-second loop |
| **Area** | `compositor/` (switcher state) · `shell/` (switcher overlay) |
| **Depends on** | T-05 |
| **Blocks** | T-17 |
| **Legacy detail** | [legacy/12-app-switcher.md](legacy/12-app-switcher.md) · [04-shell.md](../design/04-shell.md) · [legacy/11-mission-control-workspace-ux.md](legacy/11-mission-control-workspace-ux.md) (recency) |

## Demo

```
Cmd+Tab (hold Cmd) → an overlay shows the live apps in recency order, with
their windows scaled/previewed; Tab cycles, Shift+Tab reverses
→ release Cmd → the selected app's most recent window activates (switching
Space if needed)
→ Cmd+` cycles windows within the app
→ Escape cancels
```

Capture: `docs/captures/t06-app-switcher.*`, plus reduced motion.

## Why now

The loop defined in the roadmap ends with "app switch". With T-01–T-05 done,
every other step of the loop is real; this slice completes it. It also
exercises cross-Space activation, which validates the T-05 hit-test/selection
work from a second direction.

## Inherited and reused

- `WindowModel` recency (`touch_recency`/`recency`) and `apps_by_recency`
  ordering from the legacy T-10 click-tree fix.
- `activate_app` (most-recent window, restore-if-minimized, switch-Space) and
  `select_overview_toplevel`.
- `AppSwitcher` events already emitted by the private protocol
  (`df_toplevel_manager.app_switcher`); the shell currently parses but does
  not consume them.
- T-04's scene-transform pass for live previews; T-05's overlay pattern.

## Scope

### In

1. **Switcher state** (compositor): open on `Cmd+Tab`, cycle on Tab/Shift+Tab,
   reverse with the arrow keys, commit on modifier release, cancel on Escape.
2. **Switcher overlay** (shell): a centered `overlay` chrome surface with one
   card per app in recency order, showing the app's most recent live
   surface(s) scaled via the T-04 pass — never thumbnails; app icon/name as
   the accessible fallback.
3. **Commit** activates the selected app through the existing `activate_app`
   path (cross-Space, restore-if-minimized).
4. **Cmd+` window cycling** within the focused app using the existing window
   chooser semantics.
5. **Reduced motion** variant; interruptible while held.
6. **Keyboard-only, single-hand** operation; accessible names on the cards.

### Out / explicitly deferred

- Per-app badge counts (post-gate backlog).
- Switcher on multi-monitor edge cases beyond lockstep (T-16).

## Acceptance

- [ ] The demo runs and the captures are committed.
- [ ] Cross-Space activation works: switching to an app on another Space
      activates that Space, restores a minimized window, and focuses it.
- [ ] Escape cancels with no focus change; modifier release commits exactly
      once.
- [ ] Reduced-motion variant passes.
- [ ] `make e2e` and `make soak` stay green; T-01…T-05 demos still pass.

## Test plan

- Headless: switcher state machine unit tests (open/cycle/reverse/commit/
  cancel, recency order); protocol conformance for the `app_switcher` events;
  synthetic-input drive of Cmd+Tab.
- Nested: capture review (Wayland + X11 apps, cross-Space, reduced motion).
- Regression: the full T-01→T-05 loop.

## Risks

- **Modifier-release semantics** differ across toolkits; drive the state from
  the compositor's shortcut engine (which already owns the chord), not from a
  client.
- **Preview cost**: only render the selected app's live surface at full
  fidelity; keep the rest as scaled elements within the frame budget.

## Hand-off

- T-17's loop checklist includes the switcher.
- T-14 later supplies richer app identity (icons/names) from the app index.
