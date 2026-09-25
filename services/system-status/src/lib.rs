// SPDX-License-Identifier: MIT
//! The bridge host: the T-07 system adapters as one session-bus service for
//! the shell menu bar (T-07.5a).
//!
//! The C++ shell never links a Rust adapter (design/01-architecture.md: D-Bus
//! is the services seam). Instead this crate hosts the concrete adapters and
//! serves their typed snapshots over `org.dragonfruit.SystemStatus1` on the
//! user session bus ([adr/0029]).
//!
//! # The seam
//!
//! [`StatusHost`] is the whole bridge above the adapters and below D-Bus. It
//! owns one [`NetworkManagerAdapter`] and one [`AudioAdapter`], and turns the
//! two menu surfaces into JSON views:
//!
//! * [`wifi_view`] — the Wi-Fi status slot, glyph/label, and the network list.
//! * [`audio_view`] — the volume status slot, glyph/label, and the sink list.
//!
//! The views are deliberately flat JSON so the shell side is a decoder, not a
//! second model: the adapter already owns aggregation, defaults, and the
//! three-state degradation.
//!
//! # Actions
//!
//! [`StatusHost::join`], [`StatusHost::set_volume`], [`StatusHost::set_mute`],
//! and [`StatusHost::toggle_mute`] are the explicit user actions. They call
//! the adapter write path once and report the outcome; they never invent a
//! snapshot (the adapter's read state stays the single source of truth).
//!
//! # No polling
//!
//! The host reads an adapter only when its daemon signals a change (the live
//! signal subscription is the follow-up recorded in `docs/PROGRESS.md`) or on
//! an explicit refresh; it never polls. The adapter crate owns the read/write
//! mechanics; this crate never names a daemon.
//!
//! [adr/0029]: ../../../docs/design/adr/0029-system-status-bridge-host.md

use dragonfruit_audio::{AudioAdapter, AudioSource, SetOutcome};
use dragonfruit_networkmanager::{
    JoinRequest, JoinResult, NetworkManagerAdapter, NetworkManagerSource, WifiAccess,
};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

pub mod dbus;

/// The stable well-known name the shell looks up on the session bus.
pub const DBUS_NAME: &str = "org.dragonfruit.SystemStatus1";
/// The object path both menu interfaces are served at.
pub const DBUS_PATH: &str = "/org/dragonfruit/SystemStatus1";
/// The Wi-Fi interface name under [`DBUS_NAME`].
pub const WIFI_INTERFACE: &str = "org.dragonfruit.SystemStatus1.Wifi";
/// The audio interface name under [`DBUS_NAME`].
pub const AUDIO_INTERFACE: &str = "org.dragonfruit.SystemStatus1.Audio";

/// The host owns the two menu-bar adapters and exposes their snapshots and
/// actions. Generic over the transport seams so CI drives it with the mocks.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusHost<N, A> {
    wifi: NetworkManagerAdapter<N>,
    audio: AudioAdapter<A>,
}

impl<N: NetworkManagerSource, A: AudioSource> StatusHost<N, A> {
    /// A host over the two adapter sources.
    pub fn new(wifi: N, audio: A) -> Self {
        StatusHost {
            wifi: NetworkManagerAdapter::new(wifi),
            audio: AudioAdapter::new(audio),
        }
    }

    /// Re-read Wi-Fi once. Called on startup and when NetworkManager signals.
    pub fn refresh_wifi(&mut self) {
        self.wifi.refresh();
    }

    /// Re-read audio once. Called on startup and when WirePlumber signals.
    pub fn refresh_audio(&mut self) {
        self.audio.refresh();
    }

    /// Re-read both daemons once (the startup sync).
    pub fn refresh(&mut self) {
        self.refresh_wifi();
        self.refresh_audio();
    }

    /// The Wi-Fi status view the shell renders.
    pub fn wifi_view(&self) -> Value {
        wifi_view(&self.wifi)
    }

    /// The audio status view the shell renders.
    pub fn audio_view(&self) -> Value {
        audio_view(&self.audio)
    }

    /// The Wi-Fi view as a JSON string (the D-Bus `State()` payload).
    pub fn wifi_state(&self) -> String {
        self.wifi_view().to_string()
    }

    /// The audio view as a JSON string (the D-Bus `State()` payload).
    pub fn audio_state(&self) -> String {
        self.audio_view().to_string()
    }

    /// Join a network. One explicit write; the report is JSON.
    pub fn join(&mut self, ssid: &str, secret: Option<&str>) -> Value {
        let result = self
            .wifi
            .join(&JoinRequest::new(ssid, secret.map(str::to_owned)));
        join_report(&result)
    }

    /// Set the default sink's linear volume (0..=1). One explicit write.
    pub fn set_volume(&mut self, volume: f32) -> Value {
        let outcome = self.audio.set_volume(volume);
        write_report(&outcome)
    }

    /// Set the default sink's mute state. One explicit write.
    pub fn set_mute(&mut self, muted: bool) -> Value {
        let outcome = self.audio.set_mute(muted);
        write_report(&outcome)
    }

    /// Flip the default sink's mute state. One explicit write.
    pub fn toggle_mute(&mut self) -> Value {
        let outcome = self.audio.toggle_mute();
        write_report(&outcome)
    }

    /// The Wi-Fi adapter (read-only), for tests and introspection.
    pub fn wifi(&self) -> &NetworkManagerAdapter<N> {
        &self.wifi
    }

    /// The audio adapter (read-only), for tests and introspection.
    pub fn audio(&self) -> &AudioAdapter<A> {
        &self.audio
    }

    /// The Wi-Fi adapter, mutably (mostly for tests that drive a mock source).
    pub fn wifi_mut(&mut self) -> &mut NetworkManagerAdapter<N> {
        &mut self.wifi
    }

    /// The audio adapter, mutably (mostly for tests that drive a mock source).
    pub fn audio_mut(&mut self) -> &mut AudioAdapter<A> {
        &mut self.audio
    }
}

/// The `kind` discriminator every section carries, so one decode path can
/// reject a payload from an unexpected interface.
const KIND_WIFI: &str = "wifi";
const KIND_AUDIO: &str = "audio";

/// Build the Wi-Fi status view from an adapter.
///
/// The three states map straight to the contract: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the live
/// glyph/label and the network list.
pub fn wifi_view<N: NetworkManagerSource>(adapter: &NetworkManagerAdapter<N>) -> Value {
    let read_only = adapter.is_read_only();
    let note = adapter.degradation_note().unwrap_or_default();
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_WIFI, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_WIFI,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = snapshot(state) else {
        return json!({ "kind": KIND_WIFI, "state": "unavailable" });
    };
    let networks: Vec<Value> = snapshot
        .access_points
        .iter()
        .map(|ap| {
            json!({
                "ssid": ap.ssid,
                "strength": ap.strength,
                "security": ap.security.label(),
                "secured": ap.security.is_secured(),
                "active": ap.active,
                "band": ap.band().label(),
            })
        })
        .collect();
    json!({
        "kind": KIND_WIFI,
        "state": "available",
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "radioEnabled": snapshot.enabled,
        "wifiState": snapshot.state.name(),
        "connectivity": snapshot.connectivity.label(),
        "activeSsid": snapshot.active_ssid,
        "readOnly": read_only,
        "note": note,
        "networkCount": snapshot.network_count(),
        "networks": networks,
    })
}

/// Build the audio status view from an adapter.
pub fn audio_view<A: AudioSource>(adapter: &AudioAdapter<A>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_AUDIO, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_AUDIO,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = snapshot(state) else {
        return json!({ "kind": KIND_AUDIO, "state": "unavailable" });
    };
    let sinks: Vec<Value> = snapshot
        .sinks
        .iter()
        .map(|sink| {
            json!({
                "id": sink.id,
                "name": sink.name,
                "description": sink.description,
                "volume": sink.volume,
                "percent": sink.volume_percent(),
                "muted": sink.muted,
                "default": sink.default,
            })
        })
        .collect();
    json!({
        "kind": KIND_AUDIO,
        "state": "available",
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "volume": snapshot.volume(),
        "percent": snapshot.volume_percent(),
        "muted": snapshot.muted(),
        "defaultSink": snapshot.default_sink,
        "sinkCount": snapshot.sink_count(),
        "sinks": sinks,
    })
}

/// Read the snapshot out of an available state, by value-free reference.
fn snapshot<T>(state: &dragonfruit_system_adapters::AdapterState<T>) -> Option<&T> {
    state.snapshot()
}

/// The JSON report for a [`JoinResult`].
pub fn join_report(result: &JoinResult) -> Value {
    match result {
        JoinResult::Accepted => json!({ "outcome": "accepted" }),
        JoinResult::Denied { note } => json!({ "outcome": "denied", "note": note }),
        JoinResult::Absent => json!({ "outcome": "absent" }),
        JoinResult::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

/// The JSON report for a volume/mute [`SetOutcome`].
pub fn write_report(outcome: &SetOutcome) -> Value {
    match outcome {
        SetOutcome::Applied => json!({ "outcome": "applied" }),
        SetOutcome::Absent => json!({ "outcome": "absent" }),
        SetOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

/// Whether a Wi-Fi adapter may join (the shell disables the join affordance
/// when it may not). Kept here so the shell does not re-derive the policy.
pub fn can_join(access: &WifiAccess) -> bool {
    !access.is_read_only()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_audio::{AudioData, MockAudio, SinkData};
    use dragonfruit_networkmanager::{
        AccessPointData, MockNetworkManager, NetworkManagerData, WifiDeviceData,
    };

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

    fn audio_data() -> AudioData {
        AudioData {
            default_sink: Some("speakers".to_owned()),
            sinks: vec![SinkData {
                id: 7,
                name: "speakers".to_owned(),
                description: "Built-in Speakers".to_owned(),
                volume: 0.6,
                muted: false,
            }],
        }
    }

    #[test]
    fn an_absent_daemon_projects_a_hidden_wifi_slot() {
        let host = StatusHost::new(MockNetworkManager::absent(), MockAudio::absent());
        let view = host.wifi_view();
        assert_eq!(view["kind"], "wifi");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn an_available_daemon_projects_the_network_list_and_actions() {
        let mut host = StatusHost::new(
            MockNetworkManager::present(wifi_data()),
            MockAudio::present(audio_data()),
        );
        host.refresh();
        let view = host.wifi_view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["glyph"], "wifi-secure");
        assert_eq!(view["activeSsid"], "home");
        assert_eq!(view["networkCount"], 2);
        // Strongest first: home (82) before cafe (40).
        assert_eq!(view["networks"][0]["ssid"], "home");
        assert_eq!(view["networks"][0]["strength"], 82);
        assert_eq!(view["networks"][0]["security"], "WPA2");
        assert_eq!(view["networks"][0]["active"], true);
        assert_eq!(view["networks"][0]["band"], "5 GHz");
        assert_eq!(view["readOnly"], false);
    }

    #[test]
    fn the_audio_view_carries_the_default_sink_and_volume() {
        let mut host = StatusHost::new(
            MockNetworkManager::absent(),
            MockAudio::present(audio_data()),
        );
        host.refresh();
        let view = host.audio_view();
        assert_eq!(view["kind"], "audio");
        assert_eq!(view["state"], "available");
        assert_eq!(view["glyph"], "volume");
        // JSON carries the f32 as a float; compare the renderable percent too.
        assert!((view["volume"].as_f64().unwrap() - 0.6).abs() < 1e-6);
        assert_eq!(view["percent"], 60);
        assert_eq!(view["muted"], false);
        assert_eq!(view["defaultSink"], "speakers");
        assert_eq!(view["sinks"][0]["description"], "Built-in Speakers");
    }

    #[test]
    fn a_join_applies_through_the_adapter() {
        let mut host = StatusHost::new(
            MockNetworkManager::present(wifi_data()),
            MockAudio::absent(),
        );
        host.refresh();
        let report = host.join("cafe", Some("hunter2"));
        assert_eq!(report["outcome"], "accepted");
        // The adapter saw exactly one explicit write, never a loop.
        assert_eq!(host.wifi().source().activations(), 1);
    }

    #[test]
    fn a_polkit_denial_degrades_and_is_reported() {
        let mut host = StatusHost::new(
            MockNetworkManager::present(wifi_data()),
            MockAudio::absent(),
        );
        host.refresh();
        host.wifi_mut().source_mut().deny_joins("not authorized");
        let report = host.join("home", None);
        assert_eq!(report["outcome"], "denied");
        assert_eq!(report["note"], "not authorized");
        assert!(host.wifi().is_read_only());
    }

    #[test]
    fn volume_and_mute_apply_through_the_adapter() {
        let mut host = StatusHost::new(
            MockNetworkManager::absent(),
            MockAudio::present(audio_data()),
        );
        host.refresh();
        assert_eq!(host.set_volume(0.9)["outcome"], "applied");
        assert_eq!(host.audio().source().volume(), Some(0.9));
        assert_eq!(host.set_mute(true)["outcome"], "applied");
        assert_eq!(host.audio().source().muted(), Some(true));
        // Toggle reads the last *pushed* snapshot, so re-read first.
        host.refresh_audio();
        assert_eq!(host.toggle_mute()["outcome"], "applied");
        assert_eq!(host.audio().source().muted(), Some(false));
        assert_eq!(host.audio().source().volume_writes(), 1);
        assert_eq!(host.audio().source().mute_writes(), 2);
    }

    #[test]
    fn an_absent_daemon_write_reports_absence_and_hides() {
        let mut host = StatusHost::new(MockNetworkManager::absent(), MockAudio::absent());
        assert_eq!(host.set_volume(0.5)["outcome"], "absent");
        assert!(host.audio().state().is_unavailable());
    }
}
