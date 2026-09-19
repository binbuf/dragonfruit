// SPDX-License-Identifier: MIT OR Apache-2.0
//! Window lifecycle and focus broadcasts (T-04 FR-4).
//!
//! The compositor is the sole owner of window state; the shell, Dock, app
//! switcher, and menu-broker are observers. They learn about window state
//! changes from this event stream over the private protocol (T-07), never
//! by polling. Until T-07 exists, [`WindowDispatch`] is the single outbox,
//! mirroring [`InputDispatch`](crate::input::dispatch::InputDispatch).

use std::collections::VecDeque;

use super::state::WindowState;
use super::WindowId;

/// What happened to a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEventKind {
    /// A toplevel was mapped into the scene.
    Mapped,
    /// A window left the scene.
    Unmapped,
    /// The window became the active window.
    Focused,
    /// The window stopped being the active window.
    Unfocused,
    /// The client changed its title.
    TitleChanged,
    /// The client changed its `app_id`.
    AppIdChanged,
    /// The window changed state (minimized, restored, zoomed, ...).
    StateChanged(WindowState),
}

impl WindowEventKind {
    /// A stable label for logs and the audit trail.
    pub const fn name(&self) -> &'static str {
        match self {
            WindowEventKind::Mapped => "mapped",
            WindowEventKind::Unmapped => "unmapped",
            WindowEventKind::Focused => "focused",
            WindowEventKind::Unfocused => "unfocused",
            WindowEventKind::TitleChanged => "title-changed",
            WindowEventKind::AppIdChanged => "app-id-changed",
            WindowEventKind::StateChanged(_) => "state-changed",
        }
    }
}

/// One broadcast about one window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellWindowEvent {
    pub kind: WindowEventKind,
    pub id: WindowId,
    pub app_id: Option<String>,
    pub title: Option<String>,
}

/// The bounded outbox every window event is written to.
#[derive(Debug)]
pub struct WindowDispatch {
    log: VecDeque<ShellWindowEvent>,
    capacity: usize,
}

impl Default for WindowDispatch {
    fn default() -> Self {
        WindowDispatch::new()
    }
}

impl WindowDispatch {
    /// The default bounded capacity; the shell drains every loop iteration,
    /// so this only bounds a pathological producer.
    pub const DEFAULT_CAPACITY: usize = 1024;

    pub fn new() -> Self {
        WindowDispatch {
            log: VecDeque::new(),
            capacity: Self::DEFAULT_CAPACITY,
        }
    }

    /// Record an event, evicting the oldest if at capacity.
    pub fn push(&mut self, event: ShellWindowEvent) {
        if self.log.len() >= self.capacity {
            self.log.pop_front();
        }
        self.log.push_back(event);
    }

    /// Record a state-change broadcast.
    pub fn state_changed(
        &mut self,
        id: WindowId,
        state: WindowState,
        app_id: Option<String>,
        title: Option<String>,
    ) {
        self.push(ShellWindowEvent {
            kind: WindowEventKind::StateChanged(state),
            id,
            app_id,
            title,
        });
    }

    /// Take every pending event.
    pub fn drain(&mut self) -> Vec<ShellWindowEvent> {
        self.log.drain(..).collect()
    }

    /// Number of pending events.
    pub fn len(&self) -> usize {
        self.log.len()
    }

    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_are_bounded_and_drain_in_order() {
        let mut dispatch = WindowDispatch::new();
        for i in 0..(WindowDispatch::DEFAULT_CAPACITY + 10) {
            dispatch.push(ShellWindowEvent {
                kind: WindowEventKind::Mapped,
                id: WindowId(i as u64),
                app_id: None,
                title: None,
            });
        }
        assert_eq!(dispatch.len(), WindowDispatch::DEFAULT_CAPACITY);
        let events = dispatch.drain();
        assert_eq!(events.len(), WindowDispatch::DEFAULT_CAPACITY);
        assert!(dispatch.is_empty());
        // The oldest ten were evicted.
        assert_eq!(events[0].id, WindowId(10));
    }

    #[test]
    fn focus_events_carry_identity() {
        let mut dispatch = WindowDispatch::new();
        dispatch.push(ShellWindowEvent {
            kind: WindowEventKind::Focused,
            id: WindowId(7),
            app_id: Some("org.example.App".into()),
            title: Some("Example".into()),
        });
        let events = dispatch.drain();
        assert_eq!(events[0].kind, WindowEventKind::Focused);
        assert_eq!(events[0].app_id.as_deref(), Some("org.example.App"));
    }
}
