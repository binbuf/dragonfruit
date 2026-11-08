// SPDX-License-Identifier: MIT
//! The General sub-panes' half of the bridge host (T226).
//!
//! The host owns one [`GeneralAdapter`] over `dragonfruit-general-adapter` and
//! projects its typed snapshot to the flat JSON view the Settings app draws.
//! Date & Time, Language & Region, Login Items, Sharing, Backup, Storage usage,
//! and Startup Disk each carry their own `state`, so one absent owner never
//! hides the rest.
//!
//! A write never invents a snapshot: the Settings client calls `Refresh` after
//! the write and the read state stays the single source of truth.

use dragonfruit_general_adapter::{GeneralAdapter, GeneralOutcome, GeneralSnapshot, GeneralSource};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

const KIND_GENERAL: &str = "general";

/// The bridge host for the General group.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralHost<S> {
    adapter: GeneralAdapter<S>,
}

impl<S: GeneralSource> GeneralHost<S> {
    /// A host over a General source.
    pub fn new(source: S) -> Self {
        GeneralHost {
            adapter: GeneralAdapter::new(source),
        }
    }

    /// Re-read every General sub-pane once.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The General view the Settings app renders.
    pub fn view(&self) -> Value {
        general_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Set the system time zone. One explicit write.
    pub fn set_timezone(&mut self, timezone: &str) -> Value {
        general_report(self.adapter.set_timezone(timezone))
    }

    /// Enable or disable NTP. One explicit write.
    pub fn set_ntp(&mut self, enabled: bool) -> Value {
        general_report(self.adapter.set_ntp(enabled))
    }

    /// Set the wall clock. One explicit write.
    pub fn set_time(&mut self, value: &str) -> Value {
        general_report(self.adapter.set_time(value))
    }

    /// Set the system locale. One explicit write.
    pub fn set_locale(&mut self, locale: &str) -> Value {
        general_report(self.adapter.set_locale(locale))
    }

    /// Enable or disable one autostart entry. One explicit write.
    pub fn set_autostart_enabled(&mut self, id: &str, enabled: bool) -> Value {
        general_report(self.adapter.set_autostart_enabled(id, enabled))
    }

    /// Enable or disable one systemd user unit. One explicit write.
    pub fn set_user_unit_enabled(&mut self, unit: &str, enabled: bool) -> Value {
        general_report(self.adapter.set_user_unit_enabled(unit, enabled))
    }

    /// Enable or disable one sharing service. One explicit write.
    pub fn set_sharing_enabled(&mut self, id: &str, enabled: bool) -> Value {
        general_report(self.adapter.set_sharing_enabled(id, enabled))
    }

    /// Run one backup through the selected backend. One explicit write.
    pub fn run_backup(&mut self, destination: &str) -> Value {
        general_report(self.adapter.run_backup(destination))
    }

    /// Open the selected backend's restore entry. One explicit write.
    pub fn open_restore(&mut self) -> Value {
        general_report(self.adapter.open_restore())
    }

    /// The adapter, read-only, for tests.
    pub fn adapter(&self) -> &GeneralAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably, for tests.
    pub fn adapter_mut(&mut self) -> &mut GeneralAdapter<S> {
        &mut self.adapter
    }
}

/// Build the General view from an adapter.
pub fn general_view<S: GeneralSource>(adapter: &GeneralAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_GENERAL, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_GENERAL,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_GENERAL, "state": "unavailable" });
    };
    general_snapshot_view(snapshot)
}

/// The pure projection from a decoded snapshot.
pub fn general_snapshot_view(snapshot: &GeneralSnapshot) -> Value {
    let errors: Value = snapshot
        .errors
        .iter()
        .map(|(key, message)| (key.clone(), json!(message)))
        .collect::<serde_json::Map<String, Value>>()
        .into();
    json!({
        "kind": KIND_GENERAL,
        "state": "available",
        "time": time_view(snapshot),
        "locale": locale_view(snapshot),
        "login": login_view(snapshot),
        "sharing": sharing_view(snapshot),
        "backup": backup_view(snapshot),
        "storage": storage_view(snapshot),
        "startup": startup_view(snapshot),
        "errors": errors,
    })
}

fn sub_state(available: bool, error: Option<&str>) -> &'static str {
    if available {
        "available"
    } else if error.is_some() {
        "error"
    } else {
        "unavailable"
    }
}

fn time_view(snapshot: &GeneralSnapshot) -> Value {
    let Some(time) = &snapshot.time else {
        return json!({
            "state": sub_state(false, snapshot.error("time")),
            "error": snapshot.error("time"),
        });
    };
    let label = if time.ntp_enabled {
        format!("{} · Automatic", time.timezone)
    } else {
        format!("{} · Manual", time.timezone)
    };
    json!({
        "state": "available",
        "timezone": time.timezone,
        "ntpEnabled": time.ntp_enabled,
        "ntpSynchronized": time.ntp_synchronized,
        "canNtp": time.can_ntp,
        "localTimeMs": time.local_time_ms,
        "timezones": time.timezones,
        "label": label,
    })
}

fn locale_view(snapshot: &GeneralSnapshot) -> Value {
    let Some(locale) = &snapshot.locale else {
        return json!({
            "state": sub_state(false, snapshot.error("locale")),
            "error": snapshot.error("locale"),
        });
    };
    json!({
        "state": "available",
        "locale": locale.locale,
        "language": locale.language,
        "formatsLocale": locale.formats_locale,
        "keymap": locale.keymap,
        "availableLocales": locale.available_locales,
        "label": locale.locale,
    })
}

fn login_view(snapshot: &GeneralSnapshot) -> Value {
    let Some(login) = &snapshot.login else {
        return json!({
            "state": sub_state(false, snapshot.error("login")),
            "error": snapshot.error("login"),
        });
    };
    let autostart: Vec<Value> = login
        .autostart
        .iter()
        .map(|item| {
            json!({
                "id": item.id,
                "name": item.name,
                "command": item.command,
                "source": item.source,
                "path": item.path,
                "enabled": item.enabled,
                "system": item.system,
                "canToggle": item.can_toggle,
            })
        })
        .collect();
    let user_units: Vec<Value> = login
        .user_units
        .iter()
        .map(|unit| {
            json!({
                "id": unit.id,
                "name": unit.name,
                "enabled": unit.enabled,
                "active": unit.active,
                "canToggle": unit.can_toggle,
            })
        })
        .collect();
    let count = autostart.len() + user_units.len();
    json!({
        "state": "available",
        "autostart": autostart,
        "userUnits": user_units,
        "extensionsAvailable": login.extensions_available,
        "extensionsNote": login.extensions_note,
        "count": count,
        "label": if count == 0 { "No login items".to_owned() } else { format!("{count} items") },
    })
}

fn sharing_view(snapshot: &GeneralSnapshot) -> Value {
    let Some(sharing) = &snapshot.sharing else {
        return json!({
            "state": sub_state(false, snapshot.error("sharing")),
            "error": snapshot.error("sharing"),
        });
    };
    let services: Vec<Value> = sharing
        .services
        .iter()
        .filter(|service| service.installed)
        .map(|service| {
            json!({
                "id": service.id,
                "name": service.name,
                "description": service.description,
                "unit": service.unit,
                "installed": service.installed,
                "enabled": service.enabled,
                "active": service.active,
                "canToggle": service.can_toggle,
                "needsAuth": service.needs_auth,
            })
        })
        .collect();
    let active = services
        .iter()
        .filter(|service| service["active"] == true)
        .count();
    json!({
        "state": "available",
        "services": services,
        "installedCount": services.len(),
        "activeCount": active,
        "label": if services.is_empty() {
            "No sharing services".to_owned()
        } else {
            format!("{active} of {} on", services.len())
        },
    })
}

fn backup_view(snapshot: &GeneralSnapshot) -> Value {
    let Some(backup) = &snapshot.backup else {
        return json!({
            "state": sub_state(false, snapshot.error("backup")),
            "error": snapshot.error("backup"),
        });
    };
    let candidates: Vec<Value> = backup
        .candidates
        .iter()
        .map(|candidate| {
            json!({
                "id": candidate.id,
                "label": candidate.label,
                "installed": candidate.installed,
                "canRun": candidate.can_run,
                "canRestore": candidate.can_restore,
            })
        })
        .collect();
    json!({
        "state": "available",
        "backend": backup.backend,
        "backendLabel": backup.backend_label,
        "candidates": candidates,
        "detectedDestination": backup.detected_destination,
        "detectedSchedule": backup.detected_schedule,
        "lastRun": backup.last_run,
        "running": backup.running,
        "canRun": backup.can_run,
        "canRestore": backup.can_restore,
        "message": backup.message,
        "label": if backup.backend.is_empty() {
            "No backup backend".to_owned()
        } else {
            format!("{} · {}", backup.backend_label, backup.detected_schedule)
        },
    })
}

fn storage_view(snapshot: &GeneralSnapshot) -> Value {
    let Some(storage) = &snapshot.storage else {
        return json!({
            "state": sub_state(false, snapshot.error("storage")),
            "error": snapshot.error("storage"),
        });
    };
    let categories: Vec<Value> = storage
        .categories
        .iter()
        .map(|category| {
            json!({
                "id": category.id,
                "label": category.label,
                "bytes": category.bytes,
                "glyph": category.glyph,
            })
        })
        .collect();
    let volumes: Vec<Value> = storage
        .volumes
        .iter()
        .map(|volume| {
            json!({
                "device": volume.device,
                "mountPoint": volume.mount_point,
                "category": volume.category,
                "totalBytes": volume.total_bytes,
                "usedBytes": volume.used_bytes,
                "availableBytes": volume.available_bytes,
                "removable": volume.removable,
            })
        })
        .collect();
    json!({
        "state": "available",
        "categories": categories,
        "volumes": volumes,
        "totalBytes": storage.total_bytes,
        "usedBytes": storage.used_bytes,
        "freeBytes": storage.free_bytes,
        "label": format!("{} used", bytes_label(storage.used_bytes)),
    })
}

fn startup_view(snapshot: &GeneralSnapshot) -> Value {
    let Some(startup) = &snapshot.startup else {
        return json!({
            "state": sub_state(false, snapshot.error("startup")),
            "error": snapshot.error("startup"),
        });
    };
    json!({
        "state": "available",
        "device": startup.device,
        "disk": startup.disk,
        "filesystem": startup.filesystem,
        "mountPoint": startup.mount_point,
        "totalBytes": startup.total_bytes,
        "bootMode": startup.boot_mode,
        "secureBoot": startup.secure_boot,
        "label": startup.label,
    })
}

/// A short binary size label for the General row descriptions.
pub fn bytes_label(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    if bytes >= GIB {
        let gib = (bytes + GIB / 2) / GIB;
        format!("{gib} GB")
    } else {
        let mib = (bytes + 1024 * 1024 / 2) / (1024 * 1024);
        format!("{mib} MB")
    }
}

/// The JSON report for a General write (`applied`/`absent`/`denied`/`failed`).
pub fn general_report(outcome: GeneralOutcome) -> Value {
    match outcome {
        GeneralOutcome::Applied => json!({ "outcome": "applied" }),
        GeneralOutcome::Absent => json!({ "outcome": "absent" }),
        GeneralOutcome::Denied(message) => json!({ "outcome": "denied", "message": message }),
        GeneralOutcome::Failed(error) => {
            json!({ "outcome": "failed", "message": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_general_adapter::{fixture, MockGeneral};

    #[test]
    fn an_absent_host_is_unavailable() {
        let mut host = GeneralHost::new(MockGeneral::absent());
        host.refresh();
        assert_eq!(host.view()["state"], "unavailable");
    }

    #[test]
    fn the_fixture_projects_every_sub_pane() {
        let mut host = GeneralHost::new(MockGeneral::present());
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["time"]["state"], "available");
        assert_eq!(view["time"]["timezone"], "America/Los_Angeles");
        assert_eq!(view["locale"]["locale"], "en_US.UTF-8");
        assert_eq!(
            view["login"]["autostart"][0]["id"],
            "org.dragonfruit.Backup"
        );
        assert_eq!(view["sharing"]["installedCount"], 3);
        assert_eq!(view["backup"]["backend"], "timeshift");
        assert_eq!(view["storage"]["categories"].as_array().unwrap().len(), 3);
        assert_eq!(view["startup"]["bootMode"], "UEFI");
    }

    #[test]
    fn a_missing_sub_pane_is_a_local_absence_not_a_whole_view_error() {
        let mut data = fixture();
        data.time = None;
        data.errors
            .insert("time".to_owned(), "timedatectl: not found".to_owned());
        let mut host = GeneralHost::new(MockGeneral::with_data(data));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["time"]["state"], "error");
        assert_eq!(view["time"]["error"], "timedatectl: not found");
        assert_eq!(view["locale"]["state"], "available");
    }

    #[test]
    fn writes_report_the_outcome() {
        let mut host = GeneralHost::new(MockGeneral::present());
        host.refresh();
        assert_eq!(host.set_ntp(false)["outcome"], "applied");
        assert_eq!(host.set_timezone("UTC")["outcome"], "applied");
        assert_eq!(host.set_locale("de_DE.UTF-8")["outcome"], "applied");
        assert_eq!(
            host.set_sharing_enabled("file", false)["outcome"],
            "applied"
        );
        assert_eq!(host.run_backup("/mnt/backup")["outcome"], "applied");
        assert_eq!(host.open_restore()["outcome"], "applied");
        assert_eq!(host.adapter().source().writes(), 6);
    }

    #[test]
    fn a_denied_write_keeps_the_reason() {
        let mut host = GeneralHost::new(MockGeneral::present().fail_writes("not authorized"));
        host.refresh();
        let report = host.set_sharing_enabled("file", true);
        assert_eq!(report["outcome"], "failed");
        assert_eq!(report["message"], "not authorized");
    }

    #[test]
    fn byte_labels_are_human_readable() {
        assert_eq!(bytes_label(480 * 1024 * 1024 * 1024), "480 GB");
        assert_eq!(bytes_label(20 * 1024 * 1024), "20 MB");
    }
}
