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

/// One app in recency order, carrying its windows most-recent first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherApp {
    /// The raw `app_id` (Wayland) / resolved `WM_CLASS` (X11); T-23 later
    /// supplies richer identity.
    pub app_id: String,
    /// The window a commit activates when the app is not window-cycled: the
    /// app's most-recent window (`windows[0]`). [`AppSwitcher::commit`] hands
    /// back the cursor-selected window instead.
    pub window: WindowId,
    /// Every window of the app, most-recent first. Cmd+` (T-06.2b) cycles
    /// through these within the selected app; the entry is the app-level view.
    pub windows: Vec<WindowId>,
}

impl SwitcherApp {
    pub fn new(app_id: impl Into<String>, window: WindowId) -> Self {
        SwitcherApp::with_windows(app_id, vec![window])
    }

    /// Build an entry from the app's windows, most-recent first. The app's
    /// most-recent window is `windows[0]`; an empty list is a programming
    /// error (an app always has at least one window).
    pub fn with_windows(app_id: impl Into<String>, windows: Vec<WindowId>) -> Self {
        let window = *windows
            .first()
            .expect("a switcher app always has at least one window");
        SwitcherApp {
            app_id: app_id.into(),
            window,
            windows,
        }
    }

    /// The app's window at `cursor`, wrapping, or `None` when the app has no
    /// windows (which cannot happen for a live entry).
    pub fn window_at(&self, cursor: usize) -> Option<WindowId> {
        if self.windows.is_empty() {
            return None;
        }
        Some(self.windows[cursor % self.windows.len()])
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
    /// The selected app's window cursor: Cmd+` (T-06.2b) moves it within
    /// `entries[selected].windows`, wrapping.
    window_cursor: usize,
    direction: i32,
    /// The last within-app window-cycle direction (`0` at rest).
    window_direction: i32,
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

    /// The selected app's window a commit activates: the cursor-selected one
    /// (the app's most-recent until Cmd+` cycles within the app).
    pub fn selected_window(&self) -> Option<WindowId> {
        let entry = self
            .selected_index()
            .and_then(|index| self.entries.get(index))?;
        entry.window_at(self.window_cursor)
    }

    /// The selected app's window cursor (`0` = most recent). The overlay
    /// renders the cursor-selected live surface as the app's preview.
    pub fn window_cursor(&self) -> usize {
        self.window_cursor
    }

    /// The last app-cycle direction (`0` at rest); the private protocol's
    /// `direction` argument.
    pub fn direction(&self) -> i32 {
        self.direction
    }

    /// The last within-app window-cycle direction (`0` at rest).
    pub fn window_direction(&self) -> i32 {
        self.window_direction
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
        self.window_cursor = 0;
        self.window_direction = 0;
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

    /// Open (or re-seed) the switcher **on** `focused` rather than one app
    /// away from it, so Cmd+` (T-06.2b) cycles windows inside the frontmost
    /// app. Falls back to the most recent app when nothing is focused; an
    /// empty list stays closed. Returns whether it is active.
    pub fn open_focused(&mut self, entries: Vec<SwitcherApp>, focused: Option<&str>) -> bool {
        self.entries = entries;
        self.direction = 0;
        self.window_cursor = 0;
        self.window_direction = 0;
        if self.entries.is_empty() {
            self.reset();
            return false;
        }
        self.selected = focused
            .and_then(|app| self.entries.iter().position(|entry| entry.app_id == app))
            .unwrap_or(0);
        self.active = true;
        true
    }

    /// Move the app selection one step, wrapping at both ends. The window
    /// cursor resets to the new app's most recent window (the app-level view).
    /// A no-op when the switcher is closed.
    pub fn step(&mut self, step: SwitchStep) {
        if !self.active || self.entries.is_empty() {
            return;
        }
        let len = self.entries.len() as i32;
        self.selected = (((self.selected as i32 + step.direction()) % len + len) % len) as usize;
        self.window_cursor = 0;
        self.direction = step.direction();
    }

    /// Move the window cursor within the selected app, wrapping at both ends
    /// (Cmd+` / Cmd+Shift+`). A no-op when closed or when the app has a single
    /// window; the app selection, and so the overlay highlight, does not move.
    pub fn step_window(&mut self, step: SwitchStep) {
        if !self.active {
            return;
        }
        let Some(entry) = self.entries.get(self.selected) else {
            return;
        };
        let len = entry.windows.len() as i32;
        if len <= 1 {
            return;
        }
        self.window_cursor =
            (((self.window_cursor as i32 + step.direction()) % len + len) % len) as usize;
        self.window_direction = step.direction();
    }

    /// Select the entry that owns `window`, placing the window cursor on it
    /// (a pointer click on a live preview, T-06.2b). Returns whether it
    /// matched a live entry.
    pub fn select_window(&mut self, window: WindowId) -> bool {
        if !self.active {
            return false;
        }
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.windows.contains(&window))
        else {
            return false;
        };
        self.selected = index;
        self.window_cursor = self.entries[index]
            .windows
            .iter()
            .position(|candidate| *candidate == window)
            .unwrap_or(0);
        true
    }

    /// Commit the selection: return the chosen app (with `window` set to the
    /// cursor-selected window) and close the switcher.
    ///
    /// The active state is cleared before the value is handed back, so a
    /// second call (a stray second release, a duplicate event) returns `None`
    /// and can never activate twice.
    pub fn commit(&mut self) -> Option<SwitcherApp> {
        if !self.active {
            return None;
        }
        let mut entry = self.entries.get(self.selected).cloned()?;
        if let Some(window) = self.selected_window() {
            entry.window = window;
        }
        self.reset();
        Some(entry)
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
        self.window_cursor = 0;
        self.direction = 0;
        self.window_direction = 0;
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

    fn multi() -> Vec<SwitcherApp> {
        // `app.multi` has three windows, most recent first.
        vec![
            SwitcherApp::new("app.other", WindowId(9)),
            SwitcherApp::with_windows("app.multi", vec![WindowId(3), WindowId(2), WindowId(1)]),
        ]
    }

    #[test]
    fn open_focused_keeps_the_focused_app_and_its_most_recent_window() {
        let mut switcher = AppSwitcher::new();
        assert!(switcher.open_focused(multi(), Some("app.multi")));
        assert_eq!(switcher.selected_app(), Some("app.multi"));
        assert_eq!(switcher.selected_window(), Some(WindowId(3)));
        assert_eq!(switcher.window_cursor(), 0);
        assert_eq!(switcher.direction(), 0);
    }

    #[test]
    fn window_cycling_wraps_within_the_selected_app() {
        let mut switcher = AppSwitcher::new();
        switcher.open_focused(multi(), Some("app.multi"));
        switcher.step_window(SwitchStep::Forward);
        assert_eq!(switcher.selected_window(), Some(WindowId(2)));
        assert_eq!(switcher.selected_app(), Some("app.multi"));
        switcher.step_window(SwitchStep::Forward);
        assert_eq!(switcher.selected_window(), Some(WindowId(1)));
        switcher.step_window(SwitchStep::Forward);
        assert_eq!(
            switcher.selected_window(),
            Some(WindowId(3)),
            "wraps forward"
        );
        switcher.step_window(SwitchStep::Backward);
        assert_eq!(switcher.selected_window(), Some(WindowId(1)), "wraps back");
        assert_eq!(switcher.window_direction(), -1);
    }

    #[test]
    fn window_cycling_is_a_no_op_on_a_single_window_app() {
        let mut switcher = AppSwitcher::new();
        switcher.open_focused(multi(), Some("app.other"));
        switcher.step_window(SwitchStep::Forward);
        assert_eq!(switcher.selected_window(), Some(WindowId(9)));
        assert_eq!(switcher.window_direction(), 0);
    }

    #[test]
    fn app_cycling_resets_the_window_cursor() {
        let mut switcher = AppSwitcher::new();
        switcher.open_focused(multi(), Some("app.multi"));
        switcher.step_window(SwitchStep::Forward);
        assert_eq!(switcher.window_cursor(), 1);
        switcher.step(SwitchStep::Forward);
        assert_eq!(switcher.selected_app(), Some("app.other"));
        assert_eq!(switcher.window_cursor(), 0);
        switcher.step(SwitchStep::Backward);
        assert_eq!(switcher.selected_app(), Some("app.multi"));
        assert_eq!(switcher.selected_window(), Some(WindowId(3)));
    }

    #[test]
    fn commit_returns_the_cursor_selected_window_exactly_once() {
        let mut switcher = AppSwitcher::new();
        switcher.open_focused(multi(), Some("app.multi"));
        switcher.step_window(SwitchStep::Forward);
        let chosen = switcher.commit().expect("an active selection commits");
        assert_eq!(chosen.app_id, "app.multi");
        assert_eq!(chosen.window, WindowId(2));
        assert!(!switcher.is_active());
        assert_eq!(switcher.commit(), None);
    }

    #[test]
    fn select_window_moves_the_selection_and_cursor() {
        let mut switcher = AppSwitcher::new();
        switcher.open_focused(multi(), Some("app.multi"));
        assert!(switcher.select_window(WindowId(1)));
        assert_eq!(switcher.selected_app(), Some("app.multi"));
        assert_eq!(switcher.selected_window(), Some(WindowId(1)));
        assert!(switcher.select_window(WindowId(9)));
        assert_eq!(switcher.selected_app(), Some("app.other"));
        assert!(!switcher.select_window(WindowId(404)));
    }
}
