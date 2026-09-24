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
use std::sync::mpsc::{self, Receiver};
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

/// The render-path counters the compositor dumps on SIGUSR1 (FR-2 idle trace,
/// T-02.4b). Mirrors `compositor/tests/idle_trace.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderStats {
    frames_rendered: u64,
    frames_skipped_no_damage: u64,
    direct_scanouts: u64,
    animation_frames_stepped: u64,
}

fn parse_stats(line: &str) -> Option<RenderStats> {
    let rest = line.split("frames_rendered=").nth(1)?;
    let mut parts = rest.split_whitespace();
    let frames_rendered = parts.next()?.parse().ok()?;
    let frames_skipped_no_damage = parts
        .next()?
        .strip_prefix("frames_skipped_no_damage=")?
        .parse()
        .ok()?;
    let direct_scanouts = parts
        .next()?
        .strip_prefix("direct_scanouts=")?
        .parse()
        .ok()?;
    let animation_frames_stepped = parts
        .next()?
        .strip_prefix("animation_frames_stepped=")?
        .parse()
        .ok()?;
    Some(RenderStats {
        frames_rendered,
        frames_skipped_no_damage,
        direct_scanouts,
        animation_frames_stepped,
    })
}

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    synthetic_path: Option<PathBuf>,
    stats_rx: Receiver<RenderStats>,
    reader: Option<std::thread::JoinHandle<()>>,
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
        // The reader thread ends when the child's stdout pipe closes.
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
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
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(path) = synthetic_path {
            command.env("DRAGONFRUIT_SYNTHETIC_INPUT", path);
        }
        let mut child = command.spawn().expect("failed to start compositor");
        let stdout = child.stdout.take().expect("stdout piped");

        // Stream the SIGUSR1 render-stats lines into a channel so a test can
        // assert the idle trace (T-02.4b).
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(stats) = parse_stats(&line) {
                    if tx.send(stats).is_err() {
                        break;
                    }
                }
            }
        });

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
            stats_rx: rx,
            reader: Some(reader),
        }
    }

    /// SIGUSR1 the compositor to dump the render counters.
    fn signal(&self, sig: i32) {
        unsafe {
            kill(self.child.id() as i32, sig);
        }
    }

    /// Wait for the next SIGUSR1 render-stats sample.
    fn sample(&self) -> RenderStats {
        self.stats_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("compositor did not emit render stats after SIGUSR1")
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
        // The stdout pipe closed with the child; drain the reader thread.
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

const SIGUSR1: i32 = 10;
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

    /// Send several newline-separated commands as one datagram and read the
    /// reply to the trailing query. Because the compositor applies every line
    /// in the datagram before the animation clock's timer can fire, a test can
    /// observe the exact state a command leaves *immediately* — e.g. the
    /// origin of a just-started zoom, before its first frame.
    #[track_caller]
    fn query_batch(&self, commands: &str) -> String {
        self.socket
            .send_to(commands.as_bytes(), &self.path)
            .unwrap_or_else(|err| panic!("failed to send synthetic {commands:?}: {err}"));
        let mut buf = [0u8; 16 * 1024];
        let len = self
            .socket
            .recv(&mut buf)
            .unwrap_or_else(|err| panic!("no reply to {commands:?}: {err}"));
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

/// One parsed `query motion` line (T-02.1b/T-02.2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct MotionReport {
    window: u64,
    kind: String,
    active: bool,
    completed: bool,
    frames: u64,
    origin: (i32, i32, i32, i32),
    target: (i32, i32, i32, i32),
}

/// Parse the `motion` lines of a report (ignoring the trailing `end`).
fn parse_motion(report: &str) -> Vec<MotionReport> {
    report
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next()? != "motion" {
                return None;
            }
            let n = |parts: &mut std::str::SplitWhitespace| -> Option<i32> {
                parts.next()?.parse().ok()
            };
            let window = parts.next()?.parse().ok()?;
            let kind = parts.next()?.to_string();
            let active = parts.next()? == "1";
            let completed = parts.next()? == "1";
            let frames = parts.next()?.parse().ok()?;
            let origin = (
                n(&mut parts)?,
                n(&mut parts)?,
                n(&mut parts)?,
                n(&mut parts)?,
            );
            let target = (
                n(&mut parts)?,
                n(&mut parts)?,
                n(&mut parts)?,
                n(&mut parts)?,
            );
            Some(MotionReport {
                window,
                kind,
                active,
                completed,
                frames,
                origin,
                target,
            })
        })
        .collect()
}

/// One parsed `query degrade` line (T-04.4a).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DegradeReport {
    tier: String,
    forced: bool,
    budget_us: u32,
    samples: u64,
    downgrades: u64,
    upgrades: u64,
    tier_frames: (u64, u64, u64),
}

/// Parse the `degrade` line of a `query degrade` reply (ignoring `end`).
fn parse_degrade(report: &str) -> DegradeReport {
    let line = report
        .lines()
        .find(|line| line.starts_with("degrade "))
        .unwrap_or_else(|| panic!("no degrade line in {report:?}"));
    let field = |name: &str| -> String {
        line.split_whitespace()
            .skip(1)
            .find_map(|token| token.strip_prefix(name)?.strip_prefix('='))
            .unwrap_or_else(|| panic!("missing {name} in {line:?}"))
            .to_string()
    };
    let tier_frames = field("tiers");
    let mut frames = (0u64, 0u64, 0u64);
    for part in tier_frames.split(',') {
        let (name, value) = part
            .split_once(':')
            .unwrap_or_else(|| panic!("bad tier counter in {tier_frames:?}"));
        let value: u64 = value
            .parse()
            .unwrap_or_else(|_| panic!("bad tier counter in {tier_frames:?}"));
        match name {
            "full" => frames.0 = value,
            "reduced" => frames.1 = value,
            "minimal" => frames.2 = value,
            other => panic!("unknown tier counter {other:?}"),
        }
    }
    DegradeReport {
        tier: field("tier"),
        forced: field("forced") == "1",
        budget_us: field("budget_us").parse().expect("budget_us"),
        samples: field("samples").parse().expect("samples"),
        downgrades: field("downgrades").parse().expect("downgrades"),
        upgrades: field("upgrades").parse().expect("upgrades"),
        tier_frames: frames,
    }
}

/// The `query material` reply (T-04.4b): the live scheme and resolved tones.
struct MaterialReport {
    scheme: String,
    chrome: String,
    elevated: String,
    border: String,
    accent: String,
}

/// Parse the `material` line of a `query material` reply (ignoring `end`).
fn parse_material(report: &str) -> MaterialReport {
    let line = report
        .lines()
        .find(|line| line.starts_with("material "))
        .unwrap_or_else(|| panic!("no material line in {report:?}"));
    let field = |name: &str| -> String {
        line.split_whitespace()
            .skip(1)
            .find_map(|token| token.strip_prefix(name)?.strip_prefix('='))
            .unwrap_or_else(|| panic!("missing {name} in {line:?}"))
            .to_string()
    };
    MaterialReport {
        scheme: field("scheme"),
        chrome: field("chrome"),
        elevated: field("elevated"),
        border: field("border"),
        accent: field("accent"),
    }
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
    // `CompositorProcess::start` returns as soon as the socket *node* exists,
    // but a stale node left by a killed run (or a window between the node
    // appearing and the listener accepting) can make a single connect race to
    // ECONNREFUSED (T-09 verify flake). Retry briefly so a startup race can
    // never fail the suite.
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

/// Build and map a toplevel advertising `app_id` before its first commit, so
/// the compositor can match a launch-origin hint to it (T-02.1b).
#[allow(clippy::type_complexity)]
fn map_toplevel_with_app_id(
    state: &mut TestClient,
    queue: &mut EventQueue<TestClient>,
    app_id: &str,
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
    toplevel.set_app_id(app_id.into());
    toplevel.set_title("Appear".into());
    // First commit with no buffer triggers the initial configure.
    surface.commit();
    queue.roundtrip(state).expect("initial configure");

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

/// Build and map a toplevel that requests `mode` via `xdg-decoration`
/// (T-01.5). Returns the decoration proxy so a test can flip the mode later.
#[allow(clippy::type_complexity)]
fn map_toplevel_with_decoration(
    state: &mut TestClient,
    queue: &mut EventQueue<TestClient>,
    mode: zxdg_toplevel_decoration_v1::Mode,
) -> (
    wl_surface::WlSurface,
    xdg_surface::XdgSurface,
    xdg_toplevel::XdgToplevel,
    zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
    std::fs::File,
) {
    let qh = queue.handle();
    let compositor = state.compositor.clone().unwrap();
    let wm_base = state.xdg_wm_base.clone().unwrap();
    let manager = state
        .decoration_manager
        .clone()
        .expect("zxdg_decoration_manager_v1 not advertised");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Decorated".into());
    let decoration = manager.get_toplevel_decoration(&toplevel, &qh, ());
    decoration.set_mode(mode);
    surface.commit();
    queue.roundtrip(state).expect("initial configure");
    let (buffer, file) = shm_buffer(state, &qh, WINDOW_W, WINDOW_H);
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();
    queue.roundtrip(state).expect("map commit");

    (surface, xdg_surface, toplevel, decoration, file)
}

/// T-01.5 acceptance: the decoration-tier matrix renders side by side — the
/// compositor default and an explicit server-side request each get exactly
/// one of our titlebars, and an explicit client-side request keeps its own
/// (no double decoration).
#[test]
fn decoration_tier_matrix_default_explicit_ssd_and_csd_side_by_side() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-tier-matrix-synth-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-tier-matrix",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // Default: no `xdg-decoration` request, compositor default SSD.
    let (d_surface, d_xdg, d_toplevel, d_file) = map_toplevel(&mut state, &mut queue);
    // Explicit server-side (Qt-style).
    let (s_surface, s_xdg, s_toplevel, s_decoration, s_file) = map_toplevel_with_decoration(
        &mut state,
        &mut queue,
        zxdg_toplevel_decoration_v1::Mode::ServerSide,
    );
    // Explicit client-side (a GTK headerbar): keeps its own decoration.
    let (c_surface, c_xdg, c_toplevel, c_decoration, c_file) = map_toplevel_with_decoration(
        &mut state,
        &mut queue,
        zxdg_toplevel_decoration_v1::Mode::ClientSide,
    );

    let reports = wait_for_report(&mut state, &mut queue, &input, |reports| reports.len() >= 3);
    assert_eq!(reports.len(), 3, "three tracked windows: {reports:?}");
    assert_eq!(
        reports.iter().filter(|report| report.server_side).count(),
        2,
        "the default and explicit-SSD windows carry one titlebar each: {reports:?}"
    );

    let csd = reports
        .iter()
        .find(|report| !report.server_side)
        .expect("the CSD window must be tracked");
    assert_eq!(
        csd.titlebar,
        (0, 0, 0, 0),
        "a CSD client must keep its own decoration, not get a compositor titlebar"
    );
    assert_eq!(
        (csd.content.2, csd.content.3),
        (WINDOW_W, WINDOW_H),
        "the CSD client keeps its full content size"
    );

    for ssd in reports.iter().filter(|report| report.server_side) {
        assert_eq!(ssd.titlebar.3, TITLEBAR_HEIGHT, "token titlebar height");
        assert_eq!(ssd.titlebar.1 + ssd.titlebar.3, ssd.content.1);
        assert_eq!(ssd.titlebar.2, ssd.content.2);
        assert_eq!((ssd.content.2, ssd.content.3), (WINDOW_W, WINDOW_H));
    }

    c_decoration.destroy();
    c_toplevel.destroy();
    c_xdg.destroy();
    c_surface.destroy();
    s_decoration.destroy();
    s_toplevel.destroy();
    s_xdg.destroy();
    s_surface.destroy();
    d_toplevel.destroy();
    d_xdg.destroy();
    d_surface.destroy();
    drop(c_file);
    drop(s_file);
    drop(d_file);
    drop(conn);
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// T-01.5 double-decoration guard: a runtime `xdg-decoration` flip on a
/// zoomed window reflows the client through `configure_window_size` — a CSD
/// request reclaims the titlebar strip, an SSD request gives it back.
#[test]
fn runtime_decoration_tier_change_reflows_a_zoomed_window() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-tier-reflow-synth-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-tier-reflow",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let (surface, xdg_surface, toplevel, decoration, file) = map_toplevel_with_decoration(
        &mut state,
        &mut queue,
        zxdg_toplevel_decoration_v1::Mode::ServerSide,
    );
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

    // Zoom: the client fits below the titlebar.
    click_light(&input, &report, 2);
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed
    });
    assert!(report.server_side);
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT)
    );

    // Flip to CSD: the titlebar goes away and the client reclaims the strip.
    decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ClientSide);
    queue.roundtrip(&mut state).expect("csd mode request");
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed && !report.server_side
    });
    assert_eq!(report.titlebar, (0, 0, 0, 0));
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H),
        "an undecorated zoomed window fills the whole output (no decoration gap)"
    );

    // Flip back to SSD: the titlebar returns and the client shrinks by it.
    decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide);
    queue.roundtrip(&mut state).expect("ssd mode request");
    report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed && report.server_side
    });
    assert_eq!(report.titlebar.3, TITLEBAR_HEIGHT);
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT),
        "a decorated zoomed window fits under its titlebar"
    );

    decoration.destroy();
    toplevel.destroy();
    xdg_surface.destroy();
    surface.destroy();
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

/// Query `query motion` until the report satisfies `predicate`, retrying
/// while the clock catches up.
fn wait_for_motion(
    input: &SyntheticInput,
    mut predicate: impl FnMut(&[MotionReport]) -> bool,
) -> Vec<MotionReport> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let motions = parse_motion(&input.query("query motion"));
        if predicate(&motions) {
            return motions;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for motion report; last={motions:?}"
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

/// T-02.1b acceptance: a launched window appears from the Dock tile the shell
/// handed off (`set launch_origin`) and commits its final geometry.
#[test]
fn window_appear_plays_from_the_dock_tile_origin_and_commits_the_target() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-appear-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-appear",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    // Hand the compositor the Dock tile before the app maps. The synchronous
    // query forces the preceding datagram to be processed before the map
    // commit, so the origin cannot race the appearance.
    let app_id = "org.dragonfruit.Appear";
    input.send(&format!("set launch-origin {app_id} 100 120 48 48"));
    let _ = input.query("query motion");

    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    // Let the appear run to completion.
    std::thread::sleep(Duration::from_millis(500));
    let appear = parse_motion(&input.query("query motion"));
    assert_eq!(
        appear.len(),
        1,
        "one window must carry a motion record: {appear:?}"
    );
    let a = &appear[0];
    assert_eq!(a.kind, "appear", "the mapping motion is an appear: {a:?}");
    assert_eq!(
        a.origin,
        (100, 120, 48, 48),
        "the appear must start from the shell-supplied Dock tile: {a:?}"
    );
    assert!(
        a.completed && !a.active,
        "the appear must have committed: {a:?}"
    );
    assert!(
        a.frames >= 5,
        "a full appear must step multiple frames, got {}: {a:?}",
        a.frames
    );

    // The committed geometry is the appearance's target, and it is the model
    // geometry the shell sees.
    let decorations = parse_decorations(&input.query("query decorations"));
    let d = decorations
        .iter()
        .find(|d| d.window == a.window)
        .expect("window tracked");
    assert_eq!(
        d.content, a.target,
        "the appear target must be the committed geometry: {d:?} vs {a:?}"
    );
    assert_eq!((d.content.2, d.content.3), (WINDOW_W, WINDOW_H));

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.1b acceptance: with reduced motion the appear collapses to a single
/// clock step through the same commit path, and the centered/origin geometry
/// still lands.
#[test]
fn reduced_motion_appear_takes_a_single_frame() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-appear-reduced-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-appear-reduced",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.AppearReduced";
    input.send("set reduced-motion on");
    input.send(&format!("set launch-origin {app_id} 50 60 48 48"));
    let _ = input.query("query motion");

    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    std::thread::sleep(Duration::from_millis(300));
    let appear = parse_motion(&input.query("query motion"));
    assert_eq!(
        appear.len(),
        1,
        "one window must carry a motion record: {appear:?}"
    );
    let a = &appear[0];
    assert_eq!(a.kind, "appear");
    assert_eq!(
        a.origin,
        (50, 60, 48, 48),
        "the reduced-motion appear keeps the supplied origin: {a:?}"
    );
    assert!(a.completed, "the reduced-motion appear must commit: {a:?}");
    assert_eq!(
        a.frames, 1,
        "reduced motion must collapse the appear to one frame: {a:?}"
    );

    let decorations = parse_decorations(&input.query("query decorations"));
    let d = decorations
        .iter()
        .find(|d| d.window == a.window)
        .expect("window tracked");
    assert_eq!(d.content, a.target);

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.2 acceptance: minimize shrinks a window into its Dock tile and
/// restore grows it back out, the states and outbox events update
/// immediately, and the sequence resolves with no orphaned ghost and nothing
/// left live on the clock.
#[test]
fn minimize_and_restore_play_through_the_dock_tile_and_resolve_cleanly() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-minimize-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-minimize",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // Hand the compositor the Dock tile before the app maps, then wait the
    // appear out so the restore target is a known floating geometry.
    let app_id = "org.dragonfruit.Minimize";
    input.send(&format!("set launch-origin {app_id} 100 120 48 48"));
    let _ = input.query("query motion");

    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;
    assert_eq!(appear.origin, (100, 120, 48, 48));

    // --- minimize: state/outbox update at once, geometry is preserved -----
    input.send(&format!("minimize {window}"));
    let minimized = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Minimized
    });
    assert_eq!(
        minimized.content, floating,
        "minimize must keep the restore geometry: {minimized:?}"
    );
    let events = input.query("query events");
    assert!(
        events.contains(&format!("event state-changed {window} minimized")),
        "the minimize state change must be in the outbox: {events:?}"
    );

    // --- the minimize ghost completes and no orphan remains --------------
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "minimize" && m.completed)
    });
    let minimize = motions
        .iter()
        .find(|m| m.kind == "minimize")
        .expect("a minimize record");
    assert_eq!(minimize.target, floating);
    assert_eq!(minimize.origin, (100, 120, 48, 48));
    assert!(
        minimize.frames >= 3,
        "a full minimize must step multiple frames, got {}: {minimize:?}",
        minimize.frames
    );
    let decorations = parse_decorations(&input.query("query decorations"));
    assert!(
        decorations
            .iter()
            .any(|d| d.window == window && d.state == WindowStateReport::Minimized),
        "no orphan: the minimized window must still be tracked: {decorations:?}"
    );

    // --- restore: grows back out of the tile to the original geometry -----
    input.send(&format!("restore {window}"));
    let restored = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating
    });
    assert_eq!(
        restored.content, floating,
        "restore must return to the original geometry: {restored:?}"
    );
    let events = input.query("query events");
    assert!(
        events.contains(&format!("event state-changed {window} floating")),
        "the restore state change must be in the outbox: {events:?}"
    );

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "restore" && m.completed)
    });
    let restore = motions
        .iter()
        .find(|m| m.kind == "restore")
        .expect("a restore record");
    assert_eq!(restore.target, floating);
    assert_eq!(restore.origin, (100, 120, 48, 48));
    assert!(
        motions.iter().all(|m| m.completed),
        "nothing may be left live on the clock: {motions:?}"
    );

    // The window is still tracked at its final geometry, so the sequence left
    // no orphan and no stale entry.
    let decorations = parse_decorations(&input.query("query decorations"));
    let final_report = decorations
        .iter()
        .find(|d| d.window == window)
        .expect("the restored window is tracked");
    assert_eq!(final_report.content, floating);
    assert_eq!(final_report.state, WindowStateReport::Floating);

    let _ = conn;
    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.2 reduced-motion acceptance: minimize and restore each take a single
/// clock step through the same commit path, and the state changes stay
/// legible (minimized, then floating at the original geometry).
#[test]
fn reduced_motion_minimize_and_restore_take_a_single_frame() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-minimize-reduced-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-minimize-reduced",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.MinimizeReduced";
    input.send("set reduced-motion on");
    input.send(&format!("set launch-origin {app_id} 30 40 48 48"));
    let _ = input.query("query motion");

    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions.iter().find(|m| m.kind == "appear").unwrap();
    let window = appear.window;
    let floating = appear.target;
    assert_eq!(appear.frames, 1, "reduced-motion appear is one frame");

    // Minimize: one step, state minimized.
    input.send(&format!("minimize {window}"));
    wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Minimized
    });
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "minimize" && m.completed)
    });
    let minimize = motions.iter().find(|m| m.kind == "minimize").unwrap();
    assert_eq!(minimize.frames, 1, "reduced minimize is one frame");
    assert_eq!(minimize.origin, (30, 40, 48, 48));

    // Restore: one step, floating at the original geometry.
    input.send(&format!("restore {window}"));
    let restored = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating
    });
    assert_eq!(restored.content, floating);
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "restore" && m.completed)
    });
    let restore = motions.iter().find(|m| m.kind == "restore").unwrap();
    assert_eq!(restore.frames, 1, "reduced restore is one frame");
    assert!(
        motions.iter().all(|m| m.completed),
        "nothing may be left live: {motions:?}"
    );

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.3 acceptance: zoom animates from the floating geometry to the usable
/// area (below the SSD titlebar), unzoom animates back, and each commits the
/// final geometry. The transition is render-only: the model geometry changes
/// immediately and the motion record carries the interpolated origin/target.
#[test]
fn zoom_and_unzoom_interpolate_between_geometries_and_commit() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-zoom-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-zoom",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.Zoom";
    input.send(&format!("set launch-origin {app_id} 100 120 48 48"));
    let _ = input.query("query motion");
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    // Settle the appear so the floating geometry is known and stable.
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;

    // --- zoom: the motion is recorded immediately, leaving from floating ---
    let report = input.query_batch(&format!("zoom {window}\nquery motion"));
    let motions = parse_motion(&report);
    let zoom = motions
        .iter()
        .find(|m| m.kind == "zoom")
        .expect("a zoom record after `zoom`");
    assert_eq!(zoom.window, window);
    assert_eq!(
        zoom.origin, floating,
        "zoom must leave from the floating geometry: {zoom:?}"
    );
    assert_eq!(
        zoom.target,
        (0, TITLEBAR_HEIGHT, OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT),
        "zoom must fill the usable area below the titlebar: {zoom:?}"
    );
    assert!(
        zoom.active && !zoom.completed,
        "the zoom must be in flight immediately: {zoom:?}"
    );
    let zoomed = zoom.target;

    // It steps several intermediate frames and commits the target.
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "zoom" && m.completed)
    });
    let zoom = motions
        .iter()
        .find(|m| m.kind == "zoom")
        .expect("the zoom record");
    assert!(
        zoom.frames >= 3,
        "a full zoom must step multiple frames, got {}: {zoom:?}",
        zoom.frames
    );
    let report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed
    });
    assert_eq!(
        report.content, zoomed,
        "the committed geometry must be the zoom target: {report:?}"
    );

    // --- unzoom: back to floating, leaving from the zoomed geometry --------
    let report = input.query_batch(&format!("unzoom {window}\nquery motion"));
    let motions = parse_motion(&report);
    let unzoom = motions
        .iter()
        .find(|m| m.kind == "zoom" && m.origin == zoomed)
        .expect("an unzoom record leaving from the zoomed geometry");
    assert_eq!(
        unzoom.target, floating,
        "unzoom must return to the floating geometry: {unzoom:?}"
    );
    assert!(unzoom.active && !unzoom.completed);

    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "zoom" && m.target == floating && m.completed)
    });
    let unzoom = motions
        .iter()
        .find(|m| m.kind == "zoom" && m.target == floating)
        .expect("the unzoom record");
    assert!(
        unzoom.frames >= 3,
        "a full unzoom must step multiple frames: {unzoom:?}"
    );
    let report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating
    });
    assert_eq!(
        report.content, floating,
        "unzoom must restore the original geometry: {report:?}"
    );

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.3 acceptance: fullscreen grows a window into the full output geometry
/// and unfullscreen shrinks it back, both on the shared clock with an
/// intermediate frame count and a committed final geometry.
#[test]
fn fullscreen_and_unfullscreen_interpolate_between_geometries_and_commit() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-fullscreen-motion-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-fullscreen-motion",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.Fullscreen";
    input.send(&format!("set launch-origin {app_id} 60 80 48 48"));
    let _ = input.query("query motion");
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;

    // --- fullscreen: grows from floating to the full output geometry -------
    let report = input.query_batch(&format!("fullscreen {window}\nquery motion"));
    let motions = parse_motion(&report);
    let fullscreen = motions
        .iter()
        .find(|m| m.kind == "fullscreen")
        .expect("a fullscreen record");
    assert_eq!(
        fullscreen.origin, floating,
        "fullscreen must leave from the floating geometry: {fullscreen:?}"
    );
    assert_eq!(
        fullscreen.target,
        (0, 0, OUTPUT_W, OUTPUT_H),
        "fullscreen must fill the whole output: {fullscreen:?}"
    );
    assert!(fullscreen.active && !fullscreen.completed);
    let full = fullscreen.target;

    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "fullscreen" && m.completed)
    });
    let fullscreen = motions
        .iter()
        .find(|m| m.kind == "fullscreen")
        .expect("the fullscreen record");
    assert!(
        fullscreen.frames >= 3,
        "a full fullscreen must step multiple frames: {fullscreen:?}"
    );
    let report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Fullscreen
    });
    assert_eq!(report.content, full);

    // --- unfullscreen: shrinks back to the floating geometry ---------------
    let report = input.query_batch(&format!("unfullscreen {window}\nquery motion"));
    let motions = parse_motion(&report);
    let unfullscreen = motions
        .iter()
        .find(|m| m.kind == "fullscreen" && m.origin == full)
        .expect("an unfullscreen record leaving from the full geometry");
    assert_eq!(unfullscreen.target, floating);

    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "fullscreen" && m.target == floating && m.completed)
    });
    let unfullscreen = motions
        .iter()
        .find(|m| m.kind == "fullscreen" && m.target == floating)
        .expect("the unfullscreen record");
    assert!(unfullscreen.frames >= 3);
    let report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating
    });
    assert_eq!(report.content, floating);

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.3 interruptibility: an unzoom requested while a zoom is mid-flight
/// starts from the window's *current* interpolated rect — it does not wait
/// for the zoom to finish — and still commits the floating geometry.
#[test]
fn zoom_retargets_mid_flight_without_waiting() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-zoom-interrupt-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-zoom-interrupt",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.ZoomInterrupt";
    input.send(&format!("set launch-origin {app_id} 100 120 48 48"));
    let _ = input.query("query motion");
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;

    // Start the zoom and catch it before it settles (at least one frame in).
    input.send(&format!("zoom {window}"));
    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "zoom" && !m.completed && m.frames >= 1)
    });
    let zoom = motions
        .iter()
        .find(|m| m.kind == "zoom")
        .expect("a live zoom record");
    let zoomed = zoom.target;
    assert!(
        !zoom.completed,
        "the zoom must still be in flight to test interruption: {zoom:?}"
    );

    // Retarget immediately.
    let report = input.query_batch(&format!("unzoom {window}\nquery motion"));
    let motions = parse_motion(&report);
    let unzoom = motions
        .iter()
        .find(|m| m.kind == "zoom" && m.target == floating)
        .expect("the retargeted unzoom record");
    assert!(
        unzoom.origin != zoomed,
        "the unzoom must not wait for the zoom to finish: {unzoom:?}"
    );
    assert!(
        unzoom.origin.2 > floating.2 && unzoom.origin.2 < zoomed.2,
        "the unzoom must start from the partial interpolated rect: {:?} (floating={floating:?}, zoomed={zoomed:?})",
        unzoom.origin
    );
    assert!(
        unzoom.origin.3 > floating.3 && unzoom.origin.3 < zoomed.3,
        "the retargeted height must also be partial: {:?}",
        unzoom.origin
    );

    // It resolves cleanly to the floating geometry.
    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "zoom" && m.target == floating && m.completed)
    });
    let unzoom = motions
        .iter()
        .find(|m| m.kind == "zoom" && m.target == floating)
        .expect("the retargeted unzoom record");
    assert!(unzoom.frames >= 1);
    let report = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating
    });
    assert_eq!(report.content, floating);

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.3 reduced motion: zoom/unzoom and fullscreen/unfullscreen each take a
/// single clock step through the same commit path, and the geometry still
/// lands exactly.
#[test]
fn reduced_motion_zoom_and_fullscreen_take_a_single_frame() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-zoom-reduced-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-zoom-reduced",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.ZoomReduced";
    input.send("set reduced-motion on");
    input.send(&format!("set launch-origin {app_id} 30 40 48 48"));
    let _ = input.query("query motion");
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;
    assert_eq!(appear.frames, 1, "reduced-motion appear is one frame");
    let zoomed = (0, TITLEBAR_HEIGHT, OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT);
    let full = (0, 0, OUTPUT_W, OUTPUT_H);

    // Zoom: one step, state zoomed at the usable geometry.
    input.send(&format!("zoom {window}"));
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "zoom" && m.completed)
    });
    let zoom = motions.iter().find(|m| m.kind == "zoom").unwrap();
    assert_eq!(zoom.frames, 1, "reduced-motion zoom is one frame: {zoom:?}");
    assert_eq!(zoom.target, zoomed);
    wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Zoomed && report.content == zoomed
    });

    // Unzoom: one step, floating again.
    input.send(&format!("unzoom {window}"));
    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "zoom" && m.target == floating && m.completed)
    });
    let unzoom = motions
        .iter()
        .find(|m| m.kind == "zoom" && m.target == floating)
        .unwrap();
    assert_eq!(
        unzoom.frames, 1,
        "reduced-motion unzoom is one frame: {unzoom:?}"
    );

    // Fullscreen: one step, the whole output.
    input.send(&format!("fullscreen {window}"));
    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "fullscreen" && m.completed)
    });
    let fullscreen = motions.iter().find(|m| m.kind == "fullscreen").unwrap();
    assert_eq!(
        fullscreen.frames, 1,
        "reduced-motion fullscreen is one frame: {fullscreen:?}"
    );
    assert_eq!(fullscreen.target, full);
    wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Fullscreen && report.content == full
    });

    // Unfullscreen: one step, floating again.
    input.send(&format!("unfullscreen {window}"));
    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "fullscreen" && m.target == floating && m.completed)
    });
    let unfullscreen = motions
        .iter()
        .find(|m| m.kind == "fullscreen" && m.target == floating)
        .unwrap();
    assert_eq!(
        unfullscreen.frames, 1,
        "reduced-motion unfullscreen is one frame: {unfullscreen:?}"
    );
    assert!(
        motions.iter().all(|m| m.completed),
        "nothing may be left live: {motions:?}"
    );

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-02.4a acceptance: a closing window fades/scales out as a ghost that takes
/// no input, and its removal from the model commits exactly once when the
/// motion settles. The window stays tracked (and out of `Space`) while the
/// ghost is live.
#[test]
fn close_fades_out_inert_and_commits_removal_exactly_once() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-close-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-close",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.Close";
    let tile = (100, 120, 48, 48);
    input.send(&format!(
        "set launch-origin {app_id} {} {} {} {}",
        tile.0, tile.1, tile.2, tile.3
    ));
    let _ = input.query("query motion");
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    // Settle the appear so the floating geometry is known and stable.
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;

    // Prove the open window takes input: click its content.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some(),
    );
    let (cx, cy) = (floating.0 + floating.2 / 2, floating.1 + floating.3 / 2);
    motion_to(&input, cx, cy);
    click(&input);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.pointer_button_serials.is_empty(),
    );
    state.pointer_button_serials.clear();

    // --- close: the ghost is recorded immediately, still tracked ----------
    // The duplicate `close` proves a re-entrant request is a no-op: the live
    // ghost owns the single removal.
    let report = input.query_batch(&format!("close {window}\nclose {window}\nquery motion"));
    let motions = parse_motion(&report);
    let closes: Vec<&MotionReport> = motions.iter().filter(|m| m.kind == "close").collect();
    assert_eq!(
        closes.len(),
        1,
        "a second close must not start a second ghost: {motions:?}"
    );
    let close = closes[0];
    assert_eq!(close.window, window);
    assert_eq!(
        close.origin, tile,
        "the close ghost shrinks into the app's Dock tile: {close:?}"
    );
    assert_eq!(
        close.target, floating,
        "the close ghost leaves from the window's geometry: {close:?}"
    );
    assert!(
        close.active && !close.completed,
        "the close must be in flight immediately: {close:?}"
    );
    assert!(
        parse_decorations(&input.query("query decorations"))
            .iter()
            .any(|d| d.window == window),
        "the window must stay in the model until the ghost settles"
    );

    // The compositor asked the client to close.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_close_count > 0,
    );

    // --- input-inert: the ghost is not hit by the pointer ------------------
    // It left `Space` at close time, so a motion back over its old rect finds
    // no surface and the following click reaches no client.
    state.pointer_button_serials.clear();
    motion_to(&input, cx, cy);
    click(&input);
    queue.roundtrip(&mut state).expect("roundtrip after click");
    std::thread::sleep(Duration::from_millis(50));
    queue.roundtrip(&mut state).expect("roundtrip after click");
    assert!(
        state.pointer_button_serials.is_empty(),
        "a closing ghost must not receive input: {:?}",
        state.pointer_button_serials
    );

    // --- removal commits exactly once --------------------------------------
    let decorations = wait_for_report(&mut state, &mut queue, &input, |reports| {
        !reports.iter().any(|report| report.window == window)
    });
    assert!(
        decorations.iter().all(|report| report.window != window),
        "the closed window must be gone from the model: {decorations:?}"
    );
    let events = input.query("query events");
    let closed = events
        .lines()
        .filter(|line| *line == format!("event closed {window}"))
        .count();
    assert_eq!(
        closed, 1,
        "removal must be broadcast exactly once: {events:?}"
    );
    let motions = parse_motion(&input.query("query motion"));
    assert!(
        motions.iter().all(|m| m.window != window),
        "no motion record may outlive the closed window: {motions:?}"
    );

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// Count the exact `event <kind> <window>` lines in a `query events` report.
fn count_events(report: &str, needle: &str) -> usize {
    report.lines().filter(|line| *line == needle).count()
}

/// Poll `query events` (which peeks, not drains) until `needle` appears.
#[track_caller]
fn wait_for_event(input: &SyntheticInput, needle: &str, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        let report = input.query("query events");
        if report.lines().any(|line| line == needle) {
            return report;
        }
        assert!(
            Instant::now() < deadline,
            "event {needle:?} never appeared: {report:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// T-02.4b acceptance: a live close ghost **reverses mid-flight** from its
/// current interpolated rect (no waiting, no jump), the window is never
/// removed, it released focus the moment it became a ghost (no phantom focus
/// target), and after the loop settles the idle trace stays flat.
#[test]
fn close_reverses_mid_flight_and_the_idle_trace_stays_flat() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-close-reverse-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-close-reverse",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let app_id = "org.dragonfruit.CloseReverse";
    let tile = (100, 120, 48, 48);
    input.send(&format!(
        "set launch-origin {app_id} {} {} {} {}",
        tile.0, tile.1, tile.2, tile.3
    ));
    let _ = input.query("query motion");
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;

    // Focus the window by clicking its content, so the close can be shown to
    // release the keyboard focus when it becomes a ghost.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some(),
    );
    let (cx, cy) = (floating.0 + floating.2 / 2, floating.1 + floating.3 / 2);
    motion_to(&input, cx, cy);
    click(&input);
    wait_for_event(
        &input,
        &format!("event focused {window}"),
        Duration::from_secs(5),
    );

    // --- close: ghost at once, focus released, no removal yet --------------
    let report = input.query_batch(&format!("close {window}\nquery motion"));
    let motions = parse_motion(&report);
    let close = motions
        .iter()
        .find(|m| m.kind == "close")
        .expect("a close record");
    assert!(
        close.active && !close.completed,
        "the close must be in flight immediately: {close:?}"
    );
    assert_eq!(close.origin, tile, "the ghost shrinks into the Dock tile");
    assert_eq!(close.target, floating, "the ghost leaves from the geometry");
    // The ghost releases the keyboard focus it held (no phantom target).
    let events = wait_for_event(
        &input,
        &format!("event unfocused {window}"),
        Duration::from_secs(2),
    );
    assert_eq!(
        count_events(&events, &format!("event closed {window}")),
        0,
        "the close must not commit while the ghost is live: {events}"
    );

    // Catch it mid-flight (at least one frame in) and reverse it.
    let motions = wait_for_motion(&input, |motions| {
        motions
            .iter()
            .any(|m| m.kind == "close" && !m.completed && m.frames >= 1)
    });
    let close = motions
        .iter()
        .find(|m| m.kind == "close")
        .expect("a live close record");
    assert!(!close.completed, "the close must still be in flight");

    let report = input.query_batch(&format!("interrupt-close {window}\nquery motion"));
    let motions = parse_motion(&report);
    assert!(
        motions.iter().all(|m| !(m.kind == "close" && m.active)),
        "the close must be replaced by the reverse: {motions:?}"
    );
    let restore = motions
        .iter()
        .find(|m| m.kind == "restore")
        .expect("the reverse restore record");
    assert_eq!(
        restore.target, floating,
        "the reverse returns to the window geometry: {restore:?}"
    );
    assert_ne!(
        restore.origin, floating,
        "the reverse must start from the partial ghost, not jump: {restore:?}"
    );
    assert!(
        restore.origin.2 > tile.2 && restore.origin.2 < floating.2,
        "the reverse starts from the interpolated ghost rect: origin={:?} (tile={tile:?}, floating={floating:?})",
        restore.origin
    );
    let events = input.query("query events");
    assert_eq!(
        count_events(&events, &format!("event closed {window}")),
        0,
        "an interrupted close must not commit removal: {events}"
    );

    // The reverse resolves back to the window and the entry stays tracked.
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "restore" && m.completed)
    });
    let restore = motions
        .iter()
        .find(|m| m.kind == "restore")
        .expect("the completed restore");
    assert!(restore.frames >= 1);
    wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating && report.content == floating
    });
    assert!(
        parse_decorations(&input.query("query decorations"))
            .iter()
            .any(|d| d.window == window),
        "the reversed window stays in the model"
    );

    // --- close again, this time to completion ------------------------------
    input.send(&format!("close {window}"));
    let decorations = wait_for_report(&mut state, &mut queue, &input, |reports| {
        !reports.iter().any(|report| report.window == window)
    });
    assert!(
        decorations.iter().all(|report| report.window != window),
        "the settled close must remove the window: {decorations:?}"
    );
    let events = input.query("query events");
    assert_eq!(
        count_events(&events, &format!("event closed {window}")),
        1,
        "the settled close commits removal exactly once: {events:?}"
    );
    let motions = parse_motion(&input.query("query motion"));
    assert!(
        motions.iter().all(|m| m.window != window),
        "no motion record may outlive the closed window: {motions:?}"
    );

    // --- idle trace flat after the loop ------------------------------------
    // The last close disarms the clock; a correct compositor renders nothing
    // and steps no animation while idle. SIGUSR1 dumps the counters.
    std::thread::sleep(Duration::from_millis(300));
    proc.signal(SIGUSR1);
    let _warmup = proc.sample();
    std::thread::sleep(Duration::from_millis(250));
    proc.signal(SIGUSR1);
    let first = proc.sample();
    std::thread::sleep(Duration::from_millis(600));
    proc.signal(SIGUSR1);
    let second = proc.sample();
    assert_eq!(
        second.frames_rendered,
        first.frames_rendered,
        "idle after the loop rendered {} new frame(s): {first:?} -> {second:?}",
        second.frames_rendered - first.frames_rendered,
    );
    assert_eq!(
        second.animation_frames_stepped, first.animation_frames_stepped,
        "the animation clock ticked while idle: {first:?} -> {second:?}",
    );
    eprintln!(
        "close interrupt trace: reverse origin={:?}, idle flat at frames_rendered={}, \
         animation_frames_stepped={}",
        restore.origin, second.frames_rendered, second.animation_frames_stepped,
    );

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-04.4a acceptance: the material degrade tier is selectable over the
/// synthetic harness (and reported), and a T-02 lifecycle motion still
/// interpolates and commits at the most degraded tier — the tier changes the
/// material geometry, never the scene mapping.
#[test]
fn material_degrade_tiers_select_and_transitions_stay_correct() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-degrade-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-degrade",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // The default selection is Full, unpinned, against the 16 ms frame budget.
    let report = parse_degrade(&input.query("query degrade"));
    assert_eq!(report.tier, "full");
    assert!(!report.forced);
    assert_eq!(report.budget_us, 16_000);
    assert_eq!(report.downgrades, 0);
    assert_eq!(report.upgrades, 0);

    // Every tier is selectable and, once forced, reported as pinned.
    for tier in ["reduced", "minimal", "full"] {
        input.send(&format!("set degrade-tier {tier}"));
        let report = parse_degrade(&input.query("query degrade"));
        assert_eq!(report.tier, tier, "forced tier must be reported");
        assert!(report.forced, "a forced tier reports forced=1");
    }

    // The budget is selectable too.
    input.send("set degrade-budget 12000");
    assert_eq!(
        parse_degrade(&input.query("query degrade")).budget_us,
        12_000
    );

    // A lifecycle motion at the most degraded tier: the reusable scene
    // transform still maps the committed surface onto the interpolated rect,
    // and the motion commits the same target geometry as at Full.
    input.send("set degrade-tier minimal");
    let before = parse_degrade(&input.query("query degrade"));

    let app_id = "org.dragonfruit.Degrade";
    input.send(&format!("set launch-origin {app_id} 100 120 48 48"));
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);

    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    let window = appear.window;
    let floating = appear.target;
    assert_eq!(appear.origin, (100, 120, 48, 48));

    input.send(&format!("minimize {window}"));
    let minimized = wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Minimized
    });
    assert_eq!(
        minimized.content, floating,
        "minimize must keep the restore geometry at Minimal: {minimized:?}"
    );
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "minimize" && m.completed)
    });
    let minimize = motions
        .iter()
        .find(|m| m.kind == "minimize")
        .expect("a minimize record");
    assert_eq!(minimize.target, floating);
    assert_eq!(minimize.origin, (100, 120, 48, 48));
    assert!(
        minimize.frames >= 3,
        "the degraded pass must still animate, got {} frames: {minimize:?}",
        minimize.frames
    );

    // Restore resolves back to the original geometry at the same tier.
    input.send(&format!("restore {window}"));
    wait_for_window(&mut state, &mut queue, &input, window, |report| {
        report.state == WindowStateReport::Floating && report.content == floating
    });
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "restore" && m.completed)
    });
    assert!(
        motions.iter().all(|m| m.completed),
        "nothing may be left live on the clock: {motions:?}"
    );

    // The pass is instrumented: the forced tier is still reported and the
    // rendered animation frames were counted at it.
    let after = parse_degrade(&input.query("query degrade"));
    assert_eq!(after.tier, "minimal");
    assert!(after.forced);
    assert!(
        after.samples > before.samples,
        "the motion must have rendered frames the degrade counter observed: \
         {before:?} -> {after:?}"
    );
    assert!(
        after.tier_frames.2 > before.tier_frames.2,
        "the rendered frames must be counted at the minimal tier: {before:?} -> {after:?}"
    );

    let _ = conn;
    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-04.4b acceptance: the live light/dark scheme is selectable and reported
/// over the synthetic harness, both schemes resolve to their own token tones,
/// and the reduced-motion policy composes with either scheme — a reduced
/// appear/minimize still collapses to a single clock frame and commits the
/// same geometry. This is the headless half of the track sign-off: the render
/// path is exercised without a GPU, so both schemes cannot silently regress.
#[test]
fn material_schemes_select_and_reduced_motion_stays_single_frame() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-scheme-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-scheme",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    // The default scheme reads over arbitrary client pixels: dark, with the
    // dark token tones resolved for every compositor material.
    let dark = parse_material(&input.query("query material"));
    assert_eq!(dark.scheme, "dark");
    assert_eq!(dark.chrome, "2d2534ff");
    assert_eq!(dark.elevated, "2d2534ff");
    assert_eq!(dark.accent, "e15c98ff");

    // Light selects the light token set.
    input.send("set color-scheme light");
    let light = parse_material(&input.query("query material"));
    assert_eq!(light.scheme, "light");
    assert_eq!(light.chrome, "ffffffff");
    assert_eq!(light.elevated, "ffffffff");
    assert_eq!(light.border, "ded6e5ff");
    assert_eq!(light.accent, "b32a66ff");
    assert_ne!(
        light.chrome, dark.chrome,
        "light and dark chrome must not resolve to the same tone"
    );
    assert_ne!(light.border, dark.border);

    // A full (non-reduced) lifecycle motion still animates and commits the
    // supplied origin while the light scheme is live.
    let app_id = "org.dragonfruit.Scheme";
    input.send(&format!("set launch-origin {app_id} 70 80 48 48"));
    let (surface, _xdg_surface, toplevel, _file) =
        map_toplevel_with_app_id(&mut state, &mut queue, app_id);
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "appear" && m.completed)
    });
    let appear = motions
        .iter()
        .find(|m| m.kind == "appear")
        .expect("an appear record");
    assert_eq!(appear.origin, (70, 80, 48, 48));
    assert_eq!(appear.target.2, WINDOW_W);
    assert!(
        appear.frames >= 3,
        "a full appear at the light scheme must animate, got {} frames: {appear:?}",
        appear.frames
    );

    // Reduced motion composes with the scheme: the minimize collapses to one
    // frame and commits the same restore geometry.
    input.send("set reduced-motion on");
    input.send("set color-scheme dark");
    input.send(&format!("minimize {}", appear.window));
    let motions = wait_for_motion(&input, |motions| {
        motions.iter().any(|m| m.kind == "minimize" && m.completed)
    });
    let minimize = motions
        .iter()
        .find(|m| m.kind == "minimize")
        .expect("a minimize record");
    assert_eq!(
        minimize.frames, 1,
        "reduced motion must collapse the minimize to one frame: {minimize:?}"
    );
    assert_eq!(
        minimize.target, appear.target,
        "the reduced minimize keeps the restore geometry: {minimize:?}"
    );

    // The scheme round-trips independently of the motion policy.
    assert_eq!(
        parse_material(&input.query("query material")).scheme,
        "dark"
    );

    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// One parsed `grid window` line from `query grid` (T-05.1a): the window id,
/// the committed source rect, the interpolated render target, and the cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GridWindowReport {
    window: u64,
    source: (i32, i32, i32, i32),
    target: (i32, i32, i32, i32),
    cell: (i32, i32, i32, i32),
}

/// The parsed `grid material` line from `query grid` (T-05.1b): the active
/// material degrade tier and the shadow/blur it selects for the grid.
#[derive(Debug, Clone, PartialEq)]
struct GridMaterialReport {
    tier: String,
    blur: bool,
    shadow_layers: u32,
    shadow_radius: f64,
    shadow_opacity: f64,
}

/// The parsed `query grid` reply (T-05.1a/T-05.1b).
#[derive(Debug, Clone, PartialEq)]
struct GridReport {
    active: bool,
    progress: f64,
    material: Option<GridMaterialReport>,
    windows: Vec<GridWindowReport>,
}

/// Parse a `query grid` report (T-05.1a). `grid none` means the overview is
/// closed; otherwise the first line carries `progress=` and each `grid window`
/// line is one live surface.
fn parse_grid(report: &str) -> GridReport {
    let mut parsed = GridReport {
        active: false,
        progress: 0.0,
        material: None,
        windows: Vec::new(),
    };
    for line in report.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() != Some("grid") {
            continue;
        }
        match parts.next() {
            Some("none") => parsed.active = false,
            Some(token) if token.starts_with("progress=") => {
                parsed.active = true;
                parsed.progress = token["progress=".len()..].parse().unwrap_or(0.0);
            }
            Some("material") => {
                let fields: std::collections::HashMap<&str, &str> =
                    parts.filter_map(|value| value.split_once('=')).collect();
                parsed.material = Some(GridMaterialReport {
                    tier: fields.get("tier").copied().unwrap_or_default().to_string(),
                    blur: fields.get("blur").copied() == Some("1"),
                    shadow_layers: fields
                        .get("shadow_layers")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0),
                    shadow_radius: fields
                        .get("shadow_radius")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0.0),
                    shadow_opacity: fields
                        .get("shadow_opacity")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0.0),
                });
            }
            Some("output") => {}
            Some("window") => {
                let Some(window) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
                    continue;
                };
                let values: Vec<i32> = parts
                    .filter_map(|value| value.parse::<i32>().ok())
                    .collect();
                if values.len() != 12 {
                    continue;
                }
                parsed.windows.push(GridWindowReport {
                    window,
                    source: (values[0], values[1], values[2], values[3]),
                    target: (values[4], values[5], values[6], values[7]),
                    cell: (values[8], values[9], values[10], values[11]),
                });
            }
            _ => {}
        }
    }
    parsed
}

/// Poll `query grid` until `pred` holds (T-05.1a).
#[track_caller]
fn wait_for_grid(
    input: &SyntheticInput,
    timeout: Duration,
    mut pred: impl FnMut(&GridReport) -> bool,
) -> GridReport {
    let deadline = Instant::now() + timeout;
    loop {
        let report = parse_grid(&input.query("query grid"));
        if pred(&report) {
            return report;
        }
        assert!(
            Instant::now() < deadline,
            "grid never reached the expected state: {report:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// T-05.1a acceptance: with Mission Control open, the **live** window surfaces
/// are transformed into the documented grid — one uniform scale, centered in
/// non-overlapping cells, each mapped from its committed geometry. The
/// transform interpolates on the shared overview progress and reverses when
/// the overview closes; the client windows stay mapped throughout (never a
/// thumbnail).
#[test]
fn overview_grid_transforms_live_surfaces_into_the_grid() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-grid-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-grid",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (_conn, mut queue, mut state) = connect(&proc.socket_path);

    // Closed: no grid.
    assert!(
        !parse_grid(&input.query("query grid")).active,
        "there must be no grid before Mission Control opens"
    );

    let mut mapped = Vec::new();
    for app in [
        "org.dragonfruit.GridA",
        "org.dragonfruit.GridB",
        "org.dragonfruit.GridC",
    ] {
        let (surface, _xdg_surface, toplevel, file) =
            map_toplevel_with_app_id(&mut state, &mut queue, app);
        mapped.push((surface, toplevel, file));
    }
    let decorations = parse_decorations(&input.query("query decorations"));
    assert_eq!(decorations.len(), 3, "three live surfaces: {decorations:?}");

    // Open Mission Control and catch it mid-transition: the transform is the
    // interpolated rect, not yet the grid cell.
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    let mid = wait_for_grid(&input, Duration::from_secs(5), |grid| {
        grid.active && grid.progress > 0.0 && grid.progress < 1.0 && grid.windows.len() == 3
    });
    for window in &mid.windows {
        assert_ne!(
            window.target, window.source,
            "mid-transition the transform interpolates: {window:?}"
        );
    }
    input.send("swipe-update 0 -100");
    input.send("swipe-end");

    // Settled: three live surfaces placed in the grid at progress 1.
    let grid = wait_for_grid(&input, Duration::from_secs(5), |grid| {
        grid.active && (grid.progress - 1.0).abs() < 1e-6 && grid.windows.len() == 3
    });
    let sources: Vec<_> = grid.windows.iter().map(|window| window.source).collect();
    let targets: Vec<_> = grid.windows.iter().map(|window| window.target).collect();
    let cells: Vec<_> = grid.windows.iter().map(|window| window.cell).collect();

    // Every placement maps the window's committed geometry.
    for source in &sources {
        assert_eq!(
            (source.2, source.3),
            (WINDOW_W, WINDOW_H),
            "the source is the live committed rect: {source:?}"
        );
    }
    // One uniform scale: every target has the same size.
    for target in &targets {
        assert_eq!(
            (target.2, target.3),
            (targets[0].2, targets[0].3),
            "all windows share one grid scale: {targets:?}"
        );
    }
    // Cells never overlap (no occlusion by construction).
    for (index, a) in cells.iter().enumerate() {
        for b in cells.iter().skip(index + 1) {
            let overlaps = a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3;
            assert!(!overlaps, "grid cells must not overlap: {a:?} / {b:?}");
        }
    }
    // Each target is centered in its cell and inside the output (integer
    // division can leave at most a one-pixel offset).
    for (target, cell) in targets.iter().zip(&cells) {
        let center_dx = (target.0 + target.2 / 2) - (cell.0 + cell.2 / 2);
        let center_dy = (target.1 + target.3 / 2) - (cell.1 + cell.3 / 2);
        assert!(center_dx.abs() <= 1 && center_dy.abs() <= 1);
        assert!(
            target.0 >= 0
                && target.1 >= 0
                && target.0 + target.2 <= OUTPUT_W
                && target.1 + target.3 <= OUTPUT_H,
            "the grid target stays on the output: {target:?}"
        );
    }
    // The windows are still the three live client surfaces, not thumbnails.
    assert_eq!(
        parse_decorations(&input.query("query decorations")).len(),
        3,
        "the live surfaces stay mapped in the grid"
    );

    // Closing reverses: a second toggle returns to the normal scene.
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    input.send("swipe-update 0 -100");
    input.send("swipe-end");
    wait_for_grid(&input, Duration::from_secs(5), |grid| !grid.active);

    for (surface, toplevel, _file) in mapped {
        surface.destroy();
        toplevel.destroy();
    }
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-05.1b acceptance: a committing client ("video") keeps advancing at the
/// reduced grid scale — fresh buffers keep producing compositor frames while
/// Mission Control is open and the grid keeps tracking the one live surface
/// (never a frozen thumbnail) — and the T-04 material degrade tier is
/// selectable and applied to the grid composition.
#[test]
fn overview_grid_keeps_live_video_advancing_and_applies_degrade_tier() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-video-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-grid-video",
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // A committing "video" surface, mapped with a real buffer.
    let (surface, _xdg_surface, toplevel, initial) =
        map_toplevel_with_app_id(&mut state, &mut queue, "org.dragonfruit.Video");
    let mut files = vec![initial];

    // Open Mission Control and settle it.
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    input.send("swipe-update 0 -100");
    input.send("swipe-end");
    let settled = wait_for_grid(&input, Duration::from_secs(5), |grid| {
        grid.active && (grid.progress - 1.0).abs() < 1e-6 && grid.windows.len() == 1
    });
    // Full is the default grid material: the high-elevation token shadow with
    // the material blur on.
    let full = settled
        .material
        .expect("the grid reports the material it composes with");
    assert_eq!(full.tier, "full");
    assert!(full.blur, "Full keeps the material blur on");
    assert_eq!(
        full.shadow_layers, 8,
        "the high elevation token layer count"
    );
    assert!(full.shadow_radius > 0.0 && full.shadow_opacity > 0.0);

    // --- the live surface keeps advancing while scaled --------------------
    // Every commit must produce a render on the headless backend, exactly as
    // a real backend would, and the surface must stay the one live window in
    // the grid (a thumbnail substitution would unmap it).
    proc.signal(SIGUSR1);
    let before = proc.sample();
    for _ in 0..3 {
        let qh = queue.handle();
        let (buffer, file) = shm_buffer(&state, &qh, WINDOW_W, WINDOW_H);
        surface.attach(Some(&buffer), 0, 0);
        surface.commit();
        queue.roundtrip(&mut state).expect("video commit");
        files.push(file);
    }
    proc.signal(SIGUSR1);
    let after = proc.sample();
    assert!(
        after.frames_rendered > before.frames_rendered,
        "the playing video must keep producing frames in the grid: {before:?} -> {after:?}"
    );

    // The grid still reports the one live surface at its committed geometry:
    // the transform follows the live window, not a cached copy.
    let live = parse_grid(&input.query("query grid"));
    assert!(
        live.active,
        "the grid stays open across the commits: {live:?}"
    );
    assert_eq!(
        live.windows.len(),
        1,
        "one live surface in the grid: {live:?}"
    );
    assert_eq!(
        (live.windows[0].source.2, live.windows[0].source.3),
        (WINDOW_W, WINDOW_H),
        "the grid source is the committed live rect: {:?}",
        live.windows[0],
    );
    assert_eq!(
        parse_decorations(&input.query("query decorations")).len(),
        1,
        "the live surface stays mapped while it advances in the grid"
    );

    // --- the degrade tier is selectable and applied to the grid -----------
    for (tier, blur, layers) in [("reduced", true, 4u32), ("minimal", false, 2u32)] {
        input.send(&format!("set degrade-tier {tier}"));
        let report = wait_for_grid(&input, Duration::from_secs(5), |grid| {
            grid.active && grid.material.as_ref().is_some_and(|m| m.tier == tier)
        });
        let material = report.material.expect("the grid reports its material");
        assert_eq!(material.blur, blur, "blur state at {tier}");
        assert_eq!(material.shadow_layers, layers, "shadow layers at {tier}");
        assert!(
            material.shadow_layers < full.shadow_layers,
            "{tier} must tighten the grid shadow below Full"
        );
        assert!(
            material.shadow_radius < full.shadow_radius,
            "{tier} must shrink the grid shadow geometry below Full"
        );
    }

    // Restoring Full restores the token material, and the grid is still the
    // same live surface throughout.
    input.send("set degrade-tier full");
    let restored = wait_for_grid(&input, Duration::from_secs(5), |grid| {
        grid.active
            && grid
                .material
                .as_ref()
                .is_some_and(|material| material.tier == "full" && material.blur)
    });
    assert_eq!(restored.material.unwrap().shadow_layers, full.shadow_layers);
    assert_eq!(
        parse_decorations(&input.query("query decorations")).len(),
        1,
        "the live surface survives the tier changes"
    );

    // Closing reverses to no grid at all.
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    input.send("swipe-update 0 -100");
    input.send("swipe-end");
    wait_for_grid(&input, Duration::from_secs(5), |grid| !grid.active);

    let _ = conn;
    drop(files);
    surface.destroy();
    toplevel.destroy();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}
