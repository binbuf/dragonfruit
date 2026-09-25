// SPDX-License-Identifier: MIT
//! Optimistic semantics and state preservation (T-10.2b, [09-files.md]).
//!
//! The design's hard rule is **no I/O on the UI thread, and optimistic
//! rendering reconciled by monitors**: rename, new-folder, and trash must be
//! visible within one frame, then confirmed or snapped back by the change
//! monitor. [`OptimisticModel`] is that rule's headless half. It wraps a
//! [`DirectoryModel`] and a [`Selection`] and applies a mutation to the model
//! **synchronously**, so the next paint already shows it, while keeping a
//! pending record that can be [`OptimisticModel::confirm`]ed (the real
//! operation succeeded) or [`OptimisticModel::revert`]ed (it failed, or the
//! monitor never saw it).
//!
//! # One frame, then reconciliation
//!
//! `begin_rename` / `begin_new_folder` / `begin_delete` each edit the model in
//! place and return an [`OpId`]. The edit is visible immediately — no worker,
//! no await. The caller then runs the real operation off the UI thread
//! (through the [`FileOps`] seam) and reports the outcome:
//!
//! * success → [`OptimisticModel::confirm`], which simply retires the pending
//!   record and leaves the model as painted;
//! * failure → [`OptimisticModel::revert`], which restores the captured node
//!   or selection, and the item snaps back.
//!
//! The `*_via` convenience methods do both halves in one synchronous call for
//! callers already on a worker thread; they confirm on success and revert on
//! error, so the model and the filesystem never disagree.
//!
//! # State preservation
//!
//! Node ids are stable and are never renumbered by an optimistic edit, so a
//! [`Selection`] keyed by id survives a rename, a new folder, and a re-sort.
//! Deleting a selected node drops it from the selection; reverting the delete
//! puts it back. Sorting is model state ([`DirectoryModel::set_sort`]) and is
//! untouched by every operation; each edit re-sorts the affected node into the
//! current order.
//!
//! # Deferred
//!
//! The change monitor/folder watcher (T-10.3b) will drive `confirm`/`revert`
//! automatically; today the operation result does. Trash (`trash://`) is
//! T-10.3a; [`OptimisticModel::begin_delete`] is the permanent-delete path the
//! trash path will reuse, and [`OptimisticModel::trash_via`] is that reuse.
//! Undo/redo, progress, conflict policy, and the journal are later still.
//!
//! ```no_run
//! use std::ffi::OsStr;
//! use std::sync::Arc;
//! use dragonfruit_files_core::{
//!     DirectoryModel, FileOps, Location, OptimisticModel, StdFsOps,
//! };
//!
//! let ops = StdFsOps::new();
//! let dir = Location::file("/tmp");
//! let mut optimistic = OptimisticModel::new(DirectoryModel::new());
//! // ... the listing has since delivered nodes ...
//! # let node_id = dragonfruit_files_core::NodeId::UNSET;
//! # if node_id.is_set() {
//! // Optimistic: paints now, runs for real, and reconciles.
//! let _ = optimistic.rename_via(&ops, node_id, OsStr::new("renamed"));
//! # }
//! # let _ = Arc::new(ops);
//! ```
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::ffi::{OsStr, OsString};
use std::sync::Arc;

use crate::{
    generated_name, DirectoryModel, FileOps, FolderWatcher, ListingEvent, ListingHandle, Location,
    Node, NodeId, NodeKind, OperationError, Selection, SourceError, TrashOps, WatchEvent,
    WatchEventKind, WatchHandle, NEW_FOLDER_BASE,
};

/// A token for one in-flight optimistic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OpId(u64);

impl OpId {
    /// The raw value, for a bridge that needs a plain integer.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// What a pending operation changed, and how to undo it.
enum PendingKind {
    /// The node was renamed; `previous` is the pre-rename node to restore.
    Rename { node: NodeId, previous: Node },
    /// A provisional node was inserted; revert removes it.
    Create { node: NodeId },
    /// The node was removed; revert re-inserts it at `position` and restores
    /// its selection position, if it was selected.
    Remove {
        node: Node,
        position: usize,
        selection_index: Option<usize>,
    },
}

/// One pending optimistic edit.
struct PendingOp {
    id: OpId,
    kind: PendingKind,
}

/// A [`DirectoryModel`] plus selection, with optimistic mutations and
/// confirm/revert reconciliation.
pub struct OptimisticModel {
    model: DirectoryModel,
    selection: Selection,
    pending: Vec<PendingOp>,
    next_op: u64,
}

impl Default for OptimisticModel {
    fn default() -> Self {
        Self::new(DirectoryModel::new())
    }
}

impl OptimisticModel {
    /// Wrap an existing model (one that may already be streaming).
    pub fn new(model: DirectoryModel) -> Self {
        Self {
            model,
            selection: Selection::new(),
            pending: Vec::new(),
            next_op: 0,
        }
    }

    /// The underlying listing model. Read-only: mutations go through the
    /// optimistic methods so they can be reverted.
    pub fn model(&self) -> &DirectoryModel {
        &self.model
    }

    /// The current selection.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Mutable access to the selection, for the view's own selection gestures.
    pub fn selection_mut(&mut self) -> &mut Selection {
        &mut self.selection
    }

    /// What the model sorts under. Unchanged by every operation.
    pub fn sort_spec(&self) -> &crate::SortSpec {
        self.model.sort_spec()
    }

    /// Change the model's order. Node ids and the selection are untouched, so
    /// a view re-sorts without losing what was selected.
    pub fn set_sort(&mut self, spec: crate::SortSpec) {
        self.model.set_sort(spec);
    }

    /// Begin streaming `location`, clearing the selection and every pending
    /// edit: the old directory's in-flight changes no longer apply.
    pub fn begin(
        &mut self,
        source: Arc<dyn crate::DirectorySource>,
        location: Location,
    ) -> ListingHandle {
        self.pending.clear();
        self.selection.clear();
        self.model.begin(source, location)
    }

    /// Forward one listing event to the model, as `DirectoryModel::apply`.
    pub fn apply(&mut self, event: ListingEvent) -> bool {
        self.model.apply(event)
    }

    /// Apply every ready event, as `DirectoryModel::drain`.
    pub fn drain(&mut self, handle: &ListingHandle) -> usize {
        self.model.drain(handle)
    }

    /// Begin watching the current location, as `DirectoryModel::begin_watch`.
    pub fn begin_watch(
        &mut self,
        watcher: Arc<dyn FolderWatcher>,
        location: &Location,
    ) -> Result<WatchHandle, SourceError> {
        self.model.begin_watch(watcher, location)
    }

    /// Fold one external change into the model, reconciling pending optimistic
    /// edits first (T-10.3b).
    ///
    /// A watch event that matches a pending edit's target state **confirms** it
    /// (the filesystem agreed); an event that contradicts it **reverts** it (the
    /// item vanished before confirmation, or came back after one). Either way
    /// the change is then folded into the model incrementally, so a watcher
    /// resolves each pending op at most once and never re-lists.
    pub fn apply_watch(&mut self, event: WatchEvent) -> bool {
        let decisions = self.pending_decisions(&event);
        for (op, confirm) in decisions {
            if confirm {
                self.confirm(op);
            } else {
                self.revert(op);
            }
        }
        self.model.apply_watch(event)
    }

    /// Apply every ready watch event, as `DirectoryModel::drain_watch`. Returns
    /// how many events changed the model.
    pub fn drain_watch(&mut self, handle: &WatchHandle) -> usize {
        let mut applied = 0;
        while let Some(event) = handle.try_recv() {
            if self.apply_watch(event) {
                applied += 1;
            }
        }
        applied
    }

    /// Decide whether `event` resolves `pending`: `Some(true)` to confirm,
    /// `Some(false)` to revert, `None` to leave it pending.
    fn pending_outcome(&self, pending: &PendingOp, event: &WatchEvent) -> Option<bool> {
        match &pending.kind {
            PendingKind::Create { node } | PendingKind::Rename { node, .. } => {
                // The current model row carries the edit's target URI (a rename
                // already moved it; a create is the generated name).
                let target = self.model.node(*node)?.uri();
                match &event.kind {
                    WatchEventKind::Created(node) | WatchEventKind::Modified(node) => {
                        (node.uri() == target).then_some(true)
                    }
                    WatchEventKind::Renamed { to, .. } => (to.uri() == target).then_some(true),
                    // A create/rename target that was removed before the
                    // operation confirmed: snap a create back. A rename that
                    // succeeded and was then deleted confirms, and the removal
                    // is folded afterwards.
                    WatchEventKind::Removed { uri } => match pending.kind {
                        PendingKind::Create { .. } if uri == target => Some(false),
                        _ => None,
                    },
                }
            }
            PendingKind::Remove { node, .. } => {
                let target = node.uri();
                match &event.kind {
                    WatchEventKind::Removed { uri } => (uri == target).then_some(true),
                    WatchEventKind::Created(node) | WatchEventKind::Modified(node) => {
                        (node.uri() == target).then_some(false)
                    }
                    WatchEventKind::Renamed { from_uri, .. } => {
                        (from_uri == target).then_some(false)
                    }
                }
            }
        }
    }

    fn pending_decisions(&self, event: &WatchEvent) -> Vec<(OpId, bool)> {
        self.pending
            .iter()
            .filter_map(|pending| {
                self.pending_outcome(pending, event)
                    .map(|ok| (pending.id, ok))
            })
            .collect()
    }

    /// How many optimistic edits are awaiting confirmation or revert.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Whether `op` is still pending.
    pub fn is_pending(&self, op: OpId) -> bool {
        self.pending.iter().any(|pending| pending.id == op)
    }

    /// The pending op ids, in the order they were begun.
    pub fn pending_ids(&self) -> impl Iterator<Item = OpId> + '_ {
        self.pending.iter().map(|pending| pending.id)
    }

    /// Rename `id` in the model now, returning the pending [`OpId`].
    ///
    /// The node's id, the model's sort spec, and the selection are all
    /// unchanged; only the name and URI move, and the node re-sorts into the
    /// current order. `None` if `id` is unknown.
    pub fn begin_rename(&mut self, id: NodeId, new_name: impl Into<OsString>) -> Option<OpId> {
        let previous = self.model.node(id)?.clone();
        if !self.model.rename_node(id, new_name) {
            return None;
        }
        Some(self.push(PendingKind::Rename { node: id, previous }))
    }

    /// Insert a provisional `untitled folder` under `parent` now, returning
    /// the pending [`OpId`].
    ///
    /// The name is generated against the nodes already in the model with the
    /// same [`generated_name`] helper the real operation uses, so the
    /// optimistic row and the confirmed one usually agree. The new node is a
    /// directory, sorts folders-first under the current spec, and does not
    /// disturb the selection.
    pub fn begin_new_folder(&mut self, parent: &Location) -> OpId {
        let name = generated_name(OsStr::new(NEW_FOLDER_BASE), |candidate| {
            self.model
                .nodes()
                .iter()
                .any(|node| node.name() == candidate)
        });
        let uri = parent.child(&name).uri().to_owned();
        let node = Node::new(&name, uri, NodeKind::Directory);
        let id = self.model.insert_node(node);
        self.push(PendingKind::Create { node: id })
    }

    /// Remove `id` from the model now, returning the pending [`OpId`].
    ///
    /// This is the optimistic path for Move to Trash and Delete Immediately;
    /// both hide the row within a frame and revert if the operation fails. A
    /// selected node is deselected and re-selected if reverted. `None` if `id`
    /// is unknown.
    pub fn begin_delete(&mut self, id: NodeId) -> Option<OpId> {
        let (node, position) = self.model.remove_node(id)?;
        let selection_index = self.selection.position_of(id);
        self.selection.deselect(id);
        Some(self.push(PendingKind::Remove {
            node,
            position,
            selection_index,
        }))
    }

    /// Retire a pending edit whose real operation succeeded: the model already
    /// shows the result, so nothing is restored. Returns `false` if `op` was
    /// not pending.
    pub fn confirm(&mut self, op: OpId) -> bool {
        match self.pending.iter().position(|pending| pending.id == op) {
            Some(index) => {
                self.pending.remove(index);
                true
            }
            None => false,
        }
    }

    /// Undo a pending edit whose real operation failed: restore the captured
    /// node, selection membership, and sort position. Returns `false` if `op`
    /// was not pending.
    ///
    /// For exact arrival positions when several removals are pending, revert
    /// in reverse order (the pending list is a stack).
    pub fn revert(&mut self, op: OpId) -> bool {
        let Some(index) = self.pending.iter().position(|pending| pending.id == op) else {
            return false;
        };
        match self.pending.remove(index).kind {
            PendingKind::Rename { node, previous } => {
                self.model.replace_node(node, previous);
            }
            PendingKind::Create { node } => {
                self.model.remove_node(node);
                self.selection.deselect(node);
            }
            PendingKind::Remove {
                node,
                position,
                selection_index,
            } => {
                let id = node.id();
                if self.model.restore_node(node, id, position) {
                    if let Some(index) = selection_index {
                        self.selection.insert_at(index, id);
                    }
                }
            }
        }
        true
    }

    /// Whether the current listing is complete.
    pub fn is_complete(&self) -> bool {
        self.model.is_complete()
    }

    /// The number of listed nodes.
    pub fn len(&self) -> usize {
        self.model.len()
    }

    /// Whether the model is empty.
    pub fn is_empty(&self) -> bool {
        self.model.is_empty()
    }

    /// Rename `id` optimistically and run the real rename through `ops`,
    /// confirming on success and reverting on error.
    ///
    /// Synchronous and possibly blocking: call it off the UI thread, like
    /// every other [`FileOps`] call.
    pub fn rename_via(
        &mut self,
        ops: &dyn FileOps,
        id: NodeId,
        new_name: &OsStr,
    ) -> Result<OpId, OperationError> {
        let location = self.location_of(id)?;
        let op = self
            .begin_rename(id, new_name.to_os_string())
            .ok_or_else(|| OperationError::NotFound(id.to_string()))?;
        match ops.rename(&location, new_name) {
            Ok(actual) => {
                self.retarget(op, &actual);
                self.confirm(op);
                Ok(op)
            }
            Err(error) => {
                self.revert(op);
                Err(error)
            }
        }
    }

    /// Create the next `untitled folder` optimistically and run the real
    /// [`FileOps::new_folder`], confirming on success and reverting on error.
    pub fn new_folder_via(
        &mut self,
        ops: &dyn FileOps,
        parent: &Location,
    ) -> Result<OpId, OperationError> {
        let op = self.begin_new_folder(parent);
        match ops.new_folder(parent) {
            Ok(actual) => {
                self.retarget(op, &actual);
                self.confirm(op);
                Ok(op)
            }
            Err(error) => {
                self.revert(op);
                Err(error)
            }
        }
    }

    /// Delete `id` optimistically and run the real permanent delete through
    /// `ops`, confirming on success and reverting on error.
    pub fn delete_via(&mut self, ops: &dyn FileOps, id: NodeId) -> Result<OpId, OperationError> {
        let location = self.location_of(id)?;
        let op = self
            .begin_delete(id)
            .ok_or_else(|| OperationError::NotFound(id.to_string()))?;
        match ops.delete(&location) {
            Ok(()) => {
                self.confirm(op);
                Ok(op)
            }
            Err(error) => {
                self.revert(op);
                Err(error)
            }
        }
    }

    /// Move `id` to Trash optimistically and run the real [`TrashOps::trash`]
    /// through `trash`, confirming on success and reverting on error
    /// (T-10.3a).
    ///
    /// Reuses [`OptimisticModel::begin_delete`] exactly as ADR 0045 records —
    /// the row disappears within a frame, and snaps back if the trashing
    /// fails. The returned [`TrashedItem`](crate::TrashedItem) is dropped here;
    /// a view uses it to offer Put Back.
    pub fn trash_via(&mut self, trash: &dyn TrashOps, id: NodeId) -> Result<OpId, OperationError> {
        let location = self.location_of(id)?;
        let op = self
            .begin_delete(id)
            .ok_or_else(|| OperationError::NotFound(id.to_string()))?;
        match trash.trash(&location) {
            Ok(_) => {
                self.confirm(op);
                Ok(op)
            }
            Err(error) => {
                self.revert(op);
                Err(error)
            }
        }
    }

    fn mint(&mut self) -> OpId {
        self.next_op += 1;
        OpId(self.next_op)
    }

    fn push(&mut self, kind: PendingKind) -> OpId {
        let id = self.mint();
        self.pending.push(PendingOp { id, kind });
        id
    }

    fn location_of(&self, id: NodeId) -> Result<Location, OperationError> {
        let node = self
            .model
            .node(id)
            .ok_or_else(|| OperationError::NotFound(id.to_string()))?;
        Location::parse(node.uri()).map_err(|error| OperationError::InvalidName(error.to_string()))
    }

    /// Align a pending create/rename with the location the real operation
    /// reported, so an optimistic name that differed from the filesystem's
    /// choice is corrected before confirmation.
    fn retarget(&mut self, op: OpId, location: &Location) {
        let Some(pending) = self.pending.iter().find(|pending| pending.id == op) else {
            return;
        };
        let node_id = match pending.kind {
            PendingKind::Create { node } | PendingKind::Rename { node, .. } => node,
            PendingKind::Remove { .. } => return,
        };
        let Some(mut node) = self.model.node(node_id).cloned() else {
            return;
        };
        if let Some(name) = location_name(location) {
            node.set_name(name);
        }
        node.set_uri(location.uri().to_owned());
        self.model.replace_node(node_id, node);
    }
}

/// The final path component of a `file://` location, as raw bytes. `None` for
/// foreign schemes (whose names the URI already carries verbatim).
fn location_name(location: &Location) -> Option<OsString> {
    location
        .to_file_path()
        .and_then(|path| path.file_name().map(OsStr::to_os_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockSource;
    use std::time::Duration;

    fn file(label: &str) -> Node {
        Node::new(label, format!("file:///fixture/{label}"), NodeKind::File)
    }

    fn loaded(names: &[&str]) -> OptimisticModel {
        let batches: Vec<Vec<Node>> = names.iter().map(|name| vec![file(name)]).collect();
        let source = Arc::new(MockSource::streaming(batches));
        let mut optimistic = OptimisticModel::new(DirectoryModel::new());
        let handle = optimistic.begin(source, Location::file("/fixture"));
        while !optimistic.is_complete() {
            let Some(event) = handle.recv_timeout(Duration::from_secs(5)) else {
                break;
            };
            optimistic.apply(event);
        }
        optimistic
    }

    fn node_id(optimistic: &OptimisticModel, name: &str) -> NodeId {
        optimistic
            .model()
            .nodes()
            .iter()
            .find(|node| node.display_name() == name)
            .expect("node")
            .id()
    }

    #[test]
    fn revert_restores_a_removed_node_and_its_selection() {
        let mut optimistic = loaded(&["a", "b"]);
        let a = node_id(&optimistic, "a");
        optimistic.selection_mut().select(a);
        let op = optimistic.begin_delete(a).expect("pending delete");
        assert!(optimistic.model().node(a).is_none());
        assert!(!optimistic.selection().contains(a));
        optimistic.model().check_consistent().expect("consistent");
        assert!(optimistic.revert(op));
        assert_eq!(
            optimistic
                .model()
                .node(a)
                .map(|n| n.display_name().into_owned()),
            Some("a".to_owned())
        );
        assert!(optimistic.selection().contains(a));
        optimistic.model().check_consistent().expect("consistent");
    }

    #[test]
    fn confirm_keeps_the_optimistic_result() {
        let mut optimistic = loaded(&["a"]);
        let a = node_id(&optimistic, "a");
        let op = optimistic.begin_rename(a, "z").expect("pending rename");
        assert!(optimistic.confirm(op));
        assert!(!optimistic.is_pending(op));
        assert_eq!(
            optimistic
                .model()
                .node(a)
                .map(|n| n.display_name().into_owned()),
            Some("z".to_owned())
        );
        assert!(!optimistic.confirm(op), "confirm is idempotent-safe");
    }

    #[test]
    fn new_folder_uses_the_shared_generator_against_model_names() {
        let mut optimistic = loaded(&["untitled folder", "untitled folder 2"]);
        let parent = Location::file("/fixture");
        let op = optimistic.begin_new_folder(&parent);
        let created = optimistic
            .model()
            .nodes()
            .iter()
            .find(|node| node.name() == OsStr::new("untitled folder 3"))
            .expect("generated name");
        assert!(created.is_dir());
        assert!(optimistic.is_pending(op));
        optimistic.revert(op);
        assert!(optimistic
            .model()
            .nodes()
            .iter()
            .all(|node| node.name() != OsStr::new("untitled folder 3")));
    }
}
