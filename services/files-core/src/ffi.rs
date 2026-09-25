// SPDX-License-Identifier: MIT
//! The hand-written C ABI the Qt model facade links against (T-10.4b).
//!
//! [09-files.md] says `files-core` is embedded by the Files window through a
//! "thin Qt model bridge". The project has no cxx-qt dependency and the
//! pinned toolchain is offline, so the bridge is a **hand-written
//! `QAbstractListModel` facade over a C ABI** ([adr/0049]). The C++ side owns
//! Qt model semantics and never touches the filesystem; this module owns the
//! listing, the sort, and the stable [`NodeId`]s.
//!
//! The ABI is deliberately small and snapshot-shaped:
//!
//! * [`df_files_begin`] starts a listing for a URI and returns an opaque
//!   session (the Rust `DirectoryModel` + its [`ListingHandle`]).
//! * [`df_files_poll`] blocks up to a timeout for one listing event and
//!   returns the **current ordered snapshot** of the model. Streaming is real
//!   (the model paints a prefix after the first batch); the snapshot keeps the
//!   C++ facade a pure pass-through, so there is exactly one collation — the
//!   one in [`crate::sort`]. T-10.5 replaces the whole-snapshot delivery with
//!   an incremental/windowed one; see [adr/0049].
//! * [`df_files_set_sort`] changes the ordering in place, [`df_files_snapshot`]
//!   returns the current order without waiting, and [`df_files_free`] retires
//!   the session (cancelling its worker).
//!
//! Callbacks are not used: the C++ facade polls from a timer on its own
//! thread, so a slow directory never blocks the UI and an idle window stops
//! polling once the listing completes.
//!
//! [09-files.md]: ../../../docs/design/09-files.md
//! [adr/0049]: ../../../docs/design/adr/0049-files-core-c-abi-bridge.md

use std::ffi::{c_char, CString};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use crate::sort::{SortDirection, SortKey, SortSpec};
use crate::{DirectoryModel, ListingHandle, ListingState, Location, Node, NodeKind, StdFsSource};

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

/// The opaque listing session.
pub struct FfiSession {
    model: DirectoryModel,
    handle: ListingHandle,
}

impl FfiSession {
    fn snapshot_event(&self) -> *mut df_files_event {
        let nodes: Vec<df_files_node> = self.model.ordered().map(to_ffi_node).collect();
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
    let mut model = DirectoryModel::new();
    let handle = model.begin(Arc::new(StdFsSource::new()), location);
    Box::into_raw(Box::new(FfiSession { model, handle }))
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
/// current ordered snapshot (for `BATCH`). Returns null only for a null
/// session. The caller frees the result with [`df_files_event_free`].
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
    match session
        .handle
        .recv_timeout(Duration::from_millis(u64::from(timeout_ms)))
    {
        Some(event) => {
            session.model.apply(event);
            match session.model.state() {
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
/// [`df_files_set_sort`] to repaint the same rows in the new order.
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
}
