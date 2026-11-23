// SPDX-License-Identifier: MIT
//! The operations engine: rename, new folder, move, copy, delete
//! (T-10.2a, [09-files.md]).
//!
//! Every mutation a view performs flows through one engine, never through
//! `std::fs` in the UI ([09-files.md] hard rule). This is that engine's
//! headless core: [`FileOps`] is the platform seam and [`StdFsOps`] is the
//! sanctioned `std::fs` fallback for hosts without GIO/GVfs — the same
//! degradation ADR [0043](../../../docs/design/adr/0043-files-core-fallback-is-the-shipping-backend.md)
//! records for listing. GIO implements the same trait when its headers join
//! the toolchain, so no caller changes.
//!
//! # What T-10.2a owns
//!
//! * [`FileOps`] — the seam: `rename`, `create_dir` (new folder),
//!   `copy_as`/`copy`, `move_as`/`move_to`, `delete`, plus `exists` for name
//!   probing and a provided [`FileOps::new_folder`] that generates
//!   `untitled folder`, `untitled folder 2`, … (the one next-available-name
//!   helper the design requires for new-folder, duplicate, paste, and
//!   compress).
//! * [`StdFsOps`] — the fallback. Copy is recursive and recreates symlinks
//!   (never following them); a cross-device move copies into a hidden
//!   destination sibling, renames it into place, then removes the source
//!   (ADR 0323); delete is permanent and recursive. Only `file://` is
//!   resolved; `trash://`/`recent://`/remote schemes return
//!   [`OperationError::UnsupportedScheme`].
//! * [`OperationError`] — the per-operation failure, classifying the common
//!   errno cases so a view can render "The item is gone", "Stop", and so on.
//! * [`ConflictPolicy`] — the paste policy the clipboard module applies
//!   (Replace / Skip / Keep Both, T330).
//!
//! # Deferred on purpose
//!
//! Undo/redo, the journal, merge, and per-byte progress are later tasks.
//! A destination that already exists fails with
//! [`OperationError::AlreadyExists`] rather than guessing a policy; the paste
//! layer resolves the conflict explicitly. Because none of these calls may run
//! on the UI thread, callers drive them from a worker; the trait is
//! `Send + Sync`.
//!
//! ```no_run
//! use std::sync::Arc;
//! use dragonfruit_files_core::{FileOps, Location, StdFsOps};
//!
//! let ops = Arc::new(StdFsOps::new());
//! let folder = ops.new_folder(&Location::file("/tmp")).expect("new folder");
//! ops.rename(&folder, std::ffi::OsStr::new("renamed")).expect("rename");
//! # let _ = ops.delete(&Location::file("/tmp/renamed"));
//! ```
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::Location;

/// The base name [`FileOps::new_folder`] generates.
pub const NEW_FOLDER_BASE: &str = "untitled folder";

/// How a paste resolves a destination name that already exists.
///
/// T330: the Files conflict sheet offers these three. `Replace` removes the
/// existing destination first (folder-over-folder is a replace, not a merge);
/// `Skip` leaves it untouched; `KeepBoth` numbers the incoming item with the
/// one [`generated_name`] helper (`report.txt`, `report 2.txt`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// Remove the existing destination, then place the source.
    Replace,
    /// Leave the existing destination and skip the item.
    Skip,
    /// Place the source under the next available name.
    KeepBoth,
}

impl ConflictPolicy {
    /// Parse the wire/ABI spelling (`replace`, `skip`, `keep-both`).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "replace" => Some(Self::Replace),
            "skip" => Some(Self::Skip),
            "keep-both" | "keep_both" | "keepboth" => Some(Self::KeepBoth),
            _ => None,
        }
    }
}

/// A filesystem operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    /// The location's scheme has no implementation in this backend.
    UnsupportedScheme(String),
    /// The source or destination does not exist.
    NotFound(String),
    /// The caller lacks permission.
    PermissionDenied(String),
    /// A destination of that name already exists.
    AlreadyExists(String),
    /// A path component is not a directory.
    NotADirectory(String),
    /// A recursive delete hit a non-empty directory it may not remove.
    DirectoryNotEmpty(String),
    /// The supplied name is empty, is `.`/`..`, or contains a separator.
    InvalidName(String),
    /// A `.trashinfo` file could not be parsed (T-10.3a).
    MalformedTrashInfo(String),
    /// Any other I/O failure, with a message from the OS.
    Io(String),
}

impl OperationError {
    /// Classify an [`std::io::Error`] with the path it came from.
    pub fn from_io(error: &std::io::Error, context: impl fmt::Display) -> Self {
        use std::io::ErrorKind;
        let context = context.to_string();
        match error.kind() {
            ErrorKind::NotFound => OperationError::NotFound(context),
            ErrorKind::PermissionDenied => OperationError::PermissionDenied(context),
            ErrorKind::AlreadyExists => OperationError::AlreadyExists(context),
            ErrorKind::NotADirectory => OperationError::NotADirectory(context),
            ErrorKind::DirectoryNotEmpty => OperationError::DirectoryNotEmpty(context),
            _ => OperationError::Io(format!("{context}: {error}")),
        }
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationError::UnsupportedScheme(scheme) => {
                write!(f, "unsupported location scheme: {scheme}")
            }
            OperationError::NotFound(what) => write!(f, "not found: {what}"),
            OperationError::PermissionDenied(what) => write!(f, "permission denied: {what}"),
            OperationError::AlreadyExists(what) => write!(f, "already exists: {what}"),
            OperationError::NotADirectory(what) => write!(f, "not a directory: {what}"),
            OperationError::DirectoryNotEmpty(what) => write!(f, "directory not empty: {what}"),
            OperationError::InvalidName(what) => write!(f, "invalid name: {what}"),
            OperationError::MalformedTrashInfo(what) => {
                write!(f, "malformed trash info: {what}")
            }
            OperationError::Io(message) => write!(f, "I/O error: {message}"),
        }
    }
}

impl std::error::Error for OperationError {}

/// The one next-available-name helper ([09-files.md]).
///
/// Returns `base` if `exists` says it is free, otherwise `base 2`,
/// `base 3`, … — the numbering New Folder, Duplicate, Paste, and Compress all
/// share, so generated names never collide across features. It preserves the
/// base's raw bytes (`OsString`), so duplicating a non-UTF-8 name keeps its
/// bytes and only appends an ASCII suffix.
pub fn generated_name(base: &OsStr, mut exists: impl FnMut(&OsStr) -> bool) -> OsString {
    if !exists(base) {
        return base.to_os_string();
    }
    let mut counter: u64 = 2;
    loop {
        let mut candidate = base.to_os_string();
        candidate.push(format!(" {counter}"));
        if !exists(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

/// The platform seam for mutating operations ([09-files.md]).
///
/// One implementation serves browser windows, the desktop surface, and the
/// portal FileChooser. Methods are synchronous and may block; callers run
/// them off the UI thread. T-10.2b wraps this seam with optimistic semantics.
pub trait FileOps: Send + Sync + 'static {
    /// Rename `from` to `new_name` in its own parent, returning the new
    /// location. Fails with [`OperationError::AlreadyExists`] if a different
    /// entry already has that name.
    fn rename(&self, from: &Location, new_name: &OsStr) -> Result<Location, OperationError>;

    /// Create a subdirectory `name` under `parent`, returning its location.
    fn create_dir(&self, parent: &Location, name: &OsStr) -> Result<Location, OperationError>;

    /// Create the next available `untitled folder` under `parent`.
    ///
    /// The provided implementation uses [`generated_name`] and
    /// [`FileOps::create_dir`], so every backend numbers generated folders the
    /// same way.
    fn new_folder(&self, parent: &Location) -> Result<Location, OperationError> {
        let name = generated_name(OsStr::new(NEW_FOLDER_BASE), |candidate| {
            self.exists(&parent.child(candidate))
        });
        self.create_dir(parent, &name)
    }

    /// Copy `from` into directory `into` under `name`. Recursive for
    /// directories; symlinks are recreated, never followed. Returns the copy.
    /// Fails with [`OperationError::AlreadyExists`] when the name is taken.
    fn copy_as(
        &self,
        from: &Location,
        into: &Location,
        name: &OsStr,
    ) -> Result<Location, OperationError>;

    /// Move `from` into directory `into` under `name`. Returns the moved
    /// location. Same-filesystem moves are one `rename(2)`; a cross-device
    /// move copies into a hidden destination sibling, renames that into
    /// place, and only then removes the source (ADR 0323).
    fn move_as(
        &self,
        from: &Location,
        into: &Location,
        name: &OsStr,
    ) -> Result<Location, OperationError>;

    /// Copy `from` into directory `into`, keeping its file name. Recursive for
    /// directories; symlinks are recreated, never followed. Returns the copy.
    fn copy(&self, from: &Location, into: &Location) -> Result<Location, OperationError> {
        let name = entry_name(from)?;
        self.copy_as(from, into, &name)
    }

    /// Move `from` into directory `into`, keeping its file name. Returns the
    /// moved location. Cross-device moves copy fully, then delete the source.
    fn move_to(&self, from: &Location, into: &Location) -> Result<Location, OperationError> {
        let name = entry_name(from)?;
        self.move_as(from, into, &name)
    }

    /// Permanently delete `target`, recursively for a directory. This is the
    /// immediate delete; moving to Trash is T-10.3a.
    fn delete(&self, target: &Location) -> Result<(), OperationError>;

    /// Whether an entry exists at `location` (a broken symlink counts).
    fn exists(&self, location: &Location) -> bool;
}

/// The sanctioned `std::fs` operations backend.
///
/// GIO/GVfs is the intended platform plumbing; its headers are absent from the
/// pinned toolchain, so this fallback is the shipping backend, marked for
/// replacement by ADR [0043](../../../docs/design/adr/0043-files-core-fallback-is-the-shipping-backend.md)
/// just like [`crate::StdFsSource`].
///
/// The two optional fields are deterministic test seams for the
/// cross-filesystem move rule: `force_cross_device` selects the copy-to-temp
/// path without a second mount, and `fail_after_entries` stops the copy
/// mid-tree so a test can prove the temp is removed and the source survives.
/// Production callers use [`StdFsOps::new`].
#[derive(Debug, Default, Clone, Copy)]
pub struct StdFsOps {
    force_cross_device: bool,
    fail_after_entries: Option<usize>,
}

impl StdFsOps {
    /// Create the fallback operations backend.
    pub fn new() -> Self {
        Self::default()
    }

    /// Exercise the cross-filesystem move path even when both locations are on
    /// one filesystem. Tests use this instead of depending on a second mount.
    pub fn with_forced_cross_device(mut self) -> Self {
        self.force_cross_device = true;
        self
    }

    /// Stop a copy after `entries` successful entries and fail it. The
    /// cross-device move removes the partial temp and leaves the source.
    pub fn with_failure_after(mut self, entries: usize) -> Self {
        self.fail_after_entries = Some(entries);
        self
    }
}

/// The final path component of a `file://` location, as raw bytes.
fn entry_name(location: &Location) -> Result<OsString, OperationError> {
    path_of(location)?
        .file_name()
        .map(OsStr::to_os_string)
        .ok_or_else(|| OperationError::InvalidName(location.display()))
}

/// Resolve a location to a local path, refusing non-`file://` schemes.
pub(crate) fn path_of(location: &Location) -> Result<PathBuf, OperationError> {
    if !location.is_file() {
        return Err(OperationError::UnsupportedScheme(
            location.scheme().to_owned(),
        ));
    }
    location
        .to_file_path()
        .ok_or_else(|| OperationError::UnsupportedScheme(location.scheme().to_owned()))
}

/// Whether `path` names an entry, counting a broken symlink as present.
pub(crate) fn lexists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Reject names that are empty, `.`/`..`, or contain a path separator. A
/// rename/new-folder name is one component, never a path.
fn validate_name(name: &OsStr) -> Result<(), OperationError> {
    let path = Path::new(name);
    let mut components = path.components();
    match components.next() {
        Some(std::path::Component::Normal(_)) if components.next().is_none() => Ok(()),
        _ => Err(OperationError::InvalidName(
            name.to_string_lossy().into_owned(),
        )),
    }
}

/// Progress state for one recursive copy. The test seam stops the copy after
/// a deterministic number of entries; production leaves it `None`.
#[derive(Default)]
struct CopyState {
    copied: usize,
    fail_after: Option<usize>,
}

impl CopyState {
    fn step(&mut self, path: &Path) -> Result<(), OperationError> {
        if let Some(limit) = self.fail_after {
            if self.copied >= limit {
                return Err(OperationError::Io(format!(
                    "simulated copy failure after {} entries ({})",
                    self.copied,
                    path.display()
                )));
            }
        }
        self.copied += 1;
        Ok(())
    }
}

/// Copy permissions and modification time where the filesystem supports them.
/// Metadata is best-effort (ADR 0323): losing it must not fail an otherwise
/// complete copy or move.
fn preserve_metadata(from: &Path, to: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(from) {
        let _ = std::fs::set_permissions(to, metadata.permissions());
        if let Ok(modified) = metadata.modified() {
            if let Ok(file) = std::fs::File::open(to) {
                let _ = file.set_times(std::fs::FileTimes::new().set_modified(modified));
            }
        }
    }
}

/// Recursively copy `from` to `to`. Directories are recreated, then their
/// metadata restored after the children arrive (so a read-only directory can
/// still receive them); symlinks are recreated with their raw target, never
/// followed; regular files use `fs::copy`. Metadata is best-effort.
pub(crate) fn copy_entry(from: &Path, to: &Path) -> Result<(), OperationError> {
    copy_entry_with_state(from, to, &mut CopyState::default())
}

fn copy_entry_with_state(
    from: &Path,
    to: &Path,
    state: &mut CopyState,
) -> Result<(), OperationError> {
    let metadata = std::fs::symlink_metadata(from)
        .map_err(|error| OperationError::from_io(&error, from.display()))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        let target = std::fs::read_link(from)
            .map_err(|error| OperationError::from_io(&error, from.display()))?;
        create_symlink(&target, to)?;
        state.step(from)
    } else if file_type.is_dir() {
        std::fs::create_dir(to).map_err(|error| OperationError::from_io(&error, to.display()))?;
        let entries = std::fs::read_dir(from)
            .map_err(|error| OperationError::from_io(&error, from.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| OperationError::from_io(&error, from.display()))?;
            copy_entry_with_state(&entry.path(), &to.join(entry.file_name()), state)?;
        }
        // After the children, so creating them does not rewrite the mtime.
        preserve_metadata(from, to);
        state.step(from)
    } else {
        std::fs::copy(from, to)
            .map(|_| ())
            .map_err(|error| OperationError::from_io(&error, to.display()))?;
        preserve_metadata(from, to);
        state.step(from)
    }
}

/// Make a copied tree removable even when the source had read-only directory
/// modes. Cleanup must not fail because metadata preservation already applied
/// the source's permissions to the partial temp (ADR 0323).
fn make_tree_writable(path: &Path) {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return;
    }
    let mut permissions = metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(permissions.mode() | 0o700);
    }
    #[cfg(not(unix))]
    permissions.set_readonly(false);
    let _ = std::fs::set_permissions(path, permissions);
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            make_tree_writable(&entry.path());
        }
    }
}

/// A free hidden sibling in the destination folder. It is on the destination
/// filesystem by construction, so the final rename cannot cross a mount.
fn temp_sibling(parent: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    loop {
        let candidate = parent.join(format!(
            ".dragonfruit-move-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed) + 1
        ));
        if !lexists(&candidate) {
            return candidate;
        }
    }
}

/// Permanently remove `path`, recursing into directories.
pub(crate) fn remove_entry(path: &Path) -> Result<(), OperationError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| OperationError::from_io(&error, path.display()))?;
    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|error| OperationError::from_io(&error, path.display()))
    } else {
        std::fs::remove_file(path).map_err(|error| OperationError::from_io(&error, path.display()))
    }
}

/// Remove a partial temp tree, making read-only directories writable first.
fn remove_temp(path: &Path) {
    make_tree_writable(path);
    let _ = remove_entry(path);
}

#[cfg(unix)]
fn create_symlink(target: &Path, to: &Path) -> Result<(), OperationError> {
    std::os::unix::fs::symlink(target, to)
        .map_err(|error| OperationError::from_io(&error, to.display()))
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, to: &Path) -> Result<(), OperationError> {
    Err(OperationError::Io(format!(
        "{}: copying symlinks is unsupported on this platform",
        to.display()
    )))
}

/// The destination path `from` takes when placed in `into` under `name`, plus
/// the guard that the source is not being copied or moved inside itself.
fn destination_for(
    from_path: &Path,
    into_path: &Path,
    into: &Location,
    name: &OsStr,
) -> Result<PathBuf, OperationError> {
    validate_name(name)?;
    if !into_path.is_dir() {
        return Err(OperationError::NotADirectory(into.display()));
    }
    let target = into_path.join(name);
    if target.starts_with(from_path) {
        return Err(OperationError::InvalidName(format!(
            "cannot place {} inside itself",
            from_path.display()
        )));
    }
    Ok(target)
}

/// Cross-filesystem move (ADR 0323): copy the whole entry into a hidden temp
/// sibling on the destination filesystem, rename that into place, and only
/// then remove the source. Any copy or rename failure removes the temp and
/// leaves the source complete. A source-removal failure after the destination
/// rename keeps the move successful: the destination is the only complete
/// copy, and deleting it would lose data.
fn move_across_filesystems(
    from: &Path,
    target: &Path,
    fail_after: Option<usize>,
) -> Result<(), OperationError> {
    let parent = target
        .parent()
        .ok_or_else(|| OperationError::InvalidName(target.display().to_string()))?;
    let temp = temp_sibling(parent);
    let mut state = CopyState {
        copied: 0,
        fail_after,
    };
    if let Err(error) = copy_entry_with_state(from, &temp, &mut state) {
        remove_temp(&temp);
        return Err(error);
    }
    // The name was reserved before the copy. Re-check so a concurrently
    // created entry is never overwritten by the rename.
    if lexists(target) {
        remove_temp(&temp);
        return Err(OperationError::AlreadyExists(
            Location::file(target).display(),
        ));
    }
    if let Err(error) = std::fs::rename(&temp, target) {
        remove_temp(&temp);
        return Err(OperationError::from_io(&error, target.display()));
    }
    // The destination is complete; source removal is cleanup.
    make_tree_writable(from);
    let _ = remove_entry(from);
    Ok(())
}

impl FileOps for StdFsOps {
    fn rename(&self, from: &Location, new_name: &OsStr) -> Result<Location, OperationError> {
        validate_name(new_name)?;
        let from_path = path_of(from)?;
        if !lexists(&from_path) {
            return Err(OperationError::NotFound(from.display()));
        }
        let parent = from_path
            .parent()
            .ok_or_else(|| OperationError::InvalidName(from.display()))?;
        let target = parent.join(new_name);
        if target == from_path {
            return Ok(from.clone());
        }
        if lexists(&target) {
            return Err(OperationError::AlreadyExists(
                Location::file(&target).display(),
            ));
        }
        std::fs::rename(&from_path, &target)
            .map_err(|error| OperationError::from_io(&error, from.display()))?;
        Ok(Location::file(&target))
    }

    fn create_dir(&self, parent: &Location, name: &OsStr) -> Result<Location, OperationError> {
        validate_name(name)?;
        let parent_path = path_of(parent)?;
        if !parent_path.is_dir() {
            return Err(OperationError::NotADirectory(parent.display()));
        }
        let target = parent_path.join(name);
        if lexists(&target) {
            return Err(OperationError::AlreadyExists(
                Location::file(&target).display(),
            ));
        }
        std::fs::create_dir(&target)
            .map_err(|error| OperationError::from_io(&error, Location::file(&target).display()))?;
        Ok(Location::file(&target))
    }

    fn copy_as(
        &self,
        from: &Location,
        into: &Location,
        name: &OsStr,
    ) -> Result<Location, OperationError> {
        let from_path = path_of(from)?;
        if !lexists(&from_path) {
            return Err(OperationError::NotFound(from.display()));
        }
        let into_path = path_of(into)?;
        let target = destination_for(&from_path, &into_path, into, name)?;
        if lexists(&target) {
            return Err(OperationError::AlreadyExists(
                Location::file(&target).display(),
            ));
        }
        let mut state = CopyState {
            copied: 0,
            fail_after: self.fail_after_entries,
        };
        copy_entry_with_state(&from_path, &target, &mut state)?;
        Ok(Location::file(&target))
    }

    fn move_as(
        &self,
        from: &Location,
        into: &Location,
        name: &OsStr,
    ) -> Result<Location, OperationError> {
        let from_path = path_of(from)?;
        if !lexists(&from_path) {
            return Err(OperationError::NotFound(from.display()));
        }
        let into_path = path_of(into)?;
        let target = destination_for(&from_path, &into_path, into, name)?;
        if lexists(&target) {
            return Err(OperationError::AlreadyExists(
                Location::file(&target).display(),
            ));
        }
        if !self.force_cross_device {
            match std::fs::rename(&from_path, &target) {
                Ok(()) => return Ok(Location::file(&target)),
                Err(error) if error.kind() == std::io::ErrorKind::CrossesDevices => {
                    // The temp-and-rename rule below.
                }
                Err(error) => return Err(OperationError::from_io(&error, from.display())),
            }
        }
        move_across_filesystems(&from_path, &target, self.fail_after_entries)?;
        Ok(Location::file(&target))
    }

    fn delete(&self, target: &Location) -> Result<(), OperationError> {
        let path = path_of(target)?;
        if !lexists(&path) {
            return Err(OperationError::NotFound(target.display()));
        }
        remove_entry(&path)
    }

    fn exists(&self, location: &Location) -> bool {
        match path_of(location) {
            Ok(path) => lexists(&path),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_names_number_from_two() {
        let taken = ["untitled folder", "untitled folder 2"];
        let name = generated_name(OsStr::new("untitled folder"), |candidate| {
            taken.iter().any(|taken| OsStr::new(taken) == candidate)
        });
        assert_eq!(name, OsString::from("untitled folder 3"));
    }

    #[test]
    fn generated_names_keep_the_first_candidate_when_free() {
        let name = generated_name(OsStr::new("copy"), |_| false);
        assert_eq!(name, OsString::from("copy"));
    }

    #[test]
    #[cfg(unix)]
    fn generated_names_preserve_non_utf8_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let base = OsStr::from_bytes(b"odd\xffname");
        let name = generated_name(base, |candidate| candidate == base);
        assert_eq!(name.as_bytes(), b"odd\xffname 2");
    }

    #[test]
    fn an_empty_name_is_rejected() {
        assert!(matches!(
            validate_name(OsStr::new("")),
            Err(OperationError::InvalidName(_))
        ));
    }

    #[test]
    fn a_name_with_a_separator_is_rejected() {
        assert!(matches!(
            validate_name(OsStr::new("a/b")),
            Err(OperationError::InvalidName(_))
        ));
        assert!(matches!(
            validate_name(OsStr::new("..")),
            Err(OperationError::InvalidName(_))
        ));
        assert!(validate_name(OsStr::new("plain name.txt")).is_ok());
    }

    #[test]
    fn io_errors_are_classified() {
        let not_found = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        assert_eq!(
            OperationError::from_io(&not_found, "/x"),
            OperationError::NotFound("/x".to_owned())
        );
        let exists = std::io::Error::new(std::io::ErrorKind::AlreadyExists, "there");
        assert_eq!(
            OperationError::from_io(&exists, "/x"),
            OperationError::AlreadyExists("/x".to_owned())
        );
    }
}
