// SPDX-License-Identifier: MIT
//! One suspend/resume cycle (T-12.5a).
//!
//! A suspend is a logind `PrepareForSleep` round trip: the compositor
//! **quiesces** before the machine sleeps (no new frames, no user input) and
//! then **re-inits** on wake (every output repaints, clients receive their
//! pending frame callbacks, input routes again). The session survives the
//! cycle: no client is disconnected, the scene is untouched, and no service
//! restarts.
//!
//! [`SuspendModel`] is the compositor's one piece of durable state: whether
//! the session is suspended and how many complete cycles it has run. It is
//! deliberately backend-agnostic and pure, so the headless conformance test
//! can drive it without a display, a GPU, or sleeping the CI machine.
//!
//! The state is applied by [`DfState::suspend_session`] and
//! [`DfState::resume_session`]; the DRM backend also clears and re-creates its
//! output surfaces across a logind session pause (FR-6), so the same model
//! covers both a VT switch and a sleep/wake. The production trigger is the
//! session manager forwarding logind's `PrepareForSleep`; the headless
//! synthetic harness ([`crate::input::synthetic`]) is the test trigger.

/// The compositor's suspend/resume state.
///
/// `suspended` is the one flag every render and input path consults while
/// asleep; `cycles` counts completed suspend→resume round trips, so a later
/// soak (T-16) can assert the count.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SuspendModel {
    suspended: bool,
    cycles: u32,
}

impl SuspendModel {
    /// Awake, with no completed cycles.
    pub const fn new() -> Self {
        SuspendModel {
            suspended: false,
            cycles: 0,
        }
    }

    /// Whether the session is currently suspended (quiesced).
    pub const fn is_suspended(&self) -> bool {
        self.suspended
    }

    /// How many suspend→resume cycles have completed.
    pub const fn cycles(&self) -> u32 {
        self.cycles
    }

    /// Enter the suspended state. Returns `false` when already suspended
    /// (a duplicate logind hook is idempotent).
    pub fn suspend(&mut self) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        true
    }

    /// Leave the suspended state and count one completed cycle. Returns
    /// `false` when already awake (a duplicate resume hook is idempotent).
    pub fn resume(&mut self) -> bool {
        if !self.suspended {
            return false;
        }
        self.suspended = false;
        self.cycles += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspend_and_resume_are_idempotent_and_count_cycles() {
        let mut model = SuspendModel::new();
        assert!(!model.is_suspended());
        assert_eq!(model.cycles(), 0);

        // A duplicate prepare does not count twice.
        assert!(model.suspend());
        assert!(!model.suspend());
        assert!(model.is_suspended());
        assert_eq!(model.cycles(), 0);

        // A duplicate resume does not count twice.
        assert!(model.resume());
        assert!(!model.resume());
        assert!(!model.is_suspended());
        assert_eq!(model.cycles(), 1);

        // A second full cycle keeps the session alive and increments.
        assert!(model.suspend());
        assert!(model.resume());
        assert_eq!(model.cycles(), 2);
    }
}
