// SPDX-License-Identifier: MIT
//! The transport seam: the raw Menu Bar configuration read and its mock.
//!
//! A [`MenuBarSource`] is the only thing that talks to the host stack. Menu Bar
//! configuration is **shell-native**: the shell's `MenuBar`
//! (`shell/menubar/MenuBar.qml`) owns the chrome, the status-item model, and the
//! clock, the menu-broker (`services/menu-broker`) resolves the focused app's
//! global menu, and `settingsd` owns the durable preferences they read. There
//! is no external daemon to wrap, so this crate does not re-render the bar or
//! re-resolve a menu; it is the **projection** — the effective auto-hide mode,
//! background and global-menu flags, clock options, and the per-control
//! availability the bar is showing — delivered through a seam so the shell
//! bridge (T-15.9b) and CI's [`MockMenuBar`] answer identically.
//!
//! The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — the host stack answered; `data` is the live read.
//! * `Ok(None)` — the host stack is absent (no menu-bar bridge). A normal
//!   state; the item hides.
//! * `Err(error)` — the host stack is present but could not be read; the item
//!   shows visible and inert with the message.

use dragonfruit_system_adapters::AdapterError;

use crate::model::{ClockOptions, MenuBarAutoHide, MenuBarControl};

/// The live availability of one menu-bar control slot.
///
/// This mirrors the shell's `StatusItem` (`available` hides the slot,
/// `enabled` keeps it visible but dimmed) so a projection maps straight onto
/// the status row. A `Clock` slot is always present when the bar is; an
/// adapter-backed control is absent when its daemon is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuBarControlState {
    /// Whether the slot is part of the bar at all.
    pub available: bool,
    /// Whether the slot is interactive (a present adapter that failed is
    /// visible but inert).
    pub enabled: bool,
}

impl MenuBarControlState {
    /// The control is absent: its daemon is gone or the user turned it off.
    pub const ABSENT: MenuBarControlState = MenuBarControlState {
        available: false,
        enabled: false,
    };

    /// The control is present and interactive.
    pub const PRESENT: MenuBarControlState = MenuBarControlState {
        available: true,
        enabled: true,
    };

    /// The control is present but not interactive (its adapter failed).
    pub const INERT: MenuBarControlState = MenuBarControlState {
        available: true,
        enabled: false,
    };

    /// An absent slot.
    pub const fn absent() -> Self {
        MenuBarControlState::ABSENT
    }

    /// A present, interactive slot.
    pub const fn present() -> Self {
        MenuBarControlState::PRESENT
    }

    /// A present but inert slot.
    pub const fn inert() -> Self {
        MenuBarControlState::INERT
    }

    /// Whether the slot is drawn.
    pub const fn is_visible(self) -> bool {
        self.available
    }

    /// Whether the slot is interactive.
    pub const fn is_enabled(self) -> bool {
        self.available && self.enabled
    }
}

impl Default for MenuBarControlState {
    /// The safe "not shown" default, so a source that omits a control does not
    /// accidentally show it.
    fn default() -> Self {
        MenuBarControlState::ABSENT
    }
}

/// The raw result of one Menu Bar configuration read: the effective chrome and
/// clock configuration plus the live availability of every control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBarData {
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
    /// The availability of each control, indexed by [`MenuBarControl::index`].
    pub controls: [MenuBarControlState; 7],
}

impl Default for MenuBarData {
    /// The shell's shipped menu bar: visible, auto-hiding only in full screen,
    /// drawing its chrome, with the global app menu on, the date shown, and a
    /// fully populated status row.
    fn default() -> Self {
        MenuBarData {
            hidden: false,
            auto_hide: MenuBarAutoHide::default(),
            show_background: true,
            global_menu: true,
            clock: ClockOptions::default(),
            controls: [MenuBarControlState::PRESENT; 7],
        }
    }
}

impl MenuBarData {
    /// The availability of `control`.
    pub fn control(&self, control: MenuBarControl) -> MenuBarControlState {
        self.controls[control.index()]
    }

    /// Set the availability of `control`.
    pub fn set_control(&mut self, control: MenuBarControl, state: MenuBarControlState) {
        self.controls[control.index()] = state;
    }
}

/// Reads the Menu Bar configuration host stack over some transport.
pub trait MenuBarSource {
    /// One read of the host stack.
    fn read(&mut self) -> Result<Option<MenuBarData>, AdapterError>;
}

/// A fixture-backed source with a simulated host-stack lifecycle.
///
/// The mock is the CI path: it serves [`MenuBarData`] with no shell and no
/// Wayland connection, and `kill`/`restart` exercise absence and re-subscribe
/// the way losing the menu-bar bridge would. `push` drives the configuration
/// and control availability so a test observes the change stream.
#[derive(Debug, Clone, PartialEq)]
pub struct MockMenuBar {
    present: bool,
    data: Option<MenuBarData>,
    failure: Option<AdapterError>,
    reads: u32,
}

impl MockMenuBar {
    /// The host stack is not reachable.
    pub fn absent() -> Self {
        MockMenuBar {
            present: false,
            data: None,
            failure: None,
            reads: 0,
        }
    }

    /// A host stack that answers with `data`.
    pub fn present(data: MenuBarData) -> Self {
        MockMenuBar {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
        }
    }

    /// A host stack that fails every read (e.g. the bridge went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockMenuBar {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
        }
    }

    /// The host stack publishes fresh state.
    pub fn push(&mut self, data: MenuBarData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The host stack goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host stack comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated host stack is reachable.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }
}

impl MenuBarSource for MockMenuBar {
    fn read(&mut self) -> Result<Option<MenuBarData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> MenuBarData {
        MenuBarData::default()
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockMenuBar::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.is_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockMenuBar::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockMenuBar::failing("menu-bar bridge: timeout");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "menu-bar bridge: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockMenuBar::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn push_replaces_the_served_data() {
        let mut mock = MockMenuBar::present(data());
        let next = MenuBarData {
            auto_hide: MenuBarAutoHide::Never,
            ..data()
        };
        mock.push(next);
        assert_eq!(
            mock.read().unwrap().unwrap().auto_hide,
            MenuBarAutoHide::Never
        );
    }

    #[test]
    fn the_defaults_mirror_the_shipped_bar() {
        let default = MenuBarData::default();
        assert!(!default.hidden);
        assert_eq!(default.auto_hide, MenuBarAutoHide::FullScreen);
        assert!(default.show_background);
        assert!(default.global_menu);
        assert!(default.clock.show_date);
        assert_eq!(default.controls, [MenuBarControlState::PRESENT; 7]);
    }

    #[test]
    fn control_states_classify_visibility_and_enabledness() {
        assert!(!MenuBarControlState::absent().is_visible());
        assert!(MenuBarControlState::present().is_visible());
        assert!(MenuBarControlState::present().is_enabled());
        assert!(MenuBarControlState::inert().is_visible());
        assert!(!MenuBarControlState::inert().is_enabled());
        assert_eq!(
            MenuBarControlState::default(),
            MenuBarControlState::absent()
        );
    }

    #[test]
    fn set_control_writes_the_indexed_slot() {
        let mut raw = data();
        raw.set_control(MenuBarControl::Bluetooth, MenuBarControlState::inert());
        assert_eq!(
            raw.control(MenuBarControl::Bluetooth),
            MenuBarControlState::INERT
        );
    }
}
