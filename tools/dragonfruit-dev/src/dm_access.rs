// SPDX-License-Identifier: MIT
//! How the display-manager seam reads and writes its state files (T344).
//!
//! GDM's AccountsService file, SDDM's state file, and the autologin drop-ins
//! are root-owned on a normal host. The harness runs as the desktop user, so a
//! direct write is refused; silently reporting `Unsupported` made the round
//! trip log the user out without Dragonfruit selected
//! ([ADR 0355](../../docs/design/adr/0355-dm-session-write-elevation.md)).
//!
//! This module is the seam between the DM adapters and the host:
//!
//! * [`DmFiles`] is the adapter-facing file API. [`LocalFiles`] is a plain
//!   direct read/write (fixtures and user-owned `~/.dmrc`); [`ElevatedFiles`]
//!   retries a denied operation through [`Privilege`].
//! * [`HostPrivilege`] raises `pkexec` (a graphical polkit prompt, so the
//!   shell's `--return` path works without a terminal) or `sudo`. A refusal is
//!   a visible [`io::ErrorKind::PermissionDenied`], never a silent success.
//! * [`AccountsService`] is GDM's documented system-bus API. GDM prefers
//!   `SetXSession` over editing the daemon's own file; a host without the
//!   D-Bus tool falls back to the elevated file path.
//!
//! Elevation is used only for the one DM file, never for the session itself.

use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

/// Read, write, and remove display-manager state files.
pub trait DmFiles: Send + Sync {
    /// The file's text, or `None` when it does not exist.
    fn read(&self, path: &Path) -> io::Result<Option<String>>;
    /// Write `text`, creating parents. An existing file is replaced.
    fn write(&self, path: &Path, text: &str) -> io::Result<()>;
    /// Remove `path`; a missing file is not an error.
    fn remove(&self, path: &Path) -> io::Result<()>;
}

/// The plain filesystem: what a fixture uses and what a user-owned file needs.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalFiles;

impl DmFiles for LocalFiles {
    fn read(&self, path: &Path) -> io::Result<Option<String>> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

/// A host privilege helper: `pkexec` (polkit, graphical) or `sudo`.
pub trait Privilege: Send + Sync {
    /// Whether a helper is installed at all.
    fn available(&self) -> bool;
    /// Write `text` to `path` as root, creating parents.
    fn write(&self, path: &Path, text: &str) -> io::Result<()>;
    /// Read `path` as root. A missing file is `None`.
    fn read(&self, path: &Path) -> io::Result<Option<String>>;
    /// Remove `path` as root; a missing file is not an error.
    fn remove(&self, path: &Path) -> io::Result<()>;
}

/// The live host's polkit/sudo helper.
#[derive(Debug, Clone)]
pub struct HostPrivilege {
    program: Option<&'static str>,
}

impl HostPrivilege {
    /// Resolve the helper once. `pkexec` is preferred: the shell's return
    /// action has no terminal, so `sudo` cannot read a password there.
    pub fn new() -> Self {
        let program = if crate::demo::which("pkexec").is_some() {
            Some("pkexec")
        } else if crate::demo::which("sudo").is_some() {
            Some("sudo")
        } else {
            None
        };
        HostPrivilege { program }
    }

    fn program(&self) -> io::Result<&'static str> {
        self.program.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the display-manager file is root-owned and neither pkexec nor sudo is installed",
            )
        })
    }

    fn run(&self, args: &[&str]) -> io::Result<Output> {
        let program = self.program()?;
        Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .output()
    }
}

impl Default for HostPrivilege {
    fn default() -> Self {
        Self::new()
    }
}

impl Privilege for HostPrivilege {
    fn available(&self) -> bool {
        self.program.is_some()
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        let program = self.program()?;
        // `tee` keeps the existing file's mode/ownership and truncates in
        // place; `install` would reset the mode, which matters for the
        // 0600 AccountsService user file.
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                let parent_text = parent.to_string_lossy();
                let output = self.run(&["install", "-d", "-m", "0755", parent_text.as_ref()])?;
                if !output.status.success() {
                    return Err(classify(&output, parent));
                }
            }
        }
        let path_text = path.to_string_lossy();
        let mut child = match Command::new(program)
            .args(["tee", "--", path_text.as_ref()])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{program} is not available to write {}", path.display()),
                ))
            }
            Err(error) => return Err(error),
        };
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        let output = child.wait_with_output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(classify(&output, path))
        }
    }

    fn read(&self, path: &Path) -> io::Result<Option<String>> {
        let path_text = path.to_string_lossy();
        match self.run(&["cat", "--", path_text.as_ref()]) {
            Ok(output) if output.status.success() => {
                Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
            }
            Ok(output) if is_missing(&output) => Ok(None),
            Ok(output) => Err(classify(&output, path)),
            Err(error) => Err(error),
        }
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        let path_text = path.to_string_lossy();
        match self.run(&["rm", "-f", "--", path_text.as_ref()]) {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(classify(&output, path)),
            Err(error) => Err(error),
        }
    }
}

/// Direct access with a privileged retry when the host refuses the write.
pub struct ElevatedFiles {
    direct: Box<dyn DmFiles>,
    privilege: Box<dyn Privilege>,
}

impl ElevatedFiles {
    /// Direct access first, then `privilege`.
    pub fn new(direct: Box<dyn DmFiles>, privilege: Box<dyn Privilege>) -> Self {
        ElevatedFiles { direct, privilege }
    }

    /// The live host: the filesystem plus `pkexec`/`sudo`.
    pub fn host() -> Self {
        Self::new(Box::new(LocalFiles), Box::new(HostPrivilege::new()))
    }
}

impl DmFiles for ElevatedFiles {
    fn read(&self, path: &Path) -> io::Result<Option<String>> {
        match self.direct.read(path) {
            Ok(value) => Ok(value),
            Err(error) if denied(&error) && self.privilege.available() => self.privilege.read(path),
            Err(error) => Err(error),
        }
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        match self.direct.write(path, text) {
            Ok(()) => Ok(()),
            Err(error) if denied(&error) => {
                if !self.privilege.available() {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!(
                            "cannot write {}: permission denied and no elevation helper is installed",
                            path.display()
                        ),
                    ));
                }
                self.privilege.write(path, text)
            }
            Err(error) => Err(error),
        }
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        match self.direct.remove(path) {
            Ok(()) => Ok(()),
            Err(error) if denied(&error) && self.privilege.available() => {
                self.privilege.remove(path)
            }
            Err(error) => Err(error),
        }
    }
}

/// GDM's AccountsService API (`org.freedesktop.Accounts` on the system bus).
pub trait AccountsService: Send + Sync {
    /// Whether the host tool needed to reach the API is installed.
    fn available(&self) -> bool;
    /// The user's explicit `XSession`, if any.
    fn xsession(&self, user: &str) -> io::Result<Option<String>>;
    /// Set the user's `XSession`; an empty string clears it.
    fn set_xsession(&self, user: &str, session: &str) -> io::Result<()>;
}

/// Runs one host command. Tests replace this with a recorded fake.
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<Output>;
}

/// The real command runner.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<Output> {
        Command::new(program).args(args).output()
    }
}

/// The `gdbus`-backed AccountsService client (the dev tool links no D-Bus
/// crate, matching the rest of the harness).
pub struct GdbusAccounts {
    program: String,
    runner: Arc<dyn CommandRunner>,
}

impl GdbusAccounts {
    /// A client over the host's `gdbus`, or `None` when it is not installed.
    pub fn detect() -> Option<Self> {
        let path = crate::demo::which("gdbus")?;
        Some(GdbusAccounts {
            program: path.to_string_lossy().into_owned(),
            runner: Arc::new(SystemRunner),
        })
    }

    /// A client over an explicit program and runner (tests).
    #[allow(dead_code)]
    pub fn new(program: impl Into<String>, runner: Arc<dyn CommandRunner>) -> Self {
        GdbusAccounts {
            program: program.into(),
            runner,
        }
    }

    fn call(&self, object_path: &str, method: &str, args: &[&str]) -> io::Result<String> {
        let mut argv = vec![
            "call",
            "--system",
            "--timeout",
            "5",
            "--dest",
            "org.freedesktop.Accounts",
            "--object-path",
            object_path,
            "--method",
            method,
        ];
        argv.extend_from_slice(args);
        let output = self.runner.run(&self.program, &argv)?;
        if !output.status.success() {
            return Err(classify(&output, Path::new(method)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn user_path(&self, user: &str) -> io::Result<String> {
        let quoted = quote(user);
        let reply = self.call(
            "/org/freedesktop/Accounts",
            "org.freedesktop.Accounts.FindUserByName",
            &[quoted.as_str()],
        )?;
        parse_object_path(&reply)
    }
}

impl AccountsService for GdbusAccounts {
    fn available(&self) -> bool {
        true
    }

    fn xsession(&self, user: &str) -> io::Result<Option<String>> {
        let path = self.user_path(user)?;
        let reply = self.call(
            &path,
            "org.freedesktop.DBus.Properties.Get",
            &["'org.freedesktop.Accounts.User'", "'XSession'"],
        )?;
        parse_variant_string(&reply)
    }

    fn set_xsession(&self, user: &str, session: &str) -> io::Result<()> {
        let path = self.user_path(user)?;
        let quoted = quote(session);
        self.call(
            &path,
            "org.freedesktop.Accounts.User.SetXSession",
            &[quoted.as_str()],
        )
        .map(|_| ())
    }
}

/// Whether an I/O error is the host refusing access.
pub(crate) fn denied(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied
        || matches!(error.raw_os_error(), Some(libc::EACCES | libc::EPERM))
}

/// Quote one GVariant string argument for `gdbus`.
fn quote(value: &str) -> String {
    format!("'{value}'")
}

/// Extract the object path from a `FindUserByName` reply
/// (`(objectpath '/org/freedesktop/Accounts/User1000',)`).
fn parse_object_path(reply: &str) -> io::Result<String> {
    let marker = "objectpath '";
    let start = reply.find(marker).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("AccountsService did not return a user object: {reply}"),
        )
    })? + marker.len();
    let rest = &reply[start..];
    let end = rest.find('\'').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("AccountsService returned a malformed object path: {reply}"),
        )
    })?;
    Ok(rest[..end].to_string())
}

/// Extract the first quoted string from a property reply
/// (`(<'dragonfruit'>,)`); an empty string is no explicit selection.
fn parse_variant_string(reply: &str) -> io::Result<Option<String>> {
    let start = reply.find('\'').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("AccountsService did not return XSession: {reply}"),
        )
    })? + 1;
    let rest = &reply[start..];
    let end = rest.find('\'').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("AccountsService returned a malformed XSession: {reply}"),
        )
    })?;
    let value = &rest[..end];
    Ok((!value.is_empty()).then(|| value.to_string()))
}

fn output_text(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    text.trim().to_owned()
}

fn is_missing(output: &Output) -> bool {
    let text = output_text(output).to_ascii_lowercase();
    text.contains("no such file") || text.contains("does not exist")
}

/// Map a refused host command to a permission error, everything else to a
/// generic failure. Both carry the tool's message so the arm can print it.
fn classify(output: &Output, path: &Path) -> io::Error {
    let text = output_text(output);
    let lower = text.to_ascii_lowercase();
    let refusal = lower.contains("not authorized")
        || lower.contains("permission denied")
        || lower.contains("operation not permitted")
        || lower.contains("authentication")
        || lower.contains("incorrect password")
        || lower.contains("dismissed")
        || lower.contains("must have a tty")
        || lower.contains("a terminal is required")
        || lower.contains("no password")
        || lower.contains("policy");
    let message = if text.is_empty() {
        format!(
            "the host refused the display-manager write to {}",
            path.display()
        )
    } else {
        format!("{}: {text}", path.display())
    };
    if refusal {
        io::Error::new(io::ErrorKind::PermissionDenied, message)
    } else {
        io::Error::other(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::ExitStatus;
    use std::sync::Mutex;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "dragonfruit-t344-access-{tag}-{}-{nonce}",
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

    /// A direct backend that refuses every write, as a root-owned file does.
    struct ReadOnlyFiles;

    impl DmFiles for ReadOnlyFiles {
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

    /// A privilege helper that writes into the same scratch tree and records
    /// every call.
    struct FakePrivilege {
        calls: Mutex<Vec<String>>,
    }

    impl FakePrivilege {
        fn new() -> Self {
            FakePrivilege {
                calls: Mutex::new(Vec::new()),
            }
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

    #[test]
    fn a_denied_direct_write_is_retried_through_the_privilege() {
        let scratch = Scratch::new("retry");
        let path = scratch.0.join("var/lib/sddm/state.conf");
        let files = ElevatedFiles::new(Box::new(ReadOnlyFiles), Box::new(FakePrivilege::new()));
        files
            .write(&path, "[LastUser]\nSession=dragonfruit\n")
            .expect("elevated write");
        assert_eq!(
            std::fs::read_to_string(&path).expect("file"),
            "[LastUser]\nSession=dragonfruit\n"
        );
    }

    #[test]
    fn a_denied_write_without_a_helper_is_a_loud_permission_error() {
        struct NoPrivilege;
        impl Privilege for NoPrivilege {
            fn available(&self) -> bool {
                false
            }
            fn write(&self, _path: &Path, _text: &str) -> io::Result<()> {
                unreachable!("not available")
            }
            fn read(&self, _path: &Path) -> io::Result<Option<String>> {
                unreachable!("not available")
            }
            fn remove(&self, _path: &Path) -> io::Result<()> {
                unreachable!("not available")
            }
        }

        let scratch = Scratch::new("no-helper");
        let path = scratch.0.join("var/lib/AccountsService/users/dfdev");
        let files = ElevatedFiles::new(Box::new(ReadOnlyFiles), Box::new(NoPrivilege));
        let error = files.write(&path, "[User]\n").expect_err("denied");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("no elevation helper"), "{error}");
    }

    #[test]
    fn elevated_read_and_remove_are_idempotent() {
        let scratch = Scratch::new("read-remove");
        let path = scratch.0.join("var/lib/AccountsService/users/dfdev");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, "[User]\nXSession=existing\n").expect("seed");
        let files = ElevatedFiles::new(Box::new(ReadOnlyFiles), Box::new(FakePrivilege::new()));
        assert_eq!(
            files.read(&path).expect("read").as_deref(),
            Some("[User]\nXSession=existing\n")
        );
        files.remove(&path).expect("remove");
        assert!(!path.exists());
        files.remove(&path).expect("second remove");
    }

    #[test]
    fn a_missing_elevated_file_reads_none() {
        let scratch = Scratch::new("missing");
        let path = scratch.0.join("var/lib/sddm/state.conf");
        let files = ElevatedFiles::new(Box::new(ReadOnlyFiles), Box::new(FakePrivilege::new()));
        assert_eq!(files.read(&path).expect("read"), None);
    }

    struct FakeRunner {
        replies: Mutex<Vec<(String, io::Result<String>)>>,
        calls: Mutex<Vec<String>>,
    }

    impl FakeRunner {
        fn new(replies: Vec<(&str, &str)>) -> Self {
            FakeRunner {
                replies: Mutex::new(
                    replies
                        .into_iter()
                        .map(|(method, reply)| (method.to_string(), Ok(reply.to_string())))
                        .collect(),
                ),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls").clone()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, program: &str, args: &[&str]) -> io::Result<Output> {
            let joined = format!("{program} {}", args.join(" "));
            self.calls.lock().expect("calls").push(joined);
            let method = args
                .iter()
                .position(|arg| *arg == "--method")
                .and_then(|index| args.get(index + 1))
                .copied()
                .unwrap_or("");
            let mut replies = self.replies.lock().expect("replies");
            let position = replies
                .iter()
                .position(|(candidate, _)| candidate == method)
                .unwrap_or(0);
            let (_, reply) = replies.remove(position);
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: reply.unwrap_or_default().into_bytes(),
                stderr: Vec::new(),
            })
        }
    }

    #[test]
    fn gdbus_reads_the_user_path_and_xsession() {
        let runner = Arc::new(FakeRunner::new(vec![
            (
                "org.freedesktop.Accounts.FindUserByName",
                "(objectpath '/org/freedesktop/Accounts/User1000',)",
            ),
            ("org.freedesktop.DBus.Properties.Get", "(<'dragonfruit'>,)"),
        ]));
        let accounts = GdbusAccounts::new("gdbus", runner.clone());
        assert_eq!(
            accounts.xsession("dfdev").expect("xsession").as_deref(),
            Some("dragonfruit")
        );
        let calls = runner.calls();
        assert!(calls[0].contains("FindUserByName 'dfdev'"), "{calls:?}");
        assert!(calls[1].contains("--object-path /org/freedesktop/Accounts/User1000"));
        assert!(calls[1].contains("'org.freedesktop.Accounts.User' 'XSession'"));
    }

    #[test]
    fn gdbus_sets_xsession_on_the_resolved_user_object() {
        let runner = Arc::new(FakeRunner::new(vec![
            (
                "org.freedesktop.Accounts.FindUserByName",
                "(objectpath '/org/freedesktop/Accounts/User1000',)",
            ),
            ("org.freedesktop.Accounts.User.SetXSession", "()"),
        ]));
        let accounts = GdbusAccounts::new("gdbus", runner.clone());
        accounts.set_xsession("dfdev", "dragonfruit").expect("set");
        let calls = runner.calls();
        assert!(
            calls[1].contains("org.freedesktop.Accounts.User.SetXSession 'dragonfruit'"),
            "{calls:?}"
        );
    }

    #[test]
    fn an_empty_xsession_is_no_explicit_selection() {
        assert_eq!(parse_variant_string("(<''>,)").expect("parse"), None);
        assert_eq!(
            parse_variant_string("(<'existing'>,)")
                .expect("parse")
                .as_deref(),
            Some("existing")
        );
    }

    #[test]
    fn a_malformed_accounts_reply_is_an_error() {
        assert!(parse_object_path("()").is_err());
        assert!(parse_variant_string("()").is_err());
    }

    #[test]
    fn local_files_round_trip_and_remove() {
        let scratch = Scratch::new("local");
        let path = scratch.0.join("etc/sddm.conf.d/autologin.conf");
        LocalFiles.write(&path, "[Autologin]\n").expect("write");
        assert_eq!(
            LocalFiles.read(&path).expect("read").as_deref(),
            Some("[Autologin]\n")
        );
        LocalFiles.remove(&path).expect("remove");
        assert_eq!(LocalFiles.read(&path).expect("read"), None);
        LocalFiles.remove(&path).expect("remove again");
    }
}
