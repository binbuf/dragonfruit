// SPDX-License-Identifier: GPL-3.0-only
// The C ABI of `dragonfruit-files-core` (T-10.4b, ADR 0049), mirrored by
// `services/files-core/src/ffi.rs`. This header is the *only* place the Qt app
// touches the Rust library: `FilesDirectoryModel` is a thin
// `QAbstractListModel` over these functions, and no business logic crosses the
// seam.
//
// Keep the struct layout and the status/kind constants in lockstep with the
// `#[repr(C)]` definitions in `ffi.rs`.
#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Event status codes.
#define DF_FILES_STATUS_BATCH 0
#define DF_FILES_STATUS_DONE 1
#define DF_FILES_STATUS_ERROR 2
#define DF_FILES_STATUS_TIMEOUT 3

// Delta kinds (T332): what an incremental delta carries.
#define DF_FILES_DELTA_RESET 0    // rows replace the whole model, in order
#define DF_FILES_DELTA_CHANGED 1  // changes: held rows replaced in place
#define DF_FILES_DELTA_INSERT 2   // rows: new rows at their final ranks
#define DF_FILES_DELTA_REMOVED 3  // rows are gone; repaint from a reset

// Node kinds.
#define DF_NODE_DIRECTORY 0
#define DF_NODE_FILE 1
#define DF_NODE_SYMLINK 2
#define DF_NODE_OTHER 3

// Clipboard modes (T330).
#define DF_FILES_CLIPBOARD_COPY 0
#define DF_FILES_CLIPBOARD_CUT 1

// Conflict policies for a paste (T330).
#define DF_FILES_CONFLICT_REPLACE 0
#define DF_FILES_CONFLICT_SKIP 1
#define DF_FILES_CONFLICT_KEEP_BOTH 2

// One listed item. All strings are NUL-terminated and owned by the event that
// carries them; null means absent.
typedef struct df_files_node {
    uint64_t id;
    const char *name;  // lossy UTF-8 display name
    const char *uri;
    int32_t kind;      // DF_NODE_*
    int32_t has_size;
    uint64_t size;
    int32_t has_modified;
    int64_t modified_ms;  // Unix milliseconds
    const char *symlink_target;
} df_files_node;

// One event from df_files_poll / df_files_snapshot.
typedef struct df_files_event {
    int32_t status;  // DF_FILES_STATUS_*
    const char *error;
    df_files_node *nodes;
    uint32_t count;
} df_files_event;

// One ordered row in an incremental delta (T-10.5): a node and its final rank
// in the ordered projection.
typedef struct df_files_row {
    uint32_t rank;
    df_files_node node;
} df_files_row;

// One in-place row change (T332): the node whose fields changed and its rank
// in the ordered projection after the change. The id is unchanged.
typedef struct df_files_change {
    uint32_t rank;
    df_files_node node;
} df_files_change;

// An incremental event (T-10.5/T332). `kind` says what it carries:
// DF_FILES_DELTA_RESET replaces the whole model with `rows`, in order;
// DF_FILES_DELTA_INSERT inserts `rows` at their final ranks (the existing rows
// keep their relative order); DF_FILES_DELTA_CHANGED replaces the held rows
// named by `changes`; DF_FILES_DELTA_REMOVED means rows vanished and the
// caller repaints from a reset. `total` is the model length after the event.
// All memory is owned by the delta.
typedef struct df_files_delta {
    int32_t status;  // DF_FILES_STATUS_*
    const char *error;
    int32_t kind;  // DF_FILES_DELTA_*
    df_files_row *rows;
    uint32_t row_count;
    df_files_change *changes;
    uint32_t change_count;
    uint32_t total;
} df_files_delta;

// Start listing `uri`. Returns an opaque session, or null for a URI that is
// not a parseable `scheme://` location. Free with df_files_free.
void *df_files_begin(const char *uri);

// Start a session over `count` synthetic files (T-10.5 perf fixture), in
// batches of `batch` (0 = default). Free with df_files_free.
void *df_files_begin_synthetic(const char *uri, uint32_t count, uint32_t batch);

// Retire a session and cancel its listing worker. Null is ignored.
void df_files_free(void *session);

// Block up to `timeout_ms` for the next listing event. A BATCH carries the
// model's current ordered snapshot. Free the result with df_files_event_free.
// Returns null only for a null session.
df_files_event *df_files_poll(void *session, uint32_t timeout_ms);

// Return the current ordered snapshot without waiting (used after a sort
// change). Free with df_files_event_free.
df_files_event *df_files_snapshot(void *session);

// Block up to `timeout_ms` for the next listing event, as an incremental
// delta: streaming batches carry only the nodes that arrived, at their final
// ranks. A completed operation is folded first and always resets. Free with
// df_files_delta_free. Returns null only for a null session.
df_files_delta *df_files_poll_delta(void *session, uint32_t timeout_ms);

// Return the full ordered model as a reset delta without waiting (used after a
// sort change or an optimistic begin). Free with df_files_delta_free.
df_files_delta *df_files_snapshot_delta(void *session);

// Free a delta (and the row strings it owns). Null is ignored.
void df_files_delta_free(df_files_delta *delta);

// Whether the session watches its folder for external changes (T332): 1 when
// it does, 0 for the Trash, a synthetic listing, an unwatchable folder or a
// null session. A watched settled window may sleep, knowing a change will wake
// its poll; an unwatched one keeps its own timer policy.
int32_t df_files_watching(const void *session);

// Set the sort order in place. Returns 0 on success, -1 on a null session or
// an unknown key/direction.
int32_t df_files_set_sort(void *session, const char *key, const char *direction,
                          int32_t folders_first);

// Optimistically rename `node_id` to `new_name` and queue the real rename on
// the operations worker. Returns the operation id, or 0 when the node is
// unknown or `new_name` is null. The model is already repainted when this
// returns; the outcome is folded by the next df_files_poll.
uint64_t df_files_begin_rename(void *session, uint64_t node_id, const char *new_name);

// Optimistically insert the next `untitled folder` under `parent_uri` and
// queue the real creation. Returns the operation id, or 0 for a null session
// or an unparseable parent.
uint64_t df_files_begin_new_folder(void *session, const char *parent_uri);

// Optimistically remove `node_id` (Move to Trash) and queue the real trash
// operation. Returns the operation id, or 0 when the node is unknown.
uint64_t df_files_begin_trash(void *session, uint64_t node_id);

// Optimistically clear a `trash://` listing (Empty Trash) and queue the real
// empty. Returns the operation id, or 0 when the session is not listing the
// Trash or the listing is already empty.
uint64_t df_files_begin_empty_trash(void *session);

// Paste one clipboard item into `target_uri` (T330). `kind` is DF_NODE_*,
// `mode` is DF_FILES_CLIPBOARD_*, and `policy` is DF_FILES_CONFLICT_*.
// Returns a non-zero token when queued, 0 for a null session or a bad URI.
// A copy into the current folder paints its row before returning; a cut whose
// source row is in the current model hides it. The worker's outcome confirms
// or reverts on the next poll.
uint64_t df_files_begin_paste(void *session, const char *target_uri,
                              const char *source_uri, const char *name,
                              int32_t kind, int32_t mode, int32_t policy);

// Per-item paste progress (T330). `active == 0 && total == done && total > 0`
// means the batch finished. `mode` is DF_FILES_CLIPBOARD_*, or -1 before the
// first paste.
typedef struct df_files_paste_progress {
    uint32_t active;
    uint32_t total;
    uint32_t done;
    uint32_t copied;
    uint32_t skipped;
    uint32_t failed;
    int32_t mode;
} df_files_paste_progress;

df_files_paste_progress df_files_paste_state(const void *session);

// Undo (redo == 0) or redo (redo != 0) the top step of the session's undo
// stack, on the operations worker (T331). Returns 1 when queued, 0 when there
// is nothing to replay or a replay is already running. Poll until
// df_files_pending_ops is 0, then read df_files_undo_state.
int32_t df_files_undo(void *session, int32_t redo);

// The undo stack's availability and the last replay's sentence or refusal.
// Every string is NUL-terminated and owned by `session`; copy it (or pass the
// struct to df_files_undo_state_free) before the next session call.
//
// The function is `df_files_undo_status` because C cannot give the struct
// typedef and a function the same name (the same reason
// `df_files_paste_state` queries `df_files_paste_progress`).
typedef struct df_files_undo_state {
    uint32_t undo_len;
    uint32_t redo_len;
    const char *undo_label;  // next undo's label, or null
    const char *redo_label;  // next redo's label, or null
    int32_t active;          // non-zero while a replay is on the worker
    int32_t ok;              // non-zero when the last replay succeeded
    int32_t refused;         // non-zero when the last replay was refused
    const char *message;     // last sentence or refusal reason, or null
} df_files_undo_state;

df_files_undo_state df_files_undo_status(const void *session);

// Free the strings a df_files_undo_state returned. Null pointers are ignored.
void df_files_undo_state_free(df_files_undo_state state);

// Record a Move to Trash the shell ran through another path (the Dock's Trash
// bridge): `uri` is the item's old URI, `path` its store path. Returns 1 when
// recorded, 0 otherwise.
int32_t df_files_note_trash(void *session, uint64_t node_id, const char *uri,
                            const char *path);

// Put Back the item df_files_note_trash recorded under `node_id`. Synchronous;
// returns 1 on success, 0 on refusal (df_files_take_error says why).
int32_t df_files_put_back(void *session, uint64_t node_id);

// Wait up to `timeout_ms` for every queued operation outcome and fold it into
// the model on the calling thread (T331). An undo/redo calls this right after
// df_files_undo so the replayed rows are painted before the handler returns;
// a slow replay keeps the poll timer. Returns the outcomes folded.
uint32_t df_files_await_ops(void *session, uint32_t timeout_ms);

// How many operations are still awaiting their worker outcome. The facade
// keeps polling while this is non-zero.
uint32_t df_files_pending_ops(const void *session);

// Take the most recent operation failure message (caller frees with
// df_files_string_free) and clear it. Null when there is none.
char *df_files_take_error(void *session);

// Free a string returned by df_files_take_error. Null is ignored.
void df_files_string_free(char *value);

// Free an event (and the strings/nodes it owns). Null is ignored.
void df_files_event_free(df_files_event *event);

#ifdef __cplusplus
}
#endif