// SPDX-License-Identifier: MIT
//! The suspend/resume cycle (T-12.5a).
//!
//! The idle chain reaches [`IdleStage::Suspend`](crate::idle::IdleStage::Suspend)
//! after the configured delay; this module turns that into one platform
//! suspend and keeps the session alive across the wake. It is the consumer
//! [ADR 0071](../../docs/design/adr/0071-idle-inhibitors-and-wake-restore.md)
//! names for `IdleEvent::Enter(Suspend)` / `Restore(Suspend)`.
//!
//! The cycle is deliberately two layers:
//!
//! * [`SuspendCycle`] is pure state. It knows whether a suspend is only
//!   *requested* or already *confirmed* by the platform, and counts completed
//!   cycles. It is clock-free and backend-free, so it is exhaustively unit
//!   tested.
//! * [`SuspendController`] binds the cycle to a [`SuspendBackend`] — the seam
//!   that asks logind to suspend. In CI there is no system bus, so the tests
//!   use a recording [`MockSuspend`]; the real session provides a logind
//!   backend behind the same trait.
//!
//! A suspend is a round trip driven by two signals: the idle chain's
//! `Enter(Suspend)` (we *request* it) and logind's `PrepareForSleep(true)`
//! (the platform *confirms* it). A wake is `PrepareForSleep(false)` or, if the
//! request never landed, a user activity that *cancels* it. In every path the
//! session composition is untouched: the services keep running and only the
//! screen state changes.

use crate::idle::{IdleEvent, IdleStage};

/// Where the session is in the suspend/resume cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendState {
    /// Running normally.
    Awake,
    /// The idle chain asked to suspend, but the platform has not confirmed
    /// sleep yet. A user activity in this window cancels the request.
    Requested,
    /// The platform confirmed sleep (`PrepareForSleep(true)`).
    Asleep,
}

impl SuspendState {
    /// The stable name used in logs and tests.
    pub const fn as_str(self) -> &'static str {
        match self {
            SuspendState::Awake => "awake",
            SuspendState::Requested => "requested",
            SuspendState::Asleep => "asleep",
        }
    }

    /// Whether the session is awake.
    pub const fn is_awake(self) -> bool {
        matches!(self, SuspendState::Awake)
    }

    /// Whether a suspend is requested but not yet confirmed.
    pub const fn is_requested(self) -> bool {
        matches!(self, SuspendState::Requested)
    }

    /// Whether the platform confirmed sleep.
    pub const fn is_asleep(self) -> bool {
        matches!(self, SuspendState::Asleep)
    }
}

impl std::fmt::Display for SuspendState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What the platform must do because of a cycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendRequest {
    /// Ask the platform to suspend now (logind `Manager.Suspend`).
    Suspend,
    /// The request is no longer needed (activity arrived first); abort it.
    Cancel,
}

/// The seam to the platform's suspend mechanism.
///
/// The production implementation calls logind over D-Bus; tests record calls.
/// `Cancel` is best-effort: a platform that cannot abort a not-yet-landed
/// request may ignore it, and the cycle is still consistent because it only
/// waits for the real `PrepareForSleep`.
pub trait SuspendBackend {
    /// Ask the platform to suspend.
    fn request_suspend(&mut self) -> Result<(), String>;
    /// Abort a suspend request that has not been confirmed yet.
    fn cancel_suspend(&mut self) -> Result<(), String>;
}

/// The pure suspend/resume cycle state.
///
/// Transitions are:
///
/// ```text
/// Awake ──request──▶ Requested ──prepare(true)──▶ Asleep
///   ▲                    │                          │
///   │                    └──────── activity ────────┘ (cancel)
///   └──────────────────── prepare(false) ───────────┘ (resume, +1 cycle)
/// ```
///
/// `prepare_for_sleep(true)` also accepts `Awake` directly: a lid close or a
/// power button suspends the machine without this session asking, and logind
/// still emits the hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuspendCycle {
    state: SuspendState,
    cycles: u32,
}

impl SuspendCycle {
    /// Awake, with no completed cycles.
    pub const fn new() -> Self {
        SuspendCycle {
            state: SuspendState::Awake,
            cycles: 0,
        }
    }

    /// The current state.
    pub const fn state(&self) -> SuspendState {
        self.state
    }

    /// Whether the session is awake.
    pub const fn is_awake(&self) -> bool {
        self.state.is_awake()
    }

    /// Whether a suspend is requested but not yet confirmed.
    pub const fn is_requested(&self) -> bool {
        self.state.is_requested()
    }

    /// Whether the platform confirmed sleep.
    pub const fn is_asleep(&self) -> bool {
        self.state.is_asleep()
    }

    /// How many suspend→resume cycles have completed.
    pub const fn cycles(&self) -> u32 {
        self.cycles
    }

    /// The idle chain reached `Suspend`: request a suspend.
    ///
    /// A request from `Requested` or `Asleep` is a no-op; returns `Some` only
    /// on the transition out of `Awake`. A wake (activity or
    /// `prepare_for_sleep(false)`) may then complete the cycle.
    pub fn request(&mut self) -> Option<SuspendRequest> {
        if !self.state.is_awake() {
            return None;
        }
        self.state = SuspendState::Requested;
        Some(SuspendRequest::Suspend)
    }

    /// The platform reported `PrepareForSleep`.
    ///
    /// `entering` confirms sleep (from `Requested` or, for an externally
    /// triggered suspend, straight from `Awake`); the confirmation does not
    /// itself complete the cycle — the wake does. Leaving sleep resumes the
    /// session and counts one cycle. A duplicate hook is a no-op.
    pub fn prepare_for_sleep(&mut self, entering: bool) -> Option<SuspendRequest> {
        if entering {
            if self.state.is_asleep() {
                return None;
            }
            self.state = SuspendState::Asleep;
            None
        } else {
            if !self.state.is_asleep() {
                return None;
            }
            self.state = SuspendState::Awake;
            self.cycles += 1;
            None
        }
    }

    /// Record user activity.
    ///
    /// Activity only matters before sleep lands: a `Requested` suspend is
    /// cancelled and the session returns to `Awake`. Activity while already
    /// asleep is ignored — the wake path is `PrepareForSleep(false)`.
    pub fn activity(&mut self) -> Option<SuspendRequest> {
        if !self.state.is_requested() {
            return None;
        }
        self.state = SuspendState::Awake;
        Some(SuspendRequest::Cancel)
    }
}

impl Default for SuspendCycle {
    fn default() -> Self {
        SuspendCycle::new()
    }
}

/// A recording backend for tests, mirroring the crate's other mock adapters.
#[derive(Debug, Default, Clone)]
pub struct MockSuspend {
    suspend_requests: u32,
    cancel_requests: u32,
    fail: bool,
}

impl MockSuspend {
    /// A backend that records every call and never fails.
    pub fn new() -> Self {
        Self::default()
    }

    /// A backend whose `request_suspend` always fails, to exercise the error
    /// path without a bus.
    pub fn failing() -> Self {
        MockSuspend {
            fail: true,
            ..MockSuspend::default()
        }
    }

    /// How many times `request_suspend` was called.
    pub fn suspend_requests(&self) -> u32 {
        self.suspend_requests
    }

    /// How many times `cancel_suspend` was called.
    pub fn cancel_requests(&self) -> u32 {
        self.cancel_requests
    }
}

impl SuspendBackend for MockSuspend {
    fn request_suspend(&mut self) -> Result<(), String> {
        self.suspend_requests += 1;
        if self.fail {
            Err("mock suspend failure".into())
        } else {
            Ok(())
        }
    }

    fn cancel_suspend(&mut self) -> Result<(), String> {
        self.cancel_requests += 1;
        Ok(())
    }
}

/// The cycle plus the platform backend that actually suspends the machine.
pub struct SuspendController<B: SuspendBackend> {
    cycle: SuspendCycle,
    backend: B,
    last_error: Option<String>,
}

impl<B: SuspendBackend> SuspendController<B> {
    /// Start awake with `backend`.
    pub fn new(backend: B) -> Self {
        SuspendController {
            cycle: SuspendCycle::new(),
            backend,
            last_error: None,
        }
    }

    /// The pure cycle.
    pub fn cycle(&self) -> &SuspendCycle {
        &self.cycle
    }

    /// The current state.
    pub fn state(&self) -> SuspendState {
        self.cycle.state()
    }

    /// Whether the session is awake.
    pub fn is_awake(&self) -> bool {
        self.cycle.is_awake()
    }

    /// Whether the platform confirmed sleep.
    pub fn is_asleep(&self) -> bool {
        self.cycle.is_asleep()
    }

    /// How many suspend→resume cycles have completed.
    pub fn cycles(&self) -> u32 {
        self.cycle.cycles()
    }

    /// The backend, for inspection (a test reads the mock's call counts).
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// The last backend error, if the platform rejected a request.
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Drive the cycle from an idle-chain event.
    ///
    /// `Enter(Suspend)` requests a suspend; `Restore(Suspend)` cancels one
    /// that has not landed. Every other stage is ignored — the backlight and
    /// lock stages are a different consumer's business. Returns whether the
    /// cycle state changed.
    pub fn on_idle_event(&mut self, event: IdleEvent) -> bool {
        match event {
            IdleEvent::Enter(IdleStage::Suspend) => self.request(),
            IdleEvent::Restore(IdleStage::Suspend) => self.activity(),
            _ => false,
        }
    }

    /// Drive the cycle from logind's `PrepareForSleep`. Returns whether the
    /// cycle state changed.
    pub fn prepare_for_sleep(&mut self, entering: bool) -> bool {
        let before = self.cycle.state();
        self.cycle.prepare_for_sleep(entering);
        self.cycle.state() != before
    }

    /// Record user activity; cancels a not-yet-confirmed request.
    pub fn activity(&mut self) -> bool {
        let request = self.cycle.activity();
        self.apply(request)
    }

    /// Request a suspend from the idle chain.
    fn request(&mut self) -> bool {
        let request = self.cycle.request();
        self.apply(request)
    }

    /// Run the backend for `request` and report whether state changed.
    fn apply(&mut self, request: Option<SuspendRequest>) -> bool {
        match request {
            None => false,
            Some(SuspendRequest::Suspend) => {
                if let Err(error) = self.backend.request_suspend() {
                    self.last_error = Some(error);
                }
                true
            }
            Some(SuspendRequest::Cancel) => {
                if let Err(error) = self.backend.cancel_suspend() {
                    self.last_error = Some(error);
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_idempotent_until_sleep_lands() {
        let mut cycle = SuspendCycle::new();
        assert!(cycle.is_awake());
        assert_eq!(cycle.request(), Some(SuspendRequest::Suspend));
        assert!(cycle.is_requested());
        // A second Enter(Suspend) does not request twice.
        assert_eq!(cycle.request(), None);

        // Confirming sleep moves to Asleep without counting a cycle yet.
        assert_eq!(cycle.prepare_for_sleep(true), None);
        assert!(cycle.is_asleep());
        assert_eq!(cycle.cycles(), 0);

        // Waking completes exactly one cycle.
        assert_eq!(cycle.prepare_for_sleep(false), None);
        assert!(cycle.is_awake());
        assert_eq!(cycle.cycles(), 1);
        // A duplicate wake is a no-op.
        assert_eq!(cycle.prepare_for_sleep(false), None);
        assert_eq!(cycle.cycles(), 1);
    }

    #[test]
    fn activity_cancels_a_request_that_never_landed() {
        let mut cycle = SuspendCycle::new();
        assert_eq!(cycle.request(), Some(SuspendRequest::Suspend));
        assert_eq!(cycle.activity(), Some(SuspendRequest::Cancel));
        assert!(cycle.is_awake());
        assert_eq!(cycle.cycles(), 0);
        // Activity while already awake is a no-op, and cannot cancel sleep.
        assert_eq!(cycle.activity(), None);
        cycle.request();
        cycle.prepare_for_sleep(true);
        assert_eq!(cycle.activity(), None, "wake, not activity, ends sleep");
        assert!(cycle.is_asleep());
    }

    #[test]
    fn an_external_suspend_confirms_straight_from_awake() {
        // A lid close suspends without this session asking; logind still
        // emits the hook, and the wake counts a cycle.
        let mut cycle = SuspendCycle::new();
        assert_eq!(cycle.prepare_for_sleep(true), None);
        assert!(cycle.is_asleep());
        assert_eq!(cycle.prepare_for_sleep(false), None);
        assert_eq!(cycle.cycles(), 1);
    }

    #[test]
    fn a_spurious_wake_does_not_count_a_cycle() {
        let mut cycle = SuspendCycle::new();
        assert_eq!(cycle.prepare_for_sleep(false), None);
        assert!(cycle.is_awake());
        assert_eq!(cycle.cycles(), 0);
    }

    #[test]
    fn the_controller_requests_and_cancels_through_the_backend() {
        let mut controller = SuspendController::new(MockSuspend::new());
        assert!(controller.is_awake());

        assert!(controller.on_idle_event(IdleEvent::Enter(IdleStage::Suspend)));
        assert_eq!(controller.backend().suspend_requests(), 1);
        // A repeated Enter does not hit the backend again.
        assert!(!controller.on_idle_event(IdleEvent::Enter(IdleStage::Suspend)));
        assert_eq!(controller.backend().suspend_requests(), 1);

        assert!(controller.on_idle_event(IdleEvent::Restore(IdleStage::Suspend)));
        assert_eq!(controller.backend().cancel_requests(), 1);
        assert!(controller.is_awake());
    }

    #[test]
    fn the_controller_ignores_other_idle_stages() {
        let mut controller = SuspendController::new(MockSuspend::new());
        for event in [
            IdleEvent::Enter(IdleStage::Dim),
            IdleEvent::Enter(IdleStage::Blank),
            IdleEvent::Enter(IdleStage::Lock),
            IdleEvent::Restore(IdleStage::Dim),
            IdleEvent::Restore(IdleStage::Lock),
        ] {
            assert!(!controller.on_idle_event(event), "{event:?}");
        }
        assert_eq!(controller.backend().suspend_requests(), 0);
    }

    #[test]
    fn a_full_controller_cycle_keeps_the_session_alive() {
        let mut controller = SuspendController::new(MockSuspend::new());
        controller.on_idle_event(IdleEvent::Enter(IdleStage::Suspend));
        assert!(controller.prepare_for_sleep(true));
        assert!(controller.is_asleep());
        assert!(controller.prepare_for_sleep(false));
        assert!(controller.is_awake());
        assert_eq!(controller.cycles(), 1);
        assert_eq!(controller.backend().suspend_requests(), 1);
        assert_eq!(controller.backend().cancel_requests(), 0);
        assert_eq!(controller.last_error(), None);
    }

    #[test]
    fn a_backend_error_is_recorded_without_breaking_the_cycle() {
        let mut controller = SuspendController::new(MockSuspend::failing());
        assert!(controller.on_idle_event(IdleEvent::Enter(IdleStage::Suspend)));
        assert_eq!(controller.backend().suspend_requests(), 1);
        assert_eq!(controller.last_error(), Some("mock suspend failure"));
        // The cycle still tracks the platform's real state.
        controller.prepare_for_sleep(true);
        controller.prepare_for_sleep(false);
        assert_eq!(controller.cycles(), 1);
    }
}
