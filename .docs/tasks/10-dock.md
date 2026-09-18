# T-10 — Dock

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `shell/dock/` |
| **Depends on** | [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-09](09-menu-bar.md) · [T-23](23-app-index.md) (identity; stub until it lands) |
| **Blocks** | Phase-2 exit (core interaction loop) |
| **Estimate** | L |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [13-roadmap.md](../design/13-roadmap.md) |

## Summary

The macOS-style Dock: pinned apps with running indicators and launch
bounce, temporary entries for unpinned running apps, magnification,
auto-hide, drag rearrangement, contextual menus, window chooser, and the
Files-backed Trash on the right end.

## Background

The Dock obtains window/app state **directly from the compositor** rather
than inferring it through public protocols, using app-index to resolve
`app_id` / `WM_CLASS` to `.desktop` applications
([04-shell.md](../design/04-shell.md)). The Dock's lifecycle polish —
launch failures, app exit mid-animation, windows on other workspaces,
inconsistent identifiers — is explicitly called out as "the difficult part"
in the design doc.

## Scope

### In scope

1. **Click semantics** (the macOS decision tree from
   [04-shell.md](../design/04-shell.md)):
   ```text
   Pinned application
           ├── not running → launch
           └── running
                 ├── one window  → activate
                 └── multiple    → window chooser
   Running but not pinned → temporary Dock entry
   Minimized window       → optionally appear on right side (preference)
   ```
2. **App identity**: entries resolve via `app-index` (T-23): `app_id`
   primary, Xwayland `WM_CLASS` fallback, heuristics for inconsistent
   identifiers.
3. **Visual/interaction behaviors**:
   - **Magnification** (progress-based, interruptible — same gesture
     pipeline rule).
   - **Auto-hide** with reveal on pointer-to-edge (reserved-zone
     bookkeeping via T-07 chrome protocol, interplay with Zoom geometry
     from T-04).
   - **Bounce feedback** as launch attention via `xdg-activation`
     (compositor signals the activation request).
   - **Running indicators**, per-app not per-window.
   - **Drag rearrangement** of pinned entries.
   - **Contextual menus** (Quit, Keep in Dock, Show All Windows, Options).
4. **Window chooser** (multi-window click target): thumbnails of the app's
   windows across workspaces, select → activate (compositor request).
5. **Trash** on the Dock's right end:
   - A Files-backed location whose **badge watches the same GVfs
     `trash://` mount Files does** — one source of truth that also catches
     deletions by other applications ([09-files.md](../design/09-files.md)).
   - Click → open Trash in Files (activation via `org.dragonfruit.Files1` /
     `.desktop` launch); drag files onto it → move to trash; badge shows
     full/empty; context menu offers Empty Trash.
   - Zero direct coupling between the shell and Files — both watch GVfs.
6. **Pinned-app persistence**: default set + user ordering stored under
   desktop settings via `settingsd` (T-15 keys), not a Dock-private file.

### Out of scope

- Files' Trash window behavior ([T-18](18-files-app.md)).
- Dock preferences UI (Settings → Desktop & Dock, T-16) — model hooks here.
- Minimized-windows pane polish beyond the preference flag (Mission
  Control's bottom strip covers restore, T-11).

## Requirements

- FR-1: The full click tree above is implemented and property-tested,
  including: launch failure (bounce stops, notice, no stuck running
  indicator), app exit while animating (animation resolves, entry
  removed), windows opening on other workspaces (indicator + chooser
  correctness).
- FR-2: Magnification is progress-driven and interruptible; constant-time
  per frame (no dropped-frame budget violations).
- FR-3: Auto-hide reveal and hide honor reserved zones and never occlude
  the menu bar; Zoom (T-04) geometry adapts when the Dock auto-hides.
- FR-4: Bounce feedback triggers only on launch-attention events
  (`xdg-activation`), with a maximum duration.
- FR-5: Window chooser lists windows across all Spaces; selection
  activates the window's Space then the window (compositor round-trip).
- FR-6: Trash badge updates from GVfs `trash://` events, including
  deletions made by other applications.
- FR-7: Dock renders on every output; identity inconsistencies (missing
  `.desktop`) degrade to a generic icon + app_id text, never a crash.
- FR-8: Idle Dock: zero polling (all state via compositor events, app-index
  events, GVfs events).

## Acceptance criteria

- [ ] Core interaction loop steps pass: launch → Dock animation → … →
      minimize → restore from Dock → close (Phase-2 exit).
- [ ] Lifecycle edge-case suite (launch failure, exit mid-animation,
      cross-workspace windows, inconsistent identifiers) scripted and
      passing.
- [ ] Magnification at 60 Hz on baseline hardware.
- [ ] Trash badge integration test with Files and a third-party deletion.

## Test plan

- Nested UI tests for the click tree with a test app producing 0/1/N
  windows on various Spaces.
- Restart test: shell restart re-syncs Dock state from compositor.
- Perf: magnification frame budget measured (Phase perf table).

## Risks / open questions

- App identity misses are the top UX risk — feed every miss into
  app-index heuristics (T-23).
- Decide default pinned set (Files, Settings, Terminal?, Browser?) at
  vertical-slice time; trivially changeable.
