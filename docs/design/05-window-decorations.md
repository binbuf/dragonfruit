# Window Decorations (Traffic Lights)

## Summary

Left-side close/minimize/maximize "traffic lights" are achievable with three
levels of quality, bounded by a Wayland limitation we accept from the outset.

## The Wayland constraint

The Wayland `xdg-shell` specification explicitly says an `xdg_toplevel` is, by
default, responsible for its own intended visual representation, including its
title bar and window controls. Modern Wayland applications can draw their
**own** titlebar. The compositor cannot reliably replace part of an
application's pixels without creating ugly overlapping controls or breaking
application interaction.

## Three quality tiers

| Tier | Applications | Quality |
|---|---|---|
| 1 | Our own applications (Settings, Files, …) | **Perfect** — exactly our chosen left-side controls, via our design system (see [10-design-system.md](10-design-system.md)) |
| 2 | Applications accepting server-side decoration | **Very good** — the compositor draws its own titlebar and traffic lights |
| 3 | Applications drawing custom client-side decorations (CSD) | **Best effort** — we do not inject controls |

Tier 3 can be improved through GTK/Qt configuration and themes so cooperative
applications place controls on the left.

## Negotiation

Decoration mode follows `xdg-decoration`:

- A client that requests server-side decoration gets our compositor-drawn
  titlebar (Tier 2).
- A client that never sends a decoration request gets the compositor default,
  which is **SSD** — cooperative-by-default, CSD only when the client asks.
- Expected third-party outcomes: Qt apps typically request SSD (very good);
  GTK apps draw their own headerbars (Tier 3, steerable left via GTK
  settings we document and configure); Electron/Chromium and most SDL games
  are CSD (Tier 3).
- Xwayland windows: the X server is an ordinary `xdg-shell` client and does
  not draw Wayland CSD, so **X11 applications reliably land in Tier 2** —
  compositor-drawn decoration without per-app cooperation.

## Behavior spec

Our decorations, wherever they appear, behave identically:

- Traffic-light glyphs reveal on hover of the button cluster; colorless
  otherwise.
- Double-click on the titlebar zooms; right-click opens the window menu
  (Move to Space, Minimize, Zoom, Close — see
  [03-workspaces.md](03-workspaces.md)).
- Fullscreen windows hide the titlebar; a hover reveal keeps controls
  reachable.

## Product promise

> **"Traffic lights wherever the application permits native desktop
> decoration"** — not "all windows."

We never inject a fake compositor titlebar above every CSD application; that
would make the desktop *less* premium, not more. This is one of the two
accepted compromises (see [00-overview.md](00-overview.md)).

## Decoration policy

- Server-side decoration is offered and preferred for cooperative clients.
- Our decoration visual design (traffic lights, typography, shadows,
  translucency) is owned by the design system so compositor-drawn titlebars
  and first-party app titlebars look identical.
- Third-party decoration themes ship later, as a compatibility work item in
  the roadmap (see [ROADMAP.md](../ROADMAP.md)).

## Implementation status (T-01)

The T-01 loop lands the SSD titlebar element from the generated tokens
(`compositor/src/window/decoration.rs`): a flat token fill with square
corners, left-side close/minimize/zoom lights, and hover/disabled *drawing*.
The titlebar reserves its height as a top inset (`component.titlebar.height`,
40 logical px) above the client area, so a zoomed client is configured one
titlebar shorter through the single `configure_window_size` path; fullscreen
hides the titlebar and reserves nothing. A client that requests CSD via
`xdg-decoration` (or an X11 client marked undecorated) is never given a
compositor titlebar — the tier-3 dignity rule.

Explicitly deferred: real materials (blur/shadow/rounded corners, T-04),
drag-to-move and double-click zoom on the titlebar (T-01.3), the window
menu (T-01.4), and the live color scheme (settings, T-08).

T-01.2 adds the traffic-light interaction on top of the element: the
compositor tracks which button cluster the pointer is over (glyph reveal),
and a left press on a light is consumed before client routing and mapped to
the existing window state machine — close (`xdg_toplevel.close` /
`WM_DELETE_WINDOW`), minimize, and zoom/unzoom. There is no new window state
and no second owner of truth. The titlebar is only hit-tested when no client
surface (toplevel, popup, or input region) and no chrome surface is under the
pointer, so popups, IME, and per-surface input regions keep priority. Reduced
motion is N/A here: no animation is introduced (lifecycle motion is T-02).
