# 0230 — Cross-distro adapter contract and stub backends

## Status

accepted — T221 lands the contract; T254–T259 implement the concrete
backends behind it.

## Context

[ADR 0221](0221-cross-distro-host-service-contract.md) proposed one Rust trait
per distro-administered capability (updates, firewall, users/groups, time,
locale, autologin, default apps) with a tested backend per distribution. T221
has to turn that proposal into something a Settings pane can consume before
the Settings tasks (T222–T229) and the backend tasks (T254–T259) start.

The risk is a pane that shells out to `dnf`/`apt`/`pacman`, or an adapter that
assumes one distro's admin group, firewall, or autologin file. A second risk is
treating "the host tool is not installed" as an error dialog, which would make
a minimal VM look broken.

## Decision

- **One dependency-free contract crate.** `services/distro-adapters`
  (`dragonfruit-distro-adapters`) owns the traits, the status model, the
  command/API matrix, the dependency sets, the mock, and the stubs. It has no
  dependencies, exactly like `services/system-adapters`, so no package manager
  or D-Bus stack leaks into the contract.
- **One trait per capability**, all extending `CapabilityAdapter`:
  `UpdateProvider`, `FirewallProvider`, `UserGroupProvider`, `TimeProvider`,
  `LocaleProvider`, `AutologinProvider`, `DefaultAppsProvider`,
  `AppDiscoveryProvider`. A pane asks `available()` / `status()`; it never
  names a distribution. `CapabilityStatus::view()` is the flat wire projection.
- **Three states, one typed error.** `Available`, `Absent`, `Unsupported`.
  `Absent` is a normal feature-level state, not an error. `DistroError` has
  `NotSupported` (the T221 stub / unknown distro), `Absent`, and `Failed`; the
  `NotSupported` display starts with `not supported:`.
- **Detection from `/etc/os-release` + service probes**, never a hardcoded
  distro list. `ID` first, then `ID_LIKE`, Ubuntu before Debian; unknown is a
  first-class `Unknown` family. The tier and admin groups follow
  [distro-compatibility.md](../distro-compatibility.md) §2/§6.
- **Four stub backends.** Fedora, Ubuntu, Debian, and Arch each select their
  matrix row and return `NotSupported` from every method today. The stub
  carries the exact command/API the eventual backend will use, so tests and
  diagnostics are meaningful before T254–T259 land.
- **One matrix, not duplicated in the UI.** `MATRIX` holds the command, exact
  invocation/API, probe commands, packages, and note for every capability ×
  supported family. The design doc renders the same table for humans.
- **Dependency sets are explicit.** Required/recommended/optional with an
  `AbsenceBehavior` per dependency. `packaging/optional-services.json` stays
  canonical for package names/licenses; the Rust table references its ids.
- **Mock + contract tests in the gate.** `MockBackend` drives all three states
  and every write; `tests/contract.rs` covers the matrix, every stub method,
  detection, the fixture probe, the mock, and the dependency references.
  `make distro-contract-check` runs the tests and the live probe; it is in
  `make gates`.

## Consequences

- T254–T259 implement concrete backends behind the existing traits; the
  Settings UI does not change when a stub flips to `Available`.
- A Settings host exposes `CapabilityStatus::view()` per capability; QML never
  links Rust and never shells out.
- The probe is safe in CI and on read-only hosts: it checks executable bits
  and reads `/etc/os-release`, it never runs a distro command.
- The clean-VM matrix (T259) is documented now; a package is not done until a
  clean VM reaches a usable desktop and the absence pass is recorded.
- Adding a distribution is additive: a new `DistroFamily` row and matrix
  entries, not a UI change. A distribution outside the matrix stays
  best-effort `Unknown`, not a guessed backend.
