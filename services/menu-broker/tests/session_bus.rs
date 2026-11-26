// SPDX-License-Identifier: GPL-3.0-or-later
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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dragonfruit_menu_broker::dbus::{
    spawn_owner_cleaner, MenuBroker1, DBUS_NAME, DBUS_PATH, INTERFACE, PUBLISHER_PATH,
    PUBLISHER_TIMEOUT,
};
use zbus::blocking::Connection;
use zbus::interface;

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
    let connection = zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .method_timeout(PUBLISHER_TIMEOUT)
        .serve_at(DBUS_PATH, object.clone())
        .expect("serve the menu broker interface")
        .build()
        .expect("build the service connection");
    spawn_owner_cleaner(connection.clone(), object);
    connection
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
    assert_eq!(app_menu.len(), 9);
    assert_eq!(app_menu[0]["label"], "About Settings");
    assert_eq!(app_menu[2]["label"], "Services");
    assert_eq!(app_menu[4]["label"], "Hide Settings");
    // Focused + visible => Hide and Hide Others on, Show All off.
    assert_eq!(app_menu[4]["enabled"], true);
    assert_eq!(app_menu[5]["enabled"], false);
    assert_eq!(app_menu[6]["enabled"], false);

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
    assert_eq!(app_menu[4]["enabled"], true); // Hide a
    assert_eq!(app_menu[5]["enabled"], true); // Hide Others
    assert_eq!(app_menu[6]["enabled"], false); // Show All

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
    assert_eq!(app_menu[4]["enabled"], false);
    assert_eq!(app_menu[5]["enabled"], false);
    assert_eq!(app_menu[6]["enabled"], true);
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
    assert_eq!(app_menu[8]["label"], "Quit Files");
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
fn a_dbusmenu_bridge_publish_resolves_at_the_dbusmenu_tier() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    let bridge_model = r#"{ "appName": "Konsole",
        "applicationMenuItems": [],
        "menus": [ { "title": "File", "items": [
            { "label": "New", "enabled": true, "shortcut": "Ctrl+N", "action": "dbusmenu:1" },
            { "label": "Quit", "enabled": false, "action": "dbusmenu:2" } ] } ] }"#;
    assert!(call::<bool>(
        &client,
        "PublishDbusMenu",
        ("org.example.Konsole", bridge_model),
    )
    .unwrap());

    call::<bool>(&client, "SetFocusedApp", ("org.example.Konsole",)).unwrap();
    let resolved = json(&call::<String>(&client, "ResolveFocused", ()).unwrap());
    assert_eq!(resolved["tier"], "dbusmenu");
    assert_eq!(resolved["appName"], "Konsole");
    assert_eq!(resolved["menus"][0]["items"][0]["enabled"], true);
    assert_eq!(resolved["menus"][0]["items"][1]["enabled"], false);

    // The bridge-born accelerator is focus-scoped like any other, alongside
    // the synthesized fixed menu's chords.
    let accelerators = json(&call::<String>(&client, "FocusedAccelerators", ()).unwrap());
    let bridge = accelerators
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["action"] == "dbusmenu:1")
        .expect("the bridge accelerator is present");
    assert_eq!(bridge["chord"], "Control+N");
    let dispatched = json(&call::<String>(&client, "Dispatch", ("Ctrl+N",)).unwrap());
    assert_eq!(dispatched["kind"], "application");
    assert_eq!(dispatched["action"], "dbusmenu:1");

    // A malformed bridge payload is rejected without clobbering the model.
    assert!(!call::<bool>(
        &client,
        "PublishDbusMenu",
        ("org.example.Konsole", "{not json"),
    )
    .unwrap());
    assert_eq!(
        json(&call::<String>(&client, "ResolveFocused", ()).unwrap())["tier"],
        "dbusmenu"
    );
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

/// One activation the broker forwarded to the fake publisher.
#[derive(Debug, Clone, PartialEq)]
struct Delivered {
    app_id: String,
    action: String,
    item_json: String,
    generation: u64,
    window_id: u32,
}

/// The publisher-side endpoint the broker calls at the unique bus name that
/// called `Publish`.
#[derive(Clone, Default)]
struct FakePublisher {
    delivered: Arc<Mutex<Vec<Delivered>>>,
}

#[interface(name = "org.dragonfruit.MenuPublisher1")]
impl FakePublisher {
    fn activate(
        &self,
        app_id: &str,
        action: &str,
        item_json: &str,
        generation: u64,
        window_id: u32,
    ) -> bool {
        self.delivered.lock().unwrap().push(Delivered {
            app_id: app_id.to_owned(),
            action: action.to_owned(),
            item_json: item_json.to_owned(),
            generation,
            window_id,
        });
        // `help` models an app that rejects a command it knows but cannot
        // perform in its current state.
        action != "help"
    }
}

#[test]
fn an_activation_is_delivered_to_the_publishers_registered_owner() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    let publisher = FakePublisher::default();
    let publisher_connection = bus.connect();
    publisher_connection
        .object_server()
        .at(PUBLISHER_PATH, publisher.clone())
        .expect("serve the publisher activation endpoint");

    let reply = publisher_connection
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some(INTERFACE),
            "Publish",
            &("org.dragonfruit.Settings", SETTINGS_FIXTURE),
        )
        .unwrap();
    assert!(reply.body().deserialize::<bool>().unwrap());

    let resolved =
        json(&call::<String>(&client, "Resolve", ("org.dragonfruit.Settings",)).unwrap());
    let generation = resolved["generation"].as_u64().unwrap();
    assert!(generation >= 1);

    let result = json(
        &call::<String>(
            &client,
            "Activate",
            (
                "org.dragonfruit.Settings",
                "edit.undo",
                r#"{"action":"edit.undo","enabled":true}"#,
                generation,
                7u32,
            ),
        )
        .unwrap(),
    );
    assert_eq!(result["ok"], true);
    let delivered = publisher.delivered.lock().unwrap().clone();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].app_id, "org.dragonfruit.Settings");
    assert_eq!(delivered[0].action, "edit.undo");
    assert_eq!(delivered[0].generation, generation);
    assert_eq!(delivered[0].window_id, 7);
    assert!(delivered[0].item_json.contains("edit.undo"));

    // A stale generation is rejected before it reaches the app.
    let result = json(
        &call::<String>(
            &client,
            "Activate",
            (
                "org.dragonfruit.Settings",
                "edit.undo",
                r#"{"action":"edit.undo","enabled":true}"#,
                generation + 1,
                7u32,
            ),
        )
        .unwrap(),
    );
    assert_eq!(result["ok"], false);
    assert_eq!(result["reason"], "stale-generation");
    assert_eq!(publisher.delivered.lock().unwrap().len(), 1);

    // The publisher's own rejection is surfaced, not converted to success.
    let result = json(
        &call::<String>(
            &client,
            "Activate",
            (
                "org.dragonfruit.Settings",
                "help",
                r#"{"action":"help","enabled":true}"#,
                generation,
                7u32,
            ),
        )
        .unwrap(),
    );
    assert_eq!(result["ok"], false);
    assert_eq!(result["reason"], "rejected");
}

#[test]
fn a_disconnected_publisher_withdraws_its_model_and_endpoint() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);
    let client = bus.connect();

    let publisher = FakePublisher::default();
    let publisher_connection = bus.connect();
    publisher_connection
        .object_server()
        .at(PUBLISHER_PATH, publisher)
        .expect("serve the publisher activation endpoint");
    let reply = publisher_connection
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some(INTERFACE),
            "Publish",
            &("org.dragonfruit.Settings", SETTINGS_FIXTURE),
        )
        .unwrap();
    assert!(reply.body().deserialize::<bool>().unwrap());
    assert_eq!(call::<u32>(&client, "PublisherCount", ()).unwrap(), 1);

    drop(publisher_connection);

    // The owner cleaner drops the crashed publisher's model without waiting
    // for a Withdraw call.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if call::<u32>(&client, "PublisherCount", ()).unwrap() == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(call::<u32>(&client, "PublisherCount", ()).unwrap(), 0);
    let resolved =
        json(&call::<String>(&client, "Resolve", ("org.dragonfruit.Settings",)).unwrap());
    assert_eq!(resolved["tier"], "none");
    assert_eq!(resolved["generation"], 0);
}
