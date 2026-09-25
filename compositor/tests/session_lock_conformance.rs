// SPDX-License-Identifier: MIT
//! T-12.3a acceptance: `ext-session-lock-v1` protocol conformance.
//!
//! Spawns the compositor on the headless backend and drives a real
//! `wayland-client` through a full lock:
//!
//! * the manager global is advertised and bindable,
//! * `lock` is confirmed with the `locked` event (fail-secure),
//! * every advertised output gets a lock surface, and the compositor
//!   configures it to exactly that output's size (the "UI covers every
//!   output" contract),
//! * the client can ack the configure, attach a full-output buffer, commit it,
//!   and later `unlock_and_destroy` without a protocol error.

use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1 as lock_mgr, ext_session_lock_surface_v1 as lock_surface,
    ext_session_lock_v1 as lock,
};

/// The headless backend's default mode (`backend::HEADLESS_MODE_SIZE`).
const OUTPUT_W: u32 = 1280;
const OUTPUT_H: u32 = 720;

/// One connected lock client.
#[derive(Default)]
struct LockTest {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    outputs: Vec<wl_output::WlOutput>,
    lock_manager: Option<lock_mgr::ExtSessionLockManagerV1>,
    locked: bool,
    finished: bool,
    /// `(serial, width, height)` for every lock-surface configure.
    configures: Vec<(u32, u32, u32)>,
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
                "wl_output" => {
                    state
                        .outputs
                        .push(registry.bind(name, version.min(4), qh, ()));
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
    socket_path: std::path::PathBuf,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl CompositorProcess {
    fn start(socket_name: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(socket_name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start compositor");

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = std::path::PathBuf::from(runtime_dir).join(socket_name);

        let deadline = Instant::now() + Duration::from_secs(10);
        while !socket_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before socket appeared: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "compositor socket never appeared"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        Self { child, socket_path }
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
    let proc = CompositorProcess::start(&socket_name);

    let stream = UnixStream::connect(&proc.socket_path).expect("failed to connect");
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = LockTest::default();

    // Advertise + bind the globals.
    queue.roundtrip(&mut state).expect("roundtrip failed");
    let compositor = state.compositor.clone().expect("wl_compositor bound");
    let manager = state
        .lock_manager
        .clone()
        .expect("ext_session_lock_manager_v1 advertised");
    assert_eq!(state.outputs.len(), 1, "one headless output");

    // Lock the session and create a lock surface covering the output.
    let lock = manager.lock(&qh, ());
    let surface = compositor.create_surface(&qh, ());
    let lock_surface = lock.get_lock_surface(&surface, &state.outputs[0], &qh, ());

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
    let (buffer, _file) = shm_buffer(&state, &qh, width as i32, height as i32);
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
    let second = manager.lock(&qh, ());
    let second_surface = compositor.create_surface(&qh, ());
    let _second_lock_surface = second.get_lock_surface(&second_surface, &state.outputs[0], &qh, ());
    wait_for(
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        "second configure",
        |s| s.configures.len() >= 2,
    );
    second.unlock_and_destroy();
}
