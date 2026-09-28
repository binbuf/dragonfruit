// SPDX-License-Identifier: MIT
//! The Privacy and Security snapshot the pane renders, decoded from one raw
//! portal read.
//!
//! The model owns only the projection a consumer should not repeat: it types
//! each portal permission table as a category, each resource id as a row, and
//! each application permission as a tristate, orders the lists, derives the
//! labels and glyph the pane header draws, and computes the change stream
//! between two reads. It never grants, revokes, or stores a permission —
//! `xdg-desktop-portal`'s PermissionStore owns the operation and the value;
//! this is the data it publishes.
//!
//! The category list is the curated set of portal tables
//! ([`KNOWN_TABLES`]) in the order the Settings pane draws them. A table the
//! store has no entry for is an empty category (secondary text `None`), never
//! missing: a pane shows every privacy row and its honest count.
//!
//! The event half is a pure diff: [`PrivacySnapshot::changes`] reports what
//! moved between two reads (a resource added/removed, an application
//! added/removed/changed) without any polling.

use crate::source::{AppPermissionData, PrivacyData, ResourceData, TableData};

/// One portal permission table the adapter understands, with the label the
/// pane draws.
///
/// The table names are the ones `xdg-desktop-portal` uses
/// (`shared/xdp-types.h`): `devices` holds the camera resource, `location`,
/// `notifications`, `background`, `screenshot`, `screencast`,
/// `remote-desktop`, `usb`, `input-capture`, `gamemode`, `inhibit`,
/// `realtime`, `wallpaper`, and `desktop-used-apps` are the rest. They are a
/// projection of the host stack, never a second registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownTable {
    /// The PermissionStore table name.
    pub table: &'static str,
    /// The label the pane draws for the category.
    pub label: &'static str,
}

/// The portal permission tables the adapter projects, in the pane's order.
pub const KNOWN_TABLES: [KnownTable; 14] = [
    KnownTable {
        table: "devices",
        label: "Camera",
    },
    KnownTable {
        table: "location",
        label: "Location Services",
    },
    KnownTable {
        table: "notifications",
        label: "Notifications",
    },
    KnownTable {
        table: "screencast",
        label: "Screen Recording",
    },
    KnownTable {
        table: "remote-desktop",
        label: "Remote Desktop",
    },
    KnownTable {
        table: "screenshot",
        label: "Screenshot",
    },
    KnownTable {
        table: "background",
        label: "Background",
    },
    KnownTable {
        table: "usb",
        label: "USB Devices",
    },
    KnownTable {
        table: "input-capture",
        label: "Input Capture",
    },
    KnownTable {
        table: "gamemode",
        label: "Game Mode",
    },
    KnownTable {
        table: "inhibit",
        label: "Inhibit",
    },
    KnownTable {
        table: "realtime",
        label: "Realtime",
    },
    KnownTable {
        table: "wallpaper",
        label: "Wallpaper",
    },
    KnownTable {
        table: "desktop-used-apps",
        label: "Default Apps",
    },
];

/// The label for a table, falling back to the table name when the adapter does
/// not know it.
pub fn table_label(table: &str) -> &str {
    KNOWN_TABLES
        .iter()
        .find(|known| known.table == table)
        .map(|known| known.label)
        .unwrap_or(table)
}

/// The pane's order for a table. Known tables keep [`KNOWN_TABLES`] order;
/// an unknown table sorts after every known one, by name.
pub fn table_rank(table: &str) -> usize {
    KNOWN_TABLES
        .iter()
        .position(|known| known.table == table)
        .unwrap_or(KNOWN_TABLES.len())
}

/// The portal permission tristate, derived from the stored permission strings.
///
/// The portal stores exactly one of `yes`, `no`, or `ask` for a
/// yes/no/ask permission; anything else (an empty list, a two-string accuracy
/// record, a future value) is [`PermissionState::Unset`], never a guess. The
/// raw strings stay on [`AppPermission::permissions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PermissionState {
    /// The application is allowed (`yes`).
    Allowed,
    /// The application is denied (`no`).
    Denied,
    /// The portal prompts (`ask`).
    Ask,
    /// No recognizable tristate is stored.
    #[default]
    Unset,
}

impl PermissionState {
    /// Every permission state.
    pub const ALL: [PermissionState; 4] = [
        PermissionState::Allowed,
        PermissionState::Denied,
        PermissionState::Ask,
        PermissionState::Unset,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            PermissionState::Allowed => "allowed",
            PermissionState::Denied => "denied",
            PermissionState::Ask => "ask",
            PermissionState::Unset => "unset",
        }
    }

    /// The short label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            PermissionState::Allowed => "Allowed",
            PermissionState::Denied => "Denied",
            PermissionState::Ask => "Ask",
            PermissionState::Unset => "Not Set",
        }
    }

    /// The permission strings a write stores for this state (`[]` for
    /// [`PermissionState::Unset`]).
    pub const fn permissions(self) -> &'static [&'static str] {
        match self {
            PermissionState::Allowed => &["yes"],
            PermissionState::Denied => &["no"],
            PermissionState::Ask => &["ask"],
            PermissionState::Unset => &[],
        }
    }

    /// The state for a stored permission list; an unrecognized list is
    /// `Unset`.
    pub fn from_permissions(permissions: &[String]) -> PermissionState {
        if permissions.len() == 1 {
            match permissions[0].as_str() {
                "yes" => PermissionState::Allowed,
                "no" => PermissionState::Denied,
                "ask" => PermissionState::Ask,
                _ => PermissionState::Unset,
            }
        } else {
            PermissionState::Unset
        }
    }

    /// Parse the stable id back to a state; an unknown id is `Unset`.
    pub fn from_id(id: &str) -> PermissionState {
        PermissionState::ALL
            .into_iter()
            .find(|state| state.id() == id)
            .unwrap_or(PermissionState::Unset)
    }
}

/// One application permission the pane draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppPermission {
    /// The application id.
    pub app: String,
    /// The derived tristate.
    pub state: PermissionState,
    /// The stored permission strings, verbatim.
    pub permissions: Vec<String>,
}

impl AppPermission {
    fn from_data(data: &AppPermissionData) -> Self {
        AppPermission {
            app: data.app.clone(),
            state: PermissionState::from_permissions(&data.permissions),
            permissions: data.permissions.clone(),
        }
    }

    /// Whether the application is allowed.
    pub fn is_allowed(&self) -> bool {
        self.state == PermissionState::Allowed
    }

    /// Whether the application is explicitly denied.
    pub fn is_denied(&self) -> bool {
        self.state == PermissionState::Denied
    }
}

/// One resource entry in a category (a PermissionStore resource id and its
/// applications).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionResource {
    /// The resource id (`camera`, a MIME type, …).
    pub id: String,
    /// Every application with a stored permission, by app id.
    pub apps: Vec<AppPermission>,
}

impl PermissionResource {
    fn from_data(data: &ResourceData) -> Self {
        let mut apps: Vec<AppPermission> = data.apps.iter().map(AppPermission::from_data).collect();
        apps.sort_by(|left, right| left.app.cmp(&right.app));
        PermissionResource {
            id: data.id.clone(),
            apps,
        }
    }

    /// Every application in this resource.
    pub fn apps(&self) -> &[AppPermission] {
        &self.apps
    }

    /// The application with `app`, when the resource has one.
    pub fn app(&self, app: &str) -> Option<&AppPermission> {
        self.apps.iter().find(|entry| entry.app == app)
    }

    /// How many applications the resource lists.
    pub fn app_count(&self) -> usize {
        self.apps.len()
    }
}

/// One privacy category the pane draws: a portal permission table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionCategory {
    /// The PermissionStore table name (the category id).
    pub table: String,
    /// The label the pane draws.
    pub label: String,
    /// Every resource entry in the table, by id.
    pub resources: Vec<PermissionResource>,
}

impl PermissionCategory {
    fn from_data(data: &TableData) -> Self {
        let mut resources: Vec<PermissionResource> = data
            .resources
            .iter()
            .map(PermissionResource::from_data)
            .collect();
        resources.sort_by(|left, right| left.id.cmp(&right.id));
        PermissionCategory {
            table: data.table.clone(),
            label: table_label(&data.table).to_owned(),
            resources,
        }
    }

    /// The category id (the table name).
    pub fn id(&self) -> &str {
        &self.table
    }

    /// The label the pane draws.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Every resource entry in the category.
    pub fn resources(&self) -> &[PermissionResource] {
        &self.resources
    }

    /// The resource with `id`, when the category has one.
    pub fn resource(&self, id: &str) -> Option<&PermissionResource> {
        self.resources.iter().find(|resource| resource.id == id)
    }

    /// Every application permission across the category's resources.
    pub fn apps(&self) -> impl Iterator<Item = &AppPermission> {
        self.resources
            .iter()
            .flat_map(|resource| resource.apps.iter())
    }

    /// How many application permissions the category holds.
    pub fn app_count(&self) -> usize {
        self.apps().count()
    }

    /// How many applications the category grants.
    pub fn granted_count(&self) -> usize {
        self.apps().filter(|app| app.is_allowed()).count()
    }

    /// The secondary text the pane draws: `None`, `1 app`, or `N apps`.
    pub fn summary(&self) -> String {
        match self.app_count() {
            0 => "None".to_owned(),
            1 => "1 app".to_owned(),
            count => format!("{count} apps"),
        }
    }
}

/// The Privacy and Security snapshot a pane renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacySnapshot {
    categories: Vec<PermissionCategory>,
}

impl Default for PrivacySnapshot {
    fn default() -> Self {
        PrivacySnapshot::from_data(&PrivacyData::default())
    }
}

impl PrivacySnapshot {
    /// Build the snapshot from one raw read. Every known table is always a
    /// category (empty when the store has no entry), so a pane shows every
    /// privacy row and its honest count; a table the adapter does not know is
    /// kept too, at the end.
    pub fn from_data(data: &PrivacyData) -> Self {
        let mut categories: Vec<PermissionCategory> = KNOWN_TABLES
            .iter()
            .map(|known| PermissionCategory {
                table: known.table.to_owned(),
                label: known.label.to_owned(),
                resources: vec![],
            })
            .collect();
        for table in &data.tables {
            let category = PermissionCategory::from_data(table);
            match categories
                .iter_mut()
                .find(|existing| existing.table == category.table)
            {
                Some(existing) => existing.resources = category.resources,
                None => categories.push(category),
            }
        }
        categories.sort_by(|left, right| {
            table_rank(&left.table)
                .cmp(&table_rank(&right.table))
                .then_with(|| left.table.cmp(&right.table))
        });
        PrivacySnapshot { categories }
    }

    /// Every category, in the pane's order.
    pub fn categories(&self) -> &[PermissionCategory] {
        &self.categories
    }

    /// The category for `table`, when the snapshot has one.
    pub fn category(&self, table: &str) -> Option<&PermissionCategory> {
        self.categories
            .iter()
            .find(|category| category.table == table)
    }

    /// Every application permission across every category.
    pub fn apps(&self) -> impl Iterator<Item = &AppPermission> {
        self.categories.iter().flat_map(|category| category.apps())
    }

    /// How many application permissions the host stack holds.
    pub fn app_count(&self) -> usize {
        self.apps().count()
    }

    /// How many applications the host stack grants.
    pub fn granted_count(&self) -> usize {
        self.apps().filter(|app| app.is_allowed()).count()
    }

    /// How many applications the host stack denies.
    pub fn denied_count(&self) -> usize {
        self.apps().filter(|app| app.is_denied()).count()
    }

    /// Whether the host stack records any permission at all. This is the
    /// status slot's hide rule: an empty store is available but not present.
    pub fn present(&self) -> bool {
        self.app_count() > 0
    }

    /// The Control Center label: `No App Permissions`, `1 App`, `N Apps`.
    pub fn label(&self) -> String {
        match self.app_count() {
            0 => "No App Permissions".to_owned(),
            1 => "1 App".to_owned(),
            count => format!("{count} Apps"),
        }
    }

    /// The design-system glyph the pane header draws.
    pub fn glyph(&self) -> &'static str {
        "privacy"
    }

    /// The categories with at least one application permission.
    pub fn active_categories(&self) -> Vec<&PermissionCategory> {
        self.categories
            .iter()
            .filter(|category| category.app_count() > 0)
            .collect()
    }

    /// The domain changes between `previous` and this snapshot, in order.
    pub fn changes(&self, previous: &PrivacySnapshot) -> Vec<PrivacyChange> {
        let mut changes = Vec::new();
        for category in &self.categories {
            let previous_category = previous.category(&category.table);
            for resource in &category.resources {
                let previous_resource =
                    previous_category.and_then(|category| category.resource(&resource.id));
                match previous_resource {
                    None => changes.push(PrivacyChange::ResourceAdded {
                        table: category.table.clone(),
                        id: resource.id.clone(),
                    }),
                    Some(previous_resource) => {
                        for app in &resource.apps {
                            match previous_resource.app(&app.app) {
                                None => changes.push(PrivacyChange::AppAdded {
                                    table: category.table.clone(),
                                    id: resource.id.clone(),
                                    app: app.app.clone(),
                                }),
                                Some(previous_app) if previous_app.state != app.state => {
                                    changes.push(PrivacyChange::AppChanged {
                                        table: category.table.clone(),
                                        id: resource.id.clone(),
                                        app: app.app.clone(),
                                    });
                                }
                                Some(_) => {}
                            }
                        }
                        for app in &previous_resource.apps {
                            if resource.app(&app.app).is_none() {
                                changes.push(PrivacyChange::AppRemoved {
                                    table: category.table.clone(),
                                    id: resource.id.clone(),
                                    app: app.app.clone(),
                                });
                            }
                        }
                    }
                }
            }
            if let Some(previous_category) = previous_category {
                for resource in &previous_category.resources {
                    if category.resource(&resource.id).is_none() {
                        changes.push(PrivacyChange::ResourceRemoved {
                            table: category.table.clone(),
                            id: resource.id.clone(),
                        });
                    }
                }
            }
        }
        changes
    }
}

/// One change between two portal reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyChange {
    /// A resource entry appeared in a table.
    ResourceAdded {
        /// The table name.
        table: String,
        /// The resource id.
        id: String,
    },
    /// A resource entry disappeared from a table.
    ResourceRemoved {
        /// The table name.
        table: String,
        /// The resource id.
        id: String,
    },
    /// An application permission appeared.
    AppAdded {
        /// The table name.
        table: String,
        /// The resource id.
        id: String,
        /// The application id.
        app: String,
    },
    /// An application permission disappeared.
    AppRemoved {
        /// The table name.
        table: String,
        /// The resource id.
        id: String,
        /// The application id.
        app: String,
    },
    /// An application's permission moved.
    AppChanged {
        /// The table name.
        table: String,
        /// The resource id.
        id: String,
        /// The application id.
        app: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> PrivacyData {
        PrivacyData {
            tables: vec![
                TableData {
                    table: "location".to_owned(),
                    resources: vec![ResourceData {
                        id: "location".to_owned(),
                        apps: vec![AppPermissionData {
                            app: "org.example.Maps".to_owned(),
                            permissions: vec![
                                "exact".to_owned(),
                                "2026-09-01T00:00:00Z".to_owned(),
                            ],
                        }],
                    }],
                },
                TableData {
                    table: "devices".to_owned(),
                    resources: vec![ResourceData {
                        id: "camera".to_owned(),
                        apps: vec![
                            AppPermissionData {
                                app: "org.example.Snapshot".to_owned(),
                                permissions: vec!["no".to_owned()],
                            },
                            AppPermissionData {
                                app: "org.mozilla.firefox".to_owned(),
                                permissions: vec!["yes".to_owned()],
                            },
                        ],
                    }],
                },
                TableData {
                    table: "notifications".to_owned(),
                    resources: vec![ResourceData {
                        id: "notification".to_owned(),
                        apps: vec![AppPermissionData {
                            app: "org.example.Calendar".to_owned(),
                            permissions: vec!["ask".to_owned()],
                        }],
                    }],
                },
            ],
        }
    }

    #[test]
    fn the_snapshot_orders_and_types_the_read() {
        let snapshot = PrivacySnapshot::from_data(&data());
        assert!(snapshot.present());
        assert_eq!(snapshot.app_count(), 4);
        assert_eq!(snapshot.granted_count(), 1);
        assert_eq!(snapshot.denied_count(), 1);
        assert_eq!(snapshot.glyph(), "privacy");
        assert_eq!(snapshot.label(), "4 Apps");

        // The pane order is the curated table order: Camera before Location.
        assert_eq!(snapshot.categories()[0].id(), "devices");
        assert_eq!(snapshot.categories()[0].label(), "Camera");
        assert_eq!(snapshot.categories()[1].id(), "location");
        assert_eq!(snapshot.categories()[1].label(), "Location Services");

        let camera = snapshot.category("devices").unwrap();
        assert_eq!(camera.summary(), "2 apps");
        assert_eq!(camera.granted_count(), 1);
        let resource = camera.resource("camera").unwrap();
        // Applications are sorted by id.
        assert_eq!(resource.apps()[0].app, "org.example.Snapshot");
        assert!(resource.apps()[0].is_denied());
        assert!(resource.app("org.mozilla.firefox").unwrap().is_allowed());

        // A non-tristate record keeps its raw permissions and reads Unset.
        let location = snapshot.category("location").unwrap();
        let maps = location.resource("location").unwrap().apps[0].clone();
        assert_eq!(maps.state, PermissionState::Unset);
        assert_eq!(maps.permissions.len(), 2);
    }

    #[test]
    fn every_known_table_is_always_a_category() {
        // Only three tables have entries; every other known table is an honest
        // empty category a pane still draws.
        let snapshot = PrivacySnapshot::from_data(&data());
        assert_eq!(snapshot.categories().len(), KNOWN_TABLES.len());
        let usb = snapshot.category("usb").expect("a known, empty category");
        assert_eq!(usb.app_count(), 0);
        assert_eq!(usb.summary(), "None");
    }

    #[test]
    fn an_empty_store_is_not_present_but_is_a_snapshot() {
        let snapshot = PrivacySnapshot::from_data(&PrivacyData { tables: vec![] });
        assert!(!snapshot.present());
        assert_eq!(snapshot.label(), "No App Permissions");
        assert_eq!(snapshot.app_count(), 0);
        assert_eq!(snapshot.categories().len(), KNOWN_TABLES.len());
        assert!(snapshot
            .categories()
            .iter()
            .all(|category| category.app_count() == 0));
        assert_eq!(snapshot.active_categories().len(), 0);
    }

    #[test]
    fn a_single_app_label_is_singular() {
        let snapshot = PrivacySnapshot::from_data(&PrivacyData {
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
        });
        assert_eq!(snapshot.label(), "1 App");
        assert_eq!(snapshot.category("devices").unwrap().summary(), "1 app");
    }

    #[test]
    fn an_unknown_table_keeps_its_name_and_sorts_last() {
        let snapshot = PrivacySnapshot::from_data(&PrivacyData {
            tables: vec![
                TableData {
                    table: "zzz-future".to_owned(),
                    resources: vec![ResourceData {
                        id: "x".to_owned(),
                        apps: vec![AppPermissionData {
                            app: "app".to_owned(),
                            permissions: vec!["yes".to_owned()],
                        }],
                    }],
                },
                TableData {
                    table: "devices".to_owned(),
                    resources: vec![],
                },
            ],
        });
        assert_eq!(snapshot.categories()[0].id(), "devices");
        assert_eq!(snapshot.categories()[KNOWN_TABLES.len()].id(), "zzz-future");
        assert_eq!(
            snapshot.categories()[KNOWN_TABLES.len()].label(),
            "zzz-future"
        );
    }

    #[test]
    fn each_app_move_is_a_change() {
        let previous = PrivacySnapshot::from_data(&data());

        let mut raw = data();
        // Grant the denied app (changed), remove the ask app (removed), add one.
        let devices = raw
            .tables
            .iter_mut()
            .find(|table| table.table == "devices")
            .unwrap();
        devices.resources[0].apps[0].permissions = vec!["yes".to_owned()];
        devices.resources[0].apps.push(AppPermissionData {
            app: "org.gimp.GIMP".to_owned(),
            permissions: vec!["ask".to_owned()],
        });
        let notifications = raw
            .tables
            .iter_mut()
            .find(|table| table.table == "notifications")
            .unwrap();
        notifications.resources[0].apps.clear();
        let next = PrivacySnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&PrivacyChange::AppChanged {
            table: "devices".to_owned(),
            id: "camera".to_owned(),
            app: "org.example.Snapshot".to_owned(),
        }));
        assert!(changes.contains(&PrivacyChange::AppAdded {
            table: "devices".to_owned(),
            id: "camera".to_owned(),
            app: "org.gimp.GIMP".to_owned(),
        }));
        assert!(changes.contains(&PrivacyChange::AppRemoved {
            table: "notifications".to_owned(),
            id: "notification".to_owned(),
            app: "org.example.Calendar".to_owned(),
        }));
    }

    #[test]
    fn a_resource_appearing_and_disappearing_is_a_change() {
        let previous = PrivacySnapshot::from_data(&data());

        let mut raw = data();
        raw.tables.push(TableData {
            table: "usb".to_owned(),
            resources: vec![ResourceData {
                id: "usb".to_owned(),
                apps: vec![AppPermissionData {
                    app: "org.example.DiskUtility".to_owned(),
                    permissions: vec!["yes".to_owned()],
                }],
            }],
        });
        let next = PrivacySnapshot::from_data(&raw);
        assert!(next
            .changes(&previous)
            .contains(&PrivacyChange::ResourceAdded {
                table: "usb".to_owned(),
                id: "usb".to_owned(),
            }));

        let back = PrivacySnapshot::from_data(&data());
        assert!(back
            .changes(&next)
            .contains(&PrivacyChange::ResourceRemoved {
                table: "usb".to_owned(),
                id: "usb".to_owned(),
            }));
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = PrivacySnapshot::from_data(&data());
        assert!(snapshot.changes(&snapshot).is_empty());
    }

    #[test]
    fn permission_states_round_trip() {
        for state in PermissionState::ALL {
            assert_eq!(PermissionState::from_id(state.id()), state);
        }
        assert_eq!(PermissionState::from_id("bogus"), PermissionState::Unset);
        assert_eq!(
            PermissionState::from_permissions(&["yes".to_owned()]),
            PermissionState::Allowed
        );
        assert_eq!(
            PermissionState::from_permissions(&["no".to_owned()]),
            PermissionState::Denied
        );
        assert_eq!(
            PermissionState::from_permissions(&["ask".to_owned()]),
            PermissionState::Ask
        );
        assert_eq!(
            PermissionState::from_permissions(&["exact".to_owned(), "now".to_owned()]),
            PermissionState::Unset
        );
        assert_eq!(PermissionState::Allowed.permissions(), &["yes"]);
        assert_eq!(PermissionState::Denied.permissions(), &["no"]);
        assert_eq!(PermissionState::Ask.permissions(), &["ask"]);
        assert!(PermissionState::Unset.permissions().is_empty());
    }

    #[test]
    fn the_table_labels_and_ranks_are_stable() {
        assert_eq!(table_label("devices"), "Camera");
        assert_eq!(table_label("remote-desktop"), "Remote Desktop");
        assert_eq!(table_label("what"), "what");
        assert!(table_rank("devices") < table_rank("location"));
        assert!(table_rank("location") < table_rank("notifications"));
        assert_eq!(table_rank("what"), KNOWN_TABLES.len());
    }
}
