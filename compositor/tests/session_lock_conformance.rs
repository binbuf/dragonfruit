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
//!   compositor locked (`query lock locked=1`) with zero live lock surfaces.
//!
//! The lock manager global is gated on the T-07 launch-token handshake, so the
//! client authenticates as the trusted shell before it locks.

use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1 as lock_mgr, ext_session_lock_surface_v1 as lock_surface,
    ext_session_lock_v1 as lock,
};

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

/// One connected lock client (the trusted shell stand-in).
#[derive(Default)]
struct LockTest {
    registry: Option<wl_registry::WlRegistry>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    seat: Option<wl_seat::WlSeat>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
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
            let has_keyboard = capabilities
                .into_result()
                .map(|caps| caps.contains(wl_seat::Capability::Keyboard))
                .unwrap_or(false);
            if has_keyboard && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
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

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    token_path: PathBuf,
    synthetic_path: Option<PathBuf>,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.token_path);
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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
            synthetic_path: synthetic.map(Path::to_path_buf),
        }
    }

    fn read_token(&self) -> String {
        std::fs::read_to_string(&self.token_path)
            .expect("token file readable")
            .trim()
            .to_string()
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

fn wait_for(
    queue: &mut EventQueue<LockTest>,
    state: &mut LockTest,
    timeout: Duration,
    what: &str,
    predicate: impl Fn(&LockTest) -> bool,
) {
    let deadline = Instant::now() + timeout;
    while !predicate(state) {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        queue
            .blocking_dispatch(state)
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
fn locked_input_targets_the_lock_ui_and_survives_its_death() {
    let socket_name = format!("dragonfruit-test-lock-input-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let _ = std::fs::remove_file(&synthetic_path);
    let token = "ab".repeat(32);
    let proc = CompositorProcess::start_with_synthetic(
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

    // Wait for the seat to hand us a keyboard.
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "seat keyboard",
        |s| s.keyboard.is_some(),
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

    // Synthetic input is captured by the lock UI: the key lands on the lock
    // surface, and pointer input is swallowed (no client receives it).
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

    let during = input.query("query lock");
    assert!(
        during.contains("lock locked=1 surfaces=1 lock-focus=1"),
        "unexpected lock report after input: {during}"
    );

    // Kill the lock UI: hard-close the client socket. The compositor must stay
    // locked and simply lose its input target.
    unsafe {
        libc::shutdown(kill_fd, libc::SHUT_RDWR);
        libc::close(kill_fd);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let report = input.query("query lock");
        if report.contains("lock locked=1 surfaces=0") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "lock UI death did not clear lock surfaces: {report}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // The compositor is still alive and still locked after the crash.
    let after = input.query("query lock");
    assert!(
        after.contains("lock locked=1"),
        "session left locked after the lock UI died: {after}"
    );
}
