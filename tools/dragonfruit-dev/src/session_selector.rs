// SPDX-License-Identifier: MIT
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
    /// No supported, unambiguous DM was found (or its state was unreadable).
    /// Nothing was written; the caller must let the human pick in the greeter.
    Unsupported,
}

impl Selected {
    /// Whether the host degraded to "pick it in the greeter".
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Selected::Unsupported)
    }

    /// The DM that changed, if any.
    pub fn dm(&self) -> Option<Dm> {
        match self {
            Selected::Changed { dm, .. } => Some(*dm),
            Selected::Unsupported => None,
        }
    }

    /// The session the DM selected before the change, if any.
    pub fn previous(&self) -> Option<&str> {
        match self {
            Selected::Changed { previous, .. } => previous.as_deref(),
            Selected::Unsupported => None,
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
}

impl SessionSelector {
    /// A selector for the real host, reading running DM processes from `/proc`.
    pub fn for_host(user: &str) -> Self {
        SessionSelector {
            paths: DmPaths::host(user),
            running: running_process_names(),
        }
    }

    /// A selector over fixture paths equivalent to `/` and `$HOME`.
    pub fn fixture(root: impl Into<PathBuf>, home: impl Into<PathBuf>, user: &str) -> Self {
        SessionSelector {
            paths: DmPaths::fixture(root, home, user),
            running: Vec::new(),
        }
    }

    /// Seed the running-process list explicitly (tests, or a caller that has
    /// already probed the host).
    pub fn with_running(mut self, names: impl IntoIterator<Item = String>) -> Self {
        self.running = names.into_iter().collect();
        self
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

    /// Point the next login at [`DRAGONFRUIT_SESSION`], returning the previous
    /// selection so the caller can restore it. An unsupported/ambiguous host,
    /// or an unreadable state file, degrades to [`Selected::Unsupported`] with
    /// no write.
    pub fn select_dragonfruit(&self) -> Selected {
        let Some(dm) = self.dm() else {
            return Selected::Unsupported;
        };
        let adapter = self.session_for(dm);
        let previous = match adapter.read() {
            Ok(previous) => previous,
            Err(_) => return Selected::Unsupported,
        };
        match adapter.select(DRAGONFRUIT_SESSION) {
            Ok(()) => Selected::Changed { dm, previous },
            Err(_) => Selected::Unsupported,
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
            Dm::Gdm => Box::new(GdmSession::new(self.paths.gdm_users_file())),
            Dm::Sddm => Box::new(SddmSession::new(self.paths.sddm_state_file())),
            Dm::Lightdm => Box::new(LightdmSession::new(self.paths.lightdm_dmrc())),
        }
    }

    fn autologin_for(&self, dm: Dm) -> Box<dyn DmAutologin> {
        match dm {
            Dm::Gdm => Box::new(GdmAutologin::new(self.paths.gdm_custom_conf())),
            Dm::Sddm => Box::new(SddmAutologin::new(self.paths.sddm_autologin_conf())),
            Dm::Lightdm => Box::new(LightdmAutologin::new(self.paths.lightdm_conf())),
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
/// (`/var/lib/AccountsService/users/<user>`), `[User] XSession=`.
pub struct GdmSession {
    path: PathBuf,
}

impl GdmSession {
    pub fn new(path: PathBuf) -> Self {
        GdmSession { path }
    }
}

impl DmSession for GdmSession {
    fn dm(&self) -> Dm {
        Dm::Gdm
    }

    fn read(&self) -> io::Result<Option<String>> {
        read_key(&self.path, "User", "XSession")
    }

    fn select(&self, session: &str) -> io::Result<()> {
        edit(&self.path, &[("User", "XSession", Some(session))])
    }

    fn restore(&self, previous: Option<&str>) -> io::Result<()> {
        edit(&self.path, &[("User", "XSession", previous)])
    }
}

// --- SDDM: state file -----------------------------------------------------

/// SDDM's selection is the `[LastUser] Session=` in its state file
/// (`/var/lib/sddm/state.conf`).
pub struct SddmSession {
    path: PathBuf,
}

impl SddmSession {
    pub fn new(path: PathBuf) -> Self {
        SddmSession { path }
    }
}

impl DmSession for SddmSession {
    fn dm(&self) -> Dm {
        Dm::Sddm
    }

    fn read(&self) -> io::Result<Option<String>> {
        read_key(&self.path, "LastUser", "Session")
    }

    fn select(&self, session: &str) -> io::Result<()> {
        edit(&self.path, &[("LastUser", "Session", Some(session))])
    }

    fn restore(&self, previous: Option<&str>) -> io::Result<()> {
        edit(&self.path, &[("LastUser", "Session", previous)])
    }
}

// --- LightDM: ~/.dmrc -----------------------------------------------------

/// LightDM's selection is `[Desktop] Session=` in `~/.dmrc`.
pub struct LightdmSession {
    path: PathBuf,
}

impl LightdmSession {
    pub fn new(path: PathBuf) -> Self {
        LightdmSession { path }
    }
}

impl DmSession for LightdmSession {
    fn dm(&self) -> Dm {
        Dm::Lightdm
    }

    fn read(&self) -> io::Result<Option<String>> {
        read_key(&self.path, "Desktop", "Session")
    }

    fn select(&self, session: &str) -> io::Result<()> {
        edit(&self.path, &[("Desktop", "Session", Some(session))])
    }

    fn restore(&self, previous: Option<&str>) -> io::Result<()> {
        edit(&self.path, &[("Desktop", "Session", previous)])
    }
}

// --- GDM autologin --------------------------------------------------------

/// GDM autologin lives in `/etc/gdm/custom.conf` `[daemon]`
/// (`AutomaticLoginEnable`, `AutomaticLogin`). GDM needs root to change it.
pub struct GdmAutologin {
    path: PathBuf,
}

impl GdmAutologin {
    pub fn new(path: PathBuf) -> Self {
        GdmAutologin { path }
    }
}

impl DmAutologin for GdmAutologin {
    fn dm(&self) -> Dm {
        Dm::Gdm
    }

    fn snapshot(&self) -> io::Result<AutologinState> {
        let text = read_optional(&self.path)?;
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
            &self.path,
            &[
                ("daemon", "AutomaticLogin", Some(user)),
                ("daemon", "AutomaticLoginEnable", Some("true")),
            ],
        )
    }

    fn disarm(&self) -> io::Result<()> {
        edit(
            &self.path,
            &[("daemon", "AutomaticLoginEnable", Some("false"))],
        )
    }

    fn restore(&self, saved: &AutologinState) -> io::Result<()> {
        edit(
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
}

impl SddmAutologin {
    pub fn new(path: PathBuf) -> Self {
        SddmAutologin { path }
    }
}

impl DmAutologin for SddmAutologin {
    fn dm(&self) -> Dm {
        Dm::Sddm
    }

    fn snapshot(&self) -> io::Result<AutologinState> {
        let text = read_optional(&self.path)?;
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
            &self.path,
            &[
                ("Autologin", "User", Some(user)),
                ("Autologin", "Session", Some(session)),
            ],
        )
    }

    fn disarm(&self) -> io::Result<()> {
        edit(
            &self.path,
            &[("Autologin", "User", None), ("Autologin", "Session", None)],
        )
    }

    fn restore(&self, saved: &AutologinState) -> io::Result<()> {
        edit(
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
}

impl LightdmAutologin {
    pub fn new(path: PathBuf) -> Self {
        LightdmAutologin { path }
    }
}

impl DmAutologin for LightdmAutologin {
    fn dm(&self) -> Dm {
        Dm::Lightdm
    }

    fn snapshot(&self) -> io::Result<AutologinState> {
        let text = read_optional(&self.path)?;
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
            &self.path,
            &[
                ("Seat:*", "autologin-user", Some(user)),
                ("Seat:*", "autologin-session", Some(session)),
            ],
        )
    }

    fn disarm(&self) -> io::Result<()> {
        edit(
            &self.path,
            &[
                ("Seat:*", "autologin-user", None),
                ("Seat:*", "autologin-session", None),
            ],
        )
    }

    fn restore(&self, saved: &AutologinState) -> io::Result<()> {
        edit(
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

fn read_optional(path: &Path) -> io::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e),
    }
}

fn read_key(path: &Path, section: &str, key: &str) -> io::Result<Option<String>> {
    let text = read_optional(path)?;
    if text.is_empty() {
        return Ok(None);
    }
    Ok(KeyFile::parse(&text).get(section, key))
}

/// Apply `edits` to a keyfile. A missing file is created (with parents) only
/// when something is set; removing a key from a file that does not exist is a
/// no-op, so an unsupported/unarmed host is never touched.
fn edit(path: &Path, edits: &[Edit<'_>]) -> io::Result<()> {
    let existing = read_optional(path)?;
    let existed = path.exists();
    if !existed && edits.iter().all(|(_, _, value)| value.is_none()) {
        return Ok(());
    }
    let mut file = KeyFile::parse(&existing);
    for (section, key, value) in edits {
        match value {
            Some(value) => file.set(section, key, value),
            None => file.remove(section, key),
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, file.to_text())
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
}
