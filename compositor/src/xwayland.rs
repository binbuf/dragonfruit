// SPDX-License-Identifier: MIT
//! Xwayland integration (T-06): lifecycle, X11 window management, identity,
//! clipboard bridging, and Tier-2 decoration marking.
//!
//! Xwayland is spawned **eagerly** at session start. The ticket allows lazy
//! or eager; eager is chosen because true lazy start requires the launcher
//! to know an application's toolkit *before* it runs, which is the
//! `app-index` service's job (T-23). Eager start makes FR-1 ("Xwayland
//! starts automatically on first X11 client launch") true for every launch
//! path today: the server is up and `DISPLAY` is exported before any client
//! can ask for it. A missing `Xwayland` binary is not fatal — the
//! compositor logs and continues (Wayland-only sessions still work).
//!
//! X11 windows are ordinary [`Window`]s from Smithay's point of view; this
//! module maps them through the exact same [`WindowModel`], `Space`, and
//! workspace machinery as `xdg_toplevel`s (T-04/T-05). X11 clients cannot
//! draw Wayland CSD, so they are marked [`DecorationTier::ServerSide`]
//! (Tier 2) unless `_MOTIF_WM_HINTS` explicitly opts out; T-13 renders the
//! actual titlebar.
//!
//! Clipboard (not primary) selection is bridged both directions through the
//! data-device selection the shell's `wlr-data-control` manager observes.
//! XDnD is *not* implemented by Smithay 0.7's XWM, so drag-and-drop across
//! the boundary is a known gap (see PROGRESS.md) until a bridge lands.

use std::path::PathBuf;
use std::process::Stdio;

use smithay::desktop::Window;
use smithay::input::pointer::Focus as PointerFocus;
use smithay::reexports::calloop::RegistrationToken;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Rectangle, SERIAL_COUNTER};
use smithay::wayland::seat::WaylandFocus as _;
use smithay::wayland::selection::data_device::{
    clear_data_device_selection, current_data_device_selection_userdata,
    request_data_device_client_selection, set_data_device_selection,
};
use smithay::wayland::selection::SelectionTarget;
use smithay::wayland::xwayland_shell::{XWaylandShellHandler, XWaylandShellState};
use smithay::xwayland::xwm::{
    Reorder, ResizeEdge as X11ResizeEdge, WmWindowProperty, X11Window, XwmId,
};
use smithay::xwayland::{X11Surface, X11Wm, XWayland, XWaylandEvent, XwmHandler};

use crate::state::DfState;
use crate::window::grab::{MoveGrab, ResizeGrab};
use crate::window::{
    cascaded_geometry, centered_on, DecorationTier, ResizeEdge, WindowEvent, WindowEventKind,
    WindowState, CASCADE_STEP,
};

/// The Xwayland server, X11 window manager, and `DISPLAY` hand-off state.
#[derive(Debug, Default)]
pub struct XwaylandState {
    /// The X11 window manager, once Xwayland is ready.
    pub wm: Option<X11Wm>,
    /// `DISPLAY` value exported to launched apps, e.g. `:0`.
    pub display: Option<String>,
    /// The raw display number.
    pub display_number: Option<u32>,
    /// Whether the server is currently up.
    pub running: bool,
    /// Set when the server died and should be respawned (FR-6).
    pub pending_restart: bool,
    /// The calloop token for the `XWayland` event source (removed on restart).
    pub source_token: Option<RegistrationToken>,
    /// How many times the server has been started (audit/diagnostics).
    pub start_count: u32,
    /// Override-redirect windows (menus, tooltips, DnD icons): mapped into
    /// the scene but never registered as user windows.
    pub override_windows: Vec<(X11Window, Window)>,
}

/// Path of the per-session `DISPLAY` hand-off file. The dev tool reads it
/// to export `DISPLAY` to `--launch`ed children; the file is removed on
/// teardown.
pub fn display_file(socket_name: &str) -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join(format!("{socket_name}.x11-display")))
}

fn write_display_file(socket_name: &str, display: &str) {
    let Some(path) = display_file(socket_name) else {
        return;
    };
    if let Err(err) = std::fs::write(&path, format!("{display}\n")) {
        eprintln!(
            "dragonfruit-compositor: failed to write {}: {err}",
            path.display()
        );
    }
}

/// Remove the `DISPLAY` hand-off file (clean teardown).
pub fn remove_display_file(socket_name: &str) {
    if let Some(path) = display_file(socket_name) {
        let _ = std::fs::remove_file(path);
    }
}

/// Remove the X11 socket and lock files for `display_number`.
///
/// `XWayland::spawn` owns these through its `X11Lock`, but Smithay 0.7's
/// `X11Wm` source closure forms a calloop `Rc` cycle, so the `XWayland`
/// source (and its lock) is never dropped at teardown (see the note in
/// `session.rs`). Unlink them explicitly so repeated sessions do not
/// accumulate stale `/tmp/.X<n>-lock` files.
pub fn cleanup_x11_files(display_number: u32) {
    let _ = std::fs::remove_file(format!("/tmp/.X{display_number}-lock"));
    let _ = std::fs::remove_file(format!("/tmp/.X11-unix/X{display_number}"));
}

/// Strip NULs from an X11 property string. `WM_NAME`/`WM_CLASS` are
/// conventionally NUL-terminated and Smithay does not trim them; an
/// interior NUL would otherwise reach the private protocol's `CString`
/// serialization (and panic it).
fn x11_string(value: &str) -> String {
    value.replace('\0', "")
}

/// Spawn Xwayland and register its event source. Idempotent; returns
/// `Ok(())` even when the binary is missing (Wayland-only sessions are
/// still valid).
pub fn start(state: &mut DfState) -> Result<(), String> {
    if state.xwayland.wm.is_some() || state.xwayland.pending_restart {
        return Ok(());
    }

    let (xwayland, client) = match XWayland::spawn(
        &state.display_handle,
        None,
        std::iter::empty::<(String, String)>(),
        true,
        Stdio::null(),
        Stdio::null(),
        |_| (),
    ) {
        Ok(pair) => pair,
        Err(err) => {
            // A Wayland-only session is still a valid session; do not
            // abort. The X11 app zoo (T-30) reports this to the user.
            eprintln!(
                "dragonfruit-compositor: Xwayland unavailable ({err}); continuing Wayland-only"
            );
            return Ok(());
        }
    };

    state.xwayland.start_count += 1;
    let socket_name = state.socket_name.clone();

    let token = state
        .loop_handle
        .insert_source(xwayland, move |event, _, state| match event {
            XWaylandEvent::Ready {
                x11_socket,
                display_number,
            } => {
                // Use the state's handle rather than capturing one: a
                // captured LoopHandle in a source closure forms an Rc cycle
                // with calloop's LoopInner and keeps the loop (and its
                // socket) alive past teardown.
                let loop_handle = state.loop_handle.clone();
                match X11Wm::start_wm(loop_handle, x11_socket, client.clone()) {
                    Ok(wm) => {
                        let display = format!(":{display_number}");
                        // Export to this process's environment so any child
                        // we launch inherits it, and hand it to the dev tool
                        // through the runtime file.
                        std::env::set_var("DISPLAY", &display);
                        write_display_file(&socket_name, &display);
                        state.xwayland.wm = Some(wm);
                        state.xwayland.display_number = Some(display_number);
                        state.xwayland.display = Some(display.clone());
                        state.xwayland.running = true;
                        state.xwayland.pending_restart = false;
                        println!(
                            "dragonfruit-compositor: Xwayland ready: DISPLAY={display} \
                             (start #{})",
                            state.xwayland.start_count
                        );
                    }
                    Err(err) => {
                        eprintln!("dragonfruit-compositor: failed to start X11 WM: {err}");
                        state.xwayland.running = false;
                    }
                }
            }
            XWaylandEvent::Error => {
                eprintln!("dragonfruit-compositor: Xwayland exited; scheduling restart");
                state.xwayland.wm = None;
                state.xwayland.running = false;
                state.xwayland.display = None;
                // Keep `display_number` so `maybe_restart` can clean up the
                // dead server's socket/lock before respawning.
                state.xwayland.pending_restart = true;
            }
        })
        .map_err(|err| format!("failed to register Xwayland source: {err}"))?;

    state.xwayland.source_token = Some(token);
    Ok(())
}

/// Remove a dead Xwayland source and spawn a fresh instance (FR-6). Called
/// from the session loop after dispatch so the borrow of the event source
/// has ended.
pub fn maybe_restart(state: &mut DfState) {
    if !state.xwayland.pending_restart {
        return;
    }
    if let Some(token) = state.xwayland.source_token.take() {
        state.loop_handle.remove(token);
    }
    state.xwayland.wm = None;
    state.xwayland.running = false;
    state.xwayland.pending_restart = false;
    if let Some(number) = state.xwayland.display_number {
        cleanup_x11_files(number);
    }
    remove_display_file(&state.socket_name);
    if let Err(err) = start(state) {
        eprintln!("dragonfruit-compositor: Xwayland restart failed: {err}");
    }
}

// --- X11 window management --------------------------------------------------

impl DfState {
    /// The registered window for an X11 surface (identity is the X11 window
    /// id; `X11Surface` itself is not `PartialEq`).
    pub(crate) fn x11_window_for(&self, surface: &X11Surface) -> Option<Window> {
        let id = surface.window_id();
        self.windows
            .windows()
            .find(|window| window.x11_surface().map(|s| s.window_id()) == Some(id))
            .cloned()
    }

    /// The registered window whose X11 window id is `id` (transient parent).
    fn window_for_x11_id(&self, id: X11Window) -> Option<Window> {
        self.windows
            .windows()
            .find(|window| window.x11_surface().map(|s| s.window_id()) == Some(id))
            .cloned()
    }

    /// Resolve an X11 `WM_CLASS` to an application id. Falls back to the raw
    /// class so Dock/switcher grouping works even without a desktop entry;
    /// every miss is recorded for the T-23 heuristics.
    fn resolve_x11_identity(&mut self, instance: &str, class: &str) -> Option<String> {
        let instance = x11_string(instance);
        let class = x11_string(class);
        if let Some(identity) = self.app_resolver.resolve_wm_class(&instance, &class) {
            return Some(identity.desktop_id);
        }
        let fallback = if !class.is_empty() { class } else { instance };
        (!fallback.is_empty()).then_some(fallback)
    }

    /// Tier-2 (SSD) unless the client explicitly asks to be undecorated.
    fn x11_decoration_tier(surface: &X11Surface) -> DecorationTier {
        if surface.is_decorated() {
            DecorationTier::ClientSide
        } else {
            DecorationTier::ServerSide
        }
    }

    /// Give keyboard focus to `window` once its `wl_surface` is associated.
    fn focus_window(&mut self, window: &Window) {
        let Some(surface) = window.wl_surface().map(|s| s.into_owned()) else {
            return;
        };
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.set_focus(self, Some(surface), SERIAL_COUNTER.next_serial());
        }
    }

    /// Map an X11 window through the same window model as an xdg toplevel.
    fn map_x11_window(&mut self, surface: &X11Surface) -> Option<Window> {
        let (window, newly_mapped) = match self.x11_window_for(surface) {
            Some(window) => {
                if let Some(transition) =
                    self.windows
                        .apply(&window, WindowEvent::Restore, Rectangle::default())
                {
                    if transition.changed {
                        let geometry = self.windows.geometry(&window).unwrap_or_default();
                        let _ = surface.configure(Some(geometry));
                        self.broadcast_state(&window);
                    }
                }
                (window, false)
            }
            None => {
                let window = Window::new_x11_window(surface.clone());
                let size = surface.geometry().size;
                let parent = surface
                    .is_transient_for()
                    .and_then(|id| self.window_for_x11_id(id));
                let geometry = if let Some(parent) = &parent {
                    let parent_geometry = self.windows.geometry(parent).unwrap_or_default();
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
                let app_id = self.resolve_x11_identity(&surface.instance(), &surface.class());
                self.windows.set_app_id(&window, app_id);
                self.windows
                    .set_title(&window, Some(x11_string(&surface.title())));
                self.windows
                    .set_decorations(&window, Self::x11_decoration_tier(surface));
                if let Some(parent) = parent {
                    self.windows.set_parent(&window, &parent);
                }
                self.assign_new_window_space(&window, id);
                (window, true)
            }
        };

        if self.window_on_active_space(&window) {
            let geometry = self.windows.geometry(&window).unwrap_or_default();
            let _ = surface.configure(Some(geometry));
            self.space.map_element(window.clone(), geometry.loc, true);
        }
        if newly_mapped {
            self.broadcast_window(&window, WindowEventKind::Mapped);
        }
        self.focus_window(&window);
        self.needs_redraw = true;
        Some(window)
    }

    /// Handle an X11 window leaving the screen (iconify/withdraw).
    fn unmap_x11_window(&mut self, surface: &X11Surface) {
        // Override-redirect windows are tracked separately.
        if let Some(pos) = self
            .xwayland
            .override_windows
            .iter()
            .position(|(id, _)| *id == surface.window_id())
        {
            let (_, window) = self.xwayland.override_windows.remove(pos);
            self.space.unmap_elem(&window);
            self.needs_redraw = true;
            return;
        }

        let Some(window) = self.x11_window_for(surface) else {
            return;
        };
        self.space.unmap_elem(&window);
        if !surface.is_override_redirect() {
            let _ = surface.set_mapped(false);
        }
        if self
            .windows
            .state(&window)
            .is_some_and(|state| state != WindowState::Minimized)
        {
            if let Some(transition) =
                self.windows
                    .apply(&window, WindowEvent::Minimize, Rectangle::default())
            {
                if transition.changed {
                    self.broadcast_state(&window);
                }
            }
        }
        self.needs_redraw = true;
    }

    /// Handle an X11 window being destroyed.
    fn destroy_x11_window(&mut self, surface: &X11Surface) {
        if let Some(pos) = self
            .xwayland
            .override_windows
            .iter()
            .position(|(id, _)| *id == surface.window_id())
        {
            let (_, window) = self.xwayland.override_windows.remove(pos);
            self.space.unmap_elem(&window);
            self.needs_redraw = true;
            return;
        }

        let Some(window) = self.x11_window_for(surface) else {
            return;
        };
        self.dismiss_popups_for(&window);
        let was_fullscreen = self.windows.state(&window) == Some(WindowState::Fullscreen);
        let id = self.windows.remove(&window);
        if self.active_window.as_ref() == Some(&window) {
            self.active_window = None;
        }
        if let Some(id) = id {
            if was_fullscreen {
                self.workspaces.exit_fullscreen(id);
            }
            self.workspaces.forget_window(id);
            self.window_dispatch.push(crate::window::ShellWindowEvent {
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

impl XwmHandler for DfState {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwayland
            .wm
            .as_mut()
            .expect("X11 WM callback without a live X11Wm")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Err(err) = surface.set_mapped(true) {
            eprintln!("dragonfruit-compositor: failed to map X11 window: {err}");
            return;
        }
        self.map_x11_window(&surface);
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let location = surface.geometry().loc;
        let window = Window::new_x11_window(surface.clone());
        self.space.map_element(window.clone(), location, true);
        self.xwayland
            .override_windows
            .push((surface.window_id(), window));
        self.needs_redraw = true;
    }

    fn unmapped_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        self.unmap_x11_window(&surface);
    }

    fn destroyed_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        self.destroy_x11_window(&surface);
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        surface: X11Surface,
        _x: Option<i32>,
        _y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        // The compositor owns placement; only the size is granted.
        let mut geometry = self
            .x11_window_for(&surface)
            .and_then(|window| self.windows.geometry(&window))
            .unwrap_or_else(|| surface.geometry());
        if let Some(w) = w {
            geometry.size.w = w.max(1) as i32;
        }
        if let Some(h) = h {
            geometry.size.h = h.max(1) as i32;
        }
        if let Some(window) = self.x11_window_for(&surface) {
            self.windows.set_floating_geometry(&window, geometry);
            self.space.map_element(window.clone(), geometry.loc, false);
            self.configure_window_size(&window, geometry.size);
        } else {
            let _ = surface.configure(geometry);
        }
        self.needs_redraw = true;
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        surface: X11Surface,
        geometry: Rectangle<i32, Logical>,
        _above: Option<X11Window>,
    ) {
        // Override-redirect windows position themselves; keep the scene in
        // sync. Managed windows are compositor-positioned, so their
        // ConfigureNotify is just the echo of our own configure.
        if !surface.is_override_redirect() {
            return;
        }
        if let Some(pos) = self
            .xwayland
            .override_windows
            .iter()
            .position(|(id, _)| *id == surface.window_id())
        {
            let window = self.xwayland.override_windows[pos].1.clone();
            self.space.map_element(window, geometry.loc, false);
            self.needs_redraw = true;
        }
    }

    fn property_notify(&mut self, _xwm: XwmId, surface: X11Surface, property: WmWindowProperty) {
        let Some(window) = self.x11_window_for(&surface) else {
            return;
        };
        match property {
            WmWindowProperty::Title => {
                if self
                    .windows
                    .set_title(&window, Some(x11_string(&surface.title())))
                {
                    self.broadcast_window(&window, WindowEventKind::TitleChanged);
                }
            }
            WmWindowProperty::Class => {
                let app_id = self.resolve_x11_identity(&surface.instance(), &surface.class());
                if self.windows.set_app_id(&window, app_id) {
                    self.broadcast_window(&window, WindowEventKind::AppIdChanged);
                }
            }
            WmWindowProperty::MotifHints => {
                let tier = Self::x11_decoration_tier(&surface);
                self.windows.set_decorations(&window, tier);
                self.needs_redraw = true;
            }
            WmWindowProperty::TransientFor => {
                if let Some(parent) = surface
                    .is_transient_for()
                    .and_then(|id| self.window_for_x11_id(id))
                {
                    self.windows.set_parent(&window, &parent);
                }
            }
            _ => {}
        }
    }

    fn maximize_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(window) = self.x11_window_for(&surface) {
            let _ = surface.set_maximized(true);
            self.zoom_window(&window);
        }
    }

    fn unmaximize_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(window) = self.x11_window_for(&surface) {
            let _ = surface.set_maximized(false);
            self.unzoom_window(&window);
        }
    }

    fn fullscreen_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(window) = self.x11_window_for(&surface) {
            let _ = surface.set_fullscreen(true);
            self.fullscreen_window(&window);
        }
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(window) = self.x11_window_for(&surface) {
            let _ = surface.set_fullscreen(false);
            self.unfullscreen_window(&window);
        }
    }

    fn minimize_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(window) = self.x11_window_for(&surface) {
            let _ = surface.set_suspended(true);
            self.minimize_window(&window);
        }
    }

    fn unminimize_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(window) = self.x11_window_for(&surface) {
            let _ = surface.set_suspended(false);
            self.restore_window(&window);
        }
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        surface: X11Surface,
        _button: u32,
        edges: X11ResizeEdge,
    ) {
        let Some(window) = self.x11_window_for(&surface) else {
            return;
        };
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(start_data) = pointer.grab_start_data() else {
            return;
        };
        let Some(edge) = ResizeEdge::from_x11(edges) else {
            return;
        };
        let Some(initial_geometry) = self.space.element_geometry(&window) else {
            return;
        };
        pointer.set_grab(
            self,
            ResizeGrab::new(start_data, window, edge, initial_geometry),
            SERIAL_COUNTER.next_serial(),
            PointerFocus::Clear,
        );
    }

    fn move_request(&mut self, _xwm: XwmId, surface: X11Surface, _button: u32) {
        let Some(window) = self.x11_window_for(&surface) else {
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
            SERIAL_COUNTER.next_serial(),
            PointerFocus::Clear,
        );
    }

    fn allow_selection_access(&mut self, _xwm: XwmId, _selection: SelectionTarget) -> bool {
        // Only while an X11 window holds keyboard focus, so a background
        // X client cannot read the Wayland clipboard.
        let Some(keyboard) = self.seat.get_keyboard() else {
            return false;
        };
        keyboard
            .current_focus()
            .and_then(|surface| self.window_for_surface(&surface))
            .map(|window| window.x11_surface().is_some())
            .unwrap_or(false)
    }

    fn send_selection(
        &mut self,
        _xwm: XwmId,
        selection: SelectionTarget,
        mime_type: String,
        fd: std::os::unix::io::OwnedFd,
    ) {
        if !matches!(selection, SelectionTarget::Clipboard) {
            // Primary selection is not advertised; the fd is dropped.
            return;
        }
        if let Err(err) = request_data_device_client_selection(&self.seat, mime_type, fd) {
            eprintln!("dragonfruit-compositor: X11 clipboard read failed: {err:?}");
        }
    }

    fn new_selection(&mut self, _xwm: XwmId, selection: SelectionTarget, mime_types: Vec<String>) {
        if matches!(selection, SelectionTarget::Clipboard) {
            set_data_device_selection(&self.display_handle, &self.seat, mime_types, ());
        }
    }

    fn cleared_selection(&mut self, _xwm: XwmId, selection: SelectionTarget) {
        if !matches!(selection, SelectionTarget::Clipboard) {
            return;
        }
        if current_data_device_selection_userdata(&self.seat).is_some() {
            clear_data_device_selection(&self.display_handle, &self.seat);
        }
    }

    fn disconnected(&mut self, _xwm: XwmId) {
        eprintln!("dragonfruit-compositor: X11 WM disconnected; scheduling restart");
        self.xwayland.wm = None;
        self.xwayland.running = false;
        self.xwayland.pending_restart = true;
    }
}

impl XWaylandShellHandler for DfState {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.xwayland_shell_state
    }

    fn surface_associated(&mut self, _xwm: XwmId, _wl_surface: WlSurface, surface: X11Surface) {
        // The wl_surface now exists: refresh the window's committed state
        // and give it focus if nothing else owns it. Rendering is triggered
        // by the following commit.
        if let Some(window) = self.x11_window_for(&surface) {
            window.on_commit();
            if self.active_window.is_none() {
                self.focus_window(&window);
            }
        }
        self.needs_redraw = true;
    }
}
