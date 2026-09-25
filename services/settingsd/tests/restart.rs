// SPDX-License-Identifier: MIT
//! T-08.3 acceptance: `settingsd` survives a hard kill and re-syncs.
//!
//! Unlike `session_bus.rs` (which serves an in-process object and models a
//! restart by reloading a `Persistence`), this test drives the **real
//! `dragonfruit-settingsd` binary** over a private session bus and `SIGKILL`s
//! it. It proves the three restart guarantees the T-08 track promises:
//!
//! 1. a `Set` that returned has already been persisted, so `kill -9` loses
//!    no setting;
//! 2. the daemon's well-known name is released on death;
//! 3. a client that reappears calls `GetAll` and re-syncs from the durable
//!    file — including a value changed on disk while the daemon was down,
//!    which is therefore not silently dropped.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use dragonfruit_settingsd::dbus::{DBUS_NAME, DBUS_PATH};
use dragonfruit_settingsd::value::Value;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedValue;

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
            .expect("dbus-daemon is required to run the T-08 restart test");
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

/// A scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dragonfruit-settingsd-restart-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path)
    }

    fn settings_file(&self) -> PathBuf {
        self.0.join("dragonfruit").join("settings.json")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Whether `name` currently has an owner on the bus.
fn has_owner(client: &Connection, name: &str) -> bool {
    client
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &(name,),
        )
        .ok()
        .and_then(|reply| reply.body().deserialize::<bool>().ok())
        .unwrap_or(false)
}

fn wait_for_owner(client: &Connection, name: &str, present: bool) {
    for _ in 0..400 {
        if has_owner(client, name) == present {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!(
        "the bus never reported {name} as {}",
        if present { "owned" } else { "released" }
    );
}

/// Spawn the real daemon against the private bus and a scratch config home.
fn spawn_daemon(bus: &PrivateBus, scratch: &Scratch) -> Child {
    let child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-settingsd"))
        .env("DBUS_SESSION_BUS_ADDRESS", &bus.address)
        .env("XDG_CONFIG_HOME", &scratch.0)
        .env("HOME", &scratch.0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the settingsd binary runs");
    let watcher = bus.connect();
    wait_for_owner(&watcher, DBUS_NAME, true);
    child
}

fn call<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
    client: &Connection,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error> {
    let reply = client.call_method(Some(DBUS_NAME), DBUS_PATH, Some(DBUS_NAME), method, &body)?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

fn get(client: &Connection, key: &str) -> Value {
    let owned: OwnedValue = call(client, "Get", key).expect("Get succeeds");
    Value::from_owned_value(&owned).expect("a known key returns a supported value kind")
}

fn set(client: &Connection, key: &str, value: &Value) {
    client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some(DBUS_NAME),
            "Set",
            &(key, value.to_owned_value()),
        )
        .unwrap_or_else(|error| panic!("Set({key}) failed: {error}"));
}

#[test]
fn kill_9_then_restart_loses_no_setting_and_resyncs_from_disk() {
    let bus = PrivateBus::start();
    let scratch = Scratch::new();
    let path = scratch.settings_file();
    let client = bus.connect();

    // --- first daemon: set two keys; every Set returned, so both are durable.
    let mut daemon = spawn_daemon(&bus, &scratch);
    set(&client, "dock.autohide", &Value::Bool(true));
    set(
        &client,
        "appearance.colorScheme",
        &Value::Text("dark".into()),
    );

    let on_disk: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("a Set wrote the file"))
            .expect("the file is JSON");
    assert_eq!(
        on_disk["keys"]["dock.autohide"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        on_disk["keys"]["appearance.colorScheme"],
        serde_json::Value::String("dark".into())
    );

    // --- kill -9: the process dies without unwinding; the name is released.
    daemon.kill().expect("SIGKILL the daemon");
    daemon.wait().expect("reap the daemon");
    wait_for_owner(&client, DBUS_NAME, false);

    // A change made while the daemon is down, written straight to the durable
    // file it owns (the interim/external shape). The rest of the file is
    // untouched, so the two writes above must survive.
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the file survives the kill"))
            .expect("the file is JSON");
    document["keys"]["dock.size"] = serde_json::Value::from(0.9);
    std::fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();

    // --- restart: a fresh process over the same config home.
    let mut restarted = spawn_daemon(&bus, &scratch);

    // Every write that had returned before the kill is still there...
    assert_eq!(get(&client, "dock.autohide"), Value::Bool(true));
    assert_eq!(
        get(&client, "appearance.colorScheme"),
        Value::Text("dark".into())
    );
    // ...and the client's reappearance snapshot carries the while-down change.
    let snapshot: std::collections::HashMap<String, OwnedValue> =
        call(&client, "GetAll", ()).expect("GetAll succeeds");
    let size = Value::from_owned_value(snapshot.get("dock.size").expect("dock.size present"))
        .expect("dock.size is a supported value");
    assert_eq!(size, Value::Number(0.9));

    // The restarted daemon still serves the full schema.
    let keys: Vec<String> = call(&client, "ListKeys", ()).expect("ListKeys succeeds");
    assert!(keys.contains(&"dock.size".to_owned()));

    let _ = restarted.kill();
    let _ = restarted.wait();
}
