// SPDX-License-Identifier: MIT
//! T-15.12a acceptance: the Printers and Scanners adapter reports state and
//! events against a mock, and an absent host stack is a normal hidden state.
//! No CUPS, SANE, or hardware is involved.

use dragonfruit_printer_adapter::{
    MockPrint, PrintChange, PrintData, PrintJobData, PrintOutcome, PrintSnapshot, PrintSource,
    PrinterAdapter, PrinterState, ScannerKind,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the live CUPS + SANE source
/// builds from `lpstat -l -t` and `scanimage -L`.
fn fixture() -> PrintData {
    serde_json::from_str(include_str!("fixtures/printers-workstation.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_queues_and_scanners() {
    let mut adapter = PrinterAdapter::new(MockPrint::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("the host stack answered");
    assert!(snapshot.present());
    assert_eq!(snapshot.glyph(), "printer");
    assert_eq!(snapshot.label(), "2 Printers, 1 Scanner");
    assert_eq!(snapshot.printer_count(), 2);
    assert_eq!(snapshot.scanner_count(), 1);
    assert_eq!(snapshot.queued_job_count(), 1);

    // The default queue sorts first.
    assert_eq!(snapshot.printers()[0].name, "Canon_MF230");
    assert!(snapshot.printers()[0].is_default());
    assert_eq!(snapshot.printers()[0].display_name(), "Canon MF230");
    assert_eq!(snapshot.printers()[0].location, "Office");
    assert_eq!(snapshot.printers()[0].uri, "ipp://192.168.0.5/ipp/print");
    assert_eq!(snapshot.printers()[0].state_label(), "Idle");
    assert_eq!(snapshot.printers()[0].state_message, "Idle, Last Used");
    assert!(snapshot.printers()[0].is_accepting_jobs());
    assert_eq!(snapshot.printers()[0].job_count(), 1);
    assert_eq!(
        snapshot
            .default_printer()
            .map(|printer| printer.name.as_str()),
        Some("Canon_MF230")
    );

    let hp = snapshot.printer_by_name("HP_LaserJet").unwrap();
    assert_eq!(hp.state, PrinterState::Processing);
    assert_eq!(hp.state_label(), "Printing");
    assert_eq!(
        snapshot
            .scanner_by_device("epson2:net:192.168.0.7")
            .unwrap()
            .kind,
        ScannerKind::Flatbed
    );

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::PRINTER);
    assert_eq!(slot.id, AdapterId::PRINTER);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::PRINTER);
}

#[test]
fn the_lifecycle_resubscribes_after_the_host_stack_returns() {
    let mut adapter = PrinterAdapter::new(MockPrint::present(fixture()));
    adapter.refresh();
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // Both halves go away: the item hides, never an error.
    adapter.source_mut().kill_cups();
    adapter.source_mut().kill_sane();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::PRINTER).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // They come back: a re-subscribe and a re-sync, no user-visible error.
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
fn a_queue_move_is_an_observable_event() {
    let mut adapter = PrinterAdapter::new(MockPrint::present(fixture()));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();

    // A job enters the default queue and the default destination moves.
    let mut next = fixture();
    next.printers.as_mut().unwrap()[0].jobs.push(PrintJobData {
        id: 13,
        user: "sam".to_owned(),
        size: 32,
    });
    next.default_printer = "HP_LaserJet".to_owned();
    for printer in next.printers.as_mut().unwrap() {
        printer.is_default = printer.name == "HP_LaserJet";
    }
    adapter.source_mut().push(next);
    adapter.refresh();

    let changes = adapter.drain_changes();
    assert!(changes.contains(&PrintChange::PrinterChanged {
        name: "Canon_MF230".to_owned()
    }));
    assert!(changes.contains(&PrintChange::JobAdded {
        printer: "Canon_MF230".to_owned(),
        id: 13
    }));
    assert!(changes.contains(&PrintChange::DefaultPrinterChanged {
        from: Some("Canon_MF230".to_owned()),
        to: Some("HP_LaserJet".to_owned()),
    }));
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn one_absent_half_is_a_normal_available_state() {
    // CUPS answers but there is no SANE: the queue list stays live and only
    // the scanner half is absent.
    let mut adapter = PrinterAdapter::new(MockPrint::present(PrintData {
        scanners: None,
        ..fixture()
    }));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert_eq!(snapshot.printer_count(), 2);
    assert!(snapshot.printers_available());
    assert!(!snapshot.scanners_available());
    assert!(snapshot.scanners().is_empty());
    assert_eq!(snapshot.label(), "2 Printers");

    // The queue writes are unaffected: absence is layered.
    assert!(adapter.set_default_printer("HP_LaserJet").is_applied());
    assert!(adapter.state().slot(AdapterId::PRINTER).visible);
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No host stack at all: hidden, absent, no error, no panic.
    let mut adapter = PrinterAdapter::new(MockPrint::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());
    assert_eq!(
        adapter.set_default_printer("Canon_MF230"),
        PrintOutcome::Absent
    );

    // The stack is present but unreadable: visible, inert, with the message.
    let mut failing = PrinterAdapter::new(MockPrint::failing("CUPS: timeout"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(failing.state().error().unwrap().message(), "CUPS: timeout");
}

#[test]
fn the_writes_are_explicit_and_do_not_invent_a_snapshot() {
    let mut adapter = PrinterAdapter::new(MockPrint::present(fixture()));
    adapter.refresh();
    let before = adapter.snapshot().unwrap().clone();
    let _ = adapter.drain_events();
    let _ = adapter.drain_changes();

    assert!(adapter.set_default_printer("HP_LaserJet").is_applied());
    assert!(adapter
        .set_printer_accepting_jobs("Canon_MF230", false)
        .is_applied());
    assert!(adapter.cancel_job(12).is_applied());

    assert_eq!(adapter.source().default_sets(), 1);
    assert_eq!(adapter.source().accept_sets(), 1);
    assert_eq!(adapter.source().job_cancels(), 1);
    assert_eq!(adapter.source().writes(), 3);

    // The adapter state is unchanged until the host re-reads; a write never
    // invents a snapshot.
    assert_eq!(adapter.snapshot().unwrap(), &before);
    assert!(adapter.drain_events().is_empty());

    // The daemon (mock) published the result; the host re-reads.
    adapter.refresh();
    let after = adapter.snapshot().unwrap();
    assert_eq!(
        after.default_printer().map(|printer| printer.name.as_str()),
        Some("HP_LaserJet")
    );
    assert!(!after
        .printer_by_name("Canon_MF230")
        .unwrap()
        .is_accepting_jobs());
    assert_eq!(after.queued_job_count(), 0);
}

#[test]
fn a_denied_write_is_reported_and_the_read_state_stays_live() {
    let mut adapter =
        PrinterAdapter::new(MockPrint::present(fixture()).deny_writes("cups: not authorized"));
    adapter.refresh();

    let outcome = adapter.set_default_printer("Canon_MF230");
    assert_eq!(
        outcome,
        PrintOutcome::Denied("cups: not authorized".to_owned())
    );
    assert_eq!(outcome.denial_note(), Some("cups: not authorized"));
    assert!(adapter.state().is_available());
    assert!(adapter.state().slot(AdapterId::PRINTER).visible);
}

#[test]
fn a_failed_write_is_not_a_denial_and_leaves_the_read_state_live() {
    let mut adapter = PrinterAdapter::new(MockPrint::present(fixture()).fail_writes("CUPS: busy"));
    adapter.refresh();

    let outcome = adapter.set_printer_accepting_jobs("Canon_MF230", false);
    assert_eq!(outcome.denial_note(), None);
    assert!(matches!(outcome, PrintOutcome::Failed(_)));
    assert_eq!(
        outcome.error().map(|error| error.message()),
        Some("CUPS: busy")
    );
    assert!(adapter.state().is_available());
}

#[test]
fn the_vocabulary_round_trips_through_its_stable_ids() {
    for state in PrinterState::ALL {
        assert_eq!(PrinterState::from_id(state.id()), state);
        assert_eq!(PrinterState::from_code(state.code()), state);
    }
    assert_eq!(PrinterState::from_id("bogus"), PrinterState::Unknown);
    for kind in ScannerKind::ALL {
        assert_eq!(ScannerKind::from_id(kind.id()), kind);
    }
    assert_eq!(ScannerKind::from_id("bogus"), ScannerKind::Unknown);
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    let source = MockPrint::absent();
    assert!(!source.cups_present());
    assert_eq!(source.reads(), 0);
    // The trait-object path is the same seam the live host fills.
    let mut boxed: Box<dyn PrintSource> = Box::new(MockPrint::absent());
    assert_eq!(boxed.read(), Ok(None));
}

#[test]
fn an_empty_read_is_available_but_not_present() {
    // Both daemons answer but have nothing configured: the adapter is
    // available, and `present()` is what hides the pane.
    let mut adapter = PrinterAdapter::new(MockPrint::present(PrintData {
        printers: Some(vec![]),
        default_printer: String::new(),
        scanners: Some(vec![]),
    }));
    adapter.refresh();
    assert!(adapter.state().is_available());
    assert!(!adapter.snapshot().unwrap().present());
    assert_eq!(
        adapter.snapshot().unwrap().label(),
        "No Printers or Scanners"
    );
}

#[test]
fn the_live_print_reads_when_a_host_stack_is_present() {
    // The live tool path is exercised only where CUPS or SANE exists (the test
    // machine); CI without either reports absence and skips.
    let mut source = dragonfruit_printer_adapter::HostPrint::new();
    match source.read() {
        Ok(None) => {
            // Neither daemon here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = PrintSnapshot::from_data(&data);
            // The typed projection never panics on whatever the tools report.
            let _ = snapshot.label();
            let _ = snapshot.changes(&snapshot);
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
