# 0028 — The audio adapter talks to WirePlumber through its control CLI

## Status

accepted

## Context

T-07.3 adds the audio adapter behind the dependency-free contract
([0024](0024-system-adapter-contract.md)) in its own crate
([0026](0026-concrete-adapters-in-their-own-crates.md)), giving the menu bar
the default sink's volume/mute and the per-sink list. Unlike NetworkManager,
PipeWire/WirePlumber expose **no stable D-Bus API** for sink volume, and the
pinned toolchain has no `libpipewire-0.3` headers or pkg-config file (only the
runtime `.so`), so a native client cannot be built in CI. The native PipeWire
control protocol is also large; re-implementing it here is out of scope. The
track's risk note is explicit: the WirePlumber surface moves fast, so pin it and
isolate the churn ([07-menu-bar-live-status.md]).

## Decision

- **The live source is the WirePlumber control CLI, behind the `AudioSource`
  seam.** `CommandAudio` reads the graph with `pw-dump` (JSON) and writes the
  default sink with `wpctl set-volume` / `wpctl set-mute`. The adapter, the
  model, and the shell never see a CLI, a JSON blob, or a PipeWire type.
- **The PipeWire JSON schema is pinned in one place.** `AudioData::from_pw_dump`
  is the only code that knows the `pw-dump` shape; it is tested against a
  captured fixture (`services/audio/tests/fixtures/pw-dump-office.json`).
  WirePlumber stores channel volume on a cubic curve, so the parser un-cubes it
  to the linear 0..=1 value the menu shows.
- **Absence is a normal state.** A tool that cannot run, or that exits non-zero
  because no PipeWire core is reachable, maps to `Unavailable` (hidden); output
  that parses but is malformed maps to `Error` (visible, inert). Nothing blocks
  session startup.
- **A later native client drops in behind the same trait.** If T-15 adds
  `libpipewire` to the toolchain, a `PipewireAudio` source replaces
  `CommandAudio` without touching the adapter or the shell.

Rejected: adding `libpipewire`/`libpulse` bindings (no dev headers in the
pinned toolchain, and a native build dependency in CI for a menu-bar read),
the PulseAudio protocol over `pipewire-pulse` (large protocol to implement,
and `pactl` is the same CLI class), and a D-Bus client (there is no volume
interface to call).

## Consequences

- The adapter's tests are fixture/mock driven and run with no PipeWire; the
  live CLI path is smoke-tested where a session is present ("one real daemon
  where available").
- `make e2e` runs `cargo test -p dragonfruit-audio`.
- T-07.5 owns the event bridge: it must call `AudioAdapter::refresh()` on a
  WirePlumber change (`pw-mon` / `pactl subscribe`) rather than poll.
- Routing and device switching stay deferred to T-15, which may revisit the
  native-client option behind `AudioSource`.

[07-menu-bar-live-status.md]: ../tracks/07-menu-bar-live-status.md