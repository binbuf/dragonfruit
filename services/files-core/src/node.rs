// SPDX-License-Identifier: MIT
//! The listed-item model ([09-files.md]).
//!
//! A [`Node`] is one row in a listing. It keeps the **original name bytes**,
//! not a display string, so a name that is not valid UTF-8 still lists,
//! selects, and (later) trashes; [`Node::display_name`] is the lossy decode a
//! view renders. The [`NodeId`] is assigned by [`crate::DirectoryModel`] and
//! is stable for the lifetime of a session, so selection and drag state never
//! glitch when a listing is refreshed (rename survival is the watcher's job,
//! T-10.3b).
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt;
use std::time::SystemTime;

/// A stable per-session identifier for a [`Node`].
///
/// [`NodeId::UNSET`] is the placeholder a source emits; the model replaces it
/// with a real, monotonically assigned id as it appends the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u64);

impl NodeId {
    /// The placeholder id for a node that has not entered a model yet.
    pub const UNSET: NodeId = NodeId(0);

    /// The raw value, for a bridge that needs a plain integer.
    pub fn get(self) -> u64 {
        self.0
    }

    /// Whether the model has assigned this node a real id.
    pub fn is_set(self) -> bool {
        self.0 != 0
    }

    /// Build an id from a model-assigned counter value.
    pub(crate) fn from_raw(value: u64) -> Self {
        NodeId(value)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The coarse kind of a listed item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// A directory that can be entered.
    Directory,
    /// A regular file.
    File,
    /// A symbolic link; its target is in [`Node::symlink_target`].
    Symlink,
    /// Sockets, FIFOs, devices, and anything else.
    Other,
}

impl NodeKind {
    /// Whether this kind is a directory.
    pub fn is_dir(self) -> bool {
        matches!(self, NodeKind::Directory)
    }
}

/// One listed item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    id: NodeId,
    name: OsString,
    uri: String,
    kind: NodeKind,
    size: Option<u64>,
    modified: Option<SystemTime>,
    symlink_target: Option<String>,
}

impl Node {
    /// A node with no metadata yet, as a source emits it. The id is
    /// [`NodeId::UNSET`] until [`crate::DirectoryModel`] appends it.
    pub fn new(name: impl Into<OsString>, uri: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId::UNSET,
            name: name.into(),
            uri: uri.into(),
            kind,
            size: None,
            modified: None,
            symlink_target: None,
        }
    }

    /// Set (or override) the stable id. The model owns this.
    pub(crate) fn set_id(&mut self, id: NodeId) {
        self.id = id;
    }

    /// Replace the name bytes. The optimistic layer uses this to render a
    /// rename before the filesystem confirms it; the model re-sorts after.
    pub(crate) fn set_name(&mut self, name: OsString) {
        self.name = name;
    }

    /// Replace the URI that accompanies a rename or a confirmed creation.
    pub(crate) fn set_uri(&mut self, uri: String) {
        self.uri = uri;
    }

    /// Builder: attach a byte size (files only, by convention).
    pub fn with_size(mut self, size: Option<u64>) -> Self {
        self.size = size;
        self
    }

    /// Builder: attach a modification time.
    pub fn with_modified(mut self, modified: Option<SystemTime>) -> Self {
        self.modified = modified;
        self
    }

    /// Builder: attach a symlink target.
    pub fn with_symlink_target(mut self, target: Option<String>) -> Self {
        self.symlink_target = target;
        self
    }

    /// The stable per-session id. [`NodeId::UNSET`] before insertion.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// The original name bytes as an `OsStr`.
    pub fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }

    /// The name decoded for display; invalid UTF-8 becomes the replacement
    /// glyph but the original bytes are untouched.
    pub fn display_name(&self) -> Cow<'_, str> {
        self.name.to_string_lossy()
    }

    /// The item's URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// The item's kind.
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Whether this item is a directory.
    pub fn is_dir(&self) -> bool {
        self.kind.is_dir()
    }

    /// The byte size, when known and meaningful (regular files).
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// The last modification time, when known.
    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }

    /// The symlink target rendered as text, when this is a symlink.
    pub fn symlink_target(&self) -> Option<&str> {
        self.symlink_target.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_is_lossy_but_name_bytes_are_not() {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let node = Node::new(
                std::ffi::OsStr::from_bytes(b"bad\xffname"),
                "file:///tmp/bad%FFname",
                NodeKind::File,
            );
            assert_eq!(node.name().as_bytes(), b"bad\xffname");
            assert!(node.display_name().contains('\u{fffd}'));
        }
    }

    #[test]
    fn a_fresh_node_has_no_id() {
        let node = Node::new("a", "file:///tmp/a", NodeKind::File);
        assert_eq!(node.id(), NodeId::UNSET);
        assert!(!node.id().is_set());
    }
}
