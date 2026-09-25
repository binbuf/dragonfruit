// SPDX-License-Identifier: MIT
//! The session supervisor: staged launch and in-place restart (T-12.1a).
//!
//! [`Supervisor`] owns the child processes of a [`SessionPlan`]. Call
//! [`Supervisor::start`] once to launch stage 0, then [`Supervisor::tick`]
//! on a timer (the `--exec` loop uses 50 ms). Each tick reaps every child
//! that has exited, applies its [`RestartPolicy`], and starts the next stage
//! once the current one is up. An anchor exit — the compositor — ends the
//! session and stops everything else; the supervisor never restarts it.
//!
//! The supervisor is deliberately synchronous and runtime-free so the kill
//! test can drive it a tick at a time.

use std::collections::HashMap;
use std::io;
use std::process::{Child, Command, ExitStatus, Stdio};

use crate::plan::{RestartPolicy, ServiceSpec, SessionPlan};

/// How a child left this world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitOutcome {
    /// Exited with status 0.
    Success,
    /// Exited with a non-zero status.
    Failure,
    /// Killed by signal `i32`.
    Signaled(i32),
}

impl ExitOutcome {
    /// Classify a [`std::process::ExitStatus`].
    pub fn from_status(status: ExitStatus) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return ExitOutcome::Signaled(signal);
            }
        }
        if status.success() {
            ExitOutcome::Success
        } else {
            ExitOutcome::Failure
        }
    }

    /// Whether the exit was clean (status 0).
    pub const fn is_success(self) -> bool {
        matches!(self, ExitOutcome::Success)
    }
}

impl std::fmt::Display for ExitOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExitOutcome::Success => write!(f, "exit 0"),
            ExitOutcome::Failure => write!(f, "failure"),
            ExitOutcome::Signaled(signal) => write!(f, "signal {signal}"),
        }
    }
}

/// Where one service is in its supervised life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Not started yet (a later stage, or the supervisor has not run).
    Pending,
    /// Spawned and expected to be alive.
    Running,
    /// Exited and not scheduled to restart.
    Exited,
    /// Killed by session teardown.
    Stopped,
    /// Could not be spawned, and no restart was attempted.
    Failed,
}

/// Where the whole session is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// `start` has not been called.
    Idle,
    /// Some stages have not started yet.
    Starting,
    /// Every stage is up.
    Running,
    /// The anchor exited or the session was shut down.
    Ended,
}

/// A lifecycle transition worth reporting (and asserting on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorEvent {
    /// A service was spawned for the first time.
    Started { name: String, pid: u32, stage: u32 },
    /// A service was respawned after an exit.
    Restarted {
        name: String,
        pid: u32,
        count: u32,
        after: ExitOutcome,
    },
    /// A service exited and was not restarted (yet).
    Exited { name: String, outcome: ExitOutcome },
    /// A spawn failed; the service is [`ServiceState::Failed`].
    SpawnFailed { name: String, error: String },
    /// The anchor exited (or failed to start); the session is over.
    SessionEnded { reason: String },
    /// [`Supervisor::shutdown`] stopped the session.
    Shutdown,
}

impl std::fmt::Display for SupervisorEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupervisorEvent::Started { name, pid, stage } => {
                write!(f, "started {name} (pid {pid}, stage {stage})")
            }
            SupervisorEvent::Restarted {
                name,
                pid,
                count,
                after,
            } => write!(f, "restarted {name} (pid {pid}, #{count}, after {after})"),
            SupervisorEvent::Exited { name, outcome } => write!(f, "exited {name} ({outcome})"),
            SupervisorEvent::SpawnFailed { name, error } => {
                write!(f, "cannot start {name}: {error}")
            }
            SupervisorEvent::SessionEnded { reason } => write!(f, "session ended: {reason}"),
            SupervisorEvent::Shutdown => write!(f, "session shut down"),
        }
    }
}

struct Slot {
    spec: ServiceSpec,
    child: Option<Child>,
    state: ServiceState,
    pid: Option<u32>,
    restarts: u32,
    last_exit: Option<ExitOutcome>,
    ready: bool,
}

/// The process supervisor for one session.
pub struct Supervisor {
    plan: SessionPlan,
    slots: Vec<Slot>,
    index: HashMap<String, usize>,
    stages: Vec<u32>,
    next_stage: usize,
    state: SessionState,
    events: Vec<SupervisorEvent>,
}

impl Supervisor {
    /// Build a supervisor for `plan`.
    ///
    /// # Panics
    ///
    /// Panics if `plan` breaks an invariant (see
    /// [`SessionPlan::validate`]); use [`SessionPlan::validate`] first for a
    /// fallible path.
    pub fn new(plan: SessionPlan) -> Self {
        plan.validate().expect("invalid session plan");
        let stages = plan.stages();
        let slots: Vec<Slot> = plan
            .services
            .iter()
            .cloned()
            .map(|spec| Slot {
                spec,
                child: None,
                state: ServiceState::Pending,
                pid: None,
                restarts: 0,
                last_exit: None,
                ready: false,
            })
            .collect();
        let index = plan
            .services
            .iter()
            .enumerate()
            .map(|(position, spec)| (spec.name.clone(), position))
            .collect();
        Supervisor {
            plan,
            slots,
            index,
            stages,
            next_stage: 1,
            state: SessionState::Idle,
            events: Vec::new(),
        }
    }

    /// The plan this supervisor owns.
    pub fn plan(&self) -> &SessionPlan {
        &self.plan
    }

    /// The session's state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// One service's state, by name.
    pub fn service_state(&self, name: &str) -> Option<ServiceState> {
        self.slot(name).map(|slot| slot.state)
    }

    /// The live pid of one service, if it is running.
    pub fn running_pid(&self, name: &str) -> Option<u32> {
        self.slot(name).and_then(|slot| slot.pid)
    }

    /// How many times one service has been restarted.
    pub fn restarts(&self, name: &str) -> Option<u32> {
        self.slot(name).map(|slot| slot.restarts)
    }

    /// The last recorded exit of one service.
    pub fn last_exit(&self, name: &str) -> Option<ExitOutcome> {
        self.slot(name).and_then(|slot| slot.last_exit)
    }

    /// Whether one service has signalled readiness.
    pub fn is_ready(&self, name: &str) -> bool {
        self.slot(name).is_some_and(|slot| slot.ready)
    }

    /// Whether any service is currently running.
    pub fn any_running(&self) -> bool {
        self.slots
            .iter()
            .any(|slot| slot.state == ServiceState::Running)
    }

    /// Launch stage 0. A second call is a no-op.
    pub fn start(&mut self) -> io::Result<()> {
        if self.state != SessionState::Idle {
            return Ok(());
        }
        self.state = SessionState::Starting;
        self.launch_stage(0);
        self.advance_stages();
        Ok(())
    }

    /// Reap exited children, apply their policies, and advance the stages.
    /// Returns everything that happened since the last tick.
    pub fn tick(&mut self) -> Vec<SupervisorEvent> {
        if matches!(self.state, SessionState::Idle | SessionState::Ended) {
            return std::mem::take(&mut self.events);
        }
        self.reap();
        if self.state != SessionState::Ended {
            self.advance_stages();
        }
        std::mem::take(&mut self.events)
    }

    /// Mark a running service ready, unblocking its stage's gate. Returns
    /// `false` if the service is unknown or not running.
    pub fn set_ready(&mut self, name: &str) -> bool {
        match self.index.get(name) {
            Some(&position) if self.slots[position].state == ServiceState::Running => {
                self.slots[position].ready = true;
                true
            }
            _ => false,
        }
    }

    /// Send `SIGKILL` to a managed service without applying its policy — the
    /// kill seam the headless restart test uses, and what a would-be
    /// `coredump` handler would call. The next [`Supervisor::tick`] reaps the
    /// death and decides whether to restart.
    pub fn kill(&mut self, name: &str) -> bool {
        match self.index.get(name) {
            Some(&position) => match self.slots[position].child.as_mut() {
                Some(child) => child.kill().is_ok(),
                None => false,
            },
            None => false,
        }
    }

    /// Stop every running child and end the session.
    pub fn shutdown(&mut self) {
        if self.state == SessionState::Ended {
            return;
        }
        self.events.push(SupervisorEvent::Shutdown);
        self.end_session();
    }

    fn slot(&self, name: &str) -> Option<&Slot> {
        self.index.get(name).map(|&position| &self.slots[position])
    }

    /// Spawn the services of `stage_pos` that are still pending.
    fn launch_stage(&mut self, stage_pos: usize) {
        let Some(&stage) = self.stages.get(stage_pos) else {
            return;
        };
        for position in 0..self.slots.len() {
            if self.slots[position].spec.stage != stage
                || self.slots[position].state != ServiceState::Pending
            {
                continue;
            }
            let name = self.slots[position].spec.name.clone();
            let ends_session = self.slots[position].spec.ends_session;
            match self.spawn_child(position) {
                Ok(pid) => self
                    .events
                    .push(SupervisorEvent::Started { name, pid, stage }),
                Err(error) => {
                    self.slots[position].state = ServiceState::Failed;
                    self.events.push(SupervisorEvent::SpawnFailed {
                        name: name.clone(),
                        error: error.to_string(),
                    });
                    if ends_session {
                        self.events.push(SupervisorEvent::SessionEnded {
                            reason: format!("{name} failed to start"),
                        });
                        self.end_session();
                        return;
                    }
                }
            }
        }
    }

    /// Spawn slot `position` and record it. Returns the new pid.
    fn spawn_child(&mut self, position: usize) -> io::Result<u32> {
        let spec = &self.slots[position].spec;
        let child = Command::new(&spec.program)
            .args(&spec.args)
            .envs(spec.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;
        let pid = child.id();
        let slot = &mut self.slots[position];
        slot.child = Some(child);
        slot.pid = Some(pid);
        slot.state = ServiceState::Running;
        slot.ready = false;
        Ok(pid)
    }

    fn reap(&mut self) {
        for position in 0..self.slots.len() {
            let status = match self.slots[position].child.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(status)) => Some(status),
                    Ok(None) | Err(_) => None,
                },
                None => None,
            };
            if let Some(status) = status {
                self.handle_exit(position, status);
                if self.state == SessionState::Ended {
                    return;
                }
            }
        }
    }

    fn handle_exit(&mut self, position: usize, status: ExitStatus) {
        let outcome = ExitOutcome::from_status(status);
        let name = self.slots[position].spec.name.clone();
        self.slots[position].child = None;
        self.slots[position].pid = None;
        self.slots[position].ready = false;
        self.slots[position].last_exit = Some(outcome);
        self.slots[position].state = ServiceState::Exited;
        self.events.push(SupervisorEvent::Exited {
            name: name.clone(),
            outcome,
        });

        if self.slots[position].spec.ends_session {
            self.events.push(SupervisorEvent::SessionEnded {
                reason: format!("{name} ({outcome})"),
            });
            self.end_session();
            return;
        }

        let restart = match self.slots[position].spec.policy {
            RestartPolicy::Always => true,
            RestartPolicy::OnFailure => !outcome.is_success(),
            RestartPolicy::Never => false,
        };
        if !restart {
            return;
        }
        match self.spawn_child(position) {
            Ok(pid) => {
                self.slots[position].restarts += 1;
                let count = self.slots[position].restarts;
                self.events.push(SupervisorEvent::Restarted {
                    name,
                    pid,
                    count,
                    after: outcome,
                });
            }
            Err(error) => {
                self.slots[position].state = ServiceState::Failed;
                self.events.push(SupervisorEvent::SpawnFailed {
                    name,
                    error: error.to_string(),
                });
            }
        }
    }

    /// Start every stage whose predecessor is up, in order. `start` has
    /// already launched stage 0, so the first stage to consider is
    /// `next_stage`, gated on the stage before it.
    fn advance_stages(&mut self) {
        if self.state == SessionState::Ended {
            return;
        }
        while self.next_stage < self.stages.len() {
            let previous = self.stages[self.next_stage - 1];
            if !self.stage_ready(previous) {
                self.state = SessionState::Starting;
                return;
            }
            let position = self.next_stage;
            self.next_stage += 1;
            self.launch_stage(position);
            if self.state == SessionState::Ended {
                return;
            }
        }
        self.state = SessionState::Running;
    }

    /// A stage is ready to advance when every service in it has been spawned
    /// and every `gate` service that is still running has signalled
    /// readiness. A gate service that already exited does not block forever.
    fn stage_ready(&self, stage: u32) -> bool {
        for slot in &self.slots {
            if slot.spec.stage != stage {
                continue;
            }
            if slot.state == ServiceState::Pending {
                return false;
            }
            if slot.spec.gate && slot.state == ServiceState::Running && !slot.ready {
                return false;
            }
        }
        true
    }

    /// Stop everything and mark the session ended. The anchor stays
    /// [`ServiceState::Exited`]; surviving children become
    /// [`ServiceState::Stopped`].
    fn end_session(&mut self) {
        for slot in &mut self.slots {
            if let Some(mut child) = slot.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            slot.pid = None;
            slot.ready = false;
            if slot.state == ServiceState::Running {
                slot.state = ServiceState::Stopped;
            }
        }
        self.state = SessionState::Ended;
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            if let Some(mut child) = slot.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_outcome_classifies_statuses() {
        assert!(ExitOutcome::Success.is_success());
        assert!(!ExitOutcome::Failure.is_success());
        assert!(!ExitOutcome::Signaled(9).is_success());
    }
}
