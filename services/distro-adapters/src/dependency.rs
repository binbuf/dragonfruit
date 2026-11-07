// SPDX-License-Identifier: MIT
//! The required/recommended/optional dependency sets and their absence
//! behaviour.
//!
//! The package markers themselves stay canonical in
//! [`packaging/optional-services.json`]; every [`Dependency`] that references
//! an optional service carries that table's id instead of duplicating its
//! package name or license. The Rust table exists so the adapter layer can
//! answer "what happens when this is absent?" without reading packaging
//! metadata at runtime.
//!
//! The policy is [distro-compatibility.md] §15: a missing required **session**
//! service shows a banner and disables the affected feature; a missing
//! optional service is a one-line feature absence; a missing compatibility
//! layer disables only that layer. Nothing blocks the rest of the desktop.
//!
//! [distro-compatibility.md]: ../../../docs/design/distro-compatibility.md

use crate::capability::Capability;

/// One dependency set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DependencySet {
    /// The core desktop cannot start or be usable without it.
    Required,
    /// Most users want it; the feature still has an absence path.
    Recommended,
    /// Useful when the host already provides it; no first-party feature
    /// depends on it.
    Optional,
}

impl DependencySet {
    /// Every set.
    pub const ALL: [DependencySet; 3] = [
        DependencySet::Required,
        DependencySet::Recommended,
        DependencySet::Optional,
    ];

    /// The stable id used by the probe and packaging metadata.
    pub const fn id(self) -> &'static str {
        match self {
            DependencySet::Required => "required",
            DependencySet::Recommended => "recommended",
            DependencySet::Optional => "optional",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            DependencySet::Required => "Required",
            DependencySet::Recommended => "Recommended",
            DependencySet::Optional => "Optional",
        }
    }
}

/// What happens when a dependency is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbsenceBehavior {
    /// A shell banner; the affected feature disables; the session continues.
    SessionBanner,
    /// One feature disables with a one-line note; no fake toggle.
    FeatureDisabled,
    /// The whole optional pane is hidden, not a dead entry.
    PaneHidden,
    /// One row is hidden or disabled with a reason.
    RowHidden,
    /// A built-in fallback takes over.
    BuiltinFallback,
    /// The feature stays readable but its writes disable.
    ReadOnly,
    /// A build-time dependency; the build fails, the runtime does not.
    BuildFailure,
}

impl AbsenceBehavior {
    /// Every behaviour.
    pub const ALL: [AbsenceBehavior; 7] = [
        AbsenceBehavior::SessionBanner,
        AbsenceBehavior::FeatureDisabled,
        AbsenceBehavior::PaneHidden,
        AbsenceBehavior::RowHidden,
        AbsenceBehavior::BuiltinFallback,
        AbsenceBehavior::ReadOnly,
        AbsenceBehavior::BuildFailure,
    ];

    /// The stable id used by diagnostics.
    pub const fn id(self) -> &'static str {
        match self {
            AbsenceBehavior::SessionBanner => "session-banner",
            AbsenceBehavior::FeatureDisabled => "feature-disabled",
            AbsenceBehavior::PaneHidden => "pane-hidden",
            AbsenceBehavior::RowHidden => "row-hidden",
            AbsenceBehavior::BuiltinFallback => "builtin-fallback",
            AbsenceBehavior::ReadOnly => "read-only",
            AbsenceBehavior::BuildFailure => "build-failure",
        }
    }

    /// The one-line policy the design doc and the pane use.
    pub const fn description(self) -> &'static str {
        match self {
            AbsenceBehavior::SessionBanner => {
                "Shell banner; the affected feature disables; the session continues."
            }
            AbsenceBehavior::FeatureDisabled => {
                "One feature disables with a one-line note; no fake toggle."
            }
            AbsenceBehavior::PaneHidden => "The optional pane is hidden, not a dead entry.",
            AbsenceBehavior::RowHidden => "The row is hidden or disabled with a reason.",
            AbsenceBehavior::BuiltinFallback => "A built-in fallback takes over.",
            AbsenceBehavior::ReadOnly => "The feature stays readable; writes disable.",
            AbsenceBehavior::BuildFailure => {
                "Build-time dependency: the build fails; the runtime is unaffected."
            }
        }
    }
}

/// One dependency and its absence path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dependency {
    /// The stable id.
    pub id: &'static str,
    /// The human name.
    pub name: &'static str,
    /// The set it belongs to.
    pub set: DependencySet,
    /// The capability it backs, when it backs exactly one.
    pub capability: Option<Capability>,
    /// What happens when it is missing.
    pub absence: AbsenceBehavior,
    /// The id in `packaging/optional-services.json`, when it is in that
    /// canonical marker table.
    pub optional_service: Option<&'static str>,
}

/// Every dependency, grouped by set in the declared order.
pub const DEPENDENCIES: &[Dependency] = &[
    // ------------------------------------------------------------- required
    Dependency {
        id: "wayland",
        name: "Wayland, DRM/KMS, and a Mesa/GPU driver",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "libinput",
        name: "libinput + udev",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "systemd-logind",
        name: "systemd + logind",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "networkmanager",
        name: "NetworkManager",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "bluez",
        name: "BlueZ",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "pipewire",
        name: "PipeWire + WirePlumber",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "upower",
        name: "UPower",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "udisks2",
        name: "UDisks2",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "polkit",
        name: "polkit",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "xdg-desktop-portal",
        name: "xdg-desktop-portal + a desktop backend",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "qt6",
        name: "Qt 6 runtime",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::SessionBanner,
        optional_service: None,
    },
    Dependency {
        id: "xwayland",
        name: "Xwayland (X11 application compatibility)",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: None,
    },
    Dependency {
        id: "rust-toolchain",
        name: "Rust toolchain (build time)",
        set: DependencySet::Required,
        capability: None,
        absence: AbsenceBehavior::BuildFailure,
        optional_service: None,
    },
    // ---------------------------------------------------------- recommended
    Dependency {
        id: "cups",
        name: "CUPS",
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("print.cups"),
    },
    Dependency {
        id: "sane",
        name: "SANE",
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("print.sane"),
    },
    Dependency {
        id: "colord",
        name: "colord",
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("color.colord"),
    },
    Dependency {
        id: "fprintd",
        name: "fprintd",
        set: DependencySet::Recommended,
        capability: Some(Capability::UsersGroups),
        absence: AbsenceBehavior::RowHidden,
        optional_service: Some("auth.fprintd"),
    },
    Dependency {
        id: "packagekit",
        name: "PackageKit",
        set: DependencySet::Recommended,
        capability: Some(Capability::Updates),
        absence: AbsenceBehavior::BuiltinFallback,
        optional_service: Some("packages.packagekit"),
    },
    Dependency {
        id: "accountsservice",
        name: "accountsservice",
        set: DependencySet::Recommended,
        capability: Some(Capability::UsersGroups),
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("accounts.accountsservice"),
    },
    Dependency {
        id: "goa",
        name: "GNOME Online Accounts", // df-allow-desktop-name: third-party service name
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::PaneHidden,
        optional_service: Some("accounts.goa"),
    },
    Dependency {
        id: "ibus",
        name: "ibus",
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::RowHidden,
        optional_service: Some("ime.ibus"),
    },
    Dependency {
        id: "fcitx5",
        name: "fcitx5",
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::RowHidden,
        optional_service: Some("ime.fcitx5"),
    },
    Dependency {
        id: "orca",
        name: "Orca",
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("accessibility.orca"),
    },
    Dependency {
        id: "speech-dispatcher",
        name: "speech-dispatcher",
        set: DependencySet::Recommended,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("accessibility.speech"),
    },
    Dependency {
        id: "flatpak",
        name: "Flatpak",
        set: DependencySet::Recommended,
        capability: Some(Capability::AppDiscovery),
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("packages.flatpak"),
    },
    // -------------------------------------------------------------- optional
    Dependency {
        id: "tracker",
        name: "Tracker",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::BuiltinFallback,
        optional_service: Some("search.tracker"),
    },
    Dependency {
        id: "baloo",
        name: "Baloo",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::BuiltinFallback,
        optional_service: Some("search.baloo"),
    },
    Dependency {
        id: "recoll",
        name: "Recoll",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::BuiltinFallback,
        optional_service: Some("search.recoll"),
    },
    Dependency {
        id: "restic",
        name: "restic",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("backup.restic"),
    },
    Dependency {
        id: "pika",
        name: "Pika Backup",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("backup.pika"),
    },
    Dependency {
        id: "timeshift",
        name: "Timeshift",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("backup.timeshift"),
    },
    Dependency {
        id: "localsend",
        name: "LocalSend",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::PaneHidden,
        optional_service: Some("transfer.localsend"),
    },
    Dependency {
        id: "snapd",
        name: "snapd",
        set: DependencySet::Optional,
        capability: Some(Capability::AppDiscovery),
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("packages.snapd"),
    },
    Dependency {
        id: "firewalld",
        name: "firewalld",
        set: DependencySet::Optional,
        capability: Some(Capability::Firewall),
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("firewall.firewalld"),
    },
    Dependency {
        id: "ufw",
        name: "ufw",
        set: DependencySet::Optional,
        capability: Some(Capability::Firewall),
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("firewall.ufw"),
    },
    Dependency {
        id: "nftables",
        name: "nftables",
        set: DependencySet::Optional,
        capability: Some(Capability::Firewall),
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: None,
    },
    Dependency {
        id: "iptables",
        name: "iptables",
        set: DependencySet::Optional,
        capability: Some(Capability::Firewall),
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: None,
    },
    Dependency {
        id: "espeak-ng",
        name: "espeak-ng",
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::FeatureDisabled,
        optional_service: Some("accessibility.espeak"),
    },
    Dependency {
        id: "gnome-software",   // df-allow-desktop-name: third-party package id
        name: "GNOME Software", // df-allow-desktop-name: third-party app name
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::PaneHidden,
        optional_service: Some("software.gnome-software"), // df-allow-desktop-name: canonical marker id
    },
    Dependency {
        id: "discover",
        name: "KDE Discover", // df-allow-desktop-name: third-party app name
        set: DependencySet::Optional,
        capability: None,
        absence: AbsenceBehavior::PaneHidden,
        optional_service: Some("software.discover"),
    },
];

/// Every dependency in `set`.
pub fn dependencies(set: DependencySet) -> impl Iterator<Item = &'static Dependency> {
    DEPENDENCIES
        .iter()
        .filter(move |dependency| dependency.set == set)
}

/// The required set.
pub fn required() -> impl Iterator<Item = &'static Dependency> {
    dependencies(DependencySet::Required)
}

/// The recommended set.
pub fn recommended() -> impl Iterator<Item = &'static Dependency> {
    dependencies(DependencySet::Recommended)
}

/// The optional set.
pub fn optional() -> impl Iterator<Item = &'static Dependency> {
    dependencies(DependencySet::Optional)
}

/// One dependency by id.
pub fn dependency(id: &str) -> Option<&'static Dependency> {
    DEPENDENCIES.iter().find(|dependency| dependency.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical marker table, read at compile time so the test catches
    /// drift without a runtime dependency or a JSON parser.
    const OPTIONAL_SERVICES: &str = include_str!("../../../packaging/optional-services.json");

    #[test]
    fn every_set_has_entries() {
        for set in DependencySet::ALL {
            assert!(dependencies(set).count() > 0, "{set:?}");
        }
        assert_eq!(
            required().count() + recommended().count() + optional().count(),
            DEPENDENCIES.len()
        );
    }

    #[test]
    fn every_dependency_has_an_absence_path() {
        for dependency in DEPENDENCIES {
            assert!(!dependency.id.is_empty());
            assert!(!dependency.name.is_empty());
            assert!(!dependency.absence.id().is_empty());
            assert!(!dependency.absence.description().is_empty());
        }
    }

    #[test]
    fn optional_service_references_exist_in_the_canonical_table() {
        for dependency in DEPENDENCIES {
            if let Some(service) = dependency.optional_service {
                let needle = format!("\"id\": \"{service}\"");
                assert!(
                    OPTIONAL_SERVICES.contains(&needle),
                    "{} references {service}, missing from packaging/optional-services.json",
                    dependency.id
                );
            }
        }
    }

    #[test]
    fn the_required_set_names_the_shared_host_stack() {
        for id in [
            "wayland",
            "systemd-logind",
            "networkmanager",
            "bluez",
            "pipewire",
            "upower",
            "udisks2",
            "polkit",
            "xdg-desktop-portal",
        ] {
            assert!(dependency(id).is_some(), "missing required {id}");
            assert_eq!(dependency(id).unwrap().set, DependencySet::Required);
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = DEPENDENCIES.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), DEPENDENCIES.len());
    }

    #[test]
    fn absence_behaviour_ids_are_stable() {
        for behavior in AbsenceBehavior::ALL {
            assert!(!behavior.id().is_empty());
            assert!(!behavior.description().is_empty());
        }
        assert_eq!(AbsenceBehavior::PaneHidden.id(), "pane-hidden");
        assert_eq!(DependencySet::Recommended.id(), "recommended");
    }
}
