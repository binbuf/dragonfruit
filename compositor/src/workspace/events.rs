// SPDX-License-Identifier: MIT
//! Workspace event broadcasts (T-05 FR-8).
//!
//! The compositor is the sole owner of workspace state; the shell, Dock,
//! and Mission Control are observers. They learn about workspace changes
//! from this stream over the private protocol (T-07), never by polling.
//! Until T-07 exists, [`WorkspaceDispatch`] is the single outbox, mirroring
//! [`WindowDispatch`](crate::window::WindowDispatch).
//!
//! Events are pushed by [`WorkspaceModel`](super::WorkspaceModel) in the
//! same call that mutates the model, before any scene change or render
//! pass, so the shell can never observe a frame where its state and the
//! compositor's disagree.

#![allow(dead_code)] // Forward-looking workspace event stream (T-07 consumer).

use std::collections::VecDeque;

use super::SpaceId;
use crate::window::WindowId;

/// What happened to the workspace model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEventKind {
    /// An output gained a fresh Space list.
    OutputAdded,
    /// An output's Spaces were destroyed (its windows were migrated first).
    OutputRemoved,
    /// A Space was created at `index`.
    Created { index: usize, space: SpaceId },
    /// A Space was removed.
    Removed { space: SpaceId },
    /// A Space moved to `index`.
    Reordered { space: SpaceId, index: usize },
    /// An output's active Space changed.
    Activated { index: usize, space: SpaceId },
    /// A window was assigned to a Space.
    WindowAssigned { window: WindowId, space: SpaceId },
}

impl WorkspaceEventKind {
    /// A stable label for logs and the audit trail.
    pub const fn name(&self) -> &'static str {
        match self {
            WorkspaceEventKind::OutputAdded => "output-added",
            WorkspaceEventKind::OutputRemoved => "output-removed",
            WorkspaceEventKind::Created { .. } => "space-created",
            WorkspaceEventKind::Removed { .. } => "space-removed",
            WorkspaceEventKind::Reordered { .. } => "space-reordered",
            WorkspaceEventKind::Activated { .. } => "space-activated",
            WorkspaceEventKind::WindowAssigned { .. } => "window-assigned",
        }
    }
}

/// One workspace broadcast. `output` is the affected display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEvent {
    pub kind: WorkspaceEventKind,
    pub output: String,
}

impl WorkspaceEvent {
    pub fn output_added(output: &str) -> Self {
        WorkspaceEvent {
            kind: WorkspaceEventKind::OutputAdded,
            output: output.to_string(),
        }
    }

    pub fn output_removed(output: &str) -> Self {
        WorkspaceEvent {
            kind: WorkspaceEventKind::OutputRemoved,
            output: output.to_string(),
        }
    }

    pub fn created(output: &str, index: usize, space: SpaceId) -> Self {
        WorkspaceEvent {
            kind: WorkspaceEventKind::Created { index, space },
            output: output.to_string(),
        }
    }

    pub fn removed(output: &str, space: SpaceId) -> Self {
        WorkspaceEvent {
            kind: WorkspaceEventKind::Removed { space },
            output: output.to_string(),
        }
    }

    pub fn reordered(output: &str, space: SpaceId, index: usize) -> Self {
        WorkspaceEvent {
            kind: WorkspaceEventKind::Reordered { space, index },
            output: output.to_string(),
        }
    }

    pub fn activated(output: &str, index: usize, space: SpaceId) -> Self {
        WorkspaceEvent {
            kind: WorkspaceEventKind::Activated { index, space },
            output: output.to_string(),
        }
    }

    pub fn window_assigned(window: WindowId, space: SpaceId) -> Self {
        WorkspaceEvent {
            kind: WorkspaceEventKind::WindowAssigned { window, space },
            output: String::new(),
        }
    }
}

/// The bounded outbox every workspace event is written to.
#[derive(Debug)]
pub struct WorkspaceDispatch {
    log: VecDeque<WorkspaceEvent>,
    capacity: usize,
}

impl Default for WorkspaceDispatch {
    fn default() -> Self {
        WorkspaceDispatch::new()
    }
}

impl WorkspaceDispatch {
    /// The default bounded capacity; the shell drains every loop iteration,
    /// so this only bounds a pathological producer.
    pub const DEFAULT_CAPACITY: usize = 1024;

    pub fn new() -> Self {
        WorkspaceDispatch {
            log: VecDeque::new(),
            capacity: Self::DEFAULT_CAPACITY,
        }
    }

    /// Record an event, evicting the oldest if at capacity.
    pub fn push(&mut self, event: WorkspaceEvent) {
        if self.log.len() >= self.capacity {
            self.log.pop_front();
        }
        self.log.push_back(event);
    }

    /// Take every pending event.
    pub fn drain(&mut self) -> Vec<WorkspaceEvent> {
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
        let mut dispatch = WorkspaceDispatch::new();
        for i in 0..(WorkspaceDispatch::DEFAULT_CAPACITY + 5) {
            dispatch.push(WorkspaceEvent::created("DP-1", i, SpaceId(i as u64)));
        }
        assert_eq!(dispatch.len(), WorkspaceDispatch::DEFAULT_CAPACITY);
        let events = dispatch.drain();
        assert_eq!(events.len(), WorkspaceDispatch::DEFAULT_CAPACITY);
        assert!(dispatch.is_empty());
        // The oldest five were evicted.
        match &events[0].kind {
            WorkspaceEventKind::Created { index, .. } => assert_eq!(*index, 5),
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn assignment_events_carry_the_window_and_space() {
        let event = WorkspaceEvent::window_assigned(WindowId(2), SpaceId(9));
        assert_eq!(
            event.kind,
            WorkspaceEventKind::WindowAssigned {
                window: WindowId(2),
                space: SpaceId(9)
            }
        );
    }
}
