# 0099 — The XDnD bridge is a codec + translation model now; the X11 connection half is a documented gap

## Status

accepted — implements (partially) the XDnD item of
[14-global-menu-app-index-compat.md](../tracks/14-global-menu-app-index-compat.md)
and closes the "known gap" recorded by legacy T-06 into an explicit,
test-covered boundary for T-17.

## Context

File drag-and-drop across the X11/Wayland boundary is the task T-14.5 was
budgeted to close. Smithay 0.7 does not implement XDnD: its `XwmHandler` trait
exposes no client-message hook, `X11Wm`'s `RustConnection` and atom table are
private, and `X11Surface` cannot send X client messages. A bridge therefore
needs both a protocol model and its own X11 connection (the wlroots `xwm.c`
approach), plus a compositor-side Wayland drag (Smithay's `start_dnd` exists,
but the X→Wayland half still needs the X connection to observe `XdndEnter`
messages on a bridge window).

One session cannot land a reliable, testable bridge that owns an X connection
and drives `start_dnd`. The task allows the alternative: "or the gap is
explicitly documented at T-17." Doing both — a real, tested protocol model now
and an explicit gap note — narrows the work for the follow-up without faking
an untested integration.

## Decision

- **The protocol half lives in `compositor/src/xdnd.rs`** (exposed through the
  new `dragonfruit-compositor` library target): the 17 standard XDnD atom
  names, the `XdndEnter`/`Position`/`Status`/`Leave`/`Drop`/`Finished` client
  message codec (format-32 `[u32; 5]`), the `text/uri-list` parse/format and
  `file://` percent-coding, and the two state machines (`XdndTarget` for the
  X11-source direction, `XdndSource` for the Wayland-source direction). It has
  no `x11rb`/Smithay dependency so the wire format is unit-testable.
- **The X11 connection half is not wired.** No bridge window is created, no
  atoms are interned at runtime, and no `SelectionRequest`/`XdndSelection`
  transfer is served. `compositor/src/xwayland.rs` keeps its current selection
  bridge unchanged.
- **Conformance is proven against a live Xwayland server**: the new
  `compositor/tests/xdnd_conformance.rs` interns the atoms, advertises an
  `XdndAware` window, and round-trips `Enter`/`Position`/`Status`/`Finished`
  through real X ClientMessages, then drives `XdndTarget` to show a
  `text/uri-list` drag is accepted and a non-file drag refused.
- **The gap is recorded where the gate reads it**: a "Known compatibility
  gaps" note in [17-premium-gate.md](../tracks/17-premium-gate.md) states that
  XDnD file drops across the boundary are not live and are waived at T-17
  unless the connection half lands first.

Alternatives rejected: patching or vendoring Smithay (violates the exact-pin
policy in [14-risks.md](../14-risks.md)); a separate bridge process (larger
than the remaining compositor work and needs its own portal/seat surface); a
synchronous `x11rb` connection inside `DfState` (Smithay types are not
`Send`, and one connection cannot safely drive both the XWM and a drag).

## Consequences

- The wire format and translation decisions are frozen and tested; a later
  session adds only the connection/runtime half: a bridge X window with
  `XdndAware`, atom interning, watching `ClientMessage`/`SelectionNotify`,
  serving `XdndSelection` as `text/uri-list`, and calling Smithay's
  `start_dnd` on the X→Wayland drop.
- T-14.6 (strange-app zoo) and T-17 must treat cross-boundary file DnD as a
  known, waived gap, not a regression. The zoo can still drag files *within*
  one side.
- The compositor now has a library target; later pure policy modules may move
  there for the same testability reason.