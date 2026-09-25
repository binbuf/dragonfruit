// SPDX-License-Identifier: MIT
//! The sanctioned `std::fs` fallback backend.
//!
//! GIO/GVfs is the intended platform plumbing ([09-files.md]): it gives
//! `trash://`, `recent://`, the udisks2 volume monitor, and remote change
//! events for free, and we must not re-implement any of it. The GIO headers
//! are not part of this host's pinned toolchain (`pkg-config --exists
//! gio-2.0` fails), so T-10.1b's finalized decision is to **keep this
//! fallback as the shipping backend** behind the same [`DirectorySource`]
//! seam. It is **marked for replacement, not silently shipped**:
//! [`SANCTIONED_FALLBACK_MARKER`] names GIO/GVfs and the seam, and
//! [adr/0043](../../../docs/design/adr/0043-files-core-fallback-is-the-shipping-backend.md)
//! records the decision and the conditions for adding the real backend.
//!
//! The fallback reads a `file://` location with `std::fs::read_dir`. It
//! preserves raw name bytes, follows symlinks for size/mtime while keeping the
//! link kind and target, and skips a single entry whose metadata cannot be
//! read rather than aborting the whole listing.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::fs::ReadDir;

use crate::source::{DirectoryReader, DirectorySource, SourceError};
use crate::{Location, Node, NodeKind};

/// Marker documenting that [`StdFsSource`] is the sanctioned fallback and
/// that GIO/GVfs has not replaced it. It is deliberately worded so a build
/// that ships without the real backend is still visibly degraded: a GIO
/// reader lands behind the `DirectorySource` seam and removes this marker.
pub const SANCTIONED_FALLBACK_MARKER: &str =
    "sanctioned StdFs fallback for files-core listing; GIO/GVfs backend not linked — replace behind the DirectorySource seam when GIO is pinned (ADR 0043)";

/// A [`DirectorySource`] over `std::fs::read_dir`, for hosts without GIO.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdFsSource;

impl StdFsSource {
    /// Create the fallback source.
    pub fn new() -> Self {
        Self
    }

    /// The replacement marker for this backend.
    pub const fn marker(&self) -> &'static str {
        SANCTIONED_FALLBACK_MARKER
    }
}

impl DirectorySource for StdFsSource {
    fn open(&self, location: &Location) -> Result<Box<dyn DirectoryReader>, SourceError> {
        if !location.is_file() {
            return Err(SourceError::UnsupportedScheme(location.scheme().to_owned()));
        }
        let path = location
            .to_file_path()
            .ok_or_else(|| SourceError::UnsupportedScheme(location.scheme().to_owned()))?;
        let entries = std::fs::read_dir(&path)
            .map_err(|error| SourceError::from_io(&error, location.display()))?;
        Ok(Box::new(StdFsReader {
            parent: location.clone(),
            entries,
        }))
    }
}

struct StdFsReader {
    parent: Location,
    entries: ReadDir,
}

impl DirectoryReader for StdFsReader {
    fn next_batch(&mut self, max: usize) -> Result<Option<Vec<Node>>, SourceError> {
        let cap = max.max(1);
        let mut nodes = Vec::new();
        while nodes.len() < cap {
            match self.entries.next() {
                None => break,
                Some(Err(error)) => {
                    if nodes.is_empty() {
                        return Err(SourceError::from_io(&error, self.parent.display()));
                    }
                    break;
                }
                Some(Ok(entry)) => nodes.push(node_for(&self.parent, entry)),
            }
        }
        if nodes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(nodes))
        }
    }
}

/// Build a node for one directory entry. Never fails: a missing `file_type`
/// degrades to [`NodeKind::Other`] and metadata errors simply leave the size
/// and dates unset, so one odd entry does not abort the listing.
fn node_for(parent: &Location, entry: std::fs::DirEntry) -> Node {
    node_for_path(parent, entry.file_name(), &entry.path())
}

/// Build a node for one named entry at `path` under `parent`.
///
/// Shared by the listing fallback and the T-10.3b watcher, which needs to
/// materialize the single entry a kernel event named without re-listing the
/// directory. Like the listing, it reads metadata with `symlink_metadata` (no
/// follow) so a symlink stays a symlink; a metadata failure leaves the size and
/// dates unset rather than failing.
pub(crate) fn node_for_path(
    parent: &Location,
    name: std::ffi::OsString,
    path: &std::path::Path,
) -> Node {
    let uri = parent.child(&name).uri().to_owned();
    let kind = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => NodeKind::Directory,
        Ok(metadata) if metadata.file_type().is_file() => NodeKind::File,
        Ok(metadata) if metadata.file_type().is_symlink() => NodeKind::Symlink,
        _ => NodeKind::Other,
    };

    let mut node = Node::new(name, uri, kind);
    if kind == NodeKind::Symlink {
        if let Ok(target) = std::fs::read_link(path) {
            node = node.with_symlink_target(Some(target.to_string_lossy().into_owned()));
        }
    }
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if kind == NodeKind::File {
            node = node.with_size(Some(metadata.len()));
        }
        node = node.with_modified(metadata.modified().ok());
    }
    node
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fallback_is_marked_for_replacement() {
        let source = StdFsSource::new();
        let marker = source.marker();
        assert!(marker.contains("GIO/GVfs"), "marker: {marker}");
        assert!(marker.contains("replace"), "marker: {marker}");
        assert!(marker.contains("DirectorySource"), "marker: {marker}");
    }

    #[test]
    fn a_foreign_scheme_is_rejected() {
        let location = Location::parse("trash:///").expect("valid");
        let error = StdFsSource::new()
            .open(&location)
            .err()
            .expect("unsupported");
        assert_eq!(error, SourceError::UnsupportedScheme("trash".to_owned()));
    }

    #[test]
    fn a_missing_directory_reports_not_found() {
        let location = Location::file("/definitely/not/here/dragonfruit-t101a");
        let error = StdFsSource::new().open(&location).err().expect("not found");
        assert!(matches!(error, SourceError::NotFound(_)), "got {error:?}");
    }
}
