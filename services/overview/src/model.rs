// SPDX-License-Identifier: MIT
//! The Mission Control / hot corners snapshot the pane and tile render,
//! decoded from one raw read.
//!
//! The model owns the projection a consumer should not repeat: it names the
//! four corners and the five assignable actions, folds the gesture-gating trio
//! into one value, and carries the runtime overview state. It never detects a
//! corner or runs a transition — that is the compositor's
//! `HotCornerDetector`/`OverviewMachine` — it is the data those publish.
//!
//! A running compositor always answers `Available`; the only adapter-level
//! absence is a missing bridge (`AdapterState::Unavailable`), which is a
//! normal hidden state, exactly as the input adapter treats a missing
//! libinput.
//!
//! The event half is a pure diff: [`MissionControlSnapshot::changes`] reports
//! what moved between two reads (an assignment changed, Mission Control
//! opened, the selection moved, …) without any polling.

use std::time::Duration;

use crate::source::MissionControlData;

/// One of the four screen corners.
///
/// The indices match the compositor's `HotCorner` ordering and the
/// `df_toplevel_manager` `hot_corner` enum, so a projection maps straight
/// onto the wire without a lookup table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HotCorner {
    /// The top-left corner.
    TopLeft,
    /// The top-right corner.
    TopRight,
    /// The bottom-left corner.
    BottomLeft,
    /// The bottom-right corner.
    BottomRight,
}

impl HotCorner {
    /// Every corner, in the wire order.
    pub const ALL: [HotCorner; 4] = [
        HotCorner::TopLeft,
        HotCorner::TopRight,
        HotCorner::BottomLeft,
        HotCorner::BottomRight,
    ];

    /// The array index for this corner.
    pub const fn index(self) -> usize {
        match self {
            HotCorner::TopLeft => 0,
            HotCorner::TopRight => 1,
            HotCorner::BottomLeft => 2,
            HotCorner::BottomRight => 3,
        }
    }

    /// The corner at `index`, if the index is in range.
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(HotCorner::TopLeft),
            1 => Some(HotCorner::TopRight),
            2 => Some(HotCorner::BottomLeft),
            3 => Some(HotCorner::BottomRight),
            _ => None,
        }
    }

    /// A human label for the pane.
    pub const fn label(self) -> &'static str {
        match self {
            HotCorner::TopLeft => "Top Left",
            HotCorner::TopRight => "Top Right",
            HotCorner::BottomLeft => "Bottom Left",
            HotCorner::BottomRight => "Bottom Right",
        }
    }
}

/// The action assigned to a hot corner.
///
/// The set mirrors the compositor's `HotCornerAction`; `None` is an
/// unassigned corner that does nothing (no accidental triggers, legacy T-14
/// FR-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum HotCornerAction {
    /// Nothing (the default for an unassigned corner).
    #[default]
    None,
    /// Open Mission Control.
    MissionControl,
    /// Open Notification Center.
    NotificationCenter,
    /// Reveal the desktop.
    DesktopReveal,
    /// Lock the screen.
    LockScreen,
}

impl HotCornerAction {
    /// Every action, unassigned first, in the order the pane lists them.
    pub const ALL: [HotCornerAction; 5] = [
        HotCornerAction::None,
        HotCornerAction::MissionControl,
        HotCornerAction::NotificationCenter,
        HotCornerAction::DesktopReveal,
        HotCornerAction::LockScreen,
    ];

    /// The pane/popup label.
    pub const fn label(self) -> &'static str {
        match self {
            HotCornerAction::None => "–",
            HotCornerAction::MissionControl => "Mission Control",
            HotCornerAction::NotificationCenter => "Notification Center",
            HotCornerAction::DesktopReveal => "Desktop",
            HotCornerAction::LockScreen => "Lock Screen",
        }
    }

    /// The stable id used by the settings key and the wire.
    pub const fn id(self) -> &'static str {
        match self {
            HotCornerAction::None => "none",
            HotCornerAction::MissionControl => "mission-control",
            HotCornerAction::NotificationCenter => "notification-center",
            HotCornerAction::DesktopReveal => "desktop-reveal",
            HotCornerAction::LockScreen => "lock-screen",
        }
    }

    /// Parse the stable id back to an action.
    pub fn from_id(id: &str) -> Option<Self> {
        HotCornerAction::ALL
            .into_iter()
            .find(|action| action.id() == id)
    }

    /// Whether the corner performs an action (the opposite of `None`).
    pub const fn is_assigned(self) -> bool {
        !matches!(self, HotCornerAction::None)
    }
}

/// The gesture-gating trio the compositor applies live.
///
/// This is the same input policy `df_toplevel_manager.set_input_policy`
/// carries; the adapter reports it beside the corner map because a gesture, a
/// corner, a shortcut, and the menu-bar button all drive the one overview
/// machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GestureGating {
    /// The gesture-recognition master switch.
    pub enabled: bool,
    /// Whether a horizontal swipe switches Spaces.
    pub space_switch: bool,
    /// Whether a vertical swipe opens Mission Control.
    pub mission_control: bool,
}

impl GestureGating {
    /// Whether any gesture can open Mission Control.
    pub const fn opens_mission_control(self) -> bool {
        self.enabled && self.mission_control
    }
}

/// The runtime state of the one overview machine.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OverviewState {
    /// Whether the overview (Mission Control) is open.
    pub active: bool,
    /// The selected window while the overview is open, if any.
    pub selected_window: Option<String>,
    /// The number of Spaces (workspaces).
    pub spaces: u32,
    /// The number of visible windows in the overview.
    pub windows: u32,
}

impl OverviewState {
    /// Whether a window is selected in the overview.
    pub fn has_selection(&self) -> bool {
        self.selected_window.is_some()
    }
}

/// The Mission Control / hot corners snapshot a pane or status slot renders.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionControlSnapshot {
    /// The assignment per corner, indexed by [`HotCorner::index`].
    pub corners: [HotCornerAction; 4],
    /// Distance from the corner (logical px) that counts as "in" it.
    pub inset: f64,
    /// How long the pointer must rest in a corner before it fires.
    pub dwell: Duration,
    /// The live gesture gating.
    pub gestures: GestureGating,
    /// The live overview state.
    pub overview: OverviewState,
}

impl MissionControlSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &MissionControlData) -> Self {
        MissionControlSnapshot {
            corners: data.corners,
            inset: data.inset,
            dwell: Duration::from_millis(u64::from(data.dwell_ms)),
            gestures: GestureGating {
                enabled: data.gestures_enabled,
                space_switch: data.gesture_space_switch,
                mission_control: data.gesture_mission_control,
            },
            overview: OverviewState {
                active: data.overview_active,
                selected_window: data.selected_window.clone(),
                spaces: data.spaces,
                windows: data.windows,
            },
        }
    }

    /// The action assigned to `corner`.
    pub fn corner(&self, corner: HotCorner) -> HotCornerAction {
        self.corners[corner.index()]
    }

    /// The corners that perform an action, in corner order.
    pub fn assigned_corners(&self) -> Vec<(HotCorner, HotCornerAction)> {
        HotCorner::ALL
            .into_iter()
            .filter_map(|corner| {
                let action = self.corner(corner);
                action.is_assigned().then_some((corner, action))
            })
            .collect()
    }

    /// The corners assigned to Mission Control.
    pub fn mission_control_corners(&self) -> Vec<HotCorner> {
        self.assigned_corners()
            .into_iter()
            .filter_map(|(corner, action)| {
                (action == HotCornerAction::MissionControl).then_some(corner)
            })
            .collect()
    }

    /// Whether Mission Control has any pointer/gesture trigger at all.
    pub fn mission_control_reachable(&self) -> bool {
        self.gestures.opens_mission_control() || !self.mission_control_corners().is_empty()
    }

    /// A one-line label for the status item: what Mission Control answers to.
    pub fn label(&self) -> String {
        let corners = self.mission_control_corners().len();
        match (self.gestures.opens_mission_control(), corners) {
            (true, 0) => "Gesture".to_owned(),
            (true, n) => format!("Gesture, {n} corner(s)"),
            (false, 0) => "No trigger".to_owned(),
            (false, n) => format!("{n} corner(s)"),
        }
    }

    /// The design-system glyph for the status tile.
    pub fn glyph(&self) -> &'static str {
        "overview"
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// corner assignments, then gesture gating, then the overview runtime.
    ///
    /// This is the "event" half of the adapter: the bridge can diff two reads
    /// to learn what moved without polling each field. An instantaneous
    /// *trigger* is not a diff — it is reported through the adapter's trigger
    /// outbox.
    pub fn changes(&self, previous: &MissionControlSnapshot) -> Vec<MissionControlChange> {
        let mut changes = Vec::new();
        for corner in HotCorner::ALL {
            let from = previous.corner(corner);
            let to = self.corner(corner);
            if from != to {
                changes.push(MissionControlChange::CornerAssignment { corner, from, to });
            }
        }
        if self.gestures != previous.gestures {
            changes.push(MissionControlChange::GestureGating {
                from: previous.gestures,
                to: self.gestures,
            });
        }
        if self.overview.active != previous.overview.active {
            changes.push(if self.overview.active {
                MissionControlChange::OverviewOpened
            } else {
                MissionControlChange::OverviewClosed
            });
        }
        if self.overview.selected_window != previous.overview.selected_window {
            changes.push(MissionControlChange::SelectionChanged {
                from: previous.overview.selected_window.clone(),
                to: self.overview.selected_window.clone(),
            });
        }
        if self.overview.spaces != previous.overview.spaces {
            changes.push(MissionControlChange::SpaceCountChanged {
                from: previous.overview.spaces,
                to: self.overview.spaces,
            });
        }
        if self.overview.windows != previous.overview.windows {
            changes.push(MissionControlChange::WindowCountChanged {
                from: previous.overview.windows,
                to: self.overview.windows,
            });
        }
        changes
    }
}

/// A hot corner that fired.
///
/// A trigger is instantaneous, so it is never part of a snapshot; the adapter's
/// trigger outbox carries it to the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotCornerTrigger {
    /// The corner the pointer rested in.
    pub corner: HotCorner,
    /// The action the corner performs.
    pub action: HotCornerAction,
}

/// A change between two Mission Control snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionControlChange {
    /// A corner's assignment changed.
    CornerAssignment {
        /// The corner.
        corner: HotCorner,
        /// The previous assignment.
        from: HotCornerAction,
        /// The new assignment.
        to: HotCornerAction,
    },
    /// The gesture-gating trio changed.
    GestureGating {
        /// The previous gating.
        from: GestureGating,
        /// The new gating.
        to: GestureGating,
    },
    /// Mission Control opened.
    OverviewOpened,
    /// Mission Control closed.
    OverviewClosed,
    /// The overview selection moved.
    SelectionChanged {
        /// The previous selection.
        from: Option<String>,
        /// The new selection.
        to: Option<String>,
    },
    /// The Space count changed.
    SpaceCountChanged {
        /// The previous count.
        from: u32,
        /// The new count.
        to: u32,
    },
    /// The visible-window count changed.
    WindowCountChanged {
        /// The previous count.
        from: u32,
        /// The new count.
        to: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> MissionControlData {
        MissionControlData::default()
    }

    #[test]
    fn corners_index_and_round_trip() {
        for corner in HotCorner::ALL {
            assert_eq!(HotCorner::from_index(corner.index()), Some(corner));
        }
        assert_eq!(HotCorner::from_index(4), None);
        assert_eq!(HotCorner::TopLeft.label(), "Top Left");
    }

    #[test]
    fn actions_round_trip_through_their_ids() {
        for action in HotCornerAction::ALL {
            assert_eq!(HotCornerAction::from_id(action.id()), Some(action));
        }
        assert_eq!(HotCornerAction::from_id("bogus"), None);
        assert!(!HotCornerAction::None.is_assigned());
        assert!(HotCornerAction::MissionControl.is_assigned());
    }

    #[test]
    fn the_default_snapshot_has_mission_control_on_the_top_left() {
        let snapshot = MissionControlSnapshot::from_data(&data());
        assert_eq!(
            snapshot.corner(HotCorner::TopLeft),
            HotCornerAction::MissionControl
        );
        assert_eq!(
            snapshot.corner(HotCorner::TopRight),
            HotCornerAction::NotificationCenter
        );
        assert_eq!(snapshot.mission_control_corners(), vec![HotCorner::TopLeft]);
        assert!(snapshot.mission_control_reachable());
        assert_eq!(snapshot.label(), "Gesture, 1 corner(s)");
        assert_eq!(snapshot.dwell, Duration::from_millis(150));
        assert_eq!(snapshot.inset, 4.0);
    }

    #[test]
    fn an_unassigned_corner_is_not_listed() {
        let mut raw = data();
        raw.corners = [HotCornerAction::None; 4];
        let snapshot = MissionControlSnapshot::from_data(&raw);
        assert!(snapshot.assigned_corners().is_empty());
        assert!(snapshot.mission_control_corners().is_empty());
    }

    #[test]
    fn gestures_can_reach_mission_control_without_a_corner() {
        let mut raw = data();
        raw.corners = [HotCornerAction::None; 4];
        assert!(MissionControlSnapshot::from_data(&raw).mission_control_reachable());
        assert_eq!(MissionControlSnapshot::from_data(&raw).label(), "Gesture");
    }

    #[test]
    fn disabling_both_leaves_no_trigger() {
        let mut raw = data();
        raw.corners = [HotCornerAction::None; 4];
        raw.gesture_mission_control = false;
        let snapshot = MissionControlSnapshot::from_data(&raw);
        assert!(!snapshot.mission_control_reachable());
        assert_eq!(snapshot.label(), "No trigger");
    }

    #[test]
    fn gesture_gating_requires_both_switches() {
        assert!(GestureGating {
            enabled: true,
            mission_control: true,
            ..Default::default()
        }
        .opens_mission_control());
        assert!(!GestureGating {
            enabled: false,
            mission_control: true,
            ..Default::default()
        }
        .opens_mission_control());
    }

    #[test]
    fn a_corner_reassignment_is_a_change() {
        let previous = MissionControlSnapshot::from_data(&data());
        let mut raw = data();
        raw.corners[HotCorner::TopLeft.index()] = HotCornerAction::None;
        raw.corners[HotCorner::BottomRight.index()] = HotCornerAction::MissionControl;
        let next = MissionControlSnapshot::from_data(&raw);
        let changes = next.changes(&previous);
        assert_eq!(
            changes,
            vec![
                MissionControlChange::CornerAssignment {
                    corner: HotCorner::TopLeft,
                    from: HotCornerAction::MissionControl,
                    to: HotCornerAction::None,
                },
                MissionControlChange::CornerAssignment {
                    corner: HotCorner::BottomRight,
                    from: HotCornerAction::LockScreen,
                    to: HotCornerAction::MissionControl,
                },
            ]
        );
    }

    #[test]
    fn the_overview_open_selection_and_counts_are_changes() {
        let previous = MissionControlSnapshot::from_data(&data());
        let mut raw = data();
        raw.overview_active = true;
        raw.selected_window = Some("app:1".to_owned());
        raw.spaces = 4;
        raw.windows = 2;
        let next = MissionControlSnapshot::from_data(&raw);
        let changes = next.changes(&previous);
        assert!(changes.contains(&MissionControlChange::OverviewOpened));
        assert!(changes.contains(&MissionControlChange::SelectionChanged {
            from: None,
            to: Some("app:1".to_owned()),
        }));
        assert!(changes.contains(&MissionControlChange::SpaceCountChanged { from: 3, to: 4 }));
        assert!(changes.contains(&MissionControlChange::WindowCountChanged { from: 0, to: 2 }));
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = MissionControlSnapshot::from_data(&data());
        assert!(snapshot.changes(&snapshot).is_empty());
    }
}
