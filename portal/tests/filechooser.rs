// SPDX-License-Identifier: MIT
//! T-13.2a acceptance: the FileChooser portal round-trips open/save requests
//! with a test client, using files-core for normalization and listing.
//!
//! Every test stands up a private `dbus-daemon` session bus and starts the
//! backend with [`xdg_desktop_portal_dragonfruit::dbus::serve`] — the same
//! code path the daemon uses. The test client is a second connection: it acts
//! as the *presenter* (the T-13.2b picker's role) by observing the
//! `FileChooserOpened` signal and calling the diagnostic
//! `CompleteFileChooser`. The standard `OpenFile`/`SaveFile` call blocks until
//! that completion, exactly as the portal frontend expects.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use xdg_desktop_portal_dragonfruit::interfaces::FILE_CHOOSER_INTERFACE;
use xdg_desktop_portal_dragonfruit::model::{DBUS_NAME, DBUS_PATH, STATUS_INTERFACE};
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::OwnedValue;
use zbus::MatchRule;

/// The presenter sees exactly this when a request waits.
type OpenedSignal = (
    String,
    String,
    String,
    String,
    String,
    HashMap<String, OwnedValue>,
);

/// The `(response, results)` reply a chooser method returns.
type ChooserReply = (u32, HashMap<String, OwnedValue>);

/// The handle a chooser call runs on its own thread.
type ChooserCall = std::thread::JoinHandle<Result<ChooserReply, zbus::Error>>;

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
            .expect("dbus-daemon is required to run the T-13.2a integration test");
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

/// Call a method on the standard backend object.
fn call<T>(
    client: &Connection,
    interface: &str,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error>
where
    T: serde::de::DeserializeOwned + zbus::zvariant::Type,
{
    let reply = client.call_method(Some(DBUS_NAME), DBUS_PATH, Some(interface), method, &body)?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

/// Call a method on the diagnostic interface.
fn diagnostic<T>(
    client: &Connection,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> T
where
    T: serde::de::DeserializeOwned + zbus::zvariant::Type,
{
    call(client, STATUS_INTERFACE, method, body).expect("diagnostic method call")
}

/// Call the blocking chooser method on its own connection, so a presenter on
/// the main connection can complete it.
fn chooser_on_its_own_connection(
    bus: &PrivateBus,
    method: &'static str,
    handle: String,
    options: HashMap<String, OwnedValue>,
) -> ChooserCall {
    let connection = bus.connect();
    std::thread::spawn(move || {
        let path =
            zbus::zvariant::ObjectPath::try_from(handle.as_str()).expect("a valid request handle");
        call(
            &connection,
            FILE_CHOOSER_INTERFACE,
            method,
            (path, "org.example.App", "", "Choose a file", options),
        )
    })
}

/// Subscribe to the diagnostic `FileChooserOpened` signal.
fn opened_stream(client: &Connection) -> mpsc::Receiver<OpenedSignal> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path(DBUS_PATH)
        .expect("valid path")
        .interface(STATUS_INTERFACE)
        .expect("valid interface")
        .member("FileChooserOpened")
        .expect("valid member")
        .build();
    let (sender, receiver) = mpsc::channel();
    let connection = client.clone();
    std::thread::spawn(move || {
        if let Ok(iterator) = MessageIterator::for_match_rule(rule, &connection, Some(8)) {
            for message in iterator {
                let Ok(message) = message else { break };
                if let Ok(body) = message.body().deserialize::<OpenedSignal>() {
                    if sender.send(body).is_err() {
                        break;
                    }
                }
            }
        }
    });
    // Arm the match rule before the caller triggers the request.
    std::thread::sleep(Duration::from_millis(150));
    receiver
}

/// Poll `check` until `Some`, or panic after `timeout`.
fn wait_for<T>(timeout: Duration, mut check: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = check() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for the state");
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Read a `u` property from the FileChooser interface.
fn version(client: &Connection) -> u32 {
    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(FILE_CHOOSER_INTERFACE, "Version"),
        )
        .expect("read Version");
    let value: OwnedValue = reply.body().deserialize().expect("property body decodes");
    u32::try_from(value).expect("Version is a u32")
}

#[test]
fn the_file_chooser_interface_is_served_at_the_standard_path() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    assert_eq!(version(&client), 3);
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
    assert!(introspection.contains(FILE_CHOOSER_INTERFACE));
    for method in ["OpenFile", "SaveFile", "SaveFiles"] {
        assert!(introspection.contains(method), "introspects {method}");
    }
}

#[test]
fn the_test_client_drives_an_open_request_to_a_normalized_path() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let handle = "/org/freedesktop/portal/desktop/request/1_1/df_open";
    let opened = opened_stream(&client);

    // The frontend's OpenFile blocks on its own connection.
    let pending =
        chooser_on_its_own_connection(&bus, "OpenFile", handle.to_owned(), HashMap::new());

    // The presenter sees the request, complete it with a mix of a plain path,
    // a file:// URI, and a foreign URI that must be discarded.
    let (signal_handle, kind, app_id, parent, title, _options) = opened
        .recv_timeout(Duration::from_secs(5))
        .expect("FileChooserOpened arrives");
    assert_eq!(signal_handle, handle);
    assert_eq!(kind, "open");
    assert_eq!(app_id, "org.example.App");
    assert_eq!(parent, "");
    assert_eq!(title, "Choose a file");

    let completed: bool = diagnostic(
        &client,
        "CompleteFileChooser",
        (
            handle,
            vec![
                "/tmp/one two".to_owned(),
                "file:///tmp/three".to_owned(),
                "trash:///discarded".to_owned(),
            ],
        ),
    );
    assert!(completed, "the presenter completed the request");
    assert!(
        diagnostic::<Vec<(String, String)>>(&client, "PendingFileChoosers", ()).is_empty(),
        "the request left the registry"
    );

    let (response, results) = pending
        .join()
        .expect("the OpenFile thread")
        .expect("OpenFile succeeds");
    assert_eq!(response, 0, "success");
    let uris = uris_of(&results);
    assert_eq!(
        uris,
        vec!["file:///tmp/one%20two", "file:///tmp/three"],
        "paths and file URIs normalize; foreign URIs are discarded"
    );
    assert_eq!(
        results.get("writable").and_then(|v| bool::try_from(v).ok()),
        Some(false)
    );
}

#[test]
fn the_test_client_drives_a_save_request_to_a_path() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let handle = "/org/freedesktop/portal/desktop/request/1_1/df_save";
    let opened = opened_stream(&client);

    let pending =
        chooser_on_its_own_connection(&bus, "SaveFile", handle.to_owned(), HashMap::new());

    let (signal_handle, kind, _, _, _, _) = opened
        .recv_timeout(Duration::from_secs(5))
        .expect("FileChooserOpened arrives");
    assert_eq!(signal_handle, handle);
    assert_eq!(kind, "save");

    let completed: bool = diagnostic(
        &client,
        "CompleteFileChooser",
        (handle, vec!["/tmp/notes/report.txt".to_owned()]),
    );
    assert!(completed);

    let (response, results) = pending
        .join()
        .expect("the SaveFile thread")
        .expect("SaveFile succeeds");
    assert_eq!(response, 0);
    assert_eq!(uris_of(&results), vec!["file:///tmp/notes/report.txt"]);
    // Save results carry no `writable` key.
    assert!(!results.contains_key("writable"));
}

#[test]
fn a_cancelled_request_answers_with_the_cancelled_code() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let handle = "/org/freedesktop/portal/desktop/request/1_1/df_cancel";
    let pending =
        chooser_on_its_own_connection(&bus, "SaveFile", handle.to_owned(), HashMap::new());

    // Wait until the request is registered, then cancel it.
    wait_for(Duration::from_secs(5), || {
        diagnostic::<Vec<(String, String)>>(&client, "PendingFileChoosers", ())
            .iter()
            .any(|(h, _)| h == handle)
            .then_some(())
    });
    let cancelled: bool = diagnostic(&client, "CancelFileChooser", (handle,));
    assert!(cancelled);

    let (response, results) = pending
        .join()
        .expect("the SaveFile thread")
        .expect("SaveFile returns");
    assert_eq!(response, 1, "cancelled");
    assert!(results.is_empty());

    // An empty or unknown handle is not completed.
    assert!(!diagnostic::<bool>(
        &client,
        "CompleteFileChooser",
        (
            "/org/freedesktop/portal/desktop/request/1_1/missing",
            vec!["/tmp/x".to_owned()]
        )
    ));
}

#[test]
fn the_presenter_lists_a_directory_through_files_core() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let dir = std::env::temp_dir().join(format!("df-portal-chooser-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("zeta.txt"), b"hello").expect("file");
    std::fs::create_dir(dir.join("alpha")).expect("subdir");

    let uri = format!("file://{}", dir.display());
    let rows: Vec<(String, String, bool, u64)> =
        diagnostic(&client, "ListDirectory", (uri.as_str(),));
    let names: Vec<&str> = rows.iter().map(|(name, _, _, _)| name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "zeta.txt"], "files-core collation");
    assert!(rows[0].2, "alpha is a directory");
    assert_eq!(rows[1].3, 5, "zeta.txt is five bytes");
    assert!(rows[0].1.starts_with("file://"));

    // A foreign scheme is refused, not silently emptied.
    let refused: Result<Vec<(String, String, bool, u64)>, _> =
        call(&client, STATUS_INTERFACE, "ListDirectory", ("trash:///",));
    assert!(refused.is_err());

    let _ = std::fs::remove_dir_all(&dir);
}

/// The `uris` result decoded from a response vardict.
fn uris_of(results: &HashMap<String, OwnedValue>) -> Vec<String> {
    results
        .get("uris")
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| Vec::<String>::try_from(value).ok())
        .unwrap_or_default()
}
