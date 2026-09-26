// SPDX-License-Identifier: MIT
//! Headless acceptance tests for the lock-auth helper (T-12.3b).
//!
//! The task's test plan is "helper success/failure paths against a PAM test
//! service". These tests install throwaway `permit`/`deny` services under a
//! temp directory, point the helper at them with `--confdir` (Linux-PAM
//! `pam_start_confdir`), and exercise the **real** libpam and the **real**
//! helper binary. No `/etc/pam.d` entry and no root are needed.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use dragonfruit_lock_auth::{AuthResult, Authenticator, PamAuthenticator};

const HELPER: &str = env!("CARGO_BIN_EXE_dragonfruit-pam-helper");

/// A `pam_permit` service: authentication always succeeds.
const PERMIT: &str = "auth required pam_permit.so\naccount required pam_permit.so\n";

/// A `pam_deny` service: authentication always fails.
const DENY: &str = "auth required pam_deny.so\naccount required pam_permit.so\n";

fn service_dir(tag: &str) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix(&format!("df-lock-auth-{tag}-"))
        .tempdir()
        .expect("temp service dir");
    std::fs::write(dir.path().join("permit"), PERMIT).expect("write permit");
    std::fs::write(dir.path().join("deny"), DENY).expect("write deny");
    dir
}

struct HelperRun {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_helper(user: Option<&str>, service: &str, confdir: &Path, password: &str) -> HelperRun {
    let mut command = Command::new(HELPER);
    command.arg("--service").arg(service);
    command.arg("--confdir").arg(confdir);
    if let Some(user) = user {
        command.arg("--user").arg(user);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn dragonfruit-pam-helper");
    {
        let stdin = child.stdin.as_mut().expect("helper stdin");
        stdin
            .write_all(password.as_bytes())
            .expect("write password");
        stdin.write_all(b"\n").expect("write newline");
    }
    let output = child.wait_with_output().expect("helper output");
    HelperRun {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn dir_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("read service dir")
        .map(|entry| entry.expect("dir entry").path())
        .collect();
    entries.sort();
    entries
}

#[test]
fn permit_service_authenticates() {
    let services = service_dir("permit");
    let run = run_helper(Some("alice"), "permit", services.path(), "correct horse");
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
}

#[test]
fn deny_service_rejects_both_the_right_and_wrong_password() {
    let services = service_dir("deny");
    for password in ["whatever", ""] {
        let run = run_helper(Some("alice"), "deny", services.path(), password);
        assert_eq!(run.code, 1, "password {password:?}: {}", run.stderr);
    }
}

#[test]
fn missing_service_falls_back_to_login() {
    let services = service_dir("fallback");
    // The default `dragonfruit` service is absent, so the authenticator must
    // retry `login`. Make `login` a permit service so the fallback is visible.
    std::fs::write(services.path().join("login"), PERMIT).expect("write login");
    let authenticator = PamAuthenticator::new()
        .with_service("dragonfruit")
        .with_confdir(services.path());
    assert_eq!(
        authenticator.authenticate("alice", "correct horse"),
        AuthResult::Success
    );
}

#[test]
fn unknown_service_without_a_fallback_reports_error() {
    let services = service_dir("missing");
    let run = run_helper(Some("alice"), "no-such-service", services.path(), "x");
    assert_eq!(run.code, 2, "stderr: {}", run.stderr);
}

#[test]
fn a_missing_user_is_a_usage_error() {
    let services = service_dir("usage");
    let run = run_helper(None, "permit", services.path(), "x");
    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("--user is required"),
        "stderr: {}",
        run.stderr
    );
}

#[test]
fn the_password_is_never_printed_or_written_down() {
    let services = service_dir("secret");
    let before = dir_entries(services.path());
    let secret = "correct-horse-battery-staple";

    let run = run_helper(Some("alice"), "permit", services.path(), secret);

    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    // Never echoed to either stream...
    assert!(!run.stdout.contains(secret), "stdout leaked the password");
    assert!(!run.stderr.contains(secret), "stderr leaked the password");
    // ...and never written into the service directory (the only directory
    // this process is handed besides the inherited environment).
    assert_eq!(
        dir_entries(services.path()),
        before,
        "the helper wrote into its service directory"
    );
}

#[test]
fn library_seam_authenticates_and_rejects_against_a_real_pam_service() {
    let services = service_dir("library");

    let permit = PamAuthenticator::new()
        .with_service("permit")
        .with_confdir(services.path());
    assert_eq!(
        permit.authenticate("alice", "anything"),
        AuthResult::Success
    );

    let deny = PamAuthenticator::new()
        .with_service("deny")
        .with_confdir(services.path());
    assert_eq!(deny.authenticate("alice", "anything"), AuthResult::Denied);
}
