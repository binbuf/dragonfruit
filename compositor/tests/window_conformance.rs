// SPDX-License-Identifier: MIT OR Apache-2.0
//! T-04 acceptance: headless protocol conformance for the window model.
//!
//! Spawns the compositor on the headless backend and drives a real
//! `wayland-client` through the `xdg-shell` state machine:
//!
//! * map a toplevel with an `wl_shm` buffer (T-02 "windowed client
//!   end-to-end" on the headless backend),
//! * maximize/unmaximize (must map to Zoom and restore the floating size —
//!   FR-1/FR-2),
//! * fullscreen/unfullscreen (must restore the state it was entered from),
//! * create a constrained `xdg_popup` and assert the configure stays inside
//!   the output (FR-11).
//!
//! This is the protocol-level companion to the pure state-machine and
//! positioner unit tests.

use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::xdg::shell::client::{
    xdg_popup, xdg_positioner, xdg_surface, xdg_toplevel, xdg_wm_base,
};

/// Headless output size (see `backend/mod.rs::HEADLESS_MODE_SIZE`).
const OUTPUT_W: i32 = 1280;
const OUTPUT_H: i32 = 720;

/// The toplevel buffer size; the compositor's floating geometry (and thus
/// the restored configure after unzoom/unfullscreen) uses this.
const WINDOW_W: i32 = 200;
const WINDOW_H: i32 = 150;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToplevelConfigure {
    width: i32,
    height: i32,
    states: Vec<u32>,
}

impl ToplevelConfigure {
    fn has(&self, state: xdg_toplevel::State) -> bool {
        self.states.contains(&u32::from(state))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PopupConfigure {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Default)]
struct TestClient {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    /// Serial of each pointer button press delivered to the client.
    pointer_button_serials: Vec<u32>,
    toplevel_configures: Vec<ToplevelConfigure>,
    popup_configures: Vec<PopupConfigure>,
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
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(6), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base = Some(registry.bind(name, version.min(6), qh, ()));
                }
                "wl_seat" => {
                    state.seat = Some(registry.bind(name, version.min(9), qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
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
        if let wl_pointer::Event::Button { serial, .. } = event {
            state.pointer_button_serials.push(serial);
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

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Configure {
            width,
            height,
            states,
        } = event
        {
            state.toplevel_configures.push(ToplevelConfigure {
                width,
                height,
                states: parse_states(&states),
            });
        }
    }
}

impl Dispatch<xdg_positioner::XdgPositioner, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &xdg_positioner::XdgPositioner,
        _: xdg_positioner::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_popup::XdgPopup, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &xdg_popup::XdgPopup,
        event: xdg_popup::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_popup::Event::Configure {
            x,
            y,
            width,
            height,
        } = event
        {
            state.popup_configures.push(PopupConfigure {
                x,
                y,
                width,
                height,
            });
        }
    }
}

/// Decode the `xdg_toplevel.configure` states array (native-endian u32s).
fn parse_states(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    synthetic_path: Option<PathBuf>,
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
        // Best-effort cleanup for a hard-killed run (SIGKILL skips the
        // compositor's own socket removal).
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(self.socket_path.with_extension("lock"));
        if let Some(path) = &self.synthetic_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl CompositorProcess {
    fn start(socket_name: &str) -> Self {
        Self::start_with_synthetic(socket_name, None)
    }

    /// Start with the T-03 synthetic-input harness bound at
    /// `synthetic_path` (`DRAGONFRUIT_SYNTHETIC_INPUT`).
    fn start_with_synthetic(socket_name: &str, synthetic_path: Option<&Path>) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let mut command = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"));
        command
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(path) = synthetic_path {
            command.env("DRAGONFRUIT_SYNTHETIC_INPUT", path);
        }
        let mut child = command.spawn().expect("failed to start compositor");

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(runtime_dir).join(&socket_name);

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
        if let Some(path) = synthetic_path {
            while !path.exists() {
                assert!(
                    Instant::now() < deadline,
                    "synthetic-input socket never appeared"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        Self {
            child,
            socket_path,
            synthetic_path: synthetic_path.map(Path::to_path_buf),
        }
    }

    /// SIGTERM the compositor and assert it tears down cleanly (no stray
    /// socket). Consumes the process; `Drop` cleans up any leftovers.
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
        assert!(
            !self.socket_path.exists(),
            "teardown leak: socket {} survived exit",
            self.socket_path.display()
        );
    }
}

const SIGTERM: i32 = 15;

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

/// Sends synthetic-input commands to the T-03 harness socket.
struct SyntheticInput {
    socket: std::os::unix::net::UnixDatagram,
    path: PathBuf,
}

impl SyntheticInput {
    fn connect(path: &Path) -> Self {
        let socket = std::os::unix::net::UnixDatagram::unbound().expect("unbound datagram");
        Self {
            socket,
            path: path.to_path_buf(),
        }
    }

    #[track_caller]
    fn send(&self, command: &str) {
        self.socket
            .send_to(command.as_bytes(), &self.path)
            .unwrap_or_else(|err| panic!("failed to send synthetic {command:?}: {err}"));
    }
}

/// Dispatch until `pred` is true or the timeout expires (bounded with
/// `poll`, so a missing event fails rather than hangs). `#[track_caller]`
/// points a timeout panic at the waiting call site.
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

fn connect(socket_path: &Path) -> (Connection, EventQueue<TestClient>, TestClient) {
    let stream = std::os::unix::net::UnixStream::connect(socket_path).expect("failed to connect");
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _display = conn.display();
    let _registry = conn.display().get_registry(&qh, ());

    let mut state = TestClient::default();
    queue.roundtrip(&mut state).expect("roundtrip failed");
    queue.roundtrip(&mut state).expect("roundtrip failed");
    assert!(state.compositor.is_some(), "wl_compositor not advertised");
    assert!(state.shm.is_some(), "wl_shm not advertised");
    assert!(state.xdg_wm_base.is_some(), "xdg_wm_base not advertised");
    (conn, queue, state)
}

/// Create a shared-memory pool from an anonymous temp file and return a
/// buffer plus the backing file (which must outlive the first flush).
fn shm_buffer(
    state: &TestClient,
    qh: &QueueHandle<TestClient>,
    width: i32,
    height: i32,
) -> (wl_buffer::WlBuffer, std::fs::File) {
    let shm = state.shm.clone().expect("wl_shm bound");
    let stride = width * 4;
    let size = (stride * height) as usize;

    // A unique name per buffer: a test may map several windows, and the
    // backing file is created with `create_new`.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SHM_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SHM_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-conformance-{}-{seq}-{:p}.shm",
        std::process::id(),
        &shm
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

/// Build and map a toplevel, returning its proxies.
#[allow(clippy::type_complexity)]
fn map_toplevel(
    state: &mut TestClient,
    queue: &mut EventQueue<TestClient>,
) -> (
    wl_surface::WlSurface,
    xdg_surface::XdgSurface,
    xdg_toplevel::XdgToplevel,
    std::fs::File,
) {
    let qh = queue.handle();
    let compositor = state.compositor.clone().unwrap();
    let wm_base = state.xdg_wm_base.clone().unwrap();

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Conformance".into());
    // First commit with no buffer triggers the initial configure.
    surface.commit();
    queue.roundtrip(state).expect("initial configure");

    // Acknowledge the initial configure happened in the handler, then map.
    let (buffer, file) = shm_buffer(state, &qh, WINDOW_W, WINDOW_H);
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();
    queue.roundtrip(state).expect("map commit");

    (surface, xdg_surface, toplevel, file)
}

#[test]
fn toplevel_zoom_and_fullscreen_round_trip_over_protocol() {
    let proc = CompositorProcess::start("dragonfruit-conformance-toplevel");
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, _xdg_surface, toplevel, _file) = map_toplevel(&mut state, &mut queue);

    // --- maximize maps to Zoom, fills the output -------------------------
    state.toplevel_configures.clear();
    toplevel.set_maximized();
    queue.roundtrip(&mut state).expect("maximize configure");
    let zoomed = state
        .toplevel_configures
        .last()
        .expect("maximize must produce a configure")
        .clone();
    assert!(
        zoomed.has(xdg_toplevel::State::Maximized),
        "maximize configure must carry the maximized state: {zoomed:?}"
    );
    assert_eq!((zoomed.width, zoomed.height), (OUTPUT_W, OUTPUT_H));

    // --- unmaximize restores the floating geometry -----------------------
    state.toplevel_configures.clear();
    toplevel.unset_maximized();
    queue.roundtrip(&mut state).expect("unmaximize configure");
    let restored = state
        .toplevel_configures
        .last()
        .expect("unmaximize must produce a configure")
        .clone();
    assert!(
        !restored.has(xdg_toplevel::State::Maximized),
        "unmaximize configure must clear the maximized state: {restored:?}"
    );
    assert_eq!((restored.width, restored.height), (WINDOW_W, WINDOW_H));

    // --- fullscreen and back --------------------------------------------
    state.toplevel_configures.clear();
    toplevel.set_fullscreen(None);
    queue.roundtrip(&mut state).expect("fullscreen configure");
    let fullscreen = state
        .toplevel_configures
        .last()
        .expect("fullscreen must produce a configure")
        .clone();
    assert!(fullscreen.has(xdg_toplevel::State::Fullscreen));
    assert_eq!((fullscreen.width, fullscreen.height), (OUTPUT_W, OUTPUT_H));

    state.toplevel_configures.clear();
    toplevel.unset_fullscreen();
    queue.roundtrip(&mut state).expect("unfullscreen configure");
    let restored = state
        .toplevel_configures
        .last()
        .expect("unfullscreen must produce a configure")
        .clone();
    assert!(!restored.has(xdg_toplevel::State::Fullscreen));
    assert_eq!((restored.width, restored.height), (WINDOW_W, WINDOW_H));

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
}

#[test]
fn popup_configure_is_constrained_to_the_output() {
    let proc = CompositorProcess::start("dragonfruit-conformance-popup");
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let (_parent_surface, parent_xdg_surface, _parent_toplevel, _file) =
        map_toplevel(&mut state, &mut queue);
    let qh = queue.handle();
    let compositor = state.compositor.clone().unwrap();
    let wm_base = state.xdg_wm_base.clone().unwrap();

    // A deliberately oversized popup anchored near the bottom-right, with
    // every constraint adjustment enabled: the compositor must flip/slide/
    // resize it so the configure stays inside the output (FR-11).
    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let positioner = wm_base.create_positioner(&qh, ());
    positioner.set_size(2000, 2000);
    positioner.set_anchor_rect(0, 0, WINDOW_W, WINDOW_H);
    positioner.set_anchor(xdg_positioner::Anchor::BottomRight);
    positioner.set_gravity(xdg_positioner::Gravity::BottomRight);
    positioner.set_constraint_adjustment(xdg_positioner::ConstraintAdjustment::all());
    let _popup = xdg_surface.get_popup(Some(&parent_xdg_surface), &positioner, &qh, ());
    positioner.destroy();
    surface.commit();

    queue.roundtrip(&mut state).expect("popup configure");
    let popup = state
        .popup_configures
        .last()
        .expect("popup must produce a configure");
    assert!(
        popup.width >= 1 && popup.height >= 1,
        "popup configure must not be empty: {popup:?}"
    );
    assert!(
        popup.width <= OUTPUT_W && popup.height <= OUTPUT_H,
        "popup configure must fit the output: {popup:?}"
    );

    surface.destroy();
    xdg_surface.destroy();
    proc.shutdown();
}

/// T-04 test-plan requirement: a client sending contradictory or
/// nonsensical state requests must never crash the compositor. Exercise
/// no-op transitions on a floating window, contradictory size hints,
/// re-entrant fullscreen/maximize, and destroying a fullscreen toplevel,
/// then prove the session is still alive by mapping a fresh window.
#[test]
fn malformed_client_requests_never_crash() {
    let proc = CompositorProcess::start("dragonfruit-conformance-malformed-window");
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, xdg_surface, toplevel, _file) = map_toplevel(&mut state, &mut queue);

    // No-op transitions from the floating state.
    toplevel.unset_maximized();
    toplevel.unset_fullscreen();
    queue.roundtrip(&mut state).expect("no-op transitions");

    // Contradictory size hints (min > max), then a maximize.
    toplevel.set_min_size(500, 500);
    toplevel.set_max_size(100, 100);
    toplevel.set_maximized();
    queue
        .roundtrip(&mut state)
        .expect("contradictory size hints");
    assert!(
        !state.toplevel_configures.is_empty(),
        "a maximize after contradictory hints must still configure"
    );

    // Re-entrant / interleaved state requests.
    toplevel.set_fullscreen(None);
    toplevel.set_maximized();
    toplevel.unset_fullscreen();
    toplevel.unset_maximized();
    queue
        .roundtrip(&mut state)
        .expect("interleaved state requests");

    // Destroy a toplevel while it is fullscreen: the compositor owns the
    // dedicated Space and must clean it up without panicking.
    toplevel.set_fullscreen(None);
    queue
        .roundtrip(&mut state)
        .expect("fullscreen before destroy");
    toplevel.destroy();
    surface.destroy();
    xdg_surface.destroy();
    queue
        .roundtrip(&mut state)
        .expect("destroy fullscreen toplevel");

    // Still alive: a fresh window maps and configures.
    state.toplevel_configures.clear();
    let (surface2, xdg_surface2, toplevel2, _file2) = map_toplevel(&mut state, &mut queue);
    assert!(
        !state.toplevel_configures.is_empty(),
        "the compositor must still configure a fresh window"
    );

    toplevel2.destroy();
    surface2.destroy();
    xdg_surface2.destroy();
    proc.shutdown();
}

/// T-04 test-plan requirement: interactive move/resize over the protocol.
/// The synthetic-input harness (T-03) supplies the seat button that starts
/// the pointer grab, so this exercises `xdg_toplevel.move`/`resize` end to
/// end instead of only the pure grab math.
///
/// Resize is asserted through the client's `xdg_toplevel.configure` (the
/// new size). Move has no client-visible geometry event in xdg-shell, so
/// the assertion is that the grab runs, the window survives, and the
/// session still serves a subsequent resize.
#[test]
fn move_and_resize_requests_are_served_over_protocol() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-window-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-move-resize",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, _xdg_surface, toplevel, _file) = map_toplevel(&mut state, &mut queue);

    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some(),
    );
    let seat = state.seat.clone().expect("wl_seat bound");
    let _pointer = state.pointer.clone().expect("wl_pointer bound");

    // Click the centered window: sets pointer focus and arms grab data.
    // BTN_LEFT == 0x110 == 272.
    input.send("motion-abs 0.5 0.5");
    input.send("button 272 down");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.pointer_button_serials.is_empty(),
    );
    let serial = *state.pointer_button_serials.last().unwrap();

    // --- interactive move -------------------------------------------------
    toplevel._move(&seat, serial);
    // Order the request before the synthetic motion on the same loop.
    queue.roundtrip(&mut state).expect("move request");
    input.send("motion 40 30");
    queue.roundtrip(&mut state).expect("move motion");
    input.send("button 272 up");
    queue.roundtrip(&mut state).expect("move release");
    assert!(
        !state.toplevel_configures.is_empty(),
        "the window must survive an interactive move"
    );

    // --- interactive resize from the bottom-right -------------------------
    state.pointer_button_serials.clear();
    state.toplevel_configures.clear();
    input.send("button 272 down");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.pointer_button_serials.is_empty(),
    );
    let serial = *state.pointer_button_serials.last().unwrap();
    toplevel.resize(&seat, serial, xdg_toplevel::ResizeEdge::BottomRight);
    queue.roundtrip(&mut state).expect("resize request");
    input.send("motion 60 40");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_configures
                .iter()
                .any(|c| c.width >= WINDOW_W + 50 && c.height >= WINDOW_H + 30)
        },
    );
    input.send("button 272 up");
    let _ = queue.roundtrip(&mut state);

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}
