// SPDX-License-Identifier: MIT
//! `dragonfruit-app-index` — the T-14.1a application identity service.
//!
//! Serves `org.dragonfruit.AppIndex1` (identity resolution and themed icons)
//! on the user session bus. A session without a bus is reported and exited,
//! never blocking a session.
//!
//! Debug helpers:
//!
//! ```text
//! dragonfruit-app-index --resolve <identity>        # one record object
//! dragonfruit-app-index --resolve-window <class>    # one record object
//! dragonfruit-app-index --enumerate                 # every record, JSON array
//! dragonfruit-app-index --misses                    # the miss set
//! dragonfruit-app-index --icon <name> [size]        # themed icon path
//! ```

use std::process::ExitCode;

use dragonfruit_app_index::icons::IconTheme;
use dragonfruit_app_index::index::AppIndex;
use dragonfruit_app_index::{dbus, view};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => serve(),
        Some("--resolve") => {
            let Some(identity) = args.get(1) else {
                return usage("--resolve needs an identity");
            };
            let mut index = AppIndex::load();
            match index.resolve(identity) {
                Some(resolved) => {
                    println!(
                        "{}",
                        view::record_json(
                            &resolved.record,
                            resolved.source,
                            &IconTheme::from_env()
                        )
                    );
                    ExitCode::SUCCESS
                }
                None => ExitCode::from(1),
            }
        }
        Some("--resolve-window") => {
            let Some(class) = args.get(1) else {
                return usage("--resolve-window needs a WM_CLASS");
            };
            let mut index = AppIndex::load();
            match index.resolve_window("", "", class) {
                Some(resolved) => {
                    println!(
                        "{}",
                        view::record_json(
                            &resolved.record,
                            resolved.source,
                            &IconTheme::from_env()
                        )
                    );
                    ExitCode::SUCCESS
                }
                None => ExitCode::from(1),
            }
        }
        Some("--enumerate") => {
            let index = AppIndex::load();
            println!("{}", view::records_json(&index, &IconTheme::from_env()));
            ExitCode::SUCCESS
        }
        Some("--misses") => {
            let index = AppIndex::load();
            println!("{}", view::misses_json(&index));
            ExitCode::SUCCESS
        }
        Some("--icon") => {
            let Some(name) = args.get(1) else {
                return usage("--icon needs an icon name");
            };
            let size = args
                .get(2)
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(view::DEFAULT_ICON_SIZE);
            match IconTheme::from_env().lookup(name, size) {
                Some(path) => {
                    println!("{}", path.display());
                    ExitCode::SUCCESS
                }
                None => ExitCode::from(1),
            }
        }
        Some("-h" | "--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("dragonfruit-app-index: unknown argument {other:?}");
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
                "dragonfruit-app-index: cannot serve {} on the session bus: {error}",
                dbus::DBUS_NAME
            );
            ExitCode::FAILURE
        }
    }
}

fn usage(message: &str) -> ExitCode {
    eprintln!("dragonfruit-app-index: {message}");
    print_help();
    ExitCode::from(2)
}

fn print_help() {
    println!(
        "dragonfruit-app-index — T-14.1a application identity service\n\
         \n\
         Serves org.dragonfruit.AppIndex1 (identity resolution, themed icons)\n\
         on the user session bus.\n\
         Options:\n\
           --resolve <identity>        resolve a Wayland app_id / desktop id\n\
           --resolve-window <class>    resolve an X11 WM_CLASS\n\
           --enumerate                 print every installed record\n\
           --misses                    print the identity miss set\n\
           --icon <name> [size]        print a themed icon path\n\
           -h, --help                  show this help"
    );
}
