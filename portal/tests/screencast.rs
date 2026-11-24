// SPDX-License-Identifier: MIT
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
