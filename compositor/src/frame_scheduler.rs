// SPDX-License-Identifier: MIT
//! Per-output frame requests and earliest-deadline scheduling (T-21.2b, T188b).
//!
//! [`crate::presentation`] answers "when is this output next due?"; this module
//! answers "which outputs actually want a frame, and why?". The two request
//! kinds are deliberately separate:
//!
//! * **urgent** — input, a client/chrome commit, a resize, or a global scene
//!   change. Urgent work is presented immediately and never waits for a
//!   deadline. A global scene change may mark every output urgent.
//! * **animation** — a compositor transition is live on that output. An
//!   animation request is only presented when the output's own presentation
//!   clock says its deadline is due, so a 144 Hz tick cannot force a 60 Hz
//!   output to present every 6.94 ms.
//!
//! The scheduler is pure: it owns request bits and the in-flight marker, not a
//! timer or a renderer. [`Self::next_interval`] is the earliest-deadline
//! arming input the calloop timer consumes; [`Self::should_present`] is the
//! per-output render gate. An output that is not requested is never woken by
//! another output's animation, and a settled output has no request left to
//! arm a timer.
//!
//! ## No busy loop
//!
//! A never-presented clock reports its first frame as due immediately, so its
//! interval is zero. A clock that has presented and is *still* due means the
//! previous attempt did not present (for example a no-damage frame); returning
//! zero there would spin the event loop, so the scheduler returns that
//! output's period instead. The next real presentation re-anchors the clock
//! through [`crate::presentation::PresentationClock::note_presented`].
//!
//! An output with a frame already queued on the DRM rail is marked in flight
//! and is excluded from both the render gate and the timer until
//! `frame_finish()` clears it. That is what keeps `queue_frame()` at exactly
//! one call per output-frame instead of queueing an empty retry that fights
//! the next deadline.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::presentation::PresentationClocks;

/// One output's request bits. Both are independent: an urgent frame may also
/// belong to a live animation, and clearing the urgent bit must not stop the
/// animation request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Requests {
    urgent: bool,
    animation: bool,
}

/// The per-output frame request set and in-flight markers.
#[derive(Debug, Clone, Default)]
pub struct FrameScheduler {
    outputs: BTreeMap<String, Requests>,
    in_flight: BTreeSet<String>,
}

impl FrameScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    fn entry(&mut self, output: &str) -> &mut Requests {
        self.outputs.entry(output.to_owned()).or_default()
    }

    /// Request an immediate frame for `output` (input, commit, resize, or a
    /// global scene change routed per output).
    pub fn request_urgent(&mut self, output: &str) {
        self.entry(output).urgent = true;
    }

    /// Request immediate frames for every named output. This is the "global
    /// scene change" form; the caller supplies the live output set so the
    /// scheduler never invents an output name.
    pub fn request_all_urgent<'a>(&mut self, outputs: impl IntoIterator<Item = &'a str>) {
        for output in outputs {
            self.request_urgent(output);
        }
    }

    /// Request an animation frame for `output`. The request stays until the
    /// animation settles (or the output presents its settled frame) and is
    /// gated by the output's own presentation deadline.
    pub fn request_animation(&mut self, output: &str) {
        self.entry(output).animation = true;
    }

    /// Clear the urgent bit after the output has been handed to the renderer.
    pub fn clear_urgent(&mut self, output: &str) {
        if let Some(requests) = self.outputs.get_mut(output) {
            requests.urgent = false;
        }
    }

    /// Clear the animation bit for an output whose transition settled and
    /// whose final frame has been presented.
    pub fn clear_animation(&mut self, output: &str) {
        if let Some(requests) = self.outputs.get_mut(output) {
            requests.animation = false;
        }
    }

    /// Clear both request bits (output removed or suspended).
    pub fn clear(&mut self, output: &str) {
        self.outputs.remove(output);
        self.in_flight.remove(output);
    }

    /// Keep only animation requests whose output is still animating. Urgent
    /// bits are untouched. Used on every animation tick so an output whose
    /// motion finished is not kept on the clock just because another output
    /// is still live.
    pub fn retain_animation<'a>(&mut self, keep: impl IntoIterator<Item = &'a str>) {
        let keep: BTreeSet<&str> = keep.into_iter().collect();
        for (output, requests) in &mut self.outputs {
            if !keep.contains(output.as_str()) {
                requests.animation = false;
            }
        }
    }

    /// Mark that a frame for `output` is queued on the backend and has not
    /// reached its presentation boundary yet.
    pub fn mark_in_flight(&mut self, output: &str) {
        self.in_flight.insert(output.to_owned());
    }

    /// Clear the in-flight marker when the queued frame completes (or the
    /// backend rejected it and will retry).
    pub fn clear_in_flight(&mut self, output: &str) {
        self.in_flight.remove(output);
    }

    pub fn is_urgent(&self, output: &str) -> bool {
        self.outputs
            .get(output)
            .is_some_and(|requests| requests.urgent)
    }

    pub fn is_animating(&self, output: &str) -> bool {
        self.outputs
            .get(output)
            .is_some_and(|requests| requests.animation)
    }

    pub fn is_in_flight(&self, output: &str) -> bool {
        self.in_flight.contains(output)
    }

    /// Whether any output has a live animation request (including one whose
    /// animation has settled but whose final frame is still owed).
    pub fn has_animation(&self) -> bool {
        self.outputs.values().any(|requests| requests.animation)
    }

    /// Outputs with an animation request, in name order.
    pub fn animation_outputs(&self) -> impl Iterator<Item = &str> {
        self.outputs
            .iter()
            .filter(|(_, requests)| requests.animation)
            .map(|(output, _)| output.as_str())
    }

    /// Whether `output` should be handed to the renderer now.
    ///
    /// Urgent work always wins. Animation work needs an animation request, no
    /// frame already in flight, and a due presentation clock. An output with
    /// no clock is treated as due for its first frame (a synthetic output or a
    /// clock installed after the request).
    pub fn should_present(&self, output: &str, now: Duration, clocks: &PresentationClocks) -> bool {
        let Some(requests) = self.outputs.get(output) else {
            return false;
        };
        if requests.urgent {
            return true;
        }
        if !requests.animation || self.in_flight.contains(output) {
            return false;
        }
        clocks.get(output).map_or(true, |clock| clock.is_due(now))
    }

    /// The earliest interval to arm the animation timer with, or `None` when
    /// no output is animating.
    ///
    /// The interval is the minimum `time_until_due` over requested outputs
    /// that are not in flight. A never-presented clock yields zero (first
    /// frame now). A clock that has presented and is still due yields its
    /// period instead of zero so a no-damage frame cannot busy-loop the event
    /// loop; the next presentation re-anchors the real deadline.
    pub fn next_interval(&self, now: Duration, clocks: &PresentationClocks) -> Option<Duration> {
        let mut earliest: Option<Duration> = None;
        for output in self.animation_outputs() {
            if self.in_flight.contains(output) {
                continue;
            }
            let interval = match clocks.get(output) {
                None => Duration::ZERO,
                Some(clock) if clock.is_idle() => Duration::ZERO,
                Some(clock) => {
                    let due = clock.time_until_due(now);
                    if due.is_zero() {
                        clock.period
                    } else {
                        due
                    }
                }
            };
            earliest = Some(earliest.map_or(interval, |current| current.min(interval)));
        }
        earliest
    }

    /// The earliest period among animation-requested outputs, used only as a
    /// diagnostic/timer floor. `None` when no output is animating.
    pub fn animation_period(&self, clocks: &PresentationClocks) -> Option<Duration> {
        self.animation_outputs()
            .filter_map(|output| clocks.get(output).map(|clock| clock.period))
            .min()
    }

    pub fn urgent_count(&self) -> usize {
        self.outputs
            .values()
            .filter(|requests| requests.urgent)
            .count()
    }

    pub fn animation_count(&self) -> usize {
        self.outputs
            .values()
            .filter(|requests| requests.animation)
            .count()
    }

    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::PresentationClocks;
    use crate::refresh::{OutputRefresh, RefreshSource, VrrState};

    fn refresh(mhz: i32) -> OutputRefresh {
        OutputRefresh::from_mhz(mhz, RefreshSource::Mode, VrrState::Unknown)
    }

    fn clocks() -> PresentationClocks {
        let mut clocks = PresentationClocks::new();
        clocks.update("slow", refresh(60_000));
        clocks.update("fast", refresh(144_000));
        clocks.update("idle", refresh(60_000));
        clocks
    }

    #[test]
    fn urgent_is_immediate_and_independent_of_the_animation_deadline() {
        let clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        let now = Duration::from_millis(1_000);
        scheduler.request_animation("slow");
        scheduler.request_urgent("slow");

        assert!(scheduler.should_present("slow", now, &clocks));
        assert!(scheduler.is_animating("slow"));

        scheduler.clear_urgent("slow");
        // Just presented, so the animation deadline is a full period away.
        let mut clocks = clocks;
        assert!(clocks.note_presented("slow", now));
        assert!(!scheduler.should_present("slow", now, &clocks));
        assert!(scheduler.should_present("slow", now + Duration::from_nanos(16_666_666), &clocks));
    }

    #[test]
    fn sixty_and_144_hz_outputs_present_at_their_own_periods() {
        let mut clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        scheduler.request_animation("slow");
        scheduler.request_animation("fast");

        let start = Duration::from_secs(1);
        assert!(clocks.note_presented("slow", start));
        assert!(clocks.note_presented("fast", start));

        let slow_period = clocks.get("slow").unwrap().period;
        let fast_period = clocks.get("fast").unwrap().period;
        assert_eq!(slow_period, Duration::from_nanos(16_666_666));
        assert_eq!(fast_period, Duration::from_nanos(6_944_444));

        // At one fast period only the 144 Hz output is due.
        let fast_tick = start + fast_period;
        assert!(scheduler.should_present("fast", fast_tick, &clocks));
        assert!(!scheduler.should_present("slow", fast_tick, &clocks));
        // The fast output presents at that tick; the next timer still arms
        // from the fast output's next deadline, while the slow output is not
        // due yet. The tick must not make the slow output present.
        assert!(clocks.note_presented("fast", fast_tick));
        assert_eq!(
            scheduler.next_interval(fast_tick, &clocks),
            Some(fast_period),
            "the timer arms from the fast output's next deadline"
        );
        assert!(!scheduler.should_present("slow", fast_tick + fast_period, &clocks));

        // A 144 Hz timer tick must not force the 60 Hz output to present.
        let mut slow_frames = 0;
        let mut fast_frames = 0;
        let mut now = start;
        for _ in 0..6 {
            now += fast_period;
            if scheduler.should_present("fast", now, &clocks) {
                fast_frames += 1;
                assert!(clocks.note_presented("fast", now));
            }
            if scheduler.should_present("slow", now, &clocks) {
                slow_frames += 1;
                assert!(clocks.note_presented("slow", now));
            }
        }
        assert_eq!(
            fast_frames, 5,
            "the fast output keeps its 144 Hz cadence after the first tick"
        );
        assert_eq!(
            slow_frames, 2,
            "the 60 Hz output presents twice in six 144 Hz periods, not six times"
        );
    }

    #[test]
    fn an_idle_output_is_not_woken_by_another_outputs_animation() {
        let clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        scheduler.request_animation("fast");
        scheduler.request_urgent("fast");

        // The idle clock is never presented and therefore reports due, but it
        // has no request and must not be a timer candidate.
        assert!(clocks.get("idle").unwrap().is_due(Duration::from_secs(10)));
        assert!(!scheduler.should_present("idle", Duration::from_secs(10), &clocks));
        assert_eq!(scheduler.animation_count(), 1);
        assert_eq!(
            scheduler.next_interval(Duration::from_secs(10), &clocks),
            Some(Duration::ZERO),
            "only the requested fast output's first frame is a candidate"
        );
    }

    #[test]
    fn the_timer_uses_the_earliest_due_deadline_and_skips_in_flight_frames() {
        let mut clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        scheduler.request_animation("slow");
        scheduler.request_animation("fast");

        let start = Duration::from_secs(2);
        assert!(clocks.note_presented("slow", start));
        assert!(clocks.note_presented("fast", start));
        let slow_period = clocks.get("slow").unwrap().period;
        let fast_period = clocks.get("fast").unwrap().period;

        assert_eq!(scheduler.next_interval(start, &clocks), Some(fast_period));

        // A queued fast frame is not re-queued; the timer waits for the slow
        // output instead of firing on the in-flight output's old deadline.
        scheduler.mark_in_flight("fast");
        assert!(!scheduler.should_present("fast", start + fast_period, &clocks));
        assert_eq!(
            scheduler.next_interval(start + fast_period, &clocks),
            Some(slow_period - fast_period)
        );

        // The completion clears the marker and re-anchors the fast clock; the
        // next timer is again the fast output's period.
        scheduler.clear_in_flight("fast");
        let presented = start + fast_period;
        assert!(clocks.note_presented("fast", presented));
        assert_eq!(
            scheduler.next_interval(presented, &clocks),
            Some(fast_period)
        );
    }

    #[test]
    fn a_settled_output_has_no_request_and_no_timer() {
        let clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        scheduler.request_animation("slow");
        scheduler.request_urgent("slow");
        scheduler.clear_urgent("slow");
        scheduler.clear_animation("slow");

        assert!(!scheduler.should_present("slow", Duration::from_secs(10), &clocks));
        assert!(!scheduler.has_animation());
        assert_eq!(
            scheduler.next_interval(Duration::from_secs(10), &clocks),
            None
        );
        assert_eq!(scheduler.animation_count(), 0);
        assert_eq!(scheduler.urgent_count(), 0);
    }

    #[test]
    fn a_global_scene_change_marks_every_output_urgent() {
        let clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        scheduler.request_all_urgent(["slow", "fast", "idle"]);

        assert_eq!(scheduler.urgent_count(), 3);
        let now = Duration::from_secs(5);
        for output in ["slow", "fast", "idle"] {
            assert!(
                scheduler.should_present(output, now, &clocks),
                "{output} must present an urgent frame immediately"
            );
        }
    }

    #[test]
    fn a_due_clock_without_a_presentation_uses_its_period_not_zero() {
        let mut clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        scheduler.request_animation("slow");

        // The first frame is due immediately: zero is a legitimate arming
        // interval, not a busy loop.
        assert_eq!(
            scheduler.next_interval(Duration::ZERO, &clocks),
            Some(Duration::ZERO)
        );

        // A presented clock that is already due again (a no-damage attempt)
        // returns its period so the event loop blocks instead of spinning.
        let start = Duration::from_secs(1);
        assert!(clocks.note_presented("slow", start));
        let overdue = start + Duration::from_secs(1);
        assert_eq!(
            scheduler.next_interval(overdue, &clocks),
            Some(Duration::from_nanos(16_666_666))
        );
    }

    #[test]
    fn mixed_refresh_60_and_144_with_one_idle_and_one_animating() {
        // The T188c mixed-refresh hook: a 60 Hz output and a 144 Hz output
        // both animate, while a third 60 Hz output stays idle. The scheduler
        // must present each animating output at its own period and never wake
        // the idle output. The simulation advances in 100 us steps, so the
        // counts are the real cadence, not a single-interval inference.
        let mut clocks = PresentationClocks::new();
        clocks.update("slow-60", refresh(60_000));
        clocks.update("fast-144", refresh(144_000));
        clocks.update("idle-60", refresh(60_000));

        let mut scheduler = FrameScheduler::new();
        scheduler.request_animation("slow-60");
        scheduler.request_animation("fast-144");
        // No request for idle-60: it must never be a timer or render candidate.

        let start = Duration::from_secs(1);
        assert!(clocks.note_presented("slow-60", start));
        assert!(clocks.note_presented("fast-144", start));

        let mut now = start;
        let mut slow_frames = 0;
        let mut fast_frames = 0;
        let mut idle_frames = 0;
        for _ in 0..10_000 {
            now += Duration::from_micros(100);
            if scheduler.should_present("slow-60", now, &clocks) {
                slow_frames += 1;
                assert!(clocks.note_presented("slow-60", now));
            }
            if scheduler.should_present("fast-144", now, &clocks) {
                fast_frames += 1;
                assert!(clocks.note_presented("fast-144", now));
            }
            if scheduler.should_present("idle-60", now, &clocks) {
                idle_frames += 1;
                assert!(clocks.note_presented("idle-60", now));
            }
        }

        assert!(
            (58..=60).contains(&slow_frames),
            "the 60 Hz output presents about 60 times in one second, got {slow_frames}"
        );
        assert!(
            (140..=144).contains(&fast_frames),
            "the 144 Hz output presents about 144 times in one second, got {fast_frames}"
        );
        assert_eq!(
            idle_frames, 0,
            "an output with no animation request is never presented"
        );
        assert_eq!(clocks.get("idle-60").unwrap().presentations, 0);

        // Settle the fast output: only the slow one keeps animating. The fast
        // output must not be presented again, and the idle output still stays
        // untouched.
        scheduler.clear_animation("fast-144");
        let fast_before = fast_frames;
        let slow_before = slow_frames;
        for _ in 0..10_000 {
            now += Duration::from_micros(100);
            if scheduler.should_present("slow-60", now, &clocks) {
                slow_frames += 1;
                assert!(clocks.note_presented("slow-60", now));
            }
            assert!(!scheduler.should_present("fast-144", now, &clocks));
            assert!(!scheduler.should_present("idle-60", now, &clocks));
        }
        assert!(
            (58..=60).contains(&(slow_frames - slow_before)),
            "the remaining 60 Hz animation keeps its cadence"
        );
        assert_eq!(
            fast_frames, fast_before,
            "a settled 144 Hz output is not woken by the 60 Hz animation"
        );
    }

    #[test]
    fn retaining_animation_drops_outputs_that_settled() {
        let clocks = clocks();
        let mut scheduler = FrameScheduler::new();
        scheduler.request_animation("slow");
        scheduler.request_animation("fast");
        scheduler.request_urgent("slow");

        scheduler.retain_animation(["fast"]);
        assert!(!scheduler.is_animating("slow"));
        assert!(scheduler.is_urgent("slow"), "urgent is a separate request");
        assert!(scheduler.is_animating("fast"));
        assert_eq!(scheduler.animation_count(), 1);
        assert_eq!(
            scheduler.next_interval(Duration::ZERO, &clocks),
            Some(Duration::ZERO),
            "the retained fast output is still the only timer candidate"
        );
    }
}
