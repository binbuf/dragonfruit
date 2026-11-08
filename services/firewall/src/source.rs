// SPDX-License-Identifier: MIT
//! The transport seam and its fixture.

use dragonfruit_distro_adapters::{FirewallBackend, Port};
use dragonfruit_system_adapters::AdapterError;

/// One raw firewall read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FirewallData {
    /// The detected manager.
    pub backend: FirewallBackend,
    /// Whether the manager reports the firewall enabled.
    pub enabled: bool,
    /// Whether the manager can report its enabled state.
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

/// The result of one firewall write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirewallWriteOutcome {
    /// The manager accepted the write; the host re-reads.
    Applied,
    /// polkit refused the write. `note` records why.
    Denied(String),
    /// No supported manager is present.
    Absent,
    /// The write failed for a reason other than authorization.
    Failed(AdapterError),
    /// The manager does not support the write (nftables has no enable/disable).
    Unsupported,
}

/// Reads the firewall manager and performs its explicit writes.
pub trait FirewallSource {
    /// One read. `Ok(None)` means no supported manager is installed; that is a
    /// normal state, not an error.
    fn read(&mut self) -> Result<Option<FirewallData>, AdapterError>;

    /// Enable or disable the firewall. One explicit write.
    fn set_enabled(&mut self, _enabled: bool) -> FirewallWriteOutcome {
        FirewallWriteOutcome::Unsupported
    }

    /// Open a port. One explicit write.
    fn open_port(&mut self, _port: Port) -> FirewallWriteOutcome {
        FirewallWriteOutcome::Unsupported
    }

    /// Close a port. One explicit write.
    fn close_port(&mut self, _port: Port) -> FirewallWriteOutcome {
        FirewallWriteOutcome::Unsupported
    }

    /// Open a named service. One explicit write.
    fn open_service(&mut self, _service: &str) -> FirewallWriteOutcome {
        FirewallWriteOutcome::Unsupported
    }

    /// Close a named service. One explicit write.
    fn close_service(&mut self, _service: &str) -> FirewallWriteOutcome {
        FirewallWriteOutcome::Unsupported
    }
}

/// A fixture-backed firewall with a simulated manager lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockFirewall {
    present: bool,
    data: FirewallData,
    failure: Option<AdapterError>,
    deny: Option<String>,
    reads: u32,
    writes: u32,
}

impl MockFirewall {
    /// A host with no supported firewall manager.
    pub fn absent() -> Self {
        MockFirewall {
            present: false,
            data: FirewallData::default(),
            failure: None,
            deny: None,
            reads: 0,
            writes: 0,
        }
    }

    /// A present firewalld with two open ports and two services.
    pub fn present() -> Self {
        MockFirewall {
            present: true,
            data: FirewallData {
                backend: FirewallBackend::Firewalld,
                enabled: true,
                status_known: true,
                default_zone: Some("public".to_owned()),
                open_ports: vec![
                    Port::new(22, dragonfruit_distro_adapters::Protocol::Tcp),
                    Port::new(443, dragonfruit_distro_adapters::Protocol::Tcp),
                ],
                open_services: vec!["ssh".to_owned(), "https".to_owned()],
                note: None,
            },
            failure: None,
            deny: None,
            reads: 0,
            writes: 0,
        }
    }

    /// A host where nftables is detected but has no service status.
    pub fn nftables_unavailable() -> Self {
        let mut mock = MockFirewall::present();
        mock.data.backend = FirewallBackend::Nftables;
        mock.data.enabled = false;
        mock.data.status_known = false;
        mock.data.default_zone = None;
        mock.data.open_ports.clear();
        mock.data.open_services.clear();
        mock.data.note =
            Some("nftables has no service status; enable/disable is managed by the host.".to_owned());
        mock
    }

    /// A present manager that fails every read.
    pub fn failing(message: impl Into<String>) -> Self {
        let mut mock = MockFirewall::present();
        mock.failure = Some(AdapterError::new(message));
        mock
    }

    /// Make every write a polkit denial with `note`.
    pub fn deny_writes(&mut self, note: impl Into<String>) {
        self.deny = Some(note.into());
    }

    /// The manager goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The manager comes back.
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// How many reads the source has served.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many writes the source has served.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    /// The simulated manager data.
    pub fn data(&self) -> &FirewallData {
        &self.data
    }

    fn begin_write(&mut self) -> Option<FirewallWriteOutcome> {
        self.writes += 1;
        if !self.present {
            return Some(FirewallWriteOutcome::Absent);
        }
        if let Some(error) = &self.failure {
            return Some(FirewallWriteOutcome::Failed(error.clone()));
        }
        if !self.data.status_known {
            return Some(FirewallWriteOutcome::Unsupported);
        }
        self.deny
            .clone()
            .map(FirewallWriteOutcome::Denied)
    }
}

impl FirewallSource for MockFirewall {
    fn read(&mut self) -> Result<Option<FirewallData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone()))
    }

    fn set_enabled(&mut self, enabled: bool) -> FirewallWriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        self.data.enabled = enabled;
        FirewallWriteOutcome::Applied
    }

    fn open_port(&mut self, port: Port) -> FirewallWriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        if !self.data.open_ports.contains(&port) {
            self.data.open_ports.push(port);
        }
        FirewallWriteOutcome::Applied
    }

    fn close_port(&mut self, port: Port) -> FirewallWriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        self.data.open_ports.retain(|open| *open != port);
        FirewallWriteOutcome::Applied
    }

    fn open_service(&mut self, service: &str) -> FirewallWriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        if !self.data.open_services.iter().any(|open| open == service) {
            self.data.open_services.push(service.to_owned());
        }
        FirewallWriteOutcome::Applied
    }

    fn close_service(&mut self, service: &str) -> FirewallWriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        self.data.open_services.retain(|open| open != service);
        FirewallWriteOutcome::Applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockFirewall::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_present_mock_serves_the_rules_and_mutates_on_writes() {
        let mut mock = MockFirewall::present();
        let read = mock.read().unwrap().unwrap();
        assert_eq!(read.backend, FirewallBackend::Firewalld);
        assert_eq!(read.open_ports.len(), 2);
        assert_eq!(
            mock.set_enabled(false),
            FirewallWriteOutcome::Applied
        );
        assert_eq!(
            mock.open_port(Port::new(8080, dragonfruit_distro_adapters::Protocol::Tcp)),
            FirewallWriteOutcome::Applied
        );
        assert_eq!(
            mock.close_service("https"),
            FirewallWriteOutcome::Applied
        );
        assert!(!mock.data().enabled);
        assert_eq!(mock.data().open_ports.len(), 3);
        assert_eq!(mock.data().open_services, vec!["ssh"]);
        assert_eq!(mock.writes(), 3);
    }

    #[test]
    fn a_denied_write_is_reported_and_does_not_mutate() {
        let mut mock = MockFirewall::present();
        mock.deny_writes("not authorized");
        assert_eq!(
            mock.set_enabled(false),
            FirewallWriteOutcome::Denied("not authorized".to_owned())
        );
        assert!(mock.data().enabled);
    }

    #[test]
    fn an_unavailable_status_refuses_writes() {
        let mut mock = MockFirewall::nftables_unavailable();
        assert_eq!(
            mock.set_enabled(true),
            FirewallWriteOutcome::Unsupported
        );
        assert!(!mock.data().enabled);
    }
}
