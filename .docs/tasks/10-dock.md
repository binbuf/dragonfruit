# T-10 — Dock

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `shell/dock/` |
| **Depends on** | [T-04](04-window-model.md) (window states, Zoom geometry, focus/minimize broadcasts) · [T-05](05-spaces-model.md) (cross-Space windows, minimized exclusion) · [T-07](07-private-shell-protocols.md) (`df_shell`, `df_layer_surface`, `df_toplevel_manager`) · [T-08](08-design-system.md) (tokens, `ContextMenu`, `Popover`, `Icon`, motion) · [T-09](09-menu-bar.md) (shell bootstrap, offscreen-render pattern, chrome input routing) · [T-15](15-settingsd-settings-model.md) (pinned-set keys; interim persistence below) · [T-23](23-app-index.md) (identity/launch; stub until it lands) |
| **Coordinates with** | [T-11](11-mission-control-workspace-ux.md) (Show All Windows, minimized strip) · [T-12](12-app-switcher.md) (shared app identity/recency) · [T-13](13-window-decorations-ssd.md) (minimize animation) · [T-16](16-settings-app.md) (Desktop & Dock pane) · [T-18](18-files-app.md) (Trash, `org.dragonfruit.Files1`) · [T-25](25-notifications-and-osd.md) (launch-failure notice) |
| **Blocks** | Phase-2 exit (core interaction loop) |
| **Estimate** | L (Dock core) + L (polish slice) |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [08-settings.md](../design/08-settings.md) · [09-files.md](../design/09-files.md) · [10-design-system.md](../design/10-design-system.md) · [03-workspaces.md](../design/03-workspaces.md) · [02-compositor.md](../design/02-compositor.md) · [14-risks.md](../design/14-risks.md) · [ROADMAP.md](../ROADMAP.md) |

## Summary

The macOS-style Dock: pinned apps with running indicators and launch
bounce, temporary entries for unpinned running apps, magnification,
auto-hide, drag rearrangement, contextual menus, a multi-window chooser,
the Downloads stack, and the Files-backed Trash on the right end.

The Dock is the second half of the core interaction loop
([ROADMAP.md](../ROADMAP.md)): it is how a session *starts* an app and how a
minimized window comes *back*. It is a persistent chrome surface (not a
window), it renders on every output, and it holds no authoritative state of
its own — every entry is a projection of compositor, app-index, GVfs, or
settings state.

This document specifies the Dock **as a lifecycle**: every event source,
every entry state, every transition, and every failure mode. The ticket's
test plan turns each row of the edge-case matrix (section 22) into a scripted
scenario.

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
or shell restart, a Trash mount that disappears, and output hotplug.

The real macOS screenshots in `.docs/reference/macos/` (notably the desktop
with Dock, the Files sidebar, and the **Desktop & Dock** settings pane) are
a **style and information-architecture reference only** — per
[14-risks.md](../design/14-risks.md) we do not copy Apple's assets or
branding. The Desktop & Dock pane reference fixes the setting *set* we must
eventually expose (section 19); the visual design is ours.

## Design

### 1. Terms and invariants

| Term | Meaning |
|---|---|
| **Dock** | One chrome surface per output, plus its overlay popovers. |
| **Entry** | One clickable item in the Dock (app, minimized window, stack, or Trash). |
| **App entry** | An entry keyed by an app-index identity; holds that app's windows. |
| **Window entry** | A minimized-window entry (only when `dock.minimizeIntoTileIcon` is off). |
| **Region** | A contiguous run of entries between dividers (section 4). |
| **Bar** | The visible baseline strip of thickness `B`. |
| **Magnified band** | The transparent area above/beside the bar the icons grow into. |

Invariants, held at all times:

1. **Single ownership.** The Dock never stores window, Space, focus, identity,
   or settings state as authoritative; it projects and caches only what it
   needs to render, and reconciles from events (section 3).
2. **Per app, not per window.** At most one app entry exists per resolved
   identity. Minimized-window entries are the *only* per-window entries and
   only when the preference says so.
3. **Event-driven.** No polling. The only timers are the bounded animation
   clocks and the auto-hide reveal/re-hide delays (section 15).
4. **Reserved zone = baseline only.** Magnified icons, bounce, menus, and
   popovers never change the usable area (section 2).
5. **Interruptible, progress-based motion.** Magnification, auto-hide, drag
   gaps, and bounce are recomputed per frame; there is no discrete
   "instant" code path ([10-design-system.md](../design/10-design-system.md)).
6. **Restart-safe.** A shell restart re-syncs from the compositor, app-index,
   and settingsd with no Dock-local state to lose (pins live in settingsd,
   window/Space state in the compositor).

### 2. Surfaces, layers, geometry, reserved zones

The Dock is one `top`-layer `df_layer_surface` per output, namespace
`"dock"`, anchored to its configured edge (`dock.position`: bottom, left, or
right), keyboard interaction `none` while idle. Context menus, the window
chooser, and the Trash confirmation are separate `overlay` surfaces
(`exclusive_zone = -1`, keyboard `on_demand`), following the T-09 pattern of
one offscreen render split into per-surface rectangles.

Geometry is driven by four quantities:

```text
baseline thickness   B = f(dock.size)              # the visible bar
magnified extent     M = f(dock.magnification)      # peak icon size, M >= B
bounce room          R = launch/attention overshoot
popover extent       P = menus/chooser beyond the bar
```

- **Reserved zone** reports **B only**, never M, R, or P. Magnified icons
  and the bounce rise above the bar into the window area, exactly like
  macOS; windows are not re-laid-out while the pointer sweeps the Dock.
- The surface itself is large enough to contain `M + R` (a transparent band
  above/beside the bar). Its **input region** covers the bar and the
  currently magnified/bouncing icon rectangles; the transparent padding
  passes pointer input through to windows beneath (T-04 input-region
  handling). The shell updates the input region each animation frame, so a
  click in the transparent band never gets swallowed.
- `B` is subtracted from the usable area so Zoom (T-04) fills "Space minus
  menu bar and Dock". When `dock.autohide` is on, the reserved zone is
  **0 at all times** — the revealed Dock overlays content and does not
  resize anything.
- On left/right positions the Dock is vertical: the magnification axis, the
  indicator edge, and the reserved zone all rotate with it, and the top of
  the bar starts below the menu-bar reserved zone so it can never occlude
  the menu bar (FR-3).
- **Blur/translucency**: the bar is a translucent design-system material;
  the compositor must composite the Dock above the window space and provide
  the blur/translucency pass behind it (the T-08 "joint blur tuning with
  T-02/T-13" item). The current `chrome_render_elements` filter
  (`layer >= 2`) already composites `top`/`overlay`; the Dock needs no new
  layer, but the material contract is an explicit T-02/T-08 dependency.
- The Dock anchors on outputs attached after startup via the same
  `output = NULL` all-outputs rule the menu bar uses (T-09 FR-1), reusing
  `LayerSurfaceState::matches_output`; each output gets its own layout,
  magnification, auto-hide, and popover state.
- **Rendering split**: the Dock, like the menu bar, is one offscreen QML
  scene rendered by the shell; the shell commits the bar rectangle to the
  `top` surface and each popover rectangle to its `overlay` surface. The
  Dock's continuous animation requires the **snapshot-renderer fix** from
  the T-09 backlog — commit from `QQuickWindow::afterRendering` (or a
  render-control path) while the scene is dirty, rather than sampling with a
  timer (section 21).

### 3. Event sources

Every Dock state change is a reaction to one of these; there is no other
input path.

| Source | Events consumed | Affects |
|---|---|---|
| `df_toplevel_manager` | `toplevel`, `focused`, `attention`, `workspace`, `workspace_activated`, `output`, `overview_changed` | entries, indicators, chooser, bounce, per-output instances |
| `df_toplevel` | `app_id`, `title`, `state`, `workspace_entered/left`, `output_entered/left`, `closed` | identity, window lists, minimized markers, indicator |
| app-index (`org.dragonfruit.AppIndex1`) | resolve, subscribe (installed/uninstalled/updated) | identity, icon/name, pinned-app validity, launch |
| settingsd (`org.dragonfruit.Settings1`) | `dock.*` change signals | layout, position, magnification, auto-hide, pins, indicators |
| GVfs `trash://` | `GFileMonitor` change | Trash icon and badge |
| session/output hotplug | `df_output` add/remove, `reserved_zone` | per-output Dock instances |

### 4. Entry model

The Dock is an ordered list of entries in regions separated by divider
rules:

```text
[ pinned apps | temporary running apps | recent/suggested ]  │  [ minimized windows ]  │  [ Trash ]
        left region (grows right)            right region (grows left)
```

| Entry kind | Created by | Lifetime |
|---|---|---|
| **Pinned** | `dock.pinned` (user order) or "Keep in Dock" | until removed |
| **Temporary running** | a `df_toplevel` for an app not pinned | while the app has ≥1 window (MVP rule; see 4.1) |
| **Recent/suggested** | app-index recency (`dock.showRecentApps`) | bounded (≤3), not user-ordered |
| **Minimized window** | a minimized window when `dock.minimizeIntoTileIcon` is **off** | while minimized |
| **Downloads stack** | the Downloads folder (when stacks ship) | session |
| **Trash** | permanent | session |

Per-entry data the Dock holds (a projection, never authoritative):

```text
identity      resolved app record (id, name, themed icon, launch semantics)
kind          pinned | temporary | recent | minimized | stack | trash
windows       ordered df_toplevel handles (most-recent-first, all Spaces)
launch        idle | launching | failed        (transient, animation only)
attention     none | bouncing                  (transient, animation only)
badge         count (Trash in MVP; app badges deferred)
order         index within its region
```

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
  left region, before the first divider.

#### 4.1 "Running" vs "has windows" — a deliberate divergence

macOS keeps an app in the Dock from launch until quit, even with zero open
windows, because the Dock tracks *processes*. Our compositor exposes
*windows*, not processes, and the private protocol has no "app running"
event. The MVP rule is therefore:

> A temporary entry exists **while the app has ≥1 window**. When the last
> window closes, the entry is removed even if the process is still alive.

This is close to macOS in practice (most Linux apps quit with their last
window) and is honest about the state we own. The escape hatch is app-index:
when T-23 grows a **launch registry** (it already owns launch and activation
tokens), it can expose an `app_running`/`app_exited` signal, and the Dock
can switch to process-accurate lifetime without touching the rest of the
entry model. This is recorded as a decision (section 25), not an accident.

#### 4.2 Identity changes at runtime

`df_toplevel.app_id` can arrive late or change. On every identity update:

- re-resolve through app-index; if the resolved app changes, move the window
  to the correct entry (possibly merging two temporary entries);
- if resolution misses, keep a generic entry keyed by the raw identity
  (generic glyph + `app_id`/`WM_CLASS` label), never crash, and log the miss
  to app-index heuristics (T-23);
- if a pinned entry's app becomes unresolvable (uninstalled), degrade to
  "application not found" with the Remove action (section 13).

### 5. Layout

```text
bottom dock:  entries laid left→right in the left region, centered as a
              group; the right region is laid right→left from the right edge;
              Trash is the last right-region item.
vertical dock: the same, top→bottom, with the reserved zone on the chosen edge.
```

- The left and right regions are laid out independently and meet at the
  middle; a divider line separates them. When the content is narrower than
  the output the group is centered; when it is wider the Dock is **clipped
  to the output** and does not shrink icons (section 5.1).
- The **separator** between regions is a drag handle: dragging it resizes
  the Dock (writes `dock.size`), matching the macOS behavior; Control-click
  opens the divider menu (section 13).
- Entry spacing is a token (`component.dock.gap`); icon size derives from
  `B` minus bar padding.

#### 5.1 Overflow

A Dock that exceeds the output is an error state, not a normal one:

- the Dock scrolls? **No** — macOS shrinks; we instead **clamp `dock.size`
  at layout time** so the pinned set always fits at the minimum icon size,
  and overflow entries are hidden behind a "»" affordance is **out of
  scope** for MVP;
- the temporary/recent regions shrink first; pinned entries are never
  dropped;
- a warning is logged once per session; the Settings pane explains the
  constraint.

### 6. Dock lifecycle state machine

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
        │    ├──multi-window─────► chooser-open   │
        │    └──idle timeout (autohide)───────────┼──► hidden
        └─────────────────────────────────────────┘
                             │
                             ▼
        hidden ──edge dwell / drag / attention / shortcut──► peeking ──► visible
```

- `peeking` is the transient revealed-but-not-yet-hovered state during
  auto-hide reveal; it becomes `visible` on pointer enter, and reverses to
  `hidden` if the pointer leaves before entering the bar.
- `magnifying`, `pressed`, `rearranging`, `menu-open`, and `chooser-open`
  are mutually exclusive per output except `menu-open`/`chooser-open`, which
  suppress magnification.
- The Dock is **per output**: each output runs this machine independently
  for pointer states; entry/ordering/settings are global (section 18).

### 7. App-entry lifecycle state machine

```text
not-running ──launch──► launching(bounce) ──first window──► running
                                     └──failure/timeout──► not-running + notice
running ──last window closes──► temporary: removed
                               pinned:    not-running
running ──all windows minimized──► running (indicator stays; minimized entry)
attention event ──► bouncing (bounded), stops on click or focus
running ──app quits──► (window-based) not-running / removed
```

Every transition is **event-driven**. There is no polling and no timer
except: the launch/attention timeout, the auto-hide reveal/re-hide delays,
and the animation clocks. Idle Dock contributes zero wakeups (FR-8).

### 8. Launch

Click semantics (the macOS decision tree from
[04-shell.md](../design/04-shell.md)):

```text
App entry (pinned or temporary)
        │
        ├── not running → launch (app-index, activation token)
        │
        └── running
              ├── one window, not minimized  → activate it
              ├── one window, minimized      → unminimize + activate
              ├── multiple windows           → window chooser (section 9)
              └── zero windows (process up)  → activate (app opens a window)
```

- **App-level activation** uses `df_toplevel_manager.activate_app(app_id)`:
  the compositor picks the app's most recent window, switches to its Space
  if needed, and restores it if minimized. This is the correct request for a
  Dock click — not per-window `df_toplevel.activate`, which the chooser uses
  for a specific window.
- **Zero-window running app**: the Dock cannot distinguish this from
  not-running under the MVP window rule; it falls into the launch path and
  app-index's activation request reaches the live instance. When the launch
  registry lands (4.1) this becomes a distinct branch.

Modifiers (all compositor/app-index requests, not Dock-local state):

| Modifier | Action |
|---|---|
| Click | the tree above |
| Option-click | hide the current app and switch to the clicked app |
| Option-Command-click | switch to the clicked app and hide all others |
| Command-click | reveal the app's bundle/executable in Files ("Show in Files") |
| Control-click | context menu (section 13) |
| Middle-click | *out of scope* (no macOS equivalent) |

Launch sequence:

1. **Press** scales the entry down (design-system `Button` pressed state,
   `motion.hover`); release commits. A press that moves beyond the drag
   threshold becomes a rearrangement (section 12) instead.
2. Pinned/not-running or temporary → `app-index.launch(identity,
   activation_token)`. The entry enters `launching`: with
   `dock.animateOpening` on it bounces (section 8.1); with it off it shows a
   quiet "launching" affordance (dim + static dot).
3. The compositor resolves the `xdg-activation` request and emits
   `attention`; the Dock uses that, not a self-timer, as the authoritative
   "launching" signal (FR-4).
4. **First window maps** (`df_toplevel` announce for the app) → the bounce
   stops mid-flight and settles; the entry becomes `running`; the running
   indicator appears.
5. **Launch failure / timeout** → the bounce stops, the entry returns to
   `not-running`, and a one-shot notice is raised through the notification
   service (T-25). There is never a stuck indicator and never a second
   concurrent launch for the same entry.
6. **App exits while animating** → the bounce or drag animation resolves to
   completion, the entry is removed (temporary) or set `not-running`
   (pinned), any open chooser/menu for it dismisses, and the running dot
   clears.

#### 8.1 Bounce

- **Launch bounce**: 1–3 hops over ~0.6 s, amplitude ≈ `B/4`, sinusoidal
  (the entry leaves the bar and returns), then settles. It is a
  *compositor-clock* animation, not a Qt animation, so it can be
  interrupted by the first-window event without a visual snap.
- **Attention bounce**: taller (~`B/2`), repeating, bounded by a maximum
  duration (~2 s) and a maximum hop count; stops immediately on click or
  when the app's window gains focus (FR-4). It never repeats forever.
- **Reduced motion**: one subtle pulse (scale only, no translation) or
  nothing; the state change is still legible.

Interim note: until T-23 lands, the Dock carries a stub resolver and
launcher built on GIO `AppInfo` for `.desktop` lookup/launch and a minimal
identity map, explicitly marked for deletion when app-index replaces it
(the T-09 `demoAppMenu` precedent). The public behavior above does not
change when the stub is swapped out.

### 9. Activation and the window chooser

- **One window** → `df_toplevel.activate` (compositor raises and focuses;
  switches Space if the window lives elsewhere).
- **Multiple windows** → the **window chooser**: an `overlay` popover
  anchored to the entry, with an arrow pointing at it.
  - Lists the app's windows **across all Spaces**, most-recent first, with
    title, Space name, and a minimized marker. The first row is
    **Show All Windows**.
  - The frontmost window carries a checkmark.
  - Selecting a row activates the window's Space, then raises/focuses the
    window (`select_overview_toplevel` semantics). A minimized window is
    restored, not merely focused.
  - **Show All Windows** asks T-11 to enter Mission Control filtered to the
    app's windows; until that filter exists it expands the chooser to the
    full window list.
  - Keyboard: arrows move, Return activates, Escape dismisses; the chooser
    flips and scrolls to stay on screen; typing jumps by window title.
  - The chooser tracks live state: a window closing removes its row; the
    app exiting dismisses the chooser; a new window prepends a row.
- **Option-click** on a running app hides the current app and switches
  (macOS semantics); **Option-Command-click** switches and hides all
  others. These are compositor requests, not Dock-local state.
- **Thumbnails are deferred.** The T-07 open question (trusted-shell-only
  toplevel-thumbnail event vs title-list fallback) is resolved here as
  **title list first**; live imagery is an additive, token-gated protocol
  extension later. The chooser's data model is shaped so imagery slots in
  without a rewrite (a `thumbnail` field on the row model, always null in
  MVP).

### 10. Running indicators, attention, badges

- **One indicator per app entry**, shown when the app has ≥1 window on any
  Space, minimized included (never per window). It is a small dot on the
  Dock-edge side (bottom for a bottom Dock; the screen-edge side for a
  vertical Dock), scaled with the icon during magnification, hidden when
  `dock.showIndicators` is off.
- The indicator animates in/out with the design-system `motion.focus`
  token; under reduced motion it appears instantly.
- **Attention** (`attention` event) triggers the higher bounce (section
  8.1), bounded and stopped by clicking the entry or the window becoming
  focused (FR-4).
- **Badges** are an app-provided count, distinct from the running dot. MVP
  renders only the **Trash** badge/state (section 16); general app badges
  are an extension point (app-index/notification service) and out of scope
  for the first vertical slice. The entry model already has the field.

### 11. Minimize and restore

- **Minimize** is a compositor state transition (T-04); the Dock only
  observes it.
- When `dock.minimizeIntoTileIcon` is **off**, a minimized window gets its
  own right-region entry; clicking it calls `df_toplevel.unminimize` then
  `activate`, restoring it to its Space.
- When **on**, no separate entry appears; the owning app entry's indicator
  and menu reflect the minimized window, and clicking the app restores the
  most recent minimized window via `activate_app`.
- **Minimize animation** (`dock.minimizedAnimation`: `genie` | `scale` |
  `none`) is a compositor-side effect (T-04/T-13). MVP ships `scale`;
  `genie` is a compositor effect behind the same key, and `none` is the
  reduced-motion fallback. The Dock does not animate the window itself.
- The minimized right region is most-recent-first and bounded only by the
  layout; Mission Control's bottom strip (T-11) is the full view.

### 12. Drag and drop

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
  remains as a temporary entry. There is no "poof" animation (macOS removed
  it in Yosemite); the label is the affordance.
- **External drops**:
  - an app dropped from Files or a launcher pins it (an alias entry);
  - a **file/folder dropped on an app icon** opens it with that app
    (`app-index`/GIO launch with the file argument); multiple files are one
    launch with multiple arguments;
  - a file/folder dropped on the **Trash** moves to trash (section 16);
  - a file/folder dropped on the **Downloads stack** copies/moves into the
    Downloads folder;
  - a drop on empty Dock or the divider is a no-op.
- **Spring-loading**: dragging over a Dock folder/stack opens it after a
  hover delay (shared constant with Files, T-18). Out of MVP unless stacks
  ship; the hook exists.
- **Cross-output drag is not supported** in the MVP: the Dock is
  per-output, and a drag leaving the source output snaps back.
- **Keyboard reordering** is a later accessibility enhancement
  (section 20).

### 13. Context menus

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
| Downloads stack | file list · Open in Files · Sort By ▸ · Remove from Dock |
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

### 14. Magnification

Magnification is **progress-based and interruptible** (the design-system
gesture rule): every frame it is recomputed from the current pointer
position, so it reverses instantly and never runs as a discrete animation.

```text
baseline size      B  (dock.size)
peak size          M  (dock.magnification; M = B when off)
falloff radius     R  (a multiple of B, a token, not user-facing)
pointer position   x_c  on the Dock axis
gap                G  (component.dock.gap token)

for each entry centered at x_i:
    d = |x_i - x_c|
    t = clamp(1 - d / R, 0, 1)
    scale_i = B + (M - B) * (1 - cos(pi * t)) / 2      # cosine falloff

layout: centers are recomputed from the scaled widths + scaled gaps,
        anchored so the entry under the pointer stays under the pointer;
        neighbours are pushed outward — no overlap, no clipping.
```

- The cosine falloff (not a linear ramp) is what makes neighbouring icons
  swell smoothly instead of stepping.
- The **anchor rule** is exact: the entry whose center is nearest `x_c`
  stays centered under the pointer at every progress value. This is what
  makes the interaction feel direct rather than laggy.
- Gaps between entries scale with the two adjacent icon scales
  (`G * (s_i + s_{i+1}) / 2`) so the group expands smoothly; the baseline
  gap is a token.
- At the Dock ends the group is allowed to extend toward the screen edge
  but never past it; neighbours are pushed inward so no icon is clipped.
- A short spring using `motion.dock-magnify` smooths pointer sampling and
  gives the characteristic slight overshoot as the pointer enters/leaves;
  under reduced motion the spring is removed and sizes track the pointer
  directly (duration 0).
- **Constant-time per frame** over the visible entries, textures cached, one
  pass; the Dock's damage rect is the bar plus the magnified band, never the
  full output (FR-2).
- The reserved zone does not change during magnification (section 2), so
  windows never jump while the pointer crosses the Dock.
- On left/right positions the same math runs on the vertical axis.
- Magnification is suppressed while a context menu, chooser, or drag is
  active, and while the app switcher or Mission Control owns input.

### 15. Auto-hide

`dock.autohide` is a settings key now, surfaced in Settings → Desktop &
Dock later (T-16). Behavior:

- **Hidden**: the bar is translated off-screen by `B` plus the edge margin
  (a whole-surface translation; the reserved zone is already 0). The
  transparent magnified band is not part of the hit area when hidden.
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
- If the Dock must hide while a popover is open (settings change, output
  change), the popover dismisses; it never floats detached.
- **Never occlude the menu bar**: on left/right positions the revealed bar
  starts below the menu-bar reserved zone; on bottom it cannot reach it.
- **Fullscreen Spaces** suppress auto-hide reveal (the Dock and menu bar
  are hidden in fullscreen, macOS semantics); a pointer at the edge reveals
  both, and a modifier gesture can still summon the Dock if T-11 defines
  one.
- Toggling `dock.autohide` or changing `dock.size`/position recomputes the
  usable area and animates any windows currently in the Zoomed state to the
  new geometry (compositor-side, T-04), so the transition reads as one
  motion.
- The reveal/hide translation mechanism (margin animation vs. oversized
  surface with rendered offset) is an implementation decision validated
  against the compositor's layer placement (section 25).

### 16. Trash

- A permanent entry at the far right whose state watches the **same GVfs
  `trash://` mount Files does** — one source of truth that also catches
  deletions by other applications. The Dock uses `GFileMonitor` on the
  trash backend; it never re-implements the freedesktop Trash spec and
  there is **zero IPC between the shell and Files**
  ([09-files.md](../design/09-files.md)).
- Icon reflects empty vs non-empty (full trash shows the crumpled-paper
  state); the badge updates within one GVfs event, including deletions made
  by other apps (FR-6).
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

### 17. Downloads stack and recents

Two macOS Dock behaviors are specified here but **deferred behind settings**
so the vertical slice stays small:

- **Downloads stack**: a right-region stack entry for `~/Downloads`, with a
  fan/grid/list popover on click, file drops that move/copy into the
  folder, and a badge for new items. Depends on Files-core (T-17) folder
  monitoring; ships after the core loop.
- **Recent/suggested apps** (`dock.showRecentApps`, macOS default on): up
  to three recently used apps not already pinned, toward the right end of
  the left region, keyed by app-index recency (shared with T-12). MVP
  defaults this **off**; the entry kind and layout slot exist so enabling
  it is not a rewrite.

Both are marked out of the first vertical slice in Scope.

### 18. Multiple outputs

- The Dock renders on **every** output; identity, ordering, and settings
  are global, while hover/magnification, auto-hide reveal, and open
  popovers are per-output (only the output under the pointer
  magnifies/reveals).
- The pointer's output is the "active" Dock; keyboard navigation
  (Fn-Control-F3) targets the focused output.
- Output hotplug adds/removes a Dock instance without disturbing others;
  an output detaching while a popover is open on it dismisses the popover.
- **macOS "Displays have separate Spaces"** maps to our per-output Space
  lists (T-05): the Dock is present on all displays. The macOS alternative
  (Dock on the main display only) is a possible future setting; not in MVP.
- **Known limitation carried from T-09**: chrome surfaces are created with
  `output = None` and `LayerSurfaceState::configure` sizes from the *first*
  output, so on mixed-resolution multi-monitor the Dock buffer is sized to
  the first output and stretched on others. Per-output chrome sizing and
  per-output reserved zones are T-11/T-16; `matches_output` makes adding
  per-output placement a filter change, not a redesign.

### 19. Settings model

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
| `dock.showRecentApps` | bool | false (macOS: on) | Dock |

- Every key is observed via settingsd change signals, never polled; a live
  change re-lays-out and re-animates the Dock without a restart (FR-12).
- `dock.titlebarDoubleClick` is surfaced here (matching the macOS pane) but
  implemented by the compositor/SSD; the Dock only owns the key's settings
  model. macOS offers Fill/Zoom/Minimize/No Action; T-04 has no distinct
  "Fill" state, so `zoom` maps Fill and Zoom for MVP (decision, section 25).
- The default pinned set is a vertical-slice decision and trivially
  changeable (section 25).
- Auto-hide delays and bounce constants are design-system motion tokens, not
  settings keys, until there is a reason to expose them.

### 20. Accessibility

- **Keyboard**: Fn-Control-F3 moves focus into the Dock; Left/Right (or
  Up/Down on vertical docks) moves between entries; Return opens; a menu
  key/Up opens the context menu; typing jumps by app name. Super+Option+D
  toggles auto-hide. While keyboard focus is inside the Dock, the surface
  takes `on_demand` keyboard interaction and releases it on leave.
- **AT-SPI**: the Dock exposes list/listitem roles with per-entry names
  including state ("running", "2 windows", "minimized"); menus and the
  window chooser expose menu roles per the design-system contract. The
  design-system `FocusRing` marks the focused entry. **Risk**: the shell is
  a plain offscreen Qt executable; exposing the Dock to AT-SPI needs the Qt
  accessibility bridge on the session bus (the T-08 "live AT-SPI dump"
  item). This must be validated early, not at the end (section 25).
- **Reduced motion**: bounce becomes a single subtle pulse or nothing;
  magnification loses its spring overshoot; auto-hide reveal/hide is a
  short fade or instant; the minimize animation is `none`/`scale`.
- **VoiceOver-style labels** are part of the per-entry definition of done,
  verified per component rather than re-proven here (T-08 quality gate).

### 21. Idle, rendering, and performance

- **Zero polling** (FR-8): all state arrives from `df_toplevel_manager`
  events, `df_toplevel` events, app-index subscribe events,
  `GFileMonitor` on `trash://`, and settingsd signals. Timers exist only
  for the bounded animations and auto-hide delays listed in sections 6–7.
- **Snapshot renderer**: the Dock animates continuously (magnification,
  bounce, auto-hide, gaps). The shell's on-demand `grabWindow` path from
  T-09 is not sufficient; T-10 must land the durable fix — commit from
  `QQuickWindow::afterRendering` (or a render-control path) while the scene
  is dirty — so every animated frame reaches the compositor without a
  sampling timer (T-09 deferred-polish item 2, shared here by design).
- **Magnification** holds the 60 Hz, no-dropped-frame budget on baseline
  Intel/AMD (FR-2); this is measured in the dev loop, not deferred to
  polish.
- **Damage** is confined to the Dock band plus the magnified extent; the
  idle Dock renders nothing and wakes nothing. The shell sends a fresh
  input region each frame so the transparent band never captures input.
- **Constant work per frame**: magnification is O(visible entries); the
  compositor blur behind the bar is the material cost to watch (T-02/T-08).

### 22. Lifecycle edge-case matrix

| Situation | Required behavior |
|---|---|
| Launch failure / app exits before mapping | bounce stops; entry returns to not-running; one-shot notice; no stuck indicator; retry allowed |
| App exits mid-bounce/drag | animation resolves; temporary entry removed, pinned entry not-running; chooser/menu dismiss |
| App uninstalled while pinned | app-index event updates/removes the entry; a stale pin degrades to "application not found" with Remove |
| App_id changes after mapping | window re-homed to the correct entry; entries merged; miss logged |
| Windows on another Space | indicator reflects them; chooser lists and activates the Space then the window |
| All windows minimized | running indicator stays; `minimizeIntoTileIcon` decides entry vs no entry; click restores |
| Last window closes, process alive | MVP: temporary entry removed (section 4.1); pinned entry not-running |
| Missing `.desktop` / identity miss | generic icon + `app_id`/`WM_CLASS` label; no launch; miss logged to app-index heuristics; never a crash (FR-7) |
| Missing themed icon | fallback chain → generic app glyph |
| Shell restart | re-sync from compositor + app-index + settingsd; no polling; popovers gone |
| settingsd restart | re-read keys on reappearance |
| app-index restart | re-resolve identities and re-query pins; entries rebuild from compositor windows |
| Output hotplug | Dock added/removed per output; no disturbance to others; popover on a removed output dismisses |
| Position/size/autohide changed live | reverse and re-layout; Zoomed windows animate to new usable area; open popover dismisses if the bar moves |
| Trash mount unavailable | dimmed entry, disabled menu; session unaffected |
| Rapid window open/close | count/chooser/indicator update without flicker or stale rows |
| Dock overflows the output | clamp size at layout; hide temporary/recent first; pinned never dropped; log once |
| Launch while already launching | no second launch; the existing launch is coalesced |
| Attention for an unpinned, unlaunched app | temporary entry appears (window exists) and bounces; not treated as a launch |
| Drag dropped on a window underneath | the drop does not leak to the window; the Dock owns the drag once initiated |
| Click in the transparent magnified band | passes through to the window (input region excludes it) |

## Scope

### In scope

1. The full click tree, activation, and launch lifecycle (sections 8–9).
2. App identity via app-index (T-23), with a deletable interim stub.
3. Visual/interaction behaviors: magnification, auto-hide, bounce,
   running indicators, drag rearrangement, context menus, window chooser,
   multi-output rendering, accessibility.
4. Trash entry and state via GVfs (section 16).
5. Pinned-app and layout persistence under settingsd keys (section 19).
6. The snapshot-renderer fix needed for continuous animation (section 21).

### Out of scope

- Files' Trash window behavior ([T-18](18-files-app.md)).
- The Settings → Desktop & Dock UI ([T-16](16-settings-app.md)); the Dock
  ships the model hooks and interim persistence only.
- Minimized-windows pane polish beyond the preference flag (Mission
  Control's bottom strip covers restore, [T-11](11-mission-control-workspace-ux.md)).
- **Downloads stack and recent/suggested apps** in the first vertical slice
  (specified in section 17, deferred).
- Launchpad, general app badges, folder stacks beyond Downloads, and the
  app switcher ([T-12](12-app-switcher.md)).
- Toplevel thumbnails in the chooser (additive, token-gated protocol
  extension later).
- Cross-output drag and keyboard reordering (sections 12, 20).

## Requirements

- FR-1: The full click tree above is implemented and property-tested,
  including: launch failure (bounce stops, notice, no stuck running
  indicator), app exit while animating (animation resolves, entry removed),
  windows opening on other Spaces (indicator + chooser correctness), and
  uninstalled/missing-identity apps (generic entry, never a crash).
- FR-2: Magnification is progress-driven and interruptible; constant-time
  per frame (no dropped-frame budget violations); the pointer-anchored
  entry stays under the pointer; reserved zone stays at baseline thickness
  throughout.
- FR-3: Auto-hide reveal and hide honor reserved zones and never occlude
  the menu bar; Zoom (T-04) geometry adapts when the Dock auto-hides or its
  size/position changes; popovers dismiss rather than float detached.
- FR-4: Bounce feedback triggers only on launch-attention events
  (`xdg-activation`), with a maximum duration, and stops on click or focus;
  launch bounce and attention bounce are distinguishable.
- FR-5: Window chooser lists windows across all Spaces; selection
  activates the window's Space then the window (compositor round-trip),
  restoring minimized windows; the chooser tracks live state.
- FR-6: Trash state updates from GVfs `trash://` events, including
  deletions made by other applications; the shell performs drops with GIO
  and holds no coupling to Files.
- FR-7: Dock renders on every output; identity inconsistencies (missing
  `.desktop`) degrade to a generic icon + app_id text, never a crash.
- FR-8: Idle Dock: zero polling (all state via compositor events,
  app-index events, GVfs events, settingsd signals).
- FR-9: Drag rearrangement supports reorder, promote-to-pinned,
  remove-from-Dock, and external app/file drops, with live gap animation
  and reduced-motion variants.
- FR-10: Context menus match section 13, including live window lists,
  modifier variants, state tracking, and the Trash confirmation.
- FR-11: The Dock is keyboard-navigable and exposes AT-SPI list/menu roles
  with state-bearing labels; reduced-motion variants exist for every Dock
  animation.
- FR-12: Live settings changes (pin set, size, magnification, position,
  auto-hide, indicators) apply without restart via settingsd signals.
- FR-13: The Dock's input region excludes the transparent magnified band
  and the hidden bar, so clicks pass through to windows beneath.
- FR-14: Continuous Dock animation commits every frame via the
  scene-graph-driven render path (no timer sampling), holding the 60 Hz
  budget.

## Acceptance criteria

- [ ] Core interaction loop steps pass: launch → Dock animation → … →
      minimize → restore from Dock → close (Phase-2 exit).
- [ ] Lifecycle edge-case suite (launch failure, exit mid-animation,
      cross-workspace windows, inconsistent identifiers, identity change)
      scripted and passing.
- [ ] Magnification at 60 Hz on baseline hardware, with the pointer-anchored
      entry staying under the pointer.
- [ ] Trash state integration test with Files and a third-party deletion.
- [ ] Drag rearrangement and context-menu walkthroughs scripted.
- [ ] Keyboard + AT-SPI walkthrough and reduced-motion pass.
- [x] Multi-output: Dock appears on hotplug and per-output popovers do not
      float across outputs. *(Scripted: `overlay_popover_is_per_output` covers
      the popover half; `dock_follows_output_hotplug` attaches a second output
      and asserts the Dock's baseline reserved zone reaches the new output
      specifically and the Dock surface reconfigures. The shell creates its
      Dock with `output = None`, so the compositor renders it on every output.)*

## Test plan

- Nested UI tests for the click tree with a test app producing 0/1/N
  windows on various Spaces.
- Headless: entry model, identity resolution and re-resolution, settings
  keys, GVfs trash events, auto-hide state machine, magnification math
  (anchor, edges, vertical axis).
- Restart tests: shell restart and settingsd/app-index restart re-sync Dock
  state.
- Perf: magnification frame budget measured (Phase perf table); damage-rect
  assertion for the magnified band.
- Accessibility: keyboard walkthrough + per-entry AT-SPI roles; live AT-SPI
  bridge validated early (section 25).

## Risks / open questions

- **App identity misses** are the top UX risk — feed every miss into
  app-index heuristics (T-23).
- **Snapshot renderer** is a hard prerequisite for Dock animation quality;
  landing it is part of this ticket, shared with the T-09 backlog.
- **AT-SPI from the offscreen shell** is unproven; validate the Qt
  accessibility bridge on the session bus before building the a11y surface.
- **Auto-hide translation mechanism** (margin animation vs. oversized
  surface) needs a compositor placement check before implementation.
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

## Decisions required before implementation

These are the only open choices that change the design surface. Recommended
answers are listed so work can start with defaults.

1. **Temporary-entry lifetime** — window-based (MVP rule, section 4.1) vs.
   process-based via a future app-index launch registry. *Recommended:
   window-based now; add the registry hook later.*
2. **Downloads stack and recents** — ship in the first slice or defer
   behind settings (section 17). *Recommended: defer; keep the entry kinds.*
3. **`dock.titlebarDoubleClick` values** — keep `zoom|minimize|none`, or add
   `fill` once T-04 has a Fill state. *Recommended: keep the three; map Fill
   to Zoom in the Settings pane copy.*
4. **`dock.showRecentApps` default** — macOS defaults on; we default off.
   *Recommended: off for MVP, flip when app-index recency is reliable.*
5. **Auto-hide translation** — margin animation vs. oversized surface with a
   rendered offset. *Recommended: margin animation, pending a placement
   check.*
6. **AT-SPI approach** — Qt accessibility bridge on the offscreen shell vs.
   a dedicated shell accessibility shim. *Recommended: spike the bridge
   first; it is a go/no-go for FR-11.*
