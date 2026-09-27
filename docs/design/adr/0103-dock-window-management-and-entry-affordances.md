# 0103 — Dock grouped-window popover, entry affordances, and overflow

## Status

accepted

## Context

A 2026-09-27 UI/UX review compared the Dock with a mature reference dock
(`punchi-dock-remastered`, a KDE Plasma 6 plasmoid: GPL-3.0-or-later, behavior
reference only — no code, assets, or strings are copied; see the clean-room note
below). It found the Dock's *window-management* surface is the least finished
part of an otherwise strong dock:

- the window chooser is a passive text list with no per-window actions and no
  row limit (`shell/dock/DockWindowChooser.qml`);
- an entry with several windows shows no count, so grouping is invisible;
- the chooser is click-only, never retargets, and its anchor is a live delegate
  that a pin-reorder or badge change can destroy (the T-14.7c model-reset
  follow-up);
- overflow silently clamps the icon size and hides temporary/recent entries
  (legacy T-10 §5.1) instead of offering an affordance;
- Empty Trash is a synchronous call with a confirmation and no busy/result
  state (`TrashBridge::empty`);
- minimized windows never acknowledge their Dock entry;
- the Dock's entry tile rect is never handed to the compositor, so appear/
  minimize/restore motion uses the centered fallback (`PROGRESS.md` T-02.2);
- the Dock has no keyboard reordering.

## Decision

- **The grouped-window popover is a first-class surface.** It gains per-window
  Close and Minimize/Restore actions, a bounded row viewport with scrolling, and
  the entry gains a window-count badge. Two windows or more is the grouping
  case; zero/one window keeps the current activate/restore click.
- **Hover-open is opt-in.** A new `dock.chooserOnHover` key (default off) lets a
  dwell open the chooser and a pointer move retarget it along the Dock; the
  default preserves the macOS click-to-choose contract. The popover uses a
  snapshot anchor that survives a delegate rebuild.
- **Overflow gets a terminal affordance.** When running groups do not fit, the
  last slot becomes an overflow cell opening a "More windows" list. Pinned
  entries are never hidden; the icon-size clamp stays the final fallback. This
  **supersedes** legacy T-10 §5.1 for the running-groups case only.
- **Empty Trash is asynchronous** with idle/busy/succeeded/failed states and a
  Try Again path. Determinate `N of M` progress waits on a files-core progress
  seam and is deferred.
- **Minimize-to-icon reaction is opt-in** (`dock.minimizeReaction`, default off)
  and has a reduced-motion variant.
- **The entry tile rect is handed to the compositor at launch**
  (`df_toplevel_manager.set_launch_origin`, v4) so native window motion targets
  the real icon.
- **Keyboard reordering joins the Dock keyboard model** (`Ctrl+Shift+Arrow`
  moves the focused pinned entry).
- **Live window thumbnails are deferred.** Our popovers are offscreen QML
  surfaces and the compositor deliberately draws live surfaces, not thumbnails;
  a per-window preview protocol is a separate project. The popover's row model
  keeps room for a `thumbnail` field so it can slot in without a rewrite.

## Consequences

- `shell/dock/DockWindowChooser.qml`, `shell/dock/Dock.qml`,
  `shell/src/dockprojection.*`, `shell/src/shellcontroller.*`, and
  `shell/src/shellprotocol.*` grow; the chooser becomes the reusable
  window-management surface the overflow cell and hover path share.
- New settings keys (`dock.chooserOnHover`, `dock.minimizeReaction`) carry
  schema, persistence, `docs/settings-keys.md`, and the Desktop & Dock pane;
  settingsd stays the only writer.
- `set_launch_origin` gains its first caller; the compositor remains the motion
  owner and needs no new protocol.
- The one-popover-at-a-time invariant and the pre-sized popover buffer
  (ADR 0100) still hold; the anchor snapshot must not allocate a new surface.
- Clean room: the reference is behavior only. Its GPL-3.0-or-later license
  forbids copying code, assets, or strings into this MIT repo; implement from
  our own design-system components and tokens.

## References

- [04-shell.md](../04-shell.md) "Dock" and "Dock activation and launch".
- Legacy [10-dock.md](../../tasks/legacy/10-dock.md) §9 (chooser), §5.1
  (overflow), §13 (menus), §16 (Trash), §20 (keyboard).
- ADRs [0089](0089-dock-plate-geometry-and-live-panel-rect.md),
  [0092](0092-dock-folder-stacks-and-folder-pins.md),
  [0100](0100-dock-motion-phase-map-and-popover-buffer.md),
  [0101](0101-dock-activation-result-and-launch-display.md).
- Units T-14.7l–t in [ROADMAP.md](../../ROADMAP.md).