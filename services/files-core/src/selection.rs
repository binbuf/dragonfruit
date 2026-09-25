// SPDX-License-Identifier: MIT
//! The selected set of node ids ([09-files.md]).
//!
//! Selection is view state, but it is keyed by the model's stable [`NodeId`],
//! so it survives a re-sort, a rename, and the optimistic layer's edits — a
//! node keeps its id for the session. Keeping it in `files-core` rather than
//! re-deriving it per view lets browser windows, the desktop surface, and the
//! portal chooser agree on what is selected.
//!
//! Insertion order is preserved (the order the user selected in), which a
//! Quick Look or "open all" flow needs; membership is O(1) through a mirror
//! set. `select` ignores [`NodeId::UNSET`], because a selection always names
//! rows the model has assigned an id.
//!
//! ## State preservation across operations
//!
//! The optimistic layer ([`crate::OptimisticModel`]) edits the model in place
//! but never renumbers nodes, so a view's selection needs no fix-up on
//! rename, new folder, or re-sort. Only a confirmed delete loses an id; an
//! unconfirmed one is restored on revert ([`Selection::select`] again). A
//! selection can be reconciled against the model at any time with
//! [`Selection::prune`].
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::collections::HashSet;

use crate::{DirectoryModel, NodeId};

/// An insertion-ordered set of selected [`NodeId`]s.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Selection {
    order: Vec<NodeId>,
    index: HashSet<NodeId>,
}

impl Selection {
    /// An empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many nodes are selected.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Whether `id` is selected.
    pub fn contains(&self, id: NodeId) -> bool {
        self.index.contains(&id)
    }

    /// The selected ids in the order they were selected.
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.order.iter().copied()
    }

    /// The selected ids as a slice.
    pub fn as_slice(&self) -> &[NodeId] {
        &self.order
    }

    /// The insertion position of `id`, if selected.
    pub fn position_of(&self, id: NodeId) -> Option<usize> {
        if !self.index.contains(&id) {
            return None;
        }
        self.order.iter().position(|&existing| existing == id)
    }

    /// Add `id` at the end. Returns whether it was newly added.
    ///
    /// [`NodeId::UNSET`] is ignored so a placeholder id can never enter the
    /// selection.
    pub fn select(&mut self, id: NodeId) -> bool {
        if !id.is_set() || !self.index.insert(id) {
            return false;
        }
        self.order.push(id);
        true
    }

    /// Re-insert `id` at `position` (clamped), preserving the selection order
    /// around it. Used to undo an optimistic delete. Returns whether it was
    /// newly added.
    pub fn insert_at(&mut self, position: usize, id: NodeId) -> bool {
        if !id.is_set() || !self.index.insert(id) {
            return false;
        }
        let position = position.min(self.order.len());
        self.order.insert(position, id);
        true
    }

    /// Remove `id`. Returns whether it was present.
    pub fn deselect(&mut self, id: NodeId) -> bool {
        if !self.index.remove(&id) {
            return false;
        }
        self.order.retain(|&existing| existing != id);
        true
    }

    /// Add `id` if absent, remove it if present. Returns the new membership.
    pub fn toggle(&mut self, id: NodeId) -> bool {
        if self.contains(id) {
            self.deselect(id);
            false
        } else {
            self.select(id);
            true
        }
    }

    /// Deselect everything.
    pub fn clear(&mut self) {
        self.order.clear();
        self.index.clear();
    }

    /// Replace the whole selection with `ids`, dropping placeholders and
    /// duplicates while keeping first-seen order.
    pub fn replace_all(&mut self, ids: impl IntoIterator<Item = NodeId>) {
        self.clear();
        for id in ids {
            self.select(id);
        }
    }

    /// Keep only the ids `keep` accepts; the selection orders itself.
    pub fn retain(&mut self, mut keep: impl FnMut(NodeId) -> bool) {
        self.order.retain(|&id| keep(id));
        self.index = self.order.iter().copied().collect();
    }

    /// Drop selected ids the model no longer holds — a confirmed delete, or a
    /// new listing after navigation. This is the one reconciliation a view
    /// needs; it never touches the ids that are still present.
    pub fn prune(&mut self, model: &DirectoryModel) {
        self.retain(|id| model.node(id).is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: u64) -> NodeId {
        NodeId::from_raw(value)
    }

    #[test]
    fn selection_preserves_insertion_order_and_deduplicates() {
        let mut selection = Selection::new();
        assert!(selection.select(id(1)));
        assert!(selection.select(id(2)));
        assert!(!selection.select(id(1)));
        assert_eq!(selection.as_slice(), &[id(1), id(2)]);
        assert_eq!(selection.len(), 2);
        assert!(selection.contains(id(2)));
    }

    #[test]
    fn placeholders_are_never_selected() {
        let mut selection = Selection::new();
        assert!(!selection.select(NodeId::UNSET));
        assert!(selection.is_empty());
    }

    #[test]
    fn toggle_and_deselect_are_symmetric() {
        let mut selection = Selection::new();
        assert!(selection.toggle(id(3)));
        assert!(!selection.toggle(id(3)));
        assert!(selection.is_empty());
        selection.select(id(3));
        assert!(selection.deselect(id(3)));
        assert!(!selection.deselect(id(3)));
    }

    #[test]
    fn insert_at_restores_a_deleted_position() {
        let mut selection = Selection::new();
        selection.select(id(1));
        selection.select(id(2));
        selection.select(id(3));
        let position = selection.position_of(id(2)).expect("selected");
        selection.deselect(id(2));
        assert_eq!(selection.as_slice(), &[id(1), id(3)]);
        selection.insert_at(position, id(2));
        assert_eq!(selection.as_slice(), &[id(1), id(2), id(3)]);
        assert_eq!(selection.position_of(NodeId::UNSET), None);
    }

    #[test]
    fn replace_all_keeps_first_seen_order() {
        let mut selection = Selection::new();
        selection.select(id(9));
        selection.replace_all([id(2), NodeId::UNSET, id(1), id(2)]);
        assert_eq!(selection.as_slice(), &[id(2), id(1)]);
    }
}
