// SPDX-License-Identifier: MIT
//! Per-output presentation clock (T-21.2b, T188a).
//!
//! [`crate::refresh`] answers "how long is one frame on this output?"; this
//! module answers "when is this output next due?". It is pure: timestamps are
//! monotonic [`Duration`]s supplied by the backend, never [`std::time::Instant`]s,
//! and nothing here arms a timer or touches the renderer. T188b consumes the
//! clocks for earliest-deadline scheduling; T188a only records presentations.
//!
//! A clock starts idle: `last_presented` and `next_deadline` are `None`, and
//! the first frame is considered due immediately. [`PresentationClock::note_presented`]
//! records a presented frame and sets `next_deadline = timestamp + period`.
//! A deadline that passes without a presentation stays due
//! ([`PresentationClock::time_until_due`] is zero); the next presentation
//! re-anchors the deadline to the actual presentation time instead of trying
//! to catch up with missed periods. That keeps a slow frame from producing a
//! burst of catch-up frames.
//!
//! VRR is carried from the refresh model and is not interpreted here: one
//! interval is never enough to infer VRR. T188c owns adaptation.
//!
//! ## Timestamp sources
//!
//! * **DRM** — `backend::drm::frame_finish()` passes the vblank's monotonic
//!   `DrmEventTime` when the kernel reports one, falling back to the
//!   compositor clock. It calls [`PresentationClock::note_presented`] only
//!   when `frame_submitted()` confirms a queued frame; an idle vblank is not a
//!   presentation.
//! * **Nested** — Smithay's winit `submit()` only queues the buffer swap and
//!   returns no platform timestamp. The nested backend therefore uses
//!   `state.clock.now()` immediately after a successful `submit()` as the
//!   completion timestamp, and only on the damage path (a no-damage swap
//!   presents no new frame).

use std::collections::BTreeMap;
use std::time::Duration;

use crate::refresh::{OutputRefresh, RefreshSource, VrrState};

/// One output's pure presentation clock.
///
/// The period and refresh come from the same [`OutputRefresh`] that drives the
/// animation clock and the degrade budget, so the three cannot disagree about
/// what one frame costs. The clock owns no timer and no renderer state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresentationClock {
    /// The last presented frame's monotonic timestamp. `None` until the
    /// output presents its first frame.
    pub last_presented: Option<Duration>,
    /// The next deadline, `last_presented + period`. `None` while idle: a
    /// never-presented output has no scheduled deadline, and its first frame
    /// is due immediately.
    pub next_deadline: Option<Duration>,
    /// One frame at the output's current refresh.
    pub period: Duration,
    /// The output's nominal refresh in Hz, for stats/traces.
    pub refresh_hz: f64,
    /// VRR state carried from the refresh model. It never changes `period`
    /// here; T188c decides what a variable interval means.
    pub vrr: VrrState,
}

impl PresentationClock {
    /// Build a clock from the resolved output refresh.
    pub fn new(refresh: OutputRefresh) -> Self {
        Self {
            last_presented: None,
            next_deadline: None,
            period: refresh.period,
            refresh_hz: refresh.refresh_hz(),
            vrr: refresh.vrr,
        }
    }

    /// Build a clock from a bare period, for a synthetic output or a unit
    /// test. A zero or out-of-range period is resolved through the same
    /// 24–500 Hz policy as a mode; zero falls back to 60 Hz, so a bad mode can
    /// never make the clock spin.
    pub fn with_period(period: Duration, vrr: VrrState) -> Self {
        if period.is_zero() {
            return Self::new(OutputRefresh::fallback(vrr));
        }
        let nanos = period.as_nanos().max(1);
        let mhz = (1_000_000_000_000u128 / nanos).min(i32::MAX as u128) as i32;
        Self::new(OutputRefresh::from_mhz(mhz, RefreshSource::Mode, vrr))
    }

    /// Update the period/refresh/VRR after a mode change, preserving the last
    /// presentation and rebasing the next deadline onto the new period. An
    /// idle clock stays idle.
    pub fn set_refresh(&mut self, refresh: OutputRefresh) {
        self.period = refresh.period;
        self.refresh_hz = refresh.refresh_hz();
        self.vrr = refresh.vrr;
        self.next_deadline = self
            .last_presented
            .map(|last| last.saturating_add(self.period));
    }

    /// Record a presented frame at `timestamp` (monotonic). Returns `false`
    /// and leaves the clock untouched for a stale timestamp older than the
    /// last presentation, so an out-of-order completion cannot move the clock
    /// backwards.
    ///
    /// The next deadline is anchored to the actual presentation, not to the
    /// previous deadline: a late frame does not trigger a catch-up burst.
    pub fn note_presented(&mut self, timestamp: Duration) -> bool {
        if let Some(last) = self.last_presented {
            if timestamp < last {
                return false;
            }
        }
        self.last_presented = Some(timestamp);
        self.next_deadline = Some(timestamp.saturating_add(self.period));
        true
    }

    /// Whether this output has ever presented a frame. An idle clock produces
    /// no presentation samples until [`Self::note_presented`] is called.
    pub fn is_idle(&self) -> bool {
        self.last_presented.is_none()
    }

    /// Whether the output's next frame is due at `now`. A clock that has never
    /// presented has no deadline and is due immediately (the first frame); a
    /// missed deadline also stays due until the next presentation re-anchors
    /// it.
    ///
    /// This is a timing predicate, not a render request: an idle output must
    /// not be woken just because its first-frame state is "due". T188b gates
    /// this on its per-output request set.
    pub fn is_due(&self, now: Duration) -> bool {
        self.next_deadline.map_or(true, |deadline| now >= deadline)
    }

    /// Time from `now` until the next deadline, saturating at zero when the
    /// output is already due or has never presented.
    pub fn time_until_due(&self, now: Duration) -> Duration {
        self.next_deadline
            .map(|deadline| deadline.saturating_sub(now))
            .unwrap_or(Duration::ZERO)
    }
}

/// The per-output presentation clocks, keyed by output name.
///
/// `BTreeMap` keeps iteration and tie-breaking deterministic, matching
/// [`crate::refresh::RefreshSchedule`]. T188b arms one timer from the earliest
/// due deadline in this map. Only outputs with an active frame request are
/// candidates: an idle output is never woken by another output's animation,
/// and a never-presented output's first frame is due only once it is
/// requested.
#[derive(Debug, Clone, Default)]
pub struct PresentationClocks {
    outputs: BTreeMap<String, PresentationClock>,
}

impl PresentationClocks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update one output's clock from its resolved refresh. An
    /// existing clock keeps its last presentation and rebases its deadline.
    /// Returns the updated clock.
    pub fn update(&mut self, output: &str, refresh: OutputRefresh) -> &mut PresentationClock {
        let clock = self
            .outputs
            .entry(output.to_owned())
            .or_insert_with(|| PresentationClock::new(refresh));
        clock.set_refresh(refresh);
        clock
    }

    /// Remove an output (hotplug/unplug). Returns its last clock.
    pub fn remove(&mut self, output: &str) -> Option<PresentationClock> {
        self.outputs.remove(output)
    }

    /// Drop every output clock.
    pub fn clear(&mut self) {
        self.outputs.clear();
    }

    /// One output's clock.
    pub fn get(&self, output: &str) -> Option<&PresentationClock> {
        self.outputs.get(output)
    }

    /// One output's clock, mutably.
    pub fn get_mut(&mut self, output: &str) -> Option<&mut PresentationClock> {
        self.outputs.get_mut(output)
    }

    /// Record a presented frame for `output`. Returns `false` when the output
    /// has no clock (for example a presentation arriving after hotplug
    /// removal) or the timestamp was stale.
    pub fn note_presented(&mut self, output: &str, timestamp: Duration) -> bool {
        self.outputs
            .get_mut(output)
            .is_some_and(|clock| clock.note_presented(timestamp))
    }

    /// Iterate outputs in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PresentationClock)> {
        self.outputs
            .iter()
            .map(|(name, clock)| (name.as_str(), clock))
    }

    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refresh::{ModeRefresh, FALLBACK_PERIOD};

    fn refresh(mhz: i32) -> OutputRefresh {
        OutputRefresh::from_mhz(mhz, RefreshSource::Mode, VrrState::Unknown)
    }

    #[test]
    fn a_clock_uses_the_resolved_refresh_and_starts_idle() {
        let refresh = OutputRefresh::from_mhz(143_999, RefreshSource::Mode, VrrState::On);
        let clock = PresentationClock::new(refresh);

        assert_eq!(clock.period, Duration::from_nanos(6_944_492));
        assert!((clock.refresh_hz - 143.999).abs() < 1e-9);
        assert_eq!(clock.vrr, VrrState::On);
        assert!(clock.is_idle());
        assert_eq!(clock.last_presented, None);
        assert_eq!(clock.next_deadline, None);
    }

    #[test]
    fn due_and_not_due_are_relative_to_the_next_deadline() {
        let mut clock = PresentationClock::new(refresh(60_000));
        let period = clock.period;

        // The first frame is due immediately; there is no deadline yet.
        assert!(clock.is_due(Duration::ZERO));
        assert_eq!(clock.time_until_due(Duration::ZERO), Duration::ZERO);

        let presented = Duration::from_millis(1_000);
        assert!(clock.note_presented(presented));
        assert_eq!(clock.last_presented, Some(presented));
        assert_eq!(clock.next_deadline, Some(presented + period));

        assert!(!clock.is_due(presented), "just presented, not due again");
        assert_eq!(clock.time_until_due(presented), period);
        assert!(!clock.is_due(presented + period - Duration::from_nanos(1)));
        assert_eq!(
            clock.time_until_due(presented + period - Duration::from_nanos(1)),
            Duration::from_nanos(1)
        );
        assert!(clock.is_due(presented + period));
        assert_eq!(
            clock.time_until_due(presented + period),
            Duration::ZERO,
            "a reached deadline is due, not negative"
        );
    }

    #[test]
    fn a_missed_deadline_stays_due_and_reanchors_on_presentation() {
        let mut clock = PresentationClock::new(refresh(60_000));
        let period = clock.period;
        assert!(clock.note_presented(Duration::ZERO));

        // Three periods late: the clock is still due and reports no negative
        // time. It does not queue three catch-up frames.
        let late = period * 3 + Duration::from_millis(5);
        assert!(clock.is_due(late));
        assert_eq!(clock.time_until_due(late), Duration::ZERO);

        assert!(clock.note_presented(late));
        assert_eq!(clock.last_presented, Some(late));
        assert_eq!(clock.next_deadline, Some(late + period));
        assert!(!clock.is_due(late), "the late frame re-anchored the clock");
        assert_eq!(clock.time_until_due(late), period);
    }

    #[test]
    fn a_stale_timestamp_cannot_move_the_clock_backwards() {
        let mut clock = PresentationClock::new(refresh(60_000));
        let period = clock.period;
        let presented = Duration::from_millis(100);
        assert!(clock.note_presented(presented));

        assert!(
            !clock.note_presented(presented - Duration::from_millis(1)),
            "an older completion must be ignored"
        );
        assert_eq!(clock.last_presented, Some(presented));
        assert_eq!(clock.next_deadline, Some(presented + period));
    }

    #[test]
    fn sixty_and_144_hz_outputs_keep_their_own_deadlines() {
        let mut clocks = PresentationClocks::new();
        clocks.update("HDMI-A-1", refresh(60_000));
        clocks.update("eDP-1", refresh(144_000));

        let slow_period = clocks.get("HDMI-A-1").expect("slow output").period;
        let fast_period = clocks.get("eDP-1").expect("fast output").period;
        assert_eq!(slow_period, Duration::from_nanos(16_666_666));
        assert_eq!(fast_period, Duration::from_nanos(6_944_444));
        assert!(fast_period < slow_period);

        assert!(clocks.note_presented("HDMI-A-1", Duration::ZERO));
        assert!(clocks.note_presented("eDP-1", Duration::ZERO));

        // At one 144 Hz period only the fast output is due.
        assert!(clocks.get("eDP-1").unwrap().is_due(fast_period));
        assert!(!clocks.get("HDMI-A-1").unwrap().is_due(fast_period));
        assert_eq!(
            clocks.get("HDMI-A-1").unwrap().time_until_due(fast_period),
            slow_period - fast_period
        );

        // At one 60 Hz period both are due; the fast output has missed
        // deadlines but remains due rather than being skipped.
        assert!(clocks.get("HDMI-A-1").unwrap().is_due(slow_period));
        assert!(clocks.get("eDP-1").unwrap().is_due(slow_period));
    }

    #[test]
    fn an_idle_output_produces_no_samples_until_it_presents() {
        let mut clocks = PresentationClocks::new();
        clocks.update("idle", refresh(60_000));
        let clock = clocks.get("idle").expect("idle output");
        let period = clock.period;

        assert!(clock.is_idle());
        assert_eq!(clock.last_presented, None);
        assert_eq!(clock.next_deadline, None);
        assert!(
            clock.is_due(Duration::from_secs(10)),
            "a never-presented output is due for its first frame"
        );
        assert_eq!(
            clock.time_until_due(Duration::from_secs(10)),
            Duration::ZERO
        );

        // Until a presentation is recorded, no amount of elapsed time makes
        // the clock less idle or creates a deadline.
        assert!(!clocks.note_presented("missing", Duration::ZERO));
        assert!(clocks.note_presented("idle", Duration::from_secs(5)));
        let clock = clocks.get("idle").expect("idle output");
        assert!(!clock.is_idle());
        assert!(!clock.is_due(Duration::from_secs(5)));
        assert_eq!(
            clock.time_until_due(Duration::from_secs(5)),
            period,
            "an idle output that just presented is not due until its period"
        );
    }

    #[test]
    fn invalid_periods_fall_back_or_clamp_instead_of_spinning() {
        // A zero period is a bad mode: 60 Hz, never a zero-length deadline.
        let zero = PresentationClock::with_period(Duration::ZERO, VrrState::On);
        assert_eq!(zero.period, FALLBACK_PERIOD);
        assert!((zero.refresh_hz - 60.0).abs() < 1e-9);
        assert_eq!(zero.vrr, VrrState::On);
        assert!(zero.period > Duration::ZERO);

        // An out-of-range period is clamped by the same 24–500 Hz policy as a
        // mode rather than being accepted verbatim.
        let slow = PresentationClock::with_period(Duration::from_secs(1), VrrState::Unknown);
        assert_eq!(slow.period, Duration::from_nanos(41_666_666));
        assert!((slow.refresh_hz - 24.0).abs() < 1e-9);

        let fast = PresentationClock::with_period(Duration::from_nanos(1), VrrState::Unknown);
        assert_eq!(fast.period, Duration::from_nanos(2_000_000));
        assert!((fast.refresh_hz - 500.0).abs() < 1e-9);
    }

    #[test]
    fn a_mode_change_keeps_the_last_presentation_and_rebases_the_deadline() {
        let mut clock = PresentationClock::new(refresh(60_000));
        let presented = Duration::from_millis(100);
        assert!(clock.note_presented(presented));

        let fast = OutputRefresh::from_mhz(144_000, RefreshSource::Mode, VrrState::Off);
        clock.set_refresh(fast);
        assert_eq!(clock.period, fast.period);
        assert!((clock.refresh_hz - 144.0).abs() < 1e-9);
        assert_eq!(clock.vrr, VrrState::Off);
        assert_eq!(clock.last_presented, Some(presented));
        assert_eq!(clock.next_deadline, Some(presented + fast.period));

        // An idle clock stays idle across a mode change.
        let mut idle = PresentationClock::new(refresh(60_000));
        idle.set_refresh(fast);
        assert!(idle.is_idle());
        assert_eq!(idle.next_deadline, None);
    }

    #[test]
    fn the_clock_map_updates_and_removes_outputs() {
        let mut clocks = PresentationClocks::new();
        assert!(clocks.is_empty());
        assert_eq!(clocks.len(), 0);

        clocks.update("eDP-1", refresh(60_000));
        clocks.update("DP-1", refresh(144_000));
        assert_eq!(clocks.len(), 2);
        assert_eq!(clocks.iter().count(), 2);

        // A mode change replaces the period without losing the presentation.
        assert!(clocks.note_presented("eDP-1", Duration::from_millis(10)));
        clocks.update("eDP-1", refresh(144_000));
        let edp = clocks.get("eDP-1").expect("eDP-1");
        assert_eq!(edp.period, Duration::from_nanos(6_944_444));
        assert_eq!(edp.last_presented, Some(Duration::from_millis(10)));

        let removed = clocks.remove("DP-1").expect("DP-1 was present");
        assert_eq!(removed.period, Duration::from_nanos(6_944_444));
        assert!(clocks.remove("DP-1").is_none(), "remove is idempotent");
        assert_eq!(clocks.len(), 1);

        clocks.clear();
        assert!(clocks.is_empty());
        assert!(!clocks.note_presented("eDP-1", Duration::ZERO));
    }

    #[test]
    fn a_schedule_mode_maps_to_the_same_period_as_the_refresh_model() {
        // The clock must not re-derive a different period from a schedule
        // entry; it consumes `OutputRefresh` verbatim.
        let mut schedule = crate::refresh::RefreshSchedule::new();
        let resolved = schedule.update("eDP-1", Some(ModeRefresh::new(143_999)));
        let clock = PresentationClock::new(resolved);
        assert_eq!(clock.period, resolved.period);
        assert_eq!(clock.period, Duration::from_nanos(6_944_492));
        assert_eq!(clock.vrr, resolved.vrr);
    }
}
