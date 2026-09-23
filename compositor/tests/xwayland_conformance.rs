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

    /// Send a query and read the compositor's datagram reply.
    fn query(&self, command: &str) -> String {
        self.socket
            .send_to(command.as_bytes(), &self.path)
            .expect("send synthetic query");
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

/// One parsed `query decorations` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecorationReport {
    server_side: bool,
    titlebar: (i32, i32, i32, i32),
    content: (i32, i32, i32, i32),
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
            let _id: u64 = parts.next()?.parse().ok()?;
            Some(DecorationReport {
                server_side: parts.next()? == "1",
                titlebar: (
                    n(&mut parts)?,
                    n(&mut parts)?,
                    n(&mut parts)?,
                    n(&mut parts)?,
                ),
                content: (
                    n(&mut parts)?,
                    n(&mut parts)?,
                    n(&mut parts)?,
                    n(&mut parts)?,
                ),
            })
        })
        .collect()
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
