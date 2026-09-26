// SPDX-License-Identifier: MIT
//! T-13.1a acceptance: the portal backend registers on a real (private)
//! session bus and the `xdg-desktop-portal` frontend's absence is graceful.
//!
//! Every test stands up a private `dbus-daemon` session bus and starts the
//! backend with [`xdg_desktop_portal_dragonfruit::dbus::serve`] — the same
//! code path the daemon uses — then talks to it through a second connection,
//! the shape the frontend uses. Nothing links the model directly: the
//! assertions go through D-Bus.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use xdg_desktop_portal_dragonfruit::model::{
    FrontendPresence, DBUS_NAME, DBUS_PATH, FRONTEND_NAME, STATUS_INTERFACE,
};
use zbus::blocking::{Connection, MessageIterator};
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
            .expect("dbus-daemon is required to run the T-13.1a integration test");
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

    /// Start the backend on this bus; returns the service connection. The
    /// connection must be kept alive for the backend to stay registered.
    fn serve(&self) -> (Connection, xdg_desktop_portal_dragonfruit::Backend) {
        xdg_desktop_portal_dragonfruit::dbus::serve(Some(self.address.as_str()))
            .expect("the backend serves on a private bus")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Whether `name` currently has an owner on the bus.
fn name_has_owner(client: &Connection, name: &str) -> bool {
    let reply = client
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &(name,),
        )
        .expect("NameHasOwner call");
    reply.body().deserialize::<bool>().expect("bool reply")
}

/// Call a no-argument method on the diagnostic interface.
fn call<T>(client: &Connection, method: &str) -> T
where
    T: serde::de::DeserializeOwned + zbus::zvariant::Type,
{
    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some(STATUS_INTERFACE),
            method,
            &(),
        )
        .expect("diagnostic method call");
    reply.body().deserialize::<T>().expect("reply body decodes")
}

/// Wait for the next `FrontendChanged` signal on a separate connection.
struct FrontendWaiter {
    receiver: mpsc::Receiver<(bool, String)>,
}

impl FrontendWaiter {
    fn start(client: &Connection) -> Self {
        let rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .path(DBUS_PATH)
            .expect("valid path")
            .interface(STATUS_INTERFACE)
            .expect("valid interface")
            .member("FrontendChanged")
            .expect("valid member")
            .build();
        let (sender, receiver) = mpsc::channel();
        let connection = client.clone();
        std::thread::spawn(move || {
            if let Ok(iterator) = MessageIterator::for_match_rule(rule, &connection, Some(8)) {
                for message in iterator {
                    let Ok(message) = message else { break };
                    if let Ok(body) = message.body().deserialize::<(bool, String)>() {
                        if sender.send(body).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        FrontendWaiter { receiver }
    }

    fn next(&self, timeout: Duration) -> Option<(bool, String)> {
        self.receiver.recv_timeout(timeout).ok()
    }
}

#[test]
fn the_backend_registers_its_standard_name_on_a_private_bus() {
    let bus = PrivateBus::start();
    let (_service, backend) = bus.serve();
    let client = bus.connect();

    assert!(
        name_has_owner(&client, DBUS_NAME),
        "the backend owns {DBUS_NAME}"
    );
    assert_eq!(call::<String>(&client, "BackendName"), "dragonfruit");
    assert_eq!(call::<String>(&client, "DbusName"), DBUS_NAME);
    assert_eq!(call::<String>(&client, "ObjectPath"), DBUS_PATH);
    assert_eq!(
        call::<Vec<String>>(&client, "Interfaces"),
        vec![
            "org.freedesktop.impl.portal.Settings".to_owned(),
            "org.freedesktop.impl.portal.GlobalShortcuts".to_owned(),
            "org.freedesktop.impl.portal.FileChooser".to_owned(),
            "org.freedesktop.impl.portal.Screenshot".to_owned(),
            "org.freedesktop.impl.portal.ScreenCast".to_owned(),
        ]
    );

    // The standard path exists and serves the diagnostic interface, so the
    // object path a portal frontend will call is live from this task on.
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
    assert!(
        introspection.contains(STATUS_INTERFACE),
        "the served path introspects"
    );

    // The local object and the bus agree.
    assert_eq!(backend.status().dbus_name, DBUS_NAME);
}

#[test]
fn an_absent_frontend_is_graceful() {
    let bus = PrivateBus::start();
    let (_service, backend) = bus.serve();
    let client = bus.connect();

    assert!(!name_has_owner(&client, FRONTEND_NAME), "no frontend");
    assert!(
        !call::<bool>(&client, "FrontendPresent"),
        "the backend reports the frontend absent"
    );
    assert_eq!(call::<String>(&client, "Frontend"), "absent");
    assert_eq!(call::<String>(&client, "FrontendOwner"), "");
    assert_eq!(backend.status().frontend, FrontendPresence::Absent);

    // Absence is not an error: every method still answers.
    assert_eq!(call::<String>(&client, "BackendName"), "dragonfruit");
}

#[test]
fn a_present_frontend_is_detected_at_startup() {
    let bus = PrivateBus::start();
    let frontend = bus.connect();
    frontend
        .request_name(FRONTEND_NAME)
        .expect("the mock frontend owns its name");
    let owner = frontend
        .unique_name()
        .expect("the connection has a unique name")
        .to_string();

    let (_service, backend) = bus.serve();
    let client = bus.connect();

    assert!(call::<bool>(&client, "FrontendPresent"));
    assert_eq!(call::<String>(&client, "Frontend"), "present");
    assert_eq!(call::<String>(&client, "FrontendOwner"), owner);
    assert_eq!(
        backend.status().frontend,
        FrontendPresence::Present {
            owner: owner.clone()
        }
    );
}

#[test]
fn the_frontend_presence_tracks_the_name_owner_live() {
    let bus = PrivateBus::start();
    let (_service, backend) = bus.serve();
    let client = bus.connect();
    let waiter = FrontendWaiter::start(&client);

    // Give the watch thread a moment to arm its NameOwnerChanged match rule;
    // the startup probe already recorded `absent`.
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(backend.status().frontend, FrontendPresence::Absent);

    let frontend = bus.connect();
    frontend
        .request_name(FRONTEND_NAME)
        .expect("the mock frontend owns its name");
    let (present, owner) = waiter
        .next(Duration::from_secs(5))
        .expect("FrontendChanged(present) arrives");
    assert!(present, "the frontend appeared");
    assert_eq!(owner, frontend.unique_name().unwrap().to_string());
    assert!(backend.status().frontend.is_present());

    frontend
        .release_name(FRONTEND_NAME)
        .expect("the mock frontend releases its name");
    let (present, owner) = waiter
        .next(Duration::from_secs(5))
        .expect("FrontendChanged(absent) arrives");
    assert!(!present, "the frontend left");
    assert_eq!(owner, "");
    assert_eq!(backend.status().frontend, FrontendPresence::Absent);
}
