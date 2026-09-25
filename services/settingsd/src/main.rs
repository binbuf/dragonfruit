// SPDX-License-Identifier: MIT
//! `dragonfruit-settingsd` — the T-08.1a desktop-settings daemon.
//!
//! Serves `org.dragonfruit.Settings1` on the user session bus: the named,
//! schema-documented desktop keys (Dock, Spaces, gestures, appearance,
//! animation policy, input repeat) with get/set and change signals. A session
//! without a bus is reported and exited, never blocking a session.
//!
//! Debug helpers:
//!
//! ```text
//! dragonfruit-settingsd --print-keys    # the schema, one key per line
//! dragonfruit-settingsd --print-schema  # the schema revision
//! dragonfruit-settingsd --config-path   # the resolved settings file
//! ```

use std::process::ExitCode;

use dragonfruit_settingsd::dbus::{self, DBUS_NAME};
use dragonfruit_settingsd::persist::{self, Persistence};
use dragonfruit_settingsd::schema::{KEYS, SCHEMA_VERSION};
use dragonfruit_settingsd::Settings;

fn main() -> ExitCode {
    let mut print_keys = false;
    let mut print_schema = false;
    let mut print_config_path = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--print-keys" => print_keys = true,
            "--print-schema" => print_schema = true,
            "--config-path" => print_config_path = true,
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("dragonfruit-settingsd: unknown argument {other:?}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    if print_schema {
        println!("{SCHEMA_VERSION}");
        return ExitCode::SUCCESS;
    }
    if print_keys {
        print_keys_table();
        return ExitCode::SUCCESS;
    }
    if print_config_path {
        match persist::default_path() {
            Some(path) => println!("{}", path.display()),
            None => println!("(no $XDG_CONFIG_HOME or $HOME)"),
        }
        return ExitCode::SUCCESS;
    }

    let (settings, persistence) = load_settings();
    match dbus::run(settings, persistence) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "dragonfruit-settingsd: cannot serve {DBUS_NAME} on the session bus: {error}"
            );
            ExitCode::FAILURE
        }
    }
}

/// Load the persisted settings, migrating an older file in place. A session
/// with no config home runs from schema defaults; an unreadable or malformed
/// file is reported and never blocks the daemon.
fn load_settings() -> (Settings, Option<Persistence>) {
    let Some(path) = persist::default_path() else {
        eprintln!("dragonfruit-settingsd: no $XDG_CONFIG_HOME or $HOME; using schema defaults");
        return (Settings::new(), None);
    };
    match persist::load(&path) {
        Ok(loaded) => {
            if loaded.migrated {
                match loaded.persistence.save(&loaded.settings) {
                    Ok(()) => eprintln!(
                        "dragonfruit-settingsd: migrated {} to schema {SCHEMA_VERSION}",
                        path.display()
                    ),
                    Err(error) => eprintln!(
                        "dragonfruit-settingsd: cannot write migrated {}: {error}",
                        path.display()
                    ),
                }
            }
            (loaded.settings, Some(loaded.persistence))
        }
        Err(error) => {
            eprintln!(
                "dragonfruit-settingsd: cannot read {}: {error}; using schema defaults",
                path.display()
            );
            (Settings::new(), Some(Persistence::new(path)))
        }
    }
}

/// Print the schema as `key<TAB>type<TAB>owner<TAB>consumer<TAB>summary`,
/// the in-terminal view of the in-code key documentation.
fn print_keys_table() {
    for spec in KEYS {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            spec.key,
            spec.kind.signature(),
            spec.owner,
            spec.consumer,
            spec.summary
        );
    }
}

fn print_help() {
    println!(
        "dragonfruit-settingsd — T-08.1a desktop-settings daemon\n\
         \n\
         Serves org.dragonfruit.Settings1 on the user session bus.\n\
         Options:\n\
           --print-keys     print the key schema (key, type, owner, consumer, summary)\n\
           --print-schema   print the current schema revision\n\
           --config-path    print the resolved settings file path\n\
           -h, --help       show this help"
    );
}
