// SPDX-License-Identifier: MIT
//! `dragonfruit-app-index` — the T-14.1a/T-14.1b application identity service.
//!
//! Serves `org.dragonfruit.AppIndex1` (identity resolution, themed icons,
//! install/update events, the launch registry, and recency) on the user
//! session bus. A session without a bus is reported and exited, never blocking
//! a session.
//!
//! Debug helpers:
//!
//! ```text
//! dragonfruit-app-index --resolve <identity>        # one record object
//! dragonfruit-app-index --resolve-window <class>    # one record object
//! dragonfruit-app-index --enumerate                 # every record, JSON array
//! dragonfruit-app-index --misses                    # the miss set
//! dragonfruit-app-index --refresh                   # rescan + index events
//! dragonfruit-app-index --icon <name> [size]        # themed icon path
//! ```

use std::collections::HashMap;
use std::process::ExitCode;

use dragonfruit_app_index::icons::IconTheme;
use dragonfruit_app_index::index::AppIndex;
use dragonfruit_app_index::{dbus, view};
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

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
        Some("--refresh") => {
            let mut index = AppIndex::load();
            let events = index.refresh();
            println!("{}", view::index_events_json(&events));
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
        Some("--mock-tray") => {
            let icon = args.get(1).cloned().unwrap_or_else(|| "firefox".to_owned());
            mock_tray(&icon)
        }
        Some("--mock-menu") => {
            let app_id = args
                .get(1)
                .cloned()
                .unwrap_or_else(|| "org.example.MockMenu".to_owned());
            mock_menu(&app_id)
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

/// A debug-only mock StatusNotifierItem a demo can show in the bar (T-14.3).
/// It owns `org.dragonfruit.MockTray`, serves an item plus a DBusMenu, and
/// registers with the running app-index watcher.
struct MockTrayItem {
    icon: String,
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl MockTrayItem {
    #[zbus(property)]
    fn id(&self) -> String {
        "mock-tray".to_owned()
    }

    #[zbus(property)]
    fn title(&self) -> String {
        "Mock Tray".to_owned()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "Active".to_owned()
    }

    #[zbus(property)]
    fn icon_name(&self) -> String {
        self.icon.clone()
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn menu(&self) -> ObjectPath<'static> {
        ObjectPath::try_from("/MenuBar").expect("valid path")
    }

    fn activate(&self, _x: i32, _y: i32) {}
    fn secondary_activate(&self, _x: i32, _y: i32) {}
    fn context_menu(&self, _x: i32, _y: i32) {}
}

struct MockTrayMenu;

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl MockTrayMenu {
    fn about_to_show(&self, _id: i32) -> bool {
        true
    }

    fn get_layout(&self, _parent: i32, _depth: i32, _names: Vec<String>) -> (u32, OwnedValue) {
        (1, mock_layout())
    }

    fn event(&self, id: i32, event_id: String, _data: OwnedValue, _timestamp: u32) -> bool {
        eprintln!("dragonfruit-app-index: mock tray menu event {id} ({event_id})");
        true
    }
}

fn mock_row(id: i32, label: &str, extra: &[(&str, OwnedValue)]) -> OwnedValue {
    let mut props: HashMap<String, OwnedValue> = HashMap::new();
    props.insert(
        "label".to_owned(),
        OwnedValue::try_from(Value::from(label)).unwrap(),
    );
    props.insert("enabled".to_owned(), OwnedValue::from(true));
    for (key, value) in extra {
        props.insert(
            (*key).to_owned(),
            value.try_clone().unwrap_or(OwnedValue::from(true)),
        );
    }
    OwnedValue::try_from(Value::from((id, props, Vec::<OwnedValue>::new()))).unwrap()
}

fn mock_layout() -> OwnedValue {
    let child = mock_row(7, "Preferences", &[]);
    let mut submenu_props: HashMap<String, OwnedValue> = HashMap::new();
    submenu_props.insert(
        "label".to_owned(),
        OwnedValue::try_from(Value::from("Tools")).unwrap(),
    );
    submenu_props.insert(
        "children-display".to_owned(),
        OwnedValue::try_from(Value::from("submenu")).unwrap(),
    );
    let submenu = OwnedValue::try_from(Value::from((5i32, submenu_props, vec![child]))).unwrap();
    let mut separator_props: HashMap<String, OwnedValue> = HashMap::new();
    separator_props.insert(
        "type".to_owned(),
        OwnedValue::try_from(Value::from("separator")).unwrap(),
    );
    let separator = OwnedValue::try_from(Value::from((
        2i32,
        separator_props,
        Vec::<OwnedValue>::new(),
    )))
    .unwrap();
    OwnedValue::try_from(Value::from((
        0i32,
        HashMap::<String, OwnedValue>::new(),
        vec![mock_row(1, "Show Window", &[]), separator, submenu],
    )))
    .unwrap()
}

fn mock_tray(icon: &str) -> ExitCode {
    let connection = match zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.name("org.dragonfruit.MockTray"))
        .and_then(|builder| {
            builder.serve_at(
                "/StatusNotifierItem",
                MockTrayItem {
                    icon: icon.to_owned(),
                },
            )
        })
        .and_then(|builder| builder.serve_at("/MenuBar", MockTrayMenu))
        .and_then(|builder| builder.build())
    {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("dragonfruit-app-index: mock tray cannot serve: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The watcher may not have taken its name yet; retry briefly.
    let mut registered = false;
    for _ in 0..20 {
        if let Ok(reply) = connection.call_method(
            Some(dbus::DBUS_NAME),
            dbus::WATCHER_PATH,
            Some(dbus::WATCHER_INTERFACE),
            "RegisterStatusNotifierItem",
            &("org.dragonfruit.MockTray",),
        ) {
            if reply.body().deserialize::<bool>().unwrap_or(false) {
                registered = true;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    eprintln!(
        "dragonfruit-app-index: mock tray registered={registered} icon={icon}; Ctrl-C to stop"
    );
    loop {
        std::thread::park();
    }
}

/// A debug-only mock DBusMenu global-menu app (T-14.4). It owns a well-known
/// name, serves a `com.canonical.dbusmenu` at `/MenuBar`, then registers with
/// the running app-index AppMenu.Registrar under `app_id`, so the bridge
/// fetches, projects, and pushes it into the menu-broker.
struct MockGlobalMenu;

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl MockGlobalMenu {
    fn about_to_show(&self, _id: i32) -> bool {
        true
    }

    fn get_layout(&self, _parent: i32, _depth: i32, _names: Vec<String>) -> (u32, OwnedValue) {
        (1, mock_global_layout())
    }

    fn event(&self, id: i32, event_id: String, _data: OwnedValue, _timestamp: u32) -> bool {
        eprintln!("dragonfruit-app-index: mock menu event {id} ({event_id})");
        true
    }
}

fn mock_shortcut() -> OwnedValue {
    let sequence = vec![vec!["<Ctrl>".to_owned(), "N".to_owned()]];
    OwnedValue::try_from(Value::from(sequence)).unwrap()
}

fn mock_global_layout() -> OwnedValue {
    // File › New (enabled, Ctrl+N), Quit (disabled); Edit › Cut.
    let new = mock_row(1, "New", &[("shortcut", mock_shortcut())]);
    let quit = mock_row(2, "Quit", &[("enabled", OwnedValue::from(false))]);
    let mut file_props: HashMap<String, OwnedValue> = HashMap::new();
    file_props.insert(
        "label".to_owned(),
        OwnedValue::try_from(Value::from("_File")).unwrap(),
    );
    file_props.insert(
        "children-display".to_owned(),
        OwnedValue::try_from(Value::from("submenu")).unwrap(),
    );
    let file = OwnedValue::try_from(Value::from((10i32, file_props, vec![new, quit]))).unwrap();

    let cut = mock_row(3, "Cut", &[]);
    let mut edit_props: HashMap<String, OwnedValue> = HashMap::new();
    edit_props.insert(
        "label".to_owned(),
        OwnedValue::try_from(Value::from("Edit")).unwrap(),
    );
    edit_props.insert(
        "children-display".to_owned(),
        OwnedValue::try_from(Value::from("submenu")).unwrap(),
    );
    let edit = OwnedValue::try_from(Value::from((11i32, edit_props, vec![cut]))).unwrap();

    OwnedValue::try_from(Value::from((
        0i32,
        HashMap::<String, OwnedValue>::new(),
        vec![file, edit],
    )))
    .unwrap()
}

fn mock_menu(app_id: &str) -> ExitCode {
    let connection = match zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.name(app_id))
        .and_then(|builder| builder.serve_at("/MenuBar", MockGlobalMenu))
        .and_then(|builder| builder.build())
    {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("dragonfruit-app-index: mock menu cannot serve: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The registrar may not have taken its name yet; retry briefly.
    let mut registered = false;
    for _ in 0..20 {
        if let Ok(reply) = connection.call_method(
            Some(crate::dbus::DBUS_NAME),
            crate::dbus::APP_MENU_REGISTRAR_PATH,
            Some(crate::dbus::APP_MENU_REGISTRAR_INTERFACE),
            "RegisterWindowForApp",
            &(
                1u32,
                app_id,
                ObjectPath::try_from("/MenuBar").expect("valid path"),
            ),
        ) {
            if reply.body().deserialize::<bool>().unwrap_or(false) {
                registered = true;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    eprintln!(
        "dragonfruit-app-index: mock menu app_id={app_id} registered={registered}; Ctrl-C to stop"
    );
    loop {
        std::thread::park();
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
         Serves org.dragonfruit.AppIndex1 (identity resolution, themed icons,\n\
         install/update events, launch registry, recency) on the user session\n\
         bus.\n\
         Options:\n\
           --resolve <identity>        resolve a Wayland app_id / desktop id\n\
           --resolve-window <class>    resolve an X11 WM_CLASS\n\
           --enumerate                 print every installed record\n\
           --misses                    print the identity miss set\n\
           --refresh                   rescan and print install/uninstall/update events\n\
           --icon <name> [size]        print a themed icon path\n\
           --mock-tray [icon]          serve a debug StatusNotifierItem + DBusMenu\n\
           --mock-menu [appId]         serve a debug global DBusMenu and register it\n\
           -h, --help                  show this help"
    );
}
