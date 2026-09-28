// SPDX-License-Identifier: MIT
//! The Printers and Scanners adapter (T-15.12a).
//!
//! Printers and Scanners do not wrap a single desktop daemon the way Bluetooth
//! or UPower do: they project the **host stack**, which has two independent
//! halves, both reused, never reimplemented:
//!
//! * the **printers** half — CUPS, read and written through the client tools
//!   CUPS ships (`lpstat`/`lpadmin`/`cupsaccept`/`cupsreject`/`cancel`); and
//! * the **scanners** half — SANE, enumerated through `scanimage -L`.
//!
//! The tools' human-oriented output churn is pinned in one parser
//! ([`printers_from_lpstat`], [`scanners_from_scanimage`]) and stays behind the
//! [`PrintSource`] seam, exactly as the audio adapter isolates `pw-dump`/`wpctl`
//! and the input adapter isolates `libinput list-devices`.
//!
//! # The read path
//!
//! 1. A [`PrintSource`] returns the raw [`PrintData`] once per
//!    [`PrinterAdapter::refresh`] (or absence/error).
//! 2. [`PrintSnapshot::from_data`] types every queue and scanner, resolves the
//!    default destination, orders the lists, and derives the labels and glyph
//!    the pane draws.
//! 3. The adapter drives the shared subscription lifecycle, so a host stack
//!    that comes and goes re-subscribes and re-syncs with no user-visible
//!    error.
//!
//! # Absence is layered and normal
//!
//! The whole adapter is `Unavailable` only when **neither** CUPS nor SANE is
//! reachable. CUPS that answers but has no queue is `Available` with an empty
//! printer list; a running host with **no SANE** is `Available` with
//! `scanners: None`, so only the scanner half disables. That mirrors the
//! battery/UDisks2 and account adapters' per-half hide rules. A present host
//! stack that cannot be read is `Error`, visible and inert with the message.
//! Nothing blocks session startup.
//!
//! # Explicit writes
//!
//! The pane's `Default printer` choice and its per-queue accept/reject and job
//! cancellation are explicit user actions over the same seam
//! ([`PrinterAdapter::set_default_printer`] and friends). They invent no
//! snapshot: CUPS publishes the resulting state and the host re-reads, so the
//! snapshot stays the single source of truth. Durable presentation preferences
//! are `settingsd`'s and land with T-15.12b.
//!
//! # Testing
//!
//! CI has neither CUPS nor SANE, so the adapter is driven by [`MockPrint`] over
//! the source seam. `kill_cups`/`restart_cups` (and the SANE pair) exercise the
//! layered absence and re-subscribe; `push` drives the queue and device lists.
//! [`HostPrint`] reads the live stack, and the tool output format is pinned by
//! fixtures (see
//! [adr/0140](../../../docs/design/adr/0140-printers-and-scanners-adapter.md)).
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod adapter;
mod model;
mod printers;
mod source;

pub use adapter::PrinterAdapter;
pub use model::{PrintChange, PrintJob, PrintSnapshot, Printer, Scanner};
pub use printers::{
    printers_from_lpstat, scanners_from_scanimage, HostPrint, CANCEL_BIN, CUPSACCEPT_BIN,
    CUPSREJECT_BIN, LPADMIN_BIN, LPSTAT_BIN, SCANIMAGE_BIN,
};
pub use source::{
    MockPrint, PrintData, PrintJobData, PrintOutcome, PrintSource, PrinterData, PrinterState,
    ScanDeviceData, ScannerKind,
};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_printer_slot() {
        let adapter = PrinterAdapter::new(MockPrint::absent());
        assert_eq!(
            <PrinterAdapter<MockPrint> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::PRINTER
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = PrinterAdapter::new(MockPrint::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_vocabulary_ids_are_stable() {
        assert_eq!(PrinterState::Idle.id(), "idle");
        assert_eq!(PrinterState::Processing.id(), "processing");
        assert_eq!(PrinterState::Stopped.id(), "stopped");
        assert_eq!(PrinterState::Unknown.id(), "unknown");
        assert_eq!(ScannerKind::Flatbed.id(), "flatbed");
        assert_eq!(ScannerKind::Sheetfed.id(), "sheetfed");
        assert_eq!(ScannerKind::Unknown.id(), "unknown");
    }
}
