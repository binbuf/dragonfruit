// SPDX-License-Identifier: MIT
//! The transport seam: the raw host-stack read and its mock.
//!
//! A [`SystemSource`] is the only thing that talks to the host stack. That
//! stack has two halves:
//!
//! * the **host identity** — the distribution release, kernel, architecture,
//!   DMI model/serial, processor, memory, and computer name the About and
//!   General rows render, read straight from the host's own files
//!   ([`crate::HostSystem`]); and
//! * the **distribution update provider** — the check/install/reboot state
//!   behind a [`crate::UpdateProvider`] seam that the distro owns.
//!
//! This crate reuses both; it never reimplements package management or update
//! resolution. The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — the host stack answered; `data` is the live read.
//! * `Ok(None)` — the host stack is absent. A normal state; the item hides.
//! * `Err(error)` — the host stack is present but could not be read; the item
//!   shows visible and inert with the message.
//!
//! Absence is layered, as the battery adapter's is. A host that answers has a
//! live [`SystemIdentity`]; the update provider may be absent *within* that
//! snapshot (`SystemData::updates` is `None`), which disables only the update
//! controls. The whole adapter is `Unavailable` only when neither half is
//! reachable — a normal hidden state, never an error.

use dragonfruit_system_adapters::AdapterError;

/// How serious an available update is.
///
/// The provider's own vocabulary narrowed to the three classes the pane draws
/// a badge for. An unrecognized severity is [`UpdateSeverity::Normal`], never
/// a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum UpdateSeverity {
    /// A routine update (the default).
    #[default]
    Normal,
    /// An important update the provider flags for attention.
    Important,
    /// A security update.
    Security,
}

impl UpdateSeverity {
    /// Every severity, most routine first.
    pub const ALL: [UpdateSeverity; 3] = [
        UpdateSeverity::Normal,
        UpdateSeverity::Important,
        UpdateSeverity::Security,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            UpdateSeverity::Normal => "normal",
            UpdateSeverity::Important => "important",
            UpdateSeverity::Security => "security",
        }
    }

    /// The short label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            UpdateSeverity::Normal => "Update",
            UpdateSeverity::Important => "Important",
            UpdateSeverity::Security => "Security Update",
        }
    }

    /// Parse the stable id back to a severity; an unknown id is `Normal`.
    pub fn from_id(id: &str) -> UpdateSeverity {
        UpdateSeverity::ALL
            .into_iter()
            .find(|severity| severity.id() == id)
            .unwrap_or(UpdateSeverity::Normal)
    }
}

/// The distribution update provider's reported phase.
///
/// This is the provider's state machine, projected unchanged: the adapter
/// never starts a check or an install by itself, it reports the phase the
/// provider publishes and lets the pane issue the explicit writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum UpdatePhase {
    /// No check has run; the update state is unknown.
    #[default]
    Idle,
    /// A check is in progress.
    Checking,
    /// The last check found no updates.
    UpToDate,
    /// The last check found updates (see [`UpdateData::updates`]).
    Available,
    /// An install is in progress.
    Installing,
    /// Updates were installed and the host must restart to finish.
    RebootRequired,
    /// The last check or install failed (see [`UpdateData::message`]).
    Failed,
}

impl UpdatePhase {
    /// Every phase, in the provider's natural order.
    pub const ALL: [UpdatePhase; 7] = [
        UpdatePhase::Idle,
        UpdatePhase::Checking,
        UpdatePhase::UpToDate,
        UpdatePhase::Available,
        UpdatePhase::Installing,
        UpdatePhase::RebootRequired,
        UpdatePhase::Failed,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            UpdatePhase::Idle => "idle",
            UpdatePhase::Checking => "checking",
            UpdatePhase::UpToDate => "up-to-date",
            UpdatePhase::Available => "available",
            UpdatePhase::Installing => "installing",
            UpdatePhase::RebootRequired => "reboot-required",
            UpdatePhase::Failed => "failed",
        }
    }

    /// Parse the stable id back to a phase; an unknown id is `Idle`.
    pub fn from_id(id: &str) -> UpdatePhase {
        UpdatePhase::ALL
            .into_iter()
            .find(|phase| phase.id() == id)
            .unwrap_or(UpdatePhase::Idle)
    }

    /// Whether an operation is in flight (the pane shows progress, not a
    /// button).
    pub const fn is_busy(self) -> bool {
        matches!(self, UpdatePhase::Checking | UpdatePhase::Installing)
    }
}

/// One update the provider offers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateItem {
    /// The provider's stable identifier (a package name or nevra).
    pub id: String,
    /// The display name.
    pub name: String,
    /// A one-line summary.
    pub summary: String,
    /// The installed version.
    pub current_version: String,
    /// The version the update would install.
    pub available_version: String,
    /// The severity badge.
    pub severity: UpdateSeverity,
}

/// The distribution update provider's live state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateData {
    /// The phase the provider reports.
    pub phase: UpdatePhase,
    /// The updates on offer, in the provider's order.
    pub updates: Vec<UpdateItem>,
    /// When the last check completed (milliseconds since the Unix epoch);
    /// `None` if no check has completed.
    pub last_checked_ms: Option<u64>,
    /// The provider's status or error text, when it has one.
    pub message: Option<String>,
}

impl UpdateData {
    /// Whether a check has ever completed.
    pub const fn has_checked(&self) -> bool {
        self.last_checked_ms.is_some()
    }

    /// How many updates are on offer.
    pub fn update_count(&self) -> usize {
        self.updates.len()
    }

    /// How many of the offered updates are security updates.
    pub fn security_count(&self) -> usize {
        self.updates
            .iter()
            .filter(|update| update.severity == UpdateSeverity::Security)
            .count()
    }
}

/// The host identity the About and General rows render.
///
/// Every field is a plain read of the host's own sources; an unknown value is
/// the empty string (or `0` bytes), never an invented one. The live reads live
/// in [`crate::HostSystem`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemIdentity {
    /// The computer name (hostname).
    pub host_name: String,
    /// The distribution's pretty name (`PRETTY_NAME`).
    pub os_name: String,
    /// The distribution's version (`VERSION_ID`).
    pub os_version: String,
    /// The distribution's id (`ID`).
    pub os_id: String,
    /// The running kernel release.
    pub kernel: String,
    /// The machine architecture.
    pub architecture: String,
    /// The hardware model (`/sys/.../dmi/id/product_name`).
    pub device_model: String,
    /// The processor description.
    pub processor: String,
    /// The total physical memory in bytes (`0` when unknown).
    pub memory_bytes: u64,
    /// The hardware serial (`/sys/.../dmi/id/product_serial`); often empty.
    pub serial: String,
}

impl Default for SystemIdentity {
    /// The honest "not read" identity: a local computer with empty fields.
    fn default() -> Self {
        SystemIdentity {
            host_name: "localhost".to_owned(),
            os_name: String::new(),
            os_version: String::new(),
            os_id: String::new(),
            kernel: String::new(),
            architecture: String::new(),
            device_model: String::new(),
            processor: String::new(),
            memory_bytes: 0,
            serial: String::new(),
        }
    }
}

impl SystemIdentity {
    /// The OS label the About rows draw: `<PRETTY_NAME> <VERSION_ID>` with the
    /// empty half omitted, falling back to the os id, then to `Linux`.
    pub fn os_label(&self) -> String {
        let name = if self.os_name.is_empty() {
            if self.os_id.is_empty() {
                "Linux".to_owned()
            } else {
                self.os_id.clone()
            }
        } else {
            self.os_name.clone()
        };
        if self.os_version.is_empty() {
            name
        } else {
            format!("{name} {}", self.os_version)
        }
    }

    /// The memory label the About rows draw, rounded to the nearest gigabyte;
    /// `Unknown` when the read is missing.
    pub fn memory_label(&self) -> String {
        const GIB: u64 = 1024 * 1024 * 1024;
        if self.memory_bytes == 0 {
            return "Unknown".to_owned();
        }
        let gib = (self.memory_bytes + GIB / 2) / GIB;
        format!("{gib} GB")
    }

    /// Whether the host exposed a serial number.
    pub fn has_serial(&self) -> bool {
        !self.serial.is_empty()
    }

    /// The device name the About heading draws: the model when the host
    /// exposed one, else the computer name.
    pub fn device_name(&self) -> String {
        if self.device_model.is_empty() {
            self.host_name.clone()
        } else {
            self.device_model.clone()
        }
    }
}

/// The raw result of one host-stack read: the identity that is always read,
/// plus the update provider's state when the provider is present.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SystemData {
    /// The host identity (the About/General half).
    pub identity: SystemIdentity,
    /// The distribution update provider's state; `None` when the provider is
    /// absent (only the update controls disable).
    pub updates: Option<UpdateData>,
}

impl SystemData {
    /// Whether the distribution update provider is present.
    pub const fn updates_available(&self) -> bool {
        self.updates.is_some()
    }
}

/// The result of one update-provider write (check, install, or reboot).
///
/// These are explicit user actions, never a poll. A write that lands invents
/// no snapshot: the provider publishes the resulting state and the host
/// re-reads, so the snapshot stays the single source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// The provider accepted the request.
    Applied,
    /// The provider is absent; there is nothing to do.
    Absent,
    /// The request failed (including a denied privileged operation).
    Failed(AdapterError),
}

impl UpdateOutcome {
    /// Whether the provider accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, UpdateOutcome::Applied)
    }

    /// The failure, when the request failed.
    pub fn error(&self) -> Option<&AdapterError> {
        match self {
            UpdateOutcome::Failed(error) => Some(error),
            UpdateOutcome::Applied | UpdateOutcome::Absent => None,
        }
    }
}

/// Reads the host stack over some transport.
pub trait SystemSource {
    /// One read of the host stack.
    fn read(&mut self) -> Result<Option<SystemData>, AdapterError>;

    /// Ask the provider to check for updates. One explicit write.
    ///
    /// The default is [`UpdateOutcome::Absent`], so a source without a write
    /// half still satisfies the trait.
    fn check(&mut self) -> UpdateOutcome {
        UpdateOutcome::Absent
    }

    /// Ask the provider to install the available updates. One explicit write.
    fn install(&mut self) -> UpdateOutcome {
        UpdateOutcome::Absent
    }

    /// Ask the provider to restart the host. One explicit write.
    fn reboot(&mut self) -> UpdateOutcome {
        UpdateOutcome::Absent
    }
}

/// A fixture-backed source with a simulated host-stack lifecycle.
///
/// The mock is the CI path: it serves [`SystemData`] with no host provider on
/// the machine, and `kill`/`restart` exercise absence and re-subscribe the way
/// masking the provider would. `push` drives the identity and the update state
/// so a test observes the change stream. Its writes mutate the simulated
/// provider state the way the provider would, so a subsequent `read` sees the
/// change.
#[derive(Debug, Clone, PartialEq)]
pub struct MockSystem {
    present: bool,
    data: Option<SystemData>,
    failure: Option<AdapterError>,
    write_failure: Option<AdapterError>,
    reads: u32,
    checks: u32,
    installs: u32,
    reboots: u32,
}

impl MockSystem {
    /// The host stack is not reachable.
    pub fn absent() -> Self {
        MockSystem {
            present: false,
            data: None,
            failure: None,
            write_failure: None,
            reads: 0,
            checks: 0,
            installs: 0,
            reboots: 0,
        }
    }

    /// A host stack that answers with `data`.
    pub fn present(data: SystemData) -> Self {
        MockSystem {
            present: true,
            data: Some(data),
            failure: None,
            write_failure: None,
            reads: 0,
            checks: 0,
            installs: 0,
            reboots: 0,
        }
    }

    /// A host stack that fails every read (e.g. the provider went
    /// unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockSystem {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            write_failure: None,
            reads: 0,
            checks: 0,
            installs: 0,
            reboots: 0,
        }
    }

    /// A host stack whose writes fail.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// The host stack publishes fresh state.
    pub fn push(&mut self, data: SystemData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The host stack goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host stack comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated host stack is reachable.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many checks the source has accepted.
    pub fn checks(&self) -> u32 {
        self.checks
    }

    /// How many installs the source has accepted.
    pub fn installs(&self) -> u32 {
        self.installs
    }

    /// How many reboots the source has accepted.
    pub fn reboots(&self) -> u32 {
        self.reboots
    }

    fn outcome(&self) -> UpdateOutcome {
        if !self.present {
            return UpdateOutcome::Absent;
        }
        if let Some(error) = &self.write_failure {
            return UpdateOutcome::Failed(error.clone());
        }
        // A reachable host with no update provider has nothing to write to;
        // absence is layered the way `SystemData::updates` is.
        match &self.data {
            Some(data) if data.updates.is_some() => UpdateOutcome::Applied,
            _ => UpdateOutcome::Absent,
        }
    }
}

impl SystemSource for MockSystem {
    fn read(&mut self) -> Result<Option<SystemData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn check(&mut self) -> UpdateOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.checks += 1;
            if let Some(updates) = self.data.as_mut().and_then(|data| data.updates.as_mut()) {
                updates.phase = if updates.updates.is_empty() {
                    UpdatePhase::UpToDate
                } else {
                    UpdatePhase::Available
                };
            }
        }
        outcome
    }

    fn install(&mut self) -> UpdateOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.installs += 1;
            if let Some(updates) = self.data.as_mut().and_then(|data| data.updates.as_mut()) {
                updates.updates.clear();
                updates.phase = UpdatePhase::UpToDate;
            }
        }
        outcome
    }

    fn reboot(&mut self) -> UpdateOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.reboots += 1;
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> SystemData {
        SystemData {
            identity: SystemIdentity {
                host_name: "dragon".to_owned(),
                os_name: "Dragonfruit Linux".to_owned(),
                os_version: "44".to_owned(),
                os_id: "dragonfruit".to_owned(),
                kernel: "6.12.0".to_owned(),
                architecture: "x86_64".to_owned(),
                device_model: "Dragonfruit Book".to_owned(),
                processor: "Example CPU".to_owned(),
                memory_bytes: 16 * 1024 * 1024 * 1024,
                serial: "SERIAL".to_owned(),
            },
            updates: Some(UpdateData {
                phase: UpdatePhase::Available,
                updates: vec![UpdateItem {
                    id: "kernel".to_owned(),
                    name: "kernel".to_owned(),
                    summary: "The Linux kernel".to_owned(),
                    current_version: "6.11".to_owned(),
                    available_version: "6.12".to_owned(),
                    severity: UpdateSeverity::Security,
                }],
                last_checked_ms: Some(1000),
                message: None,
            }),
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockSystem::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.is_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockSystem::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockSystem::failing("update provider: timeout");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "update provider: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockSystem::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn push_replaces_the_served_data() {
        let mut mock = MockSystem::present(data());
        let mut next = data();
        next.identity.host_name = "renamed".to_owned();
        mock.push(next);
        assert_eq!(mock.read().unwrap().unwrap().identity.host_name, "renamed");
    }

    #[test]
    fn a_check_moves_the_phase_from_the_list() {
        let mut mock = MockSystem::present(data());
        assert_eq!(mock.check(), UpdateOutcome::Applied);
        assert_eq!(mock.checks(), 1);
        assert_eq!(
            mock.read().unwrap().unwrap().updates.unwrap().phase,
            UpdatePhase::Available
        );

        let mut empty = data();
        empty.updates = Some(UpdateData {
            phase: UpdatePhase::Idle,
            ..UpdateData::default()
        });
        let mut mock = MockSystem::present(empty);
        assert_eq!(mock.check(), UpdateOutcome::Applied);
        assert_eq!(
            mock.read().unwrap().unwrap().updates.unwrap().phase,
            UpdatePhase::UpToDate
        );
    }

    #[test]
    fn an_install_clears_the_list() {
        let mut mock = MockSystem::present(data());
        assert_eq!(mock.install(), UpdateOutcome::Applied);
        assert_eq!(mock.installs(), 1);
        let updates = mock.read().unwrap().unwrap().updates.unwrap();
        assert!(updates.updates.is_empty());
        assert_eq!(updates.phase, UpdatePhase::UpToDate);
    }

    #[test]
    fn a_reboot_is_a_write() {
        let mut mock = MockSystem::present(data());
        assert_eq!(mock.reboot(), UpdateOutcome::Applied);
        assert_eq!(mock.reboots(), 1);
    }

    #[test]
    fn writes_while_absent_are_absent() {
        let mut mock = MockSystem::absent();
        assert_eq!(mock.check(), UpdateOutcome::Absent);
        assert_eq!(mock.install(), UpdateOutcome::Absent);
        assert_eq!(mock.reboot(), UpdateOutcome::Absent);
        assert_eq!(mock.checks(), 0);
        assert_eq!(mock.installs(), 0);
        assert_eq!(mock.reboots(), 0);
    }

    #[test]
    fn a_failing_write_reports_the_error() {
        let mut mock = MockSystem::present(data()).fail_writes("polkit: not authorized");
        let outcome = mock.install();
        assert_eq!(
            outcome.error().map(AdapterError::message),
            Some("polkit: not authorized")
        );
        assert!(!outcome.is_applied());
    }

    #[test]
    fn severity_ids_round_trip_and_unknown_is_normal() {
        for severity in UpdateSeverity::ALL {
            assert_eq!(UpdateSeverity::from_id(severity.id()), severity);
        }
        assert_eq!(UpdateSeverity::from_id("bogus"), UpdateSeverity::Normal);
        assert_eq!(UpdateSeverity::Security.label(), "Security Update");
    }

    #[test]
    fn phase_ids_round_trip_and_unknown_is_idle() {
        for phase in UpdatePhase::ALL {
            assert_eq!(UpdatePhase::from_id(phase.id()), phase);
        }
        assert_eq!(UpdatePhase::from_id("bogus"), UpdatePhase::Idle);
        assert!(UpdatePhase::Checking.is_busy());
        assert!(UpdatePhase::Installing.is_busy());
        assert!(!UpdatePhase::Available.is_busy());
    }

    #[test]
    fn identity_labels_are_honest() {
        let identity = data().identity;
        assert_eq!(identity.os_label(), "Dragonfruit Linux 44");
        assert_eq!(identity.memory_label(), "16 GB");
        assert!(identity.has_serial());
        assert_eq!(identity.device_name(), "Dragonfruit Book");

        let unknown = SystemIdentity::default();
        assert_eq!(unknown.os_label(), "Linux");
        assert_eq!(unknown.memory_label(), "Unknown");
        assert!(!unknown.has_serial());
        assert_eq!(unknown.device_name(), "localhost");
    }

    #[test]
    fn update_data_counts_the_list() {
        let updates = data().updates.unwrap();
        assert_eq!(updates.update_count(), 1);
        assert_eq!(updates.security_count(), 1);
        assert!(updates.has_checked());
    }
}
