// SPDX-License-Identifier: MIT
//! The session-bus service (T-13.1a).
//!
//! The backend owns one well-known name, [`DBUS_NAME`], and serves the
//! standard object path [`DBUS_PATH`]. T-13.1a installs the service and its
//! registration only; the concrete `org.freedesktop.impl.portal.*`
//! interfaces are added additively by T-13.1b and later tasks.
//!
//! Alongside the standard interfaces (session bus / object manager plumbing
//! and `org.freedesktop.DBus.Introspectable`, provided by zbus), the backend
//! serves [`STATUS_INTERFACE`] at the same path: a Dragonfruit diagnostic
//! surface that reports the identity and whether the `xdg-desktop-portal`
//! frontend is on the bus. It is not part of the portal contract.
//!
//! ## Frontend absence is normal
//!
//! The backend does not depend on the frontend: it registers regardless and
//! keeps serving. It only *observes* the frontend's well-known name — once at
//! startup and then live via `org.freedesktop.DBus.NameOwnerChanged` — and
//! exposes that presence on [`STATUS_INTERFACE`] so a session with no
//! `xdg-desktop-portal` is a visible, supported state rather than an error
//! (`tests/session_bus.rs` proves both paths).

use zbus::blocking::connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedValue};

use crate::chooser::{self, SharedChooser};
use crate::interfaces::{
    FileChooserPortal, GlobalShortcuts, ScreenCastPortal, ScreenshotPortal, SettingsPortal,
    FILE_CHOOSER_INTERFACE, SCREENCAST_INTERFACE, SCREENSHOT_INTERFACE, SETTINGS_INTERFACE,
};
use crate::model::{
    BackendStatus, FrontendPresence, FrontendTracker, BACKEND_INTERFACES, BACKEND_NAME, DBUS_NAME,
    DBUS_PATH, FRONTEND_NAME, STATUS_INTERFACE,
};
use crate::screencast::{self, SharedScreenCast};
use crate::screenshot::{self, SharedScreenshot};
use crate::shortcuts::{self, SharedRegistry, GLOBAL_SHORTCUTS_INTERFACE};

/// The `org.dragonfruit.Portal1` object. It shares the standard portal path
/// with the concrete `org.freedesktop.impl.portal.*` interfaces.
#[derive(Clone)]
pub struct Backend {
    frontend: FrontendTracker,
    shortcuts: SharedRegistry,
    chooser: SharedChooser,
    screenshot: SharedScreenshot,
    screencast: SharedScreenCast,
}

impl Backend {
    /// A diagnostic object over `frontend`, with its own empty session,
    /// chooser, screenshot, and screencast registries.
    pub fn new(frontend: FrontendTracker) -> Self {
        Backend {
            frontend,
            shortcuts: shortcuts::registry(),
            chooser: chooser::registry(),
            screenshot: screenshot::registry(),
            screencast: screencast::registry(),
        }
    }

    /// A diagnostic object sharing `shortcuts` with the served
    /// GlobalShortcuts interface (the activation bridge), with its own empty
    /// chooser, screenshot, and screencast registries.
    pub fn with_shortcuts(frontend: FrontendTracker, shortcuts: SharedRegistry) -> Self {
        Backend {
            frontend,
            shortcuts,
            chooser: chooser::registry(),
            screenshot: screenshot::registry(),
            screencast: screencast::registry(),
        }
    }

    /// A diagnostic object sharing every served registry. The FileChooser's
    /// success path drives [`Backend::complete_file_chooser`] through
    /// `chooser`, the Screenshot's drives [`Backend::complete_screenshot`]
    /// through `screenshot`, and the ScreenCast's drives
    /// [`Backend::complete_screen_cast`] through `screencast`; tests and
    /// `initialize` use this constructor.
    pub fn with_services(
        frontend: FrontendTracker,
        shortcuts: SharedRegistry,
        chooser: SharedChooser,
        screenshot: SharedScreenshot,
        screencast: SharedScreenCast,
    ) -> Self {
        Backend {
            frontend,
            shortcuts,
            chooser,
            screenshot,
            screencast,
        }
    }

    /// The frontend tracker behind this object.
    pub fn frontend_tracker(&self) -> &FrontendTracker {
        &self.frontend
    }

    /// The session registry shared with the GlobalShortcuts interface.
    pub fn shortcuts(&self) -> &SharedRegistry {
        &self.shortcuts
    }

    /// The request registry shared with the FileChooser interface.
    pub fn chooser(&self) -> &SharedChooser {
        &self.chooser
    }

    /// The request registry shared with the Screenshot interface.
    pub fn screenshot(&self) -> &SharedScreenshot {
        &self.screenshot
    }

    /// The session/picker registry shared with the ScreenCast interface.
    pub fn screencast(&self) -> &SharedScreenCast {
        &self.screencast
    }

    /// The backend's current status.
    pub fn status(&self) -> BackendStatus {
        BackendStatus::new(self.frontend.presence())
    }
}

#[interface(name = "org.dragonfruit.Portal1")]
impl Backend {
    /// The portal backend name used in `portals.conf` (`dragonfruit`).
    fn backend_name(&self) -> String {
        BACKEND_NAME.to_owned()
    }

    /// The standard backend well-known name.
    fn dbus_name(&self) -> String {
        DBUS_NAME.to_owned()
    }

    /// The standard backend object path.
    fn object_path(&self) -> String {
        DBUS_PATH.to_owned()
    }

    /// The backend build version.
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    /// The advertised `org.freedesktop.impl.portal.*` interfaces.
    fn interfaces(&self) -> Vec<String> {
        BACKEND_INTERFACES
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    /// `present` / `absent` / `unknown` — the frontend-presence label.
    fn frontend(&self) -> String {
        self.frontend.presence().label().to_owned()
    }

    /// Whether the `xdg-desktop-portal` frontend is on the bus.
    fn frontend_present(&self) -> bool {
        self.frontend.presence().is_present()
    }

    /// The frontend's unique name when present, else the empty string.
    fn frontend_owner(&self) -> String {
        self.frontend
            .presence()
            .owner()
            .unwrap_or_default()
            .to_owned()
    }

    /// The frontend appeared or left. Only the diagnostic interface emits
    /// this; the portal frontend never needs it.
    #[zbus(signal)]
    async fn frontend_changed(
        emitter: &SignalEmitter<'_>,
        present: bool,
        owner: &str,
    ) -> zbus::Result<()>;

    /// Diagnostic activation bridge (T-13.1b): raise the standard
    /// GlobalShortcuts `Activated` signal for a session/shortcut that the
    /// portal knows. Returns whether it was delivered.
    ///
    /// This is not part of the portal contract: the compositor's shortcut
    /// engine is the one arbiter, and a later task feeds it each bound
    /// shortcut and calls back here (the same in-process registry) when it
    /// fires. Tests and the diagnostic surface use this method directly.
    async fn activate_shortcut(
        &self,
        session_handle: &str,
        shortcut_id: &str,
        timestamp: u64,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<bool, zbus::fdo::Error> {
        self.emit_shortcut_signal(emitter, "Activated", session_handle, shortcut_id, timestamp)
            .await
    }

    /// The release half of [`Backend::activate_shortcut`], raising
    /// `Deactivated`.
    async fn deactivate_shortcut(
        &self,
        session_handle: &str,
        shortcut_id: &str,
        timestamp: u64,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<bool, zbus::fdo::Error> {
        self.emit_shortcut_signal(
            emitter,
            "Deactivated",
            session_handle,
            shortcut_id,
            timestamp,
        )
        .await
    }

    /// The `(handle, kind)` pairs of the FileChooser requests waiting for a
    /// presenter. Diagnostic only: the portal frontend never calls it.
    fn pending_file_choosers(&self) -> Vec<(String, String)> {
        chooser::lock(&self.chooser)
            .handles()
            .into_iter()
            .map(|(handle, kind)| (handle, kind.as_str().to_owned()))
            .collect()
    }

    /// Complete a waiting FileChooser request with a presenter's selection.
    /// Each selection is normalized to a canonical `file://` URI through
    /// files-core; a foreign scheme is discarded. Returns whether a request
    /// was waiting at `handle`.
    ///
    /// This is the T-13.2a presenter seam: the test client calls it, and the
    /// T-13.2b picker calls it once the user chooses.
    fn complete_file_chooser(
        &self,
        handle: &str,
        selections: Vec<String>,
    ) -> Result<bool, zbus::fdo::Error> {
        Ok(chooser::lock(&self.chooser)
            .complete(handle, &selections)
            .is_some())
    }

    /// Resolve a waiting FileChooser request as cancelled. Returns whether a
    /// request was waiting at `handle`.
    fn cancel_file_chooser(&self, handle: &str) -> Result<bool, zbus::fdo::Error> {
        Ok(chooser::lock(&self.chooser).cancel(handle).is_some())
    }

    /// The `(handle, mode)` pairs of the Screenshot requests waiting for a
    /// presenter. Diagnostic only: the portal frontend never calls it.
    fn pending_screenshots(&self) -> Vec<(String, String)> {
        screenshot::lock(&self.screenshot)
            .handles()
            .into_iter()
            .map(|(handle, mode)| (handle, mode.as_str().to_owned()))
            .collect()
    }

    /// Complete a waiting Screenshot request with the presenter's captured
    /// URI. The URI is normalized to a canonical `file://` URI through
    /// files-core; a foreign scheme yields an error response. Returns whether
    /// a request was waiting at `handle`.
    ///
    /// This is the T-13.3a presenter seam: the shell's selection overlay calls
    /// it, and the test client uses it to drive the blocking round trip.
    fn complete_screenshot(&self, handle: &str, uri: &str) -> Result<bool, zbus::fdo::Error> {
        Ok(screenshot::lock(&self.screenshot)
            .complete(handle, uri)
            .is_some())
    }

    /// Resolve a waiting Screenshot request as cancelled. Returns whether a
    /// request was waiting at `handle`.
    fn cancel_screenshot(&self, handle: &str) -> Result<bool, zbus::fdo::Error> {
        Ok(screenshot::lock(&self.screenshot).cancel(handle).is_some())
    }

    /// The `(request handle, session handle)` pairs of the ScreenCast picker
    /// requests waiting for a presenter. Diagnostic only.
    fn pending_screen_casts(&self) -> Vec<(String, String)> {
        screencast::lock(&self.screencast).handles()
    }

    /// Complete a waiting ScreenCast picker request with the presenter's
    /// chosen sources, each `(source handle, source type bit)`. Returns
    /// whether a request was waiting at `handle`.
    ///
    /// This is the T-13.4a presenter seam: the shell's source picker calls it,
    /// and the test client uses it to drive the blocking round trip. An empty
    /// selection is an error response (never a silent success).
    fn complete_screen_cast(
        &self,
        handle: &str,
        selections: Vec<(String, u32)>,
    ) -> Result<bool, zbus::fdo::Error> {
        let selections: Vec<screencast::ScreenCastSelection> = selections
            .into_iter()
            .map(|(id, source_type)| screencast::ScreenCastSelection {
                id,
                source_type: screencast::SourceType::parse(source_type)
                    .unwrap_or(screencast::SourceType::Monitor),
            })
            .collect();
        Ok(screencast::lock(&self.screencast)
            .complete(handle, selections)
            .is_some())
    }

    /// Resolve a waiting ScreenCast picker request as cancelled. Returns
    /// whether a request was waiting at `handle`.
    fn cancel_screen_cast(&self, handle: &str) -> Result<bool, zbus::fdo::Error> {
        Ok(screencast::lock(&self.screencast).cancel(handle).is_some())
    }

    /// List a local directory through files-core for the picker. Each row is
    /// `(name, uri, is-directory, size)`. Diagnostic only.
    fn list_directory(
        &self,
        uri: &str,
    ) -> Result<Vec<(String, String, bool, u64)>, zbus::fdo::Error> {
        crate::chooser::list_directory(uri)
            .map(|listing| {
                listing
                    .entries
                    .into_iter()
                    .map(|entry| {
                        (
                            entry.name,
                            entry.uri,
                            entry.directory,
                            entry.size.unwrap_or(0),
                        )
                    })
                    .collect()
            })
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    /// A FileChooser request is waiting for a presenter. Diagnostic only; the
    /// standard interface emits no such signal.
    #[zbus(signal)]
    async fn file_chooser_opened(
        emitter: &SignalEmitter<'_>,
        handle: &str,
        kind: &str,
        app_id: &str,
        parent_window: &str,
        title: &str,
        options: std::collections::HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;

    /// A Screenshot request is waiting for a presenter. Diagnostic only; the
    /// standard interface emits no such signal.
    #[zbus(signal)]
    async fn screenshot_opened(
        emitter: &SignalEmitter<'_>,
        handle: &str,
        mode: &str,
        app_id: &str,
        parent_window: &str,
        options: std::collections::HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;

    /// A ScreenCast picker request is waiting for a presenter. Diagnostic
    /// only; the standard interface emits no such signal. The full options
    /// (including `cursor_mode`) travel in `options`.
    #[zbus(signal)]
    async fn screen_cast_opened(
        emitter: &SignalEmitter<'_>,
        handle: &str,
        session_handle: &str,
        app_id: &str,
        types: u32,
        multiple: bool,
        options: std::collections::HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

impl Backend {
    /// Emit one of the GlobalShortcuts activation signals when the registry
    /// knows the session and shortcut. The signal is emitted on the standard
    /// interface even though the method lives on the diagnostic one.
    async fn emit_shortcut_signal(
        &self,
        emitter: SignalEmitter<'_>,
        member: &str,
        session_handle: &str,
        shortcut_id: &str,
        timestamp: u64,
    ) -> Result<bool, zbus::fdo::Error> {
        if !shortcuts::lock(&self.shortcuts).has_shortcut(session_handle, shortcut_id) {
            return Ok(false);
        }
        let Ok(path) = ObjectPath::try_from(session_handle) else {
            return Ok(false);
        };
        emitter
            .connection()
            .emit_signal(
                None::<&str>,
                DBUS_PATH,
                GLOBAL_SHORTCUTS_INTERFACE,
                member,
                &(
                    path,
                    shortcut_id,
                    timestamp,
                    std::collections::HashMap::<String, OwnedValue>::new(),
                ),
            )
            .await?;
        Ok(true)
    }
}

/// Ask the bus whether `name` currently has an owner. A bus error is not a
/// fatal condition: the caller maps it to [`FrontendPresence::Unknown`].
pub fn name_has_owner(connection: &connection::Connection, name: &str) -> zbus::Result<bool> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(name,),
    )?;
    reply.body().deserialize::<bool>()
}

/// The unique name that owns `name`, if any.
pub fn name_owner(connection: &connection::Connection, name: &str) -> zbus::Result<String> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "GetNameOwner",
        &(name,),
    )?;
    reply.body().deserialize::<String>()
}

/// One synchronous probe of the frontend's presence. Never fails: an
/// unreachable bus is [`FrontendPresence::Unknown`], and an unowned name is
/// [`FrontendPresence::Absent`].
pub fn probe_frontend(connection: &connection::Connection) -> FrontendPresence {
    match name_has_owner(connection, FRONTEND_NAME) {
        Ok(true) => FrontendPresence::Present {
            owner: name_owner(connection, FRONTEND_NAME).unwrap_or_default(),
        },
        Ok(false) => FrontendPresence::Absent,
        Err(_) => FrontendPresence::Unknown,
    }
}

/// Serve the diagnostic [`STATUS_INTERFACE`] plus the concrete T-13.1b portal
/// interfaces (`Settings`, `GlobalShortcuts`) at [`DBUS_PATH`], then start the
/// live frontend and settings watches. Call this after the connection owns
/// [`DBUS_NAME`]. The connection must outlive the returned object; the watch
/// threads hold their own clones.
pub fn initialize(connection: &connection::Connection) -> Backend {
    let frontend = FrontendTracker::new();
    frontend.set(probe_frontend(connection));
    let registry = shortcuts::registry();
    let chooser = chooser::registry();
    let screenshot = screenshot::registry();
    let screencast = screencast::registry();
    let backend = Backend::with_services(
        frontend.clone(),
        registry.clone(),
        chooser.clone(),
        screenshot.clone(),
        screencast.clone(),
    );

    if let Err(error) = connection.object_server().at(DBUS_PATH, backend.clone()) {
        eprintln!("xdg-desktop-portal-dragonfruit: cannot serve {STATUS_INTERFACE} at {DBUS_PATH}: {error}");
    }

    let store = crate::settings::store();
    if let Err(error) = connection
        .object_server()
        .at(DBUS_PATH, SettingsPortal::new(store.clone()))
    {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {SETTINGS_INTERFACE} at {DBUS_PATH}: \
             {error}"
        );
    }
    if let Err(error) = connection
        .object_server()
        .at(DBUS_PATH, GlobalShortcuts::new(registry))
    {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {GLOBAL_SHORTCUTS_INTERFACE} at \
             {DBUS_PATH}: {error}"
        );
    }
    if let Err(error) = connection
        .object_server()
        .at(DBUS_PATH, FileChooserPortal::new(chooser))
    {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {FILE_CHOOSER_INTERFACE} at \
             {DBUS_PATH}: {error}"
        );
    }
    if let Err(error) = connection
        .object_server()
        .at(DBUS_PATH, ScreenshotPortal::new(screenshot))
    {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {SCREENSHOT_INTERFACE} at \
             {DBUS_PATH}: {error}"
        );
    }
    if let Err(error) = connection
        .object_server()
        .at(DBUS_PATH, ScreenCastPortal::new(screencast))
    {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {SCREENCAST_INTERFACE} at \
             {DBUS_PATH}: {error}"
        );
    }

    spawn_frontend_watch(connection.clone(), frontend);
    crate::settings::spawn_settings_sync(connection.clone(), store);
    backend
}

/// Watch `org.freedesktop.DBus.NameOwnerChanged` for the frontend name and
/// keep `tracker` live, emitting [`STATUS_INTERFACE`]'s `FrontendChanged`
/// each time the state flips.
pub fn spawn_frontend_watch(connection: connection::Connection, tracker: FrontendTracker) {
    let _ = std::thread::Builder::new()
        .name("dragonfruit-portal-frontend".to_owned())
        .spawn(move || frontend_watch_loop(connection, tracker));
}

fn frontend_watch_loop(connection: connection::Connection, tracker: FrontendTracker) {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.DBus")
        .expect("valid sender")
        .interface("org.freedesktop.DBus")
        .expect("valid interface")
        .member("NameOwnerChanged")
        .expect("valid member")
        .add_arg(FRONTEND_NAME)
        .expect("valid arg")
        .build();

    let iterator = match zbus::blocking::MessageIterator::for_match_rule(rule, &connection, Some(4))
    {
        Ok(iterator) => iterator,
        Err(_) => return,
    };

    for message in iterator {
        if message.is_err() {
            break;
        }
        let presence = probe_frontend(&connection);
        if !tracker.set(presence.clone()) {
            continue;
        }
        let present = presence.is_present();
        let owner = presence.owner().unwrap_or_default();
        let _ = connection.emit_signal(
            None::<&str>,
            DBUS_PATH,
            STATUS_INTERFACE,
            "FrontendChanged",
            &(present, owner),
        );
    }
}

/// Build a service connection on `address` (a session bus, or a private bus
/// when given), own [`DBUS_NAME`], and serve the diagnostic object with the
/// live frontend watch. Returns the connection and the backend object; the
/// caller keeps the connection alive for as long as it wants to serve.
///
/// A missing frontend never makes this fail; only an unreachable bus or an
/// unavailable name does.
pub fn serve(address: Option<&str>) -> zbus::Result<(connection::Connection, Backend)> {
    let builder = match address {
        Some(address) => connection::Builder::address(address)?,
        None => connection::Builder::session()?,
    };
    let connection = builder.name(DBUS_NAME)?.build()?;
    let backend = initialize(&connection);
    Ok((connection, backend))
}

/// Serve the backend on the session bus until the process is asked to stop.
/// Returns an error only when the bus or the well-known name cannot be
/// obtained; a session with no frontend is not an error.
pub fn run() -> zbus::Result<()> {
    let (_connection, _backend) = serve(None)?;

    // The object server runs on its own executor; parking the main thread
    // keeps the process (and the name) alive without a poll loop.
    loop {
        std::thread::park();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_served_identity_follows_the_backend_contract() {
        let backend = Backend::new(FrontendTracker::new());
        assert_eq!(backend.backend_name(), "dragonfruit");
        assert_eq!(backend.dbus_name(), DBUS_NAME);
        assert_eq!(backend.object_path(), DBUS_PATH);
        assert_eq!(backend.frontend(), "unknown");
        assert!(!backend.frontend_present());
        assert_eq!(backend.frontend_owner(), "");
        assert!(df_ipc::is_valid_dbus_name(STATUS_INTERFACE));
    }
}
