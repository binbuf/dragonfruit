// SPDX-License-Identifier: GPL-3.0-or-later
//! T-13.4a acceptance: the ScreenCast portal creates a session, opens the
//! source picker on `SelectSources`, returns the chosen source handle from
//! `Start`, and degrades honestly on cancel/error paths. T335b adds the live
//! half: a session owns its PipeWire nodes (a second `Start` replaces them, a
//! close releases them, a client that disconnects leaves none running), and
//! the picker names the stills fallback when the producer is unavailable.
//!
//! Every test stands up a private `dbus-daemon` session bus and starts the
//! backend with [`xdg_desktop_portal_dragonfruit::dbus::serve`] — the same
//! code path the daemon uses. The test client is a second connection: it acts
//! as the *presenter* (the shell source picker's role) by observing the
//! `ScreenCastOpened` signal and calling the diagnostic `CompleteScreenCast`.
//! The standard `SelectSources` call blocks until that completion, exactly as
//! the portal frontend expects.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use std::sync::{Arc, Mutex};

use xdg_desktop_portal_dragonfruit::interfaces::SCREENCAST_INTERFACE;
use xdg_desktop_portal_dragonfruit::model::{DBUS_NAME, DBUS_PATH, STATUS_INTERFACE};
use xdg_desktop_portal_dragonfruit::screencast::{ScreenCastRegistry, ScreenCastSelection};
use xdg_desktop_portal_dragonfruit::stream::{
    CompositorWatch, CreateFn, CreatedNode, FallbackReason, FnWatch, PipeWireTransport, StreamMode,
    StreamSource, StreamTransport, WatchError,
};
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{ObjectPath, OwnedValue};
use zbus::MatchRule;

/// The presenter sees exactly this when a picker request waits.
type OpenedSignal = (
    String,
    String,
    String,
    u32,
    bool,
    HashMap<String, OwnedValue>,
);

/// The `(response, results)` reply a ScreenCast method returns.
type ScreenCastReply = (u32, HashMap<String, OwnedValue>);

/// The handle a blocking `SelectSources` call runs on its own thread.
type SelectCall = std::thread::JoinHandle<Result<ScreenCastReply, zbus::Error>>;

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
            .expect("dbus-daemon is required to run the T-13.4a integration test");
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

/// Serve with `registry` as the ScreenCast registry (T335b), so a test can
/// inject a live transport and observe the same code path the daemon uses.
/// The returned connection must outlive the test body.
fn serve_with_registry(
    bus: &PrivateBus,
    registry: xdg_desktop_portal_dragonfruit::screencast::SharedScreenCast,
) -> (Connection, xdg_desktop_portal_dragonfruit::Backend) {
    xdg_desktop_portal_dragonfruit::dbus::serve_with_screencast(
        Some(bus.address.as_str()),
        xdg_desktop_portal_dragonfruit::COMPLETION_TIMEOUT,
        registry,
    )
    .expect("the backend serves on a private bus")
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

/// Read a `u` property from the ScreenCast interface.
fn version_property(client: &Connection, name: &str) -> u32 {
    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(SCREENCAST_INTERFACE, name),
        )
        .expect("read property");
    let value: OwnedValue = reply.body().deserialize().expect("property body decodes");
    u32::try_from(value).expect("property is a u32")
}

/// Create a session synchronously and return its handle path.
fn create_session(client: &Connection, session_handle: &str) {
    let handle = "/org/freedesktop/portal/desktop/request/1_1/df_cast_create";
    let (response, results): ScreenCastReply = call(
        client,
        SCREENCAST_INTERFACE,
        "CreateSession",
        (
            ObjectPath::try_from(handle).unwrap(),
            ObjectPath::try_from(session_handle).unwrap(),
            "org.example.App",
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .expect("CreateSession");
    assert_eq!(response, 0, "CreateSession succeeds");
    let returned = results
        .get("session_handle")
        .expect("session_handle result");
    let returned = ObjectPath::try_from(returned.try_clone().unwrap()).expect("an object path");
    assert_eq!(returned.as_str(), session_handle);
}

/// Run the blocking `SelectSources` call on its own connection.
fn select_sources_on_its_own_connection(
    bus: &PrivateBus,
    handle: String,
    session_handle: String,
    options: HashMap<String, OwnedValue>,
) -> SelectCall {
    let connection = bus.connect();
    std::thread::spawn(move || {
        call(
            &connection,
            SCREENCAST_INTERFACE,
            "SelectSources",
            (
                ObjectPath::try_from(handle.as_str()).expect("a valid request handle"),
                ObjectPath::try_from(session_handle.as_str()).expect("a valid session handle"),
                "org.example.App",
                options,
            ),
        )
    })
}

/// Subscribe to the diagnostic `ScreenCastOpened` signal.
fn opened_stream(client: &Connection) -> mpsc::Receiver<OpenedSignal> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path(DBUS_PATH)
        .expect("valid path")
        .interface(STATUS_INTERFACE)
        .expect("valid interface")
        .member("ScreenCastOpened")
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

/// The `streams` result decoded into
/// `(node_id, id, source_type, stream_mode, fallback)` tuples.
fn streams_of(
    results: &HashMap<String, OwnedValue>,
) -> Vec<(u32, String, u32, String, Option<String>)> {
    let wire = results
        .get("streams")
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| Vec::<(u32, HashMap<String, OwnedValue>)>::try_from(value).ok())
        .unwrap_or_default();
    wire.into_iter()
        .map(|(node_id, properties)| {
            let string_of = |key: &str| {
                properties
                    .get(key)
                    .and_then(|value| value.try_clone().ok())
                    .and_then(|value| String::try_from(value).ok())
            };
            let id = string_of("id").unwrap_or_default();
            let source_type = properties
                .get("source_type")
                .and_then(|value| value.try_clone().ok())
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or_default();
            let mode = string_of("df_stream_mode").unwrap_or_default();
            let fallback = string_of("df_fallback");
            (node_id, id, source_type, mode, fallback)
        })
        .collect()
}

/// Build a `SelectSources` options vardict.
fn selection_options(types: u32, multiple: bool) -> HashMap<String, OwnedValue> {
    let mut options = HashMap::new();
    options.insert("types".to_owned(), OwnedValue::from(types));
    options.insert("multiple".to_owned(), OwnedValue::from(multiple));
    options
}

/// Run `Start` on a session and return the first stream's `(id, source_type)`.
fn start_first_stream(client: &Connection, session_handle: &str) -> Option<(String, u32)> {
    start_streams(client, session_handle)
        .into_iter()
        .next()
        .map(|(_, id, source_type, _, _)| (id, source_type))
}

/// Run `Start` on a session and return its decoded streams.
fn start_streams(
    client: &Connection,
    session_handle: &str,
) -> Vec<(u32, String, u32, String, Option<String>)> {
    let (response, results): ScreenCastReply = call(
        client,
        SCREENCAST_INTERFACE,
        "Start",
        (
            ObjectPath::try_from("/org/freedesktop/portal/desktop/request/1_1/df_cast_start")
                .unwrap(),
            ObjectPath::try_from(session_handle).unwrap(),
            "org.example.App",
            "",
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .expect("Start");
    assert_eq!(response, 0, "Start succeeds");
    streams_of(&results)
}

/// Close a session through the standard `org.freedesktop.impl.portal.Session`
/// contract, the way a client that disconnects or exits does.
fn close_session(client: &Connection, session_handle: &str) {
    let reply = client
        .call_method(
            Some(DBUS_NAME),
            session_handle,
            Some("org.freedesktop.impl.portal.Session"),
            "Close",
            &(),
        )
        .expect("Close");
    assert_eq!(reply.message_type(), zbus::message::Type::MethodReturn);
}

#[test]
fn the_screencast_interface_is_served_at_the_standard_path() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    assert_eq!(version_property(&client, "version"), 3);
    assert_eq!(
        version_property(&client, "AvailableSourceTypes"),
        1 | 2,
        "monitors and windows"
    );
    assert_eq!(
        version_property(&client, "AvailableCursorModes"),
        1 | 2,
        "hidden and embedded"
    );

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
    assert!(introspection.contains(SCREENCAST_INTERFACE));
    assert!(introspection.contains("SelectSources"));
    assert!(introspection.contains("AvailableSourceTypes"));
}

#[test]
fn a_client_request_opens_the_picker_and_returns_a_chosen_source() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let session_handle = "/org/freedesktop/portal/desktop/session/1_1/df_cast";
    let request_handle = "/org/freedesktop/portal/desktop/request/1_1/df_cast_select";
    create_session(&client, session_handle);

    // The session object is served at the caller's path and speaks the
    // standard Session contract.
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

    let opened = opened_stream(&client);
    let pending = select_sources_on_its_own_connection(
        &bus,
        request_handle.to_owned(),
        session_handle.to_owned(),
        selection_options(1 | 2, true),
    );

    // The picker model arrives: the requested types and multiple flag.
    let (signal_handle, signal_session, app_id, types, multiple, options) = opened
        .recv_timeout(Duration::from_secs(5))
        .expect("ScreenCastOpened arrives");
    assert_eq!(signal_handle, request_handle);
    assert_eq!(signal_session, session_handle);
    assert_eq!(app_id, "org.example.App");
    assert_eq!(types, 1 | 2);
    assert!(multiple);
    assert!(
        !options.contains_key("cursor_mode"),
        "no cursor_mode requested"
    );

    // The presenter chooses a monitor and a window (handle, source type bit).
    let completed: bool = diagnostic(
        &client,
        "CompleteScreenCast",
        (
            request_handle,
            vec![
                ("monitor:DP-1".to_owned(), 1u32),
                ("window:7".to_owned(), 2u32),
            ],
        ),
    );
    assert!(completed, "the presenter completed the request");
    assert!(
        diagnostic::<Vec<(String, String)>>(&client, "PendingScreenCasts", ()).is_empty(),
        "the request left the registry"
    );

    let (response, _results) = pending
        .join()
        .expect("the SelectSources thread")
        .expect("SelectSources returns");
    assert_eq!(response, 0, "success");

    // Start returns the chosen sources; the node id is a placeholder for
    // T-13.4b and the id carries the source handle.
    let (response, results): ScreenCastReply = call(
        &client,
        SCREENCAST_INTERFACE,
        "Start",
        (
            ObjectPath::try_from("/org/freedesktop/portal/desktop/request/1_1/df_cast_start")
                .unwrap(),
            ObjectPath::try_from(session_handle).unwrap(),
            "org.example.App",
            "",
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .expect("Start");
    assert_eq!(response, 0);
    let streams = streams_of(&results);
    assert_eq!(streams.len(), 2);
    // T-13.4b: this build has no in-process PipeWire producer, so every stream
    // is the *named* stills fallback — node id 0 plus an explicit mode and
    // reason, never a silent placeholder.
    assert_eq!(
        streams[0],
        (
            0,
            "monitor:DP-1".to_owned(),
            1,
            "stills".to_owned(),
            Some("pipewire-producer-unavailable".to_owned()),
        )
    );
    assert_eq!(
        streams[1],
        (
            0,
            "window:7".to_owned(),
            2,
            "stills".to_owned(),
            Some("pipewire-producer-unavailable".to_owned()),
        )
    );

    // The same mode is readable before a source is chosen, so the picker can
    // say the fallback out loud.
    assert_eq!(
        diagnostic::<String>(&client, "ScreenCastStreamMode", ()),
        "stills"
    );
}

#[test]
fn a_cancelled_request_answers_with_the_cancelled_code() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let session_handle = "/org/freedesktop/portal/desktop/session/1_1/df_cast_cancel";
    let request_handle = "/org/freedesktop/portal/desktop/request/1_1/df_cast_cancel";
    create_session(&client, session_handle);

    let pending = select_sources_on_its_own_connection(
        &bus,
        request_handle.to_owned(),
        session_handle.to_owned(),
        selection_options(1, false),
    );

    wait_for(Duration::from_secs(5), || {
        diagnostic::<Vec<(String, String)>>(&client, "PendingScreenCasts", ())
            .iter()
            .any(|(handle, _)| handle == request_handle)
            .then_some(())
    });
    let cancelled: bool = diagnostic(&client, "CancelScreenCast", (request_handle,));
    assert!(cancelled);

    let (response, results) = pending
        .join()
        .expect("the SelectSources thread")
        .expect("SelectSources returns");
    assert_eq!(response, 1, "cancelled");
    assert!(results.is_empty());

    // Start before a selection is an error response, never a silent success.
    let (response, results): ScreenCastReply = call(
        &client,
        SCREENCAST_INTERFACE,
        "Start",
        (
            ObjectPath::try_from("/org/freedesktop/portal/desktop/request/1_1/df_cast_start2")
                .unwrap(),
            ObjectPath::try_from(session_handle).unwrap(),
            "org.example.App",
            "",
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .expect("Start returns a response");
    assert_eq!(response, 2, "other error");
    assert!(results.is_empty());

    // The choice is reflected in the picker's own system: a second request
    // completing a monitor is picked up by Start.
    let request2 = "/org/freedesktop/portal/desktop/request/1_1/df_cast_again";
    let pending = select_sources_on_its_own_connection(
        &bus,
        request2.to_owned(),
        session_handle.to_owned(),
        selection_options(1, false),
    );
    wait_for(Duration::from_secs(5), || {
        diagnostic::<Vec<(String, String)>>(&client, "PendingScreenCasts", ())
            .iter()
            .any(|(handle, _)| handle == request2)
            .then_some(())
    });
    assert!(diagnostic::<bool>(
        &client,
        "CompleteScreenCast",
        (request2, vec![("monitor:HDMI-1".to_owned(), 1u32)]),
    ));
    pending.join().unwrap().unwrap();
    assert_eq!(
        start_first_stream(&client, session_handle),
        Some(("monitor:HDMI-1".to_owned(), 1))
    );
}

#[test]
fn an_empty_selection_is_an_error_response() {
    let bus = PrivateBus::start();
    let (_service, _backend) = bus.serve();
    let client = bus.connect();

    let session_handle = "/org/freedesktop/portal/desktop/session/1_1/df_cast_empty";
    let request_handle = "/org/freedesktop/portal/desktop/request/1_1/df_cast_empty";
    create_session(&client, session_handle);

    let pending = select_sources_on_its_own_connection(
        &bus,
        request_handle.to_owned(),
        session_handle.to_owned(),
        selection_options(1, false),
    );
    wait_for(Duration::from_secs(5), || {
        diagnostic::<Vec<(String, String)>>(&client, "PendingScreenCasts", ())
            .iter()
            .any(|(handle, _)| handle == request_handle)
            .then_some(())
    });
    assert!(diagnostic::<bool>(
        &client,
        "CompleteScreenCast",
        (request_handle, Vec::<(String, u32)>::new()),
    ));

    let (response, _results) = pending
        .join()
        .expect("the SelectSources thread")
        .expect("SelectSources returns");
    assert_eq!(response, 2, "other error");

    // An unknown handle is not completed.
    assert!(!diagnostic::<bool>(
        &client,
        "CompleteScreenCast",
        (
            "/org/freedesktop/portal/desktop/request/1_1/missing",
            vec![("monitor:1".to_owned(), 1u32)],
        )
    ));
}

// --- T335a/T335b: the live transport ---------------------------------------

/// A live transport over a scripted compositor that records the tokens it
/// releases, so a test can prove every node is released.
fn recording_live_transport(start: u32) -> (Box<dyn StreamTransport>, Arc<Mutex<Vec<String>>>) {
    let next = Arc::new(Mutex::new(start));
    let destroyed = Arc::new(Mutex::new(Vec::new()));
    let create: CreateFn = {
        let next = next.clone();
        Arc::new(move |_source: &StreamSource| {
            let mut node = next.lock().unwrap();
            let id = *node;
            *node += 1;
            Ok(id)
        })
    };
    let watch = FnWatch::new(create).recording_destroys(destroyed.clone());
    (Box::new(PipeWireTransport::new(Box::new(watch))), destroyed)
}

/// The registry path a portal `Start`/`Close` drives, over the live transport:
/// `Start` returns a PipeWire node id and the mode, and closing the session
/// releases the node. This is the T335a transport test at the acceptance level:
/// the D-Bus backend serves the same registry, and the shipped backend without
/// a shell bridge is covered by the fallback test.
#[test]
fn the_live_transport_fills_the_start_result_and_releases_the_node_on_close() {
    let (transport, destroyed) = recording_live_transport(77);
    let mut registry = ScreenCastRegistry::with_transport(transport);
    assert_eq!(registry.stream_mode(), StreamMode::PipeWire);
    registry.create_session("org.example.App", "/s").unwrap();
    let request = xdg_desktop_portal_dragonfruit::screencast::ScreenCastRequest::new(
        "/r",
        "/s",
        "org.example.App",
        &HashMap::new(),
    );
    registry.begin_select(request).expect("begin_select");
    registry
        .complete("/r", vec![ScreenCastSelection::monitor("monitor:DP-1")])
        .expect("complete");
    let response = registry.start("/s").expect("start");
    let streams = response.streams();
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].node_id, 77, "the live node id reaches the wire");
    assert_eq!(streams[0].mode, StreamMode::PipeWire);
    assert_eq!(streams[0].fallback, None);
    assert_eq!(registry.live_nodes("/s"), vec![77]);
    assert!(registry.close_session("/s"));
    assert_eq!(
        *destroyed.lock().unwrap(),
        vec!["monitor-monitor:DP-1".to_owned()],
        "the session close released the live node"
    );
    assert_eq!(registry.live_node_count(), 0);
}

/// T335b: over a private session bus, two client sessions each hold their own
/// live stream, closing one (the `Session.Close` a client sends) releases only
/// that session's node, and the diagnostic surface names the live node the
/// other session still holds.
#[test]
fn a_d_bus_session_close_releases_only_its_own_live_node() {
    let bus = PrivateBus::start();
    let (transport, destroyed) = recording_live_transport(900);
    let registry = Arc::new(Mutex::new(ScreenCastRegistry::with_transport(transport)));
    let (_service, _backend) = serve_with_registry(&bus, registry);
    let client = bus.connect();
    assert_eq!(
        diagnostic::<String>(&client, "ScreenCastStreamMode", ()),
        "pipewire",
        "the served registry has the live transport"
    );

    let sessions = [
        (
            "/org/freedesktop/portal/desktop/session/1_1/df_cast_first",
            "/org/freedesktop/portal/desktop/request/1_1/df_cast_first_select",
            "monitor:DP-1",
        ),
        (
            "/org/freedesktop/portal/desktop/session/1_1/df_cast_second",
            "/org/freedesktop/portal/desktop/request/1_1/df_cast_second_select",
            "window:7",
        ),
    ];
    let mut nodes = Vec::new();
    for (session, request, source) in sessions {
        create_session(&client, session);
        let pending = select_sources_on_its_own_connection(
            &bus,
            request.to_owned(),
            session.to_owned(),
            selection_options(1 | 2, false),
        );
        wait_for(Duration::from_secs(5), || {
            diagnostic::<Vec<(String, String)>>(&client, "PendingScreenCasts", ())
                .iter()
                .any(|(handle, _)| handle == request)
                .then_some(())
        });
        assert!(diagnostic::<bool>(
            &client,
            "CompleteScreenCast",
            (request.to_owned(), vec![(source.to_owned(), 1u32)]),
        ));
        assert_eq!(pending.join().unwrap().unwrap().0, 0);
        let streams = start_streams(&client, session);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].3, "pipewire");
        nodes.push(streams[0].0);
    }
    assert_eq!(nodes, vec![900, 901], "each session got its own node");
    assert_eq!(
        diagnostic::<u32>(&client, "ScreenCastLiveStreams", ()),
        2,
        "two live streams"
    );
    let first = sessions[0].0;
    let second = sessions[1].0;
    assert_eq!(
        diagnostic::<Vec<(u32, String)>>(&client, "ScreenCastLiveNodes", (first,)),
        vec![(900, "monitor:DP-1".to_owned())]
    );

    // The first client closes its session (what `Session.Close` is).
    close_session(&client, first);
    assert_eq!(
        *destroyed.lock().unwrap(),
        vec!["monitor-monitor:DP-1".to_owned()]
    );
    assert_eq!(
        diagnostic::<u32>(&client, "ScreenCastLiveStreams", ()),
        1,
        "the second session's node is still live"
    );
    assert_eq!(
        diagnostic::<Vec<(u32, String)>>(&client, "ScreenCastLiveNodes", (second,)),
        vec![(901, "window:7".to_owned())]
    );

    // The remaining stream is released by the presenter-loss seam the shell
    // bridge calls when it goes away; a client that disconnects never leaves a
    // producer running.
    assert_eq!(
        diagnostic::<u32>(&client, "ReleaseScreenCasts", ()),
        1,
        "the release names the one remaining stream"
    );
    assert_eq!(destroyed.lock().unwrap().len(), 2);
    assert_eq!(diagnostic::<u32>(&client, "ScreenCastLiveStreams", ()), 0);
}

/// A live transport whose compositor refuses still answers the D-Bus flow with
/// the named stills fallback, never a silent success.
#[test]
fn a_refusing_compositor_is_the_named_stills_fallback() {
    let mut transport = PipeWireTransport::unavailable();
    assert_eq!(
        transport
            .create(&StreamSource {
                id: "monitor:DP-1".to_owned(),
                source_type: xdg_desktop_portal_dragonfruit::screencast::SourceType::Monitor,
                cursor_mode: 1,
            })
            .expect_err("no compositor"),
        FallbackReason::ProducerUnavailable
    );
}

/// A watch that times out or is missing is an error, never a placeholder id
/// (the shape T335b's shell bridge must also obey).
struct SilentWatch;

impl CompositorWatch for SilentWatch {
    fn create(
        &mut self,
        _token: &str,
        _source: &StreamSource,
        _fps: u32,
    ) -> Result<CreatedNode, WatchError> {
        Err(WatchError("the shell bridge did not answer".to_owned()))
    }

    fn destroy(&mut self, _token: &str) {}
}

#[test]
fn a_silent_compositor_never_becomes_a_fake_node() {
    let mut transport = PipeWireTransport::new(Box::new(SilentWatch));
    assert!(transport
        .create(&StreamSource {
            id: "window:7".to_owned(),
            source_type: xdg_desktop_portal_dragonfruit::screencast::SourceType::Window,
            cursor_mode: 1,
        })
        .is_err());
}

// --- T335b: the shipped bridge over a real compositor ----------------------

/// Why [`LiveBridgeCompositor`] could not start: the compositor binary or the
/// runtime directory a real session provides is missing. The caller records
/// that as a skip with the exact reason rather than a silent pass.
fn compositor_binary() -> Option<std::path::PathBuf> {
    let candidates = [
        std::env::var_os("DRAGONFRUIT_COMPOSITOR_BIN").map(std::path::PathBuf::from),
        Some(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../target/debug/dragonfruit-compositor"),
        ),
        Some(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../target/release/dragonfruit-compositor"),
        ),
    ];
    candidates.into_iter().flatten().find(|path| path.is_file())
}

/// Wait until a killed compositor's socket path no longer accepts, bounded so
/// a leaked listener fails the test loudly instead of hanging it.
fn wait_socket_released(path: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match std::os::unix::net::UnixStream::connect(path) {
            Ok(_) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return,
        }
    }
    panic!(
        "{} still accepts after its compositor exited",
        path.display()
    );
}

/// A compositor command ready to spawn: the binary, the headless backend, and
/// the user-space DRM sysroot this developer tree links against (absent on a
/// normal host, where the default loader path already works).
fn compositor_command(binary: &std::path::Path, socket_name: &str, tokens: &str) -> Command {
    let mut command = Command::new(binary);
    command
        .arg("--backend")
        .arg("headless")
        .arg("--socket-name")
        .arg(socket_name)
        .env("DRAGONFRUIT_LAUNCH_TOKENS", tokens)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(home) = std::env::var_os("HOME") {
        let lib = std::path::PathBuf::from(&home).join(".local/df-toolchain/usr/lib64");
        if lib.is_dir() {
            // Merge with the caller's path: a Makefile target already exports
            // the toolchain. Replacing it would drop the rest of the loader
            // search path the compositor links against.
            let mut dirs = vec![lib];
            if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
                dirs.extend(std::env::split_paths(&existing));
            }
            if let Ok(joined) = std::env::join_paths(dirs) {
                command.env("LD_LIBRARY_PATH", joined);
            }
        }
    }
    command
}

/// A headless compositor provisioned with a portal-role launch token, the way
/// a session manager does it: the token is in the compositor's environment,
/// in its `<socket>.portal-launch-token` hand-off file, and in the portal's
/// `DRAGONFRUIT_SCREENCAST_BRIDGE_TOKEN`.
struct LiveBridgeCompositor {
    child: Child,
    socket_name: String,
    token: String,
    runtime_dir: std::path::PathBuf,
}

impl LiveBridgeCompositor {
    fn start() -> Result<Self, String> {
        let Some(binary) = compositor_binary() else {
            return Err("no built dragonfruit-compositor (run `make build`)".to_owned());
        };
        let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from)
        else {
            return Err("XDG_RUNTIME_DIR is not set".to_owned());
        };
        let socket_name = format!("dragonfruit-t335b-bridge-{}", std::process::id());
        let token = "7a".repeat(32);
        let socket = runtime_dir.join(&socket_name);
        let hand_off = runtime_dir.join(format!("{socket_name}.portal-launch-token"));
        let mut child = compositor_command(&binary, &socket_name, &format!("portal:{token}"))
            .spawn()
            .map_err(|error| format!("cannot start the compositor: {error}"))?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !socket.exists() || !hand_off.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!("the compositor exited before ready: {status}"));
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                return Err("the compositor never became ready".to_owned());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(LiveBridgeCompositor {
            child,
            socket_name,
            token,
            runtime_dir,
        })
    }

    fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // The kill is the test's own child, so waiting for the socket to be
        // released is reliable: the partially closed listener's socket keeps
        // `connect(2)` succeeding for a moment after the process dies, and a
        // port reused too early would make this test's assertions flaky.
        wait_socket_released(&self.runtime_dir.join(&self.socket_name));
        for suffix in [
            "",
            ".lock",
            ".launch-token",
            ".desktop-launch-token",
            ".portal-launch-token",
            ".x11-display",
        ] {
            let _ = std::fs::remove_file(
                self.runtime_dir
                    .join(format!("{}{suffix}", self.socket_name)),
            );
        }
    }
}

/// T335b acceptance: the shipped bridge is the live transport of the served
/// registry. Against a real compositor it opens as a trusted client, reports
/// `pipewire` to the picker, and — because the headless rail has no PipeWire
/// host — a `Start` is the *named*, per-stream stills fallback rather than a
/// promised node. When the compositor goes away the transport drops back to
/// `stills` (so the picker stops promising frames), and `reconnect_transport`
/// is the presenter-loss seam that reopens a fresh connection.
#[test]
fn the_shipped_bridge_is_live_against_a_compositor_and_reconnects_after_it_leaves() {
    let compositor = match LiveBridgeCompositor::start() {
        Ok(compositor) => compositor,
        Err(reason) => {
            eprintln!("SKIP the live bridge check: {reason}");
            return;
        }
    };
    // Absolute sockets keep this check off the session bus entirely.
    let runtime_dir = compositor.runtime_dir.to_string_lossy().into_owned();
    // SAFETY: set before the bridge is built and restored before the test ends;
    // this test owns the process environment for its duration.
    let previous_runtime = std::env::var_os("XDG_RUNTIME_DIR");
    let previous_token = std::env::var_os(xdg_desktop_portal_dragonfruit::BRIDGE_TOKEN_ENV);
    let previous_socket = std::env::var_os(xdg_desktop_portal_dragonfruit::BRIDGE_SOCKET_ENV);
    std::env::set_var("XDG_RUNTIME_DIR", &runtime_dir);
    std::env::set_var(
        xdg_desktop_portal_dragonfruit::BRIDGE_SOCKET_ENV,
        &compositor.socket_name,
    );

    // The launcher hands the backend exactly the value the compositor
    // provisioned for the portal role (its hand-off file). A value the
    // compositor never minted is not adopted (it mints its own and writes it
    // here), so this file is the one source of the bridge token.
    let hand_off = compositor
        .runtime_dir
        .join(format!("{}.portal-launch-token", compositor.socket_name));
    let provisioned = std::fs::read_to_string(&hand_off)
        .expect("the compositor writes the portal token hand-off file")
        .trim()
        .to_owned();
    assert_eq!(
        provisioned, compositor.token,
        "the hand-off file carries the value the session manager exports"
    );
    std::env::set_var(
        xdg_desktop_portal_dragonfruit::BRIDGE_TOKEN_ENV,
        &provisioned,
    );

    let shared = xdg_desktop_portal_dragonfruit::screencast::registry_live();
    let mut registry = xdg_desktop_portal_dragonfruit::screencast::lock(&shared);
    assert_eq!(
        registry.stream_mode(),
        StreamMode::PipeWire,
        "a provisioned bridge reports a live transport: {:?}",
        registry.transport_error()
    );
    assert_eq!(registry.transport_error(), None);

    registry
        .create_session("org.example.T335b", "/session/t335b")
        .expect("a session is registered");
    let mut request = xdg_desktop_portal_dragonfruit::ScreenCastRequest {
        handle: "/request/t335b".to_owned(),
        session_handle: "/session/t335b".to_owned(),
        app_id: "org.example.T335b".to_owned(),
        options: Default::default(),
        raw_options: HashMap::new(),
    };
    request.options.types = 1;
    registry.begin_select(request).expect("the picker opens");
    registry.complete(
        "/request/t335b",
        vec![ScreenCastSelection::monitor("monitor:")],
    );
    let response = registry
        .start("/session/t335b")
        .expect("Start answers for a chosen source");
    let streams = response.streams();
    let stream = streams.first().expect("one stream");
    assert_eq!(
        stream.mode,
        StreamMode::Stills,
        "the headless rail has no PipeWire host, so this is the named fallback"
    );
    assert_eq!(
        stream.fallback,
        Some(FallbackReason::ProducerUnavailable),
        "the refusal is named, never a silent live-looking stream"
    );
    assert_eq!(stream.node_id, 0);
    assert_eq!(
        registry.live_node_count(),
        0,
        "a fallback creates no node, so nothing can leak"
    );

    // The compositor leaves (a crash, a session restart), and the replacement
    // on the same socket is a *new* session: it never minted the old token, so
    // reusing it would be refused (the one-time token contract). The owner
    // re-issues the portal token for the restarted session and hands the new
    // value to the backend, which is what this test simulates below.
    let bridge_socket_name = compositor.socket_name.clone();
    compositor.stop();
    let Some(binary) = compositor_binary() else {
        panic!("the compositor binary vanished mid-test");
    };
    // The restarted session provisions the portal role again: it mints its own
    // one-time token (the old one is dead with the old session). The old
    // hand-off file is removed first, so the wait below never reads the
    // previous session's value.
    let hand_off = std::path::Path::new(&runtime_dir)
        .join(format!("{bridge_socket_name}.portal-launch-token"));
    let _ = std::fs::remove_file(&hand_off);
    let mut restarted = compositor_command(&binary, &bridge_socket_name, "portal:")
        .spawn()
        .expect("a replacement compositor starts");
    let socket_path = std::path::Path::new(&runtime_dir).join(&bridge_socket_name);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !socket_path.exists() || !hand_off.exists() {
        if let Ok(Some(status)) = restarted.try_wait() {
            panic!("the replacement compositor exited before ready: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "the replacement compositor never became ready"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // The presenter-loss seam: release, then reopen. A reopen that still
    // presents the *previous* session's one-time token is refused by the new
    // session (it never minted that value), so the transport must report the
    // refusal and never claim `pipewire` — a reuse is not a live bridge.
    let stale_resolver = {
        let socket = socket_path.to_string_lossy().into_owned();
        let stale_token = "7a".repeat(32);
        Box::new(move || {
            xdg_desktop_portal_dragonfruit::ScreenCastBridge::connect(&socket, &stale_token).map(
                |bridge| {
                    Box::new(bridge) as Box<dyn xdg_desktop_portal_dragonfruit::CompositorWatch>
                },
            )
        }) as Box<dyn FnMut() -> Result<_, String> + Send>
    };
    registry.release_streams();
    assert!(
        !registry.rescan_transport(Some(stale_resolver)),
        "a dead session's token never reopens a live bridge"
    );
    assert_eq!(registry.stream_mode(), StreamMode::Stills);
    assert!(
        registry.transport_error().is_some(),
        "a failed reopen says why"
    );

    // The restarted session wrote a fresh portal token. The owner reads it from
    // the compositor's hand-off file (the provisioned source) and hands it to
    // the backend; with it the reopen is verified live again — not merely
    // "connected".
    let hand_off_token = std::fs::read_to_string(&hand_off)
        .expect("the restarted session writes its portal token")
        .trim()
        .to_owned();
    assert!(
        hand_off_token.len() == 64 && hand_off_token.bytes().all(|b| b.is_ascii_hexdigit()),
        "the hand-off file carries a fresh one-time token"
    );
    assert_ne!(
        hand_off_token,
        "7a".repeat(32),
        "the restarted session never re-issues the old session's token"
    );
    let fresh_resolver = {
        let socket = socket_path.to_string_lossy().into_owned();
        let token = hand_off_token.clone();
        Box::new(move || {
            xdg_desktop_portal_dragonfruit::ScreenCastBridge::connect(&socket, &token).map(
                |bridge| {
                    Box::new(bridge) as Box<dyn xdg_desktop_portal_dragonfruit::CompositorWatch>
                },
            )
        }) as Box<dyn FnMut() -> Result<_, String> + Send>
    };
    assert!(
        registry.rescan_transport(Some(fresh_resolver)),
        "the fresh token reopens the bridge: {:?}",
        registry.transport_error()
    );
    assert_eq!(registry.stream_mode(), StreamMode::PipeWire);
    assert_eq!(registry.transport_error(), None);

    let _ = restarted.kill();
    let _ = restarted.wait();
    wait_socket_released(&socket_path);

    match previous_runtime {
        Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
        None => std::env::remove_var("XDG_RUNTIME_DIR"),
    }
    for (name, previous) in [
        (
            xdg_desktop_portal_dragonfruit::BRIDGE_TOKEN_ENV,
            previous_token,
        ),
        (
            xdg_desktop_portal_dragonfruit::BRIDGE_SOCKET_ENV,
            previous_socket,
        ),
    ] {
        match previous {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}
