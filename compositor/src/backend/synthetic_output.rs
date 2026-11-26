// SPDX-License-Identifier: GPL-3.0-only
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
//! add <name> <width> <height> <x> <y> [scale] [refresh_mhz]
//! remove <name>
//! ```
//!
//! `refresh_mhz` is the optional mode refresh in millihertz (default 60000 =
//! 60 Hz). The mixed-refresh trace uses it to add a synthetic 144 Hz output
//! beside the headless 60 Hz output.
//!
//! Like the T-03 synthetic-input harness, this is test plumbing: only the
//! headless backend installs it, only when the environment variable is set,
//! and the socket is removed at teardown.

use std::collections::HashMap;
use std::os::unix::net::UnixDatagram;
use std::path::Path;

use smithay::output::Mode;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, Mode as CalloopMode, PostAction};

use crate::backend::{add_output, headless_physical_properties, remove_output, AddedOutput};
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
        refresh_mhz: i32,
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
            let refresh_mhz = parts.next().map(parse_i32).transpose()?.unwrap_or(60_000);
            SyntheticOutputCommand::Add {
                name,
                width,
                height,
                x,
                y,
                scale,
                refresh_mhz,
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
/// `live` owns the `Output` objects and their one `wl_output` global that the
/// harness created, because removing an output needs the exact instance that
/// was mapped. Returns whether the session changed.
pub fn apply_command(
    state: &mut DfState,
    live: &mut HashMap<String, AddedOutput>,
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
            refresh_mhz,
        } => {
            if width <= 0 || height <= 0 || !scale.is_finite() || scale <= 0.0 || refresh_mhz <= 0 {
                eprintln!(
                    "dragonfruit-compositor: refusing synthetic output {name:?}: \
                     bad size/scale/refresh {width}x{height}@{scale} {refresh_mhz}mHz"
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
                    refresh: refresh_mhz,
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
            // Same teardown as the DRM hotplug path: migrate this output's
            // windows to the remaining primary before its Spaces vanish,
            // unmap it from the scene, and remove its one `wl_output` global.
            remove_output(state, &output.output, output.global);
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
    live: &mut HashMap<String, AddedOutput>,
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
    let mut live: HashMap<String, AddedOutput> = HashMap::new();
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
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use smithay::reexports::wayland_server::{Display, DisplayHandle};
    use wayland_client::protocol::wl_registry;
    use wayland_client::{Connection, Dispatch, EventQueue};

    use crate::state::{DfClientState, DfState};

    /// A registry-only client: it counts the live `wl_output` globals, which is
    /// the client-visible contract of ADR 0315.
    #[derive(Default)]
    struct RegistryClient {
        outputs: Vec<u32>,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for RegistryClient {
        fn event(
            state: &mut RegistryClient,
            _registry: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            _: &wayland_client::QueueHandle<Self>,
        ) {
            match event {
                wl_registry::Event::Global {
                    name, interface, ..
                } if interface == "wl_output" => state.outputs.push(name),
                wl_registry::Event::GlobalRemove { name } => {
                    state.outputs.retain(|global| *global != name);
                }
                _ => {}
            }
        }
    }

    /// Pump the in-process server and client once. The compositor normally
    /// runs this through calloop; the test drives the same `Display` by hand.
    fn pump(
        display: &mut Display<DfState>,
        state: &mut DfState,
        conn: &Connection,
        queue: &mut EventQueue<RegistryClient>,
        client: &mut RegistryClient,
    ) {
        conn.flush().expect("flush the test client");
        display.dispatch_clients(state).expect("dispatch server");
        display.flush_clients().expect("flush server");
        // `dispatch_pending` does not read the socket; poll it without
        // blocking so the test never hangs on an empty queue.
        if let Some(guard) = conn.prepare_read() {
            use std::os::fd::AsRawFd;
            let mut pollfd = libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            if unsafe { libc::poll(&mut pollfd, 1, 0) } > 0 {
                let _ = guard.read();
            }
        }
        queue.dispatch_pending(client).expect("dispatch client");
    }

    #[track_caller]
    fn wait_for_output_count(
        display: &mut Display<DfState>,
        state: &mut DfState,
        conn: &Connection,
        queue: &mut EventQueue<RegistryClient>,
        client: &mut RegistryClient,
        expected: usize,
    ) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            pump(display, state, conn, queue, client);
            if client.outputs.len() == expected {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "expected {expected} wl_output globals, saw {:?}",
                client.outputs
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// T323 acceptance: the synthetic harness advertises exactly one
    /// `wl_output` global per added output and removes that global when the
    /// output is detached. The client is a real Wayland registry, so this is
    /// the same view an app has.
    #[test]
    fn add_and_remove_track_exactly_one_wl_output_global() {
        let event_loop: calloop::EventLoop<'static, DfState> =
            calloop::EventLoop::try_new().expect("event loop");
        let mut display: Display<DfState> = Display::new().expect("display");
        let mut dh: DisplayHandle = display.handle();
        let mut state = DfState::new(&dh, event_loop.handle(), event_loop.get_signal());

        let (server, client_stream) = std::os::unix::net::UnixStream::pair().expect("socket pair");
        dh.insert_client(server, Arc::new(DfClientState::default()))
            .expect("insert client");
        let conn = Connection::from_socket(client_stream).expect("client connection");
        let mut queue = conn.new_event_queue();
        let _registry = conn.display().get_registry(&queue.handle(), ());
        let mut client = RegistryClient::default();
        let mut live = HashMap::new();

        wait_for_output_count(&mut display, &mut state, &conn, &mut queue, &mut client, 0);

        assert!(apply_command(
            &mut state,
            &mut live,
            parse_command("add HDMI-A-1 1920 1080 1280 0").unwrap()
        ));
        wait_for_output_count(&mut display, &mut state, &conn, &mut queue, &mut client, 1);

        assert!(apply_command(
            &mut state,
            &mut live,
            parse_command("add DP-2 1280 720 3200 0").unwrap()
        ));
        wait_for_output_count(&mut display, &mut state, &conn, &mut queue, &mut client, 2);

        assert!(apply_command(
            &mut state,
            &mut live,
            parse_command("remove HDMI-A-1").unwrap()
        ));
        wait_for_output_count(&mut display, &mut state, &conn, &mut queue, &mut client, 1);

        assert!(apply_command(
            &mut state,
            &mut live,
            parse_command("remove DP-2").unwrap()
        ));
        wait_for_output_count(&mut display, &mut state, &conn, &mut queue, &mut client, 0);
        assert!(live.is_empty());
    }

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
                refresh_mhz: 60_000,
            }
        );
        assert_eq!(
            parse_command("add eDP-1 2560 1440 0 0 1.5 144000").unwrap(),
            SyntheticOutputCommand::Add {
                name: "eDP-1".into(),
                width: 2560,
                height: 1440,
                x: 0,
                y: 0,
                scale: 1.5,
                refresh_mhz: 144_000,
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
        assert!(parse_command("add DP-1 1920 1080 0 0 1.0 fast").is_err());
        assert!(parse_command("remove").is_err());
        assert!(parse_command("teleport DP-1").is_err());
    }
}
