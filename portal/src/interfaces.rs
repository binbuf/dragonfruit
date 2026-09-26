// SPDX-License-Identifier: MIT
//! The concrete portal interfaces this backend serves (T-13.1b).
//!
//! Two standard interfaces land here, both at
//! [`DBUS_PATH`](crate::model::DBUS_PATH):
//!
//! * [`SettingsPortal`] — `org.freedesktop.impl.portal.Settings`, read-only,
//!   projecting [`crate::settings`] onto the appearance and desktop
//!   namespaces.
//! * [`GlobalShortcuts`] — `org.freedesktop.impl.portal.GlobalShortcuts`,
//!   backed by [`crate::shortcuts::ShortcutRegistry`], with one
//!   [`ShortcutSessionObject`] per created session.
//!
//! The D-Bus types are the wire contract; the decisions live in the pure
//! modules so they are unit-testable without a bus.

use std::collections::HashMap;

use zbus::interface;
use zbus::object_server::{ObjectServer, SignalEmitter};
use zbus::zvariant::{ObjectPath, OwnedValue};

use crate::chooser::{
    self, ChooserKind, ChooserRequest, ChooserResponse, SharedChooser, FILE_CHOOSER_VERSION,
};
use crate::settings::{self, SettingsStore, SETTINGS_VERSION};
use crate::shortcuts::{
    lock as lock_registry, PortalShortcut, SessionError, SharedRegistry, CLOSED_BY_USER,
    GLOBAL_SHORTCUTS_VERSION, RESPONSE_SUCCESS,
};

/// The standard Settings backend interface name.
pub const SETTINGS_INTERFACE: &str = "org.freedesktop.impl.portal.Settings";

/// The standard FileChooser backend interface name.
pub const FILE_CHOOSER_INTERFACE: &str = crate::chooser::FILE_CHOOSER_INTERFACE;

/// The backend's read-only Settings object.
#[derive(Clone)]
pub struct SettingsPortal {
    store: SettingsStore,
}

impl SettingsPortal {
    /// A Settings object over a live projection.
    pub fn new(store: SettingsStore) -> Self {
        SettingsPortal { store }
    }

    /// The projection behind this object.
    pub fn store(&self) -> &SettingsStore {
        &self.store
    }
}

#[interface(name = "org.freedesktop.impl.portal.Settings")]
impl SettingsPortal {
    /// One key from one namespace, as a variant.
    fn read(&self, namespace: &str, key: &str) -> Result<OwnedValue, zbus::fdo::Error> {
        settings::lock(&self.store)
            .read(namespace, key)
            .cloned()
            .ok_or_else(|| {
                zbus::fdo::Error::InvalidArgs(format!("settings key {namespace}/{key} is not set"))
            })
    }

    /// Every key of one namespace (`a{sa{sv}}`); an empty namespace returns
    /// every namespace.
    fn read_all(&self, namespace: &str) -> HashMap<String, HashMap<String, OwnedValue>> {
        settings::lock(&self.store)
            .read_all(namespace)
            .into_iter()
            .map(|(namespace, keys)| (namespace, keys.into_iter().collect()))
            .collect()
    }

    /// The backend's Settings interface version.
    #[zbus(property(emits_changed_signal = "const"))]
    fn version(&self) -> u32 {
        SETTINGS_VERSION
    }

    /// A projected value changed. The only notification; callers never poll.
    #[zbus(signal)]
    async fn setting_changed(
        emitter: &SignalEmitter<'_>,
        namespace: &str,
        key: &str,
        value: OwnedValue,
    ) -> zbus::Result<()>;
}

/// The backend's FileChooser object (T-13.2a).
///
/// The standard interface's methods do not return until a presenter has
/// chosen: each call registers a request in [`crate::chooser::ChooserRegistry`]
/// and awaits the one-shot completion. T-13.2a ships the portal and the
/// presenter seam; the design-system picker that completes a request is
/// T-13.2b. The URI/options logic and the listing live in the pure
/// [`crate::chooser`] module.
#[derive(Clone)]
pub struct FileChooserPortal {
    chooser: SharedChooser,
}

impl FileChooserPortal {
    /// A FileChooser object over a live request registry.
    pub fn new(chooser: SharedChooser) -> Self {
        FileChooserPortal { chooser }
    }

    /// The registry behind this object.
    pub fn registry(&self) -> &SharedChooser {
        &self.chooser
    }

    /// Register one request, tell the presenter it is waiting, and await the
    /// chosen response.
    async fn request(
        &self,
        request: ChooserRequest,
        emitter: SignalEmitter<'_>,
    ) -> Result<(u32, HashMap<String, OwnedValue>), zbus::fdo::Error> {
        let completion = {
            let mut registry = chooser::lock(&self.chooser);
            registry
                .begin(request.clone())
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
        };

        // The diagnostic `FileChooserOpened` signal is how a presenter sees a
        // waiting request; the standard interface's caller never sees it.
        emitter
            .connection()
            .emit_signal(
                None::<&str>,
                crate::model::DBUS_PATH,
                crate::model::STATUS_INTERFACE,
                "FileChooserOpened",
                &(
                    request.handle.as_str(),
                    request.kind.as_str(),
                    request.app_id.as_str(),
                    request.parent_window.as_str(),
                    request.title.as_str(),
                    request.raw_options.clone(),
                ),
            )
            .await?;

        let response = completion.await.unwrap_or_else(ChooserResponse::cancelled);
        Ok((response.response, response.results))
    }
}

#[interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl FileChooserPortal {
    /// Present a chooser to open one or more files (or folders).
    async fn open_file(
        &self,
        handle: ObjectPath<'_>,
        app_id: &str,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u32, HashMap<String, OwnedValue>), zbus::fdo::Error> {
        let request = ChooserRequest::new(
            ChooserKind::OpenFile,
            handle.as_str(),
            app_id,
            parent_window,
            title,
            &options,
        );
        self.request(request, emitter).await
    }

    /// Present a chooser to pick a save location for one file.
    async fn save_file(
        &self,
        handle: ObjectPath<'_>,
        app_id: &str,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u32, HashMap<String, OwnedValue>), zbus::fdo::Error> {
        let request = ChooserRequest::new(
            ChooserKind::SaveFile,
            handle.as_str(),
            app_id,
            parent_window,
            title,
            &options,
        );
        self.request(request, emitter).await
    }

    /// Present a chooser to pick a folder for several named files.
    async fn save_files(
        &self,
        handle: ObjectPath<'_>,
        app_id: &str,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u32, HashMap<String, OwnedValue>), zbus::fdo::Error> {
        let request = ChooserRequest::new(
            ChooserKind::SaveFiles,
            handle.as_str(),
            app_id,
            parent_window,
            title,
            &options,
        );
        self.request(request, emitter).await
    }

    /// The backend's FileChooser interface version.
    #[zbus(property(emits_changed_signal = "const"))]
    fn version(&self) -> u32 {
        FILE_CHOOSER_VERSION
    }
}

/// The backend's GlobalShortcuts object.
#[derive(Clone)]
pub struct GlobalShortcuts {
    registry: SharedRegistry,
}

impl GlobalShortcuts {
    /// A GlobalShortcuts object over a live session registry.
    pub fn new(registry: SharedRegistry) -> Self {
        GlobalShortcuts { registry }
    }

    /// The session registry behind this object.
    pub fn registry(&self) -> &SharedRegistry {
        &self.registry
    }

    /// Map a caller-supplied session handle to an object path.
    fn session_path(handle: &str) -> Result<ObjectPath<'_>, zbus::fdo::Error> {
        ObjectPath::try_from(handle)
            .map_err(|_| zbus::fdo::Error::InvalidArgs(format!("session handle {handle:?}")))
    }

    /// Project the registry's shortcuts onto the wire shape.
    fn to_wire(shortcuts: &[PortalShortcut]) -> Vec<(String, HashMap<String, OwnedValue>)> {
        shortcuts
            .iter()
            .map(|shortcut| (shortcut.id.clone(), shortcut.options.clone()))
            .collect()
    }
}

#[interface(name = "org.freedesktop.impl.portal.GlobalShortcuts")]
impl GlobalShortcuts {
    /// Create a shortcut session served at `session_handle`. The frontend
    /// passes a full object path; a duplicate is refused.
    async fn create_session(
        &self,
        _handle: &str,
        session_handle: &str,
        app_id: &str,
        _options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> Result<(u32, HashMap<String, OwnedValue>), zbus::fdo::Error> {
        let path = Self::session_path(session_handle)?;
        {
            let mut registry = lock_registry(&self.registry);
            registry
                .create(app_id, session_handle)
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
        }

        let object = ShortcutSessionObject::new(session_handle.to_owned(), self.registry.clone());
        server
            .at(path, object)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        let mut results = HashMap::new();
        results.insert(
            "session_handle".to_owned(),
            OwnedValue::from(Self::session_path(session_handle)?),
        );
        Ok((RESPONSE_SUCCESS, results))
    }

    /// Bind (or rebind) a session's shortcuts and return them. Emits
    /// `ShortcutsChanged` so the frontend stays in sync.
    async fn bind_shortcuts(
        &self,
        _handle: &str,
        session_handle: &str,
        shortcuts: Vec<(String, HashMap<String, OwnedValue>)>,
        _parent_window: &str,
        _options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u32, Vec<(String, HashMap<String, OwnedValue>)>), zbus::fdo::Error> {
        let entries: Vec<PortalShortcut> = shortcuts
            .into_iter()
            .map(|(id, options)| PortalShortcut { id, options })
            .collect();
        let bound = {
            let mut registry = lock_registry(&self.registry);
            registry
                .bind(session_handle, entries)
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
        };
        let body = Self::to_wire(&bound);
        GlobalShortcuts::shortcuts_changed(
            &emitter,
            Self::session_path(session_handle)?,
            body.clone(),
        )
        .await?;
        Ok((RESPONSE_SUCCESS, body))
    }

    /// The shortcuts a session has bound.
    async fn list_shortcuts(
        &self,
        _handle: &str,
        session_handle: &str,
        _options: HashMap<String, OwnedValue>,
    ) -> Result<(u32, Vec<(String, HashMap<String, OwnedValue>)>), zbus::fdo::Error> {
        let registry = lock_registry(&self.registry);
        let listed = registry
            .list(session_handle)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
        Ok((RESPONSE_SUCCESS, Self::to_wire(listed)))
    }

    /// The backend's GlobalShortcuts interface version.
    #[zbus(property(emits_changed_signal = "const"))]
    fn version(&self) -> u32 {
        GLOBAL_SHORTCUTS_VERSION
    }

    /// A bound shortcut fired while its application was focused.
    #[zbus(signal)]
    async fn activated(
        emitter: &SignalEmitter<'_>,
        session_handle: ObjectPath<'_>,
        shortcut_id: &str,
        timestamp: u64,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;

    /// A bound shortcut was released.
    #[zbus(signal)]
    async fn deactivated(
        emitter: &SignalEmitter<'_>,
        session_handle: ObjectPath<'_>,
        shortcut_id: &str,
        timestamp: u64,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;

    /// A session's bound shortcuts changed.
    #[zbus(signal)]
    async fn shortcuts_changed(
        emitter: &SignalEmitter<'_>,
        session_handle: ObjectPath<'_>,
        shortcuts: Vec<(String, HashMap<String, OwnedValue>)>,
    ) -> zbus::Result<()>;
}

/// One shortcut session, served at the caller's session path. It implements
/// the standard `org.freedesktop.impl.portal.Session` close contract.
#[derive(Clone)]
pub struct ShortcutSessionObject {
    handle: String,
    registry: SharedRegistry,
}

impl ShortcutSessionObject {
    /// A session object at `handle`, sharing `registry`.
    pub fn new(handle: String, registry: SharedRegistry) -> Self {
        ShortcutSessionObject { handle, registry }
    }

    /// The session path this object is served at.
    pub fn handle(&self) -> &str {
        &self.handle
    }
}

#[interface(name = "org.freedesktop.impl.portal.Session")]
impl ShortcutSessionObject {
    /// Close the session: drop its shortcuts and remove the object.
    async fn close(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), zbus::fdo::Error> {
        lock_registry(&self.registry).close(&self.handle);
        ShortcutSessionObject::closed(&emitter, CLOSED_BY_USER).await?;
        server
            .remove::<ShortcutSessionObject, _>(self.handle.as_str())
            .await?;
        Ok(())
    }

    /// The session closed. `reason` follows the portal's close codes.
    #[zbus(signal)]
    async fn closed(emitter: &SignalEmitter<'_>, reason: u32) -> zbus::Result<()>;
}

/// Translate a registry failure into a D-Bus error (used by the model tests
/// and kept beside the interface for a single spelling).
impl From<SessionError> for zbus::fdo::Error {
    fn from(error: SessionError) -> Self {
        zbus::fdo::Error::Failed(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings;
    use crate::shortcuts;

    #[test]
    fn the_interface_names_are_the_standard_ones() {
        assert_eq!(SETTINGS_INTERFACE, "org.freedesktop.impl.portal.Settings");
        assert_eq!(
            crate::shortcuts::GLOBAL_SHORTCUTS_INTERFACE,
            "org.freedesktop.impl.portal.GlobalShortcuts"
        );
        assert_eq!(
            crate::shortcuts::SESSION_INTERFACE,
            "org.freedesktop.impl.portal.Session"
        );
    }

    #[test]
    fn the_file_chooser_object_names_the_standard_interface_and_version() {
        assert_eq!(
            FILE_CHOOSER_INTERFACE,
            "org.freedesktop.impl.portal.FileChooser"
        );
        assert_eq!(FILE_CHOOSER_VERSION, 3);
    }

    #[test]
    fn a_file_chooser_object_shares_its_registry() {
        let shared = crate::chooser::registry();
        let portal = FileChooserPortal::new(shared.clone());
        let request = crate::chooser::ChooserRequest::new(
            crate::chooser::ChooserKind::OpenFile,
            "/req/1",
            "org.example.App",
            "",
            "Open",
            &HashMap::new(),
        );
        crate::chooser::lock(&shared).begin(request).expect("begin");
        assert_eq!(crate::chooser::lock(portal.registry()).len(), 1);
    }

    #[test]
    fn a_settings_object_reads_through_to_its_store() {
        let store = settings::store();
        let portal = SettingsPortal::new(store.clone());
        assert!(portal
            .read(settings::APPEARANCE_NAMESPACE, "color-scheme")
            .is_ok());
        assert!(portal.read(settings::APPEARANCE_NAMESPACE, "nope").is_err());
        // The store is the same cell the sync thread writes.
        settings::lock(&store).read_all(settings::APPEARANCE_NAMESPACE);
    }

    #[test]
    fn a_shortcut_session_object_wires_to_the_registry() {
        let shared = shortcuts::registry();
        shortcuts::lock(&shared)
            .create("app", "/s")
            .expect("create");
        let object = ShortcutSessionObject::new("/s".to_owned(), shared.clone());
        assert_eq!(object.handle(), "/s");
        lock_registry(&shared).close("/s");
        assert!(lock_registry(&shared).is_empty());
    }
}
