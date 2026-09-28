// SPDX-License-Identifier: MIT
//! The General, About, and Updates half of the bridge host (T-15.10b).
//!
//! The host stack is reached by the `dragonfruit-update-adapter`; the shell
//! and the Settings app never link it. This module owns the one projection from
//! the typed [`SystemSnapshot`] to the flat JSON view the two consumers draw,
//! plus the three explicit writes the pane raises (check, install, reboot). It
//! mirrors the storage and notifications halves.
//!
//! Absence is layered, exactly as the adapter's is (ADR 0136). The whole view
//! is `unavailable` only when neither the host identity nor the update provider
//! is reachable. A reachable host that runs **no** update provider is
//! `available` with `updatesAvailable: false`: the About rows stay live and the
//! consumer disables only the update controls. A provider that is present but
//! unreadable is `error`, visible and inert with the message.
//!
//! A write never invents a snapshot: the host calls
//! [`UpdateAdapter::refresh`](dragonfruit_update_adapter::UpdateAdapter::refresh)
//! after a successful action, and the read state stays the single source of
//! truth.

use dragonfruit_system_adapters::Adapter;
use dragonfruit_update_adapter::{
    SystemSnapshot, SystemSource, UpdateAdapter, UpdateOutcome, UpdatePhase, UpdateSeverity,
};
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_UPDATES: &str = "updates";

/// The bridge host for the General/About/Updates adapter: one state path and
/// the explicit check/install/reboot writes the Settings pane offers.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdatesHost<S> {
    adapter: UpdateAdapter<S>,
}

impl<S: SystemSource> UpdatesHost<S> {
    /// A host over a host-stack source.
    pub fn new(source: S) -> Self {
        UpdatesHost {
            adapter: UpdateAdapter::new(source),
        }
    }

    /// Re-read the host stack once. Called on startup and after an explicit
    /// action; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The General/About/Updates view the pane and tile render.
    pub fn view(&self) -> Value {
        updates_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Ask the provider to check for updates. One explicit write.
    pub fn check(&mut self) -> Value {
        update_report(self.adapter.check())
    }

    /// Ask the provider to install the available updates. One explicit write.
    pub fn install(&mut self) -> Value {
        update_report(self.adapter.install())
    }

    /// Ask the provider to restart the host. One explicit write.
    pub fn reboot(&mut self) -> Value {
        update_report(self.adapter.reboot())
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &UpdateAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut UpdateAdapter<S> {
        &mut self.adapter
    }
}

/// Build the General/About/Updates view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the identity
/// and (when present) the update provider's state.
pub fn updates_view<S: SystemSource>(adapter: &UpdateAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_UPDATES, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_UPDATES,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_UPDATES, "state": "unavailable" });
    };
    updates_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn updates_snapshot_view(snapshot: &SystemSnapshot) -> Value {
    let identity = &snapshot.identity;
    let updates: Vec<Value> = snapshot
        .updates()
        .iter()
        .map(|update| {
            json!({
                "id": update.id,
                "name": update.name,
                "summary": update.summary,
                "currentVersion": update.current_version,
                "availableVersion": update.available_version,
                "severity": update.severity.id(),
                "severityLabel": update.severity.label(),
            })
        })
        .collect();
    json!({
        "kind": KIND_UPDATES,
        "state": "available",
        // The host identity (the About/General rows).
        "hostName": identity.host_name,
        "deviceName": identity.device_name(),
        "deviceModel": identity.device_model,
        "osLabel": identity.os_label(),
        "osName": identity.os_name,
        "osVersion": identity.os_version,
        "osId": identity.os_id,
        "kernel": identity.kernel,
        "architecture": identity.architecture,
        "processor": identity.processor,
        "memoryLabel": identity.memory_label(),
        "serial": identity.serial,
        "hasSerial": identity.has_serial(),
        // The distribution update provider, when present.
        "updatesAvailable": snapshot.updates_available(),
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "phase": snapshot.phase().map(UpdatePhase::id).unwrap_or(""),
        "busy": snapshot.is_busy(),
        "rebootRequired": snapshot.is_reboot_required(),
        "updateCount": snapshot.update_count(),
        "securityCount": snapshot.security_count(),
        "lastCheckedMs": snapshot.last_checked_ms(),
        "message": snapshot.message(),
        "updates": updates,
    })
}

/// The JSON report for an update write (`applied`/`absent`/`failed`), shaped
/// like the storage `/` Wi-Fi reports so one shell decode path reads every
/// action.
pub fn update_report(outcome: UpdateOutcome) -> Value {
    match outcome {
        UpdateOutcome::Applied => json!({ "outcome": "applied" }),
        UpdateOutcome::Absent => json!({ "outcome": "absent" }),
        UpdateOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

/// The severity label a consumer draws, exposed for parity with the adapter.
pub fn severity_label(severity: UpdateSeverity) -> &'static str {
    severity.label()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_update_adapter::{
        MockSystem, SystemData, SystemIdentity, UpdateData, UpdateItem,
    };

    fn data() -> SystemData {
        SystemData {
            identity: SystemIdentity {
                host_name: "dragon".to_owned(),
                os_name: "Dragonfruit Linux".to_owned(),
                os_version: "44".to_owned(),
                os_id: "dragonfruit".to_owned(),
                kernel: "6.12.0".to_owned(),
                architecture: "x86_64".to_owned(),
                device_model: "Dragonfruit Book".to_owned(),
                processor: "Example CPU".to_owned(),
                memory_bytes: 16 * 1024 * 1024 * 1024,
                serial: "SERIAL-1".to_owned(),
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
    fn an_absent_host_stack_projects_a_hidden_slot() {
        let mut host = UpdatesHost::new(MockSystem::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "updates");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_host_projects_the_identity_and_updates() {
        let mut host = UpdatesHost::new(MockSystem::present(data()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["hostName"], "dragon");
        assert_eq!(view["deviceName"], "Dragonfruit Book");
        assert_eq!(view["osLabel"], "Dragonfruit Linux 44");
        assert_eq!(view["memoryLabel"], "16 GB");
        assert_eq!(view["hasSerial"], true);
        assert_eq!(view["updatesAvailable"], true);
        assert_eq!(view["phase"], "available");
        assert_eq!(view["glyph"], "software-update");
        assert_eq!(view["label"], "1 Update Available");
        assert_eq!(view["updateCount"], 1);
        assert_eq!(view["securityCount"], 1);
        assert_eq!(view["updates"][0]["severity"], "security");
        assert_eq!(view["updates"][0]["severityLabel"], "Security Update");
    }

    #[test]
    fn a_host_without_a_provider_keeps_the_identity_live() {
        let mut data = data();
        data.updates = None;
        let mut host = UpdatesHost::new(MockSystem::present(data));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["updatesAvailable"], false);
        assert_eq!(view["phase"], "");
        assert_eq!(view["label"], "Software Update Unavailable");
        // The identity half is untouched.
        assert_eq!(view["hostName"], "dragon");
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = UpdatesHost::new(MockSystem::failing("update provider: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "update provider: timeout");
    }

    #[test]
    fn the_writes_apply_once_through_the_adapter() {
        let mut host = UpdatesHost::new(MockSystem::present(data()));
        host.refresh();
        assert_eq!(host.check()["outcome"], "applied");
        assert_eq!(host.install()["outcome"], "applied");
        assert_eq!(host.reboot()["outcome"], "applied");
        assert_eq!(host.adapter().source().checks(), 1);
        assert_eq!(host.adapter().source().installs(), 1);
        assert_eq!(host.adapter().source().reboots(), 1);
    }

    #[test]
    fn writes_while_absent_answer_absence() {
        let mut host = UpdatesHost::new(MockSystem::absent());
        host.refresh();
        assert_eq!(host.check()["outcome"], "absent");
        assert_eq!(host.install()["outcome"], "absent");
        assert_eq!(host.reboot()["outcome"], "absent");
    }

    #[test]
    fn a_failed_write_reports_the_error() {
        let mut host = UpdatesHost::new(MockSystem::present(data()).fail_writes("polkit: denied"));
        host.refresh();
        let report = host.install();
        assert_eq!(report["outcome"], "failed");
        assert_eq!(report["error"], "polkit: denied");
    }
}
