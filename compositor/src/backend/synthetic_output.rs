// SPDX-License-Identifier: MIT
//! Synthetic output harness for the headless backend (T-09 FR-1).
//!
//! The headless backend creates exactly one static output, so output
//! hotplug could not be scripted. This module is an opt-in test channel:
//! when `DRAGONFRUIT_SYNTHETIC_OUTPUT` names a `UnixDatagram` path, the
//! compositor watches it for `add`/`remove` commands and drives the same
//! [`crate::state::DfState::on_output_added`] /
//! [`crate::state::DfState::on_output_removed`] path as a DRM hotplug, so
//! the shell re-anchors its chrome and the manager re-syncs its output and
//! workspace objects exactly as it would on real hardware.
//!
//! Wire format (one command per line; a datagram may batch lines):
//!
//! ```text
//! add <name> <width> <height> <x> <y> [scale]
//! remove <name>
//! ```
//!
//! Like the T-03 synthetic-input harness, this is test plumbing: only the
//! headless backend installs it, only when the environment variable is set,
//! and the socket is removed at teardown.

use std::collections::HashMap;
use std::os::unix::net::UnixDatagram;
use std::path::Path;

use smithay::output::{Mode, Output};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, Mode as CalloopMode, PostAction};

use crate::backend::{add_output, headless_physical_properties};
use crate::state::DfState;

/// The environment variable naming the synthetic-output socket. Unset in a
/// normal session; set by headless protocol tests.
pub const ENV_SYNTHETIC_OUTPUT: &str = "DRAGONFRUIT_SYNTHETIC_OUTPUT";

/// One parsed synthetic output command.
#[derive(Debug, Clone, PartialEq)]
pub enum SyntheticOutputCommand {
    Add {
        name: String,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
        scale: f64,
    },
    Remove {
        name: String,
    },
}

fn parse_i32(token: &str) -> Result<i32, String> {
    token
        .parse::<i32>()
        .map_err(|_| format!("expected an integer, got {token:?}"))
}

fn parse_f64(token: &str) -> Result<f64, String> {
    token
        .parse::<f64>()
        .map_err(|_| format!("expected a number, got {token:?}"))
}

/// Parse one command line (the inverse of the wire format in the module
/// docs). Unknown verbs and malformed arguments are errors, never panics:
/// a bad test line must not take down the compositor.
pub fn parse_command(line: &str) -> Result<SyntheticOutputCommand, String> {
    let mut parts = line.split_whitespace();
    let Some(verb) = parts.next() else {
        return Err("empty command".into());
    };
    let command = match verb {
        "add" => {
            let name = parts.next().ok_or("add requires a name")?.to_string();
            let width = parse_i32(parts.next().ok_or("add requires a width")?)?;
            let height = parse_i32(parts.next().ok_or("add requires a height")?)?;
            let x = parse_i32(parts.next().ok_or("add requires an x position")?)?;
            let y = parse_i32(parts.next().ok_or("add requires a y position")?)?;
            let scale = parts.next().map(parse_f64).transpose()?.unwrap_or(1.0);
            SyntheticOutputCommand::Add {
                name,
                width,
                height,
                x,
                y,
                scale,
            }
        }
        "remove" => SyntheticOutputCommand::Remove {
            name: parts.next().ok_or("remove requires a name")?.to_string(),
        },
        other => return Err(format!("unknown command {other:?}")),
    };
    if parts.next().is_some() {
        return Err(format!("trailing tokens after {verb:?}"));
    }
    Ok(command)
}

/// Apply one command against the live session.
///
/// `live` owns the `Output` objects the harness created, because removing an
/// output needs the exact instance that was mapped. Returns whether the
/// session changed.
pub fn apply_command(
    state: &mut DfState,
    live: &mut HashMap<String, Output>,
    command: SyntheticOutputCommand,
) -> bool {
    match command {
        SyntheticOutputCommand::Add {
            name,
            width,
            height,
            x,
            y,
            scale,
        } => {
            if width <= 0 || height <= 0 || !scale.is_finite() || scale <= 0.0 {
                eprintln!(
                    "dragonfruit-compositor: refusing synthetic output {name:?}: \
                     bad size/scale {width}x{height}@{scale}"
                );
                return false;
            }
            if live.contains_key(&name) {
                eprintln!("dragonfruit-compositor: synthetic output {name:?} already exists");
                return false;
            }
            let output = add_output(
                state,
                &name,
                headless_physical_properties(),
                Mode {
                    size: (width, height).into(),
                    refresh: 60_000,
                },
                (x, y),
                scale,
            );
            live.insert(name, output);
            true
        }
        SyntheticOutputCommand::Remove { name } => {
            let Some(output) = live.remove(&name) else {
                eprintln!("dragonfruit-compositor: no synthetic output {name:?} to remove");
                return false;
            };
            // Same ordering as the DRM hotplug path: migrate this output's
            // windows to the remaining primary before its Spaces vanish,
            // then unmap it from the scene.
            state.on_output_removed(&output);
            state.space.unmap_output(&output);
            true
        }
    }
}

/// Apply a datagram payload (one or more newline-separated commands).
///
/// Returns the number of commands applied. Malformed lines are logged and
/// skipped, never fatal.
pub fn apply_datagram(
    state: &mut DfState,
    live: &mut HashMap<String, Output>,
    payload: &[u8],
) -> usize {
    let text = String::from_utf8_lossy(payload);
    let mut applied = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_command(line) {
            Ok(command) => {
                apply_command(state, live, command);
                applied += 1;
            }
            Err(err) => {
                eprintln!("dragonfruit-compositor: bad synthetic output {line:?}: {err}");
            }
        }
    }
    applied
}

/// Bind the synthetic-output socket and insert its event source.
///
/// Only the headless backend calls this, and only when
/// [`ENV_SYNTHETIC_OUTPUT`] is set; the socket is removed by the caller at
/// teardown.
pub fn install(state: &mut DfState, path: &Path) -> Result<(), String> {
    // A stale socket from a crashed run would make bind fail; the tests
    // use a unique path per process, but be defensive.
    let _ = std::fs::remove_file(path);
    let socket = UnixDatagram::bind(path).map_err(|e| {
        format!(
            "failed to bind synthetic-output socket {}: {e}",
            path.display()
        )
    })?;
    socket
        .set_nonblocking(true)
        .map_err(|e| format!("failed to set synthetic-output socket nonblocking: {e}"))?;
    let mut live: HashMap<String, Output> = HashMap::new();
    let _ = state
        .loop_handle
        .insert_source(
            Generic::new(socket, Interest::READ, CalloopMode::Level),
            move |_, socket, state| {
                let mut buf = [0u8; 4096];
                loop {
                    match socket.recv_from(&mut buf) {
                        Ok((len, _)) => {
                            apply_datagram(state, &mut live, &buf[..len]);
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: synthetic-output socket error: {err}"
                            );
                            break;
                        }
                    }
                }
                Ok(PostAction::Continue)
            },
        )
        .map_err(|e| format!("failed to register synthetic-output source: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_command_shapes() {
        assert_eq!(
            parse_command("add HDMI-A-1 1920 1080 1280 0").unwrap(),
            SyntheticOutputCommand::Add {
                name: "HDMI-A-1".into(),
                width: 1920,
                height: 1080,
                x: 1280,
                y: 0,
                scale: 1.0,
            }
        );
        assert_eq!(
            parse_command("add eDP-1 2560 1440 0 0 1.5").unwrap(),
            SyntheticOutputCommand::Add {
                name: "eDP-1".into(),
                width: 2560,
                height: 1440,
                x: 0,
                y: 0,
                scale: 1.5,
            }
        );
        assert_eq!(
            parse_command("remove HDMI-A-1").unwrap(),
            SyntheticOutputCommand::Remove {
                name: "HDMI-A-1".into()
            }
        );
    }

    #[test]
    fn rejects_malformed_commands_without_panicking() {
        assert!(parse_command("").is_err());
        assert!(parse_command("add").is_err());
        assert!(parse_command("add DP-1 1920").is_err());
        assert!(parse_command("add DP-1 wide high 0 0").is_err());
        assert!(parse_command("add DP-1 1920 1080 0 0 1.0 extra").is_err());
        assert!(parse_command("remove").is_err());
        assert!(parse_command("teleport DP-1").is_err());
    }
}
