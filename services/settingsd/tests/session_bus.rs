// SPDX-License-Identifier: MIT
//! T-08.1a acceptance: drive `org.dragonfruit.Settings1` over a real session
//! bus.
//!
//! Each test stands up a private `dbus-daemon` session bus, serves the
//! settings interface on it, and talks to it through a second connection —
//! the same shape a shell or compositor client will use. Nothing links the
//! store directly: every assertion goes through the D-Bus surface (`Get`,
//! `Set`, `GetAll`, `ListKeys`, the `SchemaVersion` property, and the
//! `Changed` signal).

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use dragonfruit_settingsd::dbus::{Settings1, DBUS_NAME, DBUS_PATH};
use dragonfruit_settingsd::schema::{KeySpec, KeyType, KEYS, SCHEMA_VERSION};
use dragonfruit_settingsd::value::Value;
use dragonfruit_settingsd::Settings;
use zbus::blocking::{Connection, MessageIterator};
use zbus::message::Type as MessageType;
use zbus::zvariant::OwnedValue;
use zbus::MatchRule;

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
            .expect("dbus-daemon is required to run the T-08 settings integration test");
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

/// Serve the settings interface on `bus` and return the holding connection
/// (dropping it unregisters the name).
fn serve(bus: &PrivateBus) -> Connection {
    zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .serve_at(DBUS_PATH, Settings1::new(Settings::new()))
        .expect("serve the settings interface")
        .build()
        .expect("build the service connection")
}

/// Call a method and deserialize its reply body.
fn call<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
    client: &Connection,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error> {
    let reply = client.call_method(Some(DBUS_NAME), DBUS_PATH, Some(DBUS_NAME), method, &body)?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

/// The typed D-Bus error name a failed call returned, if it was a method
/// error.
fn error_name(error: &zbus::Error) -> Option<String> {
    match error {
        zbus::Error::MethodError(name, _, _) => Some(name.to_string()),
        _ => None,
    }
}

fn get(client: &Connection, key: &str) -> Result<Value, zbus::Error> {
    let owned: OwnedValue = call(client, "Get", key)?;
    Ok(Value::from_owned_value(&owned).expect("a known key returns a supported value kind"))
}

fn set(client: &Connection, key: &str, value: &Value) -> Result<(), zbus::Error> {
    client.call_method(
        Some(DBUS_NAME),
        DBUS_PATH,
        Some(DBUS_NAME),
        "Set",
        &(key, value.to_owned_value()),
    )?;
    Ok(())
}

/// A valid value that differs from the key's default, for the round-trip
/// sweep. Picks within the declared range/enumeration so the write is legal.
fn alternative_for(spec: &KeySpec) -> Value {
    let default = spec.default.to_value();
    match spec.kind {
        KeyType::Bool => Value::Bool(!matches!(default, Value::Bool(true))),
        KeyType::Number => {
            let Value::Number(default) = default else {
                unreachable!()
            };
            let min = spec.min.unwrap_or(default - 1.0);
            let max = spec.max.unwrap_or(default + 1.0);
            let candidate = if default <= (min + max) / 2.0 {
                (max + default) / 2.0
            } else {
                (min + default) / 2.0
            };
            Value::Number(candidate)
        }
        KeyType::Integer => {
            let Value::Integer(default) = default else {
                unreachable!()
            };
            let max = spec.max.unwrap_or(default as f64 + 1.0);
            let candidate = if (default as f64) < max {
                default + 1
            } else {
                default - 1
            };
            Value::Integer(candidate)
        }
        KeyType::Text if !spec.allowed.is_empty() => {
            let Value::Text(default) = default else {
                unreachable!()
            };
            Value::Text(
                spec.allowed
                    .iter()
                    .find(|candidate| **candidate != default.as_str())
                    .expect("an enum key has at least two alternatives")
                    .to_string(),
            )
        }
        KeyType::Text => Value::Text(format!("#{:06x}", 0x00_ff_88)),
        KeyType::TextList => Value::TextList(vec!["a.desktop".to_owned(), "b.desktop".to_owned()]),
    }
}

#[test]
fn every_key_round_trips_get_set_and_change_over_the_bus() {
    let bus = PrivateBus::start();
    let service = serve(&bus);
    let client = bus.connect();

    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(DBUS_NAME)
        .expect("valid sender")
        .interface(DBUS_NAME)
        .expect("valid interface")
        .member("Changed")
        .expect("valid member")
        .build();
    let mut signals =
        MessageIterator::for_match_rule(rule, &client, Some(64)).expect("subscribe to Changed");

    let (tx, rx) = mpsc::channel::<(String, Value)>();
    let _collector = std::thread::spawn(move || {
        while let Some(Ok(message)) = signals.next() {
            if let Ok((key, value)) = message.body().deserialize::<(String, OwnedValue)>() {
                if let Some(value) = Value::from_owned_value(&value) {
                    if tx.send((key, value)).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Every key defaults correctly, then accepts a changed value.
    let mut expected = HashMap::new();
    for spec in KEYS {
        assert_eq!(
            get(&client, spec.key).unwrap(),
            spec.default.to_value(),
            "{} did not start at its default",
            spec.key
        );
        let wanted = alternative_for(spec);
        set(&client, spec.key, &wanted)
            .unwrap_or_else(|error| panic!("Set({}) failed: {error}", spec.key));
        assert_eq!(
            get(&client, spec.key).unwrap(),
            wanted,
            "{} did not round-trip through the bus",
            spec.key
        );
        expected.insert(spec.key.to_owned(), wanted);
    }

    // Every change produced exactly the signal the write implied.
    let mut seen = HashMap::new();
    while seen.len() < expected.len() {
        let (key, value) = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("a Changed signal for every real write");
        seen.insert(key, value);
    }
    assert_eq!(seen, expected);

    drop(service);
}

#[test]
fn the_snapshot_keys_and_schema_revision_are_exposed() {
    let bus = PrivateBus::start();
    let service = serve(&bus);
    let client = bus.connect();

    let all: HashMap<String, OwnedValue> = call(&client, "GetAll", ()).expect("GetAll succeeds");
    assert_eq!(all.len(), KEYS.len());
    assert_eq!(
        Value::from_owned_value(all.get("appearance.colorScheme").unwrap()).unwrap(),
        Value::Text("auto".to_owned())
    );

    let keys: Vec<String> = call(&client, "ListKeys", ()).expect("ListKeys succeeds");
    assert_eq!(keys, dragonfruit_settingsd::schema::key_names());

    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(DBUS_NAME, "SchemaVersion"),
        )
        .expect("read SchemaVersion");
    let value: OwnedValue = reply.body().deserialize().expect("property body decodes");
    let version = u32::try_from(value).expect("SchemaVersion is a u32");
    assert_eq!(version, SCHEMA_VERSION);

    drop(service);
}

#[test]
fn invalid_writes_are_typed_errors_and_leave_the_store_unchanged() {
    let bus = PrivateBus::start();
    let service = serve(&bus);
    let client = bus.connect();

    let unknown: Result<OwnedValue, _> = call(&client, "Get", "dock.nope");
    assert_eq!(
        error_name(&unknown.unwrap_err()).as_deref(),
        Some("org.dragonfruit.Settings1.Error.UnknownKey")
    );

    let mismatch = set(&client, "dock.autohide", &Value::Text("yes".into()));
    assert_eq!(
        error_name(&mismatch.unwrap_err()).as_deref(),
        Some("org.dragonfruit.Settings1.Error.TypeMismatch")
    );

    let range = set(&client, "dock.size", &Value::Number(2.0));
    assert_eq!(
        error_name(&range.unwrap_err()).as_deref(),
        Some("org.dragonfruit.Settings1.Error.OutOfRange")
    );

    let allowed = set(&client, "dock.position", &Value::Text("top".into()));
    assert_eq!(
        error_name(&allowed.unwrap_err()).as_deref(),
        Some("org.dragonfruit.Settings1.Error.NotAllowed")
    );

    // A rejected write never touched the store.
    assert_eq!(get(&client, "dock.autohide").unwrap(), Value::Bool(false));
    assert_eq!(get(&client, "dock.size").unwrap(), Value::Number(0.5));
    assert_eq!(
        get(&client, "dock.position").unwrap(),
        Value::Text("bottom".to_owned())
    );

    drop(service);
}
