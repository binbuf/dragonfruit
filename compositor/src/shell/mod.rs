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
pub mod menubar;
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
use crate::window::{
    EffectiveMaterial, MaterialRole, ReservedZones, WindowEventKind, WindowId, WindowState,
};
use crate::workspace::{SpaceId, Wallpaper, WallpaperFit};

use layer::{aggregate_reserved_for, Edge, KeyboardInteraction, LayerSurfaceState};
use menubar::{menu_bar_visible, AutoHideMode, MenuBarVisibility, MENU_BAR_REVEAL_HEIGHT};
use trust::{Refusal, TrustModel, TrustedRole};

/// The version of every private interface this compositor implements.
const INTERFACE_VERSION: u32 = 1;

/// `df_shell` is version 4 since the menu-bar `set_auto_hide` request was
/// added (T201); version 3 added the multi-region `df_layer_surface` requests,
/// and version 2 added `df_layer_surface.set_panel_rect` (T-14.7b). A Wayland
/// new_id gets its version from the parent object's version, so the factory
/// bumps in tandem with its child interface.
const SHELL_INTERFACE_VERSION: u32 = 4;

/// `df_toplevel_manager` is version 10 since the T229 pointer/layout policy
/// requests were added; `set_minimize_target` is the v11 addition (T237);
/// `set_appearance_policy` was the v9 addition (T196);
/// `activation_result` was the v8 addition (T-14.7g),
/// `set_app_accelerators` v7 (T-14.2b), `capture_screenshot` and the
/// `screenshot_saved`/`screenshot_failed` events were the v6 additions
/// (T-13.3b), `set_motion_policy`/`set_input_policy` were the v5 additions
/// (T-08.2c), `set_launch_origin` v4 (T-02.1b), `set_reduced_motion` v3
/// (T-11), and `release_keyboard_focus` v2 (T-10). The other interfaces stay
/// at [`INTERFACE_VERSION`].
const MANAGER_INTERFACE_VERSION: u32 = 11;

/// `df_output` is version 4 since the role-keyed `material_role_state` event
/// was added (T-21.2c); v3 added the global effective `material_state` event
/// (P1), and `set_brightness`/`brightness` were the v2 additions (T-11.3a).
/// The other private interfaces stay at [`INTERFACE_VERSION`].
const DF_OUTPUT_INTERFACE_VERSION: u32 = 4;

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
    pending_panel_rect: Option<Rectangle<i32, Logical>>,
    /// Accumulated `set_panel_region` metadata for the next commit. `None`
    /// means no new region list was sent; `Some(empty)` is an explicit clear
    /// that commits an empty panel set (T197).
    pending_panel_regions: Option<Vec<layer::PanelRegion>>,
}

/// One material region after placement on an output (T197).
///
/// `rect` is output-local logical and already clipped to the surface geometry
/// and the owning output. `key` is stable within the surface; `preset` is the
/// validated optical preset or `None` to derive it from the namespace/role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeRegion {
    pub key: String,
    pub preset: Option<crate::material_preset::Preset>,
    pub rect: Rectangle<i32, Logical>,
    /// Per-corner radii from region metadata, or `None` for the legacy
    /// single-rect path (the renderer then uses the role's token radius).
    pub corners: Option<crate::window::CornerRadii>,
    pub overscan: i32,
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
    /// bar slab, not the transparent magnification band above it. An empty
    /// rect means the surface has no backdrop. With multi-region metadata this
    /// is the union of the regions, kept for legacy consumers; renderers use
    /// [`Self::regions`].
    pub panel: Rectangle<i32, Logical>,
    /// Independently keyed material regions (T197), output-local and clipped
    /// to the owning output. A legacy single-rect surface contributes exactly
    /// one region with the key `panel`.
    pub regions: Vec<ChromeRegion>,
    /// A compositor-owned visual translation applied on top of `location`.
    /// The menu bar uses it for the measured reveal/hide slide (T233); every
    /// other surface leaves it at zero. Input hit-testing still uses
    /// `location`, so the top strip stays the reveal target while the bar
    /// slides into place.
    pub render_offset: Point<i32, Logical>,
    /// The layer the surface requested (`df_shell.layer`).
    pub layer: u32,
    /// The `df_layer_surface` namespace (e.g. `menubar`, `dock`). The
    /// compositor picks the material role from it (T-14.7j).
    pub namespace: String,
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
    /// Reserved zones aggregated from chrome surfaces (menu bar, Dock), the
    /// union across every output. Kept as the fallback for an output whose
    /// per-output entry is not resolved yet.
    pub reserved: ReservedZones,
    /// Per-output reserved zones, keyed by output name (T-16.1a). A chrome
    /// surface pinned to one output reserves only there; an all-output
    /// surface reserves on every output. Recomputed by
    /// [`DfState::refresh_reserved_zones`].
    pub output_reserved: HashMap<String, ReservedZones>,
    /// The output a chrome surface last took keyboard focus on. Transient
    /// `overlay` chrome with no explicit output (popovers, menus, OSD) is
    /// shown only on this output, so it never floats across displays (T-10
    /// section 18). `None` before any chrome focus.
    pub chrome_focus_output: Option<String>,
    /// T201: the menu-bar auto-hide policy and its per-output transient
    /// reveal. The bar surface is one all-output surface; this state filters
    /// it per output without creating a second surface.
    pub menu_bar: MenuBarVisibility,
    /// Windows that asked for activation and have not been seen yet.
    attention: Vec<WindowId>,
    /// Per-output brightness level in [0, 1] (T-11.3a). The compositor owns
    /// the value; a backend applies it best-effort (headless/nested keep the
    /// value only, DRM is a later task). Keyed by output name; absent = 1.0.
    output_brightness: HashMap<String, f64>,
    /// Last **global** effective material state broadcast on the legacy v3
    /// `material_state` event (P1). Kept so an older shell keeps its exact
    /// meaning; the per-role table below is the T-21.2c state.
    pub last_material_state: Option<EffectiveMaterial>,
    /// Last effective material broadcast per role (T-21.2c), keyed by the
    /// `df_output` protocol id and indexed by [`MaterialRole::wire`]. The
    /// policy is global today, but the event is per-output, so each client's
    /// output resource gets its own initial table without a duplicate
    /// broadcast to the others.
    pub last_material_role_states: HashMap<u32, [Option<EffectiveMaterial>; MaterialRole::COUNT]>,
}

impl ShellProtocolState {
    /// Register the private globals. `df_core` is advertised to everyone;
    /// the others are advertised but refuse untrusted binds.
    pub fn new(display: &DisplayHandle) -> Self {
        let core_global =
            display.create_global::<DfState, df_core::DfCore, ()>(INTERFACE_VERSION, ());
        let shell_global =
            display.create_global::<DfState, df_shell::DfShell, ()>(SHELL_INTERFACE_VERSION, ());
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
            reserved: ReservedZones::default(),
            output_reserved: HashMap::new(),
            chrome_focus_output: None,
            menu_bar: MenuBarVisibility::default(),
            attention: Vec::new(),
            output_brightness: HashMap::new(),
            last_material_state: None,
            last_material_role_states: HashMap::new(),
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

    /// The reserved zones that apply to `output_name` (T-16.1a).
    ///
    /// Falls back to the global union for an output that has not been
    /// resolved per-output yet (before the first chrome surface, or during
    /// startup), so callers never see a spurious zero reserve.
    pub fn reserved_zones_for(&self, output_name: &str) -> ReservedZones {
        self.output_reserved
            .get(output_name)
            .copied()
            .unwrap_or(self.reserved)
    }

    /// Drop a client's protocol objects when it disconnects, returning the
    /// role it had authenticated as (if any). Callers use the role to reissue
    /// that trusted process's one-time launch token.
    fn forget_client(&mut self, client: &ClientId) -> Option<TrustedRole> {
        let role = self
            .sessions
            .remove(client)
            .and_then(|session| session.role);
        // Chrome surfaces owned by the client no longer reserve zones.
        self.layers
            .retain(|entry| entry.resource.client().map(|c| c.id()) != Some(client.clone()));
        role
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

/// Whether a chrome surface currently has a buffer (is mapped).
fn surface_has_buffer(surface: &WlSurface) -> bool {
    smithay::wayland::compositor::with_states(surface, |states| {
        states
            .cached_state
            .get::<smithay::wayland::compositor::SurfaceAttributes>()
            .current()
            .buffer
            .is_some()
    })
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
        // While locked, activation must not move focus to a client (T-12.3a).
        if self.lock.is_locked() {
            return;
        }
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
    ///
    /// Zones are resolved **per output** (T-16.1a): each output keeps only
    /// the reserves from the chrome surfaces that target it
    /// ([`LayerSurfaceState::matches_output`]). The union across outputs is
    /// kept as `reserved`/`reserved_zones`, the fallback for an output that
    /// has not been resolved yet.
    pub fn refresh_reserved_zones(&mut self) {
        let names: Vec<String> = self.space.outputs().map(|output| output.name()).collect();
        let focus_output = self.shell.chrome_focus_output.clone();
        let mut per_output: HashMap<String, ReservedZones> = HashMap::new();
        let mut union = ReservedZones::default();
        for name in &names {
            let fullscreen = self.fullscreen_client_covers_output(name);
            let menu_bar_visible = self.menu_bar_visible_on(name, fullscreen);
            let layers = self
                .shell
                .layers
                .iter()
                .filter(|entry| entry.state.visible_on_output(name, focus_output.as_deref()))
                // T201: a concealed menu bar reserves nothing. The Dock keeps
                // its existing fullscreen behavior (its own auto-hide owns
                // the reserved zone).
                .filter(|entry| entry.state.namespace != "menubar" || menu_bar_visible)
                .map(|entry| &entry.state);
            let zones = aggregate_reserved_for(name, layers, ReservedZones::default());
            union.top = union.top.max(zones.top);
            union.bottom = union.bottom.max(zones.bottom);
            union.left = union.left.max(zones.left);
            union.right = union.right.max(zones.right);
            per_output.insert(name.clone(), zones);
        }
        if per_output != self.shell.output_reserved {
            self.shell.output_reserved = per_output;
            self.shell.reserved = union;
            self.reserved_zones = union;
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

    pub(crate) fn commit_chrome_panel(&mut self, surface: &WlSurface) {
        if let Some(entry) = self
            .shell
            .layers
            .iter_mut()
            .find(|entry| entry.surface == *surface)
        {
            // The new multi-region list is commit-latched with the buffer that
            // describes it. `Some(empty)` is an explicit empty panel set; it
            // clears any older single rect so removal cannot fall back to a
            // stale panel (T197).
            if let Some(regions) = entry.pending_panel_regions.take() {
                entry.state.panel_regions = Some(regions);
                entry.state.panel_rect = None;
            }
            if let Some(rect) = entry.pending_panel_rect.take() {
                entry.state.panel_rect = Some(rect);
                entry.state.panel_regions = None;
            }
        }
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
                // The visible panel: explicit multi-region metadata wins
                // (T197), then an explicitly declared single panel rect (the
                // Dock's live plate under magnification, T-14.7b), then the
                // reserved bar strip narrowed by the part the client declares
                // interactive. The Dock's input region excludes its
                // transparent magnification band, so the fallback keeps the
                // backdrop off the band and off the area beside the bar.
                let output_rect = Rectangle::from_size(output_geometry.size);
                let (panel, regions) = match entry.state.panel_regions.as_ref() {
                    Some(declared) => {
                        let regions: Vec<ChromeRegion> = declared
                            .iter()
                            .filter_map(|region| {
                                let rect =
                                    crate::window::explicit_panel_bounds(geometry, region.rect)?
                                        .intersection(output_rect)?;
                                if rect.size.w <= 0 || rect.size.h <= 0 {
                                    return None;
                                }
                                Some(ChromeRegion {
                                    key: region.key.clone(),
                                    preset: region.preset,
                                    rect,
                                    corners: Some(region.corners),
                                    overscan: region.overscan,
                                })
                            })
                            .collect();
                        let panel = regions
                            .iter()
                            .map(|region| region.rect)
                            .reduce(|a, b| a.merge(b))
                            .unwrap_or_default();
                        (panel, regions)
                    }
                    None => {
                        let panel = entry
                            .state
                            .panel_rect
                            .map(|rect| {
                                crate::window::explicit_panel_bounds(geometry, rect)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_else(|| {
                                let region = smithay::wayland::compositor::with_states(
                                    &entry.surface,
                                    |states| {
                                        states
                                            .cached_state
                                            .get::<smithay::wayland::compositor::SurfaceAttributes>(
                                            )
                                            .current()
                                            .input_region
                                            .clone()
                                    },
                                )
                                .and_then(|region| {
                                    crate::input::constraint::region_bounds(&region)
                                });
                                crate::window::panel_bounds(
                                    geometry,
                                    entry.state.reserved_rect(geometry),
                                    region,
                                )
                            })
                            .intersection(output_rect)
                            .unwrap_or_default();
                        let regions = if panel.size.w > 0 && panel.size.h > 0 {
                            vec![ChromeRegion {
                                key: "panel".to_owned(),
                                preset: None,
                                rect: panel,
                                corners: None,
                                overscan: 0,
                            }]
                        } else {
                            Vec::new()
                        };
                        (panel, regions)
                    }
                };
                // T233: the menu bar is the one surface whose visual position
                // is translated by the measured reveal progress. Keep the
                // panel/region metadata with the visual so the veil cannot be
                // left behind at the top edge.
                let render_offset = if entry.state.namespace == "menubar" {
                    Point::from((
                        0,
                        -self.menu_bar_hide_offset_on(output_name, geometry.size.h),
                    ))
                } else {
                    Point::from((0, 0))
                };
                let (panel, regions) = if render_offset != Point::from((0, 0)) {
                    (
                        Rectangle::new(panel.loc + render_offset, panel.size),
                        regions
                            .into_iter()
                            .map(|mut region| {
                                region.rect.loc += render_offset;
                                region
                            })
                            .collect(),
                    )
                } else {
                    (panel, regions)
                };
                ChromeSurface {
                    surface: entry.surface.clone(),
                    location: geometry.loc,
                    geometry,
                    panel,
                    regions,
                    render_offset,
                    layer: entry.state.layer,
                    namespace: entry.state.namespace.clone(),
                    keyboard: entry.state.keyboard,
                }
            })
            .collect();
        surfaces.sort_by_key(|chrome| chrome.layer);
        surfaces
    }

    /// Chrome surfaces that are actually visible on `output` this frame.
    ///
    /// This is [`Self::chrome_surfaces`] with the fullscreen policy applied:
    /// when a fullscreen client owns the output, persistent `top` chrome (the
    /// menu bar and Dock) is hidden so the client can direct-scan out.
    /// Transient `overlay` surfaces stay, and so does the desktop band.
    /// Frame-callback bookkeeping still uses the unfiltered list, so a hidden
    /// chrome surface is not left waiting on a callback forever.
    pub fn visible_chrome_surfaces(
        &self,
        output_name: &str,
        output_geometry: Rectangle<i32, Logical>,
    ) -> Vec<ChromeSurface> {
        let fullscreen = self.fullscreen_client_covers_output(output_name);
        let menu_bar_visible = self.menu_bar_visible_on(output_name, fullscreen);
        self.chrome_surfaces(output_name, output_geometry)
            .into_iter()
            .filter(|chrome| {
                // T201: the menu bar has its own per-output auto-hide policy;
                // every other surface keeps the T-21.3 fullscreen rule.
                if chrome.namespace == "menubar" {
                    return menu_bar_visible;
                }
                !fullscreen || crate::shell::layer::visible_in_fullscreen(chrome.layer)
            })
            .collect()
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

    // --- menu-bar auto-hide (T201) ----------------------------------------

    /// The live `menu.autoHide` policy.
    pub fn menu_bar_auto_hide(&self) -> AutoHideMode {
        self.shell.menu_bar.mode()
    }

    /// Apply a `menu.autoHide` change. A live change clears the old transient
    /// reveal; the next edge/keyboard interaction applies the new mode.
    pub fn set_menu_bar_auto_hide(&mut self, mode: AutoHideMode) {
        if !self.shell.menu_bar.set_mode(mode) {
            return;
        }
        // A live policy change starts from the resting state: `always` is
        // concealed until the next edge/keyboard reveal even if the pointer
        // happens to sit at the top edge. Settle each output against its
        // current fullscreen/active state so a `never`/desktop bar does not
        // play a reveal animation on a settings toggle.
        let names: Vec<String> = self.space.outputs().map(|output| output.name()).collect();
        for name in names {
            let fullscreen = self.fullscreen_client_covers_output(&name);
            let visible = menu_bar_visible(
                self.shell.menu_bar.mode(),
                fullscreen,
                false,
                self.menu_bar_active_on(&name),
            );
            self.shell.menu_bar.settle(&name, visible);
        }
        self.refresh_reserved_zones();
        self.needs_redraw = true;
    }

    /// Whether the menu bar is revealed on `output` right now.
    pub fn menu_bar_revealed_on(&self, output: &str) -> bool {
        self.shell.menu_bar.is_revealed(output)
    }

    /// The rendered reveal fraction (`0.0..=1.0`) for `output`. An output the
    /// reveal state has not seen yet rests at the policy's visible/invisible
    /// state, so the default `full-screen` bar is fully on screen before the
    /// first pointer motion and a concealed `always` bar starts at zero.
    pub fn menu_bar_progress_on(&self, output: &str) -> f64 {
        if self.shell.menu_bar.has_output(output) {
            return self.shell.menu_bar.progress_on(output);
        }
        let fullscreen = self.fullscreen_client_covers_output(output);
        let active = self.menu_bar_active_on(output);
        if menu_bar_visible(self.shell.menu_bar.mode(), fullscreen, false, active) {
            1.0
        } else {
            0.0
        }
    }

    /// The upward visual translation of the menu-bar surface for `output`.
    pub fn menu_bar_hide_offset_on(&self, output: &str, surface_height: i32) -> i32 {
        let progress = self.menu_bar_progress_on(output);
        if progress >= 1.0 {
            return 0;
        }
        ((1.0 - progress) * f64::from(surface_height.max(0))).round() as i32
    }

    /// The configured menu-bar surface height (the bar strip or its veil
    /// extent), used by diagnostics for the reveal translation.
    pub fn menu_bar_surface_height(&self) -> i32 {
        self.shell
            .layers
            .iter()
            .find(|entry| entry.state.namespace == "menubar")
            .and_then(|entry| {
                entry
                    .state
                    .configured
                    .map(|(_, height)| height)
                    .or(Some(entry.state.height))
            })
            .unwrap_or(MENU_BAR_REVEAL_HEIGHT)
    }

    /// Whether the menu bar contributes an input region on `output` this
    /// frame. A concealed bar is filtered out of the visible chrome list, so
    /// the reveal strip and the top bar strip are the only hit areas.
    pub fn menu_bar_input_on(&self, output: &str, fullscreen: bool) -> bool {
        self.menu_bar_visible_on(output, fullscreen)
            && self
                .shell
                .layers
                .iter()
                .any(|entry| entry.state.namespace == "menubar")
    }

    /// Advance the menu-bar reveal/hide transitions to `now_ms`. Returns
    /// whether the rendered progress changed. A visibility flip refreshes the
    /// per-output reserved zones; the caller requests a frame while the
    /// transition is live.
    pub fn step_menu_bar_visibility(&mut self, now_ms: u64) -> bool {
        if !self.shell.menu_bar.tick(now_ms) {
            return false;
        }
        self.refresh_reserved_zones();
        self.needs_redraw = true;
        true
    }

    /// Whether the menu bar is visible on `output` under the live policy.
    pub fn menu_bar_visible_on(&self, output: &str, fullscreen: bool) -> bool {
        self.shell
            .menu_bar
            .visible_on(output, fullscreen, self.menu_bar_active_on(output))
    }

    /// Whether an active menu/focus interaction keeps the bar up on `output`.
    ///
    /// The bar holds keyboard focus while a dropdown is open (the popup
    /// surface is keyboard-`none`), and a mapped `menubar-popup` overlay is
    /// the other half of the same interaction. Either one keeps a transient
    /// reveal alive after the pointer leaves the top edge.
    pub fn menu_bar_active_on(&self, output: &str) -> bool {
        // `menu_bar_keyboard_focused` is cached by `focus_changed`; querying
        // the keyboard here would deadlock when this runs from that callback.
        if self.menu_bar_keyboard_focused {
            return true;
        }
        self.shell.layers.iter().any(|entry| {
            entry.state.namespace == "menubar-popup"
                && entry
                    .state
                    .visible_on_output(output, self.shell.chrome_focus_output.as_deref())
                && surface_has_buffer(&entry.surface)
        })
    }

    /// Apply the pointer position to the menu-bar reveal on the output it is
    /// over. Called before hit-testing so a concealed bar is revealed before
    /// the same motion/click is routed.
    pub fn update_menu_bar_reveal(&mut self, location: Point<f64, Logical>) -> bool {
        let Some(output) = self.space.outputs().find(|output| {
            self.space
                .output_geometry(output)
                .is_some_and(|geometry| geometry.to_f64().contains(location))
        }) else {
            return false;
        };
        let Some(geometry) = self.space.output_geometry(&output) else {
            return false;
        };
        let name = output.name();
        let fullscreen = self.fullscreen_client_covers_output(&name);
        let active = self.menu_bar_active_on(&name);
        let local_y = location.y - f64::from(geometry.loc.y);
        let now_ms = self.now_msec();
        if !self.shell.menu_bar.update_pointer(
            now_ms,
            &name,
            local_y,
            fullscreen,
            active,
            self.menu_bar_pointer_seen,
        ) {
            return false;
        }
        self.refresh_reserved_zones();
        self.needs_redraw = true;
        // The reveal/hide delay and slide run on the shared animation clock;
        // arm it now that a transition is live.
        crate::input::schedule_animation_timer(self);
        true
    }

    /// Re-evaluate the reveal state from the current pointer and focus. Used
    /// after a focus change, unlock, and a fullscreen transition.
    pub fn sync_menu_bar_reveal(&mut self) -> bool {
        let Some(location) = self
            .seat
            .get_pointer()
            .map(|pointer| pointer.current_location())
        else {
            return false;
        };
        self.update_menu_bar_reveal(location)
    }

    /// Reveal and focus the menu bar (Ctrl+F2). The compositor owns the focus
    /// transfer; the shell owns in-bar navigation. Returns whether the bar
    /// was found and accepted focus.
    pub fn focus_menu_bar(&mut self) -> bool {
        let (output, _) = self.focused_output();
        if let Some(output) = output {
            let now_ms = self.now_msec();
            self.shell.menu_bar.reveal_at(&output, now_ms);
            self.refresh_reserved_zones();
            self.needs_redraw = true;
            crate::input::schedule_animation_timer(self);
        }
        let Some(surface) = self.chrome_surface_by_namespace("menubar") else {
            return false;
        };
        self.focus_chrome_surface(&surface)
    }

    /// Drop the menu-bar transient reveal on lock. Returns whether anything
    /// changed.
    pub fn clear_menu_bar_reveal(&mut self) -> bool {
        if !self.shell.menu_bar.clear() {
            return false;
        }
        self.refresh_reserved_zones();
        self.needs_redraw = true;
        true
    }

    /// Whether an **above-window** chrome surface currently holds the keyboard.
    ///
    /// The Files desktop surface is a layer surface too, but it composites
    /// below windows; its focus must not be treated as chrome focus, or a
    /// desktop click would clear its own keyboard (T-19.3).
    pub fn chrome_has_keyboard_focus(&self) -> bool {
        self.seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus())
            .is_some_and(|surface| self.is_above_window_chrome_surface(&surface))
    }

    /// The `df_shell.layer` value of a chrome surface, if it is one.
    pub fn chrome_layer(&self, surface: &WlSurface) -> Option<u32> {
        self.shell
            .layers
            .iter()
            .find(|entry| entry.surface == *surface)
            .map(|entry| entry.state.layer)
    }

    /// Whether `surface` is a chrome layer surface that composites **above**
    /// windows (`top`/`overlay`, T-19.3). The desktop `background` surface is
    /// excluded: it is client-facing below windows, not shell chrome.
    pub fn is_above_window_chrome_surface(&self, surface: &WlSurface) -> bool {
        self.shell
            .layers
            .iter()
            .find(|entry| entry.surface == *surface)
            .is_some_and(|entry| layer::above_windows(entry.state.layer))
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
        let target_is_none = target.is_none();
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.set_focus(self, target, SERIAL_COUNTER.next_serial());
        }
        // Smithay's `set_focus(None)` sends the leave but does not invoke
        // `focus_changed`, so clear the cached menu-bar focus here and
        // re-apply the reveal rule.
        if target_is_none {
            self.menu_bar_keyboard_focused = false;
            self.sync_menu_bar_reveal();
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

    /// Every announced `df_output` resource, keyed by output name, for
    /// per-output property broadcasts (P1 material state).
    fn output_sessions(&self) -> Vec<(String, df_output::DfOutput)> {
        self.shell
            .sessions
            .values()
            .flat_map(|session| {
                session
                    .outputs
                    .iter()
                    .map(|(name, output)| (name.clone(), output.clone()))
            })
            .collect()
    }

    /// Reply to the trusted shell that a capture was rendered and written
    /// (T-13.3b). The path is the one the shell supplied.
    pub fn broadcast_screenshot_saved(&mut self, path: &str) {
        for (_, manager) in self.manager_sessions() {
            manager.screenshot_saved(path.to_owned());
        }
    }

    /// Reply to the trusted shell that a requested capture could not be
    /// produced (T-13.3b). `reason` is for logs only.
    pub fn broadcast_screenshot_failed(&mut self, reason: &str) {
        for (_, manager) in self.manager_sessions() {
            manager.screenshot_failed(reason.to_owned());
        }
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
                DF_OUTPUT_INTERFACE_VERSION,
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

    fn send_output_properties(&mut self, resource: &df_output::DfOutput, output: &Output) {
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
        // Report the backend's actual VRR state (T188c), not a hardcoded 0.
        // Nested/headless and unsupported connectors stay 0.
        let vrr = self
            .output_vrr
            .get(&output.name())
            .copied()
            .unwrap_or(dragonfruit_compositor::refresh::VrrState::Unknown);
        resource.vrr(u32::from(
            vrr == dragonfruit_compositor::refresh::VrrState::On,
        ));
        resource.night_light(0, 6500);
        // The compositor owns brightness (T-11.3a); absent means full
        // brightness. The event is v2 (`df_output`).
        let output_name = output.name();
        let brightness = self
            .shell
            .output_brightness
            .get(&output_name)
            .copied()
            .unwrap_or(1.0);
        resource.brightness(brightness);
        let zones = self.shell.reserved_zones_for(&output_name);
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
        // Effective material (P1/T-21.2c): the shell uses this to choose the
        // translucent glass complement or the milky fallback. Send the
        // initial state with the other output properties; later changes go
        // through `broadcast_material_state`.
        if resource.version() >= 3 {
            let material = self.effective_material();
            resource.material_state(
                material.tier.index() as u32,
                u32::from(material.blur_active),
                material.fallback.wire(),
            );
        }
        let materials = self.effective_materials();
        if resource.version() >= 4 {
            for (role, material) in materials {
                resource.material_role_state(
                    role.wire(),
                    material.tier.index() as u32,
                    u32::from(material.blur_active),
                    material.fallback.wire(),
                );
            }
        }
        // This output resource now has the current table; a later broadcast
        // only sends roles that actually changed.
        self.shell.last_material_role_states.insert(
            resource.id().protocol_id(),
            materials.map(|(_, material)| Some(material)),
        );
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
        // T230: replay the current focus with the initial toplevel snapshot.
        // The shell binds after the first window has already been activated, so
        // without this the menu bar stays on the desktop (Files) until the next
        // focus change. `manager.focused` is the same event a live focus change
        // sends.
        let focused = self
            .active_window
            .as_ref()
            .and_then(|window| self.windows.id(window))
            .and_then(|id| {
                self.shell
                    .sessions
                    .get(&client.id())
                    .and_then(|session| session.toplevels.get(&id).cloned())
            });
        manager.focused(focused.as_ref());
        manager.done();
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
        self.broadcast_material_state();
    }

    /// Push the effective material state to every announced output when it
    /// changes (P1/T-21.2c). The legacy v3 event keeps its global meaning and
    /// change-only rule; the v4 role event is emitted per output only for the
    /// roles whose effective state changed. Neither is a per-frame event.
    fn broadcast_material_state(&mut self) {
        let materials = self.effective_materials();
        let global = self.effective_material();
        let global_changed = self.shell.last_material_state != Some(global);
        if global_changed {
            self.shell.last_material_state = Some(global);
        }

        for (_, output) in self.output_sessions() {
            if output.version() >= 3 && global_changed {
                output.material_state(
                    global.tier.index() as u32,
                    u32::from(global.blur_active),
                    global.fallback.wire(),
                );
            }
            if output.version() < 4 {
                continue;
            }
            let cache = self
                .shell
                .last_material_role_states
                .entry(output.id().protocol_id())
                .or_insert([None; MaterialRole::COUNT]);
            let changed: Vec<_> = materials
                .into_iter()
                .filter(|(role, material)| cache[role.wire() as usize] != Some(*material))
                .collect();
            if changed.is_empty() {
                continue;
            }
            for (role, material) in changed {
                output.material_role_state(
                    role.wire(),
                    material.tier.index() as u32,
                    u32::from(material.blur_active),
                    material.fallback.wire(),
                );
                cache[role.wire() as usize] = Some(material);
            }
        }
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
        // A lock surface for an output that just disappeared is stale
        // (T-12.3a); dropping it keeps `covers` honest across hotplug.
        self.lock.retain_live(&live);
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
                    DF_OUTPUT_INTERFACE_VERSION,
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

    /// Push the current reserved zones to every output handle (T-16.1a).
    ///
    /// Each output is sent its own zones, so a chrome surface pinned to one
    /// display does not reserve space on another.
    fn broadcast_reserved_zones(&mut self) {
        let sessions = self.manager_sessions();
        for (client, _manager) in &sessions {
            let resources: Vec<(String, df_output::DfOutput)> = self
                .shell
                .sessions
                .get(client)
                .map(|session| {
                    session
                        .outputs
                        .iter()
                        .map(|(name, resource)| (name.clone(), resource.clone()))
                        .collect()
                })
                .unwrap_or_default();
            for (name, resource) in resources {
                let zones = self.shell.reserved_zones_for(&name);
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
        if let Some(role) = state.shell.forget_client(&client) {
            reissue_token(state, role);
        }
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
        client: &Client,
        _resource: &df_shell::DfShell,
        request: df_shell::Request,
        _data: &(),
        dhandle: &DisplayHandle,
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
                let layer_value = layer.into_result().map(|value| value as u32).unwrap_or(2);
                // T-19.3: layers are role-scoped. The Files desktop process
                // owns `background`; the shell owns the above-window chrome
                // layers. A trusted client that crosses into the other's band
                // is refused (and killed), exactly like an untrusted bind.
                let permitted = state
                    .shell
                    .role(&client.id())
                    .is_some_and(|role| role.may_create_layer(layer_value));
                if !permitted {
                    eprintln!(
                        "dragonfruit-compositor: shell protocol: refusing layer {layer_value} \
                         for client without the matching role"
                    );
                    client.kill(
                        dhandle,
                        wayland_server::backend::protocol::ProtocolError {
                            code: ERROR_ACCESS_DENIED,
                            object_id: 0,
                            object_interface: "df_shell".into(),
                            message: format!(
                                "this role may not create a layer-{layer_value} surface"
                            ),
                        },
                    );
                    return;
                }
                let output_name = output
                    .as_ref()
                    .and_then(Output::from_resource)
                    .map(|output| output.name());
                let resource = data_init.init(id, LayerUserData);
                let entry = LayerEntry {
                    resource: resource.clone(),
                    surface,
                    pending_panel_rect: None,
                    pending_panel_regions: None,
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
        let mut menu_bar_mode: Option<AutoHideMode> = None;
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
                df_layer_surface::Request::SetPanelRect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    entry.pending_panel_rect = Some(layer::panel_rect(x, y, width, height));
                }
                df_layer_surface::Request::SetPanelRegion {
                    key,
                    preset,
                    x,
                    y,
                    width,
                    height,
                    top_left,
                    top_right,
                    bottom_right,
                    bottom_left,
                    overscan,
                } => {
                    if let Some(region) = layer::panel_region(
                        &key,
                        &preset,
                        x,
                        y,
                        width,
                        height,
                        top_left,
                        top_right,
                        bottom_right,
                        bottom_left,
                        overscan,
                    ) {
                        // A single-region update is incremental: start from
                        // the committed list so adding a card does not drop
                        // its siblings. `clear_panel_regions` is the explicit
                        // way to remove everything.
                        if entry.pending_panel_regions.is_none() {
                            entry.pending_panel_regions =
                                Some(entry.state.panel_regions.clone().unwrap_or_default());
                        }
                        let pending = entry
                            .pending_panel_regions
                            .as_mut()
                            .expect("pending region list initialized above");
                        if let Some(existing) = pending
                            .iter_mut()
                            .find(|existing| existing.key == region.key)
                        {
                            *existing = region;
                        } else if pending.len() < layer::MAX_PANEL_REGIONS {
                            pending.push(region);
                        } else {
                            eprintln!(
                                "dragonfruit-compositor: shell protocol: ignoring panel region \
                                 `{}` on `{}` (bound is {})",
                                region.key,
                                entry.state.namespace,
                                layer::MAX_PANEL_REGIONS
                            );
                        }
                    } else {
                        eprintln!(
                            "dragonfruit-compositor: shell protocol: rejecting invalid panel \
                             region metadata on `{}`",
                            entry.state.namespace
                        );
                    }
                }
                df_layer_surface::Request::ClearPanelRegions => {
                    entry.pending_panel_regions = Some(Vec::new());
                }
                df_layer_surface::Request::SetAutoHide { mode } => {
                    // T201: only the menu bar consumes the auto-hide policy.
                    // A different namespace is ignored rather than silently
                    // taking the Dock's own auto-hide behavior.
                    if entry.state.namespace == "menubar" {
                        menu_bar_mode = Some(AutoHideMode::from_wire(
                            mode.into_result().map(|value| value as u32).unwrap_or(2),
                        ));
                    }
                }
                df_layer_surface::Request::AckConfigure { .. } => {}
                df_layer_surface::Request::Destroy => {}
            }
        }
        if let Some(mode) = menu_bar_mode {
            state.set_menu_bar_auto_hide(mode);
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
        resource: &df_toplevel_manager::DfToplevelManager,
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
                // app-switcher commit shares the same `activate_app`. The
                // result is sent back (T-14.7g) so the shell can fall back to
                // a launch or a notice instead of leaving a dead click.
                let found = state.activate_app(&app_id);
                resource.activation_result(app_id, found as u32);
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
            df_toplevel_manager::Request::SetMinimizeTarget {
                window_id,
                x,
                y,
                width,
                height,
            } => {
                // T237: the Dock's minimized-region or per-window target when
                // `dock.minimizeIntoTileIcon` is off. A non-positive rect
                // clears it and restores the app-tile mapping.
                let origin = (width > 0 && height > 0)
                    .then(|| Rectangle::new((x, y).into(), (width.max(1), height.max(1)).into()));
                state.set_minimize_target(&window_id, origin);
                state.needs_redraw = true;
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
            df_toplevel_manager::Request::SetAppearancePolicy {
                color_scheme,
                transparency_millionths,
                reduce_transparency,
                increase_contrast,
            } => {
                // T196: the resolved scheme plus the glass/accessibility
                // preferences in one coherent snapshot. Millionths keep the
                // schema's 0.5255 default exact on the integer wire.
                let transparency = (transparency_millionths.min(1_000_000) as f32) / 1_000_000.0;
                state.set_appearance_policy(
                    color_scheme.as_deref(),
                    transparency,
                    reduce_transparency != 0,
                    increase_contrast != 0,
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
            df_toplevel_manager::Request::SetPointerPolicy {
                pointer_speed_millionths,
                natural_scroll,
                tap_to_click,
                left_handed,
                scroll_method,
                secondary_click,
                scroll_speed_millionths,
                double_click_ms,
                haptics,
            } => {
                // T229: the live pointer/trackpad half.
                state.set_pointer_policy(
                    pointer_speed_millionths,
                    natural_scroll != 0,
                    tap_to_click != 0,
                    left_handed != 0,
                    scroll_method.as_deref(),
                    secondary_click.as_deref(),
                    scroll_speed_millionths,
                    double_click_ms,
                    haptics != 0,
                );
            }
            df_toplevel_manager::Request::SetKeyboardLayout { layout, variant } => {
                // T229: the active input source.
                state.set_keyboard_layout(layout.as_deref(), variant.as_deref());
            }
            df_toplevel_manager::Request::SetAppAccelerators {
                app_id,
                accelerators,
            } => {
                // T-14.2b: the shell mirrors the menu-broker's focus-scoped
                // accelerator table for the focused app. The engine admits
                // them; only that app's focused chord can match and system
                // shortcuts still win.
                state.set_app_accelerators(&app_id, &accelerators);
            }
            df_toplevel_manager::Request::CaptureScreenshot {
                x,
                y,
                width,
                height,
                mode,
                path,
            } => {
                // T-13.3b: the trusted shell asks the compositor to render the
                // frame itself (portal-presenter-only capture) and write a PNG.
                state.request_capture(x, y, width, height, &mode, path);
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
            df_workspace::Request::SetWallpaper {
                source,
                fit,
                color: _,
            } => {
                if let Some(output) = state.workspaces.space_output(data.id).map(str::to_string) {
                    // T-09.3: only the image `source`/`fit` change; the Space
                    // keeps its solid fallback color, so a NULL source returns
                    // it to that color and an image letterboxes over it.
                    let fit = wallpaper_fit_from_wire(
                        fit.into_result().map(|value| value as u32).unwrap_or(0),
                    );
                    if state
                        .workspaces
                        .set_wallpaper_image(&output, index, source, fit)
                    {
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
        // A mode/scale/transform change reshapes the output's logical
        // geometry, so every chrome surface anchored to it must be
        // reconfigured for the new size (T-16.3b). Without this the shell
        // keeps rendering a surface sized for the old logical output and the
        // compositor places it oversized/off-screen.
        let mut geometry_changed = false;
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
                    geometry_changed = true;
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
                    geometry_changed = true;
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
                geometry_changed = true;
            }
            df_output::Request::SetVrr { .. } | df_output::Request::SetNightLight { .. } => {
                // VRR and night-light plumbing is a T-16 displays-pane item
                // (see PROGRESS.md, T-02); accepted and acked here.
            }
            df_output::Request::SetBrightness { level } => {
                // T-11.3a: the compositor owns the output brightness. Reject a
                // non-finite level; clamp the rest into [0, 1] and remember it,
                // so the `brightness` event reports the owned value. A backend
                // applies it best-effort (headless/nested keep the value only).
                let name = output.name();
                if level.is_finite() {
                    state
                        .shell
                        .output_brightness
                        .insert(name, level.clamp(0.0, 1.0));
                } else {
                    eprintln!(
                        "dragonfruit-compositor: ignoring invalid brightness request {level}"
                    );
                }
            }
            df_output::Request::Destroy => {}
        }
        // A scale/mode/transform change reshapes the output's logical
        // geometry: re-anchor every chrome surface to it so the shell
        // re-lays-out at the new size (T-16.3b). The ack below carries the
        // new geometry/scale to the shell.
        if geometry_changed {
            // Mode/scale/transform changes all recompute the refresh policy;
            // only a mode changes the period, but one path keeps the clock and
            // degrade budget in step (T-21.2a).
            state.on_output_mode_changed(&output);
            state.reconfigure_layers();
        }
        // Ack with the applied properties.
        state.send_output_properties(resource, &output);
        resource.done();
    }

    fn destroyed(
        state: &mut Self,
        client: wayland_server::backend::ClientId,
        resource: &df_output::DfOutput,
        data: &OutputUserData,
    ) {
        state
            .shell
            .last_material_role_states
            .remove(&resource.id().protocol_id());
        if let Some(session) = state.shell.sessions.get_mut(&client) {
            session.outputs.remove(&data.name);
        }
    }
}

// --- session provisioning --------------------------------------------------

/// Provision launch tokens at session start and write the shell and desktop
/// hand-off files. `DRAGONFRUIT_LAUNCH_TOKENS` (comma-separated `[role:]hex`)
/// lets the session manager (T-24) or the conformance tests supply pre-minted
/// tokens; otherwise one random token per role is minted.
///
/// A `desktop:`-tagged entry mints a [`TrustedRole::DesktopIcons`] token for
/// the Files desktop process (T-19.3), written to its own hand-off file. The
/// single `DRAGONFRUIT_LAUNCH_TOKEN`/`DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN` vars
/// are the session-manager fallbacks. The desktop token is always written so a
/// real session can start the desktop process even when the session entry only
/// imported the shell token.
pub fn provision(state: &mut DfState) {
    let mut shell_token: Option<trust::LaunchToken> = None;
    let mut desktop_token: Option<trust::LaunchToken> = None;
    if let Ok(list) = std::env::var(trust::TOKENS_ENV) {
        for entry in list
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            let Some((role, value)) = trust::parse_token_entry(entry) else {
                continue;
            };
            let token = state.shell.trust.mint_with_value(role, value);
            match role {
                TrustedRole::DesktopIcons => desktop_token.get_or_insert(token),
                TrustedRole::Shell => shell_token.get_or_insert(token),
            };
        }
    }
    // The single-token hand-off (T-24): the session manager may pass the
    // shell's token directly instead of a list.
    if shell_token.is_none() {
        if let Ok(hex) = std::env::var(trust::TOKEN_ENV) {
            if let Some(value) = trust::LaunchToken::parse_hex(&hex) {
                shell_token = Some(state.shell.trust.mint_with_value(TrustedRole::Shell, value));
            }
        }
    }
    if let Ok(hex) = std::env::var(trust::DESKTOP_TOKEN_ENV) {
        if let Some(value) = trust::LaunchToken::parse_hex(&hex) {
            desktop_token = Some(
                state
                    .shell
                    .trust
                    .mint_with_value(TrustedRole::DesktopIcons, value),
            );
        }
    }
    let shell_token = shell_token.unwrap_or_else(|| state.shell.trust.mint(TrustedRole::Shell));
    write_token_file(
        state,
        &shell_token,
        trust::TOKEN_FILE_SUFFIX,
        "launch token",
    );
    let desktop_token =
        desktop_token.unwrap_or_else(|| state.shell.trust.mint(TrustedRole::DesktopIcons));
    write_token_file(
        state,
        &desktop_token,
        trust::DESKTOP_TOKEN_FILE_SUFFIX,
        "desktop launch token",
    );
}

/// Write one token hand-off file (`0600`) and log its path.
fn write_token_file(state: &DfState, token: &trust::LaunchToken, suffix: &str, label: &str) {
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    let path = std::path::PathBuf::from(runtime).join(format!("{}{suffix}", state.socket_name));
    match write_private(&path, token.to_hex().as_bytes()) {
        Ok(()) => println!("dragonfruit-compositor: {label}: {}", path.display()),
        Err(err) => eprintln!("dragonfruit-compositor: failed to write {label}: {err}"),
    }
}

/// A trusted client disconnected (a shell crash or a `native-restart`). Its
/// one-time token is already consumed, so mint a fresh one and rewrite the
/// hand-off file. Without this, systemd's automatic shell restart is refused
/// with `token-consumed` and the desktop stays without its menu bar and Dock
/// until the whole compositor/session is restarted.
fn reissue_token(state: &mut DfState, role: TrustedRole) {
    let token = state.shell.trust.mint(role);
    let (suffix, label) = match role {
        TrustedRole::Shell => (trust::TOKEN_FILE_SUFFIX, "launch token"),
        TrustedRole::DesktopIcons => (trust::DESKTOP_TOKEN_FILE_SUFFIX, "desktop launch token"),
    };
    write_token_file(state, &token, suffix, label);
    println!(
        "dragonfruit-compositor: reissued {label} after {} disconnect",
        role.name()
    );
}

/// Remove the shell and desktop token hand-off files at teardown.
pub fn remove_token_file(socket_name: &str) {
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    for suffix in [trust::TOKEN_FILE_SUFFIX, trust::DESKTOP_TOKEN_FILE_SUFFIX] {
        let path = std::path::PathBuf::from(&runtime).join(format!("{socket_name}{suffix}"));
        let _ = std::fs::remove_file(path);
    }
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
