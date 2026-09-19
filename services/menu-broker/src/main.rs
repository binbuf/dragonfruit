// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dragonfruit-menu-broker` — scaffold stub. The real global-menu broker
//! lands in T-22; this binary exists so the workspace, CI, packaging, and
//! licensing are wired from the first commit.

use df_ipc::LOCKSTEP_VERSION;

/// Planned well-known name on the user session bus
/// (docs/ipc-versioning.md).
pub const DBUS_NAME: &str = "org.dragonfruit.MenuBroker1";

fn main() {
    debug_assert!(df_ipc::is_valid_dbus_name(DBUS_NAME));
    println!(
        "dragonfruit-menu-broker: stub (lockstep-ipc=v{LOCKSTEP_VERSION}, planned D-Bus name {DBUS_NAME})"
    );
}

#[cfg(test)]
mod tests {
    use super::DBUS_NAME;
    use df_ipc::dbus_name;

    #[test]
    fn dbus_name_follows_policy() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME), "{DBUS_NAME}");
        assert_eq!(DBUS_NAME, dbus_name("MenuBroker", 1).to_string());
    }
}
