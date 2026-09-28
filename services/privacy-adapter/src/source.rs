// SPDX-License-Identifier: MIT
//! The transport seam: the raw portal PermissionStore read, the explicit
//! permission writes, and their mock.
//!
//! A [`PrivacySource`] is the only thing that talks to the host stack, and the
//! host stack here is the **xdg-desktop-portal PermissionStore**
//! (`org.freedesktop.impl.portal.PermissionStore`, [`crate::HostPrivacy`]).
//! Portals already keep the record of which applications may reach the
//! resources they mediate (camera, location, notifications, screen casting,
//! USB, …); this adapter projects that record and never makes or stores a
//! permission decision itself.
//!
//! The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — the store answered; `data` is the live read.
//! * `Ok(None)` — the **store is absent** (no session bus, or no
//!   `org.freedesktop.impl.portal.PermissionStore` owner). A normal state; the
//!   item hides.
//! * `Err(error)` — the store is present but could not be read; the item shows
//!   visible and inert with the message.
//!
//! The store is free-form: it has tables, each with resource ids, each with a
//! map from application id to a permission string list. This adapter reads the
//! curated set of portal tables the desktop understands
//! ([`crate::KNOWN_TABLES`]); a table the store has no entry for is an empty
//! table, never an error and never an invented permission.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// One application's stored permissions for a resource.
///
/// The permission strings are opaque to the store and kept verbatim. The
/// portal tristate (`yes`/`no`/`ask`) is derived in the model, never imposed
/// here.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct AppPermissionData {
    /// The application id (`org.mozilla.firefox`, a Flatpak app id, …).
    #[serde(default)]
    pub app: String,
    /// The permission strings the store holds for this app and resource.
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// One resource entry in a permission table: a resource id and its per-app
/// permissions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct ResourceData {
    /// The resource id (`camera`, `location`, a MIME type, …).
    #[serde(default)]
    pub id: String,
    /// Every application the store has a permission for.
    #[serde(default)]
    pub apps: Vec<AppPermissionData>,
}

/// One portal permission table and every resource entry in it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct TableData {
    /// The table name (`devices`, `location`, `notifications`, …).
    #[serde(default)]
    pub table: String,
    /// Every resource entry the store holds for this table.
    #[serde(default)]
    pub resources: Vec<ResourceData>,
}

/// The raw result of one PermissionStore read: every portal table the adapter
/// understands and its entries.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct PrivacyData {
    /// The portal permission tables.
    #[serde(default)]
    pub tables: Vec<TableData>,
}

impl PrivacyData {
    /// The entry for `table`, when the read has one.
    pub fn table(&self, table: &str) -> Option<&TableData> {
        self.tables.iter().find(|entry| entry.table == table)
    }
}

/// The result of one permission write.
///
/// These are explicit user actions, never a poll. A write that lands invents
/// no snapshot: the PermissionStore publishes the resulting state and the host
/// re-reads, so the snapshot stays the single source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyOutcome {
    /// The store accepted the request.
    Applied,
    /// Policy refused the request. `note` records the store's message.
    Denied(String),
    /// There is no PermissionStore, or no such table/resource/app.
    Absent,
    /// The request failed for a reason other than authorization.
    Failed(AdapterError),
}

impl PrivacyOutcome {
    /// Whether the store accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, PrivacyOutcome::Applied)
    }

    /// The recorded denial note, when the store refused.
    pub fn denial_note(&self) -> Option<&str> {
        match self {
            PrivacyOutcome::Denied(note) => Some(note),
            _ => None,
        }
    }

    /// The failure, when the request failed for a reason other than a denial.
    pub fn error(&self) -> Option<&AdapterError> {
        match self {
            PrivacyOutcome::Failed(error) => Some(error),
            _ => None,
        }
    }
}

/// Reads the portal PermissionStore over some transport and drives its write
/// methods.
pub trait PrivacySource {
    /// One read of the permission store.
    fn read(&mut self) -> Result<Option<PrivacyData>, AdapterError>;

    /// Store `permissions` for `app` on the resource `id` in `table`. One
    /// explicit write.
    fn set_permission(
        &mut self,
        _table: &str,
        _id: &str,
        _app: &str,
        _permissions: &[String],
    ) -> PrivacyOutcome {
        PrivacyOutcome::Absent
    }

    /// Remove the stored permission for `app` on the resource `id` in `table`.
    /// One explicit write.
    fn delete_permission(&mut self, _table: &str, _id: &str, _app: &str) -> PrivacyOutcome {
        PrivacyOutcome::Absent
    }
}

/// How the simulated store answers write requests.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteBehavior {
    /// Authorized: the request is accepted (the default).
    Accept,
    /// Policy denies the request; carries the recorded note.
    Deny(String),
    /// The request fails for another reason.
    Fail(AdapterError),
}

/// The write counters the mock keeps, one per method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct WriteCounts {
    permission_sets: u32,
    permission_deletes: u32,
}

impl WriteCounts {
    fn total(&self) -> u32 {
        self.permission_sets + self.permission_deletes
    }
}

/// A fixture-backed source with a simulated PermissionStore lifecycle.
///
/// The mock is the CI path: it serves [`PrivacyData`] with no
/// `xdg-desktop-portal` on the machine, and `kill`/`restart` exercise absence
/// and re-subscribe the way masking the store would. `push` drives the tables
/// so a test observes the change stream. Its writes mutate the simulated store
/// the way `xdg-desktop-portal` would, so a subsequent `read` sees the change.
#[derive(Debug, Clone, PartialEq)]
pub struct MockPrivacy {
    present: bool,
    tables: Vec<TableData>,
    failure: Option<AdapterError>,
    behavior: WriteBehavior,
    reads: u32,
    writes: WriteCounts,
}

impl MockPrivacy {
    /// No PermissionStore is reachable.
    pub fn absent() -> Self {
        MockPrivacy {
            present: false,
            tables: vec![],
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// A store that answers with `data`.
    pub fn present(data: PrivacyData) -> Self {
        MockPrivacy {
            present: true,
            tables: data.tables,
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// A store that fails every read (e.g. the daemon went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockPrivacy {
            present: true,
            tables: vec![],
            failure: Some(AdapterError::new(message)),
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// Make every write fail with `message` (not authorization).
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.behavior = WriteBehavior::Fail(AdapterError::new(message));
        self
    }

    /// Make every write come back as a policy denial with `note`.
    pub fn deny_writes(mut self, note: impl Into<String>) -> Self {
        self.behavior = WriteBehavior::Deny(note.into());
        self
    }

    /// The store publishes fresh state.
    pub fn push(&mut self, data: PrivacyData) {
        self.present = true;
        self.failure = None;
        self.tables = data.tables;
    }

    /// The store goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The store comes back (serving the last entries, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated store is reachable.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many permission writes the source has accepted.
    pub fn permission_sets(&self) -> u32 {
        self.writes.permission_sets
    }

    /// How many permission deletions the source has accepted.
    pub fn permission_deletes(&self) -> u32 {
        self.writes.permission_deletes
    }

    /// How many write requests the source has served in total. Lets a test
    /// prove a write is one explicit call, never a loop.
    pub fn writes(&self) -> u32 {
        self.writes.total()
    }

    fn read_outcome(&self) -> Result<(), AdapterError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(())
    }

    fn write_outcome(&mut self) -> PrivacyOutcome {
        if !self.present {
            return PrivacyOutcome::Absent;
        }
        if let Some(error) = &self.failure {
            return PrivacyOutcome::Failed(error.clone());
        }
        match &self.behavior {
            WriteBehavior::Accept => PrivacyOutcome::Applied,
            WriteBehavior::Deny(note) => PrivacyOutcome::Denied(note.clone()),
            WriteBehavior::Fail(error) => PrivacyOutcome::Failed(error.clone()),
        }
    }

    fn table_mut(&mut self, table: &str) -> &mut TableData {
        if let Some(index) = self.tables.iter().position(|entry| entry.table == table) {
            return &mut self.tables[index];
        }
        self.tables.push(TableData {
            table: table.to_owned(),
            resources: vec![],
        });
        self.tables.last_mut().expect("just pushed")
    }
}

impl PrivacySource for MockPrivacy {
    fn read(&mut self) -> Result<Option<PrivacyData>, AdapterError> {
        self.reads += 1;
        self.read_outcome()?;
        if !self.present {
            return Ok(None);
        }
        Ok(Some(PrivacyData {
            tables: self.tables.clone(),
        }))
    }

    fn set_permission(
        &mut self,
        table: &str,
        id: &str,
        app: &str,
        permissions: &[String],
    ) -> PrivacyOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.permission_sets += 1;
            let table = self.table_mut(table);
            let resource = match table.resources.iter_mut().find(|entry| entry.id == id) {
                Some(resource) => resource,
                None => {
                    table.resources.push(ResourceData {
                        id: id.to_owned(),
                        apps: vec![],
                    });
                    table.resources.last_mut().expect("just pushed")
                }
            };
            match resource.apps.iter_mut().find(|entry| entry.app == app) {
                Some(entry) => entry.permissions = permissions.to_vec(),
                None => resource.apps.push(AppPermissionData {
                    app: app.to_owned(),
                    permissions: permissions.to_vec(),
                }),
            }
        }
        outcome
    }

    fn delete_permission(&mut self, table: &str, id: &str, app: &str) -> PrivacyOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.permission_deletes += 1;
            if let Some(table) = self.tables.iter_mut().find(|entry| entry.table == table) {
                if let Some(resource) = table.resources.iter_mut().find(|entry| entry.id == id) {
                    resource.apps.retain(|entry| entry.app != app);
                }
            }
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> PrivacyData {
        PrivacyData {
            tables: vec![TableData {
                table: "devices".to_owned(),
                resources: vec![ResourceData {
                    id: "camera".to_owned(),
                    apps: vec![AppPermissionData {
                        app: "org.mozilla.firefox".to_owned(),
                        permissions: vec!["yes".to_owned()],
                    }],
                }],
            }],
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockPrivacy::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.is_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockPrivacy::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockPrivacy::failing("portal: timeout");
        assert_eq!(mock.read().unwrap_err().message(), "portal: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn the_store_can_be_killed_and_restarted() {
        let mut mock = MockPrivacy::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap().unwrap(), data());
    }

    #[test]
    fn pushes_replace_the_served_state_and_presence() {
        let mut mock = MockPrivacy::present(data());
        mock.push(PrivacyData { tables: vec![] });
        assert_eq!(
            mock.read().unwrap().unwrap(),
            PrivacyData { tables: vec![] }
        );
        assert!(mock.is_present());
    }

    #[test]
    fn a_denying_mock_reports_the_note() {
        let mut mock = MockPrivacy::present(data()).deny_writes("portal: not authorized");
        let outcome = mock.delete_permission("devices", "camera", "org.mozilla.firefox");
        assert_eq!(
            outcome,
            PrivacyOutcome::Denied("portal: not authorized".to_owned())
        );
        assert_eq!(outcome.denial_note(), Some("portal: not authorized"));
    }

    #[test]
    fn a_failing_write_is_a_failure_not_a_denial() {
        let mut mock = MockPrivacy::present(data()).fail_writes("portal: busy");
        assert_eq!(
            mock.set_permission(
                "devices",
                "camera",
                "org.mozilla.firefox",
                &["no".to_owned()]
            ),
            PrivacyOutcome::Failed(AdapterError::new("portal: busy"))
        );
    }

    #[test]
    fn writes_while_absent_are_absent() {
        let mut mock = MockPrivacy::absent();
        assert_eq!(
            mock.set_permission("devices", "camera", "app", &[]),
            PrivacyOutcome::Absent
        );
        assert_eq!(
            mock.delete_permission("devices", "camera", "app"),
            PrivacyOutcome::Absent
        );
        assert_eq!(mock.writes(), 0);
    }

    #[test]
    fn an_applied_write_mutates_the_simulated_store() {
        let mut mock = MockPrivacy::present(data());
        assert!(mock
            .set_permission(
                "devices",
                "camera",
                "org.mozilla.firefox",
                &["no".to_owned()]
            )
            .is_applied());
        assert!(mock
            .set_permission(
                "location",
                "location",
                "org.example.Maps",
                &["exact".to_owned()]
            )
            .is_applied());
        assert!(mock
            .delete_permission("devices", "camera", "org.mozilla.firefox")
            .is_applied());

        let read = mock.read().unwrap().unwrap();
        let devices = read.table("devices").unwrap();
        let camera = devices
            .resources
            .iter()
            .find(|resource| resource.id == "camera")
            .unwrap();
        assert!(camera.apps.is_empty());
        let location = read.table("location").unwrap();
        assert_eq!(location.resources[0].apps[0].app, "org.example.Maps");
        assert_eq!(mock.writes(), 3);
        assert_eq!(mock.permission_sets(), 2);
        assert_eq!(mock.permission_deletes(), 1);
    }
}
