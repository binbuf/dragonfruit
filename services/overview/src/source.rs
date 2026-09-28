// SPDX-License-Identifier: MIT
//! The transport seam: the raw Mission Control / hot corners read and its mock.
//!
//! A [`MissionControlSource`] is the only thing that talks to the host stack.
//! Mission Control and hot corners are **compositor-native**: the compositor
//! already owns the corner detector (`compositor/src/input/hot_corners.rs`) and
//! the single overview state machine (`compositor/src/overview/mod.rs`), and
//! the shell learns about both over the private `df_toplevel_manager` bridge
//! (`hot_corner` and `overview_changed`). This crate does not re-detect a
//! corner or re-run a transition; it is the **projection** — the trigger
//! configuration, the runtime overview state, and the trigger event stream,
//! delivered through a seam so the shell bridge (T-15.5b) and CI's
//! [`MockMissionControl`] answer identically.
//!
//! The adapter is read-only for runtime state: the overview machine is the
//! compositor's source of truth (the one-machine rule,
//! [03-workspaces.md](../../../docs/design/03-workspaces.md)), and the shell
//! only mirrors it. Trigger *configuration* is a durable preference; it is
//! owned by `settingsd` and applied live by the compositor, exactly as the
//! input pointer keys are ([adr/0124]). Nothing above this crate sees a
//! Wayland or Smithay type.
//!
//! [03-workspaces.md]: ../../../docs/design/03-workspaces.md
//! [adr/0124]: ../../../docs/design/adr/0124-input-device-adapter.md

use std::collections::VecDeque;

use dragonfruit_system_adapters::AdapterError;

pub use crate::model::{HotCorner, HotCornerAction};

/// The raw result of one Mission Control / hot corners read: the compositor's
/// current trigger configuration and overview state.
///
/// The four corner assignments are indexed by [`HotCorner::index`]. The
/// gesture-gating trio is the same input policy the compositor applies from
/// `set_input_policy` (`gestures.enabled`, `gestures.spaceSwitch`,
/// `gestures.missionControl`); the adapter reports it beside the corner
/// assignments because a trigger and a gesture drive the same machine.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionControlData {
    /// The assignment per corner, indexed by [`HotCorner::index`].
    pub corners: [HotCornerAction; 4],
    /// Distance from the corner (logical px) that counts as "in" it.
    pub inset: f64,
    /// How long the pointer must rest in a corner before it fires.
    pub dwell_ms: u32,
    /// The gesture-recognition master switch.
    pub gestures_enabled: bool,
    /// Whether a horizontal swipe switches Spaces.
    pub gesture_space_switch: bool,
    /// Whether a vertical swipe opens Mission Control.
    pub gesture_mission_control: bool,
    /// Whether the overview (Mission Control) is open.
    pub overview_active: bool,
    /// The selected window while the overview is open, if any.
    pub selected_window: Option<String>,
    /// The number of Spaces (workspaces).
    pub spaces: u32,
    /// The number of visible windows in the overview.
    pub windows: u32,
}

impl Default for MissionControlData {
    /// The compositor's tasteful defaults, mirroring
    /// `HotCornerConfig::default()` and the schema defaults for the gesture
    /// keys. A session that runs the compositor always has a coherent corner
    /// map.
    fn default() -> Self {
        MissionControlData {
            corners: [
                HotCornerAction::MissionControl,
                HotCornerAction::NotificationCenter,
                HotCornerAction::DesktopReveal,
                HotCornerAction::LockScreen,
            ],
            inset: 4.0,
            dwell_ms: 150,
            gestures_enabled: true,
            gesture_space_switch: true,
            gesture_mission_control: true,
            overview_active: false,
            selected_window: None,
            spaces: 3,
            windows: 0,
        }
    }
}

/// Reads the Mission Control / hot corners host stack.
///
/// The read result is a three-way answer, exactly as the adapter contract needs
/// it:
///
/// * `Ok(Some(data))` — the bridge/compositor answered; `data` is the current
///   trigger configuration and overview state.
/// * `Ok(None)` — the bridge is absent (the shell has no `df_toplevel_manager`
///   global, or the compositor is not the one we speak to). A normal state;
///   the item hides.
/// * `Err(error)` — the bridge is present but the read failed; the item shows
///   visible and inert with the message.
pub trait MissionControlSource {
    /// One read of the host stack.
    fn read(&mut self) -> Result<Option<MissionControlData>, AdapterError>;

    /// Corner triggers observed since the last call, oldest first.
    ///
    /// A trigger is instantaneous — it is not part of a snapshot — so the
    /// production bridge drains its `hot_corner` events here and the mock
    /// queues them with [`MockMissionControl::trigger`]. The default is no
    /// triggers, so a source that does not publish them still satisfies the
    /// trait.
    fn take_triggers(&mut self) -> Vec<HotCorner> {
        Vec::new()
    }
}

/// A fixture-backed source with a simulated host-stack lifecycle.
///
/// The mock is the CI path: it serves [`MissionControlData`] with no
/// compositor and no Wayland connection, `kill`/`restart` exercise absence and
/// re-subscribe, and `trigger` queues a hot-corner event.
#[derive(Debug, Clone, PartialEq)]
pub struct MockMissionControl {
    present: bool,
    data: Option<MissionControlData>,
    failure: Option<AdapterError>,
    triggers: VecDeque<HotCorner>,
    reads: u32,
}

impl MockMissionControl {
    /// A host stack that is not available.
    pub fn absent() -> Self {
        MockMissionControl {
            present: false,
            data: None,
            failure: None,
            triggers: VecDeque::new(),
            reads: 0,
        }
    }

    /// A present host stack that answers with `data`.
    pub fn present(data: MissionControlData) -> Self {
        MockMissionControl {
            present: true,
            data: Some(data),
            failure: None,
            triggers: VecDeque::new(),
            reads: 0,
        }
    }

    /// A present host stack that fails every read (e.g. the bridge went
    /// unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockMissionControl {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            triggers: VecDeque::new(),
            reads: 0,
        }
    }

    /// The host stack reports fresh data.
    pub fn push(&mut self, data: MissionControlData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The host stack reports a hot-corner trigger fired.
    pub fn trigger(&mut self, corner: HotCorner) {
        self.triggers.push_back(corner);
    }

    /// The host stack becomes unavailable.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host stack comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated host stack is available.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// The corner triggers not yet drained.
    pub fn pending_triggers(&self) -> usize {
        self.triggers.len()
    }
}

impl MissionControlSource for MockMissionControl {
    fn read(&mut self) -> Result<Option<MissionControlData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn take_triggers(&mut self) -> Vec<HotCorner> {
        self.triggers.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> MissionControlData {
        MissionControlData {
            overview_active: true,
            ..MissionControlData::default()
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockMissionControl::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockMissionControl::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockMissionControl::failing("bridge: timeout");
        assert_eq!(mock.read().unwrap_err().message(), "bridge: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockMissionControl::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn triggers_queue_and_drain_in_order() {
        let mut mock = MockMissionControl::present(data());
        mock.trigger(HotCorner::TopLeft);
        mock.trigger(HotCorner::BottomRight);
        assert_eq!(mock.pending_triggers(), 2);
        assert_eq!(
            mock.take_triggers(),
            vec![HotCorner::TopLeft, HotCorner::BottomRight]
        );
        assert_eq!(mock.pending_triggers(), 0);
    }

    #[test]
    fn the_default_data_is_the_compositor_default_map() {
        let data = MissionControlData::default();
        assert_eq!(
            data.corners[HotCorner::TopLeft.index()],
            HotCornerAction::MissionControl
        );
        assert_eq!(
            data.corners[HotCorner::TopRight.index()],
            HotCornerAction::NotificationCenter
        );
        assert_eq!(
            data.corners[HotCorner::BottomLeft.index()],
            HotCornerAction::DesktopReveal
        );
        assert_eq!(
            data.corners[HotCorner::BottomRight.index()],
            HotCornerAction::LockScreen
        );
        assert!(data.gestures_enabled && data.gesture_mission_control);
    }
}
