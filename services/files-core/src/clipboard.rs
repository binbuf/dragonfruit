// SPDX-License-Identifier: GPL-3.0-only
//! The clipboard and paste model (T330, [09-files.md]).
//!
//! Files copy and cut **mark** items; nothing moves until a paste. This
//! module is that mark plus the one paste rule. The actual filesystem work
//! still goes through [`FileOps`], so paste cannot grow a second copy loop:
//! `paste_one` resolves a conflict policy, then calls
//! [`FileOps::copy_as`] / [`FileOps::move_as`].
//!
//! # Conflict policy
//!
//! A destination name that already exists is resolved by one of three
//! policies, matching the Files conflict sheet:
//!
//! * [`ConflictPolicy::Replace`] removes the existing destination first. A
//!   folder-over-folder replace is a replace, not a merge (merge is deferred).
//! * [`ConflictPolicy::Skip`] leaves the destination and reports the item as
//!   skipped, without failing the rest of the paste.
//! * [`ConflictPolicy::KeepBoth`] numbers the incoming item with the one
//!   [`generated_name`] helper (`report.txt`, `report 2.txt`, …).
//!
//! A cut into the folder the item already lives in is a skip: the item is
//! already there, and a move onto itself is not an operation.
//!
//! # Symlinks
//!
//! [`FileOps::copy_as`] recreates a symlink as a link; it never follows it.
//! This is deliberate: a copied link keeps pointing at the same target, and a
//! moved link does not silently turn into a copy of its target. A directory
//! copy therefore does not follow symlinks by default, and cannot escape the
//! tree it was asked to copy.
//!
//! # Progress
//!
//! [`Clipboard::paste_with`] runs one item at a time and calls the progress
//! callback after each, so the Qt facade can show "Copying 2 of 3" without a
//! second operation engine. The C ABI uses [`paste_one`] directly and keeps
//! the same per-item contract.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::ffi::{OsStr, OsString};

use crate::{generated_name, ConflictPolicy, FileOps, Location, NodeKind, OperationError};

/// Whether the clipboard marks items to copy or to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardMode {
    /// Copy the items; the sources stay in place.
    Copy,
    /// Cut the items; a successful paste moves them.
    Cut,
}

impl ClipboardMode {
    /// Parse the wire/ABI spelling (`copy`, `cut`).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "copy" => Some(Self::Copy),
            "cut" => Some(Self::Cut),
            _ => None,
        }
    }

    /// The wire/ABI spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Cut => "cut",
        }
    }
}

/// One marked item: its source location, display name, and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItem {
    /// Where the item lives now.
    pub location: Location,
    /// The item's final path component, kept as raw bytes.
    pub name: OsString,
    /// The kind, so a paste can paint the optimistic row without a stat.
    pub kind: NodeKind,
}

impl ClipboardItem {
    /// Build an item from a source location and a known kind.
    pub fn new(location: Location, kind: NodeKind) -> Self {
        let name = entry_name(&location).unwrap_or_default();
        Self {
            location,
            name,
            kind,
        }
    }

    /// Build an item from a source location, inferring only the name. The
    /// kind is [`NodeKind::Other`] until the caller supplies the real one.
    pub fn from_location(location: Location) -> Self {
        Self::new(location, NodeKind::Other)
    }
}

/// The marked items plus their mode. An empty clipboard has no mode.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Clipboard {
    mode: Option<ClipboardMode>,
    items: Vec<ClipboardItem>,
}

impl Clipboard {
    /// An empty clipboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `items` for copying.
    pub fn copy(items: impl IntoIterator<Item = ClipboardItem>) -> Self {
        Self {
            mode: Some(ClipboardMode::Copy),
            items: items.into_iter().collect(),
        }
    }

    /// Mark `items` for cutting. Nothing is removed until a paste.
    pub fn cut(items: impl IntoIterator<Item = ClipboardItem>) -> Self {
        Self {
            mode: Some(ClipboardMode::Cut),
            items: items.into_iter().collect(),
        }
    }

    /// Replace the marks with `items` in `mode`.
    pub fn set(&mut self, mode: ClipboardMode, items: impl IntoIterator<Item = ClipboardItem>) {
        self.mode = Some(mode);
        self.items = items.into_iter().collect();
    }

    /// Empty the clipboard. A following paste is a no-op.
    pub fn clear(&mut self) {
        self.mode = None;
        self.items.clear();
    }

    /// The mode, or `None` for an empty clipboard.
    pub fn mode(&self) -> Option<ClipboardMode> {
        self.mode
    }

    /// The marked items, in selection order.
    pub fn items(&self) -> &[ClipboardItem] {
        &self.items
    }

    /// How many items are marked.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether no item is marked.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Paste every marked item into `target`, resolving conflicts with
    /// `policy` and reporting each outcome through `progress`.
    ///
    /// One item's failure does not abort the batch; the outcome carries the
    /// per-item error and the counts. `Replace` removes an existing
    /// destination before placing the source (see the module docs).
    pub fn paste_with(
        &self,
        ops: &dyn FileOps,
        target: &Location,
        policy: ConflictPolicy,
        mut progress: impl FnMut(&PasteItemOutcome),
    ) -> PasteOutcome {
        let Some(mode) = self.mode else {
            return PasteOutcome::default();
        };
        let mut outcome = PasteOutcome {
            mode: Some(mode),
            ..PasteOutcome::default()
        };
        for item in &self.items {
            let item_outcome =
                match paste_one(ops, &item.location, target, &item.name, mode, policy) {
                    Ok(value) => value,
                    Err(error) => PasteItemOutcome {
                        source: item.location.clone(),
                        destination: target.child(&item.name),
                        name: item.name.clone(),
                        status: PasteStatus::Failed,
                        error: Some(error),
                    },
                };
            outcome.record(&item_outcome);
            progress(&item_outcome);
            outcome.items.push(item_outcome);
        }
        outcome
    }
}

/// What happened to one pasted item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteStatus {
    /// Copied with its name unchanged (or numbered for Keep Both).
    Copied,
    /// Moved with its name unchanged (or numbered for Keep Both).
    Moved,
    /// Replaced an existing destination.
    Replaced,
    /// Left alone because the destination exists (or the item was already
    /// there).
    Skipped,
    /// The filesystem operation failed; the other items continue.
    Failed,
}

impl PasteStatus {
    /// Whether the item's destination now holds the source.
    pub fn is_success(self) -> bool {
        matches!(self, Self::Copied | Self::Moved | Self::Replaced)
    }
}

/// One pasted item's outcome.
#[derive(Debug, Clone)]
pub struct PasteItemOutcome {
    /// The source location.
    pub source: Location,
    /// The destination location (the actual numbered name for Keep Both).
    pub destination: Location,
    /// The actual destination name.
    pub name: OsString,
    /// What happened.
    pub status: PasteStatus,
    /// The failure, when `status` is [`PasteStatus::Failed`].
    pub error: Option<OperationError>,
}

/// The whole paste's outcome.
#[derive(Debug, Clone, Default)]
pub struct PasteOutcome {
    /// The mode the clipboard had, or `None` for an empty clipboard.
    pub mode: Option<ClipboardMode>,
    /// One outcome per marked item, in clipboard order.
    pub items: Vec<PasteItemOutcome>,
    /// Items copied successfully.
    pub copied: usize,
    /// Items moved successfully.
    pub moved: usize,
    /// Items replaced successfully.
    pub replaced: usize,
    /// Items skipped by policy.
    pub skipped: usize,
    /// Items that failed.
    pub failed: usize,
}

impl PasteOutcome {
    /// Record one item in the aggregate counts.
    pub fn record(&mut self, item: &PasteItemOutcome) {
        match item.status {
            PasteStatus::Copied => self.copied += 1,
            PasteStatus::Moved => self.moved += 1,
            PasteStatus::Replaced => self.replaced += 1,
            PasteStatus::Skipped => self.skipped += 1,
            PasteStatus::Failed => self.failed += 1,
        }
    }

    /// How many items reached the destination.
    pub fn succeeded(&self) -> usize {
        self.copied + self.moved + self.replaced
    }
}

/// Paste one item into `target` under `name`, resolving a conflict with
/// `policy`. This is the C ABI's per-item entry point.
///
/// A cut into the item's own folder is a skip. `Replace` deletes the existing
/// destination first; `KeepBoth` numbers the incoming name with
/// [`generated_name`] against the destination's current entries; `Skip`
/// returns a skipped outcome.
pub fn paste_one(
    ops: &dyn FileOps,
    source: &Location,
    target: &Location,
    name: &OsStr,
    mode: ClipboardMode,
    policy: ConflictPolicy,
) -> Result<PasteItemOutcome, OperationError> {
    let destination_for = |candidate: &OsStr| target.child(candidate);

    // A cut item pasted into its own folder is already where it belongs.
    if mode == ClipboardMode::Cut && source.parent().as_ref() == Some(target) {
        return Ok(PasteItemOutcome {
            source: source.clone(),
            destination: destination_for(name),
            name: name.to_os_string(),
            status: PasteStatus::Skipped,
            error: None,
        });
    }

    let mut destination_name = name.to_os_string();
    let mut replaced = false;
    if ops.exists(&destination_for(&destination_name)) {
        match policy {
            ConflictPolicy::Skip => {
                return Ok(PasteItemOutcome {
                    source: source.clone(),
                    destination: destination_for(&destination_name),
                    name: destination_name,
                    status: PasteStatus::Skipped,
                    error: None,
                });
            }
            ConflictPolicy::KeepBoth => {
                let mut exists = |candidate: &OsStr| ops.exists(&destination_for(candidate));
                destination_name = generated_name(&destination_name, &mut exists);
            }
            ConflictPolicy::Replace => {
                ops.delete(&destination_for(&destination_name))?;
                replaced = true;
            }
        }
    }

    let destination = match mode {
        ClipboardMode::Copy => ops.copy_as(source, target, &destination_name)?,
        ClipboardMode::Cut => ops.move_as(source, target, &destination_name)?,
    };
    let status = match (mode, replaced) {
        (_, true) => PasteStatus::Replaced,
        (ClipboardMode::Copy, false) => PasteStatus::Copied,
        (ClipboardMode::Cut, false) => PasteStatus::Moved,
    };
    Ok(PasteItemOutcome {
        source: source.clone(),
        destination,
        name: destination_name,
        status,
        error: None,
    })
}

/// The final path component of a location, as raw bytes.
fn entry_name(location: &Location) -> Option<OsString> {
    location
        .to_file_path()
        .and_then(|path| path.file_name().map(OsStr::to_os_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_clipboard_has_no_mode() {
        let clipboard = Clipboard::new();
        assert!(clipboard.is_empty());
        assert_eq!(clipboard.mode(), None);
    }

    #[test]
    fn copy_and_cut_keep_the_mode_and_order() {
        let a = ClipboardItem::new(Location::file("/tmp/a"), NodeKind::File);
        let b = ClipboardItem::new(Location::file("/tmp/b"), NodeKind::Directory);
        let copied = Clipboard::copy([a.clone(), b.clone()]);
        assert_eq!(copied.mode(), Some(ClipboardMode::Copy));
        assert_eq!(copied.items(), &[a.clone(), b.clone()]);
        let cut = Clipboard::cut([b.clone(), a.clone()]);
        assert_eq!(cut.mode(), Some(ClipboardMode::Cut));
        assert_eq!(cut.items(), &[b, a]);
    }

    #[test]
    fn clearing_removes_the_mode_and_items() {
        let mut clipboard =
            Clipboard::copy([ClipboardItem::from_location(Location::file("/tmp/a"))]);
        clipboard.clear();
        assert!(clipboard.is_empty());
        assert_eq!(clipboard.mode(), None);
    }

    #[test]
    fn mode_parses_the_abi_spelling() {
        assert_eq!(ClipboardMode::parse("copy"), Some(ClipboardMode::Copy));
        assert_eq!(ClipboardMode::parse("cut"), Some(ClipboardMode::Cut));
        assert_eq!(ClipboardMode::parse("move"), None);
        assert_eq!(ClipboardMode::Copy.as_str(), "copy");
        assert_eq!(ClipboardMode::Cut.as_str(), "cut");
    }

    #[test]
    fn conflict_policies_parse_the_sheet_spelling() {
        assert_eq!(
            ConflictPolicy::parse("replace"),
            Some(ConflictPolicy::Replace)
        );
        assert_eq!(ConflictPolicy::parse("skip"), Some(ConflictPolicy::Skip));
        assert_eq!(
            ConflictPolicy::parse("keep-both"),
            Some(ConflictPolicy::KeepBoth)
        );
        assert_eq!(ConflictPolicy::parse("merge"), None);
    }
}
