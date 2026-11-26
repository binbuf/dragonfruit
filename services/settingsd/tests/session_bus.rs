// SPDX-License-Identifier: GPL-3.0-or-later
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
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use dragonfruit_settingsd::dbus::{Settings1, DBUS_NAME, DBUS_PATH};
use dragonfruit_settingsd::persist;
use dragonfruit_settingsd::schema::{KeySemantic, KeySpec, KeyType, KEYS, SCHEMA_VERSION};
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

/// Serve a settings object on `bus` and return the holding connection
/// (dropping it unregisters the name).
fn serve_object(bus: &PrivateBus, object: Settings1) -> Connection {
    zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .serve_at(DBUS_PATH, object)
        .expect("serve the settings interface")
        .build()
        .expect("build the service connection")
}

/// Serve a fresh in-memory settings object on `bus`.
fn serve(bus: &PrivateBus) -> Connection {
    serve_object(bus, Settings1::new(Settings::new()))
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
            "dragonfruit-settingsd-bus-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path)
    }

    fn file(&self) -> PathBuf {
        self.0.join("dragonfruit").join("settings.json")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
        KeyType::Text => match spec.semantic {
            KeySemantic::TimeOfDay => Value::Text("12:34".to_owned()),
            KeySemantic::Color => Value::Text("#00ff88".to_owned()),
            KeySemantic::AbsolutePath => Value::Text("/tmp/dragonfruit-test".to_owned()),
            KeySemantic::Shortcut(_) => Value::Text("Ctrl+Alt+T".to_owned()),
            KeySemantic::BoundedText(_) => Value::Text("Test message".to_owned()),
            _ => Value::Text(format!("#{:06x}", 0x00_ff_88)),
        },
        KeyType::TextList => match spec.semantic {
            KeySemantic::ModuleList => {
                Value::TextList(vec!["wifi".to_owned(), "volume".to_owned()])
            }
            KeySemantic::DayList => Value::TextList(vec!["mon".to_owned(), "fri".to_owned()]),
            KeySemantic::LayoutList => Value::TextList(vec!["us".to_owned(), "de".to_owned()]),
            KeySemantic::ReplacementList => Value::TextList(vec!["teh\tthe".to_owned()]),
            _ => Value::TextList(vec!["a.desktop".to_owned(), "b.desktop".to_owned()]),
        },
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

#[test]
fn the_glass_keys_round_trip_and_reject_invalid_writes_over_the_bus() {
    let bus = PrivateBus::start();
    let service = serve(&bus);
    let client = bus.connect();

    // The default is the measured midpoint; both overrides are off.
    assert_eq!(
        get(&client, "appearance.transparency").unwrap(),
        Value::Number(0.5255)
    );
    assert_eq!(
        get(&client, "accessibility.reduceTransparency").unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        get(&client, "accessibility.increaseContrast").unwrap(),
        Value::Bool(false)
    );

    // A valid write is reflected by the owner.
    set(&client, "appearance.transparency", &Value::Number(0.8)).expect("transparency accepts 0.8");
    assert_eq!(
        get(&client, "appearance.transparency").unwrap(),
        Value::Number(0.8)
    );
    set(
        &client,
        "accessibility.reduceTransparency",
        &Value::Bool(true),
    )
    .expect("reduceTransparency accepts true");
    set(
        &client,
        "accessibility.increaseContrast",
        &Value::Bool(true),
    )
    .expect("increaseContrast accepts true");
    assert_eq!(
        get(&client, "accessibility.reduceTransparency").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        get(&client, "accessibility.increaseContrast").unwrap(),
        Value::Bool(true)
    );

    // Out-of-range and wrong-type writes are typed errors and leave the value
    // untouched.
    let range = set(&client, "appearance.transparency", &Value::Number(1.5));
    assert_eq!(
        error_name(&range.unwrap_err()).as_deref(),
        Some("org.dragonfruit.Settings1.Error.OutOfRange")
    );
    assert_eq!(
        get(&client, "appearance.transparency").unwrap(),
        Value::Number(0.8)
    );
    let mismatch = set(
        &client,
        "accessibility.increaseContrast",
        &Value::Text("yes".into()),
    );
    assert_eq!(
        error_name(&mismatch.unwrap_err()).as_deref(),
        Some("org.dragonfruit.Settings1.Error.TypeMismatch")
    );
    assert_eq!(
        get(&client, "accessibility.increaseContrast").unwrap(),
        Value::Bool(true)
    );

    drop(service);
}

#[test]
fn a_set_over_the_bus_is_persisted_and_reloaded_by_a_restart() {
    let scratch = Scratch::new();
    let path = scratch.file();

    // First daemon instance: load (no file yet) and serve with persistence.
    let loaded = persist::load(&path).expect("a missing file loads defaults");
    let bus = PrivateBus::start();
    let service = serve_object(
        &bus,
        Settings1::with_persistence(loaded.settings, loaded.persistence),
    );
    let client = bus.connect();

    set(&client, "dock.autohide", &Value::Bool(true)).expect("Set persists");
    set(
        &client,
        "appearance.colorScheme",
        &Value::Text("dark".into()),
    )
    .expect("Set persists");
    set(
        &client,
        "dock.pinned",
        &Value::TextList(vec!["a.desktop".into()]),
    )
    .expect("Set persists");

    // The file is on disk in the documented shape before `Set` returns.
    assert!(path.exists(), "a real Set writes the settings file");
    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["schema"], serde_json::Value::from(SCHEMA_VERSION));
    assert_eq!(json["keys"]["dock.autohide"], serde_json::Value::Bool(true));

    // Simulate a restart: a fresh object over the same file sees the writes.
    drop(service);
    let restarted = persist::load(&path).expect("the written file reloads");
    assert_eq!(
        *restarted.settings.get("dock.autohide").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        *restarted.settings.get("appearance.colorScheme").unwrap(),
        Value::Text("dark".into())
    );
    assert_eq!(
        *restarted.settings.get("dock.pinned").unwrap(),
        Value::TextList(vec!["a.desktop".into()])
    );
    assert!(!restarted.migrated, "a written file is already current");
}

/// Read one interface property through the standard Properties interface.
fn property_variant(client: &Connection, name: &str) -> OwnedValue {
    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(DBUS_NAME, name),
        )
        .expect("property Get succeeds");
    reply.body().deserialize().expect("a variant")
}

fn property_u32(client: &Connection, name: &str) -> u32 {
    u32::try_from(property_variant(client, name)).expect("a u32 property")
}

fn property_string(client: &Connection, name: &str) -> String {
    String::try_from(property_variant(client, name)).expect("a string property")
}

/// T282: NaN/Infinity never reaches the store and is a typed D-Bus error.
#[test]
fn non_finite_and_semantic_values_are_rejected_over_the_bus() {
    let bus = PrivateBus::start();
    let service = serve(&bus);
    let client = bus.connect();

    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = set(&client, "dock.size", &Value::Number(number)).unwrap_err();
        assert_eq!(
            error_name(&error).as_deref(),
            Some("org.dragonfruit.Settings1.Error.Invalid"),
            "{number} was not rejected"
        );
    }
    assert_eq!(get(&client, "dock.size").unwrap(), Value::Number(0.5));

    for (key, value) in [
        ("focus.schedule.start", Value::Text("7am".into())),
        ("appearance.accent", Value::Text("blue".into())),
        ("wallpaper.source", Value::Text("relative.jpg".into())),
        (
            "focus.schedule.days",
            Value::TextList(vec!["monday".into()]),
        ),
        (
            "quickSettings.modules",
            Value::TextList(vec!["not-a-module".into()]),
        ),
        (
            "input.replacements",
            Value::TextList(vec!["teh the".into()]),
        ),
    ] {
        let error = set(&client, key, &value).unwrap_err();
        assert_eq!(
            error_name(&error).as_deref(),
            Some("org.dragonfruit.Settings1.Error.Invalid"),
            "{key} accepted {value:?}"
        );
    }
    drop(service);
}

/// T282: the loaded revision and the durable state are visible to clients,
/// not just stderr.
#[test]
fn the_loaded_schema_and_persistence_state_are_properties() {
    let scratch = Scratch::new();
    let path = scratch.file();
    let loaded = persist::load(&path).expect("a missing file loads defaults");
    let bus = PrivateBus::start();
    let service = serve_object(
        &bus,
        Settings1::with_persistence_at(loaded.settings, loaded.persistence, loaded.schema),
    );
    let client = bus.connect();
    assert_eq!(property_u32(&client, "LoadedSchema"), persist::UNVERSIONED);
    assert_eq!(
        property_string(&client, "PersistenceState"),
        "saved".to_owned()
    );
    set(&client, "dock.autohide", &Value::Bool(true)).unwrap();
    assert_eq!(
        property_string(&client, "PersistenceState"),
        "saved".to_owned()
    );
    drop(service);
}

/// T282: a disk failure is a visible `memory-only` state plus a signal, and
/// the live value still applies. It never takes the daemon down.
#[test]
fn a_failed_persist_surfaces_memory_only_and_keeps_the_value() {
    let scratch = Scratch::new();
    let blocker = scratch.0.join("blocked");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let path = blocker.join("settings.json");
    let bus = PrivateBus::start();
    let service = serve_object(
        &bus,
        Settings1::with_persistence(Settings::new(), persist::Persistence::new(path)),
    );
    let client = bus.connect();

    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(DBUS_NAME)
        .expect("valid sender")
        .interface(DBUS_NAME)
        .expect("valid interface")
        .member("PersistenceFailed")
        .expect("valid member")
        .build();
    let mut signals = MessageIterator::for_match_rule(rule, &client, Some(4)).expect("subscribe");

    set(&client, "dock.autohide", &Value::Bool(true)).expect("the in-memory write applies");
    assert_eq!(
        get(&client, "dock.autohide").unwrap(),
        Value::Bool(true),
        "the live value is authoritative even when the file is not writable"
    );
    assert_eq!(
        property_string(&client, "PersistenceState"),
        "memory-only".to_owned()
    );
    assert!(
        !property_string(&client, "PersistenceError").is_empty(),
        "the failure message is visible"
    );

    let signal = signals
        .next()
        .expect("a PersistenceFailed signal")
        .expect("a valid signal");
    let (key, message): (String, String) = signal.body().deserialize().unwrap();
    assert_eq!(key, "dock.autohide");
    assert!(!message.is_empty());
    drop(service);
}

/// T282: two writers cannot leave an older snapshot on disk. Each thread
/// writes a different key; the final file must contain both, whichever
/// snapshot lands last.
#[test]
fn concurrent_writers_do_not_lose_an_update() {
    let scratch = Scratch::new();
    let path = scratch.file();
    let loaded = persist::load(&path).expect("a missing file loads defaults");
    let bus = PrivateBus::start();
    let service = serve_object(
        &bus,
        Settings1::with_persistence(loaded.settings, loaded.persistence),
    );
    // Warm the file so both writers race a real save.
    let warm = bus.connect();
    set(&warm, "dock.size", &Value::Number(0.6)).unwrap();

    let mut handles = Vec::new();
    for (key, value) in [
        ("dock.autohide", Value::Bool(true)),
        ("appearance.colorScheme", Value::Text("dark".into())),
        ("workspaces.count", Value::Integer(5)),
        ("dock.magnification", Value::Number(0.8)),
    ] {
        let client = bus.connect();
        handles.push(std::thread::spawn(move || {
            set(&client, key, &value).expect("concurrent Set succeeds");
        }));
    }
    for handle in handles {
        handle.join().expect("writer thread");
    }

    let reloaded = persist::load(&path).expect("the file reloads");
    assert_eq!(
        *reloaded.settings.get("dock.autohide").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        *reloaded.settings.get("appearance.colorScheme").unwrap(),
        Value::Text("dark".into())
    );
    assert_eq!(
        *reloaded.settings.get("workspaces.count").unwrap(),
        Value::Integer(5)
    );
    assert_eq!(
        *reloaded.settings.get("dock.magnification").unwrap(),
        Value::Number(0.8)
    );
    drop(service);
}
