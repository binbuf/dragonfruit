// SPDX-License-Identifier: MIT
//! `dragonfruit-menu-broker` — the T-14.2a global-menu broker.
//!
//! Serves `org.dragonfruit.MenuBroker1` (native menu-model publication, the
//! focus/window-state input, and menu resolution with the fixed application
//! menu's live state) on the user session bus. A session without a bus is
//! reported and exited, never blocking a session.
//!
//! Debug helpers:
//!
//! ```text
//! dragonfruit-menu-broker                       # serve the session bus
//! dragonfruit-menu-broker --fixed <appId>       # the Tier-3 fixed menu, JSON
//! dragonfruit-menu-broker --resolve <appId>     # a resolved menu, JSON
//! ```

use std::process::ExitCode;

use dragonfruit_menu_broker::dbus;
use dragonfruit_menu_broker::model::Broker;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => serve(),
        Some("--fixed") => {
            let Some(app_id) = args.get(1) else {
                return usage("--fixed needs an app id");
            };
            println!("{}", Broker::new().fixed_menu(app_id));
            ExitCode::SUCCESS
        }
        Some("--resolve") => {
            let Some(app_id) = args.get(1) else {
                return usage("--resolve needs an app id");
            };
            println!("{}", Broker::new().resolve(app_id));
            ExitCode::SUCCESS
        }
        Some("-h" | "--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("dragonfruit-menu-broker: unknown argument {other:?}");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn serve() -> ExitCode {
    match dbus::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "dragonfruit-menu-broker: cannot serve {} on the session bus: {error}",
                dbus::DBUS_NAME
            );
            ExitCode::FAILURE
        }
    }
}

fn usage(message: &str) -> ExitCode {
    eprintln!("dragonfruit-menu-broker: {message}");
    print_help();
    ExitCode::from(2)
}

fn print_help() {
    println!(
        "dragonfruit-menu-broker — T-14.2a global-menu broker\n\
         \n\
         Serves org.dragonfruit.MenuBroker1 (native menu-model publication,\n\
         focus/window-state input, menu resolution with the fixed application\n\
         menu's live Hide/Hide Others/Show All state) on the user session bus.\n\
         Options:\n\
           --fixed <appId>       print the Tier-3 fixed application menu\n\
           --resolve <appId>     print the resolved menu\n\
           -h, --help            show this help"
    );
}
