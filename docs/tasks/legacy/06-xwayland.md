# T-06 — Xwayland Integration

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `compositor/` (Xwayland) |
| **Depends on** | [T-02](02-compositor-core.md) · [T-04](04-window-model.md) · [T-05](05-spaces-model.md) (workspace assignment) |
| **Blocks** | [T-13](13-window-decorations-ssd.md) (Xwayland Tier-2 decoration) · [T-30](30-compatibility-bridges.md) (X11 app zoo) |
| **Estimate** | M |
| **Design docs** | [02-compositor.md](../../design/02-compositor.md) · [05-window-decorations.md](../../design/05-window-decorations.md) · [09-files.md](../../design/09-files.md) · [ROADMAP.md](../../ROADMAP.md) |

## Summary

X11 application support through Xwayland: launch/management of the X server
as an ordinary `xdg-shell` client, WM_CLASS-based application identity, and
reliable Tier-2 (compositor-drawn SSD) decoration for X11 windows.

## Background

X11 apps rate "good," not perfect ([00-overview.md](../../design/00-overview.md)).
Xwayland is the compatibility path; "strange Xwayland applications" are an
explicit compatibility-phase work item
([ROADMAP.md](../../ROADMAP.md)) — that zoo is T-30. This ticket
delivers the working integration.

## Scope

### In scope

1. Xwayland lifecycle: lazy or eager start (pick one, document), socket
   management, environment (`DISPLAY`) exported to launched apps.
2. X11 window management through the WM path: mapping, focus, stacking,
   transient hints, icons, workspace assignment (all via T-04/T-05
   machinery).
3. **Application identity fallback**: Xwayland `WM_CLASS` resolves to
   `.desktop` applications — alongside `xdg_toplevel` `app_id` for
   Wayland clients. Resolution is owned by `app-index` (T-23, Phase 4);
   until it lands, an interim direct GIO `AppInfo` lookup covers the
   common set, and every miss feeds the T-23 heuristics.
4. **Decorations**: the X server is an ordinary `xdg-shell` client that does
   not draw Wayland CSD, so **X11 applications reliably land in Tier 2**
   (compositor-drawn SSD) without per-app cooperation
   ([05-window-decorations.md](../../design/05-window-decorations.md)).
5. Input: keyboard mapping across the X/Wayland boundary, selection
   bridging (text/images/files — surfaced to the shell's clipboard
   manager via `wlr-data-control`), and DnD bridging via the standard
   data-device ⇄ XDnD translation
   ([02-compositor.md](../../design/02-compositor.md)).
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
- FR-4: Clipboard text, images, and file lists cross the boundary in both
  directions (daily-driver clipboard bar,
  [ROADMAP.md](../../ROADMAP.md)).
- FR-5: Basic DnD (file drag from an X11 app into Files and vice versa)
  works through the same operations engine path
  ([09-files.md](../../design/09-files.md)).
- FR-6: Killing an X11 app (or Xwayland itself) never destabilizes the
  session; Xwayland crash restarts cleanly.

## Acceptance criteria

- [x] Phase-1 exit contribution: "Xwayland windows map." (headless
      conformance test maps a real X11 client and observes it in
      `_NET_CLIENT_LIST`; `xmessage` renders nested)
- [ ] Reference app matrix (Firefox, Steam, one SDL game, xterm) passes a
      scripted interact-and-close run. *(Not run on the dev host — no
      spare GPU session/Steam; `xmessage` + a raw `x11rb` client are the
      local stand-ins. First T-30 job.)*
- [x] Identity resolution rate measured with the interim resolver;
      misses feed the app-index heuristics (T-23), behavioral oddities
      the T-30 zoo. (`DfState::dump_stats` prints resolved/unresolved and
      the miss list; `AppResolver` unit tests cover the rules.)

### Known gaps

- **FR-5 (XDnD) is unmet**: Smithay 0.7's XWM has no XDnD translation, so
  file drag-and-drop across the X/Wayland boundary is not implemented
  (see PROGRESS.md; T-30/T-31).

## Test plan

- Headless: Xwayland start/stop, WM_CLASS table.
- Nested: full app matrix under `dragonfruit dev --nested`.
- DRM session: same matrix once T-24 session lands.

## Risks / open questions

- Some X11 games/WM_HINT-heavy apps misbehave; capture cases in T-30.
- Fractional scaling on Xwayland (a known ecosystem pain) — decide policy:
  scale-viewport per X11 surface vs integer scale; document the chosen
  trade-off and revisit in T-31.
