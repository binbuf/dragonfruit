// SPDX-License-Identifier: MIT
//! End-to-end bridge tests over the mock adapters (T-07.5a): the two menu
//! models decode to the actions the shell exposes, and each action applies
//! through the adapter exactly once.

use dragonfruit_audio::{AudioData, MockAudio, SinkData};
use dragonfruit_networkmanager::{
    AccessPointData, MockNetworkManager, NetworkManagerData, WifiDeviceData,
};
use dragonfruit_power::{MockPower, PowerData, PowerDeviceData, DEVICE_TYPE_BATTERY};
use dragonfruit_system_status::StatusHost;

fn wifi_data() -> NetworkManagerData {
    NetworkManagerData {
        wireless_enabled: true,
        connectivity: 4,
        devices: vec![WifiDeviceData {
            managed: true,
            state: 100,
            active_ap: Some("/ap/home".to_owned()),
            access_points: vec![
                AccessPointData {
                    path: "/ap/home".to_owned(),
                    ssid: "home".to_owned(),
                    strength: 82,
                    frequency_mhz: 5180,
                    flags: 0x1,
                    wpa_flags: 0,
                    rsn_flags: 0x108,
                },
                AccessPointData {
                    path: "/ap/cafe".to_owned(),
                    ssid: "cafe".to_owned(),
                    strength: 40,
                    frequency_mhz: 2412,
                    flags: 0,
                    wpa_flags: 0,
                    rsn_flags: 0,
                },
            ],
        }],
    }
}

fn audio_data(volume: f32, muted: bool) -> AudioData {
    AudioData {
        default_sink: Some("speakers".to_owned()),
        sinks: vec![SinkData {
            id: 7,
            name: "speakers".to_owned(),
            description: "Built-in Speakers".to_owned(),
            volume,
            muted,
        }],
    }
}

fn battery_data(percentage: f64, state: u32) -> PowerData {
    PowerData {
        on_battery: state == 2,
        devices: vec![PowerDeviceData {
            path: "/org/freedesktop/UPower/devices/battery_BAT0".to_owned(),
            kind: DEVICE_TYPE_BATTERY,
            present: true,
            power_supply: true,
            percentage,
            state,
            battery_level: 4,
            time_to_empty: 0,
            time_to_full: 0,
        }],
    }
}

#[test]
fn the_wifi_menu_model_exposes_networks_and_join() {
    let mut host = StatusHost::new(
        MockNetworkManager::present(wifi_data()),
        MockAudio::present(audio_data(0.5, false)),
        MockPower::absent(),
    );
    host.refresh();

    let view = host.wifi_view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["activeSsid"], "home");
    assert_eq!(view["networks"].as_array().unwrap().len(), 2);
    // Join is the one action the Wi-Fi menu applies.
    assert_eq!(host.join("cafe", None)["outcome"], "accepted");
    assert_eq!(host.wifi().source().activations(), 1);
    // A join never invents a snapshot; a refresh sees the daemon's answer.
    assert_eq!(host.wifi_view()["activeSsid"], "home");
}

#[test]
fn the_volume_menu_model_exposes_a_slider_and_mute() {
    let mut host = StatusHost::new(
        MockNetworkManager::present(wifi_data()),
        MockAudio::present(audio_data(0.5, false)),
        MockPower::absent(),
    );
    host.refresh();

    let view = host.audio_view();
    assert_eq!(view["state"], "available");
    assert!((view["volume"].as_f64().unwrap() - 0.5).abs() < 1e-6);
    assert_eq!(view["percent"], 50);
    assert_eq!(view["muted"], false);
    assert_eq!(view["sinks"][0]["default"], true);

    assert_eq!(host.set_volume(0.8)["outcome"], "applied");
    host.refresh_audio();
    let view = host.audio_view();
    assert!((view["volume"].as_f64().unwrap() - 0.8).abs() < 1e-6);
    assert_eq!(view["percent"], 80);

    assert_eq!(host.toggle_mute()["outcome"], "applied");
    host.refresh_audio();
    let view = host.audio_view();
    assert_eq!(view["muted"], true);
    assert_eq!(view["glyph"], "volume-muted");
    assert_eq!(host.audio().source().volume_writes(), 1);
    assert_eq!(host.audio().source().mute_writes(), 1);
}

#[test]
fn an_absent_daemon_hides_both_items_and_never_errors() {
    let mut host = StatusHost::new(
        MockNetworkManager::absent(),
        MockAudio::absent(),
        MockPower::absent(),
    );
    host.refresh();

    assert_eq!(host.wifi_view()["state"], "unavailable");
    assert_eq!(host.audio_view()["state"], "unavailable");
    assert_eq!(host.battery_view()["state"], "unavailable");
    assert_eq!(host.join("home", None)["outcome"], "absent");
    assert_eq!(host.set_volume(0.5)["outcome"], "absent");
    assert_eq!(host.set_mute(true)["outcome"], "absent");
}

/// The three menu items' `state` strings, in bar order: Wi-Fi, volume,
/// battery. `available` means the slot is live; `unavailable` means the
/// daemon is masked and the slot hides; `error` means visible but inert.
fn bar_state(host: &StatusHost<MockNetworkManager, MockAudio, MockPower>) -> [String; 3] {
    let state_of =
        |view: &serde_json::Value| view["state"].as_str().unwrap_or("missing").to_owned();
    [
        state_of(&host.wifi_view()),
        state_of(&host.audio_view()),
        state_of(&host.battery_view()),
    ]
}

/// T-07.6a: the absent-daemon masking matrix. Mask one daemon at a time and
/// the other two items must stay live; mask all three and every item hides
/// without an error and the explicit actions still answer (absence, never a
/// panic). `kill()`/`refresh` is the headless stand-in for `systemctl stop`
/// plus the daemon's `Disconnected` event.
#[test]
fn the_absent_daemon_masking_matrix_hides_only_the_masked_item() {
    let mut host = StatusHost::new(
        MockNetworkManager::present(wifi_data()),
        MockAudio::present(audio_data(0.5, false)),
        MockPower::present(battery_data(82.0, 1)),
    );
    host.refresh();

    let live = [
        "available".to_owned(),
        "available".to_owned(),
        "available".to_owned(),
    ];
    assert_eq!(bar_state(&host), live);

    // Mask Wi-Fi: only the Wi-Fi item hides.
    host.wifi_mut().source_mut().kill();
    host.refresh_wifi();
    assert_eq!(
        bar_state(&host),
        [
            "unavailable".to_owned(),
            "available".to_owned(),
            "available".to_owned()
        ]
    );
    host.wifi_mut().source_mut().restart();
    host.refresh_wifi();

    // Mask audio: only the volume item hides.
    host.audio_mut().source_mut().kill();
    host.refresh_audio();
    assert_eq!(
        bar_state(&host),
        [
            "available".to_owned(),
            "unavailable".to_owned(),
            "available".to_owned()
        ]
    );
    host.audio_mut().source_mut().restart();
    host.refresh_audio();

    // Mask power: only the battery item hides.
    host.battery_mut().source_mut().kill();
    host.refresh_battery();
    assert_eq!(
        bar_state(&host),
        [
            "available".to_owned(),
            "available".to_owned(),
            "unavailable".to_owned()
        ]
    );
    host.battery_mut().source_mut().restart();
    host.refresh_battery();
    assert_eq!(bar_state(&host), live);

    // Mask all three at once: every item hides, nothing reports `error`, and
    // the session-level actions still answer instead of panicking.
    host.wifi_mut().source_mut().kill();
    host.audio_mut().source_mut().kill();
    host.battery_mut().source_mut().kill();
    host.refresh();
    assert_eq!(
        bar_state(&host),
        [
            "unavailable".to_owned(),
            "unavailable".to_owned(),
            "unavailable".to_owned()
        ]
    );
    assert_eq!(host.join("home", None)["outcome"], "absent");
    assert_eq!(host.set_volume(0.5)["outcome"], "absent");
    assert_eq!(host.set_mute(true)["outcome"], "absent");
}

/// The masking matrix is about absence, not failure: a daemon that is present
/// but unreadable stays visible and inert (`error`), so masking one item never
/// leaks an `error` into a neighbour's view.
#[test]
fn masking_one_daemon_never_turns_a_neighbour_into_an_error() {
    let mut host = StatusHost::new(
        MockNetworkManager::failing("NetworkManager: timeout"),
        MockAudio::present(audio_data(0.5, false)),
        MockPower::present(battery_data(82.0, 1)),
    );
    host.refresh();

    assert_eq!(
        bar_state(&host),
        [
            "error".to_owned(),
            "available".to_owned(),
            "available".to_owned()
        ]
    );
    assert_eq!(host.wifi_view()["error"], "NetworkManager: timeout");

    // The audio and battery slots are untouched by the Wi-Fi error.
    assert_eq!(host.audio_view()["state"], "available");
    assert_eq!(host.battery_view()["state"], "available");
}

#[test]
fn the_battery_menu_model_is_read_only_and_never_writes() {
    let mut host = StatusHost::new(
        MockNetworkManager::absent(),
        MockAudio::absent(),
        MockPower::present(battery_data(64.0, 2)),
    );
    host.refresh();

    let view = host.battery_view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], true);
    assert_eq!(view["percent"], 64);
    assert_eq!(view["charging"], false);
    assert_eq!(view["onBattery"], true);
    assert_eq!(view["label"], "64%");
    // The battery interface exposes no write action; the mock saw one read.
    assert!(view.get("outcome").is_none());
    assert_eq!(host.battery().source().reads(), 1);
}

#[test]
fn a_machine_without_a_battery_keeps_the_item_available_but_not_present() {
    let mut host = StatusHost::new(
        MockNetworkManager::absent(),
        MockAudio::absent(),
        MockPower::present(PowerData::default()),
    );
    host.refresh();
    let view = host.battery_view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], false);
}

/// T-07.6b: the live status menu contributes zero wakeups. The host reads a
/// daemon only on the startup sync, an explicit `Refresh()` (menu open), or
/// after an action; there is no poll loop and no timer. Serving the three
/// views — what every `State()` on the bus does — is a pure read of the
/// already-pushed adapter state, so an idle bar produces no daemon traffic. A
/// regression that introduced a background poll would move these read
/// counters without a matching `refresh`.
#[test]
fn the_live_status_items_never_poll_the_daemons() {
    let mut host = StatusHost::new(
        MockNetworkManager::present(wifi_data()),
        MockAudio::present(audio_data(0.5, false)),
        MockPower::present(battery_data(82.0, 1)),
    );
    // The startup sync reads each daemon exactly once.
    host.refresh();
    assert_eq!(host.wifi().source().reads(), 1);
    assert_eq!(host.audio().source().reads(), 1);
    assert_eq!(host.battery().source().reads(), 1);

    // Rendering the live views many times touches no daemon.
    for _ in 0..50 {
        assert_eq!(host.wifi_view()["state"], "available");
        assert_eq!(host.audio_view()["state"], "available");
        assert_eq!(host.battery_view()["state"], "available");
    }
    assert_eq!(host.wifi().source().reads(), 1);
    assert_eq!(host.audio().source().reads(), 1);
    assert_eq!(host.battery().source().reads(), 1);

    // One explicit resync maps to exactly one new read per daemon; only an
    // event (or a user action) may advance these.
    host.refresh();
    assert_eq!(host.wifi().source().reads(), 2);
    assert_eq!(host.audio().source().reads(), 2);
    assert_eq!(host.battery().source().reads(), 2);
}

#[test]
fn a_daemon_restart_resyncs_the_menu() {
    let mut host = StatusHost::new(
        MockNetworkManager::present(wifi_data()),
        MockAudio::present(audio_data(0.5, false)),
        MockPower::absent(),
    );
    host.refresh();

    host.wifi_mut().source_mut().kill();
    host.refresh_wifi();
    assert_eq!(host.wifi_view()["state"], "unavailable");

    host.wifi_mut().source_mut().restart();
    host.refresh_wifi();
    assert_eq!(host.wifi_view()["state"], "available");
    assert_eq!(host.wifi_view()["activeSsid"], "home");
}
