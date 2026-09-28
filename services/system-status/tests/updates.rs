// SPDX-License-Identifier: MIT
//! Bridge tests for the General/About/Updates half of the host (T-15.10b): the
//! view the Settings pane and Control Center tile decode, plus the three
//! explicit writes applied once through the adapter.

use dragonfruit_system_status::UpdatesHost;
use dragonfruit_update_adapter::{
    MockSystem, SystemData, SystemIdentity, UpdateData, UpdateItem, UpdatePhase, UpdateSeverity,
};

fn data(with_updates: bool) -> SystemData {
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
        updates: with_updates.then(|| UpdateData {
            phase: UpdatePhase::Available,
            updates: vec![UpdateItem {
                id: "kernel".to_owned(),
                name: "kernel".to_owned(),
                summary: "The Linux kernel".to_owned(),
                current_version: "6.11".to_owned(),
                available_version: "6.12".to_owned(),
                severity: UpdateSeverity::Security,
            }],
            last_checked_ms: Some(1000),
            message: None,
        }),
    }
}

#[test]
fn the_updates_view_exposes_the_identity_and_provider() {
    let mut host = UpdatesHost::new(MockSystem::present(data(true)));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "updates");
    assert_eq!(view["state"], "available");
    assert_eq!(view["hostName"], "dragon");
    assert_eq!(view["osLabel"], "Dragonfruit Linux 44");
    assert_eq!(view["updatesAvailable"], true);
    assert_eq!(view["phase"], "available");
    assert_eq!(view["glyph"], "software-update");
    assert_eq!(view["label"], "1 Update Available");
    assert_eq!(view["updates"][0]["severityLabel"], "Security Update");
}

#[test]
fn a_host_without_a_provider_keeps_the_about_rows_live() {
    let mut host = UpdatesHost::new(MockSystem::present(data(false)));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["updatesAvailable"], false);
    assert_eq!(view["label"], "Software Update Unavailable");
    assert_eq!(view["deviceName"], "Dragonfruit Book");
}

#[test]
fn the_three_writes_apply_once_and_never_invent_a_snapshot() {
    let mut host = UpdatesHost::new(MockSystem::present(data(true)));
    host.refresh();
    assert_eq!(host.check()["outcome"], "applied");
    assert_eq!(host.install()["outcome"], "applied");
    assert_eq!(host.reboot()["outcome"], "applied");
    assert_eq!(host.adapter().source().checks(), 1);
    assert_eq!(host.adapter().source().installs(), 1);
    assert_eq!(host.adapter().source().reboots(), 1);
}

#[test]
fn absence_hides_the_item_and_writes_answer_absence() {
    let mut host = UpdatesHost::new(MockSystem::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert_eq!(host.check()["outcome"], "absent");
    assert_eq!(host.install()["outcome"], "absent");
    assert_eq!(host.reboot()["outcome"], "absent");
    assert_eq!(host.view()["state"], "unavailable");
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = UpdatesHost::new(MockSystem::failing("host stack: timeout"));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "error");
    assert_eq!(view["error"], "host stack: timeout");
}
