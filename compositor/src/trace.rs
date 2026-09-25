// SPDX-License-Identifier: MIT
//! Temporary frame-trace instrumentation (diagnostic).
//!
//! Enable with `DRAGONFRUIT_FRAME_TRACE=1`. Every line is wall-clock
//! milliseconds since the Unix epoch so it can be correlated with a client's
//! own `Date.now()` logs. Compiled in unconditionally but a no-op unless the
//! env var is set, so the hot path pays only a cached `OnceLock` read.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Whether tracing is enabled (read once).
pub fn enabled() -> bool {
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| std::env::var_os("DRAGONFRUIT_FRAME_TRACE").is_some())
}

/// Wall-clock milliseconds since the Unix epoch.
pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// One trace line: `DFTRACE <event> <detail> t=<epoch-ms>`.
pub fn log(event: &str, detail: &str) {
    if enabled() {
        eprintln!("DFTRACE {event} {detail} t={}", now_ms());
    }
}
