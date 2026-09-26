// SPDX-License-Identifier: MIT
//! `dragonfruit-pam-helper` — the small process boundary for lock-screen PAM
//! authentication (T-12.3b).
//!
//! ```text
//! dragonfruit-pam-helper --user USER [--service SERVICE] [--confdir DIR]
//! ```
//!
//! The password is read from standard input (one line); the result is the
//! exit status, and **nothing else**. The password is never written to disk,
//! logged, or echoed:
//!
//! ```text
//! exit 0  authenticated
//! exit 1  credentials rejected
//! exit 2  authentication unavailable / usage error
//! ```
//!
//! `--confdir` is the test seam: it points PAM at a throwaway configuration
//! directory (Linux-PAM `pam_start_confdir`) so the headless suite can run the
//! real libpam against a `permit`/`deny` service. Production omits it.

use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::ExitCode;

use dragonfruit_lock_auth::{AuthResult, Authenticator, PamAuthenticator, DEFAULT_SERVICE};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (user, service, confdir) = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(ParseOutcome::Help) => {
            print_usage();
            return ExitCode::SUCCESS;
        }
        Err(ParseOutcome::Usage(message)) => {
            eprintln!("dragonfruit-pam-helper: {message}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    let password = match read_password() {
        Ok(password) => password,
        Err(error) => {
            eprintln!("dragonfruit-pam-helper: cannot read the password: {error}");
            return ExitCode::from(2);
        }
    };

    let mut authenticator = PamAuthenticator::new().with_service(service);
    if let Some(dir) = confdir {
        authenticator = authenticator.with_confdir(dir);
    }
    let result = authenticator.authenticate(&user, &password);

    match result {
        AuthResult::Success => ExitCode::SUCCESS,
        AuthResult::Denied => {
            eprintln!("dragonfruit-pam-helper: authentication failed");
            ExitCode::from(1)
        }
        AuthResult::Error => {
            eprintln!("dragonfruit-pam-helper: authentication unavailable");
            ExitCode::from(2)
        }
    }
}

enum ParseOutcome {
    Help,
    Usage(String),
}

fn parse_args(args: &[String]) -> Result<(String, String, Option<PathBuf>), ParseOutcome> {
    let mut user: Option<String> = None;
    let mut service = DEFAULT_SERVICE.to_string();
    let mut confdir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].as_str();
        let mut take = |name: &str| -> Result<String, ParseOutcome> {
            index += 1;
            args.get(index)
                .cloned()
                .ok_or_else(|| ParseOutcome::Usage(format!("{name} needs a value")))
        };
        match argument {
            "-h" | "--help" => return Err(ParseOutcome::Help),
            "-u" | "--user" => user = Some(take("--user")?),
            "-s" | "--service" => service = take("--service")?,
            "--confdir" => confdir = Some(PathBuf::from(take("--confdir")?)),
            other => {
                return Err(ParseOutcome::Usage(format!("unknown argument {other:?}")));
            }
        }
        index += 1;
    }
    let Some(user) = user else {
        return Err(ParseOutcome::Usage("--user is required".to_string()));
    };
    Ok((user, service, confdir))
}

/// Read the password from standard input. When standard input is a terminal
/// the echo bit is turned off for the read and restored afterwards; a pipe
/// (the shell's path, and every test) needs no terminal handling.
fn read_password() -> io::Result<String> {
    let guard = EchoGuard::suppress();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    drop(guard);
    Ok(line.trim_end_matches(&['\n', '\r'][..]).to_string())
}

fn print_usage() {
    eprintln!(
        "usage: dragonfruit-pam-helper --user USER [--service SERVICE] [--confdir DIR]\n\
         \n\
         Reads one password line from standard input and verifies it through PAM.\n\
         Exit status: 0 authenticated, 1 rejected, 2 unavailable."
    );
}

/// Turns off terminal echo for the duration of a password read. A no-op
/// unless standard input is a terminal; best-effort by design (a failure to
/// restore echo would be worse than an echoed password, so the saved state is
/// always restored).
struct EchoGuard {
    saved: libc::termios,
}

impl EchoGuard {
    fn suppress() -> Option<Self> {
        // SAFETY: `isatty`/`tcgetattr`/`tcsetattr` are the standard termios
        // calls on file descriptor 0; `saved` is fully initialised by
        // `tcgetattr` before it is used.
        unsafe {
            if libc::isatty(0) != 1 {
                return None;
            }
            let mut saved: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(0, &mut saved) != 0 {
                return None;
            }
            let mut quiet = saved;
            quiet.c_lflag &= !(libc::ECHO as libc::tcflag_t);
            if libc::tcsetattr(0, libc::TCSANOW, &quiet) != 0 {
                return None;
            }
            Some(Self { saved })
        }
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        // SAFETY: `saved` was captured by `tcgetattr` on descriptor 0.
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.saved);
        }
    }
}
