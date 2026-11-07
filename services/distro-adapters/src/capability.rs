// SPDX-License-Identifier: MIT
//! The capability vocabulary, the three-state adapter status, and the typed
//! error every distro backend returns.
//!
//! The status is deliberately small and wire-friendly: a Settings pane asks
//! [`CapabilityAdapter::available`] and [`CapabilityAdapter::status`], then
//! renders `Available`, `Absent`, or `Unsupported` without knowing which
//! distribution it is running on. Absence is a normal state; only a selected
//! backend that is not implemented yet is `Unsupported`.

use std::fmt;

use crate::distro::DistroFamily;

/// One administration capability a Settings pane can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Software updates and package management.
    Updates,
    /// Firewall state and port rules.
    Firewall,
    /// Users, groups, and administrator membership.
    UsersGroups,
    /// Time zone and NTP.
    Time,
    /// System locale and language.
    Locale,
    /// Display-manager automatic login.
    Autologin,
    /// Default applications and MIME handlers.
    DefaultApps,
    /// Application discovery paths and install sources.
    AppDiscovery,
}

impl Capability {
    /// Every capability, in matrix order.
    pub const ALL: [Capability; 8] = [
        Capability::Updates,
        Capability::Firewall,
        Capability::UsersGroups,
        Capability::Time,
        Capability::Locale,
        Capability::Autologin,
        Capability::DefaultApps,
        Capability::AppDiscovery,
    ];

    /// The stable id used by the wire, the probe, and diagnostics.
    pub const fn id(self) -> &'static str {
        match self {
            Capability::Updates => "updates",
            Capability::Firewall => "firewall",
            Capability::UsersGroups => "users-groups",
            Capability::Time => "time",
            Capability::Locale => "locale",
            Capability::Autologin => "autologin",
            Capability::DefaultApps => "default-apps",
            Capability::AppDiscovery => "app-discovery",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            Capability::Updates => "Software Update",
            Capability::Firewall => "Firewall",
            Capability::UsersGroups => "Users & Groups",
            Capability::Time => "Date & Time",
            Capability::Locale => "Language & Region",
            Capability::Autologin => "Automatic Login",
            Capability::DefaultApps => "Default Apps",
            Capability::AppDiscovery => "Application Discovery",
        }
    }

    /// The Settings pane that consumes the capability.
    pub const fn settings_pane(self) -> &'static str {
        match self {
            Capability::Updates => "Software Update",
            Capability::Firewall => "Network",
            Capability::UsersGroups => "Users & Groups",
            Capability::Time => "Date & Time",
            Capability::Locale => "Language & Region",
            Capability::Autologin => "Users & Groups",
            Capability::DefaultApps => "Default Apps",
            Capability::AppDiscovery => "Applications",
        }
    }
}

/// Stable identity of the backend that answered. `fedora`, `ubuntu`,
/// `debian`, `arch`, `mock`, or `none`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BackendId(&'static str);

impl BackendId {
    /// The Fedora backend.
    pub const FEDORA: BackendId = BackendId("fedora");
    /// The Ubuntu backend.
    pub const UBUNTU: BackendId = BackendId("ubuntu");
    /// The Debian backend.
    pub const DEBIAN: BackendId = BackendId("debian");
    /// The Arch backend.
    pub const ARCH: BackendId = BackendId("arch");
    /// The in-memory mock used by CI and contract tests.
    pub const MOCK: BackendId = BackendId("mock");
    /// No backend (an absent host service or an unsupported distribution).
    pub const NONE: BackendId = BackendId("none");

    /// A backend id for an id the constants above do not name.
    pub const fn new(id: &'static str) -> Self {
        BackendId(id)
    }

    /// The id as it appears on the wire and in the probe.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Default for BackendId {
    fn default() -> Self {
        BackendId::NONE
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<DistroFamily> for BackendId {
    fn from(family: DistroFamily) -> Self {
        match family {
            DistroFamily::Fedora => BackendId::FEDORA,
            DistroFamily::Ubuntu => BackendId::UBUNTU,
            DistroFamily::Debian => BackendId::DEBIAN,
            DistroFamily::Arch => BackendId::ARCH,
            DistroFamily::Unknown => BackendId::NONE,
        }
    }
}

/// The three-state answer a pane renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityState {
    /// The host service or tool is present and the backend can serve it.
    Available,
    /// The host service or tool is not installed. A normal, hidden/disabled
    /// feature, never an error dialog.
    Absent,
    /// The selected backend does not implement this capability yet (the T221
    /// stubs), or the distribution is outside the supported matrix.
    Unsupported,
}

impl CapabilityState {
    /// Every state.
    pub const ALL: [CapabilityState; 3] = [
        CapabilityState::Available,
        CapabilityState::Absent,
        CapabilityState::Unsupported,
    ];

    /// The stable id used by the wire and the probe.
    pub const fn id(self) -> &'static str {
        match self {
            CapabilityState::Available => "available",
            CapabilityState::Absent => "absent",
            CapabilityState::Unsupported => "unsupported",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            CapabilityState::Available => "Available",
            CapabilityState::Absent => "Not installed",
            CapabilityState::Unsupported => "Not supported",
        }
    }
}

/// The detail carried when a capability is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityInfo {
    /// The capability.
    pub capability: Capability,
    /// The selected backend.
    pub backend: BackendId,
    /// The tool or API the backend uses (the short command name).
    pub command: &'static str,
    /// The exact invocation or API description from the matrix.
    pub api: &'static str,
}

/// The detail carried when a capability is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Absence {
    /// The capability.
    pub capability: Capability,
    /// The dependency ids that are missing.
    pub missing: Vec<&'static str>,
    /// A one-line reason the pane shows.
    pub reason: String,
}

/// The detail carried when a selected backend is not implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    /// The capability.
    pub capability: Capability,
    /// The selected backend.
    pub backend: BackendId,
    /// The command/API the eventual backend will use.
    pub command: &'static str,
    /// Why it is unsupported (the T221 stub reason or an unknown distro).
    pub reason: String,
}

/// The three-state capability status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityStatus {
    /// The host tool is present and the backend can serve it.
    Available(CapabilityInfo),
    /// The host tool is not installed. A normal feature-level absence.
    Absent(Absence),
    /// The selected backend is not implemented, or the distro is unsupported.
    Unsupported(Unsupported),
}

impl CapabilityStatus {
    /// The state projected from the detail.
    pub fn state(&self) -> CapabilityState {
        match self {
            CapabilityStatus::Available(_) => CapabilityState::Available,
            CapabilityStatus::Absent(_) => CapabilityState::Absent,
            CapabilityStatus::Unsupported(_) => CapabilityState::Unsupported,
        }
    }

    /// Whether the capability is available.
    pub fn is_available(&self) -> bool {
        matches!(self, CapabilityStatus::Available(_))
    }

    /// Whether the host tool is absent.
    pub fn is_absent(&self) -> bool {
        matches!(self, CapabilityStatus::Absent(_))
    }

    /// Whether the selected backend is unimplemented or unsupported.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, CapabilityStatus::Unsupported(_))
    }

    /// The capability this status describes.
    pub fn capability(&self) -> Capability {
        match self {
            CapabilityStatus::Available(info) => info.capability,
            CapabilityStatus::Absent(absence) => absence.capability,
            CapabilityStatus::Unsupported(unsupported) => unsupported.capability,
        }
    }

    /// The backend that answered; `none` for an absent host service.
    pub fn backend(&self) -> BackendId {
        match self {
            CapabilityStatus::Available(info) => info.backend,
            CapabilityStatus::Absent(_) => BackendId::NONE,
            CapabilityStatus::Unsupported(unsupported) => unsupported.backend,
        }
    }

    /// The command/API the capability would use, when known.
    pub fn command(&self) -> &'static str {
        match self {
            CapabilityStatus::Available(info) => info.api,
            CapabilityStatus::Absent(_) => "",
            CapabilityStatus::Unsupported(unsupported) => unsupported.command,
        }
    }

    /// The one-line reason; empty when available.
    pub fn reason(&self) -> &str {
        match self {
            CapabilityStatus::Available(_) => "",
            CapabilityStatus::Absent(absence) => &absence.reason,
            CapabilityStatus::Unsupported(unsupported) => &unsupported.reason,
        }
    }

    /// The flat, wire-friendly projection a Settings pane or bridge host
    /// serializes. `available` is redundant with `state` on purpose: a pane
    /// that only needs the boolean does not have to match on the enum.
    pub fn view(&self) -> CapabilityView {
        CapabilityView {
            capability: self.capability(),
            state: self.state(),
            available: self.is_available(),
            backend: self.backend(),
            command: self.command(),
            reason: self.reason().to_owned(),
        }
    }

    /// A one-line diagnostic summary for the probe and logs.
    pub fn summary(&self) -> String {
        match self {
            CapabilityStatus::Available(info) => {
                format!("available via {} ({})", info.backend, info.api)
            }
            CapabilityStatus::Absent(absence) => format!("absent: {}", absence.reason),
            CapabilityStatus::Unsupported(unsupported) => {
                format!("unsupported: {}", unsupported.reason)
            }
        }
    }
}

/// The wire-friendly projection of a [`CapabilityStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityView {
    /// The capability.
    pub capability: Capability,
    /// The three-state answer.
    pub state: CapabilityState,
    /// `state == Available`.
    pub available: bool,
    /// The selected backend.
    pub backend: BackendId,
    /// The command/API the capability would use.
    pub command: &'static str,
    /// The reason to show for absent/unsupported; empty when available.
    pub reason: String,
}

/// The typed error a distro backend returns.
///
/// `NotSupported` is the T221 stub answer and the unknown-distribution
/// answer. `Absent` is the normal "the host tool is not installed" answer.
/// `Failed` is a present tool that could not complete the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistroError {
    /// The selected backend does not implement the capability.
    NotSupported {
        /// The capability.
        capability: Capability,
        /// The selected backend.
        backend: BackendId,
        /// The command/API the eventual backend will use.
        command: &'static str,
    },
    /// The host tool or service is absent.
    Absent {
        /// The capability.
        capability: Capability,
        /// A one-line reason.
        reason: String,
    },
    /// The host tool is present but the request failed.
    Failed {
        /// The capability.
        capability: Capability,
        /// The failure message.
        message: String,
    },
}

impl DistroError {
    /// A `NotSupported` error for `capability` on `backend`.
    pub fn not_supported(
        capability: Capability,
        backend: BackendId,
        command: &'static str,
    ) -> Self {
        DistroError::NotSupported {
            capability,
            backend,
            command,
        }
    }

    /// An `Absent` error for `capability`.
    pub fn absent(capability: Capability, reason: impl Into<String>) -> Self {
        DistroError::Absent {
            capability,
            reason: reason.into(),
        }
    }

    /// A `Failed` error for `capability`.
    pub fn failed(capability: Capability, message: impl Into<String>) -> Self {
        DistroError::Failed {
            capability,
            message: message.into(),
        }
    }

    /// The capability the error belongs to.
    pub fn capability(&self) -> Capability {
        match self {
            DistroError::NotSupported { capability, .. }
            | DistroError::Absent { capability, .. }
            | DistroError::Failed { capability, .. } => *capability,
        }
    }

    /// Whether this is the stub/unsupported answer.
    pub fn is_not_supported(&self) -> bool {
        matches!(self, DistroError::NotSupported { .. })
    }

    /// Whether the host tool is absent.
    pub fn is_absent(&self) -> bool {
        matches!(self, DistroError::Absent { .. })
    }

    /// Whether the request failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, DistroError::Failed { .. })
    }
}

impl fmt::Display for DistroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistroError::NotSupported {
                capability,
                backend,
                command,
            } => write!(
                f,
                "not supported: {} on the {} backend ({})",
                capability.id(),
                backend,
                command
            ),
            DistroError::Absent { capability, reason } => {
                write!(f, "absent: {} ({})", capability.id(), reason)
            }
            DistroError::Failed {
                capability,
                message,
            } => write!(f, "failed: {}: {}", capability.id(), message),
        }
    }
}

impl std::error::Error for DistroError {}

/// One administration adapter.
///
/// A Settings pane holds an adapter behind this trait and asks
/// [`available`](Self::available) / [`status`](Self::status); it never names a
/// distribution. The per-capability traits ([`crate::UpdateProvider`],
/// [`crate::FirewallProvider`], …) extend this one with the typed reads and
/// writes.
pub trait CapabilityAdapter {
    /// The capability this adapter serves.
    fn capability(&self) -> Capability;

    /// The selected backend.
    fn backend(&self) -> BackendId;

    /// Whether the capability is available. The default projects
    /// [`status`](Self::status), so a backend only implements the status.
    fn available(&self) -> bool {
        self.status().is_available()
    }

    /// The three-state status the pane renders.
    fn status(&self) -> CapabilityStatus;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn available_status() -> CapabilityStatus {
        CapabilityStatus::Available(CapabilityInfo {
            capability: Capability::Updates,
            backend: BackendId::FEDORA,
            command: "dnf5",
            api: "dnf5 check-update --json",
        })
    }

    #[test]
    fn the_three_states_project_a_wire_view() {
        let view = available_status().view();
        assert!(view.available);
        assert_eq!(view.state, CapabilityState::Available);
        assert_eq!(view.capability, Capability::Updates);
        assert_eq!(view.backend, BackendId::FEDORA);
        assert_eq!(view.command, "dnf5 check-update --json");
        assert!(view.reason.is_empty());

        let absent = CapabilityStatus::Absent(Absence {
            capability: Capability::Firewall,
            missing: vec!["firewalld"],
            reason: "No firewall detected".to_owned(),
        });
        assert!(!absent.view().available);
        assert_eq!(absent.state(), CapabilityState::Absent);
        assert_eq!(absent.backend(), BackendId::NONE);
        assert_eq!(absent.reason(), "No firewall detected");

        let unsupported = CapabilityStatus::Unsupported(Unsupported {
            capability: Capability::Autologin,
            backend: BackendId::ARCH,
            command: "sddm",
            reason: "T221 stub".to_owned(),
        });
        assert!(!unsupported.view().available);
        assert_eq!(unsupported.state(), CapabilityState::Unsupported);
        assert_eq!(unsupported.command(), "sddm");
    }

    #[test]
    fn the_not_supported_error_says_so() {
        let error = DistroError::not_supported(
            Capability::Updates,
            BackendId::FEDORA,
            "dnf5 check-update --json",
        );
        assert!(error.is_not_supported());
        assert_eq!(error.capability(), Capability::Updates);
        let text = error.to_string();
        assert!(text.contains("not supported"), "{text}");
        assert!(text.contains("updates"), "{text}");
        assert!(text.contains("fedora"), "{text}");
        assert!(text.contains("dnf5 check-update --json"), "{text}");
    }

    #[test]
    fn capability_ids_and_labels_are_stable() {
        assert_eq!(Capability::Updates.id(), "updates");
        assert_eq!(Capability::UsersGroups.id(), "users-groups");
        assert_eq!(Capability::AppDiscovery.id(), "app-discovery");
        assert_eq!(Capability::Time.label(), "Date & Time");
        assert_eq!(Capability::Autologin.settings_pane(), "Users & Groups");
        assert_eq!(CapabilityState::Absent.id(), "absent");
        assert_eq!(CapabilityState::Unsupported.label(), "Not supported");
    }
}
