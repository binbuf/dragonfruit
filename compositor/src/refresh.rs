// SPDX-License-Identifier: MIT
//! Per-output refresh model (T-21.2a, T187a).
//!
//! The compositor needs each output's real mode period before it can schedule
//! animation frames or choose a frame budget. This module is the **pure**
//! policy half: it turns a mode refresh into an [`OutputRefresh`] and keeps one
//! per output in a [`RefreshSchedule`]. It has no renderer, timer, or Smithay
//! `Output` dependency, so the 60/120/144/240 Hz table is unit-testable
//! without hardware.
//!
//! Smithay's `Mode::refresh` is **millihertz** (`143_999` is 143.999 Hz), not
//! Hz. The period is therefore `1e12 / refresh_mhz` nanoseconds, and the
//! deadline is the period itself. A later task may subtract a submit margin;
//! the model does not invent one.
//!
//! The 60 Hz fallback is deliberate: a backend without a mode (headless or a
//! nested output before its first mode) must not be treated as a fast panel.
//! Positive mode values are preserved exactly inside the sane range and only
//! clamped outside it (24–500 Hz), so a 143.999 Hz panel keeps its own period
//! instead of being rounded to 144 Hz.
//!
//! VRR is carried as a tri-state ([`VrrState`]) for T188. This module does not
//! interpret it and never changes the period because of it.

use std::collections::BTreeMap;
use std::time::Duration;

/// Refresh used when an output has no mode or reports a zero/negative one.
pub const FALLBACK_REFRESH_MHZ: u32 = 60_000;

/// The 60 Hz fallback period, used for an empty schedule as well.
pub const FALLBACK_PERIOD: Duration =
    Duration::from_nanos(1_000_000_000_000 / FALLBACK_REFRESH_MHZ as u64);

/// Lowest refresh the model accepts (24 Hz).
pub const MIN_REFRESH_MHZ: u32 = 24_000;

/// Highest refresh the model accepts (500 Hz).
pub const MAX_REFRESH_MHZ: u32 = 500_000;

/// Test/diagnostic override for the animation tick and degrade budget, in Hz.
/// Unset in a normal session; nested/headless tests set `60`, `120`, `144`, or
/// `240` to exercise the table without hardware.
pub const ENV_REFRESH_HZ: &str = "DRAGONFRUIT_REFRESH_HZ";

/// Where an [`OutputRefresh`] value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshSource {
    /// The output's current mode (possibly clamped to the sane range).
    Mode,
    /// No usable mode; the 60 Hz fallback was used.
    Fallback,
    /// A test/diagnostic override (wired in T187b).
    Override,
}

impl RefreshSource {
    /// The stable stats token (`source=mode|fallback|override`).
    pub const fn as_str(self) -> &'static str {
        match self {
            RefreshSource::Mode => "mode",
            RefreshSource::Fallback => "fallback",
            RefreshSource::Override => "override",
        }
    }
}

/// Variable-refresh state carried alongside a mode (T188 consumes it).
///
/// T187a only stores the state; it never changes the period or scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VrrState {
    /// The backend did not report VRR support/state.
    #[default]
    Unknown,
    /// The output is fixed-refresh.
    Off,
    /// The output may present at a variable interval.
    On,
}

/// One mode's refresh input, without a Smithay `Output` handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeRefresh {
    /// Refresh in millihertz (Smithay's unit).
    pub refresh_mhz: i32,
    /// VRR state reported for the mode, if any.
    pub vrr: VrrState,
}

impl ModeRefresh {
    /// A mode with unknown VRR state.
    pub const fn new(refresh_mhz: i32) -> Self {
        ModeRefresh {
            refresh_mhz,
            vrr: VrrState::Unknown,
        }
    }

    /// A mode with an explicit VRR state.
    pub const fn with_vrr(refresh_mhz: i32, vrr: VrrState) -> Self {
        ModeRefresh { refresh_mhz, vrr }
    }
}

/// The resolved refresh of one output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputRefresh {
    /// The accepted refresh in millihertz (always inside 24–500 Hz).
    pub refresh_mhz: u32,
    /// One frame at `refresh_mhz`, in nanoseconds.
    pub period: Duration,
    /// The scheduling deadline. Equal to [`Self::period`] today; T188 may
    /// subtract a backend submit margin.
    pub deadline: Duration,
    /// Whether the value came from a mode, the fallback, or an override.
    pub source: RefreshSource,
    /// VRR state carried for T188; it does not affect `period` here.
    pub vrr: VrrState,
}

impl OutputRefresh {
    /// Resolve a mode refresh. A zero/negative value falls back to 60 Hz;
    /// positive values are clamped to [`MIN_REFRESH_MHZ`]..[`MAX_REFRESH_MHZ`]
    /// and otherwise keep their exact period.
    pub fn from_mhz(refresh_mhz: i32, source: RefreshSource, vrr: VrrState) -> Self {
        if refresh_mhz <= 0 {
            return Self::fallback(vrr);
        }
        let refresh_mhz = (refresh_mhz as u32).clamp(MIN_REFRESH_MHZ, MAX_REFRESH_MHZ);
        let period = period_for(refresh_mhz);
        OutputRefresh {
            refresh_mhz,
            period,
            deadline: period,
            source,
            vrr,
        }
    }

    /// The 60 Hz fallback (`source=fallback`).
    pub fn fallback(vrr: VrrState) -> Self {
        OutputRefresh {
            refresh_mhz: FALLBACK_REFRESH_MHZ,
            period: FALLBACK_PERIOD,
            deadline: FALLBACK_PERIOD,
            source: RefreshSource::Fallback,
            vrr,
        }
    }

    /// The mode's refresh in Hz, for `presentation stats` and traces.
    pub fn refresh_hz(&self) -> f64 {
        f64::from(self.refresh_mhz) / 1_000.0
    }

    /// A test/diagnostic override from a refresh in Hz. Non-finite and
    /// non-positive values are rejected; a valid value is clamped by the same
    /// 24–500 Hz policy as a mode and reports [`RefreshSource::Override`].
    pub fn override_from_hz(hz: f64) -> Option<Self> {
        if !hz.is_finite() || hz <= 0.0 {
            return None;
        }
        let mhz = (hz * 1_000.0).round().clamp(1.0, f64::from(i32::MAX)) as i32;
        Some(Self::from_mhz(
            mhz,
            RefreshSource::Override,
            VrrState::Unknown,
        ))
    }

    /// Parse `DRAGONFRUIT_REFRESH_HZ` (a decimal Hz value).
    pub fn override_from_str(value: &str) -> Option<Self> {
        Self::override_from_hz(value.trim().parse::<f64>().ok()?)
    }
}

/// One period in nanoseconds from a millihertz refresh. The caller guarantees
/// `refresh_mhz > 0`; integer truncation keeps the value below one nanosecond
/// of error per frame.
fn period_for(refresh_mhz: u32) -> Duration {
    debug_assert!(refresh_mhz > 0);
    Duration::from_nanos(1_000_000_000_000 / u64::from(refresh_mhz))
}

/// The per-output refresh table, keyed by output name.
///
/// `BTreeMap` makes iteration and tie-breaking deterministic. An empty
/// schedule still reports [`FALLBACK_PERIOD`] from [`Self::fastest_period`],
/// so the animation clock always has a valid interval before the first output
/// appears.
#[derive(Debug, Clone, Default)]
pub struct RefreshSchedule {
    outputs: BTreeMap<String, OutputRefresh>,
}

impl RefreshSchedule {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace one output's mode. `None` (or an invalid refresh)
    /// installs the 60 Hz fallback. Returns the resolved value.
    pub fn update(&mut self, output: &str, mode: Option<ModeRefresh>) -> OutputRefresh {
        let refresh = match mode {
            Some(mode) => OutputRefresh::from_mhz(mode.refresh_mhz, RefreshSource::Mode, mode.vrr),
            None => OutputRefresh::fallback(VrrState::Unknown),
        };
        self.outputs.insert(output.to_owned(), refresh);
        refresh
    }

    /// Remove an output (hotplug/unplug). Returns its last value.
    pub fn remove(&mut self, output: &str) -> Option<OutputRefresh> {
        self.outputs.remove(output)
    }

    /// Drop every output, e.g. when the backend tears down all connectors.
    pub fn clear(&mut self) {
        self.outputs.clear();
    }

    /// The output with the shortest active period, or `None` for an empty
    /// schedule. Ties resolve by output name because the map is ordered.
    pub fn fastest(&self) -> Option<&OutputRefresh> {
        self.outputs.values().min_by_key(|refresh| refresh.period)
    }

    /// The shortest active period, or the 60 Hz fallback when the schedule is
    /// empty. Ties resolve by output name because the map is ordered.
    pub fn fastest_period(&self) -> Duration {
        self.fastest()
            .map(|refresh| refresh.period)
            .unwrap_or(FALLBACK_PERIOD)
    }

    /// The resolved refresh that drives the clock: the fastest active output,
    /// or the 60 Hz fallback for an empty schedule.
    pub fn fastest_refresh(&self) -> OutputRefresh {
        self.fastest()
            .copied()
            .unwrap_or_else(|| OutputRefresh::fallback(VrrState::Unknown))
    }

    /// One output's resolved refresh.
    pub fn for_output(&self, output: &str) -> Option<&OutputRefresh> {
        self.outputs.get(output)
    }

    /// Iterate outputs in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &OutputRefresh)> {
        self.outputs
            .iter()
            .map(|(name, refresh)| (name.as_str(), refresh))
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

    /// Compare a Duration against an expected value in milliseconds.
    fn assert_ms(actual: Duration, expected_ms: f64) {
        let actual_ms = actual.as_secs_f64() * 1_000.0;
        assert!(
            (actual_ms - expected_ms).abs() < 0.01,
            "expected {expected_ms:.3} ms, got {actual_ms:.6} ms"
        );
    }

    #[test]
    fn the_60_120_144_and_240_hz_table_has_the_expected_periods() {
        let table = [
            (60_000, 16.667),
            (120_000, 8.333),
            (144_000, 6.944),
            (240_000, 4.167),
        ];
        for (mhz, expected_ms) in table {
            let refresh = OutputRefresh::from_mhz(mhz, RefreshSource::Mode, VrrState::Unknown);
            assert_eq!(refresh.refresh_mhz, mhz as u32);
            assert_eq!(refresh.source, RefreshSource::Mode);
            assert_ms(refresh.period, expected_ms);
            assert_eq!(
                refresh.deadline, refresh.period,
                "the model deadline is the period"
            );
        }
    }

    #[test]
    fn a_real_panel_keeps_its_exact_millihertz_period() {
        // 143.999 Hz is not rounded to 144.000 Hz.
        let refresh = OutputRefresh::from_mhz(143_999, RefreshSource::Mode, VrrState::Unknown);
        assert_eq!(refresh.refresh_mhz, 143_999);
        assert_eq!(refresh.period, Duration::from_nanos(6_944_492));
        assert_ms(refresh.period, 6.944);
        assert!((refresh.refresh_hz() - 143.999).abs() < 1e-9);
    }

    #[test]
    fn no_mode_or_an_invalid_mode_falls_back_to_60_hz() {
        let mut schedule = RefreshSchedule::new();
        let no_mode = schedule.update("HDMI-A-1", None);
        assert_eq!(no_mode.refresh_mhz, FALLBACK_REFRESH_MHZ);
        assert_eq!(no_mode.source, RefreshSource::Fallback);
        assert_eq!(no_mode.period, FALLBACK_PERIOD);
        assert_ms(no_mode.period, 16.667);

        for invalid in [0, -1, i32::MIN] {
            let refresh = schedule.update("HDMI-A-1", Some(ModeRefresh::new(invalid)));
            assert_eq!(refresh.refresh_mhz, FALLBACK_REFRESH_MHZ);
            assert_eq!(refresh.source, RefreshSource::Fallback);
            assert_eq!(refresh.period, FALLBACK_PERIOD);
        }
    }

    #[test]
    fn positive_modes_are_clamped_to_the_sane_range() {
        let slow = OutputRefresh::from_mhz(1_000, RefreshSource::Mode, VrrState::Unknown);
        assert_eq!(slow.refresh_mhz, MIN_REFRESH_MHZ);
        assert_ms(slow.period, 41.667);

        let fast = OutputRefresh::from_mhz(1_000_000, RefreshSource::Mode, VrrState::Unknown);
        assert_eq!(fast.refresh_mhz, MAX_REFRESH_MHZ);
        assert_ms(fast.period, 2.0);

        // A clamped mode still reports that it came from the mode.
        assert_eq!(slow.source, RefreshSource::Mode);
        assert_eq!(fast.source, RefreshSource::Mode);
    }

    #[test]
    fn updating_one_output_replaces_its_mode() {
        let mut schedule = RefreshSchedule::new();
        schedule.update("eDP-1", Some(ModeRefresh::new(60_000)));
        assert_eq!(schedule.len(), 1);
        assert_ms(schedule.for_output("eDP-1").unwrap().period, 16.667);

        let changed = schedule.update("eDP-1", Some(ModeRefresh::new(144_000)));
        assert_eq!(schedule.len(), 1, "a mode change does not add an output");
        assert_eq!(changed.refresh_mhz, 144_000);
        assert_eq!(schedule.for_output("eDP-1").unwrap().refresh_mhz, 144_000);
        assert_ms(schedule.fastest_period(), 6.944);
    }

    #[test]
    fn add_and_remove_track_hotplug() {
        let mut schedule = RefreshSchedule::new();
        assert!(schedule.is_empty());
        assert_eq!(schedule.fastest_period(), FALLBACK_PERIOD);

        schedule.update("eDP-1", Some(ModeRefresh::new(60_000)));
        schedule.update("DP-1", Some(ModeRefresh::new(144_000)));
        assert_eq!(schedule.len(), 2);
        assert_ms(schedule.fastest_period(), 6.944);

        let removed = schedule.remove("DP-1").expect("DP-1 was present");
        assert_eq!(removed.refresh_mhz, 144_000);
        assert!(schedule.for_output("DP-1").is_none());
        assert_eq!(schedule.len(), 1);
        assert_ms(schedule.fastest_period(), 16.667);

        assert!(schedule.remove("DP-1").is_none(), "remove is idempotent");
        schedule.clear();
        assert!(schedule.is_empty());
        assert_eq!(schedule.fastest_period(), FALLBACK_PERIOD);
    }

    #[test]
    fn hotplug_and_fastest_selection_follow_the_shortest_period() {
        let mut schedule = RefreshSchedule::new();
        schedule.update("DP-1", Some(ModeRefresh::new(60_000)));
        schedule.update("eDP-1", Some(ModeRefresh::new(144_000)));
        assert_ms(schedule.fastest_period(), 6.944);

        // Plugging in a 240 Hz panel makes it the fastest.
        schedule.update("HDMI-A-1", Some(ModeRefresh::new(240_000)));
        assert_ms(schedule.fastest_period(), 4.167);

        // Unplugging it leaves the 144 Hz output in charge.
        schedule.remove("HDMI-A-1");
        assert_ms(schedule.fastest_period(), 6.944);

        // Unplugging the fast panel falls back to the remaining 60 Hz output.
        schedule.remove("eDP-1");
        assert_ms(schedule.fastest_period(), 16.667);

        // No outputs at all still yields the 60 Hz fallback interval.
        schedule.remove("DP-1");
        assert_eq!(schedule.fastest_period(), FALLBACK_PERIOD);
    }

    #[test]
    fn vrr_is_carried_but_does_not_change_the_period() {
        let mut schedule = RefreshSchedule::new();
        for (vrr, name) in [
            (VrrState::Unknown, "unknown"),
            (VrrState::Off, "off"),
            (VrrState::On, "on"),
        ] {
            let refresh = schedule.update(name, Some(ModeRefresh::with_vrr(144_000, vrr)));
            assert_eq!(refresh.vrr, vrr);
            assert_ms(refresh.period, 6.944);
        }

        assert_eq!(
            schedule.for_output("unknown").unwrap().vrr,
            VrrState::Unknown
        );
        assert_eq!(schedule.for_output("off").unwrap().vrr, VrrState::Off);
        assert_eq!(schedule.for_output("on").unwrap().vrr, VrrState::On);
        assert_eq!(schedule.for_output("missing"), None);
    }

    #[test]
    fn sources_have_stable_stats_tokens() {
        assert_eq!(RefreshSource::Mode.as_str(), "mode");
        assert_eq!(RefreshSource::Fallback.as_str(), "fallback");
        assert_eq!(RefreshSource::Override.as_str(), "override");

        let overridden =
            OutputRefresh::from_mhz(120_000, RefreshSource::Override, VrrState::Unknown);
        assert_eq!(overridden.source, RefreshSource::Override);
        assert_ms(overridden.period, 8.333);
    }

    #[test]
    fn the_test_override_covers_the_60_120_144_240_table() {
        let table = [
            ("60", 60_000, 16.667),
            ("120", 120_000, 8.333),
            ("144", 144_000, 6.944),
            ("240", 240_000, 4.167),
            // A real panel value keeps its exact millihertz period.
            ("143.999", 143_999, 6.944),
        ];
        for (text, expected_mhz, expected_ms) in table {
            let refresh = OutputRefresh::override_from_str(text)
                .unwrap_or_else(|| panic!("{text:?} should parse"));
            assert_eq!(refresh.refresh_mhz, expected_mhz);
            assert_eq!(refresh.source, RefreshSource::Override);
            assert_ms(refresh.period, expected_ms);
            assert_eq!(refresh.deadline, refresh.period);
        }

        // Invalid values are ignored (a normal session keeps the mode policy).
        for invalid in ["", "0", "-1", "nan", "inf", "fast"] {
            assert_eq!(
                OutputRefresh::override_from_str(invalid),
                None,
                "{invalid:?} must not install an override"
            );
        }
    }

    #[test]
    fn fastest_refresh_carries_the_source_of_the_shortest_period() {
        let mut schedule = RefreshSchedule::new();
        let empty = schedule.fastest_refresh();
        assert_eq!(empty.source, RefreshSource::Fallback);
        assert_eq!(empty.period, FALLBACK_PERIOD);

        schedule.update("DP-1", Some(ModeRefresh::new(60_000)));
        schedule.update("eDP-1", Some(ModeRefresh::new(143_999)));
        let fastest = schedule.fastest_refresh();
        assert_eq!(fastest.source, RefreshSource::Mode);
        assert_eq!(fastest.refresh_mhz, 143_999);
        assert_eq!(fastest.period, Duration::from_nanos(6_944_492));
        assert_eq!(schedule.fastest_period(), fastest.period);

        schedule.update("HDMI-A-1", None);
        assert_eq!(schedule.fastest_refresh().refresh_mhz, 143_999);
    }
}
