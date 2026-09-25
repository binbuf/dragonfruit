// SPDX-License-Identifier: MIT
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

// Start listing `uri`. Returns an opaque session, or null for a URI that is
// not a parseable `scheme://` location. Free with df_files_free.
void *df_files_begin(const char *uri);

// Retire a session and cancel its listing worker. Null is ignored.
void df_files_free(void *session);

// Block up to `timeout_ms` for the next listing event. A BATCH carries the
// model's current ordered snapshot. Free the result with df_files_event_free.
// Returns null only for a null session.
df_files_event *df_files_poll(void *session, uint32_t timeout_ms);

// Return the current ordered snapshot without waiting (used after a sort
// change). Free with df_files_event_free.
df_files_event *df_files_snapshot(void *session);

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