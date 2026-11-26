# 0367 — Compositor-owned popup group motion (Dock chooser pilot)

Status: accepted (2026-11-25)

## Context

[T354](0365-popup-visual-transaction.md) made the accepted popup content a
buffer-bound transaction with a retained visual lifetime. The Dock chooser
still faded and scaled itself in QML while the compositor drew its sampled
material at full strength. That left two owners of one visual: the shell was on
the animation's critical path, and the material could outlive the shrinking
card. Independently multiplying the QML alpha and the material alpha does not
produce group opacity: the two layers' relative contribution changes with the
alpha, and at near-zero alpha the underlay is still darkened by the material.

[ADR 0003](0003-shared-animation-clock.md) already owns the one animation
clock, [ADR 0014](0014-reusable-scene-transform-pass.md) the reusable
transform, and [ADR 0286](0286-content-sized-popup-render-targets.md) the warm
content-sized target.

## Decision

- **One opt-in transaction edge.** `df_layer_surface.set_popup_motion(enabled)`
  (since v6) is commit-latched with the next popup content commit. The shell
  sends it only for the Dock chooser. A client that never sends it keeps the
  QML opacity/scale path, and a pre-v6 compositor is detected by the shell.
- **One transition owner.** `PopupMotion` lives on the compositor's
  `PopupTransaction`. It reuses the design-system `popupOpen`/`popupClose`
  tokens, collapses to one step under reduced motion, and retargets a reopen
  from the current alpha/scale instead of snapping. A close during an open (or
  the reverse) is the same retarget.
- **One transform.** The accepted paint extent (card, arrow, mask, rim and QML
  shadow) is scaled about its bottom-centre, matching the chooser's previous
  `transformOrigin`. The material panel's visible rect and rim radii use the
  same scale and the same group alpha. The source rect stays at the committed
  card so the filtered texture survives the transition.
- **One clock sample.** `begin_render_frame` pins `frame_clock_ms`; chrome
  content and material panels both resolve their motion frame against it.
- **Damage.** The motion frame carries the union of the previous and current
  visual extents. The transformed elements and the material alpha also flow
  through the renderer's normal element geometry/alpha damage, so the old and
  new footprints are both invalidated.
- **Autonomous finish.** `animations_active` and `animation_output_names`
  include a live popup motion on its owning output (or the chrome-focused
  output for an all-output overlay). `step_popup_motions` advances it on the
  shared clock. The shell keeps the accepted surface mapped for one closing
  commit and then stops submitting popup frames; the compositor's clock
  finishes the exit without a client frame. The shell unmaps after the token
  (or immediately at zero duration).
- **Scope.** The persistent Dock plate is a separate surface and never joins
  the motion. The attached menu chain is T356; the remaining popup families are
  T361. `DRAGONFRUIT_POPUP_MOTION=qml` selects the old QML path as a local A/B
  diagnostic.

## Group opacity reference

`popup_motion::group_over_underlay(content, material, underlay, alpha)`
composites the group internally first (`content over material`) and fades that
one result over the original lower scene. At alpha 0 it returns the underlay
exactly. `independent_over_underlay` is the wrong composition (each layer
multiplied by the same alpha) and is retained only so the deterministic pixel
fixtures can show where the relative contribution changes. The pilot applies
one transform and one alpha to the accepted content and its material panels;
folding the sampled backdrop and the content into one offscreen contribution is
the remaining native rollout item (T361/T363) and is recorded in the task
progress note.

## Consequences

- The Dock chooser no longer depends on QML frames for its outer transition.
  A late shell producer or a menu provider cannot stall the exit.
- A close or reopen is one generation-safe visual transaction: the accepted
  texture, material, damage and input policy move together.
- The software/headless path has no sampled backdrop; its accepted QML texture
  is already one contribution, so it is exact there. The nested GPU pilot uses
  the same frame for content and material.
- Other Dock popovers and the menu-bar popup keep their QML/legacy path until
  T356/T361 opt in.
- Native presentation and the exact offscreen group composite remain evidence
  rows for the later rollout; the deterministic fixtures and the headless
  protocol test are this task's evidence class.

## Verification

- `shell::popup_motion` pixel fixtures: settled, near-start, midpoint and end
  frames, full arrow/shadow reach, zero-duration/reduced motion, quick reopen,
  premultiplied group opacity at zero and near-zero alpha, and the independent
  composition's difference.
- `shell::popup` and `shell_protocol_conformance` cover the transaction motion
  lifecycle and autonomous completion under both transport declarations.
- `tst_chromestability` covers the settled closing texture, input release,
  reduced motion, quick reopen and the QML diagnostic path.
- `make verify-chrome-stability` is the gate.

[Protocol](../../../protocols/dragonfruit-shell.xml) ·
[Compositor motion](../../../compositor/src/shell/popup_motion.rs) ·
[Shell chooser](../../../shell/dock/DockWindowChooser.qml)
