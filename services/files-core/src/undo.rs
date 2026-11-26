// SPDX-License-Identifier: GPL-3.0-or-later
//! Undo for the Files operations (T331, [09-files.md]).
//!
//! Undo is a user-visible promise: the last rename, new folder, move, copy, or
//! Move to Trash can be taken back from the Edit menu and `Cmd+Z`. This module
//! is that promise's headless half. It keeps a **bounded** stack of completed
//! operations and replays each one through the same [`FileOps`]/[`TrashOps`]
//! seams every other mutation uses, so undo cannot grow a second engine.
//!
//! # What an entry records
//!
//! Every entry records the **locations** the operation produced or consumed
//! (`from`/`to` as [`Location`]s, never a stale display name), the node (for a
//! trash, its [`TrashedItem`] so Put Back is the spec's own restore), and how
//! to present it. Replaying a stale path would undo the wrong item after a
//! rename elsewhere; replaying the recorded location either finds the item or
//! fails with a reason.
//!
//! # The rule: no data loss
//!
//! An undo never overwrites and never removes something it did not create:
//!
//! * undoing a **rename** renames `to` back to `from`'s name;
//! * undoing a **new folder** deletes the created (empty at creation) folder —
//!   a folder that has since gained contents is refused, not deleted;
//! * undoing a **copy** deletes the copied items (only the copies);
//! * undoing a **move** moves each item from `to` back to `from`;
//! * undoing a **trash** puts the item back from the trash store.
//!
//! An item that no longer sits at its recorded location (moved or deleted by
//! hand, or by another application) is **not** guessed at: the reason names
//! the recorded item, because replaying some other path would undo the wrong
//! thing.
//!
//! Every check that can fail is made **before** the first filesystem change,
//! so a refused undo leaves the current state exactly as it was. When the
//! filesystem refuses anyway, the entries already undone are reversed again
//! (best effort) and the failure is reported: a partial undo is never silent.
//!
//! # Conflict reasons
//!
//! [`UndoStack::undo`] returns [`UndoError::Conflict`] with an
//! [`UndoConflict`] naming *why* — the original spot is occupied, the item
//! changed folders, the item is gone, the folder is not empty, or the trash
//! entry vanished — and the item it concerned. The shell shows that reason
//! inline; a refused undo leaves the entry on the stack so the user can try
//! again after clearing the conflict.
//!
//! # Bounds and lifetime
//!
//! The stack holds at most [`UNDO_LIMIT`] operations (oldest first out); it
//! groups consecutive pastes of one clipboard (a batch, the same token) into
//! one entry so one `Cmd+Z` takes back the whole paste. An undo pushes a
//! matching redo entry, so `Shift+Cmd+Z` replays the same recorded locations,
//! and a new operation clears the redo tail. Navigating to another folder
//! **clears both stacks**: an undo is folder-scoped by design — taking the
//! user somewhere else to undo an old action is more surprising than losing
//! it, and the git-less daily-driver Files keeps the promise small. The
//! stacks do not persist across restarts; the trash store itself is the
//! durable half (Put Back).
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::collections::VecDeque;
use std::ffi::OsString;

use crate::{FileOps, Location, NodeId, NodeKind, OperationError, TrashOps, TrashedItem};

/// How many operations one stack remembers. The design chose "bounded (e.g.
/// 20 entries)"; twenty steps is far beyond what one folder session asks for,
/// and the cap keeps the memory a window holds flat.
pub const UNDO_LIMIT: usize = 20;

/// A failed undo or redo, with the reason the shell shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoError {
    /// Nothing to undo/redo.
    Empty,
    /// The recorded operation cannot be replayed as recorded; the current
    /// state was not changed.
    Conflict(UndoConflict),
    /// The filesystem refused the replay. Entries already replayed in the
    /// same multi-item step were reversed where possible.
    Failed(OperationError),
}

impl UndoError {
    /// The human-readable reason for a notice.
    pub fn reason(&self) -> String {
        match self {
            UndoError::Empty => "There is nothing to undo".to_owned(),
            UndoError::Conflict(conflict) => conflict.reason.clone(),
            UndoError::Failed(error) => error.to_string(),
        }
    }

    /// The item the reason concerns, when the failure names one.
    pub fn item(&self) -> Option<&Location> {
        match self {
            UndoError::Empty => None,
            UndoError::Conflict(conflict) => conflict.item.as_ref(),
            UndoError::Failed(_) => None,
        }
    }
}

impl From<OperationError> for UndoError {
    fn from(error: OperationError) -> Self {
        UndoError::Failed(error)
    }
}

/// Why an undo/redo refused, and which item it concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoConflict {
    /// A stable machine-readable kind (`exists`, `moved`, `missing`,
    /// `not-empty`, `trash-gone`, `parent-gone`, `modified`).
    pub kind: &'static str,
    /// The user-facing sentence.
    pub reason: String,
    /// The item the reason is about.
    pub item: Option<Location>,
}

impl UndoConflict {
    fn new(kind: &'static str, reason: impl Into<String>, item: &Location) -> Self {
        Self {
            kind,
            reason: reason.into(),
            item: Some(item.clone()),
        }
    }
}

/// What one recorded operation was, and how to replay it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum UndoAction {
    /// `to` was renamed from `from`.
    Rename { from: Location, to: Location },
    /// `to` was created (new folder, or a pasted item `copied` from `from`).
    Create {
        to: Location,
        kind: NodeKind,
        copied: bool,
    },
    /// The item moved from `from` to `to`.
    Move { from: Location, to: Location },
    /// The item at `from` was moved to Trash; `to` is where it now lives.
    Trash { from: Location, to: Location },
}

impl UndoAction {
    /// The short user-facing label, stable across undo and redo so a redo
    /// entry says the same thing as the undo step it came from.
    fn label(&self) -> String {
        let name = match self {
            UndoAction::Rename { from, .. }
            | UndoAction::Move { from, .. }
            | UndoAction::Trash { from, .. } => entry_name(from),
            UndoAction::Create { to, .. } => entry_name(to),
        };
        match self {
            UndoAction::Rename { .. } => format!("Rename \"{name}\""),
            UndoAction::Create { copied: true, .. } => format!("Copy \"{name}\""),
            UndoAction::Create { .. } => format!("New Folder \"{name}\""),
            UndoAction::Move { .. } => format!("Move \"{name}\""),
            UndoAction::Trash { .. } => format!("Move \"{name}\" to Trash"),
        }
    }
}

/// One recorded operation: the actions (one per item; a paste is several),
/// the paste batch it belongs to, and the trash items a Trash undoes back.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UndoEntry {
    label: String,
    /// The session row this operation edited (a lone item's node, or the
    /// first item's node for a paste batch).
    node: NodeId,
    actions: Vec<UndoAction>,
    /// The paste batch token, when the entry was folded from a paste.
    batch: Option<u64>,
    /// The trash-store item of each `UndoAction::Trash`, parallel to
    /// `actions` (empty for other kinds).
    trashed: Vec<TrashedItem>,
}

impl UndoEntry {
    fn single(action: UndoAction) -> Self {
        Self {
            label: action.label(),
            node: NodeId::UNSET,
            actions: vec![action],
            batch: None,
            trashed: Vec::new(),
        }
    }

    fn for_trash(trashed: TrashedItem) -> Self {
        let from = trashed.original_location();
        let to = Location::file(trashed.file_path());
        Self {
            label: UndoAction::Trash {
                from: from.clone(),
                to: to.clone(),
            }
            .label(),
            node: NodeId::UNSET,
            actions: vec![UndoAction::Trash { from, to }],
            batch: None,
            trashed: vec![trashed],
        }
    }
}

/// What a replay changed, so the caller can update its own row view without
/// a re-list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ReplayEffect {
    /// A rename left the row at this location.
    renamed: Option<Location>,
    /// The location a `renamed` row had before this replay (the forward end,
    /// for a redo).
    was: Option<Location>,
    /// A Put Back restored the row to this location.
    restored: Option<Location>,
    /// An undone create removed the row at this location.
    removed: Option<Location>,
}

/// One completed undo or redo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoOutcome {
    /// What was undone or redone, e.g. `Rename "a.txt"`.
    pub label: String,
    /// The session row the replayed operation belongs to ([`NodeId::UNSET`]
    /// when the operation names no row, e.g. a paste item inserted then
    /// removed again). The Files facade renames or restores that row so the
    /// listing agrees with the filesystem without a re-list.
    pub node: NodeId,
    /// For a replayed rename, the location the row now has.
    pub renamed: Option<Location>,
    /// The location the renamed row had before the replay (its forward end:
    /// the model renames it back to this).
    pub renamed_from: Option<Location>,
    /// For an undone trash, the row's restored location.
    pub restored: Option<Location>,
    /// For an undone new folder or paste, the removed row's location (its
    /// optimistic row is retired by the model's own revert).
    pub removed: Option<Location>,
}

impl UndoOutcome {
    /// The status-bar sentence, e.g. `Undid "Rename a.txt"`.
    pub fn message(&self, redo: bool) -> String {
        if redo {
            format!("Redid \"{}\"", self.label)
        } else {
            format!("Undid \"{}\"", self.label)
        }
    }
}

/// The bounded undo and redo stacks for one folder session.
#[derive(Debug)]
pub struct UndoStack {
    undo: VecDeque<UndoEntry>,
    redo: Vec<UndoEntry>,
    limit: usize,
    /// The clipboard scope of the open paste batch: consecutive pastes with
    /// the same `batch` value fold into the top entry.
    batch: Option<u64>,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    /// A stack capped at [`UNDO_LIMIT`].
    pub fn new() -> Self {
        Self::with_limit(UNDO_LIMIT)
    }

    /// A stack capped at `limit` operations (tests use a small cap; zero
    /// disables recording).
    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit,
            batch: None,
        }
    }

    /// The cap.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// How many operations can be undone.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// How many can be redone.
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// The label of the next undo, if any.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|entry| entry.label.as_str())
    }

    /// The label of the next redo, if any.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|entry| entry.label.as_str())
    }

    /// Drop both stacks (a folder change).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.batch = None;
    }

    /// End the open paste batch, so the next paste starts its own entry.
    pub fn end_batch(&mut self) {
        self.batch = None;
    }

    // -- Recording ---------------------------------------------------------

    /// Push a fresh undo step, dropping the redo history (a new action makes
    /// the undone ones unreachable, exactly like a file manager's redo stack).
    /// Name the row the just-pushed step edited (a new folder's provisional
    /// row is found after the model inserted it).
    pub fn set_last_node(&mut self, node: NodeId) {
        if let Some(entry) = self.undo.back_mut() {
            entry.node = node;
        }
    }

    fn push_undo(&mut self, entry: UndoEntry) {
        self.redo.clear();
        if self.limit == 0 {
            return;
        }
        self.undo.push_back(entry);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    fn push_redo(&mut self, entry: UndoEntry) {
        if self.limit == 0 {
            return;
        }
        self.redo.push(entry);
        while self.redo.len() > self.limit {
            self.redo.remove(0);
        }
    }

    /// Record a completed rename `from` → `to`.
    pub fn record_rename(&mut self, from: Location, to: Location) {
        self.push_undo(UndoEntry::single(UndoAction::Rename { from, to }));
    }

    /// Record a completed rename that changed the session row `node`; an undo
    /// renames the row back as well as the filesystem.
    pub fn record_rename_of(&mut self, node: NodeId, from: Location, to: Location) {
        let mut entry = UndoEntry::single(UndoAction::Rename { from, to });
        entry.node = node;
        self.push_undo(entry);
    }

    /// Record a completed new folder at `created` (its row is `node`, if the
    /// folder is in the current listing).
    pub fn record_new_folder(&mut self, created: Location) {
        self.push_undo(UndoEntry::single(UndoAction::Create {
            to: created,
            kind: NodeKind::Directory,
            copied: false,
        }));
    }

    /// Record a completed copy `source` → `destination` (or a file duplicated
    /// in place).
    pub fn record_copy(&mut self, _source: Location, destination: Location) {
        self.push_undo(UndoEntry::single(UndoAction::Create {
            to: destination,
            kind: NodeKind::Other,
            copied: true,
        }));
    }

    /// Record a completed move `from` → `to`.
    pub fn record_move(&mut self, from: Location, to: Location) {
        self.push_undo(UndoEntry::single(UndoAction::Move { from, to }));
    }

    /// Record a completed Move to Trash: the item left `from` and now lives
    /// in the store as `trashed`.
    pub fn record_trash(&mut self, trashed: TrashedItem) {
        self.push_undo(UndoEntry::for_trash(trashed));
    }

    /// Record a completed trash of the session row `node`.
    pub fn record_trash_of(&mut self, node: NodeId, trashed: TrashedItem) {
        let mut entry = UndoEntry::for_trash(trashed);
        entry.node = node;
        self.push_undo(entry);
    }

    /// Record one item of a paste. Consecutive calls with the same `batch`
    /// token (one clipboard paste) fold into a single undo step, so one
    /// `Cmd+Z` takes back the whole paste.
    pub fn record_paste(
        &mut self,
        batch: u64,
        source: Location,
        destination: Location,
        moved: bool,
    ) {
        self.record_paste_of(batch, NodeId::UNSET, source, destination, moved);
    }

    /// Record one pasted item that is the session row `node`, so an undo
    /// removes or moves that row too. Consecutive calls with one batch token
    /// fold into one step.
    pub fn record_paste_of(
        &mut self,
        batch: u64,
        node: NodeId,
        source: Location,
        destination: Location,
        moved: bool,
    ) {
        let action = if moved {
            UndoAction::Move {
                from: source.clone(),
                to: destination,
            }
        } else {
            UndoAction::Create {
                to: destination,
                kind: NodeKind::Other,
                copied: true,
            }
        };
        if self.limit == 0 {
            return;
        }
        let first_label = format!("Paste \"{}\"", entry_name(&source));
        let folds_into_back = self
            .undo
            .back()
            .map(|entry| entry.batch.is_some() && entry.batch == Some(batch))
            .unwrap_or(false);
        let folding = self.batch == Some(batch) && folds_into_back;
        if folding {
            if let Some(entry) = self.undo.back_mut() {
                entry.actions.push(action);
                return;
            }
        }
        self.push_undo(UndoEntry {
            label: first_label,
            node,
            actions: vec![action],
            batch: Some(batch),
            trashed: Vec::new(),
        });
        self.batch = Some(batch);
    }

    // -- Replay ------------------------------------------------------------

    /// Undo the most recent operation. The entry stays on the stack when the
    /// undo is refused, so a refusal never changes the current state.
    pub fn undo(
        &mut self,
        ops: &dyn FileOps,
        trash: &dyn TrashOps,
    ) -> Result<UndoOutcome, UndoError> {
        let Some(entry) = self.undo.pop_back() else {
            return Err(UndoError::Empty);
        };
        match replay(&entry, Direction::Undo, ops, trash) {
            Ok((opposite, effect)) => {
                self.push_redo(opposite);
                Ok(UndoOutcome {
                    label: entry.label,
                    node: entry.node,
                    renamed: effect.renamed,
                    renamed_from: effect.was,
                    restored: effect.restored,
                    removed: effect.removed,
                })
            }
            Err(error) => {
                self.undo.push_back(entry);
                Err(error)
            }
        }
    }

    /// Redo the most recently undone operation.
    pub fn redo(
        &mut self,
        ops: &dyn FileOps,
        trash: &dyn TrashOps,
    ) -> Result<UndoOutcome, UndoError> {
        let Some(entry) = self.redo.pop() else {
            return Err(UndoError::Empty);
        };
        match replay(&entry, Direction::Redo, ops, trash) {
            Ok((opposite, effect)) => {
                if self.limit > 0 {
                    self.undo.push_back(opposite);
                    while self.undo.len() > self.limit {
                        self.undo.pop_front();
                    }
                }
                Ok(UndoOutcome {
                    label: entry.label,
                    node: entry.node,
                    renamed: effect.renamed,
                    renamed_from: effect.was,
                    restored: effect.restored,
                    removed: effect.removed,
                })
            }
            Err(error) => {
                self.redo.push(entry);
                Err(error)
            }
        }
    }
}

/// Replay every action of `entry`, then return the entry that reverses it.
///
/// The whole entry is checked first (each action's refusal conditions), so a
/// conflict changes nothing. A filesystem failure part way through reverses
/// the actions already replayed, best effort, and reports the failure.
fn replay(
    entry: &UndoEntry,
    direction: Direction,
    ops: &dyn FileOps,
    trash: &dyn TrashOps,
) -> Result<(UndoEntry, ReplayEffect), UndoError> {
    let steps: Vec<Step> = entry
        .actions
        .iter()
        .enumerate()
        .map(|(index, action)| Step::for_action(action, direction, &entry.trashed, index))
        .collect();

    for step in &steps {
        step.check(ops)?;
    }

    let mut effect = ReplayEffect::default();
    let mut done: Vec<&Step> = Vec::new();
    for step in &steps {
        if let Err(error) = step.run(ops, trash) {
            for replayed in done.into_iter().rev() {
                let _ = replayed.run(ops, trash);
            }
            return Err(UndoError::Failed(error));
        }
        if let StepKind::Rename { current, target } = &step.kind {
            effect.renamed = Some((*target).clone());
            effect.was = Some((*current).clone());
        }
        if let (StepKind::Restore { original, .. }, Direction::Undo) = (&step.kind, direction) {
            effect.restored = Some((*original).clone());
        }
        if let StepKind::RemoveCreated { created, .. } = &step.kind {
            effect.removed = Some((*created).clone());
        }
        done.push(step);
    }

    let actions: Vec<UndoAction> = steps.iter().map(Step::reverse).collect();
    let trashed: Vec<TrashedItem> = steps.iter().filter_map(Step::trashed).cloned().collect();
    Ok((
        UndoEntry {
            label: entry.label.clone(),
            node: entry.node,
            actions,
            batch: entry.batch,
            trashed,
        },
        effect,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Undo,
    Redo,
}

/// One action's replay as an undo or a redo.
struct Step<'a> {
    kind: StepKind<'a>,
    /// The recorded store item, for a Put Back.
    trashed: Option<&'a TrashedItem>,
}

enum StepKind<'a> {
    Rename {
        current: &'a Location,
        target: &'a Location,
    },
    RemoveCreated {
        created: &'a Location,
        kind: NodeKind,
        copied: bool,
    },
    Move {
        current: &'a Location,
        target: &'a Location,
    },
    Restore {
        original: &'a Location,
        stored: &'a Location,
    },
    Trash {
        original: &'a Location,
        stored: &'a Location,
    },
}

impl<'a> Step<'a> {
    fn for_action(
        action: &'a UndoAction,
        direction: Direction,
        trashed: &'a [TrashedItem],
        index: usize,
    ) -> Self {
        let kind = match (action, direction) {
            (UndoAction::Rename { from, to }, _) => StepKind::Rename {
                current: to,
                target: from,
            },
            (UndoAction::Create { to, kind, copied }, _) => StepKind::RemoveCreated {
                created: to,
                kind: *kind,
                copied: *copied,
            },
            (UndoAction::Move { from, to }, _) => StepKind::Move {
                current: to,
                target: from,
            },
            (UndoAction::Trash { from, to }, Direction::Undo) => StepKind::Restore {
                original: from,
                stored: to,
            },
            (UndoAction::Trash { from, to }, Direction::Redo) => StepKind::Trash {
                original: from,
                stored: to,
            },
        };
        let trashed = match kind {
            StepKind::Restore { .. } => trashed.get(index),
            _ => None,
        };
        Self { kind, trashed }
    }

    fn check(&self, ops: &dyn FileOps) -> Result<(), UndoError> {
        match &self.kind {
            StepKind::Rename { current, target } => check_rename(ops, current, target),
            StepKind::RemoveCreated {
                created,
                kind,
                copied,
            } => check_remove_created(ops, created, *kind, *copied),
            StepKind::Move { current, target } => check_move(ops, current, target),
            StepKind::Restore { original, stored } => {
                let item = self
                    .trashed
                    .unwrap_or_else(|| UNRECORDED.get_or_init(unrecorded_trash));
                check_trash_restore(ops, original, stored, item)
            }
            StepKind::Trash { original, stored } => check_trash(ops, original, stored),
        }
    }

    fn run(&self, ops: &dyn FileOps, trash: &dyn TrashOps) -> Result<(), OperationError> {
        match &self.kind {
            StepKind::Rename { current, target } => rename_to(ops, current, target),
            StepKind::RemoveCreated { created, .. } => ops.delete(created),
            StepKind::Move { current, target } => move_to(ops, current, target),
            StepKind::Restore { .. } => {
                let item = self
                    .trashed
                    .unwrap_or_else(|| UNRECORDED.get_or_init(unrecorded_trash));
                trash.restore(item).map(|_| ())
            }
            StepKind::Trash { original, .. } => trash.trash(original).map(|_| ()),
        }
    }

    fn reverse(&self) -> UndoAction {
        match &self.kind {
            StepKind::Rename { current, target } => UndoAction::Rename {
                from: (*current).clone(),
                to: (*target).clone(),
            },
            StepKind::RemoveCreated { created, .. } => UndoAction::Create {
                to: (*created).clone(),
                kind: NodeKind::Other,
                copied: true,
            },
            StepKind::Move { current, target } => UndoAction::Move {
                from: (*current).clone(),
                to: (*target).clone(),
            },
            StepKind::Restore { original, stored } => UndoAction::Trash {
                from: (*original).clone(),
                to: (*stored).clone(),
            },
            StepKind::Trash { original, stored } => UndoAction::Trash {
                from: (*stored).clone(),
                to: (*original).clone(),
            },
        }
    }

    fn trashed(&self) -> Option<&'a TrashedItem> {
        self.trashed
    }
}

/// The final path component of a location, as a display name.
fn entry_name(location: &Location) -> String {
    location
        .to_file_path()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| location.display())
}

/// The name `location` has, as raw bytes.
fn raw_name(location: &Location) -> Option<OsString> {
    location
        .to_file_path()
        .and_then(|path| path.file_name().map(|name| name.to_os_string()))
}

fn exists(ops: &dyn FileOps, location: &Location) -> bool {
    ops.exists(location)
}

/// The stand-in for a Trash entry whose store item was not recorded. It is
/// reported as a `trash-gone` conflict and never used for a Put Back.
fn unrecorded_trash() -> TrashedItem {
    TrashedItem::unrecorded()
}

/// A process-wide stand-in for an unrecorded Trash entry (see
/// [`unrecorded_trash`]). It lives for the process because a [`Step`] borrows
/// it for one replay; it is never passed to the trash seam.
static UNRECORDED: std::sync::OnceLock<TrashedItem> = std::sync::OnceLock::new();

/// Rename `current` to `target`'s name (the target may share its folder).
fn rename_to(
    ops: &dyn FileOps,
    current: &Location,
    target: &Location,
) -> Result<(), OperationError> {
    let Some(name) = raw_name(target) else {
        return Err(OperationError::InvalidName(target.display()));
    };
    ops.rename(current, &name).map(|_| ())
}

/// Move `current` into `target`'s folder under its name.
fn move_to(ops: &dyn FileOps, current: &Location, target: &Location) -> Result<(), OperationError> {
    let (Some(parent), Some(name)) = (target.parent(), raw_name(target)) else {
        return Err(OperationError::InvalidName(target.display()));
    };
    ops.move_as(current, &parent, &name).map(|_| ())
}

fn check_rename(ops: &dyn FileOps, current: &Location, target: &Location) -> Result<(), UndoError> {
    if !exists(ops, current) {
        return Err(missing(current));
    }
    if exists(ops, target) && target != current {
        return Err(UndoError::Conflict(UndoConflict::new(
            "exists",
            format!("\"{}\" already exists", entry_name(target)),
            target,
        )));
    }
    Ok(())
}

fn check_move(ops: &dyn FileOps, current: &Location, target: &Location) -> Result<(), UndoError> {
    if !exists(ops, current) {
        return Err(moved_or_missing(current));
    }
    let Some(parent) = target.parent() else {
        return Err(UndoError::Conflict(UndoConflict::new(
            "parent-gone",
            format!("\"{}\" has no parent folder", target.display()),
            target,
        )));
    };
    if !exists(ops, &parent) {
        return Err(UndoError::Conflict(UndoConflict::new(
            "parent-gone",
            format!("The folder \"{}\" is gone", entry_name(&parent)),
            &parent,
        )));
    }
    if exists(ops, target) && target != current {
        return Err(UndoError::Conflict(UndoConflict::new(
            "exists",
            format!("\"{}\" already exists", entry_name(target)),
            target,
        )));
    }
    Ok(())
}

/// An item expected at `current` is absent. Replaying a *guess* at another
/// path would undo the wrong item, so the recorded location is the truth: if
/// the item moved by hand, the reason says so.
fn moved_or_missing(current: &Location) -> UndoError {
    UndoError::Conflict(UndoConflict::new(
        "moved",
        format!("\"{}\" has been moved or removed", entry_name(current)),
        current,
    ))
}

fn missing(location: &Location) -> UndoError {
    UndoError::Conflict(UndoConflict::new(
        "missing",
        format!("\"{}\" is no longer there", entry_name(location)),
        location,
    ))
}

/// A new folder must still be empty; deleting something the user has since
/// put files in is data loss, not undo. A pasted copy is removed as a tree.
fn check_remove_created(
    ops: &dyn FileOps,
    created: &Location,
    kind: NodeKind,
    copied: bool,
) -> Result<(), UndoError> {
    if !exists(ops, created) {
        return Err(missing(created));
    }
    if kind == NodeKind::Directory && !copied && !is_empty_folder(created) {
        return Err(UndoError::Conflict(UndoConflict::new(
            "not-empty",
            format!("\"{}\" is not empty", entry_name(created)),
            created,
        )));
    }
    Ok(())
}

/// Whether `folder` has no live entries (a missing folder is not empty).
fn is_empty_folder(folder: &Location) -> bool {
    match folder.to_file_path() {
        Some(path) => std::fs::read_dir(&path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false),
        None => false,
    }
}

/// A Put Back needs the original spot free, its folder present, and the store
/// entry still there.
fn check_trash_restore(
    ops: &dyn FileOps,
    original: &Location,
    stored: &Location,
    item: &TrashedItem,
) -> Result<(), UndoError> {
    if !item.is_recorded() {
        return Err(UndoError::Conflict(UndoConflict::new(
            "trash-gone",
            "The Trash entry was not recorded".to_owned(),
            original,
        )));
    }
    if !exists(ops, stored) {
        return Err(UndoError::Conflict(UndoConflict::new(
            "trash-gone",
            format!("\"{}\" is no longer in the Trash", entry_name(original)),
            stored,
        )));
    }
    if exists(ops, original) {
        return Err(UndoError::Conflict(UndoConflict::new(
            "exists",
            format!("\"{}\" already exists", entry_name(original)),
            original,
        )));
    }
    if let Some(parent) = original.parent() {
        if !exists(ops, &parent) {
            return Err(UndoError::Conflict(UndoConflict::new(
                "parent-gone",
                format!("The folder \"{}\" is gone", entry_name(&parent)),
                &parent,
            )));
        }
    }
    Ok(())
}

/// Redo a Move to Trash: the item must be back where it was and the store
/// slot free.
fn check_trash(ops: &dyn FileOps, original: &Location, stored: &Location) -> Result<(), UndoError> {
    if !exists(ops, original) {
        return Err(missing(original));
    }
    if exists(ops, stored) {
        return Err(UndoError::Conflict(UndoConflict::new(
            "exists",
            format!("The Trash already holds \"{}\"", entry_name(stored)),
            stored,
        )));
    }
    Ok(())
}
