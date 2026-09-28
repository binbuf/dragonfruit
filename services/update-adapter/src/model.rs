// SPDX-License-Identifier: MIT
//! The General, About, and Updates snapshot the pane and tile render, decoded
//! from one raw read.
//!
//! The model owns only the projection a consumer should not repeat: it carries
//! the host identity, types the update state, derives the labels and glyph the
//! tile draws, and computes the change stream between two reads. It never
//! checks for updates, never installs, and never writes a settings key — the
//! distribution provider owns the update operations and `settingsd` owns the
//! durable preferences; this is the data they publish.
//!
//! The event half is a pure diff: [`SystemSnapshot::changes`] reports what
//! moved between two reads (the identity, the provider's presence, its phase,
//! the offered list, the last-check time, or its message) without any polling.

use crate::source::{
    SystemData, SystemIdentity, UpdateData, UpdateItem, UpdatePhase, UpdateSeverity,
};

/// The General, About, and Updates snapshot a pane or tile renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSnapshot {
    /// The host identity (the About/General rows).
    pub identity: SystemIdentity,
    /// The distribution update provider's state; `None` when it is absent.
    pub updates: Option<UpdateData>,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        SystemSnapshot::from_data(&SystemData::default())
    }
}

impl SystemSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &SystemData) -> Self {
        SystemSnapshot {
            identity: data.identity.clone(),
            updates: data.updates.clone(),
        }
    }

    /// Whether the distribution update provider is present.
    pub const fn updates_available(&self) -> bool {
        self.updates.is_some()
    }

    /// The provider's phase, when it is present.
    pub fn phase(&self) -> Option<UpdatePhase> {
        self.updates.as_ref().map(|updates| updates.phase)
    }

    /// Whether a check or install is currently in flight.
    pub fn is_busy(&self) -> bool {
        self.phase().is_some_and(UpdatePhase::is_busy)
    }

    /// Whether the host must restart to finish installing updates.
    pub fn is_reboot_required(&self) -> bool {
        self.phase() == Some(UpdatePhase::RebootRequired)
    }

    /// The updates on offer, in the provider's order; empty when the provider
    /// is absent or has none.
    pub fn updates(&self) -> &[UpdateItem] {
        self.updates
            .as_ref()
            .map(|updates| updates.updates.as_slice())
            .unwrap_or(&[])
    }

    /// How many updates are on offer.
    pub fn update_count(&self) -> usize {
        self.updates().len()
    }

    /// How many of the offered updates are security updates.
    pub fn security_count(&self) -> usize {
        self.updates()
            .iter()
            .filter(|update| update.severity == UpdateSeverity::Security)
            .count()
    }

    /// When the last check completed (milliseconds since the Unix epoch).
    pub fn last_checked_ms(&self) -> Option<u64> {
        self.updates
            .as_ref()
            .and_then(|updates| updates.last_checked_ms)
    }

    /// The provider's status or error text, when it has one.
    pub fn message(&self) -> Option<&str> {
        self.updates
            .as_ref()
            .and_then(|updates| updates.message.as_deref())
    }

    /// The one-line label for the Software Update tile.
    pub fn label(&self) -> String {
        let Some(updates) = &self.updates else {
            return "Software Update Unavailable".to_owned();
        };
        match updates.phase {
            UpdatePhase::Idle => "Check for Updates".to_owned(),
            UpdatePhase::Checking => "Checking for Updates…".to_owned(),
            UpdatePhase::Installing => "Installing Updates…".to_owned(),
            UpdatePhase::RebootRequired => "Restart Required".to_owned(),
            UpdatePhase::Failed => "Update Failed".to_owned(),
            UpdatePhase::UpToDate => "Up to Date".to_owned(),
            UpdatePhase::Available => match updates.updates.len() {
                0 => "Up to Date".to_owned(),
                1 => "1 Update Available".to_owned(),
                count => format!("{count} Updates Available"),
            },
        }
    }

    /// The design-system glyph for the Software Update tile.
    pub fn glyph(&self) -> &'static str {
        "software-update"
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// the identity, then the provider's presence, phase, list, last-check
    /// time, and message.
    ///
    /// This is the "event" half of the adapter: the shell bridge diffs two
    /// reads to learn what moved without polling each field.
    pub fn changes(&self, previous: &SystemSnapshot) -> Vec<UpdateChange> {
        let mut changes = Vec::new();
        if self.identity != previous.identity {
            changes.push(UpdateChange::IdentityChanged);
        }
        match (&previous.updates, &self.updates) {
            (None, Some(_)) => changes.push(UpdateChange::ProviderChanged { available: true }),
            (Some(_), None) => changes.push(UpdateChange::ProviderChanged { available: false }),
            _ => {}
        }
        if let (Some(previous), Some(current)) = (&previous.updates, &self.updates) {
            if previous.phase != current.phase {
                changes.push(UpdateChange::PhaseChanged {
                    from: previous.phase,
                    to: current.phase,
                });
            }
            if previous.updates.len() != current.updates.len() {
                changes.push(UpdateChange::UpdateListChanged {
                    from: previous.updates.len(),
                    to: current.updates.len(),
                });
            }
            if previous.last_checked_ms != current.last_checked_ms {
                changes.push(UpdateChange::LastCheckedChanged {
                    from: previous.last_checked_ms,
                    to: current.last_checked_ms,
                });
            }
            if previous.message != current.message {
                changes.push(UpdateChange::MessageChanged {
                    from: previous.message.clone(),
                    to: current.message.clone(),
                });
            }
        }
        changes
    }
}

/// A change between two General/About/Updates snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateChange {
    /// The host identity moved (a rename or a release change).
    IdentityChanged,
    /// The update provider appeared or went away.
    ProviderChanged {
        /// `true` when the provider appeared, `false` when it vanished.
        available: bool,
    },
    /// The provider's phase changed.
    PhaseChanged {
        /// The previous phase.
        from: UpdatePhase,
        /// The new phase.
        to: UpdatePhase,
    },
    /// The offered-update list changed size.
    UpdateListChanged {
        /// The previous count.
        from: usize,
        /// The new count.
        to: usize,
    },
    /// The last-check timestamp moved.
    LastCheckedChanged {
        /// The previous timestamp.
        from: Option<u64>,
        /// The new timestamp.
        to: Option<u64>,
    },
    /// The provider's status or error text moved.
    MessageChanged {
        /// The previous message.
        from: Option<String>,
        /// The new message.
        to: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> SystemData {
        SystemData {
            identity: SystemIdentity {
                host_name: "dragon".to_owned(),
                os_name: "Dragonfruit Linux".to_owned(),
                os_version: "44".to_owned(),
                ..SystemIdentity::default()
            },
            updates: Some(UpdateData {
                phase: UpdatePhase::Available,
                updates: vec![UpdateItem {
                    id: "glibc".to_owned(),
                    name: "glibc".to_owned(),
                    summary: "C library".to_owned(),
                    current_version: "2.40".to_owned(),
                    available_version: "2.41".to_owned(),
                    severity: UpdateSeverity::Security,
                }],
                last_checked_ms: Some(1000),
                message: None,
            }),
        }
    }

    #[test]
    fn the_snapshot_projects_the_identity_and_updates() {
        let snapshot = SystemSnapshot::from_data(&data());
        assert_eq!(snapshot.identity.host_name, "dragon");
        assert!(snapshot.updates_available());
        assert_eq!(snapshot.phase(), Some(UpdatePhase::Available));
        assert_eq!(snapshot.update_count(), 1);
        assert_eq!(snapshot.security_count(), 1);
        assert_eq!(snapshot.last_checked_ms(), Some(1000));
        assert_eq!(snapshot.label(), "1 Update Available");
        assert_eq!(snapshot.glyph(), "software-update");
    }

    #[test]
    fn an_absent_provider_is_a_normal_snapshot() {
        let data = SystemData {
            updates: None,
            ..data()
        };
        let snapshot = SystemSnapshot::from_data(&data);
        assert!(!snapshot.updates_available());
        assert_eq!(snapshot.phase(), None);
        assert!(snapshot.updates().is_empty());
        assert_eq!(snapshot.update_count(), 0);
        assert_eq!(snapshot.label(), "Software Update Unavailable");
        // The identity half is still live.
        assert_eq!(snapshot.identity.host_name, "dragon");
    }

    #[test]
    fn labels_follow_the_phase() {
        let mut raw = data();
        let cases = [
            (UpdatePhase::Idle, "Check for Updates"),
            (UpdatePhase::Checking, "Checking for Updates…"),
            (UpdatePhase::Installing, "Installing Updates…"),
            (UpdatePhase::RebootRequired, "Restart Required"),
            (UpdatePhase::Failed, "Update Failed"),
            (UpdatePhase::UpToDate, "Up to Date"),
        ];
        for (phase, label) in cases {
            raw.updates.as_mut().unwrap().phase = phase;
            let snapshot = SystemSnapshot::from_data(&raw);
            assert_eq!(snapshot.label(), label, "phase {phase:?}");
        }

        raw.updates.as_mut().unwrap().phase = UpdatePhase::Available;
        raw.updates.as_mut().unwrap().updates.clear();
        assert_eq!(SystemSnapshot::from_data(&raw).label(), "Up to Date");

        raw.updates.as_mut().unwrap().updates = vec![UpdateItem::default(), UpdateItem::default()];
        assert_eq!(
            SystemSnapshot::from_data(&raw).label(),
            "2 Updates Available"
        );
    }

    #[test]
    fn busy_and_reboot_flags_follow_the_phase() {
        let mut raw = data();
        raw.updates.as_mut().unwrap().phase = UpdatePhase::Checking;
        let snapshot = SystemSnapshot::from_data(&raw);
        assert!(snapshot.is_busy());
        assert!(!snapshot.is_reboot_required());

        raw.updates.as_mut().unwrap().phase = UpdatePhase::RebootRequired;
        let snapshot = SystemSnapshot::from_data(&raw);
        assert!(!snapshot.is_busy());
        assert!(snapshot.is_reboot_required());
    }

    #[test]
    fn each_move_is_a_change() {
        let previous = SystemSnapshot::from_data(&data());

        let mut raw = data();
        raw.identity.host_name = "renamed".to_owned();
        let updates = raw.updates.as_mut().unwrap();
        updates.phase = UpdatePhase::Installing;
        updates.updates.push(UpdateItem {
            id: "kernel".to_owned(),
            ..UpdateItem::default()
        });
        updates.last_checked_ms = Some(2000);
        updates.message = Some("installing".to_owned());
        let next = SystemSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&UpdateChange::IdentityChanged));
        assert!(changes.contains(&UpdateChange::PhaseChanged {
            from: UpdatePhase::Available,
            to: UpdatePhase::Installing,
        }));
        assert!(changes.contains(&UpdateChange::UpdateListChanged { from: 1, to: 2 }));
        assert!(changes.contains(&UpdateChange::LastCheckedChanged {
            from: Some(1000),
            to: Some(2000),
        }));
        assert!(changes.contains(&UpdateChange::MessageChanged {
            from: None,
            to: Some("installing".to_owned()),
        }));
        assert!(!changes.contains(&UpdateChange::ProviderChanged { available: false }));
    }

    #[test]
    fn the_provider_appearing_and_going_away_is_a_change() {
        let with = SystemSnapshot::from_data(&data());
        let without = SystemSnapshot::from_data(&SystemData {
            updates: None,
            ..data()
        });

        assert!(without
            .changes(&with)
            .contains(&UpdateChange::ProviderChanged { available: false }));
        assert!(with
            .changes(&without)
            .contains(&UpdateChange::ProviderChanged { available: true }));
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = SystemSnapshot::from_data(&data());
        assert!(snapshot.changes(&snapshot).is_empty());
        assert_eq!(
            SystemSnapshot::default(),
            SystemSnapshot::from_data(&SystemData::default())
        );
    }
}
