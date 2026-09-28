// SPDX-License-Identifier: MIT
//! The Printers and Scanners snapshot the pane renders, decoded from one raw
//! read.
//!
//! The model owns only the projection a consumer should not repeat: it types
//! each queue and scanner, resolves the default destination, orders the lists
//! (the default queue first, then by name; scanners by name), derives the
//! labels and glyph the pane header draws, and computes the change stream
//! between two reads. It never adds or removes a queue, never cancels a job,
//! and never writes a settings key — CUPS and SANE own the operations and
//! `settingsd` owns the durable preferences; this is the data they publish.
//!
//! The event half is a pure diff: [`PrintSnapshot::changes`] reports what
//! moved between two reads (a queue or scanner added, removed, or edited; the
//! default destination; a daemon appearing; a job entering or leaving a
//! queue) without any polling.

use crate::source::{
    PrintData, PrintJobData, PrinterData, PrinterState, ScanDeviceData, ScannerKind,
};

/// One job waiting in a queue the pane draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrintJob {
    /// The numeric job id.
    pub id: u32,
    /// The login name the job belongs to.
    pub user: String,
    /// The size in bytes, `0` when the daemon did not report one.
    pub size: u64,
}

impl PrintJob {
    fn from_data(data: &PrintJobData) -> Self {
        PrintJob {
            id: data.id,
            user: data.user.clone(),
            size: data.size,
        }
    }
}

/// One printer queue the pane draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Printer {
    /// The queue name.
    pub name: String,
    /// The human description, empty when unset.
    pub display_name: String,
    /// The make and model, empty when unset.
    pub make_and_model: String,
    /// The location, empty when unset.
    pub location: String,
    /// The device URI.
    pub uri: String,
    /// The queue state.
    pub state: PrinterState,
    /// The queue state message (`Idle, Last Used`, …).
    pub state_message: String,
    /// The queue accepts new jobs.
    pub accepting_jobs: bool,
    /// The device is enabled.
    pub enabled: bool,
    /// This queue is the default destination.
    pub is_default: bool,
    /// The jobs currently waiting.
    pub jobs: Vec<PrintJob>,
}

impl Printer {
    fn from_data(data: &PrinterData) -> Self {
        Printer {
            name: data.name.clone(),
            display_name: data.display_name.clone(),
            make_and_model: data.make_and_model.clone(),
            location: data.location.clone(),
            uri: data.uri.clone(),
            state: data.state,
            state_message: data.state_message.clone(),
            accepting_jobs: data.accepting_jobs,
            enabled: data.enabled,
            is_default: data.is_default,
            jobs: data.jobs.iter().map(PrintJob::from_data).collect(),
        }
    }

    /// The name to show: the description, else the queue name.
    pub fn display_name(&self) -> &str {
        if self.display_name.is_empty() {
            &self.name
        } else {
            &self.display_name
        }
    }

    /// The human state label.
    pub fn state_label(&self) -> &'static str {
        self.state.label()
    }

    /// The queue is the default destination.
    pub fn is_default(&self) -> bool {
        self.is_default
    }

    /// Whether the queue accepts new jobs.
    pub fn is_accepting_jobs(&self) -> bool {
        self.accepting_jobs
    }

    /// The number of jobs waiting in the queue.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }
}

/// One scanner device the pane draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scanner {
    /// The SANE device name.
    pub device: String,
    /// The free-form description SANE prints.
    pub description: String,
    /// The physical form derived from the description.
    pub kind: ScannerKind,
}

impl Scanner {
    fn from_data(data: &ScanDeviceData) -> Self {
        Scanner {
            device: data.device.clone(),
            description: data.description.clone(),
            kind: data.kind,
        }
    }

    /// The name to show: the description, else the device name.
    pub fn display_name(&self) -> &str {
        if self.description.is_empty() {
            &self.device
        } else {
            &self.description
        }
    }

    /// The physical form.
    pub fn kind_label(&self) -> &'static str {
        self.kind.label()
    }
}

/// The Printers and Scanners snapshot a pane renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrintSnapshot {
    /// The CUPS queues; `None` when CUPS is absent, the default queue first.
    pub printers: Option<Vec<Printer>>,
    /// The default destination name, when CUPS named one that is present.
    pub default_printer: Option<String>,
    /// The SANE devices; `None` when SANE is absent.
    pub scanners: Option<Vec<Scanner>>,
}

impl PrintSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &PrintData) -> Self {
        let printers = data.printers.as_ref().map(|printers| {
            let mut printers: Vec<Printer> = printers.iter().map(Printer::from_data).collect();
            printers.sort_by(|left, right| {
                right
                    .is_default()
                    .cmp(&left.is_default())
                    .then_with(|| left.name.cmp(&right.name))
            });
            printers
        });
        let default_printer = printers.as_ref().and_then(|printers| {
            printers
                .iter()
                .find(|printer| printer.is_default())
                .map(|printer| printer.name.clone())
        });

        let scanners = data.scanners.as_ref().map(|scanners| {
            let mut scanners: Vec<Scanner> = scanners.iter().map(Scanner::from_data).collect();
            scanners.sort_by(|left, right| {
                left.kind
                    .id()
                    .cmp(right.kind.id())
                    .then_with(|| left.device.cmp(&right.device))
            });
            scanners
        });

        PrintSnapshot {
            printers,
            default_printer,
            scanners,
        }
    }

    /// The queue list; empty when CUPS is absent.
    pub fn printers(&self) -> &[Printer] {
        self.printers.as_deref().unwrap_or(&[])
    }

    /// Whether CUPS is present.
    pub const fn printers_available(&self) -> bool {
        self.printers.is_some()
    }

    /// The scanner list; empty when SANE is absent.
    pub fn scanners(&self) -> &[Scanner] {
        self.scanners.as_deref().unwrap_or(&[])
    }

    /// Whether SANE is present.
    pub const fn scanners_available(&self) -> bool {
        self.scanners.is_some()
    }

    /// The queue with `name`, when the snapshot has one.
    pub fn printer_by_name(&self, name: &str) -> Option<&Printer> {
        self.printers().iter().find(|printer| printer.name == name)
    }

    /// The default queue, when CUPS named one that is present.
    pub fn default_printer(&self) -> Option<&Printer> {
        self.default_printer
            .as_deref()
            .and_then(|name| self.printer_by_name(name))
    }

    /// The scanner with `device`, when the snapshot has one.
    pub fn scanner_by_device(&self, device: &str) -> Option<&Scanner> {
        self.scanners()
            .iter()
            .find(|scanner| scanner.device == device)
    }

    /// Whether the read found any queue or scanner at all. A running CUPS with
    /// no queue and a running SANE with no device both answer `Available`
    /// while this is false; it is the second hide rule beside the
    /// adapter-level `Unavailable`.
    pub fn present(&self) -> bool {
        !self.printers().is_empty() || !self.scanners().is_empty()
    }

    /// How many queues the snapshot has.
    pub fn printer_count(&self) -> usize {
        self.printers().len()
    }

    /// How many scanners the snapshot has.
    pub fn scanner_count(&self) -> usize {
        self.scanners().len()
    }

    /// How many jobs are waiting across every queue.
    pub fn queued_job_count(&self) -> usize {
        self.printers().iter().map(Printer::job_count).sum()
    }

    /// The design-system glyph for the pane header and tile.
    pub fn glyph(&self) -> &'static str {
        "printer"
    }

    /// A one-line label for the pane header / tile.
    pub fn label(&self) -> String {
        let printers = self.printer_count();
        let scanners = self.scanner_count();
        if printers == 0 && scanners == 0 {
            return "No Printers or Scanners".to_owned();
        }
        let mut parts = Vec::new();
        if printers > 0 {
            parts.push(plural(printers, "Printer", "Printers"));
        }
        if scanners > 0 {
            parts.push(plural(scanners, "Scanner", "Scanners"));
        }
        parts.join(", ")
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// the daemons' presence, the queues, the default destination, the
    /// scanners, then each queue's jobs.
    ///
    /// This is the "event" half of the adapter: the shell bridge diffs two
    /// reads to learn what moved without polling each attribute.
    pub fn changes(&self, previous: &PrintSnapshot) -> Vec<PrintChange> {
        let mut changes = Vec::new();

        match (&previous.printers, &self.printers) {
            (None, Some(_)) => {
                changes.push(PrintChange::PrintersAvailabilityChanged { available: true })
            }
            (Some(_), None) => {
                changes.push(PrintChange::PrintersAvailabilityChanged { available: false })
            }
            _ => {}
        }
        if let (Some(previous_printers), Some(printers)) = (&previous.printers, &self.printers) {
            for printer in printers {
                match previous_printers
                    .iter()
                    .find(|candidate| candidate.name == printer.name)
                {
                    None => changes.push(PrintChange::PrinterAdded {
                        name: printer.name.clone(),
                    }),
                    Some(previous_printer) if previous_printer != printer => {
                        changes.push(PrintChange::PrinterChanged {
                            name: printer.name.clone(),
                        })
                    }
                    Some(_) => {}
                }
            }
            for printer in previous_printers {
                if !printers
                    .iter()
                    .any(|candidate| candidate.name == printer.name)
                {
                    changes.push(PrintChange::PrinterRemoved {
                        name: printer.name.clone(),
                    });
                }
            }
        }

        if previous.default_printer != self.default_printer {
            changes.push(PrintChange::DefaultPrinterChanged {
                from: previous.default_printer.clone(),
                to: self.default_printer.clone(),
            });
        }

        match (&previous.scanners, &self.scanners) {
            (None, Some(_)) => {
                changes.push(PrintChange::ScannersAvailabilityChanged { available: true })
            }
            (Some(_), None) => {
                changes.push(PrintChange::ScannersAvailabilityChanged { available: false })
            }
            _ => {}
        }
        if let (Some(previous_scanners), Some(scanners)) = (&previous.scanners, &self.scanners) {
            for scanner in scanners {
                match previous_scanners
                    .iter()
                    .find(|candidate| candidate.device == scanner.device)
                {
                    None => changes.push(PrintChange::ScannerAdded {
                        device: scanner.device.clone(),
                    }),
                    Some(previous_scanner) if previous_scanner != scanner => {
                        changes.push(PrintChange::ScannerChanged {
                            device: scanner.device.clone(),
                        })
                    }
                    Some(_) => {}
                }
            }
            for scanner in previous_scanners {
                if !scanners
                    .iter()
                    .any(|candidate| candidate.device == scanner.device)
                {
                    changes.push(PrintChange::ScannerRemoved {
                        device: scanner.device.clone(),
                    });
                }
            }
        }

        if let (Some(previous_printers), Some(printers)) = (&previous.printers, &self.printers) {
            for printer in printers {
                let Some(previous_printer) = previous_printers
                    .iter()
                    .find(|candidate| candidate.name == printer.name)
                else {
                    continue;
                };
                for job in &printer.jobs {
                    match previous_printer.jobs.iter().find(|j| j.id == job.id) {
                        None => changes.push(PrintChange::JobAdded {
                            printer: printer.name.clone(),
                            id: job.id,
                        }),
                        Some(previous_job) if previous_job != job => {
                            changes.push(PrintChange::JobChanged {
                                printer: printer.name.clone(),
                                id: job.id,
                            })
                        }
                        Some(_) => {}
                    }
                }
                for job in &previous_printer.jobs {
                    if !printer.jobs.iter().any(|candidate| candidate.id == job.id) {
                        changes.push(PrintChange::JobRemoved {
                            printer: printer.name.clone(),
                            id: job.id,
                        });
                    }
                }
            }
        }

        changes
    }
}

/// `1 Printer` / `2 Printers`.
fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("{count} {one}")
    } else {
        format!("{count} {many}")
    }
}

/// A change between two Printers and Scanners snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintChange {
    /// The CUPS scheduler appeared or went away.
    PrintersAvailabilityChanged {
        /// `true` when CUPS appeared, `false` when it vanished.
        available: bool,
    },
    /// A queue was added.
    PrinterAdded {
        /// The new queue's name.
        name: String,
    },
    /// A queue was deleted.
    PrinterRemoved {
        /// The removed queue's name.
        name: String,
    },
    /// A queue's state, options, or job list moved.
    PrinterChanged {
        /// The edited queue's name.
        name: String,
    },
    /// The default destination changed.
    DefaultPrinterChanged {
        /// The previous default, if any.
        from: Option<String>,
        /// The new default, if any.
        to: Option<String>,
    },
    /// The SANE stack appeared or went away.
    ScannersAvailabilityChanged {
        /// `true` when SANE appeared, `false` when it vanished.
        available: bool,
    },
    /// A scanner was added.
    ScannerAdded {
        /// The new device name.
        device: String,
    },
    /// A scanner was removed.
    ScannerRemoved {
        /// The removed device name.
        device: String,
    },
    /// A scanner's description or form moved.
    ScannerChanged {
        /// The edited device name.
        device: String,
    },
    /// A job entered a queue.
    JobAdded {
        /// The owning queue's name.
        printer: String,
        /// The job id.
        id: u32,
    },
    /// A job left a queue.
    JobRemoved {
        /// The owning queue's name.
        printer: String,
        id: u32,
    },
    /// A job's fields moved.
    JobChanged {
        /// The owning queue's name.
        printer: String,
        id: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_snapshot_orders_and_types_the_read() {
        let snapshot = PrintSnapshot::from_data(&data());
        assert!(snapshot.present());
        assert_eq!(snapshot.printer_count(), 2);
        assert_eq!(snapshot.scanner_count(), 1);
        assert_eq!(snapshot.queued_job_count(), 1);
        // The default queue sorts first.
        assert_eq!(snapshot.printers()[0].name, "Canon_MF230");
        assert!(snapshot.printers()[0].is_default());
        assert_eq!(snapshot.printers()[0].display_name(), "Canon MF230");
        assert_eq!(snapshot.printers()[0].state_label(), "Idle");
        assert_eq!(snapshot.printers()[1].name, "HP_LaserJet");
        assert_eq!(snapshot.printers()[1].job_count(), 1);
        assert_eq!(
            snapshot
                .default_printer()
                .map(|printer| printer.name.as_str()),
            Some("Canon_MF230")
        );
        assert_eq!(snapshot.glyph(), "printer");
        assert_eq!(snapshot.label(), "2 Printers, 1 Scanner");
    }

    #[test]
    fn scanners_are_typed_and_addressable() {
        let snapshot = PrintSnapshot::from_data(&data());
        assert!(snapshot.scanners_available());
        let scanner = snapshot
            .scanner_by_device("epson2:net:192.168.0.7")
            .unwrap();
        assert_eq!(scanner.display_name(), "Epson GT-1500 flatbed scanner");
        assert_eq!(scanner.kind_label(), "Flatbed");
        assert!(snapshot.scanner_by_device("missing").is_none());
    }

    #[test]
    fn a_missing_cups_half_is_a_normal_snapshot() {
        let snapshot = PrintSnapshot::from_data(&PrintData {
            printers: None,
            default_printer: String::new(),
            scanners: data().scanners,
        });
        assert!(!snapshot.printers_available());
        assert!(snapshot.printers().is_empty());
        assert_eq!(snapshot.default_printer(), None);
        // The scanner half stays live.
        assert_eq!(snapshot.scanner_count(), 1);
        assert_eq!(snapshot.label(), "1 Scanner");
    }

    #[test]
    fn an_empty_read_is_not_present_but_is_a_snapshot() {
        let snapshot = PrintSnapshot::from_data(&PrintData {
            printers: Some(vec![]),
            default_printer: String::new(),
            scanners: Some(vec![]),
        });
        assert!(!snapshot.present());
        assert!(snapshot.printers_available());
        assert!(snapshot.scanners_available());
        assert_eq!(snapshot.label(), "No Printers or Scanners");
        assert_eq!(
            PrintSnapshot::default(),
            PrintSnapshot::from_data(&PrintData::default())
        );
    }

    #[test]
    fn a_one_printer_label_is_singular() {
        let snapshot = PrintSnapshot::from_data(&PrintData {
            printers: Some(vec![PrinterData {
                name: "Canon_MF230".to_owned(),
                ..PrinterData::default()
            }]),
            default_printer: String::new(),
            scanners: None,
        });
        assert_eq!(snapshot.label(), "1 Printer");
    }

    #[test]
    fn each_queue_and_scanner_move_is_a_change() {
        let previous = PrintSnapshot::from_data(&data());

        let mut raw = data();
        raw.printers
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|printer| printer.name == "HP_LaserJet")
            .unwrap()
            .accepting_jobs = false; // HP edited
        raw.printers.as_mut().unwrap().push(PrinterData {
            name: "Brother_QL".to_owned(),
            ..PrinterData::default()
        });
        raw.scanners.as_mut().unwrap()[0].description = "Epson flatbed".to_owned();
        raw.scanners.as_mut().unwrap().push(ScanDeviceData {
            device: "pixma:04A9173E".to_owned(),
            description: "Canon PIXMA flatbed scanner".to_owned(),
            kind: ScannerKind::Flatbed,
        });
        let next = PrintSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&PrintChange::PrinterChanged {
            name: "HP_LaserJet".to_owned()
        }));
        assert!(changes.contains(&PrintChange::PrinterAdded {
            name: "Brother_QL".to_owned()
        }));
        assert!(changes.contains(&PrintChange::ScannerChanged {
            device: "epson2:net:192.168.0.7".to_owned()
        }));
        assert!(changes.contains(&PrintChange::ScannerAdded {
            device: "pixma:04A9173E".to_owned()
        }));
    }

    #[test]
    fn a_job_entering_and_leaving_a_queue_is_a_change() {
        let previous = PrintSnapshot::from_data(&data());

        let mut raw = data();
        raw.printers
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|printer| printer.name == "HP_LaserJet")
            .unwrap()
            .jobs
            .clear();
        raw.printers
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|printer| printer.name == "HP_LaserJet")
            .unwrap()
            .jobs
            .push(PrintJobData {
                id: 4,
                user: "dan".to_owned(),
                size: 8,
            });
        let next = PrintSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&PrintChange::JobRemoved {
            printer: "HP_LaserJet".to_owned(),
            id: 9
        }));
        assert!(changes.contains(&PrintChange::JobAdded {
            printer: "HP_LaserJet".to_owned(),
            id: 4
        }));
    }

    #[test]
    fn default_and_daemon_presence_moves_are_changes() {
        let with = PrintSnapshot::from_data(&data());
        let without_cups = PrintSnapshot::from_data(&PrintData {
            printers: None,
            default_printer: String::new(),
            scanners: data().scanners,
        });
        let without_sane = PrintSnapshot::from_data(&PrintData {
            printers: {
                let mut printers = data().printers.unwrap();
                for printer in &mut printers {
                    printer.is_default = printer.name == "HP_LaserJet";
                }
                Some(printers)
            },
            default_printer: "HP_LaserJet".to_owned(),
            scanners: None,
        });

        assert!(without_cups
            .changes(&with)
            .contains(&PrintChange::PrintersAvailabilityChanged { available: false }));
        assert!(with
            .changes(&without_cups)
            .contains(&PrintChange::PrintersAvailabilityChanged { available: true }));
        assert!(without_sane
            .changes(&with)
            .contains(&PrintChange::ScannersAvailabilityChanged { available: false }));
        assert!(without_sane
            .changes(&with)
            .contains(&PrintChange::DefaultPrinterChanged {
                from: Some("Canon_MF230".to_owned()),
                to: Some("HP_LaserJet".to_owned()),
            }));
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = PrintSnapshot::from_data(&data());
        assert!(snapshot.changes(&snapshot).is_empty());
    }
}
