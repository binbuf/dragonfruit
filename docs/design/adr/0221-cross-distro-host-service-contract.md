# 0221 — Cross-distro host-service contract

## Status

accepted — T221 lands the contract in
[ADR 0230](0230-distro-adapter-contract-and-stubs.md); T254–T259 implement the
concrete backends.

## Context

Dragonfruit targets Fedora, Ubuntu, Debian, and Arch. Those distributions share
systemd/logind, Wayland/DRM, libinput, NetworkManager, BlueZ,
PipeWire/WirePlumber, UPower, UDisks2, CUPS/SANE, accountsservice, polkit,
PackageKit, portals, and a display manager, but differ in package manager,
firewall, admin group, time-sync daemon, locale tooling, autologin file, and
optional services.

Embedding distro commands in QML or forking a host service would make the
desktop unmaintainable and would break the "Linux first" promise.

## Decision

- **Host services are dependencies, not forks.** Dragonfruit uses the host's
  NetworkManager, BlueZ, PipeWire/WirePlumber, UPower, UDisks2, CUPS, SANE,
  accountsservice, polkit, PackageKit, portals, and display manager.
- **Distro differences live behind adapters.** A Rust trait per capability
  (updates, firewall, users/groups, time, locale, autologin, default apps) with
  a tested backend per distro. QML talks to D-Bus/services only.
- **No shell commands in QML.** Package-manager, firewall, user, and time
  commands are adapter implementation details.
- **Required vs optional dependencies are explicit.** The core desktop works
  with the required set; every optional feature has an absence path.
- **Packaging is a first-class deliverable.** RPM, DEB, and Arch packages; CI
  build/install/smoke on Fedora, Ubuntu, Debian, and Arch.
- **Supported distro tiers** are defined in
  `docs/design/distro-compatibility.md`; a distro outside the tiers is
  best-effort, not a release blocker.
- **No live compositor handoff.** The session starts and ends as a unit; logout
  returns to the display manager.

## Consequences

- `docs/design/distro-compatibility.md` is the contract; `docs/tasks/0.2.0/`
  T254–T259 implement it.
- Settings panes depend on adapters, not distro detection in QML.
- The test matrix includes a clean VM per Tier 1 distro and a service-absence
  pass.
- A new distro backend is additive; it does not change the UI.
- The first release may hide unsupported capabilities (e.g. no firewall, no
  fingerprint reader) rather than ship a fake control.
