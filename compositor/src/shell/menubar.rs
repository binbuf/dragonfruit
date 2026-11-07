// SPDX-License-Identifier: MIT
//! Menu-bar auto-hide and per-output reveal (T201).
//!
//! `menu.autoHide` has three values and the compositor is the single owner of
//! the resulting visibility on each output:
//!
//! * `never` — visible on the desktop and over a fullscreen client;
//! * `always` — concealed until the pointer reaches the top edge or the
//!   menu-bar focus shortcut reveals it;
//! * `full-screen` — visible on the desktop, concealed over a fullscreen
//!   client until revealed on demand.
//!
//! A concealed bar is removed from the visible-chrome list, so it contributes
//! no reserved zone, input hit area, material region, or sampled/veil panel.
//! That is what keeps a fullscreen output eligible for direct scanout. A
//! revealed bar behaves exactly like the old pinned bar until it conceals.
//!
//! Reveal state is per output: revealing the bar on the display under the
//! pointer must not float it onto a second display. The menu bar itself is a
//! single all-output chrome surface, so the compositor filters that surface
//! per output from this state rather than creating a second surface.
//!
//! Active menu/focus interactions keep the bar revealed: the caller passes
//! `active` when the bar holds keyboard focus or its dropdown overlay is
//! mapped. Concealing clears the reveal state; the next pointer move at the
//! top edge (or the focus shortcut) reveals it again. Locking clears reveal so
//! an unlock starts from the policy's resting state.

use std::collections::HashSet;

/// The `menu.autoHide` policy, mirrored from the settings key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoHideMode {
    Never,
    Always,
    FullScreen,
}

impl Default for AutoHideMode {
    fn default() -> Self {
        // The settings schema default is `full-screen` and matches the
        // pre-T201 behavior: visible on the desktop, hidden over fullscreen.
        AutoHideMode::FullScreen
    }
}

impl AutoHideMode {
    /// Parse the wire value used by `df_layer_surface.set_auto_hide`.
    pub const fn from_wire(value: u32) -> Self {
        match value {
            0 => AutoHideMode::Never,
            1 => AutoHideMode::Always,
            _ => AutoHideMode::FullScreen,
        }
    }

    /// Stable name for diagnostics and logs.
    pub const fn name(self) -> &'static str {
        match self {
            AutoHideMode::Never => "never",
            AutoHideMode::Always => "always",
            AutoHideMode::FullScreen => "full-screen",
        }
    }
}

/// The top-edge band that reveals a concealed menu bar. This is the bar's
/// own logical height, so the pointer that enters the revealed strip is
/// already inside the bar's input region.
pub const MENU_BAR_REVEAL_HEIGHT: i32 = crate::design_tokens::component::menu_bar::HEIGHT as i32;

/// Whether the menu bar is visible for one output.
///
/// `fullscreen` is true when a settled fullscreen client covers the output.
/// `revealed` is the per-output transient reveal. `active` keeps the bar up
/// while a menu is open or the bar owns keyboard focus, even if the pointer
/// has left the top edge.
pub const fn menu_bar_visible(
    mode: AutoHideMode,
    fullscreen: bool,
    revealed: bool,
    active: bool,
) -> bool {
    match mode {
        AutoHideMode::Never => true,
        AutoHideMode::Always => revealed || active,
        AutoHideMode::FullScreen => !fullscreen || revealed || active,
    }
}

/// The compositor-owned reveal state for the one menu-bar surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBarVisibility {
    mode: AutoHideMode,
    revealed: HashSet<String>,
}

impl Default for MenuBarVisibility {
    fn default() -> Self {
        Self {
            mode: AutoHideMode::default(),
            revealed: HashSet::new(),
        }
    }
}

impl MenuBarVisibility {
    pub fn mode(&self) -> AutoHideMode {
        self.mode
    }

    /// Apply a new policy. A live policy change drops the old transient
    /// reveal; the next pointer/keyboard interaction applies the new mode.
    /// Returns whether anything changed.
    pub fn set_mode(&mut self, mode: AutoHideMode) -> bool {
        if self.mode == mode {
            return false;
        }
        self.mode = mode;
        self.revealed.clear();
        true
    }

    pub fn is_revealed(&self, output: &str) -> bool {
        self.revealed.contains(output)
    }

    /// Reveal the bar on one output. Returns whether the state changed.
    pub fn reveal(&mut self, output: &str) -> bool {
        self.revealed.insert(output.to_owned())
    }

    /// Conceal the bar on one output. Returns whether the state changed.
    pub fn conceal(&mut self, output: &str) -> bool {
        self.revealed.remove(output)
    }

    /// Drop every transient reveal (lock, or a policy change). Returns whether
    /// anything changed.
    pub fn clear(&mut self) -> bool {
        if self.revealed.is_empty() {
            return false;
        }
        self.revealed.clear();
        true
    }

    /// Whether the bar is visible on `output` under the current policy.
    pub fn visible_on(&self, output: &str, fullscreen: bool, active: bool) -> bool {
        menu_bar_visible(self.mode, fullscreen, self.is_revealed(output), active)
    }

    /// Apply the pointer position to the reveal state for one output.
    ///
    /// `local_y` is output-local logical. The bar reveals while the pointer is
    /// in the top band, or whenever an active menu/focus interaction holds it
    /// up. On the desktop a `full-screen`-mode bar is always visible, so its
    /// transient state is cleared: entering fullscreen then starts concealed.
    ///
    /// `pointer_seen` is false until the compositor has received a real
    /// pointer motion. The initial (0,0) seat position must not count as an
    /// edge reveal, or `always` would start visible on every session; an
    /// active keyboard focus can still reveal before the first motion.
    /// Returns whether the reveal state changed.
    pub fn update_pointer(
        &mut self,
        output: &str,
        local_y: f64,
        fullscreen: bool,
        active: bool,
        pointer_seen: bool,
    ) -> bool {
        let edge = pointer_seen && local_y < f64::from(MENU_BAR_REVEAL_HEIGHT);
        let should_reveal = match self.mode {
            AutoHideMode::Never => false,
            AutoHideMode::Always => edge || active,
            AutoHideMode::FullScreen => fullscreen && (edge || active),
        };
        if should_reveal {
            self.reveal(output)
        } else {
            self.conceal(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_is_visible_on_the_desktop_and_in_fullscreen() {
        for fullscreen in [false, true] {
            for revealed in [false, true] {
                for active in [false, true] {
                    assert!(menu_bar_visible(
                        AutoHideMode::Never,
                        fullscreen,
                        revealed,
                        active
                    ));
                }
            }
        }
    }

    #[test]
    fn always_starts_concealed_and_reveals_on_the_edge_or_focus() {
        let mut visibility = MenuBarVisibility::default();
        assert!(visibility.set_mode(AutoHideMode::Always));
        assert!(!visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.visible_on("eDP-1", true, false));

        // The pointer in the top band reveals only the owning output.
        assert!(visibility.update_pointer("eDP-1", 0.5, false, false, true));
        assert!(visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.visible_on("HDMI-A-1", false, false));

        // Moving below the bar conceals it again.
        assert!(visibility.update_pointer("eDP-1", 40.0, false, false, true));
        assert!(!visibility.visible_on("eDP-1", false, false));

        // An active menu/focus keeps it up even with the pointer elsewhere.
        assert!(visibility.update_pointer("eDP-1", 40.0, false, true, true));
        assert!(visibility.visible_on("eDP-1", true, true));
    }

    #[test]
    fn the_initial_pointer_position_does_not_count_as_an_edge_reveal() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_mode(AutoHideMode::Always);
        // The seat starts at (0,0); that is not a user edge gesture.
        assert!(!visibility.update_pointer("eDP-1", 0.0, false, false, false));
        assert!(!visibility.visible_on("eDP-1", false, false));
        // The first real motion at the top edge reveals.
        assert!(visibility.update_pointer("eDP-1", 0.0, false, false, true));
        assert!(visibility.visible_on("eDP-1", false, false));
    }

    #[test]
    fn full_screen_mode_is_normal_on_desktop_and_reveals_in_fullscreen() {
        let mut visibility = MenuBarVisibility::default();
        assert_eq!(visibility.mode(), AutoHideMode::FullScreen);
        // Desktop: visible with no transient reveal.
        assert!(visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.update_pointer("eDP-1", 200.0, false, false, true));
        // Fullscreen: concealed until the pointer reaches the top edge.
        assert!(!visibility.visible_on("eDP-1", true, false));
        assert!(visibility.update_pointer("eDP-1", 0.0, true, false, true));
        assert!(visibility.visible_on("eDP-1", true, false));
        // Leaving the edge conceals it again.
        assert!(visibility.update_pointer("eDP-1", 40.0, true, false, true));
        assert!(!visibility.visible_on("eDP-1", true, false));
        // An active menu keeps it revealed over fullscreen content.
        assert!(visibility.update_pointer("eDP-1", 40.0, true, true, true));
        assert!(visibility.visible_on("eDP-1", true, true));
    }

    #[test]
    fn live_mode_changes_clear_the_old_reveal_and_apply_immediately() {
        let mut visibility = MenuBarVisibility::default();
        visibility.reveal("eDP-1");
        assert!(visibility.set_mode(AutoHideMode::Always));
        assert!(!visibility.is_revealed("eDP-1"));
        assert!(!visibility.visible_on("eDP-1", false, false));

        visibility.reveal("eDP-1");
        assert!(visibility.set_mode(AutoHideMode::Never));
        assert!(visibility.visible_on("eDP-1", true, false));

        assert!(visibility.set_mode(AutoHideMode::FullScreen));
        assert!(visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.visible_on("eDP-1", true, false));
    }

    #[test]
    fn second_output_has_independent_reveal_and_reservation_input() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_mode(AutoHideMode::Always);
        visibility.reveal("HDMI-A-1");
        assert!(visibility.visible_on("HDMI-A-1", false, false));
        assert!(!visibility.visible_on("eDP-1", false, false));

        // The second output conceals without disturbing the first.
        assert!(visibility.update_pointer("HDMI-A-1", 50.0, false, false, true));
        assert!(!visibility.visible_on("HDMI-A-1", false, false));
        assert!(visibility.reveal("eDP-1"));
        assert!(visibility.visible_on("eDP-1", false, false));
    }

    #[test]
    fn lock_clears_reveal_and_unlock_rests_concealed() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_mode(AutoHideMode::Always);
        visibility.reveal("eDP-1");
        assert!(visibility.visible_on("eDP-1", false, false));

        // Lock drops every transient reveal; while locked nothing is drawn.
        assert!(visibility.clear());
        assert!(!visibility.clear());
        assert!(!visibility.visible_on("eDP-1", false, false));

        // Unlock starts from the resting state: hidden until the next edge
        // or focus reveal.
        assert!(!visibility.visible_on("eDP-1", false, false));
        assert!(visibility.update_pointer("eDP-1", 0.0, false, false, true));
        assert!(visibility.visible_on("eDP-1", false, false));
    }

    #[test]
    fn concealed_bar_contributes_no_reserved_zone() {
        use crate::shell::layer::{
            aggregate_reserved_for, LayerSurfaceState, ANCHOR_LEFT, ANCHOR_RIGHT, ANCHOR_TOP,
        };
        use crate::window::ReservedZones;

        let bar = LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT,
            height: MENU_BAR_REVEAL_HEIGHT,
            exclusive_zone: MENU_BAR_REVEAL_HEIGHT,
            ..Default::default()
        };
        let mut visibility = MenuBarVisibility::default();
        visibility.set_mode(AutoHideMode::Always);

        let visible = visibility.visible_on("eDP-1", false, false);
        let layers = [&bar].into_iter().filter(|_| visible);
        let zones = aggregate_reserved_for("eDP-1", layers, ReservedZones::default());
        assert_eq!(zones.top, 0, "a concealed bar must not reserve the strip");

        visibility.reveal("eDP-1");
        let visible = visibility.visible_on("eDP-1", false, false);
        let layers = [&bar].into_iter().filter(|_| visible);
        let zones = aggregate_reserved_for("eDP-1", layers, ReservedZones::default());
        assert_eq!(
            zones.top, MENU_BAR_REVEAL_HEIGHT,
            "a revealed bar reserves its strip again"
        );
    }
}
