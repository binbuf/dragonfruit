// SPDX-License-Identifier: MIT
//! `files-core` — the headless browsing model and streaming directory
//! listing (T-10.1a).
//!
//! Files is the Finder-equivalent first-party app, but the browsing core is
//! deliberately **not** a process: it is a Rust library embedded by the Files
//! window (T-10.4), the future desktop surface, and the portal FileChooser
//! ([09-files.md]). One implementation serves all three so collation,
//! selection, and trash semantics cannot drift.
//!
//! # What T-10.1a owns
//!
//! * [`Location`] — a URI-addressed, GFile-shaped directory (`file://`,
//!   later `trash://`/`recent://`/`smb://`) with raw-byte-safe encoding.
//! * [`Node`] — one listed item with a stable per-session [`NodeId`], the
//!   original name bytes, kind, size, modification time, and symlink target.
//! * [`DirectoryModel`] — the accumulated listing. Entries are appended in
//!   arrival order; every node gets one id and the id→node index stays
//!   consistent.
//! * [`DirectorySource`] / [`DirectoryReader`] — the platform seam. The
//!   listing runs on a worker thread, so no I/O ever touches the UI thread.
//! * [`StdFsSource`] — the **sanctioned fallback** used where GIO/GVfs is
//!   not available; it is explicitly marked for replacement
//!   ([adr/0042]).
//!
//! # Streaming, not blocking
//!
//! `DirectoryModel::begin` returns a [`ListingHandle`]. A worker thread opens
//! the source and pushes [`ListingEvent`]s in batches; the consumer drains
//! them from its own event loop and [`DirectoryModel::apply`] appends each
//! batch. A view paints from the first batch onward. Starting a new location
//! cancels the previous worker, and events tagged with a stale generation are
//! ignored, so navigation never glitches on a slow directory.
//!
//! ```no_run
//! use std::sync::Arc;
//! use dragonfruit_files_core::{DirectoryModel, Location, StdFsSource};
//!
//! let mut model = DirectoryModel::new();
//! let handle = model.begin(Arc::new(StdFsSource::new()), Location::file("/tmp"));
//! while !model.is_complete() {
//!     if let Some(event) = handle.recv() {
//!         model.apply(event);
//!     }
//! }
//! # let _ = model.nodes();
//! ```
//!
//! # Platform backend
//!
//! GIO/GVfs is the intended backend (`GFileMonitor`, `trash://`, the UDisks
//! volume monitor — [09-files.md]). Its headers are not part of the pinned
//! toolchain here, so T-10.1a exercises the [`StdFsSource`] fallback through
//! the same seam a GIO reader will implement; T-10.1b finalizes the
//! GIO-vs-fallback decision and the sort order on top of this model.
//!
//! [09-files.md]: ../../../docs/design/09-files.md
//! [adr/0042]: ../../../docs/design/adr/0042-files-core-streaming-listing-and-fallback.md

mod fallback;
mod listing;
mod location;
mod mock;
mod model;
mod node;
mod source;

pub use fallback::{StdFsSource, SANCTIONED_FALLBACK_MARKER};
pub use listing::{ListingEvent, ListingEventKind, ListingHandle, DEFAULT_BATCH};
pub use location::{Location, LocationError};
pub use mock::MockSource;
pub use model::{DirectoryModel, ListingState};
pub use node::{Node, NodeId, NodeKind};
pub use source::{DirectoryReader, DirectorySource, SourceError};
