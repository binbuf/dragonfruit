# T-07 — Private Shell Protocols, Versioning, and Launch Tokens

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `protocols/` + `compositor/` + shell bindings |
| **Depends on** | [T-02](02-compositor-core.md) · [T-04](04-window-model.md) · [T-05](05-spaces-model.md) |
| **Blocks** | [T-09](09-menu-bar.md) · [T-10](10-dock.md) · [T-11](11-mission-control-workspace-ux.md) · [T-12](12-app-switcher.md) · [T-22](22-global-menu-broker.md) · [T-28](28-screenshot-recording-ui.md) · [T-19](19-desktop-icons.md) |
| **Estimate** | L |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [01-architecture.md](../design/01-architecture.md) |

## Summary

The private, versioned Wayland protocols that carry compositor↔shell
communication: layer-shell-style chrome surfaces, window/workspace control
(à la `wlr-foreign-toplevel-management`), output management (à la
`wlr-output-management`), and the trust model (one-time launch tokens)
restricting access to a fixed set of session processes.

## Background

The shell consumes a private, versioned interface rather than scraping
public protocols ([01-architecture.md](../design/01-architecture.md)).
Chrome surfaces are a privilege of trusted processes, not a public
extension surface ([02-compositor.md](../design/02-compositor.md)).

## Scope

### In scope

1. **Chrome-surface protocol** (layer-shell-style):
   - Reserved zones (menu bar, Dock auto-hide anchoring), stacking layers,
     explicit keyboard-interaction modes.
   - Surfaces: menu bar, Dock, Control Center, notification banners, OSD,
     overview strip, screenshot UI.
2. **Window/workspace control protocol** (foreign-toplevel-style, extended):
   - Window enumeration, titles/app_ids, state (minimized/zoomed/
     fullscreen), workspace assignment, activate/minimize/zoom/fullscreen
     requests, "Move to Space", window menu emission.
   - **Workspace extensions**: workspace enumeration, create/remove/
     reorder, activation, and the workspace event stream
     (created/removed/reordered/activated/window-assigned) from T-05.
   - **Mission Control control**: enter/exit overview, overview selection
     events, workspace strip data (feeds T-11).
   - **App-switcher state**: recency-ordered app list, per-app window
     lists, selection requests (feeds T-12).
   - **Focus broadcasts** so shell, Dock, and menu-broker track the active
     application without polling.
3. **Output-management protocol** (wlr-output-management precedents,
   extended with our needs): mode, scale, rotation, VRR, night light,
   color controls, wallpaper per Space, output hotplug events (feeds
   Settings Displays pane, T-16).
4. **Trust model**:
   - Access restricted to a fixed set of trusted session processes, each
     provisioned with a **one-time launch token out-of-band at startup**
     (via session environment — see T-24): the shell, and — when desktop
     icons ship — the Files desktop surface (T-19).
   - `bind` attempts from any other client are **refused**.
5. **Versioning policy** ([01-architecture.md](../design/01-architecture.md)):
   - Private protocols are **versioned and additive-only** within a stable
     release: new events/requests may be added; existing ones never change
     meaning or disappear.
   - Compositor, shell, and protocol XMLs ship as a **lockstep set** per
     release; cross-version mixing is unsupported and **detected at
     handshake** (compositor refuses mismatched protocol versions).
6. **Client bindings**: generated Rust (services) and Qt/C++ (shell)
   bindings from the same XMLs; a compliance test client.

### Out of scope

- D-Bus service naming/versioning (that's
  `org.dragonfruit.*` with major-version suffix, T-15/T-24 — same additive
  rules, different channel).
- The shell's *rendering* of chrome (T-09/T-10).

## Requirements

- FR-1: Shell can create anchored, layered chrome surfaces with reserved
  zones on any edge; keyboard-interaction modes (none/on-demand/exclusive)
  work as specified.
- FR-2: Full window/workspace/output event streams arrive at a test client
  in scene-consistent order (no event describes a state the scene hasn't
  reached).
- FR-3: Requests (activate, move-to-space, create/remove/reorder workspace,
  set output mode, enter Mission Control) round-trip with acks.
- FR-4: A client without a valid launch token cannot bind any private
  interface — connection attempt is refused and logged.
- FR-5: Tokens are one-time and per-boot: replayed/captured tokens from a
  previous boot are invalid.
- FR-6: Handshake refuses version-mismatched lockstep sets (test old shell
  vs new compositor and vice versa).
- FR-7: Adding an event in a new version doesn't break an older client
  (additive-only proof test).

## Acceptance criteria

- [ ] Protocol XMLs reviewed against wlr precedents; deviations documented.
- [ ] Refusal matrix passes: untrusted client, stale token, wrong-version
      client.
- [ ] The compliance test client exercises every request/event pair.
- [ ] XMLs published under MIT license (with T-01 licensing).

## Test plan

- Protocol conformance suite (headless): every request, every event, plus
  fuzzed/malformed sequences (never crash —
  [14-risks.md](../design/14-risks.md) robustness rule).
- Integration: shell restart re-binds with a fresh token; chrome zones
  re-anchor.

## Risks / open questions

- Do not design a public extension surface — resist feature requests from
  third parties to bind chrome protocols; answer is no by design.
- Keep an eye on upstream `ext-` protocol efforts (e.g. ext-workspace);
  where a staging standard lands, prefer consuming it and shrink ours —
  but only within a major-version boundary.
