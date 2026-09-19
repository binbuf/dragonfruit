// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dragonfruit-app-index` — scaffold stub. The real application index
//! lands in T-23; this binary exists so the workspace, CI, packaging, and
//! licensing are wired from the first commit.

use df_ipc::LOCKSTEP_VERSION;

/// Planned well-known name on the user session bus
/// (docs/ipc-versioning.md).
pub const DBUS_NAME: &str = "org.dragonfruit.AppIndex1";

fn main() {
    debug_assert!(df_ipc::is_valid_dbus_name(DBUS_NAME));
    println!(
        "dragonfruit-app-index: stub (lockstep-ipc=v{LOCKSTEP_VERSION}, planned D-Bus name {DBUS_NAME})"
    );
}

#[cfg(test)]
mod tests {
    use super::DBUS_NAME;
    use df_ipc::dbus_name;

    #[test]
    fn dbus_name_follows_policy() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME), "{DBUS_NAME}");
        assert_eq!(DBUS_NAME, dbus_name("AppIndex", 1).to_string());
    }
}
