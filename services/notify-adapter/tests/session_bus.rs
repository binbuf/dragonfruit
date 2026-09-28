// SPDX-License-Identifier: MIT
//! T-15.7a acceptance: drive the live `DbusNotifications` source against the
//! real notification service over a private session bus.
//!
//! Each test stands up a private `dbus-daemon`, serves the two notification
//! interfaces exactly as `dragonfruit_notifications::dbus::run` does, and then
//! points the adapter at that bus through [`DbusNotifications::at`]. Nothing
//! links the service's queue directly: every read and write goes through the
//! same shell-facing D-Bus surface the shell uses.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use dragonfruit_notifications::dbus::{
    self, FreedesktopNotifications, ShellNotifications, DBUS_NAME, DBUS_PATH,
};
use dragonfruit_notifications::model::Queue;
use dragonfruit_notify_adapter::{
    DbusNotifications, FocusMode, FocusOutcome, NotificationsAdapter, NotificationsSource,
};
use dragonfruit_system_adapters::Adapter;
use zbus::blocking::Connection;
use zbus::zvariant::Value;

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
            .expect("dbus-daemon is required to run the T-15.7a integration test");
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
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Serve both notification interfaces on `bus` exactly as the daemon does.
fn serve(bus: &PrivateBus) -> Connection {
    let queue = Arc::new(Mutex::new(Queue::new()));
    let wake = dbus::new_wake();
    let freedesktop = FreedesktopNotifications::from_shared(queue.clone(), wake.clone());
    let shell = ShellNotifications::from_shared(queue.clone(), wake.clone());
    let connection = zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("claim the notification name")
        .serve_at(DBUS_PATH, freedesktop)
        .expect("serve the freedesktop interface")
        .build()
        .expect("build the service connection");
    connection
        .object_server()
        .at(DBUS_PATH, shell)
        .expect("serve the shell interface");
    dbus::spawn_expiry(connection.clone(), queue, wake);
    connection
}

/// Raise one notification as an app would.
fn notify(connection: &Connection, app: &str, summary: &str) {
    let hints: HashMap<String, Value<'_>> = HashMap::new();
    connection
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                app,
                0u32,
                "",
                summary,
                "",
                Vec::<String>::new(),
                hints,
                -1i32,
            ),
        )
        .expect("Notify succeeds");
}

#[test]
fn the_live_source_reads_the_policy_and_notifications() {
    let bus = PrivateBus::start();
    let service = serve(&bus);
    notify(&service, "Mail", "New message");

    let mut source = DbusNotifications::at(bus.address.clone());
    let data = source
        .read()
        .expect("a live read")
        .expect("the service is present");
    assert_eq!(data.mode, FocusMode::Off);
    assert_eq!(data.active.len(), 1);
    assert_eq!(data.active[0].app_name, "Mail");
    assert_eq!(data.active[0].summary, "New message");
    assert_eq!(data.history.len(), 1);
}

#[test]
fn the_live_source_writes_the_focus_policy() {
    let bus = PrivateBus::start();
    let _service = serve(&bus);

    let mut source = DbusNotifications::at(bus.address.clone());
    assert_eq!(source.set_focus_mode(FocusMode::Dnd), FocusOutcome::Applied);
    assert_eq!(
        source.set_focus_allow_list(vec!["Pager".to_owned()]),
        FocusOutcome::Applied
    );

    let data = source
        .read()
        .expect("a live read")
        .expect("the service is present");
    assert_eq!(data.mode, FocusMode::Dnd);
    assert_eq!(data.allow_list, vec!["Pager".to_owned()]);
}

#[test]
fn the_adapter_drives_the_live_source() {
    let bus = PrivateBus::start();
    let service = serve(&bus);
    notify(&service, "Chat", "hi");

    let mut adapter = NotificationsAdapter::new(DbusNotifications::at(bus.address.clone()));
    assert!(adapter.state().is_unavailable());
    adapter.refresh();
    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().expect("available snapshot");
    assert_eq!(snapshot.active_count(), 1);

    assert_eq!(
        adapter.set_focus_mode(FocusMode::Focus),
        FocusOutcome::Applied
    );
    adapter.refresh();
    assert_eq!(adapter.snapshot().unwrap().focus.mode, FocusMode::Focus);
    assert_eq!(
        adapter.drain_changes(),
        vec![
            dragonfruit_notify_adapter::NotificationsChange::FocusModeChanged {
                from: FocusMode::Off,
                to: FocusMode::Focus,
            }
        ]
    );
}

#[test]
fn a_bus_without_the_service_is_absence() {
    let bus = PrivateBus::start();
    let mut adapter = NotificationsAdapter::new(DbusNotifications::at(bus.address.clone()));
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(adapter.snapshot().is_none());

    assert_eq!(adapter.set_focus_mode(FocusMode::Dnd), FocusOutcome::Absent);
    assert!(adapter.state().is_unavailable());
}

#[test]
fn the_adapter_reports_the_notifications_slot() {
    let adapter = NotificationsAdapter::new(DbusNotifications::new());
    assert_eq!(
        Adapter::id(&adapter),
        dragonfruit_system_adapters::AdapterId::NOTIFICATIONS
    );
}
