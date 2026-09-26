// SPDX-License-Identifier: MIT
//! T-13.3a acceptance: the Screenshot portal round-trips a capture request for
//! each selection mode, with a test client acting as the presenter.
//!
//! Every test stands up a private `dbus-daemon` session bus and starts the
//! backend with [`xdg_desktop_portal_dragonfruit::dbus::serve`] — the same
//! code path the daemon uses. The test client is a second connection: it acts
//! as the *presenter* (the shell selection overlay's role) by observing the
//! `ScreenshotOpened` signal and calling the diagnostic
//! `CompleteScreenshot`. The standard `Screenshot` call blocks until that
//! completion, exactly as the portal frontend expects.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use xdg_desktop_portal_dragonfruit::interfaces::SCREENSHOT_INTERFACE;
use xdg_desktop_portal_dragonfruit::model::{DBUS_NAME, DBUS_PATH, STATUS_INTERFACE};
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{ObjectPath, OwnedValue, Str};
use zbus::MatchRule;

/// The presenter sees exactly this when a request waits.
type OpenedSignal = (String, String, String, String, HashMap<String, OwnedValue>);

/// The `(response, results)` reply the Screenshot method returns.
type ScreenshotReply = (u32, HashMap<String, OwnedValue>);

/// The handle a Screenshot call runs on its own thread.
type ScreenshotCall = std::thread::JoinHandle<Result<ScreenshotReply, zbus::Error>>;

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
            .expect("dbus-daemon is required to run the T-13.3a integration test");
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

/// Call the blocking Screenshot method on its own connection, so a presenter on
/// the main connection can complete it.
fn screenshot_on_its_own_connection(
    bus: &PrivateBus,
    handle: String,
    options: HashMap<String, OwnedValue>,
) -> ScreenshotCall {
    let connection = bus.connect();
    std::thread::spawn(move || {
        let path = ObjectPath::try_from(handle.as_str()).expect("a valid request handle");
        call(
            &connection,
            SCREENSHOT_INTERFACE,
            "Screenshot",
            (path, "org.example.App", "", options),
        )
    })
}

/// Subscribe to the diagnostic `ScreenshotOpened` signal.
fn opened_stream(client: &Connection) -> mpsc::Receiver<OpenedSignal> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path(DBUS_PATH)
        .expect("valid path")
        .interface(STATUS_INTERFACE)
        .expect("valid interface")
        .member("ScreenshotOpened")
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

/// The `uri` result decoded from a response vardict.
fn uri_of(results: &HashMap<String, OwnedValue>) -> Option<String> {
    results
        .get("uri")
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| String::try_from(value).ok())
}

/// Build a `mode` option vardict.
fn mode_options(mode: &str) -> HashMap<String, OwnedValue> {
    let mut options = HashMap::new();
    options.insert("mode".to_owned(), OwnedValue::from(Str::from(mode)));
    options
}

/// A real temporary file whose URI can come back from the portal.
fn capture_uri(tag: &str) -> String {
    let path = std::env::temp_dir().join(format!("df-screenshot-{tag}-{}.png", std::process::id()));
    std::fs::write(&path, b"png").expect("write a capture file");
    format!("file://{}", path.display())
}

#[test]
fn the_screenshot_interface_is_served_at_the_standard_path() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(SCREENSHOT_INTERFACE, "version"),
        )
        .expect("read version");
    let value: OwnedValue = reply.body().deserialize().expect("property body decodes");
    assert_eq!(u32::try_from(value).expect("version is a u32"), 2);

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
    assert!(introspection.contains(SCREENSHOT_INTERFACE));
    assert!(introspection.contains("Screenshot"));
}

#[test]
fn each_selection_mode_produces_a_capture_through_the_portal() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    // The presenter offers a surface for each mode and returns a capture.
    for mode in ["fullscreen", "region", "window"] {
        let handle = format!("/org/freedesktop/portal/desktop/request/1_1/df_shot_{mode}");
        let opened = opened_stream(&client);
        let pending = screenshot_on_its_own_connection(&bus, handle.clone(), mode_options(mode));

        let (signal_handle, signal_mode, app_id, parent, _options) = opened
            .recv_timeout(Duration::from_secs(5))
            .expect("ScreenshotOpened arrives");
        assert_eq!(signal_handle, handle);
        assert_eq!(signal_mode, mode, "the presenter learns the selection mode");
        assert_eq!(app_id, "org.example.App");
        assert_eq!(parent, "");

        let uri = capture_uri(mode);
        let completed: bool = diagnostic(
            &client,
            "CompleteScreenshot",
            (handle.clone(), uri.as_str()),
        );
        assert!(completed, "the presenter completed the {mode} request");
        assert!(
            diagnostic::<Vec<(String, String)>>(&client, "PendingScreenshots", ()).is_empty(),
            "the {mode} request left the registry"
        );

        let (response, results) = pending
            .join()
            .expect("the Screenshot thread")
            .expect("Screenshot succeeds");
        assert_eq!(response, 0, "success for {mode}");
        assert_eq!(uri_of(&results).as_deref(), Some(uri.as_str()));
    }
}

#[test]
fn a_cancelled_request_answers_with_the_cancelled_code() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let handle = "/org/freedesktop/portal/desktop/request/1_1/df_shot_cancel";
    let pending = screenshot_on_its_own_connection(&bus, handle.to_owned(), mode_options("region"));

    wait_for(Duration::from_secs(5), || {
        diagnostic::<Vec<(String, String)>>(&client, "PendingScreenshots", ())
            .iter()
            .any(|(h, _)| h == handle)
            .then_some(())
    });
    let cancelled: bool = diagnostic(&client, "CancelScreenshot", (handle,));
    assert!(cancelled);

    let (response, results) = pending
        .join()
        .expect("the Screenshot thread")
        .expect("Screenshot returns");
    assert_eq!(response, 1, "cancelled");
    assert!(results.is_empty());

    // An unknown handle is not completed.
    assert!(!diagnostic::<bool>(
        &client,
        "CompleteScreenshot",
        (
            "/org/freedesktop/portal/desktop/request/1_1/missing",
            "/tmp/x.png"
        )
    ));
}

#[test]
fn a_foreign_capture_uri_is_an_error_response() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let handle = "/org/freedesktop/portal/desktop/request/1_1/df_shot_foreign";
    let pending = screenshot_on_its_own_connection(&bus, handle.to_owned(), mode_options("window"));

    wait_for(Duration::from_secs(5), || {
        diagnostic::<Vec<(String, String)>>(&client, "PendingScreenshots", ())
            .iter()
            .any(|(h, _)| h == handle)
            .then_some(())
    });
    assert!(diagnostic::<bool>(
        &client,
        "CompleteScreenshot",
        (handle, "trash:///not-local")
    ));

    let (response, results) = pending
        .join()
        .expect("the Screenshot thread")
        .expect("Screenshot returns");
    assert_eq!(response, 2, "other error");
    assert!(uri_of(&results).is_none());
}
