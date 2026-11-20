# 0277 — Desktop & Dock, Overview, and Windows rows are one policy surface

- **Status:** Accepted (T276)
- **Date:** 2026-11-15
- **Context:** [24-system-settings.md](../24-system-settings.md) §4.20–4.21,
  [08-settings.md](../08-settings.md), ADR
  [0126](0126-mission-control-hot-corners-adapter.md), ADR
  [0034](0034-compositor-policy-via-shell-bridge.md)

## Context

The 2026-11-14 review found the Desktop & Dock page only partly matched the
the reference design row inventory, the Overview pane had no workspace or Mission
Control rows, hot corners had no modifiers, and the executable map advertised
rows the pane never rendered. T242 (Stage/tiling) and T245b (desktop
items/widgets) are separate tasks and have not landed, so their runtime cannot
be wired here.

The compositor already owns the corner detector, the overview state machine,
and the workspace model; settingsd already owns the corner assignments. What
was missing was one append-only way to apply the whole snapshot, and a row
contract that stays truthful when a runtime is absent.

## Decision

- **One pane owns the macOS page, one owns the Linux surface.** Desktop & Dock
  renders the Dock, Desktop & Stage, Widgets, Windows, Overview links,
  the global-menu row, and the default-browser row. Overview keeps its
  `mission-control` id and renders Overview, workspaces, gestures, Hot
  Corners, and Shortcuts.
- **A missing runtime is disabled with a user-facing reason.** Stage, recent
  apps, windows-from-an-app, widgets, tiling, and the non-tiling Windows rows
  are visible but disabled; their owning task id lives in `SettingsMap.qml`
  and these docs, never in the label or description. No row says “OPEN”.
- **Hot corners carry a modifier and the shell's action vocabulary.** Four
  `overview.hotCorner*Modifier` keys accept `none`/`command`/`shift`/`option`/
  `control`. The action list extends the original five with `launchpad`,
  `control-center`, `show-dock`, `menu-bar`, and `focus-dock`, each mapping to
  an existing `InputAction`. The detector gates its dwell on the held role.
- **One append-only policy request.** Protocol v13 adds
  `df_toplevel_manager.set_overview_policy` carrying the four assignments,
  their modifiers, the normal workspace count, and
  `overview.switchToAppSpace`. The shell sends it from the same
  `CompositorPolicy` snapshot as the motion/input keys. Unknown ids leave the
  current corner untouched.
- **Workspace count is live.** The compositor resizes each output's normal
  Space list in lockstep; removing a Space migrates its windows to the
  remaining Space first. Fullscreen Spaces are transient and never resized.
- **The default-browser row is a delegate.** `defaults.browser` stores the
  chosen desktop id; `SettingsBridge` offers installed `WebBrowser` records
  from app-index and applies the choice through the freedesktop default-apps
  delegate (`xdg-settings`). The Settings portal exposes the key read-only as
  `org.dragonfruit.desktop`. T258 owns the full portal/mimeapps matrix.

Rejected: implementing Stage, tiling, or desktop widgets in T276 (explicitly
out of scope); a second workspace or Dock model; a fake hot-corner toggle that
only persisted a key with no compositor consumer.

## Consequences

- The compositor manager interface is v13; older shells/compositors ignore the
  request and keep their built-in corners.
- `workspaces.count` gains a live owner. A window on a removed Space moves to
  the nearest remaining normal Space instead of pointing at a deleted id.
- The pane tests assert every row either applies live or is disabled with a
  reason, and the map carries the owning task for the disabled rows.
- T242/T245b can flip their rows from disabled to live without changing the
  pane's structure or the protocol.
