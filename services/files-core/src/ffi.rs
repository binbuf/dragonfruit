// SPDX-License-Identifier: GPL-3.0-or-later
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
//!   [`df_files_begin_trash`] / [`df_files_begin_empty_trash`] apply the edit
//!   to the in-memory model **synchronously** (so the next frame paints it)
//!   and enqueue the real [`crate::FileOps`]/[`crate::TrashOps`] call to a
//!   worker thread. The worker's outcome is confirmed or reverted on the next
//!   poll, so no I/O ever runs on the UI thread.
//! * [`df_files_begin_paste`] queues one clipboard item (T330) and
//!   [`df_files_paste_state`] reports the batch's per-item counters. The
//!   clipboard itself lives in the shell so it survives navigation.
//! * [`df_files_pending_ops`] tells the facade whether it must keep polling
//!   (there is an outcome still to fold), and [`df_files_take_error`] surfaces
//!   the last snapped-back failure for an inline notice.
//! * [`df_files_undo`] (T331) replays the bounded undo stack off the UI
//!   thread (`redo != 0` replays the redo stack); the outcome is folded on
//!   the next poll or by [`df_files_await_ops`], and a refusal lands in
//!   [`df_files_undo_status`] with its reason.
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

use std::collections::HashSet;
use std::ffi::{c_char, CString, OsString};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use crate::sort::{SortDirection, SortKey, SortSpec};
use crate::watch;
use crate::{
    paste_one, ClipboardMode, ConflictPolicy, DirectoryModel, DirectorySource, FileOps,
    FolderWatcher, FreedesktopTrash, ListingHandle, ListingState, Location, Node, NodeId, NodeKind,
    OpId, OperationError, OptimisticModel, PasteItemOutcome, PasteStatus, StdFsOps, StdFsSource,
    SyntheticSource, TrashMonitor, TrashOps, TrashSource, TrashState, TrashedItem, UndoError,
    UndoStack, WatchBatch, WatchEventKind, WatchHandle,
};

/// Event status: a batch (a fresh ordered snapshot follows).
pub const DF_FILES_STATUS_BATCH: i32 = 0;
/// Event status: the listing completed.
pub const DF_FILES_STATUS_DONE: i32 = 1;
/// Event status: the listing failed (`error` carries the message).
pub const DF_FILES_STATUS_ERROR: i32 = 2;
/// Event status: no event arrived within the timeout.
pub const DF_FILES_STATUS_TIMEOUT: i32 = 3;

/// Delta kind: `rows` is the full ordered model; replace every row.
pub const DF_FILES_DELTA_RESET: i32 = 0;
/// Delta kind: the rows named by `changes` were replaced in place; their ids
/// are unchanged and their ranks come from `changes`.
pub const DF_FILES_DELTA_CHANGED: i32 = 1;
/// Delta kind: `rows` are new rows, at their final `rank`s (T332).
pub const DF_FILES_DELTA_INSERT: i32 = 2;
/// Delta kind: the rows named by `changes` were removed; repaint from a reset.
pub const DF_FILES_DELTA_REMOVED: i32 = 3;

/// A [`NodeKind`] mirror for the C ABI.
pub const DF_NODE_DIRECTORY: i32 = 0;
/// A regular file.
pub const DF_NODE_FILE: i32 = 1;
/// A symbolic link.
pub const DF_NODE_SYMLINK: i32 = 2;
/// Anything else (socket, FIFO, device, unknown).
pub const DF_NODE_OTHER: i32 = 3;

/// The clipboard marks items to copy.
pub const DF_FILES_CLIPBOARD_COPY: i32 = 0;
/// The clipboard marks items to cut (move on paste).
pub const DF_FILES_CLIPBOARD_CUT: i32 = 1;

/// A conflicting destination is removed, then the source is placed.
pub const DF_FILES_CONFLICT_REPLACE: i32 = 0;
/// A conflicting destination is left alone; the item is skipped.
pub const DF_FILES_CONFLICT_SKIP: i32 = 1;
/// A conflicting destination stays; the source gets the next free name.
pub const DF_FILES_CONFLICT_KEEP_BOTH: i32 = 2;

/// Per-item paste progress for the Qt facade (T330). `active` counts queued
/// items still on the worker; `total`/`done` are the current paste batch.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct df_files_paste_progress {
    /// Items still awaiting a worker outcome.
    pub active: u32,
    /// Items queued in the current paste batch.
    pub total: u32,
    /// Items that have produced an outcome.
    pub done: u32,
    /// Items copied successfully.
    pub copied: u32,
    /// Items skipped by conflict policy.
    pub skipped: u32,
    /// Items that failed.
    pub failed: u32,
    /// `DF_FILES_CLIPBOARD_*`, or -1 for no paste yet.
    pub mode: i32,
}

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

/// One ordered row in an incremental delta: a node and its final rank in the
/// ordered projection.
#[repr(C)]
pub struct df_files_row {
    /// The node's position in the new ordered projection.
    pub rank: u32,
    /// The node itself (strings owned by the delta).
    pub node: df_files_node,
}

/// One in-place row change in a [`df_files_delta`] (T332): the node whose
/// fields changed and its rank in the ordered projection after the change.
#[repr(C)]
pub struct df_files_change {
    /// The node's rank after the change.
    pub rank: u32,
    /// The node (strings owned by the delta).
    pub node: df_files_node,
}

/// An incremental event (T-10.5/T332, [adr/0051], [adr/0339]).
///
/// `kind` says what the delta carries:
///
/// * [`DF_FILES_DELTA_RESET`] — replace the whole model with `rows`, in order
///   (the first paint, a sort change, an operation outcome, a watch removal).
/// * [`DF_FILES_DELTA_INSERT`] — insert `rows` at their final `rank`s; the
///   existing rows keep their relative order (the streaming merge only
///   interleaves new nodes; a watcher burst's creates are new rows).
/// * [`DF_FILES_DELTA_CHANGED`] — rows the receiver already holds were
///   replaced in place (a rename kept its id, a size changed). `changes`
///   names them with their new ranks; the row set and order are otherwise
///   what the receiver already paints.
/// * [`DF_FILES_DELTA_REMOVED`] — rows were removed; `changes` names the old
///   ids. The receiver repaints from a reset (a deletion shifts every later
///   rank). A zero-row removal exists so a truthful operation can answer
///   without a repaint.
///
/// `total` is the model length after the event.
///
/// [adr/0051]: ../../../docs/design/adr/0051-files-core-incremental-delta-abi.md
/// [adr/0339]: ../../../docs/design/adr/0339-files-watcher-delta-reconciliation.md
#[repr(C)]
pub struct df_files_delta {
    /// One of the `DF_FILES_STATUS_*` values.
    pub status: i32,
    /// The failure message for `ERROR`, else null.
    pub error: *const c_char,
    /// One of the `DF_FILES_DELTA_*` kinds.
    pub kind: i32,
    /// The rows (owned by this delta): a reset/insert payload.
    pub rows: *mut df_files_row,
    /// The number of rows.
    pub row_count: u32,
    /// The in-place changes (owned by this delta): a changed/removed payload.
    pub changes: *mut df_files_change,
    /// The number of changes.
    pub change_count: u32,
    /// The model's ordered length after this event.
    pub total: u32,
}

/// One real filesystem operation for the worker thread.
enum OpRequest {
    Rename {
        op: OpId,
        location: Location,
        new_name: OsString,
        /// The session row the rename edits, so the undo entry can name it.
        node: NodeId,
    },
    /// One item's `Move to Trash` is not here: see [`OpRequest::Trash`].
    NewFolder {
        op: OpId,
        parent: Location,
        /// The session row of the provisional folder.
        node: NodeId,
    },
    Trash {
        op: OpId,
        location: Location,
        /// The session row being trashed.
        node: NodeId,
    },
    EmptyTrash {
        op: OpId,
    },
    /// One clipboard item pasted into `target`. `op` is the optimistic row
    /// insert (a copy into the current folder) and `remove_op` the optimistic
    /// source removal (a cut whose source row is in the current model).
    Paste {
        op: Option<OpId>,
        remove_op: Option<OpId>,
        source: Location,
        target: Location,
        name: OsString,
        mode: ClipboardMode,
        policy: ConflictPolicy,
        skip: bool,
        /// The session row the paste painted (its optimistic insert, or the
        /// cut source's row when the item was moved out of this folder).
        node: NodeId,
    },
    /// Undo or redo the one recorded step at the top of `entry`'s stack
    /// (T331). The stack itself is moved over so the worker owns it for the
    /// replay and hands it back with the result.
    Replay {
        stack: Option<Box<UndoStack>>,
        redo: bool,
    },
}

impl OpRequest {
    /// The session row this request edits, if it edits one. The C ABI uses it
    /// to recognize an operation's own watcher echo (T332).
    fn node(&self) -> Option<NodeId> {
        match self {
            OpRequest::Rename { node, .. }
            | OpRequest::NewFolder { node, .. }
            | OpRequest::Trash { node, .. }
            | OpRequest::Paste { node, .. } => Some(*node),
            OpRequest::EmptyTrash { .. } | OpRequest::Replay { .. } => None,
        }
    }
}

/// The worker's outcome for one [`OpRequest`].
enum OpOutcome {
    Rename {
        op: OpId,
        result: Result<Location, OperationError>,
        /// Where the node lived before the rename (the undo entry's `from`).
        from: Location,
        /// The session row the rename edited.
        node: NodeId,
    },
    NewFolder {
        op: OpId,
        result: Result<Location, OperationError>,
        /// The session row of the provisional folder.
        node: NodeId,
    },
    Trash {
        op: OpId,
        result: Result<(), OperationError>,
        /// The store item the trash produced (for undo's Put Back).
        item: Option<TrashedItem>,
        /// The session row the trash removed.
        node: NodeId,
    },
    EmptyTrash {
        op: OpId,
        result: Result<(), OperationError>,
    },
    Paste {
        op: Option<OpId>,
        remove_op: Option<OpId>,
        /// The session row of the pasted item (the optimistic insert, or the
        /// cut source's row), so an undo names it.
        node: NodeId,
        result: Result<PasteItemOutcome, OperationError>,
    },
    Replay {
        stack: Option<Box<UndoStack>>,
        redo: bool,
        result: Result<crate::UndoOutcome, UndoError>,
    },
}

/// A replay's recorded end state, for the C ABI's [`df_files_undo_state`].
#[derive(Debug, Clone, Copy, Default)]
struct ReplayReport {
    /// The last replay finished and succeeded.
    ok: bool,
    /// The last replay was refused; `undo_message` carries the reason.
    refused: bool,
    /// 1 for a replay that is still on the worker.
    active: bool,
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
                        node,
                    } => OpOutcome::Rename {
                        op,
                        from: location.clone(),
                        node,
                        result: ops.rename(&location, &new_name),
                    },
                    OpRequest::NewFolder { op, parent, node } => OpOutcome::NewFolder {
                        op,
                        node,
                        result: ops.new_folder(&parent),
                    },
                    OpRequest::Trash { op, location, node } => {
                        let result = trash.trash(&location);
                        OpOutcome::Trash {
                            op,
                            node,
                            item: result.as_ref().ok().cloned(),
                            result: result.map(|_| ()),
                        }
                    }
                    OpRequest::EmptyTrash { op } => OpOutcome::EmptyTrash {
                        op,
                        result: trash.empty().map(|_| ()),
                    },
                    OpRequest::Replay { mut stack, redo } => {
                        let result = match stack.as_mut() {
                            Some(stack) => {
                                if redo {
                                    stack.redo(&ops, &trash)
                                } else {
                                    stack.undo(&ops, &trash)
                                }
                            }
                            None => Err(UndoError::Empty),
                        };
                        OpOutcome::Replay {
                            stack,
                            redo,
                            result,
                        }
                    }
                    OpRequest::Paste {
                        op,
                        remove_op,
                        source,
                        target,
                        name,
                        mode,
                        policy,
                        skip,
                        node,
                    } => OpOutcome::Paste {
                        op,
                        remove_op,
                        node,
                        result: if skip {
                            Ok(PasteItemOutcome {
                                destination: target.child(&name),
                                source,
                                name,
                                status: PasteStatus::Skipped,
                                error: None,
                            })
                        } else {
                            paste_one(&ops, &source, &target, &name, mode, policy)
                        },
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
    /// Per-item paste progress (T330). A new batch starts when the previous
    /// one is settled; the Qt facade reads it through
    /// [`df_files_paste_state`].
    paste_active: u32,
    paste_total: u32,
    paste_done: u32,
    paste_copied: u32,
    paste_skipped: u32,
    paste_failed: u32,
    paste_mode: i32,
    /// The T331 undo/redo stacks. They live in the session (one folder
    /// visit) and are cleared on a location change.
    undo: Option<Box<UndoStack>>,
    /// The open paste batch token, so the items of one clipboard paste fold
    /// into one undo step and the next paste starts a new one.
    paste_batch: u64,
    /// The session row the last queued paste painted (T331).
    paste_row: Option<NodeId>,
    /// The ids of the last successful trash, whatever this session listed:
    /// `id -> TrashedItem` for Put Back and for an undo replay.
    trash_record: std::collections::HashMap<u64, TrashedItem>,
    /// The most recent replay's report (see [`df_files_undo_state`]).
    replay: ReplayReport,
    /// The last refusal's reason, already formatted for a notice.
    undo_message: Option<String>,
    /// The next undo/redo's label, mirrored locally (T331) so the Qt facade
    /// reads the undo availability without borrowing a session-owned string.
    undo_label: Option<String>,
    redo_label: Option<String>,
    /// The node ids the receiver has already been told about, so an
    /// incremental poll can serialize only the newly arrived nodes (T-10.5).
    /// Seeded by every `reset` delta; grown by every insertion/changed delta.
    reported: HashSet<NodeId>,
    /// The live folder watch (T332). A local `file://` listing watches its own
    /// directory; the Trash and synthetic listings have none (the Trash is
    /// re-listed on navigation, the synthetic tree does not exist). Events are
    /// folded off the UI thread and delivered as one delta per burst.
    watch: Option<WatchHandle>,
    /// Whether the last delta told the receiver about `watch` (false for a
    /// session whose folder could not be watched). Reserved for the facade's
    /// watch indicator.
    #[allow(dead_code)]
    watch_reported: bool,
    /// Rows named by operations that have not finished yet. A watcher event
    /// about one of them is the operation's own echo, so a dormant poll must
    /// not treat it as external progress (the pending operation wakes it
    /// anyway).
    op_rows: std::collections::VecDeque<NodeId>,
    /// The per-item paste calls whose worker work has not started. The
    /// paste's own folder writes (a copy into the visible folder, a cut's
    /// removal) arrive as watch events; they must wake the UI so the progress
    /// counters and the confirmation keep moving.
    quiet_writes: u32,
}

impl FfiSession {
    /// Whether the session watches its folder (T332).
    fn watching(&self) -> bool {
        self.watch.is_some()
    }

    /// Block until a watch batch is ready (or `timeout` elapses) **without
    /// consuming it**. `WatchHandle` has no peek, so the worker's gate is the
    /// model side: a batch the handle still holds is polled by
    /// [`Self::drain_watch`]. This only lets a dormant poll sleep on a
    /// channel, never on a timer: the thread yields until the handle's
    /// cancellation-free wait reports readiness. Implemented as a short
    /// `recv_timeout` on a clone of the handle's readiness flag.
    fn wait_for_watch(&self, timeout: Duration) -> bool {
        let Some(handle) = self.watch.as_ref() else {
            return false;
        };
        handle.wait_ready(timeout)
    }

    /// A delta for changes the receiver has not been told about, or `None`
    /// when there are none. Used after a quiet wait so a burst that landed
    /// between the waits is still described.
    fn incremental_delta_if_pending(&mut self) -> Option<*mut df_files_delta> {
        let len = self.model.model().len();
        // A read-only probe: `incremental_delta` takes the markers itself.
        let pending = !self.model.model().added_ids().is_empty()
            || self.model.model().removal_pending()
            || self.model.model().pending_changes()
            || self.reported.len() != len;
        if pending {
            Some(self.incremental_delta())
        } else {
            None
        }
    }

    /// Queue one operation and remember the row it edits, so its own watcher
    /// echo is never mistaken for external progress (T332).
    fn queue_op(&mut self, request: OpRequest) {
        let row = request.node();
        if self.op_tx.send(request).is_ok() {
            self.pending += 1;
            if let Some(node) = row {
                self.op_rows.push_back(node);
            }
        }
    }

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

    /// A delta carrying every row in order ([`DF_FILES_DELTA_RESET`]) and
    /// re-seeding the reported id set. Used for the first paint, a sort
    /// change, and any model edit whose effect on the ordering is not
    /// append-only.
    fn reset_delta(&mut self, status: i32) -> *mut df_files_delta {
        self.reported.clear();
        let mut rows: Vec<df_files_row> = Vec::with_capacity(self.model.model().len());
        let mut ids: Vec<NodeId> = Vec::with_capacity(self.model.model().len());
        for (rank, node) in self.model.model().ordered().enumerate() {
            rows.push(df_files_row {
                rank: rank as u32,
                node: to_ffi_node(node),
            });
            ids.push(node.id());
        }
        for id in ids {
            self.reported.insert(id);
        }
        self.model.model_mut().mark_known();
        self.model.model_mut().take_watch_changes();
        self.model.model_mut().take_removal();
        let total = rows.len() as u32;
        delta_from_rows(status, DF_FILES_DELTA_RESET, rows, total)
    }

    /// A delta describing what changed since the receiver last painted its
    /// rows (T-10.5/T332): insertions at their final ranks, in-place changes
    /// for rows it already holds, or — when a row vanished — an instruction
    /// to repaint from a reset. The streaming path sends each node exactly
    /// once, so a burst that only touched metadata costs one small delta.
    fn incremental_delta(&mut self) -> *mut df_files_delta {
        let len = self.model.model().len();
        let vanished = self.model.model_mut().take_removal();
        let changed = self.model.model_mut().take_watch_changes();
        // A vanished row is reported as a reset only when a row the receiver
        // paints is gone; a removal that immediately re-created the row under
        // its new name (a burst's rename) keeps every rank honest.
        let vanished_reported = vanished
            && (self.reported.len() > len
                || self
                    .reported
                    .iter()
                    .filter(|id| self.model.model().node(**id).is_none())
                    .count()
                    > 0);
        if vanished_reported {
            return self.reset_delta(DF_FILES_STATUS_BATCH);
        }

        // Rows the receiver has never been told about, at their final ranks.
        // The streaming path and a watcher burst only interleave new rows, so
        // the insertion delta applies directly; anything else (an optimistic
        // insert landing between reported rows mid-stream) is a reset.
        let total = len as u32;
        let mut rows: Vec<df_files_row> = Vec::new();
        let mut ids: Vec<NodeId> = Vec::new();
        let mut seen = 0usize;
        for (rank, node) in self.model.model().ordered().enumerate() {
            if self.reported.contains(&node.id()) {
                seen += 1;
            } else {
                rows.push(df_files_row {
                    rank: rank as u32,
                    node: to_ffi_node(node),
                });
                ids.push(node.id());
            }
        }
        if seen + rows.len() != len {
            return self.reset_delta(DF_FILES_STATUS_BATCH);
        }
        if !rows.is_empty() {
            for id in ids {
                self.reported.insert(id);
            }
            return delta_from_rows(DF_FILES_STATUS_BATCH, DF_FILES_DELTA_INSERT, rows, total);
        }

        // In-place changes: rows the receiver paints already, at their new
        // ranks. A changed row must never be one it has not been told about.
        let changes: Vec<df_files_change> = changed
            .iter()
            .filter(|id| self.reported.contains(id))
            .filter_map(|id| {
                let rank = self.model.model().ordered_position_of(*id)?;
                let node = self.model.model().node(*id)?;
                Some(df_files_change {
                    rank: rank as u32,
                    node: to_ffi_node(node),
                })
            })
            .collect();
        if !changes.is_empty() {
            return delta_from_changes(
                DF_FILES_STATUS_BATCH,
                DF_FILES_DELTA_CHANGED,
                changes,
                total,
            );
        }
        // Nothing to repaint. A plain listing poll answers `TIMEOUT` (the
        // facade keeps its timer); a settled session answers a zero-row
        // removal so an `await` can return as soon as its operation is done.
        if self.reported.len() == len {
            delta_from_changes(
                DF_FILES_STATUS_BATCH,
                DF_FILES_DELTA_REMOVED,
                Vec::new(),
                total,
            )
        } else {
            terminal_delta(DF_FILES_STATUS_TIMEOUT, None)
        }
    }

    /// Fold the ready watch batches into the model (T332), so one burst is one
    /// delta. Returns true when the model changed — a removal included, since
    /// the receiver must repaint from a reset then. The op-echo rule keeps
    /// pending operations' own writes quiet.
    fn drain_watch(&mut self) -> bool {
        loop {
            let Some(handle) = self.watch.as_ref() else {
                return false;
            };
            // Take a copy of the ready batches so the rest of this method can
            // borrow the session mutably.
            let mut ready = Vec::new();
            while let Some(batch) = handle.try_recv_batch() {
                ready.push(batch);
            }
            if ready.is_empty() {
                return false;
            }
            let mut woke = false;
            for batch in ready {
                let progress = self.operation_progress(&batch);
                let vanished = batch.kinds.iter().any(|kind| {
                    matches!(kind, WatchEventKind::Removed { .. })
                        || matches!(kind, WatchEventKind::Renamed { .. })
                });
                self.model.apply_watch_batch(batch);
                woke |= progress || vanished;
            }
            // Probe, never consume: the delta reads the same markers.
            // A burst that only reordered held rows (a rename) is described
            // by the next delta, so it wakes the poll too.
            let pending = self.model.model().removal_pending()
                || !self.model.model().added_ids().is_empty()
                || self.model.model().pending_changes()
                || self.model.model().len() > self.reported.len();
            if woke || pending {
                return true;
            }
        }
    }

    /// Whether a dormant poll should wake for `batch`: the operation worker
    /// has a request left for one of these rows (its own write, seen by the
    /// watcher a moment after the queue call), so the UI must not sleep on
    /// the operation's result. Every other change of a settled window is
    /// described on the next delta request without a timer.
    fn operation_progress(&mut self, batch: &WatchBatch) -> bool {
        let mut woke = false;
        for kind in &batch.kinds {
            match kind {
                WatchEventKind::Created(node) | WatchEventKind::Modified(node) => {
                    woke |= self.op_rows.iter().any(|row| *row == node.id());
                }
                WatchEventKind::Removed { .. } => woke |= self.quiet_writes > 0,
                WatchEventKind::Renamed { .. } => woke |= !self.op_rows.is_empty(),
                // The folder was replaced: the receiver must re-list.
                WatchEventKind::Rescan => woke = true,
            }
        }
        woke
    }

    /// Fold every completed operation: confirm the painted result, or revert
    /// it and remember the failure message. Returns whether anything changed.
    fn drain_ops(&mut self) -> bool {
        let mut changed = false;
        // `Err` is either Empty or Disconnected; both mean "nothing more now".
        while let Ok(outcome) = self.outcome_rx.try_recv() {
            // The row list shrinks; the watch queue may have advanced by the
            // time this runs, so recount from the outcomes still queued.
            self.pending = self.pending.saturating_sub(1);
            if let Some(node) = outcome_node(&outcome) {
                if self.pending == 0 {
                    self.op_rows.clear();
                } else if let Some(at) = self.op_rows.iter().position(|row| *row == node) {
                    self.op_rows.remove(at);
                }
            }
            if self.quiet_writes > 0 {
                self.quiet_writes -= 1;
            }
            self.fold_one(outcome);
            changed = true;
        }
        changed
    }

    /// Fold one completed operation: confirm the painted result, or revert it
    /// and remember the failure message. Shared by the poll and the bounded
    /// [`df_files_await_ops`] wait.
    fn fold_one(&mut self, outcome: OpOutcome) {
        {
            self.pending = self.pending.saturating_sub(1);
            match outcome {
                OpOutcome::Rename {
                    op,
                    from,
                    node,
                    result,
                } => match result {
                    Ok(location) => {
                        self.model.retarget(op, &location);
                        self.model.confirm(op);
                        if let Some(stack) = self.undo.as_mut() {
                            stack.record_rename_of(node, from, location);
                        }
                    }
                    Err(error) => {
                        self.model.revert(op);
                        self.last_error = Some(error.to_string());
                    }
                },
                OpOutcome::NewFolder { op, node, result } => match result {
                    Ok(location) => {
                        self.model.retarget(op, &location);
                        self.model.confirm(op);
                        if let Some(stack) = self.undo.as_mut() {
                            let row = self
                                .model
                                .model()
                                .node_id_for_location(&location)
                                .unwrap_or(node);
                            stack.record_new_folder(location);
                            stack.set_last_node(row);
                        }
                    }
                    Err(error) => {
                        self.model.revert(op);
                        self.last_error = Some(error.to_string());
                    }
                },
                OpOutcome::Trash {
                    op,
                    result,
                    item,
                    node,
                } => match result {
                    Ok(()) => {
                        self.model.confirm(op);
                        if let (Some(stack), Some(item)) = (self.undo.as_mut(), item) {
                            self.trash_record.insert(op.get(), item.clone());
                            stack.record_trash_of(node, item);
                        }
                    }
                    Err(error) => {
                        self.model.revert(op);
                        self.last_error = Some(error.to_string());
                    }
                },
                OpOutcome::EmptyTrash { op, result } => match result {
                    Ok(()) => {
                        self.model.confirm(op);
                    }
                    Err(error) => {
                        self.model.revert(op);
                        self.last_error = Some(error.to_string());
                    }
                },
                OpOutcome::Paste {
                    op,
                    remove_op,
                    node,
                    result,
                } => {
                    self.paste_active = self.paste_active.saturating_sub(1);
                    self.paste_done += 1;
                    match result {
                        Ok(item) => match item.status {
                            PasteStatus::Skipped => {
                                self.paste_skipped += 1;
                                // A skipped item leaves the destination as it
                                // was; undo the provisional row.
                                if let Some(op) = op {
                                    self.model.revert(op);
                                }
                                if let Some(op) = remove_op {
                                    self.model.revert(op);
                                }
                            }
                            status => {
                                if status.is_success() {
                                    self.paste_copied += 1;
                                    if let Some(op) = op {
                                        self.model.retarget(op, &item.destination);
                                        self.model.confirm(op);
                                    }
                                    if let Some(op) = remove_op {
                                        self.model.confirm(op);
                                    }
                                    let moved = item.status == PasteStatus::Moved;
                                    // Resolve the pasted row by its final URI:
                                    // `retarget` may have retargeted the id.
                                    let node = self
                                        .model
                                        .model()
                                        .node_id_for_location(&item.destination)
                                        .unwrap_or(node);
                                    if let Some(stack) = self.undo.as_mut() {
                                        stack.record_paste_of(
                                            self.paste_batch,
                                            node,
                                            item.source.clone(),
                                            item.destination.clone(),
                                            moved,
                                        );
                                    }
                                }
                            }
                        },
                        Err(error) => {
                            self.paste_failed += 1;
                            if let Some(op) = op {
                                self.model.revert(op);
                            }
                            if let Some(op) = remove_op {
                                self.model.revert(op);
                            }
                            self.last_error = Some(error.to_string());
                        }
                    }
                }
                OpOutcome::Replay {
                    stack,
                    redo,
                    result,
                } => {
                    self.replay.active = false;
                    if let Some(stack) = stack {
                        self.undo = Some(stack);
                    }
                    match result {
                        Ok(outcome) => {
                            // The filesystem changed under the rows: apply the
                            // same edit to the session's model so the listing
                            // agrees without a re-list.
                            // Rows are resolved by the recorded locations:
                            // an id recorded at an operation's start may have
                            // been retargeted since (a paste's insert), so the
                            // name is the stable key.
                            if let Some(location) = outcome.renamed.clone() {
                                let current = outcome.renamed_from.as_ref();
                                if let Some(row) = current
                                    .and_then(|was| self.model.model().node_id_for_location(was))
                                    .or_else(|| self.model.model().node_id_for_location(&location))
                                {
                                    self.model.rename_row(row, &location);
                                }
                            }
                            if let Some(location) = outcome.removed.clone() {
                                // The create is gone from the filesystem and
                                // its optimistic record was retired: drop the
                                // row, or the listing keeps painting it.
                                if let Some(row) =
                                    self.model.model().node_id_for_location(&location)
                                {
                                    self.model.model_mut().remove_node(row);
                                }
                            }
                            if let Some(location) = outcome.restored.clone() {
                                // The store records a path, not a kind: the
                                // restored row is a directory when that path
                                // is one now.
                                let kind = if location
                                    .to_file_path()
                                    .map(|path| path.is_dir())
                                    .unwrap_or(false)
                                {
                                    crate::NodeKind::Directory
                                } else {
                                    crate::NodeKind::File
                                };
                                let row = if outcome.node.is_set() {
                                    outcome.node
                                } else {
                                    NodeId::UNSET
                                };
                                self.model.restore_row(row, &location, kind);
                            }
                            self.replay.ok = true;
                            self.replay.refused = false;
                            self.undo_message = Some(outcome.message(redo));
                            // The replayed edit changed rows the receiver
                            // already knows: the next delta must be a reset.
                            self.reported.clear();
                        }
                        Err(error) => {
                            self.replay.ok = false;
                            self.replay.refused = true;
                            let (reason, item) = match &error {
                                UndoError::Empty => (error.reason(), None),
                                UndoError::Conflict(conflict) => {
                                    (conflict.reason.clone(), conflict.item.clone())
                                }
                                UndoError::Failed(failed) => (failed.to_string(), None),
                            };
                            self.undo_message = Some(reason.clone());
                            if item.is_some() {
                                self.last_error = Some(reason);
                            }
                            // A replayed trash changed the store slot, so a
                            // recorded Put Back may be stale.
                            self.trash_record.clear();
                        }
                    }
                }
            }
            // Every fold may push or reverse a step: mirror the labels.
            self.refresh_undo_labels();
        }
    }

    /// Mirror the stacks' next labels, so the ABI hands out stable strings
    /// (T331).
    fn refresh_undo_labels(&mut self) {
        let (undo, redo) = match self.undo.as_ref() {
            Some(stack) => (
                stack.undo_label().map(str::to_owned),
                stack.redo_label().map(str::to_owned),
            ),
            None => (None, None),
        };
        self.undo_label = undo;
        self.redo_label = redo;
    }

    /// The current listing state.
    fn listing_state(&self) -> &ListingState {
        self.model.model().state()
    }
}

/// The session row an operation outcome names, for the op-echo bookkeeping.
fn outcome_node(outcome: &OpOutcome) -> Option<NodeId> {
    match outcome {
        OpOutcome::Rename { node, .. }
        | OpOutcome::NewFolder { node, .. }
        | OpOutcome::Trash { node, .. }
        | OpOutcome::Paste { node, .. } => Some(*node),
        OpOutcome::EmptyTrash { .. } | OpOutcome::Replay { .. } => None,
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

/// Move a row vector into a heap delta. The rows and their strings are then
/// owned by the caller and released with [`df_files_delta_free`].
fn delta_from_rows(
    status: i32,
    kind: i32,
    rows: Vec<df_files_row>,
    total: u32,
) -> *mut df_files_delta {
    let row_count = rows.len() as u32;
    let mut boxed = rows.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    Box::into_raw(Box::new(df_files_delta {
        status,
        error: std::ptr::null(),
        kind,
        rows: ptr,
        row_count,
        changes: std::ptr::null_mut(),
        change_count: 0,
        total,
    }))
}

/// Move a change vector (`CHANGED`) into a heap delta.
fn delta_from_changes(
    status: i32,
    kind: i32,
    changes: Vec<df_files_change>,
    total: u32,
) -> *mut df_files_delta {
    let change_count = changes.len() as u32;
    let mut boxed = changes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    Box::into_raw(Box::new(df_files_delta {
        status,
        error: std::ptr::null(),
        kind,
        rows: std::ptr::null_mut(),
        row_count: 0,
        changes: ptr,
        change_count,
        total,
    }))
}

/// A delta with no rows, for `DONE`/`ERROR`/`TIMEOUT`/`REMOVED`.
fn terminal_delta(status: i32, error: Option<String>) -> *mut df_files_delta {
    let error = error.map(|message| {
        CString::new(message)
            .unwrap_or_else(|_| CString::new("error").expect("static"))
            .into_raw()
    });
    Box::into_raw(Box::new(df_files_delta {
        status,
        error: error.unwrap_or(std::ptr::null_mut()),
        kind: DF_FILES_DELTA_RESET,
        rows: std::ptr::null_mut(),
        row_count: 0,
        changes: std::ptr::null_mut(),
        change_count: 0,
        total: 0,
    }))
}

/// Free the three strings a [`df_files_node`] owns.
unsafe fn free_node_strings(node: &mut df_files_node) {
    for field in [node.name, node.uri, node.symlink_target] {
        if !field.is_null() {
            drop(CString::from_raw(field as *mut c_char));
        }
    }
}

/// Free a node row vector allocated by [`delta_from_rows`].
unsafe fn free_rows(rows: *mut df_files_row, count: usize) {
    if rows.is_null() {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(rows, count);
    for row in slice.iter_mut() {
        free_node_strings(&mut row.node);
    }
    drop(Vec::from_raw_parts(rows, count, count));
}

/// Free a change vector allocated by [`delta_from_changes`].
unsafe fn free_changes(changes: *mut df_files_change, count: usize) {
    if changes.is_null() {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(changes, count);
    for change in slice.iter_mut() {
        free_node_strings(&mut change.node);
    }
    drop(Vec::from_raw_parts(changes, count, count));
}

/// Start listing `uri` and return an opaque session, or null when the URI is
/// not a parseable `scheme://` location. The `trash://` scheme is served by
/// [`TrashSource`] (T-10.6a); every other scheme goes to [`StdFsSource`]. The
/// caller owns the session and must release it with [`df_files_free`].
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
    let source: Arc<dyn DirectorySource> = if location.scheme() == "trash" {
        Arc::new(TrashSource::new())
    } else {
        Arc::new(StdFsSource::new())
    };
    begin_session(location, source, 0)
}

/// Start a session over a synthetic, disk-free listing of `count` files, in
/// batches of `batch` (`0` uses the default). This is the T-10.5 performance
/// fixture: the 100k scroll budget cannot be measured against a real tree
/// without spending minutes creating inodes. The Qt facade selects it only for
/// a `/synthetic` location when `DF_FILES_SYNTHETIC_COUNT` is set.
///
/// # Safety
///
/// `uri` must be a valid NUL-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_begin_synthetic(
    uri: *const c_char,
    count: u32,
    batch: u32,
) -> *mut FfiSession {
    if uri.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(uri) = std::ffi::CStr::from_ptr(uri).to_str() else {
        return std::ptr::null_mut();
    };
    let Ok(location) = Location::parse(uri) else {
        return std::ptr::null_mut();
    };
    begin_session(
        location,
        Arc::new(SyntheticSource::new(count as usize)),
        batch as usize,
    )
}

/// Build the session around a source and its model batch size (`0` = default).
unsafe fn begin_session(
    location: Location,
    source: Arc<dyn DirectorySource>,
    batch: usize,
) -> *mut FfiSession {
    let inner = if batch == 0 {
        DirectoryModel::new()
    } else {
        DirectoryModel::with_batch_size(batch)
    };
    let mut model = OptimisticModel::new(inner);
    let handle = model.begin(source, location.clone());
    // A local folder listing watches itself (T332); the Trash store and the
    // synthetic fixture do not. A folder that cannot be watched is not an
    // error: the listing works and the window simply does not auto-refresh.
    //
    // The watch shares the model's *current* generation, so it must begin
    // after `begin` — a watch opened before it would carry generation 0 and
    // every event it delivered would look retired.
    let generation = model.model().generation();
    let watch = if location.scheme() == "file" && batch == 0 {
        #[cfg(target_os = "linux")]
        {
            let watcher: Arc<dyn FolderWatcher> = Arc::new(crate::InotifyWatcher::new());
            watch::begin_entries(watcher, &location, generation).ok()
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = generation;
            None
        }
    } else {
        let _ = generation;
        None
    };
    let (op_tx, outcome_rx) = spawn_op_worker();
    Box::into_raw(Box::new(FfiSession {
        model,
        handle,
        op_tx,
        outcome_rx,
        pending: 0,
        last_error: None,
        paste_active: 0,
        paste_total: 0,
        paste_done: 0,
        paste_copied: 0,
        paste_skipped: 0,
        paste_failed: 0,
        paste_mode: -1,
        undo: Some(Box::new(UndoStack::new())),
        paste_batch: 0,
        paste_row: None,
        trash_record: std::collections::HashMap::new(),
        replay: ReplayReport::default(),
        undo_message: None,
        undo_label: None,
        redo_label: None,
        reported: HashSet::new(),
        watch_reported: watch.is_some(),
        watch,
        op_rows: std::collections::VecDeque::new(),
        quiet_writes: 0,
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

/// Block up to `timeout_ms` for the next listing event and return an
/// **incremental** delta (T-10.5/T332).
///
/// A completed operation is folded first, which always yields a reset (its
/// outcome may change a node's name/uri or remove a row). A watch burst that
/// finished is folded next: one delta describes every change of the burst
/// (insertions at their final ranks or in-place changes), and a burst that
/// removed a row asks for a reset. A streaming batch yields only the nodes
/// that arrived, at their final ranks, so the receiver's work per batch is
/// proportional to the batch — not the whole listing. `DONE`/`ERROR` carry no
/// rows; a quiet settled session answers `TIMEOUT`. Returns null only for a
/// null session. Free the result with [`df_files_delta_free`].
///
/// The call blocks on a channel receive, so no directory I/O ever runs on the
/// caller's (UI) thread.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`].
#[no_mangle]
pub unsafe extern "C" fn df_files_poll_delta(
    session: *mut FfiSession,
    timeout_ms: u32,
) -> *mut df_files_delta {
    let Some(session) = session.as_mut() else {
        return std::ptr::null_mut();
    };
    if session.drain_ops() {
        return match session.listing_state() {
            ListingState::Failed(error) => {
                terminal_delta(DF_FILES_STATUS_ERROR, Some(error.to_string()))
            }
            _ => session.reset_delta(DF_FILES_STATUS_BATCH),
        };
    }
    if session.drain_watch() {
        return session.incremental_delta();
    }
    if session.watching() && session.listing_state() == &ListingState::Complete {
        // A settled watched session wakes on its own change, never on a timer
        // (T332). The wait only arms the wake: `drain_watch` takes the batch,
        // so the batch is never consumed by the wait itself. A streaming
        // listing keeps its own wait below, so the watch never starves it.
        if !session.wait_for_watch(Duration::from_millis(u64::from(timeout_ms))) {
            return session
                .incremental_delta_if_pending()
                .unwrap_or_else(|| terminal_delta(DF_FILES_STATUS_TIMEOUT, None));
        }
        session.drain_watch();
        return session.incremental_delta();
    }
    match session
        .handle
        .recv_timeout(Duration::from_millis(u64::from(timeout_ms)))
    {
        Some(event) => {
            session.model.apply(event);
            match session.listing_state() {
                ListingState::Failed(error) => {
                    terminal_delta(DF_FILES_STATUS_ERROR, Some(error.to_string()))
                }
                ListingState::Complete => terminal_delta(DF_FILES_STATUS_DONE, None),
                _ => session.incremental_delta(),
            }
        }
        None => {
            // A burst can land between the two waits: describe it rather than
            // answer a quiet timeout.
            if session.drain_watch() {
                return session.incremental_delta();
            }
            match session.listing_state() {
                ListingState::Failed(error) => {
                    terminal_delta(DF_FILES_STATUS_ERROR, Some(error.to_string()))
                }
                _ => session
                    .incremental_delta_if_pending()
                    .unwrap_or_else(|| terminal_delta(DF_FILES_STATUS_TIMEOUT, None)),
            }
        }
    }
}

/// Whether the session watches its folder for external changes (T332).
/// `1` when it does, `0` for the Trash, a synthetic listing, a folder that
/// could not be watched, or a null session. QML reads it to decide whether a
/// settled window may sleep; a watched window wakes on a change.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_watching(session: *const FfiSession) -> i32 {
    match session.as_ref() {
        Some(session) => i32::from(session.watch.is_some()),
        None => 0,
    }
}

/// Return the full ordered model as a reset delta without waiting. Used after
/// a sort change or an optimistic begin to repaint every row. Free with
/// [`df_files_delta_free`].
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`].
#[no_mangle]
pub unsafe extern "C" fn df_files_snapshot_delta(session: *mut FfiSession) -> *mut df_files_delta {
    let Some(session) = session.as_mut() else {
        return std::ptr::null_mut();
    };
    session.reset_delta(DF_FILES_STATUS_BATCH)
}

/// Free a delta (and the row strings it owns). Null is ignored.
///
/// # Safety
///
/// `delta` must be a pointer returned by [`df_files_poll_delta`] /
/// [`df_files_snapshot_delta`] that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn df_files_delta_free(delta: *mut df_files_delta) {
    if delta.is_null() {
        return;
    }
    let delta = Box::from_raw(delta);
    free_rows(delta.rows, delta.row_count as usize);
    free_changes(delta.changes, delta.change_count as usize);
    if !delta.error.is_null() {
        drop(CString::from_raw(delta.error as *mut c_char));
    }
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
    session.queue_op(OpRequest::Rename {
        op,
        location,
        new_name: name,
        node: id,
    });
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
    let node = session
        .model
        .model()
        .nodes()
        .iter()
        .rev()
        .find(|node| {
            node.uri()
                .starts_with(&format!("{}/", parent.uri().trim_end_matches('/')))
        })
        .map(Node::id)
        .unwrap_or(NodeId::UNSET);
    session.queue_op(OpRequest::NewFolder { op, parent, node });
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
    session.queue_op(OpRequest::Trash {
        op,
        location,
        node: id,
    });
    op.get()
}

/// Optimistically clear a `trash://` session (Empty Trash) and queue the real
/// empty on the operations worker. Returns the operation id; `0` when the
/// session is not listing the Trash or the listing is already empty. The rows
/// disappear immediately and snap back if the empty fails.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`].
#[no_mangle]
pub unsafe extern "C" fn df_files_begin_empty_trash(session: *mut FfiSession) -> u64 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let is_trash = session
        .model
        .model()
        .location()
        .map(|location| location.scheme() == "trash")
        .unwrap_or(false);
    if !is_trash || session.model.is_empty() {
        return 0;
    }
    let op = session.model.begin_empty_trash();
    session.queue_op(OpRequest::EmptyTrash { op });
    op.get()
}

/// Paste one clipboard item into `target_uri` (T330). The item's `name` and
/// `kind` come from the shell's clipboard; `mode` is `DF_FILES_CLIPBOARD_*`
/// and `policy` is `DF_FILES_CONFLICT_*`. Returns a non-zero token when the
/// item was queued, `0` for a null session or an unparseable location.
///
/// The optimistic row (a copy into the current folder) is painted before this
/// returns; a cut whose source row is in the current model hides it. The
/// worker's outcome confirms or reverts on the next poll, and the per-item
/// counters behind [`df_files_paste_state`] advance as items finish.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`]; every
/// string must be a valid NUL-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_begin_paste(
    session: *mut FfiSession,
    target_uri: *const c_char,
    source_uri: *const c_char,
    name: *const c_char,
    kind: i32,
    mode: i32,
    policy: i32,
) -> u64 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let Some(target) = read_str(target_uri).and_then(|uri| Location::parse(&uri).ok()) else {
        return 0;
    };
    let Some(source) = read_str(source_uri).and_then(|uri| Location::parse(&uri).ok()) else {
        return 0;
    };
    let Some(name) = read_str(name) else {
        return 0;
    };
    let name = OsString::from(name);
    let mode = match mode {
        DF_FILES_CLIPBOARD_COPY => ClipboardMode::Copy,
        DF_FILES_CLIPBOARD_CUT => ClipboardMode::Cut,
        _ => return 0,
    };
    let policy = match policy {
        DF_FILES_CONFLICT_REPLACE => ConflictPolicy::Replace,
        DF_FILES_CONFLICT_SKIP => ConflictPolicy::Skip,
        DF_FILES_CONFLICT_KEEP_BOTH => ConflictPolicy::KeepBoth,
        _ => return 0,
    };
    let kind = match kind {
        DF_NODE_DIRECTORY => NodeKind::Directory,
        DF_NODE_FILE => NodeKind::File,
        DF_NODE_SYMLINK => NodeKind::Symlink,
        _ => NodeKind::Other,
    };

    // Consecutive per-item calls form one batch. A call after the previous
    // batch settled starts a fresh one.
    if session.paste_active == 0 && session.paste_done == session.paste_total {
        session.paste_total = 0;
        session.paste_done = 0;
        session.paste_copied = 0;
        session.paste_skipped = 0;
        session.paste_failed = 0;
    }
    session.paste_active += 1;
    session.paste_total += 1;
    session.paste_mode = match mode {
        ClipboardMode::Copy => DF_FILES_CLIPBOARD_COPY,
        ClipboardMode::Cut => DF_FILES_CLIPBOARD_CUT,
    };

    let current = session.model.model().location().cloned();
    let target_is_current = current.as_ref() == Some(&target);
    let mut op = None;
    let mut remove_op = None;
    let mut skip = false;

    if mode == ClipboardMode::Cut {
        if source.parent().as_ref() == Some(&target) {
            // Cutting into the folder the item already lives in is a skip.
            skip = true;
        } else if let Some(current) = current.as_ref() {
            if current != &target && source.parent().as_ref() == Some(current) {
                // The source row is in this model; hide it while the move
                // runs so the cut is visible immediately.
                let source_id = session
                    .model
                    .model()
                    .nodes()
                    .iter()
                    .find(|node| node.uri() == source.uri())
                    .map(Node::id);
                if let Some(id) = source_id {
                    remove_op = session.model.begin_delete(id);
                }
            }
        }
    }

    if target_is_current && !skip {
        let existing = session
            .model
            .model()
            .nodes()
            .iter()
            .find(|node| node.name() == name.as_os_str())
            .map(Node::id);
        let mut destination_name = name.clone();
        match policy {
            ConflictPolicy::Skip if existing.is_some() => skip = true,
            ConflictPolicy::KeepBoth if existing.is_some() => {
                let mut taken = |candidate: &std::ffi::OsStr| {
                    session
                        .model
                        .model()
                        .nodes()
                        .iter()
                        .any(|node| node.name() == candidate)
                };
                destination_name = crate::generated_name(&name, &mut taken);
            }
            ConflictPolicy::Replace => {
                if let Some(id) = existing {
                    remove_op = session.model.begin_delete(id);
                }
            }
            _ => {}
        }
        if !skip {
            let uri = target.child(&destination_name).uri().to_owned();
            let node = Node::new(destination_name, uri, kind);
            op = Some(session.model.begin_paste_insert(node));
        }
    }

    // The pasted item's row: the row the paste painted (a copy's insert or a
    // cut source's hidden row). Read the pending record now — the worker's
    // fold retires it.
    let node = op
        .or(remove_op)
        .map(|op| session.model.last_node_of(op))
        .filter(|node| node.is_set())
        .or_else(|| session.model.model().node_id_for_location(&source))
        .or(session.paste_row)
        .unwrap_or(NodeId::UNSET);
    session.paste_row = Some(node);
    session.queue_op(OpRequest::Paste {
        op,
        remove_op,
        source: source.clone(),
        target,
        name,
        mode,
        policy,
        skip,
        node,
    });
    // A copy into the visible folder is a new row (its id names it already);
    // a cut's removal or a skipped item has no row before the worker runs, so
    // its own watch echo is what tells a dormant window to ask again.
    if skip || !target_is_current {
        session.quiet_writes += 1;
    }
    // `1` is a queued token for a paste with no model row to point at.
    op.or(remove_op).map(OpId::get).unwrap_or(1)
}

/// Per-item paste progress (T330). The Qt facade reads this after every poll;
/// `active == 0 && total == done && total > 0` means the batch finished.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_paste_state(
    session: *const FfiSession,
) -> df_files_paste_progress {
    match session.as_ref() {
        Some(session) => df_files_paste_progress {
            active: session.paste_active,
            total: session.paste_total,
            done: session.paste_done,
            copied: session.paste_copied,
            skipped: session.paste_skipped,
            failed: session.paste_failed,
            mode: session.paste_mode,
        },
        None => df_files_paste_progress {
            mode: -1,
            ..df_files_paste_progress::default()
        },
    }
}

/// The C ABI's undo availability and the last replay's report (T331). Every
/// string is NUL-terminated and owned by `session` (copy it before the next
/// call); `message` is the last undo/redo's sentence or its refusal reason,
/// empty when nothing has been replayed.
#[repr(C)]
pub struct df_files_undo_state {
    /// Undoable steps on the stack (bounded by `UNDO_LIMIT`, 20).
    pub undo_len: u32,
    /// Redoable steps.
    pub redo_len: u32,
    /// The next undo's label (`Rename “a.txt”`), or null.
    pub undo_label: *const c_char,
    /// The next redo's label, or null.
    pub redo_label: *const c_char,
    /// Non-zero while an undo/redo is on the worker.
    pub active: i32,
    /// Non-zero when the last replay succeeded.
    pub ok: i32,
    /// Non-zero when the last replay was refused (`message` says why).
    pub refused: i32,
    /// The last sentence or refusal reason, or null.
    pub message: *const c_char,
}

/// The session's undo stacks and last replay (T331). Null is a no-undo state.
///
/// The struct type is also `df_files_undo_state`, so the function cannot
/// share that name in C (the reason [`df_files_paste_state`] queries a
/// `df_files_paste_progress` struct).
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_undo_status(session: *const FfiSession) -> df_files_undo_state {
    let Some(session) = session.as_ref() else {
        return df_files_undo_state {
            undo_len: 0,
            redo_len: 0,
            undo_label: std::ptr::null(),
            redo_label: std::ptr::null(),
            active: 0,
            ok: 0,
            refused: 0,
            message: std::ptr::null(),
        };
    };
    // The label sizes outlive this call: they are held by the session's
    // stacks, so they stay valid until the next replay.
    df_files_undo_state {
        undo_len: session.undo.as_ref().map_or(0, |stack| stack.undo_len()) as u32,
        redo_len: session.undo.as_ref().map_or(0, |stack| stack.redo_len()) as u32,
        undo_label: session
            .undo_label
            .as_ref()
            .and_then(|label| CString::new(label.clone()).ok())
            .map(|label| label.into_raw() as *const c_char)
            .unwrap_or(std::ptr::null()),
        redo_label: session
            .redo_label
            .as_ref()
            .and_then(|label| CString::new(label.clone()).ok())
            .map(|label| label.into_raw() as *const c_char)
            .unwrap_or(std::ptr::null()),
        active: i32::from(session.replay.active),
        ok: i32::from(session.replay.ok),
        refused: i32::from(session.replay.refused),
        message: session
            .undo_message
            .as_ref()
            .and_then(|message| CString::new(message.clone()).ok())
            .map(|message| message.into_raw() as *const c_char)
            .unwrap_or(std::ptr::null()),
    }
}

/// Free the strings [`df_files_undo_status`] returned. Null pointers are
/// ignored.
///
/// # Safety
///
/// `state` must be a struct returned by [`df_files_undo_state`] that has not
/// already been freed.
#[no_mangle]
pub unsafe extern "C" fn df_files_undo_state_free(state: df_files_undo_state) {
    for field in [state.undo_label, state.redo_label, state.message] {
        if !field.is_null() {
            drop(CString::from_raw(field as *mut c_char));
        }
    }
}

/// Undo (`redo == 0`) or redo (`redo != 0`) the top step of the session's
/// undo stack, on the operations worker (T331). Returns 1 when queued, 0 when
/// there is nothing to replay or a replay is already running. Poll until
/// `df_files_pending_ops` is 0 and read [`df_files_undo_state`] for the
/// outcome; a refusal leaves the stack and the filesystem untouched.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_undo(session: *mut FfiSession, redo: i32) -> i32 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    if session.replay.active {
        return 0;
    }
    let redo = redo != 0;
    let has_step = session
        .undo
        .as_ref()
        .map(|stack| {
            if redo {
                stack.redo_len() > 0
            } else {
                stack.undo_len() > 0
            }
        })
        .unwrap_or(false);
    if !has_step {
        // Nothing to replay: report the empty stack rather than a stale
        // message from an earlier replay.
        session.undo_message = Some(UndoError::Empty.reason());
        session.replay = ReplayReport {
            ok: false,
            refused: true,
            active: false,
        };
        return 0;
    }
    let stack = session.undo.take();
    session.undo_label = None;
    session.redo_label = None;
    session.replay = ReplayReport {
        ok: false,
        refused: false,
        active: true,
    };
    session.undo_message = None;
    if session
        .op_tx
        .send(OpRequest::Replay { stack, redo })
        .is_err()
    {
        session.replay.active = false;
        return 0;
    }
    session.pending += 1;
    // A replay's filesystem writes are external-looking to the watcher.
    session.quiet_writes += 1;
    1
}

/// Record a successful Move to Trash the Files shell ran through another path
/// (the Dock's Trash bridge). `node_id` names it for [`df_files_put_back`];
/// the entry is also pushed onto this session's undo stack, so `Cmd+Z` takes
/// it back too. Returns 1 on success, 0 for a null session or a foreign
/// scheme (neither is trashable here).
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null;
/// `uri` must be a valid NUL-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_note_trash(
    session: *mut FfiSession,
    node_id: u64,
    uri: *const c_char,
    path: *const c_char,
) -> i32 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let (Some(uri), Some(path)) = (read_str(uri), read_str(path)) else {
        return 0;
    };
    let Ok(original) = Location::parse(&uri) else {
        return 0;
    };
    // The store item is looked up by its recorded original path (and its
    // stored file, when the shell names one): the store's `.trashinfo` is the
    // truth; the shell only reports where the item was.
    let stored = Location::file(path).to_file_path();
    let inventory = FreedesktopTrash::new().entries().unwrap_or_default();
    let Some(item) = inventory.into_iter().find(|entry| {
        entry.original_location() == original
            && stored
                .as_ref()
                .map(|stored| entry.file_path() == stored)
                .unwrap_or(true)
    }) else {
        return 0;
    };
    session.trash_record.insert(node_id, item.clone());
    if let Some(stack) = session.undo.as_mut() {
        stack.record_trash(item);
    }
    session.refresh_undo_labels();
    1
}

/// Put Back the item [`df_files_note_trash`] recorded under `node_id`. Runs
/// synchronously on the caller's thread (the shell calls it from the action
/// that owns the Trash view; it is one rename). Returns 1 on success, 0 when
/// the id is unknown or the restore was refused; the reason then lands in
/// [`df_files_take_error`].
///
/// A session listing `trash://` names its rows after the store item (the
/// `trash:///<stored name>` URI), so its ids resolve by name instead. The
/// restored row is put back at its recorded original location and removed
/// from the Trash listing.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_put_back(session: *mut FfiSession, node_id: u64) -> i32 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let row = NodeId::from_raw(node_id);
    let recorded = session.trash_record.get(&node_id).cloned();
    let in_trash = session
        .model
        .model()
        .location()
        .map(|location| location.scheme() == "trash")
        .unwrap_or(false);
    let tab_item = if in_trash && recorded.is_none() {
        let row = session.model.model().node(row);
        let name = row.and_then(|row| {
            row.uri().strip_prefix("trash:///").map(|name| {
                // The store name is percent-encoded in the URI; the store's
                // own entries carry the raw bytes.
                crate::location::decode_path(name)
                    .file_name()
                    .map(|name| name.to_os_string())
                    .unwrap_or_else(|| std::ffi::OsString::from(name))
            })
        });
        name.and_then(|name| {
            FreedesktopTrash::new()
                .entries()
                .unwrap_or_default()
                .into_iter()
                .find(|item| item.name() == name)
        })
    } else {
        None
    };
    let Some(item) = recorded.or(tab_item) else {
        session.last_error = Some("Nothing was recorded to put back".to_owned());
        return 0;
    };
    let trash = FreedesktopTrash::new();
    match trash.restore(&item) {
        Ok(location) => {
            session.trash_record.remove(&node_id);
            if in_trash {
                session.model.model_mut().remove_node(row);
                session.reported.clear();
            } else {
                session.model.restore_row(row, &location, NodeKind::File);
            }
            session.refresh_undo_labels();
            1
        }
        Err(error) => {
            session.last_error = Some(error.to_string());
            0
        }
    }
}

/// Wait up to `timeout_ms` for every queued operation outcome and fold it into
/// the model on the caller's thread (T331). An undo/redo calls this right after
/// [`df_files_undo`] so the replayed rows are repainted before the key or menu
/// handler returns; a paste batch normally stays on the poll timer. Returns the
/// number of outcomes folded.
///
/// # Safety
///
/// `session` must be a live pointer returned by [`df_files_begin`] or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_await_ops(session: *mut FfiSession, timeout_ms: u32) -> u32 {
    let Some(session) = session.as_mut() else {
        return 0;
    };
    let deadline = std::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
    let mut folded = 0;
    while session.pending > 0 && std::time::Instant::now() < deadline {
        match session.outcome_rx.recv_timeout(Duration::from_millis(5)) {
            Ok(outcome) => {
                session.fold_one(outcome);
                folded += 1;
            }
            Err(_) => {
                // The worker is still busy (a large directory copy): stop
                // waiting and let the poll timer fold it.
                if std::time::Instant::now() >= deadline {
                    break;
                }
            }
        }
    }
    folded
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
            free_node_strings(node);
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

/// One Trash reading (T-10.6a).
#[repr(C)]
pub struct df_files_trash_state {
    /// Items in the home trash store, or 0 when unavailable.
    pub count: i32,
    /// Non-zero when the store is reachable.
    pub available: i32,
}

/// The opaque Trash monitor the shell's Dock badge reads (T-10.6a).
///
/// It owns a [`TrashMonitor`] over the environment's home trash. The C++ side
/// blocks on [`df_files_trash_monitor_wait`] from a worker thread and reads
/// [`df_files_trash_monitor_state`] on the UI thread; every method takes a
/// shared reference and the monitor is internally synchronized, so those calls
/// may overlap safely.
pub struct FfiTrashMonitor {
    monitor: TrashMonitor,
    last_error: Mutex<Option<String>>,
}

/// Create a started Trash monitor over the home trash; free it with
/// [`df_files_trash_monitor_free`].
#[no_mangle]
pub extern "C" fn df_files_trash_monitor_new() -> *mut FfiTrashMonitor {
    let monitor = TrashMonitor::new();
    monitor.start();
    Box::into_raw(Box::new(FfiTrashMonitor {
        monitor,
        last_error: Mutex::new(None),
    }))
}

/// Retire a Trash monitor, cancelling its watch. Null is ignored.
///
/// # Safety
///
/// `monitor` must be a pointer returned by [`df_files_trash_monitor_new`] that
/// has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn df_files_trash_monitor_free(monitor: *mut FfiTrashMonitor) {
    if !monitor.is_null() {
        drop(Box::from_raw(monitor));
    }
}

/// The monitor's current reading without blocking or re-scanning. Null yields
/// an unavailable, empty reading.
///
/// # Safety
///
/// `monitor` must be a live pointer returned by [`df_files_trash_monitor_new`]
/// or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_trash_monitor_state(
    monitor: *const FfiTrashMonitor,
) -> df_files_trash_state {
    match monitor.as_ref() {
        Some(monitor) => {
            let state: TrashState = monitor.monitor.state();
            df_files_trash_state {
                count: state.count as i32,
                available: i32::from(state.available),
            }
        }
        None => df_files_trash_state {
            count: 0,
            available: 0,
        },
    }
}

/// Block up to `timeout_ms` for a change to the Trash and re-scan. Returns 1
/// when a change was observed (the state is refreshed), 0 on timeout, and -1
/// for a null monitor.
///
/// # Safety
///
/// `monitor` must be a live pointer returned by [`df_files_trash_monitor_new`]
/// or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_trash_monitor_wait(
    monitor: *const FfiTrashMonitor,
    timeout_ms: u32,
) -> i32 {
    match monitor.as_ref() {
        Some(monitor) => i32::from(
            monitor
                .monitor
                .wait(Duration::from_millis(u64::from(timeout_ms))),
        ),
        None => -1,
    }
}

/// Re-scan the store now. Returns 1 when the reading changed, 0 when it did
/// not, and -1 for a null monitor.
///
/// # Safety
///
/// `monitor` must be a live pointer returned by [`df_files_trash_monitor_new`]
/// or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_trash_monitor_refresh(monitor: *const FfiTrashMonitor) -> i32 {
    match monitor.as_ref() {
        Some(monitor) => i32::from(monitor.monitor.refresh()),
        None => -1,
    }
}

/// Move the `file://` item at `uri` into the home trash. Returns 1 on success,
/// 0 on failure (recorded for [`df_files_trash_monitor_take_error`]), and -1
/// for a null monitor or an unparseable URI.
///
/// # Safety
///
/// `monitor` must be a live pointer returned by [`df_files_trash_monitor_new`]
/// or null; `uri` must be a valid NUL-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_trash_monitor_trash(
    monitor: *const FfiTrashMonitor,
    uri: *const c_char,
) -> i32 {
    let Some(monitor) = monitor.as_ref() else {
        return -1;
    };
    let Some(uri) = read_str(uri) else {
        return -1;
    };
    let Ok(location) = Location::parse(&uri) else {
        return -1;
    };
    match monitor.monitor.trash(&location) {
        Ok(_) => 1,
        Err(error) => {
            *monitor
                .last_error
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()) = Some(error.to_string());
            0
        }
    }
}

/// Remove every item from the home trash. Returns the number removed (0 for an
/// already-empty store), or -1 on failure (recorded for
/// [`df_files_trash_monitor_take_error`]) or a null monitor.
///
/// # Safety
///
/// `monitor` must be a live pointer returned by [`df_files_trash_monitor_new`]
/// or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_trash_monitor_empty(monitor: *const FfiTrashMonitor) -> i32 {
    let Some(monitor) = monitor.as_ref() else {
        return -1;
    };
    match monitor.monitor.empty() {
        Ok(removed) => removed as i32,
        Err(error) => {
            *monitor
                .last_error
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()) = Some(error.to_string());
            -1
        }
    }
}

/// Take the most recent Trash operation failure (a NUL-terminated UTF-8 string
/// owned by the caller, or null when there is none) and clear it.
///
/// # Safety
///
/// `monitor` must be a live pointer returned by [`df_files_trash_monitor_new`]
/// or null.
#[no_mangle]
pub unsafe extern "C" fn df_files_trash_monitor_take_error(
    monitor: *const FfiTrashMonitor,
) -> *mut c_char {
    let Some(monitor) = monitor.as_ref() else {
        return std::ptr::null_mut();
    };
    monitor
        .last_error
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
        .map(|message| {
            CString::new(message)
                .unwrap_or_else(|_| CString::new("error").expect("static"))
                .into_raw()
        })
        .unwrap_or(std::ptr::null_mut())
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

    /// Drain a synthetic session with the incremental ABI, returning the ids
    /// in delivered order and the total number of rows the poll sent. A
    /// streaming poll must send each node exactly once (no full snapshots).
    unsafe fn drain_incremental(session: *mut FfiSession) -> (Vec<u64>, u64, u32) {
        let mut ids: Vec<u64> = Vec::new();
        let mut rows_sent: u64 = 0;
        let mut resets = 0u32;
        loop {
            let delta = df_files_poll_delta(session, 2_000);
            assert!(!delta.is_null(), "poll returned null");
            let status = (*delta).status;
            // The listing's `DONE` delta carries no kind of its own: it is
            // not a model change, so it must not count as a reset.
            if status != DF_FILES_STATUS_DONE && (*delta).kind == DF_FILES_DELTA_RESET {
                resets += 1;
                ids.clear();
            }
            // An insertion adds rows; the listing's `DONE` carries none.
            if status != DF_FILES_STATUS_DONE {
                rows_sent += (*delta).row_count as u64;
            }
            for i in 0..(*delta).row_count as usize {
                unsafe {
                    let row: df_files_row = std::ptr::read_unaligned((*delta).rows.add(i));
                    ids.push(row.node.id);
                }
            }
            df_files_delta_free(delta);
            if status == DF_FILES_STATUS_DONE {
                break;
            }
            assert_ne!(status, DF_FILES_STATUS_ERROR, "listing failed");
        }
        (ids, rows_sent, resets)
    }

    #[test]
    fn incremental_poll_sends_each_streamed_node_once() {
        const COUNT: u32 = 5_000;
        unsafe {
            let session = df_files_begin_synthetic(c("file:///synthetic").as_ptr(), COUNT, 256);
            assert!(!session.is_null());
            let (ids, rows_sent, resets) = drain_incremental(session);
            assert_eq!(ids.len(), COUNT as usize);
            assert_eq!(
                rows_sent,
                u64::from(COUNT),
                "the streaming path must not re-send earlier rows"
            );
            assert_eq!(resets, 0, "a plain streaming listing never resets");
            let mut unique = ids.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), COUNT as usize, "ids are unique");
            df_files_free(session);
        }
    }

    #[test]
    fn a_sort_change_delivers_a_reset_delta() {
        unsafe {
            let session = df_files_begin_synthetic(c("file:///synthetic").as_ptr(), 64, 0);
            assert!(!session.is_null());
            let (_, _, _) = drain_incremental(session);
            assert_eq!(
                df_files_set_sort(session, c("size").as_ptr(), c("descending").as_ptr(), 0),
                0
            );
            let delta = df_files_snapshot_delta(session);
            assert!(!delta.is_null());
            assert_eq!(
                (*delta).kind,
                DF_FILES_DELTA_RESET,
                "a sort change repaints every row"
            );
            assert_eq!((*delta).row_count, 64);
            df_files_delta_free(delta);
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

    #[test]
    fn empty_trash_abi_rejects_a_non_trash_session() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), b"alpha").expect("write");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            assert!(!session.is_null());
            drain_listing(session);
            assert_eq!(
                df_files_begin_empty_trash(session),
                0,
                "Empty Trash is only valid on a trash:// listing"
            );
            assert_eq!(df_files_pending_ops(session), 0);
            let snapshot = df_files_snapshot(session);
            assert_eq!(names(snapshot).len(), 1, "the row is untouched");
            df_files_event_free(snapshot);
            df_files_free(session);
        }
    }

    #[test]
    fn paste_abi_paints_the_row_then_confirms_and_reports_progress() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("source")).expect("source");
        std::fs::create_dir(dir.path().join("dest")).expect("dest");
        std::fs::write(dir.path().join("source/report.txt"), b"report").expect("file");
        let dest_uri = Location::file(dir.path().join("dest")).uri().to_owned();
        let source_uri = Location::file(dir.path().join("source/report.txt"))
            .uri()
            .to_owned();
        unsafe {
            let session = df_files_begin(c(&dest_uri).as_ptr());
            assert!(!session.is_null());
            drain_listing(session);

            let token = df_files_begin_paste(
                session,
                c(&dest_uri).as_ptr(),
                c(&source_uri).as_ptr(),
                c("report.txt").as_ptr(),
                DF_NODE_FILE,
                DF_FILES_CLIPBOARD_COPY,
                DF_FILES_CONFLICT_KEEP_BOTH,
            );
            assert_ne!(token, 0);
            // The optimistic row is painted before the worker runs.
            let painted = df_files_snapshot(session);
            assert!(names(painted).iter().any(|name| name == "report.txt"));
            df_files_event_free(painted);

            let final_snapshot = await_ops(session);
            assert!(names(final_snapshot)
                .iter()
                .any(|name| name == "report.txt"));
            df_files_event_free(final_snapshot);
            assert!(dir.path().join("dest/report.txt").exists());
            let progress = df_files_paste_state(session);
            assert_eq!(progress.active, 0);
            assert_eq!(progress.total, 1);
            assert_eq!(progress.done, 1);
            assert_eq!(progress.copied, 1);
            assert_eq!(progress.failed, 0);
            assert_eq!(progress.mode, DF_FILES_CLIPBOARD_COPY);
            df_files_free(session);
        }
    }

    #[test]
    fn paste_abi_keep_both_numbers_a_conflict() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("source")).expect("source");
        std::fs::create_dir(dir.path().join("dest")).expect("dest");
        std::fs::write(dir.path().join("source/report.txt"), b"new").expect("source");
        std::fs::write(dir.path().join("dest/report.txt"), b"old").expect("dest");
        let dest_uri = Location::file(dir.path().join("dest")).uri().to_owned();
        let source_uri = Location::file(dir.path().join("source/report.txt"))
            .uri()
            .to_owned();
        unsafe {
            let session = df_files_begin(c(&dest_uri).as_ptr());
            assert!(!session.is_null());
            drain_listing(session);

            assert_ne!(
                df_files_begin_paste(
                    session,
                    c(&dest_uri).as_ptr(),
                    c(&source_uri).as_ptr(),
                    c("report.txt").as_ptr(),
                    DF_NODE_FILE,
                    DF_FILES_CLIPBOARD_COPY,
                    DF_FILES_CONFLICT_KEEP_BOTH,
                ),
                0
            );
            let final_snapshot = await_ops(session);
            let names = names(final_snapshot);
            df_files_event_free(final_snapshot);
            assert!(names.iter().any(|name| name == "report.txt"));
            assert!(names.iter().any(|name| name == "report.txt 2"));
            assert_eq!(
                std::fs::read(dir.path().join("dest/report.txt")).unwrap(),
                b"old"
            );
            assert_eq!(
                std::fs::read(dir.path().join("dest/report.txt 2")).unwrap(),
                b"new"
            );
            assert_eq!(df_files_paste_state(session).copied, 1);
            df_files_free(session);
        }
    }

    #[test]
    fn paste_abi_skip_leaves_the_destination_and_counts_the_skip() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("source")).expect("source");
        std::fs::create_dir(dir.path().join("dest")).expect("dest");
        std::fs::write(dir.path().join("source/report.txt"), b"new").expect("source");
        std::fs::write(dir.path().join("dest/report.txt"), b"old").expect("dest");
        let dest_uri = Location::file(dir.path().join("dest")).uri().to_owned();
        let source_uri = Location::file(dir.path().join("source/report.txt"))
            .uri()
            .to_owned();
        unsafe {
            let session = df_files_begin(c(&dest_uri).as_ptr());
            assert!(!session.is_null());
            drain_listing(session);

            assert_ne!(
                df_files_begin_paste(
                    session,
                    c(&dest_uri).as_ptr(),
                    c(&source_uri).as_ptr(),
                    c("report.txt").as_ptr(),
                    DF_NODE_FILE,
                    DF_FILES_CLIPBOARD_COPY,
                    DF_FILES_CONFLICT_SKIP,
                ),
                0
            );
            let final_snapshot = await_ops(session);
            assert_eq!(names(final_snapshot), vec!["report.txt".to_owned()]);
            df_files_event_free(final_snapshot);
            assert_eq!(
                std::fs::read(dir.path().join("dest/report.txt")).unwrap(),
                b"old"
            );
            let progress = df_files_paste_state(session);
            assert_eq!(progress.skipped, 1);
            assert_eq!(progress.copied, 0);
            df_files_free(session);
        }
    }

    #[test]
    fn trash_monitor_abi_reads_trashes_and_empties() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("Trash");
        let monitor = TrashMonitor::with_home_trash(&home);
        monitor.start();
        let raw = Box::into_raw(Box::new(FfiTrashMonitor {
            monitor,
            last_error: Mutex::new(None),
        }));
        unsafe {
            let state = df_files_trash_monitor_state(raw);
            assert_eq!(state.count, 0, "a fresh store is empty");
            assert_ne!(state.available, 0, "a writable store is available");

            // A files-core trash operation updates the reading.
            let source = dir.path().join("note.txt");
            std::fs::write(&source, b"hi").expect("source");
            let uri = Location::file(&source).uri().to_owned();
            assert_eq!(df_files_trash_monitor_trash(raw, c(&uri).as_ptr()), 1);
            assert_eq!(df_files_trash_monitor_state(raw).count, 1);
            assert!(!source.exists());

            assert_eq!(df_files_trash_monitor_empty(raw), 1);
            assert_eq!(df_files_trash_monitor_state(raw).count, 0);
            assert!(
                df_files_trash_monitor_take_error(raw).is_null(),
                "a clean op records no error"
            );

            // A bounded wait never fails on a live monitor; queued events from
            // our own operations above may still be drained, so only the sign
            // is asserted here (the wake path is covered in `trash_source`).
            assert!(df_files_trash_monitor_wait(raw, 50) >= 0);
            df_files_trash_monitor_free(raw);
        }
    }

    /// Read the undo state, copying its strings before freeing the struct.
    unsafe fn undo_state(session: *mut FfiSession) -> (u32, u32, String, String, bool, String) {
        let state = df_files_undo_status(session);
        let text = |value: *const c_char| {
            if value.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(value)
                    .to_string_lossy()
                    .into_owned()
            }
        };
        let result = (
            state.undo_len,
            state.redo_len,
            text(state.undo_label),
            text(state.redo_label),
            state.refused != 0,
            text(state.message),
        );
        df_files_undo_state_free(state);
        result
    }

    /// Poll until the queued replay has been folded.
    unsafe fn await_replay(session: *mut FfiSession) -> (u32, u32, String, String, bool, String) {
        for _ in 0..1000 {
            let event = df_files_poll_delta(session, 5);
            df_files_delta_free(event);
            let state = df_files_undo_status(session);
            let settled = state.active == 0 && (state.ok != 0 || state.refused != 0);
            df_files_undo_state_free(state);
            if df_files_pending_ops(session) == 0 && settled {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        undo_state(session)
    }

    #[test]
    fn undo_abi_reverts_a_rename_and_redoes_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), b"alpha").expect("write");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            drain_listing(session);
            assert_eq!(undo_state(session).0, 0, "a fresh session has no history");

            let snapshot = df_files_snapshot(session);
            let id = id_named(snapshot, "alpha.txt");
            df_files_event_free(snapshot);
            assert!(df_files_begin_rename(session, id, c("beta.txt").as_ptr()) > 0);
            let final_event = await_ops(session);
            df_files_event_free(final_event);
            assert!(dir.path().join("beta.txt").exists());

            let (undo_len, redo_len, label, _, refused, _) = undo_state(session);
            assert_eq!(undo_len, 1);
            assert_eq!(redo_len, 0);
            assert_eq!(label, "Rename \"alpha.txt\"");
            assert!(!refused);

            assert_eq!(df_files_undo(session, 0), 1, "the undo is queued");
            let (undo_len, redo_len, _, _, refused, message) = await_replay(session);
            assert_eq!(undo_len, 0);
            assert_eq!(redo_len, 1);
            assert!(!refused);
            assert!(message.starts_with("Undid"), "message: {message}");
            assert!(dir.path().join("alpha.txt").exists());
            assert!(!dir.path().join("beta.txt").exists());

            assert_eq!(df_files_undo(session, 1), 1, "the redo is queued");
            let (undo_len, redo_len, _, _, _, message) = await_replay(session);
            assert_eq!(undo_len, 1);
            assert_eq!(redo_len, 0);
            assert!(message.starts_with("Redid"), "message: {message}");
            assert!(dir.path().join("beta.txt").exists());
            df_files_free(session);
        }
    }

    #[test]
    fn undo_abi_reports_a_conflict_reason_and_keeps_the_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), b"alpha").expect("write");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            drain_listing(session);
            let snapshot = df_files_snapshot(session);
            let id = id_named(snapshot, "alpha.txt");
            df_files_event_free(snapshot);
            assert!(df_files_begin_rename(session, id, c("beta.txt").as_ptr()) > 0);
            let final_event = await_ops(session);
            df_files_event_free(final_event);

            // A newcomer takes the old name: the undo must refuse, not clobber.
            std::fs::write(dir.path().join("alpha.txt"), b"newcomer").expect("squatter");
            assert_eq!(df_files_undo(session, 0), 1);
            let (undo_len, redo_len, _, _, refused, message) = await_replay(session);
            assert!(refused, "a conflict is reported");
            assert_eq!(message, "\"alpha.txt\" already exists");
            assert_eq!(undo_len, 1, "the step stays undoable");
            assert_eq!(redo_len, 0);
            assert_eq!(
                std::fs::read(dir.path().join("alpha.txt")).unwrap(),
                b"newcomer"
            );
            assert_eq!(
                std::fs::read(dir.path().join("beta.txt")).unwrap(),
                b"alpha"
            );
            // The refusal is also the inline notice.
            let error = df_files_take_error(session);
            assert!(!error.is_null());
            df_files_string_free(error);
            df_files_free(session);
        }
    }

    #[test]
    fn undo_abi_enables_an_empty_stack_truthfully() {
        let dir = tempfile::tempdir().expect("tempdir");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            drain_listing(session);
            assert_eq!(df_files_undo(session, 0), 0, "nothing to undo");
            let (_, _, label, _, refused, message) = undo_state(session);
            assert!(label.is_empty());
            assert!(refused);
            assert_eq!(message, UndoError::Empty.reason());
            assert_eq!(df_files_pending_ops(session), 0);
            df_files_free(session);
        }
    }

    /// Serialises the tests that point `XDG_DATA_HOME` at their own trash
    /// store: the environment is process-wide, so two of them setting it at
    /// once would make each read the other's store.
    static TRASH_STORE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn undo_abi_puts_a_trashed_item_back_and_keeps_the_entry() {
        let _store = TRASH_STORE_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");
        // The trash store stays inside the temp dir, never the real one.
        let data = tempfile::tempdir().expect("data home");
        let previous = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", data.path());
        std::fs::write(dir.path().join("note.txt"), b"hi").expect("write");
        let uri = Location::file(dir.path()).uri().to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            drain_listing(session);
            let snapshot = df_files_snapshot(session);
            let id = id_named(snapshot, "note.txt");
            df_files_event_free(snapshot);

            let op = df_files_begin_trash(session, id);
            assert!(op > 0);
            let final_event = await_ops(session);
            df_files_event_free(final_event);
            assert!(!dir.path().join("note.txt").exists());
            assert_eq!(undo_state(session).0, 1);

            assert_eq!(df_files_undo(session, 0), 1);
            let (_, redo_len, _, _, refused, _) = await_replay(session);
            assert!(!refused);
            assert_eq!(redo_len, 1);
            assert_eq!(std::fs::read(dir.path().join("note.txt")).unwrap(), b"hi");
            df_files_free(session);
        }
        match previous {
            Some(value) => std::env::set_var("XDG_DATA_HOME", value),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }

    #[test]
    fn put_back_abi_restores_a_trash_listing_row() {
        let _store = TRASH_STORE_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");
        let data = tempfile::tempdir().expect("data home");
        let previous = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", data.path());
        std::fs::write(dir.path().join("note.txt"), b"hi").expect("write");
        let trash = FreedesktopTrash::new();
        let item = trash
            .trash(&Location::file(dir.path().join("note.txt")))
            .expect("trash the fixture");
        let stored_name = item.name().to_string_lossy().into_owned();
        unsafe {
            let session = df_files_begin(c("trash:///").as_ptr());
            assert!(!session.is_null());
            drain_listing(session);
            let snapshot = df_files_snapshot(session);
            assert_eq!(names(snapshot), vec![stored_name.clone()]);
            let id = id_named(snapshot, &stored_name);
            df_files_event_free(snapshot);
            assert!(id > 0);

            assert_eq!(df_files_put_back(session, id), 1, "Put Back restores it");
            assert_eq!(std::fs::read(dir.path().join("note.txt")).unwrap(), b"hi");
            let snapshot = df_files_snapshot(session);
            assert!(names(snapshot).is_empty(), "the Trash row is gone");
            df_files_event_free(snapshot);

            // A second Put Back of the same row refuses with a reason.
            assert_eq!(df_files_put_back(session, id), 0);
            let error = df_files_take_error(session);
            assert!(!error.is_null(), "a refused Put Back says why");
            df_files_string_free(error);
            df_files_free(session);
        }
        match previous {
            Some(value) => std::env::set_var("XDG_DATA_HOME", value),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }

    #[test]
    fn undo_abi_new_folder_and_paste_records_fold() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("source")).expect("source");
        std::fs::write(dir.path().join("source/report.txt"), b"report").expect("file");
        let uri = Location::file(dir.path()).uri().to_owned();
        let source_uri = Location::file(dir.path().join("source/report.txt"))
            .uri()
            .to_owned();
        unsafe {
            let session = df_files_begin(c(&uri).as_ptr());
            drain_listing(session);

            assert!(df_files_begin_new_folder(session, c(&uri).as_ptr()) > 0);
            let final_event = await_ops(session);
            df_files_event_free(final_event);
            assert_eq!(undo_state(session).2, "New Folder \"untitled folder\"");

            assert_eq!(df_files_undo(session, 0), 1);
            let (undo_len, _, _, _, _, _) = await_replay(session);
            assert_eq!(undo_len, 0);
            assert!(!dir.path().join("untitled folder").exists());

            // Two items of one clipboard paste are one undo step.
            for name in ["report.txt", "report.txt"] {
                assert_ne!(
                    df_files_begin_paste(
                        session,
                        c(&uri).as_ptr(),
                        c(&source_uri).as_ptr(),
                        c(name).as_ptr(),
                        DF_NODE_FILE,
                        DF_FILES_CLIPBOARD_COPY,
                        DF_FILES_CONFLICT_KEEP_BOTH,
                    ),
                    0
                );
            }
            let final_event = await_ops(session);
            df_files_event_free(final_event);
            assert!(dir.path().join("report.txt").exists());
            assert!(dir.path().join("report.txt 2").exists());
            assert_eq!(undo_state(session).0, 1, "one paste, one step");

            assert_eq!(df_files_undo(session, 0), 1);
            await_replay(session);
            assert!(!dir.path().join("report.txt").exists());
            assert!(!dir.path().join("report.txt 2").exists());
            df_files_free(session);
        }
    }
}
