// SPDX-License-Identifier: MIT
//! Temporary frame-trace instrumentation (diagnostic).
//!
//! Enable with `DRAGONFRUIT_FRAME_TRACE=1`. Every line carries two clocks:
//! wall-clock milliseconds since the Unix epoch (`t=`) so it can be
//! correlated with a client's own `Date.now()` logs, and microseconds from
//! this process's monotonic start (`mono_us=`) so events from one compositor
//! run can be ordered deterministically even when the wall clock steps.
//! Compiled in unconditionally but a no-op unless the env var is set, so the
//! hot path pays only a cached `OnceLock` read.
//!
//! The chrome presentation trace (T353) uses this family:
//!
//! * `chrome-commit surface=<ns> gen=<n> buffer=<shm|dmabuf|none>` — the
//!   compositor accepted a surface commit; `gen` is a per-surface monotonic
//!   commit generation.
//! * `frame-start frame=<id> output=<name>` — a frame build began.
//! * `frame-chrome frame=<id> chrome=<ns>:<gen>,...` — the chrome content
//!   snapshot the frame will composite (empty means no above-window chrome).
//! * `frame-callback surface=<ns> frame=<id> presented=<0|1>` — a
//!   `wl_surface.frame` callback, which is a *pacing* event, not proof that
//!   the callback's surface content was presented.
//! * `present frame=<id> output=<name> backend=<drm|nested|headless>
//!   clock=<hardware|estimate|host> chrome=<ns>:<gen>,...` — the final
//!   presentation. Only a `present` line that names the accepted chrome
//!   generation is a native content-presentation endpoint.
//! * `frame-superseded presented=<id> by_epoch=<n>` and
//!   `frame-no-damage output=<name>` label discarded/no-damage outcomes.

use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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

/// Microseconds from this process's monotonic start.
///
/// The origin is process-local, so it orders events from one compositor run
/// but is not comparable with the shell's or a client's monotonic clock.
pub fn now_mono_us() -> u128 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_micros()
}

/// One trace line: `DFTRACE <event> <detail> t=<epoch-ms> mono_us=<us>`.
pub fn log(event: &str, detail: &str) {
    if enabled() {
        eprintln!(
            "DFTRACE {event} {detail} t={} mono_us={}",
            now_ms(),
            now_mono_us()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonic_clock_advances_and_is_process_local() {
        let first = now_mono_us();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = now_mono_us();
        assert!(second >= first);
        assert!(second - first >= 1_000);
    }
}
