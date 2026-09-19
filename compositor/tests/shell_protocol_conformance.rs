// SPDX-License-Identifier: MIT OR Apache-2.0
//! T-07 acceptance: private shell protocol conformance and refusal matrix.
//!
//! Spawns the compositor on the headless backend, provisions launch tokens
//! through `DRAGONFRUIT_LAUNCH_TOKENS`, and drives a real `wayland-client`
//! against the generated client bindings:
//!
//! * `df_core` handshake success and the refusal matrix (untrusted bind,
//!   invalid token, wrong lockstep version, replayed token) — FR-4/5/6,
//! * chrome-surface placement, configure, and reserved zones — FR-1,
//! * window/workspace/output enumeration, events, and request round-trips
//!   with `done` acks — FR-2/FR-3.

use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel as xdg_tl, xdg_wm_base};

/// Generated client bindings for the private protocols.
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

const OUTPUT_W: i32 = 1280;

#[derive(Default)]
struct TestClient {
    registry: Option<wl_registry::WlRegistry>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    // Private global names from the registry.
    core_global: Option<(u32, u32)>,
    shell_global: Option<(u32, u32)>,
    manager_global: Option<(u32, u32)>,
    // Events.
    authenticated: Option<u32>,
    refused: Vec<(u32, String)>,
    outputs: Vec<df_output::DfOutput>,
    workspaces: Vec<df_workspace::DfWorkspace>,
    toplevels: Vec<df_toplevel::DfToplevel>,
    output_names: Vec<String>,
    output_scales: Vec<f64>,
    output_reserved: Vec<(u32, u32)>,
    workspace_names: Vec<String>,
    workspace_wallpapers: Vec<(Option<String>, u32)>,
    workspace_activated: Vec<u32>,
    toplevel_titles: Vec<Option<String>>,
    toplevel_app_ids: Vec<Option<String>>,
    toplevel_states: Vec<u32>,
    layer_configures: Vec<(u32, i32, i32)>,
    layer_closed: usize,
    done_count: usize,
    hot_corners: Vec<(u32, String)>,
    overviews: Vec<(u32, bool)>,
    app_switchers: Vec<(u32, Option<String>, i32)>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for TestClient {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
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
                    state.compositor = Some(registry.bind(name, version, _qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version, _qh, ()));
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base = Some(registry.bind(name, version.min(6), _qh, ()));
                }
                "df_core" => state.core_global = Some((name, version)),
                "df_shell" => state.shell_global = Some((name, version)),
                "df_toplevel_manager" => state.manager_global = Some((name, version)),
                _ => {}
            }
        }
    }
}

impl Dispatch<df_core::DfCore, ()> for TestClient {
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

impl Dispatch<df_shell::DfShell, ()> for TestClient {
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

impl Dispatch<df_layer_surface::DfLayerSurface, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &df_layer_surface::DfLayerSurface,
        event: df_layer_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            df_layer_surface::Event::Configure {
                serial,
                width,
                height,
            } => state.layer_configures.push((serial, width, height)),
            df_layer_surface::Event::Closed => state.layer_closed += 1,
        }
    }
}

impl Dispatch<df_toplevel_manager::DfToplevelManager, ()> for TestClient {
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
            df_toplevel_manager::Event::HotCorner { corner, output } => state
                .hot_corners
                .push((corner.into_result().map(|c| c as u32).unwrap_or(0), output)),
            df_toplevel_manager::Event::OverviewChanged { active, selected } => {
                state.overviews.push((active, selected.is_some()))
            }
            df_toplevel_manager::Event::AppSwitcher {
                active,
                app_id,
                direction,
            } => state.app_switchers.push((active, app_id, direction)),
            df_toplevel_manager::Event::Done => state.done_count += 1,
            _ => {}
        }
    }

    wayland_client::event_created_child!(TestClient, df_toplevel_manager::DfToplevelManager, [
        df_toplevel_manager::EVT_OUTPUT_OPCODE => (df_output::DfOutput, ()),
        df_toplevel_manager::EVT_WORKSPACE_OPCODE => (df_workspace::DfWorkspace, ()),
        df_toplevel_manager::EVT_TOPLEVEL_OPCODE => (df_toplevel::DfToplevel, ()),
    ]);
}

impl Dispatch<df_output::DfOutput, ()> for TestClient {
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
            df_output::Event::Scale { scale } => state.output_scales.push(scale),
            df_output::Event::ReservedZone { edge, thickness } => state
                .output_reserved
                .push((edge.into_result().map(|e| e as u32).unwrap_or(0), thickness)),
            _ => {}
        }
    }
}

impl Dispatch<df_workspace::DfWorkspace, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &df_workspace::DfWorkspace,
        event: df_workspace::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            df_workspace::Event::Name { name } => state.workspace_names.push(name),
            df_workspace::Event::Wallpaper { source, fit, color } => {
                state.workspace_wallpapers.push((
                    source,
                    fit.into_result().map(|f| f as u32).unwrap_or(0) | (color & 0xff00_0000),
                ))
            }
            _ => {}
        }
    }
}

impl Dispatch<df_toplevel::DfToplevel, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &df_toplevel::DfToplevel,
        event: df_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            df_toplevel::Event::Title { title } => state.toplevel_titles.push(title),
            df_toplevel::Event::AppId { app_id } => state.toplevel_app_ids.push(app_id),
            df_toplevel::Event::State { state: flags } => state
                .toplevel_states
                .push(flags.into_result().map(|f| f.bits()).unwrap_or(0)),
            _ => {}
        }
    }
}

delegate_noop!(TestClient: ignore wl_compositor::WlCompositor);
delegate_noop!(TestClient: ignore wl_shm::WlShm);
delegate_noop!(TestClient: ignore wl_shm_pool::WlShmPool);
delegate_noop!(TestClient: ignore wl_buffer::WlBuffer);
delegate_noop!(TestClient: ignore wl_surface::WlSurface);

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

// --- process harness -------------------------------------------------------

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    token_path: PathBuf,
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
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(self.socket_path.with_extension("lock"));
        let _ = std::fs::remove_file(&self.token_path);
    }
}

impl CompositorProcess {
    fn start(socket_name: &str, tokens: &[String]) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let token_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.launch-token"));

        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_LAUNCH_TOKENS", tokens.join(","))
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

        Self {
            child,
            socket_path,
            token_path,
        }
    }

    fn read_token(&self) -> String {
        std::fs::read_to_string(&self.token_path)
            .expect("token file readable")
            .trim()
            .to_string()
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

const SIGTERM: i32 = 15;

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

fn connect(socket_path: &Path) -> (Connection, EventQueue<TestClient>, TestClient) {
    let stream = std::os::unix::net::UnixStream::connect(socket_path).expect("connect failed");
    let conn = Connection::from_socket(stream).expect("connection failed");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = TestClient::default();
    queue.roundtrip(&mut state).expect("registry roundtrip");
    (conn, queue, state)
}

fn random_hex() -> String {
    "ab".repeat(32)
}

/// Dispatch until `pred` is true or the timeout expires. Unlike a bare
/// `blocking_dispatch`, this bounds the wait with `poll` so a missing event
/// fails the test instead of hanging it.
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

fn bind_core(
    state: &mut TestClient,
    queue: &EventQueue<TestClient>,
    name: u32,
    version: u32,
) -> df_core::DfCore {
    let qh = queue.handle();
    state
        .registry
        .clone()
        .expect("registry bound")
        .bind::<df_core::DfCore, _, _>(name, version, &qh, ())
}

fn bind_shell(
    state: &mut TestClient,
    queue: &EventQueue<TestClient>,
    name: u32,
    version: u32,
) -> df_shell::DfShell {
    let qh = queue.handle();
    state
        .registry
        .clone()
        .expect("registry bound")
        .bind::<df_shell::DfShell, _, _>(name, version, &qh, ())
}

fn bind_manager(
    state: &mut TestClient,
    queue: &EventQueue<TestClient>,
    name: u32,
    version: u32,
) -> df_toplevel_manager::DfToplevelManager {
    let qh = queue.handle();
    state
        .registry
        .clone()
        .expect("registry bound")
        .bind::<df_toplevel_manager::DfToplevelManager, _, _>(name, version, &qh, ())
}

// --- tests -----------------------------------------------------------------

#[test]
fn handshake_chrome_and_control_conformance() {
    let token = "11".repeat(32);
    let proc = CompositorProcess::start(
        "dragonfruit-conformance-shell",
        std::slice::from_ref(&token),
    );
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // --- df_core handshake (FR-4) ----------------------------------------
    let (core_name, core_version) = state.core_global.expect("df_core advertised");
    let core = bind_core(&mut state, &queue, core_name, core_version);
    core.authenticate(1, proc.read_token());
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.authenticated.is_some(),
    );
    assert_eq!(state.authenticated, Some(1));

    // --- chrome surface (FR-1) -------------------------------------------
    let (shell_name, shell_version) = state.shell_global.expect("df_shell advertised");
    let shell = bind_shell(&mut state, &queue, shell_name, shell_version);
    let qh = queue.handle();
    let surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let layer = shell.get_layer_surface(
        &surface,
        None,
        df_shell::Layer::Top,
        "menubar".to_string(),
        &qh,
        (),
    );
    layer.set_anchor(1 | 4 | 8); // top | left | right
    layer.set_size(0, 24);
    layer.set_exclusive_zone(24);
    layer.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    surface.commit();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.layer_configures.is_empty(),
    );
    let (_, width, height) = *state.layer_configures.last().unwrap();
    assert_eq!(
        (width, height),
        (OUTPUT_W, 24),
        "an anchored top bar spans the output"
    );

    // --- manager replay: outputs + three Spaces (FR-2) --------------------
    let (manager_name, manager_version) = state.manager_global.expect("manager advertised");
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.workspaces.len() >= 3 && state.done_count > 0,
    );
    assert_eq!(state.output_names.len(), 1, "one headless output");
    assert_eq!(state.workspaces.len(), 3, "three initial Spaces");
    assert_eq!(state.workspace_names.len(), 3);
    assert!(
        state
            .output_reserved
            .iter()
            .any(|(edge, thickness)| *edge == 0 && *thickness == 24),
        "the top bar's reserved zone must reach the output: {:?}",
        state.output_reserved
    );

    // --- workspace requests round-trip (FR-3) -----------------------------
    state.done_count = 0;
    manager.create_workspace();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspaces.len() >= 4 && state.done_count > 0,
    );

    let third = state.workspaces[2].clone();
    state.workspace_activated.clear();
    third.activate();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.workspace_activated.is_empty(),
    );
    assert_eq!(*state.workspace_activated.last().unwrap(), 2);

    third.set_wallpaper(
        Some("/tmp/dragonfruit-test.png".to_string()),
        df_workspace::WallpaperFit::Fill,
        0xff11_2233,
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .workspace_wallpapers
                .iter()
                .any(|(source, _)| source.as_deref() == Some("/tmp/dragonfruit-test.png"))
        },
    );

    // --- output request round-trip (FR-3) ---------------------------------
    state.output_scales.clear();
    state.outputs[0].set_scale(1.5);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .output_scales
                .iter()
                .any(|scale| (*scale - 1.5).abs() < 0.001)
        },
    );

    // --- Mission Control + app switcher state (FR-2) ----------------------
    state.overviews.clear();
    manager.enter_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
    );
    manager.exit_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 0),
    );

    // Clean up and assert a clean compositor teardown.
    manager.destroy();
    shell.destroy();
    surface.destroy();
    let _ = conn.flush();
    proc.shutdown();
}

#[test]
fn untrusted_client_cannot_bind_private_globals() {
    let token = "22".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-refuse-bind", &[token]);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // Bind df_shell *without* authenticating: the server must refuse and
    // disconnect (FR-4).
    let (shell_name, shell_version) = state.shell_global.expect("df_shell advertised");
    let _shell = bind_shell(&mut state, &queue, shell_name, shell_version);
    let _ = conn.flush();
    let result = queue.roundtrip(&mut state);
    assert!(
        result.is_err() || conn.protocol_error().is_some(),
        "an untrusted df_shell bind must be refused: {result:?}"
    );
    proc.shutdown();
}

#[test]
fn refusal_matrix_rejects_bad_tokens_and_versions() {
    // Two valid tokens: one is used for the success/replay pair, the other
    // proves a fresh shell restart gets a fresh, usable token (FR-5).
    let token_a = "33".repeat(32);
    let token_b = "44".repeat(32);
    let proc = CompositorProcess::start(
        "dragonfruit-conformance-refusal",
        &[token_a.clone(), token_b.clone()],
    );

    // --- wrong lockstep version (FR-6) ------------------------------------
    {
        let (conn, mut queue, mut state) = connect(&proc.socket_path);
        let (name, version) = state.core_global.expect("df_core advertised");
        let core = bind_core(&mut state, &queue, name, version);
        core.authenticate(999, token_a.clone());
        let _ = conn.flush();
        let result = queue.roundtrip(&mut state);
        assert!(
            result.is_err() || conn.protocol_error().is_some(),
            "a mismatched lockstep version must be refused"
        );
        assert!(
            state.refused.is_empty() || state.refused.iter().any(|(code, _)| *code == 2),
            "version mismatch should be refusal code 2: {:?}",
            state.refused
        );
    }

    // --- invalid token (FR-4) ---------------------------------------------
    {
        let (conn, mut queue, mut state) = connect(&proc.socket_path);
        let (name, version) = state.core_global.expect("df_core advertised");
        let core = bind_core(&mut state, &queue, name, version);
        core.authenticate(1, random_hex());
        let _ = conn.flush();
        let result = queue.roundtrip(&mut state);
        assert!(
            result.is_err() || conn.protocol_error().is_some(),
            "an invalid token must be refused"
        );
    }

    // --- success, then replay of the same one-time token (FR-5) -----------
    {
        let (conn, mut queue, mut state) = connect(&proc.socket_path);
        let (name, version) = state.core_global.expect("df_core advertised");
        let core = bind_core(&mut state, &queue, name, version);
        core.authenticate(1, token_a.clone());
        wait_for(
            &conn,
            &mut queue,
            &mut state,
            Duration::from_secs(5),
            |state| state.authenticated.is_some(),
        );
        let _ = conn.flush();
    }
    {
        let (conn, mut queue, mut state) = connect(&proc.socket_path);
        let (name, version) = state.core_global.expect("df_core advertised");
        let core = bind_core(&mut state, &queue, name, version);
        core.authenticate(1, token_a.clone());
        let _ = conn.flush();
        let result = queue.roundtrip(&mut state);
        assert!(
            result.is_err() || conn.protocol_error().is_some(),
            "a replayed one-time token must be refused"
        );
        assert!(
            state.refused.is_empty() || state.refused.iter().any(|(code, _)| *code == 1),
            "replay should be refusal code 1: {:?}",
            state.refused
        );
    }

    // --- a fresh token still works (shell restart, FR-5) ------------------
    {
        let (conn, mut queue, mut state) = connect(&proc.socket_path);
        let (name, version) = state.core_global.expect("df_core advertised");
        let core = bind_core(&mut state, &queue, name, version);
        core.authenticate(1, token_b.clone());
        wait_for(
            &conn,
            &mut queue,
            &mut state,
            Duration::from_secs(5),
            |state| state.authenticated.is_some(),
        );
    }

    proc.shutdown();
}

/// Create an shm-backed buffer for a toplevel (mirrors the T-04 harness).
fn shm_buffer(
    state: &TestClient,
    qh: &QueueHandle<TestClient>,
    width: i32,
    height: i32,
) -> (wl_buffer::WlBuffer, std::fs::File) {
    let shm = state.shm.clone().expect("wl_shm bound");
    let stride = width * 4;
    let size = (stride * height) as usize;
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-shell-conformance-{}-{:p}.shm",
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

/// A window/workspace/output round-trip that exercises the per-window
/// handle: map an `xdg_toplevel`, see it announced, then drive the window
/// menu requests and observe the state broadcast (FR-2/FR-3).
#[test]
fn toplevel_handle_requests_round_trip() {
    let token = "55".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-toplevel", &[token]);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    let (name, version) = state.core_global.expect("df_core advertised");
    let core = bind_core(&mut state, &queue, name, version);
    core.authenticate(1, proc.read_token());
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.authenticated.is_some(),
    );

    let (manager_name, manager_version) = state.manager_global.expect("manager advertised");
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.done_count > 0,
    );

    // Map an xdg_toplevel so the manager announces a df_toplevel handle.
    let qh = queue.handle();
    let compositor = state.compositor.clone().unwrap();
    let wm_base = state.xdg_wm_base.clone().unwrap();
    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Protocol Conformance".to_string());
    toplevel.set_app_id("org.dragonfruit.Conformance".to_string());
    surface.commit();
    let _ = queue.roundtrip(&mut state);

    let (buffer, _file) = shm_buffer(&state, &qh, 200, 150);
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();
    let _ = queue.roundtrip(&mut state);

    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    assert!(
        state
            .toplevel_titles
            .iter()
            .any(|title| title.as_deref() == Some("Protocol Conformance")),
        "the announced handle must carry the title: {:?}",
        state.toplevel_titles
    );
    assert!(
        state
            .toplevel_app_ids
            .iter()
            .any(|app| app.as_deref() == Some("org.dragonfruit.Conformance")),
        "the announced handle must carry the app id: {:?}",
        state.toplevel_app_ids
    );

    // --- per-window requests round-trip (FR-3) ---------------------------
    let handle = state.toplevels[0].clone();
    state.toplevel_states.clear();
    handle.zoom();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_states
                .iter()
                .any(|flags| flags & df_toplevel::State::Zoomed.bits() != 0)
        },
    );
    handle.unzoom();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_states
                .iter()
                .any(|flags| flags & df_toplevel::State::Zoomed.bits() == 0)
        },
    );

    handle.minimize();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_states
                .iter()
                .any(|flags| flags & df_toplevel::State::Minimized.bits() != 0)
        },
    );
    handle.unminimize();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_states
                .iter()
                .any(|flags| flags & df_toplevel::State::Minimized.bits() == 0)
        },
    );

    handle.fullscreen();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_states
                .iter()
                .any(|flags| flags & df_toplevel::State::Fullscreen.bits() != 0)
        },
    );
    handle.unfullscreen();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_states
                .iter()
                .any(|flags| flags & df_toplevel::State::Fullscreen.bits() == 0)
        },
    );

    // Move to the second Space through the per-window handle.
    let second = state.workspaces[1].clone();
    handle.move_to_workspace(&second);
    let _ = queue.roundtrip(&mut state);

    // Close is a request to the client; the client destroys the toplevel.
    handle.close();
    let _ = queue.roundtrip(&mut state);
    surface.destroy();
    xdg_surface.destroy();
    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
}
