// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dragonfruit compositor (T-02): one session, three backends.
//!
//! The compositor owns the seat, the scene graph, and the standard
//! Wayland protocol surface; everything else (windows/workspaces policy,
//! shell protocols, Xwayland) is built on this base by later tickets.
//! Backend selection is a flag, not a fork (FR-1):
//!
//! ```text
//! dragonfruit-compositor --backend nested|drm|headless [--socket-name NAME]
//! ```

use std::process::ExitCode;

mod backend;
mod identity;
mod input;
mod render;
mod session;
mod state;
mod window;
mod workspace;
mod xwayland;

pub use df_ipc::LOCKSTEP_VERSION;

const EXIT_USAGE: u8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Run as a window on the host Wayland session (daily development).
    Nested,
    /// Own the physical display via DRM/KMS (real sessions, logind seat).
    Drm,
    /// Run without any display (CI, soak tests).
    Headless,
}

pub struct Args {
    pub backend: Backend,
    pub socket_name: Option<String>,
}

fn parse_args(iter: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        backend: Backend::Nested,
        socket_name: None,
    };
    let mut it = iter;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--backend" => match it.next().as_deref() {
                Some("nested") => args.backend = Backend::Nested,
                Some("drm") => args.backend = Backend::Drm,
                Some("headless") => args.backend = Backend::Headless,
                other => {
                    return Err(format!(
                        "unknown backend {other:?}, expected `nested`, `drm`, or `headless`"
                    ))
                }
            },
            "--socket-name" => {
                let Some(name) = it.next() else {
                    return Err("--socket-name requires a value".into());
                };
                if name.len() > 100 {
                    return Err("socket name too long for a Unix socket path".into());
                }
                args.socket_name = Some(name);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(args)
}

fn print_help() {
    println!(
        "dragonfruit-compositor — Dragonfruit Wayland compositor\n\
         \n\
         USAGE:\n\
         \x20   dragonfruit-compositor [--backend nested|drm|headless] [--socket-name NAME]\n\
         \n\
         The Wayland socket is created in $XDG_RUNTIME_DIR and removed on exit."
    );
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("dragonfruit-compositor: {err}");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    // Lockstep from the first commit: compositor, shell, and protocol XMLs
    // ship as one set; cross-version mixing is rejected (docs/ipc-versioning.md).
    df_ipc::assert_lockstep_compatible(LOCKSTEP_VERSION)
        .expect("the built-in lockstep version must be compatible with itself");

    let socket_name = args
        .socket_name
        .unwrap_or_else(|| format!("dragonfruit-{}", std::process::id()));

    let result = match args.backend {
        Backend::Headless => backend::headless::run(&socket_name),
        Backend::Nested => backend::nested::run(&socket_name),
        Backend::Drm => backend::drm::run(&socket_name),
    };

    match result {
        Ok(()) => {
            println!("dragonfruit-compositor: clean exit");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("dragonfruit-compositor: {err}");
            ExitCode::FAILURE
        }
    }
}
