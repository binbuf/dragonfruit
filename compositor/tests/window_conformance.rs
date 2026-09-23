// SPDX-License-Identifier: MIT
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
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
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

/// The design-system SSD titlebar height (`component.titlebar.height` in
/// `design-system/tokens/tokens.json`). The compositor reserves this strip
/// above a Tier-2 client's content (T-01.1).
const TITLEBAR_HEIGHT: i32 = 40;

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
    decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    /// Serial of each pointer button press delivered to the client.
    pointer_button_serials: Vec<u32>,
    toplevel_configures: Vec<ToplevelConfigure>,
    popup_configures: Vec<PopupConfigure>,
    /// Number of `xdg_toplevel.close` events delivered (T-01.2 close control).
    toplevel_close_count: usize,
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
                "zxdg_decoration_manager_v1" => {
                    state.decoration_manager = Some(registry.bind(name, version.min(1), qh, ()));
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
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                state.toplevel_configures.push(ToplevelConfigure {
                    width,
                    height,
                    states: parse_states(&states),
                });
            }
            xdg_toplevel::Event::Close => {
                state.toplevel_close_count += 1;
            }
            _ => {}
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

impl Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        _: zxdg_toplevel_decoration_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
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

/// Sends synthetic-input commands to the T-03 harness socket and reads the
/// `query` replies (T-01.1 decoration introspection). The reply socket is
/// bound so the compositor can address its answer back to us.
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

/// The window state as reported by `query decorations` (T-01.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowStateReport {
    Floating,
    Zoomed,
    Minimized,
    Fullscreen,
    Unknown,
}

/// One parsed `query decorations` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecorationReport {
    window: u64,
    server_side: bool,
    titlebar: (i32, i32, i32, i32),
    content: (i32, i32, i32, i32),
    state: WindowStateReport,
    /// Assigned Space id, or -1 when unassigned (T-01.4).
    space: i64,
}

/// Parse the `decoration` lines of a report (ignoring the trailing `end`).
fn parse_decorations(report: &str) -> Vec<DecorationReport> {
    report
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next()? != "decoration" {
                return None;
            }
            let n = |parts: &mut std::str::SplitWhitespace| -> Option<i32> {
                parts.next()?.parse().ok()
            };
            let window = parts.next()?.parse().ok()?;
            let server_side = parts.next()? == "1";
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
            let state = match parts.next() {
                Some("floating") => WindowStateReport::Floating,
                Some("zoomed") => WindowStateReport::Zoomed,
                Some("minimized") => WindowStateReport::Minimized,
                Some("fullscreen") => WindowStateReport::Fullscreen,
                _ => WindowStateReport::Unknown,
            };
            let space = parts.next().and_then(|t| t.parse().ok()).unwrap_or(-1);
            Some(DecorationReport {
                window,
                server_side,
                titlebar,
                content,
                state,
                space,
            })
        })
        .collect()
}

/// Traffic-light geometry from `component.trafficLights` tokens, used to aim
/// the synthetic pointer at each control (T-01.2).
const LIGHT_DIAMETER: i32 = 12;
const LIGHT_GAP: i32 = 8;
const LIGHT_INSET: i32 = 12;

/// The global-space center of a traffic light on a reported titlebar.
fn light_center(report: &DecorationReport, index: i32) -> (i32, i32) {
    let (tx, ty, _tw, th) = report.titlebar;
    let cx = tx + LIGHT_INSET + LIGHT_DIAMETER / 2 + index * (LIGHT_DIAMETER + LIGHT_GAP);
    let cy = ty + th / 2;
    (cx, cy)
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
    // The client area fills the usable output *below* its SSD titlebar
    // (T-01.1): zoom configures the content, and the titlebar occupies the
    // top strip.
    assert_eq!(
        (zoomed.width, zoomed.height),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT)
    );

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

/// T-01.1 acceptance: a mapped Wayland toplevel carries an SSD titlebar with
/// the token height and insets, a CSD client carries none, and the client
/// area sits directly below the titlebar. Asserted over the T-03
/// synthetic-input observation socket.
#[test]
fn ssd_toplevel_carries_a_titlebar_and_csd_does_not() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-decoration-synth-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-decoration",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // No `xdg-decoration` request: the compositor default is SSD.
    let (surface, xdg_surface, toplevel, file) = map_toplevel(&mut state, &mut queue);
    let report = wait_for_report(&mut state, &mut queue, &input, |reports| {
        reports.iter().any(|report| report.server_side)
    });
    let ssd = *report
        .iter()
        .find(|report| report.server_side)
        .expect("a mapped SSD toplevel must carry a titlebar");

    // Expected insets: the titlebar is the token height, exactly above the
    // client area and sharing its horizontal extent.
    assert_eq!(
        ssd.titlebar.3, TITLEBAR_HEIGHT,
        "titlebar height must come from the design tokens: {ssd:?}"
    );
    assert_eq!(ssd.titlebar.0, ssd.content.0);
    assert_eq!(ssd.titlebar.2, ssd.content.2);
    assert_eq!(
        ssd.titlebar.1 + ssd.titlebar.3,
        ssd.content.1,
        "the client area must be configured directly below the titlebar"
    );
    assert_eq!((ssd.content.2, ssd.content.3), (WINDOW_W, WINDOW_H));

    // A CSD client asks for client-side decoration before its first commit.
    let qh = queue.handle();
    let compositor = state.compositor.clone().unwrap();
    let wm_base = state.xdg_wm_base.clone().unwrap();
    let manager = state
        .decoration_manager
        .clone()
        .expect("zxdg_decoration_manager_v1 not advertised");
    let csd_surface = compositor.create_surface(&qh, ());
    let csd_xdg_surface = wm_base.get_xdg_surface(&csd_surface, &qh, ());
    let csd_toplevel = csd_xdg_surface.get_toplevel(&qh, ());
    csd_toplevel.set_title("CSD".into());
    let decoration = manager.get_toplevel_decoration(&csd_toplevel, &qh, ());
    decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ClientSide);
    csd_surface.commit();
    queue.roundtrip(&mut state).expect("csd initial configure");
    let (csd_buffer, csd_file) = shm_buffer(&state, &qh, WINDOW_W, WINDOW_H);
    csd_surface.attach(Some(&csd_buffer), 0, 0);
    csd_surface.commit();
    queue.roundtrip(&mut state).expect("csd map commit");

    let report = wait_for_report(&mut state, &mut queue, &input, |reports| {
        reports.len() >= 2 && reports.iter().any(|report| !report.server_side)
    });
    assert_eq!(report.len(), 2, "two tracked windows: {report:?}");
    let csd = *report
        .iter()
        .find(|report| !report.server_side)
        .expect("the CSD window must be tracked");
    assert_eq!(
        csd.titlebar,
        (0, 0, 0, 0),
        "a CSD client must not be given a compositor titlebar"
    );
    assert_eq!(
        (csd.content.2, csd.content.3),
        (WINDOW_W, WINDOW_H),
        "the CSD client keeps its own full content size"
    );
    assert_eq!(
        report.iter().filter(|report| report.server_side).count(),
        1,
        "only the SSD window carries a titlebar: {report:?}"
    );

    decoration.destroy();
    csd_toplevel.destroy();
    csd_xdg_surface.destroy();
    csd_surface.destroy();
    toplevel.destroy();
    xdg_surface.destroy();
    surface.destroy();
    drop(csd_file);
    drop(file);
    drop(conn);
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// Query `query decorations`, retrying while the scene catches up.
fn wait_for_report(
    state: &mut TestClient,
    queue: &mut EventQueue<TestClient>,
    input: &SyntheticInput,
    mut predicate: impl FnMut(&[DecorationReport]) -> bool,
) -> Vec<DecorationReport> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        queue.roundtrip(state).expect("roundtrip while waiting");
        let reports = parse_decorations(&input.query("query decorations"));
        if predicate(&reports) {
            return reports;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for decoration report"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Query `query decorations` until the window with `window` id satisfies
/// `predicate`, retrying while the scene catches up.
fn wait_for_window(
    state: &mut TestClient,
    queue: &mut EventQueue<TestClient>,
    input: &SyntheticInput,
    window: u64,
    mut predicate: impl FnMut(&DecorationReport) -> bool,
) -> DecorationReport {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        queue.roundtrip(state).expect("roundtrip while waiting");
        let reports = parse_decorations(&input.query("query decorations"));
        if let Some(report) = reports.iter().find(|report| report.window == window) {
            if predicate(report) {
                return *report;
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for window {window} state"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Aim the synthetic pointer at the traffic light at `index`
/// (0=close, 1=minimize, 2=zoom) and click it.
fn click_light(input: &SyntheticInput, report: &DecorationReport, index: i32) {
    let (cx, cy) = light_center(report, index);
    let nx = f64::from(cx) / f64::from(OUTPUT_W);
    let ny = f64::from(cy) / f64::from(OUTPUT_H);
    input.send(&format!("motion-abs {nx} {ny}"));
    input.send("button 272 down");
    input.send("button 272 up");
}

/// T-01.2 acceptance: synthetic pointer clicks on the traffic lights drive
/// zoom/unzoom, minimize, and close through the existing window state
/// machine, observed over the protocol.
#[test]
fn traffic_lights_drive_zoom_minimize_and_close() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-traffic-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-traffic",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, xdg_surface, toplevel, file) = map_toplevel(&mut state, &mut queue);
    let reports = wait_for_report(&mut state, &mut queue, &input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && report.state == WindowStateReport::Floating)
    });
    let window = reports
        .iter()
        .find(|report| report.server_side)
        .expect("a mapped SSD toplevel")
        .window;
    let mut report = *reports
        .iter()
        .find(|report| report.window == window)
        .unwrap();

    // --- green (zoom): fills the usable area below the titlebar ----------
    click_light(&input, &report, 2);
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed
    });
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT),
        "zoom must fill the usable area: {report:?}"
    );

    // --- green again (unzoom): restores the floating size ----------------
    click_light(&input, &report, 2);
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating
    });
    assert_eq!((report.content.2, report.content.3), (WINDOW_W, WINDOW_H));

    // --- yellow (minimize): hidden, no titlebar --------------------------
    click_light(&input, &report, 1);
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Minimized
    });
    assert!(
        !report.server_side && report.titlebar == (0, 0, 0, 0),
        "a minimized window draws no titlebar: {report:?}"
    );

    // --- red (close) on a fresh window -----------------------------------
    let (surface2, xdg_surface2, toplevel2, file2) = map_toplevel(&mut state, &mut queue);
    let reports = wait_for_report(&mut state, &mut queue, &input, |reports| {
        reports.iter().filter(|report| report.server_side).count() == 1
    });
    let close_report = *reports
        .iter()
        .find(|report| report.server_side)
        .expect("the fresh window must carry a titlebar");
    click_light(&input, &close_report, 0);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_close_count > 0,
    );

    surface2.destroy();
    toplevel2.destroy();
    xdg_surface2.destroy();
    toplevel.destroy();
    xdg_surface.destroy();
    surface.destroy();
    drop(file2);
    drop(file);
    drop(conn);
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// Move the synthetic pointer to a global-space point (normalized like
/// libinput's absolute coordinates).
fn motion_to(input: &SyntheticInput, x: i32, y: i32) {
    let nx = f64::from(x) / f64::from(OUTPUT_W);
    let ny = f64::from(y) / f64::from(OUTPUT_H);
    input.send(&format!("motion-abs {nx} {ny}"));
}

/// A left-button down/up at the current pointer location (BTN_LEFT = 272).
fn click(input: &SyntheticInput) {
    input.send("button 272 down");
    input.send("button 272 up");
}

/// The center of a reported titlebar, away from the traffic-light cluster.
fn titlebar_center(report: &DecorationReport) -> (i32, i32) {
    let (tx, ty, tw, th) = report.titlebar;
    (tx + tw / 2, ty + th / 2)
}

/// One parsed `query window-menu` line (T-01.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MenuReport {
    open: bool,
    window: u64,
    rect: (i32, i32, i32, i32),
    highlighted: i64,
    submenu_open: bool,
    submenu_highlighted: i64,
    submenu_rect: (i32, i32, i32, i32),
}

fn parse_menu(report: &str) -> MenuReport {
    let line = report
        .lines()
        .find(|line| line.starts_with("window-menu"))
        .expect("a window-menu line");
    let mut parts = line.split_whitespace();
    assert_eq!(parts.next(), Some("window-menu"));
    let mut n = || -> i64 {
        parts
            .next()
            .and_then(|token| token.parse().ok())
            .expect("window-menu numeric field")
    };
    let open = n() != 0;
    let window = n() as u64;
    let rect = (n() as i32, n() as i32, n() as i32, n() as i32);
    let highlighted = n();
    let submenu_open = n() != 0;
    let submenu_highlighted = n();
    let submenu_rect = (n() as i32, n() as i32, n() as i32, n() as i32);
    MenuReport {
        open,
        window,
        rect,
        highlighted,
        submenu_open,
        submenu_highlighted,
        submenu_rect,
    }
}

/// `component.contextMenu` geometry used to aim at rows (T-01.4).
const MENU_PADDING: i32 = 6;
const MENU_ROW_HEIGHT: i32 = 26;
const MENU_MIN_WIDTH: i32 = 180;

fn panel_row_center(rect: (i32, i32, i32, i32), index: i32) -> (i32, i32) {
    let (x, y, _, _) = rect;
    (
        x + MENU_PADDING + (MENU_MIN_WIDTH - 2 * MENU_PADDING) / 2,
        y + MENU_PADDING + index * MENU_ROW_HEIGHT + MENU_ROW_HEIGHT / 2,
    )
}

fn menu_row_center(menu: &MenuReport, index: i32) -> (i32, i32) {
    panel_row_center(menu.rect, index)
}

fn submenu_row_center(menu: &MenuReport, index: i32) -> (i32, i32) {
    panel_row_center(menu.submenu_rect, index)
}

/// Right-click the window's titlebar center to open the window menu.
fn open_window_menu(input: &SyntheticInput, report: &DecorationReport) {
    let (cx, cy) = titlebar_center(report);
    motion_to(input, cx, cy);
    input.send("button 273 down"); // BTN_RIGHT
    input.send("button 273 up");
}

/// Left-click at a global-space point.
fn click_at(input: &SyntheticInput, x: i32, y: i32) {
    motion_to(input, x, y);
    input.send("button 272 down");
    input.send("button 272 up");
}

/// Poll `query window-menu` until `predicate` holds.
fn wait_for_menu(
    input: &SyntheticInput,
    mut predicate: impl FnMut(&MenuReport) -> bool,
) -> MenuReport {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let menu = parse_menu(&input.query("query window-menu"));
        if predicate(&menu) {
            return menu;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for window-menu state; last={menu:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Linux evdev key codes (the synthetic harness adds the xkb +8 offset).
const KEY_ESC: u32 = 1;
const KEY_ENTER: u32 = 28;
const KEY_DOWN: u32 = 108;

fn key_tap(input: &SyntheticInput, code: u32) {
    input.send(&format!("key {code} down"));
    input.send(&format!("key {code} up"));
}

/// T-01.4 acceptance: a right-click on the SSD titlebar opens the window menu;
/// Escape, a click-away, and focus loss dismiss it; the keyboard navigates
/// and activates it. The keyboard path zooms the window to prove the command
/// is routed through the existing state machine.
#[test]
fn window_menu_opens_dismisses_and_is_keyboard_operable() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-menu-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-menu",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, xdg_surface, toplevel, file) = map_toplevel(&mut state, &mut queue);
    let reports = wait_for_report(&mut state, &mut queue, &input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && report.state == WindowStateReport::Floating)
    });
    let report = *reports
        .iter()
        .find(|report| report.server_side)
        .expect("a mapped SSD toplevel");
    let window = report.window;

    // --- right-click opens the menu --------------------------------------
    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    assert_eq!(menu.window, window, "the menu acts on the clicked window");
    assert_eq!(menu.highlighted, 0, "the first row is highlighted on open");
    assert!(!menu.submenu_open);

    // --- Escape dismisses -------------------------------------------------
    key_tap(&input, KEY_ESC);
    let menu = wait_for_menu(&input, |menu| !menu.open);
    assert!(!menu.open, "Escape must dismiss the menu");

    // --- click-away dismisses --------------------------------------------
    open_window_menu(&input, &report);
    wait_for_menu(&input, |menu| menu.open);
    click_at(&input, 5, 5);
    wait_for_menu(&input, |menu| !menu.open);

    // --- keyboard: Down, Down, Enter activates Zoom -----------------------
    open_window_menu(&input, &report);
    wait_for_menu(&input, |menu| menu.open);
    key_tap(&input, KEY_DOWN);
    key_tap(&input, KEY_DOWN);
    let menu = wait_for_menu(&input, |menu| menu.open && menu.highlighted == 2);
    assert_eq!(menu.highlighted, 2, "Down must move the highlight to Zoom");
    key_tap(&input, KEY_ENTER);
    let zoomed = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed
    });
    assert_eq!(
        (zoomed.content.2, zoomed.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT),
        "Enter on Zoom must zoom the window: {zoomed:?}"
    );
    wait_for_menu(&input, |menu| !menu.open);

    surface.destroy();
    toplevel.destroy();
    xdg_surface.destroy();
    drop(file);
    drop(conn);
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// Wait for a newly mapped SSD window: one whose id is not in `seen`.
fn wait_for_new_window(
    state: &mut TestClient,
    queue: &mut EventQueue<TestClient>,
    input: &SyntheticInput,
    seen: &[u64],
) -> DecorationReport {
    let reports = wait_for_report(state, queue, input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && !seen.contains(&report.window))
    });
    *reports
        .iter()
        .find(|report| report.server_side && !seen.contains(&report.window))
        .expect("a newly mapped SSD window")
}

/// T-01.4 acceptance: pointer clicks on the window-menu rows run Zoom,
/// Minimize, Close, and Move to Space for a Wayland window, all through the
/// existing window state machine.
#[test]
fn window_menu_runs_zoom_minimize_close_and_move_to_space() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-menu-commands-synth-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-menu-commands",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);
    // Every window id the test has already acted on, so a phase always aims
    // at the window it just mapped (not a stale one from a previous phase).
    let mut seen: Vec<u64> = Vec::new();

    // --- Zoom then Unzoom through the menu -------------------------------
    let (surface, xdg_surface, toplevel, file) = map_toplevel(&mut state, &mut queue);
    let mut report = wait_for_new_window(&mut state, &mut queue, &input, &seen);
    seen.push(report.window);
    let window = report.window;

    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 2); // Zoom
    click_at(&input, cx, cy);
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed
    });
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT)
    );

    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 2); // Zoom toggles back
    click_at(&input, cx, cy);
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating
    });
    assert_eq!((report.content.2, report.content.3), (WINDOW_W, WINDOW_H));

    // --- Minimize through the menu ---------------------------------------
    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 1); // Minimize
    click_at(&input, cx, cy);
    let minimized = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Minimized
    });
    assert!(!minimized.server_side);

    surface.destroy();
    toplevel.destroy();
    xdg_surface.destroy();
    drop(file);

    // --- Close through the menu ------------------------------------------
    let (surface2, xdg_surface2, toplevel2, file2) = map_toplevel(&mut state, &mut queue);
    let close_report = wait_for_new_window(&mut state, &mut queue, &input, &seen);
    seen.push(close_report.window);
    open_window_menu(&input, &close_report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 3); // Close
    click_at(&input, cx, cy);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_close_count > 0,
    );
    surface2.destroy();
    toplevel2.destroy();
    xdg_surface2.destroy();
    drop(file2);

    // --- Move to Space through the submenu -------------------------------
    let (surface3, xdg_surface3, toplevel3, file3) = map_toplevel(&mut state, &mut queue);
    let move_report = wait_for_new_window(&mut state, &mut queue, &input, &seen);
    seen.push(move_report.window);
    let before_space = move_report.space;
    assert!(before_space >= 0, "a mapped window is assigned a Space");

    open_window_menu(&input, &move_report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 0); // Move to Space
    click_at(&input, cx, cy);
    let menu = wait_for_menu(&input, |menu| menu.submenu_open);
    let (cx, cy) = submenu_row_center(&menu, 1); // Space 2
    click_at(&input, cx, cy);
    wait_for_window(
        &mut state,
        &mut queue,
        &input,
        move_report.window,
        |report| report.space != before_space,
    );
    wait_for_menu(&input, |menu| !menu.open);

    surface3.destroy();
    toplevel3.destroy();
    xdg_surface3.destroy();
    drop(file3);
    drop(conn);
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// T-01.3 acceptance: a drag from the SSD titlebar starts the existing
/// interactive move grab and moves the window; a double-click dispatches the
/// configured `dock.titlebarDoubleClick` action (default zoom, minimize when
/// set). Movement is observed through the window's content geometry.
#[test]
fn titlebar_drag_and_double_click_move_and_zoom() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-drag-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-drag",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, xdg_surface, toplevel, file) = map_toplevel(&mut state, &mut queue);
    let reports = wait_for_report(&mut state, &mut queue, &input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && report.state == WindowStateReport::Floating)
    });
    let window = reports
        .iter()
        .find(|report| report.server_side)
        .expect("a mapped SSD toplevel")
        .window;
    let before = *reports
        .iter()
        .find(|report| report.window == window)
        .unwrap();

    // --- drag from the titlebar moves the window -------------------------
    let (cx, cy) = titlebar_center(&before);
    motion_to(&input, cx, cy);
    input.send("button 272 down");
    input.send("motion 60 40");
    input.send("button 272 up");
    let after = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.content.0 - before.content.0 == 60 && report.content.1 - before.content.1 == 40
    });
    assert_eq!(
        (
            after.content.0 - before.content.0,
            after.content.1 - before.content.1
        ),
        (60, 40),
        "the titlebar drag must move the window by the pointer delta"
    );

    // --- double-click (default zoom) zooms the window --------------------
    // Wait out the drag press so it cannot pair with the next click.
    std::thread::sleep(Duration::from_millis(500));
    let (cx, cy) = titlebar_center(&after);
    motion_to(&input, cx, cy);
    click(&input);
    click(&input);
    let zoomed = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed
    });
    assert_eq!(
        (zoomed.content.2, zoomed.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT),
        "double-click must zoom the window: {zoomed:?}"
    );

    // --- configured `minimize` double-click minimizes --------------------
    input.send("set titlebar-double-click minimize");
    std::thread::sleep(Duration::from_millis(500));
    let (cx, cy) = titlebar_center(&zoomed);
    motion_to(&input, cx, cy);
    click(&input);
    click(&input);
    let minimized = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Minimized
    });
    assert!(
        !minimized.server_side && minimized.titlebar == (0, 0, 0, 0),
        "a configured minimize double-click hides the titlebar: {minimized:?}"
    );

    surface.destroy();
    toplevel.destroy();
    xdg_surface.destroy();
    drop(file);
    drop(conn);
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// T-01.3 acceptance: a fullscreen window hides its titlebar until the
/// pointer hovers its top reveal strip, then the titlebar overlays the top of
/// the content and reserves no inset.
#[test]
fn fullscreen_hover_reveals_the_titlebar() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-fullscreen-synth-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-fullscreen",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, xdg_surface, toplevel, file) = map_toplevel(&mut state, &mut queue);
    let reports = wait_for_report(&mut state, &mut queue, &input, |reports| {
        reports.iter().any(|report| report.server_side)
    });
    let window = reports
        .iter()
        .find(|report| report.server_side)
        .expect("a mapped SSD toplevel")
        .window;

    toplevel.set_fullscreen(None);
    let hidden = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Fullscreen
    });
    assert!(
        !hidden.server_side && hidden.titlebar == (0, 0, 0, 0),
        "fullscreen hides the titlebar until hovered: {hidden:?}"
    );
    assert_eq!(
        (hidden.content.2, hidden.content.3),
        (OUTPUT_W, OUTPUT_H),
        "fullscreen fills the output"
    );

    // Hover the top reveal strip: the titlebar appears, overlaid on the
    // content, reserving no inset.
    motion_to(&input, OUTPUT_W / 2, TITLEBAR_HEIGHT / 2);
    let revealed = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Fullscreen && report.server_side
    });
    assert_eq!(revealed.titlebar.3, TITLEBAR_HEIGHT);
    assert_eq!(
        revealed.titlebar.1, revealed.content.1,
        "overlay at content top"
    );
    assert_eq!(revealed.titlebar.2, revealed.content.2);

    // Moving away hides it again.
    motion_to(&input, OUTPUT_W / 2, OUTPUT_H / 2);
    let hidden_again = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Fullscreen && !report.server_side
    });
    assert_eq!(hidden_again.titlebar, (0, 0, 0, 0));

    surface.destroy();
    toplevel.destroy();
    xdg_surface.destroy();
    drop(file);
    drop(conn);
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
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
