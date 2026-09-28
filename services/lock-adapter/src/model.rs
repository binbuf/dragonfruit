// SPDX-License-Identifier: MIT
//! The Lock Screen policy snapshot the pane and tile render, decoded from one
//! raw read.
//!
//! The model owns only the projection a consumer should not repeat: it turns
//! the runtime `locked` flag into a typed [`LockState`], carries the session's
//! [`IdlePolicy`] unchanged (it reuses the idle engine's stage vocabulary, it
//! does not re-time anything), names the four lock-screen display options, and
//! computes the change stream between two reads. It never decides to lock or
//! unlock — the compositor owns the one fail-secure lock state
//! ([ADR 0067]) — it is the data that state publishes.
//!
//! A reachable host stack always answers `Available`; the only adapter-level
//! absence is a missing compositor bridge (`AdapterState::Unavailable`), which
//! is a normal hidden state, exactly as `dragonfruit-overview` treats a missing
//! bridge. The event half is a pure diff:
//! [`LockPolicySnapshot::changes`] reports what moved between two reads (the
//! lock state, an idle stage delay, a display option, the message) without any
//! polling.
//!
//! [ADR 0067]: ../../../docs/design/adr/0067-session-lock-protocol-and-ui.md

use std::time::Duration;

use dragonfruit_session::{IdlePolicy, IdleStage};

use crate::source::{LockDisplay, LockPolicyData};

/// The four configurable idle/lock stages, in chain order.
///
/// `Active` is not a delay and is excluded; the session's `IdleStage` is the
/// reused vocabulary, so a consumer and the idle engine agree on the names.
pub const IDLE_STAGES: [IdleStage; 4] = [
    IdleStage::Dim,
    IdleStage::Blank,
    IdleStage::Lock,
    IdleStage::Suspend,
];

/// Whether the session is currently locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LockState {
    /// The desktop is unlocked and interactive.
    #[default]
    Unlocked,
    /// The session is locked; input is captured by the lock UI.
    Locked,
}

impl LockState {
    /// The state for a compositor `locked` flag.
    pub const fn from_locked(locked: bool) -> Self {
        if locked {
            LockState::Locked
        } else {
            LockState::Unlocked
        }
    }

    /// Whether the session is locked.
    pub const fn is_locked(self) -> bool {
        matches!(self, LockState::Locked)
    }

    /// The stable name used in logs and on the wire (`locked` / `unlocked`).
    pub const fn name(self) -> &'static str {
        match self {
            LockState::Locked => "locked",
            LockState::Unlocked => "unlocked",
        }
    }

    /// The human label the pane and tile show.
    pub const fn label(self) -> &'static str {
        match self {
            LockState::Locked => "Locked",
            LockState::Unlocked => "Unlocked",
        }
    }
}

/// One of the four lock-screen display toggles.
///
/// The stable ids match the settingsd keys T-15.8b declares
/// (`lock.showUserNameAndPhoto`, `lock.showPasswordHints`,
/// `lock.showMessageWhenLocked`, `lock.showPowerButtons`), so a projection maps
/// straight onto the settings schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockDisplayOption {
    /// `Show user name and photo`.
    ShowUserNameAndPhoto,
    /// `Show password hints`.
    ShowPasswordHints,
    /// `Show message when locked`.
    ShowMessageWhenLocked,
    /// `Show the Sleep, Restart, and Shut Down buttons`.
    ShowPowerButtons,
}

impl LockDisplayOption {
    /// Every display option, in the pane's row order.
    pub const ALL: [LockDisplayOption; 4] = [
        LockDisplayOption::ShowUserNameAndPhoto,
        LockDisplayOption::ShowPasswordHints,
        LockDisplayOption::ShowMessageWhenLocked,
        LockDisplayOption::ShowPowerButtons,
    ];

    /// The stable id, the suffix of the matching settingsd key.
    pub const fn id(self) -> &'static str {
        match self {
            LockDisplayOption::ShowUserNameAndPhoto => "showUserNameAndPhoto",
            LockDisplayOption::ShowPasswordHints => "showPasswordHints",
            LockDisplayOption::ShowMessageWhenLocked => "showMessageWhenLocked",
            LockDisplayOption::ShowPowerButtons => "showPowerButtons",
        }
    }

    /// The row label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            LockDisplayOption::ShowUserNameAndPhoto => "Show user name and photo",
            LockDisplayOption::ShowPasswordHints => "Show password hints",
            LockDisplayOption::ShowMessageWhenLocked => "Show message when locked",
            LockDisplayOption::ShowPowerButtons => "Show the Sleep, Restart, and Shut Down buttons",
        }
    }
}

/// The Lock Screen policy snapshot a pane or tile renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LockPolicySnapshot {
    /// The runtime lock state.
    pub state: LockState,
    /// The effective idle/lock stage delays (the session's own policy).
    pub idle: IdlePolicy,
    /// The lock-screen display options.
    pub display: LockDisplay,
}

impl LockPolicySnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &LockPolicyData) -> Self {
        LockPolicySnapshot {
            state: LockState::from_locked(data.locked),
            idle: data.idle,
            display: data.display.clone(),
        }
    }

    /// Whether the session is currently locked.
    pub fn is_locked(&self) -> bool {
        self.state.is_locked()
    }

    /// The human lock-state label.
    pub fn state_label(&self) -> &'static str {
        self.state.label()
    }

    /// The design-system glyph name for the item.
    pub fn glyph(&self) -> &'static str {
        "lock"
    }

    /// The configured delay for `stage`, if enabled.
    pub fn idle_delay(&self, stage: IdleStage) -> Option<Duration> {
        self.idle.delay(stage)
    }

    /// The display-off delay (the blank stage), if enabled.
    ///
    /// This is the Linux adaptation of the macOS "Turn display off when
    /// inactive" row: the session's idle engine blanks at one delay, not a
    /// separate battery and adapter pair.
    pub fn display_off_after(&self) -> Option<Duration> {
        self.idle.delay(IdleStage::Blank)
    }

    /// The lock delay (the lock stage), if enabled.
    pub fn lock_after(&self) -> Option<Duration> {
        self.idle.delay(IdleStage::Lock)
    }

    /// Whether `option` is currently on.
    pub fn display_option(&self, option: LockDisplayOption) -> bool {
        match option {
            LockDisplayOption::ShowUserNameAndPhoto => self.display.show_user_name_and_photo,
            LockDisplayOption::ShowPasswordHints => self.display.show_password_hints,
            LockDisplayOption::ShowMessageWhenLocked => self.display.show_message_when_locked,
            LockDisplayOption::ShowPowerButtons => self.display.show_power_buttons,
        }
    }

    /// The custom locked message, empty when none is set.
    pub fn message(&self) -> &str {
        &self.display.message
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// the lock state, each idle stage delay, each display option, then the
    /// message.
    ///
    /// This is the "event" half of the adapter: the host diffs two reads to
    /// learn what moved without polling each field. A stage delay is reported
    /// per stage so a consumer can label it with the reused `IdleStage`.
    pub fn changes(&self, previous: &LockPolicySnapshot) -> Vec<LockPolicyChange> {
        let mut changes = Vec::new();
        if self.state != previous.state {
            changes.push(LockPolicyChange::StateChanged {
                from: previous.state,
                to: self.state,
            });
        }
        for stage in IDLE_STAGES {
            let from = previous.idle.delay(stage);
            let to = self.idle.delay(stage);
            if from != to {
                changes.push(LockPolicyChange::IdleDelayChanged { stage, from, to });
            }
        }
        for option in LockDisplayOption::ALL {
            let from = previous.display_option(option);
            let to = self.display_option(option);
            if from != to {
                changes.push(LockPolicyChange::DisplayOptionChanged { option, from, to });
            }
        }
        if self.display.message != previous.display.message {
            changes.push(LockPolicyChange::MessageChanged {
                from: previous.display.message.clone(),
                to: self.display.message.clone(),
            });
        }
        changes
    }
}

/// A change between two Lock Screen policy snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockPolicyChange {
    /// The session locked or unlocked.
    StateChanged {
        /// The previous state.
        from: LockState,
        /// The new state.
        to: LockState,
    },
    /// One idle/lock stage delay moved (including enable/disable).
    IdleDelayChanged {
        /// The stage whose delay moved.
        stage: IdleStage,
        /// The previous delay, `None` when the stage was disabled.
        from: Option<Duration>,
        /// The new delay, `None` when the stage is disabled.
        to: Option<Duration>,
    },
    /// One lock-screen display toggle moved.
    DisplayOptionChanged {
        /// The option that moved.
        option: LockDisplayOption,
        /// The previous value.
        from: bool,
        /// The new value.
        to: bool,
    },
    /// The custom locked message moved.
    MessageChanged {
        /// The previous message.
        from: String,
        /// The new message.
        to: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::LockDisplay;

    fn display() -> LockDisplay {
        LockDisplay {
            show_user_name_and_photo: true,
            show_password_hints: false,
            show_message_when_locked: false,
            message: String::new(),
            show_power_buttons: true,
        }
    }

    fn data(locked: bool) -> LockPolicyData {
        LockPolicyData {
            locked,
            idle: IdlePolicy::new(),
            display: display(),
        }
    }

    #[test]
    fn the_snapshot_projects_the_lock_state_and_policy() {
        let snapshot = LockPolicySnapshot::from_data(&data(true));
        assert!(snapshot.is_locked());
        assert_eq!(snapshot.state, LockState::Locked);
        assert_eq!(snapshot.state_label(), "Locked");
        assert_eq!(snapshot.glyph(), "lock");
        assert_eq!(snapshot.display_off_after(), Some(Duration::from_secs(300)));
        assert_eq!(snapshot.lock_after(), Some(Duration::from_secs(600)));
        assert!(snapshot.display_option(LockDisplayOption::ShowUserNameAndPhoto));
        assert!(!snapshot.display_option(LockDisplayOption::ShowPasswordHints));
        assert!(snapshot.message().is_empty());
    }

    #[test]
    fn an_unlocked_default_is_not_locked() {
        let snapshot = LockPolicySnapshot::from_data(&LockPolicyData::default());
        assert!(!snapshot.is_locked());
        assert_eq!(snapshot.state_label(), "Unlocked");
        assert_eq!(snapshot.state, LockState::Unlocked);
    }

    #[test]
    fn a_disabled_stage_reads_as_none() {
        let mut idle = IdlePolicy::disabled();
        idle.set_delay(IdleStage::Lock, Some(Duration::from_secs(120)));
        let snapshot = LockPolicySnapshot::from_data(&LockPolicyData {
            locked: false,
            idle,
            display: display(),
        });
        assert_eq!(snapshot.display_off_after(), None);
        assert_eq!(snapshot.lock_after(), Some(Duration::from_secs(120)));
        assert_eq!(snapshot.idle_delay(IdleStage::Dim), None);
    }

    #[test]
    fn the_display_option_ids_are_the_settings_key_suffixes() {
        let ids: Vec<&str> = LockDisplayOption::ALL.iter().map(|o| o.id()).collect();
        assert_eq!(
            ids,
            vec![
                "showUserNameAndPhoto",
                "showPasswordHints",
                "showMessageWhenLocked",
                "showPowerButtons",
            ]
        );
        assert_eq!(
            LockDisplayOption::ShowMessageWhenLocked.label(),
            "Show message when locked"
        );
    }

    #[test]
    fn locking_is_a_change() {
        let previous = LockPolicySnapshot::from_data(&data(false));
        let next = LockPolicySnapshot::from_data(&data(true));
        assert_eq!(
            next.changes(&previous),
            vec![LockPolicyChange::StateChanged {
                from: LockState::Unlocked,
                to: LockState::Locked,
            }]
        );
    }

    #[test]
    fn idle_delays_display_options_and_message_are_changes() {
        let previous = LockPolicySnapshot::from_data(&data(false));

        let mut idle = IdlePolicy::new();
        idle.set_delay(IdleStage::Blank, Some(Duration::from_secs(900)));
        idle.set_delay(IdleStage::Lock, None);
        let mut display = display();
        display.show_password_hints = true;
        display.show_message_when_locked = true;
        display.message = "Back at 3.".to_owned();
        let next = LockPolicySnapshot::from_data(&LockPolicyData {
            locked: false,
            idle,
            display,
        });

        let changes = next.changes(&previous);
        assert!(changes.contains(&LockPolicyChange::IdleDelayChanged {
            stage: IdleStage::Blank,
            from: Some(Duration::from_secs(300)),
            to: Some(Duration::from_secs(900)),
        }));
        assert!(changes.contains(&LockPolicyChange::IdleDelayChanged {
            stage: IdleStage::Lock,
            from: Some(Duration::from_secs(600)),
            to: None,
        }));
        assert!(changes.contains(&LockPolicyChange::DisplayOptionChanged {
            option: LockDisplayOption::ShowPasswordHints,
            from: false,
            to: true,
        }));
        assert!(changes.contains(&LockPolicyChange::MessageChanged {
            from: String::new(),
            to: "Back at 3.".to_owned(),
        }));
        // The options that did not move are not reported.
        assert!(!changes.iter().any(|change| matches!(
            change,
            LockPolicyChange::DisplayOptionChanged {
                option: LockDisplayOption::ShowUserNameAndPhoto,
                ..
            }
        )));
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = LockPolicySnapshot::from_data(&data(true));
        assert!(snapshot.changes(&snapshot).is_empty());
    }
}
