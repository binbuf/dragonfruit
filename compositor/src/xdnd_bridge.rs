// SPDX-License-Identifier: MIT
//! The runtime half of the XDnD bridge: a dedicated X11 connection, the atom
//! table, and the bridge window that acts as the X11 **target** (T328a).
//!
//! [`crate::xdnd`] owns the wire codec and the pure [`XdndTarget`] state
//! machine. This module owns the X connection those decisions are carried on.
//! It is deliberately independent of `DfState`/Smithay so the conformance test
//! can drive it against a real Xwayland server with a fake X11 drag source.
//!
//! ## How an X11 drag reaches the bridge
//!
//! XDnD sources resolve the window under the pointer and send their client
//! messages there. A Wayland surface has no X11 window, so over one the source
//! resolves to Xwayland's root window. The bridge publishes itself as the root
//! window's `XdndProxy`; a source that finds the root window not
//! `XdndAware` follows the proxy and sends `XdndEnter`/`Position`/`Drop` to the
//! bridge window. X11 windows keep receiving their own drags directly, so
//! X11→X11 drag-and-drop is untouched.
//!
//! This is the only intercept point the X protocol offers without becoming a
//! second window manager. An X11 window that is geometrically under the
//! pointer (even when a Wayland surface is drawn above it) still receives its
//! own drag; that boundary is recorded in the hand-off and ADR 0334.
//!
//! ## Flow
//!
//! - `XdndEnter` → the [`XdndTarget`] state machine starts a drag.
//! - `XdndPosition` → the bridge answers `XdndStatus` (a `text/uri-list` drag
//!   is accepted with `XdndActionCopy`, anything else refused).
//! - `XdndDrop` → `ConvertSelection(XdndSelection, text/uri-list)` on the
//!   source; `SelectionNotify`/`PropertyNotify` (including INCR) deliver the
//!   payload; the bridge parses the `file://` list, answers `XdndFinished`,
//!   and returns [`BridgeEvent::Dropped`] with local paths.
//! - `XdndLeave` → the drag is discarded.
//!
//! Transfers are deadline-bounded and size-bounded so a wedged or hostile X11
//! peer cannot pin the compositor. Nothing here blocks on the peer: only the
//! X server is round-tripped (property reads), and the calloop source is
//! drained from [`XdndBridge::poll`].

use std::os::fd::{AsFd, OwnedFd};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use smithay::reexports::x11rb::connection::Connection;
use smithay::reexports::x11rb::protocol::xproto::{
    Atom, AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConnectionExt as _,
    CreateWindowAux, EventMask, PropMode, SelectionNotifyEvent, Window, WindowClass,
};
use smithay::reexports::x11rb::protocol::Event;
use smithay::reexports::x11rb::rust_connection::RustConnection;
use smithay::reexports::x11rb::wrapper::ConnectionExt as _;

use crate::xdnd::{
    parse_uri_list, uris_to_paths, XdndAction, XdndAtoms, XdndMessage, XdndStatus, XdndTarget,
    URI_LIST_MIME, XDND_VERSION,
};

/// How long a `ConvertSelection`/INCR transfer may take before the bridge
/// gives up and refuses the drop.
pub const TRANSFER_TIMEOUT: Duration = Duration::from_secs(2);

/// Upper bound on one `text/uri-list` payload. A file list is tiny; the cap
/// only exists so a broken peer cannot make the compositor grow unbounded.
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

/// The private property the bridge asks the source to write the selection
/// into. It lives on the bridge window.
const SELECTION_PROPERTY: &str = "DF_XDND_SELECTION";

/// What the bridge observed on the X connection. The host only acts on
/// [`BridgeEvent::Dropped`]; the rest is available for diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeEvent {
    /// A source started a drag over a Wayland surface.
    Enter {
        source: u32,
        version: u8,
        type_atoms: Vec<u32>,
    },
    /// A source moved; `accepted` is what the bridge told it.
    Position {
        source: u32,
        position: (i16, i16),
        accepted: bool,
    },
    /// The source left without dropping.
    Leave,
    /// A drop was accepted and the selection transfer is in flight. The host
    /// arms a bounded wakeup so [`XdndBridge::poll`] can expire it even when
    /// the X connection is silent.
    TransferStarted { source: u32 },
    /// A file drop was read completely. `paths` is never empty.
    Dropped {
        source: u32,
        position: (i16, i16),
        action: XdndAction,
        paths: Vec<PathBuf>,
    },
    /// A drop was refused or its transfer failed. The source has been told.
    Failed { source: u32, reason: String },
}

#[derive(Debug)]
struct PendingTransfer {
    source: u32,
    position: (i16, i16),
    action: XdndAction,
    deadline: Instant,
    buffer: Vec<u8>,
    /// Set once the source answered with an `INCR` property.
    incr: bool,
}

/// The dedicated X11 connection and bridge window for the target role.
pub struct XdndBridge {
    conn: RustConnection,
    root: Window,
    window: Window,
    atoms: XdndAtoms,
    uri_list: Atom,
    property: Atom,
    incr: Atom,
    target: XdndTarget,
    pending: Option<PendingTransfer>,
}

impl std::fmt::Debug for XdndBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XdndBridge")
            .field("root", &self.root)
            .field("window", &self.window)
            .field("atoms", &self.atoms)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl XdndBridge {
    /// Connect to `display`, intern the atom table, create the bridge window,
    /// advertise `XdndAware`, and publish the root `XdndProxy`.
    ///
    /// The connection is the bridge's own; it is never the XWM's connection.
    pub fn connect(display: &str) -> Result<Self, String> {
        let (conn, screen_num) =
            RustConnection::connect(Some(display)).map_err(|err| format!("connect: {err}"))?;
        let screen = conn
            .setup()
            .roots
            .get(screen_num)
            .ok_or_else(|| "X screen missing".to_string())?
            .clone();
        let root = screen.root;

        let atoms = intern_xdnd_atoms(&conn)?;
        let uri_list = intern(&conn, URI_LIST_MIME)?;
        let property = intern(&conn, SELECTION_PROPERTY)?;
        let incr = intern(&conn, "INCR")?;

        let window = conn
            .generate_id()
            .map_err(|err| format!("generate bridge window id: {err}"))?;
        // An override-redirect, unmapped, input-only 1×1 window: it exists to
        // own properties and receive client messages, and is invisible to the
        // WM and to the user.
        let aux = CreateWindowAux::new().override_redirect(1);
        conn.create_window(
            0, // InputOnly windows have depth 0.
            window,
            root,
            -1,
            -1,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            0, // COPY_FROM_PARENT
            &aux,
        )
        .map_err(|err| format!("create bridge window: {err}"))?;
        conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(|err| format!("select bridge window events: {err}"))?;

        // XdndAware on the bridge window, XdndProxy on the root. A source that
        // resolves the pointer to the root follows the proxy to us.
        conn.change_property32(
            PropMode::REPLACE,
            window,
            atoms.aware,
            AtomEnum::ATOM,
            &[u32::from(XDND_VERSION)],
        )
        .map_err(|err| format!("set XdndAware: {err}"))?;
        conn.change_property32(
            PropMode::REPLACE,
            root,
            atoms.proxy,
            AtomEnum::WINDOW,
            &[window],
        )
        .map_err(|err| format!("set XdndProxy on root: {err}"))?;
        conn.sync()
            .map_err(|err| format!("sync bridge setup: {err}"))?;

        Ok(Self {
            conn,
            root,
            window,
            atoms,
            uri_list,
            property,
            incr,
            target: XdndTarget::new(),
            pending: None,
        })
    }

    /// The bridge window id (the X11 target the source talks to).
    pub fn window(&self) -> Window {
        self.window
    }

    /// Xwayland's root window.
    pub fn root(&self) -> Window {
        self.root
    }

    /// The interned atom table.
    pub fn atoms(&self) -> &XdndAtoms {
        &self.atoms
    }

    /// The interned `text/uri-list` atom.
    pub fn uri_list_atom(&self) -> Atom {
        self.uri_list
    }

    /// Whether a transfer is in flight (for tests and diagnostics).
    pub fn has_pending_transfer(&self) -> bool {
        self.pending.is_some()
    }

    /// A duplicate of the X connection fd, for a calloop readiness source.
    /// The bridge keeps ownership of the connection; the duplicate is only
    /// polled for readability and never read.
    pub fn duplicate_fd(&self) -> std::io::Result<OwnedFd> {
        self.conn.stream().as_fd().try_clone_to_owned()
    }

    /// Drain every queued X event and return what the host should act on.
    ///
    /// This is the only entry point the compositor's event loop calls. It
    /// never waits on the peer; the only round-trips are property reads from
    /// the X server, which have already been ordered by the source.
    pub fn poll(&mut self) -> Result<Vec<BridgeEvent>, String> {
        let mut events = Vec::new();
        loop {
            match self.conn.poll_for_event() {
                Ok(Some(event)) => self.handle_event(event, &mut events)?,
                Ok(None) => break,
                Err(err) => return Err(format!("poll X events: {err}")),
            }
        }
        self.expire_transfer(&mut events)?;
        self.conn
            .flush()
            .map_err(|err| format!("flush X events: {err}"))?;
        Ok(events)
    }

    fn handle_event(&mut self, event: Event, events: &mut Vec<BridgeEvent>) -> Result<(), String> {
        match event {
            Event::ClientMessage(message) => {
                if message.format != 32 || message.window != self.window {
                    return Ok(());
                }
                let Some(name) = self.atoms.message_type_name(message.type_) else {
                    return Ok(());
                };
                self.handle_client_message(name, message.data.as_data32(), events)
            }
            Event::SelectionNotify(notify) => self.handle_selection_notify(notify, events),
            Event::PropertyNotify(notify) => {
                if notify.window == self.window
                    && notify.atom == self.property
                    && notify.state
                        == smithay::reexports::x11rb::protocol::xproto::Property::NEW_VALUE
                {
                    self.read_incr_chunk(events)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_client_message(
        &mut self,
        name: &'static str,
        data: [u32; 5],
        events: &mut Vec<BridgeEvent>,
    ) -> Result<(), String> {
        match name {
            "XdndEnter" => {
                let Some(XdndMessage::Enter {
                    source,
                    version,
                    more_types,
                    types,
                }) = XdndMessage::decode(name, data, &self.atoms)
                else {
                    return Ok(());
                };
                let type_atoms = if more_types {
                    self.read_type_list(source)?
                } else {
                    types
                };
                // A fresh enter replaces any drag in flight.
                self.pending = None;
                self.target.enter(source, version, type_atoms.clone());
                events.push(BridgeEvent::Enter {
                    source,
                    version,
                    type_atoms,
                });
                Ok(())
            }
            "XdndPosition" => {
                let Some(XdndMessage::Position {
                    source,
                    position,
                    action,
                    ..
                }) = XdndMessage::decode(name, data, &self.atoms)
                else {
                    return Ok(());
                };
                let status = self.target.position(position, action, self.uri_list);
                self.send_status(source, status)?;
                events.push(BridgeEvent::Position {
                    source,
                    position,
                    accepted: status.accept,
                });
                Ok(())
            }
            "XdndLeave" => {
                self.target.leave();
                self.pending = None;
                events.push(BridgeEvent::Leave);
                Ok(())
            }
            "XdndDrop" => {
                let Some(XdndMessage::Drop { source, time }) =
                    XdndMessage::decode(name, data, &self.atoms)
                else {
                    return Ok(());
                };
                let Some(request) = self.target.drop(time) else {
                    // Not accepted (no `text/uri-list`, or no live drag).
                    self.send_finished(source, false, XdndAction::None)?;
                    events.push(BridgeEvent::Failed {
                        source,
                        reason: "drop without an accepted text/uri-list drag".to_string(),
                    });
                    return Ok(());
                };
                self.begin_transfer(request.source, request.position, request.action, time)?;
                events.push(BridgeEvent::TransferStarted {
                    source: request.source,
                });
                Ok(())
            }
            // We are the target; status/finished belong to the source role
            // (T328b). Ignore them here.
            _ => Ok(()),
        }
    }

    fn begin_transfer(
        &mut self,
        source: u32,
        position: (i16, i16),
        action: XdndAction,
        time: u32,
    ) -> Result<(), String> {
        self.pending = Some(PendingTransfer {
            source,
            position,
            action,
            deadline: Instant::now() + TRANSFER_TIMEOUT,
            buffer: Vec::new(),
            incr: false,
        });
        self.conn
            .convert_selection(
                self.window,
                self.atoms.selection,
                self.uri_list,
                self.property,
                time,
            )
            .map_err(|err| format!("convert selection: {err}"))?;
        self.conn
            .flush()
            .map_err(|err| format!("flush convert selection: {err}"))
    }

    fn handle_selection_notify(
        &mut self,
        notify: SelectionNotifyEvent,
        events: &mut Vec<BridgeEvent>,
    ) -> Result<(), String> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(());
        };
        if notify.selection != self.atoms.selection {
            return Ok(());
        }
        let source = pending.source;
        if notify.property == 0 {
            self.fail_transfer(source, "the X11 source refused the selection", events)?;
            return Ok(());
        }
        self.read_property(events)
    }

    fn read_property(&mut self, events: &mut Vec<BridgeEvent>) -> Result<(), String> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(());
        };
        let source = pending.source;
        let reply = self
            .conn
            .get_property(
                true,
                self.window,
                self.property,
                AtomEnum::ANY,
                0,
                // `long_length` is in 32-bit units; ask for at most the cap.
                (MAX_PAYLOAD_BYTES / 4) as u32,
            )
            .map_err(|err| format!("get selection property: {err}"))?
            .reply()
            .map_err(|err| format!("get selection reply: {err}"))?;

        if reply.type_ == self.incr {
            if let Some(pending) = self.pending.as_mut() {
                pending.incr = true;
            }
            return Ok(());
        }
        if reply.bytes_after != 0 {
            self.fail_transfer(source, "the selection exceeded the payload cap", events)?;
            return Ok(());
        }
        if reply.format != 8 {
            self.fail_transfer(source, "the selection was not an 8-bit byte string", events)?;
            return Ok(());
        }
        self.append_bytes(&reply.value, events)
    }

    fn read_incr_chunk(&mut self, events: &mut Vec<BridgeEvent>) -> Result<(), String> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(());
        };
        if !pending.incr {
            return Ok(());
        }
        let source = pending.source;
        let reply = self
            .conn
            .get_property(
                true,
                self.window,
                self.property,
                AtomEnum::ANY,
                0,
                (MAX_PAYLOAD_BYTES / 4) as u32,
            )
            .map_err(|err| format!("get INCR chunk: {err}"))?
            .reply()
            .map_err(|err| format!("get INCR reply: {err}"))?;
        if reply.value.is_empty() {
            self.complete_transfer(events)?;
            return Ok(());
        }
        if reply.bytes_after != 0 {
            self.fail_transfer(source, "the selection exceeded the payload cap", events)?;
            return Ok(());
        }
        if reply.format != 8 {
            self.fail_transfer(source, "an INCR chunk was not 8-bit", events)?;
            return Ok(());
        }
        self.append_bytes(&reply.value, events)
    }

    fn append_bytes(&mut self, bytes: &[u8], events: &mut Vec<BridgeEvent>) -> Result<(), String> {
        let Some(pending) = self.pending.as_mut() else {
            return Ok(());
        };
        if pending.buffer.len() + bytes.len() > MAX_PAYLOAD_BYTES {
            let source = pending.source;
            self.fail_transfer(source, "the selection exceeded the payload cap", events)?;
            return Ok(());
        }
        pending.buffer.extend_from_slice(bytes);
        if !pending.incr {
            self.complete_transfer(events)?;
        }
        Ok(())
    }

    fn complete_transfer(&mut self, events: &mut Vec<BridgeEvent>) -> Result<(), String> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };
        let text = String::from_utf8_lossy(&pending.buffer);
        let uris = parse_uri_list(&text);
        let paths = uris_to_paths(&uris);
        if paths.is_empty() {
            self.send_finished(pending.source, false, XdndAction::None)?;
            events.push(BridgeEvent::Failed {
                source: pending.source,
                reason: "the selection held no local file:// URI".to_string(),
            });
            return Ok(());
        }
        // A file drop is a copy; the model's `position` already selected
        // `XdndActionCopy`.
        self.send_finished(pending.source, true, XdndAction::Copy)?;
        events.push(BridgeEvent::Dropped {
            source: pending.source,
            position: pending.position,
            action: pending.action,
            paths,
        });
        Ok(())
    }

    fn fail_transfer(
        &mut self,
        source: u32,
        reason: &str,
        events: &mut Vec<BridgeEvent>,
    ) -> Result<(), String> {
        self.pending = None;
        self.send_finished(source, false, XdndAction::None)?;
        events.push(BridgeEvent::Failed {
            source,
            reason: reason.to_string(),
        });
        Ok(())
    }

    fn expire_transfer(&mut self, events: &mut Vec<BridgeEvent>) -> Result<(), String> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(());
        };
        if Instant::now() < pending.deadline {
            return Ok(());
        }
        let source = pending.source;
        self.fail_transfer(source, "the selection transfer timed out", events)
    }

    fn read_type_list(&mut self, source: Window) -> Result<Vec<u32>, String> {
        let reply = self
            .conn
            .get_property(false, source, self.atoms.type_list, AtomEnum::ATOM, 0, 64)
            .map_err(|err| format!("get XdndTypeList: {err}"))?
            .reply()
            .map_err(|err| format!("get XdndTypeList reply: {err}"))?;
        Ok(reply
            .value32()
            .map(|atoms| atoms.collect())
            .unwrap_or_default())
    }

    fn send_status(&self, destination: Window, status: XdndStatus) -> Result<(), String> {
        let message = XdndMessage::Status {
            target: self.window,
            accept: status.accept,
            want_position: status.want_position,
            action: status.action,
        };
        self.send_client_message(destination, self.atoms.status, &message)
    }

    fn send_finished(
        &self,
        destination: Window,
        success: bool,
        action: XdndAction,
    ) -> Result<(), String> {
        let message = XdndMessage::Finished {
            target: self.window,
            success,
            action,
        };
        self.send_client_message(destination, self.atoms.finished, &message)
    }

    fn send_client_message(
        &self,
        destination: Window,
        type_atom: Atom,
        message: &XdndMessage,
    ) -> Result<(), String> {
        let data = message
            .encode(&self.atoms)
            .ok_or_else(|| "the message could not be encoded".to_string())?;
        let event = ClientMessageEvent::new(32, destination, type_atom, data);
        self.conn
            .send_event(false, destination, EventMask::NO_EVENT, event)
            .map_err(|err| format!("send XDnD message: {err}"))?;
        Ok(())
    }
}

fn intern(conn: &RustConnection, name: &str) -> Result<Atom, String> {
    conn.intern_atom(false, name.as_bytes())
        .map_err(|err| format!("intern {name}: {err}"))?
        .reply()
        .map(|reply| reply.atom)
        .map_err(|err| format!("intern {name} reply: {err}"))
}

fn intern_xdnd_atoms(conn: &RustConnection) -> Result<XdndAtoms, String> {
    let mut ids = [0u32; 17];
    for (slot, name) in ids.iter_mut().zip(XdndAtoms::names()) {
        *slot = intern(conn, name)?;
    }
    Ok(XdndAtoms {
        selection: ids[0],
        aware: ids[1],
        enter: ids[2],
        leave: ids[3],
        position: ids[4],
        status: ids[5],
        drop: ids[6],
        finished: ids[7],
        type_list: ids[8],
        action_list: ids[9],
        action_description: ids[10],
        action_copy: ids[11],
        action_move: ids[12],
        action_link: ids[13],
        action_ask: ids[14],
        action_private: ids[15],
        proxy: ids[16],
    })
}

/// Answer a `SelectionRequest` with a fixed `text/uri-list` payload. This is
/// the fake-source half of the conformance test; production sources do this
/// themselves.
pub fn serve_selection_request(
    conn: &RustConnection,
    request: &smithay::reexports::x11rb::protocol::xproto::SelectionRequestEvent,
    payload: &[u8],
) -> Result<(), String> {
    if request.property != 0 {
        conn.change_property8(
            PropMode::REPLACE,
            request.requestor,
            request.property,
            request.target,
            payload,
        )
        .map_err(|err| format!("serve selection property: {err}"))?;
    }
    let notify = SelectionNotifyEvent {
        response_type: smithay::reexports::x11rb::protocol::xproto::SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: request.time,
        requestor: request.requestor,
        selection: request.selection,
        target: request.target,
        property: request.property,
    };
    conn.send_event(false, request.requestor, EventMask::NO_EVENT, notify)
        .map_err(|err| format!("send SelectionNotify: {err}"))?;
    conn.flush()
        .map_err(|err| format!("flush SelectionNotify: {err}"))
}
