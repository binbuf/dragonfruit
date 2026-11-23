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
//! XDnD is *not* implemented by Smithay 0.7's XWM. The compositor's own
//! `xdnd_bridge` module covers the X11→Wayland target role (T328a); the
//! Wayland→X11 source role remains T328b.

use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use dragonfruit_compositor::xdnd::{XdndAction, URI_LIST_MIME};
use dragonfruit_compositor::xdnd_bridge::{
    BridgeEvent, XdndBridge, MAX_PAYLOAD_BYTES, TRANSFER_TIMEOUT,
};
use smithay::backend::input::ButtonState;
use smithay::desktop::Window;
use smithay::input::pointer::Focus as PointerFocus;
use smithay::input::pointer::{ButtonEvent, GrabStartData, MotionEvent};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::{Interest, Mode, PostAction, RegistrationToken};
use smithay::reexports::wayland_server::protocol::wl_data_device_manager::DndAction;
use smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::x11rb::connection::Connection as _;
use smithay::reexports::x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as _, PropMode};
use smithay::reexports::x11rb::rust_connection::RustConnection;
use smithay::reexports::x11rb::wrapper::ConnectionExt as _;
use smithay::utils::{Logical, Point, Rectangle, SERIAL_COUNTER};
use smithay::wayland::seat::WaylandFocus as _;
use smithay::wayland::selection::data_device::{
    clear_data_device_selection, current_data_device_selection_userdata,
    request_data_device_client_selection, set_data_device_selection, start_dnd,
    with_source_metadata, SourceMetadata,
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
    cascaded_geometry, centered_on, user_positioned_geometry, DecorationTier, ResizeEdge,
    WindowChrome, WindowEvent, WindowEventKind, WindowInsets, WindowState, CASCADE_STEP,
    DEFAULT_WINDOW_STYLE,
};

/// The Xwayland server, X11 window manager, and `DISPLAY` hand-off state.
#[derive(Debug)]
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
    /// The integer scale Xwayland renders X11 clients at, chosen once per
    /// session from the primary output: `ceil(output_scale)`, clamped to at
    /// least 1 (`docs/design/xwayland-scaling.md`, T-16.3a). It is the same value
    /// Smithay advertises as the output's `wl_output.scale`, so X11 clients
    /// always render into an integer coordinate space.
    pub integer_scale: i32,
    /// A private X11 connection for WM-owned window properties (T327).
    /// Smithay's `X11Wm` keeps its own connection private, so the compositor
    /// opens one of its own to publish `_NET_FRAME_EXTENTS` on managed
    /// windows. `None` until the first property write; reset when the server
    /// goes away.
    pub property_conn: Option<RustConnection>,
    /// The cached `_NET_FRAME_EXTENTS` atom on [`Self::property_conn`].
    pub frame_extents_atom: Option<Atom>,
    /// The dedicated X11 connection and bridge window for XDnD-as-target
    /// (T328a). `None` until Xwayland is ready or when the server goes away.
    pub dnd: Option<XdndBridge>,
    /// The calloop readiness source for [`Self::dnd`].
    pub dnd_source_token: Option<RegistrationToken>,
    /// Local paths read from the current X11 drop, served to the Wayland
    /// client through the compositor-owned data source.
    pub dnd_offer: Option<Vec<PathBuf>>,
    /// The Wayland client accepted the `text/uri-list` offer.
    pub dnd_offer_accepted: bool,
    /// The Wayland client chose an action for the offer.
    pub dnd_offer_action: bool,
    /// A synthetic drop is in flight: the release that ends the grab has not
    /// been injected yet.
    pub dnd_drop_armed: bool,
    /// The bounded fallback timer for a client that never accepts.
    pub dnd_drop_timer: Option<RegistrationToken>,
    /// The Wayland client's data source while a Wayland→X11 drag is live
    /// (T328b). `None` when the drag cannot cross (no `text/uri-list`).
    pub dnd_source: Option<WlDataSource>,
    /// The calloop source reading the `text/uri-list` payload the Wayland
    /// source writes after the X11 target asks for it.
    pub dnd_source_reader: Option<RegistrationToken>,
    /// Accumulated payload bytes from [`Self::dnd_source_reader`].
    pub dnd_source_buffer: Vec<u8>,
}

impl Default for XwaylandState {
    fn default() -> Self {
        Self {
            wm: None,
            display: None,
            display_number: None,
            running: false,
            pending_restart: false,
            source_token: None,
            start_count: 0,
            override_windows: Vec::new(),
            // Clamped to at least 1 even before the session's first output.
            integer_scale: 1,
            property_conn: None,
            frame_extents_atom: None,
            dnd: None,
            dnd_source_token: None,
            dnd_offer: None,
            dnd_offer_accepted: false,
            dnd_offer_action: false,
            dnd_drop_armed: false,
            dnd_drop_timer: None,
            dnd_source: None,
            dnd_source_reader: None,
            dnd_source_buffer: Vec::new(),
        }
    }
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

/// Map the Wayland DnD action set to the one action the bridge proposes to
/// X11. Copy is the default and the safe choice for a file drag; Move is used
/// only when the client offers nothing else, and Ask is last.
fn xdnd_action_for(actions: DndAction) -> XdndAction {
    if actions.contains(DndAction::Copy) {
        XdndAction::Copy
    } else if actions.contains(DndAction::Move) {
        XdndAction::Move
    } else if actions.contains(DndAction::Ask) {
        XdndAction::Ask
    } else {
        XdndAction::Copy
    }
}

/// Start the XDnD bridge-as-target on its own X11 connection (T328a).
///
/// The bridge publishes the root `XdndProxy` and creates the bridge window;
/// this function only owns its calloop readiness source. Called once when
/// Xwayland reports ready, and dropped when the server goes away.
fn start_xdnd_bridge(state: &mut DfState) {
    if state.xwayland.dnd.is_some() {
        return;
    }
    let Some(display) = state.xwayland.display.clone() else {
        return;
    };
    let bridge = match XdndBridge::connect(&display) {
        Ok(bridge) => bridge,
        Err(err) => {
            eprintln!("dragonfruit-compositor: XDnD target bridge unavailable: {err}");
            return;
        }
    };
    let fd = match bridge.duplicate_fd() {
        Ok(fd) => fd,
        Err(err) => {
            eprintln!("dragonfruit-compositor: XDnD bridge fd unavailable: {err}");
            return;
        }
    };
    let source = Generic::new(fd, Interest::READ, Mode::Level);
    match state.loop_handle.insert_source(source, |_, _, state| {
        state.poll_xdnd_bridge();
        Ok(PostAction::Continue)
    }) {
        Ok(token) => {
            println!(
                "dragonfruit-compositor: XDnD target bridge ready (window {:#x})",
                bridge.window()
            );
            state.xwayland.dnd = Some(bridge);
            state.xwayland.dnd_source_token = Some(token);
        }
        Err(err) => {
            eprintln!("dragonfruit-compositor: failed to register the XDnD bridge source: {err}")
        }
    }
}

/// Tear down the XDnD bridge (server restart or connection error).
fn stop_xdnd_bridge(state: &mut DfState) {
    state.end_xdnd_source();
    if let Some(token) = state.xwayland.dnd_source_token.take() {
        state.loop_handle.remove(token);
    }
    if let Some(token) = state.xwayland.dnd_drop_timer.take() {
        state.loop_handle.remove(token);
    }
    state.xwayland.dnd = None;
    state.xwayland.dnd_offer = None;
    state.xwayland.dnd_offer_accepted = false;
    state.xwayland.dnd_offer_action = false;
    state.xwayland.dnd_drop_armed = false;
}

/// Spawn Xwayland and register its event source. Idempotent; returns
/// `Ok(())` even when the binary is missing (Wayland-only sessions are
/// still valid).
pub fn start(state: &mut DfState) -> Result<(), String> {
    if state.xwayland.wm.is_some() || state.xwayland.pending_restart {
        return Ok(());
    }

    // Integer-scaled Xwayland (T-16.3a, `docs/design/xwayland-scaling.md`):
    // the server runs at `ceil(primary_output_scale)` (clamped to >= 1).
    // Smithay advertises the same value as the output's integer
    // `wl_output.scale`; the compositor never sends X11 surfaces a fractional
    // preferred scale (`render::post_repaint`), so X11 clients only ever see
    // the integer coordinate space. The per-surface fractional downscale is
    // T-16.3b.
    let integer_scale = state
        .space
        .outputs()
        .next()
        .map(|output| output.current_scale().integer_scale().max(1))
        .unwrap_or(1);
    state.xwayland.integer_scale = integer_scale;

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
                        start_xdnd_bridge(state);
                        println!(
                            "dragonfruit-compositor: Xwayland ready: DISPLAY={display} \
                             (start #{}, integer scale {})",
                            state.xwayland.start_count, state.xwayland.integer_scale
                        );
                    }
                    Err(err) => {
                        eprintln!("dragonfruit-compositor: failed to start X11 WM: {err}");
                        state.xwayland.running = false;
                    }
                }
            }
            XWaylandEvent::Error => {
                eprintln!("dragonfruit-compositor: Xwayland exited");
                state.xwayland.running = false;
                state.xwayland.display = None;
                // A live X11 WM still owns queued events for the dead server.
                // Restarting now would let those events be applied to the new
                // server's windows (a reused id can otherwise be associated
                // with a dead `wl_surface`, T326). When a WM exists, wait for
                // its `disconnected`, which is delivered only after its event
                // channel has fully drained; that path removes the windows and
                // schedules the restart. With no WM there is nothing to drain.
                if state.xwayland.wm.is_none() {
                    state.remove_all_x11_windows("Xwayland exited");
                    // Keep `display_number` so `maybe_restart` can clean up the
                    // dead server's socket/lock before respawning.
                    state.xwayland.pending_restart = true;
                } else {
                    eprintln!(
                        "dragonfruit-compositor: waiting for the X11 WM to drain before restart"
                    );
                }
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
    // The DnD bridge belongs to the dead server (T328a).
    stop_xdnd_bridge(state);
    // The property connection belongs to the dead server (T327).
    state.xwayland.property_conn = None;
    state.xwayland.frame_extents_atom = None;
    if let Some(number) = state.xwayland.display_number {
        cleanup_x11_files(number);
    }
    remove_display_file(&state.socket_name);
    // Smithay's xwayland-shell state keeps a serial → wl_surface map that is
    // never cleared when the Xwayland client disconnects. A restarted server
    // reuses Wayland object ids/serials, so without a fresh map its new
    // surface can be associated with the dead surface from the previous
    // server (and a reused X11 id then focuses a dead surface, T326).
    // Recreate the global so the new server starts with an empty map.
    let old_shell_global = state.xwayland_shell_state.global();
    state
        .display_handle
        .remove_global::<DfState>(old_shell_global);
    state.xwayland_shell_state = XWaylandShellState::new::<DfState>(&state.display_handle);
    if let Err(err) = start(state) {
        eprintln!("dragonfruit-compositor: Xwayland restart failed: {err}");
    }
}

// --- X11 window management --------------------------------------------------

/// Convert compositor window insets into the four `_NET_FRAME_EXTENTS` values
/// (left, right, top, bottom) in X11 client pixels (T327).
///
/// Xwayland's root window is the output's **logical** size (ADR 0152) and
/// Smithay sends X11 geometry unscaled (`client_scale` is 1), so one X11 pixel
/// is one compositor logical point at every integer scale. The SSD titlebar
/// height is therefore the top extent as-is. `integer_scale` is deliberately
/// not a multiplier: at scale 2 that would tell a client its frame is twice as
/// tall as the strip the compositor actually reserves. The parameter is
/// threaded through so the scale contract is explicit at the call site and a
/// future per-surface X11 scale has one place to change.
fn frame_extents(insets: WindowInsets, integer_scale: i32) -> [u32; 4] {
    debug_assert!(
        integer_scale >= 1,
        "the Xwayland integer scale is clamped to at least 1"
    );
    [
        insets.left.max(0) as u32,
        insets.right.max(0) as u32,
        insets.top.max(0) as u32,
        insets.bottom.max(0) as u32,
    ]
}

impl DfState {
    /// Open the private property connection and intern `_NET_FRAME_EXTENTS`.
    ///
    /// Smithay 0.7's `X11Wm` does not expose its X11 connection, so a second
    /// connection owned by the compositor publishes WM properties such as
    /// `_NET_FRAME_EXTENTS` (T327). It is opened lazily on the first write and
    /// dropped when the server goes away.
    fn ensure_x11_property_conn(&mut self) -> Option<Atom> {
        if let (Some(_), Some(atom)) = (
            &self.xwayland.property_conn,
            self.xwayland.frame_extents_atom,
        ) {
            return Some(atom);
        }
        self.xwayland.property_conn = None;
        self.xwayland.frame_extents_atom = None;
        let display = self.xwayland.display.clone()?;
        let (conn, _screen) = match RustConnection::connect(Some(&display)) {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!(
                    "dragonfruit-compositor: failed to open the X11 property connection: {err}"
                );
                return None;
            }
        };
        let atom = conn
            .intern_atom(false, b"_NET_FRAME_EXTENTS")
            .map_err(|err| err.to_string())
            .and_then(|cookie| cookie.reply().map_err(|err| err.to_string()));
        match atom {
            Ok(reply) => {
                self.xwayland.frame_extents_atom = Some(reply.atom);
                self.xwayland.property_conn = Some(conn);
                Some(reply.atom)
            }
            Err(err) => {
                eprintln!("dragonfruit-compositor: failed to intern _NET_FRAME_EXTENTS: {err}");
                None
            }
        }
    }

    /// Write `_NET_FRAME_EXTENTS` on one X11 window.
    ///
    /// `sync` forces a server round-trip so a client that reads the property
    /// from its first `MapNotify` sees the value; ordinary updates only flush.
    fn write_x11_frame_extents(&mut self, window_id: X11Window, extents: [u32; 4], sync: bool) {
        let Some(atom) = self.ensure_x11_property_conn() else {
            return;
        };
        let Some(conn) = self.xwayland.property_conn.as_ref() else {
            return;
        };
        let mut ok = true;
        if let Err(err) = conn.change_property32(
            PropMode::REPLACE,
            window_id,
            atom,
            AtomEnum::CARDINAL,
            &extents,
        ) {
            eprintln!(
                "dragonfruit-compositor: failed to set _NET_FRAME_EXTENTS on {window_id:#x}: {err}"
            );
            ok = false;
        }
        if ok {
            let flushed = if sync {
                conn.sync().map_err(|err| err.to_string())
            } else {
                conn.flush().map_err(|err| err.to_string())
            };
            if let Err(err) = flushed {
                eprintln!(
                    "dragonfruit-compositor: failed to flush _NET_FRAME_EXTENTS on {window_id:#x}: {err}"
                );
                ok = false;
            }
        }
        if !ok {
            // A dead connection would otherwise fail every later window.
            self.xwayland.property_conn = None;
            self.xwayland.frame_extents_atom = None;
        }
    }

    /// Publish the current frame extents for a managed X11 window (T327).
    ///
    /// Called after a map and after any change that can move the SSD titlebar
    /// strip: a decoration-tier flip, a chrome-style change, or a window state
    /// transition (fullscreen hides the strip; zoom/minimize keep it). CSD X11
    /// windows reserve no inset, so they publish zero extents.
    pub(crate) fn publish_x11_frame_extents(&mut self, window: &Window) {
        let Some(surface) = window.x11_surface() else {
            return;
        };
        if surface.is_override_redirect() {
            return;
        }
        let extents = frame_extents(self.insets_for(window), self.xwayland.integer_scale);
        self.write_x11_frame_extents(surface.window_id(), extents, false);
    }

    /// Publish extents for an X11 surface that is not (yet) in the model.
    ///
    /// Used at `CreateNotify` to seed the property before the client can map,
    /// and again just before the frame is mapped. The tier comes straight from
    /// `_MOTIF_WM_HINTS`; a new window uses the default chrome style (a later
    /// style change republishes through [`Self::publish_x11_frame_extents`]).
    ///
    /// `sync` is implicit: both call sites need the value processed before the
    /// WM sends the reparent/map requests on the other connection, so a client
    /// reading on `MapNotify` sees it.
    fn publish_x11_frame_extents_for_surface(&mut self, surface: &X11Surface) {
        if surface.is_override_redirect() {
            return;
        }
        let tier = Self::x11_decoration_tier(surface);
        let chrome = self
            .x11_window_for(surface)
            .map(|window| self.windows.window_chrome(&window))
            .unwrap_or_else(|| WindowChrome::new(DEFAULT_WINDOW_STYLE));
        let extents = frame_extents(
            WindowInsets::for_tier_with_chrome(tier, chrome),
            self.xwayland.integer_scale,
        );
        self.write_x11_frame_extents(surface.window_id(), extents, true);
    }

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

    /// Drain the XDnD bridge's X connection and act on what it observed
    /// (T328a). Called from the bridge's calloop readiness source.
    pub(crate) fn poll_xdnd_bridge(&mut self) {
        let result = self.xwayland.dnd.as_mut().map(|bridge| bridge.poll());
        match result {
            Some(Ok(events)) => {
                for event in events {
                    self.handle_xdnd_event(event);
                }
            }
            Some(Err(err)) => {
                eprintln!("dragonfruit-compositor: XDnD bridge error: {err}");
                stop_xdnd_bridge(self);
            }
            None => {}
        }
    }

    fn handle_xdnd_event(&mut self, event: BridgeEvent) {
        match event {
            BridgeEvent::Dropped { paths, .. } => {
                // The X11 source has already been told the target took the
                // data. Offer the paths to the Wayland surface under the
                // pointer; `send` serves them when the client requests them.
                self.clear_xdnd_offer();
                self.xwayland.dnd_offer = Some(paths);
                self.offer_xdnd_drop_to_wayland();
            }
            BridgeEvent::Failed { source, reason } => {
                eprintln!("dragonfruit-compositor: XDnD drop from {source:#x} failed: {reason}");
                self.clear_xdnd_offer();
            }
            BridgeEvent::Leave => self.clear_xdnd_offer(),
            BridgeEvent::TransferStarted { .. } => {
                // The bridge's deadline is only checked when the connection
                // wakes it. A one-shot wakeup guarantees a silent or wedged
                // source still expires and is refused.
                let wake = TRANSFER_TIMEOUT + Duration::from_millis(200);
                let _ =
                    self.loop_handle
                        .insert_source(Timer::from_duration(wake), |_, _, state| {
                            state.poll_xdnd_bridge();
                            TimeoutAction::Drop
                        });
            }
            BridgeEvent::Enter { .. } | BridgeEvent::Position { .. } => {}
            BridgeEvent::SourceStatus { .. } => {}
            BridgeEvent::SourceSelectionNeeded { .. } => self.request_xdnd_source_payload(),
            BridgeEvent::SourceSelectionServed { .. } => {}
            BridgeEvent::SourceFinished { .. } | BridgeEvent::SourceTimedOut { .. } => {
                self.end_xdnd_source();
            }
        }
    }

    /// Start a compositor-owned Wayland drag at the pointer and immediately
    /// deliver the X11 drop to the surface under it.
    ///
    /// The X11 source drops on button release, so by the time the bridge has
    /// the payload the physical press is over. The compositor therefore
    /// synthesises the `enter`/`drop` pair: it installs a pointer grab with the
    /// current location, sends the offer to the surface under the pointer, and
    /// releases as soon as the client has accepted both the MIME type and an
    /// action (bounded by a 750 ms fallback). The client sees a normal data
    /// offer and requests `text/uri-list` through `send`.
    fn offer_xdnd_drop_to_wayland(&mut self) {
        let Some(paths) = self.xwayland.dnd_offer.as_ref() else {
            return;
        };
        if paths.is_empty() {
            self.clear_xdnd_offer();
            return;
        }
        let Some(pointer) = self.seat.get_pointer() else {
            self.clear_xdnd_offer();
            return;
        };
        if self.lock.is_locked() || self.suspend.is_suspended() {
            // A drag cannot legitimately complete while input is captured.
            self.clear_xdnd_offer();
            return;
        }
        if pointer.is_grabbed() {
            // Never clobber a client's grab (a menu, a popup, or an earlier
            // synthetic drop). The X11 source has already been answered.
            eprintln!("dragonfruit-compositor: XDnD drop skipped because the pointer is grabbed");
            self.clear_xdnd_offer();
            return;
        }
        let location = pointer.current_location();
        let Some((surface, origin)) = crate::input::surface_under(self, location) else {
            eprintln!("dragonfruit-compositor: XDnD drop had no Wayland surface under the pointer");
            self.clear_xdnd_offer();
            return;
        };
        if self.is_above_window_chrome_surface(&surface) {
            // Shell chrome is not a data-device client.
            self.clear_xdnd_offer();
            return;
        }

        let start = GrabStartData {
            focus: Some((surface.clone(), origin.to_f64())),
            button: 0x110, // BTN_LEFT
            location,
        };
        let seat = self.seat.clone();
        let display_handle = self.display_handle.clone();
        let serial = SERIAL_COUNTER.next_serial();
        self.xwayland.dnd_drop_armed = true;
        self.xwayland.dnd_offer_accepted = false;
        self.xwayland.dnd_offer_action = false;
        start_dnd(
            &display_handle,
            &seat,
            self,
            serial,
            Some(start),
            None,
            SourceMetadata {
                mime_types: vec![URI_LIST_MIME.to_string()],
                dnd_action: DndAction::Copy,
            },
        );
        // Deliver the offer to the surface under the pointer. The grab is
        // active; this is the `wl_data_device.enter` the client answers.
        pointer.motion(
            self,
            Some((surface, origin.to_f64())),
            &MotionEvent {
                location,
                serial: SERIAL_COUNTER.next_serial(),
                time: 0,
            },
        );
        pointer.frame(self);
        // If the client never accepts, finish the offer (which cancels it)
        // so the grab cannot outlive the drop.
        let token = self
            .loop_handle
            .insert_source(
                Timer::from_duration(Duration::from_millis(750)),
                |_, _, state| {
                    state.xwayland.dnd_drop_timer = None;
                    state.finish_xdnd_wayland_drop();
                    TimeoutAction::Drop
                },
            )
            .ok();
        self.xwayland.dnd_drop_timer = token;
    }

    /// Inject the release that ends the synthetic Wayland drag. The grab's
    /// `button` handler turns the empty pressed set into a drop (validated) or
    /// a cancellation.
    pub(crate) fn finish_xdnd_wayland_drop(&mut self) {
        if !self.xwayland.dnd_drop_armed {
            return;
        }
        self.xwayland.dnd_drop_armed = false;
        if let Some(token) = self.xwayland.dnd_drop_timer.take() {
            self.loop_handle.remove(token);
        }
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();
        pointer.button(
            self,
            &ButtonEvent {
                serial,
                time: 0,
                button: 0x110, // BTN_LEFT
                state: ButtonState::Released,
            },
        );
        pointer.frame(self);
    }

    /// Finish the synthetic drop once the client has accepted both the MIME
    /// type and an action.
    pub(crate) fn maybe_finish_xdnd_wayland_drop(&mut self) {
        if self.xwayland.dnd_drop_armed
            && self.xwayland.dnd_offer_accepted
            && self.xwayland.dnd_offer_action
        {
            self.finish_xdnd_wayland_drop();
        }
    }

    /// Drop the current Wayland offer state (new drag, cancellation, finish).
    pub(crate) fn clear_xdnd_offer(&mut self) {
        self.xwayland.dnd_offer = None;
        self.xwayland.dnd_offer_accepted = false;
        self.xwayland.dnd_offer_action = false;
        self.xwayland.dnd_drop_armed = false;
        if let Some(token) = self.xwayland.dnd_drop_timer.take() {
            self.loop_handle.remove(token);
        }
    }

    /// Begin the Wayland→X11 half of the bridge (T328b). The bridge window
    /// takes `XdndSelection` ownership; the payload is fetched lazily when an
    /// X11 target asks for it. A drag without `text/uri-list` never crosses.
    pub(crate) fn begin_xdnd_source(&mut self, source: Option<WlDataSource>) {
        self.end_xdnd_source();
        let Some(source) = source else {
            return;
        };
        let metadata = with_source_metadata(&source, |metadata| {
            (metadata.mime_types.clone(), metadata.dnd_action)
        });
        let Ok((mime_types, actions)) = metadata else {
            return;
        };
        if !mime_types.iter().any(|mime| mime == URI_LIST_MIME) {
            return;
        }
        let Some(bridge) = self.xwayland.dnd.as_mut() else {
            return;
        };
        if let Err(err) = bridge.begin_source(xdnd_action_for(actions)) {
            eprintln!("dragonfruit-compositor: XDnD source unavailable: {err}");
            return;
        }
        self.xwayland.dnd_source = Some(source);
        // A drag can start while the pointer already rests over an X11 window.
        if let Some(pointer) = self.seat.get_pointer() {
            let location = pointer.current_location();
            self.update_xdnd_source(location);
        }
    }

    /// End the Wayland→X11 drag: leave any X11 target and release
    /// `XdndSelection`.
    pub(crate) fn end_xdnd_source(&mut self) {
        if let Some(bridge) = self.xwayland.dnd.as_mut() {
            if let Err(err) = bridge.end_source() {
                eprintln!("dragonfruit-compositor: XDnD source teardown: {err}");
            }
        }
        self.xwayland.dnd_source = None;
        if let Some(token) = self.xwayland.dnd_source_reader.take() {
            self.loop_handle.remove(token);
        }
        self.xwayland.dnd_source_buffer.clear();
    }

    /// Follow the live Wayland drag across X11 windows. Called after every
    /// pointer motion; a no-op when no client drag is active.
    pub(crate) fn update_xdnd_source(&mut self, location: Point<f64, Logical>) {
        if self.xwayland.dnd_source.is_none() {
            return;
        }
        let target = self.x11_window_under(location);
        let position = self.xdnd_root_position(location);
        let Some(bridge) = self.xwayland.dnd.as_mut() else {
            return;
        };
        let current = bridge.source_target();
        if target == current {
            if target.is_some() {
                let _ = bridge.source_position(position, 0);
            }
            return;
        }
        if current.is_some() {
            let _ = bridge.source_leave();
        }
        if let Some(target) = target {
            if let Err(err) = bridge.source_enter(target, position, 0) {
                eprintln!("dragonfruit-compositor: XdndEnter failed: {err}");
            }
        }
    }

    /// The client released the drag. Drop only when the X11 target answered
    /// `XdndStatus` with an accept; otherwise leave and release.
    pub(crate) fn finish_xdnd_source_drop(&mut self) {
        let result = self
            .xwayland
            .dnd
            .as_mut()
            .map(|bridge| (bridge.source_target(), bridge.source_drop(0)));
        match result {
            Some((_, Ok(true))) => {
                // The bridge's deadline bounds a target that never finishes;
                // arm a wakeup because a silent target sends no X event.
                let wake = TRANSFER_TIMEOUT + Duration::from_millis(200);
                let _ =
                    self.loop_handle
                        .insert_source(Timer::from_duration(wake), |_, _, state| {
                            state.poll_xdnd_bridge();
                            TimeoutAction::Drop
                        });
            }
            Some((_, Ok(false))) | None => self.end_xdnd_source(),
            Some((_, Err(err))) => {
                eprintln!("dragonfruit-compositor: XdndDrop failed: {err}");
                self.end_xdnd_source();
            }
        }
    }

    /// The X11 target asked for the payload before it was fetched. Ask the
    /// Wayland source to write `text/uri-list` into a pipe and read it from a
    /// calloop source, so the event loop never blocks on the client.
    fn request_xdnd_source_payload(&mut self) {
        if self.xwayland.dnd_source_reader.is_some() {
            return;
        }
        let Some(source) = self.xwayland.dnd_source.clone() else {
            return;
        };
        let Ok((reader, writer)) = UnixStream::pair() else {
            return;
        };
        if reader.set_nonblocking(true).is_err() {
            return;
        }
        source.send(URI_LIST_MIME.to_string(), writer.as_fd());
        self.xwayland.dnd_source_buffer.clear();
        let token = self
            .loop_handle
            .insert_source(
                Generic::new(reader, Interest::READ, Mode::Level),
                |_readiness, stream, state| {
                    use std::io::Read as _;
                    let mut reader = stream.as_ref();
                    let mut buf = [0u8; 8192];
                    loop {
                        match reader.read(&mut buf) {
                            Ok(0) => {
                                state.finish_xdnd_source_payload();
                                return Ok(PostAction::Remove);
                            }
                            Ok(read) => {
                                if state.xwayland.dnd_source_buffer.len() + read > MAX_PAYLOAD_BYTES
                                {
                                    state.finish_xdnd_source_payload();
                                    return Ok(PostAction::Remove);
                                }
                                state
                                    .xwayland
                                    .dnd_source_buffer
                                    .extend_from_slice(&buf[..read]);
                            }
                            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                                return Ok(PostAction::Continue);
                            }
                            Err(_) => {
                                state.finish_xdnd_source_payload();
                                return Ok(PostAction::Remove);
                            }
                        }
                    }
                },
            )
            .ok();
        self.xwayland.dnd_source_reader = token;
    }

    fn finish_xdnd_source_payload(&mut self) {
        self.xwayland.dnd_source_reader = None;
        let payload = std::mem::take(&mut self.xwayland.dnd_source_buffer);
        if let Some(bridge) = self.xwayland.dnd.as_mut() {
            if let Err(err) = bridge.set_source_payload(payload) {
                eprintln!("dragonfruit-compositor: XDnD source payload failed: {err}");
            }
        }
    }

    /// The topmost X11 window whose surface is under `location`, if any.
    fn x11_window_under(&self, location: Point<f64, Logical>) -> Option<X11Window> {
        let (surface, _) = crate::input::surface_under(self, location)?;
        let window = self.window_for_surface(&surface)?;
        window
            .x11_surface()
            .map(smithay::xwayland::X11Surface::window_id)
    }

    /// Convert a logical scene point to Xwayland root coordinates. Xwayland
    /// renders at the session's integer scale; the root is the primary
    /// output's space (multi-monitor is recorded as a follow-up).
    fn xdnd_root_position(&self, location: Point<f64, Logical>) -> (i16, i16) {
        let scale = f64::from(self.xwayland.integer_scale.max(1));
        let clamp = |value: f64| value.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
        (
            clamp((location.x * scale).round()),
            clamp((location.y * scale).round()),
        )
    }

    /// Drop every X11 window when the X server goes away (T326).
    ///
    /// `space.refresh()` drops dead `Window`s from the scene but not from
    /// [`WindowModel`], which is the shell's source; each entry must go
    /// through [`Self::remove_window`] so the shell receives `Unmapped` and
    /// focus is released. Override-redirect windows live outside the model
    /// and are unmapped directly. Clearing `override_windows` (and the model
    /// entries) is what lets a restarted server's reused X11 window id map as
    /// a **new** window instead of matching the dead entry through
    /// [`Self::x11_window_for`].
    pub(crate) fn remove_all_x11_windows(&mut self, reason: &str) {
        // Release the keyboard before the surfaces disappear. `remove_window`
        // also does this for the focused entry, but an X11 surface can hold
        // the seat focus without the `active_window` cache naming it.
        let active_is_x11 = self
            .active_window
            .as_ref()
            .is_some_and(|window| window.x11_surface().is_some());
        let seat_is_x11 = self
            .seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus())
            .and_then(|surface| self.window_for_surface(&surface))
            .is_some_and(|window| window.x11_surface().is_some());
        if active_is_x11 || seat_is_x11 {
            self.clear_keyboard_focus(reason);
        }

        let windows: Vec<Window> = self
            .windows
            .windows()
            .filter(|window| window.x11_surface().is_some())
            .cloned()
            .collect();
        for window in windows {
            self.remove_window(&window, WindowEventKind::Unmapped);
        }

        for (_, window) in self.xwayland.override_windows.drain(..) {
            self.space.unmap_elem(&window);
        }
        self.needs_redraw = true;
    }

    /// Publish the raw X11 identity for a window. The `WM_CLASS` class is
    /// preferred (falling back to the instance); `app-index` resolves it to a
    /// `.desktop` entry, so the compositor owns no desktop database (T-14.1a,
    /// ADR 0086). The raw class still lets the compositor group windows.
    fn resolve_x11_identity(&mut self, instance: &str, class: &str) -> Option<String> {
        let instance = x11_string(instance);
        let class = x11_string(class);
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
                let (placement_output, geometry) = if let Some(parent) = &parent {
                    let parent_geometry = self.windows.geometry(parent).unwrap_or_default();
                    (
                        self.output_name_for(parent),
                        centered_on(parent_geometry, size),
                    )
                } else {
                    let (output_name, output_geometry) = self.focused_output();
                    // Honor an ICCCM *user-specified* position (`USPosition`),
                    // which a window manager must; `PPosition` is only a hint,
                    // so everything else cascades.
                    let requested = surface
                        .size_hints()
                        .and_then(|hints| hints.position)
                        .filter(|(spec, _, _)| {
                            matches!(
                                spec,
                                smithay::reexports::x11rb::properties::WmSizeHintsSpecification::UserSpecified
                            )
                        })
                        .map(|(_, x, y)| (x, y));
                    let geometry = match (requested, output_geometry) {
                        (Some(position), Some(output)) => {
                            user_positioned_geometry(output, size, position)
                        }
                        _ => {
                            let index = output_name
                                .as_deref()
                                .map(|name| self.windows.cascade_index(name))
                                .unwrap_or(0);
                            // Cascade inside the focused output's usable area,
                            // clear of that display's reserved chrome (T-16.1b).
                            let zones = output_name
                                .as_deref()
                                .map(|name| self.shell.reserved_zones_for(name))
                                .unwrap_or(self.reserved_zones);
                            output_geometry
                                .map(|output| {
                                    cascaded_geometry(
                                        zones.usable(output),
                                        size,
                                        index,
                                        CASCADE_STEP,
                                    )
                                })
                                .unwrap_or_else(|| Rectangle::new(Point::from((0, 0)), size))
                        }
                    };
                    (output_name, geometry)
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
                self.assign_new_window_space(&window, id, placement_output.as_deref());
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
        // A live close ghost owns the removal: the X client may be destroyed
        // as soon as it gets WM_DELETE_WINDOW, but the model entry is removed
        // exactly once when the fade/scale-out settles (T-02.4a).
        if self.windows.is_closing(&window) {
            return;
        }
        self.remove_window(&window, WindowEventKind::Unmapped);
    }
}

impl XwmHandler for DfState {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwayland
            .wm
            .as_mut()
            .expect("X11 WM callback without a live X11Wm")
    }

    fn new_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        // T327: seed `_NET_FRAME_EXTENTS` at CreateNotify. Smithay maps the
        // client window during reparenting before `map_window_request` runs,
        // so this is the earliest point that precedes the client's first
        // `MapNotify`; the map path republishes the final tier/style/state.
        self.publish_x11_frame_extents_for_surface(&surface);
    }

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        // T327: publish `_NET_FRAME_EXTENTS` before the frame is mapped so a
        // client that reads it from its first MapNotify sees the final value.
        // The post-map call below re-reads the model (state/style may have
        // changed) and covers windows restored from minimize/fullscreen.
        self.publish_x11_frame_extents_for_surface(&surface);
        if let Err(err) = surface.set_mapped(true) {
            eprintln!("dragonfruit-compositor: failed to map X11 window: {err}");
            return;
        }
        self.map_x11_window(&surface);
        if let Some(window) = self.x11_window_for(&surface) {
            self.publish_x11_frame_extents(&window);
        }
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
                self.set_decoration_tier(&window, tier);
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

    fn disconnected(&mut self, xwm: XwmId) {
        // A restarted server installs a new `X11Wm`. A late `Closed` from the
        // previous WM source must not tear down the new server's windows
        // (T326), so only the current WM owns the teardown.
        if self.xwayland.wm.as_ref().map(|wm| wm.id()) != Some(xwm) {
            return;
        }
        eprintln!(
            "dragonfruit-compositor: X11 WM disconnected; removing X11 windows and scheduling restart"
        );
        self.remove_all_x11_windows("Xwayland disconnected");
        self.xwayland.wm = None;
        self.xwayland.running = false;
        self.xwayland.pending_restart = true;
        // Drop the property connection: it belongs to the dead server (T327).
        self.xwayland.property_conn = None;
        self.xwayland.frame_extents_atom = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_extents_are_the_ssd_strip_in_x11_pixels() {
        // SSD: the titlebar height is the top extent and nothing else.
        assert_eq!(
            frame_extents(WindowInsets::for_tier(DecorationTier::ServerSide), 1),
            [0, 0, DEFAULT_WINDOW_STYLE.height() as u32, 0]
        );
        // CSD: the client draws its own frame, so it reserves no extents.
        assert_eq!(
            frame_extents(WindowInsets::for_tier(DecorationTier::ClientSide), 1),
            [0, 0, 0, 0]
        );
    }

    #[test]
    fn frame_extents_do_not_reapply_the_integer_scale() {
        // Xwayland's X screen is the output's logical space at every integer
        // scale (ADR 0152), so a 32 pt titlebar is 32 X pixels at scale 2 as
        // well. Multiplying here would make a client reserve twice the strip.
        let insets = WindowInsets::for_tier(DecorationTier::ServerSide);
        assert_eq!(frame_extents(insets, 1), frame_extents(insets, 2));
        assert_eq!(
            frame_extents(insets, 2)[2],
            DEFAULT_WINDOW_STYLE.height() as u32
        );
    }
}
