// SPDX-License-Identifier: MIT
//! The session-bus surface: `org.dragonfruit.SystemStatus1`.
//!
//! Three interfaces live at one object path — `Wifi`, `Audio`, and `Battery`
//! — each with a `State()` read (the JSON view [`crate::wifi_view`] /
//! [`crate::audio_view`] / [`crate::battery_view`] produces) and an explicit
//! `Refresh()` re-sync; Wi-Fi, audio, and battery add the one write their menu
//! offers. The shell (C++/QML) owns the corresponding client; nothing on that
//! side links an adapter ([adr/0029]).
//!
//! [adr/0029]: ../../../docs/design/adr/0029-system-status-bridge-host.md

use std::sync::{Arc, Mutex};

use dragonfruit_audio::CommandAudio;
use dragonfruit_bluetooth::DbusBluez;
use dragonfruit_input::CommandLibinput;
use dragonfruit_networkmanager::DbusNetworkManager;
use dragonfruit_power::DbusUPower;
use dragonfruit_storage::DbusUDisks;
use zbus::blocking::connection;
use zbus::interface;

use crate::{
    BluetoothHost, InputHost, StatusHost, StorageHost, AUDIO_INTERFACE, BATTERY_INTERFACE,
    BLUETOOTH_INTERFACE, DBUS_NAME, DBUS_PATH, INPUT_INTERFACE, STORAGE_INTERFACE, WIFI_INTERFACE,
};

/// The live host: the three real network/audio/power adapter sources behind
/// the bridge.
pub type LiveHost = StatusHost<DbusNetworkManager, CommandAudio, DbusUPower>;

/// The live Bluetooth host: the real BlueZ source (system bus) behind the
/// bridge.
pub type LiveBluetooth = BluetoothHost<DbusBluez>;

/// The live storage host: the real UDisks2 source (system bus) behind the
/// bridge.
pub type LiveStorage = StorageHost<DbusUDisks>;

/// The live input host: the real `libinput` tool source behind the bridge.
pub type LiveInput = InputHost<CommandLibinput>;

/// The Wi-Fi half of the service.
pub struct WifiInterface {
    host: Arc<Mutex<LiveHost>>,
}

/// The audio half of the service.
pub struct AudioInterface {
    host: Arc<Mutex<LiveHost>>,
}

/// The battery half of the service: the charge state plus the one power-profile
/// write the Settings pane and Control Center tile offer (T-15.6b).
pub struct BatteryInterface {
    host: Arc<Mutex<LiveHost>>,
}

/// The Bluetooth half of the service (T-15.1b): state plus the four explicit
/// writes the Settings pane and Control Center tile offer.
pub struct BluetoothInterface {
    host: Arc<Mutex<LiveBluetooth>>,
}

/// The storage half of the service (T-15.2b): state plus the three explicit
/// writes the Settings pane and Control Center tile offer.
pub struct StorageInterface {
    host: Arc<Mutex<LiveStorage>>,
}

/// The read-only input half of the service (T-15.4b): the device inventory
/// libinput reports. There are no writes — the pane's preferences are
/// settingsd keys (ADR 0124).
pub struct InputInterface {
    host: Arc<Mutex<LiveInput>>,
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

    /// Route output to the sink with this PipeWire node id (T-15.3b). Returns
    /// the JSON report.
    fn set_default_sink(&self, id: u32) -> String {
        lock(&self.host).set_default_sink(id).to_string()
    }

    /// Route capture to the source with this PipeWire node id (T-15.3b).
    /// Returns the JSON report.
    fn set_default_source(&self, id: u32) -> String {
        lock(&self.host).set_default_source(id).to_string()
    }
}

#[interface(name = "org.dragonfruit.SystemStatus1.Battery")]
impl BatteryInterface {
    /// The current battery status view as JSON (the last adapter state).
    fn state(&self) -> String {
        lock(&self.host).battery_state()
    }

    /// Re-read UPower once and return the new view. The shell calls this when
    /// it opens the battery menu; the item is read-only and never writes.
    fn refresh(&self) -> String {
        let mut host = lock(&self.host);
        host.refresh_battery();
        host.battery_state()
    }

    /// Select the active power profile by its stable id (`power-saver`,
    /// `balanced`, `performance`). Returns the JSON report. One explicit write;
    /// the snapshot stays the adapter's read state (T-15.6b).
    fn set_active_profile(&self, profile: &str) -> String {
        lock(&self.host).set_active_profile(profile).to_string()
    }
}

#[interface(name = "org.dragonfruit.SystemStatus1.Bluetooth")]
impl BluetoothInterface {
    /// The current Bluetooth status view as JSON (the last adapter state).
    fn state(&self) -> String {
        lock_bluetooth(&self.host).state()
    }

    /// Re-read BlueZ once and return the new view. The shell and the Settings
    /// pane call this when they open; it is an explicit resync, not a poll.
    fn refresh(&self) -> String {
        let mut host = lock_bluetooth(&self.host);
        host.refresh();
        host.state()
    }

    /// Power the controller on or off. Returns the JSON report.
    fn set_powered(&self, powered: bool) -> String {
        lock_bluetooth(&self.host).set_powered(powered).to_string()
    }

    /// Start or stop an inquiry. Returns the JSON report.
    fn set_discovering(&self, discovering: bool) -> String {
        lock_bluetooth(&self.host)
            .set_discovering(discovering)
            .to_string()
    }

    /// Pair (bond) the device at `address`. Returns the JSON report.
    fn pair(&self, address: &str) -> String {
        lock_bluetooth(&self.host).pair(address).to_string()
    }

    /// Connect to or disconnect from the device at `address`. Returns the JSON
    /// report.
    fn set_connected(&self, address: &str, connected: bool) -> String {
        lock_bluetooth(&self.host)
            .set_connected(address, connected)
            .to_string()
    }
}

#[interface(name = "org.dragonfruit.SystemStatus1.Storage")]
impl StorageInterface {
    /// The current storage status view as JSON (the last adapter state).
    fn state(&self) -> String {
        lock_storage(&self.host).state()
    }

    /// Re-read UDisks2 once and return the new view. The shell and the Settings
    /// pane call this when they open; it is an explicit resync, not a poll.
    fn refresh(&self) -> String {
        let mut host = lock_storage(&self.host);
        host.refresh();
        host.state()
    }

    /// Mount the volume at `volume_path`. Returns the JSON report.
    fn mount(&self, volume_path: &str) -> String {
        lock_storage(&self.host).mount(volume_path).to_string()
    }

    /// Unmount the volume at `volume_path`. Returns the JSON report.
    fn unmount(&self, volume_path: &str) -> String {
        lock_storage(&self.host).unmount(volume_path).to_string()
    }

    /// Eject the drive at `drive_path`. Returns the JSON report.
    fn eject(&self, drive_path: &str) -> String {
        lock_storage(&self.host).eject(drive_path).to_string()
    }
}

#[interface(name = "org.dragonfruit.SystemStatus1.Input")]
impl InputInterface {
    /// The current input status view as JSON (the last adapter state).
    fn state(&self) -> String {
        lock_input(&self.host).state()
    }

    /// Re-read libinput once and return the new view. The shell and the
    /// Settings pane call this when they open; it is an explicit resync, not a
    /// poll. The adapter is read-only, so there are no write methods.
    fn refresh(&self) -> String {
        let mut host = lock_input(&self.host);
        host.refresh();
        host.state()
    }
}

/// Lock the shared host, recovering from a poisoned mutex: a D-Bus method may
/// panic on a bad argument, and the service must keep answering.
fn lock(host: &Arc<Mutex<LiveHost>>) -> std::sync::MutexGuard<'_, LiveHost> {
    host.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared Bluetooth host, recovering from a poisoned mutex.
fn lock_bluetooth(host: &Arc<Mutex<LiveBluetooth>>) -> std::sync::MutexGuard<'_, LiveBluetooth> {
    host.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared storage host, recovering from a poisoned mutex.
fn lock_storage(host: &Arc<Mutex<LiveStorage>>) -> std::sync::MutexGuard<'_, LiveStorage> {
    host.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared input host, recovering from a poisoned mutex.
fn lock_input(host: &Arc<Mutex<LiveInput>>) -> std::sync::MutexGuard<'_, LiveInput> {
    host.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Serve the three interfaces on the session bus until the process is asked to
/// stop. Returns an error only when the bus or the name cannot be taken; an
/// absent session bus exits with a message instead of blocking a session.
pub fn run(
    host: LiveHost,
    bluetooth: LiveBluetooth,
    storage: LiveStorage,
    input: LiveInput,
) -> zbus::Result<()> {
    let host = Arc::new(Mutex::new(host));
    let bluetooth = Arc::new(Mutex::new(bluetooth));
    let storage = Arc::new(Mutex::new(storage));
    let input = Arc::new(Mutex::new(input));
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
        .serve_at(
            DBUS_PATH,
            BatteryInterface {
                host: Arc::clone(&host),
            },
        )?
        .serve_at(
            DBUS_PATH,
            BluetoothInterface {
                host: Arc::clone(&bluetooth),
            },
        )?
        .serve_at(
            DBUS_PATH,
            StorageInterface {
                host: Arc::clone(&storage),
            },
        )?
        .serve_at(
            DBUS_PATH,
            InputInterface {
                host: Arc::clone(&input),
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
pub fn interface_names() -> [&'static str; 6] {
    [
        WIFI_INTERFACE,
        AUDIO_INTERFACE,
        BATTERY_INTERFACE,
        BLUETOOTH_INTERFACE,
        STORAGE_INTERFACE,
        INPUT_INTERFACE,
    ]
}
