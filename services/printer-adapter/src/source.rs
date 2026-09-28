// SPDX-License-Identifier: MIT
//! The transport seam: the raw CUPS/SANE read, the explicit writes, and their
//! mock.
//!
//! A [`PrintSource`] is the only thing that talks to the host stack. That stack
//! has two independent halves, both reused, never reimplemented:
//!
//! * the **printers** half — CUPS, read and written through the client tools
//!   CUPS ships ([`crate::HostPrint`]: `lpstat`, `lpadmin`, `cupsaccept`/
//!   `cupsreject`, `cancel`); and
//! * the **scanners** half — SANE, enumerated through `scanimage -L`
//!   ([`crate::HostPrint`]).
//!
//! The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — at least one half answered; `data` is the live read.
//! * `Ok(None)` — **both** halves are absent. A normal state; the item hides.
//! * `Err(error)` — the host stack is present but could not be read; the item
//!   shows visible and inert with the message.
//!
//! Absence is layered, as the battery and account adapters' is. CUPS answering
//! with no queue is a live, empty printer list (`PrintData::printers` is
//! `Some(vec![])`); SANE being absent is carried inside the snapshot
//! (`PrintData::scanners` is `None`) and only disables the scanner half. The
//! whole adapter is `Unavailable` only when neither half is reachable — a
//! normal hidden state, never an error.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// The state of a CUPS printer queue.
///
/// CUPS's `printer-state` (IPP enum `3`/`4`/`5`), narrowed to the states a
/// pane draws. An unrecognized code is [`PrinterState::Unknown`], never a
/// guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrinterState {
    /// The queue is idle (CUPS code `3`).
    Idle,
    /// The queue is processing a job (CUPS code `4`).
    Processing,
    /// The queue is stopped (CUPS code `5`).
    Stopped,
    /// The daemon did not report a state we recognize.
    #[default]
    Unknown,
}

impl PrinterState {
    /// Every printer state.
    pub const ALL: [PrinterState; 4] = [
        PrinterState::Idle,
        PrinterState::Processing,
        PrinterState::Stopped,
        PrinterState::Unknown,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            PrinterState::Idle => "idle",
            PrinterState::Processing => "processing",
            PrinterState::Stopped => "stopped",
            PrinterState::Unknown => "unknown",
        }
    }

    /// The short label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            PrinterState::Idle => "Idle",
            PrinterState::Processing => "Printing",
            PrinterState::Stopped => "Stopped",
            PrinterState::Unknown => "Unknown",
        }
    }

    /// The CUPS/Ipp integer code.
    pub const fn code(self) -> i32 {
        match self {
            PrinterState::Idle => 3,
            PrinterState::Processing => 4,
            PrinterState::Stopped => 5,
            PrinterState::Unknown => 0,
        }
    }

    /// The state for a CUPS/Ipp code; an unknown code is `Unknown`.
    pub const fn from_code(code: i32) -> PrinterState {
        match code {
            3 => PrinterState::Idle,
            4 => PrinterState::Processing,
            5 => PrinterState::Stopped,
            _ => PrinterState::Unknown,
        }
    }

    /// Parse the stable id back to a state; an unknown id is `Unknown`.
    pub fn from_id(id: &str) -> PrinterState {
        PrinterState::ALL
            .into_iter()
            .find(|state| state.id() == id)
            .unwrap_or(PrinterState::Unknown)
    }

    /// The state for the human spelling `lpstat` prints; an unrecognized
    /// spelling is `Unknown`.
    pub fn from_name(name: &str) -> PrinterState {
        let lower = name.to_ascii_lowercase();
        if lower.contains("idle") {
            PrinterState::Idle
        } else if lower.contains("printing") || lower.contains("processing") {
            PrinterState::Processing
        } else if lower.contains("stopped")
            || lower.contains("disabled")
            || lower.contains("holding")
        {
            PrinterState::Stopped
        } else {
            PrinterState::Unknown
        }
    }
}

/// One job waiting in a CUPS queue, as `lpstat` reports it.
///
/// `lpstat` lists the destination, the numeric id, the owning user, and the
/// job size; it does not print a document title, so none is invented.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct PrintJobData {
    /// The numeric job id (`12` in `Canon_MF230-12`).
    #[serde(default)]
    pub id: u32,
    /// The login name the job belongs to.
    #[serde(default)]
    pub user: String,
    /// The size in bytes, `0` when the daemon did not report one.
    #[serde(default)]
    pub size: u64,
}

/// One CUPS printer queue.
///
/// Every field is a plain read of a CUPS attribute; a missing attribute is
/// empty/`false`/`Unknown`, never an invented value. The live reads live in
/// [`crate::HostPrint`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct PrinterData {
    /// The queue name (`printer-name`).
    #[serde(default)]
    pub name: String,
    /// The human description (`printer-info`), empty when unset.
    #[serde(default)]
    pub display_name: String,
    /// `printer-make-and-model`, empty when the daemon did not report one.
    #[serde(default)]
    pub make_and_model: String,
    /// `printer-location`, empty when unset.
    #[serde(default)]
    pub location: String,
    /// The device URI (`printer-uri-supported`/`device-uri`).
    #[serde(default)]
    pub uri: String,
    /// `printer-state`.
    #[serde(default)]
    pub state: PrinterState,
    /// `printer-state-message` (`Idle, Last Used`, `toner low`, …).
    #[serde(default)]
    pub state_message: String,
    /// `printer-is-accepting-jobs`: the queue accepts new jobs.
    #[serde(default)]
    pub accepting_jobs: bool,
    /// `printer-state-reasons`/`printer-state`: the device is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// This queue is the default destination.
    #[serde(default)]
    pub is_default: bool,
    /// The jobs currently waiting in the queue.
    #[serde(default)]
    pub jobs: Vec<PrintJobData>,
}

/// The physical form of a scanner, when SANE's description names one.
///
/// This is a best-effort classification of a free-form description string: an
/// unrecognized description is [`ScannerKind::Unknown`], never a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScannerKind {
    /// A flatbed scanner.
    Flatbed,
    /// A sheet-fed or ADF scanner.
    Sheetfed,
    /// A handheld scanner.
    Handheld,
    /// SANE did not describe a form we recognize.
    #[default]
    Unknown,
}

impl ScannerKind {
    /// Every scanner kind.
    pub const ALL: [ScannerKind; 4] = [
        ScannerKind::Flatbed,
        ScannerKind::Sheetfed,
        ScannerKind::Handheld,
        ScannerKind::Unknown,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            ScannerKind::Flatbed => "flatbed",
            ScannerKind::Sheetfed => "sheetfed",
            ScannerKind::Handheld => "handheld",
            ScannerKind::Unknown => "unknown",
        }
    }

    /// The short label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            ScannerKind::Flatbed => "Flatbed",
            ScannerKind::Sheetfed => "Sheet-fed",
            ScannerKind::Handheld => "Handheld",
            ScannerKind::Unknown => "Scanner",
        }
    }

    /// Classify SANE's free-form device description; an unrecognized
    /// description is `Unknown`.
    pub fn from_description(description: &str) -> ScannerKind {
        let lower = description.to_ascii_lowercase();
        if lower.contains("flatbed") {
            ScannerKind::Flatbed
        } else if lower.contains("sheetfed") || lower.contains("sheet-fed") || lower.contains("adf")
        {
            ScannerKind::Sheetfed
        } else if lower.contains("handheld") {
            ScannerKind::Handheld
        } else {
            ScannerKind::Unknown
        }
    }

    /// Parse the stable id back to a kind; an unknown id is `Unknown`.
    pub fn from_id(id: &str) -> ScannerKind {
        ScannerKind::ALL
            .into_iter()
            .find(|kind| kind.id() == id)
            .unwrap_or(ScannerKind::Unknown)
    }
}

/// One SANE scanner device, as `scanimage -L` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct ScanDeviceData {
    /// The SANE device name (`epson2:net:192.168.0.7`).
    #[serde(default)]
    pub device: String,
    /// The free-form description SANE prints (`Epson GT-1500 flatbed
    /// scanner`), kept verbatim.
    #[serde(default)]
    pub description: String,
    /// The physical form derived from the description.
    #[serde(default)]
    pub kind: ScannerKind,
}

/// The raw result of one host-stack read: the CUPS queue list, the default
/// destination, and the SANE device list when SANE is present.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct PrintData {
    /// Every CUPS queue; `None` when CUPS is absent (only the print controls
    /// disable). `Some(vec![])` is a running CUPS with no queue.
    #[serde(default)]
    pub printers: Option<Vec<PrinterData>>,
    /// The default destination name, empty when CUPS named none.
    #[serde(default)]
    pub default_printer: String,
    /// The SANE device list; `None` when SANE is absent (only the scanner
    /// half disables).
    #[serde(default)]
    pub scanners: Option<Vec<ScanDeviceData>>,
}

impl PrintData {
    /// Whether CUPS is present.
    pub const fn cups_available(&self) -> bool {
        self.printers.is_some()
    }

    /// Whether SANE is present.
    pub const fn sane_available(&self) -> bool {
        self.scanners.is_some()
    }
}

/// The result of one print write.
///
/// These are explicit user actions, never a poll. A write that lands invents
/// no snapshot: CUPS publishes the resulting state and the host re-reads, so
/// the snapshot stays the single source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintOutcome {
    /// The daemon accepted the request.
    Applied,
    /// polkit (or CUPS policy) refused the request. `note` records the
    /// daemon's message.
    Denied(String),
    /// There is no CUPS scheduler, or no such queue or job.
    Absent,
    /// The request failed for a reason other than authorization.
    Failed(AdapterError),
}

impl PrintOutcome {
    /// Whether the daemon accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, PrintOutcome::Applied)
    }

    /// The recorded denial note, when the daemon refused.
    pub fn denial_note(&self) -> Option<&str> {
        match self {
            PrintOutcome::Denied(note) => Some(note),
            _ => None,
        }
    }

    /// The failure, when the request failed for a reason other than a denial.
    pub fn error(&self) -> Option<&AdapterError> {
        match self {
            PrintOutcome::Failed(error) => Some(error),
            _ => None,
        }
    }
}

/// Reads the host stack over some transport and drives its write methods.
pub trait PrintSource {
    /// One read of the host stack.
    fn read(&mut self) -> Result<Option<PrintData>, AdapterError>;

    /// Make `name` the default destination. One explicit write.
    fn set_default_printer(&mut self, _name: &str) -> PrintOutcome {
        PrintOutcome::Absent
    }

    /// Accept or reject new jobs on `name`. One explicit write.
    fn set_printer_accepting_jobs(&mut self, _name: &str, _accepting: bool) -> PrintOutcome {
        PrintOutcome::Absent
    }

    /// Cancel the queued job with `job_id`. One explicit write.
    fn cancel_job(&mut self, _job_id: u32) -> PrintOutcome {
        PrintOutcome::Absent
    }
}

/// How the simulated daemon answers write requests.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteBehavior {
    /// Authorized: the request is accepted (the default).
    Accept,
    /// CUPS policy/polkit denies the request; carries the recorded note.
    Deny(String),
    /// The request fails for another reason.
    Fail(AdapterError),
}

/// The write counters the mock keeps, one per method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct WriteCounts {
    default_sets: u32,
    accept_sets: u32,
    job_cancels: u32,
}

impl WriteCounts {
    fn total(&self) -> u32 {
        self.default_sets + self.accept_sets + self.job_cancels
    }
}

/// A fixture-backed source with a simulated host-stack lifecycle.
///
/// The mock is the CI path: it serves [`PrintData`] with neither CUPS nor SANE
/// on the machine, and `kill_cups`/`restart_cups` (and the SANE pair) exercise
/// the layered absence and re-subscribe the way masking a daemon would. `push`
/// drives the queue and device lists so a test observes the change stream. Its
/// writes mutate the simulated CUPS queue the way the daemon would, so a
/// subsequent `read` sees the change.
#[derive(Debug, Clone, PartialEq)]
pub struct MockPrint {
    cups_present: bool,
    sane_present: bool,
    printers: Vec<PrinterData>,
    default_printer: String,
    scanners: Vec<ScanDeviceData>,
    failure: Option<AdapterError>,
    behavior: WriteBehavior,
    reads: u32,
    writes: WriteCounts,
}

impl MockPrint {
    /// Neither CUPS nor SANE is reachable.
    pub fn absent() -> Self {
        MockPrint {
            cups_present: false,
            sane_present: false,
            printers: vec![],
            default_printer: String::new(),
            scanners: vec![],
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// A host stack that answers with `data`. Each half's presence follows the
    /// matching `Some`/`None` in `data`, so a snapshot with only printers or
    /// only scanners is a one-daemon machine.
    pub fn present(data: PrintData) -> Self {
        MockPrint {
            cups_present: data.printers.is_some(),
            sane_present: data.scanners.is_some(),
            printers: data.printers.unwrap_or_default(),
            default_printer: data.default_printer,
            scanners: data.scanners.unwrap_or_default(),
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// A host stack that fails every read (e.g. the scheduler went
    /// unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockPrint {
            cups_present: true,
            sane_present: true,
            printers: vec![],
            default_printer: String::new(),
            scanners: vec![],
            failure: Some(AdapterError::new(message)),
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// Make every write fail with `message` (not authorization).
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.behavior = WriteBehavior::Fail(AdapterError::new(message));
        self
    }

    /// Make every write come back as a policy denial with `note`.
    pub fn deny_writes(mut self, note: impl Into<String>) -> Self {
        self.behavior = WriteBehavior::Deny(note.into());
        self
    }

    /// The host stack publishes fresh state.
    pub fn push(&mut self, data: PrintData) {
        self.cups_present = data.printers.is_some();
        self.sane_present = data.scanners.is_some();
        self.printers = data.printers.unwrap_or_default();
        self.default_printer = data.default_printer;
        self.scanners = data.scanners.unwrap_or_default();
        self.failure = None;
    }

    /// CUPS goes away.
    pub fn kill_cups(&mut self) {
        self.cups_present = false;
    }

    /// CUPS comes back (serving the last queues, if any).
    pub fn restart_cups(&mut self) {
        self.cups_present = true;
        self.failure = None;
    }

    /// SANE goes away.
    pub fn kill_sane(&mut self) {
        self.sane_present = false;
    }

    /// SANE comes back (serving the last devices, if any).
    pub fn restart_sane(&mut self) {
        self.sane_present = true;
        self.failure = None;
    }

    /// Whether the simulated CUPS scheduler is reachable.
    pub fn cups_present(&self) -> bool {
        self.cups_present
    }

    /// Whether the simulated SANE stack is reachable.
    pub fn sane_present(&self) -> bool {
        self.sane_present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many default-destination writes the source has accepted.
    pub fn default_sets(&self) -> u32 {
        self.writes.default_sets
    }

    /// How many accept/reject writes the source has accepted.
    pub fn accept_sets(&self) -> u32 {
        self.writes.accept_sets
    }

    /// How many jobs the source has canceled.
    pub fn job_cancels(&self) -> u32 {
        self.writes.job_cancels
    }

    /// How many write requests the source has served in total. Lets a test
    /// prove a write is one explicit call, never a loop.
    pub fn writes(&self) -> u32 {
        self.writes.total()
    }

    fn read_outcome(&self) -> Result<(), AdapterError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(())
    }

    fn write_outcome(&mut self) -> PrintOutcome {
        if !self.cups_present {
            return PrintOutcome::Absent;
        }
        if let Some(error) = &self.failure {
            return PrintOutcome::Failed(error.clone());
        }
        match &self.behavior {
            WriteBehavior::Accept => PrintOutcome::Applied,
            WriteBehavior::Deny(note) => PrintOutcome::Denied(note.clone()),
            WriteBehavior::Fail(error) => PrintOutcome::Failed(error.clone()),
        }
    }

    fn printer_mut(&mut self, name: &str) -> Option<&mut PrinterData> {
        self.printers
            .iter_mut()
            .find(|printer| printer.name == name)
    }
}

impl PrintSource for MockPrint {
    fn read(&mut self) -> Result<Option<PrintData>, AdapterError> {
        self.reads += 1;
        self.read_outcome()?;
        if !self.cups_present && !self.sane_present {
            return Ok(None);
        }
        Ok(Some(PrintData {
            printers: self.cups_present.then(|| self.printers.clone()),
            default_printer: if self.cups_present {
                self.default_printer.clone()
            } else {
                String::new()
            },
            scanners: self.sane_present.then(|| self.scanners.clone()),
        }))
    }

    fn set_default_printer(&mut self, name: &str) -> PrintOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.default_sets += 1;
            self.default_printer = name.to_owned();
            for printer in &mut self.printers {
                printer.is_default = printer.name == name;
            }
        }
        outcome
    }

    fn set_printer_accepting_jobs(&mut self, name: &str, accepting: bool) -> PrintOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.accept_sets += 1;
            if let Some(printer) = self.printer_mut(name) {
                printer.accepting_jobs = accepting;
            }
        }
        outcome
    }

    fn cancel_job(&mut self, job_id: u32) -> PrintOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.job_cancels += 1;
            for printer in &mut self.printers {
                printer.jobs.retain(|job| job.id != job_id);
            }
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> PrintData {
        PrintData {
            printers: Some(vec![PrinterData {
                name: "Canon_MF230".to_owned(),
                display_name: "Canon MF230".to_owned(),
                state: PrinterState::Idle,
                accepting_jobs: true,
                enabled: true,
                is_default: true,
                jobs: vec![PrintJobData {
                    id: 3,
                    user: "dan".to_owned(),
                    size: 1024,
                }],
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
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockPrint::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.cups_present() && !mock.sane_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockPrint::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.cups_present() && mock.sane_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockPrint::failing("CUPS: timeout");
        assert_eq!(mock.read().unwrap_err().message(), "CUPS: timeout");
        assert!(mock.cups_present());
    }

    #[test]
    fn the_halves_can_be_killed_independently() {
        let mut mock = MockPrint::present(data());
        mock.kill_sane();
        let read = mock.read().unwrap().unwrap();
        assert!(read.cups_available() && !read.sane_available());
        assert_eq!(read.printers.as_ref().unwrap().len(), 1);
        assert!(read.scanners.is_none());

        mock.kill_cups();
        assert_eq!(mock.read(), Ok(None));

        mock.restart_cups();
        let read = mock.read().unwrap().unwrap();
        assert!(read.cups_available() && !read.sane_available());
    }

    #[test]
    fn pushes_replace_the_served_state_and_presence() {
        let mut mock = MockPrint::present(data());
        mock.push(PrintData {
            printers: Some(vec![]),
            default_printer: String::new(),
            scanners: None,
        });
        let read = mock.read().unwrap().unwrap();
        assert!(read.cups_available());
        assert!(read.printers.as_ref().unwrap().is_empty());
        assert!(!read.sane_available());
    }

    #[test]
    fn a_denying_mock_reports_the_note() {
        let mut mock = MockPrint::present(data()).deny_writes("cups: not authorized");
        let outcome = mock.cancel_job(3);
        assert_eq!(
            outcome,
            PrintOutcome::Denied("cups: not authorized".to_owned())
        );
        assert_eq!(outcome.denial_note(), Some("cups: not authorized"));
    }

    #[test]
    fn a_failing_write_is_a_failure_not_a_denial() {
        let mut mock = MockPrint::present(data()).fail_writes("cups: busy");
        assert_eq!(
            mock.set_default_printer("Canon_MF230"),
            PrintOutcome::Failed(AdapterError::new("cups: busy"))
        );
    }

    #[test]
    fn writes_while_cups_is_absent_are_absent() {
        let mut mock = MockPrint::absent();
        assert_eq!(mock.set_default_printer("x"), PrintOutcome::Absent);
        assert_eq!(
            mock.set_printer_accepting_jobs("x", false),
            PrintOutcome::Absent
        );
        assert_eq!(mock.cancel_job(1), PrintOutcome::Absent);
        assert_eq!(mock.writes(), 0);
    }

    #[test]
    fn an_applied_write_mutates_the_simulated_queue() {
        let mut mock = MockPrint::present(data());
        assert!(mock
            .set_printer_accepting_jobs("Canon_MF230", false)
            .is_applied());
        assert!(mock.set_default_printer("Other").is_applied());
        assert!(mock.cancel_job(3).is_applied());

        let read = mock.read().unwrap().unwrap();
        let printer = &read.printers.as_ref().unwrap()[0];
        assert!(!printer.accepting_jobs);
        assert!(!printer.is_default);
        assert!(printer.jobs.is_empty());
        assert_eq!(read.default_printer, "Other");
        assert_eq!(mock.writes(), 3);
    }

    #[test]
    fn printer_state_codes_and_ids_round_trip() {
        for state in PrinterState::ALL {
            assert_eq!(PrinterState::from_id(state.id()), state);
            assert_eq!(PrinterState::from_code(state.code()), state);
        }
        assert_eq!(PrinterState::from_id("bogus"), PrinterState::Unknown);
        assert_eq!(PrinterState::from_code(99), PrinterState::Unknown);
        assert_eq!(PrinterState::from_name("is idle."), PrinterState::Idle);
        assert_eq!(
            PrinterState::from_name("now printing"),
            PrinterState::Processing
        );
        assert_eq!(PrinterState::from_name("stopped"), PrinterState::Stopped);
        assert_eq!(PrinterState::Processing.label(), "Printing");
    }

    #[test]
    fn scanner_kinds_classify_and_round_trip() {
        for kind in ScannerKind::ALL {
            assert_eq!(ScannerKind::from_id(kind.id()), kind);
        }
        assert_eq!(
            ScannerKind::from_description("Epson flatbed scanner"),
            ScannerKind::Flatbed
        );
        assert_eq!(
            ScannerKind::from_description("Fujitsu ScanSnap S1500 sheetfed scanner"),
            ScannerKind::Sheetfed
        );
        assert_eq!(ScannerKind::from_id("bogus"), ScannerKind::Unknown);
        assert_eq!(ScannerKind::Unknown.label(), "Scanner");
    }
}
