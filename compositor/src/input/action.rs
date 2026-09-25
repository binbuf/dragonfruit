// SPDX-License-Identifier: MIT
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! The compositor-level input vocabulary shared by every trigger (T-03).
//!
//! Keyboard shortcuts, trackpad gestures, and hot corners all resolve to
//! the same [`InputAction`] and are dispatched through the same path, so a
//! Mission Control trigger behaves identically no matter how it started
//! ([03-workspaces.md](../../docs/design/03-workspaces.md)). The shell
//! consumes the resulting event stream over the private protocol (T-07);
//! until that lands, [`super::dispatch::InputDispatch`] is the single
//! outbox and audit log.

/// A compositor-level action produced by any input trigger.
///
/// Actions marked "progress-driven" are continuous 0→1 transitions (the
/// workspace switch and Mission Control / Desktop Reveal pipelines); the
/// rest are discrete. Both kinds flow through the same dispatch path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAction {
    /// Move to the next Space.
    WorkspaceNext,
    /// Move to the previous Space.
    WorkspacePrev,
    /// Activate a Space by index.
    WorkspaceActivate(usize),
    /// Toggle the Mission Control overview.
    MissionControl,
    /// Open the Cmd-Tab app switcher.
    AppSwitcher,
    /// Cycle windows within the switcher's selected app (Cmd+`, T-06.2b).
    AppSwitcherWindow,
    /// Take a screenshot (portal capture path).
    Screenshot,
    /// Open the notification center.
    NotificationCenter,
    /// Reveal the desktop background.
    DesktopReveal,
    /// Lock the session.
    LockScreen,
    /// Move keyboard focus into the Dock (T-10 section 20). The compositor
    /// focuses the `dock` chrome surface; the shell then owns in-Dock
    /// navigation.
    FocusDock,
    /// Toggle the Dock's auto-hide (T-10 section 15). The shell owns the
    /// setting; the compositor only routes the action.
    ToggleDock,
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
            InputAction::NotificationCenter => "notification-center",
            InputAction::DesktopReveal => "desktop-reveal",
            InputAction::LockScreen => "lock-screen",
            InputAction::FocusDock => "focus-dock",
            InputAction::ToggleDock => "toggle-dock",
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
        }
    }
}
