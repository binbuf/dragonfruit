// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! The single outbox and audit log for compositor input events (T-03).
//!
//! Every trigger — keyboard, gesture, hot corner, shell, portal — records
//! the same [`InputAction`] shape here, and progress-driven actions push
//! their [`ProgressEvent`]s onto the same stream. The private shell
//! protocol (T-07) drains this outbox; until it lands, the audit log is
//! what the acceptance matrix asserts against.

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
#[derive(Debug)]
pub struct InputDispatch {
    log: Vec<DispatchedAction>,
    outbox: Vec<ShellInputEvent>,
    max_log: usize,
}

impl Default for InputDispatch {
    fn default() -> Self {
        InputDispatch {
            log: Vec::new(),
            outbox: Vec::new(),
            max_log: 256,
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
        self.outbox.push(ShellInputEvent::Action(dispatched));
    }

    /// Queue a progress event for the shell.
    pub fn progress(&mut self, event: ProgressEvent) {
        self.outbox.push(ShellInputEvent::Progress(event));
    }

    /// Queue an application-accelerator delivery for the shell/menu-broker.
    pub fn app_accelerator(
        &mut self,
        app_id: &str,
        accelerator_id: &str,
        source: TriggerKind,
        serial: u32,
    ) {
        self.outbox
            .push(ShellInputEvent::AppAccelerator(AppAcceleratorEvent {
                app_id: app_id.to_string(),
                accelerator_id: accelerator_id.to_string(),
                source,
                serial,
            }));
    }

    /// Remove and return everything queued for the shell.
    pub fn drain(&mut self) -> Vec<ShellInputEvent> {
        std::mem::take(&mut self.outbox)
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
        assert_eq!(dispatch.drain().len(), 300);
        assert!(dispatch.drain().is_empty());
    }
}
