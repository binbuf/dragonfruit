// SPDX-License-Identifier: MIT
//! The platform seam: one trait a directory backend implements ([09-files.md]).
//!
//! `files-core` never calls `readdir`/GIO directly in the model. A backend
//! implements [`DirectorySource`] to open a location and [`DirectoryReader`]
//! to pull entries in batches. GIO/GVfs is the intended live backend; the
//! [`crate::StdFsSource`] fallback implements the same seam until GIO headers
//! are part of the toolchain. [`crate::MockSource`] is the headless test fake.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::fmt;

use crate::{Location, Node};

/// A directory could not be opened or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// The path or location does not exist.
    NotFound(String),
    /// The caller lacks permission to read the location.
    PermissionDenied(String),
    /// The location exists but is not a directory.
    NotADirectory(String),
    /// The location's scheme has no implementation in this backend.
    UnsupportedScheme(String),
    /// Any other I/O failure, with a message from the OS.
    Io(String),
}

impl SourceError {
    /// Capture an [`std::io::Error`] with the location it came from,
    /// classifying the common cases.
    pub fn from_io(error: &std::io::Error, location: impl fmt::Display) -> Self {
        use std::io::ErrorKind;
        let context = location.to_string();
        match error.kind() {
            ErrorKind::NotFound => SourceError::NotFound(context),
            ErrorKind::PermissionDenied => SourceError::PermissionDenied(context),
            ErrorKind::NotADirectory => SourceError::NotADirectory(context),
            _ => SourceError::Io(format!("{context}: {error}")),
        }
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::NotFound(what) => write!(f, "not found: {what}"),
            SourceError::PermissionDenied(what) => write!(f, "permission denied: {what}"),
            SourceError::NotADirectory(what) => write!(f, "not a directory: {what}"),
            SourceError::UnsupportedScheme(scheme) => {
                write!(f, "unsupported location scheme: {scheme}")
            }
            SourceError::Io(message) => write!(f, "I/O error: {message}"),
        }
    }
}

impl std::error::Error for SourceError {}

/// Opens a [`Location`] for streaming.
///
/// Implementations are held by the listing worker, so they must be `Send +
/// Sync + 'static`; all I/O happens on that worker thread.
pub trait DirectorySource: Send + Sync + 'static {
    /// Open `location`, returning a reader whose [`DirectoryReader::next_batch`]
    /// pulls entries incrementally.
    fn open(&self, location: &Location) -> Result<Box<dyn DirectoryReader>, SourceError>;
}

/// A streaming cursor over one directory's entries.
///
/// Each call returns the next batch of at most `max` nodes, in the backend's
/// arrival order, or `Ok(None)` at end-of-listing.
pub trait DirectoryReader: Send {
    /// The next batch, or `None` when the listing is exhausted.
    fn next_batch(&mut self, max: usize) -> Result<Option<Vec<Node>>, SourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_errors_are_classified() {
        let not_found = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        assert_eq!(
            SourceError::from_io(&not_found, "/x"),
            SourceError::NotFound("/x".to_owned())
        );
        let denied = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no");
        assert_eq!(
            SourceError::from_io(&denied, "/x"),
            SourceError::PermissionDenied("/x".to_owned())
        );
    }
}
