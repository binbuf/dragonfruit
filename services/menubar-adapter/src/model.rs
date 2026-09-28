// SPDX-License-Identifier: MIT
//! The Menu Bar configuration snapshot the pane and tile render, decoded from
//! one raw read.
//!
//! The model owns only the projection a consumer should not repeat: it names
//! the auto-hide modes and the bar controls, types the clock options, folds the
//! per-control availability into stable slots, and computes the change stream
//! between two reads. It never renders the bar, resolves a menu, or decides
//! which item is live — the shell's `MenuBar` and the menu-broker own those;
//! this is the data they publish.
//!
//! A running shell always answers `Available`; the only adapter-level absence
//! is a missing menu-bar bridge (`AdapterState::Unavailable`), which is a
//! normal hidden state, exactly as `dragonfruit-overview` treats a missing
//! compositor bridge. The event half is a pure diff:
//! [`MenuBarSnapshot::changes`] reports what moved between two reads (the
//! bar hid, the auto-hide mode changed, a clock option toggled, a control
//! came or went) without any polling.

use crate::source::{MenuBarControlState, MenuBarData};

/// How the menu bar hides and shows.
///
/// This is the macOS `Automatically hide and show the menu bar` popup adapted
/// to the shell: `Never` keeps the bar always visible, `Always` hides it until
/// the pointer reaches the top edge or a menu opens, and `FullScreen` hides it
/// only while a window is full-screen. The stable ids are the settingsd values
/// T-15.9b declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum MenuBarAutoHide {
    /// The bar is always visible.
    Never,
    /// The bar hides whenever it is not pointed at or open.
    Always,
    /// The bar hides only in full-screen (the shipped default).
    #[default]
    FullScreen,
}

impl MenuBarAutoHide {
    /// Every mode, in the pane's popup order.
    pub const ALL: [MenuBarAutoHide; 3] = [
        MenuBarAutoHide::Never,
        MenuBarAutoHide::Always,
        MenuBarAutoHide::FullScreen,
    ];

    /// The stable id used by the settings key and the wire.
    pub const fn id(self) -> &'static str {
        match self {
            MenuBarAutoHide::Never => "never",
            MenuBarAutoHide::Always => "always",
            MenuBarAutoHide::FullScreen => "full-screen",
        }
    }

    /// The popup label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            MenuBarAutoHide::Never => "Never",
            MenuBarAutoHide::Always => "Always",
            MenuBarAutoHide::FullScreen => "In Full Screen Only",
        }
    }

    /// Parse the stable id back to a mode.
    pub fn from_id(id: &str) -> Option<Self> {
        MenuBarAutoHide::ALL
            .into_iter()
            .find(|mode| mode.id() == id)
    }
}

/// One configurable control that can appear in the menu bar.
///
/// The set is the Linux adaptation of the macOS `Menu Bar Controls` list:
/// `Spotlight` is our deferred Search pane so it is omitted, and `AirDrop`,
/// `Screen Mirroring`, and `Display` have no Linux equivalent and are omitted.
/// The ids match the shell's status-item ids (`shell/menubar/StatusItem.qml`,
/// `shell/src/shellcontroller.cpp`), so a projection maps straight onto the
/// status row; `Sound` carries the audio adapter's `volume` id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuBarControl {
    /// The menu-bar clock.
    Clock,
    /// Wi-Fi (NetworkManager).
    Wifi,
    /// Bluetooth (BlueZ).
    Bluetooth,
    /// Battery (UPower).
    Battery,
    /// Sound/output volume (PipeWire/WirePlumber).
    Sound,
    /// Focus / Do Not Disturb (the notification service).
    Focus,
    /// Accessibility status (T-11).
    Accessibility,
}

impl MenuBarControl {
    /// Every control, in the bar's left-to-right and list order.
    pub const ALL: [MenuBarControl; 7] = [
        MenuBarControl::Clock,
        MenuBarControl::Wifi,
        MenuBarControl::Bluetooth,
        MenuBarControl::Battery,
        MenuBarControl::Sound,
        MenuBarControl::Focus,
        MenuBarControl::Accessibility,
    ];

    /// The array index for this control.
    pub const fn index(self) -> usize {
        match self {
            MenuBarControl::Clock => 0,
            MenuBarControl::Wifi => 1,
            MenuBarControl::Bluetooth => 2,
            MenuBarControl::Battery => 3,
            MenuBarControl::Sound => 4,
            MenuBarControl::Focus => 5,
            MenuBarControl::Accessibility => 6,
        }
    }

    /// The control at `index`, if the index is in range.
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(MenuBarControl::Clock),
            1 => Some(MenuBarControl::Wifi),
            2 => Some(MenuBarControl::Bluetooth),
            3 => Some(MenuBarControl::Battery),
            4 => Some(MenuBarControl::Sound),
            5 => Some(MenuBarControl::Focus),
            6 => Some(MenuBarControl::Accessibility),
            _ => None,
        }
    }

    /// The stable id: the shell status-item id.
    pub const fn id(self) -> &'static str {
        match self {
            MenuBarControl::Clock => "clock",
            MenuBarControl::Wifi => "wifi",
            MenuBarControl::Bluetooth => "bluetooth",
            MenuBarControl::Battery => "battery",
            MenuBarControl::Sound => "volume",
            MenuBarControl::Focus => "focus",
            MenuBarControl::Accessibility => "accessibility",
        }
    }

    /// The row label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            MenuBarControl::Clock => "Clock",
            MenuBarControl::Wifi => "Wi-Fi",
            MenuBarControl::Bluetooth => "Bluetooth",
            MenuBarControl::Battery => "Battery",
            MenuBarControl::Sound => "Sound",
            MenuBarControl::Focus => "Focus",
            MenuBarControl::Accessibility => "Accessibility",
        }
    }
}

/// One clock display option.
///
/// These are the two options the shell's [`MenuBarClock`] already renders
/// (`shell/menubar/MenuBarClock.qml`). Richer macOS `Clock Options...` rows
/// (day of week, 24-hour, flashing separators) are deliberately not modelled
/// until the clock consumes them; see the follow-up note.
///
/// [`MenuBarClock`]: ../../../shell/menubar/MenuBarClock.qml
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClockOption {
    /// Show the date beside the time (`showDate`).
    ShowDate,
    /// Display the time with seconds (`showSeconds`).
    ShowSeconds,
}

impl ClockOption {
    /// Every clock option, in the pane's row order.
    pub const ALL: [ClockOption; 2] = [ClockOption::ShowDate, ClockOption::ShowSeconds];

    /// The stable id, the settingsd key suffix and the shell property name.
    pub const fn id(self) -> &'static str {
        match self {
            ClockOption::ShowDate => "showDate",
            ClockOption::ShowSeconds => "showSeconds",
        }
    }

    /// The row label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            ClockOption::ShowDate => "Show date",
            ClockOption::ShowSeconds => "Display the time with seconds",
        }
    }
}

/// The clock's display options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockOptions {
    /// Show the date beside the time.
    pub show_date: bool,
    /// Display the time with seconds.
    pub show_seconds: bool,
}

impl Default for ClockOptions {
    /// The shell's shipped clock: the date is shown, seconds are not
    /// (`shell/src/shellcontroller.cpp` sets `showDate` true).
    fn default() -> Self {
        ClockOptions {
            show_date: true,
            show_seconds: false,
        }
    }
}

impl ClockOptions {
    /// The value of `option`.
    pub const fn option(self, option: ClockOption) -> bool {
        match option {
            ClockOption::ShowDate => self.show_date,
            ClockOption::ShowSeconds => self.show_seconds,
        }
    }

    /// Set the value of `option`.
    pub fn set(&mut self, option: ClockOption, value: bool) {
        match option {
            ClockOption::ShowDate => self.show_date = value,
            ClockOption::ShowSeconds => self.show_seconds = value,
        }
    }
}

/// The Menu Bar configuration snapshot a pane or tile renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBarSnapshot {
    /// Whether the bar is currently hidden by auto-hide.
    pub hidden: bool,
    /// The effective auto-hide mode.
    pub auto_hide: MenuBarAutoHide,
    /// Whether the bar draws its background material.
    pub show_background: bool,
    /// Whether the focused app's menus appear in the global bar (`menu.global`).
    pub global_menu: bool,
    /// The clock display options.
    pub clock: ClockOptions,
    /// The live availability of each control, indexed by
    /// [`MenuBarControl::index`].
    pub controls: [MenuBarControlState; 7],
}

impl Default for MenuBarSnapshot {
    fn default() -> Self {
        MenuBarSnapshot::from_data(&MenuBarData::default())
    }
}

impl MenuBarSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &MenuBarData) -> Self {
        let mut controls = [MenuBarControlState::default(); 7];
        for control in MenuBarControl::ALL {
            controls[control.index()] = data.control(control);
        }
        MenuBarSnapshot {
            hidden: data.hidden,
            auto_hide: data.auto_hide,
            show_background: data.show_background,
            global_menu: data.global_menu,
            clock: data.clock,
            controls,
        }
    }

    /// Whether the bar is currently hidden by auto-hide.
    pub const fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// The value of `option`.
    pub const fn clock_option(&self, option: ClockOption) -> bool {
        self.clock.option(option)
    }

    /// The availability of `control`.
    pub fn control(&self, control: MenuBarControl) -> MenuBarControlState {
        self.controls[control.index()]
    }

    /// The controls the bar is currently showing, in bar order.
    pub fn visible_controls(&self) -> Vec<MenuBarControl> {
        MenuBarControl::ALL
            .into_iter()
            .filter(|control| self.control(*control).is_visible())
            .collect()
    }

    /// A one-line label for the status tile: the auto-hide mode, or `Hidden`
    /// while the bar is actually off-screen.
    pub fn label(&self) -> &'static str {
        if self.hidden {
            "Hidden"
        } else {
            self.auto_hide.label()
        }
    }

    /// The design-system glyph for the status tile.
    pub fn glyph(&self) -> &'static str {
        "menu-bar"
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// the runtime hidden flag, the auto-hide mode, the background and global
    /// menu flags, each clock option, then each control slot.
    ///
    /// This is the "event" half of the adapter: the shell bridge diffs two
    /// reads to learn what moved without polling each field.
    pub fn changes(&self, previous: &MenuBarSnapshot) -> Vec<MenuBarChange> {
        let mut changes = Vec::new();
        if self.hidden != previous.hidden {
            changes.push(MenuBarChange::VisibilityChanged {
                from: previous.hidden,
                to: self.hidden,
            });
        }
        if self.auto_hide != previous.auto_hide {
            changes.push(MenuBarChange::AutoHideChanged {
                from: previous.auto_hide,
                to: self.auto_hide,
            });
        }
        if self.show_background != previous.show_background {
            changes.push(MenuBarChange::BackgroundChanged {
                from: previous.show_background,
                to: self.show_background,
            });
        }
        if self.global_menu != previous.global_menu {
            changes.push(MenuBarChange::GlobalMenuChanged {
                from: previous.global_menu,
                to: self.global_menu,
            });
        }
        for option in ClockOption::ALL {
            let from = previous.clock.option(option);
            let to = self.clock.option(option);
            if from != to {
                changes.push(MenuBarChange::ClockOptionChanged { option, from, to });
            }
        }
        for control in MenuBarControl::ALL {
            let from = previous.control(control);
            let to = self.control(control);
            if from != to {
                changes.push(MenuBarChange::ControlChanged { control, from, to });
            }
        }
        changes
    }
}

/// A change between two Menu Bar configuration snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuBarChange {
    /// The bar was hidden or shown by auto-hide.
    VisibilityChanged {
        /// The previous hidden flag.
        from: bool,
        /// The new hidden flag.
        to: bool,
    },
    /// The auto-hide mode changed.
    AutoHideChanged {
        /// The previous mode.
        from: MenuBarAutoHide,
        /// The new mode.
        to: MenuBarAutoHide,
    },
    /// The background material toggle moved.
    BackgroundChanged {
        /// The previous value.
        from: bool,
        /// The new value.
        to: bool,
    },
    /// The global application-menu toggle moved.
    GlobalMenuChanged {
        /// The previous value.
        from: bool,
        /// The new value.
        to: bool,
    },
    /// One clock option moved.
    ClockOptionChanged {
        /// The option that moved.
        option: ClockOption,
        /// The previous value.
        from: bool,
        /// The new value.
        to: bool,
    },
    /// A control slot's availability or interactivity moved.
    ControlChanged {
        /// The control that moved.
        control: MenuBarControl,
        /// The previous slot state.
        from: MenuBarControlState,
        /// The new slot state.
        to: MenuBarControlState,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> MenuBarData {
        MenuBarData::default()
    }

    #[test]
    fn auto_hide_modes_round_trip_and_default_to_full_screen() {
        for mode in MenuBarAutoHide::ALL {
            assert_eq!(MenuBarAutoHide::from_id(mode.id()), Some(mode));
        }
        assert_eq!(MenuBarAutoHide::from_id("bogus"), None);
        assert_eq!(MenuBarAutoHide::default(), MenuBarAutoHide::FullScreen);
        assert_eq!(MenuBarAutoHide::FullScreen.label(), "In Full Screen Only");
    }

    #[test]
    fn controls_index_and_round_trip() {
        for control in MenuBarControl::ALL {
            assert_eq!(MenuBarControl::from_index(control.index()), Some(control));
        }
        assert_eq!(MenuBarControl::from_index(7), None);
        assert_eq!(MenuBarControl::Sound.id(), "volume");
        assert_eq!(MenuBarControl::Wifi.label(), "Wi-Fi");
    }

    #[test]
    fn clock_option_ids_are_the_shell_property_names() {
        let ids: Vec<&str> = ClockOption::ALL.iter().map(|option| option.id()).collect();
        assert_eq!(ids, vec!["showDate", "showSeconds"]);
        assert_eq!(
            ClockOption::ShowSeconds.label(),
            "Display the time with seconds"
        );
    }

    #[test]
    fn the_shipped_clock_shows_the_date_only() {
        let snapshot = MenuBarSnapshot::from_data(&data());
        assert!(snapshot.clock_option(ClockOption::ShowDate));
        assert!(!snapshot.clock_option(ClockOption::ShowSeconds));
    }

    #[test]
    fn the_snapshot_projects_the_auto_hide_and_flags() {
        let snapshot = MenuBarSnapshot::from_data(&data());
        assert!(!snapshot.is_hidden());
        assert_eq!(snapshot.auto_hide, MenuBarAutoHide::FullScreen);
        assert!(snapshot.show_background);
        assert!(snapshot.global_menu);
        assert_eq!(snapshot.glyph(), "menu-bar");
        assert_eq!(snapshot.label(), "In Full Screen Only");
    }

    #[test]
    fn visible_controls_follow_availability() {
        let mut raw = data();
        raw.controls[MenuBarControl::Bluetooth.index()] = MenuBarControlState::absent();
        let snapshot = MenuBarSnapshot::from_data(&raw);
        let visible = snapshot.visible_controls();
        assert!(!visible.contains(&MenuBarControl::Bluetooth));
        assert!(visible.contains(&MenuBarControl::Clock));
        assert_eq!(visible.len(), MenuBarControl::ALL.len() - 1);
    }

    #[test]
    fn a_hidden_bar_labels_as_hidden() {
        let mut raw = data();
        raw.hidden = true;
        let snapshot = MenuBarSnapshot::from_data(&raw);
        assert!(snapshot.is_hidden());
        assert_eq!(snapshot.label(), "Hidden");
    }

    #[test]
    fn each_configuration_move_is_a_change() {
        let previous = MenuBarSnapshot::from_data(&data());

        let mut raw = data();
        raw.auto_hide = MenuBarAutoHide::Always;
        raw.show_background = false;
        raw.global_menu = false;
        raw.clock.show_seconds = true;
        raw.controls[MenuBarControl::Bluetooth.index()] = MenuBarControlState::absent();
        let next = MenuBarSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&MenuBarChange::AutoHideChanged {
            from: MenuBarAutoHide::FullScreen,
            to: MenuBarAutoHide::Always,
        }));
        assert!(changes.contains(&MenuBarChange::BackgroundChanged {
            from: true,
            to: false,
        }));
        assert!(changes.contains(&MenuBarChange::GlobalMenuChanged {
            from: true,
            to: false,
        }));
        assert!(changes.contains(&MenuBarChange::ClockOptionChanged {
            option: ClockOption::ShowSeconds,
            from: false,
            to: true,
        }));
        assert!(changes.contains(&MenuBarChange::ControlChanged {
            control: MenuBarControl::Bluetooth,
            from: MenuBarControlState::present(),
            to: MenuBarControlState::absent(),
        }));
        // The date did not move, so it is not reported.
        assert!(!changes.iter().any(|change| matches!(
            change,
            MenuBarChange::ClockOptionChanged {
                option: ClockOption::ShowDate,
                ..
            }
        )));
    }

    #[test]
    fn hiding_the_bar_is_a_visibility_change() {
        let previous = MenuBarSnapshot::from_data(&data());
        let mut raw = data();
        raw.hidden = true;
        let next = MenuBarSnapshot::from_data(&raw);
        assert_eq!(
            next.changes(&previous),
            vec![MenuBarChange::VisibilityChanged {
                from: false,
                to: true,
            }]
        );
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = MenuBarSnapshot::from_data(&data());
        assert!(snapshot.changes(&snapshot).is_empty());
        assert_eq!(snapshot, MenuBarSnapshot::default());
    }

    #[test]
    fn clock_options_set_and_read_each_option() {
        let mut options = ClockOptions::default();
        options.set(ClockOption::ShowSeconds, true);
        options.set(ClockOption::ShowDate, false);
        assert!(options.option(ClockOption::ShowSeconds));
        assert!(!options.option(ClockOption::ShowDate));
    }
}
