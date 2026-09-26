// SPDX-License-Identifier: MIT
//! `xdg-desktop-portal-dragonfruit` — the T-13.1a portal backend service.
//!
//! Serves the standard backend name
//! `org.freedesktop.impl.portal.desktop.dragonfruit` on the user session bus.
//! A session without a bus, or with no `xdg-desktop-portal` frontend, is
//! reported and never blocks startup.
//!
//! Debug helpers:
//!
//! ```text
//! xdg-desktop-portal-dragonfruit --print-interfaces   # one per line
//! xdg-desktop-portal-dragonfruit --print-identity     # name, path, backend
//! xdg-desktop-portal-dragonfruit --check-frontend     # present/absent/unknown
//! xdg-desktop-portal-dragonfruit --install-data PREFIX
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use xdg_desktop_portal_dragonfruit as portal;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => {}
        Some("--print-interfaces") => {
            for interface in portal::BACKEND_INTERFACES {
                println!("{interface}");
            }
            return ExitCode::SUCCESS;
        }
        Some("--print-identity") => {
            println!(
                "{}\t{}\t{}",
                portal::DBUS_NAME,
                portal::DBUS_PATH,
                portal::BACKEND_NAME
            );
            return ExitCode::SUCCESS;
        }
        Some("--check-frontend") => return check_frontend(),
        Some("--install-data") => {
            let Some(prefix) = args.next().map(PathBuf::from) else {
                eprintln!("xdg-desktop-portal-dragonfruit: --install-data needs a PREFIX");
                return ExitCode::from(2);
            };
            return match portal::install_into(&prefix) {
                Ok(written) => {
                    for path in written {
                        println!("{}", path.display());
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!(
                        "xdg-desktop-portal-dragonfruit: cannot install discoverability data \
                         under {}: {error}",
                        prefix.display()
                    );
                    ExitCode::FAILURE
                }
            };
        }
        Some("-h" | "--help") => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("xdg-desktop-portal-dragonfruit: unknown argument {other:?}");
            print_help();
            return ExitCode::from(2);
        }
    }

    match portal::dbus::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "xdg-desktop-portal-dragonfruit: cannot serve {} on the session bus: {error}",
                portal::DBUS_NAME
            );
            ExitCode::FAILURE
        }
    }
}

/// Print whether the `xdg-desktop-portal` frontend is on the session bus. An
/// unreachable bus or an absent frontend is reported, never an error: the
/// backend is valid without it.
fn check_frontend() -> ExitCode {
    match zbus::blocking::Connection::session() {
        Ok(connection) => {
            println!("{}", portal::probe_frontend(&connection).label());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("xdg-desktop-portal-dragonfruit: no session bus ({error}); frontend unknown");
            println!("unknown");
            ExitCode::SUCCESS
        }
    }
}

fn print_help() {
    println!(
        "xdg-desktop-portal-dragonfruit — T-13.1a portal backend\n\
         \n\
         Serves {name} on the user session bus and degrades\n\
         gracefully when the xdg-desktop-portal frontend is absent.\n\
         Options:\n\
           --print-interfaces         list advertised portal interfaces\n\
           --print-identity           print D-Bus name, object path, backend\n\
           --check-frontend           print present/absent/unknown and exit 0\n\
           --install-data PREFIX      install .portal, portals.conf, .service\n\
           -h, --help                 show this help",
        name = portal::DBUS_NAME
    );
}
