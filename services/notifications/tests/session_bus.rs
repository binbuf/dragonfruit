// SPDX-License-Identifier: MIT
//! T-11.1a acceptance: drive `org.freedesktop.Notifications` and the
//! shell-facing `org.dragonfruit.Notifications1` over a real session bus.
//!
//! Each test stands up a private `dbus-daemon` session bus, serves both
//! interfaces at the standard path (exactly as [`dragonfruit_notifications::dbus::run`]
//! does), and talks to them through a second connection — the same shape an
//! app and the Qt shell use. Nothing links the model directly: every
//! assertion goes through the D-Bus surface (`Notify`, `CloseNotification`,
//! `GetCapabilities`, `GetServerInformation`, `Banners`, `History`, `Dismiss`,
//! `Expire`, the `Changed` and `NotificationClosed` signals).

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use dragonfruit_notifications::dbus::{
    self, FreedesktopNotifications, ShellNotifications, DBUS_NAME, DBUS_PATH,
    FREEDESKTOP_INTERFACE, SHELL_INTERFACE,
};
use dragonfruit_notifications::model::Queue;
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
            .expect("dbus-daemon is required to run the T-11.1a integration test");
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

/// Serve both notification interfaces on `bus` exactly as the daemon does:
/// the freedesktop interface at build time, the shell interface added at the
/// same path, and the expiry thread running. Returns the service connection
/// and its shared queue.
fn serve(bus: &PrivateBus) -> (Connection, Arc<Mutex<Queue>>) {
    let queue = Arc::new(Mutex::new(Queue::new()));
    let wake = dbus::new_wake();
    let freedesktop = FreedesktopNotifications::from_shared(queue.clone(), wake.clone());
    let shell = ShellNotifications::from_shared(queue.clone(), wake.clone());
    let connection = zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .serve_at(DBUS_PATH, freedesktop)
        .expect("serve the freedesktop interface")
        .build()
        .expect("build the service connection");
    connection
        .object_server()
        .at(DBUS_PATH, shell)
        .expect("serve the shell interface at the same path");
    dbus::spawn_expiry(connection.clone(), queue.clone(), wake);
    (connection, queue)
}

/// Call a freedesktop method and deserialize the reply body.
fn freedesktop_call<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
    client: &Connection,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error> {
    let reply = client.call_method(
        Some(DBUS_NAME),
        DBUS_PATH,
        Some(FREEDESKTOP_INTERFACE),
        method,
        &body,
    )?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

/// Call a shell-facing method and deserialize the reply body.
fn shell_call<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
    client: &Connection,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error> {
    let reply = client.call_method(
        Some(DBUS_NAME),
        DBUS_PATH,
        Some(SHELL_INTERFACE),
        method,
        &body,
    )?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

/// A `Notify` with empty hints and no actions.
fn notify(
    client: &Connection,
    app_name: &str,
    replaces_id: u32,
    summary: &str,
    body: &str,
    expire_timeout: i32,
) -> u32 {
    let hints = HashMap::<String, OwnedValue>::new();
    freedesktop_call(
        client,
        "Notify",
        (
            app_name,
            replaces_id,
            "",
            summary,
            body,
            Vec::<String>::new(),
            hints,
            expire_timeout,
        ),
    )
    .expect("Notify succeeds")
}

/// Parse a JSON array payload into a `serde_json::Value`.
fn json_array(payload: &str) -> serde_json::Value {
    serde_json::from_str(payload).expect("the shell view is valid JSON")
}

#[test]
fn a_notify_round_trips_and_updates_the_banner_and_history_models() {
    let bus = PrivateBus::start();
    let (_service, _queue) = serve(&bus);
    let client = bus.connect();

    let id = notify(&client, "Mail", 0, "New message", "From Ada", -1);
    assert_eq!(id, 1);

    // The banner model has it.
    let banners = json_array(&shell_call::<String>(&client, "Banners", ()).unwrap());
    assert_eq!(banners.as_array().unwrap().len(), 1);
    assert_eq!(banners[0]["id"], 1);
    assert_eq!(banners[0]["appName"], "Mail");
    assert_eq!(banners[0]["summary"], "New message");
    assert_eq!(banners[0]["body"], "From Ada");
    assert_eq!(banners[0]["urgency"], "normal");
    // The deadline resolves the `< 0 == service default` rule (5s).
    let deadline = banners[0]["deadline"].as_u64().unwrap();
    let created = banners[0]["createdAt"].as_u64().unwrap();
    assert_eq!(deadline - created, 5000);

    // It is recorded in the history immediately (reason `null` while open).
    let history = json_array(&shell_call::<String>(&client, "History", ()).unwrap());
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["id"], 1);
    assert!(history[0]["reason"].is_null());

    // The server answers the informational calls.
    let capabilities: Vec<String> = freedesktop_call(&client, "GetCapabilities", ()).unwrap();
    assert!(capabilities.contains(&"body".to_owned()));
    assert!(!capabilities.contains(&"actions".to_owned()));
    let (name, vendor, version, spec): (String, String, String, String) =
        freedesktop_call(&client, "GetServerInformation", ()).unwrap();
    assert_eq!(
        (name.as_str(), vendor.as_str(), spec.as_str()),
        ("Dragonfruit", "Dragonfruit", "1.2")
    );
    assert!(!version.is_empty());
}

#[test]
fn the_shell_changed_signal_fires_on_notify() {
    let bus = PrivateBus::start();
    let (_service, _queue) = serve(&bus);
    let client = bus.connect();

    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(DBUS_NAME)
        .expect("valid sender")
        .interface(SHELL_INTERFACE)
        .expect("valid interface")
        .member("Changed")
        .expect("valid member")
        .build();
    let mut signals =
        MessageIterator::for_match_rule(rule, &client, Some(8)).expect("subscribe to Changed");

    notify(&client, "Chat", 0, "Ping", "Are you there?", -1);

    let message = signals
        .next()
        .transpose()
        .expect("the bus is readable")
        .expect("a Changed signal arrives");
    assert_eq!(message.header().member().unwrap().as_str(), "Changed");
}

#[test]
fn close_notification_emits_closed_and_clears_the_banner_but_keeps_history() {
    let bus = PrivateBus::start();
    let (_service, _queue) = serve(&bus);
    let client = bus.connect();

    let id = notify(&client, "Mail", 0, "New message", "From Ada", 0);
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(DBUS_NAME)
        .expect("valid sender")
        .interface(FREEDESKTOP_INTERFACE)
        .expect("valid interface")
        .member("NotificationClosed")
        .expect("valid member")
        .build();
    let mut signals = MessageIterator::for_match_rule(rule, &client, Some(8))
        .expect("subscribe to NotificationClosed");

    freedesktop_call::<()>(&client, "CloseNotification", (id,)).unwrap();

    let message = signals
        .next()
        .transpose()
        .expect("the bus is readable")
        .expect("a NotificationClosed signal arrives");
    let (closed_id, reason): (u32, u32) = message.body().deserialize().unwrap();
    assert_eq!((closed_id, reason), (id, 3));

    let banners = json_array(&shell_call::<String>(&client, "Banners", ()).unwrap());
    assert!(banners.as_array().unwrap().is_empty());
    let history = json_array(&shell_call::<String>(&client, "History", ()).unwrap());
    assert_eq!(history[0]["reason"], "closed");
    assert!(history[0]["closedAt"].as_u64().is_some());
}

#[test]
fn dismiss_and_expire_record_their_reasons() {
    let bus = PrivateBus::start();
    let (_service, _queue) = serve(&bus);
    let client = bus.connect();

    let first = notify(&client, "Chat", 0, "one", "", 0);
    let second = notify(&client, "Chat", 0, "two", "", 0);
    assert!(shell_call::<bool>(&client, "Dismiss", (first,)).unwrap());
    assert!(shell_call::<bool>(&client, "Expire", (second,)).unwrap());
    assert!(!shell_call::<bool>(&client, "Expire", (second,)).unwrap());

    let history = json_array(&shell_call::<String>(&client, "History", ()).unwrap());
    // Most recent first: `second` (expired) then `first` (dismissed).
    assert_eq!(history[0]["id"], second);
    assert_eq!(history[0]["reason"], "expired");
    assert_eq!(history[1]["id"], first);
    assert_eq!(history[1]["reason"], "dismissed");
}

#[test]
fn a_replaces_id_updates_the_same_banner() {
    let bus = PrivateBus::start();
    let (_service, _queue) = serve(&bus);
    let client = bus.connect();

    let first = notify(&client, "Mail", 0, "Connecting", "", 0);
    let replaced = notify(&client, "Mail", first, "Connected", "Inbox ready", 0);
    assert_eq!(replaced, first);

    let banners = json_array(&shell_call::<String>(&client, "Banners", ()).unwrap());
    assert_eq!(banners.as_array().unwrap().len(), 1);
    assert_eq!(banners[0]["summary"], "Connected");
}

#[test]
fn the_service_expires_a_banner_and_emits_the_expired_reason() {
    let bus = PrivateBus::start();
    let (_service, _queue) = serve(&bus);
    let client = bus.connect();

    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(DBUS_NAME)
        .expect("valid sender")
        .interface(FREEDESKTOP_INTERFACE)
        .expect("valid interface")
        .member("NotificationClosed")
        .expect("valid member")
        .build();
    let mut signals = MessageIterator::for_match_rule(rule, &client, Some(8))
        .expect("subscribe to NotificationClosed");

    let id = notify(&client, "Mail", 0, "Short-lived", "", 60);

    let message = signals
        .next()
        .transpose()
        .expect("the bus is readable")
        .expect("the expiry thread emits NotificationClosed");
    let (closed_id, reason): (u32, u32) = message.body().deserialize().unwrap();
    assert_eq!((closed_id, reason), (id, 1));

    let banners = json_array(&shell_call::<String>(&client, "Banners", ()).unwrap());
    assert!(banners.as_array().unwrap().is_empty());
    let history = json_array(&shell_call::<String>(&client, "History", ()).unwrap());
    assert_eq!(history[0]["reason"], "expired");
}

#[test]
fn do_not_disturb_suppresses_the_banner_over_the_bus() {
    let bus = PrivateBus::start();
    let (_service, _queue) = serve(&bus);
    let client = bus.connect();

    shell_call::<()>(&client, "SetDoNotDisturb", (true,)).unwrap();
    let id = notify(&client, "Mail", 0, "quiet", "", 0);

    let banners = json_array(&shell_call::<String>(&client, "Banners", ()).unwrap());
    assert!(banners.as_array().unwrap().is_empty());
    let history = json_array(&shell_call::<String>(&client, "History", ()).unwrap());
    assert_eq!(history[0]["id"], id);
}

#[test]
fn a_late_expiry_is_lazy_until_the_deadline() {
    // The model expires only at the deadline; a 250 ms banner is still active
    // at 100 ms and gone at 300 ms (the timing half is covered by the
    // signal-driven test above, this one checks the boundary).
    let mut queue = Queue::new();
    queue.notify(
        dragonfruit_notifications::model::NotifyRequest {
            summary: "x".to_owned(),
            expire_timeout_ms: 250,
            ..Default::default()
        },
        1000,
    );
    assert!(queue.banners().len() == 1);
    assert!(queue.expire_due(1099).is_empty());
    assert_eq!(queue.expire_due(1250), vec![1]);
}
