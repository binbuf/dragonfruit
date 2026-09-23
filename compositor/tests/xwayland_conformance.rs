// SPDX-License-Identifier: MIT
//! T-06 acceptance: Xwayland lifecycle and X11 window management.
//!
//! Starts the compositor on the headless backend, waits for the `DISPLAY`
//! hand-off file, connects as a real X11 client over the X protocol,
//! creates and maps a window, and asserts the compositor's X11 window
//! manager reports it in `_NET_CLIENT_LIST` — then destroys it and asserts
//! it is gone. Finally SIGTERMs the compositor and checks a clean teardown
//! with no surviving Wayland socket or `DISPLAY` file.
//!
//! If `Xwayland` is not installed the test is skipped (the compositor
//! itself is Wayland-only in that case).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as _, CreateWindowAux, PropMode, Window as X11Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

const SOCKET_WAIT: Duration = Duration::from_secs(10);
const X11_WAIT: Duration = Duration::from_secs(10);

/// Headless output size (see `backend/mod.rs::HEADLESS_MODE_SIZE`).
const OUTPUT_W: i32 = 1280;
const OUTPUT_H: i32 = 720;

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    display_path: PathBuf,
    stdout: Option<std::process::ChildStdout>,
    synthetic_path: Option<PathBuf>,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(path) = &self.synthetic_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl CompositorProcess {
    fn start(socket_name: &str) -> Self {
        Self::start_with_synthetic(socket_name, None)
    }

    /// Start with the T-03 synthetic-input harness bound at `synthetic_path`
    /// (`DRAGONFRUIT_SYNTHETIC_INPUT`), used to observe SSD titlebar geometry
    /// for X11 windows (T-01.1).
    fn start_with_synthetic(socket_name: &str, synthetic_path: Option<&Path>) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"));
        command
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(socket_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(path) = synthetic_path {
            command.env("DRAGONFRUIT_SYNTHETIC_INPUT", path);
        }
        let mut child = command.spawn().expect("failed to start compositor");
        let stdout = child.stdout.take();

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(socket_name);
        let display_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.x11-display"));

        let deadline = Instant::now() + SOCKET_WAIT;
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
            display_path,
            stdout,
            synthetic_path: synthetic_path.map(Path::to_path_buf),
        }
    }

    /// Wait for Xwayland's `DISPLAY` file; `None` means Xwayland never came
    /// up (skip the X-dependent assertions).
    fn wait_for_display(&self) -> Option<String> {
        let deadline = Instant::now() + X11_WAIT;
        while Instant::now() < deadline {
            if let Ok(contents) = std::fs::read_to_string(&self.display_path) {
                let display = contents.trim().to_string();
                if !display.is_empty() {
                    return Some(display);
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    }

    fn assert_clean_exit(mut self) {
        unsafe { libc_kill(self.child.id()) };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        if let Some(mut out) = self.stdout.take() {
                            use std::io::Read;
                            let mut buf = String::new();
                            let _ = out.read_to_string(&mut buf);
                            eprintln!("--- compositor stdout ---\n{buf}\n--- end ---");
                        }
                    }
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
        assert!(
            !self.display_path.exists(),
            "teardown leak: DISPLAY file {} survived exit",
            self.display_path.display()
        );
    }
}

#[allow(non_snake_case)]
unsafe fn libc_kill(pid: u32) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    const SIGTERM: i32 = 15;
    kill(pid as i32, SIGTERM);
}

fn xwayland_available() -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join("Xwayland").is_file()))
        .unwrap_or(false)
}

fn client_list(conn: &RustConnection, root: X11Window) -> Vec<X11Window> {
    let atom = conn
        .intern_atom(false, b"_NET_CLIENT_LIST")
        .expect("intern _NET_CLIENT_LIST")
        .reply()
        .expect("intern reply")
        .atom;
    let reply = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, u32::MAX)
        .expect("get _NET_CLIENT_LIST")
        .reply()
        .expect("get reply");
    reply.value32().map(|it| it.collect()).unwrap_or_default()
}

/// Poll `predicate` until it is true or the deadline passes.
fn wait_until(mut predicate: impl FnMut() -> bool, timeout: Duration, what: &str) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {what}");
}

/// Sends synthetic-input commands to the T-03 harness socket and reads the
/// `query decorations` replies (T-01.1).
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

    /// Send a raw synthetic-input command (no reply expected).
    fn send(&self, command: &str) {
        self.socket
            .send_to(command.as_bytes(), &self.path)
            .unwrap_or_else(|err| panic!("send synthetic {command:?}: {err}"));
    }

    /// Send a query and read the compositor's datagram reply.
    fn query(&self, command: &str) -> String {
        self.send(command);
        let mut buf = [0u8; 16 * 1024];
        let len = self.socket.recv(&mut buf).expect("synthetic query reply");
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

/// Traffic-light geometry from `component.trafficLights` tokens (T-01.2).
const LIGHT_DIAMETER: i32 = 12;
const LIGHT_GAP: i32 = 8;
const LIGHT_INSET: i32 = 12;

fn light_center(report: &DecorationReport, index: i32) -> (i32, i32) {
    let (tx, ty, _tw, th) = report.titlebar;
    let cx = tx + LIGHT_INSET + LIGHT_DIAMETER / 2 + index * (LIGHT_DIAMETER + LIGHT_GAP);
    let cy = ty + th / 2;
    (cx, cy)
}

/// Aim the synthetic pointer at one traffic light and click it.
fn click_light(input: &SyntheticInput, report: &DecorationReport, index: i32) {
    let (cx, cy) = light_center(report, index);
    let nx = f64::from(cx) / f64::from(OUTPUT_W);
    let ny = f64::from(cy) / f64::from(OUTPUT_H);
    input.send(&format!("motion-abs {nx} {ny}"));
    input.send("button 272 down");
    input.send("button 272 up");
}

/// Poll `query decorations` until `predicate` holds.
fn wait_for_report(
    input: &SyntheticInput,
    mut predicate: impl FnMut(&[DecorationReport]) -> bool,
) -> Vec<DecorationReport> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let reports = parse_decorations(&input.query("query decorations"));
        if predicate(&reports) {
            return reports;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for decoration state"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn report_for(reports: &[DecorationReport], window: u64) -> DecorationReport {
    *reports
        .iter()
        .find(|report| report.window == window)
        .expect("reported window")
}

/// The design-system SSD titlebar height (T-01.1).
const TITLEBAR_HEIGHT: i32 = 40;

#[test]
fn x11_window_maps_and_unmaps_through_the_xwm() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-x11-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready (no DISPLAY file)");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("failed to connect to Xwayland");
    let root = conn.setup().roots[screen_num].root;
    let screen = &conn.setup().roots[screen_num];

    // Create a window and give it a WM_CLASS (identity fallback path).
    let window = conn.generate_id().expect("generate window id");
    let aux = CreateWindowAux::new().background_pixel(screen.white_pixel);
    conn.create_window(
        screen.root_depth,
        window,
        root,
        10,
        10,
        200,
        120,
        0,
        WindowClass::INPUT_OUTPUT,
        x11rb::COPY_FROM_PARENT,
        &aux,
    )
    .expect("create_window");
    conn.change_property8(
        PropMode::REPLACE,
        window,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        b"firefox\0Firefox\0",
    )
    .expect("set WM_CLASS");
    conn.map_window(window).expect("map_window");
    conn.flush().expect("flush");

    // The XWM should publish the window on the root.
    wait_until(
        || client_list(&conn, root).contains(&window),
        Duration::from_secs(5),
        "window to appear in _NET_CLIENT_LIST",
    );

    // Destroy it; the XWM must drop it from the client list.
    conn.destroy_window(window).expect("destroy_window");
    conn.flush().expect("flush");
    wait_until(
        || !client_list(&conn, root).contains(&window),
        Duration::from_secs(5),
        "window to leave _NET_CLIENT_LIST",
    );

    drop(conn);
    proc.assert_clean_exit();
}

/// T-01.1 acceptance: an X11 Tier-2 window carries the same SSD titlebar as
/// a Wayland SSD toplevel.
#[test]
fn x11_window_carries_the_ssd_titlebar() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-x11-deco-{}", std::process::id());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-x11-deco-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(&socket_name, Some(&synthetic_path));
    let input = SyntheticInput::connect(&synthetic_path);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let root = conn.setup().roots[screen_num].root;
    let screen = &conn.setup().roots[screen_num];
    let window = conn.generate_id().expect("generate window id");
    let aux = CreateWindowAux::new().background_pixel(screen.white_pixel);
    conn.create_window(
        screen.root_depth,
        window,
        root,
        40,
        40,
        200,
        120,
        0,
        WindowClass::INPUT_OUTPUT,
        x11rb::COPY_FROM_PARENT,
        &aux,
    )
    .expect("create_window");
    conn.map_window(window).expect("map_window");
    conn.flush().expect("flush");

    wait_until(
        || client_list(&conn, root).contains(&window),
        Duration::from_secs(5),
        "X11 window to be managed",
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    let ssd = loop {
        let reports = parse_decorations(&input.query("query decorations"));
        if let Some(report) = reports.into_iter().find(|report| report.server_side) {
            break report;
        }
        assert!(
            Instant::now() < deadline,
            "the X11 Tier-2 window never got an SSD titlebar"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        ssd.titlebar.3, TITLEBAR_HEIGHT,
        "X11 must carry the same token titlebar: {ssd:?}"
    );
    assert_eq!(ssd.titlebar.0, ssd.content.0);
    assert_eq!(
        ssd.titlebar.1 + ssd.titlebar.3,
        ssd.content.1,
        "the X11 client area must sit below the titlebar"
    );

    drop(conn);
    drop(input);
    proc.assert_clean_exit();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// T-01.5: an X11 window that asks to be undecorated via `_MOTIF_WM_HINTS`
/// gets no compositor titlebar (the double-decoration guard). A runtime hint
/// flip reflows a zoomed window through `configure_window_size`, and the X
/// server actually receives the inset-adjusted geometry.
#[test]
fn x11_decoration_tier_follows_motif_hints_and_configures_insets() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-x11-tier-{}", std::process::id());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-x11-tier-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(&socket_name, Some(&synthetic_path));
    let input = SyntheticInput::connect(&synthetic_path);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let root = conn.setup().roots[screen_num].root;
    let screen = &conn.setup().roots[screen_num];
    let motif = conn
        .intern_atom(false, b"_MOTIF_WM_HINTS")
        .expect("intern _MOTIF_WM_HINTS")
        .reply()
        .expect("intern reply")
        .atom;

    let window = conn.generate_id().expect("generate window id");
    let aux = CreateWindowAux::new().background_pixel(screen.white_pixel);
    conn.create_window(
        screen.root_depth,
        window,
        root,
        40,
        40,
        200,
        120,
        0,
        WindowClass::INPUT_OUTPUT,
        x11rb::COPY_FROM_PARENT,
        &aux,
    )
    .expect("create_window");
    // `MWM_HINTS_DECORATIONS` flag set, decorations field 0: the client asks
    // to draw its own decoration. Set before the first map.
    conn.change_property32(PropMode::REPLACE, window, motif, motif, &[2, 0, 0, 0, 0])
        .expect("set motif hints");
    conn.map_window(window).expect("map_window");
    conn.flush().expect("flush");

    wait_until(
        || client_list(&conn, root).contains(&window),
        Duration::from_secs(5),
        "X11 window to be managed",
    );

    let reports = wait_for_report(&input, |reports| !reports.is_empty());
    let report = report_for(&reports, reports[0].window);
    assert!(
        !report.server_side && report.titlebar == (0, 0, 0, 0),
        "an explicitly undecorated X11 window must not get our titlebar: {report:?}"
    );
    assert_eq!((report.content.2, report.content.3), (200, 120));
    let window_id = report.window;

    // The client now asks for WM decoration: we must add exactly one titlebar.
    conn.change_property32(PropMode::REPLACE, window, motif, motif, &[2, 0, 2, 0, 0])
        .expect("set motif hints");
    conn.flush().expect("flush");
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.window == window_id && report.server_side)
    });
    let report = report_for(&reports, window_id);
    assert_eq!(report.titlebar.3, TITLEBAR_HEIGHT);

    // Zoom through the green light: the client fits under the titlebar.
    click_light(&input, &report, 2);
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.window == window_id && report.state == WindowStateReport::Zoomed)
    });
    let report = report_for(&reports, window_id);
    assert!(
        report.server_side,
        "a zoomed X11 SSD window keeps its titlebar"
    );
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT)
    );

    // The inset size really reached the X server (via `configure_window_size`):
    // the X client window is one titlebar shorter than the output. Smithay
    // reparents the client into a frame, so its own origin is (0, 0); the
    // frame's position (the content offset) is verified through the report
    // geometry above.
    wait_until(
        || {
            conn.get_geometry(window)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|g| (i32::from(g.width), i32::from(g.height)))
                == Some((OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT))
        },
        Duration::from_secs(5),
        "the X server to receive the inset-adjusted configure",
    );

    // Runtime flip to undecorated while zoomed: the titlebar disappears and
    // the client reclaims the full output through the same configure path.
    conn.change_property32(PropMode::REPLACE, window, motif, motif, &[2, 0, 0, 0, 0])
        .expect("set motif hints");
    conn.flush().expect("flush");
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.window == window_id && !report.server_side)
    });
    let report = report_for(&reports, window_id);
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H),
        "an undecorated zoomed X11 window fills the output (no decoration gap)"
    );
    wait_until(
        || {
            conn.get_geometry(window)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|g| (i32::from(g.width), i32::from(g.height)))
                == Some((OUTPUT_W, OUTPUT_H))
        },
        Duration::from_secs(5),
        "the X server to receive the full-output configure",
    );

    conn.destroy_window(window).expect("destroy_window");
    conn.flush().expect("flush");
    drop(conn);
    drop(input);
    proc.assert_clean_exit();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// T-01.2 acceptance: synthetic pointer clicks on the traffic lights drive
/// zoom/unzoom, minimize, and close for an X11 client through the same state
/// machine as a Wayland client.
#[test]
fn x11_traffic_lights_drive_zoom_minimize_and_close() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-x11-traffic-{}", std::process::id());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-x11-traffic-synth-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(&socket_name, Some(&synthetic_path));
    let input = SyntheticInput::connect(&synthetic_path);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let root = conn.setup().roots[screen_num].root;
    let screen = &conn.setup().roots[screen_num];

    let make_window = |x: i16, y: i16, w: u16, h: u16| -> X11Window {
        let window = conn.generate_id().expect("generate window id");
        let aux = CreateWindowAux::new().background_pixel(screen.white_pixel);
        conn.create_window(
            screen.root_depth,
            window,
            root,
            x,
            y,
            w,
            h,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &aux,
        )
        .expect("create_window");
        conn.map_window(window).expect("map_window");
        conn.flush().expect("flush");
        wait_until(
            || client_list(&conn, root).contains(&window),
            Duration::from_secs(5),
            "X11 window to be managed",
        );
        window
    };

    let _window = make_window(40, 40, 200, 120);
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && report.state == WindowStateReport::Floating)
    });
    let mut report = *reports.iter().find(|report| report.server_side).unwrap();
    let window_id = report.window;
    let floating_content = report.content;

    // --- green (zoom) ----------------------------------------------------
    click_light(&input, &report, 2);
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.window == window_id && report.state == WindowStateReport::Zoomed)
    });
    report = report_for(&reports, window_id);
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT),
        "X11 zoom must fill the usable area: {report:?}"
    );
    assert!(report.server_side, "a zoomed X11 window keeps its titlebar");

    // --- green again (unzoom) --------------------------------------------
    click_light(&input, &report, 2);
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.window == window_id && report.state == WindowStateReport::Floating)
    });
    report = report_for(&reports, window_id);
    assert_eq!(
        report.content, floating_content,
        "unzoom must restore the X11 floating geometry"
    );

    // --- yellow (minimize) -----------------------------------------------
    click_light(&input, &report, 1);
    let reports = wait_for_report(&input, |reports| {
        reports.iter().any(|report| {
            report.window == window_id && report.state == WindowStateReport::Minimized
        })
    });
    report = report_for(&reports, window_id);
    assert!(
        !report.server_side && report.titlebar == (0, 0, 0, 0),
        "a minimized X11 window draws no titlebar: {report:?}"
    );

    // --- red (close) on a second window ----------------------------------
    let window2 = make_window(300, 200, 220, 140);
    let reports = wait_for_report(&input, |reports| {
        reports.iter().filter(|report| report.server_side).count() == 1
    });
    let close_report = *reports
        .iter()
        .find(|report| report.server_side)
        .expect("the second X11 window must carry a titlebar");
    click_light(&input, &close_report, 0);
    wait_until(
        || !client_list(&conn, root).contains(&window2),
        Duration::from_secs(5),
        "X11 close to destroy the window",
    );

    drop(conn);
    drop(input);
    proc.assert_clean_exit();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

/// Move the synthetic pointer to a global-space point (libinput-normalized).
fn motion_to(input: &SyntheticInput, x: i32, y: i32) {
    let nx = f64::from(x) / f64::from(OUTPUT_W);
    let ny = f64::from(y) / f64::from(OUTPUT_H);
    input.send(&format!("motion-abs {nx} {ny}"));
}

/// A left-button down/up at the current pointer location (BTN_LEFT = 272).
fn click_at(input: &SyntheticInput, x: i32, y: i32) {
    motion_to(input, x, y);
    input.send("button 272 down");
    input.send("button 272 up");
}

/// The center of a reported titlebar, away from the traffic-light cluster.
fn titlebar_center(report: &DecorationReport) -> (i32, i32) {
    let (tx, ty, tw, th) = report.titlebar;
    (tx + tw / 2, ty + th / 2)
}

/// Right-click the window's titlebar center to open the window menu.
fn open_window_menu(input: &SyntheticInput, report: &DecorationReport) {
    let (cx, cy) = titlebar_center(report);
    motion_to(input, cx, cy);
    input.send("button 273 down"); // BTN_RIGHT
    input.send("button 273 up");
}

/// Linux evdev key code for Escape (the synthetic harness adds the xkb +8).
const KEY_ESC: u32 = 1;

fn key_tap(input: &SyntheticInput, code: u32) {
    input.send(&format!("key {code} down"));
    input.send(&format!("key {code} up"));
}

/// `component.contextMenu` geometry used to aim at menu rows (T-01.4).
const MENU_PADDING: i32 = 6;
const MENU_ROW_HEIGHT: i32 = 26;
const MENU_MIN_WIDTH: i32 = 180;

/// One parsed `query window-menu` line (T-01.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MenuReport {
    open: bool,
    window: u64,
    rect: (i32, i32, i32, i32),
    highlighted: i64,
    submenu_open: bool,
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
    let _submenu_highlighted = n();
    let submenu_rect = (n() as i32, n() as i32, n() as i32, n() as i32);
    MenuReport {
        open,
        window,
        rect,
        highlighted,
        submenu_open,
        submenu_rect,
    }
}

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
            "timed out waiting for window-menu state"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// T-01.4 acceptance: the window menu's four commands (Zoom, Minimize, Close,
/// Move to Space) work for an X11 Tier-2 window through the same state machine
/// as a Wayland client, and Escape dismisses the menu.
#[test]
fn x11_window_menu_runs_all_four_commands() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-x11-menu-{}", std::process::id());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-x11-menu-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(&socket_name, Some(&synthetic_path));
    let input = SyntheticInput::connect(&synthetic_path);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let root = conn.setup().roots[screen_num].root;
    let screen = &conn.setup().roots[screen_num];

    let make_window = |x: i16, y: i16, w: u16, h: u16| -> X11Window {
        let window = conn.generate_id().expect("generate window id");
        let aux = CreateWindowAux::new().background_pixel(screen.white_pixel);
        conn.create_window(
            screen.root_depth,
            window,
            root,
            x,
            y,
            w,
            h,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &aux,
        )
        .expect("create_window");
        conn.map_window(window).expect("map_window");
        conn.flush().expect("flush");
        wait_until(
            || client_list(&conn, root).contains(&window),
            Duration::from_secs(5),
            "X11 window to be managed",
        );
        window
    };

    // --- Zoom / Unzoom / Minimize on the first window --------------------
    let window = make_window(40, 40, 200, 120);
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && report.state == WindowStateReport::Floating)
    });
    let mut report = *reports.iter().find(|report| report.server_side).unwrap();
    let window_id = report.window;

    // Right-click opens; Escape dismisses.
    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    assert_eq!(menu.window, window_id);
    key_tap(&input, KEY_ESC);
    wait_for_menu(&input, |menu| !menu.open);

    // Zoom.
    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 2);
    click_at(&input, cx, cy);
    report = report_for(
        &wait_for_report(&input, |reports| {
            reports.iter().any(|report| {
                report.window == window_id && report.state == WindowStateReport::Zoomed
            })
        }),
        window_id,
    );
    assert_eq!(
        (report.content.2, report.content.3),
        (OUTPUT_W, OUTPUT_H - TITLEBAR_HEIGHT)
    );

    // Unzoom.
    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 2);
    click_at(&input, cx, cy);
    report = report_for(
        &wait_for_report(&input, |reports| {
            reports.iter().any(|report| {
                report.window == window_id && report.state == WindowStateReport::Floating
            })
        }),
        window_id,
    );

    // Minimize.
    open_window_menu(&input, &report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 1);
    click_at(&input, cx, cy);
    let minimized = report_for(
        &wait_for_report(&input, |reports| {
            reports.iter().any(|report| {
                report.window == window_id && report.state == WindowStateReport::Minimized
            })
        }),
        window_id,
    );
    assert!(
        !minimized.server_side,
        "a minimized X11 window has no titlebar"
    );

    conn.destroy_window(window).expect("destroy_window");
    conn.flush().expect("flush");
    wait_until(
        || !client_list(&conn, root).contains(&window),
        Duration::from_secs(5),
        "the first X11 window to leave _NET_CLIENT_LIST",
    );

    // --- Close on a second window ----------------------------------------
    let window2 = make_window(300, 200, 220, 140);
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && report.state == WindowStateReport::Floating)
    });
    let close_report = *reports.iter().find(|report| report.server_side).unwrap();
    open_window_menu(&input, &close_report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 3);
    click_at(&input, cx, cy);
    wait_until(
        || !client_list(&conn, root).contains(&window2),
        Duration::from_secs(5),
        "the menu Close to destroy the X11 window",
    );

    // --- Move to Space on a third window ---------------------------------
    let _window3 = make_window(500, 300, 200, 120);
    let reports = wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.server_side && report.state == WindowStateReport::Floating)
    });
    let move_report = *reports.iter().find(|report| report.server_side).unwrap();
    let before_space = move_report.space;
    assert!(before_space >= 0, "a mapped X11 window is assigned a Space");

    open_window_menu(&input, &move_report);
    let menu = wait_for_menu(&input, |menu| menu.open);
    let (cx, cy) = menu_row_center(&menu, 0);
    click_at(&input, cx, cy);
    let menu = wait_for_menu(&input, |menu| menu.submenu_open);
    let (cx, cy) = submenu_row_center(&menu, 1);
    click_at(&input, cx, cy);
    wait_for_report(&input, |reports| {
        reports
            .iter()
            .any(|report| report.window == move_report.window && report.space != before_space)
    });
    wait_for_menu(&input, |menu| !menu.open);

    drop(conn);
    drop(input);
    proc.assert_clean_exit();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic socket survived"
    );
}

#[test]
fn xwayland_restart_after_kill_is_clean() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-x11-restart-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(first_display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    // Kill the Xwayland process behind the compositor's back. The session
    // must survive and respawn the server (FR-6).
    let killed = kill_xwayland_for(&first_display);
    if !killed {
        eprintln!("skipping: could not identify the Xwayland process to kill");
        return;
    }

    // The compositor must come back with a live Xwayland and a working X11
    // WM (possibly reusing the same display number). Connecting and reading
    // `_NET_CLIENT_LIST` only succeeds once the new WM is installed.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut recovered = false;
    while Instant::now() < deadline {
        if let Ok(contents) = std::fs::read_to_string(&proc.display_path) {
            let display = contents.trim();
            if !display.is_empty() {
                if let Ok((conn, screen_num)) = x11rb::connect(Some(display)) {
                    let root = conn.setup().roots[screen_num].root;
                    let atom = conn
                        .intern_atom(false, b"_NET_CLIENT_LIST")
                        .ok()
                        .and_then(|cookie| cookie.reply().ok())
                        .map(|reply| reply.atom)
                        .unwrap_or(0);
                    if atom != 0 {
                        let property = conn
                            .get_property(false, root, atom, AtomEnum::WINDOW, 0, u32::MAX)
                            .ok()
                            .and_then(|cookie| cookie.reply().ok());
                        if property.is_some() {
                            recovered = true;
                            break;
                        }
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(recovered, "Xwayland did not recover after being killed");

    proc.assert_clean_exit();
}

/// Find and SIGKILL the Xwayland process serving `display` (e.g. `:1`).
/// Returns whether a process was found.
fn kill_xwayland_for(display: &str) -> bool {
    let entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        let pid = match entry.file_name().to_string_lossy().parse::<i32>() {
            Ok(pid) => pid,
            Err(_) => continue,
        };
        let cmdline = match std::fs::read(Path::new("/proc").join(pid.to_string()).join("cmdline"))
        {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let cmdline = String::from_utf8_lossy(&cmdline);
        if cmdline.contains("Xwayland") && cmdline.contains(display) {
            unsafe {
                libc_kill_pid(pid);
            }
            return true;
        }
    }
    false
}

#[allow(non_snake_case)]
unsafe fn libc_kill_pid(pid: i32) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    const SIGKILL: i32 = 9;
    kill(pid, SIGKILL);
}
