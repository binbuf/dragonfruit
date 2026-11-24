// SPDX-License-Identifier: MIT
//! The folder watcher: change events folded into the model (T-10.3b,
//! [09-files.md]).
//!
//! The design's hard rule is **no I/O on the UI thread, and optimistic
//! rendering reconciled by monitors**; one change monitor runs per visible
//! directory, fanned out to views, and an idle window does zero polling. This
//! module is that monitor's headless half, and the sibling of the streaming
//! listing in [`crate::listing`].
//!
//! A backend implements [`FolderWatcher`], whose [`FolderWatcher::watch`]
//! returns a [`WatchReader`] that blocks for the next change. [`begin`] opens
//! the reader on the caller's thread (so an unsupported scheme or a missing
//! folder is reported immediately) and then moves it to a named worker that
//! forwards [`WatchEvent`]s over an `mpsc` channel. The consumer drains them
//! from its own event loop with [`crate::DirectoryModel::drain_watch`], so a
//! re-listed frame never touches the watcher and the watcher never touches the
//! UI thread.
//!
//! A change is described incrementally — one [`WatchEventKind`], never a full
//! re-listing. [`crate::DirectoryModel::apply_watch`] folds it into the model,
//! keeping node ids stable across a rename, so a view's selection does not
//! glitch.
//!
//! # The shipping backend is a marked fallback
//!
//! GIO/GVfs's `GFileMonitor` is the intended backend ([09-files.md]); its
//! headers are not part of the pinned toolchain (`pkg-config --exists gio-2.0`
//! fails), the same degradation listing and trash record
//! ([adr/0043](../../../docs/design/adr/0043-files-core-fallback-is-the-shipping-backend.md)).
//! [`InotifyWatcher`] speaks inotify — the local mechanism `GFileMonitor`
//! wraps — directly over `libc`, and is **marked for replacement, not silently
//! shipped**: [`SANCTIONED_WATCHER_FALLBACK_MARKER`] names GIO/GVfs and the
//! seam. A `FolderWatcher` over GIO slots in without a model or view change.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::source::SourceError;
use crate::{Location, Node};

/// How long the worker blocks for a change before re-checking cancellation.
/// The backend is event-driven; this is a cancellation poll, not a directory
/// poll, and costs no I/O when the folder is idle.
pub const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// How long the worker keeps absorbing further kernel batches into one
/// [`WatchBatch`] once a change has arrived (T332). A burst — an archive
/// extraction, a `cp -r`, a "save with three `mv`s" — collapses into one
/// model update instead of one UI repaint per file. The backend stays
/// event-driven: this only bounds how long one delivered group waits for its
/// stragglers, so an idle folder still does zero I/O.
pub const WATCH_COALESCE_WINDOW: Duration = Duration::from_millis(50);

/// Marker documenting that [`InotifyWatcher`] is the sanctioned fallback and
/// that GIO/GVfs has not replaced it. It is deliberately worded so a build
/// that ships without the real backend is still visibly degraded: a
/// `GFileMonitor`-backed watcher lands behind the `FolderWatcher` seam and
/// removes this marker.
pub const SANCTIONED_WATCHER_FALLBACK_MARKER: &str =
    "sanctioned inotify fallback for files-core change monitoring; GIO/GVfs GFileMonitor not linked — replace behind the FolderWatcher seam when GIO is pinned (ADR 0047)";

/// What changed in a watched folder.
///
/// Every variant is incremental: a `Created`/`Modified`/`Renamed` carries the
/// freshly stat'd [`Node`] (the watcher reads that one entry, never the whole
/// directory), and a `Removed` names the vanished entry by URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventKind {
    /// An entry appeared.
    Created(Node),
    /// An entry's metadata changed (write, chmod, close-write).
    Modified(Node),
    /// An entry disappeared (deleted or moved out of the folder).
    Removed {
        /// The URI of the entry that is gone.
        uri: String,
    },
    /// An entry was renamed or moved within the folder. The node carries its
    /// new name and URI; the model keeps the original id so selection and
    /// drag state survive.
    Renamed {
        /// The URI of the entry under its previous name.
        from_uri: String,
        /// The entry under its new name.
        to: Node,
    },
    /// The watched folder itself was replaced (an in-place rename gives it a
    /// new inode) or came back under its name: the listing at the same
    /// location is a different tree, so the model drops its rows and asks the
    /// receiver for a reset re-list (T333).
    Rescan,
}

/// One coalesced group of changes from a watcher worker (T332).
///
/// The worker forwards one batch per burst; its [`kinds`](Self::kinds) are in
/// kernel order and carry the real paths (renames already paired), so the
/// consumer folds the whole burst through
/// [`DirectoryModel::apply_watch_batch`] — one `dataChanged` for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchBatch {
    /// The model generation the batch belongs to.
    pub generation: u64,
    /// The changes, in the order the kernel reported them.
    pub kinds: Vec<WatchEventKind>,
}

impl WatchBatch {
    /// Construct a batch for `generation`.
    pub fn new(generation: u64, kinds: Vec<WatchEventKind>) -> Self {
        Self { generation, kinds }
    }

    /// The generation this batch belongs to.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether the batch carries no change.
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

/// One event from a watcher worker, tagged with the generation it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEvent {
    /// The model generation this event belongs to.
    pub generation: u64,
    /// What changed.
    pub kind: WatchEventKind,
}

impl WatchEvent {
    /// Construct an event for `generation`.
    pub fn new(generation: u64, kind: WatchEventKind) -> Self {
        Self { generation, kind }
    }

    /// The generation this event belongs to.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// Opens a folder for change monitoring.
///
/// Implementations are held by the watcher worker, so they must be `Send +
/// Sync + 'static`. Opening happens on the caller's thread; all watching
/// happens on the worker.
pub trait FolderWatcher: Send + Sync + 'static {
    /// Begin watching `location`, returning a reader whose
    /// [`WatchReader::next_batch`] blocks for changes.
    ///
    /// `entries_only` tells the backend that the location is itself a folder
    /// whose *listing* is watched (the Files window), so a directory entry is
    /// reported as [`NodeKind::Directory`](crate::NodeKind::Directory) even
    /// while it is empty (inotify's `IN_ISDIR`). A monitor of one subdirectory
    /// inside a listing (the Dock's Trash `info/`) passes `false` and gets the
    /// historical behaviour. Backends without an equivalent hint may ignore it.
    fn watch(&self, location: &Location) -> Result<Box<dyn WatchReader>, SourceError> {
        self.watch_entries(location, false)
    }

    /// Begin watching `location`, as [`Self::watch`], with the listing hint.
    fn watch_entries(
        &self,
        location: &Location,
        entries_only: bool,
    ) -> Result<Box<dyn WatchReader>, SourceError> {
        let _ = entries_only;
        self.watch(location)
    }
}

/// A blocking cursor over one folder's change events.
pub trait WatchReader: Send {
    /// Block for up to `timeout` for the next group of changes.
    ///
    /// Returns an empty vector when the timeout elapses with no change, so the
    /// worker can re-check cancellation. An `Err` is terminal for the watch.
    fn next_batch(&mut self, timeout: Duration) -> Result<Vec<WatchEventKind>, SourceError>;
}

/// A live watch the consumer drains.
///
/// Dropping the handle cancels the worker at the next poll.
pub struct WatchHandle {
    generation: u64,
    receiver: Receiver<WatchBatch>,
    /// A batch that [`WatchHandle::wait_ready`] took and returned, so a peek
    /// is not a consume (T332).
    front: std::sync::Mutex<Vec<WatchBatch>>,
    /// A flattened, generation-tagged event stream (T332). The per-event
    /// surface is kept for callers that handle one change at a time; the
    /// batch surface is the one that coalesces.
    events: Receiver<WatchEvent>,
    cancel: Arc<AtomicBool>,
}

impl WatchHandle {
    /// The generation shared with the model that began this watch.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The cancellation flag, shared with the worker and the model.
    pub(crate) fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    /// Block until a batch is ready or `timeout` elapses, **without taking
    /// it**. Returns `true` when a batch is queued (the caller then
    /// [`Self::try_recv_batch`]es); returns `false` if a batch was already
    /// taken meanwhile by another probe. Used by the C ABI's dormant poll, so
    /// a settled window sleeps on a channel rather than a timer.
    pub(crate) fn wait_ready(&self, timeout: Duration) -> bool {
        match self.receiver.recv_timeout(timeout) {
            Ok(batch) => {
                // Put it back so every consumer sees the same queue.
                self.put_back(batch);
                true
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => false,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => false,
        }
    }

    /// Return a batch to the front of the queue. `mpsc` has no push-front, so
    /// the worker's own forwarding cannot interleave here: the C ABI is the
    /// only caller and it holds the session exclusively.
    fn put_back(&self, batch: WatchBatch) {
        self.front
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(batch);
    }

    /// The next batch: a batch returned by [`Self::wait_ready`] first, then
    /// the worker's queue.
    fn next_batch(&self) -> Option<WatchBatch> {
        let returned = self.front.lock().unwrap_or_else(|p| p.into_inner()).pop();
        returned.or_else(|| self.receiver.try_recv().ok())
    }

    /// Ask the worker to stop at the next poll.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// The next ready event, without blocking.
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.events.try_recv().ok()
    }

    /// The next event, waiting up to `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<WatchEvent> {
        self.events.recv_timeout(timeout).ok()
    }

    /// The next event, blocking until one arrives or the worker ends.
    pub fn recv(&self) -> Option<WatchEvent> {
        self.events.recv().ok()
    }

    /// The next coalesced batch, without blocking (T332).
    pub fn try_recv_batch(&self) -> Option<WatchBatch> {
        self.next_batch()
    }

    /// The next coalesced batch, waiting up to `timeout`.
    pub fn recv_batch_timeout(&self, timeout: Duration) -> Option<WatchBatch> {
        if let Some(batch) = self.try_recv_batch() {
            return Some(batch);
        }
        self.receiver.recv_timeout(timeout).ok()
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

/// Open `location` with `watcher` and spawn its forwarding worker.
///
/// Opening runs here, synchronously, so a foreign scheme or a missing folder
/// is an immediate `Err` rather than a silent background failure.
///
/// The worker coalesces: it blocks for a change, then keeps collecting the
/// backend's next batches for [`WATCH_COALESCE_WINDOW`] so one burst reaches
/// the consumer as one [`WatchBatch`].
pub(crate) fn begin(
    watcher: Arc<dyn FolderWatcher>,
    location: &Location,
    generation: u64,
) -> Result<WatchHandle, SourceError> {
    begin_with_hint(watcher, location, generation, false)
}

/// [`begin`], stating whether `location` is itself a listing (T332).
pub(crate) fn begin_entries(
    watcher: Arc<dyn FolderWatcher>,
    location: &Location,
    generation: u64,
) -> Result<WatchHandle, SourceError> {
    begin_with_hint(watcher, location, generation, true)
}

/// [`begin_entries`], also handing back the coalesced batch receiver: the FFI
/// session wakes on the batch channel (T332), while the handle's per-event
/// channel serves model users that fold events one at a time.
pub(crate) fn begin_entries_split(
    watcher: Arc<dyn FolderWatcher>,
    location: &Location,
    generation: u64,
) -> Result<(WatchHandle, Receiver<WatchBatch>), SourceError> {
    begin_with_hint_split(watcher, location, generation, true)
}

fn begin_with_hint(
    watcher: Arc<dyn FolderWatcher>,
    location: &Location,
    generation: u64,
    entries_only: bool,
) -> Result<WatchHandle, SourceError> {
    begin_with_hint_split(watcher, location, generation, entries_only)
        .map(|(handle, _batches)| handle)
}

/// [`begin_with_hint`], also returning the batch receiver the worker fills.
fn begin_with_hint_split(
    watcher: Arc<dyn FolderWatcher>,
    location: &Location,
    generation: u64,
    entries_only: bool,
) -> Result<(WatchHandle, Receiver<WatchBatch>), SourceError> {
    let mut reader = watcher.watch_entries(location, entries_only)?;
    // Two batch consumers, each receiving every batch: the handle's own
    // receiver (model users) and the caller's (the FFI session that wakes on
    // batches). The worker stops only when both are gone.
    let (batch_sender, handle_batches) = mpsc::channel();
    let (handle_sender, batches) = mpsc::channel();
    let (event_sender, events) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = Arc::clone(&cancel);

    let spawned = thread::Builder::new()
        .name("files-core-watcher".to_owned())
        .spawn(move || loop {
            if worker_cancel.load(Ordering::SeqCst) {
                break;
            }
            // Never `continue` out of this arm with events pending: a backend
            // that re-arms its watch (T333) reports that as its own event
            // alongside the changes, so an empty batch is not a reason to skip
            // forwarding.
            match reader.next_batch(WATCH_POLL_INTERVAL) {
                Ok(kinds) if kinds.is_empty() => continue,
                Ok(mut kinds) => {
                    // Absorb the rest of the burst. A backend that keeps
                    // reporting changes back to back (an extraction) is
                    // drained until a quiet window, so the consumer sees one
                    // batch; an idle folder returns nothing here.
                    loop {
                        match reader.next_batch(WATCH_COALESCE_WINDOW) {
                            Ok(more) if more.is_empty() => break,
                            Ok(more) => kinds.extend(more),
                            Err(_) => break,
                        }
                    }
                    for kind in &kinds {
                        if event_sender
                            .send(WatchEvent::new(generation, kind.clone()))
                            .is_err()
                        {
                            return;
                        }
                    }
                    let batch = WatchBatch::new(generation, kinds);
                    // The handle's receiver is the model user's; the caller's
                    // channel (if any) is the FFI session's wake-up and must
                    // not keep a dropped handle's worker alive.
                    let _ = handle_sender.send(batch.clone());
                    if batch_sender.send(batch).is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        });

    if let Err(error) = spawned {
        return Err(SourceError::Io(format!(
            "could not start watcher worker: {error}"
        )));
    }

    Ok((
        WatchHandle {
            generation,
            receiver: handle_batches,
            front: std::sync::Mutex::new(Vec::new()),
            events,
            cancel,
        },
        batches,
    ))
}

/// The kind of change a name-and-mask pair describes, before it is turned into
/// a [`WatchEventKind`] with a stat'd node. Exposed so the inotify backend's
/// decision table can be unit-tested without a kernel watch.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InotifyChange {
    Created,
    Modified,
    Removed,
    MovedFrom,
    MovedTo,
}

/// Classify one inotify event mask. `IN_MOVED_FROM`/`IN_MOVED_TO` are paired by
/// cookie later; the rest map straight through. Returns `None` for masks that
/// carry no listing change (self events, `IN_IGNORED`, `IN_UNMOUNT`).
#[cfg(target_os = "linux")]
fn classify(mask: u32) -> Option<InotifyChange> {
    if mask & libc::IN_MOVED_FROM != 0 {
        Some(InotifyChange::MovedFrom)
    } else if mask & libc::IN_MOVED_TO != 0 {
        Some(InotifyChange::MovedTo)
    } else if mask & libc::IN_CREATE != 0 {
        Some(InotifyChange::Created)
    } else if mask & libc::IN_DELETE != 0 {
        Some(InotifyChange::Removed)
    } else if mask & (libc::IN_MODIFY | libc::IN_ATTRIB | libc::IN_CLOSE_WRITE) != 0 {
        Some(InotifyChange::Modified)
    } else {
        None
    }
}

/// Whether a single inotify mask names a directory (`IN_ISDIR`, T332).
#[cfg(target_os = "linux")]
fn is_directory_mask(mask: u32) -> bool {
    mask & libc::IN_ISDIR != 0
}

/// A [`FolderWatcher`] over Linux inotify, the local mechanism GIO's
/// `GFileMonitor` wraps. This is the sanctioned fallback, marked for
/// replacement ([`SANCTIONED_WATCHER_FALLBACK_MARKER`]); it watches exactly one
/// directory, non-recursively, because only that directory's listing is
/// visible.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct InotifyWatcher;

#[cfg(target_os = "linux")]
impl InotifyWatcher {
    /// Create the inotify fallback.
    pub fn new() -> Self {
        Self
    }

    /// The replacement marker for this backend.
    pub const fn marker(&self) -> &'static str {
        SANCTIONED_WATCHER_FALLBACK_MARKER
    }
}

#[cfg(target_os = "linux")]
impl FolderWatcher for InotifyWatcher {
    fn watch(&self, location: &Location) -> Result<Box<dyn WatchReader>, SourceError> {
        self.watch_entries(location, false)
    }

    fn watch_entries(
        &self,
        location: &Location,
        entries_only: bool,
    ) -> Result<Box<dyn WatchReader>, SourceError> {
        use std::ffi::CString;
        use std::io;
        use std::os::unix::ffi::OsStrExt;

        if !location.is_file() {
            return Err(SourceError::UnsupportedScheme(location.scheme().to_owned()));
        }
        let path = location
            .to_file_path()
            .ok_or_else(|| SourceError::UnsupportedScheme(location.scheme().to_owned()))?;
        // Watch the folder's canonical path (T333): an in-place rename
        // replaces the folder by a new inode, and a watch on the original path
        // would be stale (`IN_DELETE_SELF`/`IN_MOVE_SELF`) while the listing
        // at the same name keeps working. The worker's first read arms the
        // parent too, so the folder's own name reappearing is visible; when it
        // does, the watch re-arms on the name's new inode and the receiver is
        // asked to re-list.
        let watch_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        if fd < 0 {
            return Err(SourceError::from_io(
                &io::Error::last_os_error(),
                location.display(),
            ));
        }
        let c_path = CString::new(watch_path.as_os_str().as_bytes())
            .map_err(|_| SourceError::Io(format!("path contains NUL: {}", location.display())))?;
        let mask = libc::IN_CREATE
            | libc::IN_DELETE
            | libc::IN_MOVED_FROM
            | libc::IN_MOVED_TO
            | libc::IN_MODIFY
            | libc::IN_ATTRIB
            | libc::IN_CLOSE_WRITE
            | libc::IN_DELETE_SELF
            | libc::IN_MOVE_SELF;
        let wd = unsafe { libc::inotify_add_watch(fd, c_path.as_ptr(), mask) };
        if wd < 0 {
            let error = SourceError::from_io(&io::Error::last_os_error(), location.display());
            unsafe { libc::close(fd) };
            return Err(error);
        }
        // The folder's own name, so the parent's report of it (armed by the
        // worker before its first read) can be recognised.
        let base_name = watch_path.file_name().map(std::ffi::OsStr::to_os_string);
        Ok(Box::new(InotifyReader {
            fd,
            wd,
            parent: location.clone(),
            base_name,
            parent_armed: false,
            listening: entries_only,
            moved: std::collections::HashMap::new(),
            dot_hidden: crate::fallback::dot_hidden_names(&path),
            entries_only,
        }))
    }
}

#[cfg(target_os = "linux")]
struct InotifyReader {
    fd: libc::c_int,
    /// The folder inode's own descriptor: it reports the folder's entries and
    /// is the one the owner may re-arm.
    wd: libc::c_int,
    parent: Location,
    /// The folder's own name, to recognise it in the parent's events.
    base_name: Option<std::ffi::OsString>,
    /// Whether the parent's replacement watch is armed (it is opened by the
    /// worker before the first blocking read, T333).
    parent_armed: bool,
    /// The owning session lists this folder (`entries_only`, T332): only then
    /// does the folder's replacement matter, so only then is its parent
    /// watched (T333).
    listening: bool,
    /// `IN_MOVED_FROM` names awaiting their `IN_MOVED_TO`, by cookie.
    moved: std::collections::HashMap<u32, std::ffi::OsString>,
    /// Names the folder's `.hidden` file marks hidden (GIO semantics).
    dot_hidden: std::collections::HashSet<std::ffi::OsString>,
    /// Whether this watch is a folder's own listing (the model's watch), so a
    /// directory entry is reported as [`crate::NodeKind::Directory`] from
    /// `IN_ISDIR` even while it is empty (T332). A monitor of one subdirectory
    /// inside a listing (the Trash monitor's `info/`) leaves this false.
    entries_only: bool,
}

#[cfg(target_os = "linux")]
impl InotifyReader {
    /// Stat one entry the kernel named, if it still exists and is not hidden.
    /// This is the whole point of the watcher: no directory is re-listed.
    /// Hidden entries are not in the listing, so no event may add one.
    ///
    /// `kernel_dir` is `IN_ISDIR`: for a folder's own listing a directory is
    /// reported as such even when it is empty (T332).
    fn entry(&self, name: &std::ffi::OsStr, kernel_dir: bool) -> Option<Node> {
        if crate::fallback::is_hidden_name(name, &self.dot_hidden) {
            return None;
        }
        let path = self.parent.to_file_path()?.join(name);
        if std::fs::symlink_metadata(&path).is_err() {
            return None;
        }
        let mut node = crate::fallback::node_for_path(&self.parent, name.to_os_string(), &path);
        if self.entries_only && kernel_dir && node.kind() != crate::NodeKind::Symlink {
            node = node.with_kind(crate::NodeKind::Directory);
        }
        Some(node)
    }

    /// Open the parent's replacement watch and return its descriptor (a
    /// watch of a folder's parent sees its own name appear and disappear).
    fn watch_parent(&mut self) -> Option<libc::c_int> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let path = self.parent.to_file_path()?;
        let parent = path.parent()?;
        let c_parent = CString::new(parent.as_os_str().as_bytes()).ok()?;
        let wd = unsafe {
            libc::inotify_add_watch(
                self.fd,
                c_parent.as_ptr(),
                libc::IN_CREATE | libc::IN_MOVED_TO | libc::IN_DELETE | libc::IN_MOVED_FROM,
            )
        };
        (wd >= 0).then_some(wd)
    }

    fn removed(&self, name: &std::ffi::OsStr) -> WatchEventKind {
        WatchEventKind::Removed {
            uri: self.parent.child(name).uri().to_owned(),
        }
    }

    /// Re-arm this reader on the folder's name after its inode changed (an
    /// in-place rename, T333). `inotify_add_watch` on the path adds a fresh
    /// descriptor for the new inode; the replaced tree's rows are unknown to
    /// the receiver, so the caller also asks for a reset. Returns whether a
    /// watch is now installed (a name that is briefly absent re-arms when it
    /// comes back).
    fn rearm(&mut self) -> bool {
        let Some(name) = self.base_name.clone() else {
            return false;
        };
        let Some(path) = self.parent.to_file_path() else {
            return false;
        };
        let Some(path) = path.parent().map(|parent| parent.join(&name)) else {
            return false;
        };
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        if !path.is_dir() {
            return false;
        }
        let mask = libc::IN_CREATE
            | libc::IN_DELETE
            | libc::IN_MOVED_FROM
            | libc::IN_MOVED_TO
            | libc::IN_MODIFY
            | libc::IN_ATTRIB
            | libc::IN_CLOSE_WRITE
            | libc::IN_DELETE_SELF
            | libc::IN_MOVE_SELF;
        // The folder's own descriptor must be refreshed first; the parent's
        // `inotify_add_watch` would otherwise replace it and hand back a new
        // number for the folder.
        let folder_wd = unsafe { libc::inotify_add_watch(self.fd, c_path.as_ptr(), mask) };
        if folder_wd < 0 {
            return false;
        }
        self.wd = folder_wd;

        self.moved.clear();
        self.dot_hidden = crate::fallback::dot_hidden_names(&path);
        true
    }

    fn drain_fd(&mut self) -> Result<Vec<WatchEventKind>, SourceError> {
        use std::io;
        use std::os::unix::ffi::OsStringExt;

        /// A read buffer big enough for many events at one `NAME_MAX` name.
        const BUFFER: usize = 16 * 1024;

        let mut buffer = [0u8; BUFFER];
        let count = unsafe {
            libc::read(
                self.fd,
                buffer.as_mut_ptr().cast::<libc::c_void>(),
                buffer.len(),
            )
        };
        if count < 0 {
            let error = io::Error::last_os_error();
            if matches!(error.kind(), io::ErrorKind::WouldBlock) {
                return Ok(Vec::new());
            }
            return Err(SourceError::from_io(&error, self.parent.display()));
        }
        let count = count as usize;
        let mut rescan = false;
        let mut kinds = Vec::new();
        let header = std::mem::size_of::<libc::inotify_event>();
        let mut offset = 0usize;
        while offset + header <= count {
            let event = unsafe {
                std::ptr::read_unaligned(buffer.as_ptr().add(offset).cast::<libc::inotify_event>())
            };
            let name_start = offset + header;
            let name_len = event.len as usize;
            let name = if name_len > 0 && name_start + name_len <= count {
                let raw = &buffer[name_start..name_start + name_len];
                let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
                Some(std::ffi::OsString::from_vec(raw[..end].to_vec()))
            } else {
                None
            };
            offset = name_start + name_len;

            // Events that name no entry (`IN_DELETE_SELF`, `IN_IGNORED`) are
            // not listing changes.
            let (Some(name), Some(change)) = (name, classify(event.mask)) else {
                continue;
            };
            let kernel_dir = is_directory_mask(event.mask);
            // A directory appears or disappears under our own name: the
            // folder was replaced (an in-place rename gives it a new inode)
            // or came back. The watch re-arms on the name and the receiver
            // re-lists, because the listing at this location is a new tree
            // (T333).
            let folder_event = self.base_name.as_ref() == Some(&name)
                && event.mask & libc::IN_ISDIR != 0
                && matches!(
                    change,
                    InotifyChange::Created
                        | InotifyChange::MovedTo
                        | InotifyChange::MovedFrom
                        | InotifyChange::Removed
                );
            if folder_event {
                let appeared = matches!(change, InotifyChange::Created | InotifyChange::MovedTo);
                if appeared {
                    let _ = self.rearm();
                }
                rescan = true;
                continue;
            }
            match change {
                InotifyChange::MovedFrom => {
                    self.moved.insert(event.cookie, name);
                }
                InotifyChange::MovedTo => match self.moved.remove(&event.cookie) {
                    Some(from) => {
                        if let Some(to) = self.entry(&name, kernel_dir) {
                            kinds.push(WatchEventKind::Renamed {
                                from_uri: self.parent.child(&from).uri().to_owned(),
                                to,
                            });
                        } else if !crate::fallback::is_hidden_name(&from, &self.dot_hidden) {
                            // Visible → hidden: the row disappears from the
                            // listing instead of lingering with its old name.
                            kinds.push(self.removed(&from));
                        }
                    }
                    None => {
                        if let Some(node) = self.entry(&name, kernel_dir) {
                            kinds.push(WatchEventKind::Created(node));
                        }
                    }
                },
                InotifyChange::Created => {
                    if let Some(node) = self.entry(&name, kernel_dir) {
                        kinds.push(WatchEventKind::Created(node));
                    }
                }
                InotifyChange::Removed => kinds.push(self.removed(&name)),
                InotifyChange::Modified => {
                    if let Some(node) = self.entry(&name, kernel_dir) {
                        kinds.push(WatchEventKind::Modified(node));
                    }
                }
            }
        }
        // A move whose destination never arrived inside the folder moved *out*:
        // report it as a removal rather than leaving the row stale.
        let moved_out: Vec<_> = self.moved.drain().map(|(_, name)| name).collect();
        for name in moved_out {
            kinds.push(self.removed(&name));
        }
        if rescan {
            kinds.push(WatchEventKind::Rescan);
        }
        Ok(kinds)
    }
}

#[cfg(target_os = "linux")]
impl WatchReader for InotifyReader {
    fn next_batch(&mut self, timeout: Duration) -> Result<Vec<WatchEventKind>, SourceError> {
        // Arm the parent's replacement watch once, before the first read, so
        // the folder reporting itself is a real change (T333).
        if !self.parent_armed {
            self.parent_armed = true;
            if self.listening {
                let parent = self.watch_parent();
                debug_assert!(parent.is_none() || parent != Some(self.wd));
            }
        }
        let millis = timeout.as_millis().min(i32::MAX as u128) as i32;
        let mut pollfd = libc::pollfd {
            fd: self.fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pollfd, 1, millis) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if matches!(error.kind(), std::io::ErrorKind::Interrupted) {
                return Ok(Vec::new());
            }
            return Err(SourceError::from_io(&error, self.parent.display()));
        }
        if ready == 0 {
            return Ok(Vec::new());
        }
        self.drain_fd()
    }
}

#[cfg(target_os = "linux")]
impl Drop for InotifyReader {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::*;

    #[test]
    fn the_fallback_is_marked_for_replacement() {
        let marker = SANCTIONED_WATCHER_FALLBACK_MARKER;
        assert!(marker.contains("GIO/GVfs"), "marker: {marker}");
        assert!(marker.contains("replace"), "marker: {marker}");
        assert!(marker.contains("FolderWatcher"), "marker: {marker}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn masks_classify_to_listing_changes() {
        assert_eq!(classify(libc::IN_CREATE), Some(InotifyChange::Created));
        assert_eq!(classify(libc::IN_DELETE), Some(InotifyChange::Removed));
        assert_eq!(
            classify(libc::IN_MOVED_FROM),
            Some(InotifyChange::MovedFrom)
        );
        assert_eq!(classify(libc::IN_MOVED_TO), Some(InotifyChange::MovedTo));
        assert_eq!(classify(libc::IN_MODIFY), Some(InotifyChange::Modified));
        assert_eq!(
            classify(libc::IN_CLOSE_WRITE),
            Some(InotifyChange::Modified)
        );
        assert_eq!(classify(libc::IN_ATTRIB), Some(InotifyChange::Modified));
        // Self events carry no listing change.
        assert_eq!(classify(libc::IN_DELETE_SELF), None);
        assert_eq!(classify(libc::IN_IGNORED), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_foreign_scheme_cannot_be_watched() {
        let location = Location::parse("trash:///").expect("valid");
        let error = InotifyWatcher::new()
            .watch(&location)
            .err()
            .expect("unsupported");
        assert_eq!(error, SourceError::UnsupportedScheme("trash".to_owned()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_missing_folder_reports_not_found() {
        let location = Location::file("/definitely/not/here/dragonfruit-t103b");
        let error = InotifyWatcher::new()
            .watch(&location)
            .err()
            .expect("not found");
        assert!(matches!(error, SourceError::NotFound(_)), "got {error:?}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn masks_carry_the_directory_hint() {
        assert!(is_directory_mask(libc::IN_CREATE | libc::IN_ISDIR));
        assert!(!is_directory_mask(libc::IN_CREATE));
        assert!(!is_directory_mask(libc::IN_MOVED_TO));
    }

    /// Drive the worker over a real temp folder until `want` batches arrived,
    /// failing after a bounded wait.
    #[cfg(target_os = "linux")]
    fn batches_until(handle: &WatchHandle, want_batches: usize) -> Vec<WatchBatch> {
        use std::time::Instant;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut batches = Vec::new();
        while batches.len() < want_batches && Instant::now() < deadline {
            match handle.recv_batch_timeout(Duration::from_millis(20)) {
                // A folder reports its parent's name once when the reader is
                // armed (T333): that is not a listing change, so it does not
                // count towards the wanted batch.
                Some(batch)
                    if batch
                        .kinds
                        .iter()
                        .all(|kind| matches!(kind, WatchEventKind::Rescan)) => {}
                Some(batch) => batches.push(batch),
                None => continue,
            }
        }
        batches
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_burst_arrives_as_one_batch() {
        // T332: an extraction-sized burst — many creates written back to
        // back — must reach the consumer as one coalesced batch, not one
        // batch per file. The per-event stream carries the same changes.
        let dir = tempfile::tempdir().expect("temp dir");
        let watched = dir.path().join("listing");
        std::fs::create_dir(&watched).expect("mkdir");
        let location = Location::file(&watched);
        let handle = begin_entries(Arc::new(InotifyWatcher::new()), &location, 7).expect("watch");
        assert_eq!(handle.generation(), 7);
        // Let the kernel watch settle before the burst.
        std::thread::sleep(Duration::from_millis(100));

        for index in 0..24 {
            std::fs::write(watched.join(format!("burst-{index:02}.txt")), b"x").expect("write");
        }

        let batches = batches_until(&handle, 1);
        assert!(!batches.is_empty(), "the burst never arrived");
        let batch = &batches[0];
        assert_eq!(batch.generation(), 7);
        assert!(!batch.is_empty());
        let created: Vec<String> = batch
            .kinds
            .iter()
            .filter_map(|kind| match kind {
                WatchEventKind::Created(node) => Some(node.display_name().into_owned()),
                _ => None,
            })
            .collect();
        assert!(
            created.len() > 1,
            "a burst must coalesce several creates into one batch, got {created:?}"
        );
        // The per-event stream still reports every change of the batch.
        let mut events = batch.kinds.len();
        while let Some(event) = handle.try_recv() {
            assert_eq!(event.generation(), 7);
            events -= 1;
        }
        assert_eq!(events, 0, "the event stream mirrors the batch");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_created_folder_is_reported_as_a_directory() {
        // The listing's own watch honours IN_ISDIR, so a newly created (still
        // empty) folder reaches the model as a folder, not a file (T332).
        let dir = tempfile::tempdir().expect("temp dir");
        let watched = dir.path().join("listing");
        std::fs::create_dir(&watched).expect("mkdir");
        let location = Location::file(&watched);
        let handle = begin_entries(Arc::new(InotifyWatcher::new()), &location, 1).expect("watch");
        std::thread::sleep(Duration::from_millis(100));
        std::fs::create_dir(watched.join("New Folder")).expect("mkdir");

        let batches = batches_until(&handle, 1);
        let created = batches
            .iter()
            .flat_map(|batch| batch.kinds.iter())
            .find_map(|kind| match kind {
                WatchEventKind::Created(node) if node.display_name() == "New Folder" => {
                    Some(node.clone())
                }
                _ => None,
            })
            .expect("the created folder arrived");
        assert_eq!(created.kind(), crate::NodeKind::Directory);
        assert!(created.is_dir());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn an_idle_watch_sends_nothing() {
        // The debounce must not turn the watcher into a poller: with no
        // change, no batch is ever delivered. (The listing is a folder below
        // the temp root: the root is shared and noisy on a developer host.)
        let dir = tempfile::tempdir().expect("temp dir");
        let watched = dir.path().join("idle");
        std::fs::create_dir(&watched).expect("mkdir");
        let location = Location::file(&watched);
        let handle = begin_entries(Arc::new(InotifyWatcher::new()), &location, 1).expect("watch");
        for _ in 0..10 {
            // A folder replaces itself under the same name (T333): that is a
            // reset request, not a listing change.
            if let Some(batch) = handle.recv_batch_timeout(WATCH_COALESCE_WINDOW) {
                assert!(
                    batch
                        .kinds
                        .iter()
                        .all(|kind| matches!(kind, WatchEventKind::Rescan)),
                    "an idle folder produces no change batch: {:?}",
                    batch.kinds
                );
            }
        }
    }
}
