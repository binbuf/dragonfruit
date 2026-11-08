# Motion, HUDs, sound, and feedback

| | |
|---|---|
| **Status** | target design |
| **Date** | 2026-11-06 |
| **Scope** | every transition, HUD/OSD, sound, and pointer feedback in the shell |
| **Related** | [22-total-experience.md](22-total-experience.md), [evidence/acceptance](evidence-and-acceptance.md), `macos-design-review/docs/CAPTURE-PLAN.md`, [ADR 0218](adr/0218-dock-motion-raised-cosine-and-tooltip.md) |

## 1. Why a motion language

Motion is how a desktop explains cause and effect. A window that minimizes
toward its Dock icon tells the user where it went; a menu that grows from its
title tells the user what it belongs to; a HUD that fades after a key press
confirms the key worked without stealing focus.

Per-surface durations chosen by eye produce a desktop that feels inconsistent
even when every individual animation is plausible. This document defines one
motion language: one clock, one token set, measured curves, interruption rules,
and reduced-motion behaviour.

## 2. Principles

1. **Motion explains, not decorates.** Every animation has a spatial or causal
   story (origin, destination, parent, state change).
2. **One clock.** All animations run on the compositor's animation clock and
   presentation deadlines (T-21). No private timers.
3. **One token set.** Duration/curve tokens live in the design system and are
   shared by QML and the compositor. A surface may choose a token; it may not
   invent a curve.
4. **Interruptible by default.** Gesture-driven and user-reversible transitions
   are progress-based. A new input retargets from the current visible state.
5. **No overshoot unless measured.** Do not add bounce/spring for delight. Use
   a spring only where the reference fits one better than a bezier.
6. **Reduced motion is a first-class path.** Every animation has a
   reduced-motion variant that preserves the state change and the spatial
   relationship without travel/scale.
7. **Feedback is multimodal.** Visual + optional sound + optional haptic (where
   hardware exists) confirm the same action; none is the only channel.
8. **Transients never block.** HUDs, banners, and tooltips do not take focus or
   block input unless the reference does.

## 3. Measured baseline

These values are already measured and must be preserved:

| Transition | Measured | Source |
|---|---|---|
| Dock magnification envelope | 135 ms smoothstep (`3u² − 2u³`), no overshoot | ADR 0218 |
| Dock magnification spatial profile | raised cosine, R = 125.1 pt, peak 1.217× default / 2.047× diagnostic | ADR 0218 |
| Dock tooltip | ~1.4 s dwell, 128×33 pt, 16.5 pt radius, 8 pt above plate | ADR 0218 |
| Dock launch lift | 15 pt over ~0.3 s (full multi-bounce unresolved) | ADR 0218 |
| Genie minimize/restore | ~0.45–0.5 s each way | ADR 0218 |
| Menu close | ~183 ms, ~5% shrink, translate toward anchor | Liquid Glass report |
| Menu open | unresolved within one 60 fps frame | Liquid Glass report |
| Menu bar veil | no blur; alpha 0.451 at top over black, zero by 95 pt | Liquid Glass report |
| Window open/close | T-02 measured; preserve current tokens | T-02 |
| Workspace switch / overview | T-05 measured; preserve current tokens | T-05 |
| App switcher | T-06 measured; preserve current tokens | T-06 |
| OSD | T-11 measured; preserve current tokens | T-11 |

The expanded campaign must fill the missing rows (menu open, HUDs, Stage
Manager, Quick Look, Search, drawer, lock/unlock, tiling) before those
transitions are tuned.

The 2026-11-08 structural review did **not** measure motion for the Spotlight
Apps drawer, Notification Center, Control Center, or fullscreen title-bar
reveal. Those transitions remain `OPEN` in
[23-macos-structural-surface-review.md](23-macos-structural-surface-review.md)
§9; do not invent a duration or curve for them. The review did confirm the
structural geometry that motion must preserve: the drawer is a Spotlight Apps
grid/list, Notification Center is a right-edge ramp, Control Center modules
grow the outer panel, and Finder accessory bars are content.

## 4. Motion tokens

The design system owns the tokens. Proposed groups:

```text
motion.instant            0 ms
motion.fast               ~100-120 ms
motion.standard           ~160-200 ms
motion.slow               ~250-300 ms
motion.overlay            ~200-250 ms
motion.windowOpen         measured
motion.windowClose        measured
motion.windowMinimize     measured (genie/scale)
motion.windowRestore      measured
motion.windowZoom         measured
motion.windowFullscreen   measured
motion.menuOpen           measured
motion.menuClose          measured (183 ms)
motion.menuBarReveal      measured
motion.dockHover          135 ms (existing)
motion.dockReveal         measured
motion.overviewEnter      measured
motion.overviewExit       measured
motion.workspaceSwitch    measured
motion.switcherOpen       measured
motion.switcherCommit     measured
motion.searchOpen         measured
motion.searchClose        measured
motion.drawerOpen         measured
motion.drawerClose        measured
motion.hudIn              measured
motion.hudOut             measured
motion.notificationIn     measured
motion.notificationOut    measured
motion.lock               measured
motion.unlock             measured
motion.quickLook          measured
motion.stage              measured
```

Curves:

- `smoothstep` — the measured Dock/menu family.
- `easeOut` — entering overlays, HUDs, notifications.
- `easeIn` — exiting/dismissing.
- `easeInOut` — spatial moves where the reference is symmetric.
- `linear` — progress indicators and gesture tracking.
- `spring` — only where a measured spring fit is better; record the
  stiffness/damping and the residual.

No surface uses `Easing.OutBounce` or `OutElastic` unless the reference
explicitly bounces.

## 5. Transition inventory

The full `A-*` inventory is in the capture plan. Implementation priority:

### 5.1 Window lifecycle

| Transition | Spatial story | Required states |
|---|---|---|
| Open | From launch origin (Dock tile) to window frame | normal, reduced motion, interrupted |
| Close | Window frame to launch origin/ghost | normal, reduced motion, interrupted |
| Minimize (genie) | Window frame warps into the Dock icon | normal, reduced motion |
| Minimize (scale) | Window frame scales into the Dock icon | normal, reduced motion |
| Restore | Dock icon to window frame | normal, reduced motion |
| Zoom | Window frame morphs to the zoomed frame | normal, reduced motion |
| Fullscreen enter/exit | Window frame to output frame, chrome reveal | normal, reduced motion |
| Focus/unfocus | Shadow/titlebar state, no travel | instant or short fade |
| App quit | Last window close + Dock indicator update | normal, reduced motion |

### 5.2 Dock

- Magnification enter/track/exit (existing 135 ms; preserve).
- Launch bounce (measured amplitude/hops; unresolved multi-bounce stays
  unresolved).
- Auto-hide reveal/hide (interactive, progress-based).
- Reorder settle (gap close).
- Window chooser open/close (anchored to the tile).
- Tooltip appear/dismiss (dwell + fade).
- Trash empty progress (indeterminate ring, success/failure).
- Folder stack open/close.

### 5.3 Overview / workspaces

- Overview enter/exit (grid scale, wallpaper, chrome).
- Workspace switch (slide/parallax; per-output).
- Drag window between workspaces (ghost/drop/settle).
- Desktop Reveal (windows move aside).
- Add/remove workspace (layout settle).
- Fullscreen workspace enter/exit.

### 5.4 App switcher

- Open (scale/fade).
- Cycle (selection movement, scroll into view).
- Commit (zoom to app/window).
- Cancel (fade).
- Cmd+` window cycle.

### 5.5 Search and drawer

- Search open/close, result update, preview open.
- Drawer open/close, page change, folder open, launch zoom.

### 5.6 Menus and popovers

- Menu open/close (measured 183 ms close).
- Submenu open/close.
- Menu-bar reveal/hide.
- Tooltip.
- Sheet/alert.
- Context menu.

### 5.7 System panels and transients

- Quick Settings open/close, module expand/collapse.
- Notifications banner enter/exit, stack, history open/close.
- HUDs: volume, brightness, keyboard backlight, charging, low battery,
  mirroring, Focus.
- Lock/unlock, login, logout.
- Quick Look open/close, fullscreen.
- Stage Manager enter/exit/switch.
- Window tiling snap.

## 6. Interruption and retargeting

- A new input during an animation retargets from the current value; it never
  restarts from the beginning or snaps.
- A reverse gesture (e.g. Dock leave during enter, overview cancel) uses the
  current progress as the new start.
- A window that closes during its open animation cancels cleanly; no orphaned
  ghost.
- A notification that arrives during another banner's exit stacks or replaces
  per policy; it never overlaps two banners.
- A HUD of the same kind replaces the current value in place; it does not stack
  or restart the timeout from zero unless the value changes.
- Reduced motion collapses travel/scale but preserves the state change and any
  dwell that is required for comprehension.

## 7. HUD/OSD system

### 7.1 Behaviour

- One HUD per kind at a time; a newer value updates in place.
- Position: centered on the active display, near the bottom or top per the
  measured reference; do not cover the focused window's controls.
- Timeout: measured (typically 1–2 s); a new value resets the timer.
- Input: the HUD never takes focus; the key/gesture still reaches the app.
- Per-display: appears on the display where the action occurred (active
  display for global keys).
- Accessibility: the value is announced to AT-SPI; the HUD is not the only
  feedback.

### 7.2 Kinds

| HUD | Trigger | Content | Notes |
|---|---|---|---|
| Volume | Volume keys, menu | Speaker glyph, bar, percentage, mute | Replaces in place |
| Brightness | Brightness keys, Quick Settings | Sun glyph, bar | Per-display |
| Keyboard backlight | Backlight keys | Keyboard glyph, bar | Hardware-dependent |
| Mute | Mute key | Slashed speaker | Distinct from volume 0 |
| Charging | Power plug | Battery glyph, percentage, charging | Enter/exit |
| Low battery | Threshold | Warning glyph, percentage | Alert, not silent |
| Battery full | Charge complete | Battery glyph | Optional |
| Screen casting | Start/stop | Display glyph, device name | Active indicator |
| Focus | Focus toggle | Focus glyph, mode | Replaces DND |
| Wi-Fi off/on | Toggle | Wi-Fi glyph | Optional |
| Bluetooth off/on | Toggle | Bluetooth glyph | Optional |
| Nearby transfer | Start/stop | Device glyph | If adapted |
| Screenshot flash | Screenshot | White flash | Very short |
| Screen recording | Start/stop | Red dot/menu-bar indicator | Persistent while active |
| Camera/mic in use | Privacy | Green/orange dot | Persistent while active |

### 7.3 Acceptance

- HUD appears within one frame of the trigger.
- The value is legible at 1×/1.5×/2×.
- The HUD does not steal focus or block input.
- The timeout and fade match the measured reference.
- Reduced motion removes scale/travel but keeps the value visible for the same
  dwell.
- AT-SPI announces the value.

## 8. Sound language

### 8.1 Principles

- Sound confirms, never startles.
- Original/permissively licensed sounds only.
- Respect `sound.uiEffects`, `sound.volumeFeedback`, and the alert volume.
- Do not play UI sounds when the output is muted or the user disabled them.
- Do not play a sound for every hover; reserve sounds for committed actions,
  errors, and alerts.

### 8.2 Sound set

| Sound | Trigger | Notes |
|---|---|---|
| Startup | Login | Optional; `sound.playOnStartup` |
| Alert | Notification/alert | Per-app setting |
| Error | Failed action | Short, non-punitive |
| Volume feedback | Volume key | Optional; `sound.volumeFeedback` |
| Toggle/click | Button/toggle | Optional; `sound.uiEffects` |
| Trash | Move to Trash / empty | Optional |
| Lock/unlock | Lock/unlock | Optional |
| Notification | Banner | Per-app |
| Screenshot | Capture | Optional |

### 8.3 Acceptance

- Sounds are mixed at a consistent loudness; no clipping.
- The sound does not delay the visual feedback.
- The setting is live; disabling stops the next sound immediately.
- No Apple sounds or close derivatives.

## 9. Cursor and pointer feedback

- Cursor theme: Capitaine or another permissively licensed theme; original
  assets only.
- Sizes: 24/32/48/64 at 1×/1.5×/2×; crisp at each scale.
- Shapes: arrow, text, pointer/hand, resize (8 directions), move, drag copy,
  drag link, busy, wait, help, not-allowed, crosshair, cell, zoom.
- Hotspots are exact; no off-by-one at HiDPI.
- Hide while typing (if enabled); reveal on pointer move.
- Cursor is composited by the compositor; it never appears in screenshots
  unless the capture path includes it.
- Drag feedback: the dragged item/ghost follows the pointer; the cursor shape
  reflects copy/move/link.
- Reduced motion does not affect the cursor; it is direct manipulation.

## 10. Feedback rules

- **Immediate.** A press/hover state changes within one frame.
- **Optimistic.** Rename/trash/new folder show the result immediately; failure
  reverts with an explanation.
- **Progress.** Operations longer than ~200 ms show a progress state; longer
  than ~2 s show a cancel/background option.
- **Completion.** Success is visible but not modal; failure is visible and
  actionable.
- **No silent failure.** Every failed action has a message with a next step.
- **No double feedback.** A banner + HUD + sound for the same event is too
  much; choose the reference's combination.

## 11. Reduced motion

When `accessibility.reduceMotion` is on:

- Window open/close/minimize/restore/zoom/fullscreen: no travel/scale; the
  final state appears immediately, with a short cross-fade if needed for
  comprehension.
- Dock magnification: tracks the pointer directly; no engagement animation.
- Overview: grid appears/disappears without travel; the workspace switch is a
  cut or a short fade.
- App switcher: selection still moves; open/close is instant.
- Menus: open/close is instant; submenus still position correctly.
- HUDs: no scale/travel; the value remains visible for the same dwell.
- Notifications: no slide; fade or cut.
- Cursor: unchanged (direct manipulation).
- Sound: unchanged, but no sound is used as a substitute for motion.

The reduced-motion path is tested for every animation, not just the big ones.

## 12. Performance and scheduling

- All animations use the per-output presentation clock (T-21.2).
- No animation runs on a private 16 ms timer.
- A settled surface produces zero wakeups.
- The compositor and shell each do at most one frame of work per presented
  frame.
- Heavy effects degrade through the material tiers; motion itself does not
  silently drop frames. If the budget is exceeded, reduce the effect tier, not
  the animation duration.
- Record p50/p95/p99 frame times and missed deadlines for each animation in the
  acceptance harness.

## 13. Acceptance

- [ ] Every `A-*` row has a measured duration/curve and a reduced-motion path.
- [ ] Every HUD has a trigger, timeout, position, accessibility announcement,
      and reduced-motion path.
- [ ] Every sound has a source/license and a setting.
- [ ] The cursor is crisp at 1×/1.5×/2× and has correct hotspots.
- [ ] No animation uses an unmeasured bounce/spring.
- [ ] No animation blocks input or steals focus.
- [ ] p95/p99 frame times are within the per-surface budget.
- [ ] The motion token set is shared by QML and the compositor; no duplicated
      durations.

## 14. References

- [ADR 0218 — Dock motion](adr/0218-dock-motion-raised-cosine-and-tooltip.md)
- [macOS structural surface review](23-macos-structural-surface-review.md)
- [Design system — motion](10-design-system.md)
- `macos-design-review/docs/CAPTURE-PLAN.md` — motion protocol
- [Evidence and acceptance](evidence-and-acceptance.md)
- [Total experience design](22-total-experience.md)
