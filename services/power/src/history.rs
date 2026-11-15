// SPDX-License-Identifier: MIT
//! The battery history sampler (T273).
//!
//! UPower reports the *current* charge and health, not a charge history, so
//! the Battery pane cannot draw a chart from a single read. This module is the
//! honest sampler the pane reads instead: the adapter records one point per
//! successful read, in a bounded in-memory ring, and the pane charts those
//! points. History therefore starts when the system-status service starts and
//! is a normal, explicit empty state before the first samples arrive — never
//! invented data.
//!
//! The sampler is pure: time is an opaque epoch-seconds value supplied by the
//! caller, so tests drive it without sleeping. A sample is only recorded when
//! the level moved or the sampling interval elapsed, so a daemon that emits
//! several signals in one minute does not fill the ring with duplicates.

use std::collections::VecDeque;

/// The minimum spacing between two samples with the same level, in seconds.
pub const SAMPLE_INTERVAL_SECS: u64 = 60;

/// The longest range the pane can ask for: ten days, in seconds.
pub const MAX_RANGE_SECS: u64 = 10 * 24 * 60 * 60;

/// The ring capacity: one sample per minute for the ten-day range.
pub const MAX_SAMPLES: usize = (MAX_RANGE_SECS / SAMPLE_INTERVAL_SECS) as usize;

/// One battery level reading at an epoch-seconds instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatterySample {
    /// Seconds since the Unix epoch.
    pub at: u64,
    /// The charge, 0–100.
    pub percent: u8,
}

/// A bounded, monotonic battery history.
///
/// A new history is empty. [`record`](Self::record) appends a point when the
/// instant is newer than the last point and either the level moved or
/// [`SAMPLE_INTERVAL_SECS`] elapsed. [`window`](Self::window) returns the
/// points inside a range, oldest first, with the age in seconds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BatteryHistory {
    samples: VecDeque<BatterySample>,
}

impl BatteryHistory {
    /// An empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `percent` at `at`. Returns whether a point was stored.
    ///
    /// A stale instant (not newer than the last sample) is ignored, as is a
    /// repeat of the same level inside [`SAMPLE_INTERVAL_SECS`]. The ring
    /// drops its oldest point when it is full.
    pub fn record(&mut self, at: u64, percent: u8) -> bool {
        let percent = percent.min(100);
        if let Some(last) = self.samples.back_mut() {
            if at < last.at {
                return false;
            }
            if at == last.at {
                // Two reads in the same second: keep one point at that
                // instant, with the newest level.
                if last.percent == percent {
                    return false;
                }
                last.percent = percent;
                return true;
            }
            if last.percent == percent && at - last.at < SAMPLE_INTERVAL_SECS {
                return false;
            }
        }
        self.samples.push_back(BatterySample { at, percent });
        while self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
        true
    }

    /// Drop every point (a battery disappeared, or both daemons went away).
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// The number of stored points.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether no point has been recorded.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The instant of the oldest stored point.
    pub fn started_at(&self) -> Option<u64> {
        self.samples.front().map(|sample| sample.at)
    }

    /// The instant of the newest stored point.
    pub fn latest_at(&self) -> Option<u64> {
        self.samples.back().map(|sample| sample.at)
    }

    /// The points inside `range_secs` of `now`, oldest first, as
    /// `(seconds_ago, percent)` pairs.
    ///
    /// `range_secs` is clamped to [`MAX_RANGE_SECS`]. A future sample (a clock
    /// step backwards) is treated as age zero rather than a negative age.
    pub fn window(&self, now: u64, range_secs: u64) -> Vec<(u64, u8)> {
        let range = range_secs.min(MAX_RANGE_SECS);
        self.samples
            .iter()
            .filter(|sample| now.saturating_sub(sample.at) <= range)
            .map(|sample| (now.saturating_sub(sample.at), sample.percent))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_history_has_no_points() {
        let history = BatteryHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert_eq!(history.started_at(), None);
        assert!(history.window(1_000, 86_400).is_empty());
    }

    #[test]
    fn a_level_change_is_recorded_even_inside_the_interval() {
        let mut history = BatteryHistory::new();
        assert!(history.record(1_000, 70));
        assert!(history.record(1_010, 69));
        assert_eq!(history.len(), 2);
        assert_eq!(history.latest_at(), Some(1_010));
    }

    #[test]
    fn an_unchanged_level_inside_the_interval_is_deduplicated() {
        let mut history = BatteryHistory::new();
        assert!(history.record(1_000, 70));
        assert!(!history.record(1_030, 70));
        assert_eq!(history.len(), 1);
        assert!(history.record(1_060, 70));
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn a_same_instant_read_replaces_the_point_and_a_backwards_one_is_ignored() {
        let mut history = BatteryHistory::new();
        assert!(history.record(2_000, 50));
        assert!(history.record(2_000, 49));
        assert_eq!(history.len(), 1);
        assert_eq!(history.window(2_000, 60), vec![(0, 49)]);
        assert!(!history.record(2_000, 49));
        assert!(!history.record(1_999, 48));
        assert_eq!(history.len(), 1);
        assert_eq!(history.window(2_000, 60), vec![(0, 49)]);
    }

    #[test]
    fn the_window_returns_ages_oldest_first() {
        let mut history = BatteryHistory::new();
        history.record(1_000, 80);
        history.record(1_100, 78);
        history.record(1_200, 75);
        assert_eq!(
            history.window(1_300, 350),
            vec![(300, 80), (200, 78), (100, 75)]
        );
        assert_eq!(history.window(1_300, 250), vec![(200, 78), (100, 75)]);
    }

    #[test]
    fn a_future_point_is_age_zero_not_negative() {
        let mut history = BatteryHistory::new();
        history.record(1_000, 80);
        assert_eq!(history.window(900, 200), vec![(0, 80)]);
    }

    #[test]
    fn the_ring_is_bounded_and_drops_the_oldest_point() {
        let mut history = BatteryHistory::new();
        for index in 0..(MAX_SAMPLES as u64 + 5) {
            history.record(index * SAMPLE_INTERVAL_SECS, (index % 100) as u8);
        }
        assert_eq!(history.len(), MAX_SAMPLES);
        assert_eq!(
            history.started_at(),
            Some(5 * SAMPLE_INTERVAL_SECS),
            "the oldest five points must have been dropped"
        );
    }

    #[test]
    fn clear_drops_every_point() {
        let mut history = BatteryHistory::new();
        history.record(1_000, 50);
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn a_level_is_clamped_to_the_battery_range() {
        let mut history = BatteryHistory::new();
        history.record(1_000, 240);
        assert_eq!(history.window(1_000, 60), vec![(0, 100)]);
    }
}
