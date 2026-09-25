// SPDX-License-Identifier: MIT
//! Trash: the freedesktop Trash specification (T-10.3a, [09-files.md]).
//!
//! The design's rule is **consume the spec, never re-implement GVfs**. On a
//! host where GIO/GVfs is linked, `trash://` and its `.trashinfo` store are
//! owned by GVfs and this module is the thin request surface. On this host
//! GIO's headers are absent from the pinned toolchain (ADR
//! [0043](../../../docs/design/adr/0043-files-core-fallback-is-the-shipping-backend.md)),
//! so [`FreedesktopTrash`] is the **sanctioned fallback**: it speaks the same
//! freedesktop Trash spec directly, reading and writing the same
//! `$XDG_DATA_HOME/Trash` (or `$HOME/.local/share/Trash`) store other
//! applications use. That is what makes "deletions made by other applications
//! appear in our Trash too" true even offline. The fallback is **marked for
//! replacement** by [`SANCTIONED_TRASH_FALLBACK_MARKER`], not silently shipped.
//!
//! # The store
//!
//! A trash directory holds two siblings:
//!
//! ```text
//! Trash/
//!   files/  the moved items themselves, under their (possibly de-duplicated) names
//!   info/   <name>.trashinfo, one per item:
//!           [Trash Info]
//!           Path=/home/u/Documents/report.txt   (percent-encoded, absolute)
//!           DeletionDate=2026-09-25T12:34:56
//! ```
//!
//! [`TrashOps::trash`] moves a `file://` item into the store and writes its
//! `.trashinfo`; [`TrashOps::restore`] reads the recorded original path and
//! moves the item back; [`TrashOps::empty`] removes everything. Items on a
//! filesystem other than the home trash's -- a removable disk -- go to that
//! volume's `.Trash/$UID` (when a sticky shared trash exists) or
//! `.Trash-$UID`, so trash stays on the medium that holds it, per the spec.
//!
//! # Deferred
//!
//! Listing `trash://` as a [`DirectorySource`](crate::DirectorySource) is the
//! Dock's Trash source (T-10.6a); the folder watcher is T-10.3b. This module
//! owns the mutations and enumeration the UI and Dock will build on. The
//! recorded `DeletionDate` is UTC rather than the spec's local time (a display
//! detail; restore never reads it).
//!
//! ```no_run
//! use dragonfruit_files_core::{FreedesktopTrash, Location, TrashOps};
//!
//! let trash = FreedesktopTrash::new();
//! let item = trash.trash(&Location::file("/tmp/report.txt")).expect("trash");
//! let restored = trash.restore(&item).expect("put back");
//! # assert_eq!(restored, Location::file("/tmp/report.txt"));
//! ```
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::location::{decode_path, encode_path};
use crate::ops::{copy_entry, generated_name, lexists, path_of, remove_entry};
use crate::{Location, OperationError};

/// Marker documenting that [`FreedesktopTrash`] is the sanctioned fallback and
/// that GIO/GVfs has not replaced it. Like the listing marker, it is worded so
/// a build that ships without the real backend is visibly degraded: a GIO trash
/// backend lands behind the [`TrashOps`] seam and removes this marker.
pub const SANCTIONED_TRASH_FALLBACK_MARKER: &str =
    "sanctioned freedesktop-spec trash fallback for files-core; GIO/GVfs trash backend not linked — replace behind the TrashOps seam when GIO is pinned (ADR 0046)";

/// The info file suffix the spec reserves.
const TRASHINFO_SUFFIX: &str = ".trashinfo";

/// One item in the trash store, with everything a Put Back needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashedItem {
    name: OsString,
    original_path: PathBuf,
    deletion_date: String,
    file_path: PathBuf,
    info_path: PathBuf,
}

impl TrashedItem {
    /// The name the item has inside the trash store (the original name, or a
    /// de-duplicated variant when that name was already taken).
    pub fn name(&self) -> &OsStr {
        &self.name
    }

    /// The absolute path the item had before it was trashed.
    pub fn original_path(&self) -> &Path {
        &self.original_path
    }

    /// The original path as a [`Location`], for a view or the Dock.
    pub fn original_location(&self) -> Location {
        Location::file(&self.original_path)
    }

    /// The `DeletionDate` recorded in the `.trashinfo`, verbatim.
    pub fn deletion_date(&self) -> &str {
        &self.deletion_date
    }

    /// The item's path inside the trash store's `files/`.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// The item's `.trashinfo` path inside the trash store's `info/`.
    pub fn info_path(&self) -> &Path {
        &self.info_path
    }
}

/// The platform seam for trash ([09-files.md]).
///
/// A sibling of [`FileOps`](crate::FileOps): [`FileOps::delete`](crate::FileOps::delete)
/// stays **permanent**, while this trait moves items to the freedesktop trash
/// store and puts them back. One implementation serves Files, the desktop
/// surface, and the portal chooser. Methods are synchronous and may block;
/// callers run them off the UI thread, and
/// [`OptimisticModel::trash_via`](crate::OptimisticModel::trash_via) wraps them
/// with optimistic rendering.
pub trait TrashOps: Send + Sync + 'static {
    /// Move the item at `target` (a `file://` location) into the trash store
    /// and record its original path. Returns the stored item.
    fn trash(&self, target: &Location) -> Result<TrashedItem, OperationError>;

    /// Put `item` back at its recorded original path and remove its
    /// `.trashinfo`. Fails with [`OperationError::AlreadyExists`] if something
    /// is there now, or [`OperationError::NotFound`] if the original parent is
    /// gone (the UI offers a destination picker instead).
    fn restore(&self, item: &TrashedItem) -> Result<Location, OperationError>;

    /// Remove every item from the home trash store, returning how many items
    /// were removed.
    fn empty(&self) -> Result<usize, OperationError>;

    /// Enumerate the home trash store. A malformed or unreadable `.trashinfo`
    /// is skipped rather than failing the whole listing, because other
    /// applications share this store.
    fn entries(&self) -> Result<Vec<TrashedItem>, OperationError>;

    /// The home trash directory this backend reads (`…/Trash`).
    fn home_trash(&self) -> &Path;
}

/// The home trash directory per the freedesktop spec and the XDG base-dir
/// spec: `$XDG_DATA_HOME/Trash`, else `$HOME/.local/share/Trash`.
pub fn default_home_trash() -> PathBuf {
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(data_home).join("Trash");
    }
    if let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(home).join(".local/share/Trash");
    }
    PathBuf::from(".local/share/Trash")
}

/// The sanctioned freedesktop-spec trash backend.
///
/// GIO/GVfs is the intended platform plumbing; its headers are absent from the
/// pinned toolchain, so this fallback is the shipping backend, marked for
/// replacement by ADR [0046](../../../docs/design/adr/0046-files-core-trash-seam-and-spec-fallback.md).
/// It reads and writes the same on-disk store GVfs uses, so trash contents stay
/// a single source of truth across applications.
#[derive(Debug, Clone)]
pub struct FreedesktopTrash {
    home: PathBuf,
}

impl Default for FreedesktopTrash {
    fn default() -> Self {
        Self::new()
    }
}

impl FreedesktopTrash {
    /// A trash backend over the environment's home trash.
    pub fn new() -> Self {
        Self::with_home_trash(default_home_trash())
    }

    /// A trash backend over an explicit home trash directory. Tests point this
    /// at a temp dir so the round-trip never touches the real user trash.
    pub fn with_home_trash(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }

    /// The replacement marker for this backend.
    pub const fn marker(&self) -> &'static str {
        SANCTIONED_TRASH_FALLBACK_MARKER
    }

    fn files_dir(&self) -> PathBuf {
        self.home.join("files")
    }

    fn info_dir(&self) -> PathBuf {
        self.home.join("info")
    }

    /// Choose the trash store for `target`: the home trash when it shares a
    /// filesystem, otherwise the target's volume trash (freedesktop spec).
    fn trash_dir_for(&self, target: &Path) -> Result<PathBuf, OperationError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let target_device = fs::metadata(target)
                .map_err(|error| OperationError::from_io(&error, target.display()))?
                .dev();
            fs::create_dir_all(&self.home)
                .map_err(|error| OperationError::from_io(&error, self.home.display()))?;
            let home_meta = fs::metadata(&self.home)
                .map_err(|error| OperationError::from_io(&error, self.home.display()))?;
            if target_device == home_meta.dev() {
                return Ok(self.home.clone());
            }
            let uid = home_meta.uid();
            let top = filesystem_top(target)?;
            let shared = top.join(".Trash");
            let dir = if usable_shared_trash(&shared) {
                shared.join(uid.to_string())
            } else {
                top.join(format!(".Trash-{uid}"))
            };
            create_private_dir(&dir)?;
            Ok(dir)
        }
        #[cfg(not(unix))]
        {
            let _ = target;
            fs::create_dir_all(&self.home)
                .map_err(|error| OperationError::from_io(&error, self.home.display()))?;
            Ok(self.home.clone())
        }
    }
}

impl TrashOps for FreedesktopTrash {
    fn trash(&self, target: &Location) -> Result<TrashedItem, OperationError> {
        let path = path_of(target)?;
        if !lexists(&path) {
            return Err(OperationError::NotFound(target.display()));
        }
        let trash_root = self.trash_dir_for(&path)?;
        let files = trash_root.join("files");
        let info = trash_root.join("info");
        fs::create_dir_all(&files)
            .map_err(|error| OperationError::from_io(&error, files.display()))?;
        fs::create_dir_all(&info)
            .map_err(|error| OperationError::from_io(&error, info.display()))?;

        let original_name = path
            .file_name()
            .ok_or_else(|| OperationError::InvalidName(path.display().to_string()))?;
        let stored = generated_name(original_name, |candidate| lexists(&files.join(candidate)));
        let file_path = files.join(&stored);
        let info_path = info.join(with_trashinfo_suffix(&stored));

        let deletion_date = format_deletion_date(SystemTime::now());
        let body = trash_info_body(&path, &deletion_date);
        fs::write(&info_path, body.as_bytes())
            .map_err(|error| OperationError::from_io(&error, info_path.display()))?;

        if let Err(error) = fs::rename(&path, &file_path) {
            match error.kind() {
                // The store shares `target`'s filesystem by construction, so
                // this is defensive: copy fully, then delete, never lose data.
                std::io::ErrorKind::CrossesDevices => {
                    if let Err(copy_error) =
                        copy_entry(&path, &file_path).and_then(|()| remove_entry(&path))
                    {
                        let _ = fs::remove_file(&info_path);
                        return Err(copy_error);
                    }
                }
                _ => {
                    let _ = fs::remove_file(&info_path);
                    return Err(OperationError::from_io(&error, target.display()));
                }
            }
        }

        Ok(TrashedItem {
            name: stored,
            original_path: path,
            deletion_date,
            file_path,
            info_path,
        })
    }

    fn restore(&self, item: &TrashedItem) -> Result<Location, OperationError> {
        if !lexists(&item.file_path) {
            return Err(OperationError::NotFound(
                item.file_path.display().to_string(),
            ));
        }
        if let Some(parent) = item.original_path.parent() {
            if !parent.is_dir() {
                return Err(OperationError::NotFound(parent.display().to_string()));
            }
        }
        if lexists(&item.original_path) {
            return Err(OperationError::AlreadyExists(
                item.original_path.display().to_string(),
            ));
        }
        if let Err(error) = fs::rename(&item.file_path, &item.original_path) {
            match error.kind() {
                std::io::ErrorKind::CrossesDevices => {
                    copy_entry(&item.file_path, &item.original_path)?;
                    remove_entry(&item.file_path)?;
                }
                _ => return Err(OperationError::from_io(&error, item.file_path.display())),
            }
        }
        let _ = fs::remove_file(&item.info_path);
        Ok(Location::file(&item.original_path))
    }

    fn empty(&self) -> Result<usize, OperationError> {
        let mut removed = 0;
        let files = self.files_dir();
        if files.is_dir() {
            for entry in fs::read_dir(&files)
                .map_err(|error| OperationError::from_io(&error, files.display()))?
            {
                let entry =
                    entry.map_err(|error| OperationError::from_io(&error, files.display()))?;
                remove_entry(&entry.path())?;
                removed += 1;
            }
        }
        let info = self.info_dir();
        if info.is_dir() {
            for entry in fs::read_dir(&info)
                .map_err(|error| OperationError::from_io(&error, info.display()))?
            {
                let entry =
                    entry.map_err(|error| OperationError::from_io(&error, info.display()))?;
                let path = entry.path();
                if path.is_dir() {
                    remove_entry(&path)?;
                } else {
                    fs::remove_file(&path)
                        .map_err(|error| OperationError::from_io(&error, path.display()))?;
                }
            }
        }
        Ok(removed)
    }

    fn entries(&self) -> Result<Vec<TrashedItem>, OperationError> {
        let files = self.files_dir();
        let info = self.info_dir();
        let mut items = Vec::new();
        let reader = match fs::read_dir(&info) {
            Ok(reader) => reader,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(items),
            Err(error) => return Err(OperationError::from_io(&error, info.display())),
        };
        for entry in reader {
            let entry = entry.map_err(|error| OperationError::from_io(&error, info.display()))?;
            let info_path = entry.path();
            if !info_path.is_file() {
                continue;
            }
            let Some(name) = strip_trashinfo_suffix(&entry.file_name()) else {
                continue;
            };
            let text = match fs::read_to_string(&info_path) {
                Ok(text) => text,
                Err(_) => continue,
            };
            let (original_path, deletion_date) = match parse_trash_info(&text) {
                Ok(parsed) => parsed,
                Err(_) => continue,
            };
            items.push(TrashedItem {
                file_path: files.join(&name),
                name,
                original_path,
                deletion_date,
                info_path,
            });
        }
        Ok(items)
    }

    fn home_trash(&self) -> &Path {
        &self.home
    }
}

/// Strip the `.trashinfo` suffix from a raw info-file name, preserving the
/// original name bytes.
fn strip_trashinfo_suffix(name: &OsStr) -> Option<OsString> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let bytes = name.as_bytes();
        let suffix = TRASHINFO_SUFFIX.as_bytes();
        if bytes.len() > suffix.len() && bytes.ends_with(suffix) {
            return Some(OsString::from_vec(
                bytes[..bytes.len() - suffix.len()].to_vec(),
            ));
        }
        None
    }
    #[cfg(not(unix))]
    {
        name.to_string_lossy()
            .strip_suffix(TRASHINFO_SUFFIX)
            .map(OsString::from)
    }
}

/// Append the reserved suffix to a stored item name.
fn with_trashinfo_suffix(name: &OsStr) -> OsString {
    let mut info = name.to_os_string();
    info.push(TRASHINFO_SUFFIX);
    info
}

/// The body of a `.trashinfo` file: header, percent-encoded absolute original
/// path, and the deletion timestamp.
fn trash_info_body(original: &Path, deletion_date: &str) -> String {
    format!(
        "[Trash Info]\nPath={}\nDeletionDate={deletion_date}\n",
        encode_path(original)
    )
}

/// Parse a `.trashinfo` body into its original path and deletion date.
///
/// Unknown keys are ignored, so files written by other spec-conforming
/// applications still round-trip. A missing `Path` or a relative path is
/// rejected.
pub fn parse_trash_info(text: &str) -> Result<(PathBuf, String), OperationError> {
    let mut path: Option<PathBuf> = None;
    let mut date = String::new();
    let mut in_section = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim() == "[Trash Info]" {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(value) = line.strip_prefix("Path=") {
            path = Some(decode_path(value));
        } else if let Some(value) = line.strip_prefix("DeletionDate=") {
            date = value.to_owned();
        }
    }
    let path =
        path.ok_or_else(|| OperationError::MalformedTrashInfo("missing Path entry".to_owned()))?;
    if !path.is_absolute() {
        return Err(OperationError::MalformedTrashInfo(format!(
            "Path is not absolute: {}",
            path.display()
        )));
    }
    Ok((path, date))
}

/// Format `time` as the spec's `YYYY-MM-DDThh:mm:ss`. The spec asks for local
/// time; this fallback records UTC, which restore never depends on.
pub fn format_deletion_date(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format_iso8601_utc(seconds)
}

/// The calendar date and time-of-day for `unix_seconds`, rendered ISO 8601.
fn format_iso8601_utc(unix_seconds: i64) -> String {
    let days = unix_seconds.div_euclid(86_400);
    let seconds_of_day = unix_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
}

/// Convert a day count since 1970-01-01 to a proleptic Gregorian date
/// (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// The top directory of the filesystem holding `path`, found by walking up
/// while the device id is unchanged.
#[cfg(unix)]
fn filesystem_top(path: &Path) -> Result<PathBuf, OperationError> {
    use std::os::unix::fs::MetadataExt;
    let device = fs::metadata(path)
        .map_err(|error| OperationError::from_io(&error, path.display()))?
        .dev();
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.as_os_str().is_empty() {
            break;
        }
        match fs::metadata(parent) {
            Ok(metadata) if metadata.dev() == device => current = parent,
            _ => break,
        }
    }
    Ok(current.to_path_buf())
}

/// A shared `.Trash` is usable only when it is a real directory carrying the
/// sticky bit, per the spec.
#[cfg(unix)]
fn usable_shared_trash(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match fs::symlink_metadata(path) {
        Ok(metadata) => metadata.is_dir() && metadata.mode() & 0o1000 != 0,
        Err(_) => false,
    }
}

/// Create a per-user trash directory with `0700` permissions.
#[cfg(unix)]
fn create_private_dir(path: &Path) -> Result<(), OperationError> {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path).map_err(|error| OperationError::from_io(&error, path.display()))?;
    let mut permissions = fs::metadata(path)
        .map_err(|error| OperationError::from_io(&error, path.display()))?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)
        .map_err(|error| OperationError::from_io(&error, path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trash_info_round_trips_a_path_with_spaces() {
        let path = PathBuf::from("/home/u/My Documents/report final.txt");
        let body = trash_info_body(&path, "2026-09-25T01:02:03");
        assert!(body.starts_with("[Trash Info]\n"));
        assert!(body.contains("Path=/home/u/My%20Documents/report%20final.txt"));
        assert!(body.contains("DeletionDate=2026-09-25T01:02:03"));
        let (parsed, date) = parse_trash_info(&body).expect("parse");
        assert_eq!(parsed, path);
        assert_eq!(date, "2026-09-25T01:02:03");
    }

    #[test]
    fn trash_info_keeps_unknown_keys_and_ignores_them() {
        let body = "[Trash Info]\nPath=/tmp/a\nX-GVfs-Future=1\nDeletionDate=2020-01-02T03:04:05\n";
        let (parsed, date) = parse_trash_info(body).expect("parse");
        assert_eq!(parsed, PathBuf::from("/tmp/a"));
        assert_eq!(date, "2020-01-02T03:04:05");
    }

    #[test]
    fn a_relative_or_missing_path_is_malformed() {
        assert!(matches!(
            parse_trash_info("[Trash Info]\nDeletionDate=2020-01-01T00:00:00\n"),
            Err(OperationError::MalformedTrashInfo(_))
        ));
        assert!(matches!(
            parse_trash_info("[Trash Info]\nPath=relative/x\n"),
            Err(OperationError::MalformedTrashInfo(_))
        ));
    }

    #[test]
    fn deletion_dates_are_iso8601_utc() {
        assert_eq!(format_iso8601_utc(0), "1970-01-01T00:00:00");
        assert_eq!(format_iso8601_utc(1_000_000_000), "2001-09-09T01:46:40");
        // A leap day.
        assert_eq!(format_iso8601_utc(1_709_164_800), "2024-02-29T00:00:00");
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_trashinfo_names_strip_without_loss() {
        use std::os::unix::ffi::OsStrExt;
        let name = OsStr::from_bytes(b"odd\xffname.txt.trashinfo");
        let base = strip_trashinfo_suffix(name).expect("suffix");
        assert_eq!(base.as_bytes(), b"odd\xffname.txt");
        assert_eq!(strip_trashinfo_suffix(OsStr::new("plain")), None);
    }

    #[test]
    fn the_fallback_is_marked_for_replacement() {
        let marker = FreedesktopTrash::new().marker();
        assert!(marker.contains("GIO/GVfs"), "marker: {marker}");
        assert!(marker.contains("replace"), "marker: {marker}");
        assert!(marker.contains("TrashOps"), "marker: {marker}");
    }
}
