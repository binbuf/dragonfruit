# 0029 — The system adapters reach the shell through a session-bus status host

## Status

accepted

## Context

T-07.2–T-07.4 landed the NetworkManager, audio, and power adapters as Rust
crates behind the dependency-free contract
([0024](0024-system-adapter-contract.md), [0026](0026-concrete-adapters-in-their-own-crates.md)).
The menu bar, however, is C++/QML, and no process hosted the adapters for it:
ADR 0026 and the T-07 follow-ups explicitly left the bridge ("D-Bus service
vs. embedded") to T-07.5. The shell must stay crashable/restartable
independently of the compositor (design/04-shell.md), the adapters must not
leak their daemon stack above the contract, and design/09-files.md names
cxx-qt as the project's only Rust-in-Qt seam — so embedding the adapters in
the shell is out.

## Decision

- **A dedicated session-bus host in its own crate.** `services/system-status`
  (`dragonfruit-system-status`) owns the concrete adapters and serves them on
  the user session bus at `org.dragonfruit.SystemStatus1`, object path
  `/org/dragonfruit/SystemStatus1`, with two interfaces: `…SystemStatus1.Wifi`
  and `…SystemStatus1.Audio`. The host depends only on the adapter crates and
  `zbus`; the shell never links an adapter.
- **JSON state, typed actions.** Each interface exposes `State()` and
  `Refresh()` returning the adapter's decoded view as a JSON string (the same
  view `dragonfruit_system_status::wifi_view`/`audio_view` build), and the one
  write the menu offers (`Join`, `SetVolume`, `SetMute`, `ToggleMute`). The
  adapter owns aggregation, defaults, and the three-state degradation; the
  shell side is a decoder, not a second model.
- **One shell-side model, one client seam.** `shell/src/systemstatusmodel.*`
  decodes the JSON into the maps the QML popovers draw and raises the request
  signals; `shell/src/systemstatusclient.*` is the host client, with a live
  `DbusSystemStatusClient` and a fixture `MockSystemStatusClient` for
  `--placeholders` (and headless use). The design-system `Popup` draws the
  Wi-Fi network list/join and the volume slider/mute.
- **No polling above the adapter.** The host refreshes on its own startup and
  on the shell's explicit `Refresh()` (menu open) and action replies;
  daemon-signal-driven refresh is the follow-up T-07.6 wires, not a poll loop.

Rejected: embedding the adapters in the compositor (couples system
integration into the compositor and its restart); a cxx-qt bridge in the shell
(design/09-files.md keeps that seam for Files alone); a CLI-per-action bridge
(the design's services convention is D-Bus, and JSON-per-call forks a process
per interaction).

## Consequences

- T-07.5b adds the read-only `Power`/battery interface to the same service and
  deletes `--placeholders`.
- T-07.6a exercises the absent-daemon matrix against the host; T-07.6b records
  the idle trace. The daemon-signal subscription (NetworkManager signals,
  `pw-mon`, UPower `PropertiesChanged`) lands with them; until then a view
  updates on open/action, not on every daemon push.
- T-11 (Control Center) and T-15 (remaining adapters) consume the same
  service rather than re-hosting adapters.
- `make e2e` runs `cargo test -p dragonfruit-system-status`, so the bridge
  core cannot regress silently; the D-Bus transport itself is compile-checked
  and exercised on a host with a session bus, like the adapter live paths.