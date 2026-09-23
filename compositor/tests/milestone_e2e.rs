// SPDX-License-Identifier: MIT
//! Foundation milestone end-to-end test (T-01…T-07).
//!
//! The per-ticket conformance suites each exercise one subsystem in
//! isolation. This test composes the whole Foundation vertical slice in a
//! single live session, the way the phase exit criterion describes it
//! (docs/tasks/legacy/00-index.md, "Phase exit criteria" 1):
//!
//! 1. Start the compositor on the headless backend and provision a shell
//!    launch token (T-01/T-02/T-07).
//! 2. Connect as the shell: `df_core` handshake, `df_shell` menu-bar chrome
//!    with a reserved zone, `df_toplevel_manager` scene replay (T-07).
//! 3. Map a Wayland `xdg_toplevel` (T-04) *and* an X11 window through
//!    Xwayland (T-06) while the shell is attached.
//! 4. Assert both windows are announced to the shell with identity and
//!    that the menu bar's reserved zone reached the output.
//! 5. Drive the interaction loop through the shell protocol: activate a
//!    Space (T-05), move a window to it, zoom/minimize/restore it (T-04),
//!    and observe the state broadcasts.
//! 6. SIGTERM and assert a clean teardown: no surviving socket, token,
//!    `DISPLAY` file, or orphaned Xwayland process.
//!
//! Xwayland-dependent assertions are skipped when `Xwayland` is not
//! installed (the session is Wayland-only then, which is a normal state).

use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel as xdg_tl, xdg_wm_base};

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

mod shell_client {
    #![allow(unused_imports, clippy::single_component_path_imports)]
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-shell.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("../protocols/dragonfruit-shell.xml");
}

mod toplevel_client {
    #![allow(unused_imports, clippy::single_component_path_imports)]
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-toplevel.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("../protocols/dragonfruit-toplevel.xml");
}

use core_client::df_core;
use shell_client::{df_layer_surface, df_shell};
use toplevel_client::{df_output, df_toplevel, df_toplevel_manager, df_workspace};

/// Headless output size (see `backend/mod.rs::HEADLESS_MODE_SIZE`).
const OUTPUT_W: i32 = 1280;

/// The toplevel buffer size; the restored floating geometry after
/// unzoom/unminimize.
const WINDOW_W: i32 = 200;
const WINDOW_H: i32 = 150;

const MENUBAR_HEIGHT: i32 = 24;

#[derive(Default)]
struct E2eClient {
    // Registry handles.
    registry: Option<wl_registry::WlRegistry>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    // Private global names.
    core_global: Option<(u32, u32)>,
    shell_global: Option<(u32, u32)>,
    manager_global: Option<(u32, u32)>,
    // Private protocol state.
    authenticated: Option<u32>,
    refused: Vec<(u32, String)>,
    outputs: Vec<df_output::DfOutput>,
    workspaces: Vec<df_workspace::DfWorkspace>,
    toplevels: Vec<df_toplevel::DfToplevel>,
    output_names: Vec<String>,
    output_reserved: Vec<(u32, u32)>,
    workspace_names: Vec<String>,
    workspace_activated: Vec<u32>,
    layer_configures: Vec<(u32, i32, i32)>,
    done_count: usize,
    // Per-toplevel properties keyed by protocol id (the announcement
    // carries title/app_id before we have a convenient handle name).
    toplevel_titles: Vec<(u32, Option<String>)>,
    toplevel_app_ids: Vec<(u32, Option<String>)>,
    toplevel_states: Vec<(u32, u32)>,
    workspace_entered: Vec<df_workspace::DfWorkspace>,
}

impl E2eClient {
    fn toplevel_with_title(&self, title: &str) -> Option<df_toplevel::DfToplevel> {
        self.toplevels
            .iter()
            .find(|handle| {
                let id = handle.id().protocol_id();
                self.toplevel_titles
                    .iter()
                    .any(|(tid, t)| *tid == id && t.as_deref() == Some(title))
            })
            .cloned()
    }

    fn toplevel_has_state(&self, handle: &df_toplevel::DfToplevel, flag: u32) -> bool {
        let id = handle.id().protocol_id();
        self.toplevel_states
            .iter()
            .any(|(tid, flags)| *tid == id && flags & flag != 0)
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for E2eClient {
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
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(6), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base = Some(registry.bind(name, version.min(6), qh, ()));
                }
                "df_core" => state.core_global = Some((name, version)),
                "df_shell" => state.shell_global = Some((name, version)),
                "df_toplevel_manager" => state.manager_global = Some((name, version)),
                _ => {}
            }
        }
    }
}

impl Dispatch<df_core::DfCore, ()> for E2eClient {
    fn event(
        state: &mut Self,
        _: &df_core::DfCore,
        event: df_core::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            df_core::Event::Authenticated { lockstep_version } => {
                state.authenticated = Some(lockstep_version)
            }
            df_core::Event::Refused { code, message } => state.refused.push((code, message)),
        }
    }
}

impl Dispatch<df_shell::DfShell, ()> for E2eClient {
    fn event(
        _: &mut Self,
        _: &df_shell::DfShell,
        _: df_shell::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<df_layer_surface::DfLayerSurface, ()> for E2eClient {
    fn event(
        state: &mut Self,
        _: &df_layer_surface::DfLayerSurface,
        event: df_layer_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let df_layer_surface::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            state.layer_configures.push((serial, width, height));
        }
    }
}

impl Dispatch<df_toplevel_manager::DfToplevelManager, ()> for E2eClient {
    fn event(
        state: &mut Self,
        _: &df_toplevel_manager::DfToplevelManager,
        event: df_toplevel_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            df_toplevel_manager::Event::Output { id } => state.outputs.push(id),
            df_toplevel_manager::Event::Workspace { id } => state.workspaces.push(id),
            df_toplevel_manager::Event::Toplevel { id } => state.toplevels.push(id),
            df_toplevel_manager::Event::WorkspaceActivated { index, .. } => {
                state.workspace_activated.push(index)
            }
            df_toplevel_manager::Event::Done => state.done_count += 1,
            _ => {}
        }
    }

    wayland_client::event_created_child!(E2eClient, df_toplevel_manager::DfToplevelManager, [
        df_toplevel_manager::EVT_OUTPUT_OPCODE => (df_output::DfOutput, ()),
        df_toplevel_manager::EVT_WORKSPACE_OPCODE => (df_workspace::DfWorkspace, ()),
        df_toplevel_manager::EVT_TOPLEVEL_OPCODE => (df_toplevel::DfToplevel, ()),
    ]);
}

impl Dispatch<df_output::DfOutput, ()> for E2eClient {
    fn event(
        state: &mut Self,
        _: &df_output::DfOutput,
        event: df_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            df_output::Event::Name { name } => state.output_names.push(name),
            df_output::Event::ReservedZone { edge, thickness } => state
                .output_reserved
                .push((edge.into_result().map(|e| e as u32).unwrap_or(0), thickness)),
            _ => {}
        }
    }
}

impl Dispatch<df_workspace::DfWorkspace, ()> for E2eClient {
    fn event(
        state: &mut Self,
        _: &df_workspace::DfWorkspace,
        event: df_workspace::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let df_workspace::Event::Name { name } = event {
            state.workspace_names.push(name);
        }
    }
}

impl Dispatch<df_toplevel::DfToplevel, ()> for E2eClient {
    fn event(
        state: &mut Self,
        resource: &df_toplevel::DfToplevel,
        event: df_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = resource.id().protocol_id();
        match event {
            df_toplevel::Event::Title { title } => state.toplevel_titles.push((id, title)),
            df_toplevel::Event::AppId { app_id } => state.toplevel_app_ids.push((id, app_id)),
            df_toplevel::Event::State { state: flags } => state
                .toplevel_states
                .push((id, flags.into_result().map(|f| f.bits()).unwrap_or(0))),
            df_toplevel::Event::WorkspaceEntered { workspace } => {
                state.workspace_entered.push(workspace)
            }
            _ => {}
        }
    }
}

delegate_noop!(E2eClient: ignore wl_compositor::WlCompositor);
delegate_noop!(E2eClient: ignore wl_shm::WlShm);
delegate_noop!(E2eClient: ignore wl_shm_pool::WlShmPool);
delegate_noop!(E2eClient: ignore wl_buffer::WlBuffer);
delegate_noop!(E2eClient: ignore wl_surface::WlSurface);

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for E2eClient {
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

impl Dispatch<xdg_surface::XdgSurface, ()> for E2eClient {
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

impl Dispatch<xdg_tl::XdgToplevel, ()> for E2eClient {
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

// --- process harness -------------------------------------------------------

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    token_path: PathBuf,
    display_path: PathBuf,
    stderr_path: PathBuf,
    x11_display: Option<String>,
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
        // The compositor's log is the only clue when it panics mid-test;
        // surface it on failure (including an unwind from another thread).
        if std::thread::panicking() {
            if let Ok(log) = std::fs::read_to_string(&self.stderr_path) {
                eprintln!("--- compositor stderr ---\n{log}--- end ---");
            }
        }
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(self.socket_path.with_extension("lock"));
        let _ = std::fs::remove_file(&self.token_path);
        let _ = std::fs::remove_file(&self.display_path);
        let _ = std::fs::remove_file(&self.stderr_path);
    }
}

impl CompositorProcess {
    fn start(socket_name: &str, token: &str) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let token_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.launch-token"));
        let display_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.x11-display"));
        let stderr_path = std::env::temp_dir().join(format!("{socket_name}.stderr"));
        let stderr_file = std::fs::File::create(&stderr_path).expect("create stderr log");

        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_LAUNCH_TOKENS", token)
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_file))
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

        Self {
            child,
            socket_path,
            token_path,
            display_path,
            stderr_path,
            x11_display: None,
        }
    }

    fn read_token(&self) -> String {
        std::fs::read_to_string(&self.token_path)
            .expect("token file readable")
            .trim()
            .to_string()
    }

    /// Wait for Xwayland's `DISPLAY` hand-off file (T-06); `None` means
    /// Xwayland is unavailable and the X11 half of the test is skipped.
    fn wait_for_display(&mut self) -> Option<String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Ok(contents) = std::fs::read_to_string(&self.display_path) {
                let display = contents.trim().to_string();
                if !display.is_empty() {
                    self.x11_display = Some(display.clone());
                    return Some(display);
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    }

    /// SIGTERM and assert the whole session tears down cleanly: exit 0, no
    /// socket, no token, no `DISPLAY` file, no orphaned Xwayland.
    fn shutdown(mut self) {
        let display = self.x11_display.clone();
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
        assert!(
            !self.display_path.exists(),
            "teardown leak: DISPLAY file survived"
        );
        if let Some(display) = display {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if !xwayland_running_for(&display) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            panic!("teardown leak: Xwayland for {display} survived the session");
        }
    }
}

const SIGTERM: i32 = 15;

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

fn connect(socket_path: &Path) -> (Connection, EventQueue<E2eClient>, E2eClient) {
    let stream = std::os::unix::net::UnixStream::connect(socket_path).expect("connect failed");
    let conn = Connection::from_socket(stream).expect("connection failed");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = E2eClient::default();
    queue.roundtrip(&mut state).expect("registry roundtrip");
    (conn, queue, state)
}

/// Dispatch until `pred` is true or the timeout expires, bounded with
/// `poll` so a missing event fails instead of hanging the test.
fn wait_for(
    conn: &Connection,
    queue: &mut EventQueue<E2eClient>,
    state: &mut E2eClient,
    timeout: Duration,
    mut pred: impl FnMut(&E2eClient) -> bool,
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
    state: &E2eClient,
    qh: &QueueHandle<E2eClient>,
    width: i32,
    height: i32,
) -> (wl_buffer::WlBuffer, std::fs::File) {
    let shm = state.shm.clone().expect("wl_shm bound");
    let stride = width * 4;
    let size = (stride * height) as usize;
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-milestone-e2e-{}-{:p}.shm",
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

/// Map a Wayland toplevel on the app connection.
fn map_wayland_toplevel(
    state: &mut E2eClient,
    queue: &mut EventQueue<E2eClient>,
    title: &str,
    app_id: &str,
) -> (wl_surface::WlSurface, std::fs::File) {
    let qh = queue.handle();
    let compositor = state.compositor.clone().unwrap();
    let wm_base = state.xdg_wm_base.clone().unwrap();

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title(title.to_string());
    toplevel.set_app_id(app_id.to_string());
    surface.commit();
    queue.roundtrip(state).expect("initial configure");

    let (buffer, file) = shm_buffer(state, &qh, WINDOW_W, WINDOW_H);
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();
    queue.roundtrip(state).expect("map commit");
    (surface, file)
}

// --- X11 helpers -----------------------------------------------------------

fn xwayland_available() -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join("Xwayland").is_file()))
        .unwrap_or(false)
}

/// Map an X11 window with a `WM_CLASS`, `WM_NAME`, and `_NET_WM_NAME`.
fn map_x11_window(display: &str) -> Option<x11rb::rust_connection::RustConnection> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{
        AtomEnum, ConnectionExt as _, CreateWindowAux, PropMode, WindowClass,
    };
    use x11rb::wrapper::ConnectionExt as _;

    let (conn, screen_num) = x11rb::connect(Some(display)).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let screen = &conn.setup().roots[screen_num];

    let window = conn.generate_id().ok()?;
    let aux = CreateWindowAux::new().background_pixel(screen.white_pixel);
    conn.create_window(
        screen.root_depth,
        window,
        root,
        10,
        10,
        WINDOW_W as u16,
        WINDOW_H as u16,
        0,
        WindowClass::INPUT_OUTPUT,
        x11rb::COPY_FROM_PARENT,
        &aux,
    )
    .ok()?;
    conn.change_property8(
        PropMode::REPLACE,
        window,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        b"dragonfruit-e2e\0DragonfruitE2E\0",
    )
    .ok()?;
    conn.change_property8(
        PropMode::REPLACE,
        window,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        b"E2E X11\0",
    )
    .ok()?;
    let net_wm_name = conn
        .intern_atom(false, b"_NET_WM_NAME")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let utf8_string = conn
        .intern_atom(false, b"UTF8_STRING")
        .ok()?
        .reply()
        .ok()?
        .atom;
    conn.change_property8(
        PropMode::REPLACE,
        window,
        net_wm_name,
        utf8_string,
        b"E2E X11\0",
    )
    .ok()?;
    conn.map_window(window).ok()?;
    conn.flush().ok()?;
    Some(conn)
}

/// Scan `/proc` for an Xwayland process serving `display` (e.g. `:1`).
fn xwayland_running_for(display: &str) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
            continue;
        };
        let cmdline = String::from_utf8_lossy(&cmdline);
        if cmdline.contains("Xwayland") && cmdline.contains(display) {
            return true;
        }
    }
    false
}

// --- the milestone test ----------------------------------------------------

#[test]
fn foundation_vertical_slice_end_to_end() {
    let token = "ee".repeat(32);
    let mut proc = CompositorProcess::start("dragonfruit-milestone-e2e", &token);
    let x11_display = if xwayland_available() {
        proc.wait_for_display()
    } else {
        eprintln!("note: Xwayland is not installed; running the Wayland-only slice");
        None
    };

    // --- shell connects and authenticates (T-07 FR-4) --------------------
    let (shell_conn, mut shell_queue, mut shell) = connect(&proc.socket_path);
    let (core_name, core_version) = shell.core_global.expect("df_core advertised");
    let core = {
        let qh = shell_queue.handle();
        shell
            .registry
            .clone()
            .unwrap()
            .bind::<df_core::DfCore, _, _>(core_name, core_version, &qh, ())
    };
    core.authenticate(1, proc.read_token());
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| s.authenticated.is_some(),
    );
    assert_eq!(
        shell.authenticated,
        Some(1),
        "lockstep handshake must succeed"
    );

    // --- menu-bar chrome with a reserved zone (T-07 FR-1) ----------------
    let (shell_name, shell_version) = shell.shell_global.expect("df_shell advertised");
    let shell_global = {
        let qh = shell_queue.handle();
        shell
            .registry
            .clone()
            .unwrap()
            .bind::<df_shell::DfShell, _, _>(shell_name, shell_version, &qh, ())
    };
    let chrome_surface = {
        let qh = shell_queue.handle();
        shell.compositor.clone().unwrap().create_surface(&qh, ())
    };
    let layer = shell_global.get_layer_surface(
        &chrome_surface,
        None,
        df_shell::Layer::Top,
        "menubar".to_string(),
        &shell_queue.handle(),
        (),
    );
    layer.set_anchor(1 | 4 | 8); // top | left | right
    layer.set_size(0, MENUBAR_HEIGHT);
    layer.set_exclusive_zone(MENUBAR_HEIGHT);
    layer.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    chrome_surface.commit();
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| !s.layer_configures.is_empty(),
    );
    let (_, bar_w, bar_h) = *shell.layer_configures.last().unwrap();
    assert_eq!(
        (bar_w, bar_h),
        (OUTPUT_W, MENUBAR_HEIGHT),
        "the menu bar must span the output"
    );

    // --- manager replays the scene (T-07 FR-2) ---------------------------
    let (manager_name, manager_version) = shell.manager_global.expect("manager advertised");
    let manager = {
        let qh = shell_queue.handle();
        shell
            .registry
            .clone()
            .unwrap()
            .bind::<df_toplevel_manager::DfToplevelManager, _, _>(
                manager_name,
                manager_version,
                &qh,
                (),
            )
    };
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| !s.outputs.is_empty() && s.workspaces.len() >= 3 && s.done_count > 0,
    );
    assert_eq!(shell.output_names.len(), 1, "one headless output");
    assert_eq!(shell.workspaces.len(), 3, "three initial Spaces");
    assert_eq!(shell.workspace_names.len(), 3);
    assert!(
        shell
            .output_reserved
            .iter()
            .any(|(edge, thickness)| *edge == 0 && *thickness == MENUBAR_HEIGHT as u32),
        "the menu bar's reserved zone must reach the output: {:?}",
        shell.output_reserved
    );

    // --- a Wayland app maps (T-04) ---------------------------------------
    let (app_conn, mut app_queue, mut app) = connect(&proc.socket_path);
    let (wayland_surface, _wayland_file) = map_wayland_toplevel(
        &mut app,
        &mut app_queue,
        "E2E Wayland",
        "org.dragonfruit.E2E",
    );

    // --- an X11 app maps through Xwayland (T-06) -------------------------
    let _x11_conn = x11_display.as_deref().and_then(map_x11_window);

    // --- both windows are announced to the shell with identity -----------
    let expected = if x11_display.is_some() && _x11_conn.is_some() {
        2
    } else {
        1
    };
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(10),
        |s| {
            s.toplevels.len() >= expected
                && s.toplevel_with_title("E2E Wayland").is_some()
                && (expected < 2 || s.toplevel_with_title("E2E X11").is_some())
        },
    );
    let wayland_handle = shell
        .toplevel_with_title("E2E Wayland")
        .expect("the Wayland window must be announced");
    assert!(
        shell.toplevel_app_ids.iter().any(|(id, app_id)| {
            *id == wayland_handle.id().protocol_id()
                && app_id.as_deref() == Some("org.dragonfruit.E2E")
        }),
        "the Wayland handle must carry its app id: {:?}",
        shell.toplevel_app_ids
    );
    if expected == 2 {
        assert!(
            shell.toplevel_with_title("E2E X11").is_some(),
            "the X11 window must be announced to the shell: {:?}",
            shell.toplevel_titles
        );
    }

    // --- Space activation and window move (T-05) -------------------------
    let second_space = shell.workspaces[1].clone();
    shell.workspace_activated.clear();
    second_space.activate();
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| s.workspace_activated.contains(&1),
    );

    shell.workspace_entered.clear();
    wayland_handle.move_to_workspace(&second_space);
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| s.workspace_entered.iter().any(|w| w == &second_space),
    );

    // --- window state machine through the shell (T-04) -------------------
    use df_toplevel::State;
    shell.toplevel_states.clear();
    wayland_handle.zoom();
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| s.toplevel_has_state(&wayland_handle, State::Zoomed.bits()),
    );
    shell.toplevel_states.clear();
    wayland_handle.unzoom();
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| {
            s.toplevel_states
                .iter()
                .any(|(_, flags)| flags & State::Zoomed.bits() == 0)
        },
    );
    shell.toplevel_states.clear();
    wayland_handle.minimize();
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| s.toplevel_has_state(&wayland_handle, State::Minimized.bits()),
    );
    shell.toplevel_states.clear();
    wayland_handle.unminimize();
    wait_for(
        &shell_conn,
        &mut shell_queue,
        &mut shell,
        Duration::from_secs(5),
        |s| {
            s.toplevel_states
                .iter()
                .any(|(_, flags)| flags & State::Minimized.bits() == 0)
        },
    );

    // --- teardown --------------------------------------------------------
    wayland_surface.destroy();
    let _ = app_conn.flush();
    drop(_x11_conn);
    drop(app);
    drop(app_queue);
    layer.destroy();
    manager.destroy();
    shell_global.destroy();
    chrome_surface.destroy();
    let _ = shell_conn.flush();
    drop(shell);
    drop(shell_queue);
    drop(shell_conn);
    proc.shutdown();
}
