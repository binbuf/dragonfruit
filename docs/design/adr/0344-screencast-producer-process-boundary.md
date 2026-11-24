# 0344 — The ScreenCast producer is a host tool fed by the capture seam

- Status: Accepted
- Date: 2026-11-24
- Task: T335a
- Supersedes: decision 5 of ADR
  [0081](0081-screencast-stream-negotiation-and-stills-fallback.md) (the live
  producer is not built); amends decisions 1, 3, and 4 there.

## Context

ADR 0081 named the stills fallback and left the live PipeWire producer unbuilt.
T335a must turn a chosen source into a live `Video/Source` node a portal client
links to, fed from the compositor's existing capture pass, without adding a
second capture path or a per-frame screencopy.

Two boundaries were open: which process owns the node, and how the frames
travel. `libpipewire` is not in the pinned toolchain (the portal and the
compositor do not link it, the same discipline as the audio adapter over
WirePlumber's CLI, ADR 0028). The compositor's capture is already an offscreen
render of the element list it is about to composite
(`backend::nested::capture_frame`), shared with the screenshot path.

## Decision

1. **A disposable host tool owns the node.** `tools/screencast-producer`
   (`dragonfruit-screencast-producer`) registers the node with `pw-cli` and
   pumps frames into it. The compositor spawns one per stream. A compositor
   that hung in a frame path hangs the session (ADR 0158), and a node it
   registered is a node it can never outlive; the narrow, failure-prone half
   (node registration, the frame pump) therefore lives in a process whose
   death costs a stream, not the desktop.

2. **Frames travel as one packet per frame on a Unix socket.** The compositor
   binds the socket and the tool connects, bounded by
   `HOST_CONNECT_TIMEOUT`. A packet is `u32 width, u32 height, u32 stride,
   u32 length` then RGBA8888, so a frame is never split or interleaved. The
   tool tolerates a `SOCK_STREAM` peer as well as the shipped
   `SOCK_SEQPACKET` one (the shell bridge cannot open a seqpacket socket),
   enforcing the same header-then-body framing itself.

3. **The producer reuses the capture seam and never renders twice.** Each
   backend hands the whole-output readback of `capture_frame` to
   `ScreencastProducer::offer_image_to_output` after the live frame's element
   list is built. There is no second render, no screencopy, and the trusted
   shell's chrome surfaces are already excluded by the capture pass.

4. **Geometry and rate are fixed at creation and enforced by the producer.**
   A stream is sized when its node is created; the producer crops the output
   readback to that size. A frame is offered at most once per `1 / fps`, and an
   unchanged picture is not re-sent (an idle desktop is one frame, not thirty a
   second). A frame of the wrong size is refused and named, never padded.

5. **Failure is the named fallback.** No PipeWire host, a missing tool, a tool
   that never connects, or a graph without the node all produce
   `pipewire-producer-unavailable`; a stream is released rather than left half
   live. The portal's `PipeWireTransport` reports the mode as `pipewire` and
   turns every create failure into that fallback, so a client is never handed
   a node id that does not exist.

6. **The node id is resolved through `pw-cli`, not guessed.** The tool names
   the node deterministically; the compositor resolves its id from
   `pw-cli list-objects Node` with one pinned parser. T335b returns that id in
   `Start`.

## Consequences

- A live stream needs `pipewire` and `pw-cli` on the host. Without them the
  stills fallback is selected and the picker keeps saying so; nothing else in
  the session changes.
- The composer still pays one offscreen readback per stream-bearing output per
  rendered frame (the GPU readback the screenshot path already uses). A GPU
  buffer handoff is a future optimization, not a correctness gap.
- The cross-process frame path is proven on a host with a PipeWire daemon by
  `make t335a-screencast` (node registered, one 1920x1200 frame pumped whole,
  node removed on teardown).
- T335b owns the private-protocol command the trusted shell uses to reach the
  producer, and the shell bridge that answers the portal's `CompositorWatch`.
