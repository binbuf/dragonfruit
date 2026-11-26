# 0365 — Popup visual transaction and lifetime owner

Status: accepted (2026-11-25)

## Context

Popup state used to be a set of independent layer requests: placement,
material regions and input were staged separately, and the accepted pixels were
identified only by the surface commit. A GPU frame could complete after the
popup had retargeted, closed, or fallen back from dmabuf to shm, so the older
frame could overwrite newer content. A logical close released input
immediately but had no owner for the visual tail, and a shell exit, output
unplug or dropped buffer had no bounded release for the retained content.

[ADR 0294](0294-shell-popup-frame-lifecycle.md) established the staged commit,
the empty-input close, and superseded-frame discard.
[ADR 0286](0286-content-sized-popup-render-targets.md) gave the popup an
independent content-sized target, and
[ADR 0364](0364-chrome-presentation-trace.md) joined an accepted surface
generation to presentation. None of those named one owner for the popup's
visual lifetime, and the shell-side generation was not carried to the
compositor.

## Decision

- **One transaction per popup surface.** The additive v5
  `df_layer_surface.set_popup_transaction(generation, content_serial, phase,
  transport, input_active, retain_ms)` is commit-latched with the pixels it
  describes. It joins the accepted content with the generation, the owning
  output (from the layer surface), anchor/crop/paint extents (the committed
  layer geometry), material coverage (the committed panel regions) and the
  input policy.
- **Generation fences the lifecycle; content serial fences frames.** `open`,
  `retarget`, `closing`, `complete` and `cancel` are monotonic generation
  edges. Within a generation, `content_serial` must advance; a lower serial or
  generation is rejected as stale, a retarget after completion/cancellation or
  a geometry retarget during the closing tail is superseded. A declared
  transport that differs from the committed buffer is accepted only as the
  documented dmabuf→shm fallback and is still fenced by the serial.
- **Input is independent of the visual.** `closing` always sets
  `input_active = 0`; the compositor's hit test skips a transaction whose input
  policy is inactive even while the surface still paints. The retained
  snapshot is frozen: a late retarget cannot move or resize it.
- **Bounded retained lifetime.** `retain_ms` (clamped to 5 s) starts on the
  first closing edge and is not extended by later closing frames. `complete`
  keeps the snapshot until that deadline; `cancel`, output removal, surface
  destruction and client disconnect release it immediately. A legacy client
  that never sends the request keeps the pre-T354 behavior: whole-surface input
  policy, no retained snapshot.
- **Ownership.** The shell owns the logical transaction and the shm snapshot
  (an implicitly shared `QImage`, one frame). The compositor owns the imported
  GPU texture or shm buffer named by the accepted generation and releases it
  with the same lifetime. Compositor animation of the retained snapshot is
  T355; the attached menu chain is T356; the remaining popup families are T361.

## Consequences

- The menu-bar and Dock popups send one transaction per committed frame. A
  frame that completes after a retarget or transport switch is dropped instead
  of overwriting the accepted content.
- Closing releases input on the first edge; the visual stays valid for a
  documented 750 ms window and is released even if the shell stops submitting
  frames (the next commit or maintenance pass expires it).
- Older shells bind v4 and never send the request. Older compositors are
  detected by the layer version and the shell falls back to the existing
  geometry/material/input requests.
- The compositor now owns a small retained-state record per popup surface.
  T355 consumes it for the compositor animation; until then the record is the
  contract and the shell keeps the closing surface mapped for its QML tail.

## Verification

- `tst_popupvisualtransaction` covers open/retarget, stale generation and
  serial, closing input release, the frozen closing snapshot, completion and
  expiry, cancellation, output removal, superseded reopen, both transports and
  the retain clamp.
- `shell::popup` unit tests pin the compositor state machine.
- `shell_protocol_conformance` covers stale/superseded frames, closing
  snapshots, input release, content changes, output unplug, client disconnect,
  both transport declarations, the fallback switch, and a legacy client with
  no transaction.
- `make verify-chrome-stability` is the gate.

[Protocol](../../../protocols/dragonfruit-shell.xml) ·
[Shell owner](../../../shell/src/popupvisualtransaction.h) ·
[Compositor owner](../../../compositor/src/shell/popup.rs)
