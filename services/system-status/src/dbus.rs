// SPDX-License-Identifier: MIT
//! The session-bus surface: `org.dragonfruit.SystemStatus1`.
//!
//! Two interfaces live at one object path — `Wifi` and `Audio` — each with a
//! `State()` read (the JSON view [`crate::wifi_view`]/[`crate::audio_view`]
//! produces), an explicit `Refresh()` re-sync, and the one write the menu
//! offers. The shell (C++/QML) owns the corresponding client; nothing on that
//! side links an adapter ([adr/0029]).
//!
//! [adr/0029]: ../../../docs/design/adr/0029-system-status-bridge-host.md

use std::sync::{Arc, Mutex};

use dragonfruit_audio::CommandAudio;
use dragonfruit_networkmanager::DbusNetworkManager;
use zbus::blocking::connection;
use zbus::interface;

use crate::{StatusHost, AUDIO_INTERFACE, DBUS_NAME, DBUS_PATH, WIFI_INTERFACE};

/// The live host: the two real adapter sources behind the bridge.
pub type LiveHost = StatusHost<DbusNetworkManager, CommandAudio>;

/// The Wi-Fi half of the service.
pub struct WifiInterface {
    host: Arc<Mutex<LiveHost>>,
}

/// The audio half of the service.
pub struct AudioInterface {
    host: Arc<Mutex<LiveHost>>,
}

#[interface(name = "org.dragonfruit.SystemStatus1.Wifi")]
impl WifiInterface {
    /// The current Wi-Fi status view as JSON (the last adapter state).
    fn state(&self) -> String {
        lock(&self.host).wifi_state()
    }

    /// Re-read NetworkManager once and return the new view. The shell calls
    /// this when it opens the Wi-Fi menu; it is an explicit resync, not a poll
    /// loop above the adapter.
    fn refresh(&self) -> String {
        let mut host = lock(&self.host);
        host.refresh_wifi();
        host.wifi_state()
    }

    /// Join a network. `secret` may be empty for an open network; the report
    /// is JSON (`accepted`/`denied`/`absent`/`failed`).
    fn join(&self, ssid: &str, secret: &str) -> String {
        let mut host = lock(&self.host);
        let secret = (!secret.is_empty()).then_some(secret);
        host.join(ssid, secret).to_string()
    }
}

#[interface(name = "org.dragonfruit.SystemStatus1.Audio")]
impl AudioInterface {
    /// The current audio status view as JSON (the last adapter state).
    fn state(&self) -> String {
        lock(&self.host).audio_state()
    }

    /// Re-read WirePlumber once and return the new view.
    fn refresh(&self) -> String {
        let mut host = lock(&self.host);
        host.refresh_audio();
        host.audio_state()
    }

    /// Set the default sink's linear volume (0..=1). Returns the JSON report.
    fn set_volume(&self, volume: f64) -> String {
        lock(&self.host).set_volume(volume as f32).to_string()
    }

    /// Set the default sink's mute state. Returns the JSON report.
    fn set_mute(&self, muted: bool) -> String {
        lock(&self.host).set_mute(muted).to_string()
    }

    /// Flip the default sink's mute state. Returns the JSON report.
    fn toggle_mute(&self) -> String {
        lock(&self.host).toggle_mute().to_string()
    }
}

/// Lock the shared host, recovering from a poisoned mutex: a D-Bus method may
/// panic on a bad argument, and the service must keep answering.
fn lock(host: &Arc<Mutex<LiveHost>>) -> std::sync::MutexGuard<'_, LiveHost> {
    host.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Serve the two interfaces on the session bus until the process is asked to
/// stop. Returns an error only when the bus or the name cannot be taken; an
/// absent session bus exits with a message instead of blocking a session.
pub fn run(host: LiveHost) -> zbus::Result<()> {
    let host = Arc::new(Mutex::new(host));
    let connection = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(
            DBUS_PATH,
            WifiInterface {
                host: Arc::clone(&host),
            },
        )?
        .serve_at(
            DBUS_PATH,
            AudioInterface {
                host: Arc::clone(&host),
            },
        )?
        .build()?;

    // The blocking object server runs on its own executor; parking the main
    // thread keeps the process (and the name) alive without a poll loop.
    let _ = connection;
    loop {
        std::thread::park();
    }
}

/// The interface names the service serves, for logs and tests.
pub fn interface_names() -> [&'static str; 2] {
    [WIFI_INTERFACE, AUDIO_INTERFACE]
}
