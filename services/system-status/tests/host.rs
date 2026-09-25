// SPDX-License-Identifier: MIT
//! End-to-end bridge tests over the mock adapters (T-07.5a): the two menu
//! models decode to the actions the shell exposes, and each action applies
//! through the adapter exactly once.

use dragonfruit_audio::{AudioData, MockAudio, SinkData};
use dragonfruit_networkmanager::{
    AccessPointData, MockNetworkManager, NetworkManagerData, WifiDeviceData,
};
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

#[test]
fn the_wifi_menu_model_exposes_networks_and_join() {
    let mut host = StatusHost::new(
        MockNetworkManager::present(wifi_data()),
        MockAudio::present(audio_data(0.5, false)),
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
    let mut host = StatusHost::new(MockNetworkManager::absent(), MockAudio::absent());
    host.refresh();

    assert_eq!(host.wifi_view()["state"], "unavailable");
    assert_eq!(host.audio_view()["state"], "unavailable");
    assert_eq!(host.join("home", None)["outcome"], "absent");
    assert_eq!(host.set_volume(0.5)["outcome"], "absent");
    assert_eq!(host.set_mute(true)["outcome"], "absent");
}

#[test]
fn a_daemon_restart_resyncs_the_menu() {
    let mut host = StatusHost::new(
        MockNetworkManager::present(wifi_data()),
        MockAudio::present(audio_data(0.5, false)),
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
