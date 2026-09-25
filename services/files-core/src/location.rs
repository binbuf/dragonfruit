// SPDX-License-Identifier: MIT
//! URI-addressed locations ([09-files.md]).
//!
//! A [`Location`] is GFile-shaped: a scheme plus a URI. T-10.1a's fallback
//! source only resolves the `file://` scheme, but the type carries any
//! `scheme://…` URI unchanged so `trash://`, `recent://`, and the GVfs
//! transports slot in without a model change.
//!
//! Filenames are raw bytes (the design's hard rule): percent-encoding keeps a
//! non-UTF-8 name intact through `OsStr` ↔ URI, and decoding restores the
//! original bytes rather than a lossy approximation.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

/// A directory location addressed by URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    uri: String,
}

/// Why a URI could not become a [`Location`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationError {
    /// The URI was the empty string.
    Empty,
    /// The URI had no `scheme://` prefix.
    NotAUnixUri,
    /// The scheme was empty or contained characters a scheme cannot hold.
    InvalidScheme,
}

impl fmt::Display for LocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocationError::Empty => f.write_str("location URI is empty"),
            LocationError::NotAUnixUri => f.write_str("location URI has no scheme:// prefix"),
            LocationError::InvalidScheme => f.write_str("location URI has an invalid scheme"),
        }
    }
}

impl std::error::Error for LocationError {}

impl Location {
    /// Parse an absolute `scheme://…` URI. The authority/path are kept as-is;
    /// only the scheme is validated, so unknown schemes survive to the source.
    pub fn parse(uri: &str) -> Result<Self, LocationError> {
        if uri.is_empty() {
            return Err(LocationError::Empty);
        }
        let scheme_end = uri.find("://").ok_or(LocationError::NotAUnixUri)?;
        let scheme = &uri[..scheme_end];
        if scheme.is_empty()
            || !scheme
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
        {
            return Err(LocationError::InvalidScheme);
        }
        Ok(Self {
            uri: uri.to_owned(),
        })
    }

    /// The `file://` location for `path`. Relative paths are resolved against
    /// the current directory so the URI is always absolute.
    pub fn file<P: AsRef<Path>>(path: P) -> Self {
        let absolute =
            std::path::absolute(path.as_ref()).unwrap_or_else(|_| path.as_ref().to_path_buf());
        Self {
            uri: encode_file_uri(&absolute),
        }
    }

    /// The scheme, e.g. `file`.
    pub fn scheme(&self) -> &str {
        // `parse`/`file` guarantee the prefix, so this never panics.
        &self.uri[..self.uri.find("://").expect("validated URI")]
    }

    /// The full URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Whether this is a `file://` location, the only scheme the T-10.1a
    /// fallback source resolves.
    pub fn is_file(&self) -> bool {
        self.scheme() == "file"
    }

    /// The local path for a `file://` location, decoding percent-escapes back
    /// to raw bytes. `None` for any other scheme.
    pub fn to_file_path(&self) -> Option<PathBuf> {
        if !self.is_file() {
            return None;
        }
        let rest = self.uri.strip_prefix("file://")?;
        Some(decode_path(rest))
    }

    /// The child location named `name` under this directory. For `file://`
    /// locations the path is joined and re-encoded; other schemes append to
    /// the URI text.
    pub fn child(&self, name: &OsStr) -> Location {
        if let Some(path) = self.to_file_path() {
            return Location::file(path.join(name));
        }
        let base = self.uri.strip_suffix('/').unwrap_or(&self.uri);
        Location {
            uri: format!("{base}/{}", percent_encode(&os_bytes(name))),
        }
    }

    /// The containing location, or `None` at a filesystem root or for an
    /// authority-only URI such as `trash://`.
    ///
    /// For `file://` the path is walked; other schemes are treated as URI
    /// segments, so operations on foreign schemes can still name a parent.
    pub fn parent(&self) -> Option<Location> {
        if let Some(path) = self.to_file_path() {
            return path.parent().map(Location::file);
        }
        let prefix_end = self.uri.find("://").map(|index| index + 3)?;
        let trimmed = self.uri.strip_suffix('/').unwrap_or(&self.uri);
        if trimmed.len() <= prefix_end {
            return None;
        }
        match trimmed.rfind('/') {
            Some(index) if index >= prefix_end => Location::parse(&trimmed[..index]).ok(),
            _ => None,
        }
    }

    /// A human-facing rendering: the decoded path for `file://`, the URI
    /// otherwise.
    pub fn display(&self) -> String {
        match self.to_file_path() {
            Some(path) => path.to_string_lossy().into_owned(),
            None => self.uri.clone(),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.uri)
    }
}

/// `file://` + a percent-encoded absolute path, keeping `/` separators.
fn encode_file_uri(path: &Path) -> String {
    let bytes = os_bytes(path.as_os_str());
    let mut out = String::with_capacity("file://".len() + bytes.len());
    out.push_str("file://");
    for byte in bytes {
        if byte == b'/' {
            out.push('/');
        } else if is_unreserved(byte) {
            out.push(byte as char);
        } else {
            push_escape(&mut out, byte);
        }
    }
    out
}

fn decode_path(rest: &str) -> PathBuf {
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    PathBuf::from(os_string_from_bytes(out))
}

/// Percent-encode every byte outside the RFC 3986 unreserved set. Path
/// separators are handled by the caller.
pub(crate) fn percent_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        if is_unreserved(byte) {
            out.push(byte as char);
        } else {
            push_escape(&mut out, byte);
        }
    }
    out
}

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

fn push_escape(out: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push('%');
    out.push(HEX[(byte >> 4) as usize] as char);
    out.push(HEX[(byte & 0x0f) as usize] as char);
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(unix)]
fn os_bytes(value: &OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_bytes(value: &OsStr) -> Vec<u8> {
    value.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
fn os_string_from_bytes(bytes: Vec<u8>) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes)
}

#[cfg(not(unix))]
fn os_string_from_bytes(bytes: Vec<u8>) -> OsString {
    OsString::from(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_round_trips_a_space() {
        let location = Location::file("/tmp/hello world");
        assert_eq!(location.uri(), "file:///tmp/hello%20world");
        assert_eq!(
            location.to_file_path(),
            Some(PathBuf::from("/tmp/hello world"))
        );
        assert_eq!(location.scheme(), "file");
        assert!(location.is_file());
    }

    #[test]
    fn child_appends_the_name_to_the_path() {
        let parent = Location::file("/tmp");
        let child = parent.child(OsStr::new("a b"));
        assert_eq!(child.uri(), "file:///tmp/a%20b");
        assert_eq!(child.to_file_path(), Some(PathBuf::from("/tmp/a b")));
    }

    #[test]
    fn root_child_is_absolute() {
        let child = Location::file("/").child(OsStr::new("etc"));
        assert_eq!(child.to_file_path(), Some(PathBuf::from("/etc")));
    }

    #[test]
    fn child_of_a_foreign_root_keeps_the_scheme_prefix() {
        let root = Location::parse("trash:///").expect("parses");
        let child = root.child(OsStr::new("a b"));
        assert_eq!(child.uri(), "trash:///a%20b");
        assert_eq!(child.scheme(), "trash");
    }

    #[test]
    fn parent_walks_a_file_path_to_its_parent() {
        let location = Location::file("/tmp/a b/c");
        assert_eq!(
            location.parent().and_then(|p| p.to_file_path()),
            Some(PathBuf::from("/tmp/a b"))
        );
        assert_eq!(Location::file("/").parent(), None);
    }

    #[test]
    fn parent_of_a_foreign_scheme_is_a_uri_prefix() {
        let location = Location::parse("trash:///foo").expect("parses");
        assert_eq!(
            location.parent().map(|p| p.uri().to_owned()),
            Some("trash://".to_owned())
        );
        assert_eq!(Location::parse("trash:///").expect("parses").parent(), None);
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_names_survive_encoding() {
        use std::os::unix::ffi::OsStrExt;
        let raw = OsStr::from_bytes(b"bad\xffname");
        let location = Location::file("/tmp").child(raw);
        let decoded = location.to_file_path().expect("file path");
        assert_eq!(decoded.file_name().unwrap().as_bytes(), b"bad\xffname");
    }

    #[test]
    fn foreign_schemes_are_carried_not_resolved() {
        let location = Location::parse("trash:///").expect("parses");
        assert_eq!(location.scheme(), "trash");
        assert!(!location.is_file());
        assert_eq!(location.to_file_path(), None);
        assert_eq!(location.display(), "trash:///");
    }

    #[test]
    fn malformed_uris_are_rejected() {
        assert_eq!(Location::parse(""), Err(LocationError::Empty));
        assert_eq!(
            Location::parse("no-scheme"),
            Err(LocationError::NotAUnixUri)
        );
        assert_eq!(Location::parse("://x"), Err(LocationError::InvalidScheme));
        assert_eq!(
            Location::parse("a b://x"),
            Err(LocationError::InvalidScheme)
        );
    }
}
