// SPDX-License-Identifier: MIT
//! T-12.3a/T-12.3c acceptance: `ext-session-lock-v1` protocol conformance and
//! locked-input capture / kill-resistance.
//!
//! Spawns the compositor on the headless backend and drives a real
//! `wayland-client` through a full lock:
//!
//! * the manager global is advertised and bindable by the trusted shell,
//! * `lock` is confirmed with the `locked` event (fail-secure),
//! * every advertised output gets a lock surface, and the compositor
//!   configures it to exactly that output's size (the "UI covers every
//!   output" contract),
//! * the client can ack the configure, attach a full-output buffer, commit it,
//!   and later `unlock_and_destroy` without a protocol error.
//!
//! T-12.3c adds:
//!
//! * **input capture.** With the lock up, the seat keyboard is focused on the
//!   lock surface and synthetic input is delivered there — never to a normal
//!   client. Before the lock surface exists, input is dropped.
//! * **kill-resistance.** Disconnecting the lock client (a crash) leaves the
//!   compositor fail-secure (`locked=1`) with zero live lock surfaces; because
//!   the shell is the lock UI, the compositor then ends the session cleanly
//!   (ADR 0312) instead of leaving a frozen lock.
//!
//! The lock manager global is gated on the T-07 launch-token handshake, so the
//! client authenticates as the trusted shell before it locks.

use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_registry, wl_seat, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1 as lock_mgr, ext_session_lock_surface_v1 as lock_surface,
    ext_session_lock_v1 as lock,
};
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use wayland_protocols::xdg::shell::client::{
    xdg_popup, xdg_positioner, xdg_surface, xdg_toplevel, xdg_wm_base,
};

mod common;

/// Generated client bindings for the private `df_core` handshake.
mod core_client {
    #![allow(unused_imports, clippy::single_component_path_imports)]
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-core.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("../protocols/dragonfruit-core.xml");
}

use core_client::df_core;

/// The headless backend's default mode (`backend::HEADLESS_MODE_SIZE`).
const OUTPUT_W: u32 = 1280;
const OUTPUT_H: u32 = 720;

/// The normal client's toplevel and popup sizes (smaller than the output).
const APP_W: i32 = 360;
const APP_H: i32 = 260;
const POPUP_W: i32 = 120;
const POPUP_H: i32 = 80;

/// One connected lock client (the trusted shell stand-in).
#[derive(Default)]
struct LockTest {
    registry: Option<wl_registry::WlRegistry>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    seat: Option<wl_seat::WlSeat>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    outputs: Vec<wl_output::WlOutput>,
    lock_manager: Option<lock_mgr::ExtSessionLockManagerV1>,
    core_global: Option<(u32, u32)>,
    authenticated: bool,
    locked: bool,
    finished: bool,
    /// `(serial, width, height)` for every lock-surface configure.
    configures: Vec<(u32, u32, u32)>,
    /// Keyboard enter surfaces (protocol ids) and keycodes delivered here.
    keyboard_enters: Vec<u32>,
    keys: Vec<u32>,
    /// Pointer enter surfaces and button codes delivered to the lock UI.
    pointer_enters: Vec<u32>,
    pointer_buttons: Vec<u32>,
    /// The lock surface's backing `wl_surface` protocol id.
    lock_surface_id: u32,
}

impl Dispatch<wl_registry::WlRegistry, ()> for LockTest {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "wl_seat" => {
                    state.seat = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "wl_output" => {
                    state
                        .outputs
                        .push(registry.bind(name, version.min(4), qh, ()));
                }
                "df_core" => {
                    state.core_global = Some((name, version));
                }
                "ext_session_lock_manager_v1" => {
                    state.lock_manager = Some(registry.bind(name, version.min(1), qh, ()));
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(LockTest: ignore wl_compositor::WlCompositor);
delegate_noop!(LockTest: ignore wl_shm::WlShm);
delegate_noop!(LockTest: ignore wl_shm_pool::WlShmPool);
delegate_noop!(LockTest: ignore wl_buffer::WlBuffer);
delegate_noop!(LockTest: ignore wl_surface::WlSurface);
delegate_noop!(LockTest: ignore wl_output::WlOutput);
delegate_noop!(LockTest: ignore lock_mgr::ExtSessionLockManagerV1);

impl Dispatch<df_core::DfCore, ()> for LockTest {
    fn event(
        state: &mut Self,
        _: &df_core::DfCore,
        event: df_core::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let df_core::Event::Authenticated { .. } = event {
            state.authenticated = true;
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for LockTest {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let Ok(caps) = capabilities.into_result() else {
                return;
            };
            if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            }
            if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for LockTest {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { surface, .. } => {
                state.keyboard_enters.push(surface.id().protocol_id());
            }
            wl_keyboard::Event::Key { key, .. } => state.keys.push(key),
            _ => {}
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for LockTest {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface, .. } => {
                state.pointer_enters.push(surface.id().protocol_id());
            }
            wl_pointer::Event::Button { button, .. } => state.pointer_buttons.push(button),
            _ => {}
        }
    }
}

impl Dispatch<lock::ExtSessionLockV1, ()> for LockTest {
    fn event(
        state: &mut Self,
        _: &lock::ExtSessionLockV1,
        event: lock::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            lock::Event::Locked => state.locked = true,
            lock::Event::Finished => state.finished = true,
            _ => {}
        }
    }
}

impl Dispatch<lock_surface::ExtSessionLockSurfaceV1, ()> for LockTest {
    fn event(
        state: &mut Self,
        surface: &lock_surface::ExtSessionLockSurfaceV1,
        event: lock_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let lock_surface::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            // The compositor expects the ack before the buffer commit.
            surface.ack_configure(serial);
            state.configures.push((serial, width, height));
        }
    }
}

/// A normal (untrusted) client with a toplevel and an optional popup.
///
/// T305 needs a second client so the test can prove that the lock excludes a
/// popup's keyboard grab and a held pointer drag while the trusted lock client
/// still receives input.
#[derive(Default)]
struct AppClient {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    seat: Option<wl_seat::WlSeat>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    outputs: usize,
    toplevel_surface: Option<wl_surface::WlSurface>,
    popup_surface: Option<wl_surface::WlSurface>,
    keyboard_enters: Vec<u32>,
    keyboard_leaves: Vec<u32>,
    keys: Vec<u32>,
    pointer_enters: Vec<u32>,
    pointer_leaves: usize,
    pointer_motions: usize,
    pointer_buttons: Vec<u32>,
    popup_configure: Option<(i32, i32)>,
    popup_done: bool,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppClient {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "wl_seat" => {
                    state.seat = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "xdg_wm_base" => {
                    state.wm_base = Some(registry.bind(name, version.min(2), qh, ()));
                }
                "zxdg_decoration_manager_v1" => {
                    state.decoration_manager = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "wl_output" => {
                    let _: wl_output::WlOutput = registry.bind(name, version.min(4), qh, ());
                    state.outputs += 1;
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(AppClient: ignore wl_compositor::WlCompositor);
delegate_noop!(AppClient: ignore wl_shm::WlShm);
delegate_noop!(AppClient: ignore wl_shm_pool::WlShmPool);
delegate_noop!(AppClient: ignore wl_buffer::WlBuffer);
delegate_noop!(AppClient: ignore wl_surface::WlSurface);
delegate_noop!(AppClient: ignore wl_output::WlOutput);
delegate_noop!(AppClient: ignore xdg_toplevel::XdgToplevel);
delegate_noop!(AppClient: ignore xdg_positioner::XdgPositioner);
delegate_noop!(AppClient: ignore zxdg_decoration_manager_v1::ZxdgDecorationManagerV1);
delegate_noop!(AppClient: ignore zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1);

impl Dispatch<wl_seat::WlSeat, ()> for AppClient {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let Ok(caps) = capabilities.into_result() else {
                return;
            };
            if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            }
            if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppClient {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { surface, .. } => {
                state.keyboard_enters.push(surface.id().protocol_id());
            }
            wl_keyboard::Event::Leave { surface, .. } => {
                state.keyboard_leaves.push(surface.id().protocol_id());
            }
            wl_keyboard::Event::Key { key, .. } => state.keys.push(key),
            _ => {}
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for AppClient {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface, .. } => {
                state.pointer_enters.push(surface.id().protocol_id());
            }
            wl_pointer::Event::Leave { .. } => state.pointer_leaves += 1,
            wl_pointer::Event::Motion { .. } => state.pointer_motions += 1,
            wl_pointer::Event::Button { button, .. } => state.pointer_buttons.push(button),
            _ => {}
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for AppClient {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for AppClient {
    fn event(
        _: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
        }
    }
}

impl Dispatch<xdg_popup::XdgPopup, ()> for AppClient {
    fn event(
        state: &mut Self,
        _: &xdg_popup::XdgPopup,
        event: xdg_popup::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_popup::Event::Configure { width, height, .. } => {
                state.popup_configure = Some((width, height));
            }
            xdg_popup::Event::PopupDone => state.popup_done = true,
            _ => {}
        }
    }
}

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    token_path: PathBuf,
    log_path: PathBuf,
    synthetic_path: Option<PathBuf>,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.token_path);
        let _ = std::fs::remove_file(&self.log_path);
        common::cleanup_compositor_artifacts(&self.socket_path);
        if let Some(path) = &self.synthetic_path {
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_file(path.with_extension("reply"));
        }
    }
}

impl CompositorProcess {
    fn start(socket_name: &str, tokens: &[String]) -> Self {
        Self::start_with_synthetic(socket_name, tokens, None)
    }

    fn start_with_synthetic(
        socket_name: &str,
        tokens: &[String],
        synthetic: Option<&Path>,
    ) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let token_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.launch-token"));
        // Capture the compositor's own log so a test can assert the one-line
        // session-ending reason after the process has exited. Per-frame
        // output would be a bug; this file only holds lifecycle lines.
        let log_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.log"));
        let log = std::fs::File::create(&log_path).expect("failed to create compositor log");
        let log_err = log.try_clone().expect("failed to clone compositor log");

        let mut command = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"));
        command
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_LAUNCH_TOKENS", tokens.join(","));
        if let Some(path) = synthetic {
            command.env("DRAGONFRUIT_SYNTHETIC_INPUT", path);
        }
        let mut child = command
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err))
            .spawn()
            .expect("failed to start compositor");

        let deadline = Instant::now() + Duration::from_secs(10);
        while !socket_path.exists() || !token_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before ready: {status}");
            }
            assert!(Instant::now() < deadline, "compositor never became ready");
            std::thread::sleep(Duration::from_millis(10));
        }
        if let Some(path) = synthetic {
            while !path.exists() {
                assert!(Instant::now() < deadline, "synthetic socket never appeared");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        Self {
            child,
            socket_path,
            token_path,
            log_path,
            synthetic_path: synthetic.map(Path::to_path_buf),
        }
    }

    fn read_token(&self) -> String {
        std::fs::read_to_string(&self.token_path)
            .expect("token file readable")
            .trim()
            .to_string()
    }

    /// Wait for the compositor to exit on its own and return its status.
    #[track_caller]
    fn wait_for_exit(&mut self, timeout: Duration) -> std::process::ExitStatus {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return status,
                Ok(None) => {
                    assert!(
                        Instant::now() < deadline,
                        "compositor did not exit within {timeout:?}"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("wait failed: {err}"),
            }
        }
    }

    fn read_log(&self) -> String {
        std::fs::read_to_string(&self.log_path).unwrap_or_default()
    }
}

/// Client end of the compositor's synthetic-input harness
/// (`DRAGONFRUIT_SYNTHETIC_INPUT`).
struct SyntheticInput {
    socket: std::os::unix::net::UnixDatagram,
    path: PathBuf,
    reply_path: PathBuf,
}

impl SyntheticInput {
    fn connect(path: &Path) -> Self {
        let reply_path = path.with_extension("reply");
        let _ = std::fs::remove_file(&reply_path);
        let socket = std::os::unix::net::UnixDatagram::bind(&reply_path)
            .expect("failed to bind synthetic reply socket");
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set reply timeout");
        Self {
            socket,
            path: path.to_path_buf(),
            reply_path,
        }
    }

    #[track_caller]
    fn send(&self, command: &str) {
        self.socket
            .send_to(command.as_bytes(), &self.path)
            .unwrap_or_else(|err| panic!("failed to send synthetic {command:?}: {err}"));
    }

    /// Send a query and read the compositor's datagram reply.
    #[track_caller]
    fn query(&self, command: &str) -> String {
        self.send(command);
        let mut buf = [0u8; 16 * 1024];
        let len = self
            .socket
            .recv(&mut buf)
            .unwrap_or_else(|err| panic!("no reply to {command:?}: {err}"));
        String::from_utf8_lossy(&buf[..len]).into_owned()
    }
}

impl Drop for SyntheticInput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.reply_path);
    }
}

fn wait_for<T>(
    queue: &mut EventQueue<T>,
    state: &mut T,
    timeout: Duration,
    what: &str,
    predicate: impl Fn(&T) -> bool,
) {
    let deadline = Instant::now() + timeout;
    while !predicate(state) {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        // A roundtrip always returns (the compositor answers `wl_display.sync`)
        // and flushes every event queued before it, so a regression that sends
        // no event fails at the deadline instead of blocking forever.
        queue
            .roundtrip(state)
            .expect("dispatch failed while waiting");
    }
}

fn connect(socket_path: &Path) -> (Connection, EventQueue<LockTest>, LockTest) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let stream = loop {
        match UnixStream::connect(socket_path) {
            Ok(stream) => break stream,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                ) =>
            {
                assert!(Instant::now() < deadline, "connect failed: {err}");
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("connect failed: {err}"),
        }
    };
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let registry = conn.display().get_registry(&qh, ());
    let mut state = LockTest {
        registry: Some(registry),
        ..Default::default()
    };
    queue.roundtrip(&mut state).expect("roundtrip failed");
    (conn, queue, state)
}

/// Connect one normal client (a toplevel/popup owner, not the trusted shell).
fn connect_app(socket_path: &Path) -> (Connection, EventQueue<AppClient>, AppClient) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let stream = loop {
        match UnixStream::connect(socket_path) {
            Ok(stream) => break stream,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                ) =>
            {
                assert!(Instant::now() < deadline, "connect failed: {err}");
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("connect failed: {err}"),
        }
    };
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = AppClient::default();
    queue.roundtrip(&mut state).expect("roundtrip failed");
    (conn, queue, state)
}

/// Complete the `df_core` handshake so this client can own the session lock.
fn authenticate(state: &mut LockTest, queue: &mut EventQueue<LockTest>, token: String) {
    let (name, version) = state.core_global.expect("df_core advertised");
    let qh = queue.handle();
    let core = state
        .registry
        .clone()
        .expect("registry bound")
        .bind::<df_core::DfCore, _, _>(name, version, &qh, ());
    core.authenticate(1, token);
    wait_for(queue, state, Duration::from_secs(5), "authenticated", |s| {
        s.authenticated
    });
}

fn shm_buffer(
    state: &LockTest,
    qh: &QueueHandle<LockTest>,
    width: i32,
    height: i32,
) -> (wl_buffer::WlBuffer, std::fs::File) {
    let shm = state.shm.clone().expect("wl_shm bound");
    let stride = width * 4;
    let size = (stride * height) as usize;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SHM_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SHM_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-lock-conformance-{}-{seq}.shm",
        std::process::id()
    ));
    let file = std::fs::File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("failed to create shm backing file");
    file.set_len(size as u64).expect("failed to size shm file");
    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(0, width, height, stride, wl_shm::Format::Argb8888, qh, ());
    pool.destroy();
    (buffer, file)
}

fn app_shm_buffer(
    state: &AppClient,
    qh: &QueueHandle<AppClient>,
    width: i32,
    height: i32,
) -> (wl_buffer::WlBuffer, std::fs::File) {
    let shm = state.shm.clone().expect("wl_shm bound");
    let stride = width * 4;
    let size = (stride * height) as usize;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SHM_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SHM_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-lock-app-{}-{seq}.shm",
        std::process::id()
    ));
    let file = std::fs::File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("failed to create shm backing file");
    file.set_len(size as u64).expect("failed to size shm file");
    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(0, width, height, stride, wl_shm::Format::Argb8888, qh, ());
    pool.destroy();
    (buffer, file)
}

/// Map a decorated toplevel and return the objects a popup needs.
fn map_app_toplevel(
    client: &mut AppClient,
    queue: &mut EventQueue<AppClient>,
) -> (
    wl_surface::WlSurface,
    xdg_surface::XdgSurface,
    xdg_toplevel::XdgToplevel,
    std::fs::File,
) {
    let qh = queue.handle();
    let compositor = client.compositor.clone().expect("wl_compositor bound");
    let wm_base = client.wm_base.clone().expect("xdg_wm_base bound");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Lock grab test".into());
    // The compositor draws the titlebar only for a server-decorated toplevel;
    // the drag case presses that titlebar.
    if let Some(manager) = client.decoration_manager.clone() {
        let decoration = manager.get_toplevel_decoration(&toplevel, &qh, ());
        decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide);
    }
    surface.commit();
    queue.roundtrip(client).expect("initial configure");

    let (buffer, file) = app_shm_buffer(client, &qh, APP_W, APP_H);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, APP_W, APP_H);
    surface.commit();
    queue.roundtrip(client).expect("map commit");

    client.toplevel_surface = Some(surface.clone());
    (surface, xdg_surface, toplevel, file)
}

/// Map a grabbing `xdg_popup` rooted at `parent`.
fn map_app_popup(
    client: &mut AppClient,
    queue: &mut EventQueue<AppClient>,
    parent: &xdg_surface::XdgSurface,
) -> (wl_surface::WlSurface, xdg_popup::XdgPopup, std::fs::File) {
    let qh = queue.handle();
    let compositor = client.compositor.clone().expect("wl_compositor bound");
    let wm_base = client.wm_base.clone().expect("xdg_wm_base bound");
    let seat = client.seat.clone().expect("wl_seat bound");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let positioner = wm_base.create_positioner(&qh, ());
    positioner.set_size(POPUP_W, POPUP_H);
    positioner.set_anchor_rect(0, 0, 1, 1);
    positioner.set_anchor(xdg_positioner::Anchor::BottomRight);
    positioner.set_gravity(xdg_positioner::Gravity::BottomRight);
    let popup = xdg_surface.get_popup(Some(parent), &positioner, &qh, ());
    // The grab must be requested before the popup is mapped (`committed`).
    popup.grab(&seat, 1);
    positioner.destroy();
    surface.commit();
    queue.roundtrip(client).expect("popup configure");

    let (buffer, file) = app_shm_buffer(client, &qh, POPUP_W, POPUP_H);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, POPUP_W, POPUP_H);
    surface.commit();
    queue.roundtrip(client).expect("popup map commit");

    client.popup_surface = Some(surface.clone());
    (surface, popup, file)
}

/// One parsed `query decorations` line (the fields T305 needs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Decoration {
    titlebar: (i32, i32, i32, i32),
    content: (i32, i32, i32, i32),
}

fn first_decoration(report: &str) -> Option<Decoration> {
    report.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        if parts.next()? != "decoration" {
            return None;
        }
        let n =
            |parts: &mut std::str::SplitWhitespace| -> Option<i32> { parts.next()?.parse().ok() };
        let _window: u64 = parts.next()?.parse().ok()?;
        let _server_side = parts.next()?;
        let titlebar = (
            n(&mut parts)?,
            n(&mut parts)?,
            n(&mut parts)?,
            n(&mut parts)?,
        );
        let content = (
            n(&mut parts)?,
            n(&mut parts)?,
            n(&mut parts)?,
            n(&mut parts)?,
        );
        Some(Decoration { titlebar, content })
    })
}

/// Read the first window's decoration report, waiting for the map to settle.
#[track_caller]
fn wait_for_decoration(input: &SyntheticInput) -> Decoration {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let report = input.query("query decorations");
        if let Some(decoration) = first_decoration(&report) {
            return decoration;
        }
        assert!(Instant::now() < deadline, "window never appeared: {report}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Move the synthetic pointer to a global logical point.
fn motion_abs(input: &SyntheticInput, x: i32, y: i32) {
    let nx = f64::from(x) / f64::from(OUTPUT_W);
    let ny = f64::from(y) / f64::from(OUTPUT_H);
    input.send(&format!("motion-abs {nx} {ny}"));
}

/// Start a titlebar move grab and prove it is live by moving the window.
#[track_caller]
fn start_titlebar_drag(input: &SyntheticInput) -> (i32, i32) {
    let decoration = wait_for_decoration(input);
    let (tx, ty, tw, th) = decoration.titlebar;
    assert!(tw > 0 && th > 0, "expected an SSD titlebar: {decoration:?}");
    motion_abs(input, tx + tw / 2, ty + th / 2);
    input.send("button 272 down");

    let before = decoration.content;
    input.send("motion 40 25");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let now = wait_for_decoration(input).content;
        if now != before {
            return (now.0, now.1);
        }
        assert!(
            Instant::now() < deadline,
            "titlebar drag never moved the window"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn lock_covers_every_output_and_confirms() {
    let socket_name = format!("dragonfruit-test-lock-{}", std::process::id());
    let token = "11".repeat(32);
    let proc = CompositorProcess::start(&socket_name, std::slice::from_ref(&token));

    let (_conn, mut queue, mut state) = connect(&proc.socket_path);
    authenticate(&mut state, &mut queue, proc.read_token());

    // Advertise + bind the globals.
    let compositor = state.compositor.clone().expect("wl_compositor bound");
    let manager = state
        .lock_manager
        .clone()
        .expect("ext_session_lock_manager_v1 advertised");
    assert_eq!(state.outputs.len(), 1, "one headless output");

    // Lock the session and create a lock surface covering the output.
    let lock = manager.lock(&queue.handle(), ());
    let surface = compositor.create_surface(&queue.handle(), ());
    let lock_surface = lock.get_lock_surface(&surface, &state.outputs[0], &queue.handle(), ());

    // The compositor confirms the lock (fail-secure) even before any lock
    // surface has been committed.
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "locked",
        |s| s.locked,
    );
    assert!(!state.finished, "lock was not confirmed");

    // Every output is configured to exactly its own size: the UI covers the
    // whole screen.
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "lock-surface configure",
        |s| !s.configures.is_empty(),
    );
    let (_, width, height) = state.configures[0];
    assert_eq!(
        (width, height),
        (OUTPUT_W, OUTPUT_H),
        "lock surface must cover the full output"
    );
    assert_eq!(
        state.configures.len(),
        state.outputs.len(),
        "one configure per output"
    );

    // Ack + full-output buffer + commit: the surface is mapped and locked.
    let (buffer, _file) = shm_buffer(&state, &queue.handle(), width as i32, height as i32);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, width as i32, height as i32);
    surface.commit();
    queue.roundtrip(&mut state).expect("roundtrip failed");

    // Unlock cleanly; the compositor clears the lock and the client object is
    // destroyed, so no later event should arrive.
    lock.unlock_and_destroy();
    lock_surface.destroy();
    queue.roundtrip(&mut state).expect("roundtrip failed");
    assert!(
        !state.finished,
        "unlock produced an unexpected finished event"
    );

    // The compositor is still healthy after a full lock/unlock cycle: a
    // second lock can be requested and confirmed.
    let second = manager.lock(&queue.handle(), ());
    let second_surface = compositor.create_surface(&queue.handle(), ());
    let _second_lock_surface =
        second.get_lock_surface(&second_surface, &state.outputs[0], &queue.handle(), ());
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "second configure",
        |s| s.configures.len() >= 2,
    );
    second.unlock_and_destroy();
}

#[test]
fn repeated_lock_cycles_never_leak_lock_state() {
    let socket_name = format!("dragonfruit-test-lock-cycles-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let _ = std::fs::remove_file(&synthetic_path);
    let token = "cd".repeat(32);
    let proc = CompositorProcess::start_with_synthetic(
        &socket_name,
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);

    let (_conn, mut queue, mut state) = connect(&proc.socket_path);
    authenticate(&mut state, &mut queue, proc.read_token());

    let compositor = state.compositor.clone().expect("wl_compositor bound");
    let manager = state
        .lock_manager
        .clone()
        .expect("ext_session_lock_manager_v1 advertised");
    assert_eq!(state.outputs.len(), 1, "one headless output");

    // Repeated lock/unlock loops must not accumulate a lock or a lock
    // surface: every cycle confirms (`locked=1 surfaces=1`), covers the
    // output, and returns to (`locked=0 surfaces=0`). A leak would show as a
    // growing `surfaces` count or a session that never unlocks.
    const CYCLES: usize = 25;
    for cycle in 0..CYCLES {
        let lock = manager.lock(&queue.handle(), ());
        let surface = compositor.create_surface(&queue.handle(), ());
        let lock_surface = lock.get_lock_surface(&surface, &state.outputs[0], &queue.handle(), ());

        wait_for(
            &mut queue,
            &mut state,
            Duration::from_secs(5),
            "locked",
            |s| s.locked,
        );
        let want = cycle + 1;
        wait_for(
            &mut queue,
            &mut state,
            Duration::from_secs(5),
            "lock-surface configure",
            |s| s.configures.len() >= want,
        );
        let (_, width, height) = state.configures[cycle];
        assert_eq!(
            (width, height),
            (OUTPUT_W, OUTPUT_H),
            "cycle {cycle}: lock surface must cover the full output"
        );

        let reported = input.query("query lock");
        assert!(
            reported.contains("lock locked=1 surfaces=1"),
            "cycle {cycle}: lock did not cover the output: {reported}"
        );

        let (buffer, _file) = shm_buffer(&state, &queue.handle(), width as i32, height as i32);
        surface.attach(Some(&buffer), 0, 0);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();
        queue.roundtrip(&mut state).expect("roundtrip failed");

        lock.unlock_and_destroy();
        lock_surface.destroy();
        queue.roundtrip(&mut state).expect("roundtrip failed");
        assert!(!state.finished, "cycle {cycle}: unlock produced finished");

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let report = input.query("query lock");
            if report.contains("lock locked=0 surfaces=0") {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "cycle {cycle}: lock state leaked after unlock: {report}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    // The compositor is still healthy after every cycle: one more lock
    // confirms and covers the output.
    let last = manager.lock(&queue.handle(), ());
    let last_surface = compositor.create_surface(&queue.handle(), ());
    let _last_lock_surface =
        last.get_lock_surface(&last_surface, &state.outputs[0], &queue.handle(), ());
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "final configure",
        |s| s.configures.len() > CYCLES,
    );
    assert!(
        input
            .query("query lock")
            .contains("lock locked=1 surfaces=1"),
        "the compositor could not lock after {CYCLES} cycles"
    );
    last.unlock_and_destroy();
}

#[test]
fn untrusted_client_cannot_lock() {
    let socket_name = format!("dragonfruit-test-lock-untrusted-{}", std::process::id());
    // One shell token is provisioned, but this client never redeems it.
    let token = "22".repeat(32);
    let proc = CompositorProcess::start(&socket_name, std::slice::from_ref(&token));

    let (_conn, mut queue, mut state) = connect(&proc.socket_path);
    let manager = state
        .lock_manager
        .clone()
        .expect("ext_session_lock_manager_v1 advertised");
    let _lock = manager.lock(&queue.handle(), ());
    // The compositor drops the confirmation, so the client gets `finished`
    // and the session never locks. The global is still healthy.
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "finished (refusal)",
        |s| s.finished,
    );
    assert!(!state.locked, "an untrusted client locked the session");
}

#[test]
fn locked_input_targets_the_lock_ui_and_its_death_ends_the_session() {
    let socket_name = format!("dragonfruit-test-lock-input-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let _ = std::fs::remove_file(&synthetic_path);
    let token = "ab".repeat(32);
    let mut proc = CompositorProcess::start_with_synthetic(
        &socket_name,
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let token = proc.read_token();

    // The lock client is the trusted shell. `dup` lets the test hard-close the
    // connection later, simulating a crash.
    let stream = UnixStream::connect(&proc.socket_path).expect("failed to connect to compositor");
    let raw_fd = stream.as_raw_fd();
    let kill_fd = unsafe { libc::dup(raw_fd) };
    assert!(kill_fd >= 0, "dup failed");
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let registry = conn.display().get_registry(&qh, ());
    let mut state = LockTest {
        registry: Some(registry),
        ..Default::default()
    };
    queue.roundtrip(&mut state).expect("roundtrip failed");
    let (core_name, core_version) = state.core_global.expect("df_core advertised");
    let core = state
        .registry
        .clone()
        .unwrap()
        .bind::<df_core::DfCore, _, _>(core_name, core_version, &qh, ());
    core.authenticate(1, token);
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "authenticated",
        |s| s.authenticated,
    );

    // Wait for the seat to hand us a keyboard and a pointer.
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "seat keyboard",
        |s| s.keyboard.is_some(),
    );
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "seat pointer",
        |s| s.pointer.is_some(),
    );

    // Lock and map the lock surface.
    let compositor = state.compositor.clone().expect("wl_compositor bound");
    let manager = state
        .lock_manager
        .clone()
        .expect("ext_session_lock_manager_v1 advertised");
    let lock = manager.lock(&qh, ());
    let surface = compositor.create_surface(&qh, ());
    let _lock_surface = lock.get_lock_surface(&surface, &state.outputs[0], &qh, ());
    state.lock_surface_id = surface.id().protocol_id();
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "locked",
        |s| s.locked,
    );
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "configure",
        |s| !s.configures.is_empty(),
    );
    let (_, width, height) = state.configures[0];
    let (buffer, _file) = shm_buffer(&state, &qh, width as i32, height as i32);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, width as i32, height as i32);
    surface.commit();

    // The lock surface is the keyboard target: the compositor focuses it when
    // it is created.
    let lock_id = state.lock_surface_id;
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "keyboard enter on the lock surface",
        |s| s.keyboard_enters.contains(&lock_id),
    );

    // The lock is up and the lock surface owns the keyboard.
    let before = input.query("query lock");
    assert!(
        before.contains("lock locked=1 surfaces=1 lock-focus=1"),
        "unexpected lock report before input: {before}"
    );

    // Synthetic input is captured by the lock UI: keys and pointer events land
    // on the lock surface, and no other client receives them.
    input.send("key 30 down");
    input.send("key 30 up");
    input.send("motion-abs 0.5 0.5");
    input.send("button 272 down");
    input.send("button 272 up");
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "key delivered to the lock surface",
        |s| s.keys.contains(&30),
    );
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "pointer enter on the lock surface",
        |s| s.pointer_enters.contains(&lock_id),
    );
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "button delivered to the lock surface",
        |s| s.pointer_buttons.contains(&272),
    );

    let during = input.query("query lock");
    assert!(
        during.contains("lock locked=1 surfaces=1 lock-focus=1"),
        "unexpected lock report after input: {during}"
    );

    // Kill the lock UI: hard-close the client socket. The compositor stays
    // fail-secure (locked=1) but loses its only input target; because the
    // shell is the lock UI, the safe outcome is to end the session rather than
    // leave a frozen lock (ADR 0312). The one-line reason is logged at exit
    // with the lock state, so the conformance suite asserts `locked=1` after
    // the lock client disappears even though no further query can be served.
    unsafe {
        libc::shutdown(kill_fd, libc::SHUT_RDWR);
        libc::close(kill_fd);
    }
    let status = proc.wait_for_exit(Duration::from_secs(10));
    assert!(
        status.success(),
        "compositor must exit cleanly after lock-client loss: {status}"
    );
    let log = proc.read_log();
    assert!(
        log.contains("lock locked=1"),
        "the session did not stay fail-secure while ending: {log}"
    );
    assert!(
        log.contains("trusted shell disconnected while locked"),
        "the exit reason was not logged: {log}"
    );
}

#[test]
fn lost_lock_client_ends_the_session_and_a_fresh_start_can_lock_again() {
    let socket_name = format!("dragonfruit-test-lock-loss-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let _ = std::fs::remove_file(&synthetic_path);
    let token = "ac".repeat(32);
    let mut proc = CompositorProcess::start_with_synthetic(
        &socket_name,
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);

    // Keep a duplicate of the socket fd so the test can hard-close the lock
    // client exactly like a crashed shell.
    let stream = UnixStream::connect(&proc.socket_path).expect("failed to connect to compositor");
    let kill_fd = unsafe { libc::dup(stream.as_raw_fd()) };
    assert!(kill_fd >= 0, "dup failed");
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let registry = conn.display().get_registry(&qh, ());
    let mut state = LockTest {
        registry: Some(registry),
        ..Default::default()
    };
    queue.roundtrip(&mut state).expect("roundtrip failed");
    authenticate(&mut state, &mut queue, proc.read_token());
    let locked = lock_and_map(&mut state, &mut queue);
    let before = input.query("query lock");
    assert!(
        before.contains("lock locked=1 surfaces=1"),
        "the lock was not established before the client died: {before}"
    );

    unsafe {
        libc::shutdown(kill_fd, libc::SHUT_RDWR);
        libc::close(kill_fd);
    }
    let status = proc.wait_for_exit(Duration::from_secs(10));
    assert!(
        status.success(),
        "the compositor exits cleanly on lock-client loss: {status}"
    );
    let log = proc.read_log();
    assert!(
        log.contains("lock locked=1"),
        "the session did not stay fail-secure while ending: {log}"
    );

    // The dead client and the old process are gone; a fresh start reuses the
    // same socket name (proving the teardown removed the hand-off files) and
    // can lock again.
    drop(locked);
    drop(conn);
    drop(input);
    drop(proc);

    let fresh_token = "ad".repeat(32);
    let fresh = CompositorProcess::start(&socket_name, std::slice::from_ref(&fresh_token));
    let (_fresh_conn, mut fresh_queue, mut fresh_state) = connect(&fresh.socket_path);
    authenticate(&mut fresh_state, &mut fresh_queue, fresh.read_token());
    let _fresh_locked = lock_and_map(&mut fresh_state, &mut fresh_queue);
    assert!(
        fresh_state.locked,
        "a fresh compositor could not lock again after lock-client loss"
    );
    assert!(
        !fresh_state.configures.is_empty(),
        "the fresh lock did not configure a surface"
    );
}

/// A mapped lock UI: the protocol objects must outlive the assertions.
struct LockedUi {
    _lock: lock::ExtSessionLockV1,
    _surface: wl_surface::WlSurface,
    _lock_surface: lock_surface::ExtSessionLockSurfaceV1,
    _buffer: wl_buffer::WlBuffer,
    _file: std::fs::File,
}

/// Request the lock as the trusted shell and map a full-output lock surface.
fn lock_and_map(lock_state: &mut LockTest, lock_queue: &mut EventQueue<LockTest>) -> LockedUi {
    wait_for(
        lock_queue,
        lock_state,
        Duration::from_secs(5),
        "seat keyboard",
        |s| s.keyboard.is_some(),
    );
    wait_for(
        lock_queue,
        lock_state,
        Duration::from_secs(5),
        "seat pointer",
        |s| s.pointer.is_some(),
    );
    let compositor = lock_state.compositor.clone().expect("wl_compositor bound");
    let manager = lock_state
        .lock_manager
        .clone()
        .expect("ext_session_lock_manager_v1 advertised");
    let lock = manager.lock(&lock_queue.handle(), ());
    let surface = compositor.create_surface(&lock_queue.handle(), ());
    let lock_surface =
        lock.get_lock_surface(&surface, &lock_state.outputs[0], &lock_queue.handle(), ());
    lock_state.lock_surface_id = surface.id().protocol_id();
    wait_for(
        lock_queue,
        lock_state,
        Duration::from_secs(5),
        "locked",
        |s| s.locked,
    );
    wait_for(
        lock_queue,
        lock_state,
        Duration::from_secs(5),
        "lock-surface configure",
        |s| !s.configures.is_empty(),
    );
    let (_, width, height) = lock_state.configures[0];
    let (buffer, file) = shm_buffer(
        lock_state,
        &lock_queue.handle(),
        width as i32,
        height as i32,
    );
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, width as i32, height as i32);
    surface.commit();
    let lock_id = lock_state.lock_surface_id;
    wait_for(
        lock_queue,
        lock_state,
        Duration::from_secs(5),
        "keyboard enter on the lock surface",
        |s| s.keyboard_enters.contains(&lock_id),
    );
    LockedUi {
        _lock: lock,
        _surface: surface,
        _lock_surface: lock_surface,
        _buffer: buffer,
        _file: file,
    }
}

#[test]
fn locked_popup_grab_never_receives_lock_screen_keys() {
    let socket_name = format!("dragonfruit-test-lock-popup-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let _ = std::fs::remove_file(&synthetic_path);
    let token = "5a".repeat(32);
    let proc = CompositorProcess::start_with_synthetic(
        &socket_name,
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);

    // The trusted shell owns the lock; a second client owns the popup.
    let (_lock_conn, mut lock_queue, mut lock_state) = connect(&proc.socket_path);
    authenticate(&mut lock_state, &mut lock_queue, proc.read_token());
    let (_app_conn, mut app_queue, mut app) = connect_app(&proc.socket_path);
    wait_for(
        &mut app_queue,
        &mut app,
        Duration::from_secs(5),
        "app seat keyboard",
        |s| s.keyboard.is_some(),
    );

    let (toplevel_surface, parent_xdg, _toplevel, _toplevel_file) =
        map_app_toplevel(&mut app, &mut app_queue);
    let toplevel_id = toplevel_surface.id().protocol_id();

    // Activate the window so its popup is the active window's popup. A popup
    // grab belongs to a window that was clicked, so this matches the real
    // path (and makes `release_client_grabs` find it).
    let decoration = wait_for_decoration(&input);
    let (cx, cy, cw, ch) = decoration.content;
    motion_abs(&input, cx + cw / 2, cy + ch / 2);
    input.send("button 272 down");
    input.send("button 272 up");
    wait_for(
        &mut app_queue,
        &mut app,
        Duration::from_secs(5),
        "app toplevel keyboard focus",
        |s| s.keyboard_enters.contains(&toplevel_id),
    );

    // A grabbing popup: while unlocked its keyboard grab wins, which proves
    // the defect the lock path must clear.
    let (_popup_surface, _popup, _popup_file) =
        map_app_popup(&mut app, &mut app_queue, &parent_xdg);
    input.send("key 30 down");
    input.send("key 30 up");
    wait_for(
        &mut app_queue,
        &mut app,
        Duration::from_secs(5),
        "popup grab received the pre-lock key",
        |s| s.keys.contains(&30),
    );

    // Lock: the compositor must unset the keyboard grab, dismiss the popup,
    // and focus the lock surface.
    let _locked = lock_and_map(&mut lock_state, &mut lock_queue);
    wait_for(
        &mut app_queue,
        &mut app,
        Duration::from_secs(5),
        "popup dismissed by the lock",
        |s| s.popup_done,
    );

    let app_keys_before = app.keys.len();
    input.send("key 31 down");
    input.send("key 31 up");
    wait_for(
        &mut lock_queue,
        &mut lock_state,
        Duration::from_secs(5),
        "locked key delivered to the lock surface",
        |s| s.keys.contains(&31),
    );

    // Flush anything the compositor might have sent the app before the lock
    // surface's key arrived. A leaked key would already be queued here.
    app_queue.roundtrip(&mut app).expect("app roundtrip failed");
    assert_eq!(
        app.keys.len(),
        app_keys_before,
        "a locked key reached the popup client: {:?}",
        app.keys
    );
    assert!(
        !app.keys.contains(&31),
        "the popup client received the lock-screen key"
    );

    let report = input.query("query lock");
    assert!(
        report.contains("lock locked=1 surfaces=1 lock-focus=1"),
        "the lock surface did not own the keyboard: {report}"
    );
}

#[test]
fn lock_releases_a_held_pointer_drag() {
    let socket_name = format!("dragonfruit-test-lock-drag-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let _ = std::fs::remove_file(&synthetic_path);
    let token = "5b".repeat(32);
    let proc = CompositorProcess::start_with_synthetic(
        &socket_name,
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);

    let (_lock_conn, mut lock_queue, mut lock_state) = connect(&proc.socket_path);
    authenticate(&mut lock_state, &mut lock_queue, proc.read_token());
    let (_app_conn, mut app_queue, mut app) = connect_app(&proc.socket_path);
    let (_surface, _parent, _toplevel, _file) = map_app_toplevel(&mut app, &mut app_queue);

    // A held titlebar move is the strongest pointer grab: it consumes motion
    // and moves the window.
    start_titlebar_drag(&input);

    let _locked = lock_and_map(&mut lock_state, &mut lock_queue);
    // The lock sends the app a pointer leave; drain it before counting.
    app_queue.roundtrip(&mut app).expect("app roundtrip failed");
    let motions_before = app.pointer_motions;
    let buttons_before = app.pointer_buttons.len();
    let lock_buttons_before = lock_state.pointer_buttons.len();
    let location_before = wait_for_decoration(&input).content;

    // Motion and buttons while locked must go to the lock surface only; the
    // held drag must not continue moving the window.
    input.send("motion 70 60");
    input.send("button 272 up");
    input.send("button 272 down");
    input.send("button 272 up");
    wait_for(
        &mut lock_queue,
        &mut lock_state,
        Duration::from_secs(5),
        "locked pointer buttons on the lock surface",
        |s| s.pointer_buttons.len() > lock_buttons_before,
    );

    app_queue.roundtrip(&mut app).expect("app roundtrip failed");
    assert_eq!(
        app.pointer_motions, motions_before,
        "a locked pointer motion reached the dragged client"
    );
    assert_eq!(
        app.pointer_buttons.len(),
        buttons_before,
        "a locked pointer button reached the dragged client"
    );
    let location_after = wait_for_decoration(&input).content;
    assert_eq!(
        location_after, location_before,
        "the held drag survived the lock and moved the window"
    );
}

#[test]
fn suspend_resume_releases_a_held_pointer_drag() {
    let socket_name = format!("dragonfruit-test-suspend-drag-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let _ = std::fs::remove_file(&synthetic_path);
    let token = "5c".repeat(32);
    let proc = CompositorProcess::start_with_synthetic(
        &socket_name,
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);

    let (_app_conn, mut app_queue, mut app) = connect_app(&proc.socket_path);
    let (_surface, _parent, _toplevel, _file) = map_app_toplevel(&mut app, &mut app_queue);
    start_titlebar_drag(&input);

    input.send("suspend");
    wait_for_session(&input, 1);
    input.send("resume");
    wait_for_session(&input, 0);

    // The grab was torn down by the suspend. A motion after resume must not
    // keep moving the window.
    let location_before = wait_for_decoration(&input).content;
    input.send("motion 60 45");
    let location_after = wait_for_decoration(&input).content;
    assert_eq!(
        location_after, location_before,
        "a held drag survived suspend/resume and moved the window"
    );
    input.send("button 272 up");
}

/// Wait until `query session` reports the requested suspend flag.
#[track_caller]
fn wait_for_session(input: &SyntheticInput, suspended: u32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let report = input.query("query session");
        if report.contains(&format!("session suspended={suspended} ")) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "session never reached suspended={suspended}: {report}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
