// SPDX-License-Identifier: MIT
//! The second-VT dev harness (T-12.6c).
//!
//! `dragonfruit dev --real [--user dfdev]` starts a real Dragonfruit session on
//! a **free VT for a dedicated user**, leaving the host desktop on its own VT
//! untouched ([11-session-and-dev-workflow.md](../../docs/design/11-session-and-dev-workflow.md#mode-a--second-vt-no-logout-the-daily-mode)).
//! Returning is a VT switch; teardown is `loginctl terminate-session`.
//!
//! The decisions here are pure and unit-tested so the hardware half is cheap to
//! validate:
//!
//! * [`preflight`] refuses a target user that already holds a seat graphical
//!   session — a second graphical session for the same user collides through
//!   shared user-session state, so a dedicated user is required.
//! * [`select_free_vt`] picks the smallest free VT from a `loginctl` snapshot,
//!   never assuming VT1 (a host may run the desktop on any VT).
//! * [`SecondVtPlan`] builds the exact `systemd-run`/`chvt`/`loginctl` commands,
//!   with no side effects.
//!
//! [`Logind`] runs `loginctl`; its command is injectable so tests drive a mock.
//! The contract is frozen in
//! [ADR 0175](../../docs/design/adr/0175-second-vt-dev-harness.md).

use std::io;
use std::path::PathBuf;
use std::process::Command;

/// The dedicated development user the second-VT mode defaults to.
pub const DEFAULT_DEV_USER: &str = "dfdev";

/// The lowest VT the harness will use. VT1 is not assumed to be the host's
/// desktop, but VTs below 2 are conventionally reserved; a real host desktop
/// reports its VT and it is excluded as occupied.
pub const FIRST_FREE_VT: u32 = 2;
/// The highest VT the harness probes.
pub const LAST_VT: u32 = 12;

/// The socket the shipped session units use (the `WAYLAND_DISPLAY` value).
pub const SESSION_SOCKET: &str = dragonfruit_session::env::DEFAULT_SOCKET_NAME;

/// The transient systemd unit name for the second-VT session.
pub const UNIT: &str = "dragonfruit-second-vt";

/// One logind session, as `loginctl show-session <id> -p …` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogindSession {
    /// The session id (`loginctl list-sessions`).
    pub id: String,
    /// `Name=` — the Unix user (not the UID).
    pub user: String,
    /// `Seat=` — empty for a non-seat session such as a user manager.
    pub seat: String,
    /// `VTNr=` — `0`/absent when the session has no VT.
    pub vtnr: Option<u32>,
    /// `Type=` — `wayland`, `x11`, `tty`, `unspecified`, …
    pub session_type: String,
    /// `Class=` — `user`, `greeter`, `manager`, …
    pub class: String,
    /// `Active=yes|no`.
    pub active: bool,
    /// `State=` — `active`, `online`, `closing`, …
    pub state: String,
}

impl LogindSession {
    /// Whether this session already renders graphics on a seat: active, on a
    /// seat, a user session, of a graphical type. This is what a second
    /// graphical session for the same user would collide with.
    pub fn is_seat_graphical(&self) -> bool {
        self.active
            && self.class == "user"
            && !self.seat.is_empty()
            && matches!(self.session_type.as_str(), "wayland" | "x11")
    }

    /// The session's VT, if it has one on a seat.
    pub fn vt(&self) -> Option<u32> {
        self.vtnr.filter(|vt| *vt > 0)
    }
}

/// Parse `loginctl list-sessions --no-legend`: one session id per line, first
/// whitespace-separated token.
pub fn parse_session_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let Some(id) = line.split_whitespace().next() else {
            continue;
        };
        if !id.is_empty() && id != "SESSION" {
            ids.push(id.to_string());
        }
    }
    ids
}

/// Parse `loginctl show-session <id> -p …` `KEY=VALUE` output.
pub fn parse_show_session(id: &str, text: &str) -> LogindSession {
    let mut session = LogindSession {
        id: id.to_string(),
        ..LogindSession::default()
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Name" => session.user = value.to_string(),
            "Seat" => session.seat = value.to_string(),
            "VTNr" => session.vtnr = value.parse().ok().filter(|vt| *vt > 0),
            "Type" => session.session_type = value.to_string(),
            "Class" => session.class = value.to_string(),
            "Active" => session.active = truthy(value),
            "State" => session.state = value.to_string(),
            _ => {}
        }
    }
    session
}

fn truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "yes" | "true" | "1"
    )
}

/// Why a user may not run the second-VT harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preflight {
    /// The user has no seat-holding graphical session: safe to start one.
    Ready,
    /// The user already holds a seat graphical session; the harness must not
    /// start a colliding one. The string is the user-facing refusal.
    Refuse(String),
}

impl Preflight {
    #[cfg(test)]
    pub fn is_ready(&self) -> bool {
        matches!(self, Preflight::Ready)
    }

    /// The one-line marker the harness logs.
    pub fn marker(&self, user: &str) -> String {
        match self {
            Preflight::Ready => format!("Second VT: READY user={user}"),
            Preflight::Refuse(_) => format!("Second VT: REFUSE user={user}"),
        }
    }
}

/// The dedicated-user preflight: refuse when `user` already holds an active
/// seat graphical session.
pub fn preflight(user: &str, sessions: &[LogindSession]) -> Preflight {
    let holding: Vec<&LogindSession> = sessions
        .iter()
        .filter(|session| session.user == user && session.is_seat_graphical())
        .collect();
    if holding.is_empty() {
        return Preflight::Ready;
    }
    let where_: Vec<String> = holding
        .iter()
        .map(|session| {
            let vt = session
                .vt()
                .map(|vt| format!("VT{vt}"))
                .unwrap_or_else(|| "no VT".to_string());
            format!("session {} ({}, {vt})", session.id, session.seat)
        })
        .collect();
    Preflight::Refuse(format!(
        "user {user:?} already holds a seat graphical session: {}.\n\
         A second graphical session for the same user collides through shared \
         user-session state (XDG_RUNTIME_DIR, portals, environment); use a \
         dedicated development user, e.g. `dragonfruit dev --real --user {DEFAULT_DEV_USER}` \
         (create it once with `sudo useradd -m {DEFAULT_DEV_USER}`).",
        where_.join(", ")
    ))
}

/// The smallest free VT on the seat, skipping every VT any session occupies.
/// `None` when the probed range is full.
pub fn select_free_vt(sessions: &[LogindSession]) -> Option<u32> {
    let used: Vec<u32> = sessions.iter().filter_map(LogindSession::vt).collect();
    (FIRST_FREE_VT..=LAST_VT).find(|vt| !used.contains(vt))
}

/// The id of a user's session currently on `vt`, if any. Teardown targets this.
pub fn session_on_vt<'a>(
    sessions: &'a [LogindSession],
    user: &str,
    vt: u32,
) -> Option<&'a LogindSession> {
    sessions
        .iter()
        .find(|session| session.user == user && session.vt() == Some(vt))
}

/// A `loginctl` runner. The command is injectable so unit tests drive a mock.
#[derive(Debug, Clone)]
pub struct Logind {
    command: PathBuf,
}

impl Logind {
    /// The host's `loginctl`. `DF_LOGINCTL` overrides it (the validation
    /// script points the harness at a mock), otherwise it resolves on `PATH`.
    pub fn host() -> Self {
        Logind {
            command: std::env::var_os("DF_LOGINCTL")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("loginctl")),
        }
    }

    /// A `loginctl` replaced by `command` (a mock in tests).
    #[cfg(test)]
    pub fn with_command(command: impl Into<PathBuf>) -> Self {
        Logind {
            command: command.into(),
        }
    }

    /// Snapshot every session via `list-sessions` + one `show-session` each.
    pub fn sessions(&self) -> io::Result<Vec<LogindSession>> {
        let list = self.run(&["list-sessions", "--no-legend"])?;
        let mut sessions = Vec::new();
        for id in parse_session_ids(&list) {
            let text = self.run(&[
                "show-session",
                &id,
                "-p",
                "Name",
                "-p",
                "Seat",
                "-p",
                "VTNr",
                "-p",
                "Type",
                "-p",
                "Class",
                "-p",
                "Active",
                "-p",
                "State",
            ])?;
            sessions.push(parse_show_session(&id, &text));
        }
        Ok(sessions)
    }

    /// Terminate a session by id (`loginctl terminate-session`).
    pub fn terminate_session(&self, id: &str) -> io::Result<()> {
        self.run_checked(&["terminate-session", id])
    }

    fn run(&self, args: &[&str]) -> io::Result<String> {
        let output = Command::new(&self.command).args(args).output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "{} {} failed: {}",
                self.command.display(),
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn run_checked(&self, args: &[&str]) -> io::Result<()> {
        self.run(args).map(|_| ())
    }
}

/// The uid/gid a dedicated user resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserIds {
    pub uid: u32,
    pub gid: u32,
}

/// Parse one user's uid/gid from `/etc/passwd` text.
pub fn passwd_ids(text: &str, user: &str) -> Option<UserIds> {
    for line in text.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() >= 4 && fields[0] == user {
            let uid = fields[2].parse().ok()?;
            let gid = fields[3].parse().ok()?;
            return Some(UserIds { uid, gid });
        }
    }
    None
}

/// Resolve a user on the host. `None` when the user does not exist.
/// `DF_PASSWD` overrides `/etc/passwd` (the validation script's mock).
pub fn lookup_user(user: &str) -> Option<UserIds> {
    let text = match std::env::var_os("DF_PASSWD") {
        Some(path) => std::fs::read_to_string(path).ok()?,
        None => std::fs::read_to_string("/etc/passwd").ok()?,
    };
    passwd_ids(&text, user)
}

/// The dedicated-user bootstrap command the refusal suggests.
pub fn create_user_command(user: &str) -> Vec<String> {
    vec![
        "sudo".to_string(),
        "useradd".to_string(),
        "-m".to_string(),
        user.to_string(),
    ]
}

/// The exact commands a privileged runner executes to start the second-VT
/// session, and to switch to / away from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondVtPlan {
    pub user: String,
    pub uid: u32,
    pub gid: u32,
    pub vt: u32,
    pub socket: String,
    pub entry: PathBuf,
}

impl SecondVtPlan {
    pub fn new(
        user: impl Into<String>,
        ids: UserIds,
        vt: u32,
        socket: impl Into<String>,
        entry: impl Into<PathBuf>,
    ) -> Self {
        SecondVtPlan {
            user: user.into(),
            uid: ids.uid,
            gid: ids.gid,
            vt,
            socket: socket.into(),
            entry: entry.into(),
        }
    }

    /// The VT device the session's PAM login is attached to.
    pub fn tty(&self) -> PathBuf {
        PathBuf::from(format!("/dev/tty{}", self.vt))
    }

    /// The dedicated user's runtime directory.
    pub fn runtime_dir(&self) -> PathBuf {
        PathBuf::from(format!("/run/user/{}", self.uid))
    }

    /// The `chvt` command that switches the seat to the free VT. This is what
    /// makes the new DRM session's libseat session active.
    pub fn switch_command(&self) -> Vec<String> {
        vec!["chvt".to_string(), self.vt.to_string()]
    }

    /// The `systemd-run` command that starts the session entry as a PAM login
    /// session on the free VT, as the dedicated user, with its own
    /// `XDG_RUNTIME_DIR` and user manager.
    pub fn start_command(&self) -> Vec<String> {
        vec![
            "systemd-run".to_string(),
            "--quiet".to_string(),
            "--collect".to_string(),
            "--unit".to_string(),
            UNIT.to_string(),
            "--uid".to_string(),
            self.uid.to_string(),
            "--gid".to_string(),
            self.gid.to_string(),
            "--property".to_string(),
            "PAMName=login".to_string(),
            "--property".to_string(),
            format!("TTYPath={}", self.tty().display()),
            "--property".to_string(),
            "StandardInput=tty".to_string(),
            "--property".to_string(),
            "StandardOutput=journal".to_string(),
            "--property".to_string(),
            "StandardError=journal".to_string(),
            "--setenv".to_string(),
            format!("XDG_RUNTIME_DIR={}", self.runtime_dir().display()),
            "--setenv".to_string(),
            "XDG_SESSION_TYPE=wayland".to_string(),
            self.entry.display().to_string(),
        ]
    }

    /// The command that switches the host back to `host_vt` (the VT the host
    /// desktop was on), so returning is a plain VT switch.
    pub fn return_command(&self, host_vt: u32) -> Vec<String> {
        vec!["chvt".to_string(), host_vt.to_string()]
    }

    /// The one-line marker the harness logs.
    pub fn marker(&self) -> String {
        format!(
            "Second VT: START user={} uid={} vt={} seat=seat0 socket={}",
            self.user, self.uid, self.vt, self.socket
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;

    fn session(id: &str, user: &str, vt: Option<u32>, kind: &str, class: &str) -> LogindSession {
        LogindSession {
            id: id.to_string(),
            user: user.to_string(),
            seat: if vt.is_some() {
                "seat0".to_string()
            } else {
                String::new()
            },
            vtnr: vt,
            session_type: kind.to_string(),
            class: class.to_string(),
            active: true,
            state: "active".to_string(),
        }
    }

    #[test]
    fn parses_a_show_session_snapshot() {
        let text =
            "Name=user\nSeat=seat0\nVTNr=2\nType=wayland\nClass=user\nActive=yes\nState=active\n";
        let session = parse_show_session("2", text);
        assert_eq!(session.id, "2");
        assert_eq!(session.user, "user");
        assert_eq!(session.seat, "seat0");
        assert_eq!(session.vtnr, Some(2));
        assert_eq!(session.session_type, "wayland");
        assert_eq!(session.class, "user");
        assert!(session.active);
        assert!(session.is_seat_graphical());
    }

    #[test]
    fn a_user_manager_is_not_a_seat_graphical_session() {
        let text =
            "Name=user\nSeat=\nVTNr=0\nType=unspecified\nClass=manager\nActive=yes\nState=active\n";
        let session = parse_show_session("3", text);
        assert!(session.vt().is_none());
        assert!(!session.is_seat_graphical());
    }

    #[test]
    fn parses_session_ids_from_the_list_format() {
        let text = " 2 1000 user seat0 /dev/tty2\n 3 1000 user -     1437\n";
        assert_eq!(parse_session_ids(text), vec!["2", "3"]);
        // A header line is not an id.
        assert_eq!(
            parse_session_ids("SESSION UID USER\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn preflight_refuses_a_user_holding_a_seat_graphical_session() {
        let sessions = vec![
            session("2", "user", Some(2), "wayland", "user"),
            session("3", "user", None, "unspecified", "manager"),
        ];
        let refusal = preflight("user", &sessions);
        assert!(!refusal.is_ready());
        let Preflight::Refuse(message) = refusal else {
            panic!("expected refusal");
        };
        assert!(message.contains("dedicated"), "{message}");
        assert!(message.contains("useradd"), "{message}");
        assert!(message.contains(&DEFAULT_DEV_USER.to_string()), "{message}");
        assert_eq!(
            Preflight::Refuse(message).marker("user"),
            "Second VT: REFUSE user=user"
        );
    }

    #[test]
    fn preflight_ignores_another_users_graphical_session() {
        let sessions = vec![session("2", "someone", Some(2), "wayland", "user")];
        assert!(preflight("dfdev", &sessions).is_ready());
        assert_eq!(
            preflight("dfdev", &sessions).marker("dfdev"),
            "Second VT: READY user=dfdev"
        );
    }

    #[test]
    fn preflight_allows_a_console_only_user() {
        // A tty login is not graphical; the session can still be started.
        let sessions = vec![session("4", "dfdev", Some(4), "tty", "user")];
        assert!(preflight("dfdev", &sessions).is_ready());
    }

    #[test]
    fn select_free_vt_skips_every_occupied_vt() {
        // The host desktop is on VT2 (not VT1) — the selection must not guess.
        let sessions = vec![session("2", "user", Some(2), "wayland", "user")];
        assert_eq!(select_free_vt(&sessions), Some(3));

        // Several occupied VTs, including a non-graphical one.
        let sessions = vec![
            session("1", "user", Some(1), "tty", "user"),
            session("2", "user", Some(2), "wayland", "user"),
            session("3", "user", Some(3), "x11", "user"),
        ];
        assert_eq!(select_free_vt(&sessions), Some(4));
    }

    #[test]
    fn select_free_vt_returns_none_when_the_range_is_full() {
        let sessions: Vec<LogindSession> = (FIRST_FREE_VT..=LAST_VT)
            .map(|vt| session("s", "user", Some(vt), "tty", "user"))
            .collect();
        assert_eq!(select_free_vt(&sessions), None);
    }

    #[test]
    fn finds_a_users_session_on_a_vt_for_teardown() {
        let sessions = vec![
            session("2", "user", Some(2), "wayland", "user"),
            session("7", "dfdev", Some(7), "wayland", "user"),
        ];
        assert_eq!(
            session_on_vt(&sessions, "dfdev", 7).map(|s| s.id.as_str()),
            Some("7")
        );
        assert!(session_on_vt(&sessions, "dfdev", 2).is_none());
    }

    #[test]
    fn parses_and_looks_up_passwd_ids() {
        let passwd = "root:x:0:0:root:/root:/bin/bash\ndfdev:x:1001:1001::/home/dfdev:/bin/bash\n";
        assert_eq!(
            passwd_ids(passwd, "dfdev"),
            Some(UserIds {
                uid: 1001,
                gid: 1001
            })
        );
        assert_eq!(passwd_ids(passwd, "nobody"), None);
    }

    #[test]
    fn plan_builds_the_start_switch_and_teardown_commands() {
        let plan = SecondVtPlan::new(
            "dfdev",
            UserIds {
                uid: 1001,
                gid: 1001,
            },
            3,
            SESSION_SOCKET,
            "services/session/dragonfruit-session-entry",
        );
        assert_eq!(plan.tty(), PathBuf::from("/dev/tty3"));
        assert_eq!(plan.runtime_dir(), PathBuf::from("/run/user/1001"));

        let start = plan.start_command();
        assert_eq!(start[0], "systemd-run");
        assert!(start.iter().any(|arg| arg == "PAMName=login"));
        assert!(start.iter().any(|arg| arg == "TTYPath=/dev/tty3"));
        assert!(start.iter().any(|arg| arg == "--uid"));
        assert!(start.iter().any(|arg| arg == "1001"));
        assert!(start
            .iter()
            .any(|arg| arg == "XDG_RUNTIME_DIR=/run/user/1001"));
        assert_eq!(
            start.last().unwrap(),
            "services/session/dragonfruit-session-entry"
        );

        assert_eq!(plan.switch_command(), vec!["chvt", "3"]);
        assert_eq!(plan.return_command(1), vec!["chvt", "1"]);
        assert_eq!(
            plan.marker(),
            "Second VT: START user=dfdev uid=1001 vt=3 seat=seat0 socket=dragonfruit-wayland"
        );
    }

    #[test]
    fn create_user_command_is_the_documented_once_bootstrap() {
        assert_eq!(
            create_user_command("dfdev"),
            vec!["sudo", "useradd", "-m", "dfdev"]
        );
    }

    /// A mock `loginctl`: `list-sessions` prints fixture ids and
    /// `show-session <id> …` prints the matching fixture. This is the task's
    /// "VT-selection and preflight logic against a mocked `loginctl`".
    fn mock_loginctl(dir: &Path) -> PathBuf {
        let path = dir.join("loginctl");
        let script = r#"#!/bin/sh
set -eu
case "${1:-}" in
  list-sessions)
    printf ' 2 1000 user seat0 /dev/tty2\n 3 1000 user - 1437\n'
    ;;
  show-session)
    case "${2:-}" in
      2) printf 'Name=user\nSeat=seat0\nVTNr=2\nType=wayland\nClass=user\nActive=yes\nState=active\n' ;;
      3) printf 'Name=user\nSeat=\nVTNr=0\nType=unspecified\nClass=manager\nActive=yes\nState=active\n' ;;
      *) printf 'Name=\nSeat=\nVTNr=0\nType=unspecified\nClass=other\nActive=no\nState=closing\n' ;;
    esac
    ;;
  terminate-session) printf 'ok\n' ;;
  *) printf 'unexpected: %s\n' "$*" >&2; exit 1 ;;
esac
"#;
        let mut file = std::fs::File::create(&path).expect("create mock");
        file.write_all(script.as_bytes()).expect("write mock");
        drop(file);
        let mut perms = std::fs::metadata(&path).expect("stat").permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod");
        path
    }

    #[test]
    fn a_mocked_loginctl_drives_preflight_and_vt_selection() {
        let dir = std::env::temp_dir().join(format!(
            "dragonfruit-t126c-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("scratch");
        let mock = mock_loginctl(&dir);
        let logind = Logind::with_command(&mock);

        let sessions = logind.sessions().expect("mock sessions");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "2");
        assert_eq!(sessions[0].user, "user");
        assert!(sessions[0].is_seat_graphical());
        assert!(!sessions[1].is_seat_graphical());

        // The mock's host user holds the seat on VT2, so the user is refused
        // and dfdev gets VT3.
        assert!(!preflight("user", &sessions).is_ready());
        assert!(preflight("dfdev", &sessions).is_ready());
        assert_eq!(select_free_vt(&sessions), Some(3));

        logind.terminate_session("2").expect("terminate via mock");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_user_is_not_in_passwd() {
        assert_eq!(passwd_ids("root:x:0:0\n", "dfdev"), None);
    }
}
