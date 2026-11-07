// SPDX-License-Identifier: MIT
//! Cross-distro administration adapter contract (T221).
//!
//! Dragonfruit targets Fedora, Ubuntu, Debian, and Arch. Those distributions
//! share the runtime host stack (systemd/logind, Wayland/DRM, libinput,
//! NetworkManager, BlueZ, PipeWire/WirePlumber, UPower, UDisks2, polkit,
//! portals), but differ in the administration tools behind Settings: the
//! package manager, firewall, users/groups tooling, time sync, locale tooling,
//! autologin file, default-app registration, and extra app-discovery paths.
//!
//! This crate is the **dependency-free contract** between a Settings pane and
//! those tools. It defines one trait per capability, a three-state status
//! (`Available` / `Absent` / `Unsupported`), a per-distro command/API matrix,
//! the required/recommended/optional dependency sets with their absence
//! behaviour, a mock backend for CI, and stub backends for the four supported
//! distros. The concrete backends land in T254–T259; the stubs return the
//! typed [`DistroError::NotSupported`] error today.
//!
//! # The rules
//!
//! * The UI never shells out to a distro command. A pane asks an adapter for
//!   [`CapabilityAdapter::available`] and [`CapabilityAdapter::status`] and
//!   renders the typed answer; it never names a distribution.
//! * Absence is a state, not an error dialog. [`CapabilityStatus::Absent`] is a
//!   normal "the host tool is not installed" answer; only a backend that was
//!   selected but is not implemented yet reports `Unsupported`.
//! * The adapter is selected at startup from `/etc/os-release` plus service
//!   detection, never from a hardcoded distro list in the UI.
//! * A missing optional service disables only the affected feature. The
//!   required/recommended/optional sets and their absence paths live in
//!   [`dependencies`](crate::dependencies) and mirror [12-packaging.md].
//!
//! # The live probe
//!
//! `dragonfruit-distro-probe` prints the detected distribution, the selected
//! stub backend, the host capability probe, and each adapter's status. The
//! same report is exercised by `tests/contract.rs` and by
//! `make distro-contract-check`.
//!
//! [12-packaging.md]: ../../../docs/design/12-packaging.md

mod capability;
mod dependency;
mod distro;
mod matrix;
mod mock;
mod probe;
mod stub;
mod traits;

pub use capability::{
    Absence, BackendId, Capability, CapabilityAdapter, CapabilityInfo, CapabilityState,
    CapabilityStatus, CapabilityView, DistroError, Unsupported,
};
pub use dependency::{
    dependencies, dependency, optional, recommended, required, AbsenceBehavior, Dependency,
    DependencySet, DEPENDENCIES,
};
pub use distro::{parse_os_release, Distro, DistroFamily, SupportTier};
pub use matrix::{spec, specs_for, BackendSpec, MATRIX};
pub use mock::{
    MockAppDiscovery, MockAutologin, MockBackend, MockDefaultApps, MockFirewall, MockLocale,
    MockTime, MockUpdates, MockUsers,
};
pub use probe::{DetectedCapability, DistroReport, HostProbe, SystemProbe};
pub use stub::{
    StubAppDiscovery, StubAutologin, StubBackend, StubDefaultApps, StubFirewall, StubLocale,
    StubTime, StubUpdates, StubUsers, STUB_REASON,
};
pub use traits::*;
