# T-27 — Portal Backend: xdg-desktop-portal-dragonfruit

| | |
|---|---|
| **Phase** | 5 · Desktop infrastructure |
| **Area** | `portal/` |
| **Depends on** | [T-02](02-compositor-core.md) (capture path) · [T-17](17-files-core.md) (FileChooser) · [T-24](24-session-lifecycle.md) (`portals.conf`, session env) · [T-25](25-notifications-and-osd.md) (notification proxying if needed) |
| **Blocks** | Daily-driver bar (portals) · Phase-5 exit (Flatpak browser matrix) · [T-28](28-screenshot-recording-ui.md) |
| **Estimate** | L |
| **Design docs** | [07-system-integration.md](../design/07-system-integration.md) · [02-compositor.md](../design/02-compositor.md) · [09-files.md](../design/09-files.md) · [11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md) |

## Summary

`xdg-desktop-portal-dragonfruit`: our DE-specific portal backend providing
the native file chooser (Files in chooser mode), screenshots, screen
sharing (PipeWire streams with explicit consent), GlobalShortcuts, and the
`portals.conf` wiring — so browsers, Electron programs, Flatpak apps, and
conferencing programs work out of the box.

## Background

Portal support arrives **surprisingly early** once the desktop is a real
Wayland session
([11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md)).
The portal project is designed around a common frontend with
DE-specific backends ([07-system-integration.md](../design/07-system-integration.md)).
The FileChooser is delegated to **Files in chooser mode** over the same
`files-core` — one browsing implementation, one set of semantics; on macOS
the open/save panel *is* the file manager's
([09-files.md](../design/09-files.md)). Capture is portal-only by policy
([02-compositor.md](../design/02-compositor.md)).

## Scope

### In scope

1. **FileChooser**: delegate to Files chooser mode — the same `files-core`
   models, collation, trash semantics, and views (a small Files window);
   the portal frontend handles document-store export for sandboxed
   callers; open/save/file-picker variants; remember-recent integration
   (GVfs `recent://`).
2. **Screenshot**: single-frame capture requests serviced through the
   compositor's capture path; consent gate (on first use / per policy);
   per-window / per-monitor selection semantics surfaced to T-28's UI.
3. **Screen sharing**: PipeWire streams per-window or per-monitor with
   **explicit user consent** each time policy requires; the only capture
   path in the system (the compositor refuses non-portal grabs — T-02
   FR-8).
4. **GlobalShortcuts**: sandboxed applications register shortcuts here;
   the portal talks to the compositor's global shortcut engine (T-03)
   with the same focus rules.
5. **Wallpaper, Settings (read-only), Inhibit, Notification-forwarding**
   portal interfaces where they map cleanly onto our services — implement
   the set the reference matrix needs; document deliberately-skipped
   interfaces.
6. **`portals.conf`**: restrict portal backends to
   `xdg-desktop-portal-dragonfruit` plus the generic/GTK backends so
   Flatpak apps work out of the box
   ([12-packaging.md](../design/12-packaging.md)).
7. **Registration**: `XDG_CURRENT_DESKTOP=dragonfruit` selects the backend
   (T-24 environment); `security-context` metadata respected for
   sandboxed callers (identity in consent prompts).
8. **Restartability**: fails soft — an unavailable backend yields
   portal-level fallbacks (generic backends), never a broken app
   ([01-architecture.md](../design/01-architecture.md)).

### Out of scope

- The screenshot/recording **UI** (T-28) — this ticket services requests;
  that ticket draws the selection overlay.
- Files' chooser-mode UI internals (T-18 covers the shared view stack;
  chooser bindings here).

## Requirements

- FR-1: **Phase-5 exit — portals work for a Flatpak browser: file chooser,
  screenshot, screen share.**
- FR-2: FileChooser chooser-mode shares every semantic with Files windows
  (collation, sort, hidden toggle, spring-load if applicable, recent
  locations) — the "one implementation" hard rule
  ([09-files.md](../design/09-files.md)).
- FR-3: Screenshot + screencast consent flow: per-request user approval
  with per-window/per-monitor choice; locked session refuses (with T-26).
- FR-4: GlobalShortcuts round-trip: register → trigger while app focused →
  event delivered; conflicts with system shortcuts resolve per T-03 rules.
- FR-5: Portal backend selection works on a real session (browser test
  verifies `xdg-desktop-portal` routes to us).
- FR-6: Kill/restart of the portal service mid-request: callers receive
  errors, never hangs (fails soft).

## Acceptance criteria

- [ ] Flatpak browser matrix passes (chooser, screenshot, screen share) —
      Phase-5 exit criterion.
- [ ] Non-Flatpak (Electron, conferencing app) spot checks pass.
- [ ] Consent dialogs render in our design system (with T-29's agent
      styling where polkit is involved).
- [ ] Capture-only-through-portal invariant holds (re-test T-02 FR-8 end
      to end).

## Test plan

- Portal conformance tests (upstream `xdg-desktop-portal` test suite
  where available) plus our matrix.
- Sandboxed-caller identity tests via `security-context`.
- Kill drills on the portal service.

## Risks / open questions

- Interface churn upstream — pin per Fedora 44, additive tracking.
- Screen-share restore-after-suspend / monitor-hotplug during a stream —
  define stream teardown behavior explicitly (coordinate T-31 soak).
