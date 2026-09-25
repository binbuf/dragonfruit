// SPDX-License-Identifier: MIT
//! The hand-written C ABI the Qt model facade links against (T-10.4b/T-10.4c).
//!
//! [09-files.md] says `files-core` is embedded by the Files window through a
//! "thin Qt model bridge". The project has no cxx-qt dependency and the
//! pinned toolchain is offline, so the bridge is a **hand-written
//! `QAbstractListModel` facade over a C ABI** ([adr/0049]). The C++ side owns
//! Qt model semantics and never touches the filesystem; this module owns the
//! listing, the sort, the stable [`NodeId`]s, and — since T-10.4c — the
//! optimistic operations.
//!
//! The ABI is deliberately small and snapshot-shaped:
//!
//! * [`df_files_begin`] starts a listing for a URI and returns an opaque
//!   session (the Rust [`OptimisticModel`] + its [`ListingHandle`] + the
//!   operations worker).
//! * [`df_files_poll`] first drains any completed operation (confirming or
//!   reverting it), then blocks up to a timeout for one listing event and
//!   returns the **current ordered snapshot** of the model. Streaming is real
//!   (the model paints a prefix after the first batch); the snapshot keeps the
//!   C++ facade a pure pass-through, so there is exactly one collation — the
//!   one in [`crate::sort`]. T-10.5 replaces the whole-snapshot delivery with
//!   an incremental/windowed one; see [adr/0049].
//! * [`df_files_begin_rename`] / [`df_files_begin_new_folder`] /
//!   [`df_files_begin_trash`] apply the edit to the in-memory model
//!   **synchronously** (so the next frame paints it) and enqueue the real
//!   [`crate::FileOps`]/[`crate::TrashOps`] call to a worker thread. The
//!   worker's outcome is confirmed or reverted on the next poll, so no I/O
//!   ever runs on the UI thread.
//! * [`df_files_pending_ops`] tells the facade whether it must keep polling
//!   (there is an outcome still to fold), and [`df_files_take_error`] surfaces
//!   the last snapped-back failure for an inline notice.
//! * [`df_files_set_sort`] changes the ordering in place, [`df_files_snapshot`]
//!   returns the current order without waiting, and [`df_files_free`] retires
//!   the session (cancelling its listing worker and op worker).
//!
//! Callbacks are not used: the C++ facade polls from a timer on its own
//! thread, so a slow directory never blocks the UI and an idle window stops
//! polling once the listing completes and no operation is pending.
//!
//! [09-files.md]: ../../../docs/design/09-files.md
//! [adr/0049]: ../../../docs/design/adr/0049-files-core-c-abi-bridge.md

use std::ffi::{c_char, CString, OsString};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use crate::sort::{SortDirection, SortKey, SortSpec};
use crate::{
    DirectoryModel, FileOps, FreedesktopTrash, ListingHandle, ListingState, Location, Node, NodeId,
    NodeKind, OpId, OperationError, OptimisticModel, StdFsOps, StdFsSource, TrashOps,
};

/// Event status: a batch (a fresh ordered snapshot follows).
pub const DF_FILES_STATUS_BATCH: i32 = 0;
/// Event status: the listing completed.
pub const DF_FILES_STATUS_DONE: i32 = 1;
/// Event status: the listing failed (`error` carries the message).
pub const DF_FILES_STATUS_ERROR: i32 = 2;
/// Event status: no event arrived within the timeout.
pub const DF_FILES_STATUS_TIMEOUT: i32 = 3;

/// A [`NodeKind`] mirror for the C ABI.
pub const DF_NODE_DIRECTORY: i32 = 0;
/// A regular file.
pub const DF_NODE_FILE: i32 = 1;
/// A symbolic link.
pub const DF_NODE_SYMLINK: i32 = 2;
/// Anything else (socket, FIFO, device, unknown).
pub const DF_NODE_OTHER: i32 = 3;

/// One listed item, owned by the event that carries it.
#[repr(C)]
pub struct df_files_node {
    /// The stable per-session node id.
    pub id: u64,
    /// The display name (lossy UTF-8), NUL-terminated.
    pub name: *const c_char,
    /// The item URI, NUL-terminated.
    pub uri: *const c_char,
    /// One of the `DF_NODE_*` kinds.
    pub kind: i32,
    /// Whether `size` is meaningful.
    pub has_size: i32,
    /// The byte size (valid when `has_size`).
    pub size: u64,
    /// Whether `modified_ms` is meaningful.
    pub has_modified: i32,
    /// The modification time in Unix milliseconds (valid when `has_modified`).
    pub modified_ms: i64,
    /// The symlink target, or null.
    pub symlink_target: *const c_char,
}

/// One event from [`df_files_poll`] / [`df_files_snapshot`].
#[repr(C)]
pub struct df_files_event {
    /// One of the `DF_FILES_STATUS_*` values.
    pub status: i32,
    /// The failure message for `ERROR`, else null.
    pub error: *const c_char,
    /// The ordered snapshot (owned by this event).
    pub nodes: *mut df_files_node,
    /// The number of nodes in `nodes`.
    pub count: u32,
}

/// One real filesystem operation for the worker thread.
enum OpRequest {
    Rename {
        op: OpId,
        location: Location,
        new_name: OsString,
    },
    NewFolder {
        op: OpId,
        parent: Location,
    },
    Trash {
        op: OpId,
        location: Location,
    },
}

/// The worker's outcome for one [`OpRequest`].
enum OpOutcome {
    Rename {
        op: OpId,
        result: Result<Location, OperationError>,
    },
    NewFolder {
        op: OpId,
        result: Result<Location, OperationError>,
    },
    Trash {
        op: OpId,
        result: Result<(), OperationError>,
    },
}

/// Spawn the one operations worker. It owns the [`StdFsOps`]/[`FreedesktopTrash`]
/// backends and runs every real mutation off the UI thread; dropping the
/// request sender ends it.
fn spawn_op_worker() -> (Sender<OpRequest>, Receiver<OpOutcome>) {
    let (request_tx, request_rx) = mpsc::channel::<OpRequest>();
    let (outcome_tx, outcome_rx) = mpsc::channel::<OpOutcome>();
    std::thread::Builder::new()
        .name(String::from("files-core-ops"))
        .spawn(move || {
            let ops = StdFsOps::new();
            let trash = FreedesktopTrash::new();
            while let Ok(request) = request_rx.recv() {
                let outcome = match request {
                    OpRequest::Rename {
                        op,
                        location,
                        new_name,
                    } => OpOutcome::Rename {
                        op,
                        result: ops.rename(&location, &new_name),
                    },
                    OpRequest::NewFolder { op, parent } => OpOutcome::NewFolder {
                        op,
                        result: ops.new_folder(&parent),
                    },
                    OpRequest::Trash { op, location } => OpOutcome::Trash {
                        op,
                        result: trash.trash(&location).map(|_| ()),
                    },
                };
                if outcome_tx.send(outcome).is_err() {
                    break;
                }
            }
        })
        .expect("spawn files-core-ops worker");
    (request_tx, outcome_rx)
}

/// The opaque listing + operations session.
pub struct FfiSession {
    model: OptimisticModel,
    handle: ListingHandle,
    op_tx: Sender<OpRequest>,
    outcome_rx: Receiver<OpOutcome>,
    pending: u32,
    last_error: Option<String>,
}

impl FfiSession {
    fn snapshot_event(&self) -> *mut df_files_event {
        let nodes: Vec<df_files_node> = self.model.model().ordered().map(to_ffi_node).collect();
        let count = nodes.len() as u32;
        let mut boxed = nodes.into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        Box::into_raw(Box::new(df_files_event {
            status: DF_FILES_STATUS_BATCH,
            error: std::ptr::null(),
            nodes: ptr,
            count,
        }))
    }

    fn terminal_event(status: i32, error: Option<String>) -> *mut df_files_event {
        let error = error.map(|message| {
            CString::new(message)
                .unwrap_or_else(|_| CString::new("error").expect("static"))
                .into_raw()
        });
        Box::into_raw(Box::new(df_files_event {
            status,
            error: error.unwrap_or(std::ptr::null_mut()),
            nodes: std::ptr::null_mut(),
            count: 0,
        }))
    }

    /// Fold every completed operation: confirm the painted result, or revert
    /// it and remember the failure message. Returns whether anything changed.
    fn drain_ops(&mut self) -> bool {
        let mut changed = false;
        // `Err` is either Empty or Disconnected; both mean "nothing more now".
        while let Ok(outcome) = self.outcome_rx.try_recv() {
            changed = true;
            self.pending = self.pending.saturating_sub(1);
            match outcome {
                OpOutcome::Rename { op, result } => match result {
                    Ok(location) => {
                        self.model.retarget(op, &location);
                        self.model.confirm(op);
                    }
                    Err(error) => {
                        self.model.revert(op);
                        self.last_error = Some(error.to_string());
                    }
                },
                OpOutcome::NewFolder { op, result } => match result {
                    Ok(location) => {
                        self.model.retarget(op, &location);
                        self.model.confirm(op);
                    }
                    Err(error) => {
                        self.model.revert(op);
                        self.last_error = Some(error.to_string());
                    }
                },
                OpOutcome::Trash { op, result } => match result {
                    Ok(()) => {
                        self.model.confirm(op);
                    }
                    Err(error) => {
                        self.model.revert(op);
                        self.last_error = Some(error.to_string());
                    }
                },
            }
        }
        changed
    }

    /// The current listing state.
    fn listing_state(&self) -> &ListingState {
        self.model.model().state()
    }
}

fn to_ffi_node(node: &Node) -> df_files_node {
    let cstring = |value: &str| -> *const c_char {
        CString::new(value)
            .unwrap_or_else(|_| CString::new("").expect("empty"))
            .into_raw()
    };
    let modified_ms = node
        .modified()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    df_files_node {
        id: node.id().get(),
        name: cstring(&node.display_name()),
        uri: cstring(node.uri()),
        kind: match node.kind() {
            NodeKind::Directory => DF_NODE_DIRECTORY,
            NodeKind::File => DF_NODE_FILE,
            NodeKind::Symlink => DF_NODE_SYMLINK,
            NodeKind::Other => DF_NODE_OTHER,
        },
        has_size: i32::from(node.size().is_some()),
        size: node.size().unwrap_or(0),
        has_modified: i32::from(node.modified().is_some()),
        modified_ms,
        symlink_target: node
            .symlink_target()
            .map(cstring)
            .unwrap_or(std::ptr::null()),
    }
}

/// Start listing `uri` and return an opaque session, or null when the URI is
/// not a parseable `scheme://` location. The caller owns the session and must
/// release it with [`df_files_free`].
///
/// # Safety
///
/// `uri` must be a valid NUL-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_begin(uri: *const c_char) -> *mut FfiSession {
    if uri.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(uri) = std::ffi::CStr::from_ptr(uri).to_str() else {
        return std::ptr::null_mut();
    };
    let Ok(location) = Location::parse(uri) else {
        return std::ptr::null_mut();
    };
    let mut model = OptimisticModel::new(DirectoryModel::new());
    let handle = model.begin(Arc::new(StdFsSource::new()), location);
    let (op_tx, outcome_rx) = spawn_op_worker();
    Box::into_raw(Box::new(FfiSession {
        model,
        handle,
        op_tx,
        outcome_rx,
        pending: 0,
        last_error: None,
    }))
}

/// Retire a session, cancelling its listing worker. Null is ignored.
///
/// # Safety
///
/// `session` must be a pointer returned by [`df_files_begin`] that has not
/// already been freed.
#[no_mangle]
pub unsafe extern "C" fn df_files_free(session: *mut FfiSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Block up to `timeout_ms` for the next listing event and return the model's
/// current ordered snapshot (for `BATCH`). A completed operation is folded
/// first (confirm/revert) and, when it changed the model, is reported as a
/// fresh `BATCH` snapshot. Returns null only for a null session. The caller
/// frees the result with [`df_files_event_free`].
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`].
#[no_mangle]
pub unsafe extern "C" fn df_files_poll(
    session: *mut FfiSession,
    timeout_ms: u32,
) -> *mut df_files_event {
    let Some(session) = session.as_mut() else {
        return std::ptr::null_mut();
    };
    if session.drain_ops() {
        return match session.listing_state() {
            ListingState::Failed(error) => {
                FfiSession::terminal_event(DF_FILES_STATUS_ERROR, Some(error.to_string()))
            }
            _ => session.snapshot_event(),
        };
    }
    match session
        .handle
        .recv_timeout(Duration::from_millis(u64::from(timeout_ms)))
    {
        Some(event) => {
            session.model.apply(event);
            match session.listing_state() {
                ListingState::Failed(error) => {
                    FfiSession::terminal_event(DF_FILES_STATUS_ERROR, Some(error.to_string()))
                }
                ListingState::Complete => FfiSession::terminal_event(DF_FILES_STATUS_DONE, None),
                _ => session.snapshot_event(),
            }
        }
        None => FfiSession::terminal_event(DF_FILES_STATUS_TIMEOUT, None),
    }
}

/// Return the model's current ordered snapshot without waiting. Used after
/// [`df_files_set_sort`] or an optimistic begin to repaint the same rows.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`].
#[no_mangle]
pub unsafe extern "C" fn df_files_snapshot(session: *mut FfiSession) -> *mut df_files_event {
    let Some(session) = session.as_mut() else {
        return std::ptr::null_mut();
    };
    session.snapshot_event()
}

/// Change the session's sort order in place. `key` is one of `name`, `kind`,
/// `size`, `modified`; `direction` is `ascending`/`descending`. Returns 0 on
/// success, -1 on a bad key/direction or a null session.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`]; `key` and
/// `direction` must be valid NUL-terminated C strings or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_set_sort(
    session: *mut FfiSession,
    key: *const c_char,
    direction: *const c_char,
    folders_first: i32,
) -> i32 {
    let Some(session) = session.as_mut() else {
        return -1;
    };
    let key = read_str(key);
    let direction = read_str(direction);
    let (Some(key), Some(direction)) = (
        key.as_deref().and_then(SortKey::parse),
        direction.as_deref().and_then(SortDirection::parse),
    ) else {
        return -1;
    };
    session.model.set_sort(SortSpec {
        key,
        direction,
        folders_first: folders_first != 0,
    });
    0
}

/// Optimistically rename node `node_id` to `new_name` and queue the real
/// rename on the operations worker. Returns the operation id (`0` when the
/// node is unknown or `new_name` is null/invalid). The model is already
/// repainted when this returns; the outcome is folded by the next
/// [`df_files_poll`].
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`]; `new_name`
/// must be a valid NUL-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_begin_rename(
    session: *mut FfiSession,
    node_id: u64,
    new_name: *const c_char,
) -> u64 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let Some(name) = read_str(new_name) else {
        return 0;
    };
    let id = NodeId::from_raw(node_id);
    let Some(node) = session.model.model().node(id) else {
        return 0;
    };
    let Ok(location) = Location::parse(node.uri()) else {
        return 0;
    };
    let name = OsString::from(name);
    let Some(op) = session.model.begin_rename(id, name.clone()) else {
        return 0;
    };
    let _ = session.op_tx.send(OpRequest::Rename {
        op,
        location,
        new_name: name,
    });
    session.pending += 1;
    op.get()
}

/// Optimistically insert the next `untitled folder` under `parent_uri` and
/// queue the real creation. Returns the operation id, or `0` for a null
/// session / unparseable parent. See [`df_files_begin_rename`] for the
/// optimistic contract.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`];
/// `parent_uri` must be a valid NUL-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_begin_new_folder(
    session: *mut FfiSession,
    parent_uri: *const c_char,
) -> u64 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let Some(parent) = read_str(parent_uri).and_then(|uri| Location::parse(&uri).ok()) else {
        return 0;
    };
    let op = session.model.begin_new_folder(&parent);
    let _ = session.op_tx.send(OpRequest::NewFolder { op, parent });
    session.pending += 1;
    op.get()
}

/// Optimistically remove node `node_id` (Move to Trash) and queue the real
/// trash operation. Returns the operation id, or `0` when the node is unknown.
/// The row disappears immediately and snaps back if the trash fails.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`].
#[no_mangle]
pub unsafe extern "C" fn df_files_begin_trash(session: *mut FfiSession, node_id: u64) -> u64 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let id = NodeId::from_raw(node_id);
    let Some(node) = session.model.model().node(id) else {
        return 0;
    };
    let Ok(location) = Location::parse(node.uri()) else {
        return 0;
    };
    let Some(op) = session.model.begin_delete(id) else {
        return 0;
    };
    let _ = session.op_tx.send(OpRequest::Trash { op, location });
    session.pending += 1;
    op.get()
}

/// How many operations are still awaiting their worker outcome. The facade
/// keeps polling while this is non-zero, even after the listing completes.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_pending_ops(session: *const FfiSession) -> u32 {
    match session.as_ref() {
        Some(session) => session.pending,
        None => 0,
    }
}

/// Take the most recent operation failure message (a NUL-terminated UTF-8
/// string owned by the caller, or null when there is none) and clear it.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_take_error(session: *mut FfiSession) -> *mut c_char {
    let Some(session) = session.as_mut() else {
        return std::ptr::null_mut();
    };
    session
        .last_error
        .take()
        .map(|message| {
            CString::new(message)
                .unwrap_or_else(|_| CString::new("error").expect("static"))
                .into_raw()
        })
        .unwrap_or(std::ptr::null_mut())
}

/// Free a string returned by [`df_files_take_error`]. Null is ignored.
///
/// # Safety
///
/// `value` must be a pointer returned by [`df_files_take_error`] that has not
/// already been freed.
#[no_mangle]
pub unsafe extern "C" fn df_files_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}

/// Free an event returned by [`df_files_poll`] / [`df_files_snapshot`]. Null is
/// ignored.
///
/// # Safety
///
/// `event` must be a pointer returned by this module that has not already been
/// freed.
#[no_mangle]
pub unsafe extern "C" fn df_files_event_free(event: *mut df_files_event) {
    if event.is_null() {
        return;
    }
    let event = Box::from_raw(event);
    if !event.nodes.is_null() {
        let nodes = std::slice::from_raw_parts_mut(event.nodes, event.count as usize);
        for node in nodes {
            for field in [node.name, node.uri, node.symlink_target] {
                if !field.is_null() {
                    drop(CString::from_raw(field as *mut c_char));
                }
            }
        }
        drop(Vec::from_raw_parts(
            event.nodes,
            event.count as usize,
            event.count as usize,
        ));
    }
    if !event.error.is_null() {
        drop(CString::from_raw(event.error as *mut c_char));
    }
}

unsafe fn read_str(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .ok()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(value: &str) -> CString {
        CString::new(value).expect("no nul")
    }

    /// Poll until no operation is pending, returning the final snapshot (which
    /// the caller owns). Bounded so a hung worker fails rather than hangs.
    unsafe fn await_ops(session: *mut FfiSession) -> *mut df_files_event {
        // A completed listing's `recv_timeout` returns at once, so a tight poll
        // loop can outrun the worker's first schedule. Sleep a little so the
        // worker's outcome is actually observed.
        for _ in 0..1000 {
            let event = df_files_poll(session, 5);
            let status = if event.is_null() { -1 } else { (*event).status };
            df_files_event_free(event);
            if df_files_pending_ops(session) == 0 {
                return df_files_snapshot(session);
            }
            if status == DF_FILES_STATUS_ERROR {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        df_files_snapshot(session)
    }

    unsafe fn drain_listing(session: *mut FfiSession) {
        for _ in 0..500 {
            let event = df_files_poll(session, 20);
            if event.is_null() {
                return;
            }
            let status = (*event).status;
            df_files_event_free(event);
            if status == DF_FILES_STATUS_DONE || status == DF_FILES_STATUS_ERROR {
                return;
            }
        }
    }

    unsafe fn names(event: *const df_files_event) -> Vec<String> {
        let count = (*event).count as usize;
        let nodes = std::slice::from_raw_parts((*event).nodes, count);
        nodes
            .iter()
            .map(|node| {
                std::ffi::CStr::from_ptr(node.name)
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn a_bad_uri_yields_no_session() {
        unsafe {
            assert!(df_files_begin(std::ptr::null()).is_null());
            assert!(df_files_begin(c("not-a-uri").as_ptr()).is_null());
            assert!(df_files_begin(c("").as_ptr()).is_null());
        }
    }

    #[test]
    fn a_missing_directory_reports_an_error() {
        unsafe {
            let session = df_files_begin(c("file:///definitely/not/here/t104b").as_ptr());
            assert!(!session.is_null());
            let event = df_files_poll(session, 2_000);
            assert!(!event.is_null());
            assert_eq!((*event).status, DF_FILES_STATUS_ERROR);
            assert!(!(*event).error.is_null());
            df_files_event_free(event);
            df_files_free(session);
        }
    }

    #[test]
    fn sorting_rejects_bad_keys() {
        unsafe {
            let session = df_files_begin(c("file:///tmp").as_ptr());
            assert!(!session.is_null());
            assert_eq!(
                df_files_set_sort(session, c("nope").as_ptr(), c("asc").as_ptr(), 1),
                -1
            );
            assert_eq!(
                df_files_set_sort(session, c("name").as_ptr(), c("sideways").as_ptr(), 1),
                -1
            );
            assert_eq!(
                df_files_set_sort(session, c("name").as_ptr(), c("asc").as_ptr(), 1),
                0
            );
            df_files_free(session);
        }
    }

    /// Find a node's id by display name in a snapshot.
    unsafe fn id_named(event: *const df_files_event, name: &str) -> u64 {
        let count = (*event).count as usize;
        let nodes = std::slice::from_raw_parts((*event).nodes, count);
        for node in nodes {
            let display = std::ffi::CStr::from_ptr(node.name).to_string_lossy();
            if display == name {
                return node.id;
            }
        }
        0
    }

    #[test]
    fn optimistic_rename_commits_on_success() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), b"alpha").expect("write");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            assert!(!session.is_null());
            drain_listing(session);

            let snapshot = df_files_snapshot(session);
            let id = id_named(snapshot, "alpha.txt");
            df_files_event_free(snapshot);
            assert!(id > 0);

            let op = df_files_begin_rename(session, id, c("renamed.txt").as_ptr());
            assert!(op > 0);
            assert_eq!(df_files_pending_ops(session), 1);
            // The edit is already painted, before any worker outcome.
            let painted = df_files_snapshot(session);
            assert!(names(painted).iter().any(|name| name == "renamed.txt"));
            df_files_event_free(painted);

            let final_event = await_ops(session);
            assert_eq!(df_files_pending_ops(session), 0);
            let final_names = names(final_event);
            df_files_event_free(final_event);
            assert!(final_names.iter().any(|name| name == "renamed.txt"));
            assert!(!final_names.iter().any(|name| name == "alpha.txt"));
            assert!(dir.path().join("renamed.txt").exists());

            let error = df_files_take_error(session);
            assert!(error.is_null(), "a successful op records no error");
            df_files_free(session);
        }
    }

    #[test]
    fn optimistic_rename_reverts_on_conflict() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), b"alpha").expect("write");
        std::fs::write(dir.path().join("beta.txt"), b"beta").expect("write");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            assert!(!session.is_null());
            drain_listing(session);

            let snapshot = df_files_snapshot(session);
            let id = id_named(snapshot, "alpha.txt");
            df_files_event_free(snapshot);

            // Renaming onto the existing `beta.txt` fails with AlreadyExists.
            let op = df_files_begin_rename(session, id, c("beta.txt").as_ptr());
            assert!(op > 0);

            let final_event = await_ops(session);
            assert_eq!(df_files_pending_ops(session), 0);
            let final_names = names(final_event);
            df_files_event_free(final_event);
            // Snapped back to the original name.
            assert!(final_names.iter().any(|name| name == "alpha.txt"));
            assert!(dir.path().join("alpha.txt").exists());

            let error = df_files_take_error(session);
            assert!(!error.is_null(), "a reverted op records an error");
            df_files_string_free(error);
            assert!(
                df_files_take_error(session).is_null(),
                "error is taken once"
            );
            df_files_free(session);
        }
    }

    #[test]
    fn optimistic_new_folder_commits_and_creates_on_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            assert!(!session.is_null());
            drain_listing(session);

            let op = df_files_begin_new_folder(session, c(&uri).as_ptr());
            assert!(op > 0);
            // Painted immediately.
            let painted = df_files_snapshot(session);
            assert!(names(painted).iter().any(|name| name == "untitled folder"));
            df_files_event_free(painted);

            let final_event = await_ops(session);
            df_files_event_free(final_event);
            assert_eq!(df_files_pending_ops(session), 0);
            assert!(dir.path().join("untitled folder").is_dir());
            df_files_free(session);
        }
    }
}
