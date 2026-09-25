// SPDX-License-Identifier: MIT
//! The Trash source the Dock and Files share (T-10.6a, [09-files.md]).
//!
//! [09-files.md] makes `trash://` the one Trash source: the Dock badge, Files'
//! Trash location, and the eventual portal all read the same store, so a
//! deletion by any application appears everywhere at once. The shell cannot
//! call GIO/GVfs from C++ here (the headers are not pinned), so the source
//! lives in `files-core` beside [`crate::FreedesktopTrash`] and the shell reads
//! it through the C ABI ([`crate::ffi`]).
//!
//! Two surfaces sit on the same [`FreedesktopTrash`] store:
//!
//! * [`TrashSource`] is a [`DirectorySource`] over the `trash://` scheme, so
//!   the Files window lists the Trash through the exact same streaming model as
//!   any other location. Items are named by their stored name and addressed as
//!   `trash:///<name>`.
//! * [`TrashMonitor`] is the Dock's read path: a live count, a reachability
//!   flag, and change notifications driven by the T-10.3b [`FolderWatcher`]
//!   rather than a directory poll. An idle Trash does zero I/O.
//!
//! Both consume, never re-implement, the freedesktop Trash spec: the store is
//! [`FreedesktopTrash`], the sanctioned fallback on this host
//! ([adr/0046](../../../docs/design/adr/0046-files-core-trash-seam-and-spec-fallback.md)).
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::fallback::node_for_path;
use crate::source::{DirectoryReader, DirectorySource, SourceError};
use crate::watch::{FolderWatcher, WatchHandle};
#[cfg(target_os = "linux")]
use crate::InotifyWatcher;
use crate::{FreedesktopTrash, Location, Node, OperationError, TrashOps, TrashedItem};

/// The Dock's Trash reading: how many items the store holds and whether it is
/// reachable. `count` counts the home trash's `.trashinfo` records (the one
/// store the Dock badge watches); `available` is false when the store cannot
/// be read or created, so the Dock dims the entry instead of blocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrashState {
    /// Items currently in the home trash store.
    pub count: usize,
    /// Whether the store is reachable.
    pub available: bool,
}

impl Default for TrashState {
    fn default() -> Self {
        Self {
            count: 0,
            available: true,
        }
    }
}

/// A [`DirectorySource`] over `trash://`.
///
/// Opening `trash://` enumerates the home trash store; each item becomes a
/// [`Node`] whose URI is `trash:///<stored name>` and whose kind/metadata come
/// from the payload under `files/`. The listing is a fresh snapshot per open —
/// the store is small and the Trash is re-opened on navigation.
#[derive(Debug, Clone)]
pub struct TrashSource {
    store: FreedesktopTrash,
}

impl Default for TrashSource {
    fn default() -> Self {
        Self::new()
    }
}

impl TrashSource {
    /// A source over the environment's home trash.
    pub fn new() -> Self {
        Self {
            store: FreedesktopTrash::new(),
        }
    }

    /// A source over an explicit home trash directory (tests point this at a
    /// temp dir so enumeration never touches the real user trash).
    pub fn with_home_trash(home: impl Into<PathBuf>) -> Self {
        Self {
            store: FreedesktopTrash::with_home_trash(home),
        }
    }
}

impl DirectorySource for TrashSource {
    fn open(&self, location: &Location) -> Result<Box<dyn DirectoryReader>, SourceError> {
        if location.scheme() != "trash" {
            return Err(SourceError::UnsupportedScheme(location.scheme().to_owned()));
        }
        let items = self
            .store
            .entries()
            .map_err(|error| SourceError::Io(error.to_string()))?;
        let root = Location::parse("trash:///").expect("static trash root is valid");
        let nodes: Vec<Node> = items
            .iter()
            .map(|item| node_for_path(&root, item.name().to_os_string(), item.file_path()))
            .collect();
        Ok(Box::new(TrashReader { nodes, cursor: 0 }))
    }
}

struct TrashReader {
    nodes: Vec<Node>,
    cursor: usize,
}

impl DirectoryReader for TrashReader {
    fn next_batch(&mut self, max: usize) -> Result<Option<Vec<Node>>, SourceError> {
        if self.cursor >= self.nodes.len() {
            return Ok(None);
        }
        let end = (self.cursor + max.max(1)).min(self.nodes.len());
        let batch = self.nodes[self.cursor..end].to_vec();
        self.cursor = end;
        Ok(Some(batch))
    }
}

/// The Dock's live Trash state ([09-files.md], section "Trash").
///
/// `TrashMonitor` wraps the one store and a [`FolderWatcher`] over its `info/`
/// directory: any `.trashinfo` created, removed, or rewritten refreshes the
/// count, so a deletion by a third-party application moves the Dock badge
/// without polling. The store directories are created on [`start`](Self::start)
/// (the spec expects `files/` and `info/` to exist), so a fresh account reads
/// as an available, empty Trash rather than an error.
pub struct TrashMonitor {
    store: FreedesktopTrash,
    watcher: Option<Arc<dyn FolderWatcher>>,
    handle: Mutex<Option<WatchHandle>>,
    state: Mutex<TrashState>,
}

impl Default for TrashMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl TrashMonitor {
    /// A monitor over the environment's home trash, watching with the platform
    /// fallback ([`InotifyWatcher`] on Linux).
    pub fn new() -> Self {
        #[cfg(target_os = "linux")]
        let watcher: Option<Arc<dyn FolderWatcher>> = Some(Arc::new(InotifyWatcher::new()));
        #[cfg(not(target_os = "linux"))]
        let watcher: Option<Arc<dyn FolderWatcher>> = None;
        Self::with_backend(FreedesktopTrash::new(), watcher)
    }

    /// A monitor over an explicit store and watcher. Tests point the store at a
    /// temp dir; a `None` watcher is a monitor with no change notifications.
    pub fn with_backend(store: FreedesktopTrash, watcher: Option<Arc<dyn FolderWatcher>>) -> Self {
        Self {
            store,
            watcher,
            handle: Mutex::new(None),
            state: Mutex::new(TrashState::default()),
        }
    }

    /// A monitor over an explicit home trash directory, using the platform
    /// fallback watcher where available.
    pub fn with_home_trash(home: impl Into<PathBuf>) -> Self {
        let store = FreedesktopTrash::with_home_trash(home);
        #[cfg(target_os = "linux")]
        let watcher: Option<Arc<dyn FolderWatcher>> = Some(Arc::new(InotifyWatcher::new()));
        #[cfg(not(target_os = "linux"))]
        let watcher: Option<Arc<dyn FolderWatcher>> = None;
        Self::with_backend(store, watcher)
    }

    /// The home trash directory this monitor reads.
    pub fn home_trash(&self) -> &Path {
        self.store.home_trash()
    }

    /// Create the store and begin watching it. A store whose directories cannot
    /// be created reads as unavailable (a mount/permission failure) and never
    /// blocks the session.
    pub fn start(&self) {
        if !self.ensure_dirs() {
            *self.lock_state() = TrashState {
                count: 0,
                available: false,
            };
            return;
        }
        self.refresh();
        if let Some(watcher) = &self.watcher {
            let info = self.store.home_trash().join("info");
            if let Ok(handle) = crate::watch::begin(Arc::clone(watcher), &Location::file(&info), 0)
            {
                *self.handle.lock().unwrap() = Some(handle);
            }
        }
    }

    /// The last reading.
    pub fn state(&self) -> TrashState {
        *self.lock_state()
    }

    /// Re-scan now. Returns whether the reading changed; emits nothing (the C
    /// ABI caller compares or just re-reads).
    pub fn refresh(&self) -> bool {
        let (count, available) = self.scan();
        let mut state = self.lock_state();
        let changed = state.count != count || state.available != available;
        *state = TrashState { count, available };
        changed
    }

    /// Block up to `timeout` for a change to the store; refresh and return
    /// whether one arrived. A monitor with no watcher always times out.
    pub fn wait(&self, timeout: Duration) -> bool {
        let event = {
            let handle = self.handle.lock().unwrap();
            handle
                .as_ref()
                .and_then(|handle| handle.recv_timeout(timeout))
        };
        match event {
            Some(_) => {
                self.refresh();
                true
            }
            None => false,
        }
    }

    /// Move `target` into the store (the Dock's drop action, T-10.6b routes it
    /// here) and refresh.
    pub fn trash(&self, target: &Location) -> Result<TrashedItem, OperationError> {
        let item = self.store.trash(target)?;
        self.refresh();
        Ok(item)
    }

    /// Remove every item from the store (the Dock's Empty Trash action) and
    /// refresh.
    pub fn empty(&self) -> Result<usize, OperationError> {
        let removed = self.store.empty()?;
        self.refresh();
        Ok(removed)
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, TrashState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn ensure_dirs(&self) -> bool {
        let home = self.store.home_trash();
        if home.as_os_str().is_empty() || home == Path::new("/") {
            return false;
        }
        std::fs::create_dir_all(home.join("files")).is_ok()
            && std::fs::create_dir_all(home.join("info")).is_ok()
    }

    fn scan(&self) -> (usize, bool) {
        let home = self.store.home_trash();
        if home.as_os_str().is_empty() || home == Path::new("/") {
            return (0, false);
        }
        if !self.ensure_dirs() {
            return (0, false);
        }
        match self.store.entries() {
            Ok(items) => (items.len(), true),
            Err(_) => (0, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trash_info(original: &str) -> String {
        format!("[Trash Info]\nPath={original}\nDeletionDate=2026-01-02T03:04:05\n")
    }

    #[test]
    fn trash_source_lists_the_store_as_nodes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("Trash");
        std::fs::create_dir_all(home.join("files")).expect("files dir");
        std::fs::create_dir_all(home.join("info")).expect("info dir");
        std::fs::write(home.join("files/report.txt"), b"hello").expect("payload");
        std::fs::write(
            home.join("info/report.txt.trashinfo"),
            trash_info("/home/u/report.txt").as_bytes(),
        )
        .expect("info");

        let source = TrashSource::with_home_trash(&home);
        let root = Location::parse("trash:///").expect("root");
        let mut reader = source.open(&root).expect("open");
        let batch = reader.next_batch(64).expect("batch").expect("some");
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].display_name(), "report.txt");
        assert_eq!(batch[0].uri(), "trash:///report.txt");
        assert_eq!(batch[0].kind(), crate::NodeKind::File);
        assert_eq!(batch[0].size(), Some(5));
        assert!(reader.next_batch(64).expect("done").is_none());
    }

    #[test]
    fn trash_source_rejects_a_foreign_scheme() {
        let source = TrashSource::new();
        let error = source
            .open(&Location::file("/tmp"))
            .err()
            .expect("unsupported");
        assert_eq!(error, SourceError::UnsupportedScheme("file".to_owned()));
    }

    #[test]
    fn trash_monitor_reports_third_party_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("Trash");
        let monitor = TrashMonitor::with_home_trash(&home);
        monitor.start();
        assert_eq!(
            monitor.state(),
            TrashState {
                count: 0,
                available: true
            }
        );

        // A deletion by another application appears as a new info record plus
        // its payload.
        std::fs::create_dir_all(home.join("files")).expect("files");
        std::fs::write(home.join("files/third"), b"x").expect("payload");
        std::fs::write(
            home.join("info/third.trashinfo"),
            trash_info("/home/u/third").as_bytes(),
        )
        .expect("info");
        assert!(monitor.refresh(), "the reading changed");
        assert_eq!(monitor.state().count, 1);

        std::fs::remove_file(home.join("info/third.trashinfo")).expect("remove info");
        std::fs::remove_file(home.join("files/third")).expect("remove payload");
        assert!(monitor.refresh());
        assert_eq!(monitor.state().count, 0);
    }

    #[test]
    fn trash_monitor_watches_and_wakes_on_a_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("Trash");
        let monitor = Arc::new(TrashMonitor::with_home_trash(&home));
        monitor.start();

        let writer = {
            let home = home.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(150));
                std::fs::write(
                    home.join("info/async.trashinfo"),
                    trash_info("/home/u/async").as_bytes(),
                )
                .expect("info");
            })
        };
        assert!(
            monitor.wait(Duration::from_secs(5)),
            "the watcher woke on the third-party write"
        );
        assert_eq!(monitor.state().count, 1);
        writer.join().expect("writer");
    }

    #[test]
    fn trash_monitor_trashes_and_empties() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("Trash");
        let source = dir.path().join("note.txt");
        std::fs::write(&source, b"hello").expect("source");

        let monitor = TrashMonitor::with_home_trash(&home);
        monitor.start();
        assert_eq!(monitor.state().count, 0);

        monitor
            .trash(&Location::file(&source))
            .expect("trash the note");
        assert_eq!(monitor.state().count, 1);
        assert!(!source.exists());

        assert_eq!(monitor.empty().expect("empty"), 1);
        assert_eq!(monitor.state().count, 0);
    }

    #[test]
    fn a_fresh_account_reads_as_an_available_empty_trash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("Trash");
        let monitor = TrashMonitor::with_home_trash(&home);
        monitor.start();
        assert!(monitor.state().available);
        assert_eq!(monitor.state().count, 0);
        assert!(home.join("info").is_dir(), "the store is created on demand");
    }

    #[test]
    fn an_unsafe_root_reads_as_unavailable() {
        let monitor = TrashMonitor::with_home_trash("/");
        monitor.start();
        assert!(!monitor.state().available);
    }
}
