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
//! [09-files.md]: ../../../docs/design/09-files.md

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::listing::{self, ListingEvent, ListingEventKind, ListingHandle, DEFAULT_BATCH};
use crate::source::{DirectorySource, SourceError};
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
            cancel.store(true, Ordering::SeqCst);
        }
        self.generation += 1;
        self.location = Some(location.clone());
        self.nodes.clear();
        self.index.clear();
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
                for mut node in nodes {
                    self.next_id += 1;
                    let id = NodeId::from_raw(self.next_id);
                    node.set_id(id);
                    self.index.insert(id, self.nodes.len());
                    self.nodes.push(node);
                }
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
        Ok(())
    }
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
}
