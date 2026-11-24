// SPDX-License-Identifier: MIT
// The listing slice of the `dragonfruit-files-core` C ABI the shell links
// (T-13.2b). Mirrors the `#[repr(C)]` definitions in
// `services/files-core/src/ffi.rs`; keep the two in lockstep. The file-chooser
// picker browses through these functions so the shell, Files windows, and the
// portal's `ListDirectory` all share the one browsing implementation.
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

// One listed item. All strings are NUL-terminated and owned by the event that
// carries them; null means absent.
typedef struct df_files_node {
    uint64_t id;
    const char *name;  // lossy UTF-8 display name
    const char *uri;
    int32_t kind;  // DF_NODE_*
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

// One ordered row in an incremental delta: a node and its final rank in the
// ordered projection.
typedef struct df_files_row {
    uint32_t rank;
    df_files_node node;
} df_files_row;

// One in-place row change (T332): the node whose fields changed and its rank
// after the change. The id is unchanged.
typedef struct df_files_change {
    uint32_t rank;
    df_files_node node;
} df_files_change;

// An incremental event. `kind` says what it carries (DF_FILES_DELTA_*):
// RESET replaces the whole model with `rows`, in order; INSERT adds `rows` at
// their final ranks (the existing rows keep their relative order); CHANGED
// replaces the held rows named by `changes`; REMOVED means rows vanished and
// the caller repaints from a reset. `total` is the model length after the
// event.
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

// Retire a session and cancel its listing worker. Null is ignored.
void df_files_free(void *session);

// Block up to `timeout_ms` for the next listing event. A BATCH carries the
// model's current ordered snapshot. Free the result with df_files_event_free.
// Returns null only for a null session.
df_files_event *df_files_poll(void *session, uint32_t timeout_ms);

// Free an event (and the strings/nodes it owns). Null is ignored.
void df_files_event_free(df_files_event *event);

// Block up to `timeout_ms` for the next listing event as an incremental
// delta. A zero timeout is a non-blocking poll: the picker drains ready
// deltas from a Qt timer so a large folder never freezes the shell. A settled
// watched session wakes on its folder's next change (T332).
df_files_delta *df_files_poll_delta(void *session, uint32_t timeout_ms);

// Free a delta (and the row strings it owns). Null is ignored.
void df_files_delta_free(df_files_delta *delta);

// Whether the session watches its folder for external changes (T332): 1 when
// it does, 0 for the Trash, a synthetic listing or a null session. A watched
// picker keeps its blocking poll armed so a folder change repaints it.
int32_t df_files_watching(const void *session);

// Optimistically insert the next `untitled folder` under `parent_uri` and
// queue the real creation. Returns the operation id, or 0 for a null session
// or an unparseable parent.
uint64_t df_files_begin_new_folder(void *session, const char *parent_uri);

// How many operations are still awaiting their worker outcome.
uint32_t df_files_pending_ops(const void *session);

// Take the most recent operation failure message (caller frees with
// df_files_string_free) and clear it. Null when there is none.
char *df_files_take_error(void *session);

// Free a string returned by df_files_take_error. Null is ignored.
void df_files_string_free(char *value);

#ifdef __cplusplus
}
#endif