// SPDX-License-Identifier: MIT
//! `dragonfruit-system-status` — the T-07.5a bridge host.
//!
//! Hosts the NetworkManager and audio adapters and serves their snapshots and
//! actions to the shell on the user session bus
//! (`org.dragonfruit.SystemStatus1`). A session without a bus or without a
//! daemon is a normal state: the service still starts, and each item degrades
//! on its own.
//!
//! Debug helpers:
//!
//! ```text
//! dragonfruit-system-status --print-wifi     # refresh and print the JSON view
//! dragonfruit-system-status --print-audio
//! dragonfruit-system-status --print-battery
//! dragonfruit-system-status --print-bluetooth
//! dragonfruit-system-status --print-storage
//! dragonfruit-system-status --print-input
//! dragonfruit-system-status --print-notifications
//! dragonfruit-system-status --print-updates
//! dragonfruit-system-status --print-accounts
//! ```

use std::process::ExitCode;

use dragonfruit_account_adapter::HostAccounts;
use dragonfruit_audio::CommandAudio;
use dragonfruit_bluetooth::DbusBluez;
use dragonfruit_input::CommandLibinput;
use dragonfruit_networkmanager::DbusNetworkManager;
use dragonfruit_notify_adapter::DbusNotifications;
use dragonfruit_power::DbusUPower;
use dragonfruit_storage::DbusUDisks;
use dragonfruit_system_status::dbus;
use dragonfruit_system_status::AccountsHost;
use dragonfruit_system_status::BluetoothHost;
use dragonfruit_system_status::InputHost;
use dragonfruit_system_status::NotificationsHost;
use dragonfruit_system_status::StatusHost;
use dragonfruit_system_status::StorageHost;
use dragonfruit_system_status::UpdatesHost;
use dragonfruit_update_adapter::HostSystem;

fn main() -> ExitCode {
    let mut print_wifi = false;
    let mut print_audio = false;
    let mut print_battery = false;
    let mut print_bluetooth = false;
    let mut print_storage = false;
    let mut print_input = false;
    let mut print_notifications = false;
    let mut print_updates = false;
    let mut print_accounts = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--print-wifi" => print_wifi = true,
            "--print-audio" => print_audio = true,
            "--print-battery" => print_battery = true,
            "--print-bluetooth" => print_bluetooth = true,
            "--print-storage" => print_storage = true,
            "--print-input" => print_input = true,
            "--print-notifications" => print_notifications = true,
            "--print-updates" => print_updates = true,
            "--print-accounts" => print_accounts = true,
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("dragonfruit-system-status: unknown argument {other:?}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    let mut host = StatusHost::new(
        DbusNetworkManager::new(),
        CommandAudio::new(),
        DbusUPower::new(),
    );
    let mut bluetooth = BluetoothHost::new(DbusBluez::new());
    let mut storage = StorageHost::new(DbusUDisks::new());
    let mut input = InputHost::new(CommandLibinput::new());
    let mut notifications = NotificationsHost::new(DbusNotifications::new());
    // The host identity is always read; the distribution update provider is
    // absent until packaging attaches one (ADR 0136), so the update controls
    // report absence rather than inventing state.
    let mut updates = UpdatesHost::new(HostSystem::new());
    // AccountsService over the system bus, with no distribution group provider
    // attached until packaging supplies one (ADR 0138), so the group controls
    // report absence rather than inventing state.
    let mut accounts = AccountsHost::new(HostAccounts::new());

    if print_accounts {
        accounts.refresh();
        println!("{}", accounts.state());
        return ExitCode::SUCCESS;
    }
    if print_updates {
        updates.refresh();
        println!("{}", updates.state());
        return ExitCode::SUCCESS;
    }
    if print_input {
        input.refresh();
        println!("{}", input.state());
        return ExitCode::SUCCESS;
    }
    if print_notifications {
        notifications.refresh();
        println!("{}", notifications.state());
        return ExitCode::SUCCESS;
    }
    if print_storage {
        storage.refresh();
        println!("{}", storage.state());
        return ExitCode::SUCCESS;
    }
    if print_bluetooth {
        bluetooth.refresh();
        println!("{}", bluetooth.state());
        return ExitCode::SUCCESS;
    }
    if print_wifi {
        host.refresh_wifi();
        println!("{}", host.wifi_state());
        return ExitCode::SUCCESS;
    }
    if print_audio {
        host.refresh_audio();
        println!("{}", host.audio_state());
        return ExitCode::SUCCESS;
    }
    if print_battery {
        host.refresh_battery();
        println!("{}", host.battery_state());
        return ExitCode::SUCCESS;
    }

    host.refresh();
    bluetooth.refresh();
    storage.refresh();
    input.refresh();
    notifications.refresh();
    updates.refresh();
    accounts.refresh();
    match dbus::run(
        host,
        bluetooth,
        storage,
        input,
        notifications,
        updates,
        accounts,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "dragonfruit-system-status: cannot serve {} on the session bus: {error}",
                dragonfruit_system_status::DBUS_NAME
            );
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "dragonfruit-system-status — T-07.5a system status bridge host\n\
         \n\
         Serves org.dragonfruit.SystemStatus1 on the user session bus.\n\
         Options:\n\
           --print-wifi     refresh NetworkManager and print the Wi-Fi JSON view\n\
           --print-audio    refresh WirePlumber and print the audio JSON view\n\
           --print-battery  refresh UPower and print the battery JSON view\n\
           --print-bluetooth refresh BlueZ and print the Bluetooth JSON view\n\
           --print-storage  refresh UDisks2 and print the storage JSON view\n\
           --print-input    refresh libinput and print the input JSON view\n\
           --print-notifications refresh the notification service and print its JSON view\n\
           --print-updates  refresh the host stack and print the updates JSON view\n\
           --print-accounts refresh AccountsService/the group provider and print the\n\
                            Users and Groups JSON view\n\
           -h, --help       show this help"
    );
}
