// SPDX-License-Identifier: GPL-3.0-or-later
//! The portal's live screen-share bridge to the compositor (T335b).
//!
//! [`crate::stream::CompositorWatch`] is the seam that turns a chosen source
//! into a PipeWire node. This module is the shipped implementation of that
//! seam: a small, trusted Wayland client that presents the launch token the
//! compositor minted for the portal backend (session environment, T-24),
//! binds `df_toplevel_manager` at v17, and asks it for one stream per chosen
//! source (`create_screencast`) or releases one (`destroy_screencast`).
//!
//! ## Why a second trusted client, not a shell relay
//!
//! The shell already owns one private-protocol connection, and its manager is
//! bound at v16 because the shell never sends `create_screencast`
//! ([ADR 0345](../../docs/design/adr/0345-screencast-bridge-and-portal-ownership.md)).
//! Trust is per process, not per connection: the compositor mints one token
//! per trusted role, so the portal gets **its own** token and its own
//! connection, and the compositor still releases a portal's streams when that
//! connection goes away. No new socket, framing, or version negotiation is
//! invented — the bridge speaks the existing `df_core` + `df_toplevel_manager`
//! set ([ADR 0346](../../docs/design/adr/0346-screencast-bridge-trusted-client.md)),
//! and its one connection *is* the lifetime of every stream it created.
//!
//! ## Absent, refused, and failed are all named
//!
//! A bridge is built only when the portal was provisioned a token and a
//! compositor socket ([`from_environment`]). Every creation after that is
//! bounded: a refused handshake, a disconnect, a silent compositor, or a
//! compositor that answers `screencast_failed` is a [`WatchError`] carrying the
//! portal's fallback label — never a placeholder node id. The picker names the
//! stills fallback, and a live session never degrades silently.
//!
//! ## Threads
//!
//! Wayland event queues are per thread, so one [`ScreenCastBridge::connect`]
//! opens its connection, authenticates, and binds the globals on the thread
//! that will later issue requests; [`CompositorWatch::create`] rejects a call
//! from another thread rather than corrupting the queue. The bridge is owned by
//! the registry's mutex, which is what orders a request against the one read
//! loop.

use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use wayland_client::{
    event_created_child,
    globals::{registry_queue_init, GlobalListContents},
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};

/// The registry's global list: `registry_queue_init` dispatches into this and
/// the bridge only reads the result.
impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, GlobalListContents>
    for BridgeState
{
    fn event(
        _state: &mut Self,
        _proxy: &wayland_client::protocol::wl_registry::WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

use crate::stream::{CompositorWatch, CreatedNode, FallbackReason, StreamSource, WatchError};

/// Environment variable carrying the portal backend's one-time launch token,
/// minted for the portal role by the compositor's session provisioning. The
/// name lives in [`df_ipc`] so both sides read one definition.
pub const BRIDGE_TOKEN_ENV: &str = df_ipc::SCREENCAST_BRIDGE_TOKEN_ENV;
/// Environment variable naming the compositor's Wayland socket. Defaults to
/// `wayland-0` in the session runtime directory.
pub const BRIDGE_SOCKET_ENV: &str = "DRAGONFRUIT_SCREENCAST_BRIDGE_SOCKET";

/// The longest a create waits for the compositor's answer before naming the
/// fallback. The compositor bounds its own host spawn below this, so a healthy
/// session always answers first.
pub const CREATE_TIMEOUT: Duration = Duration::from_secs(8);

/// The `df_toplevel_manager` version this bridge requests: the first set that
/// carries `create_screencast`/`destroy_screencast`.
pub const MANAGER_VERSION: u32 = 17;

/// The private-protocol lockstep set the handshake presents.
pub const LOCKSTEP_VERSION: u32 = 1;

/// The generated client bindings for the two private interfaces the bridge
/// speaks. Only the globals are dispatched; a manager's child objects
/// (toplevels, outputs, menus) are delegated to no-ops, because this bridge is
/// a stream-only client.
mod wire {
    /// The handshake global.
    pub mod core {
        // The generated 0.31 code spells `wayland_backend::` and `bitflags::`
        // paths unqualified, so both crates come into scope here.
        #![allow(unused_imports, clippy::single_component_path_imports)]
        use bitflags;
        use wayland_backend;
        use wayland_client;
        use wayland_client::protocol::*;

        pub mod __interfaces {
            use wayland_client::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!("../protocols/dragonfruit-core.xml");
        }

        use self::__interfaces::*;

        wayland_scanner::generate_client_code!("../protocols/dragonfruit-core.xml");
    }

    /// The window/stream manager and the child objects a bind creates.
    pub mod toplevel {
        // See `core`: the generated code needs the same unqualified crates.
        #![allow(unused_imports, clippy::single_component_path_imports)]
        use bitflags;
        use wayland_backend;
        use wayland_client;
        use wayland_client::protocol::*;

        pub mod __interfaces {
            use wayland_client::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!("../protocols/dragonfruit-toplevel.xml");
        }

        use self::__interfaces::*;

        wayland_scanner::generate_client_code!("../protocols/dragonfruit-toplevel.xml");
    }
}

use wire::core::df_core;
use wire::toplevel::df_toplevel_manager;

/// What the bridge has heard back: the handshake result and every stream reply,
/// keyed by the token this bridge sent.
#[derive(Debug, Default)]
struct BridgeState {
    /// `Some(Ok(()))` once `df_core.authenticated` agreed the lockstep set,
    /// `Some(Err(reason))` once the compositor refused (it then disconnects).
    authenticated: Option<Result<(), String>>,
    /// One reply per create: the node id, or the named fallback reason.
    replies: Vec<(u32, Result<u32, String>)>,
}

impl BridgeState {
    fn take_reply(&mut self, token: u32) -> Option<Result<u32, String>> {
        let index = self
            .replies
            .iter()
            .position(|(candidate, _)| *candidate == token)?;
        Some(self.replies.remove(index).1)
    }
}

impl Dispatch<df_core::DfCore, ()> for BridgeState {
    fn event(
        state: &mut Self,
        _proxy: &df_core::DfCore,
        event: df_core::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            df_core::Event::Authenticated { lockstep_version } => {
                state.authenticated = Some(if lockstep_version == LOCKSTEP_VERSION {
                    Ok(())
                } else {
                    Err(format!(
                        "the compositor agreed lockstep v{lockstep_version}; this bridge speaks \
                         v{LOCKSTEP_VERSION}"
                    ))
                });
            }
            df_core::Event::Refused { code, message } => {
                state.authenticated = Some(Err(format!(
                    "the compositor refused the screen-share bridge (code {code}): {message}"
                )));
            }
        }
    }
}

impl Dispatch<df_toplevel_manager::DfToplevelManager, ()> for BridgeState {
    fn event(
        state: &mut Self,
        _proxy: &df_toplevel_manager::DfToplevelManager,
        event: df_toplevel_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            df_toplevel_manager::Event::ScreencastNode { token, node_id, .. } => {
                state.replies.push((token, Ok(node_id)))
            }
            df_toplevel_manager::Event::ScreencastFailed { token, reason, .. } => {
                state.replies.push((token, Err(reason)))
            }
            // Every other manager event is the shell's business.
            _ => {}
        }
    }

    event_created_child!(BridgeState, df_toplevel_manager::DfToplevelManager, [
        df_toplevel_manager::EVT_OUTPUT_OPCODE => (wire::toplevel::df_output::DfOutput, ()),
        df_toplevel_manager::EVT_WORKSPACE_OPCODE => (wire::toplevel::df_workspace::DfWorkspace, ()),
        df_toplevel_manager::EVT_TOPLEVEL_OPCODE => (wire::toplevel::df_toplevel::DfToplevel, ()),
    ]);
}

/// The manager's child objects only exist so a bind never traps; their events
/// are not the bridge's business.
macro_rules! delegate_children {
    ($($iface:ty),* $(,)?) => {
        $(
            impl Dispatch<$iface, ()> for BridgeState {
                fn event(
                    _state: &mut Self,
                    _proxy: &$iface,
                    _event: <$iface as Proxy>::Event,
                    _data: &(),
                    _conn: &Connection,
                    _qh: &QueueHandle<Self>,
                ) {
                }
            }
        )*
    };
}

delegate_children!(
    wire::toplevel::df_output::DfOutput,
    wire::toplevel::df_workspace::DfWorkspace,
    wire::toplevel::df_toplevel::DfToplevel,
);

/// The live bridge: one connection, one handshake, one v17 manager.
pub struct ScreenCastBridge {
    connection: Connection,
    manager: df_toplevel_manager::DfToplevelManager,
    queue: EventQueue<BridgeState>,
    state: BridgeState,
    owner: std::thread::ThreadId,
    /// The next stream token. The compositor keys streams by token, so it must
    /// never repeat one that is still live.
    next_token: u32,
    /// The last connection failure, so a health probe (and thus a reconnect)
    /// keeps reporting a dead connection instead of claiming it is live.
    last_error: Option<String>,
}

impl std::fmt::Debug for ScreenCastBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenCastBridge")
            .field("next_token", &self.next_token)
            .finish()
    }
}

impl ScreenCastBridge {
    /// Open the compositor connection, authenticate, and bind the stream
    /// manager. Every step is bounded; each failure names the step so the
    /// caller can log it and fall back to stills.
    ///
    /// Must be called on the thread that will later issue requests: wayland
    /// queues belong to one thread.
    pub fn connect(socket: &str, token_hex: &str) -> Result<Self, String> {
        let stream = UnixStream::connect(socket)
            .map_err(|error| format!("cannot connect to the compositor at {socket:?}: {error}"))?;
        let connection = Connection::from_socket(stream)
            .map_err(|error| format!("cannot adopt the compositor connection: {error}"))?;
        let (globals, mut queue) = registry_queue_init::<BridgeState>(&connection)
            .map_err(|error| format!("cannot read the compositor registry: {error}"))?;
        let mut state = BridgeState::default();
        let qh = queue.handle();

        let core_version = globals
            .contents()
            .clone_list()
            .iter()
            .find(|global| global.interface == df_core::DfCore::interface().name)
            .map(|global| global.version)
            .ok_or_else(|| "the compositor does not advertise df_core".to_owned())?;
        let manager_global = globals
            .contents()
            .clone_list()
            .iter()
            .find(|global| {
                global.interface == df_toplevel_manager::DfToplevelManager::interface().name
            })
            .map(|global| (global.name, global.version))
            .ok_or_else(|| "the compositor does not advertise df_toplevel_manager".to_owned())?;
        if manager_global.1 < MANAGER_VERSION {
            return Err(format!(
                "df_toplevel_manager is v{}; live screen share needs v{MANAGER_VERSION}",
                manager_global.1
            ));
        }

        let core = globals
            .bind::<df_core::DfCore, _, _>(&qh, 1..=core_version.min(1), ())
            .map_err(|error| format!("cannot bind df_core: {error:?}"))?;
        core.authenticate(LOCKSTEP_VERSION, token_hex.to_owned());
        let deadline = Instant::now() + CREATE_TIMEOUT;
        wait_for(&connection, &mut queue, &mut state, deadline, |state| {
            state.authenticated.is_some()
        })?;
        match state.authenticated.take() {
            Some(Ok(())) => {}
            Some(Err(reason)) => return Err(reason),
            None => return Err("the compositor did not answer the handshake".to_owned()),
        }

        let manager = globals
            .bind::<df_toplevel_manager::DfToplevelManager, _, _>(
                &qh,
                1..=manager_global.1.min(MANAGER_VERSION),
                (),
            )
            .map_err(|error| format!("cannot bind df_toplevel_manager: {error:?}"))?;
        connection
            .flush()
            .map_err(|error| format!("cannot send the manager bind: {error}"))?;
        // A protocol error (an untrusted bind) arrives as a disconnect and is
        // reported by the first create.
        let _ = pump(&connection, &mut queue, &mut state, Instant::now());

        Ok(ScreenCastBridge {
            connection,
            manager,
            queue,
            state,
            owner: std::thread::current().id(),
            next_token: 1,
            last_error: None,
        })
    }

    /// Dispatch and flush without waiting, so an idle bridge never lets its
    /// connection read buffer grow (the registry calls this on its own thread
    /// between requests, and `--check-screencast` proves the link is alive).
    pub fn pump(&mut self) -> Result<(), WatchError> {
        pump(
            &self.connection,
            &mut self.queue,
            &mut self.state,
            Instant::now(),
        )
    }

    /// A wayland event queue belongs to one thread; a request from another is
    /// refused rather than corrupting it.
    fn check_owner_thread(&self) -> Result<(), WatchError> {
        if std::thread::current().id() != self.owner {
            return Err(WatchError(
                "a screen-share request arrived on another thread than the bridge's event queue"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    fn take_next_token(&mut self) -> u32 {
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1).max(1);
        token
    }
}

impl CompositorWatch for ScreenCastBridge {
    fn create(
        &mut self,
        _token: &str,
        source: &StreamSource,
        fps: u32,
    ) -> Result<CreatedNode, WatchError> {
        self.check_owner_thread()?;
        let token = self.take_next_token();
        // A source handle carries its kind (`monitor:DP-1`, `window:7`); the
        // compositor names outputs by the part after the kind prefix, and an
        // empty name is the primary output.
        let output = source
            .id
            .split_once(':')
            .map(|(_, name)| name)
            .unwrap_or(source.id.as_str())
            .to_owned();
        self.manager
            .create_screencast(token, output, 0, 0, fps.clamp(1, 240));
        let deadline = Instant::now() + CREATE_TIMEOUT;
        loop {
            if let Some(reply) = self.state.take_reply(token) {
                self.last_error = None;
                return match reply {
                    Ok(node_id) if node_id != 0 => Ok(CreatedNode {
                        node_id,
                        token: token.to_string(),
                    }),
                    Ok(_) => Err(WatchError(
                        FallbackReason::ProducerUnavailable.as_str().to_owned(),
                    )),
                    Err(reason) if reason.is_empty() => Err(WatchError(
                        FallbackReason::ProducerUnavailable.as_str().to_owned(),
                    )),
                    Err(reason) => Err(WatchError(reason)),
                };
            }
            if Instant::now() >= deadline {
                return Err(WatchError(format!(
                    "{}: the compositor did not answer stream {token}",
                    FallbackReason::ProducerUnavailable.as_str()
                )));
            }
            if let Err(error) = pump(&self.connection, &mut self.queue, &mut self.state, deadline) {
                self.last_error = Some(error.0.clone());
                return Err(error);
            }
        }
    }

    fn health(&mut self) -> Result<(), WatchError> {
        self.check_owner_thread()?;
        if let Some(error) = self.last_error.clone() {
            return Err(WatchError(error));
        }
        // A probe the compositor always answers: a token it does not know is a
        // no-op release (no event, no dispatch trap). Flushing it is the one
        // way to learn now, rather than at the next Start, whether the
        // connection is still alive — a killed compositor can leave its socket
        // path answering connects for a moment.
        let probe = self.take_next_token();
        self.manager.destroy_screencast(probe);
        match self.connection.flush() {
            Ok(()) => Ok(()),
            Err(error) => {
                let error = format!("the compositor connection is gone: {error}");
                self.last_error = Some(error.clone());
                Err(WatchError(error))
            }
        }
    }

    fn destroy(&mut self, token: &str) {
        // A release is best effort by contract: a gone compositor needs no
        // answer, and it releases every stream with this connection anyway.
        if std::thread::current().id() != self.owner {
            return;
        }
        let Ok(token) = token.parse::<u32>() else {
            return;
        };
        self.manager.destroy_screencast(token);
        let _ = self.connection.flush();
        let _ = pump(
            &self.connection,
            &mut self.queue,
            &mut self.state,
            Instant::now(),
        );
    }
}

/// Dispatch pending events, waiting up to `deadline` for the socket when
/// nothing is queued. Dispatch errors and a dead connection become a
/// [`WatchError`].
fn pump(
    connection: &Connection,
    queue: &mut EventQueue<BridgeState>,
    state: &mut BridgeState,
    deadline: Instant,
) -> Result<(), WatchError> {
    if let Err(error) = queue.dispatch_pending(state) {
        return Err(WatchError(format!(
            "the compositor connection failed: {error}"
        )));
    }
    queue
        .flush()
        .map_err(|error| WatchError(format!("flush: {error}")))?;
    let Some(guard) = connection.prepare_read() else {
        // A read is already prepared on another thread; dispatch again and
        // let the caller re-check its condition.
        return Ok(());
    };
    let timeout = deadline.saturating_duration_since(Instant::now());
    let mut poll_fd = libc::pollfd {
        fd: guard.connection_fd().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let millis = timeout.as_millis().min(i32::MAX as u128) as i32;
    let ready = unsafe { libc::poll(&mut poll_fd, 1, millis) };
    if ready > 0 {
        // `read` consumes the guard; dropping an unread one cancels the read,
        // which is exactly what a timeout and a poll error want.
        let _ = guard.read();
    }
    queue
        .dispatch_pending(state)
        .map_err(|error| WatchError(format!("the compositor connection failed: {error}")))?;
    Ok(())
}

/// Dispatch until `done` holds or `deadline` passes.
fn wait_for(
    connection: &Connection,
    queue: &mut EventQueue<BridgeState>,
    state: &mut BridgeState,
    deadline: Instant,
    done: impl Fn(&BridgeState) -> bool,
) -> Result<(), String> {
    while !done(state) {
        if Instant::now() >= deadline {
            return Ok(());
        }
        pump(connection, queue, state, deadline).map_err(|error| error.0)?;
    }
    Ok(())
}

/// Read `DRAGONFRUIT_SCREENCAST_BRIDGE_*` and connect, or explain why the live
/// bridge is not available. The caller keeps the error and names the fallback.
///
/// `runtime_dir` is `XDG_RUNTIME_DIR`; `None` means no session runtime, which
/// is a normal headless state, not a failure of the backend.
pub fn from_environment(
    runtime_dir: Option<&std::path::Path>,
    token_hex: Option<&str>,
    socket_name: Option<&str>,
) -> Result<ScreenCastBridge, String> {
    let Some(token_hex) = token_hex.filter(|token| !token.is_empty()) else {
        return Err(format!(
            "{BRIDGE_TOKEN_ENV} is not set: the portal has no launch token"
        ));
    };
    // A launch token is provisioned as `role:hex` (the shell and desktop
    // entries use that form); a bare value is the token itself.
    let token_hex = token_hex.rsplit(':').next().unwrap_or(token_hex);
    if parse_token(token_hex).is_none() {
        return Err(format!(
            "{BRIDGE_TOKEN_ENV} is not a 32-byte hex token; refusing to present it"
        ));
    }
    let Some(runtime_dir) = runtime_dir else {
        return Err("XDG_RUNTIME_DIR is not set: no compositor socket".to_owned());
    };
    let socket_name = socket_name
        .filter(|name| !name.is_empty())
        .unwrap_or("wayland-0");
    let socket = runtime_dir.join(socket_name);
    ScreenCastBridge::connect(&socket.to_string_lossy(), token_hex)
}

/// The compositor's handshake wants a UTF-8 string; a token is a 32-byte hex
/// value (the form the provisioning writes). Anything else is refused before
/// it reaches the bus or the socket.
fn parse_token(token_hex: &str) -> Option<[u8; 32]> {
    if token_hex.len() != 64 || !token_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut value = [0u8; 32];
    for (index, chunk) in token_hex.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).ok()?;
        value[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_must_be_thirty_two_hex_bytes() {
        assert!(parse_token(&"ab".repeat(32)).is_some());
        assert!(parse_token("ab").is_none());
        assert!(parse_token(&"zz".repeat(32)).is_none());
        assert!(parse_token(&"ab".repeat(31)).is_none());
    }

    #[test]
    fn a_missing_token_socket_or_runtime_is_a_named_absence() {
        let dir = std::path::Path::new("/run/user/1000");
        let error = from_environment(Some(dir), None, None).unwrap_err();
        assert!(error.contains(BRIDGE_TOKEN_ENV), "{error}");
        let error = from_environment(Some(dir), Some("not-hex"), None).unwrap_err();
        assert!(error.contains("hex"), "{error}");
        let error = from_environment(None, Some(&"ab".repeat(32)), None).unwrap_err();
        assert!(error.contains("XDG_RUNTIME_DIR"), "{error}");
        // A well-formed token with no compositor socket is a connect failure,
        // named with the socket it tried, never a silent success. The name is
        // one no session can have, so the probe never reaches a live
        // compositor the test host may be running.
        let absent = "dragonfruit-t335b-absent-probe";
        let error = from_environment(Some(dir), Some(&"ab".repeat(32)), Some(absent)).unwrap_err();
        assert!(error.contains(absent), "{error}");
    }

    #[test]
    fn the_handshake_speaks_the_lockstep_set_and_the_v17_manager() {
        assert_eq!(LOCKSTEP_VERSION, 1);
        assert_eq!(MANAGER_VERSION, 17);
    }
}
