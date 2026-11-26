// SPDX-License-Identifier: GPL-3.0-only
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! The compositor-level input vocabulary shared by every trigger (T-03).
//!
//! Keyboard shortcuts, trackpad gestures, and hot corners all resolve to
//! the same [`InputAction`] and are dispatched through the same path, so a
//! Overview trigger behaves identically no matter how it started
//! ([03-workspaces.md](../../docs/design/03-workspaces.md)). The shell
//! consumes the resulting event stream over the private protocol (T-07);
//! until that lands, [`super::dispatch::InputDispatch`] is the single
//! outbox and audit log.

/// A compositor-level action produced by any input trigger.
///
/// Actions marked "progress-driven" are continuous 0→1 transitions (the
/// workspace switch and Overview / Desktop Reveal pipelines); the
/// rest are discrete. Both kinds flow through the same dispatch path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAction {
    /// Move to the next Space.
    WorkspaceNext,
    /// Move to the previous Space.
    WorkspacePrev,
    /// Activate a Space by index.
    WorkspaceActivate(usize),
    /// Toggle the Overview overview.
    MissionControl,
    /// Open the Cmd-Tab app switcher.
    AppSwitcher,
    /// Cycle windows within the switcher's selected app (Cmd+`, T-06.2b).
    AppSwitcherWindow,
    /// Take a full-output screenshot (portal capture path; Cmd+Shift+3).
    Screenshot,
    /// Take a region screenshot (portal capture path; Cmd+Shift+4).
    ScreenshotRegion,
    /// Open the Notifications.
    NotificationCenter,
    /// Reveal the desktop background.
    DesktopReveal,
    /// Lock the session.
    LockScreen,
    /// End the session through logind (T313). The shell owns the confirmation
    /// and the logind call; the compositor only routes the chord.
    LogOut,
    /// Drop the trusted shell's client connection so systemd restarts it
    /// (T314). This is the manual recovery chord for a hung shell; the
    /// compositor reissues the one-time launch token on disconnect.
    RestartShell,
    /// Move keyboard focus into the Dock (T-10 section 20). The compositor
    /// focuses the `dock` chrome surface; the shell then owns in-Dock
    /// navigation.
    FocusDock,
    /// Reveal and focus the menu bar (T201). The compositor owns the reveal
    /// and focus transfer; the shell then owns in-bar navigation. This is the
    /// keyboard half of the auto-hide reveal contract.
    FocusMenuBar,
    /// Toggle the Dock's auto-hide (T-10 section 15). The shell owns the
    /// setting; the compositor only routes the action.
    ToggleDock,
    /// Toggle the Quick Settings panel (T-11.3a). The shell owns the panel;
    /// the compositor only routes the action.
    ControlCenter,
    /// Toggle the Applications drawer (T-19.2). The shell owns the launcher;
    /// the compositor only routes the action.
    ShowApps,
    /// T275: the user-recorded accessibility shortcut. The shell forwards the
    /// chord from `accessibility.shortcut`; the compositor binds it to this
    /// action and the shell opens the Accessibility pane.
    AccessibilityShortcut,
    /// T321: raise the output volume by one 5% step and show the OSD. The
    /// shell owns the audio write and the presentation; the compositor only
    /// routes the XF86 key before the focused client sees it.
    VolumeUp,
    /// T321: lower the output volume by one 5% step and show the OSD.
    VolumeDown,
    /// T321: toggle output mute and show the OSD.
    Mute,
    /// T321: raise the panel brightness by one 5% step and show the OSD.
    BrightnessUp,
    /// T321: lower the panel brightness by one 5% step and show the OSD.
    BrightnessDown,
    /// T315: the idle chain dimmed the screen. The shell applies the effect.
    IdleDim,
    /// T315: the idle chain blanked the screen. The shell applies the effect.
    IdleBlank,
    /// T315: the idle chain locked the session. The shell applies the effect.
    IdleLock,
    /// T315: the idle chain suspended the session. The shell applies the
    /// effect; the compositor only owns the timer.
    IdleSuspend,
    /// T315: input (or a new inhibitor) woke the chain from a stage. The shell
    /// restores the stage's effect.
    IdleAwake,
}

impl InputAction {
    /// Whether this action is a continuous progress pipeline (T-11
    /// consumes the 0→1 events) rather than a discrete toggle.
    pub const fn is_progress_driven(self) -> bool {
        matches!(
            self,
            InputAction::WorkspaceNext
                | InputAction::WorkspacePrev
                | InputAction::MissionControl
                | InputAction::DesktopReveal
        )
    }

    /// A stable identifier used in the audit log and shell protocol.
    pub const fn name(self) -> &'static str {
        match self {
            InputAction::WorkspaceNext => "workspace-next",
            InputAction::WorkspacePrev => "workspace-prev",
            InputAction::WorkspaceActivate(_) => "workspace-activate",
            InputAction::MissionControl => "mission-control",
            InputAction::AppSwitcher => "app-switcher",
            InputAction::AppSwitcherWindow => "app-switcher-window",
            InputAction::Screenshot => "screenshot",
            InputAction::ScreenshotRegion => "screenshot-region",
            InputAction::NotificationCenter => "notification-center",
            InputAction::DesktopReveal => "desktop-reveal",
            InputAction::LockScreen => "lock-screen",
            InputAction::LogOut => "log-out",
            InputAction::RestartShell => "restart-shell",
            InputAction::FocusDock => "focus-dock",
            InputAction::FocusMenuBar => "focus-menu-bar",
            InputAction::ToggleDock => "toggle-dock",
            InputAction::ControlCenter => "control-center",
            InputAction::ShowApps => "show-apps",
            InputAction::AccessibilityShortcut => "accessibility-shortcut",
            InputAction::VolumeUp => "volume-up",
            InputAction::VolumeDown => "volume-down",
            InputAction::Mute => "mute",
            InputAction::BrightnessUp => "brightness-up",
            InputAction::BrightnessDown => "brightness-down",
            InputAction::IdleDim => "idle-dim",
            InputAction::IdleBlank => "idle-blank",
            InputAction::IdleLock => "idle-lock",
            InputAction::IdleSuspend => "idle-suspend",
            InputAction::IdleAwake => "idle-awake",
        }
    }
}

/// The kind of gesture recognition that produced an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GestureKind {
    /// A multi-finger swipe (`fingers` is the finger count).
    Swipe { fingers: u32 },
    /// A multi-finger pinch (`fingers` is the finger count).
    Pinch { fingers: u32 },
}

impl GestureKind {
    /// The finger count regardless of kind.
    pub const fn fingers(self) -> u32 {
        match self {
            GestureKind::Swipe { fingers } | GestureKind::Pinch { fingers } => fingers,
        }
    }
}

/// One of the four screen corners of an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl HotCorner {
    /// All corners in a stable order (matches the config array).
    pub const ALL: [HotCorner; 4] = [
        HotCorner::TopLeft,
        HotCorner::TopRight,
        HotCorner::BottomLeft,
        HotCorner::BottomRight,
    ];

    /// The index into the configuration array.
    pub const fn index(self) -> usize {
        match self {
            HotCorner::TopLeft => 0,
            HotCorner::TopRight => 1,
            HotCorner::BottomLeft => 2,
            HotCorner::BottomRight => 3,
        }
    }
}

/// Where an action came from — the same action has one event shape
/// regardless of trigger ([04-shell.md](../../docs/design/04-shell.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerKind {
    Keyboard,
    Gesture(GestureKind),
    HotCorner(HotCorner),
    /// A shell-driven trigger (private protocol, T-07).
    Shell,
    /// A sandboxed application registration (portal, T-27).
    Portal,
    /// The compositor's own idle timer (T315), not a user trigger.
    Idle,
}

impl TriggerKind {
    /// A short label for logs.
    pub const fn name(self) -> &'static str {
        match self {
            TriggerKind::Keyboard => "keyboard",
            TriggerKind::Gesture(_) => "gesture",
            TriggerKind::HotCorner(_) => "hot-corner",
            TriggerKind::Shell => "shell",
            TriggerKind::Portal => "portal",
            TriggerKind::Idle => "idle",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idle::IdleStage;

    #[test]
    fn idle_actions_use_the_stage_action_strings() {
        assert_eq!(InputAction::IdleDim.name(), IdleStage::Dim.action_name());
        assert_eq!(
            InputAction::IdleBlank.name(),
            IdleStage::Blank.action_name()
        );
        assert_eq!(InputAction::IdleLock.name(), IdleStage::Lock.action_name());
        assert_eq!(
            InputAction::IdleSuspend.name(),
            IdleStage::Suspend.action_name()
        );
        assert_eq!(
            InputAction::IdleAwake.name(),
            IdleStage::Active.action_name()
        );
        assert_eq!(TriggerKind::Idle.name(), "idle");
        assert!(!InputAction::IdleDim.is_progress_driven());
    }
}
