// SPDX-License-Identifier: GPL-3.0-or-later
//! The Biometrics and Password half of the account adapter (T228).
//!
//! The host stack has three independent owners:
//!
//! * **fprintd** (`net.reactivated.Fprint` on the system bus) lists the
//!   enrolled fingers and performs enrollment/deletion. A host with no reader
//!   is a normal state: the pane shows the note and keeps the password half.
//! * **PAM** is read from `/etc/pam.d/*`: the pane shows whether
//!   `pam_fprintd.so` is configured for login/unlock and for authentication.
//!   These are read-only status rows; changing PAM is system policy, never a
//!   desktop toggle.
//! * **AccountsService** owns the password write (`SetPassword`). The password
//!   is handed straight to the daemon, never stored or logged.
//!
//! Absence is layered: a missing fprintd disables only the fingerprint rows;
//! a missing AccountsService disables only the password row. The adapter is
//! `Unavailable` only when neither owner answers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use dragonfruit_system_adapters::{
    Adapter, AdapterError, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use crate::accounts::{
    classify, daemon_connection, name_has_owner, read_users, user_path, ACCOUNTS_SERVICE,
    USER_INTERFACE,
};
use crate::source::AccountOutcome;

/// The well-known fprintd name on the system bus.
pub const FPRINT_SERVICE: &str = "net.reactivated.Fprint";
/// The fprintd manager path.
pub const FPRINT_MANAGER_PATH: &str = "/net/reactivated/Fprint/Manager";
/// The fprintd manager interface.
pub const FPRINT_MANAGER_INTERFACE: &str = "net.reactivated.Fprint.Manager";
/// The fprintd device interface.
pub const FPRINT_DEVICE_INTERFACE: &str = "net.reactivated.Fprint.Device";
/// The PAM module that provides fingerprint authentication.
pub const PAM_FPRINTD_MODULE: &str = "pam_fprintd.so";

/// The ten finger names fprintd accepts.
pub const FINGERS: [&str; 10] = [
    "left-thumb",
    "left-index-finger",
    "left-middle-finger",
    "left-ring-finger",
    "left-little-finger",
    "right-thumb",
    "right-index-finger",
    "right-middle-finger",
    "right-ring-finger",
    "right-little-finger",
];

/// A human label for a finger id.
pub fn finger_label(finger: &str) -> &str {
    match finger {
        "left-thumb" => "Left thumb",
        "left-index-finger" => "Left index finger",
        "left-middle-finger" => "Left middle finger",
        "left-ring-finger" => "Left ring finger",
        "left-little-finger" => "Left little finger",
        "right-thumb" => "Right thumb",
        "right-index-finger" => "Right index finger",
        "right-middle-finger" => "Right middle finger",
        "right-ring-finger" => "Right ring finger",
        "right-little-finger" => "Right little finger",
        _ => finger,
    }
}

/// A human label for one fprintd `EnrollStatus` stage id.
///
/// The stage ids are fprintd's own vocabulary; an unknown id is shown verbatim
/// so a future stage is never disguised as a percentage or a guess.
pub fn enroll_stage_label(stage: &str) -> &str {
    match stage {
        "" => "Waiting for the fingerprint sensor",
        "enroll-stage-passed" => "Fingerprint captured; place the same finger again",
        "enroll-completed" => "Fingerprint enrolled",
        "enroll-retry-scan" => "Scan again",
        "enroll-swipe-too-short" => "Swipe too short; try again",
        "enroll-finger-not-centered" => "Center your finger on the sensor",
        "enroll-remove-and-retry" => "Remove your finger and try again",
        "enroll-data-full" => "The fingerprint storage is full",
        "enroll-failed" => "Enrollment failed",
        "enroll-unknown-error" => "The reader reported an unknown error",
        "enroll-cancelled" => "Enrollment cancelled",
        other => other,
    }
}

/// The fingerprint reader and its enrolled fingers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FingerprintDeviceData {
    /// The device object path.
    pub path: String,
    /// The product name.
    pub name: String,
    /// `press` or `swipe`.
    pub scan_type: String,
    /// The enrolled finger ids for the current user.
    pub enrolled: Vec<String>,
    /// A note when the reader is present but unusable.
    pub note: String,
}

/// The PAM fingerprint configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PamData {
    /// Whether any PAM file mentions `pam_fprintd.so`.
    pub available: bool,
    /// Whether a login/unlock PAM file configures it.
    pub unlock: bool,
    /// Whether an authentication PAM file configures it.
    pub auth: bool,
    /// The PAM files that mention the module.
    pub files: Vec<String>,
}

/// The password-change capability.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PasswordData {
    /// Whether AccountsService is reachable.
    pub available: bool,
    /// The owner/method (`accountsservice`).
    pub method: String,
    /// The note the pane shows.
    pub note: String,
}

/// An in-flight fingerprint enrollment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnrollmentData {
    /// The finger being enrolled.
    pub finger: String,
    /// fprintd's own stage id (`enroll-stage-passed`, `enroll-completed`, …).
    /// Empty until the reader reports the first stage.
    pub stage: String,
    /// The human line derived from the stage; never a percentage.
    pub status: String,
    /// Whether fprintd reported the enrollment complete.
    pub done: bool,
}

/// The raw Biometrics and Password read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BiometricsData {
    /// The current login name.
    pub user: String,
    /// The current uid.
    pub uid: u64,
    /// Whether fprintd owns its system-bus name.
    pub fprintd_available: bool,
    /// The default reader, when one exists.
    pub device: Option<FingerprintDeviceData>,
    /// The PAM configuration.
    pub pam: PamData,
    /// The password-change capability.
    pub password: PasswordData,
    /// The in-flight enrollment, when one is running.
    pub enrolling: Option<EnrollmentData>,
    /// The last failed enrollment stage, shown at the row until the next
    /// enrollment starts. fprintd's own id, never an invented message.
    pub enrollment_error: Option<String>,
}

/// The result of one biometrics write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiometricsOutcome {
    /// The owner accepted the request.
    Applied,
    /// Policy refused the request.
    Denied(String),
    /// The owner is absent.
    Absent,
    /// The request failed.
    Failed(AdapterError),
}

impl BiometricsOutcome {
    /// Whether the owner accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, BiometricsOutcome::Applied)
    }

    /// The refusal or failure text.
    pub fn message(&self) -> Option<&str> {
        match self {
            BiometricsOutcome::Denied(message) => Some(message),
            BiometricsOutcome::Failed(error) => Some(error.message()),
            BiometricsOutcome::Applied | BiometricsOutcome::Absent => None,
        }
    }
}

/// Reads the biometrics host stack and drives its explicit writes.
pub trait BiometricsSource {
    /// One read of fprintd, PAM, and AccountsService.
    fn read_biometrics(&mut self) -> Result<Option<BiometricsData>, AdapterError>;

    /// Start enrolling `finger`. One explicit write.
    fn start_enrollment(&mut self, _finger: &str) -> BiometricsOutcome {
        BiometricsOutcome::Absent
    }

    /// Stop the in-flight enrollment. One explicit write.
    fn cancel_enrollment(&mut self) -> BiometricsOutcome {
        BiometricsOutcome::Absent
    }

    /// Delete `finger`. One explicit write.
    fn delete_finger(&mut self, _finger: &str) -> BiometricsOutcome {
        BiometricsOutcome::Absent
    }

    /// Change the current user's password. One explicit write.
    fn change_password(&mut self, _password: &str) -> BiometricsOutcome {
        BiometricsOutcome::Absent
    }
}

/// The typed Biometrics snapshot the pane renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BiometricsSnapshot {
    data: BiometricsData,
}

impl BiometricsSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &BiometricsData) -> Self {
        BiometricsSnapshot { data: data.clone() }
    }

    /// The current user.
    pub fn user(&self) -> &str {
        &self.data.user
    }

    /// The current uid.
    pub fn uid(&self) -> u64 {
        self.data.uid
    }

    /// Whether fprintd is running.
    pub fn fprintd_available(&self) -> bool {
        self.data.fprintd_available
    }

    /// The default reader.
    pub fn device(&self) -> Option<&FingerprintDeviceData> {
        self.data.device.as_ref()
    }

    /// The enrolled fingers.
    pub fn enrolled(&self) -> &[String] {
        self.data
            .device
            .as_ref()
            .map(|device| device.enrolled.as_slice())
            .unwrap_or(&[])
    }

    /// The PAM configuration.
    pub fn pam(&self) -> &PamData {
        &self.data.pam
    }

    /// The password capability.
    pub fn password(&self) -> &PasswordData {
        &self.data.password
    }

    /// The in-flight enrollment.
    pub fn enrolling(&self) -> Option<&EnrollmentData> {
        self.data.enrolling.as_ref()
    }

    /// The last failed enrollment stage, when one is pending acknowledgement.
    pub fn enrollment_error(&self) -> Option<&str> {
        self.data.enrollment_error.as_deref()
    }

    /// Whether any biometrics owner answered.
    pub fn present(&self) -> bool {
        self.data.fprintd_available || self.data.password.available || self.data.pam.available
    }

    /// The pane header label.
    pub fn label(&self) -> String {
        match self.enrolled().len() {
            0 => "No Fingerprints".to_owned(),
            1 => "1 Fingerprint".to_owned(),
            count => format!("{count} Fingerprints"),
        }
    }

    /// The design-system glyph.
    pub fn glyph(&self) -> &'static str {
        "fingerprint"
    }
}

/// The Biometrics adapter: one read path over the shared contract.
#[derive(Debug, Clone, PartialEq)]
pub struct BiometricsAdapter<S> {
    source: S,
    state: AdapterState<BiometricsSnapshot>,
    subscription: Subscription,
}

impl<S: BiometricsSource> BiometricsAdapter<S> {
    /// A fresh adapter over `source`.
    pub fn new(source: S) -> Self {
        BiometricsAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
        }
    }

    /// Read the biometrics host stack once.
    pub fn refresh(&mut self) {
        match self.source.read_biometrics() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(BiometricsSnapshot::from_data(&data));
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

    /// Start enrolling `finger`.
    pub fn start_enrollment(&mut self, finger: &str) -> BiometricsOutcome {
        self.source.start_enrollment(finger)
    }

    /// Stop the in-flight enrollment.
    pub fn cancel_enrollment(&mut self) -> BiometricsOutcome {
        self.source.cancel_enrollment()
    }

    /// Delete `finger`.
    pub fn delete_finger(&mut self, finger: &str) -> BiometricsOutcome {
        self.source.delete_finger(finger)
    }

    /// Change the current user's password.
    pub fn change_password(&mut self, password: &str) -> BiometricsOutcome {
        self.source.change_password(password)
    }

    /// The live snapshot.
    pub fn snapshot(&self) -> Option<&BiometricsSnapshot> {
        self.state.snapshot()
    }

    /// The transport.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The transport, mutably.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// How many times the adapter subscribed.
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }
}

impl<S: BiometricsSource> Adapter for BiometricsAdapter<S> {
    type Snapshot = BiometricsSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::BIOMETRICS
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

/// The host-side enrollment session.
#[derive(Debug, Default)]
pub struct BiometricsHostState {
    session: Option<EnrollmentSession>,
    /// The last failed enrollment stage, kept until a new enrollment starts.
    last_error: Option<String>,
}

#[derive(Debug)]
struct EnrollmentSession {
    device_path: String,
    finger: String,
    baseline: usize,
    started: Instant,
    /// fprintd's last `EnrollStatus(result, done)` signal.
    stage: Arc<Mutex<Option<(String, bool)>>>,
    stop: Arc<AtomicBool>,
}

impl BiometricsHostState {
    /// Whether an enrollment is in flight.
    pub fn is_enrolling(&self) -> bool {
        self.session.is_some()
    }

    /// The last failed enrollment stage, when one is pending acknowledgement.
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

/// Subscribe to fprintd's `EnrollStatus` signals for one device.
///
/// The blocking zbus iterator lives on its own thread for the duration of the
/// enrollment. The thread exits on fprintd's `done` signal, on the stop flag,
/// or when the match iterator is torn down; a cancel wakes it with one
/// `enroll-cancelled` signal so it never lingers.
fn spawn_enroll_listener(
    device_path: String,
    stage: Arc<Mutex<Option<(String, bool)>>>,
    stop: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let Ok(connection) = Connection::system() else {
            return;
        };
        let Ok(rule) = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(FPRINT_DEVICE_INTERFACE)
            .and_then(|builder| builder.member("EnrollStatus"))
            .and_then(|builder| builder.path(device_path.as_str()))
            .map(|builder| builder.build())
        else {
            return;
        };
        let Ok(iterator) = MessageIterator::for_match_rule(rule, &connection, Some(8)) else {
            return;
        };
        for message in iterator {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let Ok(message) = message else {
                continue;
            };
            let Ok((result, done)) = message.body().deserialize::<(String, bool)>() else {
                continue;
            };
            if let Ok(mut slot) = stage.lock() {
                *slot = Some((result, done));
            }
            if done {
                break;
            }
        }
    });
}

/// Wake and retire the enrollment listener.
fn stop_enroll_listener(session: &EnrollmentSession) {
    session.stop.store(true, Ordering::SeqCst);
    // The listener's match rule has no sender filter, so a matching signal
    // from our own connection unblocks its pending `next()`.
    if let Ok(connection) = Connection::system() {
        let _ = connection.emit_signal(
            None::<&str>,
            session.device_path.as_str(),
            FPRINT_DEVICE_INTERFACE,
            "EnrollStatus",
            &("enroll-cancelled", false),
        );
    }
}

/// A deterministic biometrics fixture for CI and pane tests.
pub fn biometrics_fixture() -> BiometricsData {
    BiometricsData {
        user: "dan".to_owned(),
        uid: 1000,
        fprintd_available: true,
        device: Some(FingerprintDeviceData {
            path: "/net/reactivated/Fprint/Device/0".to_owned(),
            name: "Goodix Fingerprint Sensor".to_owned(),
            scan_type: "press".to_owned(),
            enrolled: vec!["right-index-finger".to_owned()],
            note: String::new(),
        }),
        pam: PamData {
            available: true,
            unlock: true,
            auth: true,
            files: vec!["system-auth".to_owned(), "fingerprint-auth".to_owned()],
        },
        password: PasswordData {
            available: true,
            method: "accountsservice".to_owned(),
            note: "The password is changed through the account service after authorization."
                .to_owned(),
        },
        enrolling: None,
        enrollment_error: None,
    }
}

/// A deterministic biometrics source for CI and pane tests.
#[derive(Debug, Clone, PartialEq)]
pub struct MockBiometrics {
    present: bool,
    data: Option<BiometricsData>,
    write_failure: Option<AdapterError>,
    deny_writes: Option<String>,
    writes: u32,
    last_finger: Option<String>,
    last_password: Option<String>,
}

impl MockBiometrics {
    /// No biometrics owner is reachable.
    pub fn absent() -> Self {
        MockBiometrics {
            present: false,
            data: None,
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_finger: None,
            last_password: None,
        }
    }

    /// A host answering with the deterministic fixture.
    pub fn present() -> Self {
        MockBiometrics {
            present: true,
            data: Some(biometrics_fixture()),
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_finger: None,
            last_password: None,
        }
    }

    /// A host answering with `data`.
    pub fn with_data(data: BiometricsData) -> Self {
        MockBiometrics {
            present: true,
            data: Some(data),
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_finger: None,
            last_password: None,
        }
    }

    /// Make every write fail.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// Make every write a denial.
    pub fn deny_writes(mut self, note: impl Into<String>) -> Self {
        self.deny_writes = Some(note.into());
        self
    }

    /// Publish fresh state.
    pub fn push(&mut self, data: BiometricsData) {
        self.present = true;
        self.data = Some(data);
    }

    /// The host goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host comes back.
    pub fn restart(&mut self) {
        self.present = true;
    }

    /// The write count.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    /// The last finger write.
    pub fn last_finger(&self) -> Option<&str> {
        self.last_finger.as_deref()
    }

    /// The last password write.
    pub fn last_password(&self) -> Option<&str> {
        self.last_password.as_deref()
    }

    fn outcome(&self) -> BiometricsOutcome {
        if !self.present {
            return BiometricsOutcome::Absent;
        }
        if let Some(note) = &self.deny_writes {
            return BiometricsOutcome::Denied(note.clone());
        }
        if let Some(error) = &self.write_failure {
            return BiometricsOutcome::Failed(error.clone());
        }
        BiometricsOutcome::Applied
    }

    fn data_mut(&mut self) -> Option<&mut BiometricsData> {
        self.data.as_mut()
    }
}

impl BiometricsSource for MockBiometrics {
    fn read_biometrics(&mut self) -> Result<Option<BiometricsData>, AdapterError> {
        if !self.present {
            return Ok(None);
        }
        Ok(self.data.clone())
    }

    fn start_enrollment(&mut self, finger: &str) -> BiometricsOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_finger = Some(finger.to_owned());
            if let Some(data) = self.data_mut() {
                if let Some(device) = data.device.as_mut() {
                    if !device.enrolled.iter().any(|entry| entry == finger) {
                        device.enrolled.push(finger.to_owned());
                    }
                }
                data.enrolling = Some(EnrollmentData {
                    finger: finger.to_owned(),
                    stage: "enroll-completed".to_owned(),
                    status: enroll_stage_label("enroll-completed").to_owned(),
                    done: true,
                });
            }
        }
        outcome
    }

    fn cancel_enrollment(&mut self) -> BiometricsOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            if let Some(data) = self.data_mut() {
                data.enrolling = None;
            }
        }
        outcome
    }

    fn delete_finger(&mut self, finger: &str) -> BiometricsOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_finger = Some(finger.to_owned());
            if let Some(data) = self.data_mut() {
                if let Some(device) = data.device.as_mut() {
                    device.enrolled.retain(|entry| entry != finger);
                }
            }
        }
        outcome
    }

    fn change_password(&mut self, password: &str) -> BiometricsOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_password = Some(password.to_owned());
        }
        outcome
    }
}

/// Read the live biometrics host stack. The `state` carries the in-flight
/// enrollment across reads.
pub fn read_host_biometrics(
    state: &mut BiometricsHostState,
) -> Result<Option<BiometricsData>, AdapterError> {
    let user = std::env::var("USER").unwrap_or_default();
    let uid = read_users()
        .ok()
        .flatten()
        .and_then(|users| {
            users
                .into_iter()
                .find(|user| user.user_name == std::env::var("USER").unwrap_or_default())
        })
        .map(|user| user.uid)
        .unwrap_or_default();
    let pam = read_pam();
    let password = PasswordData {
        available: accounts_available(),
        method: "accountsservice".to_owned(),
        note: "The password is changed through the account service after authorization.".to_owned(),
    };
    let (fprintd_available, device, enrolling) = read_fprintd(state, &user);
    if !fprintd_available && !password.available && !pam.available {
        return Ok(None);
    }
    Ok(Some(BiometricsData {
        user,
        uid,
        fprintd_available,
        device,
        pam,
        password,
        enrolling,
        enrollment_error: state.last_error.clone(),
    }))
}

fn accounts_available() -> bool {
    match Connection::system() {
        Ok(connection) => service_available(&connection, ACCOUNTS_SERVICE),
        Err(_) => false,
    }
}

/// Whether `service` owns its name, activating it once when it is installed
/// but not running. A settings read may start the host's own service; a
/// service that is not installed fails the activation and reads as absent.
pub(crate) fn service_available(connection: &Connection, service: &str) -> bool {
    if name_has_owner(connection, service).unwrap_or(false) {
        return true;
    }
    connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "StartServiceByName",
            &(service, 0u32),
        )
        .is_ok()
}

fn read_pam() -> PamData {
    let dir = Path::new("/etc/pam.d");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return PamData::default();
    };
    let mut files = Vec::new();
    let mut unlock = false;
    let mut auth = false;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !text.contains(PAM_FPRINTD_MODULE) {
            continue;
        }
        files.push(name.to_owned());
        if matches!(
            name,
            "login"
                | "gdm-password"
                | "sddm"
                | "lightdm"
                | "system-auth"
                | "password-auth"
                | "fingerprint-auth"
        ) {
            unlock = true;
        }
        if matches!(name, "sudo" | "polkit-1" | "system-auth" | "password-auth") {
            auth = true;
        }
    }
    files.sort();
    PamData {
        available: !files.is_empty(),
        unlock,
        auth,
        files,
    }
}

fn read_fprintd(
    state: &mut BiometricsHostState,
    user: &str,
) -> (bool, Option<FingerprintDeviceData>, Option<EnrollmentData>) {
    let Ok(connection) = Connection::system() else {
        state.session = None;
        return (false, None, None);
    };
    if !service_available(&connection, FPRINT_SERVICE) {
        state.session = None;
        return (false, None, None);
    }
    let device_path = match connection.call_method(
        Some(FPRINT_SERVICE),
        FPRINT_MANAGER_PATH,
        Some(FPRINT_MANAGER_INTERFACE),
        "GetDefaultDevice",
        &(),
    ) {
        Ok(reply) => reply
            .body()
            .deserialize::<OwnedObjectPath>()
            .map(|path| path.to_string())
            .unwrap_or_default(),
        Err(_) => {
            state.session = None;
            return (true, None, None);
        }
    };
    if device_path.is_empty() {
        state.session = None;
        return (true, None, None);
    }
    let device = read_device(&connection, &device_path, user);
    let mut enrolling = None;
    if let Some(session) = state.session.as_ref() {
        let expired = session.started.elapsed() > Duration::from_secs(120);
        let stage = session
            .stage
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_default();
        let (stage_id, stage_done) = stage;
        let enrolled = device
            .as_ref()
            .map(|device| {
                device
                    .enrolled
                    .iter()
                    .any(|finger| finger == &session.finger)
                    && device.enrolled.len() > session.baseline
            })
            .unwrap_or(false);
        let failed = stage_done && stage_id != "enroll-completed" || expired;
        if enrolled || stage_done || expired {
            let _ = connection.call_method(
                Some(FPRINT_SERVICE),
                session.device_path.as_str(),
                Some(FPRINT_DEVICE_INTERFACE),
                "EnrollStop",
                &(),
            );
            let _ = connection.call_method(
                Some(FPRINT_SERVICE),
                session.device_path.as_str(),
                Some(FPRINT_DEVICE_INTERFACE),
                "Release",
                &(),
            );
            stop_enroll_listener(session);
            // A failed stage stays visible at the row; a completed one clears
            // it. The stage id is fprintd's own, never a fabricated message.
            if failed {
                state.last_error = Some(if expired {
                    "enroll-timed-out".to_owned()
                } else {
                    stage_id
                });
            } else {
                state.last_error = None;
            }
            state.session = None;
        } else {
            enrolling = Some(EnrollmentData {
                finger: session.finger.clone(),
                stage: stage_id.clone(),
                status: enroll_stage_label(&stage_id).to_owned(),
                done: false,
            });
        }
    }
    (true, device, enrolling)
}

fn read_device(
    connection: &Connection,
    device_path: &str,
    _user: &str,
) -> Option<FingerprintDeviceData> {
    let props = connection
        .call_method(
            Some(FPRINT_SERVICE),
            device_path,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(FPRINT_DEVICE_INTERFACE,),
        )
        .ok()
        .and_then(|reply| {
            reply
                .body()
                .deserialize::<HashMap<String, OwnedValue>>()
                .ok()
        })
        .unwrap_or_default();
    let name = props
        .get("name")
        .and_then(|value| value.downcast_ref::<String>().ok())
        .unwrap_or_default();
    let scan_type = props
        .get("scan-type")
        .and_then(|value| value.downcast_ref::<String>().ok())
        .unwrap_or_default();
    let enrolled = connection
        .call_method(
            Some(FPRINT_SERVICE),
            device_path,
            Some(FPRINT_DEVICE_INTERFACE),
            "ListEnrolledFingers",
            &("",),
        )
        .ok()
        .and_then(|reply| reply.body().deserialize::<Vec<String>>().ok())
        .unwrap_or_default();
    Some(FingerprintDeviceData {
        path: device_path.to_owned(),
        name,
        scan_type,
        enrolled,
        note: String::new(),
    })
}

/// Start an enrollment on the live host.
pub fn start_host_enrollment(state: &mut BiometricsHostState, finger: &str) -> BiometricsOutcome {
    if !FINGERS.contains(&finger) {
        return BiometricsOutcome::Denied(format!("unknown finger: {finger}"));
    }
    let connection = match Connection::system() {
        Ok(connection) => connection,
        Err(_) => return BiometricsOutcome::Absent,
    };
    if !service_available(&connection, FPRINT_SERVICE) {
        return BiometricsOutcome::Absent;
    }
    let device_path = match connection.call_method(
        Some(FPRINT_SERVICE),
        FPRINT_MANAGER_PATH,
        Some(FPRINT_MANAGER_INTERFACE),
        "GetDefaultDevice",
        &(),
    ) {
        Ok(reply) => reply
            .body()
            .deserialize::<OwnedObjectPath>()
            .map(|path| path.to_string())
            .unwrap_or_default(),
        Err(_) => return BiometricsOutcome::Absent,
    };
    if device_path.is_empty() {
        return BiometricsOutcome::Absent;
    }
    let user = std::env::var("USER").unwrap_or_default();
    let baseline = read_device(&connection, &device_path, &user)
        .map(|device| device.enrolled.len())
        .unwrap_or(0);
    if let Err(error) = connection.call_method(
        Some(FPRINT_SERVICE),
        device_path.as_str(),
        Some(FPRINT_DEVICE_INTERFACE),
        "Claim",
        &("",),
    ) {
        return classify_biometrics(error);
    }
    if let Err(error) = connection.call_method(
        Some(FPRINT_SERVICE),
        device_path.as_str(),
        Some(FPRINT_DEVICE_INTERFACE),
        "EnrollStart",
        &(finger,),
    ) {
        let _ = connection.call_method(
            Some(FPRINT_SERVICE),
            device_path.as_str(),
            Some(FPRINT_DEVICE_INTERFACE),
            "Release",
            &(),
        );
        return classify_biometrics(error);
    }
    let stage = Arc::new(Mutex::new(None));
    let stop = Arc::new(AtomicBool::new(false));
    spawn_enroll_listener(device_path.clone(), stage.clone(), stop.clone());
    state.last_error = None;
    state.session = Some(EnrollmentSession {
        device_path,
        finger: finger.to_owned(),
        baseline,
        started: Instant::now(),
        stage,
        stop,
    });
    BiometricsOutcome::Applied
}

/// Stop the live enrollment.
pub fn cancel_host_enrollment(state: &mut BiometricsHostState) -> BiometricsOutcome {
    let Some(session) = state.session.take() else {
        return BiometricsOutcome::Absent;
    };
    let Ok(connection) = Connection::system() else {
        stop_enroll_listener(&session);
        return BiometricsOutcome::Absent;
    };
    let _ = connection.call_method(
        Some(FPRINT_SERVICE),
        session.device_path.as_str(),
        Some(FPRINT_DEVICE_INTERFACE),
        "EnrollStop",
        &(),
    );
    let _ = connection.call_method(
        Some(FPRINT_SERVICE),
        session.device_path.as_str(),
        Some(FPRINT_DEVICE_INTERFACE),
        "Release",
        &(),
    );
    stop_enroll_listener(&session);
    BiometricsOutcome::Applied
}

/// Delete one enrolled finger on the live host.
pub fn delete_host_finger(finger: &str) -> BiometricsOutcome {
    if !FINGERS.contains(&finger) {
        return BiometricsOutcome::Denied(format!("unknown finger: {finger}"));
    }
    let connection = match Connection::system() {
        Ok(connection) => connection,
        Err(_) => return BiometricsOutcome::Absent,
    };
    if !service_available(&connection, FPRINT_SERVICE) {
        return BiometricsOutcome::Absent;
    }
    let device_path = match connection.call_method(
        Some(FPRINT_SERVICE),
        FPRINT_MANAGER_PATH,
        Some(FPRINT_MANAGER_INTERFACE),
        "GetDefaultDevice",
        &(),
    ) {
        Ok(reply) => reply
            .body()
            .deserialize::<OwnedObjectPath>()
            .map(|path| path.to_string())
            .unwrap_or_default(),
        Err(_) => return BiometricsOutcome::Absent,
    };
    if device_path.is_empty() {
        return BiometricsOutcome::Absent;
    }
    if let Err(error) = connection.call_method(
        Some(FPRINT_SERVICE),
        device_path.as_str(),
        Some(FPRINT_DEVICE_INTERFACE),
        "Claim",
        &("",),
    ) {
        return classify_biometrics(error);
    }
    let result = connection.call_method(
        Some(FPRINT_SERVICE),
        device_path.as_str(),
        Some(FPRINT_DEVICE_INTERFACE),
        "DeleteEnrolledFinger",
        &(finger,),
    );
    let _ = connection.call_method(
        Some(FPRINT_SERVICE),
        device_path.as_str(),
        Some(FPRINT_DEVICE_INTERFACE),
        "Release",
        &(),
    );
    match result {
        Ok(_) => BiometricsOutcome::Applied,
        Err(error) => classify_biometrics(error),
    }
}

/// Change the current user's password through AccountsService.
pub fn change_host_password(password: &str) -> BiometricsOutcome {
    if password.is_empty() {
        return BiometricsOutcome::Denied("the password must not be empty".to_owned());
    }
    let connection = match daemon_connection() {
        Ok(connection) => connection,
        Err(AccountOutcome::Absent) => return BiometricsOutcome::Absent,
        Err(_) => return BiometricsOutcome::Absent,
    };
    let user_name = std::env::var("USER").unwrap_or_default();
    let users = match read_users() {
        Ok(Some(users)) => users,
        Ok(None) => return BiometricsOutcome::Absent,
        Err(error) => return BiometricsOutcome::Failed(error),
    };
    let Some(user) = users.into_iter().find(|user| user.user_name == user_name) else {
        return BiometricsOutcome::Absent;
    };
    match connection.call_method(
        Some(ACCOUNTS_SERVICE),
        user_path(user.uid).as_str(),
        Some(USER_INTERFACE),
        "SetPassword",
        &(password, ""),
    ) {
        Ok(_) => BiometricsOutcome::Applied,
        Err(error) => match classify(error) {
            AccountOutcome::Denied(note) => BiometricsOutcome::Denied(note),
            AccountOutcome::Absent => BiometricsOutcome::Absent,
            AccountOutcome::Failed(error) => BiometricsOutcome::Failed(error),
            AccountOutcome::Applied => BiometricsOutcome::Applied,
        },
    }
}

fn classify_biometrics(error: zbus::Error) -> BiometricsOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    let lower = error.to_string().to_lowercase();
    if name.contains("PermissionDenied")
        || lower.contains("not authorized")
        || lower.contains("polkit")
    {
        BiometricsOutcome::Denied(format!("fprintd: {error}"))
    } else if name.contains("NoSuchDevice") {
        BiometricsOutcome::Absent
    } else {
        BiometricsOutcome::Failed(AdapterError::new(format!("fprintd: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enroll_stages_keep_fprintds_own_vocabulary() {
        assert_eq!(
            enroll_stage_label("enroll-stage-passed"),
            "Fingerprint captured; place the same finger again"
        );
        assert_eq!(
            enroll_stage_label("enroll-completed"),
            "Fingerprint enrolled"
        );
        assert_eq!(enroll_stage_label("enroll-failed"), "Enrollment failed");
        // An unknown future stage is shown verbatim, never disguised.
        assert_eq!(
            enroll_stage_label("enroll-future-stage"),
            "enroll-future-stage"
        );
        // No stage is ever a percentage.
        for stage in [
            "",
            "enroll-stage-passed",
            "enroll-completed",
            "enroll-retry-scan",
            "enroll-failed",
            "enroll-unknown-error",
        ] {
            assert!(!enroll_stage_label(stage).contains('%'));
        }
    }

    #[test]
    fn the_fixture_is_deterministic() {
        assert_eq!(biometrics_fixture(), biometrics_fixture());
    }

    #[test]
    fn an_absent_host_is_unavailable() {
        let mut adapter = BiometricsAdapter::new(MockBiometrics::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn a_present_host_reports_the_reader_and_pam() {
        let mut adapter = BiometricsAdapter::new(MockBiometrics::present());
        adapter.refresh();
        let snapshot = adapter.snapshot().unwrap();
        assert!(snapshot.fprintd_available());
        assert_eq!(snapshot.enrolled().len(), 1);
        assert!(snapshot.pam().available);
        assert!(snapshot.pam().unlock);
        assert!(snapshot.pam().auth);
        assert!(snapshot.password().available);
        assert_eq!(snapshot.label(), "1 Fingerprint");
    }

    #[test]
    fn enrollment_adds_a_finger_and_counts() {
        let mut adapter = BiometricsAdapter::new(MockBiometrics::present());
        adapter.refresh();
        let outcome = adapter.start_enrollment("left-thumb");
        assert!(outcome.is_applied());
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().enrolled().len(), 2);
        assert_eq!(adapter.source().writes(), 1);
        assert_eq!(adapter.source().last_finger(), Some("left-thumb"));
    }

    #[test]
    fn deletion_removes_a_finger() {
        let mut adapter = BiometricsAdapter::new(MockBiometrics::present());
        adapter.refresh();
        let outcome = adapter.delete_finger("right-index-finger");
        assert!(outcome.is_applied());
        adapter.refresh();
        assert!(adapter.snapshot().unwrap().enrolled().is_empty());
    }

    #[test]
    fn a_password_write_is_never_echoed_back() {
        let mut adapter = BiometricsAdapter::new(MockBiometrics::present());
        adapter.refresh();
        let outcome = adapter.change_password("correct horse battery staple");
        assert!(outcome.is_applied());
        assert_eq!(
            adapter.source().last_password(),
            Some("correct horse battery staple")
        );
    }

    #[test]
    fn a_denied_write_keeps_the_state() {
        let mut adapter =
            BiometricsAdapter::new(MockBiometrics::present().deny_writes("cancelled"));
        adapter.refresh();
        let outcome = adapter.start_enrollment("left-thumb");
        assert_eq!(outcome, BiometricsOutcome::Denied("cancelled".to_owned()));
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().enrolled().len(), 1);
    }

    #[test]
    fn finger_names_are_stable() {
        assert_eq!(finger_label("right-index-finger"), "Right index finger");
        assert_eq!(FINGERS.len(), 10);
    }
}
