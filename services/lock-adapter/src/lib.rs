// SPDX-License-Identifier: MIT
//! The Lock Screen policy adapter (T-15.8a).
//!
//! Lock Screen policy is **session- and compositor-native**. There is no
//! external daemon to wrap: the session's idle/lock policy engine
//! (`services/session/src/idle.rs`, [ADR 0070]) owns the `idle.*` stage
//! delays, and the compositor owns the one fail-secure lock state
//! (`compositor/src/lock.rs`, [ADR 0067]); the shell mirrors both over the
//! private `df_toplevel_manager` bridge. This crate is the adapter over that
//! host stack. It never re-times an idle stage or re-implements a lock
//! transition — it reuses the session's [`IdlePolicy`]/[`IdleStage`]
//! vocabulary and faces their published state:
//!
//! * the runtime **lock state** (locked / unlocked),
//! * the effective **idle/lock delays** (dim/blank/lock/suspend), and
//! * the lock-screen **display options** (identity, password hints, custom
//!   message, power buttons),
//!
//! behind a [`LockPolicySource`] seam, with a pure [`LockPolicyChange`] diff
//! as the event stream.
//!
//! # The read path
//!
//! 1. A [`LockPolicySource`] returns the raw [`LockPolicyData`] once per
//!    [`LockPolicyAdapter::refresh`] (or absence/error).
//! 2. [`LockPolicySnapshot::from_data`] types the lock state, carries the
//!    reused idle policy, and names the display options the pane and tile
//!    draw.
//! 3. The adapter drives the shared subscription lifecycle, so a host stack
//!    that comes and goes re-subscribes and re-syncs with no user-visible
//!    error.
//!
//! # Read-only runtime, settingsd-owned preferences
//!
//! The compositor owns the lock and the session owns the timing; the shell
//! only mirrors them, and the durable display preferences are `settingsd`'s.
//! A consumer that wants to change a delay or a display option writes the
//! settings key (the `lock.*` keys arrive in T-15.8b); the engine applies it
//! live; the bridge publishes the new policy; the adapter reports it. There is
//! no write method here, exactly as `dragonfruit-overview` and
//! `dragonfruit-input` have none.
//!
//! # Absence
//!
//! A missing bridge (the shell has no `df_toplevel_manager` global, or an
//! unreachable compositor) is the adapter's `Unavailable` state and hides the
//! item. This is a normal state — the idle/lock policy still applies from
//! `settingsd`'s defaults and no session startup is blocked. A present bridge
//! that cannot be read is `Error`: visible and inert with the message.
//!
//! # Testing
//!
//! CI has no compositor, so the adapter is driven by [`MockLockPolicy`] over
//! the source seam. `kill`/`restart` exercise absence and re-subscribe; `push`
//! drives the lock state, the delays, and the display options.
//!
//! [ADR 0067]: ../../../docs/design/adr/0067-session-lock-protocol-and-ui.md
//! [ADR 0070]: ../../../docs/design/adr/0070-idle-timer-engine-and-policy.md

mod adapter;
mod model;
mod source;

pub use adapter::LockPolicyAdapter;
pub use model::{LockDisplayOption, LockPolicyChange, LockPolicySnapshot, LockState, IDLE_STAGES};
pub use source::{LockDisplay, LockPolicyData, LockPolicySource, MockLockPolicy};
// Re-export the reused idle vocabulary so a consumer binds to one import.
pub use dragonfruit_session::{IdlePolicy, IdleStage};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_lock_slot() {
        let adapter = LockPolicyAdapter::new(MockLockPolicy::absent());
        assert_eq!(
            <LockPolicyAdapter<MockLockPolicy> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::LOCK
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = LockPolicyAdapter::new(MockLockPolicy::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_idle_vocabulary_is_the_session_one() {
        assert_eq!(IdleStage::Lock.as_str(), "lock");
        assert_eq!(
            IdlePolicy::new().delay(IdleStage::Lock).unwrap().as_secs(),
            600
        );
    }
}
