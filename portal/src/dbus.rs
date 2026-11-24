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

use std::time::Duration;

use zbus::blocking::connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedValue};

use crate::chooser::{self, SharedChooser};
use crate::interfaces::{
    FileChooserPortal, GlobalShortcuts, ScreenCastPortal, ScreenshotPortal, SettingsPortal,
    FILE_CHOOSER_INTERFACE, SCREENCAST_INTERFACE, SCREENSHOT_INTERFACE, SETTINGS_INTERFACE,
};
use crate::liveness;
use crate::model::{
    BackendStatus, FrontendPresence, FrontendTracker, BACKEND_INTERFACES, BACKEND_NAME, DBUS_NAME,
    DBUS_PATH, FRONTEND_NAME, PRESENTER_NAME, STATUS_INTERFACE,
};
use crate::screencast::{self, SharedScreenCast};
use crate::screenshot::{self, SharedScreenshot};
use crate::shortcuts::{self, SharedRegistry, GLOBAL_SHORTCUTS_INTERFACE};

/// The `org.dragonfruit.Portal1` object. It shares the standard portal path
/// with the concrete `org.freedesktop.impl.portal.*` interfaces.
#[derive(Clone)]
pub struct Backend {
    frontend: FrontendTracker,
    presenter: FrontendTracker,
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
            presenter: FrontendTracker::new(),
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
            presenter: FrontendTracker::new(),
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
        Self::with_services_and_presenter(
            frontend,
            FrontendTracker::new(),
            shortcuts,
            chooser,
            screenshot,
            screencast,
        )
    }

    /// A diagnostic object sharing every served registry and the live
    /// presenter-presence cell (T311). [`initialize`] uses this so the
    /// diagnostic surface and the presenter watch read the same state.
    pub fn with_services_and_presenter(
        frontend: FrontendTracker,
        presenter: FrontendTracker,
        shortcuts: SharedRegistry,
        chooser: SharedChooser,
        screenshot: SharedScreenshot,
        screencast: SharedScreenCast,
    ) -> Self {
        Backend {
            frontend,
            presenter,
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

    /// The shell-presenter tracker behind this object.
    pub fn presenter_tracker(&self) -> &FrontendTracker {
        &self.presenter
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

    /// `present` / `absent` / `unknown` — the shell presenter's presence
    /// (T311). Diagnostic only; the portal frontend never calls it.
    fn presenter(&self) -> String {
        self.presenter.presence().label().to_owned()
    }

    /// Whether the shell presenter owns [`PRESENTER_NAME`].
    fn presenter_present(&self) -> bool {
        self.presenter.presence().is_present()
    }

    /// The shell presenter's unique name when present, else the empty string.
    fn presenter_owner(&self) -> String {
        self.presenter
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

    /// How the ScreenCast transport delivers a source: `pipewire` when a live
    /// producer is available, `stills` when only the named fallback is. The
    /// shell reads it so the picker can say the fallback out loud before a
    /// source is chosen. Diagnostic only; the portal frontend never calls it.
    ///
    /// `pipewire` names the transport's *capability*, not a guarantee: whether
    /// a given stream is live or the named fallback is decided per `Start` and
    /// carried in each stream's `df_stream_mode`/`df_fallback` properties
    /// (T335b). The shipped daemon serves the stills transport, so it reports
    /// `stills`; the live client half is a documented gap
    /// ([ADR 0345](../../docs/design/adr/0345-screencast-bridge-and-portal-ownership.md)).
    fn screen_cast_stream_mode(&self) -> String {
        screencast::lock(&self.screencast)
            .stream_mode()
            .as_str()
            .to_owned()
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

    /// The live screen-share nodes one session holds, as `(node id, source
    /// id)` pairs. Diagnostic only: the shell bridge uses it to reconcile a
    /// restarted compositor's streams (T335b). Sessions are addressed by the
    /// frontend's session handle path.
    #[zbus(name = "ScreenCastLiveNodes")]
    fn screen_cast_live_nodes(&self, session_handle: &str) -> Vec<(u32, String)> {
        screencast::lock(&self.screencast).live_streams(session_handle)
    }

    /// The number of live screen-share nodes across every session. Diagnostic
    /// only; the T335b release test reads it to prove nothing leaks.
    #[zbus(name = "ScreenCastLiveStreams")]
    fn screen_cast_live_streams(&self) -> u32 {
        screencast::lock(&self.screencast).live_node_count() as u32
    }

    /// Release every live screen-share node and answer every waiting picker
    /// as cancelled. This is what the presenter-loss watch does when the
    /// trusted shell leaves; the shell bridge calls it on shutdown so a
    /// reconnecting shell never adopts a stale node (T335b). Returns the
    /// number of streams released.
    #[zbus(name = "ReleaseScreenCasts")]
    fn release_screen_casts(&self) -> u32 {
        screencast::lock(&self.screencast).release_streams().0 as u32
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

/// One synchronous probe of a well-known name's presence. Never fails: an
/// unreachable bus is [`FrontendPresence::Unknown`], and an unowned name is
/// [`FrontendPresence::Absent`].
fn probe_name(connection: &connection::Connection, name: &str) -> FrontendPresence {
    match name_has_owner(connection, name) {
        Ok(true) => FrontendPresence::Present {
            owner: name_owner(connection, name).unwrap_or_default(),
        },
        Ok(false) => FrontendPresence::Absent,
        Err(_) => FrontendPresence::Unknown,
    }
}

/// One synchronous probe of the frontend's presence.
pub fn probe_frontend(connection: &connection::Connection) -> FrontendPresence {
    probe_name(connection, FRONTEND_NAME)
}

/// One synchronous probe of the shell presenter's presence (T311).
pub fn probe_presenter(connection: &connection::Connection) -> FrontendPresence {
    probe_name(connection, PRESENTER_NAME)
}

/// Serve the diagnostic [`STATUS_INTERFACE`] plus the concrete T-13.1b portal
/// interfaces (`Settings`, `GlobalShortcuts`) at [`DBUS_PATH`], then start the
/// live frontend, presenter, and settings watches. Call this after the
/// connection owns [`DBUS_NAME`]. The connection must outlive the returned
/// object; the watch threads hold their own clones.
///
/// Interactive completions use [`liveness::COMPLETION_TIMEOUT`].
pub fn initialize(connection: &connection::Connection) -> Backend {
    initialize_with_timeout(connection, liveness::COMPLETION_TIMEOUT)
}

/// [`initialize`] with an explicit completion deadline. Tests use a short
/// bound to prove the timeout path without waiting 30 seconds.
pub fn initialize_with_timeout(connection: &connection::Connection, timeout: Duration) -> Backend {
    initialize_with_screencast(connection, timeout, screencast::registry())
}

/// [`initialize_with_timeout`] over a caller-supplied ScreenCast registry, so
/// a test (or a future host with a live transport) can serve the same objects
/// over an injected stream transport (T335b).
pub fn initialize_with_screencast(
    connection: &connection::Connection,
    timeout: Duration,
    screencast: SharedScreenCast,
) -> Backend {
    let frontend = FrontendTracker::new();
    frontend.set(probe_frontend(connection));
    let presenter = FrontendTracker::new();
    presenter.set(probe_presenter(connection));
    let registry = shortcuts::registry();
    let chooser = chooser::registry();
    let screenshot = screenshot::registry();
    let backend = Backend::with_services_and_presenter(
        frontend.clone(),
        presenter.clone(),
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
    if let Err(error) = connection.object_server().at(
        DBUS_PATH,
        FileChooserPortal::with_timeout(chooser.clone(), timeout),
    ) {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {FILE_CHOOSER_INTERFACE} at \
             {DBUS_PATH}: {error}"
        );
    }
    if let Err(error) = connection.object_server().at(
        DBUS_PATH,
        ScreenshotPortal::with_timeout(screenshot.clone(), timeout),
    ) {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {SCREENSHOT_INTERFACE} at \
             {DBUS_PATH}: {error}"
        );
    }
    if let Err(error) = connection.object_server().at(
        DBUS_PATH,
        ScreenCastPortal::with_timeout(screencast.clone(), timeout),
    ) {
        eprintln!(
            "xdg-desktop-portal-dragonfruit: cannot serve {SCREENCAST_INTERFACE} at \
             {DBUS_PATH}: {error}"
        );
    }

    spawn_frontend_watch(connection.clone(), frontend);
    spawn_presenter_watch(
        connection.clone(),
        presenter,
        chooser,
        screenshot,
        screencast,
    );
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

/// Watch the shell presenter's well-known name and keep `tracker` live.
///
/// When the presenter's owner disappears — a crash, an exit, or a restart that
/// transfers the name to a new connection — every pending FileChooser,
/// Screenshot, and ScreenCast picker request is resolved as `cancelled`, and
/// every live ScreenCast stream is released (the shell is the only party that
/// can create a node, T335b). That is the T311 rule: a caller never waits for
/// a presenter that is gone.
pub fn spawn_presenter_watch(
    connection: connection::Connection,
    tracker: FrontendTracker,
    chooser: SharedChooser,
    screenshot: SharedScreenshot,
    screencast: SharedScreenCast,
) {
    let _ = std::thread::Builder::new()
        .name("dragonfruit-portal-presenter".to_owned())
        .spawn(move || presenter_watch_loop(connection, tracker, chooser, screenshot, screencast));
}

fn presenter_watch_loop(
    connection: connection::Connection,
    tracker: FrontendTracker,
    chooser: SharedChooser,
    screenshot: SharedScreenshot,
    screencast: SharedScreenCast,
) {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.DBus")
        .expect("valid sender")
        .interface("org.freedesktop.DBus")
        .expect("valid interface")
        .member("NameOwnerChanged")
        .expect("valid member")
        .add_arg(PRESENTER_NAME)
        .expect("valid arg")
        .build();

    let iterator = match zbus::blocking::MessageIterator::for_match_rule(rule, &connection, Some(4))
    {
        Ok(iterator) => iterator,
        Err(_) => return,
    };

    for message in iterator {
        let Ok(message) = message else { break };
        // `(name, old owner, new owner)`: any non-empty old owner that is
        // replaced (including by a new connection with the same name) is a
        // presenter loss. A name appearing for the first time is not.
        let lost = message
            .body()
            .deserialize::<(String, String, String)>()
            .map(|(_, old, new)| !old.is_empty() && old != new)
            .unwrap_or(false);
        if lost {
            let screencast_cancelled = {
                let mut registry = screencast::lock(&screencast);
                let (released, cancelled) = registry.release_streams();
                if released > 0 {
                    eprintln!(
                        "xdg-desktop-portal-dragonfruit: presenter left; released {released} \
                         live screen cast stream(s)"
                    );
                }
                cancelled
            };
            let cancelled = chooser::lock(&chooser).cancel_all()
                + screenshot::lock(&screenshot).cancel_all()
                + screencast_cancelled;
            if cancelled > 0 {
                eprintln!(
                    "xdg-desktop-portal-dragonfruit: presenter left; cancelled {cancelled} \
                     pending request(s)"
                );
            }
        }
        tracker.set(probe_presenter(&connection));
    }
}

/// Build a service connection on `address` (a session bus, or a private bus
/// when given), own [`DBUS_NAME`], and serve the diagnostic object with the
/// live frontend and presenter watches. Returns the connection and the backend
/// object; the caller keeps the connection alive for as long as it wants to
/// serve.
///
/// A missing frontend never makes this fail; only an unreachable bus or an
/// unavailable name does. Interactive completions use
/// [`liveness::COMPLETION_TIMEOUT`].
pub fn serve(address: Option<&str>) -> zbus::Result<(connection::Connection, Backend)> {
    serve_with_timeout(address, liveness::COMPLETION_TIMEOUT)
}

/// [`serve`] with an explicit completion deadline (tests use a short bound).
pub fn serve_with_timeout(
    address: Option<&str>,
    timeout: Duration,
) -> zbus::Result<(connection::Connection, Backend)> {
    let builder = match address {
        Some(address) => connection::Builder::address(address)?,
        None => connection::Builder::session()?,
    };
    let connection = builder.name(DBUS_NAME)?.build()?;
    let backend = initialize_with_timeout(&connection, timeout);
    Ok((connection, backend))
}

/// [`serve_with_timeout`] over a caller-supplied ScreenCast registry (T335b):
/// the same served objects and watches, so a live transport can be exercised
/// through the real D-Bus code path.
pub fn serve_with_screencast(
    address: Option<&str>,
    timeout: Duration,
    screencast: SharedScreenCast,
) -> zbus::Result<(connection::Connection, Backend)> {
    let builder = match address {
        Some(address) => connection::Builder::address(address)?,
        None => connection::Builder::session()?,
    };
    let connection = builder.name(DBUS_NAME)?.build()?;
    let backend = initialize_with_screencast(&connection, timeout, screencast);
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
