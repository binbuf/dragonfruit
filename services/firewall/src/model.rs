// SPDX-License-Identifier: MIT
//! The firewall snapshot the Network pane and its sheet render.

use dragonfruit_distro_adapters::{FirewallBackend, Port};

use crate::source::FirewallData;

/// The live firewall state, decoded from one [`FirewallData`] read.
///
/// `status_known` is the honest distinction the pane needs: a manager that
/// cannot report whether it is enabled (nftables) is not "Off". The view maps
/// `status_known: false` to Unavailable with the note, and disables the toggle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FirewallSnapshot {
    /// The detected manager.
    pub backend: FirewallBackend,
    /// Whether the manager reports the firewall enabled.
    pub enabled: bool,
    /// Whether the manager can report its enabled state at all.
    pub status_known: bool,
    /// The default zone, when the manager has one.
    pub default_zone: Option<String>,
    /// The open port rules.
    pub open_ports: Vec<Port>,
    /// The open service rules.
    pub open_services: Vec<String>,
    /// Why the status is unavailable, or a manager note.
    pub note: Option<String>,
}

impl FirewallSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &FirewallData) -> Self {
        FirewallSnapshot {
            backend: data.backend,
            enabled: data.enabled,
            status_known: data.status_known,
            default_zone: data.default_zone.clone(),
            open_ports: data.open_ports.clone(),
            open_services: data.open_services.clone(),
            note: data.note.clone(),
        }
    }

    /// Whether the manager has a status the pane can toggle.
    pub fn is_writable(&self) -> bool {
        self.status_known && self.backend != FirewallBackend::None
    }

    /// A one-line label, e.g. `On` / `Off` / `Unavailable`.
    pub fn label(&self) -> &'static str {
        if !self.status_known {
            "Unavailable"
        } else if self.enabled {
            "On"
        } else {
            "Off"
        }
    }

    /// The number of open port rules.
    pub fn port_count(&self) -> usize {
        self.open_ports.len()
    }

    /// The number of open service rules.
    pub fn service_count(&self) -> usize {
        self.open_services.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_distro_adapters::Protocol;

    fn data() -> FirewallData {
        FirewallData {
            backend: FirewallBackend::Ufw,
            enabled: true,
            status_known: true,
            default_zone: None,
            open_ports: vec![Port::new(22, Protocol::Tcp)],
            open_services: vec!["ssh".to_owned()],
            note: None,
        }
    }

    #[test]
    fn the_snapshot_carries_the_state_and_rules() {
        let snapshot = FirewallSnapshot::from_data(&data());
        assert_eq!(snapshot.backend, FirewallBackend::Ufw);
        assert!(snapshot.enabled);
        assert!(snapshot.is_writable());
        assert_eq!(snapshot.label(), "On");
        assert_eq!(snapshot.port_count(), 1);
        assert_eq!(snapshot.service_count(), 1);
    }

    #[test]
    fn an_unknown_status_is_unavailable_not_off() {
        let mut raw = data();
        raw.backend = FirewallBackend::Nftables;
        raw.enabled = false;
        raw.status_known = false;
        raw.note = Some("nftables has no service status".to_owned());
        let snapshot = FirewallSnapshot::from_data(&raw);
        assert_eq!(snapshot.label(), "Unavailable");
        assert!(!snapshot.is_writable());
        assert_eq!(
            snapshot.note.as_deref(),
            Some("nftables has no service status")
        );
    }
}
