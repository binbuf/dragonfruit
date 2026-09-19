// SPDX-License-Identifier: MIT OR Apache-2.0
//! Compositor state for the T-01 skeleton.
//!
//! The skeleton serves an empty registry: clients can connect, sync, and
//! stay alive, but no globals are advertised. That is exactly enough to
//! prove socket, teardown, and lockstep contracts without pre-empting the
//! window manager work in T-02 and the private protocols in T-07.

use std::sync::Arc;

use smithay::reexports::wayland_server as ws;

use ws::backend::{ClientData, DisconnectReason};
use ws::protocol::{wl_callback, wl_registry};
use ws::{Client, DataInit, Dispatch, DisplayHandle};

/// Per-client data; nothing to track in the skeleton.
#[derive(Debug, Default)]
pub struct DfClientData;

impl ClientData for DfClientData {
    fn disconnected(&self, _client_id: ws::backend::ClientId, _reason: DisconnectReason) {}
}

/// Minimal compositor state: the display handle, the listening socket, and
/// the running flag. T-02 replaces this with the real window state.
pub struct DfState {
    pub display_handle: Option<DisplayHandle>,
    pub socket: Option<ws::ListeningSocket>,
    pub running: bool,
}

impl DfState {
    pub fn new() -> Self {
        Self {
            display_handle: None,
            socket: None,
            running: true,
        }
    }

    pub fn new_client_data() -> Arc<dyn ClientData> {
        Arc::new(DfClientData)
    }

    /// Accept every pending client connection (non-blocking).
    pub fn accept_clients(&mut self) {
        while let Some(socket) = self.socket.as_ref() {
            let Ok(Some(stream)) = socket.accept() else {
                break;
            };
            let Some(mut handle) = self.display_handle.clone() else {
                break;
            };
            if let Err(e) = handle.insert_client(stream, Self::new_client_data()) {
                eprintln!("dragonfruit-compositor: rejecting client: {e}");
            }
        }
    }
}

impl Default for DfState {
    fn default() -> Self {
        Self::new()
    }
}

// --- Wayland dispatch -------------------------------------------------------
//
// `wl_display` requests (sync, get_registry) are handled internally by
// wayland-server. The skeleton creates no globals, so clients get an empty
// registry and working `sync` round-trips. Bind requests target names that
// were never advertised, so they are ignored — the backend turns use of
// the never-created object into a protocol error, not a crash
// (.docs/design/14-risks.md: never crash on malformed requests).

impl Dispatch<wl_registry::WlRegistry, ()> for DfState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_registry::WlRegistry,
        _request: wl_registry::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        // No globals are advertised in the skeleton; a bind can only
        // reference names the client invented. Ignore — see module docs.
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for DfState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_callback::WlCallback,
        _request: wl_callback::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}
