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

use wayland_client::protocol::wl_data_device_manager::DndAction;
use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_data_device, wl_data_device_manager, wl_data_offer,
    wl_data_source, wl_keyboard, wl_pointer, wl_region, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
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
    /// Pointer leaves delivered to this client.
    pointer_leaves: usize,
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
    /// `xdg_toplevel.close` requests the compositor forwarded to this client
    /// (the Dock "Quit"/window-close half of the core interaction loop).
    toplevel_close_requests: usize,
    toplevel_closed: usize,
    layer_configures: Vec<(u32, i32, i32)>,
    layer_closed: usize,
    done_count: usize,
    focused: Vec<Option<df_toplevel::DfToplevel>>,
    attentions: Vec<df_toplevel::DfToplevel>,
    hot_corners: Vec<(u32, String)>,
    overviews: Vec<(u32, bool)>,
    /// `wl_callback.frame` callbacks delivered to this client (T-11 U-3).
    frame_callbacks: usize,
    app_switchers: Vec<(u32, Option<String>, i32)>,
    /// The recency cards the shell overlay draws (T-06.2a): `(index, app_id)`.
    app_switcher_entries: Vec<(u32, String)>,
    input_actions: Vec<(String, String, u32)>,
    progress_events: usize,
    app_accelerators: Vec<(String, String, String, u32)>,
    // Drag-and-drop (T-10 external drops). The same harness acts as the
    // drag source (offers a payload) and as the target (a trusted chrome
    // surface that receives the offer).
    data_device_manager: Option<wl_data_device_manager::WlDataDeviceManager>,
    data_device: Option<wl_data_device::WlDataDevice>,
    data_source: Option<wl_data_source::WlDataSource>,
    /// The bytes the source writes when the target requests `text/uri-list`.
    source_payload: Option<String>,
    data_source_sent: usize,
    data_source_cancelled: usize,
    data_source_drop_performed: usize,
    data_source_finished: usize,
    /// Target-side offer state.
    dnd_offer: Option<wl_data_offer::WlDataOffer>,
    dnd_mimes: Vec<String>,
    dnd_enters: usize,
    dnd_motions: usize,
    dnd_leaves: usize,
    dnd_drops: usize,
    /// `wl_data_offer.action` events (proves the compositor processed
    /// `set_actions` before the drop).
    dnd_action_events: usize,
    /// Read end of the pipe the target asked the source to fill.
    dnd_read_fd: Option<std::os::fd::RawFd>,
    /// The serial of the most recent pointer button press (the drag needs it).
    pointer_button_serial: Option<u32>,
    /// Per-output reserved zones, keyed by the `df_output` protocol id, so a
    /// hotplug test can prove the *new* output received a chrome reserve (not
    /// merely that some output did).
    output_reserved_by_id: Vec<(u32, u32, u32)>,
    /// `df_output` name keyed by protocol id (paired with the above).
    output_name_by_id: Vec<(u32, String)>,
    /// `df_layer_surface` configures keyed by protocol id, so a test can prove
    /// a specific chrome surface was reconfigured on output hotplug.
    layer_configures_by_id: Vec<(u32, i32, i32)>,
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
                "wl_data_device_manager" => {
                    state.data_device_manager = Some(registry.bind(name, version.min(3), _qh, ()));
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
        resource: &df_layer_surface::DfLayerSurface,
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
            } => {
                state.layer_configures.push((serial, width, height));
                state
                    .layer_configures_by_id
                    .push((resource.id().protocol_id(), width, height));
            }
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
            df_toplevel_manager::Event::AppSwitcherEntry { index, app_id } => {
                state.app_switcher_entries.push((index, app_id))
            }
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
        resource: &df_output::DfOutput,
        event: df_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            df_output::Event::Name { name } => {
                state.output_names.push(name.clone());
                state
                    .output_name_by_id
                    .push((resource.id().protocol_id(), name));
            }
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
            df_output::Event::ReservedZone { edge, thickness } => {
                let edge = edge.into_result().map(|e| e as u32).unwrap_or(0);
                state.output_reserved.push((edge, thickness));
                state
                    .output_reserved_by_id
                    .push((resource.id().protocol_id(), edge, thickness));
            }
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

impl Dispatch<wl_callback::WlCallback, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.frame_callbacks += 1;
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
delegate_noop!(TestClient: ignore wl_data_device_manager::WlDataDeviceManager);

impl Dispatch<wl_data_device::WlDataDevice, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_device::Event::DataOffer { id } => state.dnd_offer = Some(id),
            wl_data_device::Event::Enter { serial, id, .. } => {
                state.dnd_enters += 1;
                if let Some(offer) = id {
                    // Accept a mime type and pick an action: the compositor
                    // only validates (and delivers) a drop once both are set.
                    offer.accept(serial, Some("text/uri-list".to_string()));
                    offer.set_actions(DndAction::Copy | DndAction::Move, DndAction::Copy);
                    state.dnd_offer = Some(offer);
                }
            }
            wl_data_device::Event::Leave => state.dnd_leaves += 1,
            wl_data_device::Event::Motion { .. } => state.dnd_motions += 1,
            wl_data_device::Event::Drop => {
                state.dnd_drops += 1;
                if let Some(offer) = state.dnd_offer.clone() {
                    let mut fds = [0i32; 2];
                    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
                    assert_eq!(rc, 0, "pipe() failed");
                    {
                        use std::os::fd::FromRawFd;
                        // `receive` borrows the write end; dropping it after
                        // the call closes our copy, leaving the source as the
                        // only writer so the read side sees EOF.
                        let write = unsafe { std::os::fd::OwnedFd::from_raw_fd(fds[1]) };
                        offer.receive("text/uri-list".to_string(), write.as_fd());
                    }
                    state.dnd_read_fd = Some(fds[0]);
                }
            }
            wl_data_device::Event::Selection { .. } => {}
            _ => {}
        }
    }

    wayland_client::event_created_child!(TestClient, wl_data_device::WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (wl_data_offer::WlDataOffer, ()),
    ]);
}

impl Dispatch<wl_data_offer::WlDataOffer, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &wl_data_offer::WlDataOffer,
        event: wl_data_offer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_offer::Event::Offer { mime_type } => state.dnd_mimes.push(mime_type),
            wl_data_offer::Event::Action { .. } => state.dnd_action_events += 1,
            _ => {}
        }
    }
}

impl Dispatch<wl_data_source::WlDataSource, ()> for TestClient {
    fn event(
        state: &mut Self,
        _: &wl_data_source::WlDataSource,
        event: wl_data_source::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_source::Event::Send { mime_type, fd } => {
                if let Some(payload) = state.source_payload.clone() {
                    if mime_type == "text/uri-list" {
                        use std::io::Write;
                        let mut file = std::fs::File::from(fd);
                        file.write_all(payload.as_bytes())
                            .expect("write drag payload");
                        state.data_source_sent += 1;
                    }
                }
            }
            wl_data_source::Event::Cancelled => state.data_source_cancelled += 1,
            wl_data_source::Event::DndDropPerformed => state.data_source_drop_performed += 1,
            wl_data_source::Event::DndFinished => state.data_source_finished += 1,
            _ => {}
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
        if let wl_pointer::Event::Leave { .. } = event {
            state.pointer_leaves += 1;
        }
        if let wl_pointer::Event::Motion {
            surface_x,
            surface_y,
            ..
        } = event
        {
            state.pointer_motions.push((surface_x, surface_y));
        }
        if let wl_pointer::Event::Button {
            serial,
            state: button_state,
            ..
        } = event
        {
            if matches!(
                button_state.into_result(),
                Ok(wl_pointer::ButtonState::Pressed)
            ) {
                state.pointer_button_serial = Some(serial);
            }
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
        state: &mut Self,
        _: &xdg_tl::XdgToplevel,
        event: xdg_tl::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_tl::Event::Close = event {
            state.toplevel_close_requests += 1;
        }
    }
}

// --- process harness -------------------------------------------------------

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    token_path: PathBuf,
    stderr_path: PathBuf,
    stdout_path: PathBuf,
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
        let _ = std::fs::remove_file(&self.stdout_path);
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
        Self::start_impl(
            socket_name,
            tokens,
            synthetic_path,
            synthetic_output_path,
            false,
        )
    }

    /// Start with the synthetic-input harness and the U-2 per-frame timing
    /// trace enabled (`DRAGONFRUIT_FRAME_TRACE`).
    fn start_with_synthetic_trace(
        socket_name: &str,
        tokens: &[String],
        synthetic_path: Option<&Path>,
    ) -> Self {
        Self::start_impl(socket_name, tokens, synthetic_path, None, true)
    }

    fn start_impl(
        socket_name: &str,
        tokens: &[String],
        synthetic_path: Option<&Path>,
        synthetic_output_path: Option<&Path>,
        frame_trace: bool,
    ) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let token_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.launch-token"));
        let stderr_path = std::env::temp_dir().join(format!("{socket_name}.stderr"));
        let stderr_file = std::fs::File::create(&stderr_path).expect("create stderr log");
        let stdout_path = std::env::temp_dir().join(format!("{socket_name}.stdout"));
        let stdout_file = std::fs::File::create(&stdout_path).expect("create stdout log");

        let mut command = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"));
        command
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_LAUNCH_TOKENS", tokens.join(","))
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file));
        if frame_trace {
            command.env("DRAGONFRUIT_FRAME_TRACE", "1");
        }
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
            stdout_path,
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

    /// The compositor's stdout log so far (frame-timing traces, stats).
    fn read_stdout(&self) -> String {
        std::fs::read_to_string(&self.stdout_path).unwrap_or_default()
    }

    /// Send SIGUSR1 so the compositor dumps its counters/trace, then return
    /// the stdout accumulated up to `timeout`.
    fn dump_and_read_stdout(&self, timeout: Duration) -> String {
        unsafe {
            kill(self.child.id() as i32, SIGUSR1);
        }
        let deadline = Instant::now() + timeout;
        loop {
            let text = self.read_stdout();
            if text.contains("frame timing") {
                return text;
            }
            assert!(
                Instant::now() < deadline,
                "no frame-timing dump after SIGUSR1"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
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
const SIGUSR1: i32 = 10;

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
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

    // `df_output.set_transform` applies through the same path (T-09.5): the
    // Displays pane's rotation control is forwarded here by the shell.
    state.output_transforms.clear();
    state.outputs[0].set_transform(df_output::Transform::_90);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.output_transforms.contains(&1),
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
fn vertical_dock_reserves_the_left_and_right_zones() {
    let token = "1b".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-dock-vertical", &[token]);
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

    // A vertical Dock anchored left|top|bottom must reserve its left edge
    // (edge 2), the right Dock its right edge (edge 3) — T-10 section 2.
    let left_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let left = shell.get_layer_surface(
        &left_surface,
        None,
        df_shell::Layer::Top,
        "dock".to_string(),
        &qh,
        (),
    );
    left.set_anchor(4 | 1 | 2); // left | top | bottom
    left.set_size(80, 0);
    left.set_exclusive_zone(60);
    left.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    left_surface.commit();

    let right_surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let right = shell.get_layer_surface(
        &right_surface,
        None,
        df_shell::Layer::Top,
        "dock".to_string(),
        &qh,
        (),
    );
    right.set_anchor(8 | 1 | 2); // right | top | bottom
    right.set_size(80, 0);
    right.set_exclusive_zone(60);
    right.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    right_surface.commit();

    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved
                .iter()
                .any(|(edge, thickness)| *edge == 2 && *thickness == 60)
                && state
                    .output_reserved
                    .iter()
                    .any(|(edge, thickness)| *edge == 3 && *thickness == 60)
        },
    );

    drop(right);
    drop(right_surface);
    drop(left);
    drop(left_surface);
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

/// T-10 section 8/9 click tree (FR-1/FR-5) and the Phase-2 core interaction
/// loop (`docs/tasks/legacy/10-dock.md` acceptance: launch → minimize → restore from Dock
/// → close). The two private-protocol paths the Dock uses to bring a window
/// forward are `df_toplevel_manager.activate_app` (a plain click on a running
/// app entry) and `select_overview_toplevel` (a row in the window chooser).
/// Both must pick the app's most recent window, restore it when minimized,
/// switch to its Space, and focus it. Closing the window (the Dock "Quit"
/// action) must reach the client as `xdg_toplevel.close` and retire the
/// handle.
///
/// The Dock projects an entry from the window list, not from focus, so a
/// click must find the app's windows even before any of them has been
/// focused. This test maps two windows of one app and one of another, then
/// drives both requests over the protocol.
#[test]
fn dock_click_tree_activation_conformance() {
    const APP: &str = "org.dragonfruit.DockApp";
    const OTHER: &str = "org.dragonfruit.OtherApp";

    let token = "88".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-click-tree", &[token]);
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
        |state| state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // Map two windows of one app, then a third of another. Each is mapped
    // (and awaited) in turn, so announcement order is stable. A mapping is
    // what the compositor observes when the shell's launch path
    // (`launchDockApp` → app-index → new process) finally presents a window.
    let (surface_a, xdg_a, toplevel_a, _file_a) =
        map_toplevel(&mut state, &mut queue, "Dock A", APP);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    let handle_a = state.toplevels[0].clone();

    let (surface_b, xdg_b, toplevel_b, _file_b) =
        map_toplevel(&mut state, &mut queue, "Dock B", APP);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevels.len() >= 2,
    );
    let handle_b = state.toplevels[1].clone();

    let (surface_c, xdg_c, toplevel_c, _file_c) =
        map_toplevel(&mut state, &mut queue, "Other", OTHER);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevels.len() >= 3,
    );
    let handle_c = state.toplevels[2].clone();

    let focused_is = |state: &TestClient, expected: &df_toplevel::DfToplevel| {
        state
            .focused
            .iter()
            .flatten()
            .any(|handle| handle == expected)
    };

    // --- a plain Dock click with no window ever focused ------------------
    // `activate_app` must find the app's windows even though none has held
    // focus yet, and select the most recently mapped one (B).
    state.focused.clear();
    manager.activate_app(APP.to_string());
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.iter().any(Option::is_some),
    );
    assert!(
        focused_is(&state, &handle_b),
        "activate_app must pick the app's most recent window (B): {:?}",
        state.focused
    );

    // --- an unrelated app resolves to its own window ---------------------
    state.focused.clear();
    manager.activate_app(OTHER.to_string());
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.iter().any(Option::is_some),
    );
    assert!(
        focused_is(&state, &handle_c),
        "activate_app must resolve the other app's window: {:?}",
        state.focused
    );

    // --- a minimized window is restored, not merely focused --------------
    handle_b.minimize();
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
    state.focused.clear();
    state.toplevel_states.clear();
    manager.activate_app(APP.to_string());
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| focused_is(state, &handle_b),
    );
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

    // --- the chooser selects a specific window and its Space -------------
    let second = state.workspaces[1].clone();
    handle_a.move_to_workspace(&second);
    let _ = queue.roundtrip(&mut state);
    state.workspace_activated.clear();
    state.focused.clear();
    manager.select_overview_toplevel(&handle_a);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_activated.contains(&1),
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| focused_is(state, &handle_a),
    );

    // --- close: the Dock "Quit" action retires the window ----------------
    // `df_toplevel.close` is a request to the *client*, which acks by
    // tearing down its xdg surface. The manager must then announce Closed.
    // Close the frontmost (active-Space) window, matching a Dock "Quit".
    state.toplevel_close_requests = 0;
    state.toplevel_closed = 0;
    handle_a.close();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_close_requests >= 1,
    );
    toplevel_a.destroy();
    surface_a.destroy();
    xdg_a.destroy();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_closed >= 1,
    );

    // A window on a non-active Space must retire too: the Dock chooser
    // lists every Space, so a Quit there must clear the row (FR-5). C is
    // still on Space 0 while Space 1 is active.
    state.toplevel_close_requests = 0;
    let closed_before = state.toplevel_closed;
    handle_c.close();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_close_requests >= 1,
    );
    toplevel_c.destroy();
    surface_c.destroy();
    xdg_c.destroy();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_closed > closed_before,
    );

    toplevel_b.destroy();
    surface_b.destroy();
    xdg_b.destroy();
    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
}

/// T-01.6b acceptance: the Dock/menu-bar integration transitions the shell
/// projects from the private protocol, driven end to end over the shell
/// protocol — the headless half of the T-01 demo walkthrough
/// (`scripts/capture-demo.sh` is the live half).
///
/// One window walks the whole loop: it is announced with its name (title +
/// app id, the Dock/menu-bar label source), it is a running entry, focusing
/// it updates the menu bar, minimizing it produces the minimized entry,
/// `activate_app` (the Dock click) restores the running entry, and closing it
/// retires the handle.
#[test]
fn dock_entry_lifecycle_focus_running_minimize_restore_close() {
    const APP: &str = "org.dragonfruit.LifecycleApp";
    let token = "5c".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-dock-lifecycle", &[token]);
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
        |state| state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // --- a mapped window carries the Dock/menu-bar name -------------------
    let (surface, xdg_surface, toplevel, _file) =
        map_toplevel(&mut state, &mut queue, "Lifecycle", APP);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_titles
                .iter()
                .any(|title| title.as_deref() == Some("Lifecycle"))
                && state
                    .toplevel_app_ids
                    .iter()
                    .any(|app| app.as_deref() == Some(APP))
        },
    );
    let handle = state.toplevels[0].clone();

    // --- running entry: present and not minimized -------------------------
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .toplevel_states
                .iter()
                .all(|flags| flags & df_toplevel::State::Minimized.bits() == 0)
        },
    );

    // --- focus: the menu bar names the focused window ---------------------
    state.focused.clear();
    manager.activate_app(APP.to_string());
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.focused.iter().flatten().any(|f| f == &handle),
    );

    // --- minimize: the minimized entry appears ----------------------------
    state.toplevel_states.clear();
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

    // --- restore from the Dock: running entry again -----------------------
    // Focus is already proven above; here the Dock click must clear the
    // minimized state (the running entry reappears).
    state.toplevel_states.clear();
    manager.activate_app(APP.to_string());
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

    // --- close: the Dock entry resolves -----------------------------------
    state.toplevel_close_requests = 0;
    state.toplevel_closed = 0;
    handle.close();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevel_close_requests >= 1,
    );
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

/// Request a fresh `xdg-activation` token and activate `surface` with it.
/// The token round-trip is required by the protocol; the compositor under
/// test does not validate token data yet.
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

/// T-10 FR-4 / `xdg-activation`: a valid activation for a window on the
/// active Space grants keyboard focus — the launch-focus path, so a launched
/// app's first window is usable without a click — while an activation for a
/// window on a background Space must not yank the user there and instead
/// surfaces the Dock attention bounce.
#[test]
fn xdg_activation_focuses_the_active_space_window() {
    const APP: &str = "org.dragonfruit.ActivationApp";
    let token = "99".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-activation", &[token]);
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
        |state| state.workspaces.len() >= 3 && state.done_count > 0,
    );

    let (surface_a, xdg_a, toplevel_a, _file_a) =
        map_toplevel(&mut state, &mut queue, "Active", APP);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    let handle_a = state.toplevels[0].clone();

    let (surface_b, xdg_b, toplevel_b, _file_b) =
        map_toplevel(&mut state, &mut queue, "Focused", APP);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.toplevels.len() >= 2,
    );
    let handle_b = state.toplevels[1].clone();

    let focused_is = |state: &TestClient, expected: &df_toplevel::DfToplevel| {
        state
            .focused
            .iter()
            .flatten()
            .any(|handle| handle == expected)
    };

    // A shell click focuses A; the activation must move focus to B.
    handle_a.activate();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| focused_is(state, &handle_a),
    );
    state.focused.clear();
    state.attentions.clear();
    xdg_activate(&conn, &mut queue, &mut state, &surface_b);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| focused_is(state, &handle_b),
    );
    assert!(
        state.attentions.iter().any(|handle| handle == &handle_b),
        "an active-Space activation must also emit the Dock attention signal: {:?}",
        state.attentions
    );

    // Background-Space activation: bounce, no focus theft, no Space switch.
    let background = state.workspaces[1].clone();
    handle_b.move_to_workspace(&background);
    let _ = queue.roundtrip(&mut state);
    state.focused.clear();
    state.attentions.clear();
    state.workspace_activated.clear();
    xdg_activate(&conn, &mut queue, &mut state, &surface_b);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.attentions.iter().any(|handle| handle == &handle_b),
    );
    let _ = queue.roundtrip(&mut state);
    assert!(
        !focused_is(&state, &handle_b),
        "a background-Space activation must not steal focus"
    );
    assert!(
        !state.workspace_activated.contains(&1),
        "a background-Space activation must not switch Space: {:?}",
        state.workspace_activated
    );

    toplevel_a.destroy();
    surface_a.destroy();
    xdg_a.destroy();
    toplevel_b.destroy();
    surface_b.destroy();
    xdg_b.destroy();
    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
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
    // Activation always emits the Dock attention signal; it additionally
    // grants focus to an active-Space window (asserted by
    // `xdg_activation_focuses_the_active_space_window`).
    state.attentions.clear();
    xdg_activate(&conn, &mut queue, &mut state, &surface);
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
    state.app_switcher_entries.clear();
    manager.cycle_app_switcher(1);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.app_switchers.iter().any(|(active, ..)| *active == 1),
    );
    // T-06.2a: the active projection hands the shell the recency cards in
    // order (index 0 = most recent), before the batch `done`.
    assert!(
        !state.app_switcher_entries.is_empty(),
        "an active switcher broadcasts its recency entries"
    );
    assert_eq!(state.app_switcher_entries[0].0, 0);

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
/// (see `compositor/src/input/synthetic.rs` for the wire format). The reply
/// socket is bound so `query` commands can read the compositor's answer.
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
    // A discrete trigger animates on the compositor clock (T-11), so wait for
    // it to commit and open the overview before the next trigger.
    state.overviews.clear();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
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

/// T-11 FR-1: a gesture and a hot corner drive the *same* overview state
/// machine, so one opens Mission Control and the other toggles it closed.
#[test]
fn overview_state_machine_has_trigger_parity() {
    let token = "77".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-synth-overview-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-overview",
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

    // --- gesture: a four-finger vertical swipe opens Mission Control ------
    state.overviews.clear();
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
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
    );
    assert!(
        state.progress_events > 0,
        "the overview transition must emit shared progress events"
    );

    // --- hot corner: the same machine, now toggling it closed ------------
    state.overviews.clear();
    input.send("motion-abs 0.001 0.001");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 0),
    );

    // --- workspace slide: the same pipeline translates live surfaces -----
    state.workspace_activated.clear();
    input.send("swipe-begin 3");
    input.send("swipe-update -100 0");
    input.send("swipe-update -100 0");
    input.send("swipe-end");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_activated.contains(&1),
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-11 FR-1 (U-7): the private-protocol `enter_mission_control` /
/// `exit_mission_control` requests drive the *same* overview machine and
/// progress pipeline as a gesture, keyboard shortcut, or hot corner — they
/// are recorded in the shared input outbox and emit shared `progress`
/// events, not applied through a second, discrete code path.
#[test]
fn shell_request_drives_the_overview_pipeline() {
    let token = "5c".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-overview-shell", &[token]);
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

    // --- enter: the request must produce the shared action + progress ------
    state.overviews.clear();
    state.input_actions.clear();
    state.progress_events = 0;
    manager.enter_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
    );
    assert!(
        state
            .input_actions
            .iter()
            .any(|(action, source, _)| action == "mission-control" && source == "shell"),
        "the shell request must be recorded as a `shell` input action: {:?}",
        state.input_actions
    );
    assert!(
        state.progress_events > 0,
        "the shell request must emit shared progress events through the pipeline"
    );

    // --- exit: the same pipeline, targeting the closed state --------------
    state.overviews.clear();
    state.input_actions.clear();
    state.progress_events = 0;
    manager.exit_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 0),
    );
    assert!(
        state
            .input_actions
            .iter()
            .any(|(action, source, _)| action == "mission-control" && source == "shell"),
        "the exit request shares the same outbox path"
    );
    assert!(
        state.progress_events > 0,
        "the exit request shares the same progress pipeline"
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
}

/// T-11 FR-9 (U-1): the shell mirrors `accessibility.reduceMotion` into the
/// compositor over the additive v3 `set_reduced_motion` request; a subsequent
/// Mission Control request takes the single-step path (Begin/Update/End, no
/// animation clock) while still committing through the same rule.
#[test]
fn reduced_motion_request_single_steps_the_overview() {
    let token = "5f".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-conformance-reduced-motion", &[token]);
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
    assert_eq!(
        manager_version, 5,
        "the manager must advertise the additive requests through v5"
    );
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // Enable reduced motion, then open Mission Control.
    manager.set_reduced_motion(1);
    state.overviews.clear();
    state.progress_events = 0;
    manager.enter_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
    );
    // A single step emits exactly Begin + Update + committed End; the
    // animated path emits one Update per animation-clock tick.
    assert!(
        state.progress_events <= 3,
        "reduced motion must take the single-step path, saw {} progress events",
        state.progress_events
    );

    // Turning it back off restores the animated path; the request still
    // commits through the same pipeline.
    manager.set_reduced_motion(0);
    state.overviews.clear();
    manager.exit_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 0),
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
}

/// T-08.2c: the shell forwards the settingsd motion/input policy over the
/// additive v5 `set_motion_policy`/`set_input_policy` requests. The
/// compositor is the sole applier: a settingsd change alters the live
/// motion policy and the input repeat/gesture policy, and a disabled gesture
/// family stops firing immediately. The synthetic `query policy` report is
/// the end-to-end observation of the one owner.
#[test]
fn motion_and_input_policy_requests_apply_live() {
    let token = "b7".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-synth-policy-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-policy",
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
    assert_eq!(
        manager_version, 5,
        "the manager must advertise the additive requests through v5"
    );
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.workspaces.len() >= 3 && state.done_count > 0,
    );

    // A settingsd snapshot: light, minimize-on-double-click, no minimize
    // animation, slow repeat, Mission Control gestures off.
    manager.set_motion_policy(
        Some("light".to_string()),
        Some("minimize".to_string()),
        Some("none".to_string()),
    );
    manager.set_input_policy(350, 40, 1, 1, 0);
    let _ = conn.flush();

    // The protocol and the synthetic harness are two transports on the same
    // loop; poll the query until the request is applied.
    let mut report = String::new();
    for _ in 0..100 {
        report = input.query("query policy");
        if report.contains("scheme=light") && report.contains("repeat-delay=350") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        report.contains("scheme=light"),
        "policy not applied: {report:?}"
    );
    assert!(
        report.contains("titlebar-double-click=minimize"),
        "policy not applied: {report:?}"
    );
    assert!(
        report.contains("minimized-animation=none"),
        "policy not applied: {report:?}"
    );
    assert!(
        report.contains("repeat-delay=350"),
        "policy not applied: {report:?}"
    );
    assert!(
        report.contains("repeat-rate=40"),
        "policy not applied: {report:?}"
    );
    assert!(
        report.contains("gestures-enabled=1"),
        "policy not applied: {report:?}"
    );
    assert!(
        report.contains("gesture-space-switch=1"),
        "policy not applied: {report:?}"
    );
    assert!(
        report.contains("gesture-mission-control=0"),
        "policy not applied: {report:?}"
    );

    // Mission Control gestures are gated off, so a four-finger vertical swipe
    // no longer reaches the outbox.
    state.input_actions.clear();
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    input.send("swipe-update 0 -100");
    input.send("swipe-end");
    std::thread::sleep(Duration::from_millis(150));
    queue.dispatch_pending(&mut state).expect("dispatch");
    assert!(
        !state
            .input_actions
            .iter()
            .any(|(action, source, _)| action == "mission-control" && source == "gesture"),
        "a disabled gesture family must not fire: {:?}",
        state.input_actions
    );

    // Re-enabling it through the same request restores the gesture live.
    manager.set_input_policy(350, 40, 1, 1, 1);
    let _ = conn.flush();
    for _ in 0..100 {
        report = input.query("query policy");
        if report.contains("gesture-mission-control=1") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        report.contains("gesture-mission-control=1"),
        "policy not re-applied: {report:?}"
    );
    state.input_actions.clear();
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

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
}

/// T-11 FR-6 (U-8): while Mission Control owns pointer input, window
/// surfaces are not hit-tested; ownership returns explicitly when the
/// overview closes, and the normal focus path resumes.
#[test]
fn overview_owns_pointer_hit_testing_until_it_closes() {
    let token = "5d".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-synth-hittest-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-overview-hittest",
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

    // A real window, centered on the 1280x720 output.
    let (_surface, _xdg_surface, _toplevel, _file) =
        map_toplevel(&mut state, &mut queue, "HitTest", "org.dragonfruit.HitTest");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );

    // The pointer reaches the window's surface normally.
    state.pointer_enters = 0;
    state.pointer_leaves = 0;
    input.send("motion-abs 0.5 0.5");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );

    // Open Mission Control: the overview now owns hit-testing.
    manager.enter_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
    );

    let enters_before = state.pointer_enters;
    let leaves_before = state.pointer_leaves;
    input.send("motion-abs 0.5 0.5");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_leaves > leaves_before,
    );
    assert_eq!(
        state.pointer_enters, enters_before,
        "no window surface may be hit-tested while the overview owns input"
    );

    // Close the overview: ownership returns explicitly and the window is
    // hit-tested again.
    manager.exit_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 0),
    );
    let enters_before = state.pointer_enters;
    input.send("motion-abs 0.5 0.5");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > enters_before,
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-11 FR-2 (U-3): Mission Control and workspace switching transform the
/// *live* surfaces, never thumbnails. A client surface stays mapped and keeps
/// receiving `wl_surface.frame` callbacks — and can keep committing fresh
/// buffers, "a playing video keeps playing" — across a full overview and a
/// workspace round-trip. Substituting a thumbnail would require unmapping the
/// client surface, which would stop the callbacks.
#[test]
fn overview_transition_keeps_live_surfaces_mapped() {
    let token = "a1".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-synth-live-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-overview-live",
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

    // A live surface with a real buffer, as a video player would have.
    let (surface, _xdg_surface, _toplevel, file) =
        map_toplevel(&mut state, &mut queue, "Live", "org.dragonfruit.Live");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.toplevels.is_empty(),
    );
    // Keep the shm backing files alive for the whole test; the buffers
    // reference the mapping until the compositor releases them.
    let mut files = vec![file];

    // --- baseline: a normal presented frame delivers its callback ---------
    let before = state.frame_callbacks;
    files.push(present_frame(&mut state, &queue, &surface));
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.frame_callbacks > before,
    );

    // --- Mission Control: the live surface keeps playing ------------------
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    input.send("swipe-update 0 -100");
    input.send("swipe-end");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
    );
    let before = state.frame_callbacks;
    files.push(present_frame(&mut state, &queue, &surface));
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.frame_callbacks > before,
    );
    assert_eq!(
        state.toplevel_closed, 0,
        "the live surface must stay mapped in the overview"
    );

    manager.exit_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 0),
    );

    // --- workspace round-trip: the surface survives and resumes -----------
    state.workspace_activated.clear();
    input.send("swipe-begin 3");
    input.send("swipe-update -100 0");
    input.send("swipe-update -100 0");
    input.send("swipe-end");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_activated.contains(&1),
    );
    state.workspace_activated.clear();
    input.send("swipe-begin 3");
    input.send("swipe-update 100 0");
    input.send("swipe-update 100 0");
    input.send("swipe-end");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.workspace_activated.contains(&0),
    );

    // Back on its Space, the same live surface presents again.
    let before = state.frame_callbacks;
    files.push(present_frame(&mut state, &queue, &surface));
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.frame_callbacks > before,
    );

    assert_eq!(
        state.toplevel_closed, 0,
        "no thumbnail substitution: the surface was never closed"
    );
    assert_eq!(
        state.toplevels.len(),
        1,
        "the window is still the one client surface, not a thumbnail object"
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// Commit a fresh buffer and request a `wl_surface.frame` callback on
/// `surface` (T-11 U-3): the client half of "a playing video keeps playing".
/// The returned backing file must outlive the buffer.
fn present_frame(
    state: &mut TestClient,
    queue: &EventQueue<TestClient>,
    surface: &wl_surface::WlSurface,
) -> std::fs::File {
    let qh = queue.handle();
    let _callback = surface.frame(&qh, ());
    let (buffer, file) = shm_buffer(state, &qh, 200, 150);
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();
    file
}

/// T-11 FR-8 (U-2): a full overview gesture produces a per-frame timing
/// trace whose animation-clock frame intervals stay within a CI budget.
#[test]
fn overview_gesture_frame_trace_stays_within_budget() {
    let token = "5e".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-synth-frametime-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic_trace(
        "dragonfruit-conformance-overview-frametime",
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

    // A full overview gesture (open, then close) exercises the animation.
    input.send("swipe-begin 4");
    input.send("swipe-update 0 -100");
    input.send("swipe-update 0 -100");
    input.send("swipe-end");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 1),
    );
    // Close it through the shell request so the reverse transition renders
    // too (a four-finger *down* swipe is Desktop Reveal, not a close).
    manager.exit_mission_control();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.overviews.iter().any(|(active, _)| *active == 0),
    );

    let stdout = proc.dump_and_read_stdout(Duration::from_secs(5));
    let mut samples = 0usize;
    let mut max_interval_ms = 0u32;
    for line in stdout.lines() {
        let Some(rest) = line.split("frame-trace").nth(1) else {
            continue;
        };
        samples += 1;
        for field in rest.split_whitespace() {
            if let Some(value) = field.strip_prefix("interval_ms=") {
                max_interval_ms = max_interval_ms.max(value.parse().unwrap_or(0));
            }
        }
    }
    assert!(samples > 0, "the gesture must produce a frame-time trace");
    // Generous CI budget: the animation clock is 16 ms and the headless loop
    // renders on demand, so a rendered-frame gap over 100 ms means a stall.
    assert!(
        max_interval_ms <= 100,
        "frame interval {max_interval_ms} ms exceeds the CI budget"
    );

    manager.destroy();
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !synthetic_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
}

/// T-11 Slice C: Mission Control / workspace switching is **lockstep** across
/// every output. The model is lockstep by construction and
/// `apply_overview_scene` loops per output; this proves the protocol path too:
/// a keyboard switch (which now animates on the compositor clock) activates
/// the *same* Space index on both displays, and no output switches to a
/// different index.
#[test]
fn workspace_switch_is_lockstep_across_outputs() {
    let token = "ab".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let input_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-synth-lockstep-{}", std::process::id()));
    let output_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-synth-lockstep-out-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_harnesses(
        "dragonfruit-conformance-lockstep",
        std::slice::from_ref(&token),
        Some(&input_path),
        Some(&output_path),
    );
    let input = SyntheticInput::connect(&input_path);
    let outputs = SyntheticOutput::connect(&output_path);
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

    // Attach a second display; it gets its own three Spaces.
    outputs.send("add HDMI-A-1 1920 1080 1280 0");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state.output_names.iter().any(|name| name == "HDMI-A-1") && state.workspaces.len() >= 6
        },
    );

    // Ctrl+Right: the system WorkspaceNext shortcut. It drives the same
    // animated pipeline as a gesture (T-11 animation clock).
    state.workspace_activated.clear();
    input.send("key 29 down\nkey 106 down\nkey 106 up\nkey 29 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .workspace_activated
                .iter()
                .filter(|i| **i == 1)
                .count()
                >= 2
        },
    );

    let ones = state
        .workspace_activated
        .iter()
        .filter(|index| **index == 1)
        .count();
    assert_eq!(
        ones, 2,
        "every output switches to Space 1 in lockstep: {:?}",
        state.workspace_activated
    );
    assert!(
        state.workspace_activated.iter().all(|index| *index == 1),
        "no output switched to a different index: {:?}",
        state.workspace_activated
    );

    drop(manager);
    drop(core);
    let _ = conn.flush();
    proc.shutdown();
    assert!(
        !input_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
    assert!(
        !output_path.exists(),
        "teardown leak: synthetic-output socket survived"
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

/// Read a pipe until EOF (or a timeout) and return it as a string. Used to
/// drain the drag payload the target requested from the source.
fn read_pipe_to_string(fd: std::os::fd::RawFd, timeout: Duration) -> String {
    use std::io::Read;
    use std::os::fd::FromRawFd;
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let deadline = Instant::now() + timeout;
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut pollfd, 1, millis) };
        assert!(ready >= 0, "poll failed");
        if ready == 0 {
            break;
        }
        let n = file.read(&mut buf).expect("read drag payload");
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// T-10 FR-9 (external drops) — the end-to-end drag walkthrough the hand-off
/// asked for. A *separate* client starts a Wayland drag carrying a
/// `text/uri-list`; the compositor routes the drag to a trusted chrome
/// surface under the pointer (the Dock's target contract); the target
/// accepts and receives the payload; the source sees the drop.
///
/// The shell's own data-device plumbing is Qt code and cannot run in a
/// cargo test, so the target here is a raw shell stand-in that binds
/// `wl_data_device` exactly as the shell does (`tst_dock` covers the QML
/// drop logic). What this test guards is the compositor side the shell
/// relies on: the chrome-surface hit-test during a drag, the
/// offer/enter/motion/drop sequence, and the payload pipe. It needs two
/// distinct clients — smithay only sends a data offer when the drag source
/// and the target are different connections.
#[test]
fn client_drag_and_drop_reaches_a_chrome_surface() {
    let token = "ce".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-dnd-synth-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-dnd",
        std::slice::from_ref(&token),
        Some(&synthetic_path),
    );
    let input = SyntheticInput::connect(&synthetic_path);

    // --- target: a trusted chrome surface + its own data device ----------
    let (tgt_conn, mut tgt_queue, mut tgt) = connect(&proc.socket_path);
    let (name, version) = tgt.core_global.expect("df_core advertised");
    let tgt_core = bind_core(&mut tgt, &tgt_queue, name, version);
    tgt_core.authenticate(1, proc.read_token());
    wait_for(
        &tgt_conn,
        &mut tgt_queue,
        &mut tgt,
        Duration::from_secs(5),
        |state| state.authenticated.is_some(),
    );
    let (shell_name, shell_version) = tgt.shell_global.expect("df_shell advertised");
    let tgt_shell = bind_shell(&mut tgt, &tgt_queue, shell_name, shell_version);
    let tgt_qh = tgt_queue.handle();
    let tgt_surface = tgt.compositor.clone().unwrap().create_surface(&tgt_qh, ());
    let tgt_layer = tgt_shell.get_layer_surface(
        &tgt_surface,
        None,
        df_shell::Layer::Top,
        "dock".to_string(),
        &tgt_qh,
        (),
    );
    tgt_layer.set_anchor(1 | 4 | 8); // top | left | right
    tgt_layer.set_size(0, 28);
    tgt_layer.set_exclusive_zone(28);
    tgt_layer.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::OnDemand);
    tgt_surface.commit();
    let (tgt_buffer, _tgt_file) = shm_buffer(&tgt, &tgt_qh, OUTPUT_W, 28);
    tgt_surface.attach(Some(&tgt_buffer), 0, 0);
    tgt_surface.commit();
    wait_for(
        &tgt_conn,
        &mut tgt_queue,
        &mut tgt,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures
                .iter()
                .any(|(_, width, height)| *width == OUTPUT_W && *height == 28)
        },
    );
    wait_for(
        &tgt_conn,
        &mut tgt_queue,
        &mut tgt,
        Duration::from_secs(5),
        |state| state.data_device_manager.is_some() && state.seat.is_some(),
    );
    let tgt_dd = tgt.data_device_manager.clone().unwrap().get_data_device(
        &tgt.seat.clone().unwrap(),
        &tgt_qh,
        (),
    );
    tgt.data_device = Some(tgt_dd.clone());
    // Force the compositor to register the target's data device before the
    // drag begins; `update_focus` only offers to devices it already knows,
    // and the synthetic input and the Wayland request are different channels.
    tgt_queue
        .roundtrip(&mut tgt)
        .expect("target data device roundtrip");

    // --- source: map a window and arm a text/uri-list drag source --------
    let (src_conn, mut src_queue, mut src) = connect(&proc.socket_path);
    wait_for(
        &src_conn,
        &mut src_queue,
        &mut src,
        Duration::from_secs(5),
        |state| state.pointer.is_some() && state.data_device_manager.is_some(),
    );
    let (src_surface, _src_xdg, _src_toplevel, _src_file) = map_toplevel(
        &mut src,
        &mut src_queue,
        "Drag Source",
        "org.dragonfruit.DragSource",
    );
    let payload = "file:///tmp/dragonfruit-drag-source.txt\r\n".to_string();
    src.source_payload = Some(payload.clone());
    let src_qh = src_queue.handle();
    let src_source = src
        .data_device_manager
        .clone()
        .unwrap()
        .create_data_source(&src_qh, ());
    src_source.offer("text/uri-list".to_string());
    src_source.offer("application/x-dragonfruit-app".to_string());
    src_source.set_actions(DndAction::Copy | DndAction::Move);
    src.data_source = Some(src_source.clone());
    let src_dd = src.data_device_manager.clone().unwrap().get_data_device(
        &src.seat.clone().unwrap(),
        &src_qh,
        (),
    );
    src.data_device = Some(src_dd.clone());

    // Pointer over the source window; the button press is the implicit grab
    // `start_drag` validates against.
    input.send("motion-abs 0.5 0.5");
    wait_for(
        &src_conn,
        &mut src_queue,
        &mut src,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );
    input.send("button 272 down"); // BTN_LEFT
    wait_for(
        &src_conn,
        &mut src_queue,
        &mut src,
        Duration::from_secs(5),
        |state| state.pointer_button_serial.is_some(),
    );
    let serial = src.pointer_button_serial.unwrap();
    src_dd.start_drag(Some(&src_source), &src_surface, None, serial);
    src_conn.flush().expect("flush start_drag");
    // `start_drag` installs the DnD grab with `Focus::Clear`, which sends a
    // pointer leave to the source window. Wait for it before moving: the
    // grab only re-evaluates focus on a subsequent motion, and the synthetic
    // input and the Wayland request travel different channels, so moving
    // first could race the grab into place and the drag would never enter
    // the target.
    wait_for(
        &src_conn,
        &mut src_queue,
        &mut src,
        Duration::from_secs(5),
        |state| state.pointer_leaves > 0,
    );

    // --- drag onto the chrome bar and drop -------------------------------
    input.send("motion-abs 0.5 0.01");
    wait_for(
        &tgt_conn,
        &mut tgt_queue,
        &mut tgt,
        Duration::from_secs(5),
        |state| state.dnd_enters > 0,
    );
    assert!(
        tgt.dnd_mimes.iter().any(|m| m == "text/uri-list"),
        "the offer must advertise text/uri-list: {:?}",
        tgt.dnd_mimes
    );
    assert!(
        tgt.dnd_mimes
            .iter()
            .any(|m| m == "application/x-dragonfruit-app"),
        "the offer must advertise the app-alias mime the shell accepts: {:?}",
        tgt.dnd_mimes
    );
    // Wait until the compositor has processed `accept`/`set_actions` (proven
    // by the offer's `action` event) before the drop, or the drop is denied
    // as unvalidated.
    tgt_conn.flush().expect("flush accept/set_actions");
    wait_for(
        &tgt_conn,
        &mut tgt_queue,
        &mut tgt,
        Duration::from_secs(5),
        |state| state.dnd_action_events > 0,
    );
    input.send("button 272 up");
    wait_for(
        &tgt_conn,
        &mut tgt_queue,
        &mut tgt,
        Duration::from_secs(5),
        |state| state.dnd_drops > 0,
    );
    // Flush the target's `receive` so the compositor asks the source for the
    // bytes, then let the source service it.
    tgt_conn.flush().expect("flush receive");
    wait_for(
        &src_conn,
        &mut src_queue,
        &mut src,
        Duration::from_secs(5),
        |state| state.data_source_sent > 0,
    );
    let received = read_pipe_to_string(
        tgt.dnd_read_fd.expect("target read end"),
        Duration::from_secs(5),
    );
    assert_eq!(
        received, payload,
        "the drag payload must reach the chrome target intact"
    );
    assert_eq!(tgt.dnd_drops, 1, "exactly one drop must be delivered");

    drop(tgt_dd);
    drop(src_dd);
    drop(src_source);
    drop(tgt_layer);
    drop(tgt_surface);
    drop(tgt_shell);
    drop(tgt_core);
    let _ = tgt_conn.flush();
    let _ = src_conn.flush();
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

/// T-10 section 18 / acceptance ("Multi-output: Dock appears on hotplug"): the
/// Dock is a `top` chrome surface that targets every output, exactly like the
/// menu bar. Attaching a second display must announce it, report the Dock's
/// baseline reserved zone **on that display** (not merely somewhere), and
/// reconfigure the Dock surface; detaching must remove the display and its
/// Spaces without disturbing the Dock. The synthetic-output harness makes the
/// hotplug scriptable headlessly.
#[test]
fn dock_follows_output_hotplug() {
    let token = "de".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let output_path = PathBuf::from(&runtime_dir)
        .join(format!("dragonfruit-dock-hotplug-{}", std::process::id()));
    let proc = CompositorProcess::start_with_harnesses(
        "dragonfruit-conformance-dock-hotplug",
        std::slice::from_ref(&token),
        None,
        Some(&output_path),
    );
    let outputs = SyntheticOutput::connect(&output_path);

    let (conn, mut queue, mut state) = connect(&proc.socket_path);
    let (core, shell, bar_surface, bar) =
        open_trusted_bar(&conn, &mut queue, &mut state, &token, 28, 28);
    // The reserved zones arrive as `df_output` events, so the manager must be
    // bound to observe them (the same client can be shell and observer).
    let (manager_name, manager_version) = state.manager_global.expect("manager advertised");
    let manager = bind_manager(&mut state, &queue, manager_name, manager_version);
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| !state.outputs.is_empty() && state.done_count > 0,
    );

    // The Dock: bottom | left | right, 95 px tall, reserving the 60 px
    // baseline bar (the shell's own shape, T-10 section 2).
    let dock_surface = state
        .compositor
        .clone()
        .unwrap()
        .create_surface(&queue.handle(), ());
    let dock = shell.get_layer_surface(
        &dock_surface,
        None,
        df_shell::Layer::Top,
        "dock".to_string(),
        &queue.handle(),
        (),
    );
    dock.set_anchor(2 | 4 | 8); // bottom | left | right
    dock.set_size(0, 95);
    dock.set_exclusive_zone(60);
    dock.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::None);
    dock_surface.commit();

    // The static headless output gets both reserves (top bar + bottom Dock).
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

    let dock_id = dock.id().protocol_id();
    let dock_configures_before = state
        .layer_configures_by_id
        .iter()
        .filter(|(id, _, _)| *id == dock_id)
        .count();
    assert!(
        dock_configures_before >= 1,
        "the Dock configures at creation"
    );

    // --- attach a second output -------------------------------------------
    outputs.send("add HDMI-A-1 1920 1080 1280 0");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .output_name_by_id
                .iter()
                .any(|(_, name)| name == "HDMI-A-1")
        },
    );
    let hdmi_id = state
        .output_name_by_id
        .iter()
        .find(|(_, name)| name == "HDMI-A-1")
        .map(|(id, _)| *id)
        .expect("the hotplugged output has a name");

    // The new output specifically carries the Dock's baseline reserve (and the
    // menu bar's top reserve), which is what "the Dock appears on hotplug"
    // means at the protocol layer.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .output_reserved_by_id
                .iter()
                .any(|(id, edge, thickness)| *id == hdmi_id && *edge == 0 && *thickness == 28)
                && state
                    .output_reserved_by_id
                    .iter()
                    .any(|(id, edge, thickness)| *id == hdmi_id && *edge == 1 && *thickness == 60)
        },
    );
    // And the Dock surface is reconfigured for the new scene.
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .layer_configures_by_id
                .iter()
                .filter(|(id, _, _)| *id == dock_id)
                .count()
                > dock_configures_before
        },
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

    drop(dock);
    drop(dock_surface);
    drop(bar);
    drop(bar_surface);
    drop(shell);
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

/// A right-click on a chrome surface takes keyboard focus too, so a Dock
/// context menu can be dismissed by click-away/focus loss (T-10 section 13).
#[test]
fn chrome_surface_takes_keyboard_focus_on_right_click() {
    let token = "de".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-chrome-right-click-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-chrome-right-click",
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
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some() && state.keyboard.is_some(),
    );
    let _ = queue.roundtrip(&mut state);

    // Move over the bar and right-click (BTN_RIGHT == 273). A right-click on
    // chrome must take keyboard focus, unlike the window path which is
    // left-click only.
    input.send("motion-abs 0.5 0.01");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );
    state.keyboard_enters = 0;
    input.send("button 273 down\nbutton 273 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.keyboard_enters > 0,
    );

    // A left-click off the chrome (empty desktop) drops the focus.
    state.keyboard_leaves = 0;
    input.send("motion-abs 0.5 0.9\nbutton 272 down\nbutton 272 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.keyboard_leaves > 0,
    );

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

/// T-10 section 18 (compositor half): an overlay popover created without an
/// explicit output is **per-output**. It renders and hit-tests only on the
/// output chrome was last focused on, so a menu/Dock popover opened on one
/// display does not float across every display. Chrome focus captures the
/// interaction output from the pointer (a click on a chrome surface); the
/// popover is then compared on the focused output against a second,
/// hotplugged output.
#[test]
fn overlay_popover_is_per_output() {
    let token = "f0".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let input_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-per-output-input-{}",
        std::process::id()
    ));
    let output_path = PathBuf::from(&runtime_dir).join(format!(
        "dragonfruit-per-output-output-{}",
        std::process::id()
    ));
    let proc = CompositorProcess::start_with_harnesses(
        "dragonfruit-conformance-per-output",
        std::slice::from_ref(&token),
        Some(&input_path),
        Some(&output_path),
    );
    let input = SyntheticInput::connect(&input_path);
    let outputs = SyntheticOutput::connect(&output_path);
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

    // A second output to the right of the 1280x720 headless output.
    outputs.send("add HDMI-A-1 1920 1080 1280 0");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.output_names.iter().any(|name| name == "HDMI-A-1"),
    );

    // The bar: top | left | right, 28 px, reserved, OnDemand (a click focuses).
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

    // The popover: overlay top | left, offset 40/28, no reserve, no keyboard.
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
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some() && state.keyboard.is_some(),
    );

    // Click the bar on the first output so chrome focus captures HEADLESS-1.
    input.send("motion-abs 0.109375 0.0138889"); // (140, 10) on HEADLESS-1
    input.send("button 272 down");
    input.send("button 272 up");
    let _ = queue.roundtrip(&mut state);

    // On the focused output the popover is hit at popup-local (100, 72).
    state.pointer_enters = 0;
    state.pointer_enter_positions.clear();
    input.send("motion 0 90"); // (140, 100) on HEADLESS-1
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
        .expect("popover enter position");
    assert!(
        (mx - 100.0).abs() < 1.0 && (my - 72.0).abs() < 1.0,
        "the popover must be hit on the focused output: got ({mx}, {my})"
    );

    // Move into the same popup rectangle on the second output. The popover
    // must not be there: a follow-up move onto HDMI-A-1's bar is the
    // observable "processed" signal, and the popover must not have received
    // an enter at (1420, 100) first.
    state.pointer_enters = 0;
    state.pointer_enter_positions.clear();
    input.send("motion 1280 0"); // (1420, 100) on HDMI-A-1, inside the popup rect
    input.send("motion 0 -90"); // (1420, 10) onto HDMI-A-1's bar
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer_enters > 0,
    );
    assert!(
        state
            .pointer_enter_positions
            .iter()
            .all(|(x, y)| (x - 100.0).abs() >= 1.0 || (y - 72.0).abs() >= 1.0),
        "the popover must not float onto the second output: {:?}",
        state.pointer_enter_positions
    );

    // Returning to the focused output hits the popover again.
    state.pointer_enters = 0;
    state.pointer_enter_positions.clear();
    input.send("motion -1280 90"); // (140, 100) on HEADLESS-1
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
        .expect("popover re-enter position");
    assert!(
        (mx - 100.0).abs() < 1.0 && (my - 72.0).abs() < 1.0,
        "the popover must stay on the focused output: got ({mx}, {my})"
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
        !input_path.exists(),
        "teardown leak: synthetic-input socket survived"
    );
    assert!(
        !output_path.exists(),
        "teardown leak: synthetic-output socket survived"
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

/// T-10 section 20 (compositor half): the `focus-dock` system shortcut hands
/// the keyboard to the `dock` chrome surface, and `toggle-dock` reaches the
/// shell as an input action. The Dock is `OnDemand`, so it accepts focus
/// when the shortcut targets it (unlike a `None` surface).
#[test]
fn focus_dock_shortcut_hands_the_keyboard_to_the_dock() {
    let token = "dc".repeat(32);
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let synthetic_path =
        PathBuf::from(&runtime_dir).join(format!("dragonfruit-dock-focus-{}", std::process::id()));
    let proc = CompositorProcess::start_with_synthetic(
        "dragonfruit-conformance-dock-focus",
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

    // A Dock surface that can take keyboard focus on demand.
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
    dock.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::OnDemand);
    dock_surface.commit();

    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.pointer.is_some() && state.keyboard.is_some(),
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

    // --- FocusDock: Ctrl+F3 (evdev KEY_LEFTCTRL=29, KEY_F3=61) ----------
    state.keyboard_enters = 0;
    state.keyboard_leaves = 0;
    input.send("key 29 down\nkey 61 down\nkey 61 up\nkey 29 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .input_actions
                .iter()
                .any(|(action, source, _)| action == "focus-dock" && source == "keyboard")
        },
    );
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.keyboard_enters > 0,
    );

    // --- ToggleDock: Super+Option+D (evdev LEFTMETA=125, LEFTALT=56, D=32)
    state.input_actions.clear();
    input.send("key 125 down\nkey 56 down\nkey 32 down\nkey 32 up\nkey 56 up\nkey 125 up");
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| {
            state
                .input_actions
                .iter()
                .any(|(action, source, _)| action == "toggle-dock" && source == "keyboard")
        },
    );

    // --- ReleaseKeyboardFocus: Escape leaves Dock keyboard navigation ----
    // The Dock still holds the keyboard after FocusDock; the shell's release
    // request returns it to the active window (none here), so the Dock sees
    // `wl_keyboard.leave` and the compositor no longer counts it as focused.
    state.keyboard_leaves = 0;
    manager.release_keyboard_focus();
    let _ = conn.flush();
    wait_for(
        &conn,
        &mut queue,
        &mut state,
        Duration::from_secs(5),
        |state| state.keyboard_leaves > 0,
    );

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
