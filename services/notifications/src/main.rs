// SPDX-License-Identifier: MIT
//! `dragonfruit-notifications` — the T-11.1a notification service.
//!
//! Serves `org.freedesktop.Notifications` (apps) and
//! `org.dragonfruit.Notifications1` (the shell's banners and history) on the
//! user session bus. A session without a bus is reported and exited, never
//! blocking a session.
//!
//! Debug helpers:
//!
//! ```text
//! dragonfruit-notifications --print-capabilities  # one capability per line
//! dragonfruit-notifications --server-information # name, vendor, version
//! ```

use std::process::ExitCode;

use dragonfruit_notifications::dbus::{self, DBUS_NAME};

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("--print-capabilities") => {
            println!("body\nbody-markup\nicon-static\nactions");
            return ExitCode::SUCCESS;
        }
        Some("--server-information") => {
            println!("Dragonfruit\tDragonfruit\t{}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("-h" | "--help") => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("dragonfruit-notifications: unknown argument {other:?}");
            print_help();
            return ExitCode::from(2);
        }
    }

    match dbus::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "dragonfruit-notifications: cannot serve {DBUS_NAME} on the session bus: {error}"
            );
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "dragonfruit-notifications — T-11.1a notification service\n\
         \n\
         Serves org.freedesktop.Notifications (apps) and\n\
         org.dragonfruit.Notifications1 (shell banners/history) on the user\n\
         session bus.\n\
         Options:\n\
           --print-capabilities    print the advertised capabilities\n\
           --server-information    print name, vendor, version\n\
           -h, --help              show this help"
    );
}
