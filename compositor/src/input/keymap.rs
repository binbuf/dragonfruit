// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! Keymap conventions (T-03): the Cmd/Option → Super/Alt mapping is fixed
//! once in [`df_ipc::keymap`] and consumed here by the xkb keymap, and by
//! [`super::shortcuts`] for binding resolution.
//!
//! The compositor owns the seat keymap; clients cannot substitute their own
//! ([02-compositor.md](../../.docs/design/02-compositor.md)). Changing the
//! role mapping in `df-ipc` changes it for the compositor, the shell, and
//! first-party apps at once.

use smithay::input::keyboard::xkb::{self, keysyms, Keysym};
use smithay::input::keyboard::{ModifiersState, XkbConfig};

pub use df_ipc::keymap::ModifierRole;

/// A raw xkb keysym value, matching xkbcommon's `KEY_*` constants. The
/// shortcut engine binds these rather than the `xkeysym` newtype so
/// bindings stay plain data.
pub type KeysymValue = u32;

/// The xkb configuration for the fixed Dragonfruit keymap.
///
/// Layout/model/variant are intentionally empty so the user's
/// `XKB_DEFAULT_*` environment is respected; the *role mapping* (which
/// logical modifier a key plays) is what is pinned.
pub fn xkb_config() -> XkbConfig<'static> {
    XkbConfig {
        rules: df_ipc::keymap::RULES,
        model: df_ipc::keymap::MODEL,
        layout: df_ipc::keymap::LAYOUT,
        variant: df_ipc::keymap::VARIANT,
        options: Some(df_ipc::keymap::OPTIONS.to_string()),
    }
}

/// Whether a logical role is currently held, per the fixed mapping.
pub fn role_held(role: ModifierRole, mods: &ModifiersState) -> bool {
    match role {
        ModifierRole::Command => mods.logo,
        ModifierRole::Option => mods.alt,
        ModifierRole::Control => mods.ctrl,
        ModifierRole::Shift => mods.shift,
    }
}

/// A small set of logical modifier roles, used to match shortcut bindings
/// without caring about caps/num lock or layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RoleMods(u8);

const COMMAND: u8 = 1 << 0;
const OPTION: u8 = 1 << 1;
const CONTROL: u8 = 1 << 2;
const SHIFT: u8 = 1 << 3;

impl RoleMods {
    /// No modifiers.
    pub const EMPTY: RoleMods = RoleMods(0);
    /// The Cmd/Super role.
    pub const COMMAND: RoleMods = RoleMods(COMMAND);
    /// The Option/Alt role.
    pub const OPTION: RoleMods = RoleMods(OPTION);
    /// The Control role.
    pub const CONTROL: RoleMods = RoleMods(CONTROL);
    /// The Shift role.
    pub const SHIFT: RoleMods = RoleMods(SHIFT);

    /// Combine two role sets.
    pub const fn union(self, other: RoleMods) -> RoleMods {
        RoleMods(self.0 | other.0)
    }

    /// Whether every role in `other` is set here.
    pub const fn contains(self, other: RoleMods) -> bool {
        self.0 & other.0 == other.0
    }

    /// The significant roles currently held (caps/num lock and layout are
    /// deliberately ignored).
    pub fn from_state(mods: &ModifiersState) -> RoleMods {
        let mut set = RoleMods::EMPTY;
        if mods.logo {
            set = set.union(RoleMods::COMMAND);
        }
        if mods.alt {
            set = set.union(RoleMods::OPTION);
        }
        if mods.ctrl {
            set = set.union(RoleMods::CONTROL);
        }
        if mods.shift {
            set = set.union(RoleMods::SHIFT);
        }
        set
    }

    /// A human-readable chord prefix for logs, e.g. `"Cmd+Shift+"`.
    pub fn label(self) -> String {
        let mut parts = Vec::new();
        for (bit, label) in [
            (COMMAND, "Cmd"),
            (OPTION, "Option"),
            (CONTROL, "Ctrl"),
            (SHIFT, "Shift"),
        ] {
            if self.0 & bit != 0 {
                parts.push(label);
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("{}+", parts.join("+"))
        }
    }
}

/// Parse a binding from a human-readable spec like `"Cmd+Shift+3"` or
/// `"Ctrl+Left"`. Used by tests and the eventual Settings recorder; the
/// canonical mapping still comes from [`df_ipc::keymap`].
pub fn parse_binding(spec: &str) -> Option<(RoleMods, KeysymValue)> {
    let mut mods = RoleMods::EMPTY;
    let mut parts = spec.split('+').peekable();
    let mut key = None;
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            key = Some(keysym_from_name(part)?);
            break;
        }
        match part.to_ascii_lowercase().as_str() {
            "cmd" | "super" | "logo" => mods = mods.union(RoleMods::COMMAND),
            "option" | "alt" => mods = mods.union(RoleMods::OPTION),
            "ctrl" | "control" => mods = mods.union(RoleMods::CONTROL),
            "shift" => mods = mods.union(RoleMods::SHIFT),
            _ => return None,
        }
    }
    Some((mods, key?))
}

/// Resolve a key name to a raw xkb keysym (`"Tab"`, `"Left"`, `"3"`,
/// `"F11"`).
pub fn keysym_from_name(name: &str) -> Option<KeysymValue> {
    let sym: Keysym = xkb::keysym_from_name(name, xkb::KEYSYM_NO_FLAGS);
    (sym.raw() != keysyms::KEY_NoSymbol).then_some(sym.raw())
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::input::keyboard::xkb::keysyms;

    #[test]
    fn cmd_is_super_and_option_is_alt() {
        assert_eq!(ModifierRole::Command.xkb_mod_name(), "Mod4");
        assert_eq!(ModifierRole::Option.xkb_mod_name(), "Mod1");
    }

    #[test]
    fn role_mods_ignore_caps_and_num_lock() {
        let mut mods = ModifiersState {
            logo: true,
            caps_lock: true,
            num_lock: true,
            ..Default::default()
        };
        assert_eq!(RoleMods::from_state(&mods), RoleMods::COMMAND);
        mods.shift = true;
        assert_eq!(
            RoleMods::from_state(&mods),
            RoleMods::COMMAND.union(RoleMods::SHIFT)
        );
    }

    #[test]
    fn parsing_bindings_uses_the_canonical_roles() {
        let (mods, key) = parse_binding("Cmd+Shift+3").unwrap();
        assert_eq!(mods, RoleMods::COMMAND.union(RoleMods::SHIFT));
        assert_eq!(key, keysyms::KEY_3);
        let (mods, key) = parse_binding("Ctrl+Left").unwrap();
        assert_eq!(mods, RoleMods::CONTROL);
        assert_eq!(key, keysyms::KEY_Left);
        assert!(parse_binding("Hyper+X").is_none());
    }
}
