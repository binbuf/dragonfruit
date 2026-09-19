// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! The single outbox and audit log for compositor input events (T-03).
//!
//! Every trigger — keyboard, gesture, hot corner, shell, portal — records
//! the same [`InputAction`] shape here, and progress-driven actions push
//! their [`ProgressEvent`]s onto the same stream. The private shell
//! protocol (T-07) drains this outbox; until it lands, the audit log is
//! what the acceptance matrix asserts against.

use std::collections::VecDeque;

use super::action::{InputAction, TriggerKind};
use super::gestures::ProgressEvent;

/// A dispatched compositor action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchedAction {
    pub action: InputAction,
    pub source: TriggerKind,
    /// Seat serial for coherent pairing with Wayland events.
    pub serial: u32,
}

/// An application accelerator deliverable to the focused app (menu-broker,
/// T-22). The compositor matched and dispatched the chord; the owner
/// executes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppAcceleratorEvent {
    pub app_id: String,
    pub accelerator_id: String,
    pub source: TriggerKind,
    pub serial: u32,
}

/// An event bound for the shell (private protocol, T-07).
#[derive(Debug, Clone, PartialEq)]
pub enum ShellInputEvent {
    Action(DispatchedAction),
    Progress(ProgressEvent),
    AppAccelerator(AppAcceleratorEvent),
}

/// The audit log / outbox.
///
/// The action log and the shell outbox are both bounded: until the private
/// protocol (T-07) drains the outbox every loop iteration, a long-running
/// session with no shell attached must not grow without limit. The outbox
/// keeps the most recent events, which is the useful tail for a shell that
/// attaches late.
#[derive(Debug)]
pub struct InputDispatch {
    log: Vec<DispatchedAction>,
    outbox: VecDeque<ShellInputEvent>,
    max_log: usize,
    max_outbox: usize,
}

impl Default for InputDispatch {
    fn default() -> Self {
        InputDispatch {
            log: Vec::new(),
            outbox: VecDeque::new(),
            max_log: 256,
            max_outbox: 256,
        }
    }
}

impl InputDispatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a dispatched action and queue it for the shell.
    pub fn action(&mut self, action: InputAction, source: TriggerKind, serial: u32) {
        let dispatched = DispatchedAction {
            action,
            source,
            serial,
        };
        self.log.push(dispatched);
        if self.log.len() > self.max_log {
            let excess = self.log.len() - self.max_log;
            self.log.drain(0..excess);
        }
        self.push_outbox(ShellInputEvent::Action(dispatched));
    }

    /// Queue a progress event for the shell.
    pub fn progress(&mut self, event: ProgressEvent) {
        self.push_outbox(ShellInputEvent::Progress(event));
    }

    /// Queue an application-accelerator delivery for the shell/menu-broker.
    pub fn app_accelerator(
        &mut self,
        app_id: &str,
        accelerator_id: &str,
        source: TriggerKind,
        serial: u32,
    ) {
        self.push_outbox(ShellInputEvent::AppAccelerator(AppAcceleratorEvent {
            app_id: app_id.to_string(),
            accelerator_id: accelerator_id.to_string(),
            source,
            serial,
        }));
    }

    /// Append to the bounded outbox, evicting the oldest event if full.
    fn push_outbox(&mut self, event: ShellInputEvent) {
        if self.outbox.len() >= self.max_outbox {
            self.outbox.pop_front();
        }
        self.outbox.push_back(event);
    }

    /// Remove and return everything queued for the shell.
    pub fn drain(&mut self) -> Vec<ShellInputEvent> {
        self.outbox.drain(..).collect()
    }

    /// The action audit log, oldest first.
    pub fn log(&self) -> &[DispatchedAction] {
        &self.log
    }

    /// The most recently dispatched action, if any.
    pub fn last_action(&self) -> Option<DispatchedAction> {
        self.log.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_is_bounded_and_drains() {
        let mut dispatch = InputDispatch::new();
        for i in 0..300u32 {
            dispatch.action(InputAction::Screenshot, TriggerKind::Shell, i);
        }
        assert_eq!(dispatch.log().len(), 256);
        assert_eq!(
            dispatch.last_action().unwrap().action,
            InputAction::Screenshot
        );
        // Both the log and the outbox are bounded; the outbox keeps the
        // newest tail for a shell that attaches late.
        assert_eq!(dispatch.drain().len(), 256);
        assert!(dispatch.drain().is_empty());
    }

    #[test]
    fn outbox_never_grows_without_bound() {
        let mut dispatch = InputDispatch::new();
        for i in 0..10_000u32 {
            dispatch.action(InputAction::MissionControl, TriggerKind::Keyboard, i);
        }
        let drained = dispatch.drain();
        assert_eq!(drained.len(), 256);
        // The most recent action survives; the oldest were evicted.
        match drained.last().unwrap() {
            ShellInputEvent::Action(action) => assert_eq!(action.serial, 9_999),
            other => panic!("unexpected event {other:?}"),
        }
    }
}
