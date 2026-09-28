// SPDX-License-Identifier: MIT
//! The session-bus surface: `org.dragonfruit.SystemStatus1`.
//!
//! Every subsystem interface lives at one object path — `Wifi`, `Audio`,
//! `Battery`, `Bluetooth`, `Storage`, `Input`, `Notifications`, `Updates`, and
//! `Accounts` — each with a `State()` read (the JSON view the matching
//! `*_view` produces) and an explicit `Refresh()` re-sync; the interfaces that
//! own one add the writes their pane offers. The shell (C++/QML) owns the
//! corresponding client; nothing on that side links an adapter ([adr/0029]).
//!
//! [adr/0029]: ../../../docs/design/adr/0029-system-status-bridge-host.md

use std::sync::{Arc, Mutex};

use dragonfruit_account_adapter::HostAccounts;
use dragonfruit_audio::CommandAudio;
use dragonfruit_bluetooth::DbusBluez;
use dragonfruit_input::CommandLibinput;
use dragonfruit_networkmanager::DbusNetworkManager;
use dragonfruit_notify_adapter::DbusNotifications;
use dragonfruit_power::DbusUPower;
use dragonfruit_storage::DbusUDisks;
use dragonfruit_update_adapter::HostSystem;
use zbus::blocking::connection;
use zbus::interface;

use crate::{
    AccountsHost, BluetoothHost, InputHost, NotificationsHost, StatusHost, StorageHost,
    UpdatesHost, ACCOUNTS_INTERFACE, AUDIO_INTERFACE, BATTERY_INTERFACE, BLUETOOTH_INTERFACE,
    DBUS_NAME, DBUS_PATH, INPUT_INTERFACE, NOTIFICATIONS_INTERFACE, STORAGE_INTERFACE,
    UPDATES_INTERFACE, WIFI_INTERFACE,
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

/// The live notifications host: the real session-bus notification-service
/// source behind the bridge (T-15.7b).
pub type LiveNotifications = NotificationsHost<DbusNotifications>;

/// The live General/About/Updates host: the real host-stack identity read
/// (with no distribution update provider attached until packaging supplies one,
/// ADR 0136) behind the bridge (T-15.10b).
pub type LiveUpdates = UpdatesHost<HostSystem>;

/// The live Users and Groups host: AccountsService over the system bus with no
/// distribution group provider attached until packaging supplies one (ADR
/// 0138) behind the bridge (T-15.11b).
pub type LiveAccounts = AccountsHost<HostAccounts>;

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

/// The Notifications/Focus half of the service (T-15.7b): the Focus/DND
/// policy and the observed per-app notification list, plus the two Focus
/// writes the Settings pane offers.
pub struct NotificationsInterface {
    host: Arc<Mutex<LiveNotifications>>,
}

/// The General/About/Updates half of the service (T-15.10b): the host identity
/// plus the distribution update provider's state, and the three explicit update
/// writes the Settings pane offers.
pub struct UpdatesInterface {
    host: Arc<Mutex<LiveUpdates>>,
}

/// The Users and Groups half of the service (T-15.11b): the AccountsService
/// user list plus the distro group provider's list, and the eight explicit
/// account/group writes the Settings pane offers. The Control Center tile is a
/// read-only summary of the same view.
pub struct AccountsInterface {
    host: Arc<Mutex<LiveAccounts>>,
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

#[interface(name = "org.dragonfruit.SystemStatus1.Notifications")]
impl NotificationsInterface {
    /// The current Notifications/Focus status view as JSON (the last adapter
    /// state).
    fn state(&self) -> String {
        lock_notifications(&self.host).state()
    }

    /// Re-read the notification service once and return the new view. The
    /// Settings pane calls this when it opens; it is an explicit resync, not a
    /// poll.
    fn refresh(&self) -> String {
        let mut host = lock_notifications(&self.host);
        host.refresh();
        host.state()
    }

    /// Set the Focus/DND mode (`off`/`focus`/`dnd`). Returns the JSON report.
    fn set_focus_mode(&self, mode: &str) -> String {
        lock_notifications(&self.host)
            .set_focus_mode(mode)
            .to_string()
    }

    /// Replace the per-app Focus allow list. Returns the JSON report.
    fn set_focus_allow_list(&self, apps: Vec<String>) -> String {
        lock_notifications(&self.host)
            .set_focus_allow_list(apps)
            .to_string()
    }
}

#[interface(name = "org.dragonfruit.SystemStatus1.Updates")]
impl UpdatesInterface {
    /// The current General/About/Updates view as JSON (the last adapter state).
    /// The host identity is always read; the update provider may be absent
    /// within it (`updatesAvailable: false`), which disables only the update
    /// controls.
    fn state(&self) -> String {
        lock_updates(&self.host).state()
    }

    /// Re-read the host stack once and return the new view. The shell and the
    /// Settings pane call this when they open; it is an explicit resync, not a
    /// poll.
    fn refresh(&self) -> String {
        let mut host = lock_updates(&self.host);
        host.refresh();
        host.state()
    }

    /// Ask the distribution provider to check for updates. Returns the JSON
    /// report. One explicit write; the provider publishes the result and the
    /// host re-reads.
    fn check(&self) -> String {
        lock_updates(&self.host).check().to_string()
    }

    /// Ask the distribution provider to install the available updates.
    /// Returns the JSON report. One explicit write.
    fn install(&self) -> String {
        lock_updates(&self.host).install().to_string()
    }

    /// Ask the distribution provider to restart the host to finish an update.
    /// Returns the JSON report. One explicit write.
    fn reboot(&self) -> String {
        lock_updates(&self.host).reboot().to_string()
    }
}

#[interface(name = "org.dragonfruit.SystemStatus1.Accounts")]
impl AccountsInterface {
    /// The current Users and Groups view as JSON (the last adapter state). The
    /// user list is always read when AccountsService answers; the group
    /// provider may be absent within it (`groupsAvailable: false`), which
    /// disables only the group controls.
    fn state(&self) -> String {
        lock_accounts(&self.host).state()
    }

    /// Re-read the host stack once and return the new view. The shell and the
    /// Settings pane call this when they open; it is an explicit resync, not a
    /// poll.
    fn refresh(&self) -> String {
        let mut host = lock_accounts(&self.host);
        host.refresh();
        host.state()
    }

    /// Create a user account. `account_type` is a stable id (`standard` or
    /// `administrator`); an unknown id fails without reaching the daemon.
    fn create_user(&self, user_name: &str, real_name: &str, account_type: &str) -> String {
        lock_accounts(&self.host)
            .create_user(user_name, real_name, account_type)
            .to_string()
    }

    /// Delete the user with `uid`. One explicit write.
    fn delete_user(&self, uid: u64) -> String {
        lock_accounts(&self.host).delete_user(uid).to_string()
    }

    /// Set the account type of the user with `uid`. One explicit write.
    fn set_account_type(&self, uid: u64, account_type: &str) -> String {
        lock_accounts(&self.host)
            .set_account_type(uid, account_type)
            .to_string()
    }

    /// Lock or unlock the user with `uid`. One explicit write.
    fn set_locked(&self, uid: u64, locked: bool) -> String {
        lock_accounts(&self.host)
            .set_locked(uid, locked)
            .to_string()
    }

    /// Set or clear automatic login for the user with `uid`. One explicit
    /// write.
    fn set_automatic_login(&self, uid: u64, automatic_login: bool) -> String {
        lock_accounts(&self.host)
            .set_automatic_login(uid, automatic_login)
            .to_string()
    }

    /// Create a group. One explicit write (the distro provider's).
    fn create_group(&self, name: &str) -> String {
        lock_accounts(&self.host).create_group(name).to_string()
    }

    /// Delete a group. One explicit write (the distro provider's).
    fn delete_group(&self, name: &str) -> String {
        lock_accounts(&self.host).delete_group(name).to_string()
    }

    /// Replace a group's membership. One explicit write (the distro
    /// provider's).
    fn set_group_members(&self, name: &str, members: Vec<String>) -> String {
        lock_accounts(&self.host)
            .set_group_members(name, members)
            .to_string()
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

/// Lock the shared notifications host, recovering from a poisoned mutex.
fn lock_notifications(
    host: &Arc<Mutex<LiveNotifications>>,
) -> std::sync::MutexGuard<'_, LiveNotifications> {
    host.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared General/About/Updates host, recovering from a poisoned
/// mutex.
fn lock_updates(host: &Arc<Mutex<LiveUpdates>>) -> std::sync::MutexGuard<'_, LiveUpdates> {
    host.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared Users and Groups host, recovering from a poisoned mutex.
fn lock_accounts(host: &Arc<Mutex<LiveAccounts>>) -> std::sync::MutexGuard<'_, LiveAccounts> {
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
    notifications: LiveNotifications,
    updates: LiveUpdates,
    accounts: LiveAccounts,
) -> zbus::Result<()> {
    let host = Arc::new(Mutex::new(host));
    let bluetooth = Arc::new(Mutex::new(bluetooth));
    let storage = Arc::new(Mutex::new(storage));
    let input = Arc::new(Mutex::new(input));
    let notifications = Arc::new(Mutex::new(notifications));
    let updates = Arc::new(Mutex::new(updates));
    let accounts = Arc::new(Mutex::new(accounts));
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
        .serve_at(
            DBUS_PATH,
            NotificationsInterface {
                host: Arc::clone(&notifications),
            },
        )?
        .serve_at(
            DBUS_PATH,
            UpdatesInterface {
                host: Arc::clone(&updates),
            },
        )?
        .serve_at(
            DBUS_PATH,
            AccountsInterface {
                host: Arc::clone(&accounts),
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
pub fn interface_names() -> [&'static str; 9] {
    [
        WIFI_INTERFACE,
        AUDIO_INTERFACE,
        BATTERY_INTERFACE,
        BLUETOOTH_INTERFACE,
        STORAGE_INTERFACE,
        INPUT_INTERFACE,
        NOTIFICATIONS_INTERFACE,
        UPDATES_INTERFACE,
        ACCOUNTS_INTERFACE,
    ]
}
