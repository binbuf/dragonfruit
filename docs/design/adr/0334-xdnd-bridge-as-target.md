# 0334 — XDnD bridge-as-target: the root proxy, the bridge window, and a synthetic Wayland drop

## Status

accepted — implements the X11→Wayland half of the XDnD gap (G-07) for T328a.
The Wayland→X11 source role landed in T328b on this same connection, atom
table, and bridge window; see
[ADR 0335](0335-xdnd-bridge-as-source.md). Partially supersedes
[ADR 0099](0099-xdnd-bridge-model-and-documented-gap.md), whose codec and
state machines are unchanged.

## Context

Smithay 0.7's XWM implements no XDnD and does not expose its X connection.
[ADR 0099](0099-xdnd-bridge-model-and-documented-gap.md) landed the wire
codec and the `XdndTarget` state machine and recorded the connection half as
an explicit gap. T328a closes the target role: an X11 drag must reach a
Wayland surface.

The X protocol only offers one intercept point. A drag source resolves the
window under the pointer and sends `XdndEnter` there. A Wayland surface has no
X11 window, so over one the source resolves to Xwayland's root window. A source
follows the target's `XdndProxy` property only when the target is not
`XdndAware`; setting the proxy on managed X11 windows therefore cannot
intercept a drag aimed at an XdndAware client, and doing so would also break
X11→X11 drags.

## Decision

- **One dedicated X connection for the bridge**, separate from the XWM's
  connection and from the `_NET_FRAME_EXTENTS` property connection. It lives
  in the library as `compositor/src/xdnd_bridge.rs` (`XdndBridge`) so the
  conformance test can drive it against a live Xwayland server without a
  compositor. T328b reuses it; no second connection is created.
- **The bridge publishes the root `XdndProxy`** and creates an
  override-redirect, unmapped, InputOnly 1×1 bridge window with
  `XdndAware = 5`. A source that resolves the pointer to the root follows the
  proxy; X11 windows keep receiving their own drags directly. The bridge is
  not a second window manager.
- **The bridge owns the target exchange**: it interns the 17 standard atoms
  plus `text/uri-list` and a private selection property, decodes
  `Enter`/`Position`/`Drop`/`Leave`, answers `XdndStatus` (a
  `text/uri-list` drag is accepted with `XdndActionCopy`, every other flavour
  is refused) and `XdndFinished`, and reads the `XdndSelection` with
  `ConvertSelection`. Direct and `INCR` transfers are supported, bounded to
  2 s and 1 MiB.
- **The Wayland half is a synthetic drop.** The X11 source drops on button
  release, so the physical press is already over. The compositor installs a
  pointer grab at the current location, sends a `text/uri-list` offer to the
  surface under the pointer, and injects the release once the client accepted
  the MIME type and an action (750 ms fallback cancels). The payload is served
  from a detached writer thread, so a stalled reader cannot block the event
  loop.
- **`XdndFinished` reports success when the payload has been read.** Only Copy
  is accepted, so a Wayland client that later declines cannot lose the
  source. Hover-time negotiation is not mirrored; the client still gates the
  drop through `accept`/`set_actions`.

## Consequences

- A file dragged from an X11 application to a Wayland surface that is not
  geometrically under an XdndAware X11 window is delivered as a normal
  `text/uri-list` data offer. An XdndAware X11 window under the pointer still
  receives its own drag; this is an X protocol limit, recorded as a follow-up,
  not a regression.
- X11→X11 drags are untouched: no managed window carries a proxy.
- The bridge survives an Xwayland restart by being dropped and recreated with
  the new connection; a dead connection tears its calloop source down.
- The conformance suite covers the target exchange (direct and INCR), the
  root proxy, and the refusal of a non-file drag. The live X11→Files check is
  a manual nested-session step recorded in the task hand-off.

Alternatives rejected: setting `XdndProxy` on managed windows (sources ignore
it for XdndAware targets, and it would break X11→X11); a full-screen input
window (would steal X11 input); patching or vendoring Smithay; a separate
bridge process.
