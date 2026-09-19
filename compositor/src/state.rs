// SPDX-License-Identifier: MIT OR Apache-2.0
//! Compositor state and the standard Wayland protocol surface (T-02).
//!
//! One [`DfState`] serves every backend (nested, DRM, headless); backends
//! only add outputs, a renderer, and input devices on top. The protocol
//! surface advertised here is exactly the list in
//! .docs/tasks/02-compositor-core.md — no missing, no extras:
//!
//! `xdg-shell`, `xdg-output`, `presentation-time`, `linux-dmabuf`,
//! `viewporter`, `fractional-scale`, `xdg-decoration`, pointer constraints,
//! relative pointer, `cursor-shape`, `idle-inhibit`, `ext-idle-notify`,
//! `wp-content-type-manager-v1`, `wlr-data-control`, `security-context`,
//! `xdg-activation`, `ext-session-lock-v1`, `text-input`, input-methods,
//! plus the core globals (`wl_compositor`, `wl_shm`, `wl_subcompositor`,
//! `wl_seat`, `wl_data_device_manager`, `wl_output`, pointer gestures,
//! tablet manager).
//!
//! Deliberately **not** advertised: `wlr-screencopy` and every other
//! arbitrary-grab capture protocol ("if a capture is not a portal request,
//! the answer is no" — enforced by the CI grep gate), `wlr-layer-shell`
//! (replaced by our private shell protocols in T-07), `foreign-toplevel`
//! (ditto), and Xwayland plumbing (T-06).
//!
//! Window/workspace *policy* belongs to T-04/T-05; this module only keeps
//! the machinery: a Smithay [`Space`] of [`Window`]s and the seat.

use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::Color32F;
use smithay::desktop::{
    find_popup_root_surface, PopupKeyboardGrab, PopupKind, PopupManager,
    PopupPointerGrab, Space, Window,
};
use smithay::reexports::calloop::{LoopHandle, LoopSignal, RegistrationToken};
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::{wl_buffer, wl_seat, wl_surface::WlSurface};
use smithay::reexports::wayland_server::{Client, DisplayHandle, Resource as _};
use smithay::utils::{
    Clock, IsAlive, Logical, Monotonic, Point, Rectangle, Serial, Size, SERIAL_COUNTER,
};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    get_parent, with_states, CompositorClientState, CompositorHandler, CompositorState,
};
use smithay::wayland::content_type::ContentTypeState;
use smithay::wayland::cursor_shape::CursorShapeManagerState;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier};
use smithay::input::SeatHandler;
use smithay::input::Seat;
use smithay::output::Output;
use smithay::wayland::fractional_scale::{FractionalScaleHandler, FractionalScaleManagerState};
use smithay::wayland::idle_inhibit::{IdleInhibitHandler, IdleInhibitManagerState};
use smithay::wayland::idle_notify::{IdleNotifierHandler, IdleNotifierState};
use smithay::wayland::input_method::{InputMethodHandler, InputMethodManagerState};
use smithay::wayland::output::{OutputHandler, OutputManagerState};
use smithay::wayland::pointer_constraints::{PointerConstraintsHandler, PointerConstraintsState};
use smithay::wayland::pointer_gestures::PointerGesturesState;
use smithay::wayland::presentation::PresentationState;
use smithay::wayland::relative_pointer::RelativePointerManagerState;
use smithay::wayland::security_context::{
    SecurityContext, SecurityContextHandler, SecurityContextListenerSource, SecurityContextState,
};
use smithay::wayland::selection::data_device::{
    set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
    ServerDndGrabHandler,
};
use smithay::wayland::selection::wlr_data_control::{DataControlHandler, DataControlState};
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::session_lock::{SessionLockHandler, SessionLockManagerState, SessionLocker};
use smithay::wayland::shell::xdg::{
    decoration::{XdgDecorationHandler, XdgDecorationState}, PopupSurface, PositionerState,
    SurfaceCachedState, ToplevelSurface, XdgShellHandler, XdgShellState, XdgToplevelSurfaceData,
};
use smithay::wayland::seat::WaylandFocus as _;
use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::wayland::tablet_manager::{TabletManagerState, TabletSeatHandler};
use smithay::wayland::text_input::TextInputManagerState;
use smithay::wayland::viewporter::ViewporterState;
use smithay::wayland::xwayland_shell::XWaylandShellState;
use smithay::wayland::xdg_activation::{
    XdgActivationToken, XdgActivationTokenData, XdgActivationHandler, XdgActivationState,
};
use smithay::xwayland::XWaylandClientData;
use std::time::Duration;

use crate::identity::AppResolver;
use crate::input::dispatch::InputDispatch;
use crate::input::gestures::{GestureRecognizer, ProgressPipeline};
use crate::input::hot_corners::HotCornerDetector;
use crate::input::settings::InputSettings;
use crate::input::shortcuts::{GrabArbiter, ShortcutEngine};
use crate::input::{InputAction, TriggerKind};
use crate::window::grab::{MoveGrab, ResizeGrab};
use crate::window::popup::constrained_popup_geometry;
use crate::window::resize::SizeConstraints;
use crate::window::{
    cascaded_geometry, centered_on, ReservedZones, ShellWindowEvent, WindowDispatch, WindowEvent,
    WindowEventKind, WindowId, WindowMenuCommand, WindowModel, CASCADE_STEP,
};
use crate::workspace::WorkspaceModel;
use crate::xwayland::XwaylandState;

use smithay::input::pointer::Focus as PointerFocus;

use smithay::{
    delegate_compositor, delegate_content_type, delegate_cursor_shape, delegate_data_control,
    delegate_data_device, delegate_dmabuf, delegate_fractional_scale, delegate_idle_inhibit,
    delegate_idle_notify, delegate_input_method_manager, delegate_output,
    delegate_pointer_constraints, delegate_pointer_gestures, delegate_presentation,
    delegate_relative_pointer, delegate_seat, delegate_security_context, delegate_session_lock,
    delegate_shm, delegate_tablet_manager, delegate_text_input_manager, delegate_viewporter,
    delegate_xdg_activation, delegate_xdg_decoration, delegate_xdg_shell, delegate_xwayland_shell,
};

/// Per-client bookkeeping.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct DfClientState {
    pub compositor_state: CompositorClientState,
    /// Set when the client identifies itself via `security-context`
    /// (sandboxed/Flatpak clients) — the basis of T-07/T-27 access control.
    pub security_context: Option<SecurityContext>,
}

impl ClientData for DfClientState {
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

/// Render-path counters (FR-5: verified via render-path counters).
#[derive(Debug, Default, Clone, Copy)]
pub struct RenderStats {
    /// Frames actually rendered (or scanout-composited).
    pub frames_rendered: u64,
    /// Renders skipped because the damage tracker found no damage (FR-2).
    pub frames_skipped_no_damage: u64,
    /// Frames pushed through the direct-scanout path instead of GL
    /// compositing (DRM backend only).
    pub direct_scanouts: u64,
}

/// Central compositor state: protocols, outputs, scene, seat.
///
/// The protocol-state fields exist to keep their globals alive and as the
/// handler accessors (smithay's delegate pattern); many are not read
/// directly by T-02 but are consumed by later tickets.
#[allow(dead_code)]
pub struct DfState {
    pub running: bool,
    /// Set when something asked for a new frame; cleared by the render
    /// pass. Damage-driven rendering starts here (FR-2).
    pub needs_redraw: bool,
    pub display_handle: DisplayHandle,
    pub loop_handle: LoopHandle<'static, DfState>,
    pub loop_signal: LoopSignal,
    pub clock: Clock<Monotonic>,
    /// The Wayland socket name; used to name the per-session Xwayland
    /// `DISPLAY` hand-off file (T-06).
    pub socket_name: String,

    // --- protocol states ---------------------------------------------------
    pub compositor_state: CompositorState,
    pub shm_state: ShmState,
    /// `linux-dmabuf`; `None` until the backend provides a renderer (the
    /// global is created lazily because it needs the renderer's format
    /// list). Headless (no renderer) never creates it.
    pub dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    /// Format list the `linux-dmabuf` global was created with; imports of
    /// other formats are rejected at the protocol layer.
    dmabuf_formats: Vec<smithay::backend::allocator::Format>,
    pub xdg_shell_state: XdgShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_state: FractionalScaleManagerState,
    pub output_manager_state: OutputManagerState,
    pub presentation_state: PresentationState,
    pub pointer_constraints_state: PointerConstraintsState,
    pub relative_pointer_state: RelativePointerManagerState,
    pub pointer_gestures_state: PointerGesturesState,
    pub cursor_shape_state: CursorShapeManagerState,
    pub idle_inhibit_state: IdleInhibitManagerState,
    pub idle_notifier_state: IdleNotifierState<DfState>,
    pub content_type_state: ContentTypeState,
    pub data_device_state: DataDeviceState,
    pub data_control_state: DataControlState,
    pub security_context_state: SecurityContextState,
    pub xdg_activation_state: XdgActivationState,
    pub session_lock_state: SessionLockManagerState,
    pub text_input_state: TextInputManagerState,
    pub input_method_state: InputMethodManagerState,
    pub tablet_manager_state: TabletManagerState,
    /// `xwayland_shell` — the role protocol Xwayland binds before it can
    /// create a `wl_surface` for an X11 window (T-06).
    pub xwayland_shell_state: XWaylandShellState,

    // --- seat ---------------------------------------------------------------
    pub seat_state: smithay::input::SeatState<DfState>,
    pub seat: smithay::input::Seat<DfState>,

    // --- scene --------------------------------------------------------------
    pub space: Space<Window>,
    pub popups: PopupManager,
    /// Toplevels that committed but have no buffer yet; mapped into the
    /// space on first buffer (with T-02's temporary placement policy —
    /// real placement/stacking policy is T-04).
    pub pending_windows: Vec<Window>,
    /// Surfaces currently inhibiting idle.
    pub idle_inhibitors: Vec<WlSurface>,
    pub cursor_image: smithay::input::pointer::CursorImageStatus,

    // --- window model (T-04) ------------------------------------------------
    /// Compositor-owned window metadata: states, restore geometry,
    /// transient relationships, and the per-output cascade.
    pub windows: WindowModel,
    /// Window lifecycle/focus broadcasts for the shell (T-07 seam).
    pub window_dispatch: WindowDispatch,
    /// Reserved zones (menu bar, Dock) that Zoom fills around; supplied by
    /// the shell over the private protocol in T-07.
    pub reserved_zones: ReservedZones,
    /// The window that currently owns keyboard focus, if any.
    pub active_window: Option<Window>,

    // --- workspace model (T-05) --------------------------------------------
    /// Per-output ordered Space lists, fullscreen Spaces, wallpaper, window
    /// assignment, and app Space memory. The compositor is the sole owner;
    /// the shell consumes events and keeps no copy.
    pub workspaces: WorkspaceModel,

    // --- application identity (T-06 interim; T-23 owns app-index) -----------
    /// WM_CLASS -> `.desktop` resolver for X11 (and the seam T-23 replaces).
    pub app_resolver: AppResolver,

    // --- Xwayland (T-06) ----------------------------------------------------
    /// The Xwayland server, X11 window manager, and `DISPLAY` state.
    pub xwayland: XwaylandState,

    // --- input engine (T-03) ------------------------------------------------
    /// Global shortcut engine: the sole arbiter of key bindings.
    pub shortcuts: ShortcutEngine,
    /// Gesture recognizer feeding [`Self::progress`].
    pub gestures: GestureRecognizer,
    /// The single progress pipeline shared by gestures, keyboard, and hot
    /// corners.
    pub progress: ProgressPipeline,
    /// Hot-corner dwell detection.
    pub hot_corners: HotCornerDetector,
    /// The live input settings model (FR-7).
    pub input_settings: InputSettings,
    /// The outbox/audit log every trigger writes to (shell protocol T-07).
    pub input_dispatch: InputDispatch,
    /// Gate for every client grab request (FR-5).
    pub grab_arbiter: GrabArbiter,
    /// Pending hot-corner dwell timer, if armed.
    pub hot_corner_timer: Option<RegistrationToken>,

    pub stats: RenderStats,
}

impl DfState {
    /// Create all protocol globals. Runs for every backend (FR-1).
    pub fn new(
        display_handle: &DisplayHandle,
        loop_handle: LoopHandle<'static, DfState>,
        loop_signal: LoopSignal,
    ) -> Self {
        let compositor_state = CompositorState::new::<DfState>(display_handle);
        let shm_state = ShmState::new::<DfState>(display_handle, Vec::new());
        let xdg_shell_state = XdgShellState::new::<DfState>(display_handle);
        let xdg_decoration_state = XdgDecorationState::new::<DfState>(display_handle);
        let viewporter_state = ViewporterState::new::<DfState>(display_handle);
        let fractional_scale_state = FractionalScaleManagerState::new::<DfState>(display_handle);
        let output_manager_state =
            OutputManagerState::new_with_xdg_output::<DfState>(display_handle);
        let presentation_state = PresentationState::new::<DfState>(display_handle, 1);
        let pointer_constraints_state = PointerConstraintsState::new::<DfState>(display_handle);
        let relative_pointer_state = RelativePointerManagerState::new::<DfState>(display_handle);
        let pointer_gestures_state = PointerGesturesState::new::<DfState>(display_handle);
        let cursor_shape_state = CursorShapeManagerState::new::<DfState>(display_handle);
        let idle_inhibit_state = IdleInhibitManagerState::new::<DfState>(display_handle);
        let idle_notifier_state = IdleNotifierState::new(display_handle, loop_handle.clone());
        let content_type_state = ContentTypeState::new::<DfState>(display_handle);
        let data_device_state = DataDeviceState::new::<DfState>(display_handle);
        let data_control_state =
            DataControlState::new::<DfState, _>(display_handle, None, |_| true);
        let security_context_state =
            SecurityContextState::new::<DfState, _>(display_handle, |_| true);
        let xdg_activation_state = XdgActivationState::new::<DfState>(display_handle);
        // TODO(T-26): session locks should only be granted to the session's
        // lock UI; the open filter is fine while no lock UI exists.
        let session_lock_state =
            SessionLockManagerState::new::<DfState, _>(display_handle, |_| true);
        let text_input_state = TextInputManagerState::new::<DfState>(display_handle);
        // Input-method clients get a dedicated socket; the open filter is
        // tightened together with the T-07 shell-token work.
        let input_method_state =
            InputMethodManagerState::new::<DfState, _>(display_handle, |_| true);
        let tablet_manager_state = TabletManagerState::new::<DfState>(display_handle);
        let xwayland_shell_state = XWaylandShellState::new::<DfState>(display_handle);

        let mut seat_state = smithay::input::SeatState::new();
        let seat = seat_state.new_wl_seat(display_handle, "dragonfruit");

        let input_settings = InputSettings::default();
        let shortcuts = ShortcutEngine::with_bindings(input_settings.system_bindings.clone());
        let gestures = GestureRecognizer::new(input_settings.gestures);
        let progress = ProgressPipeline::new(input_settings.progress);
        let hot_corners = HotCornerDetector::new(input_settings.hot_corners);

        DfState {
            running: true,
            needs_redraw: true,
            display_handle: display_handle.clone(),
            loop_handle,
            loop_signal,
            clock: Clock::new(),
            socket_name: String::new(),
            compositor_state,
            shm_state,
            dmabuf_state: None,
            dmabuf_formats: Vec::new(),
            xdg_shell_state,
            xdg_decoration_state,
            viewporter_state,
            fractional_scale_state,
            output_manager_state,
            presentation_state,
            pointer_constraints_state,
            relative_pointer_state,
            pointer_gestures_state,
            cursor_shape_state,
            idle_inhibit_state,
            idle_notifier_state,
            content_type_state,
            data_device_state,
            data_control_state,
            security_context_state,
            xdg_activation_state,
            session_lock_state,
            text_input_state,
            input_method_state,
            tablet_manager_state,
            xwayland_shell_state,
            seat_state,
            seat,
            space: Space::default(),
            popups: PopupManager::default(),
            pending_windows: Vec::new(),
            idle_inhibitors: Vec::new(),
            cursor_image: smithay::input::pointer::CursorImageStatus::default_named(),
            windows: WindowModel::new(),
            window_dispatch: WindowDispatch::new(),
            reserved_zones: ReservedZones::default(),
            active_window: None,
            workspaces: WorkspaceModel::new(),
            app_resolver: AppResolver::load(),
            xwayland: XwaylandState::default(),
            shortcuts,
            gestures,
            progress,
            hot_corners,
            input_settings,
            input_dispatch: InputDispatch::new(),
            grab_arbiter: GrabArbiter::new(),
            hot_corner_timer: None,
            stats: RenderStats::default(),
        }
    }

    /// Advertise `linux-dmabuf`. Called by backends once their renderer
    /// exists; `main_device` enables v4 default-feedback.
    pub fn init_dmabuf(
        &mut self,
        formats: impl Iterator<Item = smithay::backend::allocator::Format>,
        main_device: Option<libc::dev_t>,
    ) {
        let dh = self.display_handle.clone();
        let formats: Vec<_> = formats.collect();
        self.dmabuf_formats = formats.clone();
        let mut dmabuf_state = DmabufState::new();
        let global = match main_device {
            Some(dev) => {
                let feedback = smithay::wayland::dmabuf::DmabufFeedbackBuilder::new(dev, formats)
                    .build()
                    .expect("failed to build default dmabuf feedback");
                dmabuf_state.create_global_with_default_feedback::<DfState>(&dh, &feedback)
            }
            None => dmabuf_state.create_global::<DfState>(&dh, formats),
        };
        self.dmabuf_state = Some((dmabuf_state, global));
    }

    /// Map pending toplevels into the scene once they have a buffer.
    ///
    /// Placement policy (T-04 FR-5/FR-6): transient dialogs center on
    /// their parent; ordinary windows open centered on the active output
    /// with a wrapping per-output cascade. Stacking and focus remain
    /// compositor state.
    pub fn map_pending_windows(&mut self) {
        let mut to_map = Vec::new();
        self.pending_windows.retain(|window| {
            let has_buffer = window.toplevel().is_some_and(|toplevel| {
                let surface = toplevel.wl_surface();
                // The buffer is consumed out of `SurfaceAttributes` by
                // `on_commit_buffer_handler` (called first in `commit`), so
                // the attached buffer must be read from the renderer state.
                smithay::backend::renderer::utils::with_renderer_surface_state(surface, |state| {
                    state.buffer().is_some()
                })
                .unwrap_or(false)
            });
            if has_buffer {
                to_map.push(window.clone());
                false
            } else {
                true
            }
        });

        let mut mapped = 0;
        let mut switched = false;
        for window in to_map {
            let size = window.bbox().size;
            let parent = window
                .toplevel()
                .and_then(|toplevel| toplevel.parent())
                .and_then(|parent| self.window_for_surface(&parent));

            let geometry = if let Some(parent) = &parent {
                let parent_geometry = self
                    .windows
                    .geometry(parent)
                    .or_else(|| self.space.element_geometry(parent))
                    .unwrap_or_default();
                centered_on(parent_geometry, size)
            } else {
                let (output_name, output_geometry) = self.primary_output();
                let index = output_name
                    .map(|name| self.windows.cascade_index(&name))
                    .unwrap_or(0);
                output_geometry
                    .map(|output| cascaded_geometry(output, size, index, CASCADE_STEP))
                    .unwrap_or_else(|| Rectangle::new(Point::from((0, 0)), size))
            };

            let id = self.windows.insert(window.clone(), geometry);
            // Capture metadata set before the first buffer commit.
            if let Some(toplevel) = window.toplevel() {
                self.windows.set_app_id(&window, toplevel_app_id(toplevel));
                self.windows.set_title(&window, toplevel_title(toplevel));
            }
            if let Some(parent) = parent {
                self.windows.set_parent(&window, &parent);
            }
            // Workspace assignment: a new window opens on the Space its app
            // remembers (activating it in lockstep), otherwise on the active
            // Space of the output it is placed on (FR-5).
            if self.assign_new_window_space(&window, id) {
                switched = true;
            }
            if self.window_on_active_space(&window) {
                self.space.map_element(window.clone(), geometry.loc, true);
            }
            self.broadcast_window(&window, WindowEventKind::Mapped);
            mapped += 1;
        }
        if switched {
            self.apply_workspace_layout();
        }
        if mapped > 0 {
            self.needs_redraw = true;
        }
    }

    /// The output new windows are placed on until Spaces (T-05) select one.
    pub fn primary_output(&self) -> (Option<String>, Option<Rectangle<i32, Logical>>) {
        match self.space.outputs().next() {
            Some(output) => (Some(output.name()), self.space.output_geometry(output)),
            None => (None, None),
        }
    }

    // --- workspaces (T-05) --------------------------------------------------

    /// The output `window` currently occupies, or the primary output.
    pub fn output_name_for(&self, window: &Window) -> Option<String> {
        if let Some(output) = self.space.outputs_for_element(window).into_iter().next() {
            return Some(output.name());
        }
        self.primary_output().0
    }

    /// Assign a newly mapped window to a Space. It goes to the Space its app
    /// remembers (activating that Space in lockstep, FR-5) or the active
    /// Space of the primary output otherwise. Returns true if the active
    /// Space changed.
    pub(crate) fn assign_new_window_space(&mut self, window: &Window, id: WindowId) -> bool {
        let (Some(output_name), _) = self.primary_output() else {
            return false;
        };
        let remembered = self
            .windows
            .app_id(window)
            .and_then(|app| self.workspaces.app_space_index(app));
        let (index, switched) = match remembered {
            Some(index) => (index, self.workspaces.activate_all(index)),
            None => (
                self.workspaces.active_index(&output_name).unwrap_or(0),
                false,
            ),
        };
        if let Some(space) = self.workspaces.space_at(&output_name, index) {
            self.workspaces.assign_window(id, space);
        }
        if let Some(app) = self.windows.app_id(window).map(str::to_string) {
            self.workspaces.remember_app(&app, index);
        }
        switched
    }

    /// Whether `window`'s assigned Space is currently active on its output.
    /// Unassigned windows are treated as active (pre-T-05 behavior).
    pub fn window_on_active_space(&self, window: &Window) -> bool {
        self.windows
            .id(window)
            .map(|id| self.window_on_active_space_id(id))
            .unwrap_or(true)
    }

    fn window_on_active_space_id(&self, id: WindowId) -> bool {
        let Some(space) = self.workspaces.window_space(id) else {
            return true;
        };
        let Some(output) = self.workspaces.space_output(space) else {
            return true;
        };
        self.workspaces.active_space(output) == Some(space)
    }

    /// Map only the visible windows of each output's active Space and unmap
    /// the rest. Minimized windows stay unmapped regardless (FR-6).
    pub fn apply_workspace_layout(&mut self) {
        let entries: Vec<(Window, WindowId)> = self
            .windows
            .windows()
            .filter_map(|window| self.windows.id(window).map(|id| (window.clone(), id)))
            .collect();
        let mut changed = false;
        for (window, id) in entries {
            let visible = self
                .windows
                .state(&window)
                .is_some_and(|state| state.is_visible());
            if visible && self.window_on_active_space_id(id) {
                let geometry = self.windows.geometry(&window).unwrap_or_default();
                self.space.map_element(window.clone(), geometry.loc, false);
                changed = true;
            } else if self.space.element_location(&window).is_some() {
                self.space.unmap_elem(&window);
                changed = true;
            }
        }
        if changed {
            self.needs_redraw = true;
        }
    }

    /// React to a workspace change: drop focus from a now-hidden window and
    /// re-apply the scene layout (FR-8: state and scene change together).
    pub fn after_workspace_change(&mut self) {
        if let Some(active) = self.active_window.clone() {
            if !self.window_on_active_space(&active) {
                if let Some(keyboard) = self.seat.get_keyboard() {
                    keyboard.set_focus(self, None, SERIAL_COUNTER.next_serial());
                }
            }
        }
        self.apply_workspace_layout();
    }

    /// Dispatch a workspace action from any trigger (keyboard, gesture, hot
    /// corner, shell). Returns true if the active Space changed.
    pub fn handle_workspace_action(&mut self, action: InputAction) -> bool {
        let changed = match action {
            InputAction::WorkspaceNext => self.workspaces.switch_all(1),
            InputAction::WorkspacePrev => self.workspaces.switch_all(-1),
            InputAction::WorkspaceActivate(index) => self.workspaces.activate_all(index),
            _ => false,
        };
        if changed {
            self.after_workspace_change();
        }
        changed
    }

    /// Move a window to the Space at `index` on its output (FR-5) and update
    /// the app's Space memory.
    pub fn move_window_to_space(&mut self, window: &Window, index: usize) {
        let Some(id) = self.windows.id(window) else {
            return;
        };
        let output = self
            .workspaces
            .window_space(id)
            .and_then(|space| self.workspaces.space_output(space).map(str::to_string))
            .or_else(|| self.output_name_for(window));
        let Some(output) = output else {
            return;
        };
        if self.workspaces.move_window(id, &output, index).is_none() {
            return;
        }
        if let Some(app) = self.windows.app_id(window).map(str::to_string) {
            self.workspaces.remember_app(&app, index);
        }
        self.after_workspace_change();
    }

    /// The wallpaper color of `output`'s active Space, for the backend's
    /// clear pass. The compositor renders the desktop background; the shell
    /// never does ([03-workspaces.md]).
    pub fn wallpaper_color_for(&self, output: &Output) -> Color32F {
        let color = self
            .workspaces
            .active_wallpaper(&output.name())
            .map(|wallpaper| wallpaper.color)
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        Color32F::new(color[0], color[1], color[2], color[3])
    }

    /// Hotplug attach: a fresh Space list for the new output (FR-7).
    pub fn on_output_added(&mut self, output: &Output) {
        self.workspaces.add_output(&output.name());
    }

    /// Hotplug detach: migrate the output's windows to the remaining primary
    /// output's current Space before its Spaces are destroyed (FR-7).
    pub fn on_output_removed(&mut self, output: &Output) {
        let removed = output.name();
        let primary = self
            .space
            .outputs()
            .map(|candidate| candidate.name())
            .find(|name| *name != removed);
        self.workspaces.remove_output(&removed, primary.as_deref());
        self.after_workspace_change();
    }

    /// The window whose toplevel (or popup root) is `surface`.
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|window| window.wl_surface().as_deref() == Some(surface))
            .cloned()
            .or_else(|| {
                self.pending_windows
                    .iter()
                    .find(|window| window.wl_surface().as_deref() == Some(surface))
                    .cloned()
            })
            .or_else(|| {
                // Popup surfaces resolve to their root toplevel.
                let popup = self.popups.find_popup(surface)?;
                let root = find_popup_root_surface(&popup).ok()?;
                self.space
                    .elements()
                    .find(|window| window.wl_surface().as_deref() == Some(&root))
                    .cloned()
            })
    }

    /// The output geometry that currently contains `window`, or the first
    /// output as a fallback.
    pub fn output_bounds_for(&self, window: &Window) -> Option<Rectangle<i32, Logical>> {
        for output in self.space.outputs_for_element(window) {
            if let Some(geometry) = self.space.output_geometry(&output) {
                return Some(geometry);
            }
        }
        self.space
            .outputs()
            .next()
            .and_then(|output| self.space.output_geometry(output))
    }

    /// The usable geometry (output minus reserved zones) of the output
    /// `window` occupies — the Zoom target (FR-1).
    pub fn usable_geometry_for(&self, window: &Window) -> Option<Rectangle<i32, Logical>> {
        let output = self.output_bounds_for(window)?;
        Some(self.reserved_zones.usable(output))
    }

    /// Move a floating window to `location` (the caller clamps to output).
    pub fn move_window(&mut self, window: &Window, location: Point<i32, Logical>) {
        let Some(size) = self.windows.floating_geometry(window).map(|geo| geo.size) else {
            return;
        };
        self.windows
            .set_floating_geometry(window, Rectangle::new(location, size));
        self.space.map_element(window.clone(), location, false);
        self.needs_redraw = true;
    }

    /// Resize a floating window and push the new size to the client.
    pub fn resize_window(&mut self, window: &Window, geometry: Rectangle<i32, Logical>) {
        if !self.windows.contains(window) {
            return;
        }
        self.windows.set_floating_geometry(window, geometry);
        self.space.map_element(window.clone(), geometry.loc, false);
        self.configure_window_size(window, geometry.size);
        self.needs_redraw = true;
    }

    /// Client min/max-size and aspect hints for a window.
    ///
    /// Wayland toplevels use `xdg_surface` cached state; X11 windows use
    /// `WM_NORMAL_HINTS` (T-06), including the aspect ratio.
    pub fn window_size_constraints(&self, window: &Window) -> SizeConstraints {
        if let Some(x11) = window.x11_surface() {
            let aspect = x11
                .size_hints()
                .and_then(|hints| hints.aspect)
                .map(|(min, _)| (min.numerator.max(0) as u32, min.denominator.max(0) as u32));
            return SizeConstraints {
                min: x11.min_size().unwrap_or_default(),
                max: x11.max_size().unwrap_or_default(),
                aspect,
            };
        }
        let Some(surface) = window.wl_surface() else {
            return SizeConstraints::default();
        };
        with_states(&surface, |states| {
            let mut cached = states.cached_state.get::<SurfaceCachedState>();
            let current = cached.current();
            SizeConstraints {
                min: current.min_size,
                max: current.max_size,
                aspect: None,
            }
        })
    }

    /// Zoom a window to fill the usable area (FR-1/FR-2).
    pub fn zoom_window(&mut self, window: &Window) {
        let Some(target) = self.usable_geometry_for(window) else {
            return;
        };
        let Some(transition) = self.windows.apply(window, WindowEvent::Zoom, target) else {
            return;
        };
        // Only touch the client's pending state when the compositor state
        // actually changed: setting Maximized on a window the state machine
        // refused (e.g. minimized) would desync the next configure from the
        // compositor's real state.
        if transition.changed {
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.set(xdg_toplevel::State::Maximized);
                });
            }
        }
        self.apply_window_transition(window, transition);
    }

    /// Return a zoomed window to its floating geometry.
    pub fn unzoom_window(&mut self, window: &Window) {
        let Some(transition) =
            self.windows
                .apply(window, WindowEvent::Unzoom, Rectangle::default())
        else {
            return;
        };
        if transition.changed {
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Maximized);
                });
            }
        }
        self.apply_window_transition(window, transition);
    }

    /// Enter fullscreen: the window moves to a dedicated Space created for
    /// it (FR-3), which appears in the strip right after the origin Space.
    pub fn fullscreen_window(&mut self, window: &Window) {
        let Some(target) = self.output_bounds_for(window) else {
            return;
        };
        // Create the fullscreen Space before applying the state so the
        // transition maps the window into an active Space. A repeated
        // fullscreen request must not create a second dedicated Space.
        let already_fullscreen =
            self.windows.state(window) == Some(crate::window::WindowState::Fullscreen);
        if !already_fullscreen {
            if let Some(id) = self.windows.id(window) {
                if let Some(origin) = self.workspaces.window_space(id) {
                    self.workspaces.enter_fullscreen(id, origin);
                }
            }
        }
        let Some(transition) = self
            .windows
            .apply(window, WindowEvent::EnterFullscreen, target)
        else {
            return;
        };
        if transition.changed {
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.set(xdg_toplevel::State::Fullscreen);
                });
            }
        }
        self.apply_window_transition(window, transition);
        if !already_fullscreen {
            // The active Space changed on every output, so hide the origin
            // Space's windows on the owner output too.
            self.apply_workspace_layout();
        }
    }

    /// Leave fullscreen for the state it was entered from, destroying the
    /// dedicated Space and returning to the origin Space (FR-3).
    pub fn unfullscreen_window(&mut self, window: &Window) {
        let Some(transition) =
            self.windows
                .apply(window, WindowEvent::ExitFullscreen, Rectangle::default())
        else {
            return;
        };
        if transition.changed {
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Fullscreen);
                });
            }
            if let Some(id) = self.windows.id(window) {
                self.workspaces.exit_fullscreen(id);
            }
        }
        self.apply_window_transition(window, transition);
        self.apply_workspace_layout();
    }

    /// Minimize a window and its transients (FR-6).
    pub fn minimize_window(&mut self, window: &Window) {
        for target in self.windows.transient_tree(window) {
            let Some(transition) =
                self.windows
                    .apply(&target, WindowEvent::Minimize, Rectangle::default())
            else {
                continue;
            };
            if transition.changed {
                self.space.unmap_elem(&target);
                self.broadcast_state(&target);
            }
        }
        self.needs_redraw = true;
    }

    /// Restore a minimized window and its transients (FR-6).
    #[allow(dead_code)] // Shell/Dock restore lands with T-07/T-10.
    pub fn restore_window(&mut self, window: &Window) {
        for target in self.windows.transient_tree(window) {
            let Some(transition) =
                self.windows
                    .apply(&target, WindowEvent::Restore, Rectangle::default())
            else {
                continue;
            };
            if transition.changed {
                let geometry = self.windows.geometry(&target).unwrap_or_default();
                // Only the requested window takes activation; transients
                // are raised with it without stealing focus from it. A
                // window whose Space is not active stays unmapped until its
                // Space is shown again (FR-6).
                let activate = &target == window;
                if self.window_on_active_space(&target) {
                    self.space
                        .map_element(target.clone(), geometry.loc, activate);
                }
                self.broadcast_state(&target);
            }
        }
        self.needs_redraw = true;
    }

    /// Apply a window-menu primitive (SSD titlebar menu T-13, protocol T-07).
    #[allow(dead_code)] // Consumed by the T-13 titlebar menu and T-07.
    pub fn window_menu_command(&mut self, window: &Window, command: WindowMenuCommand) {
        match command {
            WindowMenuCommand::MoveToSpace(index) => {
                self.move_window_to_space(window, index);
            }
            WindowMenuCommand::Minimize => self.minimize_window(window),
            WindowMenuCommand::Zoom => {
                if self.windows.state(window) == Some(crate::window::WindowState::Zoomed) {
                    self.unzoom_window(window);
                } else {
                    self.zoom_window(window);
                }
            }
            WindowMenuCommand::Close => {
                if let Some(toplevel) = window.toplevel() {
                    toplevel.send_close();
                }
            }
        }
    }

    /// Apply a state transition to the scene: map/unmap, reconfigure, and
    /// broadcast.
    fn apply_window_transition(
        &mut self,
        window: &Window,
        transition: crate::window::WindowTransition,
    ) {
        if !transition.changed {
            return;
        }
        let geometry = self.windows.geometry(window).unwrap_or_default();
        if transition.to.is_visible() {
            // A window whose Space is not the active one stays out of the
            // scene until that Space is shown (T-05).
            if self.window_on_active_space(window) {
                self.space.map_element(window.clone(), geometry.loc, true);
                self.configure_window_size(window, geometry.size);
            }
        } else {
            self.space.unmap_elem(window);
        }
        self.broadcast_state(window);
        self.needs_redraw = true;
    }

    /// Send a configure with `size` to a toplevel if it changed.
    pub(crate) fn configure_window_size(&mut self, window: &Window, size: Size<i32, Logical>) {
        // X11 has no xdg configure: push the full compositor-owned geometry
        // to the X server (T-06 FR-1). The X client reflects it via
        // ConfigureNotify.
        if let Some(x11) = window.x11_surface() {
            let geometry = self
                .windows
                .geometry(window)
                .unwrap_or_else(|| Rectangle::new(Point::from((0, 0)), size));
            let _ = x11.configure(Some(geometry));
            return;
        }
        let Some(toplevel) = window.toplevel() else {
            return;
        };
        if !toplevel.is_initial_configure_sent() {
            return;
        }
        let changed = toplevel.with_pending_state(|state| {
            if state.size == Some(size) {
                false
            } else {
                state.size = Some(size);
                true
            }
        });
        if changed {
            toplevel.send_pending_configure();
        }
    }

    /// Broadcast a window event, tagging it with identity.
    pub(crate) fn broadcast_window(&mut self, window: &Window, kind: WindowEventKind) {
        let Some(id) = self.windows.id(window) else {
            return;
        };
        self.window_dispatch.push(ShellWindowEvent {
            kind,
            id,
            app_id: self.windows.app_id(window).map(str::to_string),
            title: self.windows.title(window).map(str::to_string),
        });
    }

    /// Broadcast a state change.
    pub(crate) fn broadcast_state(&mut self, window: &Window) {
        let (Some(id), Some(state)) = (self.windows.id(window), self.windows.state(window)) else {
            return;
        };
        self.window_dispatch.state_changed(
            id,
            state,
            self.windows.app_id(window).map(str::to_string),
            self.windows.title(window).map(str::to_string),
        );
    }

    /// Dismiss every popup rooted at `window`'s toplevel surface (FR-11).
    pub(crate) fn dismiss_popups_for(&mut self, window: &Window) {
        if !window.alive() {
            return;
        }
        let Some(surface) = window.wl_surface().map(|surface| surface.into_owned()) else {
            return;
        };
        let popups: Vec<PopupKind> = PopupManager::popups_for_surface(&surface)
            .map(|(popup, _)| popup)
            .collect();
        for popup in popups {
            let _ = PopupManager::dismiss_popup(&surface, &popup);
        }
    }

    /// Compute and store a popup's constrained geometry (FR-11).
    fn set_popup_geometry(&mut self, surface: &PopupSurface, positioner: PositionerState) {
        let parent_geometry = surface
            .get_parent_surface()
            .and_then(|parent| self.window_for_surface(&parent))
            .and_then(|window| self.windows.geometry(&window))
            .unwrap_or_default();
        let output_geometry = self
            .space
            .outputs()
            .next()
            .and_then(|output| self.space.output_geometry(output))
            .unwrap_or(parent_geometry);
        let geometry = constrained_popup_geometry(positioner, parent_geometry, output_geometry);
        surface.with_pending_state(|state| {
            state.geometry = geometry;
        });
    }

    /// Update idle notification activity state after seat/input activity.
    pub fn notify_activity(&mut self) {
        self.idle_notifier_state.notify_activity(&self.seat);
    }

    /// A monotonic timestamp in milliseconds for the input pipelines.
    pub fn now_msec(&self) -> u64 {
        Duration::from(self.clock.now()).as_millis() as u64
    }

    /// Print the render-path counters (FR-2/FR-5 observability).
    ///
    /// Emitted on SIGUSR1 and on clean exit so the idle-trace (FR-2) and
    /// direct-scanout (FR-5) budgets can be measured without a debugger.
    pub fn dump_stats(&self, label: &str) {
        println!(
            "dragonfruit-compositor: render stats ({label}): \
             frames_rendered={} frames_skipped_no_damage={} direct_scanouts={}",
            self.stats.frames_rendered,
            self.stats.frames_skipped_no_damage,
            self.stats.direct_scanouts
        );
        // Identity resolution rate (T-06 acceptance): the misses are the
        // input T-23's app-index heuristics consume.
        let misses: Vec<&str> = self.app_resolver.misses().collect();
        println!(
            "dragonfruit-compositor: identity stats ({label}): apps={} resolved={} \
             unresolved={} misses={misses:?}",
            self.app_resolver.len(),
            self.app_resolver.resolved_count(),
            self.app_resolver.unresolved_count(),
        );
        if let Some(display) = &self.xwayland.display {
            println!(
                "dragonfruit-compositor: xwayland stats ({label}): display={display} \
                 starts={} running={}",
                self.xwayland.start_count, self.xwayland.running
            );
        }
    }

    /// Dispatch one compositor action from any trigger.
    ///
    /// Every trigger records the same [`InputAction`] here; progress-driven
    /// actions additionally drive the shared [`ProgressPipeline`]. Gesture
    /// sources have already driven the pipeline (so the gesture remains
    /// continuous), hence they only record the committed action.
    pub fn dispatch_input_action(&mut self, action: InputAction, source: TriggerKind, serial: u32) {
        self.input_dispatch.action(action, source, serial);
        if action.is_progress_driven() && !matches!(source, TriggerKind::Gesture(_)) {
            let now = self.now_msec();
            for event in self.progress.drive_discrete(action, source, now) {
                self.input_dispatch.progress(event);
            }
        }
        // Workspace switches take effect immediately; the progress events
        // above are the T-11 animation seam, not a second state machine.
        self.handle_workspace_action(action);
        self.needs_redraw = true;
    }

    /// Replace the input settings and apply them live (FR-7).
    #[allow(dead_code)] // Settings app (T-16) is the writer.
    pub fn set_input_settings(&mut self, settings: InputSettings) {
        self.input_settings = settings;
        self.apply_input_settings();
    }

    /// Apply the current input settings to the live seat and detectors.
    ///
    /// Keyboard repeat and the gesture/hot-corner/commit tunables take
    /// effect immediately. Per-device pointer acceleration is stored for
    /// the backend to apply when it configures libinput devices (T-16
    /// wires the Settings pane; the DRM backend owns device handles).
    #[allow(dead_code)] // Called via set_input_settings (T-16).
    pub fn apply_input_settings(&mut self) {
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.change_repeat_info(
                self.input_settings.keyboard.repeat_rate_hz,
                self.input_settings.keyboard.repeat_delay_ms,
            );
        }
        self.shortcuts
            .set_system_bindings(self.input_settings.system_bindings.clone());
        self.gestures.set_config(self.input_settings.gestures);
        self.progress.set_config(self.input_settings.progress);
        self.hot_corners.set_config(self.input_settings.hot_corners);
        self.needs_redraw = true;
    }
}

// --- Wayland dispatch -------------------------------------------------------

/// The `app_id` a toplevel currently advertises, if any.
fn toplevel_app_id(surface: &ToplevelSurface) -> Option<String> {
    with_states(surface.wl_surface(), |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .and_then(|data| data.lock().ok())
            .and_then(|attributes| attributes.app_id.clone())
    })
}

/// The title a toplevel currently advertises, if any.
fn toplevel_title(surface: &ToplevelSurface) -> Option<String> {
    with_states(surface.wl_surface(), |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .and_then(|data| data.lock().ok())
            .and_then(|attributes| attributes.title.clone())
    })
}

impl BufferHandler for DfState {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl CompositorHandler for DfState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        // The Xwayland connection carries its own client data type; every
        // other client uses ours. Both expose a `CompositorClientState`.
        if let Some(state) = client.get_data::<XWaylandClientData>() {
            return &state.compositor_state;
        }
        &client.get_data::<DfClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        // Import wl_shm buffers (glow/EGL backends); headless has no
        // renderer and simply ignores buffers.
        smithay::backend::renderer::utils::on_commit_buffer_handler::<DfState>(surface);

        // Keep popup trees in sync (moves unmapped popups into their
        // parent's tree so they stack above the toplevel — FR-11).
        self.popups.commit(surface);

        // Refresh the window's cached bounding box before placement reads
        // it. `Window::on_commit` recomputes it from the renderer surface
        // state; without this the bbox stays 0×0 and every window would be
        // placed at zero size (and restore geometry would be lost).
        let window = self
            .pending_windows
            .iter()
            .chain(self.space.elements())
            .find(|window| window.wl_surface().as_deref() == Some(surface))
            .cloned();
        if let Some(window) = window {
            window.on_commit();
        }

        // Map the toplevel once its root tree has a buffer; T-04 owns
        // placement policy.
        let mut root = surface.clone();
        while let Some(parent) = get_parent(&root) {
            root = parent;
        }
        let pending = self
            .pending_windows
            .iter()
            .any(|w| w.toplevel().is_some_and(|t| *t.wl_surface() == root));
        if pending {
            self.map_pending_windows();
        }

        // A new client buffer means new pixels: request a render pass
        // (FR-2: damage-driven rendering starts at commit granularity).
        self.needs_redraw = true;
    }
}

impl ShmHandler for DfState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl DmabufHandler for DfState {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self
            .dmabuf_state
            .as_mut()
            .expect("the backend creates the dmabuf global before clients can bind it")
            .0
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) {
        use smithay::backend::allocator::Buffer as _;
        // GPU imports happen lazily inside the renderers (which own the
        // EGL context); the protocol layer accepts buffers whose format
        // was advertised and defers the actual import to render time. A
        // buffer the GPU turns out to be unable to import fails the
        // render pass, not the compositor.
        let format = dmabuf.format();
        if self.dmabuf_formats.contains(&format) {
            let _ = notifier.successful::<DfState>();
        } else {
            notifier.failed();
        }
    }
}

impl OutputHandler for DfState {}

impl XdgShellHandler for DfState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Do not send the initial configure here — send it from the first
        // commit once the placement policy can size the window.
        let window = Window::new_wayland_window(surface.clone());
        self.pending_windows.push(window);
        if let Some(geo) = self
            .space
            .outputs()
            .next()
            .and_then(|o| self.space.output_geometry(o))
        {
            surface.with_pending_state(|state| {
                state.bounds = Some(geo.size);
            });
            surface.send_configure();
        }
        self.needs_redraw = true;
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        if let Err(err) = self.popups.track_popup(PopupKind::from(surface.clone())) {
            eprintln!("dragonfruit-compositor: failed to track popup: {err}");
        }
        // Position against the parent and constrain to the output (FR-11).
        self.set_popup_geometry(&surface, positioner);
        let _ = surface.send_configure();
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        // Popups are sanctioned xdg-shell grabs: install the standard
        // keyboard/pointer grab so click-away and Escape dismiss them
        // (FR-11). Raw client key grabs remain impossible — the shortcut
        // engine and `GrabArbiter` intercept those (T-03 FR-5).
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };
        let popup = PopupKind::from(surface);
        let Ok(root) = find_popup_root_surface(&popup) else {
            return;
        };
        let Ok(grab) = self.popups.grab_popup(root, popup, &seat, serial) else {
            return;
        };
        if let Some(keyboard) = seat.get_keyboard() {
            keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
        }
        if let Some(pointer) = seat.get_pointer() {
            pointer.set_grab(
                self,
                PopupPointerGrab::new(&grab),
                serial,
                PointerFocus::Keep,
            );
        }
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        self.set_popup_geometry(&surface, positioner);
        surface.send_repositioned(token);
        let _ = surface.send_configure();
    }

    fn move_request(&mut self, surface: ToplevelSurface, _seat: wl_seat::WlSeat, serial: Serial) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(start_data) = pointer.grab_start_data() else {
            return;
        };
        let Some(initial_location) = self.space.element_location(&window) else {
            return;
        };
        pointer.set_grab(
            self,
            MoveGrab::new(start_data, window, initial_location),
            serial,
            PointerFocus::Clear,
        );
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        serial: Serial,
        edges: xdg_toplevel::ResizeEdge,
    ) {
        let Some(edge) = crate::window::ResizeEdge::from_xdg(edges) else {
            return;
        };
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(start_data) = pointer.grab_start_data() else {
            return;
        };
        let Some(initial_geometry) = self.space.element_geometry(&window) else {
            return;
        };
        pointer.set_grab(
            self,
            ResizeGrab::new(start_data, window, edge, initial_geometry),
            serial,
            PointerFocus::Clear,
        );
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        // Maximize maps to Zoom; there is no distinct maximize state (FR-2).
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        self.zoom_window(&window);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        self.unzoom_window(&window);
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        _output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        self.fullscreen_window(&window);
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        self.unfullscreen_window(&window);
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        self.minimize_window(&window);
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        let app_id = toplevel_app_id(&surface);
        if self.windows.set_app_id(&window, app_id) {
            self.broadcast_window(&window, WindowEventKind::AppIdChanged);
        }
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        let title = toplevel_title(&surface);
        if self.windows.set_title(&window, title) {
            self.broadcast_window(&window, WindowEventKind::TitleChanged);
        }
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        // Drop an unmapped toplevel straight out of the pending list.
        self.pending_windows.retain(|window| {
            window
                .toplevel()
                .is_some_and(|t| t.wl_surface() != surface.wl_surface())
        });

        let Some(window) = self.window_for_surface(surface.wl_surface()) else {
            return;
        };
        // Transient dialogs close with their parent (FR-6).
        for child in self.windows.children(&window) {
            if let Some(toplevel) = child.toplevel() {
                toplevel.send_close();
            }
        }
        // A destroyed fullscreen window takes its dedicated Space with it
        // (T-05 FR-3), so the strip does not keep a phantom Space.
        let was_fullscreen =
            self.windows.state(&window) == Some(crate::window::WindowState::Fullscreen);
        let id = self.windows.remove(&window);
        if self.active_window.as_ref() == Some(&window) {
            self.active_window = None;
        }
        if let Some(id) = id {
            if was_fullscreen {
                self.workspaces.exit_fullscreen(id);
            }
            self.workspaces.forget_window(id);
            self.window_dispatch.push(ShellWindowEvent {
                kind: WindowEventKind::Unmapped,
                id,
                app_id: None,
                title: None,
            });
        }
        self.space.unmap_elem(&window);
        self.needs_redraw = true;
    }
}

impl XdgDecorationHandler for DfState {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        // Dragonfruit ships SSD (T-13); the negotiated default is
        // server-side decorations.
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(DecorationMode::ServerSide);
        });
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: DecorationMode) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(mode);
        });
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(DecorationMode::ServerSide);
        });
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
    }
}

impl SelectionHandler for DfState {
    type SelectionUserData = ();
}

impl DataDeviceHandler for DfState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for DfState {}
impl ServerDndGrabHandler for DfState {
    fn send(
        &mut self,
        _mime_type: String,
        _fd: std::os::unix::io::OwnedFd,
        _seat: smithay::input::Seat<DfState>,
    ) {
    }
}

impl DataControlHandler for DfState {
    fn data_control_state(&self) -> &DataControlState {
        &self.data_control_state
    }
}

impl SeatHandler for DfState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut smithay::input::SeatState<DfState> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, seat: &smithay::input::Seat<DfState>, focused: Option<&WlSurface>) {
        let focus = focused.and_then(|surface| self.display_handle.get_client(surface.id()).ok());
        set_data_device_focus(&self.display_handle, seat, focus);

        // Map the focused surface back to its window (popups resolve to
        // their root toplevel) and broadcast focus transitions (FR-4).
        let new_active = focused.and_then(|surface| self.window_for_surface(surface));
        if new_active != self.active_window {
            if let Some(old) = self.active_window.clone() {
                // The old window lost focus: dismiss its popups (FR-11)
                // and tell the shell.
                self.dismiss_popups_for(&old);
                self.broadcast_window(&old, WindowEventKind::Unfocused);
            }
            if let Some(new) = &new_active {
                self.broadcast_window(new, WindowEventKind::Focused);
                // Admit this app's accelerators for the focused window
                // (T-22 feeds the registrations; the app id scopes them).
                self.shortcuts
                    .set_focused_app(self.windows.app_id(new).map(str::to_string));
            } else {
                self.shortcuts.set_focused_app(None);
            }
            self.active_window = new_active;
        }
    }

    fn cursor_image(
        &mut self,
        _seat: &smithay::input::Seat<DfState>,
        image: smithay::input::pointer::CursorImageStatus,
    ) {
        self.cursor_image = image;
    }
}

impl TabletSeatHandler for DfState {
    fn tablet_tool_image(
        &mut self,
        _tool: &smithay::backend::input::TabletToolDescriptor,
        image: smithay::input::pointer::CursorImageStatus,
    ) {
        self.cursor_image = image;
    }
}

impl FractionalScaleHandler for DfState {}

impl PointerConstraintsHandler for DfState {
    fn new_constraint(
        &mut self,
        _surface: &WlSurface,
        _pointer: &smithay::input::pointer::PointerHandle<DfState>,
    ) {
        // TODO(T-04/T-05): confine/lock must be enforced with a pointer
        // grab; T-03 scoped out pointer-constraint grabs. See PROGRESS.md.
    }

    fn cursor_position_hint(
        &mut self,
        _surface: &WlSurface,
        _pointer: &smithay::input::pointer::PointerHandle<DfState>,
        _location: Point<f64, smithay::utils::Logical>,
    ) {
    }
}

impl InputMethodHandler for DfState {
    fn new_popup(&mut self, surface: smithay::wayland::input_method::PopupSurface) {
        if let Err(err) = self.popups.track_popup(PopupKind::from(surface)) {
            eprintln!("dragonfruit-compositor: failed to track IM popup: {err}");
        }
    }

    fn popup_repositioned(&mut self, _surface: smithay::wayland::input_method::PopupSurface) {
        // Input-method popup placement is part of the IM work in T-03.
    }

    fn dismiss_popup(&mut self, surface: smithay::wayland::input_method::PopupSurface) {
        if let Some(parent) = surface.get_parent().map(|parent| parent.surface.clone()) {
            let _ = PopupManager::dismiss_popup(&parent, &PopupKind::from(surface));
        }
    }

    fn parent_geometry(
        &self,
        parent: &WlSurface,
    ) -> smithay::utils::Rectangle<i32, smithay::utils::Logical> {
        self.space
            .elements()
            .find_map(|window| {
                (window.wl_surface().as_deref() == Some(parent)).then(|| window.geometry())
            })
            .unwrap_or_default()
    }
}

impl XdgActivationHandler for DfState {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn request_activation(
        &mut self,
        _token: XdgActivationToken,
        _token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        // Launch feedback / Dock bounce (T-12): activation requests focus
        // the surface for now; the shell-facing signaling lands with the
        // private protocols in T-07.
        let root = {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            root
        };
        if let Some(window) = self
            .space
            .elements()
            .find(|w| {
                w.wl_surface()
                    .as_deref()
                    .map(|s| *s == root)
                    .unwrap_or(false)
            })
            .cloned()
        {
            window.set_activated(true);
            self.needs_redraw = true;
        }
    }
}

impl SecurityContextHandler for DfState {
    fn context_created(&mut self, source: SecurityContextListenerSource, context: SecurityContext) {
        // Sandboxed clients connect through the security-context socket;
        // serve it on the event loop and tag its clients (T-07/T-27 build
        // access control on this).
        let result = self
            .loop_handle
            .insert_source(source, move |stream, _, state| {
                let mut dh = state.display_handle.clone();
                let client_state = std::sync::Arc::new(DfClientState {
                    compositor_state: CompositorClientState::default(),
                    security_context: Some(context.clone()),
                });
                if let Err(err) = dh.insert_client(stream, client_state) {
                    eprintln!("dragonfruit-compositor: rejecting sandboxed client: {err}");
                }
            });
        if let Err(err) = result {
            eprintln!("dragonfruit-compositor: failed to serve security-context socket: {err}");
        }
    }
}

impl IdleInhibitHandler for DfState {
    fn inhibit(&mut self, surface: WlSurface) {
        self.idle_inhibitors.push(surface);
        let inhibited = self.idle_inhibitors.iter().any(|s| s.is_alive());
        self.idle_notifier_state.set_is_inhibited(inhibited);
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        self.idle_inhibitors.retain(|s| s != &surface);
        let inhibited = self.idle_inhibitors.iter().any(|s| s.is_alive());
        self.idle_notifier_state.set_is_inhibited(inhibited);
    }
}

impl IdleNotifierHandler for DfState {
    fn idle_notifier_state(&mut self) -> &mut IdleNotifierState<DfState> {
        &mut self.idle_notifier_state
    }
}

impl SessionLockHandler for DfState {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.session_lock_state
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        // Fail-secure locking: the lock UI owns the session once confirmed.
        // The interactive lock screen arrives with T-26; until then we
        // confirm immediately so the fail-secure path is exercised.
        confirmation.lock();
    }

    fn new_surface(
        &mut self,
        _surface: smithay::wayland::session_lock::LockSurface,
        _output: smithay::reexports::wayland_server::protocol::wl_output::WlOutput,
    ) {
        // Lock-surface rendering is T-26; the surface is tracked by
        // smithay's session-lock state until then.
    }

    fn unlock(&mut self) {
        // T-26 owns the interactive unlock flow.
    }
}

// Delegate protocol dispatch to the state structs above.
delegate_compositor!(DfState);
delegate_seat!(DfState);
delegate_dmabuf!(DfState);
delegate_shm!(DfState);
delegate_xdg_shell!(DfState);
delegate_xdg_decoration!(DfState);
delegate_viewporter!(DfState);
delegate_fractional_scale!(DfState);
delegate_output!(DfState);
delegate_presentation!(DfState);
delegate_pointer_constraints!(DfState);
delegate_relative_pointer!(DfState);
delegate_pointer_gestures!(DfState);
delegate_cursor_shape!(DfState);
delegate_idle_inhibit!(DfState);
delegate_idle_notify!(DfState);
delegate_content_type!(DfState);
delegate_data_device!(DfState);
delegate_data_control!(DfState);
delegate_security_context!(DfState);
delegate_xdg_activation!(DfState);
delegate_session_lock!(DfState);
delegate_tablet_manager!(DfState);
delegate_text_input_manager!(DfState);
delegate_input_method_manager!(DfState);
delegate_xwayland_shell!(DfState);
