# T-10 — Dock

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `shell/dock/` |
| **Depends on** | [T-04](04-window-model.md) (window states, Zoom geometry) · [T-05](05-spaces-model.md) (cross-Space windows) · [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-09](09-menu-bar.md) · [T-15](15-settingsd-settings-model.md) (pinned-set keys; interim persistence below) · [T-23](23-app-index.md) (identity; stub until it lands) |
| **Blocks** | Phase-2 exit (core interaction loop) |
| **Estimate** | L |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [08-settings.md](../design/08-settings.md) · [09-files.md](../design/09-files.md) · [10-design-system.md](../design/10-design-system.md) · [03-workspaces.md](../design/03-workspaces.md) · [ROADMAP.md](../ROADMAP.md) |

## Summary

The macOS-style Dock: pinned apps with running indicators and launch
bounce, temporary entries for unpinned running apps, magnification,
auto-hide, drag rearrangement, contextual menus, a multi-window chooser,
and the Files-backed Trash on the right end.

The Dock is the second half of the core interaction loop
([ROADMAP.md](../ROADMAP.md)): it is how a session *starts* an app and how a
minimized window comes *back*. It is a persistent chrome surface (not a
window), it renders on every output, and it holds no authoritative state of
its own — every entry is a projection of compositor, app-index, GVfs, or
settings state.

## Background

The Dock obtains window/app state **directly from the compositor** rather
than inferring it through public protocols, using app-index to resolve
`app_id` / `WM_CLASS` to `.desktop` applications
([04-shell.md](../design/04-shell.md)). It is a consumer of the private
shell protocols (T-07), never a second owner of window, workspace, or
identity state.

The hard part is not drawing the Dock; it is the lifecycle: launch
failures, an app exiting mid-animation, windows opening on other Spaces,
apps with inconsistent identifiers, an uninstalled pinned app, a settingsd
or shell restart, a Trash mount that disappears, and output hotplug. This
document specifies that lifecycle in full; the ticket's test plan turns
each edge case into a scripted scenario.

The real macOS screenshots in `.docs/reference/macos/` (notably the desktop
with Dock, the Files sidebar, and the **Desktop & Dock** settings pane) are
a **style and information-architecture reference only** — per
[14-risks.md](../design/14-risks.md) we do not copy Apple's assets or
branding.

## Design

### 1. Surfaces, geometry, and reserved zones

The Dock is one `top` layer chrome surface per output, anchored to its
configured edge (`dock.position`: bottom, left, or right), namespace
`"dock"`, keyboard interaction `none` while idle. Context menus, the window
chooser, and the Trash confirmation are separate `overlay` surfaces
(`exclusive_zone = -1`, keyboard `on_demand`), following the T-09 pattern
of one offscreen render split into per-surface rectangles.

Geometry is driven by three quantities:

```text
baseline thickness   B = f(dock.size)              # the visible bar
magnified extent     M = f(dock.magnification)      # peak icon size, M >= B
bounce room          R = launch/attention overshoot
```

- **Reserved zone** reports **B only**, never M or R. Magnified icons and
  the bounce rise above the bar into the window area, exactly like macOS;
  windows are not re-laid-out while the pointer sweeps the Dock.
- The surface itself is tall enough to contain M + R (a transparent band
  above the bar). Its **input region** covers the bar and the currently
  magnified icon rectangles; the transparent padding passes pointer input
  through to windows beneath (T-04 input-region handling).
- `B` is subtracted from the usable area so Zoom (T-04) fills "Space minus
  menu bar and Dock". When `dock.autohide` is on, the reserved zone is
  **0 at all times** — the revealed Dock overlays content and does not
  resize anything.
- On left/right positions the Dock is vertical: the magnification axis, the
  indicator edge, and the reserved zone all rotate with it, and the top of
  the bar starts below the menu-bar reserved zone so it can never occlude
  the menu bar (FR-3).
- The Dock anchors on outputs attached after startup via the same
  `output = NULL` all-outputs rule the menu bar uses (T-09 FR-1); each
  output gets its own layout and auto-hide state.

### 2. Entry model

The Dock is an ordered list of entries in three regions, separated by
divider rules:

```text
[ pinned apps | temporary running apps | recent/suggested ]  │  [ minimized windows ]  │  [ Trash ]
        left region (grows right)            right region (grows left)
```

| Entry kind | Created by | Lifetime |
|---|---|---|
| **Pinned** | `dock.pinned` (user order) or "Keep in Dock" | until removed |
| **Temporary running** | compositor `toplevel` events for an app not pinned | while the app has ≥1 window |
| **Recent/suggested** | app-index recency (`dock.showRecentApps`) | bounded, not user-ordered |
| **Minimized window** | a minimized window when `dock.minimizeIntoTileIcon` is **off** | while minimized |
| **Trash** | permanent | session |

Rules:

- Entries are **per app**, not per window. Identity comes from app-index
  (`app_id` primary, Xwayland `WM_CLASS` fallback, heuristics). The Dock
  keeps one entry per resolved app record, holding a list of that app's
  `df_toplevel` handles.
- A pinned entry and a running temporary entry for the same app collapse
  into the pinned entry.
- When `dock.minimizeIntoTileIcon` is on, a minimized window does not get
  its own entry; the app entry's indicator/menu reflects it.
- The Trash is a permanent, non-reorderable entry at the far right; the
  divider before it cannot be dragged and the Trash cannot be removed.
- Ordering: the left region is `dock.pinned` order followed by temporary
  running entries in launch order; the right region (minimized windows) is
  most-recent-first. Recent/suggested entries append to the right of the
  left region.

### 3. Lifecycle state machine

The Dock as a whole:

```text
                    settings/geometry change
                             │
        ┌────────────────────▼────────────────────┐
        │                visible                  │
        │   idle ──pointer over──► magnifying     │
        │    │  ▲                    │            │
        │    │  └──pointer leaves────┘            │
        │    ├──press────────────► pressed        │
        │    ├──press+move───────► rearranging    │
        │    ├──right-click──────► menu-open      │
        │    └──idle timeout (autohide)───────────┼──► hidden
        └─────────────────────────────────────────┘
                             │
                             ▼
        hidden ──edge dwell / drag / attention / shortcut──► peeking ──► visible
```

Each app entry:

```text
pinned ──launch──► launching(bounce) ──first window──► running
                                      └──failure/timeout──► not-running + notice
running ──last window closes──► temporary: removed
                               pinned:    not-running
running ──all windows minimized──► running (indicator stays; minimized entry)
attention event ──► bouncing (bounded), stops on click or timeout
```

Every transition is **event-driven**. There is no polling and no timer
except: the launch-attention timeout, the auto-hide reveal/re-hide delays,
and the bounce animation clock. Idle Dock contributes zero wakeups (FR-8).

### 4. Launch

Click semantics (the macOS decision tree from
[04-shell.md](../design/04-shell.md)):

```text
Pinned application
        │
        ├── not running → launch
        │
        └── running
              ├── one window  → activate
              └── multiple    → window chooser

Running but not pinned
        │
        └── temporary Dock entry (same click rules)

Minimized window
        │
        └── optionally appear on right side (preference)
```

Launch sequence:

1. **Press** scales the entry down (design-system `Button` pressed state,
   `motion.hover`); release commits. A press that moves beyond the drag
   threshold becomes a rearrangement (section 7) instead.
2. Pinned, not running → `app-index.launch(identity, activation_token)`.
   The entry enters `launching`: with `dock.animateOpening` on it bounces
   (period ≈ 0.6 s, amplitude ≈ B/4) until the first window maps; with it
   off it shows a quiet "launching" affordance (dim + spinner-free dot).
3. The compositor resolves the `xdg-activation` request and emits the
   `attention` event; the Dock uses that, not a self-timer, as the
   authoritative "launching" signal (FR-4).
4. **First window maps** (`toplevel` event for the app) → the bounce stops
   mid-flight and settles; the entry becomes `running`; the running
   indicator appears.
5. **Launch failure / timeout** → the bounce stops, the entry returns to
   `not-running`, and a one-shot notice is raised through the notification
   service (T-25). There is never a stuck indicator and never a second
   concurrent launch for the same entry.
6. **App exit while animating** → the bounce or drag animation resolves to
   completion, the entry is removed (temporary) or set `not-running`
   (pinned), any open chooser/menu for it dismisses, and the running dot
   clears.

Interim note: until T-23 lands, the Dock carries a stub resolver and
launcher built on GIO `AppInfo` for `.desktop` lookup/launch and a minimal
identity map, explicitly marked for deletion when app-index replaces it
(the T-09 `demoAppMenu` precedent). The public behavior above does not
change when the stub is swapped out.

### 5. Activation and the window chooser

- **One window** → `df_toplevel.activate` (compositor raises and focuses;
  switches Space if the window lives elsewhere).
- **Multiple windows** → the **window chooser**: an `overlay` popover
  anchored to the entry.
  - Lists the app's windows **across all Spaces**, most-recent first,
    with title, Space name, and a minimized marker. The first row is
    **Show All Windows**.
  - Selecting a row activates the window's Space, then raises/focuses the
    window (compositor round-trip, `select_overview_toplevel` semantics).
    A minimized window is restored, not merely focused.
  - Keyboard: arrows move, Return activates, Escape dismisses; the chooser
    flips and scrolls to stay on screen.
  - The chooser tracks live state: a window closing removes its row; the
    app exiting dismisses the chooser.
- **Thumbnails are deferred.** The T-07 open question (trusted-shell-only
  toplevel-thumbnail event vs title-list fallback) is resolved here as
  **title list first**; live imagery is an additive, token-gated protocol
  extension later. The chooser's data model is shaped so imagery slots in
  without a rewrite.
- **Option-click** on a running app hides the current app and switches
  (macOS semantics); **Option-Command-click** switches and hides all
  others. These are compositor requests, not Dock-local state.

### 6. Running indicators, attention, badges

- **One indicator per app entry**, shown when the app has ≥1 window on any
  Space, minimized included (never per window). It is a small dot on the
  Dock-edge side, scaled with the icon during magnification, hidden when
  `dock.showIndicators` is off.
- **Attention** (`attention` event) triggers the higher bounce, bounded by
  a maximum duration and stopped by clicking the entry or the window
  becoming focused (FR-4). It never repeats forever.
- **Badges** are an app-provided count, distinct from the running dot. MVP
  renders only the **Trash** badge (section 11); general app badges are an
  extension point (app-index/notification service) and out of scope for
  the first vertical slice.

### 7. Drag and drop

- **Initiate**: press-and-hold then move beyond a small threshold lifts the
  entry (slight scale-up + shadow). Magnification is suppressed for the
  duration of the drag; icons snap to baseline size.
- **Reorder pinned**: dragging within the pinned region opens a live gap;
  the other icons animate to new positions (spring, reduced-motion aware);
  on drop the new order is written to `dock.pinned`. Reordering is
  left-region only; the right region and Trash are not user-orderable.
- **Promote**: dropping a temporary running entry inside the pinned region
  pins it ("Keep in Dock") without launching anything.
- **Remove**: dragging a pinned entry out of the Dock (past its bounds and
  a short dwell) shows a "Remove" label; dropping removes it from
  `dock.pinned` only — the application is untouched. If it is running it
  remains as a temporary entry.
- **External drops**: an app dropped from Files or a launcher pins it; a
  file/folder dropped on an app icon opens it with that app
  (`app-index`/GIO launch with the file argument); files dropped on Trash
  move to trash (section 11); a drop on empty Dock or the divider is a
  no-op.
- **Cross-output drag is not supported** in the MVP: the Dock is
  per-output, and a drag leaving the source output snaps back.
- **Keyboard reordering** is a later accessibility enhancement
  (section 14).

### 8. Context menus

Right-click / Control-click (press-and-hold on touch) opens a
design-system `ContextMenu` on an `overlay` surface, anchored to the entry,
flipping to stay on screen, dismissible by Escape, click-away, focus loss,
or the underlying app changing state. Submenus use the delayed-hover
drag-through rule from T-09.

| Target | Menu |
|---|---|
| Pinned, not running | Open · Options ▸ · Show in Files · **Remove from Dock** |
| Running app (any) | Window list (checkmark on frontmost, minimized marker) · **Show All Windows** · Keep/Remove from Dock · Options ▸ · **Quit** |
| Temporary running | Window list · Show All Windows · **Keep in Dock** · Options ▸ · Quit |
| Minimized-window entry | the owning app's window list · Show All Windows · Quit |
| Trash | Open · **Empty Trash** (disabled when empty) |
| Divider | Turn Magnification On/Off · Turn Hiding On/Off · Position on Screen ▸ · Dock Settings… |

- **Options ▸**: Assign To (This Desktop / All Desktops / None), Open at
  Login, Show in Files.
- **Modifiers**: Option turns **Quit → Force Quit**; the app-hiding items
  appear only for running apps. Labels reflect live state.
- Menus track state: if the app exits while its menu is open, the menu
  dismisses; if a window opens/closes, the window list updates.
- Empty Trash is destructive and opens a design-system confirmation sheet
  (matching Files' policy, [09-files.md](../design/09-files.md)) before
  invoking the GVfs empty operation.

### 9. Magnification

Magnification is **progress-based and interruptible** (the design-system
gesture rule): every frame it is recomputed from the current pointer
position, so it reverses instantly and never runs as a discrete animation.

```text
baseline size      B  (dock.size)
peak size          M  (dock.magnification; M = B when off)
falloff radius     R  (a multiple of B, a token, not user-facing)
pointer position   x_c  on the Dock axis

for each entry centered at x_i:
    d = |x_i - x_c|
    t = clamp(1 - d / R, 0, 1)
    scale_i = B + (M - B) * (1 - cos(pi * t)) / 2      # cosine falloff

layout: centers are recomputed from the scaled widths + constant gap,
        anchored so the entry under the pointer stays under the pointer;
        neighbours are pushed outward — no overlap, no clipping.
```

- The cosine falloff (not a linear ramp) is what makes neighbouring icons
  swell smoothly instead of stepping.
- A short spring using `motion.dock-magnify` smooths pointer sampling and
  gives the characteristic slight overshoot as the pointer enters/leaves;
  under reduced motion the spring is removed and sizes track the pointer
  directly (duration 0).
- **Constant-time per frame** over the visible entries, textures cached, one
  pass; the Dock's damage rect is the bar plus the magnified band, never the
  full output (FR-2).
- The reserved zone does not change during magnification (section 1), so
  windows never jump while the pointer crosses the Dock.
- On left/right positions the same math runs on the vertical axis.

### 10. Auto-hide

`dock.autohide` is a settings key now, surfaced in Settings → Desktop &
Dock later (T-16). Behavior:

- **Hidden**: the bar is translated off-screen by `B` plus the edge margin.
  The reserved zone is 0 (section 1), so windows and Zoom use the full
  space.
- **Reveal** triggers, in priority order:
  1. the pointer reaching the configured edge band and dwelling for the
     reveal delay (~120 ms; a fast flick reveals immediately);
  2. any drag operation whose pointer enters the edge band (reveal is
     immediate and the drag can drop onto an entry);
  3. a launch/attention event for a Dock entry, or a global shortcut
     (Fn-Control-F3 focus, Super+Option+D toggle) — immediate.
- **Re-hide** after the pointer leaves the bar and no menu, chooser, or
  drag is active, after the hide delay (~350 ms). Re-hide is suppressed
  while a context menu/window chooser is open, while dragging, while the
  app switcher or Mission Control owns input, and while keyboard focus is
  inside the Dock.
- **Never occlude the menu bar**: on left/right positions the revealed bar
  starts below the menu-bar reserved zone; on bottom it cannot reach it.
- **Fullscreen Spaces** suppress auto-hide reveal (the Dock and menu bar
  are hidden in fullscreen, macOS semantics); a modifier gesture can still
  summon it if T-11 defines one.
- Toggling `dock.autohide` or changing `dock.size`/position recomputes the
  usable area and animates any windows currently in the Zoomed state to the
  new geometry (compositor-side, T-04), so the transition reads as one
  motion.

### 11. Trash

- A permanent entry at the far right whose badge watches the **same GVfs
  `trash://` mount Files does** — one source of truth that also catches
  deletions by other applications. The Dock uses `GFileMonitor` on the
  trash backend; it never re-implements the freedesktop Trash spec and
  there is **zero IPC between the shell and Files**
  ([09-files.md](../design/09-files.md)).
- Icon reflects empty vs non-empty; the badge updates within one GVfs
  event, including deletions made by other apps (FR-6).
- **Click** opens Trash in Files (launch/activate via
  `org.dragonfruit.Files1` / the Files `.desktop`); if Files is not
  running it is launched at `trash://`.
- **Drop files** onto the entry → the shell calls GIO `g_file_trash()` on
  the GVfs backend directly (same backend Files uses; still no coupling).
  Per-item failures (busy, permission) surface as a notice, never a crash.
- **Context menu**: Open, Empty Trash (sensitive only when non-empty).
  Empty Trash confirms, then invokes the GVfs empty operation with progress
  for large trash.
- If the trash mount is unavailable (no GVfs / mount error), the entry
  renders dimmed with a disabled menu; it never blocks session start.

### 12. Multiple outputs

- The Dock renders on **every** output; identity, ordering, and settings are
  global, while hover/magnification, auto-hide reveal, and open popovers are
  per-output (only the output under the pointer magnifies/reveals).
- The pointer's output is the "active" Dock; keyboard navigation
  (Fn-Control-F3) targets the focused output.
- Output hotplug adds/removes a Dock instance without disturbing others;
  an output detaching while a popover is open on it dismisses the popover.

### 13. Settings model

The Dock is configured through named desktop-settings keys. T-15 owns
persistence; until it lands the Dock persists the same keys under
`$XDG_CONFIG_HOME/dragonfruit/` **in the eventual `org.dragonfruit.Settings1`
key shape**, so T-15 adopts the file without a migration (T-15 FR-1).

| Key | Type | Default | Consumer |
|---|---|---|---|
| `dock.pinned` | ordered app-id list | Files, Settings, Terminal, Browser | Dock |
| `dock.size` | 0..1 | 0.5 | Dock |
| `dock.magnification` | 0..1 (0 = off) | 0.5 | Dock |
| `dock.position` | `bottom` \| `left` \| `right` | `bottom` | Dock |
| `dock.autohide` | bool | false | Dock, compositor (Zoom) |
| `dock.animateOpening` | bool | true | Dock |
| `dock.showIndicators` | bool | true | Dock |
| `dock.minimizeIntoTileIcon` | bool | false | Dock |
| `dock.minimizedAnimation` | `genie` \| `scale` \| `none` | `scale` (MVP) | Dock/compositor |
| `dock.titlebarDoubleClick` | `zoom` \| `minimize` \| `none` | `zoom` | Compositor/SSD (T-13) |
| `dock.showRecentApps` | bool | false | Dock |

- Every key is observed via settingsd change signals, never polled; a live
  change re-lays-out and re-animates the Dock without a restart (FR-12).
- `dock.titlebarDoubleClick` is surfaced here (matching the macOS pane) but
  implemented by the compositor/SSD; the Dock only owns the key's settings
  model.
- The default pinned set is a vertical-slice decision and trivially
  changeable (see Risks).

### 14. Accessibility

- **Keyboard**: Fn-Control-F3 moves focus into the Dock; Left/Right (or
  Up/Down on vertical docks) moves between entries; Return opens; a menu
  key/Up opens the context menu; typing jumps by app name. Super+Option+D
  toggles auto-hide.
- **AT-SPI**: the Dock exposes list/listitem roles with per-entry names
  including state ("running", "2 windows", "minimized"); menus and the
  window chooser expose menu roles per the design-system contract. The
  design-system `FocusRing` marks the focused entry.
- **Reduced motion**: bounce becomes a single subtle pulse or nothing;
  magnification loses its spring overshoot; auto-hide reveal/hide is a
  short fade or instant; the minimize animation is `none`/`scale`.
- **VoiceOver-style labels** are part of the per-entry definition of done,
  verified per component rather than re-proven here (T-08 quality gate).

### 15. Idle and performance

- **Zero polling** (FR-8): all state arrives from `df_toplevel_manager`
  events (windows, Spaces, focus, attention), app-index subscribe events
  (install/uninstall/identity), `GFileMonitor` on `trash://`, and
  settingsd signals. Timers exist only for the bounded animations and
  auto-hide delays listed in section 3.
- **Magnification** holds the 60 Hz, no-dropped-frame budget on baseline
  Intel/AMD (FR-2); this is measured in the dev loop, not deferred to
  polish.
- **Damage** is confined to the Dock band plus the magnified extent; the
  idle Dock renders nothing and wakes nothing.

### 16. Lifecycle edge-case matrix

| Situation | Required behavior |
|---|---|
| Launch failure / app exits before mapping | bounce stops; entry returns to not-running; one-shot notice; no stuck indicator; retry allowed |
| App exits mid-bounce/drag | animation resolves; temporary entry removed, pinned entry not-running; chooser/menu dismiss |
| App uninstalled while pinned | app-index event updates/removes the entry; a stale pin degrades to "application not found" with Remove |
| Windows on another Space | indicator reflects them; chooser lists and activates the Space then the window |
| All windows minimized | running indicator stays; `minimizeIntoTileIcon` decides entry vs no entry |
| Missing `.desktop` / identity miss | generic icon + `app_id`/`WM_CLASS` label; no launch; miss logged to app-index heuristics; never a crash (FR-7) |
| Missing themed icon | fallback chain → generic app glyph |
| Shell restart | re-sync from compositor + app-index + settingsd; no polling |
| settingsd restart | re-read keys on reappearance |
| Output hotplug | Dock added/removed per output; no disturbance to others |
| Position/size/autohide changed live | reverse and re-layout; Zoomed windows animate to new usable area |
| Trash mount unavailable | dimmed entry, disabled menu; session unaffected |
| Rapid window open/close | count/chooser/indicator update without flicker or stale rows |

## Scope

### In scope

1. The full click tree and launch lifecycle (section 4).
2. App identity via app-index (T-23), with a deletable interim stub.
3. Visual/interaction behaviors: magnification, auto-hide, bounce,
   running indicators, drag rearrangement, context menus, window chooser,
   multi-output rendering, accessibility.
4. Trash entry and badge via GVfs (section 11).
5. Pinned-app and layout persistence under settingsd keys (section 13).

### Out of scope

- Files' Trash window behavior ([T-18](18-files-app.md)).
- The Settings → Desktop & Dock UI ([T-16](16-settings-app.md)); the Dock
  ships the model hooks and interim persistence only.
- Minimized-windows pane polish beyond the preference flag (Mission
  Control's bottom strip covers restore, [T-11](11-mission-control-workspace-ux.md)).
- Launchpad/stacks, general app badges, and the app switcher
  ([T-12](12-app-switcher.md)).
- Toplevel thumbnails in the chooser (additive, token-gated protocol
  extension later).

## Requirements

- FR-1: The full click tree above is implemented and property-tested,
  including: launch failure (bounce stops, notice, no stuck running
  indicator), app exit while animating (animation resolves, entry
  removed), windows opening on other workspaces (indicator + chooser
  correctness), and uninstalled/missing-identity apps (generic entry,
  never a crash).
- FR-2: Magnification is progress-driven and interruptible; constant-time
  per frame (no dropped-frame budget violations); reserved zone stays at
  baseline thickness throughout.
- FR-3: Auto-hide reveal and hide honor reserved zones and never occlude
  the menu bar; Zoom (T-04) geometry adapts when the Dock auto-hides or
  its size/position changes.
- FR-4: Bounce feedback triggers only on launch-attention events
  (`xdg-activation`), with a maximum duration, and stops on click or
  focus.
- FR-5: Window chooser lists windows across all Spaces; selection
  activates the window's Space then the window (compositor round-trip),
  restoring minimized windows.
- FR-6: Trash badge updates from GVfs `trash://` events, including
  deletions made by other applications; the shell performs drops with GIO
  and holds no coupling to Files.
- FR-7: Dock renders on every output; identity inconsistencies (missing
  `.desktop`) degrade to a generic icon + app_id text, never a crash.
- FR-8: Idle Dock: zero polling (all state via compositor events,
  app-index events, GVfs events, settingsd signals).
- FR-9: Drag rearrangement supports reorder, promote-to-pinned,
  remove-from-Dock, and external app/file drops, with live gap animation
  and reduced-motion variants.
- FR-10: Context menus match section 8, including live window lists,
  modifier variants, state tracking, and the Trash confirmation.
- FR-11: The Dock is keyboard-navigable and exposes AT-SPI list/menu roles
  with state-bearing labels; reduced-motion variants exist for every Dock
  animation.
- FR-12: Live settings changes (pin set, size, magnification, position,
  auto-hide, indicators) apply without restart via settingsd signals.

## Acceptance criteria

- [ ] Core interaction loop steps pass: launch → Dock animation → … →
      minimize → restore from Dock → close (Phase-2 exit).
- [ ] Lifecycle edge-case suite (launch failure, exit mid-animation,
      cross-workspace windows, inconsistent identifiers) scripted and
      passing.
- [ ] Magnification at 60 Hz on baseline hardware.
- [ ] Trash badge integration test with Files and a third-party deletion.
- [ ] Drag rearrangement and context-menu walkthroughs scripted.
- [ ] Keyboard + AT-SPI walkthrough and reduced-motion pass.

## Test plan

- Nested UI tests for the click tree with a test app producing 0/1/N
  windows on various Spaces.
- Headless: entry model, identity resolution, settings keys, GVfs trash
  events, auto-hide state machine.
- Restart tests: shell restart and settingsd restart re-sync Dock state.
- Perf: magnification frame budget measured (Phase perf table).
- Accessibility: keyboard walkthrough + per-entry AT-SPI roles.

## Risks / open questions

- App identity misses are the top UX risk — feed every miss into
  app-index heuristics (T-23).
- Decide the default pinned set (Files, Settings, Terminal?, Browser?) at
  vertical-slice time; trivially changeable.
- **Genie minimize** is expensive to do faithfully; MVP ships `scale` and
  treats `genie` as a compositor effect (T-04/T-13) behind the
  `dock.minimizedAnimation` key.
- **Auto-hide delays** are tunables; if they need to be user-facing they
  become settings keys, otherwise they stay design-system motion tokens.
- **Badges beyond Trash** need an owner (app-index vs notification
  service); deferred.
- **Drag across outputs** and **keyboard reordering** are explicitly
  deferred; revisit if the daily-driver bar demands them.
- The Dock is the second consumer of the shell's on-demand snapshot
  renderer; the deferred T-09 polish (commit from
  `QQuickWindow::afterRendering` while the scene is dirty) is shared with
  the Dock's magnification animation and should land here
  ([PROGRESS.md](../PROGRESS.md), "T-09 deferred polish backlog").
