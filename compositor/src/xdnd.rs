// SPDX-License-Identifier: MIT
//! XDnD (X11 drag-and-drop) protocol codec and the pure translation model
//! for the X11/Wayland bridge (T-14.5).
//!
//! Smithay 0.7's XWM implements no XDnD translation (legacy T-06 recorded the
//! gap), so the X11 side of a drag never reaches a Wayland surface and a
//! Wayland drag never reaches an X11 client. This module is the protocol half
//! of the custom bridge the T-14 track budgets for: it encodes and decodes the
//! XDnD client messages a bridge must exchange, parses the `text/uri-list`
//! payload, and carries the two drag state machines (bridge-as-X11-target and
//! bridge-as-X11-source) that decide *what* to send at each step.
//!
//! The module is deliberately free of `x11rb`/Smithay types so the wire format
//! and the state transitions are unit-testable without a display. The X11
//! connection half (interning atoms, creating the bridge window, reading
//! `SelectionRequest`) is not wired into the compositor yet; see
//! `docs/design/adr/0099-xdnd-bridge-model-and-documented-gap.md` for the
//! exact remaining work and the T-17 gate note.
//!
//! Atom ids are resolved by the caller (an X11 connection); the codec takes
//! the [`XdndAtoms`] table and the separate `text/uri-list` atom id wherever
//! it needs to name an atom.

use std::path::{Path, PathBuf};

/// The XDnD protocol version this bridge speaks and advertises on
/// `XdndAware`.
pub const XDND_VERSION: u8 = 5;

/// The MIME type of the payload this bridge translates. File drags only;
/// every other XDnD flavor is explicitly deferred (task T-14.5, "Explicitly
/// deferred").
pub const URI_LIST_MIME: &str = "text/uri-list";

/// Every standard XDnD atom name the bridge interns, in the field order of
/// [`XdndAtoms`]. `text/uri-list` is interned separately because it is a MIME
/// type, not an XDnD control atom.
pub const ATOM_NAMES: [&str; 17] = [
    "XdndSelection",
    "XdndAware",
    "XdndEnter",
    "XdndLeave",
    "XdndPosition",
    "XdndStatus",
    "XdndDrop",
    "XdndFinished",
    "XdndTypeList",
    "XdndActionList",
    "XdndActionDescription",
    "XdndActionCopy",
    "XdndActionMove",
    "XdndActionLink",
    "XdndActionAsk",
    "XdndActionPrivate",
    "XdndProxy",
];

/// The interned XDnD atom ids the codec needs. The bridge fills this from its
/// own X connection (`intern_atom`); the codec never interns anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct XdndAtoms {
    pub selection: u32,
    pub aware: u32,
    pub enter: u32,
    pub leave: u32,
    pub position: u32,
    pub status: u32,
    pub drop: u32,
    pub finished: u32,
    pub type_list: u32,
    pub action_list: u32,
    pub action_description: u32,
    pub action_copy: u32,
    pub action_move: u32,
    pub action_link: u32,
    pub action_ask: u32,
    pub action_private: u32,
    pub proxy: u32,
}

impl XdndAtoms {
    /// The atom names this table holds, in field order.
    pub fn names() -> [&'static str; 17] {
        ATOM_NAMES
    }

    /// The ids in field order, for interning in one pass.
    pub fn as_array(&self) -> [u32; 17] {
        [
            self.selection,
            self.aware,
            self.enter,
            self.leave,
            self.position,
            self.status,
            self.drop,
            self.finished,
            self.type_list,
            self.action_list,
            self.action_description,
            self.action_copy,
            self.action_move,
            self.action_link,
            self.action_ask,
            self.action_private,
            self.proxy,
        ]
    }

    /// The atom for an XDnD action (`0` for [`XdndAction::None`]).
    pub fn action_atom(&self, action: XdndAction) -> u32 {
        match action {
            XdndAction::None => 0,
            XdndAction::Copy => self.action_copy,
            XdndAction::Move => self.action_move,
            XdndAction::Link => self.action_link,
            XdndAction::Ask => self.action_ask,
            XdndAction::Private => self.action_private,
        }
    }

    /// Decode an action atom. `0` is `None`; an unknown atom is `None` too
    /// (an X client may propose an action we do not support, which is the
    /// same as proposing no action).
    pub fn action_from_atom(&self, atom: u32) -> XdndAction {
        if atom == 0 {
            return XdndAction::None;
        }
        if atom == self.action_copy {
            XdndAction::Copy
        } else if atom == self.action_move {
            XdndAction::Move
        } else if atom == self.action_link {
            XdndAction::Link
        } else if atom == self.action_ask {
            XdndAction::Ask
        } else if atom == self.action_private {
            XdndAction::Private
        } else {
            XdndAction::None
        }
    }

    /// The XDnD message-type atom name for an atom id, when it is one of the
    /// six client messages.
    pub fn message_type_name(&self, atom: u32) -> Option<&'static str> {
        if atom != 0 && atom == self.enter {
            Some("XdndEnter")
        } else if atom == self.leave {
            Some("XdndLeave")
        } else if atom == self.position {
            Some("XdndPosition")
        } else if atom == self.status {
            Some("XdndStatus")
        } else if atom == self.drop {
            Some("XdndDrop")
        } else if atom == self.finished {
            Some("XdndFinished")
        } else {
            None
        }
    }
}

/// The five XDnD actions, plus `None` (`0`, "no action").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdndAction {
    None,
    Copy,
    Move,
    Link,
    Ask,
    Private,
}

impl XdndAction {
    /// The atom name for the action, or `None` for [`XdndAction::None`].
    pub fn atom_name(self) -> Option<&'static str> {
        match self {
            XdndAction::None => None,
            XdndAction::Copy => Some("XdndActionCopy"),
            XdndAction::Move => Some("XdndActionMove"),
            XdndAction::Link => Some("XdndActionLink"),
            XdndAction::Ask => Some("XdndActionAsk"),
            XdndAction::Private => Some("XdndActionPrivate"),
        }
    }
}

/// Pack a root-window position into the 16-bit-per-axis `XdndPosition` word
/// (`x` in the high half, `y` in the low half).
pub fn pack_position(position: (i16, i16)) -> u32 {
    ((position.0 as u16 as u32) << 16) | (position.1 as u16 as u32)
}

/// Unpack an `XdndPosition` word into `(x, y)`.
pub fn unpack_position(word: u32) -> (i16, i16) {
    ((word >> 16) as u16 as i16, (word & 0xffff) as u16 as i16)
}

/// A decoded XDnD client message. Type atoms in `Enter` stay numeric; the
/// bridge resolves them to MIME names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XdndMessage {
    /// `XdndEnter { source, version, more_types, types }`. When `more_types`
    /// is set the full type list lives in the `XdndTypeList` property and
    /// `types` is empty.
    Enter {
        source: u32,
        version: u8,
        more_types: bool,
        types: Vec<u32>,
    },
    /// `XdndPosition { source, position, time, action }`.
    Position {
        source: u32,
        position: (i16, i16),
        time: u32,
        action: XdndAction,
    },
    /// `XdndStatus { target, accept, want_position, action }`.
    Status {
        target: u32,
        accept: bool,
        want_position: bool,
        action: XdndAction,
    },
    /// `XdndLeave { source }`.
    Leave { source: u32 },
    /// `XdndDrop { source, time }`.
    Drop { source: u32, time: u32 },
    /// `XdndFinished { target, success, action }`.
    Finished {
        target: u32,
        success: bool,
        action: XdndAction,
    },
}

impl XdndMessage {
    /// The `message_type` atom name for this message.
    pub fn name(&self) -> &'static str {
        match self {
            XdndMessage::Enter { .. } => "XdndEnter",
            XdndMessage::Position { .. } => "XdndPosition",
            XdndMessage::Status { .. } => "XdndStatus",
            XdndMessage::Leave { .. } => "XdndLeave",
            XdndMessage::Drop { .. } => "XdndDrop",
            XdndMessage::Finished { .. } => "XdndFinished",
        }
    }

    /// Encode this message into the `[u32; 5]` payload of an X ClientMessage
    /// (format 32). Returns `None` for an `Enter` with more than three types
    /// when the `XdndTypeList` atom is not interned: the bridge must write the
    /// full list into the `XdndTypeList` property first, which needs the atom.
    pub fn encode(&self, atoms: &XdndAtoms) -> Option<[u32; 5]> {
        match self {
            XdndMessage::Enter {
                source,
                version,
                more_types,
                types,
            } => {
                let mut l1 = u32::from(*version) << 24;
                if *more_types || types.len() > 3 {
                    if atoms.type_list == 0 {
                        return None;
                    }
                    l1 |= 1;
                    Some([*source, l1, atoms.type_list, 0, 0])
                } else {
                    let mut data = [*source, l1, 0, 0, 0];
                    for (slot, atom) in data[2..5].iter_mut().zip(types.iter()) {
                        *slot = *atom;
                    }
                    Some(data)
                }
            }
            XdndMessage::Position {
                source,
                position,
                time,
                action,
            } => Some([
                *source,
                0,
                pack_position(*position),
                *time,
                atoms.action_atom(*action),
            ]),
            XdndMessage::Status {
                target,
                accept,
                want_position,
                action,
            } => {
                let mut l1 = 0;
                if *accept {
                    l1 |= 1;
                }
                if *want_position {
                    l1 |= 1 << 1;
                }
                Some([*target, l1, 0, 0, atoms.action_atom(*action)])
            }
            XdndMessage::Leave { source } => Some([*source, 0, 0, 0, 0]),
            XdndMessage::Drop { source, time } => Some([*source, 0, 0, *time, 0]),
            XdndMessage::Finished {
                target,
                success,
                action,
            } => Some([
                *target,
                u32::from(*success),
                atoms.action_atom(*action),
                0,
                0,
            ]),
        }
    }

    /// Decode a client message from its `message_type` atom name and
    /// `[u32; 5]` payload. `type_list` is the interned `XdndTypeList` atom (to
    /// detect an oversized `XdndEnter`); pass `0` when it is not interned.
    pub fn decode(name: &str, data: [u32; 5], atoms: &XdndAtoms) -> Option<Self> {
        match name {
            "XdndEnter" => {
                let source = data[0];
                let version = (data[1] >> 24) as u8;
                let more_types = data[1] & 1 != 0;
                let mut types = Vec::new();
                if !more_types {
                    for atom in &data[2..5] {
                        if *atom != 0 {
                            types.push(*atom);
                        }
                    }
                }
                Some(XdndMessage::Enter {
                    source,
                    version,
                    more_types,
                    types,
                })
            }
            "XdndPosition" => Some(XdndMessage::Position {
                source: data[0],
                position: unpack_position(data[2]),
                time: data[3],
                action: atoms.action_from_atom(data[4]),
            }),
            "XdndStatus" => Some(XdndMessage::Status {
                target: data[0],
                accept: data[1] & 1 != 0,
                want_position: data[1] & (1 << 1) != 0,
                action: atoms.action_from_atom(data[4]),
            }),
            "XdndLeave" => Some(XdndMessage::Leave { source: data[0] }),
            "XdndDrop" => Some(XdndMessage::Drop {
                source: data[0],
                time: data[3],
            }),
            "XdndFinished" => Some(XdndMessage::Finished {
                target: data[0],
                success: data[1] & 1 != 0,
                action: atoms.action_from_atom(data[2]),
            }),
            _ => None,
        }
    }
}

/// The bridge-as-target response to an `XdndPosition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XdndStatus {
    pub accept: bool,
    pub want_position: bool,
    pub action: XdndAction,
}

impl XdndStatus {
    /// A refusal: no action, and no more position events wanted.
    pub fn rejected() -> Self {
        Self {
            accept: false,
            want_position: false,
            action: XdndAction::None,
        }
    }
}

/// The bridge-as-target response to an `XdndDrop` once the payload has been
/// read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XdndFinished {
    pub target: u32,
    pub success: bool,
    pub action: XdndAction,
}

/// The payload a drop handed to the Wayland side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropRequest {
    pub source: u32,
    pub version: u8,
    pub type_atoms: Vec<u32>,
    pub position: (i16, i16),
    pub action: XdndAction,
    pub time: u32,
}

/// The live state of one incoming X11 drag, as a bridge-as-X11-target sees
/// it. Reset on `XdndLeave`, consumed on `XdndDrop`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingDrag {
    pub source: u32,
    pub version: u8,
    pub type_atoms: Vec<u32>,
    pub position: (i16, i16),
    pub action: XdndAction,
    pub accepted: bool,
}

/// Bridge-as-X11-target: receives XDnD messages from an X11 drag source and
/// decides the replies. The bridge advertises the returned [`XdndStatus`] and
/// [`XdndFinished`] back to the source, and reads `text/uri-list` from the
/// `XdndSelection` when a [`DropRequest`] is produced.
#[derive(Debug, Default)]
pub struct XdndTarget {
    drag: Option<IncomingDrag>,
}

impl XdndTarget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle `XdndEnter`. `type_atoms` is the resolved type list (the caller
    /// reads the `XdndTypeList` property for an oversized enter).
    pub fn enter(&mut self, source: u32, version: u8, type_atoms: Vec<u32>) {
        self.drag = Some(IncomingDrag {
            source,
            version,
            type_atoms,
            position: (0, 0),
            action: XdndAction::None,
            accepted: false,
        });
    }

    /// Handle `XdndLeave`; any live drag is discarded.
    pub fn leave(&mut self) {
        self.drag = None;
    }

    /// The live drag, if any.
    pub fn drag(&self) -> Option<&IncomingDrag> {
        self.drag.as_ref()
    }

    /// Whether the live drag offers `atom` (the interned `text/uri-list`).
    pub fn offers(&self, atom: u32) -> bool {
        self.drag
            .as_ref()
            .is_some_and(|drag| drag.type_atoms.contains(&atom))
    }

    /// Handle `XdndPosition`. A drag is accepted only when the source offers
    /// `uri_list_atom`; anything else is refused (the task translates file
    /// drags only). On acceptance the reply asks the source to keep sending
    /// positions, and proposes [`XdndAction::Copy`] for a file drop.
    pub fn position(
        &mut self,
        position: (i16, i16),
        action: XdndAction,
        uri_list_atom: u32,
    ) -> XdndStatus {
        let Some(drag) = self.drag.as_mut() else {
            return XdndStatus::rejected();
        };
        drag.position = position;
        drag.action = action;
        if uri_list_atom != 0 && drag.type_atoms.contains(&uri_list_atom) {
            drag.accepted = true;
            XdndStatus {
                accept: true,
                want_position: true,
                action: XdndAction::Copy,
            }
        } else {
            drag.accepted = false;
            XdndStatus::rejected()
        }
    }

    /// Handle `XdndDrop`. Returns the payload to fetch when the drag was
    /// accepted for `text/uri-list`, or `None` when it must be refused.
    pub fn drop(&mut self, time: u32) -> Option<DropRequest> {
        let drag = self.drag.as_ref()?;
        if !drag.accepted {
            return None;
        }
        Some(DropRequest {
            source: drag.source,
            version: drag.version,
            type_atoms: drag.type_atoms.clone(),
            position: drag.position,
            action: drag.action,
            time,
        })
    }

    /// The reply to send after a drop's payload has been read (or failed).
    /// Always clears the drag.
    pub fn finished(&mut self, success: bool, action: XdndAction) -> XdndFinished {
        let target = self.drag.as_ref().map(|drag| drag.source).unwrap_or(0);
        self.drag = None;
        XdndFinished {
            target,
            success,
            action,
        }
    }
}

/// Bridge-as-X11-source: drives an XDnD drag towards an X11 target window on
/// behalf of a Wayland drag. Each method returns the client message to send.
#[derive(Debug, Clone)]
pub struct XdndSource {
    source: u32,
    target: u32,
    version: u8,
    type_atoms: Vec<u32>,
    action: XdndAction,
    accepted: Option<XdndAction>,
}

impl XdndSource {
    pub fn new(
        source: u32,
        target: u32,
        version: u8,
        type_atoms: Vec<u32>,
        action: XdndAction,
    ) -> Self {
        Self {
            source,
            target,
            version,
            type_atoms,
            action,
            accepted: None,
        }
    }

    /// The source window id the bridge owns and advertises in every message.
    pub fn source(&self) -> u32 {
        self.source
    }

    /// The target window this drag is aimed at.
    pub fn target(&self) -> u32 {
        self.target
    }

    /// The action the source proposes.
    pub fn action(&self) -> XdndAction {
        self.action
    }

    /// The action the target last accepted, if any.
    pub fn accepted_action(&self) -> Option<XdndAction> {
        self.accepted
    }

    /// The opening `XdndEnter`.
    pub fn enter(&self) -> XdndMessage {
        XdndMessage::Enter {
            source: self.source,
            version: self.version,
            more_types: self.type_atoms.len() > 3,
            types: self.type_atoms.clone(),
        }
    }

    /// An `XdndPosition` at `position` (root-window coordinates) at `time`.
    pub fn position(&self, position: (i16, i16), time: u32) -> XdndMessage {
        XdndMessage::Position {
            source: self.source,
            position,
            time,
            action: self.action,
        }
    }

    /// An `XdndDrop` at `time`.
    pub fn drop(&self, time: u32) -> XdndMessage {
        XdndMessage::Drop {
            source: self.source,
            time,
        }
    }

    /// An `XdndLeave`.
    pub fn leave(&self) -> XdndMessage {
        XdndMessage::Leave {
            source: self.source,
        }
    }

    /// Absorb an `XdndStatus` from the target. Returns `true` when the target
    /// accepted the drag.
    pub fn absorb_status(&mut self, status: &XdndMessage) -> bool {
        if let XdndMessage::Status { accept, action, .. } = status {
            self.accepted = if *accept { Some(*action) } else { None };
            *accept
        } else {
            false
        }
    }

    /// Whether the target has accepted the drag.
    pub fn is_accepted(&self) -> bool {
        self.accepted.is_some()
    }
}

// --- text/uri-list ----------------------------------------------------------

/// Parse a `text/uri-list` payload into its non-comment, non-blank lines
/// (RFC 2483). Line endings are CRLF; stray CR or LF are tolerated.
pub fn parse_uri_list(text: &str) -> Vec<String> {
    text.split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Convert the `file://` entries of a parsed URI list to local paths. Other
/// schemes (e.g. `http:`) and non-local hosts are dropped.
pub fn uris_to_paths(uris: &[String]) -> Vec<PathBuf> {
    uris.iter().filter_map(|uri| uri_to_path(uri)).collect()
}

/// Convert one `file://` URI to a local path.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // `rest` is `[authority]/path`. An empty authority (`file:///path`) is the
    // local host; any other authority is a remote host we do not serve.
    let slash = rest.find('/')?;
    let authority = &rest[..slash];
    if !authority.is_empty() && authority != "localhost" {
        return None;
    }
    Some(PathBuf::from(percent_decode(&rest[slash..])))
}

/// Convert a local path to a `file://` URI.
pub fn path_to_uri(path: &Path) -> String {
    let text = path.to_string_lossy();
    let encoded = percent_encode(&text);
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

/// Serialize paths as a `text/uri-list` payload (CRLF-terminated).
pub fn format_uri_list(paths: &[PathBuf]) -> String {
    let mut out = paths
        .iter()
        .map(|path| path_to_uri(path))
        .collect::<Vec<_>>()
        .join("\r\n");
    if !out.is_empty() {
        out.push_str("\r\n");
    }
    out
}

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/')
}

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if is_unreserved(byte) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atoms() -> XdndAtoms {
        XdndAtoms {
            selection: 100,
            aware: 101,
            enter: 102,
            leave: 103,
            position: 104,
            status: 105,
            drop: 106,
            finished: 107,
            type_list: 108,
            action_list: 109,
            action_description: 110,
            action_copy: 111,
            action_move: 112,
            action_link: 113,
            action_ask: 114,
            action_private: 115,
            proxy: 116,
        }
    }

    const URI_LIST_ATOM: u32 = 200;

    #[test]
    fn enter_round_trips_up_to_three_types() {
        let atoms = atoms();
        let message = XdndMessage::Enter {
            source: 0x1234,
            version: XDND_VERSION,
            more_types: false,
            types: vec![URI_LIST_ATOM, 201, 202],
        };
        let data = message.encode(&atoms).expect("encode");
        assert_eq!(data[0], 0x1234);
        assert_eq!(data[1] >> 24, u32::from(XDND_VERSION));
        assert_eq!(data[1] & 1, 0);
        assert_eq!(&data[2..5], &[URI_LIST_ATOM, 201, 202]);
        let decoded = XdndMessage::decode("XdndEnter", data, &atoms).expect("decode");
        assert_eq!(decoded, message);
    }

    #[test]
    fn oversized_enter_uses_the_type_list_atom() {
        let atoms = atoms();
        let message = XdndMessage::Enter {
            source: 7,
            version: 5,
            more_types: true,
            types: Vec::new(),
        };
        let data = message.encode(&atoms).expect("encode");
        assert_eq!(data[1] & 1, 1, "the more-types bit is set");
        assert_eq!(data[2], atoms.type_list);
        let decoded = XdndMessage::decode("XdndEnter", data, &atoms).expect("decode");
        assert_eq!(decoded, message);

        // Without an interned type-list atom the encode is refused.
        let bare = XdndAtoms::default();
        assert!(message.encode(&bare).is_none());
    }

    #[test]
    fn position_packs_signed_root_coordinates() {
        let atoms = atoms();
        let message = XdndMessage::Position {
            source: 9,
            position: (-12, 345),
            time: 42,
            action: XdndAction::Copy,
        };
        let data = message.encode(&atoms).expect("encode");
        assert_eq!(data[0], 9);
        assert_eq!(data[2], pack_position((-12, 345)));
        assert_eq!(data[3], 42);
        assert_eq!(data[4], atoms.action_copy);
        assert_eq!(
            XdndMessage::decode("XdndPosition", data, &atoms),
            Some(message)
        );
        assert_eq!(unpack_position(data[2]), (-12, 345));
    }

    #[test]
    fn status_and_finished_carry_flags_and_actions() {
        let atoms = atoms();
        let status = XdndMessage::Status {
            target: 5,
            accept: true,
            want_position: true,
            action: XdndAction::Copy,
        };
        let data = status.encode(&atoms).expect("encode");
        assert_eq!(data[1] & 1, 1);
        assert_eq!(data[1] & (1 << 1), 1 << 1);
        assert_eq!(
            XdndMessage::decode("XdndStatus", data, &atoms),
            Some(status)
        );

        let finished = XdndMessage::Finished {
            target: 5,
            success: true,
            action: XdndAction::Move,
        };
        let data = finished.encode(&atoms).expect("encode");
        assert_eq!(data[1], 1);
        assert_eq!(data[2], atoms.action_move, "the action atom is in l[2]");
        assert_eq!(data[4], 0);
        assert_eq!(
            XdndMessage::decode("XdndFinished", data, &atoms),
            Some(finished)
        );
    }

    #[test]
    fn unknown_action_atom_is_none() {
        let atoms = atoms();
        assert_eq!(atoms.action_from_atom(0), XdndAction::None);
        assert_eq!(atoms.action_from_atom(9999), XdndAction::None);
        assert_eq!(atoms.action_from_atom(atoms.action_link), XdndAction::Link);
    }

    #[test]
    fn target_accepts_only_uri_list_drags() {
        let mut target = XdndTarget::new();
        target.enter(1, 5, vec![URI_LIST_ATOM, 201]);
        assert!(target.offers(URI_LIST_ATOM));
        let status = target.position((10, 20), XdndAction::Copy, URI_LIST_ATOM);
        assert!(status.accept);
        assert_eq!(status.action, XdndAction::Copy);
        let request = target.drop(1234).expect("accepted drop");
        assert_eq!(request.source, 1);
        assert_eq!(request.position, (10, 20));
        assert_eq!(request.time, 1234);
        let finished = target.finished(true, XdndAction::Copy);
        assert!(finished.success);
        assert!(target.drag().is_none(), "finished clears the drag");

        // A drag that does not offer text/uri-list is refused.
        let mut text_only = XdndTarget::new();
        text_only.enter(2, 5, vec![201]);
        let status = text_only.position((0, 0), XdndAction::Copy, URI_LIST_ATOM);
        assert!(!status.accept);
        assert!(text_only.drop(1).is_none());

        // A position before any enter is refused, and leave discards the drag.
        let mut idle = XdndTarget::new();
        assert!(
            !idle
                .position((0, 0), XdndAction::Copy, URI_LIST_ATOM)
                .accept
        );
        idle.enter(3, 5, vec![URI_LIST_ATOM]);
        idle.leave();
        assert!(idle.drag().is_none());
    }

    #[test]
    fn source_message_sequence_follows_target_status() {
        let mut source = XdndSource::new(33, 44, 5, vec![URI_LIST_ATOM], XdndAction::Copy);
        assert_eq!(source.source(), 33);
        assert_eq!(source.target(), 44);
        let enter = source.enter();
        assert_eq!(enter.name(), "XdndEnter");
        assert!(matches!(&enter, XdndMessage::Enter { source: 33, .. }));
        let position = source.position((100, 200), 7);
        assert!(matches!(
            position,
            XdndMessage::Position {
                source: 33,
                position: (100, 200),
                time: 7,
                action: XdndAction::Copy,
            }
        ));

        // A refused status leaves the source unaccepted.
        let refused = XdndMessage::Status {
            target: 44,
            accept: false,
            want_position: false,
            action: XdndAction::None,
        };
        assert!(!source.absorb_status(&refused));
        assert!(!source.is_accepted());

        let accepted = XdndMessage::Status {
            target: 44,
            accept: true,
            want_position: true,
            action: XdndAction::Copy,
        };
        assert!(source.absorb_status(&accepted));
        assert_eq!(source.accepted_action(), Some(XdndAction::Copy));

        assert_eq!(source.drop(9).name(), "XdndDrop");
        assert_eq!(source.leave().name(), "XdndLeave");
    }

    #[test]
    fn atom_names_table_matches_the_fields() {
        let atoms = atoms();
        assert_eq!(XdndAtoms::names()[0], "XdndSelection");
        assert_eq!(atoms.as_array()[0], 100);
        assert_eq!(atoms.as_array()[16], 116);
        assert_eq!(atoms.message_type_name(atoms.enter), Some("XdndEnter"));
        assert_eq!(
            atoms.message_type_name(atoms.finished),
            Some("XdndFinished")
        );
        assert_eq!(atoms.message_type_name(atoms.aware), None);
        assert_eq!(atoms.action_atom(XdndAction::Private), atoms.action_private);
        assert_eq!(XdndAction::None.atom_name(), None);
        assert_eq!(XdndAction::Move.atom_name(), Some("XdndActionMove"));
    }

    #[test]
    fn uri_list_parses_comments_blanks_and_crlf() {
        let payload =
            "# a comment\r\nfile:///home/u/one.txt\r\n\r\nfile:///home/u/two%20words.txt\r\n";
        let uris = parse_uri_list(payload);
        assert_eq!(
            uris,
            vec!["file:///home/u/one.txt", "file:///home/u/two%20words.txt"]
        );
        assert_eq!(
            uris_to_paths(&uris),
            vec![
                PathBuf::from("/home/u/one.txt"),
                PathBuf::from("/home/u/two words.txt")
            ]
        );
    }

    #[test]
    fn uri_list_round_trips_paths() {
        let paths = vec![
            PathBuf::from("/home/u/one.txt"),
            PathBuf::from("/home/u/two words & more.txt"),
            PathBuf::from("/tmp/ünïcode.txt"),
        ];
        let payload = format_uri_list(&paths);
        assert!(payload.ends_with("\r\n"));
        assert!(payload.contains("file:///home/u/two%20words%20%26%20more.txt"));
        let parsed = parse_uri_list(&payload);
        assert_eq!(uris_to_paths(&parsed), paths);
    }

    #[test]
    fn uri_to_path_rejects_remote_and_other_schemes() {
        assert_eq!(uri_to_path("http://example.com/x"), None);
        assert_eq!(uri_to_path("file://remotehost/x"), None);
        assert_eq!(uri_to_path("file://localhost/x"), Some(PathBuf::from("/x")));
        assert_eq!(uri_to_path("file:///x"), Some(PathBuf::from("/x")));
    }
}
