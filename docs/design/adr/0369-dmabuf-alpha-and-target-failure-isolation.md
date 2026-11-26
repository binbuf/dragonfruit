# 0369 — DMA-BUF chrome requires alpha and isolates target failures

Status: accepted (T357, 2026-11-25)

## Context

The chrome producer renders every scene into an `RGBA8` framebuffer and clears
it to transparent. The compositor's `zwp_linux_dmabuf_v1` feedback table may
still advertise the matching opaque `XB24`/`XR24` pair for the same allocation.
An export that selects the opaque pair is importable and looks valid to a
format/modifier check, but it has no alpha channel: every cleared corner,
rounded edge, shadow feather and glass pixel becomes opaque black. The old
validator only checked the `(format, modifier)` pair, so this export could be
attached to a translucent chrome surface.

Failure handling was also scene-wide. A target whose import, format or
viewporter rejected a commit was added to a disabled set, and any disabled
label made the whole shared scene take the shared-memory path. A failed popup
therefore stopped its healthy strip from committing. At the other end, only
`no-dmabuf` disabled the rail for the session, so a lost exporter, context or
device could leave every surface retrying a dead transport. There was no
recovery point: once a label or the rail was disabled it stayed disabled until
the shell restarted.

## Decision

- **The exported format must carry alpha.** `ChromeDmabuf::validate()` accepts
  only `AB24`/`AR24`, the two supported premultiplied-alpha formats, even when
  an opaque pair appears verbatim in the feedback table. An opaque export fails
  with `opaque-format` before a `wl_buffer` exists and the caller takes the
  existing shm fallback. The producer and the protocol commit path both call
  the same validator.
- **Failures are classified by where they occur.** A pure
  `ChromeDmabuf::classifyFailure()` separates transient saturation/timeouts
  (`sync-timeout`, `ring-exhausted`, `slot-busy`, `import-pending`), a
  target-local import/format/geometry failure (`import-failed`,
  `no-viewporter`, `opaque-format`), a rail-wide transport loss
  (`no-dmabuf`, `no-egl-export`, `no-current-context`, `device-lost`), and a
  stale surface that closed while its frame was completing.
- **A target failure stays local.** The controller builds the DMA-BUF target
  list from the enabled labels and commits the same export to every eligible
  target. A failed popup target is skipped while its strip/companion still
  commits. Only when no target is eligible does the scene read the completed
  frame back once and commit the whole scene through shm. The explicit
  `DRAGONFRUIT_CHROME_GPU_SURFACES` capture selection keeps its all-or-nothing
  scene path; this decision is about failures, not capture isolation.
- **Rail-wide failures disable the rail.** A producer export/submit failure or
  a lost connection/device/context/exporter disables DMA-BUF for the session
  and records the reason, exactly like the previous hard-failure path. A stale
  surface never does.
- **Recovery is bounded and local.** A capability or output change
  (`outputScaleChanged`, `outputRemoved`, a refreshed feedback table) may clear
  the failed-target set and retry a disabled rail. `ChromeDmabufRecovery`
  allows three consecutive attempts; a successful commit restores the budget.
  Recovery clears the protocol's per-target import failures, invalidates the
  Dock baseline, and retires the producer ring generation, so the first
  recovered frame is a full redraw. Serial-based release and the full-redraw
  invariant on slot reuse are unchanged.
- **The persistent Dock plate is not touched.** The change is in the transport
  decision and the alpha gate; the plate geometry, material and motion are
  unchanged.

## Alternatives

- **Trust the advertised format order.** The compositor advertises opaque
  formats for compositing ordinary windows; selecting one for a translucent
  chrome scene is a client-side mistake, not a compositor capability.
- **Reject only when the scene has transparent pixels.** The producer cannot
  prove a frame has no transparent pixels without the readback the GPU rail
  exists to avoid; every chrome scene clears to transparent.
- **Keep the scene all-or-nothing.** Safe but wrong: one bad popup freezes the
  whole strip and turns a local failure into a visible regression.
- **Recover on every frame.** A persistent import/format failure would spin the
  render loop. Capability/output changes are rare and bounded.

## Consequences

- An opaque export is a recorded `opaque-format` fallback instead of black
  chrome. A supported alpha export still validates against the exact feedback
  pair and keeps its premultiplied pixels.
- A failed popup leaves its last committed pixels on screen while the strip
  keeps updating. It is retried on the next capability/output change; the
  retry is bounded so a dead transport cannot loop.
- `tst_dmabuffailure` covers the opaque pair, the supported alpha pixel
  fixture, target-local and rail-wide classification, the shared and
  independent target plans, the completed-frame fallback, bounded recovery and
  the full-redraw slot invariant. The nested ring smoke also asserts the
  exported format carries alpha alongside its transparent-pixel sample.

## Verification

- `make verify-chrome-stability` includes `tst_dmabuffailure` and the existing
  dmabuf/producer/capability suites.
- `bash scripts/t349-gpu-smoke.sh` reports `alpha_format_ok=1` and
  `premultiplied_ok=1` at 1x/1.5x/2x on a nested GPU host.
