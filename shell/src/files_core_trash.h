// SPDX-License-Identifier: MIT
// The trash slice of the `dragonfruit-files-core` C ABI the shell links
// (T-10.6a). Mirrors the `#[repr(C)]` definitions in
// `services/files-core/src/ffi.rs`; keep the two in lockstep. The Dock reads
// and mutates the one freedesktop Trash store through these functions, so the
// shell and Files cannot disagree about Trash state.
#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// One Trash reading. `available` is non-zero when the store is reachable.
typedef struct df_files_trash_state {
    int32_t count;
    int32_t available;
} df_files_trash_state;

// Create a started Trash monitor over the home trash. Free with
// df_files_trash_monitor_free.
void *df_files_trash_monitor_new(void);

// Retire a Trash monitor, cancelling its watch. Null is ignored.
void df_files_trash_monitor_free(void *monitor);

// The current reading, without blocking or re-scanning.
df_files_trash_state df_files_trash_monitor_state(const void *monitor);

// Block up to `timeout_ms` for a change and re-scan. Returns 1 on change, 0 on
// timeout, -1 for a null monitor.
int32_t df_files_trash_monitor_wait(const void *monitor, uint32_t timeout_ms);

// Re-scan now. Returns 1 when the reading changed, 0 when it did not, -1 for a
// null monitor.
int32_t df_files_trash_monitor_refresh(const void *monitor);

// Move the `file://` item at `uri` into the home trash. Returns 1 on success,
// 0 on failure, -1 for a null monitor or unparseable URI.
int32_t df_files_trash_monitor_trash(const void *monitor, const char *uri);

// Remove every item from the home trash. Returns the number removed, or -1 on
// failure or a null monitor.
int32_t df_files_trash_monitor_empty(const void *monitor);

// Take the most recent operation failure (free with df_files_string_free) and
// clear it. Null when there is none.
char *df_files_trash_monitor_take_error(const void *monitor);

// Free a string returned by df_files_trash_monitor_take_error. Null is ignored.
void df_files_string_free(char *value);

#ifdef __cplusplus
}
#endif