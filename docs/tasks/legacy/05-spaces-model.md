# T-05 — Spaces: The Compositor Workspace Model

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `compositor/` (workspaces) |
| **Depends on** | [T-04](04-window-model.md) · [T-02](02-compositor-core.md) |
| **Blocks** | [T-06](06-xwayland.md) (workspace assignment) · [T-07](07-private-shell-protocols.md) (workspace interface) · [T-11](11-mission-control-workspace-ux.md) · [T-12](12-app-switcher.md) (Space activation on switch) · [T-13](13-window-decorations-ssd.md) (Move-to-Space menu) · [T-14](14-hot-corners-desktop-background.md) (per-Space wallpaper) |
| **Estimate** | L |
| **Design docs** | [03-workspaces.md](../../design/03-workspaces.md) · [ROADMAP.md](../../ROADMAP.md) |

## Summary

Workspace organization as an **internal compositor primitive**: per-display
ordered Space lists, lockstep switching, fullscreen-window Spaces, per-Space
compositor-rendered wallpaper, window→Space assignment, minimized-window
exclusion, and display-hotplug preservation. Workspace events broadcast over
the private protocol; the shell never keeps a second copy.

## Background

Spaces and Mission Control are the strongest reasons we own the compositor:
they operate on real live surfaces, not screenshots
([03-workspaces.md](../../design/03-workspaces.md)). The first vertical slice
ships **three workspaces** ([ROADMAP.md](../../ROADMAP.md)).

## Scope

### In scope

1. **Model decisions** (all from
   [03-workspaces.md](../../design/03-workspaces.md)):
   - The compositor owns workspace **creation, removal, and ordering**.
   - **Spaces are per-display and ordered.** Each output has its own ordered
     Space list; a switch gesture advances the active Space on **every
     display in lockstep** (macOS semantics).
   - **Fullscreen windows occupy a dedicated Space** that exists only while
     the window is fullscreen and appears in the workspace strip accordingly
     (interplay with T-04 fullscreen state).
   - **Each Space carries its own wallpaper**, compositor-rendered as part
     of the workspace scene so the background slides with the Space during
     switches — **the shell never draws the desktop background**.
   - **Windows belong to applications, not Spaces.** Apps remember the Space
     they were assigned to; windows move between Spaces via the window menu
     ("Move to Space") or by dragging in the overview (T-11).
   - **Minimized windows are excluded from the layout** and shown as a
     separate bottom strip in Mission Control (T-11), restorable by click.
   - **Display hotplug preserves the model**: a newly attached output gets
     its own fresh Space list; detaching an output **migrates that output's
     windows to the current Space of the remaining primary output** before
     its Spaces are destroyed.
2. **Workspace events** broadcast over the private protocol (with T-07):
   created / removed / reordered / activated / window-assigned.
3. **Workspace switching** as a compositor animation primitive: live
   surfaces repositioned with scale/translation/blur, gesture-driven
   progress, reversible interaction — the actual UX polish is T-11; this
   ticket delivers the scene mechanics + event stream.
4. **Wallpaper subsystem**: per-Space wallpaper source images, per-output
   rendering into the workspace scene (fills, scales, colors), later
   configurable per Space via Settings/Wallpaper pane (T-16).
5. **App Space memory**: per-application assignment that survives window
   close/reopen within a session — keyed in Phase 1 by raw `app_id`
   (Wayland) or `WM_CLASS` (Xwayland, with T-06); refined to app-index
   identity when [T-23](23-app-index.md) lands, so this ticket has **no
   forward dependency on Phase 4**.

### Out of scope

- Mission Control overview state machine and commit rules (T-11).
- Workspace strip UI (shell; T-11).
- Wallpaper *picker* UI (T-16) — only the rendering + storage model here.

## Requirements

- FR-1: Create/remove/reorder/activate workspaces via compositor API; the
  shell does each of these through the private protocol only.
- FR-2: Per-display ordered lists: N outputs × M spaces each; switching
  advances all displays in lockstep; direct activation of a specific Space
  synchronizes all displays.
- FR-3: Fullscreen window auto-creates a Space, appears in the strip order,
  and destroys it (returning to the origin Space) on unfullscreen — the
  origin Space is remembered.
- FR-4: Wallpaper renders per Space, per output, and translates/scales with
  its Space during a switch (verified visually frame-by-frame in nested
  mode).
- FR-5: "Move to Space" moves a window; app Space memory reopens new
  windows of the same app on that Space.
- FR-6: Minimized windows are excluded from Space layout; restoring
  (Dock/overview) returns them to their Space.
- FR-7: Hotplug attach → new output with fresh Space list; detach → windows
  migrate to the remaining primary output's current Space, detached
  Spaces destroyed; no window is lost (scripted hotplug test).
- FR-8: Workspace events broadcast atomically with the scene change (no
  frame where shell state and compositor state disagree).

## Acceptance criteria

- [ ] Three workspaces in the vertical slice, switchable by keyboard
      shortcut (with T-03) and swipe gesture (with T-11 pipeline).
- [ ] Lockstep multi-monitor switch verified on a two-output headless/DRM
      test.
- [ ] Fullscreen→own-Space→unfullscreen→origin-Space round-trip passes.
- [ ] Hotplug matrix (attach/detach × windows present) passes with zero
      window loss.
- [ ] Shell holds no shadow workspace state (assert only one owner of
      workspace truth in integration test).

## Test plan

- Unit: Space list operations, migration logic.
- Headless: event broadcast ordering, fullscreen Space lifecycle.
- VM + hardware: hotplug, multi-monitor lockstep, wallpaper rendering.

## Risks / open questions

- Migration-on-detach ordering (which Space is "primary" when outputs are
  equal) — define: highest-priority remaining output per output-management
  priority; document.
- App Space memory persistence across compositor restarts is session-scope
  only; decide whether `settingsd` should persist assignments (defer).
