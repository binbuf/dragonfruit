// SPDX-License-Identifier: MIT
//! The General adapter: one read path over the shared contract (T226).
//!
//! It holds the last snapshot the General host stack published and exposes the
//! three-state [`AdapterState`] contract. The Settings host calls
//! [`refresh`](GeneralAdapter::refresh) when the pane opens or after an
//! explicit write; nothing above the adapter polls.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::GeneralSnapshot;
use crate::source::{GeneralOutcome, GeneralSource};

/// The General group's adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralAdapter<S> {
    source: S,
    state: AdapterState<GeneralSnapshot>,
    subscription: Subscription,
}

impl<S: GeneralSource> GeneralAdapter<S> {
    /// A new adapter starts unavailable and absent, the safe default.
    pub fn new(source: S) -> Self {
        GeneralAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::default(),
        }
    }

    /// Re-read the host stack once.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(snapshot)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(snapshot);
                self.subscription.changed();
            }
            Ok(None) => {
                self.subscription.absent();
                self.state = AdapterState::Unavailable;
            }
            Err(error) => {
                self.subscription.absent();
                self.state = AdapterState::with_error(error.message());
            }
        }
    }

    /// The last snapshot, when available.
    pub fn snapshot(&self) -> Option<&GeneralSnapshot> {
        self.state.snapshot()
    }

    /// The transport source.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The source, mutably (tests).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Set the system time zone.
    pub fn set_timezone(&mut self, timezone: &str) -> GeneralOutcome {
        self.source.set_timezone(timezone)
    }

    /// Enable or disable NTP.
    pub fn set_ntp(&mut self, enabled: bool) -> GeneralOutcome {
        self.source.set_ntp(enabled)
    }

    /// Set the wall clock.
    pub fn set_time(&mut self, value: &str) -> GeneralOutcome {
        self.source.set_time(value)
    }

    /// Set the system locale.
    pub fn set_locale(&mut self, locale: &str) -> GeneralOutcome {
        self.source.set_locale(locale)
    }

    /// Enable or disable one autostart entry.
    pub fn set_autostart_enabled(&mut self, id: &str, enabled: bool) -> GeneralOutcome {
        self.source.set_autostart_enabled(id, enabled)
    }

    /// Enable or disable one systemd user unit.
    pub fn set_user_unit_enabled(&mut self, unit: &str, enabled: bool) -> GeneralOutcome {
        self.source.set_user_unit_enabled(unit, enabled)
    }

    /// Enable or disable one sharing service.
    pub fn set_sharing_enabled(&mut self, id: &str, enabled: bool) -> GeneralOutcome {
        self.source.set_sharing_enabled(id, enabled)
    }

    /// Run one backup through the selected backend.
    pub fn run_backup(&mut self, destination: &str) -> GeneralOutcome {
        self.source.run_backup(destination)
    }

    /// Open the selected backend's restore entry.
    pub fn open_restore(&mut self) -> GeneralOutcome {
        self.source.open_restore()
    }
}

impl<S: GeneralSource> Adapter for GeneralAdapter<S> {
    type Snapshot = GeneralSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::GENERAL
    }

    fn state(&self) -> &AdapterState<Self::Snapshot> {
        &self.state
    }

    fn connection(&self) -> ConnectionState {
        self.subscription.state()
    }

    fn drain_events(&mut self) -> Vec<AdapterEvent> {
        self.subscription.drain_events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MockGeneral;

    #[test]
    fn an_absent_source_is_unavailable() {
        let mut adapter = GeneralAdapter::new(MockGeneral::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn a_present_source_projects_every_sub_pane() {
        let mut adapter = GeneralAdapter::new(MockGeneral::present());
        adapter.refresh();
        let snapshot = adapter.snapshot().unwrap();
        assert!(snapshot.time.is_some());
        assert!(snapshot.locale.is_some());
        assert!(snapshot.login.is_some());
        assert!(snapshot.sharing.is_some());
        assert!(snapshot.backup.is_some());
        assert!(snapshot.storage.is_some());
        assert!(snapshot.startup.is_some());
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut adapter = GeneralAdapter::new(MockGeneral::failing("timedatectl: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "timedatectl: timeout"
        );
    }

    #[test]
    fn every_write_reaches_the_source() {
        let mut adapter = GeneralAdapter::new(MockGeneral::present());
        adapter.refresh();
        assert!(adapter.set_ntp(false).is_applied());
        assert!(adapter.set_timezone("UTC").is_applied());
        assert!(adapter.set_locale("fr_FR.UTF-8").is_applied());
        assert!(adapter
            .set_autostart_enabled("org.example.Agent", true)
            .is_applied());
        assert!(adapter
            .set_user_unit_enabled("sync.service", true)
            .is_applied());
        assert!(adapter.set_sharing_enabled("file", false).is_applied());
        assert!(adapter.run_backup("/mnt/backup").is_applied());
        assert!(adapter.open_restore().is_applied());
        assert_eq!(adapter.source().writes(), 8);
    }
}
