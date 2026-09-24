// SPDX-License-Identifier: MIT
//! The shared compositor animation clock (T-02.1a).
//!
//! Every compositor-side lifecycle transition (window appear/minimize/zoom/
//! close in T-02, the overview slide in T-11, the scene transforms in T-04)
//! runs on **one** frame-scheduled clock. The discipline it enforces is the
//! one the design docs require ([02-compositor.md]):
//!
//! * **one compositor frame per animation frame** — a clock tick advances
//!   every running animation exactly once and requests exactly one render
//!   pass;
//! * **no animation ⇔ no damage** — the clock arms a single calloop timer
//!   only while an animation is live, so an idle desktop renders nothing
//!   and wakes no clients (FR-2);
//! * **reduced motion** — the single flag (`accessibility.reduceMotion`)
//!   collapses a transition to one step through the same commit path; the
//!   moved value is what changes, not the code path.
//!
//! The clock itself is pure timing plus counters; the visible effect of an
//! animation is applied by the animation's own [`Animation::advance`]. The
//! calloop half lives in [`crate::input`] (`schedule_animation_timer` /
//! `poll_animations`), because a frame is only real once the session loop
//! renders it.
#![allow(dead_code)] // Forward-looking API consumed by T-02.1b…T-02.4/T-04/T-05.

use std::time::Duration;

use crate::design_tokens::Motion;

/// The animation frame interval (~60 Hz). One compositor frame is rendered
/// per elapsed interval while an animation is live.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Evaluate a cubic-bezier easing curve `[x1, y1, x2, y2]` at `t` in
/// `0.0..=1.0` (the design-system [`Motion::curve`]). Solve `x(u) = t` with
/// Newton's method and fall back to bisection when the curve is flat, then
/// evaluate `y(u)`.
pub fn ease(curve: [f32; 4], t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let [x1, y1, x2, y2] = curve.map(f64::from);
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    // An identity curve (control points on the diagonal) eases linearly;
    // this also covers the common `[0, 0, 1, 1]` value.
    if (x1 - y1).abs() < f64::EPSILON && (x2 - y2).abs() < f64::EPSILON {
        return t;
    }
    let component = |u: f64, a1: f64, a2: f64| {
        let mu = 1.0 - u;
        3.0 * mu * mu * u * a1 + 3.0 * mu * u * u * a2 + u * u * u
    };
    let derivative = |u: f64, a1: f64, a2: f64| {
        let mu = 1.0 - u;
        3.0 * mu * mu * a1 + 6.0 * mu * u * (a2 - a1) + 3.0 * u * u * (1.0 - a2)
    };
    // Newton–Raphson, then bisection if it did not converge (flat regions).
    let mut u = t;
    for _ in 0..8 {
        let x = component(u, x1, x2) - t;
        if x.abs() < 1e-6 {
            return component(u, y1, y2).clamp(0.0, 1.0);
        }
        let d = derivative(u, x1, x2);
        if d.abs() < 1e-6 {
            break;
        }
        u = (u - x / d).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    let mut u = t;
    for _ in 0..32 {
        let x = component(u, x1, x2);
        if (x - t).abs() < 1e-6 {
            break;
        }
        if x < t {
            lo = u;
        } else {
            hi = u;
        }
        u = 0.5 * (lo + hi);
    }
    component(u, y1, y2).clamp(0.0, 1.0)
}

/// A pure timing primitive shared by every lifecycle transition: a start
/// time, a duration, and a design-system easing curve.
///
/// `progress`/`is_done` are computed from the session monotonic clock, so a
/// transition is interruptible and retargetable without bookkeeping: a new
/// `Tween` replaces the old one and the curve restarts from the new origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    pub start_ms: u64,
    pub duration_ms: u64,
    pub curve: [f32; 4],
}

impl Tween {
    pub const fn new(start_ms: u64, duration_ms: u64, curve: [f32; 4]) -> Self {
        Tween {
            start_ms,
            duration_ms,
            curve,
        }
    }

    /// Build a tween from a design-system motion token. Under reduced motion
    /// the token's `reduced_duration_ms` is used, which is `0` for every
    /// current token — the transition then completes on its first step.
    pub fn from_motion(start_ms: u64, motion: Motion, reduced_motion: bool) -> Self {
        let duration_ms = if reduced_motion {
            u64::from(motion.reduced_duration_ms)
        } else {
            u64::from(motion.duration_ms)
        };
        Tween::new(start_ms, duration_ms, motion.curve)
    }

    /// Clamped linear progress in `0.0..=1.0` (a zero-duration tween is
    /// immediately complete).
    pub fn raw_progress(&self, now_ms: u64) -> f64 {
        if self.duration_ms == 0 {
            return 1.0;
        }
        let elapsed = now_ms.saturating_sub(self.start_ms);
        (elapsed as f64 / self.duration_ms as f64).clamp(0.0, 1.0)
    }

    /// Eased progress in `0.0..=1.0`.
    pub fn progress(&self, now_ms: u64) -> f64 {
        ease(self.curve, self.raw_progress(now_ms))
    }

    /// Whether the transition has reached its end time.
    pub fn is_done(&self, now_ms: u64) -> bool {
        self.raw_progress(now_ms) >= 1.0
    }

    /// The monotonic time the transition reaches 1.0.
    pub fn finish_ms(&self) -> u64 {
        self.start_ms.saturating_add(self.duration_ms)
    }
}

/// One running animation, advanced once per clock frame.
///
/// `advance` returns `true` while the animation is still running. The state
/// type `S` is the compositor state the animation may mutate; the blanket
/// impl below lets a closure be used directly, and scene-free animations use
/// [`TweenAnimation`].
///
/// An `advance` implementation must not start another animation: a new
/// animation is picked up on the next frame. (Starting one mid-step is
/// absorbed, but keeping the rule avoids relying on that.)
pub trait Animation<S> {
    fn advance(&mut self, state: &mut S, now_ms: u64) -> bool;
}

impl<S, F> Animation<S> for F
where
    F: FnMut(&mut S, u64) -> bool,
{
    fn advance(&mut self, state: &mut S, now_ms: u64) -> bool {
        self(state, now_ms)
    }
}

/// A scene-free animation that runs a [`Tween`] to completion.
///
/// Used as the clock's calibration dummy (the synthetic-input `animate-dummy`
/// command) and as the liveness half of an animation whose visible effect is
/// applied by a separate pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TweenAnimation {
    pub tween: Tween,
}

impl TweenAnimation {
    pub const fn new(tween: Tween) -> Self {
        TweenAnimation { tween }
    }
}

impl<S> Animation<S> for TweenAnimation {
    fn advance(&mut self, _state: &mut S, now_ms: u64) -> bool {
        !self.tween.is_done(now_ms)
    }
}

/// The shared animation clock: frame interval, reduced-motion policy, the
/// running-animation set, and the frame counters that prove the discipline.
///
/// The clock owns no timer itself; [`crate::input::schedule_animation_timer`]
/// arms the calloop timer from [`Self::is_active`], and every tick calls
/// [`Self::step`] once and renders once.
pub struct AnimationClock<S> {
    animations: Vec<Box<dyn Animation<S>>>,
    reduced_motion: bool,
    frames_stepped: u64,
    animations_started: u64,
    animations_completed: u64,
}

impl<S> Default for AnimationClock<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> AnimationClock<S> {
    pub fn new() -> Self {
        AnimationClock {
            animations: Vec::new(),
            reduced_motion: false,
            frames_stepped: 0,
            animations_started: 0,
            animations_completed: 0,
        }
    }

    /// The animation frame interval (fixed at [`FRAME_INTERVAL`] today; a
    /// settable interval is the output-refresh-rate hook for T-16).
    pub fn frame_interval(&self) -> Duration {
        FRAME_INTERVAL
    }

    /// The shared reduced-motion flag (`accessibility.reduceMotion`, T-08).
    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    /// Whether any animation is running. The timer is armed exactly while
    /// this is true, which is what keeps idle damage at zero.
    pub fn is_active(&self) -> bool {
        !self.animations.is_empty()
    }

    /// Start an animation. The caller (usually
    /// [`crate::state::DfState::start_animation`]) arms the timer.
    pub fn start(&mut self, animation: impl Animation<S> + 'static) {
        self.animations.push(Box::new(animation));
        self.animations_started += 1;
    }

    /// Advance every running animation one frame at `now_ms`. Returns true
    /// while any registered animation remains live.
    ///
    /// The driver calls this exactly once per armed clock tick (a tick can
    /// also be armed by the overview's discrete slide, which has its own
    /// progress pipeline), so `frames_stepped` counts every animation frame
    /// the compositor rendered for.
    pub fn step(&mut self, state: &mut S, now_ms: u64) -> bool {
        let before = self.animations.len();
        let mut animations = std::mem::take(&mut self.animations);
        animations.retain_mut(|animation| animation.advance(state, now_ms));
        let completed = before - animations.len();
        self.animations = animations;
        self.frames_stepped += 1;
        self.animations_completed += completed as u64;
        !self.animations.is_empty()
    }

    /// Move the animations that started in `other` (a placeholder swapped in
    /// during [`Self::step`]) into this clock. Keeps an animation started
    /// from inside another animation's `advance` from being lost.
    pub fn absorb(&mut self, other: &mut AnimationClock<S>) {
        self.animations_started += other.animations_started;
        self.animations.append(&mut other.animations);
    }

    /// Animation frames stepped since startup (one per rendered frame while
    /// an animation was live).
    pub fn frames_stepped(&self) -> u64 {
        self.frames_stepped
    }

    pub fn animations_started(&self) -> u64 {
        self.animations_started
    }

    pub fn animations_completed(&self) -> u64 {
        self.animations_completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dummy animation with a known frame count, so the clock's
    /// one-frame-per-tick discipline is unit-testable.
    struct Dummy {
        tween: Tween,
        frames: u64,
    }

    impl Animation<u32> for Dummy {
        fn advance(&mut self, ticks: &mut u32, now_ms: u64) -> bool {
            *ticks += 1;
            self.frames += 1;
            !self.tween.is_done(now_ms)
        }
    }

    #[test]
    fn ease_hits_the_endpoints_and_is_monotonic() {
        let curve = [0.2, 0.0, 0.0, 1.0];
        assert_eq!(ease(curve, 0.0), 0.0);
        assert_eq!(ease(curve, 1.0), 1.0);
        let mut previous = 0.0;
        for step in 1..=100 {
            let value = ease(curve, step as f64 / 100.0);
            assert!((0.0..=1.0).contains(&value));
            assert!(value >= previous - 1e-9);
            previous = value;
        }
        // A linear curve is the identity.
        assert!((ease([0.0, 0.0, 1.0, 1.0], 0.25) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn tween_clamps_and_completes() {
        let tween = Tween::new(1000, 200, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(tween.raw_progress(900), 0.0, "before the start is clamped");
        assert_eq!(tween.raw_progress(1100), 0.5);
        assert_eq!(tween.raw_progress(1300), 1.0);
        assert_eq!(tween.raw_progress(9999), 1.0, "after the end is clamped");
        assert!(!tween.is_done(1199));
        assert!(tween.is_done(1200));
        assert_eq!(tween.finish_ms(), 1200);
    }

    #[test]
    fn zero_duration_tween_is_instantly_done() {
        let tween = Tween::new(1000, 0, [0.2, 0.0, 0.0, 1.0]);
        assert!(tween.is_done(1000));
        assert_eq!(tween.progress(1000), 1.0);
    }

    #[test]
    fn reduced_motion_collapses_a_motion_token_to_one_step() {
        use crate::design_tokens::motion;
        let full = Tween::from_motion(0, motion::WINDOW_OPEN, false);
        let reduced = Tween::from_motion(0, motion::WINDOW_OPEN, true);
        assert_eq!(full.duration_ms, u64::from(motion::WINDOW_OPEN.duration_ms));
        assert_eq!(reduced.duration_ms, 0);
        assert!(reduced.is_done(0), "reduced motion completes immediately");
        assert!(!full.is_done(0));
    }

    /// The acceptance assertion as a unit test: one clock tick = one frame,
    /// and no ticks once the animation settles.
    #[test]
    fn clock_steps_exactly_one_frame_per_tick() {
        let mut clock: AnimationClock<u32> = AnimationClock::new();
        let mut ticks = 0;
        // 48 ms at 16 ms/frame: ticks at 16, 32 and 48 → three frames.
        clock.start(Dummy {
            tween: Tween::new(0, 48, [0.0, 0.0, 1.0, 1.0]),
            frames: 0,
        });
        assert!(clock.is_active());
        assert_eq!(clock.frames_stepped(), 0, "idle clock rendered no frame");

        assert!(clock.step(&mut ticks, 16));
        assert!(clock.step(&mut ticks, 32));
        assert!(!clock.step(&mut ticks, 48), "the curve reached 1.0");
        assert!(!clock.is_active(), "the timer must stop here");

        assert_eq!(ticks, 3, "one advance per tick, never more");
        assert_eq!(clock.frames_stepped(), 3, "one compositor frame per tick");
        assert_eq!(clock.animations_started(), 1);
        assert_eq!(clock.animations_completed(), 1);
    }

    #[test]
    fn an_idle_clock_has_nothing_to_step() {
        let clock: AnimationClock<u32> = AnimationClock::new();
        assert!(!clock.is_active());
        assert_eq!(clock.frames_stepped(), 0);
        assert_eq!(clock.animations_started(), 0);
    }
}
