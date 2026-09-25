// SPDX-License-Identifier: MIT
#![allow(dead_code)] // Forward-looking protocol API consumed by T-09..T-29.

//! Private shell protocol handling (T-07).
//!
//! This module owns the compositor side of the lockstep private protocols:
//! the `df_core` handshake and trust model ([`trust`]), the chrome-surface
//! factory, and the window/workspace/output manager. The generated
//! bindings live in [`crate::shell_protocol`]; the request handlers and
//! event broadcasts live here.
//!
//! Event ordering is scene-consistent (FR-2): the manager announces an
//! object, sends its initial properties, then `done`, and every mutating
//! request is acked with `done` after the scene applies it. The shell is a
//! pure consumer — it never keeps a second copy of compositor state.

pub mod layer;
pub mod trust;

use std::collections::HashMap;

use smithay::output::{Mode, Output, Scale as OutputScale};
use smithay::utils::{Logical, Point, Rectangle, Transform, SERIAL_COUNTER};
use smithay::wayland::seat::WaylandFocus as _;
use wayland_server::backend::{ClientId, GlobalId};
use wayland_server::protocol::wl_surface::WlSurface;
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

use crate::app_switcher::SwitchStep;
use crate::input::action::{HotCorner, TriggerKind};
use crate::input::gestures::ProgressPhase;
use crate::input::ShellInputEvent;
use crate::shell_protocol::core_protocol::df_core;
use crate::shell_protocol::shell_protocol::{df_layer_surface, df_shell};
use crate::shell_protocol::toplevel_protocol::{
    df_output, df_toplevel, df_toplevel_manager, df_workspace,
};
use crate::state::DfState;
use crate::window::{WindowEventKind, WindowId, WindowState};
use crate::workspace::{SpaceId, Wallpaper, WallpaperFit};

use layer::{aggregate_reserved, Edge, KeyboardInteraction, LayerSurfaceState};
use trust::{Refusal, TrustModel, TrustedRole};

/// The version of every private interface this compositor implements.
const INTERFACE_VERSION: u32 = 1;

/// `df_toplevel_manager` is version 5 since `set_motion_policy` and
/// `set_input_policy` were added (T-08.2c); `set_launch_origin` was the v4
/// addition (T-02.1b), `set_reduced_motion` the v3 (T-11), and
/// `release_keyboard_focus` the v2 (T-10). The other interfaces stay at
/// [`INTERFACE_VERSION`].
const MANAGER_INTERFACE_VERSION: u32 = 5;

/// Error code posted when an untrusted client binds a private global.
const ERROR_ACCESS_DENIED: u32 = 1;

/// One authenticated client's private-protocol objects.
#[derive(Debug, Default)]
struct ClientSession {
    role: Option<TrustedRole>,
    factory: Option<df_shell::DfShell>,
    manager: Option<df_toplevel_manager::DfToplevelManager>,
    toplevels: HashMap<WindowId, df_toplevel::DfToplevel>,
    workspaces: HashMap<SpaceId, df_workspace::DfWorkspace>,
    outputs: HashMap<String, df_output::DfOutput>,
}

/// A chrome surface and its live state.
#[derive(Debug)]
struct LayerEntry {
    resource: df_layer_surface::DfLayerSurface,
    surface: WlSurface,
    state: LayerSurfaceState,
}

/// A chrome surface ready to composite on one output (T-09).
#[derive(Debug, Clone)]
pub struct ChromeSurface {
    pub surface: WlSurface,
    /// Output-local logical position.
    pub location: Point<i32, Logical>,
    /// The surface's full output-local logical rectangle (`location` plus its
    /// configured size). The compositor composites by `location`.
    pub geometry: Rectangle<i32, Logical>,
    /// The output-local rect the surface actually paints — the visible chrome
    /// the backdrop material (T-04.2) belongs behind. For the Dock this is the
    /// bar slab, not the transparent magnification band above it. See
    /// [`crate::window::panel_bounds`].
    pub panel: Rectangle<i32, Logical>,
    /// The layer the surface requested (`df_shell.layer`).
    pub layer: u32,
    /// Keyboard-interaction policy (`df_layer_surface.set_keyboard_interaction`).
    pub keyboard: KeyboardInteraction,
}

/// The compositor's private-shell-protocol state.
#[derive(Debug)]
pub struct ShellProtocolState {
    pub trust: TrustModel,
    pub core_global: GlobalId,
    pub shell_global: GlobalId,
    pub toplevel_global: GlobalId,
    sessions: HashMap<ClientId, ClientSession>,
    layers: Vec<LayerEntry>,
    /// Reserved zones aggregated from chrome surfaces (menu bar, Dock).
    pub reserved: crate::window::ReservedZones,
    /// The output a chrome surface last took keyboard focus on. Transient
    /// `overlay` chrome with no explicit output (popovers, menus, OSD) is
    /// shown only on this output, so it never floats across displays (T-10
    /// section 18). `None` before any chrome focus.
    pub chrome_focus_output: Option<String>,
    /// Windows that asked for activation and have not been seen yet.
    attention: Vec<WindowId>,
}

impl ShellProtocolState {
    /// Register the private globals. `df_core` is advertised to everyone;
    /// the others are advertised but refuse untrusted binds.
    pub fn new(display: &DisplayHandle) -> Self {
        let core_global =
            display.create_global::<DfState, df_core::DfCore, ()>(INTERFACE_VERSION, ());
        let shell_global =
            display.create_global::<DfState, df_shell::DfShell, ()>(INTERFACE_VERSION, ());
        let toplevel_global = display
            .create_global::<DfState, df_toplevel_manager::DfToplevelManager, ()>(
                MANAGER_INTERFACE_VERSION,
                (),
            );
        ShellProtocolState {
            trust: TrustModel::new(),
            core_global,
            shell_global,
            toplevel_global,
            sessions: HashMap::new(),
            layers: Vec::new(),
            reserved: crate::window::ReservedZones::default(),
            chrome_focus_output: None,
            attention: Vec::new(),
        }
    }

    /// Whether `client` completed the handshake.
    pub fn is_trusted(&self, client: &ClientId) -> bool {
        self.sessions.contains_key(client)
    }

    /// The role granted to `client`, if trusted.
    pub fn role(&self, client: &ClientId) -> Option<TrustedRole> {
        self.sessions.get(client).and_then(|session| session.role)
    }

    /// Drop a client's protocol objects when it disconnects.
    fn forget_client(&mut self, client: &ClientId) {
        self.sessions.remove(client);
        // Chrome surfaces owned by the client no longer reserve zones.
        self.layers
            .retain(|entry| entry.resource.client().map(|c| c.id()) != Some(client.clone()));
    }
}

// --- user data -------------------------------------------------------------

/// User data for the `df_core` handshake object.
#[derive(Debug, Default)]
pub struct CoreUserData;

/// User data for a `df_layer_surface`.
#[derive(Debug)]
pub struct LayerUserData;

/// User data for a `df_toplevel` handle.
#[derive(Debug, Clone, Copy)]
pub struct ToplevelUserData {
    pub id: WindowId,
}

/// User data for a `df_workspace` handle.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceUserData {
    pub id: SpaceId,
}

/// User data for a `df_output` handle.
#[derive(Debug, Clone)]
pub struct OutputUserData {
    pub name: String,
}

// --- helpers ---------------------------------------------------------------

fn argb_to_rgba(color: u32) -> [f32; 4] {
    let a = ((color >> 24) & 0xff) as f32 / 255.0;
    let r = ((color >> 16) & 0xff) as f32 / 255.0;
    let g = ((color >> 8) & 0xff) as f32 / 255.0;
    let b = (color & 0xff) as f32 / 255.0;
    [r, g, b, a.max(1.0 / 255.0)]
}

fn rgba_to_argb(color: [f32; 4]) -> u32 {
    let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    (channel(color[3]) << 24)
        | (channel(color[0]) << 16)
        | (channel(color[1]) << 8)
        | channel(color[2])
}

fn wallpaper_fit_from_wire(value: u32) -> WallpaperFit {
    match value {
        1 => WallpaperFit::Fit,
        2 => WallpaperFit::Stretch,
        3 => WallpaperFit::Center,
        _ => WallpaperFit::Fill,
    }
}

fn wallpaper_fit_wire(fit: WallpaperFit) -> df_workspace::WallpaperFit {
    match fit {
        WallpaperFit::Fill => df_workspace::WallpaperFit::Fill,
        WallpaperFit::Fit => df_workspace::WallpaperFit::Fit,
        WallpaperFit::Stretch => df_workspace::WallpaperFit::Stretch,
        WallpaperFit::Center => df_workspace::WallpaperFit::Center,
    }
}

fn transform_from_wire(value: u32) -> Transform {
    match value {
        1 => Transform::_90,
        2 => Transform::_180,
        3 => Transform::_270,
        4 => Transform::Flipped,
        5 => Transform::Flipped90,
        6 => Transform::Flipped180,
        7 => Transform::Flipped270,
        _ => Transform::Normal,
    }
}

fn transform_wire(transform: Transform) -> df_output::Transform {
    match transform {
        Transform::Normal => df_output::Transform::Normal,
        Transform::_90 => df_output::Transform::_90,
        Transform::_180 => df_output::Transform::_180,
        Transform::_270 => df_output::Transform::_270,
        Transform::Flipped => df_output::Transform::Flipped,
        Transform::Flipped90 => df_output::Transform::Flipped90,
        Transform::Flipped180 => df_output::Transform::Flipped180,
        Transform::Flipped270 => df_output::Transform::Flipped270,
    }
}

fn progress_phase_wire(phase: ProgressPhase) -> u32 {
    match phase {
        ProgressPhase::Begin => 0,
        ProgressPhase::Update => 1,
        ProgressPhase::End => 2,
    }
}

fn hot_corner_wire(corner: HotCorner) -> df_toplevel_manager::HotCorner {
    match corner {
        HotCorner::TopLeft => df_toplevel_manager::HotCorner::TopLeft,
        HotCorner::TopRight => df_toplevel_manager::HotCorner::TopRight,
        HotCorner::BottomLeft => df_toplevel_manager::HotCorner::BottomLeft,
        HotCorner::BottomRight => df_toplevel_manager::HotCorner::BottomRight,
    }
}

fn edge_wire(edge: Edge) -> df_output::Edge {
    match edge {
        Edge::Top => df_output::Edge::Top,
        Edge::Bottom => df_output::Edge::Bottom,
        Edge::Left => df_output::Edge::Left,
        Edge::Right => df_output::Edge::Right,
    }
}

/// Sanitize a client-controlled string before it is serialized into a
/// private-protocol event. The generated server bindings build a `CString`
/// and `unwrap` it, so an interior NUL panics the compositor; X11
/// `WM_NAME`/`WM_CLASS` are conventionally NUL-terminated and a client can
/// embed one. The window model may still hold the raw bytes (identity,
/// logging); only the wire encoding is sanitized.
fn protocol_string(value: &str) -> String {
    value.replace('\0', "")
}

fn protocol_string_opt(value: Option<String>) -> Option<String> {
    value.map(|value| protocol_string(&value))
}

// --- DfState integration ---------------------------------------------------

impl DfState {
    /// A stable window lookup by compositor id.
    pub(crate) fn window_by_id(&self, id: WindowId) -> Option<smithay::desktop::Window> {
        self.windows
            .windows()
            .find(|window| self.windows.id(window) == Some(id))
            .cloned()
    }

    /// Focus and raise a window by compositor id (shell-driven activation).
    pub fn activate_window_id(&mut self, id: WindowId) {
        let Some(window) = self.window_by_id(id) else {
            return;
        };
        if self.windows.state(&window) == Some(WindowState::Minimized) {
            self.restore_window(&window);
        }
        // Make sure its Space is showing, then focus it.
        let index = self
            .windows
            .id(&window)
            .and_then(|id| self.workspaces.window_space(id))
            .and_then(|space| {
                let output = self.workspaces.space_output(space)?;
                let ids = self.workspaces.space_ids(output);
                ids.iter().position(|candidate| *candidate == space)
            });
        if let Some(index) = index {
            if self.workspaces.activate_all(index) {
                self.after_workspace_change();
            }
        }
        let geometry = self.windows.geometry(&window).unwrap_or_default();
        if self.window_on_active_space(&window) {
            self.space.map_element(window.clone(), geometry.loc, true);
        }
        if let Some(surface) = window.wl_surface() {
            if let Some(keyboard) = self.seat.get_keyboard() {
                keyboard.set_focus(
                    self,
                    Some(surface.into_owned()),
                    SERIAL_COUNTER.next_serial(),
                );
            }
        }
        self.needs_redraw = true;
    }

    /// Record an activation/attention request for the shell (Dock bounce).
    pub fn notify_attention(&mut self, window: &smithay::desktop::Window) {
        if let Some(id) = self.windows.id(window) {
            if !self.shell.attention.contains(&id) {
                self.shell.attention.push(id);
            }
        }
    }

    /// Recompute reserved zones from the current chrome surfaces.
    pub fn refresh_reserved_zones(&mut self) {
        let zones = aggregate_reserved(
            self.shell.layers.iter().map(|entry| &entry.state),
            crate::window::ReservedZones::default(),
        );
        if zones != self.shell.reserved {
            self.shell.reserved = zones;
            self.reserved_zones = zones;
            self.needs_redraw = true;
            self.broadcast_reserved_zones();
        }
    }

    // --- chrome surfaces ---------------------------------------------------

    fn layer_entry_mut(
        &mut self,
        resource: &df_layer_surface::DfLayerSurface,
    ) -> Option<&mut LayerEntry> {
        self.shell
            .layers
            .iter_mut()
            .find(|entry| entry.resource == *resource)
    }

    /// Output geometry for a chrome surface's target output.
    fn layer_output_geometry(&self, output: &Option<String>) -> Option<Rectangle<i32, Logical>> {
        let name = output.as_deref();
        for candidate in self.space.outputs() {
            if name.map_or(true, |name| candidate.name() == name) {
                return self.space.output_geometry(candidate);
            }
        }
        None
    }

    /// Chrome surfaces mapped to `output`, ordered bottom-to-top by layer.
    ///
    /// T-09 composites these above the window space. Background/bottom layer
    /// stacking (wallpaper/desktop reveal) is a T-10/T-11 concern. Transient
    /// `overlay` surfaces with no explicit output are per-output: they appear
    /// only on the output chrome was last focused on (T-10 section 18).
    pub fn chrome_surfaces(
        &self,
        output_name: &str,
        output_geometry: Rectangle<i32, Logical>,
    ) -> Vec<ChromeSurface> {
        let mut surfaces: Vec<ChromeSurface> = self
            .shell
            .layers
            .iter()
            .filter(|entry| {
                entry
                    .state
                    .visible_on_output(output_name, self.shell.chrome_focus_output.as_deref())
            })
            .map(|entry| {
                let geometry = entry.state.geometry(output_geometry);
                let geometry = Rectangle::new(
                    (
                        geometry.loc.x - output_geometry.loc.x,
                        geometry.loc.y - output_geometry.loc.y,
                    )
                        .into(),
                    geometry.size,
                );
                // The visible panel: the reserved bar strip, narrowed by the
                // part the client declares interactive. The Dock's input region
                // excludes its transparent magnification band, so this keeps the
                // backdrop off the band and off the area beside the bar.
                let region = smithay::wayland::compositor::with_states(&entry.surface, |states| {
                    states
                        .cached_state
                        .get::<smithay::wayland::compositor::SurfaceAttributes>()
                        .current()
                        .input_region
                        .clone()
                })
                .and_then(|region| crate::input::constraint::region_bounds(&region));
                let panel = crate::window::panel_bounds(
                    geometry,
                    entry.state.reserved_rect(geometry),
                    region,
                );
                ChromeSurface {
                    surface: entry.surface.clone(),
                    location: geometry.loc,
                    geometry,
                    panel,
                    layer: entry.state.layer,
                    keyboard: entry.state.keyboard,
                }
            })
            .collect();
        surfaces.sort_by_key(|chrome| chrome.layer);
        surfaces
    }

    /// Recompute and send `configure` for a chrome surface.
    fn configure_layer(&mut self, resource: &df_layer_surface::DfLayerSurface) {
        let output_name = self
            .layer_entry_mut(resource)
            .map(|entry| entry.state.output.clone());
        let Some(output_name) = output_name else {
            return;
        };
        let Some(output) = self.layer_output_geometry(&output_name) else {
            return;
        };
        let Some(entry) = self.layer_entry_mut(resource) else {
            return;
        };
        let geometry = entry.state.geometry(output);
        let size = (geometry.size.w, geometry.size.h);
        entry.state.configured = Some(size);
        let serial = SERIAL_COUNTER.next_serial().into();
        resource.configure(serial, size.0, size.1);
    }

    /// Reconfigure every chrome surface (output hotplug/resize).
    pub fn reconfigure_layers(&mut self) {
        let resources: Vec<_> = self
            .shell
            .layers
            .iter()
            .map(|entry| entry.resource.clone())
            .collect();
        for resource in resources {
            self.configure_layer(&resource);
        }
        self.refresh_reserved_zones();
    }

    /// Whether `surface` belongs to a chrome (`df_layer_surface`) surface.
    pub fn is_chrome_surface(&self, surface: &WlSurface) -> bool {
        self.shell
            .layers
            .iter()
            .any(|entry| entry.surface == *surface)
    }

    /// The keyboard policy of a chrome surface, if it is one.
    pub fn chrome_keyboard_interaction(&self, surface: &WlSurface) -> Option<KeyboardInteraction> {
        self.shell
            .layers
            .iter()
            .find(|entry| entry.surface == *surface)
            .map(|entry| entry.state.keyboard)
    }

    /// The first chrome surface in the `namespace` group (T-10: the Dock is
    /// the only `"dock"` surface). Used to hand the Dock keyboard focus from
    /// the FocusDock shortcut or the compositor's own input routing.
    pub fn chrome_surface_by_namespace(&self, namespace: &str) -> Option<WlSurface> {
        self.shell
            .layers
            .iter()
            .find(|entry| entry.state.namespace == namespace)
            .map(|entry| entry.surface.clone())
    }

    /// Move keyboard focus into the Dock (T-10 section 20). Returns whether a
    /// Dock surface was found and accepted focus.
    pub fn focus_dock(&mut self) -> bool {
        let Some(surface) = self.chrome_surface_by_namespace("dock") else {
            return false;
        };
        self.focus_chrome_surface(&surface)
    }

    /// Whether a chrome surface currently holds the keyboard.
    pub fn chrome_has_keyboard_focus(&self) -> bool {
        self.seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus())
            .is_some_and(|surface| self.is_chrome_surface(&surface))
    }

    /// Give keyboard focus to a chrome surface, if its policy allows it.
    ///
    /// Returns true when focus was moved. `KeyboardInteraction::None` never
    /// takes focus; `OnDemand` and `Exclusive` do (T-07 FR-1).
    pub fn focus_chrome_surface(&mut self, surface: &WlSurface) -> bool {
        if self.chrome_keyboard_interaction(surface) == Some(KeyboardInteraction::None)
            || !self.is_chrome_surface(surface)
        {
            return false;
        }
        let Some(keyboard) = self.seat.get_keyboard() else {
            return false;
        };
        keyboard.set_focus(self, Some(surface.clone()), SERIAL_COUNTER.next_serial());
        true
    }

    /// Return keyboard focus to the active window after chrome closes or
    /// releases focus. Uses the window focus preserved while chrome held the
    /// keyboard (see `focus_changed`).
    pub fn restore_window_keyboard_focus(&mut self) {
        let target = self
            .active_window
            .as_ref()
            .and_then(|window| window.wl_surface().map(|surface| surface.into_owned()));
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.set_focus(self, target, SERIAL_COUNTER.next_serial());
        }
    }

    // --- scene replay ------------------------------------------------------

    fn manager_sessions(&self) -> Vec<(ClientId, df_toplevel_manager::DfToplevelManager)> {
        self.shell
            .sessions
            .iter()
            .filter_map(|(client, session)| session.manager.clone().map(|m| (client.clone(), m)))
            .collect()
    }

    fn session_outputs(&self) -> Vec<(String, Output)> {
        self.space
            .outputs()
            .map(|output| (output.name(), output.clone()))
            .collect()
    }

    /// Announce every output to one manager and send its properties.
    fn announce_outputs(
        &mut self,
        client: &Client,
        manager: &df_toplevel_manager::DfToplevelManager,
    ) {
        let outputs = self.session_outputs();
        for (name, output) in outputs {
            if self
                .shell
                .sessions
                .get(&client.id())
                .is_some_and(|session| session.outputs.contains_key(&name))
            {
                continue;
            }
            let Ok(resource) = client.create_resource::<df_output::DfOutput, _, DfState>(
                &self.display_handle,
                INTERFACE_VERSION,
                OutputUserData { name: name.clone() },
            ) else {
                continue;
            };
            manager.output(&resource);
            self.send_output_properties(&resource, &output);
            resource.done();
            if let Some(session) = self.shell.sessions.get_mut(&client.id()) {
                session.outputs.insert(name, resource);
            }
        }
    }

    fn send_output_properties(&self, resource: &df_output::DfOutput, output: &Output) {
        resource.name(output.name());
        if let Some(geometry) = self.space.output_geometry(output) {
            resource.geometry(
                geometry.loc.x,
                geometry.loc.y,
                geometry.size.w,
                geometry.size.h,
            );
        }
        let scale = output.current_scale().fractional_scale();
        resource.scale(scale);
        resource.transform(transform_wire(output.current_transform()));
        if let Some(mode) = output.current_mode() {
            resource.mode(
                0,
                mode.size.w as u32,
                mode.size.h as u32,
                mode.refresh.max(0) as u32,
            );
        }
        resource.vrr(0);
        resource.night_light(0, 6500);
        let zones = self.shell.reserved;
        for (edge, thickness) in [
            (Edge::Top, zones.top),
            (Edge::Bottom, zones.bottom),
            (Edge::Left, zones.left),
            (Edge::Right, zones.right),
        ] {
            if thickness > 0 {
                resource.reserved_zone(edge_wire(edge), thickness as u32);
            }
        }
    }

    /// Announce every workspace to one manager.
    fn announce_workspaces(
        &mut self,
        client: &Client,
        manager: &df_toplevel_manager::DfToplevelManager,
    ) {
        let slots = self.workspace_slots();
        for (space, output, index, name, fullscreen, active, wallpaper) in slots {
            if self
                .shell
                .sessions
                .get(&client.id())
                .is_some_and(|session| session.workspaces.contains_key(&space))
            {
                continue;
            }
            let Ok(resource) = client.create_resource::<df_workspace::DfWorkspace, _, DfState>(
                &self.display_handle,
                INTERFACE_VERSION,
                WorkspaceUserData { id: space },
            ) else {
                continue;
            };
            manager.workspace(&resource);
            resource.name(name);
            resource.index(index as u32);
            resource.activated(active as u32);
            resource.fullscreen(fullscreen as u32);
            resource.wallpaper(
                protocol_string_opt(wallpaper.source.clone()),
                wallpaper_fit_wire(wallpaper.fit),
                rgba_to_argb(wallpaper.color),
            );
            resource.done();
            let _ = output;
            if let Some(session) = self.shell.sessions.get_mut(&client.id()) {
                session.workspaces.insert(space, resource);
            }
        }
    }

    /// Announce every window to one manager.
    fn announce_toplevels(
        &mut self,
        client: &Client,
        manager: &df_toplevel_manager::DfToplevelManager,
    ) {
        let windows: Vec<(WindowId, Option<String>, Option<String>, WindowState)> = self
            .windows
            .windows()
            .filter_map(|window| {
                let id = self.windows.id(window)?;
                Some((
                    id,
                    self.windows.app_id(window).map(str::to_string),
                    self.windows.title(window).map(str::to_string),
                    self.windows.state(window)?,
                ))
            })
            .collect();
        for (id, app_id, title, state) in windows {
            if self
                .shell
                .sessions
                .get(&client.id())
                .is_some_and(|session| session.toplevels.contains_key(&id))
            {
                continue;
            }
            let Ok(resource) = client.create_resource::<df_toplevel::DfToplevel, _, DfState>(
                &self.display_handle,
                INTERFACE_VERSION,
                ToplevelUserData { id },
            ) else {
                continue;
            };
            manager.toplevel(&resource);
            resource.title(protocol_string_opt(title.clone()));
            resource.app_id(protocol_string_opt(app_id.clone()));
            resource.state(self.window_state_flags(id, state));
            if let Some(space) = self.workspaces.window_space(id) {
                if let Some(workspace) = self.workspace_resource(client.id(), space) {
                    resource.workspace_entered(&workspace);
                }
            }
            if let Some(output) = self.window_output_name(id) {
                if let Some(output_resource) = self.output_resource(client.id(), &output) {
                    resource.output_entered(&output_resource);
                }
            }
            resource.done();
            if let Some(session) = self.shell.sessions.get_mut(&client.id()) {
                session.toplevels.insert(id, resource);
            }
        }
    }

    fn window_state_flags(&self, id: WindowId, state: WindowState) -> df_toplevel::State {
        let mut flags = df_toplevel::State::empty();
        if state == WindowState::Minimized {
            flags |= df_toplevel::State::Minimized;
        }
        if state == WindowState::Zoomed {
            flags |= df_toplevel::State::Zoomed;
        }
        if state == WindowState::Fullscreen {
            flags |= df_toplevel::State::Fullscreen;
        }
        if self.active_window.as_ref().and_then(|w| self.windows.id(w)) == Some(id) {
            flags |= df_toplevel::State::Focused;
        }
        flags
    }

    fn workspace_resource(
        &self,
        client: ClientId,
        space: SpaceId,
    ) -> Option<df_workspace::DfWorkspace> {
        self.shell
            .sessions
            .get(&client)
            .and_then(|session| session.workspaces.get(&space).cloned())
    }

    fn output_resource(&self, client: ClientId, name: &str) -> Option<df_output::DfOutput> {
        self.shell
            .sessions
            .get(&client)
            .and_then(|session| session.outputs.get(name).cloned())
    }

    fn window_output_name(&self, id: WindowId) -> Option<String> {
        let window = self.window_by_id(id)?;
        self.output_name_for(&window)
    }

    /// `(space, output, index, name, fullscreen, active, wallpaper)` for every
    /// Space on every output, in output/order.
    #[allow(clippy::type_complexity)]
    fn workspace_slots(&self) -> Vec<(SpaceId, String, usize, String, bool, bool, Wallpaper)> {
        let mut slots = Vec::new();
        for output in self.space.outputs() {
            let name = output.name();
            for index in 0..self.workspaces.space_count(&name) {
                if let Some(info) = self.workspace_info(&name, index) {
                    slots.push(info);
                }
            }
        }
        slots
    }

    #[allow(clippy::type_complexity)]
    fn workspace_info(
        &self,
        output: &str,
        index: usize,
    ) -> Option<(SpaceId, String, usize, String, bool, bool, Wallpaper)> {
        let space = self.workspaces.space_at(output, index)?;
        let name = self.workspaces.space_names(output).get(index)?.clone();
        let wallpaper = self.workspaces.wallpaper_at(output, index)?.clone();
        let fullscreen = self.workspaces.is_fullscreen_at(output, index);
        let active = self.workspaces.active_space(output) == Some(space);
        Some((
            space,
            output.to_string(),
            index,
            name,
            fullscreen,
            active,
            wallpaper,
        ))
    }

    // --- broadcasts --------------------------------------------------------

    /// Drain every outbox and push the events to trusted managers. Called
    /// once per event-loop iteration (T-03/T-04/T-05 seam).
    pub fn broadcast_shell_events(&mut self) {
        self.broadcast_workspace_events();
        self.broadcast_window_events();
        self.broadcast_input_events();
        self.broadcast_attention();
    }

    fn broadcast_workspace_events(&mut self) {
        let events = self.workspaces.dispatch().drain();
        if events.is_empty() {
            return;
        }
        // Structural changes: resync the workspace objects from the model.
        let structural = events.iter().any(|event| {
            !matches!(
                event.kind,
                crate::workspace::WorkspaceEventKind::WindowAssigned { .. }
                    | crate::workspace::WorkspaceEventKind::Activated { .. }
            )
        });
        if structural {
            self.sync_outputs();
            self.sync_workspaces();
        }
        let sessions = self.manager_sessions();
        for event in &events {
            match event.kind {
                crate::workspace::WorkspaceEventKind::Activated { index, .. } => {
                    for (client, manager) in &sessions {
                        let _ = manager;
                        if let Some(space) = self.workspaces.space_at(&event.output, index) {
                            if let Some(resource) = self.workspace_resource(client.clone(), space) {
                                manager.workspace_activated(&resource, index as u32);
                            }
                        }
                    }
                }
                crate::workspace::WorkspaceEventKind::WindowAssigned { window, space } => {
                    for (client, _manager) in &sessions {
                        self.send_window_workspace(client.clone(), window, space);
                    }
                }
                _ => {}
            }
        }
        for (_, manager) in &sessions {
            manager.done();
        }
    }

    /// Reconcile the `df_output` objects with the scene (hotplug).
    fn sync_outputs(&mut self) {
        let sessions = self.manager_sessions();
        let outputs = self.session_outputs();
        let live: Vec<String> = outputs.iter().map(|(name, _)| name.clone()).collect();
        for (client, manager) in &sessions {
            let stale: Vec<String> = self
                .shell
                .sessions
                .get(client)
                .map(|session| {
                    session
                        .outputs
                        .keys()
                        .filter(|name| !live.contains(name))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            for name in stale {
                if let Some(session) = self.shell.sessions.get_mut(client) {
                    if let Some(resource) = session.outputs.remove(&name) {
                        resource.removed();
                    }
                }
            }
            for (name, output) in &outputs {
                if self
                    .shell
                    .sessions
                    .get(client)
                    .is_some_and(|session| session.outputs.contains_key(name))
                {
                    continue;
                }
                let Some(creator) = manager.client() else {
                    continue;
                };
                let Ok(resource) = creator.create_resource::<df_output::DfOutput, _, DfState>(
                    &self.display_handle,
                    INTERFACE_VERSION,
                    OutputUserData { name: name.clone() },
                ) else {
                    continue;
                };
                manager.output(&resource);
                self.send_output_properties(&resource, output);
                resource.done();
                if let Some(session) = self.shell.sessions.get_mut(client) {
                    session.outputs.insert(name.clone(), resource);
                }
            }
        }
    }

    /// Push the current reserved zones to every output handle.
    fn broadcast_reserved_zones(&mut self) {
        let sessions = self.manager_sessions();
        let zones = self.shell.reserved;
        for (client, _manager) in &sessions {
            let resources: Vec<df_output::DfOutput> = self
                .shell
                .sessions
                .get(client)
                .map(|session| session.outputs.values().cloned().collect())
                .unwrap_or_default();
            for resource in resources {
                for (edge, thickness) in [
                    (Edge::Top, zones.top),
                    (Edge::Bottom, zones.bottom),
                    (Edge::Left, zones.left),
                    (Edge::Right, zones.right),
                ] {
                    resource.reserved_zone(edge_wire(edge), thickness.max(0) as u32);
                }
                resource.done();
            }
        }
    }

    /// Reconcile the `df_workspace` objects with the model: announce new
    /// Spaces, drop removed ones, and refresh indices/activation.
    fn sync_workspaces(&mut self) {
        let sessions = self.manager_sessions();
        let slots = self.workspace_slots();
        let live: Vec<SpaceId> = slots.iter().map(|slot| slot.0).collect();
        for (client, manager) in &sessions {
            let client_id = client.clone();
            // Drop objects for Spaces that no longer exist.
            let stale: Vec<SpaceId> = self
                .shell
                .sessions
                .get(&client_id)
                .map(|session| {
                    session
                        .workspaces
                        .keys()
                        .copied()
                        .filter(|space| !live.contains(space))
                        .collect()
                })
                .unwrap_or_default();
            for space in stale {
                if let Some(session) = self.shell.sessions.get_mut(&client_id) {
                    if let Some(resource) = session.workspaces.remove(&space) {
                        resource.removed();
                    }
                }
            }
            for (space, _output, index, name, fullscreen, active, wallpaper) in &slots {
                let existing = self
                    .shell
                    .sessions
                    .get(&client_id)
                    .and_then(|session| session.workspaces.get(space).cloned());
                let resource = match existing {
                    Some(resource) => resource,
                    None => {
                        let Some(creator) = manager.client() else {
                            continue;
                        };
                        let Ok(resource) = creator
                            .create_resource::<df_workspace::DfWorkspace, _, DfState>(
                                &self.display_handle,
                                INTERFACE_VERSION,
                                WorkspaceUserData { id: *space },
                            )
                        else {
                            continue;
                        };
                        manager.workspace(&resource);
                        if let Some(session) = self.shell.sessions.get_mut(&client_id) {
                            session.workspaces.insert(*space, resource.clone());
                        }
                        resource
                    }
                };
                resource.name(name.clone());
                resource.index(*index as u32);
                resource.activated(*active as u32);
                resource.fullscreen(*fullscreen as u32);
                resource.wallpaper(
                    protocol_string_opt(wallpaper.source.clone()),
                    wallpaper_fit_wire(wallpaper.fit),
                    rgba_to_argb(wallpaper.color),
                );
                resource.done();
            }
        }
    }

    fn send_window_workspace(&mut self, client: ClientId, window: WindowId, space: SpaceId) {
        let resource = self
            .shell
            .sessions
            .get(&client)
            .and_then(|session| session.toplevels.get(&window).cloned());
        let Some(resource) = resource else {
            return;
        };
        if let Some(workspace) = self.workspace_resource(client, space) {
            resource.workspace_entered(&workspace);
        }
    }

    fn broadcast_window_events(&mut self) {
        // With no trusted manager there is no consumer; leave the bounded
        // outbox pending rather than draining it into the void. A late-bound
        // shell replays the scene and `announce_one_toplevel` is idempotent,
        // and a headless test can observe what the shell would have learned.
        let sessions = self.manager_sessions();
        if sessions.is_empty() {
            return;
        }
        let events = self.window_dispatch.drain();
        if events.is_empty() {
            return;
        }
        let mut focus_changed = false;
        for event in events {
            match event.kind {
                WindowEventKind::Mapped | WindowEventKind::Unmapped | WindowEventKind::Closed => {
                    // Announce/close per manager. `Unmapped` is a client
                    // destroy; `Closed` is a settled close ghost (T-02.4a).
                    // Both mean the toplevel resource goes away.
                    for (client, manager) in &sessions {
                        match event.kind {
                            WindowEventKind::Mapped => {
                                self.announce_one_toplevel(client, manager, event.id);
                            }
                            WindowEventKind::Unmapped | WindowEventKind::Closed => {
                                if let Some(session) = self.shell.sessions.get_mut(client) {
                                    if let Some(resource) = session.toplevels.remove(&event.id) {
                                        resource.closed();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                WindowEventKind::TitleChanged => {
                    for session in self.shell.sessions.values() {
                        if let Some(resource) = session.toplevels.get(&event.id) {
                            resource.title(protocol_string_opt(event.title.clone()));
                            resource.done();
                        }
                    }
                }
                WindowEventKind::AppIdChanged => {
                    for session in self.shell.sessions.values() {
                        if let Some(resource) = session.toplevels.get(&event.id) {
                            resource.app_id(protocol_string_opt(event.app_id.clone()));
                            resource.done();
                        }
                    }
                }
                WindowEventKind::StateChanged(state) => {
                    for session in self.shell.sessions.values() {
                        if let Some(resource) = session.toplevels.get(&event.id) {
                            resource.state(self.window_state_flags(event.id, state));
                            resource.done();
                        }
                    }
                }
                WindowEventKind::Focused | WindowEventKind::Unfocused => {
                    focus_changed = true;
                }
            }
        }
        if focus_changed {
            let focused = self
                .active_window
                .as_ref()
                .and_then(|window| self.windows.id(window));
            for (client, manager) in &sessions {
                let resource = focused.and_then(|id| {
                    self.shell
                        .sessions
                        .get(client)
                        .and_then(|session| session.toplevels.get(&id).cloned())
                });
                manager.focused(resource.as_ref());
                manager.done();
            }
        }
    }

    fn announce_one_toplevel(
        &mut self,
        client: &ClientId,
        manager: &df_toplevel_manager::DfToplevelManager,
        id: WindowId,
    ) {
        if self
            .shell
            .sessions
            .get(client)
            .is_some_and(|session| session.toplevels.contains_key(&id))
        {
            return;
        }
        let Some(creator) = manager.client() else {
            return;
        };
        let Ok(resource) = creator.create_resource::<df_toplevel::DfToplevel, _, DfState>(
            &self.display_handle,
            INTERFACE_VERSION,
            ToplevelUserData { id },
        ) else {
            return;
        };
        manager.toplevel(&resource);
        let (app_id, title, state) = self
            .window_by_id(id)
            .map(|window| {
                (
                    self.windows.app_id(&window).map(str::to_string),
                    self.windows.title(&window).map(str::to_string),
                    self.windows.state(&window),
                )
            })
            .unwrap_or((None, None, None));
        resource.title(protocol_string_opt(title.clone()));
        resource.app_id(protocol_string_opt(app_id.clone()));
        if let Some(state) = state {
            resource.state(self.window_state_flags(id, state));
        }
        if let Some(space) = self.workspaces.window_space(id) {
            if let Some(workspace) = self.workspace_resource(client.clone(), space) {
                resource.workspace_entered(&workspace);
            }
        }
        if let Some(output) = self.window_output_name(id) {
            if let Some(output_resource) = self.output_resource(client.clone(), &output) {
                resource.output_entered(&output_resource);
            }
        }
        resource.done();
        if let Some(session) = self.shell.sessions.get_mut(client) {
            session.toplevels.insert(id, resource);
        }
    }

    fn broadcast_input_events(&mut self) {
        let events = self.input_dispatch.drain();
        if events.is_empty() {
            return;
        }
        let sessions = self.manager_sessions();
        for event in events {
            match event {
                ShellInputEvent::Action(action) => {
                    for (_, manager) in &sessions {
                        manager.input_action(
                            action.action.name().to_string(),
                            action.source.name().to_string(),
                            action.serial,
                        );
                        if let TriggerKind::HotCorner(corner) = action.source {
                            let output = self.primary_output().0.unwrap_or_default();
                            manager.hot_corner(hot_corner_wire(corner), output);
                        }
                    }
                }
                ShellInputEvent::Progress(progress) => {
                    for (_, manager) in &sessions {
                        manager.progress(
                            progress.action.name().to_string(),
                            progress.progress,
                            progress.raw_progress,
                            progress.velocity,
                            progress_phase_wire(progress.phase),
                            progress.committed as u32,
                            progress.cancelled as u32,
                        );
                    }
                }
                ShellInputEvent::AppAccelerator(accelerator) => {
                    for (_, manager) in &sessions {
                        manager.app_accelerator(
                            protocol_string(&accelerator.app_id),
                            accelerator.accelerator_id.clone(),
                            accelerator.source.name().to_string(),
                            accelerator.serial,
                        );
                    }
                }
            }
        }
        for (_, manager) in &sessions {
            manager.done();
        }
    }

    fn broadcast_attention(&mut self) {
        let attention: Vec<WindowId> = std::mem::take(&mut self.shell.attention);
        if attention.is_empty() {
            return;
        }
        let sessions = self.manager_sessions();
        for id in attention {
            for (client, manager) in &sessions {
                if let Some(resource) = self
                    .shell
                    .sessions
                    .get(client)
                    .and_then(|session| session.toplevels.get(&id).cloned())
                {
                    manager.attention(&resource);
                }
            }
        }
        for (_, manager) in &sessions {
            manager.done();
        }
    }

    /// Broadcast Mission Control state.
    pub fn broadcast_overview(&mut self) {
        let sessions = self.manager_sessions();
        let active = self.overview.overview_active();
        let selected = self.overview.selection();
        for (client, manager) in &sessions {
            let resource = selected.and_then(|id| {
                self.shell
                    .sessions
                    .get(client)
                    .and_then(|session| session.toplevels.get(&id).cloned())
            });
            manager.overview_changed(active as u32, resource.as_ref());
            manager.done();
        }
    }

    /// Broadcast app-switcher state from the one compositor-owned machine
    /// (T-06.1). The shell renders this projection; it never owns the state.
    pub fn broadcast_app_switcher(&mut self) {
        let sessions = self.manager_sessions();
        let active = self.app_switcher.is_active();
        let app_id = self.app_switcher.selected_app().map(str::to_string);
        let direction = self.app_switcher.direction();
        let entries: Vec<(u32, String)> = self
            .app_switcher
            .entries()
            .iter()
            .enumerate()
            .map(|(index, entry)| (index as u32, entry.app_id.clone()))
            .collect();
        for (_, manager) in &sessions {
            manager.app_switcher(
                active as u32,
                protocol_string_opt(app_id.clone()),
                direction,
            );
            // The recency snapshot the overlay draws, one card per app, in
            // order, before the batch `done` (T-06.2a, additive in v4).
            for (index, entry_app_id) in &entries {
                manager.app_switcher_entry(*index, entry_app_id.clone());
            }
            manager.done();
        }
    }
}

// --- dispatch: df_core -----------------------------------------------------

impl GlobalDispatch<df_core::DfCore, ()> for DfState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<df_core::DfCore>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, CoreUserData);
    }
}

impl Dispatch<df_core::DfCore, CoreUserData> for DfState {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &df_core::DfCore,
        request: df_core::Request,
        _data: &CoreUserData,
        dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            df_core::Request::Authenticate {
                lockstep_version,
                token,
            } => {
                if state.shell.is_trusted(&client.id()) {
                    resource.refused(
                        Refusal::AlreadyAuthenticated.code(),
                        Refusal::AlreadyAuthenticated.message().to_string(),
                    );
                    return;
                }
                // The wire token is lowercase hex; decode to the raw value.
                let decoded = trust::LaunchToken::parse_hex(&token);
                let outcome = decoded
                    .as_ref()
                    .map(|value| state.shell.trust.authenticate(value, lockstep_version))
                    .unwrap_or(Err(Refusal::InvalidToken));
                match outcome {
                    Ok(role) => {
                        state.shell.sessions.insert(
                            client.id(),
                            ClientSession {
                                role: Some(role),
                                ..ClientSession::default()
                            },
                        );
                        // A trusted session client is sanctioned to install
                        // grabs (T-03 FR-5): the shell is the compositor's
                        // own session UI, not an arbitrary client.
                        state.grab_arbiter.sanction(client.id());
                        resource.authenticated(lockstep_version);
                        println!(
                            "dragonfruit-compositor: shell protocol: client authenticated as {}",
                            role.name()
                        );
                    }
                    Err(refusal) => {
                        eprintln!(
                            "dragonfruit-compositor: shell protocol: refused client ({}): {}",
                            refusal.name(),
                            refusal.message()
                        );
                        resource.refused(refusal.code(), refusal.message().to_string());
                        client.kill(
                            dhandle,
                            wayland_server::backend::protocol::ProtocolError {
                                code: refusal.code(),
                                object_id: 0,
                                object_interface: "df_core".into(),
                                message: refusal.message().into(),
                            },
                        );
                    }
                }
            }
            df_core::Request::Destroy => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        client: wayland_server::backend::ClientId,
        _resource: &df_core::DfCore,
        _data: &CoreUserData,
    ) {
        state.shell.forget_client(&client);
    }
}

// --- dispatch: df_shell / df_layer_surface ---------------------------------

impl GlobalDispatch<df_shell::DfShell, ()> for DfState {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &Client,
        resource: New<df_shell::DfShell>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        if !state.shell.is_trusted(&client.id()) {
            eprintln!(
                "dragonfruit-compositor: shell protocol: refusing df_shell bind from untrusted client"
            );
            data_init.post_error(
                resource,
                ERROR_ACCESS_DENIED,
                "df_shell requires a successful df_core.authenticate",
            );
            return;
        }
        let shell = data_init.init(resource, ());
        if let Some(session) = state.shell.sessions.get_mut(&client.id()) {
            session.factory = Some(shell);
        }
    }
}

impl Dispatch<df_shell::DfShell, ()> for DfState {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &df_shell::DfShell,
        request: df_shell::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            df_shell::Request::GetLayerSurface {
                id,
                surface,
                output,
                layer,
                namespace,
            } => {
                let output_name = output
                    .as_ref()
                    .and_then(Output::from_resource)
                    .map(|output| output.name());
                let resource = data_init.init(id, LayerUserData);
                let layer_value = layer.into_result().map(|value| value as u32).unwrap_or(2);
                let entry = LayerEntry {
                    resource: resource.clone(),
                    surface,
                    state: LayerSurfaceState {
                        layer: layer_value,
                        namespace,
                        output: output_name,
                        ..LayerSurfaceState::default()
                    },
                };
                state.shell.layers.push(entry);
                state.configure_layer(&resource);
                state.refresh_reserved_zones();
                state.needs_redraw = true;
            }
            df_shell::Request::Destroy => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        client: wayland_server::backend::ClientId,
        _resource: &df_shell::DfShell,
        _data: &(),
    ) {
        if let Some(session) = state.shell.sessions.get_mut(&client) {
            session.factory = None;
        }
    }
}

impl Dispatch<df_layer_surface::DfLayerSurface, LayerUserData> for DfState {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &df_layer_surface::DfLayerSurface,
        request: df_layer_surface::Request,
        _data: &LayerUserData,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        let mut reconfigure = false;
        let mut reserve_changed = false;
        let mut focus_chrome: Option<WlSurface> = None;
        if let Some(entry) = state.layer_entry_mut(resource) {
            match request {
                df_layer_surface::Request::SetLayer { layer } => {
                    entry.state.layer = layer.into_result().map(|value| value as u32).unwrap_or(2);
                }
                df_layer_surface::Request::SetAnchor { anchor } => {
                    entry.state.anchor = anchor;
                    reconfigure = true;
                }
                df_layer_surface::Request::SetSize { width, height } => {
                    entry.state.width = width;
                    entry.state.height = height;
                    reconfigure = true;
                }
                df_layer_surface::Request::SetMargin {
                    top,
                    right,
                    bottom,
                    left,
                } => {
                    entry.state.margin = layer::Margins {
                        top,
                        right,
                        bottom,
                        left,
                    };
                    reconfigure = true;
                }
                df_layer_surface::Request::SetExclusiveZone { zone } => {
                    entry.state.exclusive_zone = zone;
                    reserve_changed = true;
                    reconfigure = true;
                }
                df_layer_surface::Request::SetKeyboardInteraction { mode } => {
                    entry.state.keyboard = KeyboardInteraction::from_wire(
                        mode.into_result().map(|value| value as u32).unwrap_or(0),
                    );
                    if entry.state.keyboard == KeyboardInteraction::Exclusive {
                        focus_chrome = Some(entry.surface.clone());
                    }
                }
                df_layer_surface::Request::AckConfigure { .. } => {}
                df_layer_surface::Request::Destroy => {}
            }
        }
        if let Some(surface) = focus_chrome {
            state.focus_chrome_surface(&surface);
        }
        if reserve_changed {
            state.refresh_reserved_zones();
        }
        if reconfigure {
            state.configure_layer(resource);
            state.needs_redraw = true;
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        resource: &df_layer_surface::DfLayerSurface,
        _data: &LayerUserData,
    ) {
        // If the closing chrome surface held the keyboard, hand focus back to
        // the active window (a shell crash must not strand focus on a dead
        // surface). Compare before the entry is removed.
        let focused = state
            .seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus());
        let destroyed_focused = state
            .shell
            .layers
            .iter()
            .find(|entry| entry.resource == *resource)
            .is_some_and(|entry| focused.as_ref() == Some(&entry.surface));
        state
            .shell
            .layers
            .retain(|entry| entry.resource != *resource);
        if destroyed_focused {
            state.restore_window_keyboard_focus();
        }
        state.refresh_reserved_zones();
        state.needs_redraw = true;
    }
}

// --- dispatch: df_toplevel_manager -----------------------------------------

impl GlobalDispatch<df_toplevel_manager::DfToplevelManager, ()> for DfState {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &Client,
        resource: New<df_toplevel_manager::DfToplevelManager>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        if !state.shell.is_trusted(&client.id()) {
            eprintln!(
                "dragonfruit-compositor: shell protocol: refusing df_toplevel_manager bind from untrusted client"
            );
            data_init.post_error(
                resource,
                ERROR_ACCESS_DENIED,
                "df_toplevel_manager requires a successful df_core.authenticate",
            );
            return;
        }
        let manager = data_init.init(resource, ());
        if let Some(session) = state.shell.sessions.get_mut(&client.id()) {
            session.manager = Some(manager.clone());
        }
        // Replay the current scene, then done.
        state.announce_outputs(client, &manager);
        state.announce_workspaces(client, &manager);
        state.announce_toplevels(client, &manager);
        manager.done();
    }
}

impl Dispatch<df_toplevel_manager::DfToplevelManager, ()> for DfState {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &df_toplevel_manager::DfToplevelManager,
        request: df_toplevel_manager::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            df_toplevel_manager::Request::CreateWorkspace => {
                state.workspaces.create_space();
                state.sync_workspaces();
                state.needs_redraw = true;
            }
            df_toplevel_manager::Request::RemoveWorkspace { workspace } => {
                if let Some(space) = workspace.data::<WorkspaceUserData>().map(|data| data.id) {
                    if let Some(index) = state.workspace_index(space) {
                        state.workspaces.remove_space(index);
                        state.sync_workspaces();
                        state.after_workspace_change();
                    }
                }
            }
            df_toplevel_manager::Request::ReorderWorkspace { workspace, index } => {
                if let Some(space) = workspace.data::<WorkspaceUserData>().map(|data| data.id) {
                    if let Some(from) = state.workspace_index(space) {
                        state.workspaces.reorder_space(from, index as usize);
                        state.sync_workspaces();
                    }
                }
            }
            df_toplevel_manager::Request::ActivateWorkspace { workspace } => {
                if let Some(space) = workspace.data::<WorkspaceUserData>().map(|data| data.id) {
                    if let Some(index) = state.workspace_index(space) {
                        if state.workspaces.activate_all(index) {
                            state.after_workspace_change();
                        }
                    }
                }
            }
            df_toplevel_manager::Request::EnterMissionControl => {
                // U-7: the shell request drives the one overview machine and
                // its progress pipeline exactly like a gesture/hot corner.
                state.drive_overview_request(true, SERIAL_COUNTER.next_serial().into());
            }
            df_toplevel_manager::Request::ExitMissionControl => {
                state.drive_overview_request(false, SERIAL_COUNTER.next_serial().into());
            }
            df_toplevel_manager::Request::SelectOverviewToplevel { toplevel } => {
                if let Some(id) = toplevel.data::<ToplevelUserData>().map(|data| data.id) {
                    // Selection round-trip (FR-5): remember the choice, leave
                    // the overview, then activate its Space and raise/focus.
                    // The same path a pointer click on a live representation
                    // takes (T-05.2).
                    state.select_overview_window(id);
                }
            }
            df_toplevel_manager::Request::ActivateApp { app_id } => {
                // The Dock's "most recent window of an app" path; the
                // app-switcher commit shares the same `activate_app`.
                state.activate_app(&app_id);
            }
            df_toplevel_manager::Request::CycleAppSwitcher { direction } => {
                state.cycle_app_switcher(direction);
            }
            df_toplevel_manager::Request::ReleaseKeyboardFocus => {
                // T-10 section 20: the shell releases chrome keyboard focus
                // (Escape exits Dock keyboard navigation) so the active window
                // gets the keyboard back. A no-op when no chrome surface held
                // it or the active window is gone.
                if state.chrome_has_keyboard_focus() {
                    state.restore_window_keyboard_focus();
                    state.needs_redraw = true;
                }
            }
            df_toplevel_manager::Request::SetReducedMotion { enabled } => {
                // T-11 U-1: mirror the shell's reduced-motion policy into the
                // overview machine; every transition then takes the single
                // step while keeping the same commit rule (FR-9).
                state.set_reduced_motion(enabled != 0);
            }
            df_toplevel_manager::Request::SetLaunchOrigin {
                app_id,
                x,
                y,
                width,
                height,
            } => {
                // T-02.1b/T-02.2: the Dock's tile geometry for an app's
                // windows. The launching window appears from it and minimize/
                // restore scale into and out of it; absent, the compositor
                // uses a centered origin.
                state.set_launch_origin(
                    &app_id,
                    Rectangle::new((x, y).into(), (width.max(1), height.max(1)).into()),
                );
            }
            df_toplevel_manager::Request::SetMotionPolicy {
                color_scheme,
                titlebar_double_click,
                minimized_animation,
            } => {
                // T-08.2c: the shell forwards the settingsd motion/appearance
                // keys; the compositor is the only applier.
                state.set_motion_policy(
                    color_scheme.as_deref(),
                    titlebar_double_click.as_deref(),
                    minimized_animation.as_deref(),
                );
            }
            df_toplevel_manager::Request::SetInputPolicy {
                repeat_delay_ms,
                repeat_rate_hz,
                gestures_enabled,
                gesture_space_switch,
                gesture_mission_control,
            } => {
                // T-08.2c: keyboard repeat and gesture gating, applied live.
                state.set_input_policy(
                    repeat_delay_ms,
                    repeat_rate_hz,
                    gestures_enabled != 0,
                    gesture_space_switch != 0,
                    gesture_mission_control != 0,
                );
            }
            df_toplevel_manager::Request::Destroy => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        client: wayland_server::backend::ClientId,
        _resource: &df_toplevel_manager::DfToplevelManager,
        _data: &(),
    ) {
        if let Some(session) = state.shell.sessions.get_mut(&client) {
            session.manager = None;
        }
    }
}

impl DfState {
    fn workspace_index(&self, space: SpaceId) -> Option<usize> {
        let output = self.workspaces.space_output(space)?;
        self.workspaces
            .space_ids(output)
            .iter()
            .position(|candidate| *candidate == space)
    }

    /// The shell's `cycle_app_switcher` request drives the *same*
    /// compositor-owned machine as the Cmd+Tab chord, recorded with the shell
    /// trigger. Cycling never focuses; only a commit does (T-06.1).
    fn cycle_app_switcher(&mut self, direction: i32) {
        let step = if direction < 0 {
            SwitchStep::Backward
        } else {
            SwitchStep::Forward
        };
        let serial = SERIAL_COUNTER.next_serial().into();
        self.app_switcher_key(step, TriggerKind::Shell, serial);
    }
}

// --- dispatch: df_toplevel -------------------------------------------------

impl Dispatch<df_toplevel::DfToplevel, ToplevelUserData> for DfState {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &df_toplevel::DfToplevel,
        request: df_toplevel::Request,
        data: &ToplevelUserData,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        let Some(window) = state.window_by_id(data.id) else {
            return;
        };
        match request {
            df_toplevel::Request::Activate => state.activate_window_id(data.id),
            df_toplevel::Request::Close => state.close_window(&window),
            df_toplevel::Request::Minimize => state.minimize_window(&window),
            df_toplevel::Request::Unminimize => state.restore_window(&window),
            df_toplevel::Request::Zoom => state.zoom_window(&window),
            df_toplevel::Request::Unzoom => state.unzoom_window(&window),
            df_toplevel::Request::Fullscreen => state.fullscreen_window(&window),
            df_toplevel::Request::Unfullscreen => state.unfullscreen_window(&window),
            df_toplevel::Request::MoveToWorkspace { workspace } => {
                if let Some(space) = workspace.data::<WorkspaceUserData>().map(|data| data.id) {
                    if let Some(index) = state.workspace_index(space) {
                        state.move_window_to_space(&window, index);
                    }
                }
            }
            df_toplevel::Request::Destroy => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        client: wayland_server::backend::ClientId,
        resource: &df_toplevel::DfToplevel,
        data: &ToplevelUserData,
    ) {
        if let Some(session) = state.shell.sessions.get_mut(&client) {
            session.toplevels.remove(&data.id);
        }
        let _ = resource;
    }
}

// --- dispatch: df_workspace ------------------------------------------------

impl Dispatch<df_workspace::DfWorkspace, WorkspaceUserData> for DfState {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &df_workspace::DfWorkspace,
        request: df_workspace::Request,
        data: &WorkspaceUserData,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        let Some(index) = state.workspace_index(data.id) else {
            return;
        };
        match request {
            df_workspace::Request::Activate => {
                if state.workspaces.activate_all(index) {
                    state.after_workspace_change();
                }
            }
            df_workspace::Request::SetWallpaper { source, fit, color } => {
                if let Some(output) = state.workspaces.space_output(data.id).map(str::to_string) {
                    let wallpaper = Wallpaper {
                        color: argb_to_rgba(color),
                        source,
                        fit: wallpaper_fit_from_wire(
                            fit.into_result().map(|value| value as u32).unwrap_or(0),
                        ),
                    };
                    if state.workspaces.set_wallpaper(&output, index, wallpaper) {
                        state.needs_redraw = true;
                        state.sync_workspaces();
                    }
                }
            }
            df_workspace::Request::Destroy => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        client: wayland_server::backend::ClientId,
        _resource: &df_workspace::DfWorkspace,
        data: &WorkspaceUserData,
    ) {
        if let Some(session) = state.shell.sessions.get_mut(&client) {
            session.workspaces.remove(&data.id);
        }
    }
}

// --- dispatch: df_output ---------------------------------------------------

impl Dispatch<df_output::DfOutput, OutputUserData> for DfState {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &df_output::DfOutput,
        request: df_output::Request,
        data: &OutputUserData,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        let Some(output) = state
            .space
            .outputs()
            .find(|output| output.name() == data.name)
            .cloned()
        else {
            return;
        };
        match request {
            df_output::Request::SetMode {
                width,
                height,
                refresh,
            } => {
                // Client-controlled mode values must never construct an
                // invalid `Size` (a negative dimension panics inside
                // Smithay). Reject nonsensical modes instead of aborting
                // the session; the ack below still reports the live mode.
                if width > 0 && height > 0 && width <= i32::MAX as u32 && height <= i32::MAX as u32
                {
                    let mode = Mode {
                        size: (width as i32, height as i32).into(),
                        refresh: refresh as i32,
                    };
                    output.change_current_state(Some(mode), None, None, None);
                } else {
                    eprintln!(
                        "dragonfruit-compositor: ignoring invalid mode request {width}x{height}"
                    );
                }
            }
            df_output::Request::SetScale { scale } => {
                // A non-finite or non-positive scale would poison every
                // later geometry conversion; keep the current scale.
                if scale.is_finite() && scale > 0.0 {
                    output.change_current_state(
                        None,
                        None,
                        Some(OutputScale::Fractional(scale)),
                        None,
                    );
                } else {
                    eprintln!("dragonfruit-compositor: ignoring invalid scale request {scale}");
                }
            }
            df_output::Request::SetTransform { transform } => {
                let transform = transform_from_wire(
                    transform
                        .into_result()
                        .map(|value| value as u32)
                        .unwrap_or(0),
                );
                output.change_current_state(None, Some(transform), None, None);
            }
            df_output::Request::SetVrr { .. } | df_output::Request::SetNightLight { .. } => {
                // VRR and night-light plumbing is a T-16 displays-pane item
                // (see PROGRESS.md, T-02); accepted and acked here.
            }
            df_output::Request::Destroy => {}
        }
        // Ack with the applied properties.
        state.send_output_properties(resource, &output);
        resource.done();
    }

    fn destroyed(
        state: &mut Self,
        client: wayland_server::backend::ClientId,
        _resource: &df_output::DfOutput,
        data: &OutputUserData,
    ) {
        if let Some(session) = state.shell.sessions.get_mut(&client) {
            session.outputs.remove(&data.name);
        }
    }
}

// --- session provisioning --------------------------------------------------

/// Provision launch tokens at session start and write the shell hand-off
/// file. `DRAGONFRUIT_LAUNCH_TOKENS` (comma-separated hex) lets the session
/// manager (T-24) or the conformance tests supply pre-minted tokens;
/// otherwise one random shell token is minted.
pub fn provision(state: &mut DfState) {
    let mut tokens = Vec::new();
    if let Ok(list) = std::env::var(trust::TOKENS_ENV) {
        for hex in list
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            if let Some(value) = trust::LaunchToken::parse_hex(hex) {
                tokens.push(state.shell.trust.mint_with_value(TrustedRole::Shell, value));
            }
        }
    }
    // The single-token hand-off (T-24): the session manager may pass the
    // shell's token directly instead of a list.
    if tokens.is_empty() {
        if let Ok(hex) = std::env::var(trust::TOKEN_ENV) {
            if let Some(value) = trust::LaunchToken::parse_hex(&hex) {
                tokens.push(state.shell.trust.mint_with_value(TrustedRole::Shell, value));
            }
        }
    }
    if tokens.is_empty() {
        tokens.push(state.shell.trust.mint(TrustedRole::Shell));
    }
    let Some(token) = tokens.first() else {
        return;
    };
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    let path = std::path::PathBuf::from(runtime).join(format!(
        "{}{}",
        state.socket_name,
        trust::TOKEN_FILE_SUFFIX
    ));
    match write_private(&path, token.to_hex().as_bytes()) {
        Ok(()) => println!("dragonfruit-compositor: launch token: {}", path.display()),
        Err(err) => eprintln!("dragonfruit-compositor: failed to write launch token: {err}"),
    }
}

/// Remove the shell token hand-off file at teardown.
pub fn remove_token_file(socket_name: &str) {
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    let path = std::path::PathBuf::from(runtime)
        .join(format!("{socket_name}{}", trust::TOKEN_FILE_SUFFIX));
    let _ = std::fs::remove_file(path);
}

fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::{protocol_string, protocol_string_opt};

    #[test]
    fn protocol_strings_never_contain_nul() {
        // X11 `WM_NAME`/`WM_CLASS` are conventionally NUL-terminated;
        // the generated server bindings panic on an interior NUL.
        assert_eq!(protocol_string("E2E X11\0"), "E2E X11");
        assert_eq!(protocol_string("a\0b\0"), "ab");
        assert_eq!(
            protocol_string_opt(Some("t\0".to_string())),
            Some("t".to_string())
        );
        assert_eq!(protocol_string_opt(None), None);
    }
}
