# 0346 — The screen-share bridge is its own trusted portal client

- Status: Accepted
- Date: 2026-11-24
- Task: T335b (continuation)
- Supersedes: decision 3 and the "live client half is open" outcome of
  [ADR 0345](0345-screencast-bridge-and-portal-ownership.md); its decisions 1,
  2, and 4 stand unchanged

## Context

ADR 0345 landed the portal's stream ownership and the compositor's v17
`create_screencast`, but recorded the bridge that answers the portal's
`CompositorWatch` as an open gap: the shell's one private-protocol connection
binds `df_toplevel_manager` at v16 (the shell never shares screens), and
inventing a second runtime protocol for one message pair was refused.

That left a standard client unable to receive frames. Re-examining the gap:

- Trust is **per process**, not per connection. A launch token grants a role to
  the client that redeems it, and the compositor already mints one token per
  role (`shell`, `desktop`). The portal is a session service like the Files
  desktop: it can hold *its own* token and *its own* connection.
- The compositor already releases a client's streams when that client's
  `df_core` connection goes away (`DfState::release_client_streams`). A
  separate portal connection therefore gives the portal bridge the lifetime
  rule it needs for free.

## Decision

1. **The portal is a third trusted role.** `TrustedRole::Portal` is minted with
   every other token, written to `$XDG_RUNTIME_DIR/<socket>.portal-launch-token`,
   handed to the backend as `DRAGONFRUIT_SCREENCAST_BRIDGE_TOKEN` by the session
   plan (and `PassEnvironment` in the units), and re-issued when the portal's
   connection drops. The `portal:` prefix in `DRAGONFRUIT_LAUNCH_TOKENS` mints
   it (the conformance suite uses that). It may create layers on no band: it
   renders nothing.

2. **A presented token is bound, never adopted.** A single-token environment
   value must match a token the compositor already minted for that role;
   otherwise it is ignored and a fresh one is minted. (Before this, any readable
   value in `DRAGONFRUIT_LAUNCH_TOKEN` became a shell token.) A mint can never
   re-open a consumed token either: a spent one-time value is not addressable.

3. **The bridge is a Wayland client inside the portal process.**
   `portal/src/screencast_bridge.rs` connects to the compositor socket, presents
   the portal token, binds `df_toplevel_manager` at v17, and issues
   `create_screencast`/`destroy_screencast`. No new protocol, socket, or framing.

4. **One thread owns the connection.** Wayland event queues are per thread and
   zbus dispatches a `Start` on whichever executor thread is free, so
   `LiveTransport` starts one dedicated thread with the first call and every
   create, release, reopen, and health check runs there in order. A reopen is
   *verified* (handshake plus one flushed request) before the picker is told
   `pipewire`, because a killed compositor can leave its socket path answering
   for a moment.

5. **Fallback stays named and first-class.** No provisioning, a refused
   handshake, a dead connection, or a compositor with no PipeWire host is the
   per-stream `stills` fallback with `df_fallback =
   pipewire-producer-unavailable`; the transport reports `stills` so the picker
   says the app will receive still images. Live is never promised by a stream
   that cannot deliver frames.

## Consequences

- A standard client's `Start` now mints a real `Video/Source` node through an
  unsandboxed, portal-role-authenticated connection, and the picker reports
  `pipewire` only while that connection is verified.
- A portal restart cannot disturb the shell's manager object, and the compositor
  releases every portal stream when the bridge's connection ends.
- The captured pixels still come from the nested/DRM capture pass (T335a), so a
  client's frames need a session that renders; the headless rail proves the
  transport and the graph node. `make t335b-screencast-client` records that
  split, and the nested rerun is the frame proof.
- The one-time token a restarted session mints is handed to the backend by the
  session manager; a long-lived backend reads it on its next reopen (presenter
  loss calls `reconnect_transport`).
