// SPDX-License-Identifier: MIT
//! The firewall adapter: one read path plus the explicit writes over the
//! shared three-state contract.

use dragonfruit_distro_adapters::Port;
use dragonfruit_system_adapters::{
    Adapter, AdapterError, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::FirewallSnapshot;
use crate::source::{FirewallSource, FirewallWriteOutcome};

/// How much of the firewall adapter the session may use.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FirewallAccess {
    /// Reads and writes are both allowed.
    #[default]
    ReadWrite,
    /// A write was refused by polkit; `note` records why.
    ReadOnly {
        /// The recorded degradation note (the denial message).
        note: String,
    },
}

impl FirewallAccess {
    /// A read-only degradation carrying `note`.
    pub fn read_only(note: impl Into<String>) -> Self {
        FirewallAccess::ReadOnly { note: note.into() }
    }

    /// Whether writes are disabled.
    pub fn is_read_only(&self) -> bool {
        matches!(self, FirewallAccess::ReadOnly { .. })
    }

    /// The recorded degradation note, when read-only.
    pub fn note(&self) -> Option<&str> {
        match self {
            FirewallAccess::ReadWrite => None,
            FirewallAccess::ReadOnly { note } => Some(note),
        }
    }
}

/// What happened to one firewall write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirewallResult {
    /// The manager accepted the write; the host re-reads.
    Applied,
    /// polkit refused the write. The adapter is now read-only.
    Denied {
        /// The recorded denial note.
        note: String,
    },
    /// No supported manager is present.
    Absent,
    /// The write failed for a reason other than authorization.
    Failed(AdapterError),
    /// The manager does not support the write (nftables).
    Unsupported,
}

/// The firewall status adapter.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`]: a session booted without a firewall manager
/// shows "No firewall detected" and never blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirewallAdapter<S> {
    source: S,
    state: AdapterState<FirewallSnapshot>,
    subscription: Subscription,
    access: FirewallAccess,
}

impl<S: FirewallSource> FirewallAdapter<S> {
    /// A fresh adapter over `source`, absent until the first refresh.
    pub fn new(source: S) -> Self {
        FirewallAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
            access: FirewallAccess::ReadWrite,
        }
    }

    /// Read the manager once and update the state and the event stream.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(FirewallSnapshot::from_data(&data));
                self.subscription.changed();
            }
            Ok(None) => {
                self.state = AdapterState::Unavailable;
                self.subscription.absent();
            }
            Err(error) => {
                self.subscription.subscribed();
                self.state = AdapterState::Error(error);
                self.subscription.changed();
            }
        }
    }

    /// The live snapshot, when a manager answered.
    pub fn snapshot(&self) -> Option<&FirewallSnapshot> {
        self.state.snapshot()
    }

    /// Enable or disable the firewall. One explicit write.
    pub fn set_enabled(&mut self, enabled: bool) -> FirewallResult {
        self.write(|source| source.set_enabled(enabled))
    }

    /// Open a port. One explicit write.
    pub fn open_port(&mut self, port: Port) -> FirewallResult {
        self.write(|source| source.open_port(port))
    }

    /// Close a port. One explicit write.
    pub fn close_port(&mut self, port: Port) -> FirewallResult {
        self.write(|source| source.close_port(port))
    }

    /// Open a named service. One explicit write.
    pub fn open_service(&mut self, service: &str) -> FirewallResult {
        self.write(|source| source.open_service(service))
    }

    /// Close a named service. One explicit write.
    pub fn close_service(&mut self, service: &str) -> FirewallResult {
        self.write(|source| source.close_service(service))
    }

    /// Whether a polkit denial has degraded the adapter to read-only.
    pub fn is_read_only(&self) -> bool {
        self.access.is_read_only()
    }

    /// The recorded read-only degradation note, if any.
    pub fn degradation_note(&self) -> Option<&str> {
        self.access.note()
    }

    /// The transport this adapter reads.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The transport, mutably (for tests and lifecycle control).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// How many times the adapter subscribed.
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }

    fn write(&mut self, write: impl FnOnce(&mut S) -> FirewallWriteOutcome) -> FirewallResult {
        if let FirewallAccess::ReadOnly { note } = &self.access {
            return FirewallResult::Denied { note: note.clone() };
        }
        match write(&mut self.source) {
            FirewallWriteOutcome::Applied => FirewallResult::Applied,
            FirewallWriteOutcome::Denied(note) => {
                self.access = FirewallAccess::read_only(note.clone());
                FirewallResult::Denied { note }
            }
            FirewallWriteOutcome::Absent => {
                self.state = AdapterState::Unavailable;
                self.subscription.absent();
                FirewallResult::Absent
            }
            FirewallWriteOutcome::Failed(error) => FirewallResult::Failed(error),
            FirewallWriteOutcome::Unsupported => FirewallResult::Unsupported,
        }
    }
}

impl<S: FirewallSource> Adapter for FirewallAdapter<S> {
    type Snapshot = FirewallSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::FIREWALL
    }

    fn state(&self) -> &AdapterState<Self::Snapshot> {
        &self.state
    }

    fn connection(&self) -> ConnectionState {
        self.subscription.state()
    }

    fn drain_events(&mut self) -> Vec<AdapterEvent> {
        self.subscription.drain_events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MockFirewall;
    use dragonfruit_distro_adapters::{FirewallBackend, Protocol};

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = FirewallAdapter::new(MockFirewall::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert!(!adapter.state().slot(AdapterId::FIREWALL).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = FirewallAdapter::new(MockFirewall::present());
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(adapter.subscriptions(), 1);
        let snapshot = adapter.snapshot().unwrap();
        assert_eq!(snapshot.backend, FirewallBackend::Firewalld);
        assert!(snapshot.enabled);
        assert_eq!(snapshot.port_count(), 2);
        assert_eq!(snapshot.service_count(), 2);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
    }

    #[test]
    fn a_missing_manager_hides_the_slot() {
        let mut adapter = FirewallAdapter::new(MockFirewall::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::FIREWALL).visible);
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut adapter = FirewallAdapter::new(MockFirewall::failing("firewall-cmd: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "firewall-cmd: timeout"
        );
    }

    #[test]
    fn writes_apply_and_a_denial_degrades_to_read_only() {
        let mut adapter = FirewallAdapter::new(MockFirewall::present());
        adapter.refresh();
        assert_eq!(adapter.set_enabled(false), FirewallResult::Applied);
        assert_eq!(
            adapter.open_port(Port::new(8080, Protocol::Tcp)),
            FirewallResult::Applied
        );
        assert_eq!(
            adapter.close_port(Port::new(22, Protocol::Tcp)),
            FirewallResult::Applied
        );
        assert_eq!(
            adapter.open_service("mdns"),
            FirewallResult::Applied
        );

        adapter.source_mut().deny_writes("not authorized");
        assert_eq!(
            adapter.set_enabled(true),
            FirewallResult::Denied {
                note: "not authorized".to_owned()
            }
        );
        assert!(adapter.is_read_only());
        // Further writes are refused locally.
        assert!(matches!(
            adapter.open_port(Port::new(1, Protocol::Udp)),
            FirewallResult::Denied { .. }
        ));
        // Four applied writes plus the one denied write that degraded us; the
        // fifth local refusal never reached the source.
        assert_eq!(adapter.source().writes(), 5);
    }

    #[test]
    fn a_write_while_absent_reports_absence_and_hides() {
        let mut adapter = FirewallAdapter::new(MockFirewall::absent());
        assert_eq!(adapter.set_enabled(true), FirewallResult::Absent);
        assert!(adapter.state().is_unavailable());
    }

    #[test]
    fn an_nftables_status_is_unavailable_and_writes_are_unsupported() {
        let mut adapter = FirewallAdapter::new(MockFirewall::nftables_unavailable());
        adapter.refresh();
        let snapshot = adapter.snapshot().unwrap();
        assert_eq!(snapshot.backend, FirewallBackend::Nftables);
        assert!(!snapshot.status_known);
        assert_eq!(snapshot.label(), "Unavailable");
        assert_eq!(
            adapter.set_enabled(true),
            FirewallResult::Unsupported
        );
    }
}
