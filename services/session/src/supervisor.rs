// SPDX-License-Identifier: GPL-3.0-only
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
//!
//! Each service is spawned as the leader of its own process group (T-12.2),
//! so logout ([`Supervisor::shutdown`]) can tear down a service's whole tree:
//! `SIGTERM` to the group for a clean exit, then `SIGKILL` if it is still
//! alive. Killing only the direct child would leak the grandchildren a shell
//! or service wrapper starts.
//!
//! Teardown kills the group *before* reaping the leader (T345). The group id
//! is the leader's pid; while the leader is a zombie or still running, the
//! kernel cannot recycle that number, so `kill(-pgid)` can only reach our
//! group. Reaping first and then signalling the pgid is a race against pid
//! reuse: the id may already name an unrelated process group.
//!
//! A crash loop is bounded by [`RestartBackoff`] (T345): consecutive restarts
//! wait 200 ms, 400 ms, 800 ms, … (capped at 30 s) and stop after five
//! attempts. A run at least ten seconds long is a stable run and resets the
//! counter. A service waiting out its delay is [`ServiceState::Exited`] with
//! [`Supervisor::next_restart_at`] set; the next [`Supervisor::tick`] that
//! passes the deadline respawns it.

use std::collections::HashMap;
use std::io;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::env;
use crate::plan::{RestartPolicy, ServiceSpec, SessionPlan};

/// How long a child gets to exit after `SIGTERM` before the group is
/// `SIGKILL`ed.
const TERM_GRACE: Duration = Duration::from_millis(500);

/// The restart backoff and crash-loop limit for supervised services.
///
/// A crash loop must not become a fork storm: every consecutive restart waits
/// twice as long as the one before, up to [`RestartBackoff::max_delay`], and
/// after [`RestartBackoff::max_attempts`] consecutive restarts the supervisor
/// leaves the service [`ServiceState::Exited`] instead of respawning it. A run
/// at least [`RestartBackoff::stable_run`] long counts as stable and resets the
/// attempt counter, so a service that crashes once a day is always restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartBackoff {
    /// The delay before the first restart.
    pub base: Duration,
    /// The ceiling the doubled delay is clamped to.
    pub max_delay: Duration,
    /// Consecutive restarts allowed before the supervisor gives up.
    pub max_attempts: u32,
    /// How long a run must last to count as stable and reset the attempts.
    pub stable_run: Duration,
}

impl Default for RestartBackoff {
    fn default() -> Self {
        RestartBackoff {
            base: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            max_attempts: 5,
            stable_run: Duration::from_secs(10),
        }
    }
}

impl RestartBackoff {
    /// The delay before restart attempt `attempt` (1-based), doubling from
    /// [`RestartBackoff::base`] and capped at [`RestartBackoff::max_delay`].
    pub fn delay(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(31);
        self.base.saturating_mul(1u32 << shift).min(self.max_delay)
    }
}

/// Send `signal` to the process group led by `pid`. Every managed child is a
/// process-group leader ([`Command::process_group`](std::os::unix::process::CommandExt::process_group)),
/// so a negative pid reaches the service and everything it started.
fn signal_group(pid: u32, signal: libc::c_int) {
    // A negative pid targets the whole process group.
    unsafe {
        libc::kill(-(pid as libc::pid_t), signal);
    }
}

/// Stop one child and its whole process group: `SIGTERM` for a clean exit,
/// then `SIGKILL` after [`TERM_GRACE`], and a final group `SIGKILL` so
/// grandchildren cannot outlive their leader. Returns once the direct child
/// has been reaped.
fn terminate(child: &mut Child) {
    terminate_with(child, &mut |pid, signal| signal_group(pid, signal));
}

/// What became of the leader while teardown waited for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeaderWait {
    /// The leader exited and is still an unreaped zombie.
    Exited,
    /// The leader is still running at the end of the grace.
    Running,
    /// The leader is no longer our child (already reaped). Its pid — and so
    /// its process-group id — may already have been recycled.
    Gone,
}

/// Wait up to `timeout` for `pid` to exit *without* reaping it, so the caller
/// can still own its process-group id.
///
/// Linux uses a pidfd: it becomes readable when the child exits, and a zombie
/// keeps the pid allocated until it is reaped. Kernels without `pidfd_open`
/// fall back to waiting out the whole grace, which is slower but keeps the
/// same ordering guarantee.
#[cfg(target_os = "linux")]
fn wait_leader(pid: u32, timeout: Duration) -> LeaderWait {
    let opened = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) };
    if opened < 0 {
        return if io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
            LeaderWait::Gone
        } else {
            // No pidfd support. The leader is still unreaped (std owns it), so
            // the group id is still ours after the grace.
            std::thread::sleep(timeout);
            LeaderWait::Running
        };
    }
    let fd = opened as libc::c_int;
    let mut pollfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let millis = timeout.as_millis().min(libc::c_int::MAX as u128) as libc::c_int;
    let ready = unsafe { libc::poll(&mut pollfd, 1, millis) };
    unsafe {
        libc::close(fd);
    }
    if ready > 0 {
        LeaderWait::Exited
    } else {
        LeaderWait::Running
    }
}

/// Non-Linux fallback: wait out the grace, then kill the group while the
/// leader is still unreaped.
#[cfg(not(target_os = "linux"))]
fn wait_leader(_pid: u32, timeout: Duration) -> LeaderWait {
    std::thread::sleep(timeout);
    LeaderWait::Running
}

/// [`terminate`] with an injectable group-signal transport. The production
/// path sends a real `kill(-pgid, signal)`; the PGID-reuse test substitutes a
/// recorder that models the kernel recycling the id after a reap.
fn terminate_with(child: &mut Child, signal: &mut dyn FnMut(u32, libc::c_int)) {
    let pid = child.id();
    signal(pid, libc::SIGTERM);
    match wait_leader(pid, TERM_GRACE) {
        // While the leader is a zombie (or still running) its pid, and with
        // it the process-group id, cannot be recycled. Kill the whole group
        // *before* the reap below frees that id: a signal after the reap
        // could reach an unrelated group that received the same pgid.
        LeaderWait::Exited | LeaderWait::Running => signal(pid, libc::SIGKILL),
        // Someone else reaped the leader; the pgid may already name another
        // group, so never signal the number again.
        LeaderWait::Gone => {}
    }
    let _ = child.wait();
}

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
    /// A crash loop hit [`RestartBackoff::max_attempts`]; the service stays
    /// [`ServiceState::Exited`] until something else starts it.
    RestartAbandoned { name: String, attempts: u32 },
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
            SupervisorEvent::RestartAbandoned { name, attempts } => {
                write!(f, "gave up restarting {name} after {attempts} attempts")
            }
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
    /// Consecutive restarts since the last stable run (or first start).
    attempts: u32,
    /// When the backoff delay ends and the restart may be spawned.
    next_restart_at: Option<Instant>,
    /// When the current process was spawned, for the stable-run reset.
    started_at: Option<Instant>,
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
    backoff: RestartBackoff,
    events: Vec<SupervisorEvent>,
}

impl Supervisor {
    /// Build a supervisor for `plan` with the default [`RestartBackoff`].
    ///
    /// # Panics
    ///
    /// Panics if `plan` breaks an invariant (see
    /// [`SessionPlan::validate`]); use [`SessionPlan::validate`] first for a
    /// fallible path.
    pub fn new(plan: SessionPlan) -> Self {
        Self::with_backoff(plan, RestartBackoff::default())
    }

    /// Build a supervisor for `plan` with an explicit restart backoff.
    ///
    /// # Panics
    ///
    /// Panics if `plan` breaks an invariant (see
    /// [`SessionPlan::validate`]).
    pub fn with_backoff(plan: SessionPlan, backoff: RestartBackoff) -> Self {
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
                attempts: 0,
                next_restart_at: None,
                started_at: None,
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
            backoff,
            events: Vec::new(),
        }
    }

    /// The plan this supervisor owns.
    pub fn plan(&self) -> &SessionPlan {
        &self.plan
    }

    /// The restart backoff this supervisor applies.
    pub fn backoff(&self) -> RestartBackoff {
        self.backoff
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

    /// How many consecutive restarts one service has used since its last
    /// stable run.
    pub fn restart_attempts(&self, name: &str) -> Option<u32> {
        self.slot(name).map(|slot| slot.attempts)
    }

    /// When one service's scheduled restart is due, if it is waiting out its
    /// backoff. `None` means no restart is scheduled.
    pub fn next_restart_at(&self, name: &str) -> Option<Instant> {
        self.slot(name).and_then(|slot| slot.next_restart_at)
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

    /// Whether any service is running or waiting out a restart backoff. This
    /// is what a driver loop waits on: [`Supervisor::any_running`] alone would
    /// exit between a crash and its scheduled restart.
    pub fn any_active(&self) -> bool {
        self.slots
            .iter()
            .any(|slot| slot.state == ServiceState::Running || slot.next_restart_at.is_some())
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

    /// Reap exited children, apply their policies, spawn any restart whose
    /// backoff has elapsed, and advance the stages. Returns everything that
    /// happened since the last tick.
    pub fn tick(&mut self) -> Vec<SupervisorEvent> {
        if matches!(self.state, SessionState::Idle | SessionState::Ended) {
            return std::mem::take(&mut self.events);
        }
        self.reap();
        if self.state != SessionState::Ended {
            self.restart_due();
            if self.state != SessionState::Ended {
                self.advance_stages();
            }
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

    /// Send `SIGKILL` to a managed service's process group without applying
    /// its policy — the kill seam the headless restart test uses, and what a
    /// would-be `coredump` handler would call. The next [`Supervisor::tick`]
    /// reaps the death and decides whether to restart.
    pub fn kill(&mut self, name: &str) -> bool {
        match self.index.get(name) {
            Some(&position) => match self.slots[position].child.as_mut() {
                Some(child) => {
                    signal_group(child.id(), libc::SIGKILL);
                    true
                }
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
        let mut command = Command::new(&spec.program);
        command
            // A child only gets a launch token when the plan explicitly
            // attaches one (trusted/desktop_trusted). Strip inherited tokens
            // so an untrusted service cannot pick up the session's ambient
            // token from the supervisor's own environment.
            .env_remove(env::LAUNCH_TOKEN)
            .env_remove(env::DESKTOP_LAUNCH_TOKEN)
            .args(&spec.args)
            .envs(spec.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        // Each service leads its own process group, so logout teardown can
        // reach the grandchildren a wrapper starts (T-12.2).
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn()?;
        let pid = child.id();
        let slot = &mut self.slots[position];
        slot.child = Some(child);
        slot.pid = Some(pid);
        slot.state = ServiceState::Running;
        slot.ready = false;
        slot.started_at = Some(Instant::now());
        slot.next_restart_at = None;
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
        // A run at least `stable_run` long is a success, not a crash: reset
        // the consecutive-attempt counter so a long-lived service may crash
        // again later.
        let stable = self.slots[position]
            .started_at
            .is_some_and(|started| started.elapsed() >= self.backoff.stable_run);
        self.slots[position].child = None;
        self.slots[position].pid = None;
        self.slots[position].ready = false;
        self.slots[position].started_at = None;
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

        if stable {
            self.slots[position].attempts = 0;
        }
        if self.slots[position].attempts >= self.backoff.max_attempts {
            self.slots[position].next_restart_at = None;
            let attempts = self.slots[position].attempts;
            self.events
                .push(SupervisorEvent::RestartAbandoned { name, attempts });
            return;
        }
        self.slots[position].attempts += 1;
        let attempt = self.slots[position].attempts;
        self.slots[position].next_restart_at = Some(Instant::now() + self.backoff.delay(attempt));
    }

    /// Spawn every service whose restart backoff has elapsed.
    fn restart_due(&mut self) {
        let now = Instant::now();
        for position in 0..self.slots.len() {
            if !self.slots[position]
                .next_restart_at
                .is_some_and(|at| at <= now)
            {
                continue;
            }
            self.slots[position].next_restart_at = None;
            let name = self.slots[position].spec.name.clone();
            let after = self.slots[position].last_exit;
            match self.spawn_child(position) {
                Ok(pid) => {
                    self.slots[position].restarts += 1;
                    let count = self.slots[position].restarts;
                    self.events.push(SupervisorEvent::Restarted {
                        name,
                        pid,
                        count,
                        after: after.unwrap_or(ExitOutcome::Failure),
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
    /// [`ServiceState::Stopped`]. A restart still waiting out its backoff is
    /// cancelled.
    fn end_session(&mut self) {
        for slot in &mut self.slots {
            if let Some(mut child) = slot.child.take() {
                terminate(&mut child);
            }
            slot.pid = None;
            slot.ready = false;
            slot.started_at = None;
            slot.next_restart_at = None;
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
                terminate(&mut child);
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

    #[test]
    fn backoff_delay_doubles_and_caps() {
        let backoff = RestartBackoff {
            base: Duration::from_millis(100),
            max_delay: Duration::from_millis(800),
            max_attempts: 9,
            stable_run: Duration::from_secs(1),
        };
        assert_eq!(backoff.delay(0), Duration::from_millis(100));
        assert_eq!(backoff.delay(1), Duration::from_millis(100));
        assert_eq!(backoff.delay(2), Duration::from_millis(200));
        assert_eq!(backoff.delay(3), Duration::from_millis(400));
        assert_eq!(backoff.delay(4), Duration::from_millis(800));
        assert_eq!(backoff.delay(5), Duration::from_millis(800));
        assert_eq!(backoff.delay(u32::MAX), Duration::from_millis(800));
    }

    /// Signal 0 checks existence without delivering anything. A zombie still
    /// answers, which is exactly the state that keeps its pid (and so its
    /// process-group id) allocated.
    fn process_exists(pid: u32) -> bool {
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    /// T345: teardown must `SIGKILL` the group while the leader is still an
    /// unreaped zombie. The recorder stands in for the kernel's pid allocator:
    /// once the leader is reaped it models the pgid as recycled and forwards
    /// the signal to an unrelated live group. A regression that reaps first
    /// therefore records a violation and kills that unrelated group.
    #[test]
    fn teardown_kills_the_group_before_reaping_the_leader() {
        use std::os::unix::process::CommandExt;

        let mut victim = Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .expect("the victim starts");
        let victim_pid = victim.id();
        let mut unrelated = Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .expect("the unrelated group starts");
        let unrelated_pid = unrelated.id();
        assert_ne!(
            victim_pid, unrelated_pid,
            "the two process groups have different ids"
        );

        let mut violations = 0;
        let mut killed_before_reap = false;
        let mut recorder = |pgid: u32, signal: libc::c_int| {
            assert_eq!(pgid, victim_pid, "teardown signals only the victim group");
            if process_exists(pgid) {
                // The leader is alive or a zombie: the id is still ours.
                signal_group(pgid, signal);
                if signal == libc::SIGKILL {
                    killed_before_reap = true;
                }
            } else {
                // Simulated reuse: the pgid now names an unrelated group.
                violations += 1;
                signal_group(unrelated_pid, signal);
            }
        };

        terminate_with(&mut victim, &mut recorder);

        assert!(
            killed_before_reap,
            "the final group SIGKILL must precede the reap"
        );
        assert_eq!(
            violations, 0,
            "teardown must never signal a process-group id after the reap"
        );
        assert!(
            process_exists(unrelated_pid),
            "an unrelated process group survives teardown"
        );
        assert!(!process_exists(victim_pid), "the victim leader is reaped");

        terminate(&mut unrelated);
    }
}
