// SPDX-License-Identifier: MIT
//! The streaming worker and its event stream ([09-files.md]).
//!
//! `DirectoryModel::begin` calls [`begin`] here. A named worker thread opens
//! the source and forwards batches over a channel; the consumer's event loop
//! pulls them non-blockingly with [`ListingHandle::try_recv`] and appends to
//! the model. No directory I/O ever runs on the consumer's thread.
//!
//! Each listing carries a monotonically increasing generation. A handle's
//! drop — or a new `begin` — sets a cancellation flag so a superseded worker
//! stops reading; `DirectoryModel::apply` independently rejects events whose
//! generation is not current, so a straggler batch can never pollute a new
//! listing.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::source::{DirectorySource, SourceError};
use crate::{Location, Node};

/// The default number of entries a worker forwards per event.
pub const DEFAULT_BATCH: usize = 256;

/// One message from a listing worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingEvent {
    /// The listing generation this event belongs to.
    pub generation: u64,
    /// What happened.
    pub kind: ListingEventKind,
}

/// The payload of a [`ListingEvent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingEventKind {
    /// A batch of entries, in arrival order.
    Batch(Vec<Node>),
    /// The directory was fully listed.
    Done,
    /// The listing failed; no further events follow.
    Error(SourceError),
}

impl ListingEvent {
    /// Construct a batch event.
    pub fn batch(generation: u64, nodes: Vec<Node>) -> Self {
        Self {
            generation,
            kind: ListingEventKind::Batch(nodes),
        }
    }

    /// Construct the end-of-listing event.
    pub fn done(generation: u64) -> Self {
        Self {
            generation,
            kind: ListingEventKind::Done,
        }
    }

    /// Construct a failure event.
    pub fn error(generation: u64, error: SourceError) -> Self {
        Self {
            generation,
            kind: ListingEventKind::Error(error),
        }
    }

    /// The generation this event belongs to.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// A live listing the consumer drains.
///
/// Dropping the handle cancels the worker.
pub struct ListingHandle {
    generation: u64,
    receiver: Receiver<ListingEvent>,
    cancel: Arc<AtomicBool>,
}

impl ListingHandle {
    /// The generation shared with the model that began this listing.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The cancellation flag, shared with the worker and the model.
    pub(crate) fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    /// Ask the worker to stop after its current batch.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// The next ready event, without blocking.
    pub fn try_recv(&self) -> Option<ListingEvent> {
        self.receiver.try_recv().ok()
    }

    /// The next event, waiting up to `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<ListingEvent> {
        self.receiver.recv_timeout(timeout).ok()
    }

    /// The next event, blocking until one arrives or the worker ends.
    pub fn recv(&self) -> Option<ListingEvent> {
        self.receiver.recv().ok()
    }
}

impl Drop for ListingHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

/// Spawn the worker for one listing and return its handle.
pub(crate) fn begin(
    source: Arc<dyn DirectorySource>,
    location: Location,
    generation: u64,
    batch: usize,
) -> ListingHandle {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = Arc::clone(&cancel);
    // Keep a sender to report a thread-spawn failure back to the consumer.
    let failure_sender = sender.clone();

    let spawned = thread::Builder::new()
        .name("files-core-listing".to_owned())
        .spawn(move || {
            let reader = match source.open(&location) {
                Ok(reader) => reader,
                Err(error) => {
                    let _ = sender.send(ListingEvent::error(generation, error));
                    return;
                }
            };
            if worker_cancel.load(Ordering::SeqCst) {
                return;
            }
            let mut reader = reader;
            loop {
                if worker_cancel.load(Ordering::SeqCst) {
                    break;
                }
                match reader.next_batch(batch) {
                    Ok(Some(nodes)) => {
                        if sender.send(ListingEvent::batch(generation, nodes)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = sender.send(ListingEvent::done(generation));
                        break;
                    }
                    Err(error) => {
                        let _ = sender.send(ListingEvent::error(generation, error));
                        break;
                    }
                }
            }
        });

    if let Err(error) = spawned {
        let _ = failure_sender.send(ListingEvent::error(
            generation,
            SourceError::Io(format!("could not start listing worker: {error}")),
        ));
    }

    ListingHandle {
        generation,
        receiver,
        cancel,
    }
}
