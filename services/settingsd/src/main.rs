// SPDX-License-Identifier: GPL-3.0-only
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
    let mut print_schema_json_flag = false;
    let mut print_schema = false;
    let mut print_config_path = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--print-keys" => print_keys = true,
            "--print-schema-json" => print_schema_json_flag = true,
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
    if print_schema_json_flag {
        print_schema_json();
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

    let (settings, persistence, loaded_schema) = load_settings();
    match dbus::run(settings, persistence, loaded_schema) {
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
/// file is preserved and never blocks the daemon (T282).
fn load_settings() -> (Settings, Option<Persistence>, u32) {
    let Some(path) = persist::default_path() else {
        eprintln!("dragonfruit-settingsd: no $XDG_CONFIG_HOME or $HOME; using schema defaults");
        return (Settings::new(), None, SCHEMA_VERSION);
    };
    match persist::load(&path) {
        Ok(loaded) => {
            if let Some(backup) = &loaded.backup {
                eprintln!(
                    "dragonfruit-settingsd: {} was written by schema {} (this build reads {}); \
                     preserved at {} before any rewrite",
                    path.display(),
                    loaded.schema,
                    SCHEMA_VERSION,
                    backup.display()
                );
            }
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
            let loaded_schema = loaded.schema;
            (loaded.settings, Some(loaded.persistence), loaded_schema)
        }
        Err(error) => {
            // The settings file is user data: never overwrite a malformed file
            // without keeping a copy first.
            let backup = persist::backup(&path, "malformed")
                .map(|backup| backup.display().to_string())
                .unwrap_or_else(|backup_error| format!("(backup failed: {backup_error})"));
            eprintln!(
                "dragonfruit-settingsd: cannot read {}: {error}; preserved at {backup}; \
                 using schema defaults",
                path.display()
            );
            (
                Settings::new(),
                Some(Persistence::new(path)),
                persist::UNVERSIONED,
            )
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

/// Print the machine-readable schema the cross-language mirror test compares
/// against the client's `settingsSchemaDefaults()` / `settingsSchemaInfo()`
/// tables (T282). Every declared type, default, constraint, and the
/// stored-only flag is included, so drift fails a test instead of production.
fn print_schema_json() {
    use dragonfruit_settingsd::schema::KeyDefault;
    use serde_json::json;

    let keys: Vec<_> = KEYS
        .iter()
        .map(|spec| {
            let default = match spec.default {
                KeyDefault::Bool(value) => json!(value),
                KeyDefault::Number(value) => json!(value),
                KeyDefault::Integer(value) => json!(value),
                KeyDefault::Text(value) => json!(value),
                KeyDefault::List(items) => json!(items),
            };
            json!({
                "key": spec.key,
                "type": spec.kind.signature(),
                "default": default,
                "allowed": spec.allowed,
                "min": spec.min,
                "max": spec.max,
                "storedOnly": spec.stored_only,
                "since": spec.since,
                "owner": spec.owner,
                "consumer": spec.consumer,
            })
        })
        .collect();
    let document = json!({ "schema": SCHEMA_VERSION, "keys": keys });
    println!("{document}");
}

fn print_help() {
    println!(
        "dragonfruit-settingsd — T-08.1a desktop-settings daemon\n\
         \n\
         Serves org.dragonfruit.Settings1 on the user session bus.\n\
         Options:\n\
           --print-keys     print the key schema (key, type, owner, consumer, summary)\n\
           --print-schema-json  print the machine-readable schema (JSON)\n\
           --print-schema   print the current schema revision\n\
           --config-path    print the resolved settings file path\n\
           -h, --help       show this help"
    );
}
