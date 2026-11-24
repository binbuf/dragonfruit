// SPDX-License-Identifier: MIT
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! Hot-corner detection in the compositor input path (T-03).
//!
//! Corners are detected here and dispatched to the shell through the same
//! [`super::dispatch::InputDispatch`] outbox as keyboard shortcuts and
//! gestures, so an Overview corner, a Ctrl+Up, and a four-finger
//! swipe produce one identical event stream
//! ([04-shell.md](../../docs/design/04-shell.md)).
//!
//! Each output has its own corners (T-14 documents multi-monitor
//! assignment). Detection uses a dwell time so a pointer merely crossing a
//! corner does not trigger it.

use std::time::{Duration, Instant};

use smithay::utils::{Logical, Point, Rectangle};

use super::action::{HotCorner, InputAction};
use super::keymap::RoleMods;

/// The action assigned to a corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HotCornerAction {
    #[default]
    None,
    MissionControl,
    NotificationCenter,
    DesktopReveal,
    LockScreen,
    /// Open the Applications drawer (the Applications drawer slot).
    Applications,
    /// Toggle the Quick Settings panel.
    ControlCenter,
    /// Toggle the Dock's auto-hide, like the Super+Option+D chord.
    ShowDock,
    /// Reveal and focus the menu bar.
    MenuBar,
    /// Move keyboard focus into the Dock.
    FocusDock,
}

impl HotCornerAction {
    /// The stable id the settings schema and the shell wire share.
    pub const fn id(self) -> &'static str {
        match self {
            HotCornerAction::None => "none",
            HotCornerAction::MissionControl => "mission-control",
            HotCornerAction::NotificationCenter => "notification-center",
            HotCornerAction::DesktopReveal => "desktop-reveal",
            HotCornerAction::LockScreen => "lock-screen",
            HotCornerAction::Applications => "launchpad",
            HotCornerAction::ControlCenter => "control-center",
            HotCornerAction::ShowDock => "show-dock",
            HotCornerAction::MenuBar => "menu-bar",
            HotCornerAction::FocusDock => "focus-dock",
        }
    }

    /// Parse a schema id; an unknown spelling leaves the corner unchanged.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(HotCornerAction::None),
            "mission-control" => Some(HotCornerAction::MissionControl),
            "notification-center" => Some(HotCornerAction::NotificationCenter),
            "desktop-reveal" => Some(HotCornerAction::DesktopReveal),
            "lock-screen" => Some(HotCornerAction::LockScreen),
            "launchpad" => Some(HotCornerAction::Applications),
            "control-center" => Some(HotCornerAction::ControlCenter),
            "show-dock" => Some(HotCornerAction::ShowDock),
            "menu-bar" => Some(HotCornerAction::MenuBar),
            "focus-dock" => Some(HotCornerAction::FocusDock),
            _ => None,
        }
    }

    /// Map to the compositor-level action, if any. Unassigned corners do
    /// nothing (T-14 FR-2: no accidental triggers).
    pub const fn to_input_action(self) -> Option<InputAction> {
        match self {
            HotCornerAction::None => None,
            HotCornerAction::MissionControl => Some(InputAction::MissionControl),
            HotCornerAction::NotificationCenter => Some(InputAction::NotificationCenter),
            HotCornerAction::DesktopReveal => Some(InputAction::DesktopReveal),
            HotCornerAction::LockScreen => Some(InputAction::LockScreen),
            HotCornerAction::Applications => Some(InputAction::ShowApps),
            HotCornerAction::ControlCenter => Some(InputAction::ControlCenter),
            HotCornerAction::ShowDock => Some(InputAction::ToggleDock),
            HotCornerAction::MenuBar => Some(InputAction::FocusMenuBar),
            HotCornerAction::FocusDock => Some(InputAction::FocusDock),
        }
    }
}

/// The modifier a corner requires before it fires. `None` keeps the historical
/// no-modifier behavior.
pub fn parse_hot_corner_modifier(value: &str) -> Option<RoleMods> {
    match value {
        "none" | "" => Some(RoleMods::EMPTY),
        "command" => Some(RoleMods::COMMAND),
        "shift" => Some(RoleMods::SHIFT),
        "option" => Some(RoleMods::OPTION),
        "control" => Some(RoleMods::CONTROL),
        _ => None,
    }
}

/// Hot-corner configuration, applied live from the Settings model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HotCornerConfig {
    /// Assignment per corner, indexed by [`HotCorner::index`].
    pub assignments: [HotCornerAction; 4],
    /// Required modifier per corner, indexed by [`HotCorner::index`].
    pub modifiers: [RoleMods; 4],
    /// Distance from the corner (logical px) that counts as "in" it.
    pub inset: f64,
    /// How long the pointer must rest in the corner before it fires.
    pub dwell: Duration,
}

impl Default for HotCornerConfig {
    fn default() -> Self {
        // A tasteful default: Overview top-left, Notifications
        // top-right, reveal bottom-left, lock bottom-right (T-14 owns the
        // user-facing configuration UI and persistence).
        HotCornerConfig {
            assignments: [
                HotCornerAction::MissionControl,
                HotCornerAction::NotificationCenter,
                HotCornerAction::DesktopReveal,
                HotCornerAction::LockScreen,
            ],
            modifiers: [RoleMods::EMPTY; 4],
            inset: 4.0,
            dwell: Duration::from_millis(150),
        }
    }
}

/// A corner that has fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotCornerTrigger {
    pub corner: HotCorner,
    pub action: InputAction,
}

/// Dwell-based hot-corner detector.
#[derive(Debug, Clone)]
pub struct HotCornerDetector {
    config: HotCornerConfig,
    active: Option<(HotCorner, Instant)>,
    fired: bool,
    /// The significant modifier roles currently held.
    modifiers: RoleMods,
}

impl Default for HotCornerDetector {
    fn default() -> Self {
        HotCornerDetector::new(HotCornerConfig::default())
    }
}

impl HotCornerDetector {
    pub fn new(config: HotCornerConfig) -> Self {
        HotCornerDetector {
            config,
            active: None,
            fired: false,
            modifiers: RoleMods::EMPTY,
        }
    }

    pub fn config(&self) -> HotCornerConfig {
        self.config
    }

    pub fn set_config(&mut self, config: HotCornerConfig) {
        self.config = config;
        self.reset();
    }

    pub fn dwell(&self) -> Duration {
        self.config.dwell
    }

    /// Update the held modifier roles (keyboard events) and clear a corner
    /// whose required modifier is no longer held, so a stale dwell cannot
    /// fire after the user releases it.
    pub fn set_modifiers(&mut self, modifiers: RoleMods) {
        if self.modifiers == modifiers {
            return;
        }
        self.modifiers = modifiers;
        if let Some((corner, _)) = self.active {
            if !self.modifier_held(corner) {
                self.active = None;
                self.fired = false;
            }
        }
    }

    /// Whether the corner's required modifier is currently held.
    fn modifier_held(&self, corner: HotCorner) -> bool {
        self.modifiers
            .contains(self.config.modifiers[corner.index()])
    }

    /// True while a corner is occupied, assigned, and waiting out its
    /// dwell — the condition for scheduling a dwell timer.
    pub fn is_armed(&self) -> bool {
        match self.active {
            Some((corner, _)) if !self.fired => {
                self.config.assignments[corner.index()]
                    .to_input_action()
                    .is_some()
                    && self.modifier_held(corner)
            }
            _ => false,
        }
    }

    /// The corner containing `location`, if any, within `bounds`.
    pub fn corner_at(
        &self,
        location: Point<f64, Logical>,
        bounds: Rectangle<i32, Logical>,
    ) -> Option<HotCorner> {
        let x = location.x - f64::from(bounds.loc.x);
        let y = location.y - f64::from(bounds.loc.y);
        let w = f64::from(bounds.size.w);
        let h = f64::from(bounds.size.h);
        let inset = self.config.inset;
        let left = x < inset;
        let right = x > w - inset;
        let top = y < inset;
        let bottom = y > h - inset;
        match (left, right, top, bottom) {
            (true, _, true, _) => Some(HotCorner::TopLeft),
            (_, true, true, _) => Some(HotCorner::TopRight),
            (true, _, _, true) => Some(HotCorner::BottomLeft),
            (_, true, _, true) => Some(HotCorner::BottomRight),
            _ => None,
        }
    }

    /// Feed a pointer location update.
    pub fn pointer_moved(
        &mut self,
        location: Point<f64, Logical>,
        bounds: Rectangle<i32, Logical>,
        now: Instant,
    ) -> Option<HotCornerTrigger> {
        match self.corner_at(location, bounds) {
            Some(corner) => {
                match self.active {
                    Some((active, _)) if active == corner => {}
                    _ => {
                        self.active = Some((corner, now));
                        self.fired = false;
                    }
                }
                self.poll(now)
            }
            None => {
                self.reset();
                None
            }
        }
    }

    /// Evaluate dwell against the currently occupied corner. Called by the
    /// dwell timer; the pointer may not have moved.
    pub fn poll(&mut self, now: Instant) -> Option<HotCornerTrigger> {
        let (corner, entered) = self.active?;
        if self.fired {
            return None;
        }
        if now.saturating_duration_since(entered) < self.config.dwell {
            return None;
        }
        if !self.modifier_held(corner) {
            return None;
        }
        let action = self.config.assignments[corner.index()].to_input_action()?;
        self.fired = true;
        Some(HotCornerTrigger { corner, action })
    }

    /// Clear the occupied corner (pointer left all corners, or config
    /// changed).
    pub fn reset(&mut self) {
        self.active = None;
        self.fired = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rectangle<i32, Logical> {
        Rectangle::new((0, 0).into(), (1920, 1080).into())
    }

    #[test]
    fn detects_each_corner() {
        let detector = HotCornerDetector::default();
        let b = bounds();
        assert_eq!(
            detector.corner_at((1.0, 1.0).into(), b),
            Some(HotCorner::TopLeft)
        );
        assert_eq!(
            detector.corner_at((1919.0, 1.0).into(), b),
            Some(HotCorner::TopRight)
        );
        assert_eq!(
            detector.corner_at((1.0, 1079.0).into(), b),
            Some(HotCorner::BottomLeft)
        );
        assert_eq!(
            detector.corner_at((1919.0, 1079.0).into(), b),
            Some(HotCorner::BottomRight)
        );
        assert_eq!(detector.corner_at((500.0, 500.0).into(), b), None);
    }

    #[test]
    fn dwell_is_required_before_firing() {
        let mut detector = HotCornerDetector::new(HotCornerConfig {
            dwell: Duration::from_millis(100),
            ..Default::default()
        });
        let now = Instant::now();
        let loc = (1.0, 1.0).into();
        assert!(detector.pointer_moved(loc, bounds(), now).is_none());
        assert!(detector.is_armed());
        assert!(detector.poll(now + Duration::from_millis(50)).is_none());
        let fired = detector
            .poll(now + Duration::from_millis(120))
            .expect("fires after dwell");
        assert_eq!(fired.corner, HotCorner::TopLeft);
        assert_eq!(fired.action, InputAction::MissionControl);
        // Does not fire again until the pointer leaves and returns.
        assert!(detector.poll(now + Duration::from_millis(500)).is_none());
    }

    #[test]
    fn every_action_round_trips_through_its_stable_id() {
        for action in [
            HotCornerAction::None,
            HotCornerAction::MissionControl,
            HotCornerAction::NotificationCenter,
            HotCornerAction::DesktopReveal,
            HotCornerAction::LockScreen,
            HotCornerAction::Applications,
            HotCornerAction::ControlCenter,
            HotCornerAction::ShowDock,
            HotCornerAction::MenuBar,
            HotCornerAction::FocusDock,
        ] {
            assert_eq!(HotCornerAction::parse(action.id()), Some(action));
        }
        assert_eq!(HotCornerAction::parse("open-sesame"), None);
    }

    #[test]
    fn a_corner_modifier_gates_the_trigger() {
        let mut config = HotCornerConfig::default();
        config.modifiers[HotCorner::TopLeft.index()] = RoleMods::COMMAND;
        let mut detector = HotCornerDetector::new(config);
        let now = Instant::now();
        let loc = (1.0, 1.0).into();

        // Without Command the corner is not armed and cannot fire.
        assert!(detector.pointer_moved(loc, bounds(), now).is_none());
        assert!(!detector.is_armed());
        assert!(detector.poll(now + Duration::from_millis(300)).is_none());

        // Holding Command while the pointer is already in the corner arms it.
        detector.set_modifiers(RoleMods::COMMAND);
        assert!(detector.is_armed());
        let fired = detector
            .poll(now + Duration::from_millis(300))
            .expect("fires once the required modifier is held");
        assert_eq!(fired.action, InputAction::MissionControl);

        // Releasing the modifier clears the occupied corner.
        detector.set_modifiers(RoleMods::EMPTY);
        assert!(!detector.is_armed());
    }

    #[test]
    fn a_modifier_parse_rejects_unknown_roles() {
        assert_eq!(parse_hot_corner_modifier("none"), Some(RoleMods::EMPTY));
        assert_eq!(
            parse_hot_corner_modifier("command"),
            Some(RoleMods::COMMAND)
        );
        assert_eq!(parse_hot_corner_modifier("option"), Some(RoleMods::OPTION));
        assert_eq!(parse_hot_corner_modifier("hyper"), None);
    }

    #[test]
    fn the_configured_notification_center_corner_dispatches_its_action() {
        // The default top-right assignment is Notifications; the corner
        // must resolve to the same InputAction the keyboard chord uses.
        let mut detector = HotCornerDetector::default();
        let now = Instant::now();
        let loc = (1919.0, 1.0).into();
        assert!(detector.pointer_moved(loc, bounds(), now).is_none());
        let fired = detector
            .poll(now + Duration::from_millis(200))
            .expect("top-right fires after the default dwell");
        assert_eq!(fired.corner, HotCorner::TopRight);
        assert_eq!(fired.action, InputAction::NotificationCenter);
        assert_eq!(fired.action.name(), "notification-center");
    }

    #[test]
    fn unassigned_corners_never_fire() {
        let mut detector = HotCornerDetector::new(HotCornerConfig {
            assignments: [HotCornerAction::None; 4],
            dwell: Duration::ZERO,
            ..Default::default()
        });
        let now = Instant::now();
        assert!(detector
            .pointer_moved((1.0, 1.0).into(), bounds(), now)
            .is_none());
        assert!(!detector.is_armed());
    }

    #[test]
    fn crossing_a_corner_resets_the_dwell() {
        let mut detector = HotCornerDetector::new(HotCornerConfig {
            dwell: Duration::from_millis(100),
            ..Default::default()
        });
        let now = Instant::now();
        assert!(detector
            .pointer_moved((1.0, 1.0).into(), bounds(), now)
            .is_none());
        // Move away, then back: the dwell restarts.
        assert!(detector
            .pointer_moved((500.0, 500.0).into(), bounds(), now)
            .is_none());
        assert!(detector
            .pointer_moved((1.0, 1.0).into(), bounds(), now)
            .is_none());
        assert!(detector.poll(now + Duration::from_millis(50)).is_none());
    }
}
