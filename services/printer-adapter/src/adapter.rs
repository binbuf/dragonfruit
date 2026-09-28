// SPDX-License-Identifier: MIT
//! The Printers and Scanners adapter: one read path over the shared contract.
//!
//! It holds the last snapshot the host stack published and exposes the
//! three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is the
//! one place it touches the host stack; the shell bridge calls it when CUPS or
//! SANE changes, so nothing above the adapter polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "host stack absent" default, so a
//! session booted without CUPS or SANE renders a hidden item and never blocks.
//!
//! The writes ([`set_default_printer`](Self::set_default_printer),
//! [`set_printer_accepting_jobs`](Self::set_printer_accepting_jobs), and
//! [`cancel_job`](Self::cancel_job)) are explicit user actions over the same
//! seam. They invent no snapshot: CUPS publishes the resulting state and the
//! host re-reads, so the snapshot stays the single source of truth.

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{PrintChange, PrintSnapshot};
use crate::source::{PrintOutcome, PrintSource};

/// The Printers and Scanners adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct PrinterAdapter<S> {
    source: S,
    state: AdapterState<PrintSnapshot>,
    previous: Option<PrintSnapshot>,
    changes: VecDeque<PrintChange>,
    subscription: Subscription,
}

impl<S: PrintSource> PrinterAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        PrinterAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the host stack once and update the state and the event stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain change;
    /// each later read reports a [`PrintChange`] per field that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = PrintSnapshot::from_data(&data);
                if let Some(previous) = &self.previous {
                    self.changes.extend(snapshot.changes(previous));
                }
                self.previous = Some(snapshot.clone());
                self.state = AdapterState::available(snapshot);
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

    /// Make `name` the default destination. One explicit write.
    pub fn set_default_printer(&mut self, name: &str) -> PrintOutcome {
        self.source.set_default_printer(name)
    }

    /// Accept or reject new jobs on `name`. One explicit write.
    pub fn set_printer_accepting_jobs(&mut self, name: &str, accepting: bool) -> PrintOutcome {
        self.source.set_printer_accepting_jobs(name, accepting)
    }

    /// Cancel the queued job with `job_id`. One explicit write.
    pub fn cancel_job(&mut self, job_id: u32) -> PrintOutcome {
        self.source.cancel_job(job_id)
    }

    /// The live snapshot, when at least one half of the host stack answered.
    pub fn snapshot(&self) -> Option<&PrintSnapshot> {
        self.state.snapshot()
    }

    /// The transport this adapter reads.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The transport, mutably (for tests and lifecycle control).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// How many times the adapter subscribed (a restart counts again).
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }

    /// Take the domain changes pushed since the last drain, in order.
    pub fn drain_changes(&mut self) -> Vec<PrintChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: PrintSource> Adapter for PrinterAdapter<S> {
    type Snapshot = PrintSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::PRINTER
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
    use crate::source::{
        MockPrint, PrintData, PrintJobData, PrinterData, PrinterState, ScanDeviceData, ScannerKind,
    };

    fn data() -> PrintData {
        PrintData {
            printers: Some(vec![PrinterData {
                name: "Canon_MF230".to_owned(),
                display_name: "Canon MF230".to_owned(),
                state: PrinterState::Idle,
                accepting_jobs: true,
                enabled: true,
                is_default: true,
                jobs: vec![],
                ..PrinterData::default()
            }]),
            default_printer: "Canon_MF230".to_owned(),
            scanners: Some(vec![ScanDeviceData {
                device: "epson2:net:192.168.0.7".to_owned(),
                description: "Epson GT-1500 flatbed scanner".to_owned(),
                kind: ScannerKind::Flatbed,
            }]),
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = PrinterAdapter::new(MockPrint::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::PRINTER).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = PrinterAdapter::new(MockPrint::present(data()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.snapshot().unwrap().printer_count(), 1);
        assert_eq!(adapter.snapshot().unwrap().scanner_count(), 1);
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(adapter.source().reads(), 1);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
        // The first read is the baseline: no domain change.
        assert!(adapter.drain_changes().is_empty());
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter = PrinterAdapter::new(MockPrint::present(data()));
        adapter.refresh();
        let _ = adapter.drain_events();

        adapter.source_mut().kill_cups();
        adapter.source_mut().kill_sane();
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

        adapter.source_mut().restart_cups();
        adapter.source_mut().restart_sane();
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.subscriptions(), 2);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: true },
                AdapterEvent::Changed,
            ]
        );
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = PrinterAdapter::new(MockPrint::failing("CUPS: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
        assert_eq!(adapter.state().error().unwrap().message(), "CUPS: timeout");
    }

    #[test]
    fn one_absent_half_is_still_available() {
        let mut adapter = PrinterAdapter::new(MockPrint::present(PrintData {
            scanners: None,
            ..data()
        }));
        adapter.refresh();
        assert!(adapter.state().is_available());
        let snapshot = adapter.snapshot().unwrap();
        assert!(snapshot.printers_available());
        assert!(!snapshot.scanners_available());
        assert_eq!(snapshot.scanner_count(), 0);
    }

    #[test]
    fn a_queue_move_is_a_domain_event() {
        let mut adapter = PrinterAdapter::new(MockPrint::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let mut next = data();
        next.printers.as_mut().unwrap()[0].jobs.push(PrintJobData {
            id: 7,
            user: "dan".to_owned(),
            size: 8,
        });
        adapter.source_mut().push(next);
        adapter.refresh();

        assert_eq!(
            adapter.drain_changes(),
            vec![
                PrintChange::PrinterChanged {
                    name: "Canon_MF230".to_owned()
                },
                PrintChange::JobAdded {
                    printer: "Canon_MF230".to_owned(),
                    id: 7
                },
            ]
        );
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn the_writes_delegate_to_the_source() {
        let mut adapter = PrinterAdapter::new(MockPrint::present(data()));
        adapter.refresh();

        assert!(adapter.set_default_printer("Canon_MF230").is_applied());
        assert!(adapter
            .set_printer_accepting_jobs("Canon_MF230", false)
            .is_applied());
        assert!(adapter.cancel_job(1).is_applied());

        assert_eq!(adapter.source().default_sets(), 1);
        assert_eq!(adapter.source().accept_sets(), 1);
        assert_eq!(adapter.source().job_cancels(), 1);
        assert_eq!(adapter.source().writes(), 3);
    }

    #[test]
    fn writes_while_absent_are_absent() {
        let mut adapter = PrinterAdapter::new(MockPrint::absent());
        adapter.refresh();
        assert_eq!(adapter.set_default_printer("x"), PrintOutcome::Absent);
        // A write never invents state: the adapter stays absent until a read.
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::PRINTER).visible);
    }

    #[test]
    fn a_denial_is_reported_and_leaves_the_read_state_live() {
        let mut adapter =
            PrinterAdapter::new(MockPrint::present(data()).deny_writes("cups: denied"));
        adapter.refresh();
        let outcome = adapter.set_default_printer("Canon_MF230");
        assert_eq!(outcome.denial_note(), Some("cups: denied"));
        assert!(adapter.state().is_available());
        assert!(adapter.state().slot(AdapterId::PRINTER).visible);
    }
}
