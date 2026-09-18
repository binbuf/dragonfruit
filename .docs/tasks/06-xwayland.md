# T-06 — Xwayland Integration

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `compositor/` (Xwayland) |
| **Depends on** | [T-02](02-compositor-core.md) · [T-04](04-window-model.md) |
| **Blocks** | [T-30](30-compatibility-bridges.md) (X11 app zoo) |
| **Estimate** | M |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [13-roadmap.md](../design/13-roadmap.md) · [05-window-decorations.md](../design/05-window-decorations.md) |

## Summary

X11 application support through Xwayland: launch/management of the X server
as an ordinary `xdg-shell` client, WM_CLASS-based application identity, and
reliable Tier-2 (compositor-drawn SSD) decoration for X11 windows.

## Background

X11 apps rate "good," not perfect ([00-overview.md](../design/00-overview.md)).
Xwayland is the compatibility path; "strange Xwayland applications" are an
explicit compatibility-phase work item
([13-roadmap.md](../design/13-roadmap.md)) — that zoo is T-30. This ticket
delivers the working integration.

## Scope

### In scope

1. Xwayland lifecycle: lazy or eager start (pick one, document), socket
   management, environment (`DISPLAY`) exported to launched apps.
2. X11 window management through the WM path: mapping, focus, stacking,
   transient hints, icons, workspace assignment (all via T-04/T-05
   machinery).
3. **Application identity fallback**: Xwayland `WM_CLASS` resolves to
   `.desktop` applications via `app-index` (T-23) — alongside
   `xdg_toplevel` `app_id` for Wayland clients.
4. **Decorations**: the X server is an ordinary `xdg-shell` client that does
   not draw Wayland CSD, so **X11 applications reliably land in Tier 2**
   (compositor-drawn SSD) without per-app cooperation
   ([05-window-decorations.md](../design/05-window-decorations.md)).
5. Input: keyboard mapping across the X/Wayland boundary, clipboard
   bridging (text/images), DnD bridging (via wlr-data-control-era
   standard paths).
6. Basic robustness: window manager protocol quirks (ICCCM/EWMH) tolerated;
   no crash on malformed X messages.

### Out of scope

- The strange-app zoo, per-app workarounds, and long-tail compatibility
  (T-30).
- X11 screen capture / global hotkeys emulation — out of scope by policy.

## Requirements

- FR-1: Xwayland starts automatically on first X11 client launch; X11 apps
  appear, focus, resize, fullscreen, and close correctly.
- FR-2: X11 windows receive compositor-drawn SSD titlebars with traffic
  lights (with T-13) — the default and only tier for X11.
- FR-3: `WM_CLASS` → application identity works for the common set
  (Firefox X11, Steam, an SDL game, a Java app) for Dock/switcher grouping.
- FR-4: Clipboard text and images cross the boundary in both directions.
- FR-5: Basic DnD (file drag from an X11 app into Files and vice versa)
  works through the same operations engine path
  ([09-files.md](../design/09-files.md)).
- FR-6: Killing an X11 app (or Xwayland itself) never destabilizes the
  session; Xwayland crash restarts cleanly.

## Acceptance criteria

- [ ] Phase-1 exit contribution: "Xwayland windows map."
- [ ] Reference app matrix (Firefox, Steam, one SDL game, xterm) passes a
      scripted interact-and-close run.
- [ ] Identity resolution rate measured; misses logged into the T-30 zoo.

## Test plan

- Headless: Xwayland start/stop, WM_CLASS table.
- Nested: full app matrix under `dragonfruit dev --nested`.
- DRM session: same matrix once T-24 session lands.

## Risks / open questions

- Some X11 games/WM_HINT-heavy apps misbehave; capture cases in T-30.
- Fractional scaling on Xwayland (a known ecosystem pain) — decide policy:
  scale-viewport per X11 surface vs integer scale; document the chosen
  trade-off and revisit in T-31.
