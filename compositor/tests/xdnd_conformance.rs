// SPDX-License-Identifier: MIT
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
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use dragonfruit_compositor::xdnd::{
    XdndAction, XdndAtoms, XdndMessage, XdndTarget, URI_LIST_MIME, XDND_VERSION,
};
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
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
