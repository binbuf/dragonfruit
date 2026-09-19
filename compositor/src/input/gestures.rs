// SPDX-License-Identifier: MIT OR Apache-2.0
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! Gesture recognition and the shared progress pipeline (T-03).
//!
//! Trackpad swipes and pinches are recognized here, in the compositor,
//! and fed into **the same** [`ProgressPipeline`] that keyboard and
//! hot-corner triggers drive ([03-workspaces.md](../../.docs/design/03-workspaces.md)).
//! There is no gesture-only code path: every trigger produces the same
//! clamped 0→1 progress events with velocity, so a workspace switch is
//! continuous and reversible regardless of how it started.
//!
//! Thresholds are parameterized ([`GestureConfig`], [`ProgressConfig`]);
//! on-device tuning adjusts the settings, never the code.

use smithay::utils::{Logical, Point};

use super::action::{GestureKind, InputAction, TriggerKind};

/// Tunables for gesture recognition. Defaults are a starting point for
/// on-device tuning against the commit rules in
/// [03-workspaces.md](../../.docs/design/03-workspaces.md).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureConfig {
    /// Horizontal travel (logical px) for a full 0→1 workspace swipe.
    pub swipe_threshold: f64,
    /// Cumulative pinch scale for a full 0→1 transition.
    pub pinch_threshold: f64,
    /// Movement below this (px) does not lock the gesture axis.
    pub axis_deadzone: f64,
    /// Dominance ratio used to lock the axis once movement is clear.
    pub axis_lock_ratio: f64,
}

impl Default for GestureConfig {
    fn default() -> Self {
        GestureConfig {
            swipe_threshold: 300.0,
            pinch_threshold: 0.6,
            axis_deadzone: 10.0,
            axis_lock_ratio: 1.0,
        }
    }
}

/// Tunables for the progress pipeline's clamp/rubber-band/commit rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressConfig {
    /// Rubber-band strength when swiping past the first/last Space.
    pub rubber_band: f64,
    /// Commit if released at or past this progress.
    pub commit_progress: f64,
    /// Commit if release velocity (progress/second) is at least this.
    pub commit_velocity: f64,
    /// Duration used when a discrete trigger (keyboard/hot corner) drives
    /// the same pipeline.
    pub discrete_duration_ms: u64,
    /// Steps in a discrete drive; more steps = smoother animation.
    pub discrete_steps: u32,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        ProgressConfig {
            rubber_band: 0.35,
            commit_progress: 0.4,
            commit_velocity: 0.8,
            discrete_duration_ms: 250,
            discrete_steps: 5,
        }
    }
}

/// The phase of a progress event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressPhase {
    Begin,
    Update,
    End,
}

/// One progress event on the shared pipeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressEvent {
    pub action: InputAction,
    pub trigger: TriggerKind,
    pub phase: ProgressPhase,
    /// Clamped to `0.0..=1.0` (FR-4).
    pub progress: f64,
    /// Rubber-banded raw progress; may slightly exceed `0..1` past an end.
    pub raw_progress: f64,
    /// Progress units per second.
    pub velocity: f64,
    /// Only meaningful for [`ProgressPhase::End`]: the release crossed the
    /// commit threshold.
    pub committed: bool,
    pub cancelled: bool,
}

/// The single overview/workspace progress state machine.
#[derive(Debug, Clone)]
pub struct ProgressPipeline {
    config: ProgressConfig,
    active: Option<ActiveProgress>,
}

#[derive(Debug, Clone)]
struct ActiveProgress {
    action: InputAction,
    trigger: TriggerKind,
    raw: f64,
    progress: f64,
    last_progress: f64,
    velocity: f64,
    start_time: u64,
    last_time: u64,
}

impl Default for ProgressPipeline {
    fn default() -> Self {
        ProgressPipeline::new(ProgressConfig::default())
    }
}

impl ProgressPipeline {
    pub fn new(config: ProgressConfig) -> Self {
        ProgressPipeline {
            config,
            active: None,
        }
    }

    pub fn config(&self) -> ProgressConfig {
        self.config
    }

    pub fn set_config(&mut self, config: ProgressConfig) {
        self.config = config;
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn action(&self) -> Option<InputAction> {
        self.active.as_ref().map(|active| active.action)
    }

    pub fn progress(&self) -> f64 {
        self.active.as_ref().map_or(0.0, |active| active.progress)
    }

    /// Start a transition. Returns the `Begin` event.
    pub fn begin(&mut self, action: InputAction, trigger: TriggerKind, time: u64) -> ProgressEvent {
        self.active = Some(ActiveProgress {
            action,
            trigger,
            raw: 0.0,
            progress: 0.0,
            last_progress: 0.0,
            velocity: 0.0,
            start_time: time,
            last_time: time,
        });
        self.event(ProgressPhase::Begin, time, false, false)
    }

    /// Advance by a progress delta (gesture input). Applies rubber-band
    /// clamping and computes velocity.
    pub fn update(&mut self, delta: f64, time: u64) -> Option<ProgressEvent> {
        let band = self.config.rubber_band;
        let active = self.active.as_mut()?;
        active.raw += delta;
        active.progress = clamp_unit(rubber_band(active.raw, band));
        active.velocity = velocity(
            active.progress,
            active.last_progress,
            active.last_time,
            time,
        );
        active.last_progress = active.progress;
        active.last_time = time;
        Some(self.event(ProgressPhase::Update, time, false, false))
    }

    /// Set an absolute progress (discrete triggers use this so keyboard,
    /// hot-corner, and gesture triggers share one code path).
    pub fn set_progress(&mut self, progress: f64, time: u64) -> Option<ProgressEvent> {
        let active = self.active.as_mut()?;
        active.raw = progress;
        active.progress = clamp_unit(progress);
        active.velocity = velocity(
            active.progress,
            active.last_progress,
            active.last_time,
            time,
        );
        active.last_progress = active.progress;
        active.last_time = time;
        Some(self.event(ProgressPhase::Update, time, false, false))
    }

    /// Finish the transition. `cancelled` forces a non-committed release.
    pub fn end(&mut self, time: u64, cancelled: bool) -> Option<ProgressEvent> {
        let active = self.active.take()?;
        let commit_progress = self.config.commit_progress;
        let commit_velocity = self.config.commit_velocity;
        let committed = !cancelled
            && (active.progress >= commit_progress || active.velocity >= commit_velocity);
        let event = ProgressEvent {
            action: active.action,
            trigger: active.trigger,
            phase: ProgressPhase::End,
            progress: active.progress,
            raw_progress: active.raw,
            velocity: active.velocity,
            committed,
            cancelled,
        };
        let _ = (time, commit_progress, commit_velocity);
        Some(event)
    }

    /// Drive a discrete action through the exact same pipeline: begin,
    /// `steps` updates to 1.0, then a committed end. This is what keyboard
    /// shortcuts and hot corners use.
    pub fn drive_discrete(
        &mut self,
        action: InputAction,
        trigger: TriggerKind,
        start_time: u64,
    ) -> Vec<ProgressEvent> {
        let steps = self.config.discrete_steps.max(1);
        let duration = self.config.discrete_duration_ms;
        let mut events = vec![self.begin(action, trigger, start_time)];
        for step in 1..=steps {
            let time = start_time + duration * u64::from(step) / u64::from(steps);
            if let Some(event) = self.set_progress(f64::from(step) / f64::from(steps), time) {
                events.push(event);
            }
        }
        if let Some(event) = self.end(start_time + duration, false) {
            events.push(event);
        }
        events
    }

    fn event(
        &self,
        phase: ProgressPhase,
        _time: u64,
        committed: bool,
        cancelled: bool,
    ) -> ProgressEvent {
        let active = self.active.as_ref().expect("active progress");
        ProgressEvent {
            action: active.action,
            trigger: active.trigger,
            phase,
            progress: active.progress,
            raw_progress: active.raw,
            velocity: active.velocity,
            committed,
            cancelled,
        }
    }
}

fn clamp_unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// Rubber-band a raw progress value: identity in `0..=1`, asymptotically
/// damped past either end.
fn rubber_band(raw: f64, band: f64) -> f64 {
    if raw < 0.0 {
        -rubber_amount(-raw, band)
    } else if raw > 1.0 {
        1.0 + rubber_amount(raw - 1.0, band)
    } else {
        raw
    }
}

fn rubber_amount(x: f64, band: f64) -> f64 {
    if band <= 0.0 {
        0.0
    } else {
        x * band / (1.0 + x * band)
    }
}

fn velocity(progress: f64, last_progress: f64, last_time: u64, time: u64) -> f64 {
    let dt = time.saturating_sub(last_time) as f64 / 1000.0;
    if dt <= f64::EPSILON {
        0.0
    } else {
        (progress - last_progress) / dt
    }
}

/// Which axis a swipe locked onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureAxis {
    Horizontal,
    Vertical,
}

/// A recognized gesture step, ready to feed the progress pipeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureUpdate {
    pub kind: GestureKind,
    pub action: InputAction,
    pub axis: GestureAxis,
    /// Progress delta in the direction of `action` (always >= 0).
    pub progress_delta: f64,
    pub time: u64,
}

/// Recognizes libinput swipe/pinch streams into progress deltas.
#[derive(Debug, Clone)]
pub struct GestureRecognizer {
    config: GestureConfig,
    active: Option<ActiveGesture>,
}

#[derive(Debug, Clone)]
struct ActiveGesture {
    kind: GestureKind,
    axis: Option<GestureAxis>,
    last_scale: f64,
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        GestureRecognizer::new(GestureConfig::default())
    }
}

impl GestureRecognizer {
    pub fn new(config: GestureConfig) -> Self {
        GestureRecognizer {
            config,
            active: None,
        }
    }

    pub fn config(&self) -> GestureConfig {
        self.config
    }

    pub fn set_config(&mut self, config: GestureConfig) {
        self.config = config;
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn kind(&self) -> Option<GestureKind> {
        self.active.as_ref().map(|active| active.kind)
    }

    /// Begin a swipe; the axis and action resolve on the first clear update.
    pub fn begin_swipe(&mut self, fingers: u32, _time: u64) {
        self.active = Some(ActiveGesture {
            kind: GestureKind::Swipe { fingers },
            axis: None,
            last_scale: 1.0,
        });
    }

    /// Begin a pinch.
    pub fn begin_pinch(&mut self, fingers: u32, _time: u64) {
        self.active = Some(ActiveGesture {
            kind: GestureKind::Pinch { fingers },
            axis: None,
            last_scale: 1.0,
        });
    }

    /// Feed a swipe update; returns a progress step once the gesture is
    /// claimed by a system action.
    pub fn update_swipe(&mut self, delta: Point<f64, Logical>, time: u64) -> Option<GestureUpdate> {
        let config = self.config;
        let active = self.active.as_mut()?;
        let GestureKind::Swipe { fingers } = active.kind else {
            return None;
        };

        let axis = match active.axis {
            Some(axis) => axis,
            None => {
                if delta.x.abs() < config.axis_deadzone && delta.y.abs() < config.axis_deadzone {
                    return None;
                }
                let axis = if delta.x.abs() >= delta.y.abs() * config.axis_lock_ratio {
                    GestureAxis::Horizontal
                } else {
                    GestureAxis::Vertical
                };
                active.axis = Some(axis);
                axis
            }
        };

        let (action, component) = match (fingers, axis) {
            // Three/four-finger horizontal swipes switch Spaces.
            (3 | 4, GestureAxis::Horizontal) if delta.x < 0.0 => {
                (InputAction::WorkspaceNext, -delta.x)
            }
            (3 | 4, GestureAxis::Horizontal) => (InputAction::WorkspacePrev, delta.x),
            // Four-finger vertical swipes open Mission Control / reveal.
            (4, GestureAxis::Vertical) if delta.y < 0.0 => (InputAction::MissionControl, -delta.y),
            (4, GestureAxis::Vertical) => (InputAction::DesktopReveal, delta.y),
            _ => return None,
        };

        Some(GestureUpdate {
            kind: active.kind,
            action,
            axis,
            progress_delta: component / config.swipe_threshold,
            time,
        })
    }

    /// Feed a pinch update; `scale` is cumulative since gesture start.
    pub fn update_pinch(&mut self, scale: f64, _rotation: f64, time: u64) -> Option<GestureUpdate> {
        let config = self.config;
        let active = self.active.as_mut()?;
        let GestureKind::Pinch { fingers: _ } = active.kind else {
            return None;
        };
        let delta = scale - active.last_scale;
        active.last_scale = scale;
        let (action, component) = if delta >= 0.0 {
            (InputAction::MissionControl, delta)
        } else {
            (InputAction::DesktopReveal, -delta)
        };
        Some(GestureUpdate {
            kind: active.kind,
            action,
            axis: GestureAxis::Vertical,
            progress_delta: component / config.pinch_threshold,
            time,
        })
    }

    /// End the active gesture, returning its kind if there was one.
    pub fn end(&mut self) -> Option<GestureKind> {
        self.active.take().map(|active| active.kind)
    }

    pub fn cancel(&mut self) {
        self.active = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ProgressConfig {
        ProgressConfig {
            rubber_band: 0.35,
            commit_progress: 0.4,
            commit_velocity: 0.8,
            discrete_duration_ms: 200,
            discrete_steps: 4,
        }
    }

    #[test]
    fn progress_is_clamped_with_rubber_band() {
        let mut pipeline = ProgressPipeline::new(cfg());
        pipeline.begin(InputAction::WorkspaceNext, TriggerKind::Keyboard, 0);
        let event = pipeline.update(2.0, 100).unwrap();
        assert!(event.progress <= 1.0 && event.progress > 0.9);
        assert!(event.raw_progress > 1.0, "rubber band overshoots in raw");
        let event = pipeline.update(-5.0, 200).unwrap();
        assert_eq!(event.progress, 0.0);
        assert!(event.raw_progress < 0.0);
    }

    #[test]
    fn discrete_and_gesture_triggers_share_one_curve() {
        // A gesture that reaches the same raw progress values at the same
        // times must produce exactly the same events as a discrete drive.
        let mut gesture = ProgressPipeline::new(cfg());
        let trigger = TriggerKind::Gesture(GestureKind::Swipe { fingers: 4 });
        let mut gesture_curve = vec![gesture.begin(InputAction::MissionControl, trigger, 0)];
        for step in 1..=4u64 {
            let time = step * 50;
            gesture_curve.push(gesture.set_progress(step as f64 / 4.0, time).unwrap());
        }
        gesture_curve.push(gesture.end(250, false).unwrap());

        let mut discrete = ProgressPipeline::new(cfg());
        discrete.config.discrete_duration_ms = 200;
        discrete.config.discrete_steps = 4;
        let discrete_curve = discrete.drive_discrete(
            InputAction::MissionControl,
            TriggerKind::HotCorner(super::super::action::HotCorner::TopLeft),
            0,
        );

        let g: Vec<(f64, f64)> = gesture_curve
            .iter()
            .map(|e| (e.progress, e.velocity))
            .collect();
        let d: Vec<(f64, f64)> = discrete_curve
            .iter()
            .map(|e| (e.progress, e.velocity))
            .collect();
        assert_eq!(g, d);
    }

    #[test]
    fn three_finger_horizontal_swipe_switches_spaces() {
        let mut recognizer = GestureRecognizer::default();
        recognizer.begin_swipe(3, 0);
        let update = recognizer
            .update_swipe((-50.0, 5.0).into(), 10)
            .expect("claimed");
        assert_eq!(update.action, InputAction::WorkspaceNext);
        assert_eq!(update.axis, GestureAxis::Horizontal);
        assert!(update.progress_delta > 0.0);
    }

    #[test]
    fn four_finger_vertical_swipe_opens_mission_control() {
        let mut recognizer = GestureRecognizer::default();
        recognizer.begin_swipe(4, 0);
        let update = recognizer
            .update_swipe((2.0, -60.0).into(), 10)
            .expect("claimed");
        assert_eq!(update.action, InputAction::MissionControl);
        assert_eq!(update.axis, GestureAxis::Vertical);
    }

    #[test]
    fn unclaimed_gestures_are_not_claimed() {
        let mut recognizer = GestureRecognizer::default();
        recognizer.begin_swipe(3, 0);
        assert!(recognizer.update_swipe((2.0, -60.0).into(), 10).is_none());
    }

    #[test]
    fn commit_rules_match_progress_or_velocity() {
        let mut pipeline = ProgressPipeline::new(cfg());
        pipeline.begin(InputAction::WorkspaceNext, TriggerKind::Keyboard, 0);
        pipeline.set_progress(0.5, 100);
        assert!(pipeline.end(200, false).unwrap().committed);

        let mut pipeline = ProgressPipeline::new(cfg());
        pipeline.begin(InputAction::WorkspaceNext, TriggerKind::Keyboard, 0);
        pipeline.set_progress(0.1, 200);
        assert!(!pipeline.end(20, false).unwrap().committed);
        assert!(pipeline.end(0, true).is_none());
    }
}
