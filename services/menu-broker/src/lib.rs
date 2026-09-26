// SPDX-License-Identifier: MIT
//! `dragonfruit-menu-broker`: the global-menu broker (T-14.2a).
//!
//! A global menu is not a Wayland capability — an application has to export a
//! meaningful menu model ([06-global-menu.md](../../docs/design/06-global-menu.md)).
//! The broker resolves the focused application's model with a strict priority
//! order (our native API → DBusMenu/AppMenu → no exporter) and always projects
//! the fixed **application menu** with live Hide/Hide Others/Show All state.
//!
//! # What lives here
//!
//! * [`model`] — the pure resolution pipeline: parse a publisher's exported
//!   model, derive the hide verbs' live state from the window list, and resolve
//!   the focused app's menu. No D-Bus and no JSON I/O, so it is unit-tested
//!   directly against fixtures.
//! * [`accelerators`] — the focus-scoped accelerator table (T-14.2b): parse
//!   the shortcuts a published model carries, keep them per app, and resolve a
//!   chord against the focused app only, with system chords winning.
//! * [`dbus`] — the `org.dragonfruit.MenuBroker1` interface and its `run`.
//!
//! The system menu (the dragonfruit mark) stays shell/session-owned; the
//! broker owns the application menu and the exporting app's own menus.
//!
//! The native publication payload is the design-system entry shape frozen in
//! ADR [0041](../../docs/design/adr/0041-native-menu-model-publication-shape.md).

pub mod accelerators;
pub mod dbus;
pub mod model;

pub use accelerators::{Accelerator, AcceleratorTable, Chord, Dispatch, Mods};
pub use dbus::{MenuBroker1, DBUS_NAME, DBUS_PATH, INTERFACE};
pub use model::{
    default_app_name, fixed_application_menu, synthesized_application_menu, Broker, HideVerbs,
    PublishedModel, Tier, Visibility, WindowState, DESKTOP_APP_NAME,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_dbus_name_is_valid() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME), "{DBUS_NAME}");
    }
}
