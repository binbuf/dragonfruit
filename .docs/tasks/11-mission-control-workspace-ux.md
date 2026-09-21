# T-11 — Mission Control & Workspace-Switching UX

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `compositor/` (overview state machine) + `shell/` (strip/chrome) |
| **Depends on** | [T-03](03-input-keymaps-shortcuts.md) · [T-05](05-spaces-model.md) · [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-09](09-menu-bar.md) · [T-33](33-compositor-effects-materials.md) (scale/clip/blur transforms) |
| **Blocks** | Phase-2 exit (zero-dropped-frame loop) · [T-12](12-app-switcher.md) (cross-Space activation) · [T-14](14-hot-corners-desktop-background.md) (Desktop Reveal shares the pipeline) · [T-34](34-mvp-vertical-slice-gate.md) · [T-31](31-polish-hardening.md) |
| **Estimate** | XL |
| **Design docs** | [03-workspaces.md](../design/03-workspaces.md) · [10-design-system.md](../design/10-design-system.md) · [ROADMAP.md](../ROADMAP.md) |

## Summary

The single overview state machine and its animations: Mission Control
transforming **live window surfaces** (never thumbnails), gesture-progress
commit rules (one rule for every trigger), explicit hit-testing transfer, the
workspace strip, minimized-window bottom strip, and the workspace-switch
animation as a continuous reversible progress pipeline.

## Background

Mission Control does not screen-scrape: the compositor keeps rendering the
real surfaces while applying scale, translation, clipping, blur/shadow, and
workspace transformations ([03-workspaces.md](../design/03-workspaces.md)).
This is the flagship "because we own the compositor" feature and the core of
the 30-second interaction loop
([ROADMAP.md](../ROADMAP.md)).

## Delivery slices (MVP order)

The loop needs **workspace switching** before it needs the Mission Control
overview. Both are the same state machine (the one-machine rule below), so
they are delivered as slices of one ticket, not split into a second pipeline:

- **Slice A — workspace-switch pipeline (front-load for the loop).** Adjacent
  Space switching as a progress pipeline: live surfaces + per-Space wallpaper
  repositioned with scale/translation/clip/blur (T-05 scene mechanics),
  gesture/keyboard triggers, clamp/rubber-band/velocity commit, reversible at
  any progress, reduced-motion variant. Satisfies the "workspace switching"
  step of the 30-second loop on its own.
- **Slice B — Mission Control overview.** Shrink the visible Space, reveal
  neighbors, workspace strip, minimized-window bottom strip, hit-test
  transfer, selection round-trip, and dragging windows between Spaces.
- **Slice C — overview performance/edges.** FR-8 frame-time measurement,
  multi-monitor lockstep, gate-level polish — landed here or with T-34.

## Scope

### In scope

1. **The transition** (from
   [03-workspaces.md](../design/03-workspaces.md)):
   ```text
   normal scene
       │ gesture progress 0 → 1
       ▼
   shrink visible workspace
       ├── reposition live window surfaces
       ├── reveal neighboring workspaces
       ├── display workspace strip
       └── transfer hit-testing to overview controller
   ```
   And selection: `overview → activate workspace → raise/focus window →
   reverse animation`.
2. **One state machine**: gesture, keyboard, hot corner, and the Mission
   Control button all drive the **same** progress pipeline; there is no
   second, discrete "instant" code path.
3. **Commit rules**:
   - Progress clamped 0→1; **rubber-banding** when swiping past the first
     or last Space.
   - Release commits if **progress or release velocity** crosses a
     threshold; otherwise the transition animates back.
   - One rule for swipes, pinches, and hot corners.
4. **Workspace switching** (adjacent Space slide) as the same progress
   pipeline: live surfaces + per-Space wallpaper repositioned with
   scale/translation/blur (T-05 scene mechanics), reversible at any progress.
5. **Overview chrome** (shell, via private protocol):
   - Workspace strip (renders Spaces — including the dedicated Spaces of
     fullscreen windows while they exist, [T-05](05-spaces-model.md) FR-3;
     never keeps a second copy of workspace state).
   - Minimized-windows bottom strip, restorable by click.
   - Window selection → activate; window dragging between Spaces in the
     overview ("windows move between Spaces by dragging in the overview").
6. **Hit-testing transfer**: while the overview is active its controller
   owns input; on reversal, ownership returns to the normal focus path —
   explicitly, never implicitly.
7. **Reduced-motion variants** for every transition (design-system rule).
8. **Performance work**: this feature owns the "60 Hz, no dropped frames
   for the full gesture" budget — scale/blur passes must fit the frame
   budget on baseline Intel/AMD.

### Out of scope

- Spaces *model* (T-05 — events and lists already exist).
- App switcher overlay (T-12 — different feature, compositor-driven too).
- Desktop Reveal (T-14, a window-aside transition sharing the pipeline).

## Requirements

- FR-1: One overview state machine; a test drives it via gesture, keyboard
  shortcut, hot corner, and menu-bar button and asserts identical
  progress/commit behavior.
- FR-2: Real textures are transformed throughout — a playing video keeps
  playing (at reduced scale) inside the overview; no thumbnails are ever
  substituted.
- FR-3: Commit thresholds: implement the progress-or-velocity rule; the
  rubber-band constants are tunable, not hardcoded magic.
- FR-4: Interruptibility: mid-transition input reverses direction without
  waiting for the animation to finish.
- FR-5: Window selection round-trip: overview → workspace activates →
  window raises/focuses → reverse animation → normal scene with focus.
- FR-6: Minimized windows render in the bottom strip, excluded from Space
  layout, clickable to restore.
- FR-7: Dragging a window's overview representation onto another Space
  moves it there (with T-05 window-assigned events).
- FR-8: 60 Hz with zero dropped frames for the full gesture on baseline
  hardware (measured, in the dev loop, not the polish phase).
- FR-9: Reduced-motion variant passes (Phase-2 exit criterion).
- FR-10: Fullscreen windows appear as their own Space in the workspace
  strip while fullscreen and leave it on unfullscreen (T-05 FR-3).

## Acceptance criteria

- [ ] The transition diagram above is implemented and visual-frame
      reviewed in nested mode.
- [x] Trigger-parity test passes (FR-1). *(First slice: `overview` unit
      tests drive gesture/keyboard/hot-corner/shell through one machine;
      `shell_protocol_conformance::overview_state_machine_has_trigger_parity`
      proves a gesture and a hot corner toggle the same overview state; U-7
      adds `shell_request_drives_the_overview_pipeline` for the
      private-protocol requests.)*
- [ ] Frame-time trace during full gesture stays within budget on
      baseline Intel/AMD. *(U-2 landed the instrumentation and the headless
      CI-budget assertion; the hardware run is B-5.)*
- [ ] Video-in-overview test (FR-2) passes.
- [ ] Phase-2 exit: the 30-second loop including Mission Control at zero
      dropped frames.

## Test plan

- Gesture math unit tests shared with T-03 (clamp, velocity, rubber-band).
- Headless: state-machine transitions, hit-test transfer.
- Nested + hardware: frame-time instrumentation during gestures; multi-
  monitor lockstep during overview.

## Risks / open questions

- Blur + scale + many windows on iGPU is the frame-budget risk; fall back
  to cheaper transforms (e.g. scale-only) under budget pressure — decide
  threshold and keep it measurable.
- Overview window occlusion/layout algorithm (which window peeks at what
  position) needs a spec: grid vs. macOS-style cascade; pick, document,
  iterate. *Decided (U-5):* grid, documented in
  [03-workspaces.md](../design/03-workspaces.md#window-layout-and-occlusion-t-11-u-5).

## Remaining work — unblocked

Items that need no unfinished dependency (T-33 materials, image wallpaper,
or baseline Intel/AMD hardware) and can land on the existing model/protocol.
Each is scoped to be independently testable in CI.

- **U-1 · FR-9 reduced-motion wiring.** ✅ landed. `df_toplevel_manager`
  grew the additive v3 `set_reduced_motion` request; the shell mirrors
  `accessibility.reduceMotion` (`ShellController::applyDockSettings` →
  `ShellProtocol::setReducedMotion`) and the compositor forwards it to
  `OverviewMachine::set_reduced_motion` (`DfState::set_reduced_motion`).
  `overview::tests::reduced_motion_single_steps_every_transition_kind` and
  `shell_protocol_conformance::reduced_motion_request_single_steps_the_overview`
  prove every transition kind (workspace slide, Mission Control, Desktop
  Reveal) takes the single-step path and still applies the same commit rule.
  T-15/T-16 later become the canonical settings source.
- **U-2 · FR-8 frame-time measurement infrastructure.** ✅ landed.
  `RenderStats` now records per-frame compositor-work durations and the
  interval since the previous rendered frame (bounded 8192-sample ring);
  `session.rs` times each rendered frame and `dump_stats` (SIGUSR1 + clean
  exit) prints a summary plus, with `DRAGONFRUIT_FRAME_TRACE`, the full
  `frame-trace` lines. `overview_gesture_frame_trace_stays_within_budget`
  drives a full gesture and asserts the trace exists and the animation-clock
  intervals stay within a CI budget. The hardware budget run is B-5.
- **U-3 · FR-2 live-surface conformance test.** Add a headless test proving no
  thumbnails are ever substituted: a client surface stays mapped and keeps
  receiving frame callbacks (a playing video keeps playing) through a full
  overview/workspace transition. The "at reduced scale" clause is B-1.
- **U-4 · Overview chrome per-output coverage/sizing.** 🔄 partial. A
  full-output `overlay` (anchored on all four edges — Mission Control) now
  spans **every** output even when chrome focus was captured on one
  (`LayerSurfaceState::is_full_output`/`visible_on_output`, unit-tested),
  while per-output popovers keep their T-10 behavior. True per-output *sizing*
  is still open: the shell renders one offscreen scene sized from the primary
  configure, so one surface/scene per output remains the T-11/T-16 seam.
- **U-5 · Overview layout algorithm spec.** ✅ landed. The grid algorithm
  (membership, ordering, grid shape, uniform scale, no occlusion, overflow)
  is documented in
  [03-workspaces.md](../design/03-workspaces.md#window-layout-and-occlusion-t-11-u-5).
  Implementation is B-9.
- **U-6 · Reduced-motion shell-chrome variants.** ✅ landed. `Overview.qml`
  derives `revealProgress` from `Theme.reducedMotion` (instant appearance, no
  slide/fade of the strips, grid, or scrim); `tst_overview` covers it. The
  compositor half is U-1.
- **U-7 · Shell-request trigger-parity conformance.** ✅ landed.
  `enter_mission_control`/`exit_mission_control` now route through
  `DfState::drive_overview_request` into the one machine and pipeline (the
  new directional `OverviewMachine::drive_overview` commits the requested
  state instead of toggling). `shell_request_drives_the_overview_pipeline`
  asserts the shared `shell` input action and progress events for both
  directions.
- **U-8 · Hit-test-transfer conformance test.** ✅ landed.
  `overview_owns_pointer_hit_testing_until_it_closes` proves pointer motion
  does not reach window surfaces while the overview owns input and returns to
  the normal focus path once ownership returns (FR-6).

## Remaining work — blocked

- **B-1 · Mission Control live-surface transform (T-33).** Shrink the visible
  Space and reveal neighbors by scaling/clipping/blurring the *real* surfaces.
  `DfState::apply_overview_scene` returns early for `!is_workspace_switch()`;
  `render_output` cannot scale individual windows, so this needs T-33's
  reusable scene-transform render-element wrapper. Blocks FR-2 (reduced
  scale), FR-5 (live-surface click selection), and the transition diagram.
- **B-2 · Per-Space wallpaper slide (T-05 image wallpaper + T-33).** The
  wallpaper is a flat clear color today, so the compositor-rendered background
  cannot visibly slide with its Space (T-05 owns image wallpaper; T-33 owns the
  transform).
- **B-3 · Overview window hit-testing / selection on live surfaces (T-33).**
  Clicking a live window in the overview (rather than a strip/grid card) needs
  the B-1 layout so the compositor knows the on-screen representation rects.
- **B-4 · Live-surface window grid + dragging the live representation
  (T-33).** Replace/augment the shell title-card grid with the transformed
  live surfaces and drag them between Spaces (the `move_to_workspace`
  primitive and shell drag path already exist).
- **B-5 · FR-8 hardware frame-time trace within budget (hardware + T-33).**
  Run the full gesture on nested and baseline Intel/AMD DRM and assert 60 Hz,
  zero dropped frames for the whole gesture; includes the scale/blur passes.
- **B-6 · Scale/blur frame-budget degrade tiers (T-33).** Define the measurable
  threshold and fall back to cheaper transforms (e.g. scale-only, blur off)
  under budget pressure; keep it instrumented via U-2.
- **B-7 · Nested visual-frame review of the transition diagram (T-33 +
  nested hardware).** The "shrink/reveal" look and the reversed selection
  animation cannot be visually reviewed until B-1 lands.
- **B-8 · Phase-2 exit: 30-second loop with Mission Control at zero dropped
  frames (T-34, gated by T-33/T-35/T-13/T-12).**
- **B-9 · Overview occlusion/layout implementation (T-33).** Apply the U-5
  algorithm to the transformed live-surface grid.

## Requirements coverage (audit)

| Req | Status | Remaining |
|---|---|---|
| FR-1 one machine / trigger parity | ✅ landed (unit + gesture/hot-corner/shell conformance) | — |
| FR-2 live textures, never thumbnails | 🔄 live translation only | U-3 test · B-1 scale |
| FR-3 commit thresholds tunable | ✅ landed | — |
| FR-4 interruptibility | ✅ landed | — |
| FR-5 selection round-trip | ✅ landed (strips + window grid) | B-3 live-surface click |
| FR-6 minimized bottom strip | ✅ landed | — |
| FR-7 drag window between Spaces | ✅ landed (shell window grid) | B-4 live-surface grid drag |
| FR-8 60 Hz, no dropped frames | 🔄 infra + CI-budget trace landed | B-5 hardware · B-6 tiers |
| FR-9 reduced-motion variant | ✅ landed (protocol v3 + shell + QML) | T-15/T-16 canonical source |
| FR-10 fullscreen Space in strip | ✅ landed | — |

Scope/acceptance deltas: scope item 1 (transition diagram) is partial → B-1/B-7;
item 4 (workspace switch with wallpaper) → B-2; item 5 (overview chrome) is
done except the live-surface grid → B-4, and full-output overlay coverage is
landed (U-4) with per-output sizing still on the T-16 seam; item 6 (hit-test
transfer) is done, with U-8's conformance test landed; item 7 (reduced motion)
is done (U-1/U-6); item 8 (performance) has the instrumentation landed (U-2)
and the hardware budget still open → B-5/B-6. Acceptance "visual-frame reviewed
in nested mode" → B-7; "frame-time trace" → U-2 (landed) / B-5 (hardware);
"video-in-overview test" → U-3/B-1; "Phase-2 exit" → B-8. Risks: blur/scale
budget threshold → B-6; layout algorithm spec → U-5 (landed) / B-9
(implementation).
