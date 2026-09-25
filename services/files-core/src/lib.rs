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
//!   consistent. A separate sorted projection is maintained incrementally as
//!   batches arrive ([`DirectoryModel::ordered`]).
//! * [`SortSpec`]/[`SortKey`]/[`SortDirection`] — the one collation: natural
//!   (`file2` before `file10`), folders-first, stable.
//! * [`DirectorySource`] / [`DirectoryReader`] — the platform seam. The
//!   listing runs on a worker thread, so no I/O ever touches the UI thread.
//! * [`StdFsSource`] — the **sanctioned fallback** used where GIO/GVfs is
//!   not available; it is explicitly marked for replacement
//!   ([adr/0042], [adr/0043]).
//!
//! # What T-10.2a owns
//!
//! * [`FileOps`] — the one operations engine seam: rename, new folder, move,
//!   copy, and permanent delete. [`StdFsOps`] is its sanctioned fallback,
//!   the same `std::fs` degradation as listing. [`generated_name`] is the one
//!   next-available-name helper (`untitled folder`, `untitled folder 2`, …).
//!   Optimistic semantics, undo, conflicts, and trash are later tasks.
//!
//! # What T-10.2b owns
//!
//! * [`OptimisticModel`] — a [`DirectoryModel`] plus [`Selection`] that
//!   applies rename / new-folder / delete to the model **synchronously** (so
//!   the next frame paints them) and keeps a pending record. Each edit is
//!   [`OptimisticModel::confirm`]ed when the real [`FileOps`] call succeeds or
//!   [`OptimisticModel::revert`]ed when it fails, restoring the captured node
//!   and selection. The `*_via` methods run both halves in one call.
//! * [`Selection`] — an insertion-ordered set of [`NodeId`]s. Ids are never
//!   renumbered by an optimistic edit, so the selection and the sort both
//!   survive rename, new folder, and re-sort; only a confirmed delete drops
//!   an id, and a revert puts it back.
//!
//! # Sorting is incremental and stable
//!
//! [`DirectoryModel::set_sort`] changes the order and re-sorts what is already
//! loaded without touching node ids or the arrival-order slice, so selection
//! survives. Each batch is merged into the sorted projection in O(n), so a
//! view can paint sorted from the first batch onward; equal keys keep arrival
//! order ([`SortSpec::compare`] returns [`std::cmp::Ordering::Equal`] for
//! genuinely equal keys).
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
//! # for node in model.ordered().take(3) { let _ = node.display_name(); }
//! ```
//!
//! # Platform backend
//!
//! GIO/GVfs is the intended backend (`GFileMonitor`, `trash://`, the UDisks
//! volume monitor — [09-files.md]). Its headers are not part of the pinned
//! toolchain here (`pkg-config --exists gio-2.0` fails), so T-10.1b keeps
//! [`StdFsSource`] as the shipping backend behind the unchanged
//! [`DirectorySource`] seam, explicitly marked for replacement. A GIO reader
//! slots in without a model change ([adr/0043]).
//!
//! [09-files.md]: ../../../docs/design/09-files.md
//! [adr/0042]: ../../../docs/design/adr/0042-files-core-streaming-listing-and-fallback.md
//! [adr/0043]: ../../../docs/design/adr/0043-files-core-fallback-is-the-shipping-backend.md

mod fallback;
mod listing;
mod location;
mod mock;
mod model;
mod node;
mod ops;
mod optimistic;
mod selection;
pub mod sort;
mod source;

pub use fallback::{StdFsSource, SANCTIONED_FALLBACK_MARKER};
pub use listing::{ListingEvent, ListingEventKind, ListingHandle, DEFAULT_BATCH};
pub use location::{Location, LocationError};
pub use mock::MockSource;
pub use model::{DirectoryModel, ListingState};
pub use node::{Node, NodeId, NodeKind};
pub use ops::{generated_name, FileOps, OperationError, StdFsOps, NEW_FOLDER_BASE};
pub use optimistic::{OpId, OptimisticModel};
pub use selection::Selection;
pub use sort::{SortDirection, SortKey, SortSpec};
pub use source::{DirectoryReader, DirectorySource, SourceError};
