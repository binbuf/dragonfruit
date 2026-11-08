// SPDX-License-Identifier: MIT
//! The native/PackageKit update provider (T226).
//!
//! T-15.10a shipped the [`UpdateProvider`] seam and the host identity read;
//! the concrete provider was deliberately left to packaging. T226 supplies the
//! cross-distro native provider the Software Update pane and the Control Center
//! tile drive: `dnf` on Fedora, `apt-get` on Debian/Ubuntu, `pacman` on Arch,
//! and PackageKit (`pkcon`) as the portable fallback. The provider never
//! reimplements package resolution — it runs the distribution's own tool and
//! projects its answer into the typed [`UpdateData`] the adapter already owns.
//!
//! # Progress and the no-poll rule
//!
//! Check and install are explicit writes, not polls. A write sets the phase to
//! [`UpdatePhase::Checking`] or [`UpdatePhase::Installing`] and runs the tool on
//! a worker thread, so the bridge host keeps answering `State()` while the
//! package manager works. The Settings pane re-reads (`Refresh`) after the
//! write and sees the provider's current phase; it never samples on a timer.
//!
//! # Automatic policy
//!
//! The durable `updates.auto` / `updates.autoInstall` keys are settingsd's.
//! The Settings pane forwards the two values with
//! [`UpdateProvider::set_auto_policy`]; the provider stores them for the
//! lifetime of the host and, when a check it started finds updates and
//! auto-install is on, starts the install. Nothing silently enables the policy:
//! the schema defaults are `false` and only an explicit pane write turns it on.
//!
//! # Absence
//!
//! No supported backend means the provider reports `Ok(None)` from
//! [`UpdateProvider::status`] — the adapter's "host answers, no update
//! provider" state. The About rows stay live and only the update controls
//! disable (ADR 0136).

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use dragonfruit_system_adapters::AdapterError;

use crate::host::UpdateProvider;
use crate::source::{
    UpdateData, UpdateItem, UpdateOutcome, UpdatePhase, UpdatePolicy, UpdateSeverity,
};

/// The package manager the provider drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateBackend {
    /// Fedora/RHEL's `dnf`.
    Dnf,
    /// Debian/Ubuntu's `apt-get`.
    Apt,
    /// Arch's `pacman`.
    Pacman,
    /// PackageKit's `pkcon` (the portable fallback).
    PackageKit,
    /// No supported backend is installed.
    #[default]
    None,
}

impl UpdateBackend {
    /// Every backend, native first.
    pub const ALL: [UpdateBackend; 5] = [
        UpdateBackend::Dnf,
        UpdateBackend::Apt,
        UpdateBackend::Pacman,
        UpdateBackend::PackageKit,
        UpdateBackend::None,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            UpdateBackend::Dnf => "dnf",
            UpdateBackend::Apt => "apt",
            UpdateBackend::Pacman => "pacman",
            UpdateBackend::PackageKit => "packagekit",
            UpdateBackend::None => "none",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            UpdateBackend::Dnf => "DNF",
            UpdateBackend::Apt => "APT",
            UpdateBackend::Pacman => "Pacman",
            UpdateBackend::PackageKit => "PackageKit",
            UpdateBackend::None => "No update provider",
        }
    }

    /// Detect the host's backend from the tools on `PATH`, preferring the
    /// distribution's native manager over PackageKit.
    pub fn detect() -> UpdateBackend {
        for (backend, program) in [
            (UpdateBackend::Dnf, "dnf"),
            (UpdateBackend::Apt, "apt-get"),
            (UpdateBackend::Pacman, "pacman"),
            (UpdateBackend::PackageKit, "pkcon"),
        ] {
            if program_available(program) {
                return backend;
            }
        }
        UpdateBackend::None
    }
}

fn program_available(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file()
    })
}

/// One command's captured answer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandOutput {
    /// The exit status, when the process exited normally.
    pub status: Option<i32>,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

impl CommandOutput {
    /// A successful (exit 0) command.
    pub fn success(&self) -> bool {
        self.status == Some(0)
    }

    /// The most useful error text: stderr, else stdout.
    fn error_text(&self) -> String {
        let text = if self.stderr.trim().is_empty() {
            self.stdout.trim()
        } else {
            self.stderr.trim()
        };
        if text.is_empty() {
            format!("command exited with {:?}", self.status)
        } else {
            text.lines().last().unwrap_or(text).to_owned()
        }
    }
}

/// Runs one host command. The seam exists so the parser and the progress
/// machine are testable with no package manager on the CI host.
pub trait CommandRunner: Send + Sync {
    /// Run `program` with `args` and capture its answer.
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, String>;
}

/// The real process runner.
#[derive(Debug, Default)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, String> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|error| format!("{program}: {error}"))?;
        Ok(CommandOutput {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// The native/PackageKit provider behind the [`UpdateProvider`] seam.
pub struct CommandUpdateProvider {
    backend: UpdateBackend,
    runner: Arc<dyn CommandRunner>,
    state: Arc<Mutex<UpdateData>>,
    policy: Arc<Mutex<UpdatePolicy>>,
}

impl CommandUpdateProvider {
    /// Detect the host backend and use the real process runner.
    pub fn detect() -> Self {
        CommandUpdateProvider::with_runner(UpdateBackend::detect(), Arc::new(SystemRunner))
    }

    /// A provider over a chosen backend and runner (tests and packaging).
    pub fn with_runner(backend: UpdateBackend, runner: Arc<dyn CommandRunner>) -> Self {
        let mut state = UpdateData {
            phase: UpdatePhase::Idle,
            ..UpdateData::default()
        };
        if backend == UpdateBackend::None {
            state.message = Some("No update provider is installed.".to_owned());
        }
        CommandUpdateProvider {
            backend,
            runner,
            state: Arc::new(Mutex::new(state)),
            policy: Arc::new(Mutex::new(UpdatePolicy::default())),
        }
    }

    /// The selected backend.
    pub fn backend(&self) -> UpdateBackend {
        self.backend
    }

    /// The current automatic policy.
    pub fn auto_policy(&self) -> UpdatePolicy {
        *self.policy.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn start_check(&self) -> UpdateOutcome {
        if self.backend == UpdateBackend::None {
            return UpdateOutcome::Absent;
        }
        {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.phase.is_busy() {
                return UpdateOutcome::Applied;
            }
            state.phase = UpdatePhase::Checking;
            state.message = Some("Checking for updates…".to_owned());
        }
        spawn_check(
            self.backend,
            Arc::clone(&self.runner),
            Arc::clone(&self.state),
            Arc::clone(&self.policy),
        );
        UpdateOutcome::Applied
    }

    fn start_install(&self) -> UpdateOutcome {
        if self.backend == UpdateBackend::None {
            return UpdateOutcome::Absent;
        }
        {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.phase == UpdatePhase::Installing {
                return UpdateOutcome::Applied;
            }
            state.phase = UpdatePhase::Installing;
            state.message = Some("Installing updates…".to_owned());
        }
        spawn_install(
            self.backend,
            Arc::clone(&self.runner),
            Arc::clone(&self.state),
        );
        UpdateOutcome::Applied
    }

    fn start_reboot(&self) -> UpdateOutcome {
        if self.backend == UpdateBackend::None {
            return UpdateOutcome::Absent;
        }
        let runner = Arc::clone(&self.runner);
        std::thread::spawn(move || {
            let _ = runner.run("pkexec", &["systemctl", "reboot"]);
        });
        UpdateOutcome::Applied
    }
}

impl UpdateProvider for CommandUpdateProvider {
    fn status(&mut self) -> Result<Option<UpdateData>, AdapterError> {
        if self.backend == UpdateBackend::None {
            return Ok(None);
        }
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        Ok(Some(state.clone()))
    }

    fn start_check(&mut self) -> UpdateOutcome {
        CommandUpdateProvider::start_check(self)
    }

    fn start_install(&mut self) -> UpdateOutcome {
        CommandUpdateProvider::start_install(self)
    }

    fn start_reboot(&mut self) -> UpdateOutcome {
        CommandUpdateProvider::start_reboot(self)
    }

    fn set_auto_policy(&mut self, policy: UpdatePolicy) -> UpdateOutcome {
        *self.policy.lock().unwrap_or_else(|p| p.into_inner()) = policy;
        if policy.auto_check && self.backend != UpdateBackend::None {
            let _ = CommandUpdateProvider::start_check(self);
        }
        UpdateOutcome::Applied
    }
}

fn spawn_check(
    backend: UpdateBackend,
    runner: Arc<dyn CommandRunner>,
    state: Arc<Mutex<UpdateData>>,
    policy: Arc<Mutex<UpdatePolicy>>,
) {
    std::thread::spawn(move || {
        let result = run_check(backend, runner.as_ref());
        let auto_install;
        let has_updates;
        {
            let mut state = state.lock().unwrap_or_else(|p| p.into_inner());
            state.last_checked_ms = Some(now_ms());
            match result {
                Ok(updates) => {
                    has_updates = !updates.is_empty();
                    state.updates = updates;
                    if has_updates {
                        state.phase = UpdatePhase::Available;
                        state.message = None;
                    } else {
                        state.phase = UpdatePhase::UpToDate;
                        state.message = Some("Up to date".to_owned());
                    }
                }
                Err(message) => {
                    has_updates = false;
                    state.updates.clear();
                    state.phase = UpdatePhase::Failed;
                    state.message = Some(message);
                }
            }
            auto_install = policy
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .auto_install;
        }
        if auto_install && has_updates {
            {
                let mut state = state.lock().unwrap_or_else(|p| p.into_inner());
                state.phase = UpdatePhase::Installing;
                state.message = Some("Installing updates…".to_owned());
            }
            spawn_install(backend, runner, state);
        }
    });
}

fn spawn_install(
    backend: UpdateBackend,
    runner: Arc<dyn CommandRunner>,
    state: Arc<Mutex<UpdateData>>,
) {
    std::thread::spawn(move || {
        let result = run_install(backend, runner.as_ref());
        let reboot_required = result.is_ok() && reboot_required(backend, runner.as_ref());
        let mut state = state.lock().unwrap_or_else(|p| p.into_inner());
        match result {
            Ok(()) => {
                state.updates.clear();
                state.last_checked_ms = Some(now_ms());
                if reboot_required {
                    state.phase = UpdatePhase::RebootRequired;
                    state.message = Some("Restart to finish installing updates.".to_owned());
                } else {
                    state.phase = UpdatePhase::UpToDate;
                    state.message = Some("Up to date".to_owned());
                }
            }
            Err(message) => {
                state.phase = UpdatePhase::Failed;
                state.message = Some(message);
            }
        }
    });
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Run one backend check and parse the offered updates.
pub fn run_check(
    backend: UpdateBackend,
    runner: &dyn CommandRunner,
) -> Result<Vec<UpdateItem>, String> {
    match backend {
        UpdateBackend::Dnf => {
            let output = runner.run("dnf", &["-q", "check-update"])?;
            match output.status {
                Some(0) => Ok(Vec::new()),
                Some(100) => Ok(parse_dnf_check(&output.stdout)),
                _ => Err(output.error_text()),
            }
        }
        UpdateBackend::Apt => {
            let output = runner.run("apt-get", &["-s", "-o", "Debug::NoLocking=1", "upgrade"])?;
            if output.success() {
                Ok(parse_apt_check(&output.stdout))
            } else {
                Err(output.error_text())
            }
        }
        UpdateBackend::Pacman => {
            let output = runner.run("pacman", &["-Qu"])?;
            if output.success() {
                Ok(parse_pacman_check(&output.stdout))
            } else if output.status == Some(1) && output.stdout.trim().is_empty() {
                Ok(Vec::new())
            } else {
                Err(output.error_text())
            }
        }
        UpdateBackend::PackageKit => {
            let output = runner.run("pkcon", &["get-updates"])?;
            if output.success() {
                Ok(parse_packagekit_check(&output.stdout))
            } else {
                Err(output.error_text())
            }
        }
        UpdateBackend::None => Ok(Vec::new()),
    }
}

/// Run one backend install.
pub fn run_install(backend: UpdateBackend, runner: &dyn CommandRunner) -> Result<(), String> {
    let output = match backend {
        UpdateBackend::Dnf => runner.run("pkexec", &["dnf", "-y", "upgrade"])?,
        UpdateBackend::Apt => runner.run("pkexec", &["apt-get", "-y", "upgrade"])?,
        UpdateBackend::Pacman => runner.run("pkexec", &["pacman", "-Syu", "--noconfirm"])?,
        UpdateBackend::PackageKit => runner.run("pkcon", &["update", "-y"])?,
        UpdateBackend::None => return Err("No update provider is installed.".to_owned()),
    };
    if output.success() {
        Ok(())
    } else {
        Err(output.error_text())
    }
}

fn reboot_required(backend: UpdateBackend, runner: &dyn CommandRunner) -> bool {
    if std::path::Path::new("/run/reboot-required").exists()
        || std::path::Path::new("/var/run/reboot-required").exists()
    {
        return true;
    }
    if backend == UpdateBackend::Dnf {
        if let Ok(output) = runner.run("needs-restarting", &["-r"]) {
            return output.status == Some(1);
        }
    }
    false
}

/// Parse `dnf -q check-update` output. Each update line is
/// `name.arch  version  repo`.
pub fn parse_dnf_check(text: &str) -> Vec<UpdateItem> {
    let mut updates = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("Last metadata")
            || line.starts_with("Obsoleting")
            || line.starts_with("Security:")
        {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }
        let (name, _arch) = fields[0].rsplit_once('.').unwrap_or((fields[0], ""));
        let repo = fields[2];
        let severity = if repo.contains("security") {
            UpdateSeverity::Security
        } else {
            UpdateSeverity::Normal
        };
        updates.push(UpdateItem {
            id: fields[0].to_owned(),
            name: name.to_owned(),
            summary: String::new(),
            current_version: String::new(),
            available_version: fields[1].to_owned(),
            severity,
        });
    }
    updates
}

/// Parse `apt-get -s upgrade` output (`Inst name [old] (new ...)`).
pub fn parse_apt_check(text: &str) -> Vec<UpdateItem> {
    let mut updates = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("Inst ") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        let old = rest
            .split_once('[')
            .and_then(|(_, value)| value.split_once(']'))
            .map(|(value, _)| value.trim().to_owned())
            .unwrap_or_default();
        let new = rest
            .split_once('(')
            .and_then(|(_, value)| value.split_once(')'))
            .map(|(value, _)| value.split_whitespace().next().unwrap_or("").to_owned())
            .unwrap_or_default();
        let severity = if line.contains("security") {
            UpdateSeverity::Security
        } else {
            UpdateSeverity::Normal
        };
        updates.push(UpdateItem {
            id: name.to_owned(),
            name: name.to_owned(),
            summary: String::new(),
            current_version: old,
            available_version: new,
            severity,
        });
    }
    updates
}

/// Parse `pacman -Qu` output (`name old -> new`).
pub fn parse_pacman_check(text: &str) -> Vec<UpdateItem> {
    let mut updates = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 || fields[2] != "->" {
            continue;
        }
        updates.push(UpdateItem {
            id: fields[0].to_owned(),
            name: fields[0].to_owned(),
            summary: String::new(),
            current_version: fields[1].to_owned(),
            available_version: fields[3].to_owned(),
            severity: UpdateSeverity::Normal,
        });
    }
    updates
}

/// Parse `pkcon get-updates`. PackageKit's human output is
/// `Available  name;version;arch;repo  summary` (older releases use spaces).
pub fn parse_packagekit_check(text: &str) -> Vec<UpdateItem> {
    let mut updates = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("Available") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }
        let (package, summary) = rest
            .split_once("  ")
            .map(|(package, summary)| (package.trim(), summary.trim()))
            .unwrap_or((rest, ""));
        let fields: Vec<&str> = package.split(';').collect();
        let (id, version, repo) = if fields.len() >= 4 {
            (fields[0], fields[1], fields[3])
        } else {
            let mut words = package.split_whitespace();
            let id = words.next().unwrap_or(package);
            let version = words.next().unwrap_or("");
            let repo = words.last().unwrap_or("");
            (id, version, repo)
        };
        if id.is_empty() {
            continue;
        }
        let severity = if repo.contains("security") {
            UpdateSeverity::Security
        } else {
            UpdateSeverity::Normal
        };
        updates.push(UpdateItem {
            id: id.to_owned(),
            name: id.to_owned(),
            summary: summary.to_owned(),
            current_version: String::new(),
            available_version: version.to_owned(),
            severity,
        });
    }
    updates
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRunner {
        calls: Mutex<Vec<(String, Vec<String>)>>,
        answers: Mutex<Vec<Result<CommandOutput, String>>>,
    }

    impl FakeRunner {
        fn new(answers: Vec<Result<CommandOutput, String>>) -> Self {
            FakeRunner {
                calls: Mutex::new(Vec::new()),
                answers: Mutex::new(answers),
            }
        }

        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, String> {
            self.calls.lock().unwrap().push((
                program.to_owned(),
                args.iter().map(|arg| (*arg).to_owned()).collect(),
            ));
            let mut answers = self.answers.lock().unwrap();
            if answers.is_empty() {
                return Ok(CommandOutput::default());
            }
            answers.remove(0)
        }
    }

    fn output(status: i32, stdout: &str) -> CommandOutput {
        CommandOutput {
            status: Some(status),
            stdout: stdout.to_owned(),
            stderr: String::new(),
        }
    }

    #[test]
    fn the_backend_vocabulary_is_stable() {
        assert_eq!(UpdateBackend::Dnf.id(), "dnf");
        assert_eq!(UpdateBackend::PackageKit.label(), "PackageKit");
        assert_eq!(UpdateBackend::None.label(), "No update provider");
    }

    #[test]
    fn dnf_check_parses_security_and_routine_updates() {
        let updates = parse_dnf_check(
            "Last metadata expiration check: 0:01:00 ago.\n\
             glibc.x86_64        2.41-1.fc44    updates\n\
             kernel.x86_64       6.12.4-1.fc44  updates-security\n",
        );
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "glibc");
        assert_eq!(updates[0].available_version, "2.41-1.fc44");
        assert_eq!(updates[0].severity, UpdateSeverity::Normal);
        assert_eq!(updates[1].severity, UpdateSeverity::Security);
    }

    #[test]
    fn apt_check_parses_old_and_new_versions() {
        let updates = parse_apt_check(
            "Inst libc6 [2.39-0ubuntu8] (2.40-0ubuntu1 Debian:stable [amd64])\n\
             Inst openssl [3.0] (3.1 Debian:stable-security [amd64])\n",
        );
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "libc6");
        assert_eq!(updates[0].current_version, "2.39-0ubuntu8");
        assert_eq!(updates[0].available_version, "2.40-0ubuntu1");
        assert_eq!(updates[1].severity, UpdateSeverity::Security);
    }

    #[test]
    fn pacman_and_packagekit_checks_parse() {
        let updates = parse_pacman_check("glibc 2.40-1 -> 2.41-1\n");
        assert_eq!(updates[0].current_version, "2.40-1");
        assert_eq!(updates[0].available_version, "2.41-1");

        let updates =
            parse_packagekit_check("Available  glibc;2.41-1;x86_64;updates-security  C library\n");
        assert_eq!(updates[0].name, "glibc");
        assert_eq!(updates[0].available_version, "2.41-1");
        assert_eq!(updates[0].severity, UpdateSeverity::Security);
        assert_eq!(updates[0].summary, "C library");
    }

    #[test]
    fn a_dnf_check_is_one_explicit_write_and_projects_the_phase() {
        let runner = Arc::new(FakeRunner::new(vec![Ok(output(
            100,
            "glibc.x86_64 2.41-1.fc44 updates\n",
        ))]));
        let mut provider = CommandUpdateProvider::with_runner(UpdateBackend::Dnf, runner.clone());
        assert_eq!(provider.status().unwrap().unwrap().phase, UpdatePhase::Idle);
        assert!(provider.start_check().is_applied());
        // The worker updates the shared state; wait for the phase to settle.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let data = provider.status().unwrap().unwrap();
            if data.phase != UpdatePhase::Checking {
                assert_eq!(data.phase, UpdatePhase::Available);
                assert_eq!(data.update_count(), 1);
                break;
            }
            assert!(std::time::Instant::now() < deadline, "check did not settle");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(runner.calls()[0].0, "dnf");
    }

    #[test]
    fn no_backend_is_absence_not_an_error() {
        let mut provider = CommandUpdateProvider::with_runner(
            UpdateBackend::None,
            Arc::new(FakeRunner::new(vec![])),
        );
        assert_eq!(provider.status().unwrap(), None);
        assert_eq!(provider.start_check(), UpdateOutcome::Absent);
        assert_eq!(provider.start_install(), UpdateOutcome::Absent);
    }

    #[test]
    fn the_automatic_policy_is_explicit_and_defaults_off() {
        let runner = Arc::new(FakeRunner::new(vec![Ok(output(
            100,
            "glibc.x86_64 2.41-1.fc44 updates\n",
        ))]));
        let mut provider = CommandUpdateProvider::with_runner(UpdateBackend::Dnf, runner);
        assert_eq!(provider.auto_policy(), UpdatePolicy::default());
        assert!(!provider.auto_policy().auto_check);
        assert!(!provider.auto_policy().auto_install);
        provider.set_auto_policy(UpdatePolicy {
            auto_check: false,
            auto_install: true,
        });
        assert_eq!(
            provider.auto_policy(),
            UpdatePolicy {
                auto_check: false,
                auto_install: true,
            }
        );
    }
}
