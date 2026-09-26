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

#ifdef __cplusplus
}
#endif