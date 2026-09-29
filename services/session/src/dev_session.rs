// SPDX-License-Identifier: MIT
//! The real-session dev-harness state file (T-12.6b).
//!
//! The display-manager round trip (`dragonfruit dev --real --round-trip`) must
//! restore the host desktop *exactly* after a clean return **or** a crash. The
//! single source of truth is `$XDG_STATE_HOME/dragonfruit/dev-session.json`,
//! written once when the harness arms the next login and removed when the
//! round trip ends.
//!
//! This module owns the on-disk contract. The dev tool (`tools/dragonfruit-dev`)
//! writes it; the session entry reads it to export
//! `DRAGONFRUIT_DEV_RETURN=<previous session>` into the login environment, which
//! is what makes the shell's **"Quit to \<previous desktop\>"** item appear.
//! Defining the model here — next to [`crate::env`] — keeps the writer and the
//! reader on one schema.
//!
//! A "session-ready" beat is recorded in the state (`session_ready`): autologin
//! is only *committed* once the Dragonfruit session came up, so a crash during
//! startup can never loop the display manager back into Dragonfruit. Recovery
//! is a one-shot restore that also clears the armed state.
//!
//! The contract is frozen in
//! [ADR 0053](../../docs/design/adr/0053-real-session-dev-harness.md) and
//! [ADR 0174](../../docs/design/adr/0174-real-session-round-trip-state.md).

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The environment variable that tells a session it was started by the
/// harness and names the desktop to return to.
pub const DEV_RETURN: &str = "DRAGONFRUIT_DEV_RETURN";

/// The state directory under `$XDG_STATE_HOME`.
pub const STATE_DIR: &str = "dragonfruit";
/// The state file the harness writes and the round trip clears.
pub const STATE_FILE: &str = "dev-session.json";
/// The schema version of the state file.
pub const STATE_VERSION: u32 = 1;
/// The environment variable naming the dev tool that implements `--return`,
/// so a session can offer the return action without the tool on `PATH`.
pub const DEV_BIN: &str = "DRAGONFRUIT_DEV_BIN";

/// The autologin configuration the harness found before it touched anything.
/// Restoring this exact snapshot is what makes a clean round trip
/// byte-identical.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AutologinSnapshot {
    /// Whether autologin was actually armed (an enable flag *and* a user).
    pub enabled: bool,
    /// The user the display manager would have logged in automatically.
    pub user: Option<String>,
    /// The session the display manager would have started, where it records one.
    pub session: Option<String>,
}

/// An exact copy of one display-manager file the harness will edit, captured
/// before the arm so the return can restore the host **byte-for-byte**.
///
/// The per-DM keyfile edits in `session_selector` are minimal but can still
/// leave residue (a key the DM did not have is added, then removed, but the
/// file or a sibling key remains). Snapshotting the bytes sidesteps that: the
/// harness restores the file it actually saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileBackup {
    /// The absolute path that was captured.
    pub path: String,
    /// The exact contents, or `None` when the file did not exist (so the
    /// restore removes it).
    pub contents: Option<String>,
}

impl FileBackup {
    /// Capture `path`, recording its absence as `contents: None`.
    pub fn capture(path: &Path) -> io::Result<Self> {
        let contents = match std::fs::read(path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => Some(text),
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{} is not UTF-8 text", path.display()),
                    ))
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(FileBackup {
            path: path.to_string_lossy().into_owned(),
            contents,
        })
    }

    /// Write the captured contents back verbatim, or remove the file when it
    /// did not exist.
    pub fn restore(&self) -> io::Result<()> {
        match &self.contents {
            Some(contents) => {
                let path = Path::new(&self.path);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(path, contents)
            }
            None => match std::fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            },
        }
    }
}

/// The armed round trip. `previous_session` is what the display manager
/// selected before the harness changed it (`None` = no explicit selection).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevSessionState {
    /// Schema version; a future reader can reject an unknown file.
    pub version: u32,
    /// The display manager the harness edited (`"gdm"`, `"sddm"`, `"lightdm"`),
    /// or empty when detection was unsupported and only the logout happened.
    pub dm: String,
    /// The session the display manager had selected before the arm, to be put
    /// back on return or recovery.
    pub previous_session: Option<String>,
    /// The absolute path of the dev tool that implements the return, exported
    /// to the session as [`DEV_BIN`].
    #[serde(default)]
    pub dev_bin: Option<String>,
    /// Whether the harness was asked to arm autologin for the round trip.
    pub autologin_requested: bool,
    /// Whether autologin was actually committed (the session-ready beat).
    pub autologin_armed: bool,
    /// The autologin configuration before the arm, restored on return.
    pub autologin: AutologinSnapshot,
    /// Exact copies of the display-manager files the arm will edit, for a
    /// byte-identical restore.
    #[serde(default)]
    pub backups: Vec<FileBackup>,
    /// The session-ready beat: true once the Dragonfruit session came up, so a
    /// crash before this can never be mistaken for a completed round trip.
    pub session_ready: bool,
    /// Unix seconds at which the state was written.
    pub timestamp: u64,
}

impl DevSessionState {
    /// A freshly armed round trip. `autologin_armed` starts false: autologin is
    /// committed only by the session-ready beat.
    pub fn armed(
        dm: impl Into<String>,
        previous_session: Option<String>,
        autologin_requested: bool,
        autologin: AutologinSnapshot,
    ) -> Self {
        DevSessionState {
            version: STATE_VERSION,
            dm: dm.into(),
            previous_session,
            dev_bin: None,
            autologin_requested,
            autologin_armed: false,
            autologin,
            backups: Vec::new(),
            session_ready: false,
            timestamp: now_seconds(),
        }
    }

    /// Attach the exact file snapshots captured before the arm.
    pub fn with_backups(mut self, backups: Vec<FileBackup>) -> Self {
        self.backups = backups;
        self
    }

    /// Attach the dev tool's path so the session can offer the return action.
    pub fn with_dev_bin(mut self, path: Option<String>) -> Self {
        self.dev_bin = path;
        self
    }

    /// Write every captured file back byte-for-byte. Returns how many files
    /// were restored.
    pub fn restore_backups(&self) -> io::Result<usize> {
        for backup in &self.backups {
            backup.restore()?;
        }
        Ok(self.backups.len())
    }

    /// The desktop to return to, as exported in [`DEV_RETURN`].
    pub fn dev_return(&self) -> Option<&str> {
        self.previous_session.as_deref()
    }

    /// Whether this file was written by a schema this build understands.
    pub fn is_supported(&self) -> bool {
        self.version == STATE_VERSION
    }
}

/// `$XDG_STATE_HOME/dragonfruit/dev-session.json`, with the XDG default
/// (`~/.local/state`) when `XDG_STATE_HOME` is unset.
pub fn state_path(state_home: &Path) -> PathBuf {
    state_home.join(STATE_DIR).join(STATE_FILE)
}

/// The state directory from the environment, or `None` when neither
/// `XDG_STATE_HOME` nor `HOME` is set.
pub fn state_home_from_env() -> Option<PathBuf> {
    if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(state));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
}

/// Read the armed state, if the file exists. A missing file is `Ok(None)`;
/// malformed JSON is an error so the harness can report (and refuse to act on)
/// a corrupt state rather than guess.
pub fn read(state_home: &Path) -> io::Result<Option<DevSessionState>> {
    read_file(&state_path(state_home))
}

/// Read a state file at an explicit path.
pub fn read_file(path: &Path) -> io::Result<Option<DevSessionState>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let state: DevSessionState = serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not valid dev-session state: {error}", path.display()),
        )
    })?;
    Ok(Some(state))
}

/// Write the state file, creating `dragonfruit/` if needed. The write is atomic
/// (a sibling temp file renamed into place) so a crash mid-write can never leave
/// a half-armed state.
pub fn write(state_home: &Path, state: &DevSessionState) -> io::Result<()> {
    write_file(&state_path(state_home), state)
}

/// Write a state file at an explicit path.
pub fn write_file(path: &Path, state: &DevSessionState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(state)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)
}

/// Remove the state file. A missing file is not an error (return/clear is
/// idempotent, which is what crash recovery needs).
pub fn clear(state_home: &Path) -> io::Result<()> {
    clear_file(&state_path(state_home))
}

/// Remove a state file at an explicit path.
pub fn clear_file(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Unix seconds, saturating to 0 before the epoch.
pub fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "dragonfruit-t126b-{tag}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("scratch");
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn state_round_trips_json_and_clears() {
        let scratch = Scratch::new("round-trip");
        let state = DevSessionState::armed(
            "gdm",
            Some("existing".to_string()),
            true,
            AutologinSnapshot {
                enabled: false,
                user: None,
                session: None,
            },
        );
        write(&scratch.0, &state).expect("write");
        assert!(state_path(&scratch.0).exists());

        let read_back = read(&scratch.0).expect("read").expect("present");
        assert_eq!(read_back, state);
        assert_eq!(read_back.dev_return(), Some("existing"));
        assert!(!read_back.autologin_armed);
        assert!(!read_back.session_ready);
        // The dev tool's reader and the session's reader share one schema; the
        // field the entry script keys on must be a bare JSON string field.
        let text = std::fs::read_to_string(state_path(&scratch.0)).expect("text");
        assert!(
            text.contains("\"previous_session\": \"existing\""),
            "{text}"
        );

        clear(&scratch.0).expect("clear");
        assert_eq!(read(&scratch.0).expect("read"), None);
        // Clearing twice is idempotent.
        clear(&scratch.0).expect("clear again");
    }

    #[test]
    fn a_missing_file_reads_none_and_a_corrupt_file_errors() {
        let scratch = Scratch::new("corrupt");
        assert_eq!(read(&scratch.0).expect("missing is none"), None);
        write(
            &scratch.0,
            &DevSessionState::armed("sddm", None, false, AutologinSnapshot::default()),
        )
        .expect("write");
        let path = state_path(&scratch.0);
        std::fs::write(&path, "{ not json").expect("corrupt");
        assert!(read(&scratch.0).is_err(), "corrupt state is an error");
    }

    #[test]
    fn an_unknown_version_is_not_supported() {
        let scratch = Scratch::new("version");
        let mut state =
            DevSessionState::armed("lightdm", None, false, AutologinSnapshot::default());
        state.version = STATE_VERSION + 1;
        write(&scratch.0, &state).expect("write");
        let read_back = read(&scratch.0).expect("read").expect("present");
        assert!(!read_back.is_supported());
    }
}
