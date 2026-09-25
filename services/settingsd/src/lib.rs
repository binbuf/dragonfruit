// SPDX-License-Identifier: MIT
//! `dragonfruit-settingsd`: the desktop-settings provider (T-08.1a).
//!
//! `settingsd` is the single owner of desktop settings
//! ([01-architecture.md](../../docs/design/01-architecture.md)): the
//! compositor, shell, and first-party apps read and write one set of named
//! keys over `org.dragonfruit.Settings1` and react to change signals instead
//! of polling.
//!
//! # What lives here
//!
//! * [`schema`] — the key table: every key's D-Bus type, default, range, and
//!   **owner/consumer** pair, in one place. The table is the contract;
//!   changes are additive-only within a release.
//! * [`model::Settings`] — the in-memory store, with schema validation on
//!   every write and change detection so a signal means a real change.
//! * [`persist`] — the on-disk owner: `$XDG_CONFIG_HOME/dragonfruit/settings.json`
//!   in the documented `{"schema":N,"keys":{…}}` shape, adopted from the
//!   shell's interim file, written atomically, and migrated at startup.
//! * [`dbus`] — the `org.dragonfruit.Settings1` interface (`Get`, `Set`,
//!   `GetAll`, `ListKeys`, `SchemaVersion`, `Changed`) and the session-bus
//!   host. A configured `Set` persists before its `Changed` signal.
//!
//! # No polling
//!
//! There is no timer and no read loop. A value changes only through `Set`,
//! which emits exactly one `Changed` signal; `GetAll` exists so a client
//! that appears (or reappears after a settingsd restart) can resync in one
//! call.

pub mod dbus;
pub mod model;
pub mod persist;
pub mod schema;
pub mod value;

pub use dbus::{Settings1, DBUS_NAME, DBUS_PATH};
pub use model::Settings;
pub use persist::Persistence;
pub use schema::{KeyGroup, KeySpec, KeyType, KEYS, SCHEMA_VERSION};
pub use value::{SettingsError, Value};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_well_known_name_follows_the_versioning_policy() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME), "{DBUS_NAME}");
        assert_eq!(DBUS_NAME, df_ipc::dbus_name("Settings", 1).to_string());
    }

    #[test]
    fn a_fresh_store_and_interface_share_the_schema_defaults() {
        let settings = Settings::new();
        assert_eq!(
            settings.get("workspaces.count").unwrap(),
            &Value::Integer(3)
        );
        let interface = Settings1::new(settings);
        let store = interface.store().lock().unwrap();
        assert_eq!(
            store.get("appearance.accent").unwrap(),
            &Value::Text(String::new())
        );
    }
}
