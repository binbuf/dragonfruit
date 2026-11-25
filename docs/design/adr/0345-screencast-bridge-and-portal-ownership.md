# 0345 — The portal owns live screen share; the shell bridge is a documented gap

- Status: Superseded in part by
  [0346](0346-screencast-bridge-trusted-portal-client.md) (decision 3 and the
  open live-client outcome); decisions 1, 2, and 4 remain in force
- Date: 2026-11-24
- Task: T335b
- Amends: decision 6 of ADR
  [0344](0344-screencast-producer-process-boundary.md) (the T335b half of the
  private-protocol wiring)

## Context

T335a landed the live producer, the portal's `PipeWireTransport`, and the
`CompositorWatch` seam, and parked the private-protocol command and the shell
bridge as T335b. T335b must wire the transport into the session lifecycle and
let a standard client receive frames.

While wiring it two facts decided the boundary:

1. **The portal backend is not a Wayland client.** Its `CompositorWatch` is
   answered by the trusted shell over the shell's one private-protocol
   connection. That connection binds `df_toplevel_manager` at the shell's cap
   (v16); the shell has no v17 scope and no route to the compositor's
   `ScreencastProducer` that is not a new cross-process protocol.
2. **A runtime endpoint protocol would be a second private protocol** with its
   own trust, framing, versioning, and teardown rules, for one message pair.
   The repository's private-protocol policy (one XML, append-only, versioned)
   does not have a precedent for a shell-owned socket.

The session-lifecycle half — who owns a live node and when it is released — is
independent of how the node is created, and was worth landing first: it is the
half that leaks if it is wrong.

## Decision

1. **The portal is the one owner of a live stream's lifetime.** `Start`
   creates the nodes; a session owns them. A second `Start` on the same session
   releases the previous set before creating new ones; `Session.Close` (a client
   disconnect, an app exit) releases the session's nodes; the presenter-loss
   watch releases every live stream when the trusted shell leaves. A registry
   records each node with its source handle so a release and the diagnostic
   surface name the same stream.

2. **The compositor owns per-client stream teardown.** v17
   `create_screencast`/`destroy_screencast` create and release a stream by
   token, answered by `screencast_node` (the node id, output, geometry) or
   `screencast_failed` (the portal's `pipewire-producer-unavailable` label,
   never a placeholder id). A stream records the client that created it; its
   manager object going away, the `df_core` disconnect, and compositor teardown
   all release that client's streams. The failure path releases the host tool
   before replying.

3. **The shell does not claim v17.** The shell binds the manager at 16 and does
   not send `create_screencast`. The bridge that would answer the portal's
   `CompositorWatch` needs a route from the shell to the compositor's producer;
   that route is not this task, so the shipped backend's `ShellBridgeWatch`
   has no bridge configured and every `Start` is the named
   `pipewire-producer-unavailable` stills fallback. The picker names it.

4. **The fallback stays a first-class, spoken state.** The picker note says
   the app will receive still images; the stream properties keep
   `df_stream_mode = stills` and `df_fallback = pipewire-producer-unavailable`.
   A live session never silently degrades: a transport refusal is per-stream
   and named.

## Consequences

- The portal half is complete and tested end to end over a private session
  bus: two sessions hold independent live nodes, a close releases only its own,
  a repeated `Start` replaces, and the presenter-loss release drops every
  stream.
- The compositor half is tested on the conformance rail: v17 is advertised,
  a create is answered with the named fallback (the headless rail has no
  PipeWire host), a destroy is idempotent, and a disconnect leaves the session
  serving.
- **The live client half is open.** No standard client can receive frames until
  a shell bridge (or another trusted client) answers the portal's
  `CompositorWatch`. `make t335b-screencast-client` records that as `OPEN` with
  the rerun command.
- A backend that later runs inside the shell's process, or a host agent that is
  a trusted compositor client, can implement `CompositorWatch` without touching
  the registry, the session lifecycle, or the picker.
