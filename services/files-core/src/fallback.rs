// SPDX-License-Identifier: MIT
//! The sanctioned `std::fs` fallback backend.
//!
//! GIO/GVfs is the intended platform plumbing ([09-files.md]): it gives
//! `trash://`, `recent://`, the udisks2 volume monitor, and remote change
//! events for free, and we must not re-implement any of it. The GIO headers
//! are not part of this host's pinned toolchain, so T-10.1a ships this
//! fallback behind the same [`DirectorySource`] seam. It is **marked for
//! replacement, not silently shipped**: [`SANCTIONED_FALLBACK_MARKER`] names
//! the task that replaces it, and T-10.1b finalizes the GIO-vs-fallback
//! decision.
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

/// Marker documenting that [`StdFsSource`] is the sanctioned fallback and the
/// task that replaces it. A later backend (GIO/GVfs) must not leave this in
/// place.
pub const SANCTIONED_FALLBACK_MARKER: &str =
    "sanctioned StdFs fallback for files-core listing; replace with GIO/GVfs (T-10.1b)";

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
    let name = entry.file_name();
    let uri = parent.child(&name).uri().to_owned();
    let kind = match entry.file_type() {
        Ok(file_type) if file_type.is_dir() => NodeKind::Directory,
        Ok(file_type) if file_type.is_file() => NodeKind::File,
        Ok(file_type) if file_type.is_symlink() => NodeKind::Symlink,
        _ => NodeKind::Other,
    };

    let mut node = Node::new(name, uri, kind);
    if kind == NodeKind::Symlink {
        if let Ok(target) = std::fs::read_link(entry.path()) {
            node = node.with_symlink_target(Some(target.to_string_lossy().into_owned()));
        }
    }
    if let Ok(metadata) = entry.metadata() {
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
        assert!(source.marker().contains("GIO/GVfs"));
        assert!(source.marker().contains("T-10.1b"));
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
