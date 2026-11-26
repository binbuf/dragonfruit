// SPDX-License-Identifier: GPL-3.0-only
//! The runtime half of the XDnD bridge: a dedicated X11 connection, the atom
//! table, and the bridge window that acts as the X11 **target** (T328a) and
//! the X11 **source** (T328b).
//!
//! [`crate::xdnd`] owns the wire codec and the pure [`XdndTarget`] /
//! [`XdndSource`] state machines. This module owns the X connection those
//! decisions are carried on. It is deliberately independent of
//! `DfState`/Smithay so the conformance test can drive it against a real
//! Xwayland server with a fake X11 peer.
//!
//! ## How an X11 drag reaches the bridge (target role)
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
//! ## How a Wayland drag reaches an X11 target (source role)
//!
//! The same connection and bridge window act as the source: the bridge owns
//! `XdndSelection` and sends `XdndEnter`/`Position`/`Drop`/`Leave` to the X11
//! window under the pointer. It waits for an accepting `XdndStatus` before
//! dropping, serves `text/uri-list` from the Wayland client's payload, and
//! releases the selection on `XdndFinished`/cancel/timeout. No second X11
//! connection or window exists; see ADR 0335.
//!
//! ## Target flow
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
    CreateWindowAux, EventMask, PropMode, SelectionNotifyEvent, SelectionRequestEvent, Window,
    WindowClass,
};
use smithay::reexports::x11rb::protocol::Event;
use smithay::reexports::x11rb::rust_connection::RustConnection;
use smithay::reexports::x11rb::wrapper::ConnectionExt as _;

use crate::xdnd::{
    parse_uri_list, uris_to_paths, XdndAction, XdndAtoms, XdndMessage, XdndSource, XdndStatus,
    XdndTarget, URI_LIST_MIME, XDND_VERSION,
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
    /// The X11 target answered an `XdndPosition` from the source role.
    SourceStatus {
        target: u32,
        accept: bool,
        action: XdndAction,
    },
    /// The X11 target finished the source-role drag.
    SourceFinished {
        target: u32,
        success: bool,
        action: XdndAction,
    },
    /// The target requested `text/uri-list` before the bridge had the
    /// Wayland payload. The host fetches it and calls
    /// [`XdndBridge::set_source_payload`].
    SourceSelectionNeeded { target: u32 },
    /// The bridge answered a selection request from the cached payload.
    SourceSelectionServed { target: u32 },
    /// The target never sent `XdndFinished` before the deadline.
    SourceTimedOut { target: u32 },
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

/// The live Wayland→X11 source drag. The source window is the bridge window
/// (T328a's connection and window are reused; no second connection exists).
#[derive(Debug)]
struct ActiveSource {
    /// The action the source proposes (already resolved from `None`).
    action: XdndAction,
    /// The X11 target the pointer is over, once entered.
    target: Option<Window>,
    /// The pure state machine, created on enter.
    state: Option<XdndSource>,
    /// The `text/uri-list` payload fetched from the Wayland source, if any.
    payload: Option<Vec<u8>>,
    /// A selection request waiting for the payload fetch.
    pending: Option<SelectionRequestEvent>,
    /// Set after `XdndDrop`; the target must answer `XdndFinished` by then.
    deadline: Option<Instant>,
}

/// The dedicated X11 connection and bridge window for the target and source
/// roles.
pub struct XdndBridge {
    conn: RustConnection,
    root: Window,
    window: Window,
    atoms: XdndAtoms,
    uri_list: Atom,
    property: Atom,
    incr: Atom,
    targets: Atom,
    target: XdndTarget,
    pending: Option<PendingTransfer>,
    source: Option<ActiveSource>,
}

impl std::fmt::Debug for XdndBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XdndBridge")
            .field("root", &self.root)
            .field("window", &self.window)
            .field("atoms", &self.atoms)
            .field("pending", &self.pending)
            .field("source", &self.source)
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
        let targets = intern(&conn, "TARGETS")?;

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
            targets,
            target: XdndTarget::new(),
            pending: None,
            source: None,
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

    /// Whether a Wayland→X11 source drag is live.
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// The X11 target the source role is currently over, if any.
    pub fn source_target(&self) -> Option<Window> {
        self.source.as_ref().and_then(|source| source.target)
    }

    /// Whether the current X11 target accepted the source drag.
    pub fn source_is_accepted(&self) -> bool {
        self.source
            .as_ref()
            .and_then(|source| source.state.as_ref())
            .is_some_and(XdndSource::is_accepted)
    }

    /// The action the X11 target accepted, if any.
    pub fn source_accepted_action(&self) -> Option<XdndAction> {
        self.source
            .as_ref()
            .and_then(|source| source.state.as_ref())
            .and_then(XdndSource::accepted_action)
    }

    /// Start the source role for a Wayland drag. The bridge window becomes
    /// the owner of `XdndSelection`; the payload is supplied later by
    /// [`Self::set_source_payload`] when a target asks for it.
    pub fn begin_source(&mut self, action: XdndAction) -> Result<(), String> {
        self.end_source()?;
        self.conn
            .set_selection_owner(
                self.window,
                self.atoms.selection,
                smithay::reexports::x11rb::CURRENT_TIME,
            )
            .map_err(|err| format!("own XdndSelection: {err}"))?;
        self.source = Some(ActiveSource {
            action: action.resolved(),
            target: None,
            state: None,
            payload: None,
            pending: None,
            deadline: None,
        });
        self.conn
            .flush()
            .map_err(|err| format!("flush selection ownership: {err}"))
    }

    /// Enter an X11 target: send `XdndEnter` followed by the opening
    /// `XdndPosition`.
    pub fn source_enter(
        &mut self,
        target: Window,
        position: (i16, i16),
        time: u32,
    ) -> Result<(), String> {
        let Some(source) = self.source.as_mut() else {
            return Err("no active XDnD source".to_string());
        };
        let state = XdndSource::new(
            self.window,
            target,
            XDND_VERSION,
            vec![self.uri_list],
            source.action,
        );
        let enter = state.enter();
        let opening = state.position(position, time);
        source.target = Some(target);
        source.state = Some(state);
        self.send_client_message(target, self.atoms.enter, &enter)?;
        self.send_client_message(target, self.atoms.position, &opening)?;
        self.conn
            .flush()
            .map_err(|err| format!("flush XdndEnter: {err}"))
    }

    /// Send an `XdndPosition` to the current target. A no-op when the source
    /// has not entered a target.
    pub fn source_position(&mut self, position: (i16, i16), time: u32) -> Result<(), String> {
        let Some(source) = self.source.as_mut() else {
            return Ok(());
        };
        let Some(state) = source.state.as_ref() else {
            return Ok(());
        };
        let Some(target) = source.target else {
            return Ok(());
        };
        let message = state.position(position, time);
        self.send_client_message(target, self.atoms.position, &message)?;
        self.conn
            .flush()
            .map_err(|err| format!("flush XdndPosition: {err}"))
    }

    /// Send `XdndDrop` if the target accepted the drag. Returns `true` when
    /// the drop was sent; `false` when the target has not accepted (the
    /// caller must leave instead of dropping).
    pub fn source_drop(&mut self, time: u32) -> Result<bool, String> {
        let Some(source) = self.source.as_mut() else {
            return Ok(false);
        };
        let Some(state) = source.state.as_ref() else {
            return Ok(false);
        };
        if !state.is_accepted() {
            return Ok(false);
        }
        let Some(target) = source.target else {
            return Ok(false);
        };
        let message = state.drop(time);
        self.send_client_message(target, self.atoms.drop, &message)?;
        if let Some(source) = self.source.as_mut() {
            source.deadline = Some(Instant::now() + TRANSFER_TIMEOUT);
        }
        self.conn
            .flush()
            .map_err(|err| format!("flush XdndDrop: {err}"))?;
        Ok(true)
    }

    /// Send `XdndLeave` to the current target and forget it, keeping the
    /// source session (and its selection ownership) alive for a later enter.
    pub fn source_leave(&mut self) -> Result<(), String> {
        let (message, target) = match self.source.as_ref() {
            Some(source) => match (source.state.as_ref(), source.target) {
                (Some(state), Some(target)) => (Some(state.leave()), Some(target)),
                _ => (None, None),
            },
            None => (None, None),
        };
        if let (Some(message), Some(target)) = (message, target) {
            self.send_client_message(target, self.atoms.leave, &message)?;
        }
        if let Some(source) = self.source.as_mut() {
            source.state = None;
            source.target = None;
            source.deadline = None;
        }
        self.conn
            .flush()
            .map_err(|err| format!("flush XdndLeave: {err}"))
    }

    /// Supply the `text/uri-list` payload fetched from the Wayland source.
    /// Answers a selection request that arrived first.
    pub fn set_source_payload(&mut self, payload: Vec<u8>) -> Result<(), String> {
        let Some(source) = self.source.as_mut() else {
            return Ok(());
        };
        source.payload = Some(payload);
        let pending = source.pending.take();
        let Some(request) = pending else {
            return Ok(());
        };
        let payload = self
            .source
            .as_ref()
            .and_then(|source| source.payload.clone())
            .unwrap_or_default();
        self.write_selection_property(&request, &payload)?;
        self.send_selection_notify(&request, request.property)?;
        self.conn
            .flush()
            .map_err(|err| format!("flush source selection: {err}"))
    }

    /// End the source role: leave the target (when one is live), release
    /// `XdndSelection`, and forget the payload. Safe to call when idle.
    pub fn end_source(&mut self) -> Result<(), String> {
        let had_target = self
            .source
            .as_ref()
            .is_some_and(|source| source.target.is_some());
        if had_target {
            let _ = self.source_leave();
        }
        self.source = None;
        self.conn
            .set_selection_owner(
                smithay::reexports::x11rb::NONE,
                self.atoms.selection,
                smithay::reexports::x11rb::CURRENT_TIME,
            )
            .map_err(|err| format!("release XdndSelection: {err}"))?;
        self.conn
            .flush()
            .map_err(|err| format!("flush selection release: {err}"))
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
        self.expire_source(&mut events)?;
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
            Event::SelectionRequest(request) => self.handle_selection_request(request, events),
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
            "XdndStatus" => {
                let Some(XdndMessage::Status {
                    target,
                    accept,
                    action,
                    ..
                }) = XdndMessage::decode(name, data, &self.atoms)
                else {
                    return Ok(());
                };
                let accepted = self
                    .source
                    .as_mut()
                    .and_then(|source| source.state.as_mut())
                    .is_some_and(|state| {
                        state.absorb_status(&XdndMessage::Status {
                            target,
                            accept,
                            want_position: false,
                            action,
                        })
                    });
                events.push(BridgeEvent::SourceStatus {
                    target,
                    accept: accepted,
                    action,
                });
                Ok(())
            }
            "XdndFinished" => {
                let Some(XdndMessage::Finished {
                    target,
                    success,
                    action,
                }) = XdndMessage::decode(name, data, &self.atoms)
                else {
                    return Ok(());
                };
                let success = self
                    .source
                    .as_mut()
                    .and_then(|source| source.state.as_mut())
                    .and_then(|state| {
                        state.absorb_finished(&XdndMessage::Finished {
                            target,
                            success,
                            action,
                        })
                    })
                    .unwrap_or(success);
                // `XdndFinished` is the target's final word; no `XdndLeave`
                // follows it.
                if let Some(source) = self.source.as_mut() {
                    source.target = None;
                    source.state = None;
                    source.deadline = None;
                }
                self.end_source()?;
                events.push(BridgeEvent::SourceFinished {
                    target,
                    success,
                    action,
                });
                Ok(())
            }
            // We are the target; status/finished belong to the source role.
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

    /// Serve a `SelectionRequest` from the source role. `text/uri-list` is
    /// written from the cached payload; `TARGETS` advertises the one flavor
    /// this bridge translates. A request before the payload has arrived is
    /// parked and answered by [`Self::set_source_payload`].
    fn handle_selection_request(
        &mut self,
        request: SelectionRequestEvent,
        events: &mut Vec<BridgeEvent>,
    ) -> Result<(), String> {
        if self.source.is_none() || request.selection != self.atoms.selection {
            return Ok(());
        }
        if request.property == 0 {
            // An obsolete request: answer with no property.
            return self.send_selection_notify(&request, 0);
        }
        if request.target == self.targets {
            self.conn
                .change_property32(
                    PropMode::REPLACE,
                    request.requestor,
                    request.property,
                    AtomEnum::ATOM,
                    &[self.uri_list],
                )
                .map_err(|err| format!("write TARGETS: {err}"))?;
            return self.send_selection_notify(&request, request.property);
        }
        if request.target != self.uri_list {
            // Only file drags are translated (T346 records other flavors).
            return self.send_selection_notify(&request, 0);
        }
        let payload = self
            .source
            .as_ref()
            .and_then(|source| source.payload.clone());
        if let Some(payload) = payload {
            self.write_selection_property(&request, &payload)?;
            self.send_selection_notify(&request, request.property)?;
            events.push(BridgeEvent::SourceSelectionServed {
                target: request.requestor,
            });
            return Ok(());
        }
        if let Some(source) = self.source.as_mut() {
            source.pending = Some(request);
        }
        events.push(BridgeEvent::SourceSelectionNeeded {
            target: request.requestor,
        });
        Ok(())
    }

    fn write_selection_property(
        &self,
        request: &SelectionRequestEvent,
        payload: &[u8],
    ) -> Result<(), String> {
        self.conn
            .change_property8(
                PropMode::REPLACE,
                request.requestor,
                request.property,
                self.uri_list,
                payload,
            )
            .map_err(|err| format!("write text/uri-list property: {err}"))?;
        Ok(())
    }

    fn send_selection_notify(
        &self,
        request: &SelectionRequestEvent,
        property: Atom,
    ) -> Result<(), String> {
        let notify = SelectionNotifyEvent {
            response_type: smithay::reexports::x11rb::protocol::xproto::SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time: request.time,
            requestor: request.requestor,
            selection: request.selection,
            target: request.target,
            property,
        };
        self.conn
            .send_event(false, request.requestor, EventMask::NO_EVENT, notify)
            .map_err(|err| format!("send source SelectionNotify: {err}"))?;
        self.conn
            .flush()
            .map_err(|err| format!("flush source SelectionNotify: {err}"))
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

    /// A source-role drag whose target never sent `XdndFinished` is bounded:
    /// send `XdndLeave`, release the selection, and report the timeout.
    fn expire_source(&mut self, events: &mut Vec<BridgeEvent>) -> Result<(), String> {
        let Some(deadline) = self.source.as_ref().and_then(|source| source.deadline) else {
            return Ok(());
        };
        if Instant::now() < deadline {
            return Ok(());
        }
        let target = self.source_target().unwrap_or(0);
        let _ = self.source_leave();
        self.end_source()?;
        events.push(BridgeEvent::SourceTimedOut { target });
        Ok(())
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
