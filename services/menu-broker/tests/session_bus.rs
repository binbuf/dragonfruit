// SPDX-License-Identifier: MIT
//! T-14.2a acceptance: drive `org.dragonfruit.MenuBroker1` over a real session
//! bus with a fixture export.
//!
//! Each test stands up a private `dbus-daemon` session bus and serves the
//! interface exactly as [`dragonfruit_menu_broker::dbus::run`] does, then talks
//! to it through a second connection — the shape the Qt shell uses. A fixture
//! export must resolve to the expected menu model, and the fixed application
//! menu's Hide/Hide Others/Show All flags must follow the live window state.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use dragonfruit_menu_broker::dbus::{MenuBroker1, DBUS_NAME, DBUS_PATH, INTERFACE};
use zbus::blocking::Connection;

/// A private session bus that lives exactly as long as the test.
struct PrivateBus {
    address: String,
    child: Child,
}

impl PrivateBus {
    fn start() -> Self {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("dbus-daemon is required to run the T-14.2a integration test");
        let stdout = child.stdout.take().expect("piped dbus-daemon stdout");
        let mut reader = BufReader::new(stdout);
        let mut address = String::new();
        reader
            .read_line(&mut address)
            .expect("dbus-daemon prints its address");
        PrivateBus {
            address: address.trim().to_owned(),
            child,
        }
    }

    fn connect(&self) -> Connection {
        zbus::blocking::connection::Builder::address(self.address.as_str())
            .expect("valid bus address")
            .build()
            .expect("connect to the private session bus")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Serve a fresh broker on `bus` exactly as the daemon does.
fn serve(bus: &PrivateBus) -> Connection {
    let object = MenuBroker1::new();
    zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .serve_at(DBUS_PATH, object)
        .expect("serve the menu broker interface")
        .build()
        .expect("build the service connection")
}

fn call<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
    client: &Connection,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error> {
    let reply = client.call_method(Some(DBUS_NAME), DBUS_PATH, Some(INTERFACE), method, &body)?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

fn json(payload: &str) -> serde_json::Value {
    serde_json::from_str(payload).expect("the view is valid JSON")
}

/// The exact shape of `apps/settings/SettingsMenu.qml`'s `publishedModel`.
const SETTINGS_FIXTURE: &str = r#"{
    "appName": "Settings",
    "applicationMenuItems": [
        { "label": "About Settings", "action": "about" },
        { "label": "Settings\u2026", "shortcut": "Super+,", "action": "settings" },
        { "type": "separator" },
        { "label": "Hide Settings", "shortcut": "Super+H", "action": "hide" },
        { "label": "Hide Others", "shortcut": "Super+Alt+H", "action": "hide-others" },
        { "label": "Show All", "action": "show-all" },
        { "type": "separator" },
        { "label": "Quit Settings", "shortcut": "Super+Q", "action": "quit" }
    ],
    "menus": [
        { "title": "File", "items": [
            { "label": "Close Window", "shortcut": "Super+W", "action": "close" } ] },
        { "title": "Edit", "items": [
            { "label": "Undo", "shortcut": "Super+Z", "action": "edit.undo" },
            { "label": "Redo", "shortcut": "Super+Shift+Z", "action": "edit.redo" } ] },
        { "title": "View", "items": [
            { "label": "Enter Full Screen", "shortcut": "Ctrl+Super+F", "action": "fullscreen" } ] },
        { "title": "Window", "items": [ { "label": "Minimize", "action": "minimize" } ] },
        { "title": "Help", "items": [ { "label": "Settings Help", "action": "help" } ] }
    ]
}"#;

#[test]
fn a_fixture_export_resolves_to_the_expected_menu_model() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    let published: bool = call(
        &client,
        "Publish",
        ("org.dragonfruit.Settings", SETTINGS_FIXTURE),
    )
    .unwrap();
    assert!(published);

    assert!(call::<bool>(&client, "SetFocusedApp", ("org.dragonfruit.Settings",)).unwrap());
    assert!(call::<bool>(
        &client,
        "SetWindowStates",
        (r#"[ { "appId": "org.dragonfruit.Settings", "windows": 1, "minimized": false } ]"#,),
    )
    .unwrap());

    let resolved = json(&call::<String>(&client, "ResolveFocused", ()).unwrap());
    assert_eq!(resolved["appName"], "Settings");
    assert_eq!(resolved["tier"], "native");
    let app_menu = resolved["applicationMenuItems"].as_array().unwrap();
    assert_eq!(app_menu.len(), 8);
    assert_eq!(app_menu[0]["label"], "About Settings");
    assert_eq!(app_menu[3]["label"], "Hide Settings");
    // Focused + visible => Hide and Hide Others on, Show All off.
    assert_eq!(app_menu[3]["enabled"], true);
    assert_eq!(app_menu[4]["enabled"], false);
    assert_eq!(app_menu[5]["enabled"], false);

    let menus = resolved["menus"].as_array().unwrap();
    let titles: Vec<&str> = menus.iter().map(|m| m["title"].as_str().unwrap()).collect();
    assert_eq!(titles, ["File", "Edit", "View", "Window", "Help"]);
    assert_eq!(menus[0]["items"][0]["label"], "Close Window");
}

#[test]
fn the_fixed_menu_flags_follow_the_live_window_state() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    // Two visible apps, focused on the first: Hide + Hide Others, no Show All.
    call::<bool>(
        &client,
        "SetWindowStates",
        r#"[ { "appId": "a", "windows": 1, "minimized": false },
             { "appId": "b", "windows": 2, "minimized": false } ]"#,
    )
    .unwrap();
    call::<bool>(&client, "SetFocusedApp", ("a",)).unwrap();

    let policy = json(&call::<String>(&client, "Policy", ("a",)).unwrap());
    let app_menu = policy["applicationMenuItems"].as_array().unwrap();
    assert_eq!(app_menu[3]["enabled"], true); // Hide a
    assert_eq!(app_menu[4]["enabled"], true); // Hide Others
    assert_eq!(app_menu[5]["enabled"], false); // Show All

    // Hide everything: only Show All stays meaningful.
    call::<bool>(
        &client,
        "SetWindowStates",
        r#"[ { "appId": "a", "windows": 1, "minimized": true },
             { "appId": "b", "windows": 2, "minimized": true } ]"#,
    )
    .unwrap();
    let policy = json(&call::<String>(&client, "Policy", ("a",)).unwrap());
    let app_menu = policy["applicationMenuItems"].as_array().unwrap();
    assert_eq!(app_menu[3]["enabled"], false);
    assert_eq!(app_menu[4]["enabled"], false);
    assert_eq!(app_menu[5]["enabled"], true);
}

#[test]
fn the_empty_desktop_resolves_to_the_files_fixed_menu() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    let resolved = json(&call::<String>(&client, "ResolveFocused", ()).unwrap());
    assert_eq!(resolved["appName"], "Files");
    assert_eq!(resolved["tier"], "none");
    let app_menu = resolved["applicationMenuItems"].as_array().unwrap();
    assert_eq!(app_menu[7]["label"], "Quit Files");
    assert!(resolved["menus"].as_array().unwrap().is_empty());
}

#[test]
fn registrations_are_focus_scoped_and_dispatch_over_the_bus() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    call::<bool>(
        &client,
        "Publish",
        ("org.dragonfruit.Settings", SETTINGS_FIXTURE),
    )
    .unwrap();
    call::<bool>(&client, "Publish", ("org.example.Other", SETTINGS_FIXTURE)).unwrap();
    call::<bool>(&client, "SetFocusedApp", ("org.dragonfruit.Settings",)).unwrap();

    // The focused app's accelerators are exposed for the shell to register.
    let accelerators = json(&call::<String>(&client, "FocusedAccelerators", ()).unwrap());
    let pairs: Vec<(&str, &str)> = accelerators
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["action"].as_str().unwrap(), a["chord"].as_str().unwrap()))
        .collect();
    assert!(pairs.contains(&("hide", "Super+H")));
    assert!(pairs.contains(&("edit.undo", "Super+Z")));
    assert!(pairs.contains(&("fullscreen", "Super+Control+F")));

    // Dispatch routes to the focused app.
    let dispatched = json(&call::<String>(&client, "Dispatch", ("Super+Q",)).unwrap());
    assert_eq!(dispatched["kind"], "application");
    assert_eq!(dispatched["appId"], "org.dragonfruit.Settings");
    assert_eq!(dispatched["action"], "quit");

    // Focus the other app: the same chord now resolves to its action.
    call::<bool>(&client, "SetFocusedApp", ("org.example.Other",)).unwrap();
    let dispatched = json(&call::<String>(&client, "Dispatch", ("Super+Q",)).unwrap());
    assert_eq!(dispatched["appId"], "org.example.Other");

    // A reserved system chord never dispatches to an app.
    assert!(call::<bool>(&client, "SetSystemAccelerators", (r#"["Super+Q"]"#,)).unwrap());
    let dispatched = json(&call::<String>(&client, "Dispatch", ("Super+Q",)).unwrap());
    assert_eq!(dispatched["kind"], "system");

    // No focus: nothing to dispatch (a non-reserved chord).
    call::<bool>(&client, "SetFocusedApp", ("",)).unwrap();
    let dispatched = json(&call::<String>(&client, "Dispatch", ("Super+W",)).unwrap());
    assert_eq!(dispatched["kind"], "none");
}

#[test]
fn a_malformed_publish_is_rejected_and_withdraw_counts() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    assert!(!call::<bool>(&client, "Publish", ("bad", "{not json")).unwrap());
    assert_eq!(call::<u32>(&client, "PublisherCount", ()).unwrap(), 0);

    assert!(call::<bool>(&client, "Publish", ("a", SETTINGS_FIXTURE)).unwrap());
    assert_eq!(call::<u32>(&client, "PublisherCount", ()).unwrap(), 1);
    assert!(call::<bool>(&client, "Withdraw", ("a",)).unwrap());
    assert_eq!(call::<u32>(&client, "PublisherCount", ()).unwrap(), 0);
    // The revision advanced on every accepted mutation.
    assert!(call::<u64>(&client, "Revision", ()).unwrap() >= 2);
}
