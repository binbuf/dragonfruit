// SPDX-License-Identifier: MIT
//! The real-session display-manager round trip (T-12.6b).
//!
//! `dragonfruit dev --real --round-trip` closes the host session, logs into
//! Dragonfruit as a primary session, and restores the host desktop exactly as
//! it was — from the **"Quit to \<previous desktop\>"** menu item or, after a
//! crash, from the next login's one-shot recovery. There is no live compositor
//! handoff ([ADR 0053](../../docs/design/adr/0053-real-session-dev-harness.md));
//! every path crosses a logout/login boundary.
//!
//! The single source of truth is the state file
//! `$XDG_STATE_HOME/dragonfruit/dev-session.json`
//! ([`dragonfruit_session::dev_session`]). [`RoundTrip`] arms it, commits the
//! autologin only after the **session-ready beat**, and restores the previous
//! session (and the previous autologin) on return or recovery. The DM edits
//! themselves are the T-12.6a [`SessionSelector`] seam; the state file also
//! carries exact byte snapshots of the files the arm will edit, so a clean
//! round trip is byte-identical on every supported display manager.
//!
//! The contract is frozen in
//! [ADR 0174](../../docs/design/adr/0174-real-session-round-trip-state.md).

use std::path::{Path, PathBuf};

use dragonfruit_session::dev_session::{self, AutologinSnapshot, DevSessionState, FileBackup};

use crate::session_selector::{AutologinState, Selected, SessionSelector, DRAGONFRUIT_SESSION};

/// The outcome of arming the round trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArmOutcome {
    /// The state file was written and (when a supported DM was found) the next
    /// login was pointed at Dragonfruit. `selected` is `Unsupported` when the
    /// host degraded to "pick it in the greeter"; the state file still exists
    /// so the return path clears it.
    Armed {
        selected: Selected,
        autologin_requested: bool,
        state: DevSessionState,
    },
}

/// What [`RoundTrip::restore`] put back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreOutcome {
    /// The DM that was restored, if any.
    pub dm: Option<String>,
    /// The session the DM had selected before the arm.
    pub previous_session: Option<String>,
    /// How many display-manager files were written back.
    pub restored_files: usize,
    /// Whether a state file existed and was cleared.
    pub cleared: bool,
}

impl RestoreOutcome {
    /// The one-line marker the harness logs.
    pub fn marker(&self) -> String {
        if !self.cleared {
            return "Round trip: NOTHING — no armed dev session".to_string();
        }
        format!(
            "Round trip: RESTORED dm={} session={} files={}",
            self.dm.as_deref().unwrap_or("<none>"),
            self.previous_session.as_deref().unwrap_or("<none>"),
            self.restored_files,
        )
    }
}

/// The three ways to ask the host session manager to log out. All of them ask
/// the session manager; none `SIGKILL`s the host compositor, so unsaved-work
/// prompts fire normally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutPlan {
    /// Ask the session manager through the interactive session-quit tool
    /// (`gnome-session-quit --logout`). // df-allow-desktop-name
    SessionQuit,
    /// Ask the session manager through its D-Bus logout method
    /// (`KSMServerInterface.logout`). // df-allow-desktop-name
    KsmServer,
    /// The DM-agnostic fallback: `loginctl terminate-session`.
    Logind,
}

impl LogoutPlan {
    /// Pick a logout mechanism from the host desktop name and a `have(program)`
    /// probe. Unknown desktops fall back to logind.
    pub fn detect(host_desktop: &str, have: impl Fn(&str) -> bool) -> LogoutPlan {
        let desktop = host_desktop.to_ascii_lowercase();
        let is_gtk = desktop.contains("gnome"); // df-allow-desktop-name
        let quit_tool = "gnome-session-quit"; // df-allow-desktop-name
        if is_gtk && have(quit_tool) {
            return LogoutPlan::SessionQuit;
        }
        let is_qt = desktop.contains("kde") || desktop.contains("plasma"); // df-allow-desktop-name
        if is_qt && (have("qdbus6") || have("qdbus")) {
            return LogoutPlan::KsmServer;
        }
        LogoutPlan::Logind
    }

    /// The command to run. `session_id` is `$XDG_SESSION_ID`; `user` is the
    /// fallback when it is unset (the logind path then terminates the user's
    /// sessions).
    pub fn command(self, session_id: Option<&str>, user: &str) -> (String, Vec<String>) {
        match self {
            LogoutPlan::SessionQuit => (
                "gnome-session-quit".to_string(), // df-allow-desktop-name
                vec!["--logout".to_string()],
            ),
            LogoutPlan::KsmServer => {
                // `logout <confirm> <sdtype> <sdmode>`; all zero is "ask the
                // session manager", which raises the normal prompts.
                let qdbus = if which("qdbus6") { "qdbus6" } else { "qdbus" };
                (
                    qdbus.to_string(),
                    vec![
                        "org.kde.ksmserver".to_string(), // df-allow-desktop-name
                        "/KSMServer".to_string(),
                        "org.kde.KSMServerInterface.logout".to_string(), // df-allow-desktop-name
                        "0".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                    ],
                )
            }
            LogoutPlan::Logind => match session_id {
                Some(id) => (
                    "loginctl".to_string(),
                    vec!["terminate-session".to_string(), id.to_string()],
                ),
                None => (
                    "loginctl".to_string(),
                    vec!["terminate-user".to_string(), user.to_string()],
                ),
            },
        }
    }

    /// The one-line marker the harness logs.
    pub fn marker(self) -> &'static str {
        match self {
            LogoutPlan::SessionQuit => "Logout: gnome-session-quit --logout", // df-allow-desktop-name
            LogoutPlan::KsmServer => "Logout: KSMserver D-Bus logout",
            LogoutPlan::Logind => "Logout: loginctl fallback",
        }
    }
}

/// Whether `program` is on `PATH` (best effort).
fn which(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
        .unwrap_or(false)
}

/// The round trip over one host: a state file, the T-12.6a DM seam, and the
/// user the state belongs to.
pub struct RoundTrip<'a> {
    selector: &'a SessionSelector,
    state_home: PathBuf,
    user: String,
}

impl<'a> RoundTrip<'a> {
    /// A round trip writing its state under `state_home` (`$XDG_STATE_HOME`).
    pub fn new(selector: &'a SessionSelector, state_home: impl Into<PathBuf>, user: &str) -> Self {
        RoundTrip {
            selector,
            state_home: state_home.into(),
            user: user.to_string(),
        }
    }

    /// The armed state, if any. A corrupt file is an error (never guess).
    pub fn state(&self) -> std::io::Result<Option<DevSessionState>> {
        dev_session::read(&self.state_home)
    }

    /// Arm the round trip: snapshot the files the arm will edit and the
    /// autologin configuration, write the state file, and point the next login
    /// at Dragonfruit. Autologin is *not* committed here — [`commit_ready`]
    /// does that once the session came up, so a startup crash cannot loop the
    /// DM back in.
    ///
    /// [`commit_ready`]: RoundTrip::commit_ready
    pub fn arm(&self, autologin_requested: bool) -> std::io::Result<ArmOutcome> {
        let backups = self.capture_backups()?;
        let selected = self.selector.select_dragonfruit();
        let dm = self.selector.dm().map(|dm| dm.as_str()).unwrap_or("");
        let autologin = self.snapshot(autologin_requested);
        let state = DevSessionState::armed(
            dm,
            selected.previous().map(str::to_owned),
            autologin_requested,
            autologin,
        )
        .with_backups(backups)
        .with_dev_bin(
            std::env::current_exe()
                .ok()
                .map(|path| path.to_string_lossy().into_owned()),
        );
        dev_session::write(&self.state_home, &state)?;
        Ok(ArmOutcome::Armed {
            selected,
            autologin_requested,
            state,
        })
    }

    /// The **session-ready beat**: the Dragonfruit session is up, so commit the
    /// requested autologin and mark the state ready. Returns the updated state,
    /// or `None` when nothing is armed.
    pub fn commit_ready(&self) -> std::io::Result<Option<DevSessionState>> {
        let Some(mut state) = self.state()? else {
            return Ok(None);
        };
        if state.autologin_requested && !state.autologin_armed {
            // `arm_autologin` returns the pre-arm snapshot; the state file
            // already carries it, so success is "it was armed at all".
            let armed = self
                .selector
                .arm_autologin(&self.user, DRAGONFRUIT_SESSION)?
                .is_some();
            state.autologin_armed = armed;
        }
        state.session_ready = true;
        state.timestamp = dev_session::now_seconds();
        dev_session::write(&self.state_home, &state)?;
        Ok(Some(state))
    }

    /// Restore the previous desktop and clear the armed state. This is both the
    /// clean return (the menu item) and the one-shot crash recovery; it is
    /// idempotent, so a recovery after a clean return is a no-op.
    pub fn restore(&self) -> std::io::Result<RestoreOutcome> {
        let Some(state) = self.state()? else {
            return Ok(RestoreOutcome {
                dm: None,
                previous_session: None,
                restored_files: 0,
                cleared: false,
            });
        };

        // The exact snapshots are authoritative: they are what makes the round
        // trip byte-identical even when a DM file gained or lost a key.
        let mut restored_files = state.restore_backups()?;
        if restored_files == 0 {
            // Fallback for a state written without backups (an older schema):
            // use the T-12.6a semantic restore.
            self.selector.restore(state.previous_session.as_deref())?;
            restored_files = 1;
            if state.autologin_armed {
                if let Some(dm) = self.selector.dm() {
                    let saved = AutologinState {
                        dm,
                        enabled: state.autologin.enabled,
                        user: state.autologin.user.clone(),
                        session: state.autologin.session.clone(),
                    };
                    self.selector.restore_autologin(&saved)?;
                    restored_files += 1;
                }
            }
        }

        dev_session::clear(&self.state_home)?;
        Ok(RestoreOutcome {
            dm: (!state.dm.is_empty()).then_some(state.dm.clone()),
            previous_session: state.previous_session,
            restored_files,
            cleared: true,
        })
    }

    /// Capture the session-selection and autologin files the arm may edit, in
    /// that order, skipping duplicates.
    fn capture_backups(&self) -> std::io::Result<Vec<FileBackup>> {
        let mut backups = Vec::new();
        for path in [self.selector.session_file(), self.selector.autologin_file()]
            .into_iter()
            .flatten()
        {
            if backups
                .iter()
                .any(|backup: &FileBackup| backup.path == path.to_string_lossy())
            {
                continue;
            }
            backups.push(FileBackup::capture(&path)?);
        }
        Ok(backups)
    }

    /// The autologin snapshot to restore later. A host without a supported DM
    /// snapshots as "off", and the restore is a no-op.
    fn snapshot(&self, requested: bool) -> AutologinSnapshot {
        if !requested {
            return AutologinSnapshot::default();
        }
        self.selector
            .autologin_state()
            .ok()
            .flatten()
            .map(|state| AutologinSnapshot {
                enabled: state.enabled,
                user: state.user,
                session: state.session,
            })
            .unwrap_or_default()
    }
}

/// The state file path for a given `$XDG_STATE_HOME`.
pub fn state_path(state_home: &Path) -> PathBuf {
    dev_session::state_path(state_home)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_selector::Dm;

    /// A unique scratch directory, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "dragonfruit-t126b-{tag}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create scratch");
            Scratch(path)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn state_home(&self) -> PathBuf {
            self.0.join("state")
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

    /// A GDM fixture with a previous selection and autologin off.
    fn gdm_fixture(tag: &str) -> Scratch {
        let scratch = Scratch::new(tag);
        scratch.write("etc/gdm/custom.conf", "[daemon]\n");
        scratch.write(
            "var/lib/AccountsService/users/dfdev",
            "[User]\nLanguage=en_US\nXSession=existing\n",
        );
        scratch
    }

    fn round_trip<'a>(scratch: &Scratch, selector: &'a SessionSelector) -> RoundTrip<'a> {
        RoundTrip::new(selector, scratch.state_home(), "dfdev")
    }

    #[test]
    fn arm_writes_the_state_and_selects_dragonfruit() {
        let scratch = gdm_fixture("arm");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);

        let ArmOutcome::Armed {
            selected, state, ..
        } = trip.arm(false).expect("arm");
        assert_eq!(selected.dm(), Some(Dm::Gdm));
        assert_eq!(selected.previous(), Some("existing"));
        assert_eq!(state.previous_session.as_deref(), Some("existing"));
        assert_eq!(state.dm, "gdm");
        assert!(!state.autologin_armed, "autologin waits for the ready beat");
        assert!(!state.session_ready);
        // The state file is the only writer; it exists at the contract path.
        assert!(state_path(&scratch.state_home()).exists());
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nLanguage=en_US\nXSession=dragonfruit\n"
        );
    }

    #[test]
    fn a_clean_round_trip_restores_and_clears() {
        let scratch = gdm_fixture("clean");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);

        trip.arm(true).expect("arm");
        // The ready beat commits the requested autologin.
        let ready = trip.commit_ready().expect("ready").expect("armed");
        assert!(ready.autologin_armed);
        assert!(ready.session_ready);
        assert!(scratch
            .read("etc/gdm/custom.conf")
            .contains("AutomaticLoginEnable=true"));

        let outcome = trip.restore().expect("restore");
        assert!(outcome.cleared);
        assert_eq!(outcome.previous_session.as_deref(), Some("existing"));
        assert!(outcome.restored_files >= 2);
        // The previous selection is back and the state file is gone.
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nLanguage=en_US\nXSession=existing\n"
        );
        assert!(!state_path(&scratch.state_home()).exists());
    }

    #[test]
    fn a_simulated_crash_leaves_the_state_armed_and_recovery_restores_it() {
        let scratch = gdm_fixture("crash");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);

        trip.arm(true).expect("arm");
        // Commit autologin, then "crash": the process dies with the state file
        // still armed. Recovery is the same restore path.
        trip.commit_ready().expect("ready");
        assert!(state_path(&scratch.state_home()).exists());
        assert!(scratch
            .read("etc/gdm/custom.conf")
            .contains("AutomaticLoginEnable=true"));

        let outcome = trip.restore().expect("recover");
        assert!(outcome.cleared);
        // Autologin is back to off and the previous session is back.
        let config = scratch.read("etc/gdm/custom.conf");
        assert_eq!(config, "[daemon]\n", "the exact pre-arm bytes");
        assert_eq!(
            scratch.read("var/lib/AccountsService/users/dfdev"),
            "[User]\nLanguage=en_US\nXSession=existing\n"
        );
        assert!(!state_path(&scratch.state_home()).exists());
    }

    #[test]
    fn a_crash_before_the_ready_beat_never_touched_autologin() {
        let scratch = gdm_fixture("crash-early");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);

        trip.arm(true).expect("arm");
        // No ready beat: the crash happened while starting up. Autologin was
        // never committed, so the config is exactly the pre-arm bytes.
        assert_eq!(
            scratch.read("etc/gdm/custom.conf"),
            "[daemon]\n",
            "the ready beat is the only autologin writer"
        );
        let outcome = trip.restore().expect("recover");
        assert!(outcome.cleared);
        assert_eq!(scratch.read("etc/gdm/custom.conf"), "[daemon]\n");
    }

    #[test]
    fn a_clean_round_trip_is_byte_identical_for_every_dm() {
        // GDM: AccountsService file + custom.conf autologin.
        let gdm = gdm_fixture("bytes-gdm");
        run_byte_identical(
            &gdm,
            &["var/lib/AccountsService/users/dfdev", "etc/gdm/custom.conf"],
        );

        // SDDM: state file + autologin drop-in, already armed for another user.
        let sddm = Scratch::new("bytes-sddm");
        sddm.write("etc/sddm.conf", "[General]\n");
        sddm.write(
            "var/lib/sddm/state.conf",
            "[LastUser]\nUser=dfdev\nSession=other\n",
        );
        sddm.write(
            "etc/sddm.conf.d/autologin.conf",
            "[Autologin]\nUser=alice\nSession=existing\n",
        );
        run_byte_identical(
            &sddm,
            &["var/lib/sddm/state.conf", "etc/sddm.conf.d/autologin.conf"],
        );

        // LightDM: ~/.dmrc + lightdm.conf.
        let lightdm = Scratch::new("bytes-lightdm");
        lightdm.write("etc/lightdm/lightdm.conf", "[Seat:*]\n");
        lightdm.write("home/dfdev/.dmrc", "[Desktop]\nSession=existing\n");
        run_byte_identical(&lightdm, &["home/dfdev/.dmrc", "etc/lightdm/lightdm.conf"]);
    }

    /// Arm with autologin, commit the ready beat, restore, and assert every
    /// listed file is byte-for-byte what it started as.
    fn run_byte_identical(scratch: &Scratch, files: &[&str]) {
        let before: Vec<String> = files.iter().map(|f| scratch.read(f)).collect();
        let selector = selector(scratch);
        let trip = round_trip(scratch, &selector);
        trip.arm(true).expect("arm");
        trip.commit_ready().expect("ready");
        trip.restore().expect("restore");
        for (file, want) in files.iter().zip(before) {
            assert_eq!(scratch.read(file), want, "{file} is byte-identical");
        }
    }

    #[test]
    fn a_clean_round_trip_restores_a_file_that_did_not_exist() {
        // The SDDM autologin drop-in does not exist yet: arming creates it and
        // the restore removes it again.
        let scratch = Scratch::new("bytes-absent");
        scratch.write("etc/sddm.conf", "[General]\n");
        scratch.write(
            "var/lib/sddm/state.conf",
            "[LastUser]\nUser=dfdev\nSession=other\n",
        );
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);
        let drop_in = scratch.root().join("etc/sddm.conf.d/autologin.conf");
        assert!(!drop_in.exists());

        trip.arm(true).expect("arm");
        trip.commit_ready().expect("ready");
        assert!(drop_in.exists(), "the ready beat armed autologin");
        trip.restore().expect("restore");
        assert!(!drop_in.exists(), "the absent file is absent again");
    }

    #[test]
    fn an_unsupported_host_still_arms_and_clears() {
        let scratch = Scratch::new("unsupported");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);

        let ArmOutcome::Armed {
            selected, state, ..
        } = trip.arm(false).expect("arm");
        assert!(selected.is_unsupported());
        assert_eq!(state.previous_session, None);
        // The state file exists so the menu/return path still clears it.
        assert!(state_path(&scratch.state_home()).exists());
        assert!(!scratch.root().join("etc").exists(), "nothing was written");

        let outcome = trip.restore().expect("restore");
        assert!(outcome.cleared);
        assert!(!state_path(&scratch.state_home()).exists());
    }

    #[test]
    fn restore_without_armed_state_is_a_no_op() {
        let scratch = gdm_fixture("nothing");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);
        let outcome = trip.restore().expect("restore");
        assert!(!outcome.cleared);
        assert!(outcome.marker().contains("NOTHING"));
    }

    #[test]
    fn commit_ready_without_state_is_a_no_op() {
        let scratch = gdm_fixture("ready-none");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);
        assert_eq!(trip.commit_ready().expect("ready"), None);
    }

    #[test]
    fn an_arm_without_autologin_leaves_the_autologin_file_alone() {
        let scratch = gdm_fixture("no-auto");
        let selector = selector(&scratch);
        let trip = round_trip(&scratch, &selector);
        trip.arm(false).expect("arm");
        trip.commit_ready().expect("ready");
        assert_eq!(
            scratch.read("etc/gdm/custom.conf"),
            "[daemon]\n",
            "requested=false never arms"
        );
    }

    #[test]
    fn logout_plan_prefers_the_host_session_manager() {
        let nothing = |_: &str| false;
        let everything = |_: &str| true;
        let gtk = LogoutPlan::detect("GNOME", everything); // df-allow-desktop-name
        assert_eq!(gtk, LogoutPlan::SessionQuit);
        let qt = LogoutPlan::detect("KDE", everything); // df-allow-desktop-name
        assert_eq!(qt, LogoutPlan::KsmServer);
        let wl = LogoutPlan::detect("sway", everything); // df-allow-desktop-name
        assert_eq!(wl, LogoutPlan::Logind);
        // A known desktop without its tool falls back to logind.
        let gtk_absent = LogoutPlan::detect("GNOME", nothing); // df-allow-desktop-name
        assert_eq!(gtk_absent, LogoutPlan::Logind);
    }

    #[test]
    fn logout_commands_ask_the_session_manager() {
        let (program, args) = LogoutPlan::SessionQuit.command(Some("3"), "dfdev");
        assert_eq!(program, "gnome-session-quit"); // df-allow-desktop-name
        assert_eq!(args, vec!["--logout"]);
        assert!(!args.iter().any(|a| a.contains("no-prompt")));

        let (program, args) = LogoutPlan::KsmServer.command(Some("3"), "dfdev");
        assert!(program.starts_with("qdbus"));
        assert!(args.iter().any(|a| a.contains("KSMServerInterface.logout")));

        let (program, args) = LogoutPlan::Logind.command(Some("3"), "dfdev");
        assert_eq!(program, "loginctl");
        assert_eq!(args, vec!["terminate-session", "3"]);
        // No session id: log out the user's sessions instead.
        let (_, args) = LogoutPlan::Logind.command(None, "dfdev");
        assert_eq!(args, vec!["terminate-user", "dfdev"]);
    }
}
