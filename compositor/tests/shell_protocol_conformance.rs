// SPDX-License-Identifier: MIT
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
    wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_region, wl_registry, wl_seat, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1, zwp_pointer_constraints_v1,
};
use wayland_protocols::xdg::activation::v1::client::{xdg_activation_token_v1, xdg_activation_v1};
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
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    constraints: Option<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1>,
    /// Pointer motion delivered to this client, as surface-local coords.
    pointer_motions: Vec<(f64, f64)>,
    /// Pointer enters delivered to this client.
    pointer_enters: usize,
    /// Surface-local coordinates of each pointer enter.
    pointer_enter_positions: Vec<(f64, f64)>,
    /// Keyboard focus enters/leaves and keycodes delivered to this client.
    keyboard_enters: usize,
    keyboard_leaves: usize,
    keys: Vec<u32>,
    locked: usize,
    confined: usize,
    activation: Option<xdg_activation_v1::XdgActivationV1>,
    activation_token: Option<String>,
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
    output_geometries: Vec<(i32, i32, i32, i32)>,
    output_modes: Vec<(u32, u32, u32)>,
    output_transforms: Vec<u32>,
    workspace_names: Vec<String>,
    workspace_indexes: Vec<u32>,
    workspace_activated_flags: Vec<u32>,
    workspace_fullscreen: Vec<u32>,
    workspace_removed: usize,
    workspace_wallpapers: Vec<(Option<String>, u32)>,
    workspace_activated: Vec<u32>,
    toplevel_titles: Vec<Option<String>>,
    toplevel_app_ids: Vec<Option<String>>,
    toplevel_states: Vec<u32>,
    toplevel_workspace_entered: usize,
    toplevel_workspace_left: usize,
    toplevel_output_entered: usize,
    toplevel_output_left: usize,
    toplevel_closed: usize,
    layer_configures: Vec<(u32, i32, i32)>,
    layer_closed: usize,
    done_count: usize,
    focused: Vec<Option<df_toplevel::DfToplevel>>,
    attentions: Vec<df_toplevel::DfToplevel>,
    hot_corners: Vec<(u32, String)>,
    overviews: Vec<(u32, bool)>,
    app_switchers: Vec<(u32, Option<String>, i32)>,
    input_actions: Vec<(String, String, u32)>,
    progress_events: usize,
    app_accelerators: Vec<(String, String, String, u32)>,
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
                "wl_seat" => {
                    state.seat = Some(registry.bind(name, version.min(9), _qh, ()));
                }
                "xdg_activation_v1" => {
                    state.activation = Some(registry.bind(name, version.min(1), _qh, ()));
                }
                "zwp_pointer_constraints_v1" => {
                    state.constraints = Some(registry.bind(name, version.min(1), _qh, ()));
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
            df_toplevel_manager::Event::Focused { toplevel } => state.focused.push(toplevel),
            df_toplevel_manager::Event::Attention { toplevel } => state.attentions.push(toplevel),
            df_toplevel_manager::Event::InputAction {
                action,
                source,
                serial,
            } => state.input_actions.push((action, source, serial)),
            df_toplevel_manager::Event::Progress { .. } => state.progress_events += 1,
            df_toplevel_manager::Event::AppAccelerator {
                app_id,
                accelerator_id,
                source,
                serial,
            } => state
                .app_accelerators
                .push((app_id, accelerator_id, source, serial)),
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
            df_output::Event::Geometry {
                x,
                y,
                width,
                height,
            } => state.output_geometries.push((x, y, width, height)),
            df_output::Event::Mode {
                width,
                height,
                refresh,
                ..
            } => state.output_modes.push((width, height, refresh)),
            df_output::Event::Transform { transform } => state.output_transforms.push(
                transform
                    .into_result()
                    .map(|t| t as u32)
                    .unwrap_or(u32::MAX),
            ),
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
            df_workspace::Event::Index { index } => state.workspace_indexes.push(index),
            df_workspace::Event::Activated { active } => {
                state.workspace_activated_flags.push(active)
            }
            df_workspace::Event::Fullscreen { fullscreen } => {
                state.workspace_fullscreen.push(fullscreen)
            }
            df_workspace::Event::Removed => state.workspace_removed += 1,
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
            df_toplevel::Event::WorkspaceEntered { .. } => state.toplevel_workspace_entered += 1,
            df_toplevel::Event::WorkspaceLeft { .. } => state.toplevel_workspace_left += 1,
            df_toplevel::Event::OutputEntered { .. } => state.toplevel_output_entered += 1,
            df_toplevel::Event::OutputLeft { .. } => state.toplevel_output_left += 1,
            df_toplevel::Event::Closed => state.toplevel_closed += 1,
            _ => {}
        }
    }
}

delegate_noop!(TestClient: ignore wl_compositor::WlCompositor);
delegate_noop!(TestClient: ignore wl_shm::WlShm);
delegate_noop!(TestClient: ignore wl_shm_pool::WlShmPool);
delegate_noop!(TestClient: ignore wl_buffer::WlBuffer);
delegate_noop!(TestClient: ignore wl_surface::WlSurface);
delegate_noop!(TestClient: ignore wl_region::WlRegion);
delegate_noop!(TestClient: ignore zwp_pointer_constraints_v1::ZwpPointerConstraintsV1);

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
            let caps = capabilities.into_result();
            let has_pointer = caps
                .as_ref()
                .map(|caps| caps.contains(wl_seat::Capability::Pointer))
                .unwrap_or(false);
            let has_keyboard = caps
                .as_ref()
                .map(|caps| caps.contains(wl_seat::Capability::Keyboard))
                .unwrap_or(false);
            if has_pointer && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
            if has_keyboard && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
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
        if let wl_pointer::Event::Enter {
            surface_x,
            surface_y,
            ..
        } = event
        {
            state.pointer_enters += 1;
            state.pointer_enter_positions.push((surface_x, surface_y));
        }
        if let wl_pointer::Event::Motion {
            surface_x,
            surface_y,
            ..
        } = event
        {
            state.pointer_motions.push((surface_x, surface_y));
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { .. } => state.keyboard_enters += 1,
            wl_keyboard::Event::Leave { .. } => state.keyboard_leaves += 1,
            wl_keyboard::Event::Key { key, .. } => state.keys.push(key),
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
        if let zwp_locked_pointer_v1::Event::Locked = event {
            state.locked += 1;
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
        if let zwp_confined_pointer_v1::Event::Confined = event {
            state.confined += 1;
        }
    }
}

impl Dispatch<xdg_activation_v1::XdgActivationV1, ()> for TestClient {
    fn event(
        _: &mut Self,
        _: &xdg_activation_v1::XdgActivationV1,
        event: xdg_activation_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let _ = event;
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

// --- process harness -------------------------------------------------------

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    token_path: PathBuf,
    stderr_path: PathBuf,
    synthetic_path: Option<PathBuf>,
    synthetic_output_path: Option<PathBuf>,
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
        // The compositor log is the only clue when it panics mid-test;
        // surface it on failure (the T-07 malformed-traffic suite found a
        // real mode-size panic this way).
        if std::thread::panicking() {
            if let Ok(log) = std::fs::read_to_string(&self.stderr_path) {
                eprintln!("--- compositor stderr ---\n{log}--- end ---");
            }
        }
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(self.socket_path.with_extension("lock"));
        let _ = std::fs::remove_file(&self.token_path);
        let _ = std::fs::remove_file(&self.stderr_path);
        if let Some(path) = &self.synthetic_path {
            let _ = std::fs::remove_file(path);
        }
        if let Some(path) = &self.synthetic_output_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl CompositorProcess {
    fn start(socket_name: &str, tokens: &[String]) -> Self {
        Self::start_with_harnesses(socket_name, tokens, None, None)
    }

    /// Start with the T-03 synthetic-input harness bound at
    /// `synthetic_path` (`DRAGONFRUIT_SYNTHETIC_INPUT`).
    fn start_with_synthetic(
        socket_name: &str,
        tokens: &[String],
        synthetic_path: Option<&Path>,
    ) -> Self {
        Self::start_with_harnesses(socket_name, tokens, synthetic_path, None)
    }

    /// Start with the opt-in headless test harnesses bound: synthetic input
    /// (`DRAGONFRUIT_SYNTHETIC_INPUT`) and synthetic output hotplug
    /// (`DRAGONFRUIT_SYNTHETIC_OUTPUT`).
    fn start_with_harnesses(
        socket_name: &str,
        tokens: &[String],
        synthetic_path: Option<&Path>,
        synthetic_output_path: Option<&Path>,
    ) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let token_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.launch-token"));
        let stderr_path = std::env::temp_dir().join(format!("{socket_name}.stderr"));
        let stderr_file = std::fs::File::create(&stderr_path).expect("create stderr log");

        let mut command = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"));
        command
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_LAUNCH_TOKENS", tokens.join(","))
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_file));
        if let Some(path) = synthetic_path {
            command.env("DRAGONFRUIT_SYNTHETIC_INPUT", path);
        }
        if let Some(path) = synthetic_output_path {
            command.env("DRAGONFRUIT_SYNTHETIC_OUTPUT", path);
        }
        let mut child = command.spawn().expect("failed to start compositor");

        let deadline = Instant::now() + Duration::from_secs(10);
        while !socket_path.exists() || !token_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before ready: {status}");
            }
            assert!(Instant::now() < deadline, "compositor never became ready");
            std::thread::sleep(Duration::from_millis(10));
        }
        for (path, label) in [
            (synthetic_path, "synthetic-input"),
            (synthetic_output_path, "synthetic-output"),
        ] {
            if let Some(path) = path {
                while !path.exists() {
                    assert!(Instant::now() < deadline, "{label} socket never appeared");
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }

        Self {
            child,
            socket_path,
            token_path,
            stderr_path,
            synthetic_path: synthetic_path.map(Path::to_path_buf),
            synthetic_output_path: synthetic_output_path.map(Path::to_path_buf),
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
/// fails the test instead of hanging it. `#[track_caller]` makes the
/// timeout panic point at the waiting call site.
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
fn dock_surface_reserves_the_bottom_zone_and_coexists_with_the_bar() {
    let token = "1a".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-dock", &[token]);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // Authenticate and bind the manager first so we observe reserved zones.
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
    let _manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.done_count > 0,
    );

    let (shell_name, shell_version) = state.shell_global.expect("df_shell advertised");
    let shell = bind_shell(&mut state, &queue, shell_name, shell_version);
    let qh = queue.handle();

    // A menu bar reserves the top edge; a Dock reserves the bottom edge. Both
    // must reach the output independently (T-10 FR-3 reserved zones).
    let bar_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let bar = shell.get_layer_surface(
        &bar_surface,
        None,
        df_shell::Layer::Top,
        "menubar".to_string(),
        &qh,
        (),
    );
    bar.set_anchor(1 | 4 | 8); // top | left | right
    bar.set_size(0, 28);
    bar.set_exclusive_zone(28);
    bar.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::OnDemand);
    bar_surface.commit();

    let dock_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let dock = shell.get_layer_surface(
        &dock_surface,
        None,
        df_shell::Layer::Top,
        "dock".to_string(),
        &qh,
        (),
    );
    dock.set_anchor(2 | 4 | 8); // bottom | left | right
    dock.set_size(0, 95);
    dock.set_exclusive_zone(60);
    dock.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    dock_surface.commit();

    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved
                .iter()
                .any(|(edge, thickness)| *edge == 0 && *thickness == 28)
                && state
                    .output_reserved
                    .iter()
                    .any(|(edge, thickness)| *edge == 1 && *thickness == 60)
        },
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures
                .iter()
                .any(|(_, width, height)| *width == OUTPUT_W && *height == 95)
        },
    );

    drop(dock);
    drop(dock_surface);
    drop(bar);
    drop(bar_surface);
    drop(shell);
    drop(core);
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
    // A unique name per buffer: a test may map several windows and the
    // backing file is created with `create_new`.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SHM_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SHM_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-shell-conformance-{}-{seq}-{:p}.shm",
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

/// Map a toplevel and return its proxies plus the backing file (which must
/// outlive the first flush).
#[allow(clippy::type_complexity)]
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
    let compositor = state.compositor.clone().unwrap();
    let wm_base = state.xdg_wm_base.clone().unwrap();
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

/// T-07 compliance: assert every event pair the client can trigger without
/// a seat — output geometry/mode/transform, workspace index/activated/
/// fullscreen/removed, focus, attention (`xdg-activation`), app-switcher
/// state, and toplevel output/workspace/closed transitions.
///
/// The `input_action`/`progress`/`hot_corner`/`app_accelerator` events are
/// emitted from the T-03 input outbox, which needs injected input (the
/// synthetic-seat harness is still open); they are exercised by the T-03
/// unit matrix instead.
#[test]
fn event_coverage_conformance() {
    let token = "66".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-events", &[token]);
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
        |state| !state.outputs.is_empty() && state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // --- output properties (FR-2) ----------------------------------------
    assert!(
        !state.output_geometries.is_empty(),
        "the output must report its geometry"
    );
    assert!(
        !state.output_modes.is_empty(),
        "the output must report its mode"
    );
    assert!(
        !state.output_transforms.is_empty(),
        "the output must report its transform"
    );
    assert!(
        state.output_transforms.iter().all(|t| *t != u32::MAX),
        "the transform enum must decode: {:?}",
        state.output_transforms
    );

    // --- workspace index/activated (FR-2) --------------------------------
    assert!(
        state.workspace_indexes.contains(&0),
        "the first Space must report index 0: {:?}",
        state.workspace_indexes
    );
    assert!(
        state.workspace_activated_flags.contains(&1),
        "exactly one Space per output must be active: {:?}",
        state.workspace_activated_flags
    );

    // --- map a window ----------------------------------------------------
    let (surface, xdg_surface, toplevel, _file) = map_toplevel(
        &mut state,
        &mut queue,
        "Coverage",
        "org.dragonfruit.Coverage",
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    assert!(
        state.toplevel_output_entered >= 1,
        "a mapped window must report its output"
    );
    assert!(
        state.toplevel_workspace_entered >= 1,
        "a mapped window must report its Space"
    );

    let handle = state.toplevels[0].clone();

    // --- focus (FR-4) ----------------------------------------------------
    state.focused.clear();
    handle.activate();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.iter().any(|focused| focused.is_some()),
    );

    // --- attention via xdg-activation (FR-2) -----------------------------
    state.attentions.clear();
    let activation = state
        .activation
        .clone()
        .expect("xdg_activation_v1 advertised");
    let activation_token = activation.get_activation_token(&queue.handle(), ());
    activation_token.set_app_id("org.dragonfruit.Coverage".to_string());
    activation_token.commit();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.activation_token.is_some(),
    );
    let token_string = state
        .activation_token
        .clone()
        .expect("activation token generated");
    activation.activate(token_string, &surface);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .attentions
                .iter()
                .any(|handle| handle == &state.toplevels[0])
        },
    );

    // --- fullscreen creates a dedicated Space; unfullscreen removes it ---
    state.workspace_fullscreen.clear();
    state.workspace_removed = 0;
    handle.fullscreen();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_fullscreen.contains(&1),
    );
    handle.unfullscreen();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_removed >= 1,
    );

    // --- create + remove a Space -----------------------------------------
    let removed_before = state.workspace_removed;
    let before = state.workspaces.len();
    manager.create_workspace();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspaces.len() > before,
    );
    let extra = state.workspaces[before].clone();
    manager.remove_workspace(&extra);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_removed > removed_before,
    );

    // --- app switcher (FR-2) ---------------------------------------------
    state.app_switchers.clear();
    manager.cycle_app_switcher(1);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.app_switchers.iter().any(|(active, ..)| *active == 1),
    );

    // --- close/unmap -----------------------------------------------------
    state.toplevel_closed = 0;
    toplevel.destroy();
    surface.destroy();
    xdg_surface.destroy();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_closed >= 1,
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
}

/// T-07 test-plan requirement: bad private-protocol traffic must never
/// crash the compositor. Send already-authenticated handshakes, extreme
/// output values, out-of-range reorders, requests against stale workspace
/// and toplevel handles, then prove the session is still alive with a
/// fresh round-trip.
#[test]
fn malformed_private_traffic_never_crashes() {
    let token = "77".repeat(32);
    let proc = CompositorProcess::start(
        "dragonfruit-conformance-malformed",
        std::slice::from_ref(&token),
    );
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

    // Re-authenticate: refusal code 4 (already-authenticated), no disconnect.
    state.refused.clear();
    core.authenticate(1, token);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.refused.iter().any(|(code, _)| *code == 4),
    );
    assert!(
        state.authenticated.is_some(),
        "an already-authenticated refusal must not disconnect the trusted client"
    );

    let (manager_name, manager_version) = state.manager_global.expect("manager advertised");
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // --- extreme output values -------------------------------------------
    let output = state.outputs[0].clone();
    output.set_scale(f64::NAN);
    output.set_scale(0.0);
    output.set_scale(-4.0);
    output.set_mode(0, 0, 0);
    output.set_mode(u32::MAX, u32::MAX, u32::MAX);
    output.set_transform(df_output::Transform::_270);
    output.set_vrr(1);
    output.set_night_light(1, 100);
    let _ = queue.roundtrip(&mut state);

    // --- out-of-range reorder, then requests against a stale workspace ----
    let first = state.workspaces[0].clone();
    manager.reorder_workspace(&first, u32::MAX);
    let third = state.workspaces[2].clone();
    manager.remove_workspace(&third);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_removed >= 1,
    );
    third.activate();
    third.set_wallpaper(None, df_workspace::WallpaperFit::Fill, 0);
    let _ = queue.roundtrip(&mut state);

    // --- requests against a stale toplevel handle ------------------------
    let (surface, xdg_surface, toplevel, _file) = map_toplevel(
        &mut state,
        &mut queue,
        "Malformed",
        "org.dragonfruit.Malformed",
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    let handle = state.toplevels[0].clone();
    toplevel.destroy();
    surface.destroy();
    xdg_surface.destroy();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_closed >= 1,
    );
    handle.zoom();
    handle.activate();
    handle.minimize();
    handle.move_to_workspace(&first);
    let _ = queue.roundtrip(&mut state);

    // --- still alive: a fresh request round-trips ------------------------
    state.done_count = 0;
    manager.create_workspace();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.done_count > 0,
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
}

/// Sends synthetic-input commands to the compositor's T-03 harness socket
/// (see `compositor/src/input/synthetic.rs` for the wire format).
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

/// Client end of the T-09 synthetic-output harness
/// (`DRAGONFRUIT_SYNTHETIC_OUTPUT`): drives headless output hotplug.
struct SyntheticOutput {
    socket: std::os::unix::net::UnixDatagram,
    path: PathBuf,
}

impl SyntheticOutput {
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
            .unwrap_or_else(|err| panic!("failed to send synthetic output {command:?}: {err}"));
    }
}

/// T-03 acceptance (integration, headless): synthetic libinput-equivalent
/// events drive the seat through the exact same router as a real device.
/// A keyboard shortcut, a hot-corner dwell, and a four-finger gesture each
/// reach the private shell protocol as the same compositor action, and a
/// synthetic pointer click focuses a real mapped window. This also closes
/// the `input_action`/`hot_corner`/`progress` events the T-07 compliance
/// client could not previously trigger.
#[test]
fn synthetic_input_drives_shortcuts_hot_corners_and_gestures() {
    let token = "88".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-synthetic",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
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
        |state| !state.outputs.is_empty() && state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // --- keyboard: Ctrl+Right is the system WorkspaceNext shortcut -------
    state.input_actions.clear();
    state.workspace_activated.clear();
    // evdev codes: KEY_LEFTCTRL=29, KEY_RIGHT=106.
    input.send("key 29 down\nkey 106 down\nkey 106 up\nkey 29 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .input_actions
                .iter()
                .any(|(action, source, _)| action == "workspace-next" && source == "keyboard")
        },
    );
    // The shortcut really switched the Space, not just logged an action.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_activated.contains(&1),
    );

    // --- hot corner: dwell in the top-left fires Mission Control ---------
    state.hot_corners.clear();
    state.input_actions.clear();
    input.send("motion-abs 0.001 0.001");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.hot_corners.is_empty(),
    );
    // TopLeft == 0 in the `df_toplevel_manager.hot_corner` enum.
    assert_eq!(state.hot_corners[0].0, 0, "top-left corner fired");
    assert!(
        state
            .input_actions
            .iter()
            .any(|(action, source, _)| action == "mission-control" && source == "hot-corner"),
        "the hot corner must dispatch through the same outbox: {:?}",
        state.input_actions
    );

    // --- gesture: four-finger vertical swipe drives shared progress ------
    state.input_actions.clear();
    state.progress_events = 0;
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    input.send("swipe-update 0 -100");
    input.send("swipe-end");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .input_actions
                .iter()
                .any(|(action, source, _)| action == "mission-control" && source == "gesture")
        },
    );
    assert!(
        state.progress_events > 0,
        "a gesture must emit shared progress events"
    );

    // --- pointer: a synthetic click focuses a real mapped window ---------
    let (_surface, _xdg_surface, _toplevel, _file) = map_toplevel(
        &mut state,
        &mut queue,
        "Synthetic",
        "org.dragonfruit.Synthetic",
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    state.focused.clear();
    // The first window is centered on the 1280x720 output; (0.5, 0.5) is
    // its middle. BTN_LEFT == 0x110 == 272.
    input.send("motion-abs 0.5 0.5");
    input.send("button 272 down\nbutton 272 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.iter().any(|focused| focused.is_some()),
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-03 FR-5 (sanctioned half): the shell authenticates with a launch token
/// and is then allowed to install a pointer grab. Its locked pointer stays
/// frozen even as synthetic motion arrives.
#[test]
fn sanctioned_client_can_lock_the_pointer() {
    let token = "99".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-constraint-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-constraint",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
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

    let (surface, _xdg_surface, _toplevel, _file) = map_toplevel(
        &mut state,
        &mut queue,
        "Constraint",
        "org.dragonfruit.Constraint",
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some(),
    );
    // Make sure the compositor has processed `wl_seat.get_pointer` before
    // synthetic motion is aimed at the window.
    let _ = queue.roundtrip(&mut state);

    // Click to focus, then lock the pointer.
    input.send("motion-abs 0.5 0.5");
    input.send("button 272 down\nbutton 272 up");
    // The first motion only sends `wl_pointer.enter`; a follow-up relative
    // motion produces the baseline `wl_pointer.motion` we compare against.
    input.send("motion 1 1");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.pointer_motions.is_empty(),
    );

    let constraints = state
        .constraints
        .clone()
        .expect("zwp_pointer_constraints_v1 advertised");
    let pointer = state.pointer.clone().expect("wl_pointer bound");
    let _locked = constraints.lock_pointer(
        &surface,
        &pointer,
        None,
        zwp_pointer_constraints_v1::Lifetime::Persistent,
        &queue.handle(),
        (),
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.locked >= 1,
    );

    // The pointer must freeze: motion keeps arriving but stays put.
    let before = state.pointer_motions.last().copied();
    let before_len = state.pointer_motions.len();
    input.send("motion 80 60");
    input.send("motion 80 60");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_motions.len() > before_len,
    );
    assert_eq!(
        before,
        state.pointer_motions.last().copied(),
        "a locked pointer must not move"
    );

    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-03 FR-5 (malicious half): an ordinary, unsanctioned client requests the
/// same pointer lock and is refused. It receives no `locked` event and the
/// pointer keeps moving.
#[test]
fn unsanctioned_pointer_constraint_is_refused() {
    let token = "aa".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-constraint-refused-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-constraint-refused",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // No authentication: this is an ordinary application client.
    let (surface, _xdg_surface, _toplevel, _file) =
        map_toplevel(&mut state, &mut queue, "Refused", "org.dragonfruit.Refused");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some(),
    );
    // Make sure the compositor has processed `wl_seat.get_pointer` before
    // synthetic motion is aimed at the window.
    let _ = queue.roundtrip(&mut state);

    input.send("motion-abs 0.5 0.5");
    input.send("button 272 down\nbutton 272 up");
    // The first motion only sends `wl_pointer.enter`; a follow-up relative
    // motion produces the baseline `wl_pointer.motion` we compare against.
    input.send("motion 1 1");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.pointer_motions.is_empty(),
    );

    let constraints = state
        .constraints
        .clone()
        .expect("zwp_pointer_constraints_v1 advertised");
    let pointer = state.pointer.clone().expect("wl_pointer bound");
    let _locked = constraints.lock_pointer(
        &surface,
        &pointer,
        None,
        zwp_pointer_constraints_v1::Lifetime::Persistent,
        &queue.handle(),
        (),
    );
    // Order the request before the synthetic motion on the same loop.
    let _ = queue.roundtrip(&mut state);

    let before = state.pointer_motions.last().copied();
    let before_len = state.pointer_motions.len();
    input.send("motion 80 60");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_motions.len() > before_len,
    );
    assert_ne!(
        before,
        state.pointer_motions.last().copied(),
        "an unsanctioned lock must not freeze the pointer"
    );
    assert_eq!(
        state.locked, 0,
        "no locked event for an unsanctioned client"
    );

    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-04 FR-7: a window with an empty input region passes clicks through to
/// the window beneath it. The synthetic pointer aims at the overlap point;
/// with a normal region the top window focuses, and after clearing its
/// input region the click reaches the window underneath.
#[test]
fn empty_input_region_passes_clicks_through() {
    let token = "bb".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-input-region-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-input-region",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
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
        |state| state.pointer.is_some() && !state.outputs.is_empty(),
    );
    let _ = queue.roundtrip(&mut state);

    // Window A: the fallback target. Click it so focus starts somewhere.
    let (_surface_a, _xdg_a, _toplevel_a, _file_a) =
        map_toplevel(&mut state, &mut queue, "A", "org.dragonfruit.A");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    let handle_a = state.toplevels[0].clone();
    input.send("motion-abs 0.5 0.5");
    input.send("button 272 down\nbutton 272 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.last().cloned().flatten() == Some(handle_a.clone()),
    );

    // Window B: cascaded +24px, so (0.519, 0.533) is inside both windows.
    let (surface_b, _xdg_b, _toplevel_b, _file_b) =
        map_toplevel(&mut state, &mut queue, "B", "org.dragonfruit.B");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevels.len() >= 2,
    );
    let handle_b = state.toplevels[1].clone();
    input.send("motion-abs 0.519 0.533");
    input.send("button 272 down\nbutton 272 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.last().cloned().flatten() == Some(handle_b.clone()),
    );

    // Clear B's input region: the same click must now reach A.
    let compositor = state.compositor.clone().expect("wl_compositor bound");
    let region = compositor.create_region(&queue.handle(), ());
    surface_b.set_input_region(Some(&region));
    surface_b.commit();
    let _ = queue.roundtrip(&mut state);

    input.send("motion-abs 0.519 0.533");
    input.send("button 272 down\nbutton 272 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.last().cloned().flatten() == Some(handle_a.clone()),
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// Authenticate a trusted shell client and create its menu-bar chrome
/// surface, returning every proxy the caller must keep alive for the
/// surface to stay mapped.
#[allow(clippy::type_complexity)]
fn open_trusted_bar(
    conn: &Connection,
    queue: &mut EventQueue<TestClient>,
    state: &mut TestClient,
    token: &str,
    height: i32,
    exclusive_zone: i32,
) -> (
    df_core::DfCore,
    df_shell::DfShell,
    wl_surface::WlSurface,
    df_layer_surface::DfLayerSurface,
) {
    let (name, version) = state.core_global.expect("df_core advertised");
    let core = bind_core(state, queue, name, version);
    core.authenticate(1, token.to_string());
    wait_for(conn, queue, state, Duration::from_secs(5), |state| {
        state.authenticated.is_some()
    });
    let (shell_name, shell_version) = state.shell_global.expect("df_shell advertised");
    let shell = bind_shell(state, queue, shell_name, shell_version);
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
    layer.set_size(0, height);
    layer.set_exclusive_zone(exclusive_zone);
    layer.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::OnDemand);
    surface.commit();
    wait_for(conn, queue, state, Duration::from_secs(5), |state| {
        !state.layer_configures.is_empty()
    });
    (core, shell, surface, layer)
}

/// T-09 FR-5/FR-9 (the Phase-1 exit test reused): the shell is crashable and
/// restartable without disturbing the compositor's window state. A window is
/// mapped by an independent client; a first trusted shell reserves the
/// menu-bar zone; the shell connection is dropped (a crash) and the zone
/// clears while the window is untouched; a second shell with a *fresh*
/// one-time token reconnects and the bar returns at the right size.
/// `DRAGONFRUIT_LAUNCH_TOKENS` provisions both tokens up front, standing in
/// for the session manager's per-start mint (T-24).
#[test]
fn shell_restart_reanchors_chrome_and_preserves_windows() {
    let tokens = ["aa".repeat(32), "bb".repeat(32), "cc".repeat(32)];
    let proc = CompositorProcess::start("dragonfruit-conformance-restart", &tokens);

    // An observer stays connected across both shell lifetimes and watches the
    // chrome reserved zone and the window list through the private manager.
    let (obs_conn, mut obs_queue, mut obs) = connect(&proc.socket_path);
    let (name, version) = obs.core_global.expect("df_core advertised");
    let obs_core = bind_core(&mut obs, &obs_queue, name, version);
    obs_core.authenticate(1, tokens[2].clone());
    wait_for(
        &obs_conn,
        &mut obs_queue,
        &mut obs,
        Duration::from_secs(5),
        |state| state.authenticated.is_some(),
    );
    let (manager_name, manager_version) = obs.manager_global.expect("manager advertised");
    let obs_manager = bind_manager(&mut obs, &obs_queue, manager_name, manager_version);
    wait_for(
        &obs_conn,
        &mut obs_queue,
        &mut obs,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // An ordinary application maps a window and stays alive across the
    // restart; the window state is compositor-owned.
    let (app_conn, mut app_queue, mut app) = connect(&proc.socket_path);
    let (_app_surface, _app_xdg, _app_toplevel, _app_file) = map_toplevel(
        &mut app,
        &mut app_queue,
        "Restart Survivor",
        "org.dragonfruit.Restart",
    );
    wait_for(
        &obs_conn,
        &mut obs_queue,
        &mut obs,
        Duration::from_secs(5),
        |state| state.toplevels.len() == 1,
    );
    assert!(
        obs.toplevel_titles
            .iter()
            .any(|title| title.as_deref() == Some("Restart Survivor")),
        "the mapped window must be announced: {:?}",
        obs.toplevel_titles
    );

    // --- shell #1: authenticate and reserve the menu-bar zone -------------
    let (s1_conn, mut s1_queue, mut s1) = connect(&proc.socket_path);
    obs.output_reserved.clear();
    let (s1_core, s1_shell, s1_surface, s1_layer) =
        open_trusted_bar(&s1_conn, &mut s1_queue, &mut s1, &tokens[0], 28, 28);
    wait_for(
        &obs_conn,
        &mut obs_queue,
        &mut obs,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved
                .iter()
                .any(|(edge, thickness)| *edge == 0 && *thickness == 28)
        },
    );
    assert_eq!(obs.toplevels.len(), 1, "the window must be announced");
    assert_eq!(obs.toplevel_closed, 0, "the window must not be closed");

    // --- crash the shell: drop the connection without a clean destroy -----
    obs.output_reserved.clear();
    drop(s1_layer);
    drop(s1_surface);
    drop(s1_shell);
    drop(s1_core);
    drop(s1_conn);
    drop(s1_queue);
    drop(s1);

    // The chrome zone clears and the window is untouched.
    wait_for(
        &obs_conn,
        &mut obs_queue,
        &mut obs,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved
                .iter()
                .any(|(edge, thickness)| *edge == 0 && *thickness == 0)
        },
    );
    assert_eq!(
        obs.toplevels.len(),
        1,
        "the window survives the shell crash"
    );
    assert_eq!(obs.toplevel_closed, 0, "the window must not be closed");

    // --- shell #2: a fresh token reconnects; the bar returns --------------
    let (s2_conn, mut s2_queue, mut s2) = connect(&proc.socket_path);
    obs.output_reserved.clear();
    let (s2_core, s2_shell, s2_surface, s2_layer) =
        open_trusted_bar(&s2_conn, &mut s2_queue, &mut s2, &tokens[1], 28, 28);
    wait_for(
        &s2_conn,
        &mut s2_queue,
        &mut s2,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures
                .iter()
                .any(|(_, width, height)| *width == OUTPUT_W && *height == 28)
        },
    );
    wait_for(
        &obs_conn,
        &mut obs_queue,
        &mut obs,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved
                .iter()
                .any(|(edge, thickness)| *edge == 0 && *thickness == 28)
        },
    );
    assert_eq!(
        obs.toplevels.len(),
        1,
        "the window is still there after the restart"
    );
    assert_eq!(obs.toplevel_closed, 0, "the window must never be closed");
    assert!(
        obs.toplevel_titles
            .iter()
            .any(|title| title.as_deref() == Some("Restart Survivor")),
        "the restarted shell must see the same window: {:?}",
        obs.toplevel_titles
    );

    // The restarted shell got the real bar size, not the pre-layout output.
    let (_, width, height) = *s2.layer_configures.last().unwrap();
    assert_eq!((width, height), (OUTPUT_W, 28));

    drop(s2_layer);
    drop(s2_surface);
    drop(s2_shell);
    drop(s2_core);
    drop(obs_manager);
    drop(obs_core);
    let _ = app_conn.flush();
    let _ = obs_conn.flush();
    proc.shutdown();
}

/// T-09 FR-1: the menu bar follows output hotplug. The shell creates its
/// chrome surface without an explicit output (matching `wlr-layer-shell`,
/// `None` targets every display), so attaching an output must announce it
/// through the manager, give it its own three Spaces, re-send the bar's
/// reserved zone to the new output, and reconfigure the chrome; detaching
/// must remove its Spaces and migrate its windows. The headless backend's
/// synthetic-output harness makes the hotplug scriptable.
#[test]
fn shell_output_hotplug_reanchors_chrome() {
    let token = "dd".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let output_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-hotplug-{}", std::process::id()));
    let proc = CompositorProcess::start_with_harnesses(
        "dragonfruit-conformance-hotplug",
        std::slice::from_ref(&token),
        None,
        Some(&output_path),
    );
    let outputs = SyntheticOutput::connect(&output_path);

    // One trusted client is both the shell (creates the bar) and the
    // observer (binds the manager to watch outputs/workspaces).
    let (conn, mut queue, mut state) = connect(&proc.socket_path);
    let (core, _shell, _surface, _layer) =
        open_trusted_bar(&conn, &mut queue, &mut state, &token, 28, 28);
    let (manager_name, manager_version) = state.manager_global.expect("manager advertised");
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspaces.len() >= 3 && state.done_count > 0,
    );
    assert_eq!(
        state.workspaces.len(),
        3,
        "the one static headless output starts with three Spaces"
    );
    assert!(
        state.output_names.iter().any(|name| name == "HEADLESS-1"),
        "the static output is announced: {:?}",
        state.output_names
    );

    // --- attach a second output -------------------------------------------
    let configures_before = state.layer_configures.len();
    state.output_names.clear();
    state.workspace_removed = 0;
    outputs.send("add HDMI-A-1 1920 1080 1280 0");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.output_names.iter().any(|name| name == "HDMI-A-1"),
    );
    // The new output gets its own three Spaces and its geometry.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspaces.len() >= 6,
    );
    assert!(
        state.output_geometries.contains(&(1280, 0, 1920, 1080)),
        "the hotplugged output's geometry is announced: {:?}",
        state.output_geometries
    );
    // The bar's exclusive zone is reported to the new output, which is how a
    // chrome surface that targets every output surfaces its reserve.
    assert!(
        state
            .output_reserved
            .iter()
            .any(|(edge, thickness)| *edge == 0 && *thickness == 28),
        "the menu bar reserves its zone on the hotplugged output: {:?}",
        state.output_reserved
    );
    // Re-anchoring reconfigures the chrome surface for the new scene.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.layer_configures.len() > configures_before,
    );

    // --- detach it again ---------------------------------------------------
    state.workspace_removed = 0;
    outputs.send("remove HDMI-A-1");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_removed >= 3,
    );

    drop(manager);
    drop(core);
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !output_path.exists(),
        "teardown leak: synthetic-output socket survived"
    );
}

/// T-09 input routing (compositor half): a mapped chrome surface is hit-tested
/// above the window space, receives pointer enter/motion/button, and an
/// `OnDemand` surface takes keyboard focus on click so key events reach it.
/// This is what makes the menu bar (and, with T-09b, the dropdown) interactive.
#[test]
fn chrome_surface_receives_pointer_and_keyboard() {
    let token = "dd".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-chrome-input-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-chrome-input",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
    let (conn, mut queue, mut state) = connect(&proc.socket_path);

    // Authenticate and map a real menu-bar chrome surface.
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
    layer.set_size(0, 28);
    layer.set_exclusive_zone(28);
    layer.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::OnDemand);
    surface.commit();
    let (buffer, _file) = shm_buffer(&state, &qh, OUTPUT_W, 28);
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures
                .iter()
                .any(|(_, width, height)| *width == OUTPUT_W && *height == 28)
        },
    );

    // The seat must have delivered its capabilities before synthetic input.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some() && state.keyboard.is_some(),
    );
    let _ = queue.roundtrip(&mut state);

    // Move over the bar (the top 28 px of the 1280x720 output).
    input.send("motion-abs 0.5 0.01");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );

    // Click the bar: an OnDemand chrome surface takes keyboard focus.
    input.send("button 272 down\nbutton 272 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.keyboard_enters > 0,
    );

    // A key reaches the focused chrome surface (KEY_ESC == 1).
    state.keys.clear();
    input.send("key 1 down\nkey 1 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.keys.contains(&1),
    );

    // Clicking empty desktop space drops the chrome's keyboard focus so the
    // shell sees `shellFocused=false` and closes an open menu (FR-3).
    state.keyboard_leaves = 0;
    input.send("motion-abs 0.5 0.9\nbutton 272 down\nbutton 272 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.keyboard_leaves > 0,
    );

    // A second chrome surface with a non-zero origin (bottom-right panel)
    // must be hit-tested in output coordinates, not surface-local ones: the
    // hit-test origin regression used to shift the region by its origin.
    let panel_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let panel = shell.get_layer_surface(
        &panel_surface,
        None,
        df_shell::Layer::Top,
        "panel".to_string(),
        &qh,
        (),
    );
    panel.set_anchor(2 | 8); // bottom | right
    panel.set_size(120, 40);
    panel.set_margin(0, 10, 10, 0); // right 10, bottom 10
    panel.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    panel_surface.commit();
    let (panel_buffer, _panel_file) = shm_buffer(&state, &qh, 120, 40);
    panel_surface.attach(Some(&panel_buffer), 0, 0);
    panel_surface.commit();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures
                .iter()
                .any(|(_, width, height)| *width == 120 && *height == 40)
        },
    );

    state.pointer_enters = 0;
    // Panel centre: (1280 - 120 - 10 + 60, 720 - 40 - 10 + 20) = (1210, 690).
    input.send("motion-abs 0.9453125 0.9583333");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );

    drop(panel);
    drop(panel_surface);
    drop(layer);
    drop(surface);
    drop(shell);
    drop(core);
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-09 overlay dropdown (compositor half): the menu bar's transient dropdown
/// is a separate `overlay` chrome surface placed by top/left margins. It must
/// reserve no zone (the bar still owns the 28 px reserve), size to its
/// requested rectangle, and — being a layer above `top` and below the bar's
/// input strip — receive pointer input over the area it covers.
#[test]
fn overlay_popup_sits_above_the_bar_and_reserves_nothing() {
    let token = "ee".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-overlay-popup-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-overlay-popup",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
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
    let (shell_name, shell_version) = state.shell_global.expect("df_shell advertised");
    let shell = bind_shell(&mut state, &queue, shell_name, shell_version);
    // The observer side of the same client: bind the manager so the output's
    // reserved zone is announced.
    let (manager_name, manager_version) = state.manager_global.expect("manager advertised");
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    let qh = queue.handle();

    // The bar: top | left | right, 28 px, reserved.
    let bar_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let bar = shell.get_layer_surface(
        &bar_surface,
        None,
        df_shell::Layer::Top,
        "menubar".to_string(),
        &qh,
        (),
    );
    bar.set_anchor(1 | 4 | 8);
    bar.set_size(0, 28);
    bar.set_exclusive_zone(28);
    bar.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::OnDemand);
    bar_surface.commit();
    let (bar_buffer, _bar_file) = shm_buffer(&state, &qh, OUTPUT_W, 28);
    bar_surface.attach(Some(&bar_buffer), 0, 0);
    bar_surface.commit();

    // The popup: top | left, offset 40/28, an explicit rectangle, no reserve,
    // and no keyboard (the bar keeps Escape/menu navigation).
    let popup_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let popup = shell.get_layer_surface(
        &popup_surface,
        None,
        df_shell::Layer::Overlay,
        "menubar-popup".to_string(),
        &qh,
        (),
    );
    popup.set_anchor(1 | 4);
    popup.set_size(200, 300);
    popup.set_margin(28, 0, 0, 40); // top 28, left 40
    popup.set_exclusive_zone(-1);
    popup.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    popup_surface.commit();
    let (popup_buffer, _popup_file) = shm_buffer(&state, &qh, 200, 300);
    popup_surface.attach(Some(&popup_buffer), 0, 0);
    popup_surface.commit();

    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures
                .iter()
                .any(|(_, width, height)| *width == 200 && *height == 300)
        },
    );

    // The popup's negative exclusive zone opts out; the only top reserve is
    // the bar's 28 px.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved
                .iter()
                .any(|(edge, thickness)| *edge == 0 && *thickness == 28)
        },
    );
    assert!(
        state
            .output_reserved
            .iter()
            .all(|(edge, thickness)| *edge != 0 || *thickness <= 28),
        "the overlay popup must not enlarge the reserved zone: {:?}",
        state.output_reserved
    );

    // Pointer over the popup (global 140,100 → popup-local 100,72) must land
    // on the overlay surface, below the 28 px bar.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some() && state.keyboard.is_some(),
    );
    let _ = queue.roundtrip(&mut state);
    state.pointer_enters = 0;
    state.pointer_motions.clear();
    state.pointer_enter_positions.clear();
    input.send("motion-abs 0.109375 0.1388889");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );
    let (mx, my) = *state
        .pointer_enter_positions
        .last()
        .expect("popup enter position");
    assert!(
        (mx - 100.0).abs() < 1.0 && (my - 72.0).abs() < 1.0,
        "pointer must enter the overlay popup in popup-local coordinates: got ({mx}, {my})"
    );

    drop(popup);
    drop(popup_surface);
    drop(bar);
    drop(bar_surface);
    drop(manager);
    drop(shell);
    drop(core);
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-10 Dock popover (compositor half): the Dock's context-menu/window-chooser
/// surface is an `overlay` anchored bottom|left and placed with a bottom
/// margin, so it sits above the Dock. It reserves nothing (the Dock keeps its
/// bottom reserve) and receives pointer input in popover-local coordinates.
#[test]
fn dock_popup_is_bottom_anchored_and_reserves_nothing() {
    let token = "dd".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-dock-popup-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-dock-popup",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);
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
    let (shell_name, shell_version) = state.shell_global.expect("df_shell advertised");
    let shell = bind_shell(&mut state, &queue, shell_name, shell_version);
    let (manager_name, manager_version) = state.manager_global.expect("manager advertised");
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    let qh = queue.handle();

    // The Dock: bottom | left | right, 95 px tall, reserves 60 px.
    let dock_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let dock = shell.get_layer_surface(
        &dock_surface,
        None,
        df_shell::Layer::Top,
        "dock".to_string(),
        &qh,
        (),
    );
    dock.set_anchor(2 | 4 | 8);
    dock.set_size(0, 95);
    dock.set_exclusive_zone(60);
    dock.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    dock_surface.commit();
    let (dock_buffer, _dock_file) = shm_buffer(&state, &qh, OUTPUT_W, 95);
    dock_surface.attach(Some(&dock_buffer), 0, 0);
    dock_surface.commit();

    // The popup: bottom | left, 220x160, 40 px above the bottom edge and 300
    // from the left, overlay layer, no reserve, no keyboard.
    let popup_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let popup = shell.get_layer_surface(
        &popup_surface,
        None,
        df_shell::Layer::Overlay,
        "dock-popup".to_string(),
        &qh,
        (),
    );
    popup.set_anchor(2 | 4); // bottom | left
    popup.set_size(220, 160);
    popup.set_margin(0, 0, 40, 300); // bottom 40, left 300
    popup.set_exclusive_zone(-1);
    popup.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    popup_surface.commit();
    let (popup_buffer, _popup_file) = shm_buffer(&state, &qh, 220, 160);
    popup_surface.attach(Some(&popup_buffer), 0, 0);
    popup_surface.commit();

    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures
                .iter()
                .any(|(_, width, height)| *width == 220 && *height == 160)
        },
    );

    // The popup opts out of reserved zones; the only bottom reserve is the
    // Dock's 60 px.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved
                .iter()
                .any(|(edge, thickness)| *edge == 1 && *thickness == 60)
        },
    );
    assert!(
        state
            .output_reserved
            .iter()
            .all(|(edge, thickness)| *edge != 1 || *thickness <= 60),
        "the Dock popup must not enlarge the reserved zone: {:?}",
        state.output_reserved
    );

    // Pointer over the popup: global (400, 600) is popup-local (100, 80)
    // (the popup spans x 300..520, y 520..680 on the 1280x720 output).
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some() && state.keyboard.is_some(),
    );
    let _ = queue.roundtrip(&mut state);
    state.pointer_enters = 0;
    state.pointer_motions.clear();
    state.pointer_enter_positions.clear();
    input.send("motion-abs 0.3125 0.8333333");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );
    let (mx, my) = *state
        .pointer_enter_positions
        .last()
        .expect("Dock popup enter position");
    assert!(
        (mx - 100.0).abs() < 1.0 && (my - 80.0).abs() < 1.0,
        "pointer must enter the Dock popup in popup-local coordinates: got ({mx}, {my})"
    );

    drop(popup);
    drop(popup_surface);
    drop(dock);
    drop(dock_surface);
    drop(manager);
    drop(shell);
    drop(core);
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}
