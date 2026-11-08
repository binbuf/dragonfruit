// SPDX-License-Identifier: MIT
//! The General Settings host adapter (T226).
//!
//! The General group is a collection of host-owned sub-panes, not one daemon:
//! Date & Time (`timedatectl`), Language & Region (`localectl` + `settingsd`
//! formats), Login Items (XDG autostart + systemd user units), Sharing
//! (systemd system units), Backup (restic/Pika/Timeshift detection), Storage
//! usage (`df`), and the read-only Startup Disk (`findmnt`). This crate is the
//! one place those tools are named and parsed; the bridge host projects the
//! typed snapshot to JSON and the Settings app never shells out.
//!
//! # Absence is per sub-pane
//!
//! Each sub-pane is an `Option` in [`GeneralSnapshot`]. A missing tool disables
//! only its own pane and is recorded in `errors`; the others stay live. Only a
//! host where no sub-pane answers becomes `Unavailable`, which is a normal
//! hidden state and never blocks session startup.
//!
//! # The live read/write
//!
//! [`HostGeneral`] is the concrete source. It reads the real host and forwards
//! the pane's explicit writes (time zone, NTP, manual time, locale, autostart,
//! user units, sharing, backup run/restore). [`MockGeneral`] is the
//! deterministic fixture for CI, pane tests, and captures.

mod adapter;
mod host;
mod model;
mod source;

pub use adapter::GeneralAdapter;
pub use host::{
    parse_df_usage, parse_localectl, parse_startup, parse_timedatectl, parse_unit_description,
    read_autostart, set_desktop_key, HostGeneral,
};
pub use model::{
    BackupBackend, BackupData, GeneralSnapshot, LocaleData, LoginData, LoginItem, SharingData,
    SharingService, StartupDiskData, StorageCategory, StorageUsageData, TimeData, UserUnit,
    VolumeUsage,
};
pub use source::{fixture, GeneralOutcome, GeneralSource, MockGeneral};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_general_slot() {
        let adapter = GeneralAdapter::new(MockGeneral::absent());
        assert_eq!(
            <GeneralAdapter<MockGeneral> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::GENERAL
        );
    }

    #[test]
    fn the_fixture_is_deterministic() {
        assert_eq!(fixture(), fixture());
    }
}
