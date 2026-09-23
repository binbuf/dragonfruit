// SPDX-License-Identifier: MIT
//! Shared IPC contracts for the Dragonfruit lockstep set.
//!
//! Every process in the desktop — compositor, shell, services, apps —
//! embeds the constants in this crate. The values live in exactly one
//! place so the lockstep rule can be enforced mechanically:
//!
//! * [`DESKTOP_NAME`] is the public `XDG_CURRENT_DESKTOP` contract, chosen
//!   once (only `dragonfruit` is legal anywhere in the tree; enforced by
//!   `scripts/check-desktop-names.sh`).
//! * [`LOCKSTEP_VERSION`] is the version of the private-protocol set that
//!   compositor, shell, and protocol XMLs must agree on. Tests in this
//!   crate fail the build when an XML in `protocols/` disagrees.
//! * D-Bus interfaces follow `org.dragonfruit.*` names with a per-major
//!   version suffix ([`dbus_name`]).
//!
//! See docs/ipc-versioning.md for the full policy.

/// The desktop name — the public contract toolkits, portals, and
/// desktop files use to find Dragonfruit-specific behavior.
pub const DESKTOP_NAME: &str = "dragonfruit";

/// Version of the lockstep IPC set.
///
/// Compositor, shell, and private protocol XMLs ship as one set per
/// release; cross-version mixing is unsupported and detected at
/// handshake. Within a stable release the set is additive-only.
pub const LOCKSTEP_VERSION: u32 = 1;

/// Marker line that every protocol XML must carry, immediately binding the
/// file to [`LOCKSTEP_VERSION`].
pub const LOCKSTEP_MARKER: &str = "dragonfruit lockstep-version:";

/// The fixed keymap conventions (T-03).
///
/// The macOS-Cmd role maps to **Super/Mod4** and Option maps to **Alt**.
/// This mapping is chosen once, here, and consumed by the compositor's
/// shortcut engine, its xkb keymap, the menu-broker, and every first-party
/// accelerator — so applications cannot drift from the shell
/// ([02-compositor.md](../../docs/design/02-compositor.md#keymap-conventions)).
///
/// `xkb_mod_name` values match xkbcommon's `MOD_NAME_*` constants exactly;
/// `physical_name` is the user-facing key name used in menus and docs.
pub mod keymap {
    /// A logical modifier role, named the way users think about it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ModifierRole {
        /// macOS "Command" → Super/Mod4.
        Command,
        /// macOS "Option" → Alt.
        Option,
        /// Control.
        Control,
        /// Shift.
        Shift,
    }

    impl ModifierRole {
        /// All roles, for exhaustive iteration in tests and UI.
        pub const ALL: [ModifierRole; 4] = [
            ModifierRole::Command,
            ModifierRole::Option,
            ModifierRole::Control,
            ModifierRole::Shift,
        ];

        /// The xkbcommon modifier name this role resolves to
        /// (`xkbcommon::xkb::MOD_NAME_*`).
        pub const fn xkb_mod_name(self) -> &'static str {
            match self {
                ModifierRole::Command => "Mod4",
                ModifierRole::Option => "Mod1",
                ModifierRole::Control => "Control",
                ModifierRole::Shift => "Shift",
            }
        }

        /// The physical key name shown to users (macOS-style on the left,
        /// PC-style in parentheses where it differs).
        pub const fn physical_name(self) -> &'static str {
            match self {
                ModifierRole::Command => "Super",
                ModifierRole::Option => "Alt",
                ModifierRole::Control => "Control",
                ModifierRole::Shift => "Shift",
            }
        }

        /// The label used in menus and tooltips, macOS-style.
        pub const fn label(self) -> &'static str {
            match self {
                ModifierRole::Command => "Cmd",
                ModifierRole::Option => "Option",
                ModifierRole::Control => "Ctrl",
                ModifierRole::Shift => "Shift",
            }
        }
    }

    /// The Cmd role's physical xkb modifier name (`Mod4`).
    pub const COMMAND: &str = ModifierRole::Command.xkb_mod_name();
    /// The Option role's physical xkb modifier name (`Mod1`).
    pub const OPTION: &str = ModifierRole::Option.xkb_mod_name();

    /// Fixed xkb rules/model/layout/variant. Empty strings mean "respect
    /// the `XKB_DEFAULT_*` environment" (smithay's `XkbConfig` contract),
    /// so the keymap decision is about *roles*, not about forcing a
    /// layout on the user.
    pub const RULES: &str = "";
    pub const MODEL: &str = "";
    pub const LAYOUT: &str = "";
    pub const VARIANT: &str = "";
    /// Extra xkb options that are part of the fixed Dragonfruit keymap.
    pub const OPTIONS: &str = "";
}

#[cfg(test)]
mod keymap_tests {
    use super::keymap::{ModifierRole, COMMAND, OPTION};

    #[test]
    fn cmd_maps_to_super_and_option_to_alt() {
        assert_eq!(COMMAND, "Mod4");
        assert_eq!(OPTION, "Mod1");
        assert_eq!(ModifierRole::Command.physical_name(), "Super");
        assert_eq!(ModifierRole::Option.physical_name(), "Alt");
    }

    #[test]
    fn every_role_has_a_distinct_xkb_modifier() {
        let mut names: Vec<&str> = ModifierRole::ALL
            .iter()
            .map(|role| role.xkb_mod_name())
            .collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ModifierRole::ALL.len());
    }
}

/// Build a well-known D-Bus name: `org.dragonfruit.<Iface><Major>`.
///
/// Example: `dbus_name("Settings", 1)` is `org.dragonfruit.Settings1`.
pub const fn dbus_name<'a>(interface: &'a str, major: u32) -> DbusName<'a> {
    DbusName { interface, major }
}

/// A typed D-Bus well-known name (see [`dbus_name`]).
pub struct DbusName<'a> {
    interface: &'a str,
    major: u32,
}

impl std::fmt::Display for DbusName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "org.dragonfruit.{}{}", self.interface, self.major)
    }
}

/// Validate a D-Bus interface name against the Dragonfruit policy:
/// `org.dragonfruit.<PascalCase><Major>` where Major is a decimal suffix.
pub fn is_valid_dbus_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("org.dragonfruit.") else {
        return false;
    };
    let digits = rest.len() - rest.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return false;
    }
    let iface = &rest[..rest.len() - digits];
    if iface.is_empty() {
        return false;
    }
    let mut chars = iface.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_uppercase()) {
        return false;
    }
    iface.chars().all(|c| c.is_ascii_alphanumeric())
}

/// The lockstep handshake: reject cross-version mixing of the lockstep
/// set.
///
/// Every compositor↔shell (and service↔client) handshake begins by
/// comparing embedded lockstep versions. This returns the agreed version,
/// or an error the caller turns into a protocol error — never a crash.
pub fn assert_lockstep_compatible(peer_version: u32) -> Result<u32, LockstepMismatch> {
    if peer_version == LOCKSTEP_VERSION {
        Ok(LOCKSTEP_VERSION)
    } else {
        Err(LockstepMismatch {
            ours: LOCKSTEP_VERSION,
            theirs: peer_version,
        })
    }
}

/// A peer spoke a lockstep version that is not ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockstepMismatch {
    pub ours: u32,
    pub theirs: u32,
}

impl std::fmt::Display for LockstepMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "lockstep mismatch: peer speaks v{}, this build is v{} — \
             compositor, shell, and protocols must ship as one set \
             (docs/ipc-versioning.md)",
            self.theirs, self.ours
        )
    }
}

impl std::error::Error for LockstepMismatch {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_rejects_cross_version_mixing() {
        assert_eq!(
            assert_lockstep_compatible(LOCKSTEP_VERSION),
            Ok(LOCKSTEP_VERSION)
        );
        let err = assert_lockstep_compatible(LOCKSTEP_VERSION + 1).unwrap_err();
        assert_eq!(err.theirs, LOCKSTEP_VERSION + 1);
        assert_eq!(err.ours, LOCKSTEP_VERSION);
    }

    #[test]
    fn dbus_names_follow_the_policy() {
        assert!(is_valid_dbus_name("org.dragonfruit.Settings1"));
        assert!(is_valid_dbus_name("org.dragonfruit.MenuBroker2"));
        assert!(!is_valid_dbus_name("org.gnome.Settings1")); // df-allow-desktop-name: negative test
        assert!(!is_valid_dbus_name("org.dragonfruit.settings1"));
        assert!(!is_valid_dbus_name("org.dragonfruit.Settings"));
        assert!(!is_valid_dbus_name("org.dragonfruit.1"));
    }

    #[test]
    fn dbus_name_helper() {
        assert_eq!(
            dbus_name("Settings", 1).to_string(),
            "org.dragonfruit.Settings1"
        );
        assert_eq!(
            dbus_name("MenuBroker", 2).to_string(),
            "org.dragonfruit.MenuBroker2"
        );
        assert!(is_valid_dbus_name(
            dbus_name("Settings", 1).to_string().as_str()
        ));
    }

    #[test]
    fn desktop_name_is_dragonfruit() {
        assert_eq!(DESKTOP_NAME, "dragonfruit");
    }

    // --- lockstep enforcement over protocols/*.xml ----------------------------

    /// Every protocol XML must declare the lockstep version this build
    /// speaks, using the marker comment. This test is the mechanical
    /// enforcement of docs/ipc-versioning.md: it fails the build when
    /// compositor/shell code and protocol XMLs disagree.
    #[test]
    fn protocol_xmls_match_lockstep_version() {
        let protocol_dir =
            std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
        let mut checked = 0;
        let marker = format!("{LOCKSTEP_MARKER} {LOCKSTEP_VERSION}");
        for entry in std::fs::read_dir(&protocol_dir).expect("protocols/ must exist") {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "xml") {
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            assert!(
                text.contains("SPDX-License-Identifier: MIT"),
                "{}: protocol XMLs must be MIT so any third party may implement them (docs/licensing.md)",
                path.display()
            );
            assert!(
                text.contains(&marker),
                "{}: missing or stale {marker:?} (this build is lockstep v{LOCKSTEP_VERSION})",
                path.display()
            );
            assert!(
                text.contains("<interface"),
                "{}: a protocol XML must declare at least one interface",
                path.display()
            );
            checked += 1;
        }
        assert!(
            checked > 0,
            "no protocol XMLs found in {}",
            protocol_dir.display()
        );
    }
}
