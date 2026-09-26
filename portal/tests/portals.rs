// SPDX-License-Identifier: MIT
//! T-13.1b acceptance: the Settings and GlobalShortcuts portals answer a
//! real D-Bus client.
//!
//! Every test stands up a private `dbus-daemon` session bus and starts the
//! backend with [`xdg_desktop_portal_dragonfruit::dbus::serve`] — the same
//! code path the daemon uses. The Settings tests also serve a real
//! `org.dragonfruit.Settings1` object so the projection is checked against
//! the single settings owner rather than a fixture. Nothing links the
//! models directly: every assertion goes through D-Bus.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use xdg_desktop_portal_dragonfruit::interfaces::{
    FILE_CHOOSER_INTERFACE, SCREENCAST_INTERFACE, SCREENSHOT_INTERFACE, SETTINGS_INTERFACE,
};
use xdg_desktop_portal_dragonfruit::model::{DBUS_NAME, DBUS_PATH, STATUS_INTERFACE};
use xdg_desktop_portal_dragonfruit::shortcuts::GLOBAL_SHORTCUTS_INTERFACE;
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::MatchRule;

/// `a(sa{sv})`: the shortcut wire shape the portal binds and returns.
type WireShortcut = (String, HashMap<String, OwnedValue>);
type WireShortcuts = Vec<WireShortcut>;

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
            .expect("dbus-daemon is required to run the T-13.1b integration test");
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

    /// Start the backend on this bus; the returned connection must outlive
    /// the test.
    fn serve(&self) -> (Connection, xdg_desktop_portal_dragonfruit::Backend) {
        xdg_desktop_portal_dragonfruit::dbus::serve(Some(self.address.as_str()))
            .expect("the backend serves on a private bus")
    }

    /// Serve a real settingsd object and own `org.dragonfruit.Settings1`.
    fn serve_settingsd(&self) -> Connection {
        use dragonfruit_settingsd::dbus::{
            Settings1, DBUS_NAME as SETTINGS_NAME, DBUS_PATH as SETTINGS_PATH,
        };
        use dragonfruit_settingsd::Settings;

        zbus::blocking::connection::Builder::address(self.address.as_str())
            .expect("valid bus address")
            .name(SETTINGS_NAME)
            .expect("valid settings name")
            .serve_at(SETTINGS_PATH, Settings1::new(Settings::new()))
            .expect("serve the settings object")
            .build()
            .expect("build the settings service connection")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Call a method on the standard backend object and decode its reply.
fn call<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
    client: &Connection,
    interface: &str,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error> {
    let reply = client.call_method(Some(DBUS_NAME), DBUS_PATH, Some(interface), method, &body)?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

/// Read one Settings key as a variant.
fn settings_read(
    client: &Connection,
    namespace: &str,
    key: &str,
) -> Result<OwnedValue, zbus::Error> {
    call(client, SETTINGS_INTERFACE, "Read", (namespace, key))
}

/// Read one Settings namespace as `a{sa{sv}}`.
fn settings_read_all(
    client: &Connection,
    namespace: &str,
) -> HashMap<String, HashMap<String, OwnedValue>> {
    call::<HashMap<String, HashMap<String, OwnedValue>>>(
        client,
        SETTINGS_INTERFACE,
        "ReadAll",
        namespace,
    )
    .expect("ReadAll succeeds")
}

/// Read a `u` property from a portal interface.
fn version(client: &Connection, interface: &str) -> u32 {
    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(interface, "Version"),
        )
        .expect("read Version");
    let value: OwnedValue = reply.body().deserialize().expect("property body decodes");
    u32::try_from(value).expect("Version is a u32")
}

/// Set a settingsd key through the real owner.
fn settingsd_set(client: &Connection, key: &str, value: OwnedValue) {
    let reply = client.call_method(
        Some("org.dragonfruit.Settings1"),
        "/org/dragonfruit/Settings1",
        Some("org.dragonfruit.Settings1"),
        "Set",
        &(key, value),
    );
    reply.expect("settingsd Set succeeds");
}

/// Poll `check` until it returns `Some`, or panic after `timeout`.
fn wait_for<T>(timeout: Duration, mut check: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = check() {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the expected state"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Subscribe to a signal at `path` on `interface` and stream its body.
fn signal_stream_at<T>(
    client: &Connection,
    path: &str,
    interface: &str,
    member: &str,
) -> mpsc::Receiver<T>
where
    T: serde::de::DeserializeOwned + zbus::zvariant::Type + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let connection = client.clone();
    let path = path.to_owned();
    let interface = interface.to_owned();
    let member = member.to_owned();
    std::thread::spawn(move || {
        let rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .path(path.as_str())
            .expect("valid path")
            .interface(interface.as_str())
            .expect("valid interface")
            .member(member.as_str())
            .expect("valid member")
            .build();
        if let Ok(iterator) = MessageIterator::for_match_rule(rule, &connection, Some(16)) {
            for message in iterator {
                let Ok(message) = message else { break };
                if let Ok(body) = message.body().deserialize::<T>() {
                    if sender.send(body).is_err() {
                        break;
                    }
                }
            }
        }
    });
    // Give the spawned thread a moment to register the match rule before the
    // caller triggers the signal.
    std::thread::sleep(Duration::from_millis(150));
    receiver
}

/// Subscribe to a signal on the standard backend object.
fn signal_stream<T>(client: &Connection, interface: &str, member: &str) -> mpsc::Receiver<T>
where
    T: serde::de::DeserializeOwned + zbus::zvariant::Type + Send + 'static,
{
    signal_stream_at(client, DBUS_PATH, interface, member)
}

#[test]
fn the_standard_interfaces_are_served_at_the_standard_path() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let introspection: String = {
        let reply = client
            .call_method(
                Some(DBUS_NAME),
                DBUS_PATH,
                Some("org.freedesktop.DBus.Introspectable"),
                "Introspect",
                &(),
            )
            .expect("Introspect call");
        reply.body().deserialize().expect("introspection XML")
    };
    assert!(introspection.contains(STATUS_INTERFACE));
    assert!(introspection.contains(SETTINGS_INTERFACE));
    assert!(introspection.contains(GLOBAL_SHORTCUTS_INTERFACE));
    assert!(introspection.contains(FILE_CHOOSER_INTERFACE));
    assert!(introspection.contains(SCREENSHOT_INTERFACE));
    assert!(introspection.contains(SCREENCAST_INTERFACE));
}

#[test]
fn the_settings_portal_answers_appearance_and_desktop_reads() {
    let bus = PrivateBus::start();
    let _settingsd = bus.serve_settingsd();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    // The appearance namespace answers immediately, even before the sync
    // thread has spoken to settingsd: the defaults are honest.
    assert_eq!(version(&client, SETTINGS_INTERFACE), 2);
    let scheme: OwnedValue =
        settings_read(&client, "org.freedesktop.appearance", "color-scheme").expect("Read");
    assert_eq!(u32::try_from(scheme).unwrap(), 0, "auto = no preference");
    let contrast: OwnedValue =
        settings_read(&client, "org.freedesktop.appearance", "contrast").expect("Read");
    assert_eq!(u32::try_from(contrast).unwrap(), 0);

    // An absent key is a typed error, not a silent empty value.
    assert!(settings_read(&client, "org.freedesktop.appearance", "nope").is_err());
    assert!(settings_read(&client, "org.example.absent", "whatever").is_err());

    // Both namespaces are present in a ReadAll.
    let all = settings_read_all(&client, "");
    assert!(all.contains_key("org.freedesktop.appearance"));
    let appearance = settings_read_all(&client, "org.freedesktop.appearance");
    let keys = appearance
        .get("org.freedesktop.appearance")
        .expect("appearance namespace");
    assert!(keys.contains_key("color-scheme"));
    assert!(keys.contains_key("contrast"));

    // Change the owner; the portal follows live and emits SettingChanged.
    let changes: mpsc::Receiver<(String, String, OwnedValue)> =
        signal_stream(&client, SETTINGS_INTERFACE, "SettingChanged");
    settingsd_set(
        &client,
        "appearance.colorScheme",
        OwnedValue::from(zbus::zvariant::Str::from("dark")),
    );

    let scheme: OwnedValue = wait_for(Duration::from_secs(5), || {
        settings_read(&client, "org.freedesktop.appearance", "color-scheme")
            .ok()
            .filter(|value| u32::try_from(value.try_clone().unwrap()).unwrap_or(0) == 1)
    });
    assert_eq!(u32::try_from(scheme).unwrap(), 1, "dark = prefer dark");

    // The desktop namespace carries the raw settingsd key too.
    let raw: OwnedValue =
        settings_read(&client, "org.dragonfruit.desktop", "appearance.colorScheme")
            .expect("desktop key");
    assert_eq!(String::try_from(raw).unwrap(), "dark");

    // The change is announced for both the raw desktop key and the derived
    // appearance key; find the appearance one.
    let (namespace, key, value) = wait_for(Duration::from_secs(5), || {
        changes
            .recv_timeout(Duration::from_secs(5))
            .ok()
            .filter(|(namespace, _, _)| namespace == "org.freedesktop.appearance")
    });
    assert_eq!(namespace, "org.freedesktop.appearance");
    assert_eq!(key, "color-scheme");
    assert_eq!(u32::try_from(value).unwrap(), 1);
}

#[test]
fn the_globalshortcuts_portal_creates_binds_lists_and_closes_sessions() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    assert_eq!(version(&client, GLOBAL_SHORTCUTS_INTERFACE), 1);

    let session_handle = "/org/freedesktop/portal/desktop/session/1_1/df_test";
    let request_handle = "/org/freedesktop/portal/desktop/request/1_1/df_test";

    // CreateSession answers with the session path.
    let (response, results): (u32, HashMap<String, OwnedValue>) = call(
        &client,
        GLOBAL_SHORTCUTS_INTERFACE,
        "CreateSession",
        (
            request_handle,
            session_handle,
            "org.example.App",
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .expect("CreateSession");
    assert_eq!(response, 0, "success");
    let returned = results
        .get("session_handle")
        .expect("session_handle result");
    let returned =
        OwnedObjectPath::try_from(returned.try_clone().unwrap()).expect("an object path");
    assert_eq!(returned.as_str(), session_handle);

    // The session object exists and implements the standard Session contract.
    let session_introspection: String = {
        let reply = client
            .call_method(
                Some(DBUS_NAME),
                session_handle,
                Some("org.freedesktop.DBus.Introspectable"),
                "Introspect",
                &(),
            )
            .expect("Introspect the session");
        reply
            .body()
            .deserialize()
            .expect("session introspection XML")
    };
    assert!(session_introspection.contains("org.freedesktop.impl.portal.Session"));

    // A duplicate session path is refused.
    let duplicate: Result<(u32, HashMap<String, OwnedValue>), _> = call(
        &client,
        GLOBAL_SHORTCUTS_INTERFACE,
        "CreateSession",
        (
            request_handle,
            session_handle,
            "org.example.Other",
            HashMap::<String, OwnedValue>::new(),
        ),
    );
    assert!(duplicate.is_err(), "a duplicate session path is refused");

    // BindShortcuts echoes the bound shortcuts and emits ShortcutsChanged.
    let changed: mpsc::Receiver<(OwnedObjectPath, WireShortcuts)> =
        signal_stream(&client, GLOBAL_SHORTCUTS_INTERFACE, "ShortcutsChanged");
    let mut options = HashMap::new();
    options.insert(
        "description".to_owned(),
        OwnedValue::from(zbus::zvariant::Str::from("Play/Pause")),
    );
    let shortcuts = vec![("play".to_owned(), options)];
    let (response, bound): (u32, WireShortcuts) = call(
        &client,
        GLOBAL_SHORTCUTS_INTERFACE,
        "BindShortcuts",
        (
            request_handle,
            session_handle,
            shortcuts.clone(),
            "",
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .expect("BindShortcuts");
    assert_eq!(response, 0);
    assert_eq!(bound, shortcuts);

    let (signal_session, signal_shortcuts) = changed
        .recv_timeout(Duration::from_secs(5))
        .expect("ShortcutsChanged arrives");
    assert_eq!(signal_session.as_str(), session_handle);
    assert_eq!(signal_shortcuts, shortcuts);

    // ListShortcuts returns what was bound.
    let (response, listed): (u32, WireShortcuts) = call(
        &client,
        GLOBAL_SHORTCUTS_INTERFACE,
        "ListShortcuts",
        (
            request_handle,
            session_handle,
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .expect("ListShortcuts");
    assert_eq!(response, 0);
    assert_eq!(listed, shortcuts);

    // The diagnostic bridge raises the standard Activated signal and refuses
    // an unknown shortcut.
    let activated: mpsc::Receiver<(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>)> =
        signal_stream(&client, GLOBAL_SHORTCUTS_INTERFACE, "Activated");
    let delivered: bool = call(
        &client,
        STATUS_INTERFACE,
        "ActivateShortcut",
        (session_handle, "play", 42u64),
    )
    .expect("ActivateShortcut");
    assert!(delivered);
    let (signal_session, shortcut_id, timestamp, _) = activated
        .recv_timeout(Duration::from_secs(5))
        .expect("Activated arrives");
    assert_eq!(signal_session.as_str(), session_handle);
    assert_eq!(shortcut_id, "play");
    assert_eq!(timestamp, 42);

    let unknown: bool = call(
        &client,
        STATUS_INTERFACE,
        "ActivateShortcut",
        (session_handle, "missing", 43u64),
    )
    .expect("ActivateShortcut");
    assert!(!unknown, "an unbound shortcut is not delivered");

    // Close the session: the object disappears and the registry forgets it.
    let closed: mpsc::Receiver<u32> = signal_stream_at(
        &client,
        session_handle,
        "org.freedesktop.impl.portal.Session",
        "Closed",
    );
    client
        .call_method(
            Some(DBUS_NAME),
            session_handle,
            Some("org.freedesktop.impl.portal.Session"),
            "Close",
            &(),
        )
        .expect("Close");
    assert!(closed.recv_timeout(Duration::from_secs(5)).is_ok());

    let listed_after: Result<(u32, WireShortcuts), _> = call(
        &client,
        GLOBAL_SHORTCUTS_INTERFACE,
        "ListShortcuts",
        (
            request_handle,
            session_handle,
            HashMap::<String, OwnedValue>::new(),
        ),
    );
    assert!(listed_after.is_err(), "the closed session is gone");
}
