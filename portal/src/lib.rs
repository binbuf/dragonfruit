// SPDX-License-Identifier: GPL-3.0-or-later
//! `xdg-desktop-portal-dragonfruit`: the Dragonfruit `xdg-desktop-portal`
//! backend (T-13.1a).
//!
//! The backend is a real session-bus service discovered by the
//! `xdg-desktop-portal` frontend from the `dragonfruit.portal` descriptor and
//! `dragonfruit-portals.conf` this crate ships. It owns the standard name
//! `org.freedesktop.impl.portal.desktop.dragonfruit` at
//! `/org/freedesktop/portal/desktop` and degrades gracefully when the
//! frontend is absent: registering and serving do not depend on it
//! ([`dbus`]).
//!
//! # What lives here
//!
//! * [`model`] — the pure identity, advertised interfaces, and the
//!   frontend-presence state. No D-Bus, so it is unit-tested directly.
//! * [`data`] — the three discoverability files (`.portal`, `portals.conf`,
//!   D-Bus activation), their parsers, and [`data::install_into`] for
//!   packaging.
//! * [`dbus`] — the session-bus service, the live frontend watch, and the
//!   presenter-loss watch (T311).
//! * [`liveness`] — the bounded wait every interactive request uses (T311).
//! * [`chooser`] — the FileChooser's pure model (T-13.2a): the options/result
//!   vardicts, the request registry and its one-shot completion, and the
//!   files-core listing the picker drives.
//! * [`screenshot`] — the Screenshot's pure model (T-13.3a): the capture
//!   mode, the request registry and its one-shot completion, and the URI
//!   normalization the presenter's capture result uses.
//! * [`stream`] — the ScreenCast stream transport (T-13.4b, live in T335a): the
//!   negotiation seam that turns a chosen source into a live PipeWire node or
//!   names the stills-only fallback, and the [`stream::LiveTransport`] that
//!   runs the shipped bridge on one thread. A session owns its live nodes
//!   (T335b): a second `Start` replaces them, a close releases them, and the
//!   presenter-loss watch releases every stream.
//! * [`screencast_bridge`] — the portal's own trusted compositor client
//!   (T335b): it presents the portal role's launch token, binds
//!   `df_toplevel_manager` v17, and creates/releases the stream nodes.
//!
//! Settings and GlobalShortcuts landed in T-13.1b; FileChooser in T-13.2a;
//! Screenshot in T-13.3a, ScreenCast in T-13.4a, and the stream negotiation in
//! T-13.4b. The registration
//! contract is frozen in
//! [ADR 0074](../../docs/design/adr/0074-portal-backend-registration-and-frontend-degradation.md)
//! the Settings/GlobalShortcuts projection in
//! [ADR 0075](../../docs/design/adr/0075-settings-and-globalshortcuts-portals.md),
//! and the FileChooser's presenter seam in
//! [ADR 0076](../../docs/design/adr/0076-filechooser-portal-and-presenter-seam.md).

pub mod chooser;
pub mod data;
pub mod dbus;
pub mod interfaces;
pub mod liveness;
pub mod model;
pub mod screencast;
pub mod screencast_bridge;
pub mod screenshot;
pub mod settings;
pub mod shortcuts;
pub mod stream;

pub use chooser::{
    list_directory, normalize_uri, ChooserEntry, ChooserKind, ChooserOptions, ChooserRegistry,
    ChooserRequest, ChooserResponse, DirectoryListing, FILE_CHOOSER_INTERFACE,
    FILE_CHOOSER_VERSION,
};
pub use data::{
    install_into, preferred_backends, ACTIVATION_FILE, ACTIVATION_FILE_NAME, DBUS_SERVICES_DIR,
    PORTALS_CONF, PORTALS_CONF_DIR, PORTALS_CONF_NAME, PORTALS_DIR, PORTAL_FILE, PORTAL_FILE_NAME,
};
pub use dbus::{
    initialize, initialize_with_screencast, initialize_with_timeout, probe_frontend,
    probe_presenter, serve, serve_with_screencast, serve_with_timeout, Backend,
};
pub use interfaces::{
    FileChooserPortal, GlobalShortcuts, ScreenCastPortal, ScreenCastSessionObject,
    ScreenshotPortal, SettingsPortal, ShortcutSessionObject,
};
pub use liveness::{await_bounded, COMPLETION_TIMEOUT};
pub use model::{
    BackendStatus, FrontendPresence, FrontendTracker, BACKEND_INTERFACES, BACKEND_NAME, DBUS_NAME,
    DBUS_PATH, FRONTEND_NAME, PRESENTER_NAME, STATUS_INTERFACE,
};
pub use screencast::{
    ScreenCastError, ScreenCastOptions, ScreenCastRegistry, ScreenCastRequest, ScreenCastResponse,
    ScreenCastSelection, ScreenCastSession, ScreenCastStream, SourceType, SCREENCAST_INTERFACE,
    SCREENCAST_VERSION,
};
pub use screencast_bridge::{
    ScreenCastBridge, BRIDGE_SOCKET_ENV, BRIDGE_TOKEN_ENV, CREATE_TIMEOUT, LOCKSTEP_VERSION,
    MANAGER_VERSION,
};
pub use screenshot::{
    CaptureMode, ScreenshotError, ScreenshotOptions, ScreenshotRegistry, ScreenshotRequest,
    ScreenshotResponse, SCREENSHOT_INTERFACE, SCREENSHOT_VERSION,
};
pub use stream::{
    AbsentCompositor, CompositorWatch, ConnectFn, CreatedNode, FallbackReason, FnWatch,
    LiveTransport, NegotiatedStream, PipeWireTransport, ShellBridgeWatch, StillsTransport,
    StreamMode, StreamNegotiator, StreamSource, StreamTransport, WatchError, DEFAULT_STREAM_FPS,
    STREAM_FALLBACK_PROPERTY, STREAM_MODE_PROPERTY,
};

/// Open the live screen-share bridge from this process's provisioning
/// (`XDG_RUNTIME_DIR`, [`BRIDGE_TOKEN_ENV`], [`BRIDGE_SOCKET_ENV`]) as the
/// [`CompositorWatch`] the live transport consumes.
///
/// This is the one resolver the shipped daemon uses; it must be called on the
/// thread that later issues stream requests, because the bridge's wayland
/// event queue belongs to that thread. Tests substitute their own resolver at
/// the same seam.
pub fn screencast_bridge_from_environment_watch() -> Result<Box<dyn CompositorWatch>, String> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    let token = std::env::var(BRIDGE_TOKEN_ENV).ok();
    let socket = std::env::var(BRIDGE_SOCKET_ENV).ok();
    screencast_bridge::from_environment(runtime_dir.as_deref(), token.as_deref(), socket.as_deref())
        .map(|bridge| Box::new(bridge) as Box<dyn CompositorWatch>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backend_name_follows_the_lockstep_contract() {
        assert_eq!(DBUS_NAME, "org.freedesktop.impl.portal.desktop.dragonfruit");
        assert!(DBUS_NAME.starts_with("org.freedesktop.impl.portal.desktop."));
        assert!(DBUS_NAME.ends_with(df_ipc::DESKTOP_NAME));
        assert!(df_ipc::is_valid_dbus_name(STATUS_INTERFACE));
    }
}
