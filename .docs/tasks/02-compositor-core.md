# T-02 — Compositor Core: Event Loop, Backends, Renderer, Effects

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `compositor/` |
| **Depends on** | [T-01](01-repo-scaffolding-ci-licensing.md) |
| **Blocks** | [T-03](03-input-keymaps-shortcuts.md) · [T-04](04-window-model.md) · [T-05](05-spaces-model.md) · [T-06](06-xwayland.md) · [T-07](07-private-shell-protocols.md) · [T-11](11-mission-control-workspace-ux.md) · [T-13](13-window-decorations-ssd.md) |
| **Estimate** | XL |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [01-architecture.md](../design/01-architecture.md) · [13-roadmap.md](../design/13-roadmap.md) |

## Summary

The compositor process: a Rust/Smithay application with one calloop event
loop, **three backends from day one** (DRM/KMS, Wayland nested, headless),
the GBM/EGL render stack with damage tracking and direct scanout, and the
full standard Wayland protocol surface. It stays conceptually small —
**display, input, surfaces, windows, workspaces, effects, security
boundaries, and shell protocols** — and excludes all product/hardware logic.

## Background

The compositor is the heart of the desktop and the only process that touches
the seat, DRM/KMS, or raw input ([01-architecture.md](../design/01-architecture.md)).
Everything visual that differentiates Dragonfruit (Mission Control, Spaces,
Dock coherence) works because the compositor owns the scene graph
([02-compositor.md](../design/02-compositor.md)).

## Scope

### In scope

1. **Event loop**: one calloop loop multiplexing Wayland clients, libinput,
   D-Bus, and timers; rendering driven **per output on vblank**. Scene
   updates and GPU work are single-threaded per output until profiling says
   otherwise — correctness beats parallelism in a display server.
2. **Backends** (all three from day one — never require every run to own the
   physical display):
   - **DRM/KMS** — native; owns the physical display; used for real sessions.
   - **Wayland (nested)** — runs as a window inside an existing session; the
     daily dev workflow.
   - **Headless** — CI and automated testing.
3. **Renderer and effects**:
   - Smithay GBM/EGL renderer stack on DRM.
   - Effects (blur, shadows, workspace scale/clip transforms) are
     compositor render passes over **live surface buffers** — never client
     re-renders, never screenshots, never third-party recompositing.
   - Fullscreen surfaces take the **direct-scanout** path where possible;
     tearing-control honored for fullscreen games.
   - **Damage tracking**: no damage → no render → no client wakeups;
     animations drive damage every frame.
   - **Hardware cursor planes** where offered; software cursor fallback;
     cursor motion never waits on effects or damage.
   - **VRR/adaptive-sync** per output where connector+mode support it
     (exposed as a Displays-pane toggle later, T-16).
   - **Night light** (per-output gamma ramps) — compositor-owned.
   - **Multi-GPU**: render on primary node, import GBM buffers across
     devices, automatic fallback when a GPU disappears — device *loss* is
     part of the hotplug story.
4. **Standard protocol surface** ([02-compositor.md](../design/02-compositor.md)):
   - `xdg-shell`, `xdg-output`, `presentation-time`, `linux-dmabuf`
   - `viewporter` + `fractional-scale` (fractional scaling)
   - `xdg-decoration` (SSD negotiation — feeds T-13)
   - Pointer constraints, relative pointer, `cursor-shape`, `idle-inhibit`
   - `wp-content-type-manager-v1` content-type hints (fullscreen video and
     conferencing take efficient/scanout paths)
   - `wlr-data-control` (shell clipboard manager — feeds T-29)
   - `security-context` (sandboxed/Flatpak clients identify themselves)
   - Wayland color-management (`xx-color-management-v1` while staging) —
     per-output color and HDR, basis of Displays-pane color controls
   - `xdg-activation` (launch feedback / Dock bounce)
   - `ext-session-lock-v1` (fail-secure locking — feeds T-26)
   - `ext-idle-notify` (idle)
   - `text-input` / input-method protocols (input methods from the start)
   - **Capture is portal-only**: screenshots and screen sharing exist
     **exclusively** through our portal backend as PipeWire streams with
     explicit user consent. We deliberately do **not** implement
     `wlr-screencopy`-style grabs: "if a capture is not a portal request,
     the answer is no."
5. **Output management**: enumeration, modes, scale, rotation, hotplug
   events (theDisplays backend of T-16; per-display Spaces interplay in
   T-05).

### Out of scope

- Window/workspace *policy* (T-04, T-05) — this ticket builds the machinery
  they run on.
- Private shell protocols (T-07).
- Xwayland (T-06).
- Wi-Fi/Bluetooth/audio/power logic, file-manager behavior, menu models —
  explicitly never in the compositor
  ([02-compositor.md](../design/02-compositor.md) "Out of scope").

## Requirements

- FR-1: `--nested`, `--drm` (logind seat), and `--headless` backends all run
  the same session code; backend selection is a flag, not a fork.
- FR-2: Idle desktop shows **zero damage events and zero client wakeups**
  attributable to us (measure: idle trace over 60 s).
- FR-3: Input-to-photon latency under one frame in both nested and DRM
  backends.
- FR-4: One frame of compositor work per frame of animation (no frame does
  2× work, no animation frame is skipped).
- FR-5: Direct scanout engages for unobstructed fullscreen surfaces
  (verified via render-path counters).
- FR-6: Removing a GPU (unplug / forced driver loss) degrades gracefully —
  outputs re-init on the remaining node or session exits cleanly; never a
  hang.
- FR-7: All listed standard protocols advertise and pass their upstream test
  suites where they exist (e.g. weston-surface-runner style smoke tests).
- FR-8: No `wlr-screencopy` and equivalent arbitrary-grab protocols are
  advertised, ever (grep-gate in CI).

## Acceptance criteria

- [ ] All three backends run a windowed client end-to-end.
- [ ] Nested and DRM sessions both run the vertical slice (Phase-1 exit).
- [ ] Protocol surface advertise list matches this ticket exactly — no
      missing, no extras (CI check).
- [ ] Performance budgets FR-2/3/4 measured and passing on baseline
      Intel/AMD hardware.

## Test plan

- Headless CI: protocol smoke tests, renderer unit tests, damage accounting.
- VM: hotplug (display + GPU), modeset changes, VT switch, teardown.
- Real hardware: latency and scanout measurements; VRR where supported.

## Risks / open questions

- The genuinely hard part is not drawing — it is "boringly reliable"
  GPU/hotplug/suspend behavior ([02-compositor.md](../design/02-compositor.md),
  [14-risks.md](../design/14-risks.md)). Soak tests belong to T-31; hooks
  land here.
- Smithay coverage for `xx-color-management-v1` staging may lag — track
  upstream, gate the Displays color UI accordingly.
