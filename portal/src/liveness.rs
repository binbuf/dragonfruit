// SPDX-License-Identifier: GPL-3.0-only
//! Bounded waits for the interactive portals (T311).
//!
//! The portal is the caller's only path: a presenter that never answers must
//! not hang the application forever. Every FileChooser, Screenshot, and
//! ScreenCast completion is awaited through [`await_bounded`], which resolves
//! to `None` when the deadline wins. The interface then removes its pending
//! entry and answers the standard `cancelled` response — a timeout is a normal
//! outcome, not an error dialog.
//!
//! The timer is `async-io`'s, the same reactor zbus runs the service on, so a
//! bounded wait never needs a second executor or a blocking sleep.

use std::future::Future;
use std::time::Duration;

/// The bound on one interactive portal request.
///
/// Thirty seconds is long enough for a person to answer a dialog and short
/// enough that a lost presenter does not look like a hung application. The
/// presenter-loss watch ([`crate::dbus`]) normally resolves a request much
/// sooner; this deadline covers a presenter that is on the bus but silent.
pub const COMPLETION_TIMEOUT: Duration = Duration::from_secs(30);

/// Await `completion` for at most `timeout`.
///
/// Returns the completion's value when it arrives first, and `None` when the
/// deadline expires (or the completion was already closed). A `None` means the
/// caller must remove its pending entry and answer `cancelled`.
pub async fn await_bounded<F, T>(completion: &mut F, timeout: Duration) -> Option<T>
where
    F: Future<Output = Option<T>> + Unpin,
{
    futures_lite::future::race(completion, async {
        async_io::Timer::after(timeout).await;
        None
    })
    .await
}

/// Poll `future` exactly once without parking.
///
/// After a deadline fires, the registry's removal may race a presenter that
/// completed the request in the same instant. The interface polls the
/// completion once more so that a value delivered before the removal is not
/// thrown away.
pub async fn poll_ready<F>(future: &mut F) -> Option<F::Output>
where
    F: Future + Unpin,
{
    futures_lite::future::poll_once(future).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screenshot;
    use std::time::Instant;

    #[test]
    fn the_default_bound_is_thirty_seconds() {
        assert_eq!(COMPLETION_TIMEOUT, Duration::from_secs(30));
    }

    #[test]
    fn a_completion_that_never_arrives_times_out() {
        let (_completer, mut completion) = screenshot::completion::<u32>();
        let started = Instant::now();
        let outcome = futures_lite::future::block_on(await_bounded(
            &mut completion,
            Duration::from_millis(25),
        ));
        assert_eq!(outcome, None);
        assert!(started.elapsed() >= Duration::from_millis(20));
    }

    #[test]
    fn a_completed_request_wins_before_the_deadline() {
        let (completer, mut completion) = screenshot::completion::<u32>();
        assert!(completer.complete(7));
        let outcome =
            futures_lite::future::block_on(await_bounded(&mut completion, Duration::from_secs(5)));
        assert_eq!(outcome, Some(7));
    }

    #[test]
    fn a_ready_value_can_be_taken_after_a_deadline() {
        let (completer, mut completion) = screenshot::completion::<u32>();
        let timed_out = futures_lite::future::block_on(await_bounded(
            &mut completion,
            Duration::from_millis(10),
        ));
        assert_eq!(timed_out, None);

        // The presenter answered just as the deadline fired; the value is
        // still there for the interface to prefer over a false cancel.
        assert!(completer.complete(9));
        let value = futures_lite::future::block_on(poll_ready(&mut completion));
        assert_eq!(value, Some(Some(9)));
    }
}
