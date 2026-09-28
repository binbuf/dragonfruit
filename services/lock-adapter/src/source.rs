// SPDX-License-Identifier: MIT
//! The transport seam: the raw Lock Screen policy read and its mock.
//!
//! A [`LockPolicySource`] is the only thing that talks to the host stack.
//! Lock Screen policy is **session- and compositor-native**, so there is no
//! external daemon to wrap: the session's idle/lock policy engine
//! (`services/session/src/idle.rs`) owns the `idle.*` stage delays, and the
//! compositor owns the fail-secure lock state
//! (`compositor/src/lock.rs`), which the shell mirrors over the private
//! `df_toplevel_manager` bridge ([ADR 0067], [ADR 0122]). This crate does not
//! re-time an idle stage or re-implement a lock transition; it is the
//! **projection** — the effective idle/lock delays, the lock-screen display
//! options, and the runtime lock state, delivered through a seam so the shell
//! bridge (T-15.8b) and CI's [`MockLockPolicy`] answer identically.
//!
//! The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — the host stack answered; `data` is the live read.
//! * `Ok(None)` — the host stack is absent (no compositor bridge, or a session
//!   with no idle engine). A normal state; the item hides.
//! * `Err(error)` — the host stack is present but could not be read; the item
//!   shows visible and inert with the message.
//!
//! [ADR 0067]: ../../../docs/design/adr/0067-session-lock-protocol-and-ui.md
//! [ADR 0122]: ../../../docs/design/adr/0122-tahoe-interface-language-across-chrome.md

use dragonfruit_session::IdlePolicy;
use dragonfruit_system_adapters::AdapterError;

/// The lock-screen display options the host stack publishes.
///
/// These are the second untitled group of the macOS Lock Screen pane
/// ([System_Preferences.md](../../../docs/reference/System_Preferences.md)):
/// whether the lock screen shows the account identity, whether PAM may echo a
/// password hint, whether a custom message is drawn while locked, and whether
/// the sleep/restart/shut-down buttons are offered. The durable preference is
/// owned by `settingsd` (the keys land in T-15.8b); this is the projection of
/// the option the host stack is currently applying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockDisplay {
    /// Show the account name and avatar on the lock screen.
    pub show_user_name_and_photo: bool,
    /// Allow PAM to echo a password hint (T-12.3b).
    pub show_password_hints: bool,
    /// Draw a custom message while the session is locked.
    pub show_message_when_locked: bool,
    /// The custom locked message, empty when none is set.
    pub message: String,
    /// Offer the Sleep, Restart, and Shut Down buttons on the lock screen.
    pub show_power_buttons: bool,
}

impl Default for LockDisplay {
    /// The tasteful defaults the pane ships with: identity shown, no password
    /// hints, no custom message, and the power buttons offered — the macOS
    /// capture's states, adapted to the Linux lock UI.
    fn default() -> Self {
        LockDisplay {
            show_user_name_and_photo: true,
            show_password_hints: false,
            show_message_when_locked: false,
            message: String::new(),
            show_power_buttons: true,
        }
    }
}

/// The raw result of one Lock Screen policy read: the runtime lock state, the
/// effective idle/lock delays, and the lock-screen display options.
///
/// The idle delays are the session's own [`IdlePolicy`], reused rather than
/// re-modelled, so a consumer reads the same stage vocabulary the idle engine
/// applies. The display options are the projection of the settingsd policy
/// T-15.8b declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockPolicyData {
    /// Whether the session is currently locked (the compositor's flag).
    pub locked: bool,
    /// The effective idle/lock stage delays (dim/blank/lock/suspend).
    pub idle: IdlePolicy,
    /// The lock-screen display options.
    pub display: LockDisplay,
}

impl Default for LockPolicyData {
    /// An unlocked session applying the idle engine's shipped defaults and
    /// the display defaults above.
    fn default() -> Self {
        LockPolicyData {
            locked: false,
            idle: IdlePolicy::new(),
            display: LockDisplay::default(),
        }
    }
}

/// Reads the Lock Screen policy host stack over some transport.
pub trait LockPolicySource {
    /// One read of the host stack.
    fn read(&mut self) -> Result<Option<LockPolicyData>, AdapterError>;
}

/// A fixture-backed source with a simulated host-stack lifecycle.
///
/// The mock is the CI path: it serves [`LockPolicyData`] with no compositor
/// and no Wayland connection, and `kill`/`restart` exercise absence and
/// re-subscribe the way losing the shell bridge would. `push` drives the lock
/// state and the display options so a test observes the change stream.
#[derive(Debug, Clone, PartialEq)]
pub struct MockLockPolicy {
    present: bool,
    data: Option<LockPolicyData>,
    failure: Option<AdapterError>,
    reads: u32,
}

impl MockLockPolicy {
    /// The host stack is not reachable.
    pub fn absent() -> Self {
        MockLockPolicy {
            present: false,
            data: None,
            failure: None,
            reads: 0,
        }
    }

    /// A host stack that answers with `data`.
    pub fn present(data: LockPolicyData) -> Self {
        MockLockPolicy {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
        }
    }

    /// A host stack that fails every read (e.g. the bridge went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockLockPolicy {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
        }
    }

    /// The host stack publishes fresh state.
    pub fn push(&mut self, data: LockPolicyData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The host stack goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host stack comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated host stack is reachable.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }
}

impl LockPolicySource for MockLockPolicy {
    fn read(&mut self) -> Result<Option<LockPolicyData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_session::IdleStage;

    fn data(locked: bool) -> LockPolicyData {
        let mut idle = IdlePolicy::disabled();
        idle.set_delay(IdleStage::Blank, Some(std::time::Duration::from_secs(600)));
        idle.set_delay(IdleStage::Lock, Some(std::time::Duration::from_secs(900)));
        LockPolicyData {
            locked,
            idle,
            display: LockDisplay {
                show_user_name_and_photo: true,
                show_password_hints: true,
                show_message_when_locked: true,
                message: "Back at 3.".to_owned(),
                show_power_buttons: false,
            },
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockLockPolicy::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.is_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockLockPolicy::present(data(true));
        assert_eq!(mock.read(), Ok(Some(data(true))));
        assert!(mock.is_present());
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockLockPolicy::failing("lock bridge: timeout");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "lock bridge: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockLockPolicy::present(data(false));
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data(false)));
    }

    #[test]
    fn push_replaces_the_served_data() {
        let mut mock = MockLockPolicy::present(data(false));
        mock.push(data(true));
        assert!(mock.read().unwrap().unwrap().locked);
    }

    #[test]
    fn the_defaults_mirror_the_shipped_policy() {
        let default = LockPolicyData::default();
        assert!(!default.locked);
        assert_eq!(default.idle, IdlePolicy::new());
        assert!(default.display.show_user_name_and_photo);
        assert!(!default.display.show_password_hints);
        assert!(!default.display.show_message_when_locked);
        assert!(default.display.message.is_empty());
        assert!(default.display.show_power_buttons);
    }
}
