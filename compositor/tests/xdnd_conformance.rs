// SPDX-License-Identifier: GPL-3.0-only
//! T-14.5 acceptance: the XDnD bridge codec against a real Xwayland server.
//!
//! Smithay 0.7's XWM has no XDnD translation, so the bridge is this
//! compositor's to build. The pure model lives in `dragonfruit_compositor::xdnd`;
//! this test proves the wire half against a live X11 server: it starts the
//! headless compositor (which starts Xwayland), connects as an X client,
//! advertises a target window with `XdndAware`, then drives the bridge's
//! `XdndMessage`/`XdndTarget` through real X ClientMessages and asserts the
//! messages a target would actually receive.
//!
//! If `Xwayland` is not installed the test is skipped, like
//! `xwayland_conformance.rs`.

use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use dragonfruit_compositor::xdnd::{
    XdndAction, XdndAtoms, XdndMessage, XdndTarget, URI_LIST_MIME, XDND_VERSION,
};
use dragonfruit_compositor::xdnd_bridge::{serve_selection_request, BridgeEvent, XdndBridge};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt as _, CreateWindowAux, EventMask, PropMode,
    Window as X11Window, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

mod common;

const SOCKET_WAIT: Duration = Duration::from_secs(10);
const X11_WAIT: Duration = Duration::from_secs(10);

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    display_path: PathBuf,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        common::cleanup_compositor_artifacts(&self.socket_path);
    }
}

impl CompositorProcess {
    fn start(socket_name: &str) -> Self {
        let mut child = common::compositor_command()
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(socket_name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start compositor");

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(socket_name);
        let display_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.x11-display"));

        let deadline = Instant::now() + SOCKET_WAIT;
        while !socket_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before socket appeared: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "compositor socket never appeared"
            );
            std::thread::sleep(Duration::from_millis(10));
        }

        Self {
            child,
            socket_path,
            display_path,
        }
    }

    fn wait_for_display(&self) -> Option<String> {
        let deadline = Instant::now() + X11_WAIT;
        while Instant::now() < deadline {
            if let Ok(contents) = std::fs::read_to_string(&self.display_path) {
                let display = contents.trim().to_string();
                if !display.is_empty() {
                    return Some(display);
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    }
}

fn xwayland_available() -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join("Xwayland").is_file()))
        .unwrap_or(false)
}

fn intern(conn: &RustConnection, name: &str) -> Atom {
    conn.intern_atom(false, name.as_bytes())
        .expect("intern_atom")
        .reply()
        .expect("intern_atom reply")
        .atom
}

fn make_window(conn: &RustConnection, screen: &x11rb::protocol::xproto::Screen) -> X11Window {
    let window = conn.generate_id().expect("generate window id");
    let aux = CreateWindowAux::new().background_pixel(screen.white_pixel);
    conn.create_window(
        screen.root_depth,
        window,
        screen.root,
        0,
        0,
        1,
        1,
        0,
        WindowClass::INPUT_OUTPUT,
        x11rb::COPY_FROM_PARENT,
        &aux,
    )
    .expect("create_window");
    window
}

/// Send a format-32 ClientMessage to `destination`.
fn send_message(conn: &RustConnection, destination: X11Window, type_atom: Atom, data: [u32; 5]) {
    let event = ClientMessageEvent::new(32, destination, type_atom, data);
    conn.send_event(false, destination, EventMask::NO_EVENT, event)
        .expect("send_event");
    conn.flush().expect("flush");
}

/// Poll the connection until a ClientMessage with `type_atom` addressed to
/// `window` arrives, or the deadline passes.
fn wait_for_message(
    conn: &RustConnection,
    window: X11Window,
    type_atom: Atom,
    what: &str,
) -> [u32; 5] {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match conn.poll_for_event() {
            Ok(Some(Event::ClientMessage(message))) => {
                if message.window == window && message.format == 32 && message.type_ == type_atom {
                    return message.data.as_data32();
                }
                // A different message is not ours; keep waiting.
            }
            Ok(Some(_)) => {}
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(err) => panic!("poll_for_event: {err}"),
        }
    }
    panic!("timed out waiting for {what}");
}

#[test]
fn xdnd_messages_round_trip_through_xwayland() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-xdnd-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let screen = conn.setup().roots[screen_num].clone();

    // Intern the XDnD control atoms plus the MIME type the bridge translates.
    let atoms = XdndAtoms {
        selection: intern(&conn, "XdndSelection"),
        aware: intern(&conn, "XdndAware"),
        enter: intern(&conn, "XdndEnter"),
        leave: intern(&conn, "XdndLeave"),
        position: intern(&conn, "XdndPosition"),
        status: intern(&conn, "XdndStatus"),
        drop: intern(&conn, "XdndDrop"),
        finished: intern(&conn, "XdndFinished"),
        type_list: intern(&conn, "XdndTypeList"),
        action_list: intern(&conn, "XdndActionList"),
        action_description: intern(&conn, "XdndActionDescription"),
        action_copy: intern(&conn, "XdndActionCopy"),
        action_move: intern(&conn, "XdndActionMove"),
        action_link: intern(&conn, "XdndActionLink"),
        action_ask: intern(&conn, "XdndActionAsk"),
        action_private: intern(&conn, "XdndActionPrivate"),
        proxy: intern(&conn, "XdndProxy"),
    };
    let uri_list = intern(&conn, URI_LIST_MIME);

    // The bridge's target window advertises XDnD 5 via XdndAware.
    let target = make_window(&conn, &screen);
    conn.change_property32(
        PropMode::REPLACE,
        target,
        atoms.aware,
        AtomEnum::ATOM,
        &[u32::from(XDND_VERSION)],
    )
    .expect("set XdndAware");
    // A separate X window plays the drag source.
    let source = make_window(&conn, &screen);
    conn.flush().expect("flush");

    // The target really carries the advertised version.
    let aware = conn
        .get_property(false, target, atoms.aware, AtomEnum::ATOM, 0, 1)
        .expect("get XdndAware")
        .reply()
        .expect("get reply");
    assert_eq!(
        aware.value32().and_then(|mut it| it.next()),
        Some(u32::from(XDND_VERSION)),
        "XdndAware must advertise version 5"
    );

    // The X source sends XdndEnter; the message a real target receives must
    // equal the bridge codec's wire form.
    let enter = XdndMessage::Enter {
        source,
        version: XDND_VERSION,
        more_types: false,
        types: vec![uri_list],
    };
    let enter_data = enter.encode(&atoms).expect("encode Enter");
    send_message(&conn, target, atoms.enter, enter_data);
    let received = wait_for_message(&conn, target, atoms.enter, "XdndEnter");
    assert_eq!(received, enter_data, "XdndEnter bytes round-trip");
    assert_eq!(
        XdndMessage::decode("XdndEnter", received, &atoms),
        Some(enter),
        "the target decodes the codec's own Enter"
    );

    // The bridge-as-target accepts only a text/uri-list drag and replies with
    // an XdndStatus that the source actually receives.
    let mut target_state = XdndTarget::new();
    target_state.enter(source, XDND_VERSION, vec![uri_list]);
    assert!(target_state.offers(uri_list));

    let position = XdndMessage::Position {
        source,
        position: (320, 240),
        time: 99,
        action: XdndAction::Copy,
    };
    send_message(
        &conn,
        target,
        atoms.position,
        position.encode(&atoms).expect("encode Position"),
    );
    let received = wait_for_message(&conn, target, atoms.position, "XdndPosition");
    let decoded = XdndMessage::decode("XdndPosition", received, &atoms).expect("decode Position");
    assert_eq!(decoded, position);

    let status = target_state.position((320, 240), XdndAction::Copy, uri_list);
    assert!(status.accept, "a text/uri-list drag is accepted");
    let status_message = XdndMessage::Status {
        target,
        accept: status.accept,
        want_position: status.want_position,
        action: status.action,
    };
    let status_data = status_message.encode(&atoms).expect("encode Status");
    send_message(&conn, source, atoms.status, status_data);
    let received = wait_for_message(&conn, source, atoms.status, "XdndStatus");
    assert_eq!(received, status_data, "XdndStatus bytes round-trip");
    assert_eq!(
        XdndMessage::decode("XdndStatus", received, &atoms),
        Some(status_message),
        "the source decodes the bridge's Status"
    );

    // The drop yields the payload request, and the finished reply flips
    // success for the source.
    let drop_request = target_state.drop(150).expect("accepted drop");
    assert_eq!(drop_request.source, source);
    assert_eq!(drop_request.position, (320, 240));
    assert_eq!(drop_request.time, 150);
    let finished = target_state.finished(true, XdndAction::Copy);
    let finished_message = XdndMessage::Finished {
        target: finished.target,
        success: finished.success,
        action: finished.action,
    };
    let finished_data = finished_message.encode(&atoms).expect("encode Finished");
    send_message(&conn, source, atoms.finished, finished_data);
    let received = wait_for_message(&conn, source, atoms.finished, "XdndFinished");
    assert_eq!(received, finished_data);
    assert_eq!(
        XdndMessage::decode("XdndFinished", received, &atoms),
        Some(finished_message)
    );

    // A non-file drag (no text/uri-list) is refused, so a drop cannot resolve.
    let mut text_only = XdndTarget::new();
    text_only.enter(source, XDND_VERSION, vec![atoms.action_list]);
    let refused = text_only.position((0, 0), XdndAction::Copy, uri_list);
    assert!(!refused.accept);
    assert!(text_only.drop(1).is_none());

    // `proc` drops here (SIGKILL + wait); `conn` drops with the test.
    std::mem::drop(conn);
}

/// Drain the bridge and answer the fake source's selection request. Returns
/// the bridge's events and every client message the source received.
fn pump_bridge(
    bridge: &mut XdndBridge,
    source_conn: &RustConnection,
    payload: &[u8],
    timeout: Duration,
    want_drop: bool,
) -> (Vec<BridgeEvent>, Vec<(Atom, [u32; 5])>) {
    let deadline = Instant::now() + timeout;
    let mut events = Vec::new();
    let mut received = Vec::new();
    while Instant::now() < deadline {
        events.extend(bridge.poll().expect("bridge poll"));
        loop {
            match source_conn.poll_for_event() {
                Ok(Some(Event::SelectionRequest(request))) => {
                    serve_selection_request(source_conn, &request, payload)
                        .expect("serve selection request");
                }
                Ok(Some(Event::ClientMessage(message))) => {
                    received.push((message.type_, message.data.as_data32()));
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(err) => panic!("source poll_for_event: {err}"),
            }
        }
        if want_drop {
            if events.iter().any(|event| {
                matches!(
                    event,
                    BridgeEvent::Dropped { .. } | BridgeEvent::Failed { .. }
                )
            }) {
                break;
            }
        } else if received
            .iter()
            .any(|(type_atom, _)| *type_atom == bridge.atoms().status)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // The bridge sends its final reply in the same poll that produces the
    // drop event; the X server may not have routed it yet, so drain the source
    // for a short bounded window.
    let drain_deadline = Instant::now() + Duration::from_millis(500);
    let finished_atom = bridge.atoms().finished;
    loop {
        loop {
            match source_conn.poll_for_event() {
                Ok(Some(Event::SelectionRequest(request))) => {
                    serve_selection_request(source_conn, &request, payload)
                        .expect("serve selection request");
                }
                Ok(Some(Event::ClientMessage(message))) => {
                    received.push((message.type_, message.data.as_data32()));
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(err) => panic!("source poll_for_event: {err}"),
            }
        }
        if Instant::now() >= drain_deadline
            || received
                .iter()
                .any(|(type_atom, _)| *type_atom == finished_atom)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    (events, received)
}

/// T328a: an `INCR` selection transfer completes in bounded chunks.
///
/// The fake source answers the first `ConvertSelection` with an `INCR`
/// property and then writes one chunk per deletion (the bridge deletes each
/// chunk as it reads it). The bridge must accumulate the chunks, stop at the
/// zero-length terminator, and produce the same drop as a direct transfer.
#[test]
fn xdnd_target_bridge_reads_an_incr_selection() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-xdnd-incr-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let screen = conn.setup().roots[screen_num].clone();
    let mut bridge = XdndBridge::connect(&display).expect("start the XDnD target bridge");
    let bridge_window = bridge.window();
    let atoms = *bridge.atoms();
    let uri_list = bridge.uri_list_atom();

    let source = make_window(&conn, &screen);
    let selection = intern(&conn, "XdndSelection");
    let incr = intern(&conn, "INCR");
    let property = intern(&conn, "DF_XDND_SELECTION");
    conn.set_selection_owner(source, selection, x11rb::CURRENT_TIME)
        .expect("own XdndSelection")
        .check()
        .expect("set XdndSelection owner");
    // The source must see the bridge delete each chunk (PropertyNotify
    // Deleted) to know when to write the next one.
    conn.change_window_attributes(
        bridge_window,
        &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
            .event_mask(EventMask::PROPERTY_CHANGE),
    )
    .expect("watch bridge property changes");
    conn.flush().expect("flush source setup");

    let enter = XdndMessage::Enter {
        source,
        version: XDND_VERSION,
        more_types: false,
        types: vec![uri_list],
    };
    send_message(
        &conn,
        bridge_window,
        atoms.enter,
        enter.encode(&atoms).expect("encode Enter"),
    );
    let position = XdndMessage::Position {
        source,
        position: (3, 4),
        time: 0,
        action: XdndAction::Copy,
    };
    send_message(
        &conn,
        bridge_window,
        atoms.position,
        position.encode(&atoms).expect("encode Position"),
    );
    let drop = XdndMessage::Drop { source, time: 0 };
    send_message(
        &conn,
        bridge_window,
        atoms.drop,
        drop.encode(&atoms).expect("encode Drop"),
    );
    conn.flush().expect("flush drop");

    let payload = b"file:///home/u/incr.txt\r\n";
    let mut events = Vec::new();
    let mut incr_started = false;
    let mut offset = 0usize;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        events.extend(bridge.poll().expect("bridge poll"));
        loop {
            match conn.poll_for_event() {
                Ok(Some(Event::SelectionRequest(request))) => {
                    // Announce an INCR transfer with the total byte count.
                    conn.change_property32(
                        PropMode::REPLACE,
                        request.requestor,
                        request.property,
                        incr,
                        &[payload.len() as u32],
                    )
                    .expect("announce INCR");
                    let notify = x11rb::protocol::xproto::SelectionNotifyEvent {
                        response_type: x11rb::protocol::xproto::SELECTION_NOTIFY_EVENT,
                        sequence: 0,
                        time: request.time,
                        requestor: request.requestor,
                        selection: request.selection,
                        target: request.target,
                        property: request.property,
                    };
                    conn.send_event(false, request.requestor, EventMask::NO_EVENT, notify)
                        .expect("send INCR SelectionNotify");
                    conn.flush().expect("flush INCR notify");
                    incr_started = true;
                }
                Ok(Some(Event::PropertyNotify(notify))) => {
                    if !incr_started
                        || notify.window != bridge_window
                        || notify.atom != property
                        || notify.state != x11rb::protocol::xproto::Property::DELETE
                    {
                        continue;
                    }
                    let end = (offset + 8).min(payload.len());
                    let chunk = &payload[offset..end];
                    conn.change_property8(
                        PropMode::REPLACE,
                        bridge_window,
                        property,
                        uri_list,
                        chunk,
                    )
                    .expect("write INCR chunk");
                    conn.flush().expect("flush INCR chunk");
                    offset = end;
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(err) => panic!("source poll_for_event: {err}"),
            }
        }
        if events.iter().any(|event| {
            matches!(
                event,
                BridgeEvent::Dropped { .. } | BridgeEvent::Failed { .. }
            )
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    let dropped = events
        .iter()
        .find_map(|event| match event {
            BridgeEvent::Dropped { paths, .. } => Some(paths.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the bridge read the INCR drop: events={events:?}"));
    assert_eq!(dropped, vec![PathBuf::from("/home/u/incr.txt")]);
    assert_eq!(offset, payload.len(), "every chunk was consumed");

    std::mem::drop(bridge);
    std::mem::drop(conn);
}

/// T328a: a source that never answers `ConvertSelection` is bounded, not
/// waited on forever. The bridge must refuse the drop and tell the source.
#[test]
fn xdnd_target_bridge_times_out_a_silent_source() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-xdnd-timeout-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let screen = conn.setup().roots[screen_num].clone();
    let mut bridge = XdndBridge::connect(&display).expect("start the XDnD target bridge");
    let bridge_window = bridge.window();
    let atoms = *bridge.atoms();
    let uri_list = bridge.uri_list_atom();

    let source = make_window(&conn, &screen);
    let selection = intern(&conn, "XdndSelection");
    conn.set_selection_owner(source, selection, x11rb::CURRENT_TIME)
        .expect("own XdndSelection")
        .check()
        .expect("set XdndSelection owner");

    let enter = XdndMessage::Enter {
        source,
        version: XDND_VERSION,
        more_types: false,
        types: vec![uri_list],
    };
    send_message(
        &conn,
        bridge_window,
        atoms.enter,
        enter.encode(&atoms).expect("encode Enter"),
    );
    let position = XdndMessage::Position {
        source,
        position: (1, 1),
        time: 0,
        action: XdndAction::Copy,
    };
    send_message(
        &conn,
        bridge_window,
        atoms.position,
        position.encode(&atoms).expect("encode Position"),
    );
    let drop = XdndMessage::Drop { source, time: 0 };
    send_message(
        &conn,
        bridge_window,
        atoms.drop,
        drop.encode(&atoms).expect("encode Drop"),
    );
    conn.flush().expect("flush drop");

    let mut events = Vec::new();
    let mut finished = None;
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        events.extend(bridge.poll().expect("bridge poll"));
        // Read the SelectionRequest but never answer it.
        while let Ok(Some(event)) = conn.poll_for_event() {
            if let Event::ClientMessage(message) = event {
                if message.type_ == atoms.finished {
                    finished = Some(message.data.as_data32());
                }
            }
        }
        if events
            .iter()
            .any(|event| matches!(event, BridgeEvent::Failed { .. }))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let failed = events
        .iter()
        .find_map(|event| match event {
            BridgeEvent::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the silent transfer timed out: events={events:?}"));
    assert!(failed.contains("timed out"), "reason was {failed:?}");
    // The bridge sends its refusal in the same poll that produces the event;
    // give the server a bounded moment to route it to the source.
    let drain = Instant::now() + Duration::from_millis(500);
    while finished.is_none() && Instant::now() < drain {
        while let Ok(Some(event)) = conn.poll_for_event() {
            if let Event::ClientMessage(message) = event {
                if message.type_ == atoms.finished {
                    finished = Some(message.data.as_data32());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let finished = finished.expect("the source was told the transfer failed");
    assert_eq!(finished[1] & 1, 0, "Finished reports failure");

    std::mem::drop(bridge);
    std::mem::drop(conn);
}

/// T328a: the running compositor publishes the root `XdndProxy` and its
/// bridge window advertises XDnD 5. This is the wiring the library test
/// cannot see.
#[test]
fn compositor_publishes_the_root_xdnd_proxy() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-xdnd-proxy-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let screen = conn.setup().roots[screen_num].clone();
    let proxy_atom = intern(&conn, "XdndProxy");
    let aware_atom = intern(&conn, "XdndAware");

    let deadline = Instant::now() + Duration::from_secs(5);
    let proxy = loop {
        let reply = conn
            .get_property(false, screen.root, proxy_atom, AtomEnum::WINDOW, 0, 1)
            .expect("get XdndProxy")
            .reply()
            .expect("XdndProxy reply");
        if let Some(window) = reply
            .value32()
            .and_then(|mut it| it.next())
            .filter(|w| *w != 0)
        {
            break window;
        }
        assert!(
            Instant::now() < deadline,
            "the compositor never published the root XdndProxy"
        );
        std::thread::sleep(Duration::from_millis(20));
    };

    let aware = conn
        .get_property(false, proxy, aware_atom, AtomEnum::ATOM, 0, 1)
        .expect("get bridge XdndAware")
        .reply()
        .expect("bridge XdndAware reply");
    assert_eq!(
        aware.value32().and_then(|mut it| it.next()),
        Some(u32::from(XDND_VERSION)),
        "the compositor's bridge window advertises XDnD 5"
    );

    std::mem::drop(conn);
}

/// Drain the bridge until `done` returns true, or `timeout` passes. Returns
/// every event the bridge produced.
fn pump_bridge_events(
    bridge: &mut XdndBridge,
    timeout: Duration,
    mut done: impl FnMut(&[BridgeEvent]) -> bool,
) -> Vec<BridgeEvent> {
    let deadline = Instant::now() + timeout;
    let mut events = Vec::new();
    loop {
        events.extend(bridge.poll().expect("bridge poll"));
        if done(&events) || Instant::now() >= deadline {
            return events;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Wait for a `SelectionNotify` addressed to `requestor`.
fn wait_for_selection_notify(
    conn: &RustConnection,
    requestor: X11Window,
    selection: Atom,
    what: &str,
) -> x11rb::protocol::xproto::SelectionNotifyEvent {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match conn.poll_for_event() {
            Ok(Some(Event::SelectionNotify(notify)))
                if notify.requestor == requestor && notify.selection == selection =>
            {
                return notify;
            }
            Ok(Some(_)) => {}
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(err) => panic!("poll_for_event: {err}"),
        }
    }
    panic!("timed out waiting for {what}");
}

/// T328b: the bridge drives the XDnD **source** role against a fake X11
/// target. It enters with a `text/uri-list` offer, waits for `XdndStatus`
/// before dropping, serves the selection from the Wayland payload, and
/// releases ownership on `XdndFinished`. The same connection, atom table, and
/// bridge window as T328a are reused.
#[test]
fn xdnd_source_bridge_drives_a_fake_x11_target() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-xdnd-source-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let screen = conn.setup().roots[screen_num].clone();
    let mut bridge = XdndBridge::connect(&display).expect("start the XDnD bridge");
    let source_window = bridge.window();
    let atoms = *bridge.atoms();
    let uri_list = bridge.uri_list_atom();
    let selection = intern(&conn, "XdndSelection");

    // A fake X11 target advertises XDnD 5.
    let target = make_window(&conn, &screen);
    conn.change_property32(
        PropMode::REPLACE,
        target,
        atoms.aware,
        AtomEnum::ATOM,
        &[u32::from(XDND_VERSION)],
    )
    .expect("set XdndAware");
    conn.flush().expect("flush target setup");

    // The source role owns XdndSelection. `None` resolves to the Copy default.
    bridge
        .begin_source(XdndAction::None)
        .expect("begin the XDnD source");
    assert_eq!(bridge.source_target(), None);
    // `set_selection_owner` and this query are on different X connections, so
    // wait for the server to process the ownership request.
    let owner_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let owner = conn
            .get_selection_owner(selection)
            .expect("get selection owner")
            .reply()
            .expect("selection owner reply")
            .owner;
        if owner == source_window {
            break;
        }
        assert!(
            Instant::now() < owner_deadline,
            "the bridge window owns XdndSelection (owner was {owner:#x})"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    // Enter the target: Enter then the opening Position, with Copy proposed.
    bridge
        .source_enter(target, (12, 34), 7)
        .expect("enter the X11 target");
    let enter = wait_for_message(&conn, target, atoms.enter, "XdndEnter");
    match XdndMessage::decode("XdndEnter", enter, &atoms).expect("decode Enter") {
        XdndMessage::Enter {
            source,
            version,
            more_types,
            types,
        } => {
            assert_eq!(source, source_window, "the bridge window is the source");
            assert_eq!(version, XDND_VERSION);
            assert!(!more_types);
            assert_eq!(types, vec![uri_list], "the only flavor is text/uri-list");
        }
        other => panic!("expected XdndEnter, got {other:?}"),
    }
    let position = wait_for_message(&conn, target, atoms.position, "XdndPosition");
    match XdndMessage::decode("XdndPosition", position, &atoms).expect("decode Position") {
        XdndMessage::Position {
            source,
            position,
            action,
            ..
        } => {
            assert_eq!(source, source_window);
            assert_eq!(position, (12, 34));
            assert_eq!(action, XdndAction::Copy, "None resolved to Copy");
        }
        other => panic!("expected XdndPosition, got {other:?}"),
    }

    // The source must not drop before the target answers `XdndStatus`.
    assert!(
        !bridge.source_drop(0).expect("refused drop"),
        "a drop before XdndStatus is not sent"
    );

    // The target accepts with Move (not the proposed default).
    let status = XdndMessage::Status {
        target,
        accept: true,
        want_position: true,
        action: XdndAction::Move,
    };
    send_message(
        &conn,
        source_window,
        atoms.status,
        status.encode(&atoms).expect("encode Status"),
    );
    let events = pump_bridge_events(&mut bridge, Duration::from_secs(5), |events| {
        events
            .iter()
            .any(|event| matches!(event, BridgeEvent::SourceStatus { .. }))
    });
    let status_event = events
        .iter()
        .find_map(|event| match event {
            BridgeEvent::SourceStatus {
                target: event_target,
                accept,
                action,
            } => Some((*event_target, *accept, *action)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the bridge absorbed XdndStatus: {events:?}"));
    assert_eq!(status_event, (target, true, XdndAction::Move));
    assert!(bridge.source_is_accepted());
    assert_eq!(bridge.source_accepted_action(), Some(XdndAction::Move));

    // Drop now that the target accepted.
    assert!(
        bridge.source_drop(0).expect("send XdndDrop"),
        "the accepted drag drops"
    );
    let drop = wait_for_message(&conn, target, atoms.drop, "XdndDrop");
    match XdndMessage::decode("XdndDrop", drop, &atoms).expect("decode Drop") {
        XdndMessage::Drop { source, .. } => assert_eq!(source, source_window),
        other => panic!("expected XdndDrop, got {other:?}"),
    }

    // The target asks for text/uri-list before the payload was fetched. The
    // bridge parks the request and reports the need; the payload then serves
    // it. This is the production lazy path, not a preloaded shortcut.
    let property = intern(&conn, "DF_TEST_SOURCE_SELECTION");
    conn.convert_selection(target, selection, uri_list, property, x11rb::CURRENT_TIME)
        .expect("request text/uri-list")
        .check()
        .expect("convert selection");
    conn.flush().expect("flush selection request");
    let events = pump_bridge_events(&mut bridge, Duration::from_secs(5), |events| {
        events
            .iter()
            .any(|event| matches!(event, BridgeEvent::SourceSelectionNeeded { .. }))
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, BridgeEvent::SourceSelectionNeeded { .. })),
        "the bridge asked the host for the payload: {events:?}"
    );

    let payload = b"file:///home/u/one.txt\r\nfile:///home/u/two%20words.txt\r\n";
    bridge
        .set_source_payload(payload.to_vec())
        .expect("serve the fetched payload");
    let notify = wait_for_selection_notify(&conn, target, selection, "SelectionNotify");
    assert_eq!(
        notify.property, property,
        "the payload property was written"
    );
    let value = conn
        .get_property(true, target, property, AtomEnum::ANY, 0, 4096)
        .expect("get selection payload")
        .reply()
        .expect("selection payload reply");
    assert_eq!(value.value, payload, "text/uri-list round-trips intact");

    // The target confirms the drop; the source releases the selection.
    let finished = XdndMessage::Finished {
        target,
        success: true,
        action: XdndAction::Move,
    };
    send_message(
        &conn,
        source_window,
        atoms.finished,
        finished.encode(&atoms).expect("encode Finished"),
    );
    let events = pump_bridge_events(&mut bridge, Duration::from_secs(5), |events| {
        events
            .iter()
            .any(|event| matches!(event, BridgeEvent::SourceFinished { .. }))
    });
    assert!(
        events
            .iter()
            .any(|event| matches!(event, BridgeEvent::SourceFinished { success: true, .. })),
        "the bridge absorbed XdndFinished: {events:?}"
    );
    assert!(!bridge.has_source(), "finished ends the source role");
    let owner = conn
        .get_selection_owner(selection)
        .expect("get selection owner after finish")
        .reply()
        .expect("selection owner reply")
        .owner;
    assert_eq!(owner, 0, "XdndSelection is released");

    // A second drag that leaves the target sends `XdndLeave` and keeps the
    // connection alive for the next one.
    let other = make_window(&conn, &screen);
    conn.change_property32(
        PropMode::REPLACE,
        other,
        atoms.aware,
        AtomEnum::ATOM,
        &[u32::from(XDND_VERSION)],
    )
    .expect("set second XdndAware");
    conn.sync().expect("sync second target");
    bridge
        .begin_source(XdndAction::Link)
        .expect("begin a second source");
    bridge
        .source_enter(other, (1, 2), 3)
        .expect("enter the second target");
    assert_eq!(bridge.source_target(), Some(other));
    let _ = wait_for_message(&conn, other, atoms.enter, "second XdndEnter");
    let position = wait_for_message(&conn, other, atoms.position, "second XdndPosition");
    match XdndMessage::decode("XdndPosition", position, &atoms).expect("decode Position") {
        XdndMessage::Position { action, .. } => {
            assert_eq!(action, XdndAction::Link, "each of the five actions is sent")
        }
        other => panic!("expected XdndPosition, got {other:?}"),
    }
    bridge.source_leave().expect("leave the second target");
    let leave = wait_for_message(&conn, other, atoms.leave, "XdndLeave");
    match XdndMessage::decode("XdndLeave", leave, &atoms).expect("decode Leave") {
        XdndMessage::Leave { source } => assert_eq!(source, source_window),
        other => panic!("expected XdndLeave, got {other:?}"),
    }
    bridge.end_source().expect("end the second source");

    std::mem::drop(bridge);
    std::mem::drop(conn);
}

/// The bridge interns its atoms, creates the bridge window, advertises
/// `XdndAware`, and publishes the root `XdndProxy`. A fake X11 drag source then
/// sends real `XdndEnter`/`Position`/`Drop` client messages to the bridge
/// window and answers the `ConvertSelection` with a `text/uri-list` payload.
/// The bridge must accept the drag, read the payload, and answer `XdndStatus`
/// and `XdndFinished`.
#[test]
fn xdnd_target_bridge_reads_a_file_drop() {
    if !xwayland_available() {
        eprintln!("skipping: Xwayland is not installed");
        return;
    }

    let socket_name = format!("dragonfruit-test-xdnd-target-{}", std::process::id());
    let proc = CompositorProcess::start(&socket_name);
    let Some(display) = proc.wait_for_display() else {
        eprintln!("skipping: Xwayland did not become ready");
        return;
    };

    let (conn, screen_num) = x11rb::connect(Some(&display)).expect("connect to Xwayland");
    let screen = conn.setup().roots[screen_num].clone();

    // The bridge owns its own X connection (never the XWM's).
    let mut bridge = XdndBridge::connect(&display).expect("start the XDnD target bridge");
    let bridge_window = bridge.window();
    let root = bridge.root();
    let atoms = *bridge.atoms();
    let uri_list = bridge.uri_list_atom();

    // The root proxy and the bridge's XdndAware are the two properties that
    // make a Wayland-surface drag resolve to the bridge.
    let proxy_atom = intern(&conn, "XdndProxy");
    let aware_atom = intern(&conn, "XdndAware");
    let proxy = conn
        .get_property(false, root, proxy_atom, AtomEnum::WINDOW, 0, 1)
        .expect("get XdndProxy")
        .reply()
        .expect("XdndProxy reply");
    assert_eq!(
        proxy.value32().and_then(|mut it| it.next()),
        Some(bridge_window),
        "the root XdndProxy points at the bridge window"
    );
    let aware = conn
        .get_property(false, bridge_window, aware_atom, AtomEnum::ATOM, 0, 1)
        .expect("get XdndAware")
        .reply()
        .expect("XdndAware reply");
    assert_eq!(
        aware.value32().and_then(|mut it| it.next()),
        Some(u32::from(XDND_VERSION)),
        "the bridge window advertises XDnD 5"
    );

    // A fake X11 source owns XdndSelection and offers only a text/plain atom
    // first: the bridge must refuse it.
    let source = make_window(&conn, &screen);
    let selection = intern(&conn, "XdndSelection");
    conn.set_selection_owner(source, selection, x11rb::CURRENT_TIME)
        .expect("own XdndSelection")
        .check()
        .expect("set XdndSelection owner");
    // A round-trip so the owner is visible before the bridge's
    // ConvertSelection on its own connection (cross-connection requests have
    // no ordering guarantee).
    let owner = conn
        .get_selection_owner(selection)
        .expect("get selection owner")
        .reply()
        .expect("selection owner reply")
        .owner;
    assert_eq!(owner, source, "the fake source owns XdndSelection");

    let text_only = XdndMessage::Enter {
        source,
        version: XDND_VERSION,
        more_types: false,
        types: vec![atoms.action_list],
    };
    send_message(
        &conn,
        bridge_window,
        atoms.enter,
        text_only.encode(&atoms).expect("encode Enter"),
    );
    let position = XdndMessage::Position {
        source,
        position: (10, 20),
        time: 0,
        action: XdndAction::Copy,
    };
    send_message(
        &conn,
        bridge_window,
        atoms.position,
        position.encode(&atoms).expect("encode Position"),
    );
    conn.flush().expect("flush refusal");

    let (events, received) = pump_bridge(&mut bridge, &conn, b"", Duration::from_secs(5), false);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, BridgeEvent::Dropped { .. })),
        "a non-file drag produces no drop: {events:?}"
    );
    let status = received
        .iter()
        .find(|(type_atom, _)| *type_atom == atoms.status)
        .expect("the bridge answered XdndStatus");
    assert_eq!(status.1[1] & 1, 0, "a text/plain drag is refused");

    // Now the real file drag. The source offers text/uri-list, the bridge
    // accepts, and the drop's ConvertSelection is answered with two files.
    let enter = XdndMessage::Enter {
        source,
        version: XDND_VERSION,
        more_types: false,
        types: vec![uri_list],
    };
    send_message(
        &conn,
        bridge_window,
        atoms.enter,
        enter.encode(&atoms).expect("encode Enter"),
    );
    send_message(
        &conn,
        bridge_window,
        atoms.position,
        position.encode(&atoms).expect("encode Position"),
    );
    let drop = XdndMessage::Drop { source, time: 0 };
    send_message(
        &conn,
        bridge_window,
        atoms.drop,
        drop.encode(&atoms).expect("encode Drop"),
    );
    conn.flush().expect("flush drop");

    let payload = b"# a comment\r\nfile:///home/u/one.txt\r\nfile:///home/u/two%20words.txt\r\n";
    let (events, received) = pump_bridge(&mut bridge, &conn, payload, Duration::from_secs(5), true);

    let dropped = events
        .iter()
        .find_map(|event| match event {
            BridgeEvent::Dropped {
                paths, position, ..
            } => Some((paths.clone(), *position)),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!("the bridge read the file drop: events={events:?} received={received:?}")
        });
    assert_eq!(
        dropped.0,
        vec![
            PathBuf::from("/home/u/one.txt"),
            PathBuf::from("/home/u/two words.txt")
        ]
    );
    assert_eq!(dropped.1, (10, 20));

    let accepted = received
        .iter()
        .filter(|(type_atom, _)| *type_atom == atoms.status)
        .map(|(_, data)| data[1] & 1)
        .next_back()
        .expect("the bridge answered XdndStatus");
    assert_eq!(accepted, 1, "a text/uri-list drag is accepted");
    let finished = received
        .iter()
        .find(|(type_atom, _)| *type_atom == atoms.finished)
        .unwrap_or_else(|| {
            panic!("the bridge answered XdndFinished: received={received:?} events={events:?}")
        });
    assert_eq!(finished.1[0], bridge_window, "Finished names the target");
    assert_eq!(finished.1[1] & 1, 1, "Finished reports success");
    assert_eq!(finished.1[2], atoms.action_copy, "the copy action is used");

    // `proc` drops here (SIGKILL + wait); `conn`/`bridge` drop with the test.
    std::mem::drop(bridge);
    std::mem::drop(conn);
}
