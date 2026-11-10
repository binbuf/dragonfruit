// SPDX-License-Identifier: MIT
//! Menu-bar auto-hide, per-output reveal and measured motion (T201/T233).
//!
//! `menu.autoHide` has three values and the compositor is the single owner of
//! the resulting visibility on each output:
//!
//! * `never` — visible on the desktop and over a fullscreen client;
//! * `always` — concealed until the pointer reaches the top edge or the
//!   menu-bar focus shortcut reveals it;
//! * `full-screen` — visible on the desktop, concealed over a fullscreen
//!   client until revealed on demand.
//!
//! A concealed bar is removed from the visible-chrome list, so it contributes
//! no reserved zone, input hit area, material region, or sampled/veil panel.
//! That is what keeps a fullscreen output eligible for direct scanout. A
//! revealed bar behaves exactly like the old pinned bar until it conceals.
//!
//! T233 adds the measured transition around that binary policy: a pointer
//! leaving the bar keeps it up for [`MenuBarTiming::hide_delay_ms`], then the
//! bar slides out over [`MenuBarTiming::motion_ms`] with the shared
//! `menuBarReveal` curve. Revealing is immediate by reference (the top edge is
//! a hard pointer barrier); a nonzero reveal delay is supported and measured
//! by the unit tests. Reduced motion collapses both the delays and the motion
//! to zero, so the same state machine settles in one step.
//!
//! Reveal state is per output: revealing the bar on the display under the
//! pointer must not float it onto a second display. The menu bar itself is a
//! single all-output chrome surface, so the compositor filters that surface
//! per output from this state rather than creating a second surface. The
//! surface's *visual* translation is `(1 - progress) * surface height`; the
//! reserved strip and input region stay at the top while the bar is on
//! screen, and disappear at progress zero.
//!
//! Active menu/focus interactions keep the bar revealed: the caller passes
//! `active` when the bar holds keyboard focus or its dropdown overlay is
//! mapped. Concealing clears the reveal state; the next pointer move at the
//! top edge (or the focus shortcut) reveals it again. Locking clears reveal so
//! an unlock starts from the policy's resting state.

use std::collections::HashMap;

use crate::animation::Tween;
use crate::design_tokens::{component::menu_bar, motion};

/// The `menu.autoHide` policy, mirrored from the settings key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoHideMode {
    Never,
    Always,
    FullScreen,
}

impl Default for AutoHideMode {
    fn default() -> Self {
        // The settings schema default is `full-screen` and matches the
        // pre-T201 behavior: visible on the desktop, hidden over fullscreen.
        AutoHideMode::FullScreen
    }
}

impl AutoHideMode {
    /// Parse the wire value used by `df_layer_surface.set_auto_hide`.
    pub const fn from_wire(value: u32) -> Self {
        match value {
            0 => AutoHideMode::Never,
            1 => AutoHideMode::Always,
            _ => AutoHideMode::FullScreen,
        }
    }

    /// Stable name for diagnostics and logs.
    pub const fn name(self) -> &'static str {
        match self {
            AutoHideMode::Never => "never",
            AutoHideMode::Always => "always",
            AutoHideMode::FullScreen => "full-screen",
        }
    }
}

/// The top-edge band that reveals a concealed menu bar. This is the bar's
/// own logical height, so the pointer that enters the revealed strip is
/// already inside the bar's input region.
pub const MENU_BAR_REVEAL_HEIGHT: i32 = menu_bar::HEIGHT as i32;

/// The measured transition timings. The defaults come from the design tokens;
/// the unit tests build a smaller set so a delay/tween can be asserted without
/// sleeping on wall-clock time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuBarTiming {
    pub reveal_delay_ms: u64,
    pub hide_delay_ms: u64,
    pub motion_ms: u64,
    pub curve: [f32; 4],
}

impl Default for MenuBarTiming {
    fn default() -> Self {
        Self {
            reveal_delay_ms: u64::from(menu_bar::REVEAL_DELAY as u32),
            hide_delay_ms: u64::from(menu_bar::HIDE_DELAY as u32),
            motion_ms: u64::from(motion::MENU_BAR_REVEAL.duration_ms),
            curve: motion::MENU_BAR_REVEAL.curve,
        }
    }
}

impl MenuBarTiming {
    fn reveal_delay(self, reduced_motion: bool) -> u64 {
        if reduced_motion {
            0
        } else {
            self.reveal_delay_ms
        }
    }

    fn hide_delay(self, reduced_motion: bool) -> u64 {
        if reduced_motion {
            0
        } else {
            self.hide_delay_ms
        }
    }

    fn motion(self, reduced_motion: bool) -> u64 {
        if reduced_motion {
            0
        } else {
            self.motion_ms
        }
    }
}

/// Whether the menu bar is visible for one output.
///
/// `fullscreen` is true when a settled fullscreen client covers the output.
/// `revealed` is the per-output transient reveal. `active` keeps the bar up
/// while a menu is open or the bar owns keyboard focus, even if the pointer
/// has left the top edge.
pub const fn menu_bar_visible(
    mode: AutoHideMode,
    fullscreen: bool,
    revealed: bool,
    active: bool,
) -> bool {
    match mode {
        AutoHideMode::Never => true,
        AutoHideMode::Always => revealed || active,
        AutoHideMode::FullScreen => !fullscreen || revealed || active,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PendingTransition {
    at_ms: u64,
    visible: bool,
}

/// One output's reveal state and in-flight transition.
#[derive(Debug, Clone, PartialEq)]
struct OutputReveal {
    /// The pointer/focus reveal intent (`revealed` in the old API).
    revealed: bool,
    /// The current policy target (visible or concealed).
    desired_visible: bool,
    /// The rendered fraction of the bar that is on screen, `0.0..=1.0`.
    progress: f64,
    pending: Option<PendingTransition>,
    tween: Option<Tween>,
    tween_from: f64,
    tween_to: f64,
}

impl Default for OutputReveal {
    fn default() -> Self {
        Self {
            revealed: false,
            desired_visible: false,
            progress: 0.0,
            pending: None,
            tween: None,
            tween_from: 0.0,
            tween_to: 0.0,
        }
    }
}

impl OutputReveal {
    fn current_progress(&self, now_ms: u64) -> f64 {
        match self.tween {
            Some(tween) => {
                let t = tween.progress(now_ms);
                (self.tween_from + (self.tween_to - self.tween_from) * t).clamp(0.0, 1.0)
            }
            None => self.progress,
        }
    }

    fn settle(&mut self, visible: bool) {
        self.desired_visible = visible;
        self.progress = if visible { 1.0 } else { 0.0 };
        self.pending = None;
        self.tween = None;
        self.tween_from = self.progress;
        self.tween_to = self.progress;
    }
}

/// The compositor-owned reveal state for the one menu-bar surface.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuBarVisibility {
    mode: AutoHideMode,
    timing: MenuBarTiming,
    reduced_motion: bool,
    outputs: HashMap<String, OutputReveal>,
}

impl Default for MenuBarVisibility {
    fn default() -> Self {
        Self {
            mode: AutoHideMode::default(),
            timing: MenuBarTiming::default(),
            reduced_motion: false,
            outputs: HashMap::new(),
        }
    }
}

impl MenuBarVisibility {
    pub fn mode(&self) -> AutoHideMode {
        self.mode
    }

    pub fn timing(&self) -> MenuBarTiming {
        self.timing
    }

    /// Test/feature hook: replace the measured timings. The live session uses
    /// the token defaults.
    pub fn set_timing(&mut self, timing: MenuBarTiming) {
        self.timing = timing;
    }

    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    /// Apply the accessibility reduced-motion policy. A live change collapses
    /// every pending delay and running tween to its target in one step.
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        if self.reduced_motion == reduced {
            return false;
        }
        self.reduced_motion = reduced;
        if reduced {
            for output in self.outputs.values_mut() {
                if let Some(pending) = output.pending.take() {
                    output.settle(pending.visible);
                }
                if let Some(tween) = output.tween.take() {
                    output.progress = output.tween_to;
                    output.tween_from = output.progress;
                    output.tween_to = output.progress;
                    let _ = tween;
                }
            }
        }
        true
    }

    /// Apply a new policy. A live policy change drops the old transient
    /// reveal; the next pointer/keyboard interaction applies the new mode.
    /// The resting progress is `1` for `never` and `0` otherwise; the caller
    /// (`DfState`) settles each output against its fullscreen/active state.
    /// Returns whether anything changed.
    pub fn set_mode(&mut self, mode: AutoHideMode) -> bool {
        if self.mode == mode {
            return false;
        }
        self.mode = mode;
        let resting = mode == AutoHideMode::Never;
        for output in self.outputs.values_mut() {
            output.revealed = false;
            output.settle(resting);
        }
        true
    }

    pub fn is_revealed(&self, output: &str) -> bool {
        self.outputs
            .get(output)
            .map(|state| state.revealed)
            .unwrap_or(false)
    }

    /// Reveal the bar on one output at the policy level. The transition is
    /// started by the next pointer/focus sync (or [`Self::reveal_at`]).
    /// Returns whether the intent changed.
    pub fn reveal(&mut self, output: &str) -> bool {
        let state = self.outputs.entry(output.to_owned()).or_default();
        if state.revealed {
            return false;
        }
        state.revealed = true;
        true
    }

    /// Conceal the bar on one output at the policy level. Returns whether the
    /// intent changed.
    pub fn conceal(&mut self, output: &str) -> bool {
        let state = self.outputs.entry(output.to_owned()).or_default();
        if !state.revealed {
            return false;
        }
        state.revealed = false;
        true
    }

    /// Reveal immediately (Control-F2): no reveal delay, but the measured
    /// motion still runs unless reduced motion is on.
    pub fn reveal_at(&mut self, output: &str, now_ms: u64) -> bool {
        let timing = self.timing;
        let reduced_motion = self.reduced_motion;
        let mode = self.mode;
        let state = self.outputs.entry(output.to_owned()).or_default();
        let changed = !state.revealed;
        state.revealed = true;
        if mode == AutoHideMode::Never {
            state.settle(true);
            return changed;
        }
        if state.progress >= 1.0 && state.tween.is_none() {
            state.desired_visible = true;
            state.pending = None;
            return changed;
        }
        state.progress = state.current_progress(now_ms);
        state.desired_visible = true;
        start_tween(state, 1.0, now_ms, timing, reduced_motion);
        true
    }

    /// Drop every transient reveal (lock, or a policy change). Returns whether
    /// anything changed.
    pub fn clear(&mut self) -> bool {
        let mut changed = false;
        for state in self.outputs.values_mut() {
            if state.revealed
                || state.progress > 0.0
                || state.pending.is_some()
                || state.tween.is_some()
            {
                changed = true;
            }
            state.revealed = false;
            state.settle(false);
        }
        changed
    }

    /// Settle one output at an explicit visibility, without animation. Used
    /// after a mode change and when a new output joins the layout.
    pub fn settle(&mut self, output: &str, visible: bool) {
        self.outputs
            .entry(output.to_owned())
            .or_default()
            .settle(visible);
    }

    /// Whether this output has been resolved yet (a pointer/focus sync or a
    /// mode change settled it).
    pub fn has_output(&self, output: &str) -> bool {
        self.outputs.contains_key(output)
    }

    /// The rendered reveal fraction for `output` (`0.0` concealed, `1.0`
    /// fully on screen).
    pub fn progress_on(&self, output: &str) -> f64 {
        self.outputs
            .get(output)
            .map(|state| state.progress)
            .unwrap_or(0.0)
    }

    /// The upward visual translation of the menu-bar surface on `output`.
    /// The shell surface is anchored at the top; the compositor subtracts this
    /// from its render location so a concealed bar is fully above the output.
    pub fn hide_offset_on(&self, output: &str, surface_height: i32) -> i32 {
        let progress = self.progress_on(output);
        if progress >= 1.0 {
            return 0;
        }
        ((1.0 - progress) * f64::from(surface_height.max(0))).round() as i32
    }

    /// Whether the bar is visible on `output` under the current policy. The
    /// target counts as visible while a reveal is pending, and the rendered
    /// fraction keeps it visible through a hide transition; only a settled
    /// concealed bar is invisible.
    pub fn visible_on(&self, output: &str, fullscreen: bool, active: bool) -> bool {
        if self.mode == AutoHideMode::Never {
            return true;
        }
        let desired = menu_bar_visible(self.mode, fullscreen, self.is_revealed(output), active);
        desired || self.progress_on(output) > 0.0
    }

    /// Whether any output has a pending delay or a running reveal/hide tween.
    pub fn is_animating(&self, now_ms: u64) -> bool {
        self.outputs
            .values()
            .any(|state| state.pending.is_some() || state.is_animating(now_ms))
    }

    /// Whether `output` has a pending delay or a running transition.
    pub fn is_animating_on(&self, output: &str, now_ms: u64) -> bool {
        self.outputs
            .get(output)
            .map(|state| state.pending.is_some() || state.is_animating(now_ms))
            .unwrap_or(false)
    }

    /// Outputs with a live reveal/hide transition, for the shared animation
    /// clock's per-output request set.
    pub fn animating_outputs(&self, now_ms: u64) -> Vec<String> {
        self.outputs
            .iter()
            .filter(|(_, state)| state.pending.is_some() || state.is_animating(now_ms))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Advance every output's pending delay and tween to `now_ms`. Returns
    /// whether any rendered progress changed.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        for state in self.outputs.values_mut() {
            let before = state.progress;
            if let Some(pending) = state.pending {
                if now_ms >= pending.at_ms {
                    state.pending = None;
                    let target = if pending.visible { 1.0 } else { 0.0 };
                    state.progress = state.current_progress(now_ms);
                    // The tween is started at the deadline, so the first
                    // frame after it has zero elapsed motion (a real
                    // measured delay) and the curve advances from there.
                    let start = now_ms;
                    let from = state.progress;
                    let duration = self.timing.motion(self.reduced_motion);
                    state.desired_visible = pending.visible;
                    if duration == 0 {
                        state.progress = target;
                        state.tween = None;
                        state.tween_from = target;
                        state.tween_to = target;
                    } else {
                        state.tween = Some(Tween::new(start, duration, self.timing.curve));
                        state.tween_from = from;
                        state.tween_to = target;
                    }
                }
            }
            if let Some(tween) = state.tween {
                let target = state.tween_to;
                let from = state.tween_from;
                state.progress = (from + (target - from) * tween.progress(now_ms)).clamp(0.0, 1.0);
                if tween.is_done(now_ms) {
                    state.progress = target;
                    state.tween = None;
                }
            }
            if (state.progress - before).abs() > f64::EPSILON {
                changed = true;
            }
        }
        changed
    }

    /// Apply the pointer position to the reveal state for one output.
    ///
    /// `local_y` is output-local logical. The bar reveals while the pointer is
    /// in the top band, or whenever an active menu/focus interaction holds it
    /// up. On the desktop a `full-screen`-mode bar is always visible, so its
    /// transient state is cleared: entering fullscreen then starts concealed.
    ///
    /// `pointer_seen` is false until the compositor has received a real
    /// pointer motion. The initial (0,0) seat position must not count as an
    /// edge reveal, or `always` would start visible on every session; an
    /// active keyboard focus can still reveal before the first motion.
    /// Returns whether any intent or rendered state changed.
    pub fn update_pointer(
        &mut self,
        now_ms: u64,
        output: &str,
        local_y: f64,
        fullscreen: bool,
        active: bool,
        pointer_seen: bool,
    ) -> bool {
        let edge = pointer_seen && local_y < f64::from(MENU_BAR_REVEAL_HEIGHT);
        let desired_reveal = match self.mode {
            AutoHideMode::Never => false,
            AutoHideMode::Always => edge || active,
            AutoHideMode::FullScreen => fullscreen && (edge || active),
        };
        let timing = self.timing;
        let reduced_motion = self.reduced_motion;
        let mode = self.mode;
        let state = self.outputs.entry(output.to_owned()).or_default();
        let mut changed = false;
        if state.revealed != desired_reveal {
            state.revealed = desired_reveal;
            changed = true;
        }
        let desired = menu_bar_visible(mode, fullscreen, state.revealed, active);
        changed |= set_desired(state, desired, now_ms, mode, timing, reduced_motion);
        changed
    }
}

fn start_tween(
    state: &mut OutputReveal,
    target: f64,
    now_ms: u64,
    timing: MenuBarTiming,
    reduced_motion: bool,
) {
    state.pending = None;
    let duration = timing.motion(reduced_motion);
    if duration == 0 || (state.progress - target).abs() < f64::EPSILON {
        state.progress = target;
        state.tween = None;
        state.tween_from = target;
        state.tween_to = target;
        return;
    }
    state.tween = Some(Tween::new(now_ms, duration, timing.curve));
    state.tween_from = state.progress;
    state.tween_to = target;
}

fn set_desired(
    state: &mut OutputReveal,
    desired: bool,
    now_ms: u64,
    mode: AutoHideMode,
    timing: MenuBarTiming,
    reduced_motion: bool,
) -> bool {
    let mut changed = false;
    if mode == AutoHideMode::Never {
        if state.progress < 1.0
            || !state.desired_visible
            || state.pending.is_some()
            || state.tween.is_some()
        {
            state.settle(true);
            changed = true;
        }
        return changed;
    }
    if state.desired_visible != desired {
        state.desired_visible = desired;
        changed = true;
    }
    // Resolve any in-flight tween to the current frame before retargeting,
    // so a reversal starts from the visible position rather than the stale
    // stored one.
    let live_progress = state.current_progress(now_ms);
    if (live_progress - state.progress).abs() > f64::EPSILON {
        state.progress = live_progress;
        changed = true;
    }
    if desired {
        if state.progress >= 1.0 {
            state.pending = None;
            state.tween = None;
            return changed;
        }
        if state.tween.is_some() && state.tween_to >= 1.0 {
            return changed;
        }
        // A bar already partly on screen (a reversal) retargets
        // immediately; only a fully concealed bar waits for the dwell.
        let delay = if state.progress > 0.0 || state.tween.is_some() {
            0
        } else {
            timing.reveal_delay(reduced_motion)
        };
        if delay > 0 {
            state.tween = None;
            state.pending = Some(PendingTransition {
                at_ms: now_ms.saturating_add(delay),
                visible: true,
            });
            return changed;
        }
        start_tween(state, 1.0, now_ms, timing, reduced_motion);
    } else {
        if state.progress <= 0.0 {
            state.pending = None;
            state.tween = None;
            return changed;
        }
        if state.tween.is_some() && state.tween_to <= 0.0 {
            return changed;
        }
        let delay = timing.hide_delay(reduced_motion);
        if delay > 0 {
            state.tween = None;
            state.pending = Some(PendingTransition {
                at_ms: now_ms.saturating_add(delay),
                visible: false,
            });
            return changed;
        }
        start_tween(state, 0.0, now_ms, timing, reduced_motion);
    }
    changed
}

impl OutputReveal {
    fn is_animating(&self, now_ms: u64) -> bool {
        self.tween.is_some_and(|tween| !tween.is_done(now_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_timing() -> MenuBarTiming {
        MenuBarTiming {
            reveal_delay_ms: 0,
            hide_delay_ms: 200,
            motion_ms: 100,
            curve: [0.0, 0.0, 1.0, 1.0],
        }
    }

    #[test]
    fn never_is_visible_on_the_desktop_and_in_fullscreen() {
        for fullscreen in [false, true] {
            for revealed in [false, true] {
                for active in [false, true] {
                    assert!(menu_bar_visible(
                        AutoHideMode::Never,
                        fullscreen,
                        revealed,
                        active
                    ));
                }
            }
        }
    }

    #[test]
    fn always_starts_concealed_and_reveals_on_the_edge_or_focus() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(fast_timing());
        assert!(visibility.set_mode(AutoHideMode::Always));
        assert!(!visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.visible_on("eDP-1", true, false));

        // The pointer in the top band reveals only the owning output. The
        // reveal is immediate by reference; the motion is measured.
        assert!(visibility.update_pointer(0, "eDP-1", 0.5, false, false, true));
        assert!(visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.visible_on("HDMI-A-1", false, false));
        assert_eq!(visibility.progress_on("eDP-1"), 0.0);
        visibility.tick(100);
        assert!((visibility.progress_on("eDP-1") - 1.0).abs() < f64::EPSILON);

        // Moving below the bar keeps it up for the measured hide delay, then
        // slides it out.
        assert!(visibility.update_pointer(200, "eDP-1", 40.0, false, false, true));
        assert!(
            visibility.visible_on("eDP-1", false, false),
            "the hide delay holds the bar up"
        );
        visibility.tick(399);
        assert!(visibility.visible_on("eDP-1", false, false));
        visibility.tick(400);
        assert!(visibility.progress_on("eDP-1") > 0.0);
        visibility.tick(500);
        assert_eq!(visibility.progress_on("eDP-1"), 0.0);
        assert!(!visibility.visible_on("eDP-1", false, false));

        // An active menu/focus keeps it up even with the pointer elsewhere.
        assert!(visibility.update_pointer(600, "eDP-1", 40.0, false, true, true));
        assert!(visibility.visible_on("eDP-1", true, true));
    }

    #[test]
    fn a_nonzero_reveal_delay_is_measured_before_the_slide_starts() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(MenuBarTiming {
            reveal_delay_ms: 100,
            hide_delay_ms: 0,
            motion_ms: 100,
            curve: [0.0, 0.0, 1.0, 1.0],
        });
        visibility.set_mode(AutoHideMode::Always);
        visibility.update_pointer(0, "eDP-1", 0.0, false, false, true);
        // The reservation is live immediately (the bar is the pointer
        // target), but the slide does not start before the dwell.
        assert!(visibility.visible_on("eDP-1", false, false));
        assert!(visibility.is_animating_on("eDP-1", 50));
        visibility.tick(99);
        assert_eq!(visibility.progress_on("eDP-1"), 0.0);
        visibility.tick(100);
        assert_eq!(visibility.progress_on("eDP-1"), 0.0);
        visibility.tick(150);
        assert!((visibility.progress_on("eDP-1") - 0.5).abs() < 0.01);
        visibility.tick(200);
        assert_eq!(visibility.progress_on("eDP-1"), 1.0);
    }

    #[test]
    fn the_initial_pointer_position_does_not_count_as_an_edge_reveal() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(fast_timing());
        visibility.set_mode(AutoHideMode::Always);
        // The seat starts at (0,0); that is not a user edge gesture.
        assert!(!visibility.update_pointer(0, "eDP-1", 0.0, false, false, false));
        assert!(!visibility.visible_on("eDP-1", false, false));
        // The first real motion at the top edge reveals.
        assert!(visibility.update_pointer(0, "eDP-1", 0.0, false, false, true));
        assert!(visibility.visible_on("eDP-1", false, false));
    }

    #[test]
    fn full_screen_mode_is_normal_on_desktop_and_reveals_in_fullscreen() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(fast_timing());
        assert_eq!(visibility.mode(), AutoHideMode::FullScreen);
        // Desktop: visible with no transient reveal.
        visibility.settle("eDP-1", true);
        assert!(visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.update_pointer(0, "eDP-1", 200.0, false, false, true));
        // Fullscreen: the settled transition conceals it until the pointer
        // reaches the top edge.
        visibility.settle("eDP-1", false);
        assert!(!visibility.visible_on("eDP-1", true, false));
        assert!(visibility.update_pointer(100, "eDP-1", 0.0, true, false, true));
        assert!(visibility.visible_on("eDP-1", true, false));
        visibility.tick(200);
        assert_eq!(visibility.progress_on("eDP-1"), 1.0);
        // Leaving the edge conceals it again after the hide delay.
        assert!(visibility.update_pointer(300, "eDP-1", 40.0, true, false, true));
        visibility.tick(500);
        assert!(
            visibility.visible_on("eDP-1", true, false),
            "still visible while the slide-out runs"
        );
        visibility.tick(600);
        assert!(!visibility.visible_on("eDP-1", true, false));
        // An active menu keeps it revealed over fullscreen content.
        assert!(visibility.update_pointer(700, "eDP-1", 40.0, true, true, true));
        assert!(visibility.visible_on("eDP-1", true, true));
    }

    #[test]
    fn live_mode_changes_clear_the_old_reveal_and_apply_immediately() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(fast_timing());
        visibility.reveal("eDP-1");
        assert!(visibility.set_mode(AutoHideMode::Always));
        assert!(!visibility.is_revealed("eDP-1"));
        assert!(!visibility.visible_on("eDP-1", false, false));

        visibility.reveal("eDP-1");
        assert!(visibility.set_mode(AutoHideMode::Never));
        assert!(visibility.visible_on("eDP-1", true, false));

        assert!(visibility.set_mode(AutoHideMode::FullScreen));
        assert!(visibility.visible_on("eDP-1", false, false));
        assert!(!visibility.visible_on("eDP-1", true, false));
    }

    #[test]
    fn second_output_has_independent_reveal_and_reservation_input() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(fast_timing());
        visibility.set_mode(AutoHideMode::Always);
        visibility.reveal("HDMI-A-1");
        visibility.update_pointer(0, "HDMI-A-1", 0.0, false, false, true);
        visibility.tick(100);
        assert!(visibility.visible_on("HDMI-A-1", false, false));
        assert!(!visibility.visible_on("eDP-1", false, false));

        // The second output conceals without disturbing the first.
        assert!(visibility.update_pointer(200, "HDMI-A-1", 50.0, false, false, true));
        visibility.tick(400);
        visibility.tick(500);
        assert!(!visibility.visible_on("HDMI-A-1", false, false));
        assert!(visibility.reveal("eDP-1"));
        assert!(visibility.visible_on("eDP-1", false, false));
    }

    #[test]
    fn a_hotplugged_output_settles_to_its_own_resting_state() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_mode(AutoHideMode::Always);
        // The new output joins concealed until its own edge reveal.
        visibility.settle("DP-2", false);
        assert!(!visibility.visible_on("DP-2", false, false));
        // It reveals independently of the existing output.
        visibility.update_pointer(0, "DP-2", 0.0, false, false, true);
        assert!(visibility.visible_on("DP-2", false, false));
        assert!(!visibility.visible_on("eDP-1", false, false));
        // Removing it leaves no transition behind.
        visibility.settle("DP-2", false);
        assert!(!visibility.is_animating_on("DP-2", 1000));
    }

    #[test]
    fn lock_clears_reveal_and_unlock_rests_concealed() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(fast_timing());
        visibility.set_mode(AutoHideMode::Always);
        visibility.reveal("eDP-1");
        assert!(visibility.visible_on("eDP-1", false, false));

        // Lock drops every transient reveal; while locked nothing is drawn.
        assert!(visibility.clear());
        assert!(!visibility.clear());
        assert!(!visibility.visible_on("eDP-1", false, false));

        // Unlock starts from the resting state: hidden until the next edge
        // or focus reveal.
        assert!(!visibility.visible_on("eDP-1", false, false));
        assert!(visibility.update_pointer(0, "eDP-1", 0.0, false, false, true));
        assert!(visibility.visible_on("eDP-1", false, false));
    }

    #[test]
    fn reduced_motion_collapses_delays_and_motion_in_one_step() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(fast_timing());
        visibility.set_mode(AutoHideMode::Always);
        assert!(visibility.set_reduced_motion(true));

        visibility.update_pointer(0, "eDP-1", 0.0, false, false, true);
        // No pending delay and no tween: the first tick settles the reveal.
        assert!(!visibility.is_animating_on("eDP-1", 0));
        assert_eq!(visibility.progress_on("eDP-1"), 1.0);

        visibility.update_pointer(10, "eDP-1", 50.0, false, false, true);
        assert!(!visibility.is_animating_on("eDP-1", 10));
        assert_eq!(visibility.progress_on("eDP-1"), 0.0);
        assert!(!visibility.visible_on("eDP-1", false, false));
    }

    #[test]
    fn the_surface_translation_follows_the_reveal_progress() {
        let mut visibility = MenuBarVisibility::default();
        visibility.set_timing(MenuBarTiming {
            reveal_delay_ms: 0,
            hide_delay_ms: 0,
            motion_ms: 100,
            curve: [0.0, 0.0, 1.0, 1.0],
        });
        visibility.set_mode(AutoHideMode::Always);
        visibility.update_pointer(0, "eDP-1", 0.0, false, false, true);
        assert_eq!(visibility.hide_offset_on("eDP-1", 95), 95);
        visibility.tick(50);
        assert_eq!(visibility.hide_offset_on("eDP-1", 95), 48);
        visibility.tick(100);
        assert_eq!(visibility.hide_offset_on("eDP-1", 95), 0);
    }

    #[test]
    fn concealed_bar_contributes_no_reserved_zone() {
        use crate::shell::layer::{
            aggregate_reserved_for, LayerSurfaceState, ANCHOR_LEFT, ANCHOR_RIGHT, ANCHOR_TOP,
        };
        use crate::window::ReservedZones;

        let bar = LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT,
            height: MENU_BAR_REVEAL_HEIGHT,
            exclusive_zone: MENU_BAR_REVEAL_HEIGHT,
            ..Default::default()
        };
        let mut visibility = MenuBarVisibility::default();
        visibility.set_mode(AutoHideMode::Always);

        let visible = visibility.visible_on("eDP-1", false, false);
        let layers = [&bar].into_iter().filter(|_| visible);
        let zones = aggregate_reserved_for("eDP-1", layers, ReservedZones::default());
        assert_eq!(zones.top, 0, "a concealed bar must not reserve the strip");

        visibility.reveal("eDP-1");
        let visible = visibility.visible_on("eDP-1", false, false);
        let layers = [&bar].into_iter().filter(|_| visible);
        let zones = aggregate_reserved_for("eDP-1", layers, ReservedZones::default());
        assert_eq!(
            zones.top, MENU_BAR_REVEAL_HEIGHT,
            "a revealed bar reserves its strip again"
        );
    }
}
