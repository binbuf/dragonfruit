// SPDX-License-Identifier: MIT
//! A deterministic in-memory [`DirectorySource`] for headless tests.
//!
//! `files-core` is unit- and property-tested with fake adapters and no display
//! ([09-files.md]); this is the listing fake. It yields caller-supplied
//! batches in order, so a test can assert the model preserves arrival order
//! and reaches the streaming state before the listing is done.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Mutex;
use std::time::Duration;

use crate::source::{DirectoryReader, DirectorySource, SourceError};
use crate::watch::{FolderWatcher, WatchEventKind, WatchReader};
use crate::{Location, Node};

/// A fake source that replays a scripted sequence of batches.
pub struct MockSource {
    open_error: Option<SourceError>,
    batches: Mutex<VecDeque<Vec<Node>>>,
    opens: AtomicUsize,
}

impl MockSource {
    /// A source whose `open` succeeds and that then yields `batches` in order.
    pub fn streaming(batches: Vec<Vec<Node>>) -> Self {
        Self {
            open_error: None,
            batches: Mutex::new(batches.into()),
            opens: AtomicUsize::new(0),
        }
    }

    /// A source whose `open` fails with `error`.
    pub fn failing(error: SourceError) -> Self {
        Self {
            open_error: Some(error),
            batches: Mutex::new(VecDeque::new()),
            opens: AtomicUsize::new(0),
        }
    }

    /// How many times `open` has been called, so a test can prove that folding
    /// a watcher event did not restart the listing.
    pub fn opens(&self) -> usize {
        self.opens.load(AtomicOrdering::SeqCst)
    }
}

impl DirectorySource for MockSource {
    fn open(&self, _location: &Location) -> Result<Box<dyn DirectoryReader>, SourceError> {
        self.opens.fetch_add(1, AtomicOrdering::SeqCst);
        if let Some(error) = self.open_error.clone() {
            return Err(error);
        }
        let mut guard = self.batches.lock().expect("mock source poisoned");
        let batches = guard.drain(..).collect();
        Ok(Box::new(MockReader { batches }))
    }
}

struct MockReader {
    batches: VecDeque<Vec<Node>>,
}

impl DirectoryReader for MockReader {
    fn next_batch(&mut self, _max: usize) -> Result<Option<Vec<Node>>, SourceError> {
        Ok(self.batches.pop_front())
    }
}

/// A deterministic in-memory [`FolderWatcher`] for headless tests.
///
/// It replays caller-supplied batches of [`WatchEventKind`]s and then blocks
/// (returning empty batches) until the worker is cancelled, exactly as an
/// event-driven backend does. This is the "watcher fixture" of the T-10.3b test
/// plan: it lets a test assert that folding an external create/delete into the
/// model mutates it incrementally, with no re-listing.
pub struct MockWatcher {
    open_error: Option<SourceError>,
    batches: Mutex<VecDeque<Vec<WatchEventKind>>>,
}

impl MockWatcher {
    /// A watcher whose `watch` succeeds and then yields `batches` in order.
    pub fn scripted(batches: Vec<Vec<WatchEventKind>>) -> Self {
        Self {
            open_error: None,
            batches: Mutex::new(batches.into()),
        }
    }

    /// A watcher whose `watch` fails with `error`.
    pub fn failing(error: SourceError) -> Self {
        Self {
            open_error: Some(error),
            batches: Mutex::new(VecDeque::new()),
        }
    }
}

impl FolderWatcher for MockWatcher {
    fn watch(&self, _location: &Location) -> Result<Box<dyn WatchReader>, SourceError> {
        if let Some(error) = self.open_error.clone() {
            return Err(error);
        }
        let mut guard = self.batches.lock().expect("mock watcher poisoned");
        let batches = guard.drain(..).collect();
        Ok(Box::new(MockWatchReader { batches }))
    }
}

struct MockWatchReader {
    batches: VecDeque<Vec<WatchEventKind>>,
}

impl WatchReader for MockWatchReader {
    fn next_batch(&mut self, timeout: Duration) -> Result<Vec<WatchEventKind>, SourceError> {
        match self.batches.pop_front() {
            Some(kinds) => Ok(kinds),
            None => {
                // Mimic a blocking backend so the worker keeps checking its
                // cancellation flag between empty batches.
                std::thread::sleep(timeout);
                Ok(Vec::new())
            }
        }
    }
}
