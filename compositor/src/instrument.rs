// SPDX-License-Identifier: MIT
//! Performance instruments for the T-03 real-session budgets (T-03.1b).
//!
//! Two instruments share the render-stats line, appended **after** the
//! T-03.1a `client_wakeups` field so the positional trace contract is never
//! broken ([ADR 0009](../../docs/design/adr/0009-idle-trace-instrumentation.md)):
//!
//! * [`LatencyInstrument`] measures the **input-to-photon** latency: the delay
//!   between an input event reaching [`crate::input::process_input_event`] and
//!   the next compositor frame that is actually presented. `note_input` stamps
//!   the first un-credited input; `note_present` samples the delta on every
//!   presented frame (nested `render::post_repaint`, headless
//!   `render::post_repaint_headless`, DRM `backend::drm::render_surface`) and
//!   clears it. Nested is the live instrument; headless drives the exact same
//!   code path so CI can assert a sample is emitted.
//!
//! * [`ScanoutCounter`] is the **direct-scanout counter template**: a read-only
//!   snapshot of the scanout-relevant [`RenderStats`] counters. The DRM rail
//!   (`backend/drm.rs`) already increments `direct_scanouts` when
//!   `DrmCompositor` commits a primary-plane client buffer; the hardware run
//!   (T-03.2/T-03.4) snapshots this template before/after a condition to prove
//!   the counter advances for an unobstructed fullscreen client and stays flat
//!   for a window that still needs compositing. It owns no state and reads
//!   only `DfState::stats`.
//!
//! * [`GestureBudgetTrace`] (T-05.6) is the **gesture-scoped** frame-budget
//!   instrument: while an overview transition (Mission Control, a Space slide,
//!   or Desktop Reveal) is live, every rendered frame's cadence is recorded so
//!   the full gesture's hold on the 60 Hz budget can be asserted — or its
//!   shortfall reported honestly. It is the T-03 `RenderStats` frame trace
//!   scoped to the gesture instead of the whole session, and it lives on
//!   `DfState` (not inside `RenderStats`) because `DfState` owns the overview
//!   state that scopes it.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::state::RenderStats;

/// Input-to-photon latency accumulator (T-03.1b).
///
/// One shared accumulator lives on [`RenderStats`]. Latency is measured from
/// the **earliest** input since the last presented frame (the worst case for a
/// batch) to the frame that presents it; an input older than
/// [`Self::STALE_LIMIT`] when a frame finally presents is discarded, not
/// credited, because it cannot have caused that frame. This keeps an idle
/// pointer move (which produces no damage) from polluting a later,
/// unrelated redraw.
#[derive(Debug, Default)]
pub struct LatencyInstrument {
    /// Earliest input not yet credited to a presented frame.
    pending_input: Option<Instant>,
    /// Most recent sample (microseconds).
    last_us: u32,
    /// Largest sample seen since startup (microseconds).
    max_us: u32,
    /// Samples credited to a presented frame.
    samples: u64,
    /// Inputs dropped because no frame presented before the stale limit.
    dropped: u64,
    /// Bounded ring of recent samples, newest last.
    recent: VecDeque<u32>,
}

impl LatencyInstrument {
    /// Recent samples kept for the trace.
    pub const RECENT_CAPACITY: usize = 512;

    /// Inputs older than this when a frame presents are discarded rather than
    /// credited. One frame at 15 Hz is ~67 ms; 250 ms is several frames of
    /// slack for a slow attach, but far below any human-scale idle gap.
    pub const STALE_LIMIT: Duration = Duration::from_millis(250);

    /// Stamp an input event. Only the earliest input since the last presented
    /// frame is kept, so a batch reports its worst (oldest) leg.
    pub fn note_input(&mut self, at: Instant) {
        if self.pending_input.is_none() {
            self.pending_input = Some(at);
        }
    }

    /// Credit the pending input to a presented frame. A stale pending input
    /// (see [`Self::STALE_LIMIT`]) is dropped; either way the pending input is
    /// cleared so the next frame starts a fresh sample.
    pub fn note_present(&mut self, now: Instant) {
        let Some(start) = self.pending_input.take() else {
            return;
        };
        let elapsed = now.saturating_duration_since(start);
        if elapsed > Self::STALE_LIMIT {
            self.dropped += 1;
            return;
        }
        let us = elapsed.as_micros().min(u128::from(u32::MAX)) as u32;
        self.last_us = us;
        self.max_us = self.max_us.max(us);
        self.samples += 1;
        if self.recent.len() == Self::RECENT_CAPACITY {
            self.recent.pop_front();
        }
        self.recent.push_back(us);
    }

    /// Most recent input-to-present latency in microseconds (`0` before the
    /// first sample).
    pub fn last_us(&self) -> u32 {
        self.last_us
    }

    /// Largest input-to-present latency seen, in microseconds.
    pub fn max_us(&self) -> u32 {
        self.max_us
    }

    /// Samples credited to a presented frame.
    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// Inputs dropped as stale before a frame presented.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Whether the most recent sample is within one frame at `refresh_hz`.
    pub fn within_one_frame(&self, refresh_hz: u32) -> bool {
        if self.samples == 0 || refresh_hz == 0 {
            return false;
        }
        let frame_us = 1_000_000u64 / u64::from(refresh_hz);
        u64::from(self.last_us) <= frame_us
    }

    /// One-line summary for the `dump_stats` trace.
    pub fn summary(&self) -> String {
        format!(
            "last_us={} max_us={} samples={} dropped={}",
            self.last_us, self.max_us, self.samples, self.dropped
        )
    }
}

/// Direct-scanout counter template for the DRM rail (T-03.1b).
///
/// Reads only the shared [`RenderStats`] counters. `backend::drm` advances
/// `direct_scanouts` when `DrmCompositor` returns a primary-plane client buffer
/// ([`smithay::backend::drm::compositor::PrimaryPlaneElement`]); this type lets
/// T-03.2/T-03.4 take a before/after snapshot around the expected condition
/// (an unobstructed fullscreen client, no compositor chrome over it) and assert
/// [`Self::engaged_since`] is true — and false when an overlay/chrome forces a
/// composited frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScanoutCounter {
    /// Frames pushed through the direct-scanout path (DRM only).
    pub direct_scanouts: u64,
    /// Frames actually rendered (or scanout-composited).
    pub frames_rendered: u64,
    /// Renders skipped because the damage tracker found no damage.
    pub frames_skipped_no_damage: u64,
}

impl ScanoutCounter {
    /// Snapshot the scanout-relevant counters from the shared render stats.
    pub fn read(stats: &RenderStats) -> Self {
        Self {
            direct_scanouts: stats.direct_scanouts,
            frames_rendered: stats.frames_rendered,
            frames_skipped_no_damage: stats.frames_skipped_no_damage,
        }
    }

    /// Frames the compositor considered: rendered plus no-damage skips.
    pub fn frames_considered(&self) -> u64 {
        self.frames_rendered + self.frames_skipped_no_damage
    }

    /// Direct scanouts that engaged between `earlier` and `self` (saturating).
    #[allow(dead_code)] // T-03.2/T-03.4 snapshot the template on hardware.
    pub fn advanced_since(&self, earlier: &Self) -> u64 {
        self.direct_scanouts.saturating_sub(earlier.direct_scanouts)
    }

    /// Whether at least one direct scanout engaged in that window.
    #[allow(dead_code)] // T-03.2/T-03.4 snapshot the template on hardware.
    pub fn engaged_since(&self, earlier: &Self) -> bool {
        self.advanced_since(earlier) > 0
    }

    /// One-line report for the trace.
    pub fn report(&self) -> String {
        format!(
            "direct_scanouts={} frames_rendered={} frames_skipped_no_damage={} considered={}",
            self.direct_scanouts,
            self.frames_rendered,
            self.frames_skipped_no_damage,
            self.frames_considered()
        )
    }
}

/// Gesture-scoped frame-budget trace (T-05.6).
///
/// The overview gesture is the material track's named frame-budget risk
/// (blur and scale over many windows on an iGPU). The session already records
/// every rendered frame in [`RenderStats`]; this instrument narrows that to
/// the frames the compositor renders **while an overview transition/overview
/// state is live**, so the full gesture can be judged against one 60 Hz frame
/// without the idle session polluting the numbers.
///
/// A frame is over budget when its render **duration** exceeds `budget_us`;
/// a presentation is dropped when the **interval** since the previous recorded
/// frame exceeds `DROPPED_RATIO * budget_us`. The two are kept distinct so a
/// slow-but-caught-up frame is not a dropped presentation and a jittery
/// cadence is not a slow frame.
///
/// The trace auto-begins on the first recorded active frame and auto-finishes
/// when the gesture settles, so the caller only has to say "the overview is
/// active this frame". It is deliberately pure timing plus counters (no
/// clock), so it is unit-testable and inert while idle.
#[derive(Debug)]
pub struct GestureBudgetTrace {
    tracing: bool,
    frames: u64,
    over_budget_frames: u64,
    dropped_frames: u64,
    max_frame_us: u32,
    max_interval_ms: u32,
    total_frame_us: u64,
    last_at: Option<Instant>,
    budget_us: u32,
}

impl Default for GestureBudgetTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // The getters are the instrument API; T-06/T-16 read them.
impl GestureBudgetTrace {
    /// A dropped presentation is an interval past this multiple of the budget:
    /// more than half a frame late means a vsync was missed. The extra slack
    /// keeps normal timer/scheduler jitter from being reported as a shortfall.
    pub const DROPPED_RATIO: f64 = 1.5;

    /// The default budget is the shared animation frame interval (16 ms), the
    /// same knob the T-04.4a degrade controller selects against.
    pub fn new() -> Self {
        Self::with_budget_us(crate::animation::FRAME_INTERVAL.as_micros() as u32)
    }

    /// A trace against an explicit budget in microseconds.
    pub fn with_budget_us(budget_us: u32) -> Self {
        Self {
            tracing: false,
            frames: 0,
            over_budget_frames: 0,
            dropped_frames: 0,
            max_frame_us: 0,
            max_interval_ms: 0,
            total_frame_us: 0,
            last_at: None,
            budget_us: budget_us.max(1),
        }
    }

    /// Start (or restart) a gesture trace, clearing the counters.
    pub fn begin(&mut self, now: Instant) {
        self.tracing = true;
        self.frames = 0;
        self.over_budget_frames = 0;
        self.dropped_frames = 0;
        self.max_frame_us = 0;
        self.max_interval_ms = 0;
        self.total_frame_us = 0;
        self.last_at = Some(now);
    }

    /// Record one rendered frame while the overview is live. The first frame
    /// after a [`Self::begin`] has no interval and only anchors the cadence.
    ///
    /// A frame is over budget when its **render duration** exceeds the budget;
    /// a presentation is dropped when the **interval** since the previous
    /// recorded frame exceeds [`Self::DROPPED_RATIO`] budgets. The two are
    /// kept distinct because a slow-but-caught-up frame is not a dropped
    /// presentation, and a jittery cadence is not a slow frame.
    pub fn record(&mut self, now: Instant, duration: Duration) {
        if !self.tracing {
            self.begin(now);
        }
        let interval = self
            .last_at
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        self.last_at = Some(now);
        let frame_us = duration.as_micros().min(u128::from(u32::MAX)) as u32;
        let interval_ms = interval.as_millis().min(u128::from(u32::MAX)) as u32;
        self.max_frame_us = self.max_frame_us.max(frame_us);
        self.max_interval_ms = self.max_interval_ms.max(interval_ms);
        self.total_frame_us += u64::from(frame_us);
        // The first frame only anchors the cadence: there is no previous
        // presentation to measure against.
        if self.frames > 0 {
            if u64::from(frame_us) > u64::from(self.budget_us) {
                self.over_budget_frames += 1;
            }
            let dropped_us = (Self::DROPPED_RATIO * f64::from(self.budget_us)) as u128;
            if interval.as_micros() > dropped_us {
                self.dropped_frames += 1;
            }
        }
        self.frames += 1;
    }

    /// Finish the gesture trace, keeping the counters for the next query. A
    /// no-op when nothing is being traced.
    pub fn finish(&mut self) {
        self.tracing = false;
    }

    /// Whether a gesture trace is in flight.
    pub fn tracing(&self) -> bool {
        self.tracing
    }

    /// Frames recorded for the current (or last) gesture.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Frames whose render duration exceeded the budget.
    pub fn over_budget_frames(&self) -> u64 {
        self.over_budget_frames
    }

    /// Presentations that missed a vsync (interval past
    /// [`Self::DROPPED_RATIO`] budgets).
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    /// Largest rendered-frame duration seen, in microseconds.
    pub fn max_frame_us(&self) -> u32 {
        self.max_frame_us
    }

    /// Largest presented-frame interval seen, in milliseconds.
    pub fn max_interval_ms(&self) -> u32 {
        self.max_interval_ms
    }

    /// The frame budget in microseconds.
    pub fn budget_us(&self) -> u32 {
        self.budget_us
    }

    /// Whether the gesture held the budget: at least one frame and neither an
    /// over-budget frame nor a dropped presentation. The honest "60 Hz or not"
    /// answer.
    pub fn held(&self) -> bool {
        self.frames > 0 && self.over_budget_frames == 0 && self.dropped_frames == 0
    }

    /// One-line summary for the `dump_stats` / `query gesture` trace.
    pub fn summary(&self) -> String {
        format!(
            "tracing={} frames={} over_budget={} dropped={} max_frame_us={} max_interval_ms={} \
             budget_us={} held={}",
            self.tracing as u32,
            self.frames,
            self.over_budget_frames,
            self.dropped_frames,
            self.max_frame_us,
            self.max_interval_ms,
            self.budget_us,
            self.held() as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats_with(direct: u64, rendered: u64, skipped: u64) -> RenderStats {
        let mut stats = RenderStats::default();
        stats.direct_scanouts = direct;
        stats.frames_rendered = rendered;
        stats.frames_skipped_no_damage = skipped;
        stats
    }

    #[test]
    fn latency_samples_the_input_to_present_delta() {
        let t0 = Instant::now();
        let mut latency = LatencyInstrument::default();
        latency.note_input(t0);
        latency.note_present(t0 + Duration::from_millis(8));
        assert_eq!(latency.last_us(), 8_000);
        assert_eq!(latency.max_us(), 8_000);
        assert_eq!(latency.samples(), 1);
        assert_eq!(latency.dropped(), 0);
        assert!(latency.within_one_frame(60));
        assert!(!latency.within_one_frame(240), "8 ms is two 240 Hz frames");
    }

    #[test]
    fn latency_keeps_the_earliest_input_of_a_batch() {
        let t0 = Instant::now();
        let mut latency = LatencyInstrument::default();
        latency.note_input(t0);
        latency.note_input(t0 + Duration::from_millis(3));
        latency.note_present(t0 + Duration::from_millis(10));
        assert_eq!(latency.last_us(), 10_000, "worst leg of the batch");
        assert_eq!(latency.samples(), 1);
    }

    #[test]
    fn latency_discards_a_stale_input() {
        let t0 = Instant::now();
        let mut latency = LatencyInstrument::default();
        latency.note_input(t0);
        latency.note_present(t0 + LatencyInstrument::STALE_LIMIT + Duration::from_millis(1));
        assert_eq!(latency.samples(), 0);
        assert_eq!(latency.dropped(), 1);
        // The pending input was cleared, so the next frame is not credited
        // to the stale input either.
        latency.note_present(t0 + Duration::from_secs(1));
        assert_eq!(latency.samples(), 0);
        assert_eq!(latency.dropped(), 1);
    }

    #[test]
    fn latency_present_without_input_is_not_a_sample() {
        let mut latency = LatencyInstrument::default();
        latency.note_present(Instant::now());
        assert_eq!(latency.samples(), 0);
        assert_eq!(latency.last_us(), 0);
    }

    #[test]
    fn scanout_template_reads_the_render_counters() {
        let stats = stats_with(3, 10, 2);
        let counter = ScanoutCounter::read(&stats);
        assert_eq!(counter.direct_scanouts, 3);
        assert_eq!(counter.frames_rendered, 10);
        assert_eq!(counter.frames_skipped_no_damage, 2);
        assert_eq!(counter.frames_considered(), 12);

        // An unobstructed fullscreen client: the counter advanced.
        let before = counter;
        let after = ScanoutCounter::read(&stats_with(7, 16, 2));
        assert_eq!(after.advanced_since(&before), 4);
        assert!(after.engaged_since(&before));

        // A composited frame (chrome over the client): flat scanout counter.
        let still = ScanoutCounter::read(&stats_with(7, 20, 2));
        assert_eq!(still.advanced_since(&after), 0);
        assert!(!still.engaged_since(&after));
    }

    #[test]
    fn gesture_trace_holds_the_budget_on_a_steady_cadence() {
        let mut trace = GestureBudgetTrace::with_budget_us(16_000);
        assert!(!trace.tracing());
        let t0 = Instant::now();
        // 16 ms apart, 2 ms of work: the first frame anchors, the rest hold.
        trace.record(t0, Duration::from_millis(2));
        trace.record(t0 + Duration::from_millis(16), Duration::from_millis(2));
        trace.record(t0 + Duration::from_millis(32), Duration::from_millis(2));
        assert!(trace.tracing(), "recording auto-begins the trace");
        assert_eq!(trace.frames(), 3);
        assert_eq!(trace.over_budget_frames(), 0);
        assert_eq!(trace.dropped_frames(), 0);
        assert_eq!(trace.max_interval_ms(), 16);
        assert_eq!(trace.max_frame_us(), 2_000);
        assert!(trace.held());

        // A new gesture restarts the counters.
        trace.finish();
        assert!(!trace.tracing());
        trace.begin(t0 + Duration::from_secs(1));
        assert!(trace.tracing());
        assert_eq!(trace.frames(), 0);
        assert!(!trace.held(), "an empty trace is not a held gesture");
    }

    #[test]
    fn gesture_trace_records_over_budget_frames_and_dropped_presentations() {
        let mut trace = GestureBudgetTrace::with_budget_us(16_000);
        let t0 = Instant::now();
        trace.record(t0, Duration::from_millis(2));
        // A 28 ms gap is over the 1.5x drop threshold: a missed vsync.
        trace.record(t0 + Duration::from_millis(28), Duration::from_millis(30));
        assert_eq!(trace.dropped_frames(), 1);
        assert_eq!(trace.over_budget_frames(), 1, "30 ms of work exceeds 16 ms");
        // A 20 ms cadence is jittery but not a dropped presentation.
        trace.record(t0 + Duration::from_millis(48), Duration::from_millis(2));
        assert_eq!(trace.over_budget_frames(), 1);
        assert_eq!(trace.dropped_frames(), 1);
        assert_eq!(trace.max_frame_us(), 30_000);
        assert_eq!(trace.max_interval_ms(), 28);
        assert!(!trace.held(), "the shortfall is recorded, not hidden");
        trace.finish();
        assert!(!trace.tracing());
        // The summary is one parseable line.
        let summary = trace.summary();
        for field in [
            "tracing=0",
            "frames=3",
            "over_budget=1",
            "dropped=1",
            "max_frame_us=30000",
            "max_interval_ms=28",
            "budget_us=16000",
            "held=0",
        ] {
            assert!(summary.contains(field), "missing {field:?} in {summary:?}");
        }
    }
}
