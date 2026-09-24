// SPDX-License-Identifier: MIT
//! The app-switcher state machine (T-06.1).
//!
//! The compositor owns the Cmd-Tab chord (through the global shortcut engine,
//! never a client, [02-compositor.md](../../docs/design/02-compositor.md)) and
//! the window/app lists; the shell only renders the overlay from the
//! `df_toplevel_manager.app_switcher` projection ([04-shell.md]).
//!
//! The machine is deliberately pure: it holds an app-recency snapshot with
//! each app's most-recent window and the current selection. It makes the
//! open/cycle/reverse/commit/cancel rules unit-testable without a live
//! compositor or `Window`:
//!
//! * **open** on Cmd+Tab (the first selection *skips the focused app*, so a
//!   release actually switches the mac-way; a single app selects itself),
//! * **cycle** on Tab / forward, Shift+Tab and arrows / backward,
//! * **commit exactly once** on Command release (clears the active state), and
//! * **cancel** on Escape with no selection applied.
//!
//! Recency comes from [`crate::window::WindowModel::recency`]; the machine
//! never groups or sorts on its own.

#![allow(dead_code)] // The shell overlay (T-06.2a) consumes the projection.

use crate::window::WindowId;

/// One app in recency order, carrying its most recent window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherApp {
    /// The raw `app_id` (Wayland) / resolved `WM_CLASS` (X11); T-23 later
    /// supplies richer identity.
    pub app_id: String,
    /// The most recent window of the app — what a commit activates.
    pub window: WindowId,
}

impl SwitcherApp {
    pub fn new(app_id: impl Into<String>, window: WindowId) -> Self {
        SwitcherApp {
            app_id: app_id.into(),
            window,
        }
    }
}

/// The direction of one cycle step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchStep {
    /// The next (older) app: Tab / Right / Down.
    Forward,
    /// The previous (newer) app: Shift+Tab / Left / Up.
    Backward,
}

impl SwitchStep {
    /// The signed direction the private protocol carries.
    pub const fn direction(self) -> i32 {
        match self {
            SwitchStep::Forward => 1,
            SwitchStep::Backward => -1,
        }
    }
}

/// The single app-switcher state machine.
///
/// `active == false` is the only resting state; [`Self::commit`] returns the
/// chosen app exactly once because it clears `active` before returning.
#[derive(Debug, Clone, Default)]
pub struct AppSwitcher {
    entries: Vec<SwitcherApp>,
    selected: usize,
    direction: i32,
    active: bool,
}

impl AppSwitcher {
    pub fn new() -> Self {
        AppSwitcher::default()
    }

    /// Whether the overlay is up.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The app-recency snapshot, most recent first.
    pub fn entries(&self) -> &[SwitcherApp] {
        &self.entries
    }

    /// The selected index, present only while active.
    pub fn selected_index(&self) -> Option<usize> {
        self.active.then_some(self.selected)
    }

    /// The selected app id, if any.
    pub fn selected_app(&self) -> Option<&str> {
        self.selected_index()
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.app_id.as_str())
    }

    /// The selected app's most-recent window, if any.
    pub fn selected_window(&self) -> Option<WindowId> {
        self.selected_index()
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.window)
    }

    /// The last cycle direction (`0` at rest); the private protocol's
    /// `direction` argument.
    pub fn direction(&self) -> i32 {
        self.direction
    }

    /// Open (or re-seed) the switcher with `entries` in recency order.
    ///
    /// The first selection steps one app away from `focused` in `step`'s
    /// direction so a release switches apps. An empty list keeps the switcher
    /// closed (there is nothing to switch to). Returns whether it is active.
    pub fn open(
        &mut self,
        entries: Vec<SwitcherApp>,
        focused: Option<&str>,
        step: SwitchStep,
    ) -> bool {
        self.entries = entries;
        self.direction = step.direction();
        if self.entries.is_empty() {
            self.reset();
            return false;
        }
        let len = self.entries.len();
        self.selected = match focused
            .and_then(|app| self.entries.iter().position(|entry| entry.app_id == app))
        {
            Some(index) if len > 1 => {
                let delta = step.direction();
                (((index as i32 + delta) % len as i32 + len as i32) % len as i32) as usize
            }
            _ => 0,
        };
        self.active = true;
        true
    }

    /// Move the selection one step, wrapping at both ends. A no-op when the
    /// switcher is closed.
    pub fn step(&mut self, step: SwitchStep) {
        if !self.active || self.entries.is_empty() {
            return;
        }
        let len = self.entries.len() as i32;
        self.selected = (((self.selected as i32 + step.direction()) % len + len) % len) as usize;
        self.direction = step.direction();
    }

    /// Commit the selection: return the chosen app and close the switcher.
    ///
    /// The active state is cleared before the value is handed back, so a
    /// second call (a stray second release, a duplicate event) returns `None`
    /// and can never activate twice.
    pub fn commit(&mut self) -> Option<SwitcherApp> {
        if !self.active {
            return None;
        }
        let entry = self.entries.get(self.selected).cloned();
        self.reset();
        entry
    }

    /// Cancel the switcher with no selection applied. Returns whether it was
    /// active (so the caller knows to broadcast the close).
    pub fn cancel(&mut self) -> bool {
        let was_active = self.active;
        self.reset();
        was_active
    }

    fn reset(&mut self) {
        self.active = false;
        self.entries.clear();
        self.selected = 0;
        self.direction = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<SwitcherApp> {
        vec![
            SwitcherApp::new("app.newest", WindowId(3)),
            SwitcherApp::new("app.middle", WindowId(2)),
            SwitcherApp::new("app.oldest", WindowId(1)),
        ]
    }

    #[test]
    fn open_skips_the_focused_app_forward_and_backward() {
        let mut forward = AppSwitcher::new();
        assert!(forward.open(entries(), Some("app.newest"), SwitchStep::Forward));
        assert!(forward.is_active());
        assert_eq!(forward.selected_app(), Some("app.middle"));
        assert_eq!(forward.selected_window(), Some(WindowId(2)));
        assert_eq!(forward.direction(), 1);

        // Backward from the newest wraps to the oldest.
        let mut backward = AppSwitcher::new();
        assert!(backward.open(entries(), Some("app.newest"), SwitchStep::Backward));
        assert_eq!(backward.selected_app(), Some("app.oldest"));
        assert_eq!(backward.direction(), -1);
    }

    #[test]
    fn open_with_one_app_selects_itself() {
        let mut switcher = AppSwitcher::new();
        assert!(switcher.open(
            vec![SwitcherApp::new("only", WindowId(9))],
            Some("only"),
            SwitchStep::Forward
        ));
        assert_eq!(switcher.selected_app(), Some("only"));
        assert_eq!(switcher.selected_window(), Some(WindowId(9)));
    }

    #[test]
    fn open_with_no_apps_stays_closed() {
        let mut switcher = AppSwitcher::new();
        assert!(!switcher.open(Vec::new(), Some("app"), SwitchStep::Forward));
        assert!(!switcher.is_active());
        assert_eq!(switcher.selected_app(), None);
    }

    #[test]
    fn cycling_wraps_in_both_directions() {
        let mut switcher = AppSwitcher::new();
        switcher.open(entries(), Some("app.newest"), SwitchStep::Forward);
        assert_eq!(switcher.selected_app(), Some("app.middle"));
        switcher.step(SwitchStep::Forward);
        assert_eq!(switcher.selected_app(), Some("app.oldest"));
        switcher.step(SwitchStep::Forward);
        assert_eq!(switcher.selected_app(), Some("app.newest"), "wraps forward");
        switcher.step(SwitchStep::Backward);
        assert_eq!(switcher.selected_app(), Some("app.oldest"), "wraps back");
        assert_eq!(switcher.direction(), -1);
    }

    #[test]
    fn commit_returns_the_selection_exactly_once() {
        let mut switcher = AppSwitcher::new();
        switcher.open(entries(), Some("app.newest"), SwitchStep::Forward);
        switcher.step(SwitchStep::Forward);
        let chosen = switcher.commit().expect("an active selection commits");
        assert_eq!(chosen.app_id, "app.oldest");
        assert_eq!(chosen.window, WindowId(1));
        assert!(!switcher.is_active());
        assert_eq!(
            switcher.commit(),
            None,
            "a second commit must not activate again"
        );
    }

    #[test]
    fn cancel_applies_nothing_and_is_idempotent() {
        let mut switcher = AppSwitcher::new();
        switcher.open(entries(), Some("app.newest"), SwitchStep::Forward);
        assert!(switcher.cancel());
        assert!(!switcher.is_active());
        assert_eq!(switcher.selected_app(), None);
        assert!(!switcher.cancel(), "a second cancel is a no-op");
    }

    #[test]
    fn step_changes_the_selection_while_active() {
        // The compositor's input path calls `step` on repeats, not `open`, so a
        // held chord cycles rather than re-seeding the snapshot.
        let mut switcher = AppSwitcher::new();
        switcher.open(entries(), Some("app.newest"), SwitchStep::Forward);
        assert_eq!(switcher.selected_app(), Some("app.middle"));
        switcher.step(SwitchStep::Backward);
        assert_eq!(switcher.selected_app(), Some("app.newest"));
    }
}
