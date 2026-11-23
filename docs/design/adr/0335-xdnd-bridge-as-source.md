# 0335 — XDnD bridge-as-source: the same connection drives Wayland drags to X11

## Status

accepted — implements the Wayland→X11 half of the XDnD gap (G-07) for T328b.
Completes [ADR 0334](0334-xdnd-bridge-as-target.md); the codec and the target
role are unchanged. Partially supersedes
[ADR 0099](0099-xdnd-bridge-model-and-documented-gap.md), whose codec and
state machines remain the source of truth.

## Context

T328a landed the X11→Wayland half of the bridge on a dedicated X11 connection,
a 17-atom table, and an override-redirect InputOnly bridge window published as
Xwayland's root `XdndProxy`. The reverse direction — a file dragged from a
Wayland application onto an X11 application — still had no runtime. Smithay
0.7's XWM implements no XDnD, so the bridge must speak XDnD itself.

## Decision

- **Reuse the T328a connection, atom table, and bridge window.** The bridge
  window becomes the source window and owns `XdndSelection`; no second X11
  connection or window is created. The bridge interns `TARGETS` alongside the
  existing atoms.
- **Drive the full source exchange.** `begin_source` takes `XdndSelection`
  ownership; `source_enter` sends `XdndEnter` followed by the opening
  `XdndPosition`; motion sends further positions; `source_drop` sends
  `XdndDrop` **only after an accepting `XdndStatus`**; `source_leave` sends
  `XdndLeave` when the pointer leaves an X11 window. `XdndStatus` and
  `XdndFinished` are decoded on the bridge connection. A target that never
  finishes is bounded by the same 2 s transfer deadline.
- **`text/uri-list` only, five actions, Copy default.** The bridge proposes
  Copy unless the Wayland source offers only Move or Ask; the target may
  answer with any of the five concrete actions (`Copy`, `Move`, `Link`,
  `Ask`, `Private`) and the bridge records it. `None` resolves to Copy. Other
  flavours are refused and deferred to T346.
- **The Wayland source owns the payload; the bridge translates lazily.** When
  the X11 target sends `SelectionRequest`, the bridge parks it and reports
  `SourceSelectionNeeded`. The compositor asks the `WlDataSource` to write
  `text/uri-list` into a pipe and reads it from a calloop readiness source, so
  a stalled client never blocks the event loop. The cached payload answers
  the parked request and any later one; `TARGETS` advertises the one flavour.
- **The target is the topmost X11 window under the pointer.** The compositor
  reuses its `surface_under` hit test, so shell chrome and titlebars never
  become drag targets. Root coordinates are the logical point scaled by the
  session's integer Xwayland scale.
- **Release on every exit.** `XdndFinished`, cancellation, timeout, and bridge
  teardown all send `XdndLeave` (when a target is live) and release
  `XdndSelection`.

## Consequences

- A file dragged from a Wayland application to an X11 application now crosses
  the boundary in both directions; X11→X11 drags are still untouched.
- Xwayland's root is the primary output's space, so a Wayland drag over a
  secondary output uses scaled primary-space coordinates. Multi-monitor
  correctness is a recorded follow-up, not a silent claim.
- The Wayland drag source is asked for the payload only when an X11 target
  requests it. A drag that stays on Wayland never starts a transfer.
- Non-file flavours (images, text) remain out of scope and are recorded in
  T346.
- The conformance suite drives the source role against a fake X11 target:
  enter/position, status-before-drop, lazy selection serving, finished and
  release, and the leave path. The live Wayland→X11 nested check is a manual
  step recorded in the task hand-off.
