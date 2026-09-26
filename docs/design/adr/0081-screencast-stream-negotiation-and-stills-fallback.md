# 0081 — ScreenCast stream negotiation and the stills fallback

## Status

Accepted (T-13.4b).

## Context

T-13.4a ended the ScreenCast portal at the chosen source handle: `Start`
returned streams with `node_id = 0` and no explanation, and the picker closed on
selection. T-13.4b must make the chosen source visible in the client, or, when
that cannot be done in one session, name a stills-only fallback rather than let
the placeholder look like success.

A real stream is a PipeWire video node produced on the compositor side. This
build has no in-process PipeWire producer, and the portal crate does not link
`libpipewire` (the same "host stack, our process boundary" discipline as the
audio adapter over WirePlumber's CLI, [ADR 0028](0028-audio-adapter-over-wireplumber-cli.md)).
The compositor's capture path is the private single-frame request from
[ADR 0079](0079-screenshot-capture-delivery-and-save-copy.md). The streaming
gap is therefore timeboxed and documented, not hidden.

## Decision

1. **`portal::stream` owns the transport seam.** `StreamTransport` is the one
   thing that creates a live node; `StillsTransport` is the shipped
   implementation and always names [`FallbackReason::ProducerUnavailable`].
   `StreamNegotiator` turns a `StreamSource` into a `NegotiatedStream`
   (`node_id`, `mode`, `fallback`), so a later PipeWire producer implements the
   trait and the session, picker, and diagnostics do not change.

2. **A stream's mode is explicit on the wire.** Each stream carries the
   Dragonfruit-extension properties `df_stream_mode` (`pipewire` / `stills`) and,
   in fallback, `df_fallback` (the reason). `from_wire` defaults a missing mode
   to `stills`, so no peer can mistake an old result for a live stream. The
   standard frontend and clients ignore unknown properties.

3. **`Start` negotiates and the registry owns the node lifecycle.** Live nodes
   are remembered per session and released on `Close`, so a transport is not
   leaked when the frontend tears the session down.

4. **The fallback is named before the user chooses.** The diagnostic
   `ScreenCastStreamMode` method reports the mode, and the shell's picker shows
   the bridge's `streamNote` ("Live streaming is not available in this build —
   the app will receive still images.") when the mode is `stills`. A missing
   producer is a normal, spoken state.

5. **The live producer is not built in this slice.** No PipeWire node is
   created; the stills mode is the shipped answer and the gap is recorded. The
   fallback stops being selected the day a `StreamTransport` can create a node.

## Consequences

- A standard client cannot yet receive live frames; the T-13.4b acceptance is
  met by the named fallback. The gap is explicit in the stream properties, the
  diagnostic surface, the picker note, and
  [13-portals-capture-clipboard.md](../tracks/13-portals-capture-clipboard.md).
- The stream properties are Dragonfruit extensions, not part of the standard
  result contract; a real producer may additionally fill geometry/cursor
  metadata without a version bump of the backend.
- The picker note is a stopgap for the stills build; a producer with live
  streaming makes it empty without a QML change.