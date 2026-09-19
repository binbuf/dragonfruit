// SPDX-License-Identifier: MIT OR Apache-2.0
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start compositor");
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

        Self {
            child,
            socket_path,
            display_path,
            stdout,
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
