# 0258 — Dock motion: one measured launch lift, per-window minimize targets, and the genie squeeze

## Status

accepted — amends [0218](0218-dock-motion-raised-cosine-and-tooltip.md) for
T237. The unresolved multi-bounce and interactive auto-hide curve stay `OPEN`.

## Context

T199/ADR 0218 pinned the measured magnification envelope and the 15 pt launch
amplitude, but deliberately kept two placeholders:

- the launch bounce still ran the older three-hop 600 ms adaptation, while the
  2026-11-05 campaign only resolves **one 15 pt lift over ~0.3 s** (the window
  opens over the Dock, so later hops are unobservable);
- `dock.minimizeIntoTileIcon` only hid the minimized-window region; the
  compositor always minimized into the app's Dock tile, and `genie` rendered
  the same uniform scale as `scale`.

The task also requires the reserved zone to follow auto-hide, and the
minimize/restore target to be legible per display.

## Decision

- **One measured launch lift.** `kLaunchBounceMs` is 300 ms and the launch
  phase is a single linear 0→1 ramp mapped through `sin(pi * phase)`. The
  existing `component.dock.launchLift = 15` pt amplitude is unchanged. The
  attention bounce keeps its taller, repeating 400 ms hop. No second or third
  launch hop is invented.
- **Minimize target follows `dock.minimizeIntoTileIcon`.**
  `df_toplevel_manager.set_minimize_target` (additive v11) carries the shared
  minimized-region rect (empty window id) and each minimized entry's resting
  tile (decimal window id). `WindowMotion` resolves a per-window target first,
  then the region, then the app's Dock tile, then the centered fallback.
  `minimizeIntoTileIcon=on` clears the targets so the app-tile mapping wins.
  The Dock publishes resting geometry only, so a magnification sweep does not
  churn the protocol. The compositor shifts the target by the window output's
  origin delta, so the persistent all-output Dock's primary-space rect lands
  on the display the window occupies.
- **Genie is a per-axis squeeze on the one motion transform.** Minimize/restore
  under `dock.minimizedAnimation=genie` use the measured 450 ms envelope and
  interpolate the horizontal axis ahead of the vertical one
  (`1-(1-t)^1.6` vs `t^1.6`), then land exactly on the tile/window. The true
  mesh warp remains a material-pass follow-up; there is no second animation
  path, and `scale`/`none` keep their existing timing and collapse.
- **Auto-hide reserves nothing.** `Dock.reservedThickness` is 0 whenever
  `autoHide` is on, in every reveal state; the shell forwards that one value to
  the compositor. The reveal/hide slide keeps the existing 120 ms reveal delay,
  350 ms hide grace, and 160 ms `motion.dockReveal`; its curve stays `OPEN`
  until a 120 fps capture resolves it.
- **Reduced motion and display policy.** The zero-duration token path is used
  for launch, minimize/restore (including genie), auto-hide, reorder, chooser,
  and Trash progress. The persistent Dock layer stays hidden for a fullscreen
  client and reserves no zone on any output.

## Consequences

- `tst_dockcore` pins the single 300 ms launch lift.
- `tst_dock` pins the minimized-target rects (bottom/left, per-window and
  region), the empty map under `minimizeIntoTileIcon`, auto-hide's zero
  reserved zone, and reveal retargeting from the current offset.
- `window::motion` pins the target precedence and the genie squeeze; the
  compositor layer tests pin the auto-hide reserved zone and fullscreen hide.
- `make dock-motion-capture` records the launch, minimize, restore, and
  auto-hide phases alongside the existing sweep/menu/size trace.
- The full launch multi-bounce, the interactive auto-hide curve, and the genie
  mesh warp remain `OPEN` rows in the motion table.
