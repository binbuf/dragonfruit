// SPDX-License-Identifier: MIT
//! T-12.1b acceptance: the session environment reaches children, and the two
//! unit seams (`--print-env`, `--wait-socket`) behave.
//!
//! Headless: the "service" is a real `sh` probe that writes the variables it
//! sees to a file through the same [`Supervisor`] the session uses.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use dragonfruit_session::env::{self, SessionEnvironment};
use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor};

/// Run ticks until `predicate` holds or `timeout` elapses.
fn drive_until(
    supervisor: &mut Supervisor,
    timeout: Duration,
    mut predicate: impl FnMut(&Supervisor) -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        supervisor.tick();
        if predicate(supervisor) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn unique_path(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "dragonfruit-{label}-{}-{nanos}",
        std::process::id()
    ))
}

/// `sh` writes the five contract variables, one per line, to `$1`.
fn probe_args(out: &Path) -> Vec<String> {
    vec![
        "-c".to_string(),
        "printf '%s\\n' \"$XDG_CURRENT_DESKTOP\" \"$XDG_SESSION_TYPE\" \
         \"$WAYLAND_DISPLAY\" \"$DISPLAY\" \"$DRAGONFRUIT_LAUNCH_TOKEN\" > \"$1\""
            .to_string(),
        "sh".to_string(),
        out.to_string_lossy().into_owned(),
    ]
}

#[test]
fn a_trusted_child_sees_the_whole_session_environment() {
    let out = unique_path("env-trusted");
    let token = "ab".repeat(32);
    let environment = SessionEnvironment::new("df-env-test")
        .with_x11_display(":7")
        .with_launch_token(token.clone());
    let plan = SessionPlan::new(vec![ServiceSpec::new("probe", "sh")
        .args(probe_args(&out))
        .policy(RestartPolicy::Never)
        .trusted(true)])
    .with_environment(&environment);

    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("probe starts");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            matches!(s.service_state("probe"), Some(ServiceState::Exited))
        }),
        "the probe exits cleanly"
    );

    let contents = std::fs::read_to_string(&out).expect("the probe wrote its environment");
    let _ = std::fs::remove_file(&out);
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(
        lines,
        vec![
            df_ipc::DESKTOP_NAME,
            env::WAYLAND_SESSION_TYPE,
            "df-env-test",
            ":7",
            token.as_str(),
        ],
        "every contract variable reached the child: {contents:?}"
    );
    supervisor.shutdown();
}

#[test]
fn an_untrusted_child_does_not_get_the_token() {
    let out = unique_path("env-untrusted");
    let environment = SessionEnvironment::new("df-env-test").with_launch_token("ab".repeat(32));
    let plan = SessionPlan::new(vec![ServiceSpec::new("probe", "sh")
        .args(probe_args(&out))
        .policy(RestartPolicy::Never)])
    .with_environment(&environment);

    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("probe starts");
    assert!(drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.service_state("probe") == Some(ServiceState::Exited)
    }));

    let contents = std::fs::read_to_string(&out).expect("the probe wrote its environment");
    let _ = std::fs::remove_file(&out);
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(
        lines.last().copied(),
        Some(""),
        "the untrusted child sees no launch token: {contents:?}"
    );
    // XDG_CURRENT_DESKTOP and WAYLAND_DISPLAY still arrive.
    assert_eq!(lines.first().copied(), Some(df_ipc::DESKTOP_NAME));
    assert_eq!(lines.get(2).copied(), Some("df-env-test"));
    supervisor.shutdown();
}

#[test]
fn wait_socket_blocks_until_the_socket_appears() {
    let socket = unique_path("wait-socket");
    assert!(!socket.exists());
    let child = {
        let socket = socket.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            std::fs::write(socket, b"").expect("write a fake socket");
        })
    };

    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args(["--wait-socket"])
        .arg(&socket)
        .args(["--timeout", "5"])
        .output()
        .expect("the session binary runs");
    child.join().expect("the writer thread");
    let _ = std::fs::remove_file(&socket);

    assert!(
        output.status.success(),
        "wait-socket returns once the socket exists: {output:?}"
    );
}

#[test]
fn wait_socket_fails_when_the_timeout_elapses() {
    let socket = unique_path("wait-socket-missing");
    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args(["--wait-socket"])
        .arg(&socket)
        .args(["--timeout", "0"])
        .output()
        .expect("the session binary runs");
    assert!(!output.status.success(), "a missing socket times out");
}

#[test]
fn print_env_lists_the_contract_and_a_fresh_token() {
    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args([
            "--print-env",
            "--socket-name",
            "df-print-env",
            "--x11-display",
            ":9",
        ])
        .output()
        .expect("the session binary runs");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("env output is UTF-8");

    assert!(text.contains("XDG_CURRENT_DESKTOP=dragonfruit"), "{text}");
    assert!(text.contains("XDG_SESSION_TYPE=wayland"), "{text}");
    assert!(text.contains("WAYLAND_DISPLAY=df-print-env"), "{text}");
    assert!(text.contains("DISPLAY=:9"), "{text}");

    let token = text
        .lines()
        .find_map(|line| line.strip_prefix("DRAGONFRUIT_LAUNCH_TOKEN="))
        .expect("a launch token is exported");
    assert_eq!(token.len(), env::TOKEN_BYTES * 2, "{token}");
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()), "{token}");
}

#[test]
fn shutting_a_session_down_ends_the_state() {
    // Guard: the environment plan keeps the supervisor's invariants.
    let plan = SessionPlan::default_session_for(&SessionEnvironment::default_socket());
    plan.validate().expect("the environment plan is valid");
    let mut supervisor = Supervisor::new(plan);
    supervisor.shutdown();
    assert_eq!(supervisor.state(), SessionState::Ended);
}
