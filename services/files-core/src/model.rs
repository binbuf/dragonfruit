// SPDX-License-Identifier: MIT
//! The accumulated directory listing ([09-files.md]).
//!
//! [`DirectoryModel`] owns the nodes a listing has delivered so far. Batches
//! are appended in arrival order, every node gets exactly one stable
//! [`NodeId`], and the id→index map always agrees with the vector, so a view
//! can bind to the slice and look up a selection by id without re-scanning.
//!
//! The model performs no I/O. [`DirectoryModel::begin`] starts a worker (see
//! [`crate::listing`]) and returns a [`ListingHandle`]; the consumer's event
//! loop drains it with [`DirectoryModel::drain`] or [`DirectoryModel::apply`].
//!
//! # Sorting is incremental and stable
//!
//! [`DirectoryModel::nodes`] always stays in arrival order; a separate
//! `order` index gives the sorted projection ([`DirectoryModel::ordered`]).
//! Each arriving batch is merged into that index (an O(n) stable merge, not a
//! re-sort), so the sorted view is valid after every batch. Equal keys keep
//! arrival order, and changing the [`SortSpec`] re-sorts in place without
//! touching node ids, so selection and drag state survive.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;

use crate::listing::{self, ListingEvent, ListingEventKind, ListingHandle, DEFAULT_BATCH};
use crate::sort::SortSpec;
use crate::source::{DirectorySource, SourceError};
use crate::watch::{self, FolderWatcher, WatchEvent, WatchEventKind, WatchHandle};
use crate::{Location, Node, NodeId};

/// Where a listing is in its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingState {
    /// No listing has begun.
    Idle,
    /// Entries are still arriving; the model may already render a prefix.
    Streaming,
    /// The directory was listed completely.
    Complete,
    /// The listing failed; the model holds whatever arrived before the error.
    Failed(SourceError),
}

/// The node collection for one directory.
pub struct DirectoryModel {
    generation: u64,
    location: Option<Location>,
    nodes: Vec<Node>,
    index: HashMap<NodeId, usize>,
    state: ListingState,
    next_id: u64,
    batch_size: usize,
    active_cancel: Option<Arc<AtomicBool>>,
    active_watch: Option<Arc<AtomicBool>>,
    sort: SortSpec,
    order: Vec<usize>,
}

impl Default for DirectoryModel {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryModel {
    /// An empty model with the [`DEFAULT_BATCH`] batch size.
    pub fn new() -> Self {
        Self {
            generation: 0,
            location: None,
            nodes: Vec::new(),
            index: HashMap::new(),
            state: ListingState::Idle,
            next_id: 0,
            batch_size: DEFAULT_BATCH,
            active_cancel: None,
            active_watch: None,
            sort: SortSpec::default(),
            order: Vec::new(),
        }
    }

    /// An empty model that asks the worker for `batch_size` entries per event.
    /// Smaller batches make streaming more visible; larger ones cut wakeups.
    pub fn with_batch_size(batch_size: usize) -> Self {
        Self {
            batch_size: batch_size.max(1),
            ..Self::new()
        }
    }

    /// Begin streaming `location` from `source`.
    ///
    /// Any previous listing is cancelled and its generation retired, the
    /// model is cleared, and its state becomes [`ListingState::Streaming`].
    /// The returned handle is drained by the caller's event loop.
    pub fn begin(&mut self, source: Arc<dyn DirectorySource>, location: Location) -> ListingHandle {
        if let Some(cancel) = self.active_cancel.take() {
            cancel.store(true, AtomicOrdering::SeqCst);
        }
        if let Some(cancel) = self.active_watch.take() {
            cancel.store(true, AtomicOrdering::SeqCst);
        }
        self.generation += 1;
        self.location = Some(location.clone());
        self.nodes.clear();
        self.index.clear();
        self.order.clear();
        self.state = ListingState::Streaming;
        self.next_id = 0;

        let handle = listing::begin(source, location, self.generation, self.batch_size);
        self.active_cancel = Some(handle.cancel_flag());
        handle
    }

    /// Append one event from the current listing.
    ///
    /// Returns `false` and changes nothing when the event belongs to a retired
    /// generation. Batches append nodes and assign ids; `Done` and `Error`
    /// move the state.
    pub fn apply(&mut self, event: ListingEvent) -> bool {
        if event.generation != self.generation {
            return false;
        }
        match event.kind {
            ListingEventKind::Batch(nodes) => {
                let mut added = Vec::with_capacity(nodes.len());
                for mut node in nodes {
                    self.next_id += 1;
                    let id = NodeId::from_raw(self.next_id);
                    node.set_id(id);
                    let position = self.nodes.len();
                    self.index.insert(id, position);
                    self.nodes.push(node);
                    added.push(position);
                }
                self.merge_sorted(&added);
                if self.state != ListingState::Complete {
                    self.state = ListingState::Streaming;
                }
            }
            ListingEventKind::Done => {
                self.state = ListingState::Complete;
            }
            ListingEventKind::Error(error) => {
                self.state = ListingState::Failed(error);
            }
        }
        true
    }

    /// Apply every event the handle has ready. Returns how many were applied.
    pub fn drain(&mut self, handle: &ListingHandle) -> usize {
        let mut applied = 0;
        while let Some(event) = handle.try_recv() {
            if self.apply(event) {
                applied += 1;
            }
        }
        applied
    }

    /// Begin watching the current location for external changes (T-10.3b).
    ///
    /// Call this after [`Self::begin`] has listed the location: the watch
    /// shares the listing's generation, so a later [`Self::begin`] retires the
    /// watch and its stale events are ignored. Any previous watch is cancelled.
    /// A foreign scheme or a missing folder is returned as an error rather than
    /// failing in the background.
    pub fn begin_watch(
        &mut self,
        watcher: Arc<dyn FolderWatcher>,
        location: &Location,
    ) -> Result<WatchHandle, SourceError> {
        if let Some(cancel) = self.active_watch.take() {
            cancel.store(true, AtomicOrdering::SeqCst);
        }
        let handle = watch::begin(watcher, location, self.generation)?;
        self.active_watch = Some(handle.cancel_flag());
        Ok(handle)
    }

    /// Fold one external change into the model incrementally.
    ///
    /// Returns `false` and changes nothing for an event from a retired
    /// generation, or a removal that names no listed node. A `Created` for an
    /// entry already present refreshes it rather than duplicating it; a
    /// `Renamed` keeps the existing node's id, so selection and drag state
    /// survive. No listing is ever restarted.
    pub fn apply_watch(&mut self, event: WatchEvent) -> bool {
        if event.generation != self.generation {
            return false;
        }
        match event.kind {
            WatchEventKind::Created(node) => match self.node_id_for_uri(node.uri()) {
                Some(id) => self.replace_node(id, node),
                None => {
                    self.insert_node(node);
                    true
                }
            },
            WatchEventKind::Modified(node) => match self.node_id_for_uri(node.uri()) {
                Some(id) => self.replace_node(id, node),
                // A modify for an entry we never listed: list it rather than
                // drop the change.
                None => {
                    self.insert_node(node);
                    true
                }
            },
            WatchEventKind::Removed { uri } => match self.node_id_for_uri(&uri) {
                Some(id) => self.remove_node(id).is_some(),
                None => false,
            },
            WatchEventKind::Renamed { from_uri, to } => match self.node_id_for_uri(&from_uri) {
                Some(id) => self.replace_node(id, to),
                None => {
                    self.insert_node(to);
                    true
                }
            },
        }
    }

    /// Apply every ready watch event. Returns how many were applied.
    pub fn drain_watch(&mut self, handle: &WatchHandle) -> usize {
        let mut applied = 0;
        while let Some(event) = handle.try_recv() {
            if self.apply_watch(event) {
                applied += 1;
            }
        }
        applied
    }

    /// The id of the listed node whose URI is `uri`, if any.
    ///
    /// A URI is unique within one directory, so this is the watcher's identity
    /// key. Linear in the listing; watch events are infrequent, so no second
    /// index is maintained.
    pub fn node_id_for_uri(&self, uri: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .find(|node| node.uri() == uri)
            .map(Node::id)
    }

    /// The listing generation; the handle and its events share it.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The location being listed, once `begin` has run.
    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }

    /// The nodes delivered so far, in arrival order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// The number of nodes delivered so far.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether no nodes have arrived yet.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The listing lifecycle state.
    pub fn state(&self) -> &ListingState {
        &self.state
    }

    /// Whether the listing finished completely.
    pub fn is_complete(&self) -> bool {
        self.state == ListingState::Complete
    }

    /// Whether entries are still expected.
    pub fn is_streaming(&self) -> bool {
        self.state == ListingState::Streaming
    }

    /// The failure, if the listing failed.
    pub fn error(&self) -> Option<&SourceError> {
        match &self.state {
            ListingState::Failed(error) => Some(error),
            _ => None,
        }
    }

    /// Look up a node by its stable id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.index.get(&id).and_then(|&index| self.nodes.get(index))
    }

    /// Look up a node's current position.
    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        self.index.get(&id).copied()
    }

    /// The order the model sorts under. Defaults to name, ascending, folders
    /// first.
    pub fn sort_spec(&self) -> &SortSpec {
        &self.sort
    }

    /// Change the order and re-sort the nodes already delivered.
    ///
    /// Node ids and the arrival-order slice ([`Self::nodes`]) are untouched,
    /// so a view keeping selection by id is not invalidated. Future batches
    /// are merged into the new order.
    pub fn set_sort(&mut self, spec: SortSpec) {
        if self.sort == spec {
            return;
        }
        self.sort = spec;
        let mut order: Vec<usize> = (0..self.nodes.len()).collect();
        order.sort_by(|&a, &b| self.compare_positions(a, b));
        self.order = order;
    }

    /// The sorted projection as positions into [`Self::nodes`], in order.
    pub fn ordered_indices(&self) -> &[usize] {
        &self.order
    }

    /// The nodes in sorted order. Iterates a view over the same nodes.
    pub fn ordered(&self) -> impl Iterator<Item = &Node> {
        self.order.iter().map(|&index| &self.nodes[index])
    }

    /// The node at sorted position `rank`, if the listing reaches that far.
    pub fn ordered_node(&self, rank: usize) -> Option<&Node> {
        self.order
            .get(rank)
            .and_then(|&index| self.nodes.get(index))
    }

    /// The sorted rank of a node id, scanning the order (O(n)). A view that
    /// keeps a rank map should maintain it from [`Self::ordered_indices`].
    pub fn ordered_position_of(&self, id: NodeId) -> Option<usize> {
        let arrival = self.index_of(id)?;
        self.order.iter().position(|&index| index == arrival)
    }

    /// Append `node` with a freshly assigned id and return that id.
    ///
    /// This is how the optimistic layer renders a new item (New Folder,
    /// Duplicate, Paste) before the filesystem has listed it. The node joins
    /// the sorted projection under the current [`SortSpec`] immediately, so a
    /// view paints it on the next frame. The id counter only ever grows, so a
    /// later [`Self::restore_node`] of a removed id can never collide.
    pub fn insert_node(&mut self, mut node: Node) -> NodeId {
        self.next_id += 1;
        let id = NodeId::from_raw(self.next_id);
        node.set_id(id);
        let position = self.nodes.len();
        self.index.insert(id, position);
        self.nodes.push(node);
        self.rebuild_order();
        id
    }

    /// Re-insert a previously removed `node` under its original `id` at
    /// arrival `position` (clamped). Used to revert an optimistic delete.
    ///
    /// Returns `false` if `id` is [`NodeId::UNSET`] or already present, so a
    /// stale revert can never duplicate a row. Restoring does not touch the
    /// id counter.
    pub fn restore_node(&mut self, mut node: Node, id: NodeId, position: usize) -> bool {
        if !id.is_set() || self.index.contains_key(&id) {
            return false;
        }
        let position = position.min(self.nodes.len());
        node.set_id(id);
        self.nodes.insert(position, node);
        for index in self.index.values_mut() {
            if *index >= position {
                *index += 1;
            }
        }
        self.index.insert(id, position);
        self.rebuild_order();
        true
    }

    /// Remove the node `id`, returning it and its arrival position.
    ///
    /// This is how the optimistic layer hides an item (Move to Trash, Delete
    /// Immediately) before the filesystem confirms it. The id becomes free
    /// for [`Self::restore_node`]; ids are never renumbered.
    pub fn remove_node(&mut self, id: NodeId) -> Option<(Node, usize)> {
        let position = self.index.remove(&id)?;
        let node = self.nodes.remove(position);
        for index in self.index.values_mut() {
            if *index > position {
                *index -= 1;
            }
        }
        self.rebuild_order();
        Some((node, position))
    }

    /// Replace the node `id` wholesale, keeping its id, and re-sort.
    ///
    /// Used to revert an optimistic rename (restore the captured node) or to
    /// retarget an optimistic creation once the real location is known.
    pub fn replace_node(&mut self, id: NodeId, mut node: Node) -> bool {
        let Some(&position) = self.index.get(&id) else {
            return false;
        };
        node.set_id(id);
        self.nodes[position] = node;
        self.rebuild_order();
        true
    }

    /// Rename the node `id` in place, updating its URI and re-sorting.
    ///
    /// The id is deliberately unchanged, so a selection keyed by id survives
    /// the rename ([09-files.md]). Returns `false` for an unknown id.
    pub fn rename_node(&mut self, id: NodeId, new_name: impl Into<OsString>) -> bool {
        let Some(&position) = self.index.get(&id) else {
            return false;
        };
        let new_name = new_name.into();
        let uri = renamed_uri(self.nodes[position].uri(), &new_name);
        let node = &mut self.nodes[position];
        node.set_name(new_name);
        node.set_uri(uri);
        self.rebuild_order();
        true
    }

    /// Recompute the sorted projection from scratch. Optimistic edits are
    /// infrequent, so a clean rebuild is safer than patching `order` in place;
    /// `sort_by` is stable, preserving the arrival-order tie-break.
    fn rebuild_order(&mut self) {
        let mut order: Vec<usize> = (0..self.nodes.len()).collect();
        order.sort_by(|&a, &b| self.compare_positions(a, b));
        self.order = order;
    }

    /// Merge freshly appended positions into the sorted order. The appended
    /// positions are in arrival order, and every one arrived after the
    /// existing nodes, so taking the existing side on ties is exactly the
    /// stable rule.
    fn merge_sorted(&mut self, added: &[usize]) {
        if added.is_empty() {
            return;
        }
        let existing = std::mem::take(&mut self.order);
        // Each batch arrives in arrival order; stable-sort the new positions
        // so the merge below joins two sorted sequences.
        let mut incoming = added.to_vec();
        incoming.sort_by(|&a, &b| self.compare_positions(a, b));
        if existing.is_empty() {
            self.order = incoming;
            return;
        }
        let mut merged = Vec::with_capacity(existing.len() + incoming.len());
        let (mut old, mut new) = (0, 0);
        while old < existing.len() && new < incoming.len() {
            if self.compare_positions(existing[old], incoming[new]) == Ordering::Greater {
                merged.push(incoming[new]);
                new += 1;
            } else {
                merged.push(existing[old]);
                old += 1;
            }
        }
        merged.extend_from_slice(&existing[old..]);
        merged.extend_from_slice(&incoming[new..]);
        self.order = merged;
    }

    fn compare_positions(&self, a: usize, b: usize) -> Ordering {
        self.sort.compare(&self.nodes[a], &self.nodes[b])
    }

    /// The worker batch size this model was built with.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Assert the model's invariants, returning the first violation.
    ///
    /// Used by tests and by the bridge's debug assertions: every id is set,
    /// unique, resolves to the same node, and the index is exact.
    pub fn check_consistent(&self) -> Result<(), String> {
        let mut seen = HashMap::with_capacity(self.nodes.len());
        for (position, node) in self.nodes.iter().enumerate() {
            let id = node.id();
            if !id.is_set() {
                return Err(format!("node at {position} has no id"));
            }
            if let Some(previous) = seen.insert(id, position) {
                return Err(format!("id {id} appears at {previous} and {position}"));
            }
            match self.index.get(&id) {
                Some(&index) if index == position => {}
                Some(&index) => {
                    return Err(format!("id {id} indexes {index}, not {position}"));
                }
                None => return Err(format!("id {id} missing from the index")),
            }
        }
        if self.index.len() != self.nodes.len() {
            return Err(format!(
                "index has {} entries for {} nodes",
                self.index.len(),
                self.nodes.len()
            ));
        }
        if self.order.len() != self.nodes.len() {
            return Err(format!(
                "order has {} entries for {} nodes",
                self.order.len(),
                self.nodes.len()
            ));
        }
        let mut ordered = vec![false; self.nodes.len()];
        for &index in &self.order {
            if index >= self.nodes.len() {
                return Err(format!("order references node {index} out of range"));
            }
            if ordered[index] {
                return Err(format!("node {index} appears twice in the order"));
            }
            ordered[index] = true;
        }
        for pair in self.order.windows(2) {
            match self.compare_positions(pair[0], pair[1]) {
                Ordering::Greater => {
                    return Err(format!(
                        "order not sorted at nodes {} and {}",
                        pair[0], pair[1]
                    ));
                }
                Ordering::Equal if pair[0] > pair[1] => {
                    return Err(format!(
                        "order is not stable for nodes {} and {}",
                        pair[0], pair[1]
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// The URI a node takes after a rename: the old URI's parent plus the new
/// name. Falls back to the old URI when it cannot be decomposed, so a foreign
/// or malformed URI is carried rather than dropped.
fn renamed_uri(old_uri: &str, new_name: &OsStr) -> String {
    Location::parse(old_uri)
        .ok()
        .and_then(|location| location.parent())
        .map(|parent| parent.child(new_name).uri().to_owned())
        .unwrap_or_else(|| old_uri.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockSource;
    use crate::{Node, NodeKind};
    use std::time::Duration;

    fn file(label: &str) -> Node {
        Node::new(label, format!("file:///fixture/{label}"), NodeKind::File)
    }

    fn collect(model: &mut DirectoryModel, handle: &ListingHandle) {
        while !model.is_complete() {
            let Some(event) = handle.recv_timeout(Duration::from_secs(5)) else {
                break;
            };
            model.apply(event);
        }
    }

    #[test]
    fn batches_append_in_arrival_order_with_stable_ids() {
        let source = Arc::new(MockSource::streaming(vec![
            vec![file("a"), file("b")],
            vec![file("c")],
        ]));
        let location = Location::file("/fixture");
        let mut model = DirectoryModel::new();
        let handle = model.begin(source, location);
        collect(&mut model, &handle);

        let names: Vec<String> = model
            .nodes()
            .iter()
            .map(|node| node.display_name().into_owned())
            .collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        assert!(model.is_complete());
        model.check_consistent().expect("consistent");
        let first = model.nodes()[0].id();
        assert_eq!(
            model.node(first).map(Node::display_name).as_deref(),
            Some("a")
        );
        assert_eq!(model.index_of(first), Some(0));
    }

    #[test]
    fn a_stale_generation_is_ignored() {
        let mut model = DirectoryModel::new();
        let source = Arc::new(MockSource::streaming(vec![vec![file("a")]]));
        let stale = model.begin(source.clone(), Location::file("/one"));
        let stale_generation = stale.generation();
        let _fresh = model.begin(source, Location::file("/two"));

        assert!(!model.apply(ListingEvent::batch(stale_generation, vec![file("z")])));
        assert!(model.is_empty());
        model.check_consistent().expect("consistent");
    }

    #[test]
    fn an_open_error_marks_the_listing_failed() {
        let source = Arc::new(MockSource::failing(SourceError::PermissionDenied(
            "/fixture".to_owned(),
        )));
        let mut model = DirectoryModel::new();
        let handle = model.begin(source, Location::file("/fixture"));
        let event = handle
            .recv_timeout(Duration::from_secs(5))
            .expect("error event");
        model.apply(event);
        assert!(matches!(
            model.error(),
            Some(SourceError::PermissionDenied(_))
        ));
        assert!(!model.is_streaming());
        assert!(model.is_empty());
    }

    fn name_of(node: &Node) -> String {
        node.display_name().into_owned()
    }

    fn ordered_names(model: &DirectoryModel) -> Vec<String> {
        model.ordered().map(name_of).collect()
    }

    #[test]
    fn batches_merge_into_a_sorted_projection() {
        let source = Arc::new(MockSource::streaming(vec![
            vec![file("c"), file("a")],
            vec![file("d"), file("b")],
        ]));
        let mut model = DirectoryModel::new();
        let handle = model.begin(source, Location::file("/fixture"));

        // After the first batch the projection is sorted on its own.
        let first = handle.recv_timeout(Duration::from_secs(5)).expect("batch");
        model.apply(first);
        assert_eq!(ordered_names(&model), vec!["a", "c"]);
        // Arrival order is preserved for the raw slice.
        let arrival: Vec<String> = model.nodes().iter().map(name_of).collect();
        assert_eq!(arrival, vec!["c", "a"]);
        model.check_consistent().expect("consistent mid-stream");

        let second = handle.recv_timeout(Duration::from_secs(5)).expect("batch");
        model.apply(second);
        assert_eq!(ordered_names(&model), vec!["a", "b", "c", "d"]);
        assert_eq!(model.ordered_node(0).map(name_of).as_deref(), Some("a"));
        let a_id = model.ordered_node(0).expect("first").id();
        assert_eq!(model.ordered_position_of(a_id), Some(0));
        assert_eq!(model.ordered_position_of(NodeId::from_raw(999)), None);
        model.check_consistent().expect("consistent at completion");
    }

    #[test]
    fn changing_the_sort_reroutes_existing_and_future_nodes() {
        use crate::sort::{SortKey, SortSpec};
        let source = Arc::new(MockSource::streaming(vec![
            vec![file("b"), file("a")],
            vec![file("d"), file("c")],
        ]));
        let mut model = DirectoryModel::new();
        let handle = model.begin(source, Location::file("/fixture"));
        let first = handle.recv_timeout(Duration::from_secs(5)).expect("batch");
        model.apply(first);
        let before: Vec<NodeId> = model.ordered().map(Node::id).collect();

        let descending = SortSpec::new(SortKey::Name)
            .with_direction(crate::sort::SortDirection::Descending)
            .with_folders_first(false);
        model.set_sort(descending);
        assert_eq!(ordered_names(&model), vec!["b", "a"]);
        let mut after: Vec<NodeId> = model.ordered().map(Node::id).collect();
        let mut before_sorted = before;
        before_sorted.sort();
        after.sort();
        assert_eq!(before_sorted, after, "sorting must not renumber nodes");
        model.check_consistent().expect("consistent after re-sort");

        let second = handle.recv_timeout(Duration::from_secs(5)).expect("batch");
        model.apply(second);
        assert_eq!(ordered_names(&model), vec!["d", "c", "b", "a"]);
        model.check_consistent().expect("consistent after merge");
    }

    #[test]
    fn equal_keys_keep_arrival_order() {
        use crate::sort::{SortDirection, SortKey, SortSpec};
        let source = Arc::new(MockSource::streaming(vec![
            vec![
                file("same").with_size(Some(7)),
                file("same").with_size(Some(7)),
            ],
            vec![
                file("same").with_size(Some(7)),
                file("same").with_size(Some(7)),
            ],
        ]));
        let mut model = DirectoryModel::new();
        let handle = model.begin(source, Location::file("/fixture"));
        collect(&mut model, &handle);
        model.set_sort(
            SortSpec::new(SortKey::Size)
                .with_direction(SortDirection::Ascending)
                .with_folders_first(false),
        );
        let ids: Vec<NodeId> = model.ordered().map(Node::id).collect();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        assert_eq!(ids, sorted_ids, "stable sort must preserve arrival order");
        model.check_consistent().expect("consistent");
    }
}
