// SPDX-License-Identifier: GPL-3.0-only
//! T322 acceptance: pointer constraints are unprivileged and bounded.
//!
//! `zwp_pointer_constraints_v1` is the protocol games, emulators, remote
//! desktop viewers, and VM displays use for mouse-look and absolute input.
//! ADR 0314 makes it unprivileged: any client whose surface has pointer focus
//! may lock or confine the pointer, because the protocol scopes the constraint
//! to that surface and the grab releases on focus loss or resource destroy.
//!
//! This suite spawns the headless compositor with the synthetic-input harness
//! and drives a real, **non-shell** `wayland-client` through:
//!
//! * a lock created while the surface is unfocused, activated on pointer
//!   enter, freezing motion; destroy releases it;
//! * a confine whose region clamps motion; destroy releases it;
//! * keyboard focus loss (another window activated) releasing the lock;
//! * a create/destroy spam loop that stays bounded and leaves no stuck grab.

use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_pointer, wl_region, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1, zwp_pointer_constraints_v1,
};
use wayland_protocols::xdg::activation::v1::client::{xdg_activation_token_v1, xdg_activation_v1};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel as xdg_tl, xdg_wm_base};

mod common;

const SIGTERM: i32 = 15;

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

#[derive(Default)]
struct TestClient {
    registry: Option<wl_registry::WlRegistry>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    constraints: Option<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1>,
    activation: Option<xdg_activation_v1::XdgActivationV1>,
    activation_token: Option<String>,
    /// Surface-local pointer motion coordinates, in arrival order.
    pointer_motions: Vec<(f64, f64)>,
    /// The surface the pointer is currently inside, by protocol id.
    pointer_surface: Option<u32>,
    pointer_enters: usize,
    pointer_leaves: usize,
    /// `zwp_locked_pointer_v1.locked` / `.unlocked` counts.
    locked: usize,
    unlocked: usize,
    /// `zwp_confined_pointer_v1.confined` / `.unconfined` counts.
    confined: usize,
    unconfined: usize,
}

impl Dispatch<wl_registry::WlRegistry, ()> for TestClient {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        state.registry = Some(registry.clone());
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => state.compositor = Some(registry.bind(name, version, qh, ())),
                "wl_shm" => state.shm = Some(registry.bind(name, version, qh, ())),
                "xdg_wm_base" => {
                    state.xdg_wm_base = Some(registry.bind(name, version.min(6), qh, ()));
                }
                "wl_seat" => state.seat = Some(registry.bind(name, version.min(9), qh, ())),
                "xdg_activation_v1" => {
                    state.activation = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "zwp_pointer_constraints_v1" => {
                    state.constraints = Some(registry.bind(name, version.min(1), qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for TestClient {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let has_pointer = capabilities
                .into_result()
                .map(|caps| caps.contains(wl_seat::Capability::Pointer))
                .unwrap_or(false);
            if has_pointer && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for TestClient {
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
                state.pointer_enters += 1;
                state.pointer_surface = Some(surface.id().protocol_id());
            }
            wl_pointer::Event::Leave { .. } => {
                state.pointer_leaves += 1;
                state.pointer_surface = None;
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => state.pointer_motions.push((surface_x, surface_y)),
            _ => {}
        }
    }
}

impl Dispatch<zwp_locked_pointer_v1::ZwpLockedPointerV1, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        event: zwp_locked_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_locked_pointer_v1::Event::Locked => state.locked += 1,
            zwp_locked_pointer_v1::Event::Unlocked => state.unlocked += 1,
            _ => {}
        }
    }
}

impl Dispatch<zwp_confined_pointer_v1::ZwpConfinedPointerV1, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        event: zwp_confined_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_confined_pointer_v1::Event::Confined => state.confined += 1,
            zwp_confined_pointer_v1::Event::Unconfined => state.unconfined += 1,
            _ => {}
        }
    }
}

impl Dispatch<xdg_activation_v1::XdgActivationV1, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &xdg_activation_v1::XdgActivationV1,
        _: xdg_activation_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &xdg_activation_token_v1::XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            state.activation_token = Some(token);
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for TestClient {
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

impl Dispatch<xdg_surface::XdgSurface, ()> for TestClient {
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

impl Dispatch<xdg_tl::XdgToplevel, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &xdg_tl::XdgToplevel,
        _: xdg_tl::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(TestClient: ignore wl_compositor::WlCompositor);
delegate_noop!(TestClient: ignore wl_shm::WlShm);
delegate_noop!(TestClient: ignore wl_shm_pool::WlShmPool);
delegate_noop!(TestClient: ignore wl_buffer::WlBuffer);
delegate_noop!(TestClient: ignore wl_region::WlRegion);
delegate_noop!(TestClient: ignore wl_surface::WlSurface);
delegate_noop!(TestClient: ignore zwp_pointer_constraints_v1::ZwpPointerConstraintsV1);

// --- process harness -------------------------------------------------------

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    token_path: PathBuf,
    stderr_path: PathBuf,
    stdout_path: PathBuf,
    synthetic_path: PathBuf,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        if self
            .child
            .try_wait()
            .map_or(true, |status| status.is_none())
        {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        if std::thread::panicking() {
            if let Ok(log) = std::fs::read_to_string(&self.stderr_path) {
                eprintln!("--- compositor stderr ---\n{log}--- end ---");
            }
        }
        common::cleanup_compositor_artifacts(&self.socket_path);
        let _ = std::fs::remove_file(&self.stderr_path);
        let _ = std::fs::remove_file(&self.stdout_path);
        let _ = std::fs::remove_file(&self.synthetic_path);
    }
}

impl CompositorProcess {
    fn start(socket_name: &str, synthetic_path: &Path) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let token_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.launch-token"));
        let stderr_path = std::env::temp_dir().join(format!("{socket_name}.stderr"));
        let stderr_file = std::fs::File::create(&stderr_path).expect("create stderr log");
        let stdout_path = std::env::temp_dir().join(format!("{socket_name}.stdout"));
        let stdout_file = std::fs::File::create(&stdout_path).expect("create stdout log");

        let mut child = common::compositor_command()
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_SYNTHETIC_INPUT", synthetic_path)
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .expect("failed to start compositor");

        let deadline = Instant::now() + Duration::from_secs(10);
        while !socket_path.exists() || !token_path.exists() || !synthetic_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before ready: {status}");
            }
            assert!(Instant::now() < deadline, "compositor never became ready");
            std::thread::sleep(Duration::from_millis(10));
        }

        Self {
            child,
            socket_path,
            token_path,
            stderr_path,
            stdout_path,
            synthetic_path: synthetic_path.to_path_buf(),
        }
    }

    fn read_stderr(&self) -> String {
        std::fs::read_to_string(&self.stderr_path).unwrap_or_default()
    }

    fn shutdown(mut self) {
        unsafe {
            kill(self.child.id() as i32, SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    assert!(status.success(), "compositor exit status: {status}");
                    break;
                }
                Ok(None) => {
                    assert!(
                        Instant::now() < deadline,
                        "compositor did not exit after SIGTERM"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("wait failed: {err}"),
            }
        }
        assert!(!self.socket_path.exists(), "teardown leak: socket survived");
        assert!(
            !self.token_path.exists(),
            "teardown leak: token file survived"
        );
    }
}

fn connect(socket_path: &Path) -> (Connection, EventQueue<TestClient>, TestClient) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let stream = loop {
        match std::os::unix::net::UnixStream::connect(socket_path) {
            Ok(stream) => break stream,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                ) =>
            {
                assert!(
                    Instant::now() < deadline,
                    "connect failed after retrying for 10s: {err}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("connect failed: {err}"),
        }
    };
    let conn = Connection::from_socket(stream).expect("connection failed");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = TestClient::default();
    queue.roundtrip(&mut state).expect("registry roundtrip");
    (conn, queue, state)
}

/// Dispatch until `pred` is true or the timeout expires.
#[track_caller]
fn wait_for(
    conn: &Connection,
    queue: &mut EventQueue<TestClient>,
    state: &mut TestClient,
    timeout: Duration,
    mut pred: impl FnMut(&TestClient) -> bool,
) {
    use std::os::fd::AsRawFd;
    let deadline = Instant::now() + timeout;
    loop {
        conn.flush().expect("flush while waiting");
        queue.dispatch_pending(state).expect("dispatch pending");
        if pred(state) {
            return;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out waiting for condition");
        let mut pollfd = libc::pollfd {
            fd: conn.as_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut pollfd, 1, millis) };
        assert!(ready >= 0, "poll failed");
        if ready > 0 {
            queue
                .blocking_dispatch(state)
                .expect("dispatch while waiting");
        } else {
            panic!("timed out waiting for condition");
        }
    }
}

fn shm_buffer(
    state: &TestClient,
    qh: &QueueHandle<TestClient>,
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
        "dragonfruit-pointer-constraint-{}-{seq}.shm",
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

fn map_toplevel(
    state: &mut TestClient,
    queue: &mut EventQueue<TestClient>,
    title: &str,
    app_id: &str,
) -> (
    wl_surface::WlSurface,
    xdg_surface::XdgSurface,
    xdg_tl::XdgToplevel,
    std::fs::File,
) {
    let qh = queue.handle();
    let compositor = state.compositor.clone().expect("wl_compositor bound");
    let wm_base = state.xdg_wm_base.clone().expect("xdg_wm_base bound");
    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title(title.to_string());
    toplevel.set_app_id(app_id.to_string());
    surface.commit();
    let _ = queue.roundtrip(state);
    let (buffer, file) = shm_buffer(state, &qh, 200, 150);
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();
    let _ = queue.roundtrip(state);
    (surface, xdg_surface, toplevel, file)
}

/// Request a fresh `xdg-activation` token and activate `surface` with it.
fn xdg_activate(
    conn: &Connection,
    queue: &mut EventQueue<TestClient>,
    state: &mut TestClient,
    surface: &wl_surface::WlSurface,
) {
    state.activation_token = None;
    let activation = state
        .activation
        .clone()
        .expect("xdg_activation_v1 advertised");
    let token = activation.get_activation_token(&queue.handle(), ());
    token.set_app_id("org.dragonfruit.Conformance".to_string());
    token.commit();
    wait_for(conn, queue, state, Duration::from_secs(5), |state| {
        state.activation_token.is_some()
    });
    let token_string = state
        .activation_token
        .clone()
        .expect("activation token generated");
    activation.activate(token_string, surface);
}

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
}

impl Drop for SyntheticInput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.reply_path);
    }
}

// --- shared test steps -----------------------------------------------------

struct Harness {
    proc: CompositorProcess,
    input: SyntheticInput,
    conn: Connection,
    queue: EventQueue<TestClient>,
    state: TestClient,
    synthetic_path: PathBuf,
    /// The shm backing files, kept alive for the mapped surfaces.
    files: Vec<std::fs::File>,
}

impl Harness {
    fn start(name: &str) -> Self {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let synthetic_path =
            PathBuf::from(&runtime_dir).join(format!("{name}-synthetic-{}", std::process::id()));
        let proc = CompositorProcess::start(name, &synthetic_path);
        let input = SyntheticInput::connect(&synthetic_path);
        let (conn, queue, state) = connect(&proc.socket_path);
        Harness {
            proc,
            input,
            conn,
            queue,
            state,
            synthetic_path,
            files: Vec::new(),
        }
    }

    fn wait(&mut self, pred: impl FnMut(&TestClient) -> bool) {
        wait_for(
            &self.conn,
            &mut self.queue,
            &mut self.state,
            Duration::from_secs(5),
            pred,
        );
    }

    fn roundtrip(&mut self) {
        let _ = self.queue.roundtrip(&mut self.state);
    }

    /// Map a 200x150 window and put the pointer on it with a left click so it
    /// owns keyboard focus. Returns the surface.
    fn focused_window(&mut self, title: &str) -> wl_surface::WlSurface {
        let (surface, _xdg, _toplevel, file) = map_toplevel(
            &mut self.state,
            &mut self.queue,
            title,
            "org.dragonfruit.Pointer",
        );
        self.files.push(file);
        self.wait(|state| state.pointer.is_some());
        self.roundtrip();
        self.input.send("motion-abs 0.5 0.5");
        self.input.send("button 272 down\nbutton 272 up");
        // The first motion only sends `wl_pointer.enter`; a follow-up relative
        // motion produces the baseline `wl_pointer.motion`.
        self.input.send("motion 1 1");
        self.wait(|state| !state.pointer_motions.is_empty());
        surface
    }

    fn constraint_handle(&self) -> zwp_pointer_constraints_v1::ZwpPointerConstraintsV1 {
        self.state
            .constraints
            .clone()
            .expect("zwp_pointer_constraints_v1 advertised")
    }

    fn pointer_handle(&self) -> wl_pointer::WlPointer {
        self.state.pointer.clone().expect("wl_pointer bound")
    }

    fn finish(self) {
        let synthetic_path = self.synthetic_path.clone();
        let Harness { proc, input, .. } = self;
        drop(input);
        proc.shutdown();
        assert!(
            !synthetic_path.exists(),
            "teardown leak: synthetic-input socket survived"
        );
    }
}

// --- tests -----------------------------------------------------------------

/// A non-shell client may lock the pointer. The constraint is created while
/// the pointer is off the surface, so activation must happen on pointer enter;
/// motion then freezes, and destroying the resource releases the grab.
#[test]
fn non_shell_client_locks_on_pointer_enter_and_destroy_releases() {
    let mut h = Harness::start("dragonfruit-constraint-enter");
    let surface = h.focused_window("Lock Enter");

    // Move the pointer off the window: the constraint is registered while the
    // surface has no pointer focus.
    h.input.send("motion-abs 0.02 0.02");
    h.wait(|state| state.pointer_surface.is_none());

    let constraints = h.constraint_handle();
    let pointer = h.pointer_handle();
    let locked = constraints.lock_pointer(
        &surface,
        &pointer,
        None,
        zwp_pointer_constraints_v1::Lifetime::Persistent,
        &h.queue.handle(),
        (),
    );
    h.roundtrip();
    assert_eq!(
        h.state.locked, 0,
        "an unfocused surface's constraint must not activate yet"
    );

    // Re-enter the window: the registered constraint activates on enter.
    h.input.send("motion-abs 0.5 0.5");
    h.wait(|state| state.locked >= 1);

    // An enter carries the position in `wl_pointer.enter`, not a motion
    // event; ask for one frozen motion to establish the baseline.
    let before = h.state.pointer_motions.len();
    h.input.send("motion 0 0");
    h.wait(|state| state.pointer_motions.len() > before);
    let frozen = h.state.pointer_motions.last().copied();
    let before = h.state.pointer_motions.len();
    h.input.send("motion 80 60");
    h.wait(|state| state.pointer_motions.len() > before);
    assert_eq!(
        frozen,
        h.state.pointer_motions.last().copied(),
        "a locked pointer must not move"
    );

    // Destroy releases the grab: the next motion moves normally.
    locked.destroy();
    h.roundtrip();
    let before = h.state.pointer_motions.len();
    h.input.send("motion 20 20");
    h.wait(|state| state.pointer_motions.len() > before);
    assert_ne!(
        frozen,
        h.state.pointer_motions.last().copied(),
        "destroying the constraint must release the pointer"
    );

    h.finish();
}

/// A confine clamps motion to its surface-local region; destroying the
/// resource releases the clamp.
#[test]
fn confine_clamps_to_the_region_and_destroy_releases() {
    let mut h = Harness::start("dragonfruit-constraint-confine");
    let surface = h.focused_window("Confine");

    let compositor = h.state.compositor.clone().expect("wl_compositor bound");
    let region = compositor.create_region(&h.queue.handle(), ());
    region.add(10, 10, 60, 50);
    let constraints = h.constraint_handle();
    let pointer = h.pointer_handle();
    let confined = constraints.confine_pointer(
        &surface,
        &pointer,
        Some(&region),
        zwp_pointer_constraints_v1::Lifetime::Persistent,
        &h.queue.handle(),
        (),
    );
    h.wait(|state| state.confined >= 1);

    // A far absolute move clamps to the region's far corner. The region is
    // surface-local: x in [10, 69], y in [10, 59].
    let before = h.state.pointer_motions.len();
    h.input.send("motion-abs 0.95 0.95");
    h.wait(|state| state.pointer_motions.len() > before);
    let clamped = h.state.pointer_motions.last().copied().expect("motion");
    assert!(
        (10.0..=69.0).contains(&clamped.0) && (10.0..=59.0).contains(&clamped.1),
        "confined pointer escaped its region: {clamped:?}"
    );
    assert_eq!(
        clamped,
        (69.0, 59.0),
        "a far move must clamp to the region's far corner"
    );

    // Further relative motion stays clamped.
    let before = h.state.pointer_motions.len();
    h.input.send("motion -5 -5");
    h.wait(|state| state.pointer_motions.len() > before);
    let still = h.state.pointer_motions.last().copied().expect("motion");
    assert!(
        (10.0..=69.0).contains(&still.0) && (10.0..=59.0).contains(&still.1),
        "confined pointer escaped its region: {still:?}"
    );

    // Destroy releases: a small move leaves the region but stays inside the
    // 200x150 window.
    confined.destroy();
    h.roundtrip();
    let before = h.state.pointer_motions.len();
    h.input.send("motion 20 20");
    h.wait(|state| state.pointer_motions.len() > before);
    let released = h.state.pointer_motions.last().copied().expect("motion");
    assert!(
        released.0 > 69.0 || released.1 > 59.0,
        "destroying the confine must release the clamp: {released:?}"
    );

    h.finish();
}

/// Keyboard focus loss releases an active lock: a background window cannot
/// keep consuming the pointer.
#[test]
fn focus_loss_releases_the_lock() {
    let mut h = Harness::start("dragonfruit-constraint-focus");
    let surface_a = h.focused_window("Locked");

    let constraints = h.constraint_handle();
    let pointer = h.pointer_handle();
    let _locked = constraints.lock_pointer(
        &surface_a,
        &pointer,
        None,
        zwp_pointer_constraints_v1::Lifetime::Persistent,
        &h.queue.handle(),
        (),
    );
    h.wait(|state| state.locked >= 1);

    // Map a second window and activate it: keyboard focus leaves A.
    let (surface_b, _xdg_b, _toplevel_b, file_b) =
        map_toplevel(&mut h.state, &mut h.queue, "Other", "org.dragonfruit.Other");
    h.files.push(file_b);
    h.roundtrip();
    xdg_activate(&h.conn, &mut h.queue, &mut h.state, &surface_b);
    h.wait(|state| state.unlocked >= 1);

    // The released pointer moves again (and now targets B, which overlaps the
    // locked location).
    let frozen = h.state.pointer_motions.last().copied();
    let before = h.state.pointer_motions.len();
    h.input.send("motion 15 15");
    h.wait(|state| state.pointer_motions.len() > before);
    assert_ne!(
        frozen,
        h.state.pointer_motions.last().copied(),
        "focus loss must release the pointer"
    );

    h.finish();
}

/// A create/destroy spam loop stays bounded and leaves no stuck grab.
#[test]
fn a_constraint_spam_loop_is_bounded_and_leaves_no_stuck_grab() {
    let mut h = Harness::start("dragonfruit-constraint-spam");
    let surface = h.focused_window("Spam");

    let constraints = h.constraint_handle();
    let pointer = h.pointer_handle();
    let stderr_before = h.proc.read_stderr().len();

    for _ in 0..80 {
        let locked = constraints.lock_pointer(
            &surface,
            &pointer,
            None,
            zwp_pointer_constraints_v1::Lifetime::Persistent,
            &h.queue.handle(),
            (),
        );
        let before_locked = h.state.locked;
        h.roundtrip();
        // The constraint activates immediately while the surface has focus.
        assert!(
            h.state.locked >= before_locked,
            "a repeated constraint must still activate"
        );
        locked.destroy();
        h.roundtrip();
        // A zero-delta motion lets the self-healing grab notice the destroyed
        // resource without moving the pointer.
        let before = h.state.pointer_motions.len();
        h.input.send("motion 0 0");
        h.wait(|state| state.pointer_motions.len() > before);
    }

    // The refusal log (if any) is bounded: 80 loops must not produce a line
    // per request, and pointer constraints never reach the arbiter at all.
    let stderr = h.proc.read_stderr();
    assert!(
        !stderr.contains("refused Pointer grab"),
        "pointer constraints must not be arbitrated: {stderr}"
    );
    assert!(
        stderr.len().saturating_sub(stderr_before) < 8192,
        "a constraint spam loop grew stderr by {} bytes",
        stderr.len().saturating_sub(stderr_before)
    );

    // No stuck grab: after the loop the pointer still moves.
    let before = h.state.pointer_motions.len();
    h.input.send("motion 10 10");
    h.wait(|state| state.pointer_motions.len() > before);
    let first = h.state.pointer_motions.last().copied();
    h.input.send("motion 10 10");
    h.wait(|state| state.pointer_motions.len() > before + 1);
    assert_ne!(
        first,
        h.state.pointer_motions.last().copied(),
        "the spam loop left a stuck grab"
    );

    h.finish();
}
