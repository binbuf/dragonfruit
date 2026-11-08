// SPDX-License-Identifier: MIT
//! Bridge tests for the General sub-panes host (T226): each sub-view carries
//! its own state, the writes apply once through the adapter, and a missing
//! owner disables only its own pane.

use dragonfruit_general_adapter::{fixture, GeneralSnapshot, MockGeneral};
use dragonfruit_system_status::GeneralHost;

#[test]
fn the_general_view_exposes_every_sub_pane() {
    let mut host = GeneralHost::new(MockGeneral::present());
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "general");
    assert_eq!(view["state"], "available");
    assert_eq!(view["time"]["timezone"], "America/Los_Angeles");
    assert_eq!(view["locale"]["locale"], "en_US.UTF-8");
    assert_eq!(
        view["login"]["autostart"][0]["id"],
        "org.dragonfruit.Backup"
    );
    assert_eq!(view["sharing"]["services"][1]["id"], "remoteLogin");
    assert_eq!(view["backup"]["backend"], "timeshift");
    assert_eq!(view["storage"]["categories"][1]["id"], "home");
    assert_eq!(view["startup"]["secureBoot"], "On");
}

#[test]
fn one_missing_owner_is_a_local_absence() {
    let mut data: GeneralSnapshot = fixture();
    data.sharing = None;
    data.errors.insert(
        "sharing".to_owned(),
        "systemctl is not available".to_owned(),
    );
    let mut host = GeneralHost::new(MockGeneral::with_data(data));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["sharing"]["state"], "error");
    assert_eq!(view["sharing"]["error"], "systemctl is not available");
    assert_eq!(view["time"]["state"], "available");
    assert_eq!(view["backup"]["state"], "available");
}

#[test]
fn writes_apply_once_and_report_the_outcome() {
    let mut host = GeneralHost::new(MockGeneral::present());
    host.refresh();
    assert_eq!(host.set_timezone("Europe/Berlin")["outcome"], "applied");
    assert_eq!(host.set_ntp(false)["outcome"], "applied");
    assert_eq!(host.set_time("2026-11-07 12:00:00")["outcome"], "applied");
    assert_eq!(host.set_locale("de_DE.UTF-8")["outcome"], "applied");
    assert_eq!(
        host.set_autostart_enabled("org.example.Agent", true)["outcome"],
        "applied"
    );
    assert_eq!(
        host.set_user_unit_enabled("sync.service", true)["outcome"],
        "applied"
    );
    assert_eq!(
        host.set_sharing_enabled("remoteLogin", false)["outcome"],
        "applied"
    );
    assert_eq!(host.run_backup("/mnt/backup")["outcome"], "applied");
    assert_eq!(host.open_restore()["outcome"], "applied");
    assert_eq!(host.adapter().source().writes(), 9);
}

#[test]
fn absence_hides_the_host_and_writes_answer_absence() {
    let mut host = GeneralHost::new(MockGeneral::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert_eq!(host.set_ntp(true)["outcome"], "absent");
    assert_eq!(host.run_backup("/mnt/backup")["outcome"], "absent");
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = GeneralHost::new(MockGeneral::failing("general host: timeout"));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "error");
    assert_eq!(view["error"], "general host: timeout");
}
