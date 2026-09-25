// SPDX-License-Identifier: MIT
//! A deterministic in-memory [`DirectorySource`] for headless tests.
//!
//! `files-core` is unit- and property-tested with fake adapters and no display
//! ([09-files.md]); this is the listing fake. It yields caller-supplied
//! batches in order, so a test can assert the model preserves arrival order
//! and reaches the streaming state before the listing is done.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::source::{DirectoryReader, DirectorySource, SourceError};
use crate::{Location, Node};

/// A fake source that replays a scripted sequence of batches.
pub struct MockSource {
    open_error: Option<SourceError>,
    batches: Mutex<VecDeque<Vec<Node>>>,
}

impl MockSource {
    /// A source whose `open` succeeds and that then yields `batches` in order.
    pub fn streaming(batches: Vec<Vec<Node>>) -> Self {
        Self {
            open_error: None,
            batches: Mutex::new(batches.into()),
        }
    }

    /// A source whose `open` fails with `error`.
    pub fn failing(error: SourceError) -> Self {
        Self {
            open_error: Some(error),
            batches: Mutex::new(VecDeque::new()),
        }
    }
}

impl DirectorySource for MockSource {
    fn open(&self, _location: &Location) -> Result<Box<dyn DirectoryReader>, SourceError> {
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
