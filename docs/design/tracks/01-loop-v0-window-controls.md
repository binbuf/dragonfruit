# T-01 — Loop v0: Window Controls and the First End-to-End Demo

> **Track, not a single slice.** This file is the design reference. It is executed as 7 one-session tasks: [T-01.1](../../tasks/001-t-01.1-titlebar-render-element.md) · [T-01.2](../../tasks/002-t-01.2-traffic-light-actions.md) · [T-01.3](../../tasks/003-t-01.3-titlebar-drag-double-click-fullscreen-reveal.md) · [T-01.4](../../tasks/004-t-01.4-window-menu.md) · [T-01.5](../../tasks/005-t-01.5-decoration-tier-policy-and-x11-correctness.md) · [T-01.6a](../../tasks/006-t-01.6a-make-demo-harness.md) · [T-01.6b](../../tasks/007-t-01.6b-loop-integration-capture.md). Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 1 of 17 — the walking skeleton |
| **Area** | `compositor/` (SSD render element, window menu) · `shell/` (wiring only) · `scripts/` (demo) |
| **Depends on** | inherited foundation only |
| **Blocks** | T-02, T-03, T-04, T-07 |
| **Legacy detail** | [legacy/13-window-decorations-ssd.md](../../tasks/legacy/13-window-decorations-ssd.md) · [legacy/04-window-model.md](../../tasks/legacy/04-window-model.md) · [legacy/10-dock.md](../../tasks/legacy/10-dock.md) · [05-window-decorations.md](../05-window-decorations.md) |

## Demo (the thing you can watch)

```
make demo
  → nested compositor + shell + one Qt/Wayland app + one X11 app
  → click the app in the Dock
  → the window appears with our titlebar and traffic lights
  → drag it by the titlebar, double-click to zoom, click yellow to minimize,
    restore it from the Dock, click red to close
```

Capture: `docs/captures/t01-loop-v0.*` (a short recording plus stills of the
Wayland, X11, and CSD cases), reviewed against the design docs and the
design-system gallery goldens.

## Why this is first

Windows currently have **no decorations**: `xdg-decoration` negotiates
ServerSide by default but nothing draws a titlebar, and there is no
user-facing way to move, zoom, minimize, or close a window. Until that
exists, the Dock and menu bar cannot be honestly evaluated — their core loop
steps are unreachable. This is the smallest change that turns the existing
machinery into an experience.

## Inherited and reused

- Window state machine, interactive move/resize grabs, window-menu primitives
  (`legacy/04-window-model.md`).
- Dock click tree, launch, minimized entries, `activate_app`,
  `select_overview_toplevel`, `df_toplevel.close` (`legacy/10-dock.md`).
- xdg-activation first-window focus (`legacy/10-dock.md`, "xdg-activation
  focus slice").
- Decoration tier marking (`WindowModel::DecorationTier`, X11 Tier-2) and
  `SizeConstraints` (`legacy/06-xwayland.md`).
- `SsdTitlebarReference.qml`, `TitleBar`/`TrafficLights` tokens
  (`legacy/08-design-system.md`).
- `DfState::configure_window_size` as the single geometry push point
  (`legacy/06-xwayland.md`).

## Scope

### In

1. **Functional SSD render element** for Tier-2 windows:
   - titlebar geometry from the design-system tokens; a flat fill from tokens
     (real materials are T-04; square corners are acceptable here and are
     recorded as deferred).
   - traffic lights close / minimize / zoom with hover reveal and disabled
     states; left-side placement.
   - drag-to-move, double-click honoring `dock.titlebarDoubleClick` (default
     `zoom`), fullscreen hover reveal.
   - the window menu (Move to Space, Minimize, Zoom, Close) on
     right-click/Control-click of the titlebar.
   - titlebar participates in the compositor's input hit-testing; the client
     area's input regions are untouched.
2. **Wire the controls to the existing state machine** — no new window state,
   no new owner of truth.
3. **Tier policy honored**: a client requesting SSD gets our titlebar; a CSD
   client keeps its own; X11 Tier-2 gets the same titlebar.
4. **`make demo`**: one command that builds, launches the nested session with
   the shell and two apps (a Qt app and an X11 app), and prints the demo
   checklist. It must be the same path CI uses for the scripted half.
5. **Menu-bar and Dock integration check**: focused app name updates, running
   indicator, minimized entry appears, restore works, close resolves the Dock
   entry (all existing paths — this slice proves them through the new UI).
6. **Scripted coverage**: titlebar hit-test plus close/minimize/zoom over the
   protocol for a Wayland and an X11 client; added to `make e2e`.

### Out / explicitly deferred

- Real shadows, blur, rounded corners (T-04).
- Appear/close/minimize/restore motion (T-02).
- Third-party decoration themes (T-14).
- Desktop icons (post-gate backlog).

## Acceptance

- [ ] The demo checklist runs in a live nested session and the capture is
      committed.
- [ ] A Qt app (SSD), an X11 app (Tier-2), and a CSD app all render correctly:
      the first two get our titlebar, the third does not.
- [ ] Pointer drag, double-click zoom, minimize, restore-from-Dock, and close
      all work through the UI.
- [ ] The window menu's four commands work through the UI.
- [ ] Headless conformance tests for both client types stay green in
      `make e2e`.
- [ ] `make soak` passes.
- [ ] Reduced motion: no animation in this slice (recorded as N/A).

## Test plan

- Headless: map a Wayland toplevel, assert the titlebar render element exists
  with the expected insets; drive close/minimize/zoom through the titlebar
  hit-test with the synthetic-input harness.
- Headless: repeat the close/minimize path for an `x11rb` window through
  Xwayland.
- Nested: the demo checklist plus a capture review.
- Regression: existing `window_conformance`, `xwayland_conformance`,
  `shell_protocol_conformance` suites.

## Risks

- **Double decoration.** Verify a client that already draws CSD is not also
  given a titlebar, and that an SSD client does not draw its own.
- **Hit-testing.** The titlebar must sit in the compositor's element list
  without breaking popups, IME, or client input regions.
- **Geometry churn.** All size changes must keep flowing through
  `configure_window_size`; do not add a second geometry path.

## Hand-off

- T-02 consumes the titlebar/window geometry to animate from and to the Dock
  tile.
- T-04 replaces the flat titlebar fill with the material pass; keep the fill
  pluggable.
- T-14 reuses the window menu for decoration themes.
