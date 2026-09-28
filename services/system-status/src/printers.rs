// SPDX-License-Identifier: MIT
//! The Printers and Scanners half of the bridge host (T-15.12b).
//!
//! The host stack is reached by the `dragonfruit-printer-adapter`; the shell
//! and the Settings app never link it. This module owns the one projection from
//! the typed [`PrintSnapshot`] to the flat JSON view the two consumers draw,
//! plus the three explicit writes the pane raises (make a queue the default,
//! accept/reject new jobs, cancel a queued job). It mirrors the storage and
//! account halves.
//!
//! Absence is layered, exactly as the adapter's is (ADR 0140). The whole view
//! is `unavailable` only when **neither** CUPS nor SANE is reachable. A host
//! whose CUPS has no queue is `available` with `printersAvailable: true` and an
//! empty list; a host with **no SANE** is `available` with
//! `scannersAvailable: false`, so only the scanner half is marked absent. A
//! host that is present but unreadable is `error`, visible and inert with the
//! message.
//!
//! A write never invents a snapshot: the host re-reads the adapter after a
//! successful action, and the read state stays the single source of truth.

use dragonfruit_printer_adapter::{PrintOutcome, PrintSnapshot, PrintSource, PrinterAdapter};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_PRINTERS: &str = "printers";

/// The bridge host for the Printers and Scanners adapter: one state path and
/// the explicit queue writes the Settings pane offers.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintersHost<S> {
    adapter: PrinterAdapter<S>,
}

impl<S: PrintSource> PrintersHost<S> {
    /// A host over a host-stack source.
    pub fn new(source: S) -> Self {
        PrintersHost {
            adapter: PrinterAdapter::new(source),
        }
    }

    /// Re-read the host stack once. Called on startup and after an explicit
    /// action; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The Printers and Scanners view the pane and tile render.
    pub fn view(&self) -> Value {
        printers_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Make the queue `name` the default destination. One explicit write.
    pub fn set_default_printer(&mut self, name: &str) -> Value {
        printers_report(self.adapter.set_default_printer(name))
    }

    /// Accept or reject new jobs on the queue `name`. One explicit write.
    pub fn set_printer_accepting_jobs(&mut self, name: &str, accepting: bool) -> Value {
        printers_report(self.adapter.set_printer_accepting_jobs(name, accepting))
    }

    /// Cancel the queued job with `job_id`. One explicit write.
    pub fn cancel_job(&mut self, job_id: u32) -> Value {
        printers_report(self.adapter.cancel_job(job_id))
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &PrinterAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut PrinterAdapter<S> {
        &mut self.adapter
    }
}

/// Build the Printers and Scanners view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the queue and
/// scanner lists.
pub fn printers_view<S: PrintSource>(adapter: &PrinterAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_PRINTERS, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_PRINTERS,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_PRINTERS, "state": "unavailable" });
    };
    printers_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn printers_snapshot_view(snapshot: &PrintSnapshot) -> Value {
    let printers: Vec<Value> = snapshot.printers().iter().map(printer_json).collect();
    let scanners: Vec<Value> = snapshot.scanners().iter().map(scanner_json).collect();
    json!({
        "kind": KIND_PRINTERS,
        "state": "available",
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "present": snapshot.present(),
        // The two halves degrade independently; CUPS absent only disables the
        // print controls, SANE absent only the scanner list.
        "printersAvailable": snapshot.printers_available(),
        "scannersAvailable": snapshot.scanners_available(),
        "printerCount": snapshot.printer_count(),
        "scannerCount": snapshot.scanner_count(),
        "queuedJobCount": snapshot.queued_job_count(),
        "defaultPrinter": snapshot.default_printer.clone().unwrap_or_default(),
        "printers": printers,
        "scanners": scanners,
    })
}

/// One queue's flat JSON row.
fn printer_json(printer: &dragonfruit_printer_adapter::Printer) -> Value {
    let jobs: Vec<Value> = printer
        .jobs
        .iter()
        .map(|job| {
            json!({
                "id": job.id,
                "user": job.user,
                "size": job.size,
            })
        })
        .collect();
    json!({
        "name": printer.name,
        "displayName": printer.display_name(),
        "makeAndModel": printer.make_and_model,
        "location": printer.location,
        "uri": printer.uri,
        "state": printer.state.id(),
        "stateLabel": printer.state_label(),
        "stateMessage": printer.state_message,
        "acceptingJobs": printer.accepting_jobs,
        "enabled": printer.enabled,
        "isDefault": printer.is_default(),
        "jobCount": printer.job_count(),
        "jobs": jobs,
    })
}

/// One scanner's flat JSON row.
fn scanner_json(scanner: &dragonfruit_printer_adapter::Scanner) -> Value {
    json!({
        "device": scanner.device,
        "description": scanner.description,
        "displayName": scanner.display_name(),
        "kind": scanner.kind.id(),
        "kindLabel": scanner.kind_label(),
    })
}

/// The JSON report for a print write, shaped like the storage and account
/// reports so one shell decode path reads every action. A policy refusal is a
/// `denied` note; the read state stays live.
pub fn printers_report(outcome: PrintOutcome) -> Value {
    match outcome {
        PrintOutcome::Applied => json!({ "outcome": "applied" }),
        PrintOutcome::Denied(note) => json!({ "outcome": "denied", "note": note }),
        PrintOutcome::Absent => json!({ "outcome": "absent" }),
        PrintOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_printer_adapter::{
        MockPrint, PrintData, PrintJobData, PrinterData, PrinterState, ScanDeviceData, ScannerKind,
    };

    fn data() -> PrintData {
        PrintData {
            printers: Some(vec![
                PrinterData {
                    name: "HP_LaserJet".to_owned(),
                    display_name: "HP LaserJet".to_owned(),
                    state: PrinterState::Processing,
                    state_message: "Printing".to_owned(),
                    accepting_jobs: true,
                    enabled: true,
                    jobs: vec![PrintJobData {
                        id: 9,
                        user: "sam".to_owned(),
                        size: 64,
                    }],
                    ..PrinterData::default()
                },
                PrinterData {
                    name: "Canon_MF230".to_owned(),
                    display_name: "Canon MF230".to_owned(),
                    make_and_model: "Canon MF230 Series".to_owned(),
                    location: "Office".to_owned(),
                    uri: "ipp://192.168.0.5/ipp/print".to_owned(),
                    state: PrinterState::Idle,
                    state_message: "Idle, Last Used".to_owned(),
                    accepting_jobs: true,
                    enabled: true,
                    is_default: true,
                    jobs: vec![],
                },
            ]),
            default_printer: "Canon_MF230".to_owned(),
            scanners: Some(vec![ScanDeviceData {
                device: "epson2:net:192.168.0.7".to_owned(),
                description: "Epson GT-1500 flatbed scanner".to_owned(),
                kind: ScannerKind::Flatbed,
            }]),
        }
    }

    #[test]
    fn an_absent_host_stack_projects_a_hidden_slot() {
        let mut host = PrintersHost::new(MockPrint::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "printers");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_host_projects_the_queues_and_scanners() {
        let mut host = PrintersHost::new(MockPrint::present(data()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["glyph"], "printer");
        assert_eq!(view["label"], "2 Printers, 1 Scanner");
        assert_eq!(view["printersAvailable"], true);
        assert_eq!(view["scannersAvailable"], true);
        assert_eq!(view["printerCount"], 2);
        assert_eq!(view["scannerCount"], 1);
        assert_eq!(view["queuedJobCount"], 1);
        assert_eq!(view["defaultPrinter"], "Canon_MF230");
        // The default queue sorts first.
        assert_eq!(view["printers"][0]["name"], "Canon_MF230");
        assert_eq!(view["printers"][0]["displayName"], "Canon MF230");
        assert_eq!(view["printers"][0]["state"], "idle");
        assert_eq!(view["printers"][0]["stateLabel"], "Idle");
        assert_eq!(view["printers"][0]["stateMessage"], "Idle, Last Used");
        assert_eq!(view["printers"][0]["isDefault"], true);
        assert_eq!(view["printers"][1]["name"], "HP_LaserJet");
        assert_eq!(view["printers"][1]["state"], "processing");
        assert_eq!(view["printers"][1]["jobCount"], 1);
        assert_eq!(view["printers"][1]["jobs"][0]["id"], 9);
        assert_eq!(view["printers"][1]["jobs"][0]["user"], "sam");
        assert_eq!(
            view["scanners"][0]["description"],
            "Epson GT-1500 flatbed scanner"
        );
        assert_eq!(view["scanners"][0]["kind"], "flatbed");
        assert_eq!(view["scanners"][0]["kindLabel"], "Flatbed");
    }

    #[test]
    fn a_host_with_only_printers_keeps_the_scanner_half_absent() {
        let mut host = PrintersHost::new(MockPrint::present(PrintData {
            scanners: None,
            ..data()
        }));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["printersAvailable"], true);
        assert_eq!(view["scannersAvailable"], false);
        assert_eq!(view["scannerCount"], 0);
        assert_eq!(view["scanners"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = PrintersHost::new(MockPrint::failing("CUPS: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "CUPS: timeout");
    }

    #[test]
    fn the_queue_writes_apply_once_through_the_adapter() {
        let mut host = PrintersHost::new(MockPrint::present(data()));
        host.refresh();
        assert_eq!(
            host.set_default_printer("HP_LaserJet")["outcome"],
            "applied"
        );
        assert_eq!(
            host.set_printer_accepting_jobs("HP_LaserJet", false)["outcome"],
            "applied"
        );
        assert_eq!(host.cancel_job(9)["outcome"], "applied");
        assert_eq!(host.adapter().source().default_sets(), 1);
        assert_eq!(host.adapter().source().accept_sets(), 1);
        assert_eq!(host.adapter().source().job_cancels(), 1);
    }

    #[test]
    fn a_polkit_denial_is_reported_and_leaves_the_read_state_live() {
        let mut host =
            PrintersHost::new(MockPrint::present(data()).deny_writes("cups: not authorized"));
        host.refresh();
        let report = host.set_default_printer("HP_LaserJet");
        assert_eq!(report["outcome"], "denied");
        assert_eq!(report["note"], "cups: not authorized");
        assert_eq!(host.view()["state"], "available");
    }

    #[test]
    fn writes_while_absent_answer_absence() {
        let mut host = PrintersHost::new(MockPrint::absent());
        host.refresh();
        assert_eq!(host.set_default_printer("Canon_MF230")["outcome"], "absent");
        assert_eq!(
            host.set_printer_accepting_jobs("Canon_MF230", false)["outcome"],
            "absent"
        );
        assert_eq!(host.cancel_job(1)["outcome"], "absent");
        assert_eq!(host.view()["state"], "unavailable");
    }
}
