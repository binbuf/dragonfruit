# 0294 — Shell popup frame lifecycle

Status: accepted (2026-11-18)

## Context

The Dock and Menu Bar use separate overlay surfaces for transient menus.
The GPU producer can complete a frame after the popup has closed or
retargeted. On the shared-memory path, identical pixels can match the
previous image even after the surface has detached its buffer. A popup can
also remain visible during its exit after its owner has released input.

## Decision

- Stage the popup's placement, material regions, input region, and pointer
  origin until the content is ready to commit. Send the layer placement
  requests immediately before the matching surface commit. Layer placement
  requests keep their existing dispatch-time semantics.
- Closing a popup commits an empty input region immediately. The existing
  image and material can continue through the visual exit. Unmapping clears
  the damage comparison baseline so even identical content attaches a new
  buffer on reopen.
- Discard a completed popup target when its presentation generation has
  been superseded. A shared frame can still update the persistent strip.
- Give the Menu Bar one active popup owner across title menus, status
  popovers, tray menus, and overflow. A reopen cancels the pending unmap.
  Within a menu, a live row update preserves valid selection and retires
  invalid submenu branches.

## Consequences

Popup lifecycle and render-target ownership are local to the shell.
The compositor remains responsible for focus, output placement and session
state. The shell's process and GUI thread remain shared by both strips.
Refresh and action delivery to external menu services are asynchronous.

Headless and nested checks cover the shared-memory path and both DMA-BUF
target modes. Native presentation latency and mixed-output acceptance use
the [chrome performance gate](../evidence-and-acceptance.md#54-chrome-performance).

[Shell transport](../../../shell/src/shellprotocol.cpp) ·
[Popup controller](../../../shell/src/shellcontroller.cpp) ·
[Menu state](../../../design-system/components/MenuLevels.qml)
