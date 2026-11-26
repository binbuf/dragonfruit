// SPDX-License-Identifier: GPL-3.0-or-later
//! Display-manager session selection and optional autologin (T-12.6a).
//!
//! The real-session round trip (T-12.6b) must point the *next* display-manager
//! login at the Dragonfruit session and then put the previous default back.
//! Preselection is per-DM and mostly user-writable: GDM persists it in the
//! AccountsService user file (`XSession`), SDDM in its state file (`Session`),
//! and LightDM in `~/.dmrc`
//! ([11-session-and-dev-workflow.md](../../docs/design/11-session-and-dev-workflow.md)).
//!
//! This module is the seam. [`SessionSelector`] detects which DM owns the host,
//! builds the matching [`DmSession`] adapter, and edits only that DM's state
//! file. An unrecognized *or ambiguous* host is [`Selected::Unsupported`] and
//! nothing is written, so the caller falls back to "pick it in the greeter"
//! rather than guessing. Autologin is a separate [`DmAutologin`] adapter, off
//! by default and never armed by this module on its own.
//!
//! Every path is derived from an explicit root and home, so unit tests drive
//! real reads and exact-byte writes against fixture directories with no DM
//! installed. On a host the root is `/`.
//!
//! The contract is frozen in
//! [ADR 0053](../../docs/design/adr/0053-real-session-dev-harness.md).

#![allow(dead_code)] // Forward-looking harness seam consumed by T-12.6b.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dragonfruit_session::dev_session::FileBackup;

use crate::dm_access::{AccountsService, DmFiles, ElevatedFiles, LocalFiles};

/// The session id selected for Dragonfruit. It is the `dragonfruit.desktop`
/// basename in `share/wayland-sessions/` — the same value GDM's AccountsService
/// `XSession`, SDDM's state file, and LightDM's `~/.dmrc` each record.
pub const DRAGONFRUIT_SESSION: &str = df_ipc::DESKTOP_NAME;

/// The display managers this seam understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dm {
    Gdm,
    Sddm,
    Lightdm,
}

impl Dm {
    /// Every supported display manager, for detection.
    pub const ALL: [Dm; 3] = [Dm::Gdm, Dm::Sddm, Dm::Lightdm];

    /// The stable id used in logs and tests.
    pub const fn as_str(self) -> &'static str {
        match self {
            Dm::Gdm => "gdm",
            Dm::Sddm => "sddm",
            Dm::Lightdm => "lightdm",
        }
    }
}

impl fmt::Display for Dm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The outcome of pointing the next login at the Dragonfruit session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selected {
    /// A supported DM was found and its selection updated. `previous` is what
    /// it selected before (`None` = the DM had no explicit selection), to be
    /// handed back to [`SessionSelector::restore`] on return.
    Changed { dm: Dm, previous: Option<String> },
    /// No supported, unambiguous DM was found. Nothing was written; the caller
    /// must let the human pick in the greeter.
    Unsupported,
    /// A DM was detected but its selection could not be changed or did not
    /// read back. `reason` carries the host refusal (a denied elevated write,
    /// a missing helper, or a mismatched read-back). The arm must fail loudly
    /// rather than log the user out with the host desktop still selected.
    Failed { dm: Dm, reason: String },
}

impl Selected {
    /// Whether the host degraded to "pick it in the greeter".
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Selected::Unsupported)
    }

    /// Whether a detected DM's selection failed to change.
    pub fn is_failed(&self) -> bool {
        matches!(self, Selected::Failed { .. })
    }

    /// The DM that changed, if any.
    pub fn dm(&self) -> Option<Dm> {
        match self {
            Selected::Changed { dm, .. } => Some(*dm),
            Selected::Unsupported | Selected::Failed { .. } => None,
        }
    }

    /// The session the DM selected before the change, if any.
    pub fn previous(&self) -> Option<&str> {
        match self {
            Selected::Changed { previous, .. } => previous.as_deref(),
            Selected::Unsupported | Selected::Failed { .. } => None,
        }
    }

    /// The one-line marker the harness logs.
    pub fn marker(&self) -> String {
        match self {
            Selected::Changed { dm, previous } => format!(
                "DM session: SELECTED dm={dm} previous={}",
                previous.as_deref().unwrap_or("<none>")
            ),
            Selected::Unsupported => {
                "DM session: UNSUPPORTED — pick the session in the greeter".to_string()
            }
            Selected::Failed { dm, reason } => {
                format!("DM session: FAILED dm={dm} — {reason}; the round trip was not armed")
            }
        }
    }
}

/// Where each display manager keeps its state, under an explicit root. Tests
/// point `root`/`home` at a fixture; on a host `root` is `/`.
#[derive(Debug, Clone)]
pub struct DmPaths {
    root: PathBuf,
    home: PathBuf,
    user: String,
}

impl DmPaths {
    /// The default paths for a normal host.
    pub fn host(user: &str) -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/root"));
        DmPaths {
            root: PathBuf::from("/"),
            home,
            user: user.to_string(),
        }
    }

    /// Paths rooted at a fixture directory instead of `/`.
    pub fn fixture(root: impl Into<PathBuf>, home: impl Into<PathBuf>, user: &str) -> Self {
        DmPaths {
            root: root.into(),
            home: home.into(),
            user: user.to_string(),
        }
    }

    /// The login whose AccountsService user object GDM edits.
    fn user(&self) -> &str {
        &self.user
    }

    /// GDM's AccountsService user file (`[User] XSession=`).
    fn gdm_users_file(&self) -> PathBuf {
        self.root
            .join("var/lib/AccountsService/users")
            .join(&self.user)
    }

    /// GDM's daemon configuration (`AutomaticLogin*`).
    fn gdm_custom_conf(&self) -> PathBuf {
        self.root.join("etc/gdm/custom.conf")
    }

    /// SDDM's state file (`[LastUser] Session=`).
    fn sddm_state_file(&self) -> PathBuf {
        self.root.join("var/lib/sddm/state.conf")
    }

    /// SDDM's autologin drop-in (`[Autologin] User=`/`Session=`).
    fn sddm_autologin_conf(&self) -> PathBuf {
        self.root.join("etc/sddm.conf.d/autologin.conf")
    }

    /// LightDM's user default-session file (`[Desktop] Session=`).
    fn lightdm_dmrc(&self) -> PathBuf {
        self.home.join(".dmrc")
    }

    /// LightDM's seat configuration (`[Seat:*] autologin-user=`).
    fn lightdm_conf(&self) -> PathBuf {
        self.root.join("etc/lightdm/lightdm.conf")
    }
}

/// A per-DM adapter that reads and edits the session a DM starts next.
pub trait DmSession {
    /// Which display manager this adapter edits.
    fn dm(&self) -> Dm;
    /// The session id currently selected, if the DM has an explicit default.
    fn read(&self) -> io::Result<Option<String>>;
    /// Select `session` for the next login.
    fn select(&self, session: &str) -> io::Result<()>;
    /// Put back a selection previously returned by [`read`](Self::read).
    /// `None` removes the explicit selection, returning the DM to its own
    /// default.
    fn restore(&self, previous: Option<&str>) -> io::Result<()>;
}

/// The autologin configuration a DM had before the harness touched it. The
/// harness ships no autologin default: it arms one only for the round trip and
/// force-disables it on return or recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutologinState {
    pub dm: Dm,
    /// Whether autologin was actually armed (an enable flag *and* a user).
    pub enabled: bool,
    /// The user the DM would log in automatically.
    pub user: Option<String>,
    /// The session the DM would start (where the DM records one).
    pub session: Option<String>,
}

impl AutologinState {
    fn off(dm: Dm) -> Self {
        AutologinState {
            dm,
            enabled: false,
            user: None,
            session: None,
        }
    }
}

/// A per-DM adapter that arms and disarms display-manager autologin. Autologin
/// needs root on every supported DM; the harness treats a write failure as a
/// soft error and the round trip falls back to the greeter.
pub trait DmAutologin {
    /// Which display manager this adapter edits.
    fn dm(&self) -> Dm;
    /// Snapshot the current autologin configuration (off by default).
    fn snapshot(&self) -> io::Result<AutologinState>;
    /// Arm autologin for `user` starting `session`.
    fn arm(&self, user: &str, session: &str) -> io::Result<()>;
    /// Force autologin off, whatever the previous state.
    fn disarm(&self) -> io::Result<()>;
    /// Put back a snapshot previously returned by
    /// [`snapshot`](Self::snapshot).
    fn restore(&self, saved: &AutologinState) -> io::Result<()>;
}

/// Detect the host's display manager and edit its session selection.
pub struct SessionSelector {
    paths: DmPaths,
    /// Process names observed running, used to disambiguate a host with more
    /// than one DM installed. Empty in fixtures.
    running: Vec<String>,
    /// The file API every adapter edits. A fixture uses direct access; the
    /// host retries a refused root-owned write through `pkexec`/`sudo`.
    files: Arc<dyn DmFiles>,
    /// GDM's AccountsService client, when the host has `gdbus`.
    accounts: Option<Arc<dyn AccountsService>>,
}

impl SessionSelector {
    /// A selector for the real host, reading running DM processes from `/proc`
    /// and elevating only the display-manager files.
    pub fn for_host(user: &str) -> Self {
        let accounts = crate::dm_access::GdbusAccounts::detect()
            .map(|accounts| Arc::new(accounts) as Arc<dyn AccountsService>);
        SessionSelector {
            paths: DmPaths::host(user),
            running: running_process_names(),
            files: Arc::new(ElevatedFiles::host()),
            accounts,
        }
    }

    /// A selector over fixture paths equivalent to `/` and `$HOME`.
    pub fn fixture(root: impl Into<PathBuf>, home: impl Into<PathBuf>, user: &str) -> Self {
        Self::fixture_with_files(root, home, user, Arc::new(LocalFiles))
    }

    /// A fixture selector with an explicit file API (a fake DM or an elevated
    /// backend). Tests use this to simulate a root-owned state file.
    pub fn fixture_with_files(
        root: impl Into<PathBuf>,
        home: impl Into<PathBuf>,
        user: &str,
        files: Arc<dyn DmFiles>,
    ) -> Self {
        SessionSelector {
            paths: DmPaths::fixture(root, home, user),
            running: Vec::new(),
            files,
            accounts: None,
        }
    }

    /// Seed the running-process list explicitly (tests, or a caller that has
    /// already probed the host).
    pub fn with_running(mut self, names: impl IntoIterator<Item = String>) -> Self {
        self.running = names.into_iter().collect();
        self
    }

    /// Attach an AccountsService client (tests, or a caller that already has
    /// one).
    pub fn with_accounts(mut self, accounts: Arc<dyn AccountsService>) -> Self {
        self.accounts = Some(accounts);
        self
    }

    /// The file API the adapters use.
    pub fn files(&self) -> &dyn DmFiles {
        self.files.as_ref()
    }

    /// The DM that owns this host, if exactly one is installed (or one is
    /// running). `None` is "unsupported or ambiguous — do not guess".
    pub fn dm(&self) -> Option<Dm> {
        if let Some(dm) = self.running_dm() {
            return Some(dm);
        }
        let installed: Vec<Dm> = Dm::ALL
            .into_iter()
            .filter(|dm| self.dm_installed(*dm))
            .collect();
        match installed.as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// The session adapter for the detected DM, if any.
    pub fn session(&self) -> Option<Box<dyn DmSession>> {
        self.dm().map(|dm| self.session_for(dm))
    }

    /// The autologin adapter for the detected DM, if any.
    pub fn autologin(&self) -> Option<Box<dyn DmAutologin>> {
        self.dm().map(|dm| self.autologin_for(dm))
    }

    /// The file the detected DM's session selection lives in, if any. The
    /// T-12.6b round trip snapshots it for a byte-identical restore.
    pub fn session_file(&self) -> Option<PathBuf> {
        self.dm().map(|dm| match dm {
            Dm::Gdm => self.paths.gdm_users_file(),
            Dm::Sddm => self.paths.sddm_state_file(),
            Dm::Lightdm => self.paths.lightdm_dmrc(),
        })
    }

    /// The file the detected DM's autologin configuration lives in, if any.
    /// The T-12.6b round trip snapshots it for a byte-identical restore.
    pub fn autologin_file(&self) -> Option<PathBuf> {
        self.dm().map(|dm| match dm {
            Dm::Gdm => self.paths.gdm_custom_conf(),
            Dm::Sddm => self.paths.sddm_autologin_conf(),
            Dm::Lightdm => self.paths.lightdm_conf(),
        })
    }

    /// Point the next login at [`DRAGONFRUIT_SESSION`], returning the previous
    /// selection so the caller can restore it. An unsupported/ambiguous host
    /// degrades to [`Selected::Unsupported`] with no write. A detected DM whose
    /// write is refused, or whose selection does not read back, is
    /// [`Selected::Failed`] — never a silent `Unsupported` (T344).
    pub fn select_dragonfruit(&self) -> Selected {
        let Some(dm) = self.dm() else {
            return Selected::Unsupported;
        };
        let adapter = self.session_for(dm);
        let previous = match adapter.read() {
            Ok(previous) => previous,
            Err(error) => {
                return Selected::Failed {
                    dm,
                    reason: format!("could not read the current selection: {error}"),
                }
            }
        };
        if let Err(error) = adapter.select(DRAGONFRUIT_SESSION) {
            return Selected::Failed {
                dm,
                reason: error.to_string(),
            };
        }
        // Read it back: a write that reported success but did not change the
        // DM's own selection is the silent failure this task exists to remove.
        match adapter.read() {
            Ok(Some(current)) if current == DRAGONFRUIT_SESSION => {
                Selected::Changed { dm, previous }
            }
            Ok(current) => Selected::Failed {
                dm,
                reason: format!(
                    "the selection did not stick (read back {})",
                    current.as_deref().unwrap_or("<none>")
                ),
            },
            Err(error) => Selected::Failed {
                dm,
                reason: format!("could not verify the selection: {error}"),
            },
        }
    }

    /// Restore a previous session selection on the detected DM. `Ok(false)`
    /// when no supported DM is present (nothing to restore).
    pub fn restore(&self, previous: Option<&str>) -> io::Result<bool> {
        let Some(dm) = self.dm() else {
            return Ok(false);
        };
        self.session_for(dm).restore(previous)?;
        Ok(true)
    }

    /// Capture one display-manager file exactly as the arm will find it. A
    /// root-owned file is read through the same elevation the write uses.
    pub fn capture_snapshot(&self, path: &Path) -> io::Result<FileBackup> {
        Ok(FileBackup {
            path: path.to_string_lossy().into_owned(),
            contents: self.files.read(path)?,
        })
    }

    /// Write a captured file back byte-for-byte (or remove it when it did not
    /// exist), through the same elevation the arm used.
    pub fn restore_snapshot(&self, path: &Path, contents: Option<&str>) -> io::Result<()> {
        match contents {
            Some(text) => self.files.write(path, text),
            None => self.files.remove(path),
        }
    }

    /// Snapshot the detected DM's autologin configuration. `None` when no
    /// supported DM is present.
    pub fn autologin_state(&self) -> io::Result<Option<AutologinState>> {
        match self.dm() {
            Some(dm) => self.autologin_for(dm).snapshot().map(Some),
            None => Ok(None),
        }
    }

    /// Arm autologin for `user`/`session`, returning the pre-arm snapshot so
    /// the caller can restore it. `None` when no supported DM is present.
    pub fn arm_autologin(&self, user: &str, session: &str) -> io::Result<Option<AutologinState>> {
        let Some(dm) = self.dm() else {
            return Ok(None);
        };
        let adapter = self.autologin_for(dm);
        let saved = adapter.snapshot()?;
        adapter.arm(user, session)?;
        Ok(Some(saved))
    }

    /// Force autologin off on the detected DM. `Ok(false)` when no supported
    /// DM is present.
    pub fn disarm_autologin(&self) -> io::Result<bool> {
        let Some(dm) = self.dm() else {
            return Ok(false);
        };
        self.autologin_for(dm).disarm()?;
        Ok(true)
    }

    /// Put back an autologin snapshot. `Ok(false)` when no supported DM is
    /// present.
    pub fn restore_autologin(&self, saved: &AutologinState) -> io::Result<bool> {
        let Some(dm) = self.dm() else {
            return Ok(false);
        };
        self.autologin_for(dm).restore(saved)?;
        Ok(true)
    }

    fn session_for(&self, dm: Dm) -> Box<dyn DmSession> {
        match dm {
            Dm::Gdm => Box::new(GdmSession::new(
                self.paths.gdm_users_file(),
                self.paths.user(),
                Arc::clone(&self.files),
                self.accounts.clone(),
            )),
            Dm::Sddm => Box::new(SddmSession::new(
                self.paths.sddm_state_file(),
                Arc::clone(&self.files),
            )),
            Dm::Lightdm => Box::new(LightdmSession::new(
                self.paths.lightdm_dmrc(),
                Arc::clone(&self.files),
            )),
        }
    }

    fn autologin_for(&self, dm: Dm) -> Box<dyn DmAutologin> {
        match dm {
            Dm::Gdm => Box::new(GdmAutologin::new(
                self.paths.gdm_custom_conf(),
                Arc::clone(&self.files),
            )),
            Dm::Sddm => Box::new(SddmAutologin::new(
                self.paths.sddm_autologin_conf(),
                Arc::clone(&self.files),
            )),
            Dm::Lightdm => Box::new(LightdmAutologin::new(
                self.paths.lightdm_conf(),
                Arc::clone(&self.files),
            )),
        }
    }

    fn dm_installed(&self, dm: Dm) -> bool {
        match dm {
            Dm::Gdm => {
                self.paths.root.join("etc/gdm/custom.conf").exists()
                    || self.paths.root.join("etc/gdm3/custom.conf").exists()
            }
            Dm::Sddm => {
                self.paths.root.join("etc/sddm.conf").exists()
                    || self.paths.root.join("etc/sddm.conf.d").is_dir()
            }
            Dm::Lightdm => {
                self.paths.root.join("etc/lightdm/lightdm.conf").exists()
                    || self.paths.root.join("etc/lightdm").is_dir()
            }
        }
    }

    fn running_dm(&self) -> Option<Dm> {
        for name in &self.running {
            let name = name.to_ascii_lowercase();
            if name.contains("gdm") {
                return Some(Dm::Gdm);
            }
            if name.contains("sddm") {
                return Some(Dm::Sddm);
            }
            if name.contains("lightdm") {
                return Some(Dm::Lightdm);
            }
        }
        None
    }
}

/// Running process names from `/proc/*/comm`. Best-effort: a missing entry is
/// skipped. Used only to disambiguate a host with several DMs installed.
fn running_process_names() -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return names;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !pid.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
            names.push(comm.trim_end().to_string());
        }
    }
    names
}

// --- GDM: AccountsService user file --------------------------------------

/// GDM's selection lives in the AccountsService user file
/// (`/var/lib/AccountsService/users/<user>`), `[User] XSession=`. GDM prefers
/// the documented `SetXSession` D-Bus method; when `gdbus` is missing or the
/// call is refused, the same value is written to the file through elevation.
pub struct GdmSession {
    path: PathBuf,
    user: String,
    files: Arc<dyn DmFiles>,
    accounts: Option<Arc<dyn AccountsService>>,
}

impl GdmSession {
    pub fn new(
        path: PathBuf,
        user: &str,
        files: Arc<dyn DmFiles>,
        accounts: Option<Arc<dyn AccountsService>>,
    ) -> Self {
        GdmSession {
            path,
            user: user.to_string(),
            files,
            accounts,
        }
    }

    fn file_read(&self) -> io::Result<Option<String>> {
        read_key(self.files.as_ref(), &self.path, "User", "XSession")
    }

    /// Try AccountsService first. `None` means "not usable; use the file".
    fn select_via_accounts(&self, session: &str) -> Option<io::Result<()>> {
        let accounts = self.accounts.as_ref()?;
        if !accounts.available() {
            return None;
        }
        Some((|| {
            accounts.set_xsession(&self.user, session)?;
            let current = accounts.xsession(&self.user)?;
            if current.as_deref() == Some(session) {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "AccountsService did not apply XSession={session}"
                )))
            }
        })())
    }
}

impl DmSession for GdmSession {
    fn dm(&self) -> Dm {
        Dm::Gdm
    }

    fn read(&self) -> io::Result<Option<String>> {
        match self.file_read() {
            Ok(value) => Ok(value),
            Err(error) if crate::dm_access::denied(&error) => {
                if let Some(accounts) = &self.accounts {
                    if accounts.available() {
                        return accounts.xsession(&self.user);
                    }
                }
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    fn select(&self, session: &str) -> io::Result<()> {
        if let Some(result) = self.select_via_accounts(session) {
            if result.is_ok() {
                return Ok(());
            }
            // A refused or stale AccountsService call falls back to the file:
            // the D-Bus service is optional, the state file is authoritative.
        }
        edit(
            self.files.as_ref(),
            &self.path,
            &[("User", "XSession", Some(session))],
        )
    }

    fn restore(&self, previous: Option<&str>) -> io::Result<()> {
        if let Some(previous) = previous {
            if let Some(result) = self.select_via_accounts(previous) {
                if result.is_ok() {
                    return Ok(());
                }
            }
        }
        edit(
            self.files.as_ref(),
            &self.path,
            &[("User", "XSession", previous)],
        )
    }
}

// --- SDDM: state file -----------------------------------------------------

/// SDDM's selection is the `[LastUser] Session=` in its state file
/// (`/var/lib/sddm/state.conf`).
pub struct SddmSession {
    path: PathBuf,
    files: Arc<dyn DmFiles>,
}

impl SddmSession {
    pub fn new(path: PathBuf, files: Arc<dyn DmFiles>) -> Self {
        SddmSession { path, files }
    }
}

impl DmSession for SddmSession {
    fn dm(&self) -> Dm {
        Dm::Sddm
    }

    fn read(&self) -> io::Result<Option<String>> {
        read_key(self.files.as_ref(), &self.path, "LastUser", "Session")
    }

    fn select(&self, session: &str) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[("LastUser", "Session", Some(session))],
        )
    }

    fn restore(&self, previous: Option<&str>) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[("LastUser", "Session", previous)],
        )
    }
}

// --- LightDM: ~/.dmrc -----------------------------------------------------

/// LightDM's selection is `[Desktop] Session=` in `~/.dmrc`.
pub struct LightdmSession {
    path: PathBuf,
    files: Arc<dyn DmFiles>,
}

impl LightdmSession {
    pub fn new(path: PathBuf, files: Arc<dyn DmFiles>) -> Self {
        LightdmSession { path, files }
    }
}

impl DmSession for LightdmSession {
    fn dm(&self) -> Dm {
        Dm::Lightdm
    }

    fn read(&self) -> io::Result<Option<String>> {
        read_key(self.files.as_ref(), &self.path, "Desktop", "Session")
    }

    fn select(&self, session: &str) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[("Desktop", "Session", Some(session))],
        )
    }

    fn restore(&self, previous: Option<&str>) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[("Desktop", "Session", previous)],
        )
    }
}

// --- GDM autologin --------------------------------------------------------

/// GDM autologin lives in `/etc/gdm/custom.conf` `[daemon]`
/// (`AutomaticLoginEnable`, `AutomaticLogin`). GDM needs root to change it.
pub struct GdmAutologin {
    path: PathBuf,
    files: Arc<dyn DmFiles>,
}

impl GdmAutologin {
    pub fn new(path: PathBuf, files: Arc<dyn DmFiles>) -> Self {
        GdmAutologin { path, files }
    }
}

impl DmAutologin for GdmAutologin {
    fn dm(&self) -> Dm {
        Dm::Gdm
    }

    fn snapshot(&self) -> io::Result<AutologinState> {
        let text = self.files.read(&self.path)?.unwrap_or_default();
        let file = KeyFile::parse(&text);
        let user = file
            .get("daemon", "AutomaticLogin")
            .filter(|s| !s.is_empty());
        let enabled = file
            .get("daemon", "AutomaticLoginEnable")
            .is_some_and(|v| truthy(&v))
            && user.is_some();
        Ok(AutologinState {
            dm: Dm::Gdm,
            enabled,
            user,
            session: None,
        })
    }

    fn arm(&self, user: &str, _session: &str) -> io::Result<()> {
        // GDM picks the session from the AccountsService `XSession` selection,
        // so only the user and the enable flag are written here.
        edit(
            self.files.as_ref(),
            &self.path,
            &[
                ("daemon", "AutomaticLogin", Some(user)),
                ("daemon", "AutomaticLoginEnable", Some("true")),
            ],
        )
    }

    fn disarm(&self) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[("daemon", "AutomaticLoginEnable", Some("false"))],
        )
    }

    fn restore(&self, saved: &AutologinState) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[
                ("daemon", "AutomaticLogin", saved.user.as_deref()),
                (
                    "daemon",
                    "AutomaticLoginEnable",
                    Some(if saved.enabled { "true" } else { "false" }),
                ),
            ],
        )
    }
}

// --- SDDM autologin -------------------------------------------------------

/// SDDM autologin lives in an `/etc/sddm.conf.d/autologin.conf` drop-in
/// (`[Autologin] User=`/`Session=`). Needs root.
pub struct SddmAutologin {
    path: PathBuf,
    files: Arc<dyn DmFiles>,
}

impl SddmAutologin {
    pub fn new(path: PathBuf, files: Arc<dyn DmFiles>) -> Self {
        SddmAutologin { path, files }
    }
}

impl DmAutologin for SddmAutologin {
    fn dm(&self) -> Dm {
        Dm::Sddm
    }

    fn snapshot(&self) -> io::Result<AutologinState> {
        let text = self.files.read(&self.path)?.unwrap_or_default();
        let file = KeyFile::parse(&text);
        let user = file.get("Autologin", "User").filter(|s| !s.is_empty());
        let session = file.get("Autologin", "Session").filter(|s| !s.is_empty());
        Ok(AutologinState {
            dm: Dm::Sddm,
            enabled: user.is_some(),
            user,
            session,
        })
    }

    fn arm(&self, user: &str, session: &str) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[
                ("Autologin", "User", Some(user)),
                ("Autologin", "Session", Some(session)),
            ],
        )
    }

    fn disarm(&self) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[("Autologin", "User", None), ("Autologin", "Session", None)],
        )
    }

    fn restore(&self, saved: &AutologinState) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[
                ("Autologin", "User", saved.user.as_deref()),
                ("Autologin", "Session", saved.session.as_deref()),
            ],
        )
    }
}

// --- LightDM autologin ----------------------------------------------------

/// LightDM autologin lives in `/etc/lightdm/lightdm.conf` `[Seat:*]`
/// (`autologin-user`, `autologin-session`). Needs root.
pub struct LightdmAutologin {
    path: PathBuf,
    files: Arc<dyn DmFiles>,
}

impl LightdmAutologin {
    pub fn new(path: PathBuf, files: Arc<dyn DmFiles>) -> Self {
        LightdmAutologin { path, files }
    }
}

impl DmAutologin for LightdmAutologin {
    fn dm(&self) -> Dm {
        Dm::Lightdm
    }

    fn snapshot(&self) -> io::Result<AutologinState> {
        let text = self.files.read(&self.path)?.unwrap_or_default();
        let file = KeyFile::parse(&text);
        let user = file
            .get("Seat:*", "autologin-user")
            .filter(|s| !s.is_empty());
        let session = file
            .get("Seat:*", "autologin-session")
            .filter(|s| !s.is_empty());
        Ok(AutologinState {
            dm: Dm::Lightdm,
            enabled: user.is_some(),
            user,
            session,
        })
    }

    fn arm(&self, user: &str, session: &str) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[
                ("Seat:*", "autologin-user", Some(user)),
                ("Seat:*", "autologin-session", Some(session)),
            ],
        )
    }

    fn disarm(&self) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[
                ("Seat:*", "autologin-user", None),
                ("Seat:*", "autologin-session", None),
            ],
        )
    }

    fn restore(&self, saved: &AutologinState) -> io::Result<()> {
        edit(
            self.files.as_ref(),
            &self.path,
            &[
                ("Seat:*", "autologin-user", saved.user.as_deref()),
                ("Seat:*", "autologin-session", saved.session.as_deref()),
            ],
        )
    }
}

// --- keyfile helpers ------------------------------------------------------

/// One `(section, key, value)` edit; `None` value removes the key.
type Edit<'a> = (&'a str, &'a str, Option<&'a str>);

fn read_key(
    files: &dyn DmFiles,
    path: &Path,
    section: &str,
    key: &str,
) -> io::Result<Option<String>> {
    let Some(text) = files.read(path)? else {
        return Ok(None);
    };
    if text.is_empty() {
        return Ok(None);
    }
    Ok(KeyFile::parse(&text).get(section, key))
}

/// Apply `edits` to a keyfile. A missing file is created only when something
/// is set; removing a key from a file that does not exist is a no-op, so an
/// unsupported/unarmed host is never touched.
fn edit(files: &dyn DmFiles, path: &Path, edits: &[Edit<'_>]) -> io::Result<()> {
    let existing = files.read(path)?;
    let existed = existing.is_some();
    if !existed && edits.iter().all(|(_, _, value)| value.is_none()) {
        return Ok(());
    }
    let mut file = KeyFile::parse(existing.as_deref().unwrap_or(""));
    for (section, key, value) in edits {
        match value {
            Some(value) => file.set(section, key, value),
            None => file.remove(section, key),
        }
    }
    files.write(path, &file.to_text())
}

/// Whether a GDM/boolean config value means "on".
fn truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "1" | "on"
    )
}

/// A tiny INI/keyfile editor. It keeps every unrelated line, so a write
/// touches only the target `section`/`key` and the file round-trips
/// byte-for-byte when nothing needs to change.
#[derive(Debug, Clone)]
struct KeyFile {
    lines: Vec<String>,
}

impl KeyFile {
    fn parse(text: &str) -> Self {
        let body = text.strip_suffix('\n').unwrap_or(text);
        let lines = if body.is_empty() {
            Vec::new()
        } else {
            body.split('\n').map(str::to_string).collect()
        };
        KeyFile { lines }
    }

    fn section_of(line: &str) -> Option<&str> {
        let trimmed = line.trim();
        let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?;
        Some(inner.trim())
    }

    fn assignment_of(line: &str) -> Option<(&str, &str)> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            return None;
        }
        let (key, value) = trimmed.split_once('=')?;
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        Some((key, value.trim()))
    }

    fn get(&self, section: &str, key: &str) -> Option<String> {
        let mut current: Option<&str> = None;
        for line in &self.lines {
            if let Some(header) = Self::section_of(line) {
                current = Some(header);
                continue;
            }
            if current == Some(section) {
                if let Some((candidate, value)) = Self::assignment_of(line) {
                    if candidate == key {
                        return Some(value.to_string());
                    }
                }
            }
        }
        None
    }

    fn find_key(&self, section: &str, key: &str) -> Option<usize> {
        let mut current: Option<&str> = None;
        for (i, line) in self.lines.iter().enumerate() {
            if let Some(header) = Self::section_of(line) {
                current = Some(header);
                continue;
            }
            if current == Some(section) {
                if let Some((candidate, _)) = Self::assignment_of(line) {
                    if candidate == key {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    fn section_header(&self, section: &str) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| Self::section_of(line) == Some(section))
    }

    fn set(&mut self, section: &str, key: &str, value: &str) {
        if let Some(i) = self.find_key(section, key) {
            self.lines[i] = format!("{key}={value}");
            return;
        }
        match self.section_header(section) {
            Some(header) => {
                // Append at the end of the section, before the next header.
                let mut end = header + 1;
                while end < self.lines.len() && Self::section_of(&self.lines[end]).is_none() {
                    end += 1;
                }
                self.lines.insert(end, format!("{key}={value}"));
            }
            None => {
                self.lines.push(format!("[{section}]"));
                self.lines.push(format!("{key}={value}"));
            }
        }
    }

    fn remove(&mut self, section: &str, key: &str) {
        if let Some(i) = self.find_key(section, key) {
            self.lines.remove(i);
        }
    }

    fn to_text(&self) -> String {
        if self.lines.is_empty() {
            return String::new();
        }
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "dragonfruit-t126a-{tag}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create scratch");
            Scratch(path)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn home(&self) -> PathBuf {
            self.0.join("home/dfdev")
        }

        fn write(&self, relative: &str, contents: &str) -> PathBuf {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(&path, contents).expect("write fixture");
            path
        }

        fn read(&self, relative: &str) -> String {
            std::fs::read_to_string(self.0.join(relative)).expect("read fixture")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn selector(scratch: &Scratch) -> SessionSelector {
        SessionSelector::fixture(scratch.root(), scratch.home(), "dfdev")
    }

    #[test]
    fn keyfile_keeps_unrelated_lines_and_replaces_only_the_target_key() {
        let text = "# accounts service\n[User]\nLanguage=en_US\nXSession=existing\n";
        let mut file = KeyFile::parse(text);
        file.set("User", "XSession", "dragonfruit");
        assert_eq!(
            file.to_text(),
            "# accounts service\n[User]\nLanguage=en_US\nXSession=dragonfruit\n"
        );
        assert_eq!(file.get("User", "Language").as_deref(), Some("en_US"));
    }

    #[test]
    fn keyfile_appends_a_missing_section_and_removes_a_key() {
        let mut file = KeyFile::parse("");
        file.set("Desktop", "Session", "dragonfruit");
        assert_eq!(file.to_text(), "[Desktop]\nSession=dragonfruit\n");
        file.remove("Desktop", "Session");
        assert_eq!(file.to_text(), "[Desktop]\n");
    }

    #[test]
    fn gdm_reads_selects_and_restores() {
        let scratch = Scratch::new("gdm");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        scratch.write(
            "var/lib/AccountsService/users/dfdev",
            "[User]\nLanguage=en_US\nXSession=existing\n",
        );
        let selector = selector(&scratch);

        let selected = selector.select_dragonfruit();
        assert_eq!(
            selected,
            Selected::Changed {
                dm: Dm::Gdm,
                previous: Some("existing".to_string())
            }
        );
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nLanguage=en_US\nXSession=dragonfruit\n"
        );

        assert!(selector.restore(Some("existing")).expect("restore"));
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nLanguage=en_US\nXSession=existing\n"
        );
    }

    #[test]
    fn sddm_reads_selects_and_restores() {
        let scratch = Scratch::new("sddm");
        scratch.write("etc/sddm.conf", "[General]\n");
        scratch.write(
            "var/lib/sddm/state.conf",
            "[LastUser]\nUser=dfdev\nSession=other\n",
        );
        let selector = selector(&scratch);

        let selected = selector.select_dragonfruit();
        assert_eq!(
            selected,
            Selected::Changed {
                dm: Dm::Sddm,
                previous: Some("other".to_string())
            }
        );
        assert_eq!(
            scratch.read("var/lib/sddm/state.conf"),
            "[LastUser]\nUser=dfdev\nSession=dragonfruit\n"
        );

        assert!(selector.restore(Some("other")).expect("restore"));
        assert_eq!(
            scratch.read("var/lib/sddm/state.conf"),
            "[LastUser]\nUser=dfdev\nSession=other\n"
        );
    }

    #[test]
    fn lightdm_reads_selects_and_restores() {
        let scratch = Scratch::new("lightdm");
        scratch.write("etc/lightdm/lightdm.conf", "[Seat:*]\n");
        scratch.write("home/dfdev/.dmrc", "[Desktop]\nSession=existing\n");
        let selector = selector(&scratch);

        let selected = selector.select_dragonfruit();
        assert_eq!(
            selected,
            Selected::Changed {
                dm: Dm::Lightdm,
                previous: Some("existing".to_string())
            }
        );
        assert_eq!(
            scratch.read("home/dfdev/.dmrc"),
            "[Desktop]\nSession=dragonfruit\n"
        );

        assert!(selector.restore(Some("existing")).expect("restore"));
        assert_eq!(
            scratch.read("home/dfdev/.dmrc"),
            "[Desktop]\nSession=existing\n"
        );
    }

    #[test]
    fn a_missing_selection_reads_none_and_restore_removes_it() {
        let scratch = Scratch::new("gdm-none");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        let path = scratch.write(
            "var/lib/AccountsService/users/dfdev",
            "[User]\nLanguage=en\n",
        );
        let selector = selector(&scratch);

        let selected = selector.select_dragonfruit();
        assert_eq!(
            selected,
            Selected::Changed {
                dm: Dm::Gdm,
                previous: None
            }
        );
        assert!(selector.restore(None).expect("restore"));
        assert!(!std::fs::read_to_string(&path)
            .expect("read")
            .contains("XSession"));
    }

    #[test]
    fn gdm_selection_creates_the_accounts_file_and_parents() {
        let scratch = Scratch::new("gdm-create");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        let selector = selector(&scratch);
        assert_eq!(selector.select_dragonfruit().dm(), Some(Dm::Gdm));
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nXSession=dragonfruit\n"
        );
    }

    #[test]
    fn unknown_dm_is_unsupported_and_writes_nothing() {
        let scratch = Scratch::new("unknown");
        let selector = selector(&scratch);
        let selected = selector.select_dragonfruit();
        assert!(selected.is_unsupported());
        assert_eq!(selected, Selected::Unsupported);
        assert!(!scratch.root().join("var/lib/AccountsService").exists());
        assert!(!scratch.root().join("etc").exists());
        assert!(!scratch.home().join(".dmrc").exists());
        assert!(selected.marker().contains("UNSUPPORTED"));
    }

    #[test]
    fn several_installed_dms_without_a_running_one_are_unsupported() {
        let scratch = Scratch::new("ambiguous");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        scratch.write("etc/sddm.conf.d/autologin.conf", "[Autologin]\n");
        let selector = selector(&scratch);
        assert_eq!(selector.dm(), None);
        assert!(selector.select_dragonfruit().is_unsupported());
    }

    #[test]
    fn a_running_dm_disambiguates_several_installed_dms() {
        let scratch = Scratch::new("running");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        scratch.write("var/lib/sddm/state.conf", "[LastUser]\nSession=other\n");
        let selector = selector(&scratch).with_running(["sddm".to_string()]);
        assert_eq!(selector.dm(), Some(Dm::Sddm));
        assert_eq!(selector.select_dragonfruit().dm(), Some(Dm::Sddm));
    }

    #[test]
    fn gdm_autologin_defaults_off_and_round_trips() {
        let scratch = Scratch::new("gdm-auto");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        let selector = selector(&scratch);

        let before = selector.autologin_state().expect("state").expect("dm");
        assert!(!before.enabled);

        let saved = selector
            .arm_autologin("dfdev", DRAGONFRUIT_SESSION)
            .expect("arm")
            .expect("dm");
        assert!(!saved.enabled);
        let armed = selector.autologin_state().expect("state").expect("dm");
        assert!(armed.enabled);
        assert_eq!(armed.user.as_deref(), Some("dfdev"));
        assert_eq!(
            scratch.read("etc/gdm/custom.conf"),
            "[daemon]\nAutomaticLogin=dfdev\nAutomaticLoginEnable=true\n"
        );

        assert!(selector.disarm_autologin().expect("disarm"));
        let disarmed = selector.autologin_state().expect("state").expect("dm");
        assert!(!disarmed.enabled);
    }

    #[test]
    fn autologin_restores_a_pre_existing_arm() {
        let scratch = Scratch::new("gdm-auto-restore");
        scratch.write(
            "etc/gdm/custom.conf",
            "[daemon]\nAutomaticLogin=alice\nAutomaticLoginEnable=true\n",
        );
        let selector = selector(&scratch);
        let before = selector.autologin_state().expect("state").expect("dm");
        assert!(before.enabled);
        assert_eq!(before.user.as_deref(), Some("alice"));

        let saved = selector
            .arm_autologin("dfdev", DRAGONFRUIT_SESSION)
            .expect("arm")
            .expect("dm");
        assert_eq!(saved.user.as_deref(), Some("alice"));

        assert!(selector.restore_autologin(&saved).expect("restore"));
        let after = selector.autologin_state().expect("state").expect("dm");
        assert!(after.enabled);
        assert_eq!(after.user.as_deref(), Some("alice"));
    }

    #[test]
    fn sddm_autologin_defaults_off_and_round_trips() {
        let scratch = Scratch::new("sddm-auto");
        scratch.write("etc/sddm.conf.d/autologin.conf", "[Autologin]\n");
        let selector = selector(&scratch);
        assert!(
            !selector
                .autologin_state()
                .expect("state")
                .expect("dm")
                .enabled
        );

        selector
            .arm_autologin("dfdev", DRAGONFRUIT_SESSION)
            .expect("arm");
        let armed = selector.autologin_state().expect("state").expect("dm");
        assert!(armed.enabled);
        assert_eq!(armed.user.as_deref(), Some("dfdev"));
        assert_eq!(armed.session.as_deref(), Some(DRAGONFRUIT_SESSION));
        assert_eq!(
            scratch.read("etc/sddm.conf.d/autologin.conf"),
            "[Autologin]\nUser=dfdev\nSession=dragonfruit\n"
        );

        assert!(selector.disarm_autologin().expect("disarm"));
        assert!(
            !selector
                .autologin_state()
                .expect("state")
                .expect("dm")
                .enabled
        );
    }

    #[test]
    fn lightdm_autologin_defaults_off_and_round_trips() {
        let scratch = Scratch::new("lightdm-auto");
        scratch.write(
            "etc/lightdm/lightdm.conf",
            "[Seat:*]\ngreeter-session=lightdm-gtk-greeter\n",
        );
        let selector = selector(&scratch);
        assert!(
            !selector
                .autologin_state()
                .expect("state")
                .expect("dm")
                .enabled
        );

        selector
            .arm_autologin("dfdev", DRAGONFRUIT_SESSION)
            .expect("arm");
        let armed = selector.autologin_state().expect("state").expect("dm");
        assert!(armed.enabled);
        assert_eq!(armed.user.as_deref(), Some("dfdev"));
        assert_eq!(
            scratch.read("etc/lightdm/lightdm.conf"),
            "[Seat:*]\ngreeter-session=lightdm-gtk-greeter\nautologin-user=dfdev\nautologin-session=dragonfruit\n"
        );

        assert!(selector.disarm_autologin().expect("disarm"));
        let disarmed = selector.autologin_state().expect("state").expect("dm");
        assert!(!disarmed.enabled);
        assert!(!scratch
            .read("etc/lightdm/lightdm.conf")
            .contains("autologin-user"));
    }

    #[test]
    fn autologin_without_a_dm_is_a_no_op() {
        let scratch = Scratch::new("auto-none");
        let selector = selector(&scratch);
        assert_eq!(selector.autologin_state().expect("state"), None);
        assert_eq!(
            selector
                .arm_autologin("dfdev", DRAGONFRUIT_SESSION)
                .expect("arm"),
            None
        );
        assert!(!selector.disarm_autologin().expect("disarm"));
        assert!(!scratch.root().join("etc").exists());
    }

    #[test]
    fn selected_bytes_are_exact_for_each_dm() {
        // Live-check evidence: the exact bytes each adapter writes when
        // selecting the Dragonfruit session. Run with `--nocapture` to cat.
        let cases: [(&str, &str, &str, &str); 3] = [
            (
                "gdm",
                "etc/gdm/custom.conf",
                "var/lib/AccountsService/users/dfdev",
                "[User]\nLanguage=en_US\nXSession=existing\n",
            ),
            (
                "sddm",
                "etc/sddm.conf",
                "var/lib/sddm/state.conf",
                "[LastUser]\nUser=dfdev\nSession=other\n",
            ),
            (
                "lightdm",
                "etc/lightdm/lightdm.conf",
                "home/dfdev/.dmrc",
                "[Desktop]\nSession=existing\n",
            ),
        ];
        for (tag, marker, state, seed) in cases {
            let scratch = Scratch::new(tag);
            scratch.write(marker, "[Section]\n");
            scratch.write(state, seed);
            let selector = selector(&scratch);
            assert_eq!(
                selector.select_dragonfruit().dm().map(Dm::as_str),
                Some(tag)
            );
            println!("T-12.6a {tag} selected state:\n{}", scratch.read(state));
        }
    }

    #[test]
    fn marker_names_the_selected_dm_and_previous_session() {
        let selected = Selected::Changed {
            dm: Dm::Lightdm,
            previous: Some("existing".to_string()),
        };
        assert_eq!(
            selected.marker(),
            "DM session: SELECTED dm=lightdm previous=existing"
        );
    }

    // --- T344: root-owned DM files and a loud permission failure ----------

    use crate::dm_access::{DmFiles, ElevatedFiles, Privilege};

    /// A direct backend that refuses writes, as a root-owned DM file does.
    struct DeniedFiles;

    impl DmFiles for DeniedFiles {
        fn read(&self, path: &Path) -> io::Result<Option<String>> {
            LocalFiles.read(path)
        }

        fn write(&self, path: &Path, _text: &str) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{} is root-owned", path.display()),
            ))
        }

        fn remove(&self, path: &Path) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{} is root-owned", path.display()),
            ))
        }
    }

    /// A fake `pkexec`: it performs the write and records every call.
    struct FakePrivilege {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl FakePrivilege {
        fn new() -> Self {
            FakePrivilege {
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls").clone()
        }
    }

    impl Privilege for FakePrivilege {
        fn available(&self) -> bool {
            true
        }

        fn write(&self, path: &Path, text: &str) -> io::Result<()> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("write {}", path.display()));
            LocalFiles.write(path, text)
        }

        fn read(&self, path: &Path) -> io::Result<Option<String>> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("read {}", path.display()));
            LocalFiles.read(path)
        }

        fn remove(&self, path: &Path) -> io::Result<()> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("remove {}", path.display()));
            LocalFiles.remove(path)
        }
    }

    /// A fake DM: a scratch fixture with a root-owned accounts file and a
    /// fake privilege helper.
    fn fake_root_owned_gdm(scratch: &Scratch) -> (SessionSelector, Arc<FakePrivilege>) {
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        scratch.write(
            "var/lib/AccountsService/users/dfdev",
            "[User]\nLanguage=en_US\nXSession=existing\n",
        );
        let privilege = Arc::new(FakePrivilege::new());
        let files: Arc<dyn DmFiles> = Arc::new(ElevatedFiles::new(
            Box::new(DeniedFiles),
            Box::new(FakePrivilegeProxy(Arc::clone(&privilege))),
        ));
        let selector =
            SessionSelector::fixture_with_files(scratch.root(), scratch.home(), "dfdev", files);
        (selector, privilege)
    }

    /// A `Privilege` that forwards to a shared `FakePrivilege` (the trait is
    /// consumed by `ElevatedFiles`, while the test keeps the call log).
    struct FakePrivilegeProxy(Arc<FakePrivilege>);

    impl Privilege for FakePrivilegeProxy {
        fn available(&self) -> bool {
            self.0.available()
        }
        fn write(&self, path: &Path, text: &str) -> io::Result<()> {
            self.0.write(path, text)
        }
        fn read(&self, path: &Path) -> io::Result<Option<String>> {
            self.0.read(path)
        }
        fn remove(&self, path: &Path) -> io::Result<()> {
            self.0.remove(path)
        }
    }

    #[test]
    fn a_fake_root_owned_dm_write_and_restore_round_trips_through_elevation() {
        let scratch = Scratch::new("elevated-gdm");
        let (selector, privilege) = fake_root_owned_gdm(&scratch);

        let selected = selector.select_dragonfruit();
        assert_eq!(
            selected,
            Selected::Changed {
                dm: Dm::Gdm,
                previous: Some("existing".to_string())
            }
        );
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nLanguage=en_US\nXSession=dragonfruit\n"
        );
        assert!(
            privilege
                .calls()
                .iter()
                .any(|call| call.starts_with("write ") && call.contains("AccountsService")),
            "the arm elevated the root-owned write: {:?}",
            privilege.calls()
        );

        assert!(selector.restore(Some("existing")).expect("restore"));
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nLanguage=en_US\nXSession=existing\n"
        );
    }

    #[test]
    fn a_denied_root_owned_write_fails_the_selection_loudly() {
        let scratch = Scratch::new("denied-gdm");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        scratch.write(
            "var/lib/AccountsService/users/dfdev",
            "[User]\nXSession=existing\n",
        );
        let selector = SessionSelector::fixture_with_files(
            scratch.root(),
            scratch.home(),
            "dfdev",
            Arc::new(DeniedFiles),
        );

        let selected = selector.select_dragonfruit();
        assert!(selected.is_failed(), "{selected:?}");
        match &selected {
            Selected::Failed { dm, reason } => {
                assert_eq!(*dm, Dm::Gdm);
                assert!(reason.contains("root-owned"), "{reason}");
            }
            other => panic!("expected a failure, got {other:?}"),
        }
        assert!(selected.marker().contains("FAILED"));
        // Nothing was written; the caller must not log out.
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nXSession=existing\n"
        );
    }

    #[test]
    fn a_write_that_does_not_read_back_is_a_failure() {
        /// A backend that accepts the write but never changes the file.
        struct DiscardWrites;

        impl DmFiles for DiscardWrites {
            fn read(&self, path: &Path) -> io::Result<Option<String>> {
                LocalFiles.read(path)
            }
            fn write(&self, _path: &Path, _text: &str) -> io::Result<()> {
                Ok(())
            }
            fn remove(&self, _path: &Path) -> io::Result<()> {
                Ok(())
            }
        }

        let scratch = Scratch::new("no-read-back");
        scratch.write("etc/sddm.conf", "[General]\n");
        scratch.write(
            "var/lib/sddm/state.conf",
            "[LastUser]\nUser=dfdev\nSession=other\n",
        );
        let selector = SessionSelector::fixture_with_files(
            scratch.root(),
            scratch.home(),
            "dfdev",
            Arc::new(DiscardWrites),
        );
        let selected = selector.select_dragonfruit();
        assert!(selected.is_failed(), "{selected:?}");
        match selected {
            Selected::Failed { dm, reason } => {
                assert_eq!(dm, Dm::Sddm);
                assert!(reason.contains("did not stick"), "{reason}");
            }
            other => panic!("expected a read-back failure, got {other:?}"),
        }
    }

    #[test]
    fn gdm_prefers_accountsservice_and_verifies_the_new_session() {
        struct FakeAccounts {
            session: std::sync::Mutex<Option<String>>,
            set_calls: std::sync::Mutex<Vec<String>>,
            path: PathBuf,
        }

        impl crate::dm_access::AccountsService for FakeAccounts {
            fn available(&self) -> bool {
                true
            }

            fn xsession(&self, _user: &str) -> io::Result<Option<String>> {
                Ok(self.session.lock().expect("session").clone())
            }

            fn set_xsession(&self, _user: &str, session: &str) -> io::Result<()> {
                self.set_calls
                    .lock()
                    .expect("calls")
                    .push(session.to_string());
                *self.session.lock().expect("session") =
                    (!session.is_empty()).then(|| session.to_string());
                // AccountsService owns the file; mirror that in the fixture so
                // the read-back sees the daemon's write.
                let text = format!("[User]\nXSession={session}\n");
                std::fs::write(&self.path, text)
            }
        }

        let scratch = Scratch::new("accounts-service");
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        let path = scratch.write(
            "var/lib/AccountsService/users/dfdev",
            "[User]\nXSession=existing\n",
        );
        let accounts = Arc::new(FakeAccounts {
            session: std::sync::Mutex::new(Some("existing".to_string())),
            set_calls: std::sync::Mutex::new(Vec::new()),
            path,
        });
        let selector = selector(&scratch).with_accounts(accounts.clone());

        assert_eq!(
            selector.select_dragonfruit(),
            Selected::Changed {
                dm: Dm::Gdm,
                previous: Some("existing".to_string())
            }
        );
        assert_eq!(
            accounts.set_calls.lock().expect("calls").as_slice(),
            ["dragonfruit"]
        );
    }
}
