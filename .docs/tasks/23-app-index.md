# T-23 — app-index: Application Identity Service

| | |
|---|---|
| **Phase** | 4 · System integration |
| **Area** | `services/app-index/` |
| **Depends on** | [T-01](01-repo-scaffolding-ci-licensing.md) |
| **Blocks** | [T-10](10-dock.md) · [T-12](12-app-switcher.md) (identity) · [T-16](16-settings-app.md) (Search pane later) |
| **Estimate** | M |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [01-architecture.md](../design/01-architecture.md) · [08-settings.md](../design/08-settings.md) |

## Summary

`app-index`: the single owner of application identity — resolving
`app_id` / Xwayland `WM_CLASS` to `.desktop` entries (icon, name, launch
semantics) for the Dock, app switcher, and shell — with heuristics for
inconsistent identifiers, and (later) the desktop search index for the
Spotlight-equivalent.

## Background

State ownership: application identity (`app_id` → `.desktop`, icon, name)
belongs to `app-index`
([01-architecture.md](../design/01-architecture.md)). The resolver must
maintain fallbacks ([02-compositor.md](../design/02-compositor.md)):
`xdg_toplevel` `app_id` (primary, Wayland clients) → Xwayland `WM_CLASS`
(X11 clients) → heuristics for inconsistent identifiers. Restartable;
consumers re-query on reappearance.

## Scope

### In scope

1. **Resolution pipeline**:
   - Primary: `xdg_toplevel.app_id`.
   - Fallback: Xwayland `WM_CLASS` (from T-06).
   - Heuristics: fuzzy match against installed `.desktop` entries
     (case/normalization, `StartupWMClass`, executable-name matching) for
     apps with inconsistent identifiers (Electron portables, Java apps,
     some Flatpaks).
   - Output: name, icon (themed), `.desktop` entry, launch semantics
     (Exec, actions, `StartupWMClass`), Flatpak-aware via
     `security-context` info where available.
2. **Index maintenance**: watch `.desktop` directories
   (`/usr/share/applications`, `$XDG_DATA_HOME/applications`, Flatpak
   exports), incremental updates, no polling at idle beyond monitor
   events.
3. **D-Bus interface** `org.dragonfruit.AppIndex1` (versioned, additive):
   resolve(identity) → app record; enumerate; subscribe
   (installed/uninstalled/updated events) so Dock/switcher stay live.
4. **Launch API**: launch with activation tokens (feeds Dock bounce via
   `xdg-activation`), file-argument launches, activation requests for
   already-running apps.
5. **Search foundation (later, but designed now)**: the Settings **Search
   pane** routes to app-index (Spotlight-equivalent, roadmap "later") —
   schema hooks for name/keyword/metadata search over applications; Files'
   content search is *not* here ([09-files.md](../design/09-files.md) —
   we never write an indexer beyond app metadata).
6. **Heuristic learning**: every resolution miss is logged and folded back
   into the heuristic table (the Dock's identity-miss risk from T-10).

### Out of scope

- File associations (`mimeapps.list` — GIO/AppInfo in files-core; app-index
  is explicitly *not* involved there, per
  [09-files.md](../design/09-files.md)).
- File-content indexing or a general Spotlight daemon — the search story
  beyond app metadata stays "later" and must not become an indexer
  project.

## Requirements

- FR-1: Resolution matrix: common Wayland apps (app_id exact), Qt, GTK,
  Electron, Xwayland `WM_CLASS`, and at least 5 known-inconsistent cases
  resolve correctly; misses degrade to a generic record, never a crash.
- FR-2: Install/uninstall reflects in Dock and app switcher without
  restart (event-driven).
- FR-3: Launch returns an activation token the compositor honors for
  Dock-bounce feedback (with T-02/T-10).
- FR-4: Restart-safe: consumers re-query on reappearance; no Dock/switcher
  restart required.
- FR-5: Idle cost: zero (monitor-driven only) — idle-desktop budget.
- FR-6: Search hooks: name/keyword query API returns ranked apps —
  good enough for the Settings Search pane when it ships.

## Acceptance criteria

- [ ] Resolution matrix green including heuristic tier.
- [ ] Live install/uninstall demo (install an app while the session runs;
      Dock updates).
- [ ] Restart drill passes (broker pattern from T-22/T-15).

## Test plan

- Unit tests over a fixture `.desktop` corpus (weird Exec lines, missing
  icons, duplicate ids).
- Integration: launch + activation-token round trip with the compositor.

## Risks / open questions

- Flatpak/Snap identifier quirks — treat as heuristics data, not code
  paths.
- Decide whether the window→app grouping used by the switcher lives here
  or in the compositor; per design, compositor owns windows, app-index
  owns identity — the compositor consults app-index. Keep it that way.
