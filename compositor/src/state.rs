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
use smithay::desktop::{PopupKind, PopupManager, Space, Window};
use smithay::reexports::calloop::{LoopHandle, LoopSignal, RegistrationToken};
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::{wl_buffer, wl_seat, wl_surface::WlSurface};
use smithay::reexports::wayland_server::{Client, DisplayHandle, Resource as _};
use smithay::utils::{Clock, Logical, Monotonic, Point, Serial};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    get_parent, with_states, CompositorClientState, CompositorHandler, CompositorState,
    SurfaceAttributes,
};
use smithay::wayland::content_type::ContentTypeState;
use smithay::wayland::cursor_shape::CursorShapeManagerState;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier};
use smithay::input::SeatHandler;
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
    decoration::{XdgDecorationHandler, XdgDecorationState}, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
};
use smithay::wayland::seat::WaylandFocus as _;
use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::wayland::tablet_manager::{TabletManagerState, TabletSeatHandler};
use smithay::wayland::text_input::TextInputManagerState;
use smithay::wayland::viewporter::ViewporterState;
use smithay::wayland::xdg_activation::{
    XdgActivationToken, XdgActivationTokenData, XdgActivationHandler, XdgActivationState,
};
use std::time::Duration;

use crate::input::dispatch::InputDispatch;
use crate::input::gestures::{GestureRecognizer, ProgressPipeline};
use crate::input::hot_corners::HotCornerDetector;
use crate::input::settings::InputSettings;
use crate::input::shortcuts::{GrabArbiter, ShortcutEngine};
use crate::input::{InputAction, TriggerKind};

use smithay::{
    delegate_compositor, delegate_content_type, delegate_cursor_shape, delegate_data_control,
    delegate_data_device, delegate_dmabuf, delegate_fractional_scale, delegate_idle_inhibit,
    delegate_idle_notify, delegate_input_method_manager, delegate_output,
    delegate_pointer_constraints, delegate_pointer_gestures, delegate_presentation,
    delegate_relative_pointer, delegate_seat, delegate_security_context, delegate_session_lock,
    delegate_shm, delegate_tablet_manager, delegate_text_input_manager, delegate_viewporter,
    delegate_xdg_activation, delegate_xdg_decoration, delegate_xdg_shell,
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
    /// Cascading offset counter for the temporary T-02 placement policy.
    cascade: i32,

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
            seat_state,
            seat,
            space: Space::default(),
            popups: PopupManager::default(),
            pending_windows: Vec::new(),
            idle_inhibitors: Vec::new(),
            cursor_image: smithay::input::pointer::CursorImageStatus::default_named(),
            cascade: 0,
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

    /// Map pending windows into the scene once they have a buffer
    /// (temporary T-02 placement: center + cascade; T-04 owns the policy).
    pub fn map_pending_windows(&mut self) {
        let mut to_map = Vec::new();
        self.pending_windows.retain(|window| {
            let has_buffer = window.toplevel().is_some_and(|toplevel| {
                let surface = toplevel.wl_surface();
                with_states(surface, |states| {
                    states
                        .cached_state
                        .get::<SurfaceAttributes>()
                        .current()
                        .buffer
                        .is_some()
                })
            });
            if has_buffer {
                to_map.push(window.clone());
                false
            } else {
                true
            }
        });

        let output_geo = self
            .space
            .outputs()
            .next()
            .and_then(|o| self.space.output_geometry(o));
        let cascade = self.cascade;
        let mut mapped = 0;
        for (i, window) in to_map.into_iter().enumerate() {
            let size = window.bbox().size;
            let step = 24 * ((cascade + i as i32) % 8);
            let loc = output_geo
                .map(|geo| {
                    Point::<i32, Logical>::from((
                        (geo.size.w - size.w) / 2 + step,
                        (geo.size.h - size.h) / 2 + step,
                    ))
                })
                .unwrap_or_default();
            mapped = i as i32 + 1;
            self.space.map_element(window, loc, true);
        }
        self.cascade = cascade + mapped;
        if mapped > 0 {
            self.needs_redraw = true;
        }
    }

    /// Update idle notification activity state after seat/input activity.
    pub fn notify_activity(&mut self) {
        self.idle_notifier_state.notify_activity(&self.seat);
    }

    /// A monotonic timestamp in milliseconds for the input pipelines.
    pub fn now_msec(&self) -> u64 {
        Duration::from(self.clock.now()).as_millis() as u64
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

impl BufferHandler for DfState {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl CompositorHandler for DfState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<DfClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        // Import wl_shm buffers (glow/EGL backends); headless has no
        // renderer and simply ignores buffers.
        smithay::backend::renderer::utils::on_commit_buffer_handler::<DfState>(surface);

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

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        if let Err(err) = self.popups.track_popup(PopupKind::from(surface)) {
            eprintln!("dragonfruit-compositor: failed to track popup: {err}");
        }
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // Popup keyboard grabs are a privilege of sanctioned session
        // clients. No grab is installed until a trusted client (shell /
        // menu popups) is provisioned with a launch token in T-07; the
        // `GrabArbiter` is the gate every grab request must pass (T-03
        // FR-5). Clients cannot install a raw key grab at all — the
        // shortcut engine intercepts every binding first.
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
        let _ = surface.send_configure();
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        // Zoom is a distinct window state (macOS-style) — policy lands in
        // T-04; acknowledge with a full-output configure for now.
        let Some(geometry) = self
            .space
            .outputs()
            .next()
            .and_then(|o| self.space.output_geometry(o))
        else {
            return;
        };
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Maximized);
            state.size = Some(geometry.size);
        });
        surface.send_configure();
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Maximized);
            state.size = None;
        });
        surface.send_configure();
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        _output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        let Some(geometry) = self
            .space
            .outputs()
            .next()
            .and_then(|o| self.space.output_geometry(o))
        else {
            return;
        };
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Fullscreen);
            state.size = Some(geometry.size);
        });
        surface.send_configure();
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Fullscreen);
            state.size = None;
        });
        surface.send_configure();
    }

    fn minimize_request(&mut self, _surface: ToplevelSurface) {
        // Minimize/restore is shell policy (T-04/T-10); ignore for now.
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
