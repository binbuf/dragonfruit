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
//! dragonfruit-system-status --print-wifi   # refresh and print the JSON view
//! dragonfruit-system-status --print-audio
//! ```

use std::process::ExitCode;

use dragonfruit_audio::CommandAudio;
use dragonfruit_networkmanager::DbusNetworkManager;
use dragonfruit_system_status::dbus;
use dragonfruit_system_status::StatusHost;

fn main() -> ExitCode {
    let mut print_wifi = false;
    let mut print_audio = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--print-wifi" => print_wifi = true,
            "--print-audio" => print_audio = true,
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

    let mut host = StatusHost::new(DbusNetworkManager::new(), CommandAudio::new());

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

    host.refresh();
    match dbus::run(host) {
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
           --print-wifi    refresh NetworkManager and print the Wi-Fi JSON view\n\
           --print-audio   refresh WirePlumber and print the audio JSON view\n\
           -h, --help      show this help"
    );
}
