# 0121 — The audio adapter grows input devices and default-device routing

## Status

accepted

## Context

T-07.3 shipped the audio adapter (`dragonfruit-audio`) as the menu bar's
default-sink volume/mute plus the per-sink list, over WirePlumber's control CLI
([0028](0028-audio-adapter-over-wireplumber-cli.md)). That ADR deferred routing
and device switching to T-15. T-15.3a is that task; T-15.3b renders a Sound
pane whose Output/Input tabs select a device (`wpctl set-default`) and whose
slider/mute then act on the selected device. There is still no stable
`libpipewire` in the pinned toolchain, so a native client cannot replace the
CLI source in CI.

## Decision

- **Extend the existing crate; reuse WirePlumber, never reimplement it.** The
  adapter stays in `services/audio` behind `AudioSource`. One `pw-dump` read
  now decodes both device classes: `Audio/Sink` into `SinkData` and
  `Audio/Source` into `SourceData`, and both `default.audio.sink` and
  `default.audio.source` from WirePlumber's `default` metadata.
- **The model owns independent default resolution.** `AudioSnapshot` gains
  `sources` and `default_source`; each list marks its default and sorts it
  first, falling back to the first entry when WirePlumber named none (or named
  a device not in the list), exactly as the sink list already did.
- **Two explicit routing writes.** `AudioAdapter::set_default_sink(id)` and
  `set_default_source(id)` map to `wpctl set-default <node-id>` and return the
  shared `SetOutcome` (`Applied`/`Absent`/`Failed`). They do not invent a
  snapshot; the daemon pushes the new default and the host re-reads. Volume and
  mute keep targeting `@DEFAULT_AUDIO_SINK@`, so selecting a device then
  adjusts it.
- **Absence is unchanged.** A source that cannot run, or cannot reach a
  PipeWire core, is `Unavailable` (hidden); a parse failure is `Error`
  (visible, inert). Routing against an absent daemon reports `Absent`.
- **Per-application stream routing is out of scope.** No T-15.3b row maps to
  it; add it later behind the same seam if a consumer appears.

Rejected: a new `sound` crate (the T-07.3 adapter already owns the transport
and the contract), and per-device volume/mute addressing (selecting a device
makes it the default, which is the pane's interaction model).

## Consequences

- `SinkData`/`Sink` are unchanged; the public additions are `SourceData`,
  `Source`, `AudioData::{default_source,sources}`, `AudioSnapshot::{sources,
  default_source}`, and the two routing methods. Consumers that construct
  `AudioData` use `..Default::default()`.
- Tests: a new `services/audio/tests/routing_path.rs` drives the mock through
  output/input switching, the no-invent rule, a failed id, and absence; the
  `pw-dump-office.json` fixture gains one input node and both defaults.
- T-15.3b extends the status bridge with the input list and the routing writes;
  T-15.16's matrix can mask the bridge, WirePlumber, or a single device list.