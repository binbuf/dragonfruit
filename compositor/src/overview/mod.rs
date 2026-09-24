// SPDX-License-Identifier: MIT
//! Mission Control and workspace switching: **one** overview state machine
//! (T-11).
//!
//! The design rule is absolute: gesture, keyboard shortcut, hot corner, the
//! menu-bar Mission Control button, and the shell's private-protocol
//! requests all drive the *same* progress pipeline
//! ([03-workspaces.md](../../docs/design/03-workspaces.md)). There is no
//! second, discrete "instant" code path. This module owns that machine.
//!
//! It wraps the T-03 [`ProgressPipeline`] (clamp → rubber-band → velocity →
//! commit) and adds the overview-specific decisions on top:
//!
//! * what a transition *is* ([`OverviewKind`]: adjacent-Space switch,
//!   Mission Control, Desktop Reveal),
//! * what a release *asks the compositor to apply* ([`TransitionCommit`]),
//! * who owns pointer hit-testing while the overview is up
//!   ([`InputOwner`]), and
//! * the selection round-trip state (FR-5).
//!
//! It is deliberately pure (no Smithay scene access), so the trigger-parity,
//! commit, interruptibility, and hit-testing rules are unit-testable without
//! a live compositor.

#![allow(dead_code)] // Forward-looking API consumed by T-12/T-14/T-16.

pub mod grid;

use crate::input::action::{InputAction, TriggerKind};
use crate::input::gestures::{ProgressConfig, ProgressEvent, ProgressPipeline};
use crate::window::WindowId;

/// What a progress transition drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewKind {
    /// Slide to the next Space.
    WorkspaceNext,
    /// Slide to the previous Space.
    WorkspacePrev,
    /// Shrink the visible Space and reveal the overview.
    MissionControl,
    /// Push the windows aside to reveal the desktop (T-14).
    DesktopReveal,
}

impl OverviewKind {
    /// Resolve the compositor action to a transition kind, if it drives one.
    pub const fn from_action(action: InputAction) -> Option<Self> {
        match action {
            InputAction::WorkspaceNext => Some(OverviewKind::WorkspaceNext),
            InputAction::WorkspacePrev => Some(OverviewKind::WorkspacePrev),
            InputAction::MissionControl => Some(OverviewKind::MissionControl),
            InputAction::DesktopReveal => Some(OverviewKind::DesktopReveal),
            _ => None,
        }
    }

    /// The action this kind is recorded under in the audit log/shell stream.
    pub const fn action(self) -> InputAction {
        match self {
            OverviewKind::WorkspaceNext => InputAction::WorkspaceNext,
            OverviewKind::WorkspacePrev => InputAction::WorkspacePrev,
            OverviewKind::MissionControl => InputAction::MissionControl,
            OverviewKind::DesktopReveal => InputAction::DesktopReveal,
        }
    }

    /// The Space step a workspace switch takes: `+1` next, `-1` previous,
    /// `0` for the overview transitions (which do not change the Space).
    pub const fn direction(self) -> i32 {
        match self {
            OverviewKind::WorkspaceNext => 1,
            OverviewKind::WorkspacePrev => -1,
            OverviewKind::MissionControl | OverviewKind::DesktopReveal => 0,
        }
    }

    /// Whether this transition puts the overview controller in charge of
    /// pointer input while it runs (Mission Control / Desktop Reveal), as
    /// opposed to a workspace slide, which keeps the normal focus path.
    pub const fn is_overview(self) -> bool {
        matches!(
            self,
            OverviewKind::MissionControl | OverviewKind::DesktopReveal
        )
    }

    /// Whether this is an adjacent-Space slide.
    pub const fn is_workspace_switch(self) -> bool {
        matches!(
            self,
            OverviewKind::WorkspaceNext | OverviewKind::WorkspacePrev
        )
    }
}

/// The side effect a *committed* transition asks the compositor to apply.
///
/// A release that does not cross the commit threshold returns `None`: the
/// transition animates back and nothing is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionCommit {
    /// Advance the active Space by this many steps (lockstep, T-05).
    SwitchWorkspace(i32),
    /// Enter (`true`) or leave (`false`) the Mission Control overview.
    SetOverview(bool),
    /// Reveal (`true`) or hide (`false`) the desktop.
    SetDesktopReveal(bool),
}

/// Who owns pointer hit-testing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOwner {
    /// The normal focus path: chrome, then windows.
    Normal,
    /// The overview controller owns pointer input; windows are not
    /// hit-tested until ownership returns explicitly (FR-6).
    Overview,
}

/// The events plus the side effect of driving one transition.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DriveOutcome {
    pub events: Vec<ProgressEvent>,
    /// `Some` only when the release crossed the commit threshold.
    pub commit: Option<TransitionCommit>,
}

/// An in-flight discrete transition (keyboard, hot corner, shell request).
///
/// The transition is advanced by the compositor's animation clock rather than
/// run synchronously, so a keyboard/hot-corner switch slides exactly like a
/// gesture ([`Self::advance_discrete`]). This is what makes "one rule for
/// swipes, pinches, and hot corners" literally true (T-11 FR-1/FR-4).
#[derive(Debug, Clone, Copy)]
struct DiscreteProgress {
    start_time: u64,
    duration_ms: u64,
}

/// The single overview/workspace-transition state machine.
#[derive(Debug, Clone)]
pub struct OverviewMachine {
    pipeline: ProgressPipeline,
    kind: Option<OverviewKind>,
    reduced_motion: bool,
    overview_active: bool,
    desktop_revealed: bool,
    selection: Option<WindowId>,
    discrete: Option<DiscreteProgress>,
    /// Set by a directional shell request ([`Self::drive_overview`]) so the
    /// Mission Control commit targets the requested state instead of toggling
    /// (U-7). Cleared when the transition commits or is cancelled.
    requested_overview: Option<bool>,
}

impl Default for OverviewMachine {
    fn default() -> Self {
        OverviewMachine::new(ProgressConfig::default())
    }
}

impl OverviewMachine {
    pub fn new(config: ProgressConfig) -> Self {
        OverviewMachine {
            pipeline: ProgressPipeline::new(config),
            kind: None,
            reduced_motion: false,
            overview_active: false,
            desktop_revealed: false,
            selection: None,
            discrete: None,
            requested_overview: None,
        }
    }

    pub fn config(&self) -> ProgressConfig {
        self.pipeline.config()
    }

    pub fn set_config(&mut self, config: ProgressConfig) {
        self.pipeline.set_config(config);
    }

    /// Reduced motion: every discrete trigger still drives the same pipeline
    /// and commit rule, but takes a single step instead of an animation
    /// (design-system rule).
    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    /// Whether a transition is currently in flight.
    pub fn is_active(&self) -> bool {
        self.pipeline.is_active()
    }

    /// Whether the in-flight transition was started by a gesture. A discrete
    /// trigger's animation clock can still be running when the next gesture
    /// arrives; the gesture must cancel it and start its own transition so
    /// the recorded trigger is the gesture (T-11 FR-1).
    pub fn active_is_gesture(&self) -> bool {
        self.pipeline.is_active()
            && matches!(self.pipeline.trigger(), Some(TriggerKind::Gesture(_)))
    }

    /// Whether a discrete trigger's transition is waiting on the animation
    /// clock. While true the caller must keep calling
    /// [`Self::advance_discrete`] until it returns a commit.
    pub fn is_discrete(&self) -> bool {
        self.discrete.is_some()
    }

    /// Clamped progress of the in-flight transition (`0.0` when idle).
    pub fn progress(&self) -> f64 {
        self.pipeline.progress()
    }

    /// The in-flight transition's kind, if any.
    pub fn kind(&self) -> Option<OverviewKind> {
        self.kind
    }

    /// Whether the Mission Control overview is open.
    pub fn overview_active(&self) -> bool {
        self.overview_active
    }

    /// Whether the desktop is revealed (T-14).
    pub fn desktop_revealed(&self) -> bool {
        self.desktop_revealed
    }

    /// The window selected in the overview, if any (FR-5).
    pub fn selection(&self) -> Option<WindowId> {
        self.selection
    }

    /// Who owns pointer hit-testing.
    ///
    /// Ownership transfers to the overview *immediately* when an
    /// overview-kind transition begins — not only once it commits — because
    /// the overview is already consuming the gesture that opened it, and
    /// returns explicitly when the transition ends (FR-6).
    pub fn input_owner(&self) -> InputOwner {
        let in_overview_transition = self
            .kind
            .is_some_and(|kind| kind.is_overview() && self.pipeline.is_active());
        if self.overview_active || in_overview_transition {
            InputOwner::Overview
        } else {
            InputOwner::Normal
        }
    }

    /// Drive one transition from a discrete trigger (keyboard, hot corner,
    /// shell request, menu-bar button).
    ///
    /// A trigger that arrives mid-transition first cancels the in-flight one
    /// (reversibility, FR-4) rather than waiting for it to finish. The
    /// transition is *not* run synchronously: it begins here and is advanced
    /// by [`Self::advance_discrete`] on the compositor's animation clock, so
    /// the translation-based slide is visible and the commit rule is shared
    /// with gestures. Reduced motion still takes a single synchronous step.
    pub fn drive(&mut self, kind: OverviewKind, trigger: TriggerKind, now: u64) -> DriveOutcome {
        self.drive_towards(kind, trigger, now, None)
    }

    /// Drive an explicit Mission Control enter/leave request through the
    /// same pipeline as every other trigger (U-7).
    ///
    /// Unlike the toggle triggers (keyboard, hot corner, gesture), the shell
    /// request names its target state, so an explicit `enter` while already
    /// open does not close the overview. The progress curve and commit rule
    /// are identical to [`Self::drive`].
    pub fn drive_overview(&mut self, active: bool, trigger: TriggerKind, now: u64) -> DriveOutcome {
        self.drive_towards(OverviewKind::MissionControl, trigger, now, Some(active))
    }

    fn drive_towards(
        &mut self,
        kind: OverviewKind,
        trigger: TriggerKind,
        now: u64,
        target: Option<bool>,
    ) -> DriveOutcome {
        let mut events = Vec::new();
        if let Some(event) = self.cancel_active(now) {
            events.push(event);
        }
        self.kind = Some(kind);
        self.requested_overview = target;
        if self.reduced_motion {
            events.push(self.pipeline.begin(kind.action(), trigger, now));
            if let Some(event) = self.pipeline.set_progress(1.0, now) {
                events.push(event);
            }
            if let Some(event) = self.pipeline.end(now, false) {
                events.push(event);
            }
            let commit = self.commit_from_events(&events);
            return DriveOutcome { events, commit };
        }
        events.push(self.pipeline.begin(kind.action(), trigger, now));
        self.discrete = Some(DiscreteProgress {
            start_time: now,
            duration_ms: self.pipeline.config().discrete_duration_ms.max(1),
        });
        DriveOutcome {
            events,
            commit: None,
        }
    }

    /// Advance an in-flight discrete transition to `now`, returning the
    /// progress sample(s) and the commit when the curve reaches 1.0.
    ///
    /// This is the animation-clock half of [`Self::drive`]: the compositor
    /// arms a ~16 ms timer while [`Self::is_discrete`] is true and calls this
    /// until it yields a commit (or the transition is cancelled).
    pub fn advance_discrete(&mut self, now: u64) -> DriveOutcome {
        let mut events = Vec::new();
        let Some(discrete) = self.discrete else {
            return DriveOutcome::default();
        };
        let duration = discrete.duration_ms.max(1) as f64;
        let elapsed = now.saturating_sub(discrete.start_time) as f64;
        let progress = (elapsed / duration).clamp(0.0, 1.0);
        if let Some(event) = self.pipeline.set_progress(progress, now) {
            events.push(event);
        }
        let commit = if progress >= 1.0 {
            self.discrete = None;
            if let Some(event) = self.pipeline.end(now, false) {
                events.push(event);
            }
            self.commit_from_events(&events)
        } else {
            None
        };
        DriveOutcome { events, commit }
    }

    /// Begin a gesture-driven transition. Returns the `Begin` event; if a
    /// different transition was in flight it is cancelled first.
    pub fn begin_gesture(
        &mut self,
        kind: OverviewKind,
        trigger: TriggerKind,
        now: u64,
    ) -> Vec<ProgressEvent> {
        let mut events = Vec::new();
        if let Some(event) = self.cancel_active(now) {
            events.push(event);
        }
        self.kind = Some(kind);
        self.requested_overview = None;
        events.push(self.pipeline.begin(kind.action(), trigger, now));
        events
    }

    /// Advance the in-flight gesture.
    pub fn update_gesture(&mut self, delta: f64, now: u64) -> Option<ProgressEvent> {
        self.pipeline.update(delta, now)
    }

    /// Finish the in-flight gesture: apply the commit rule and report the
    /// side effect, if the release crossed the threshold.
    pub fn end_gesture(&mut self, now: u64, cancelled: bool) -> DriveOutcome {
        let mut events = Vec::new();
        if let Some(event) = self.pipeline.end(now, cancelled) {
            events.push(event);
        }
        let commit = self.commit_from_events(&events);
        DriveOutcome { events, commit }
    }

    /// Apply a committed transition to the machine's own state. The caller
    /// applies the compositor-side effect (Space switch, broadcast) and must
    /// call this exactly once per commit so the machine never diverges from
    /// the scene.
    pub fn apply_commit(&mut self, commit: TransitionCommit) {
        match commit {
            TransitionCommit::SwitchWorkspace(_) => {}
            TransitionCommit::SetOverview(active) => {
                self.overview_active = active;
                if !active {
                    self.selection = None;
                }
            }
            TransitionCommit::SetDesktopReveal(revealed) => {
                self.desktop_revealed = revealed;
            }
        }
        self.kind = None;
        self.discrete = None;
        self.requested_overview = None;
    }

    /// Record the window selected in the overview and leave the overview
    /// (the selection round-trip, FR-5). The caller then activates the
    /// window's Space and raises/focuses it; [`Self::apply_commit`] is *not*
    /// required here.
    pub fn select(&mut self, window: WindowId) {
        self.selection = Some(window);
        self.overview_active = false;
        self.kind = None;
        self.discrete = None;
        self.requested_overview = None;
    }

    /// Clear the selection without touching the overview state.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Directly set overview state (private-protocol enter/exit requests
    /// that do not animate). Keeps the machine the single source of truth.
    pub fn set_overview(&mut self, active: bool) {
        self.overview_active = active;
        self.requested_overview = None;
        if !active {
            self.selection = None;
        }
    }

    /// Cancel any in-flight transition, returning its `End` event.
    pub fn cancel_active(&mut self, now: u64) -> Option<ProgressEvent> {
        self.discrete = None;
        self.requested_overview = None;
        if !self.pipeline.is_active() {
            return None;
        }
        self.kind = None;
        self.pipeline.end(now, true)
    }

    /// The commit for the `End` event in `events`, if it crossed the
    /// threshold. Reads the machine's current overview/desktop state so a
    /// Mission Control trigger toggles rather than always opening.
    fn commit_from_events(&self, events: &[ProgressEvent]) -> Option<TransitionCommit> {
        let end = events
            .iter()
            .rev()
            .find(|event| event.phase == crate::input::gestures::ProgressPhase::End)?;
        if !end.committed {
            return None;
        }
        let kind = self
            .kind
            .or_else(|| OverviewKind::from_action(end.action))?;
        Some(match kind {
            OverviewKind::WorkspaceNext => TransitionCommit::SwitchWorkspace(1),
            OverviewKind::WorkspacePrev => TransitionCommit::SwitchWorkspace(-1),
            OverviewKind::MissionControl => TransitionCommit::SetOverview(
                self.requested_overview.unwrap_or(!self.overview_active),
            ),
            OverviewKind::DesktopReveal => {
                TransitionCommit::SetDesktopReveal(!self.desktop_revealed)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::action::{GestureKind, HotCorner};
    use crate::input::gestures::ProgressPhase;

    fn config() -> ProgressConfig {
        ProgressConfig {
            rubber_band: 0.35,
            commit_progress: 0.4,
            commit_velocity: 0.8,
            discrete_duration_ms: 200,
            discrete_steps: 4,
        }
    }

    fn machine() -> OverviewMachine {
        OverviewMachine::new(config())
    }

    fn curve(outcome: &DriveOutcome) -> Vec<(ProgressPhase, f64)> {
        outcome
            .events
            .iter()
            .map(|event| (event.phase, (event.progress * 1000.0).round() / 1000.0))
            .collect()
    }

    /// Drive a discrete trigger and run its animation clock to completion,
    /// the way the compositor's timer does.
    fn drive_to_completion(
        machine: &mut OverviewMachine,
        kind: OverviewKind,
        trigger: TriggerKind,
        now: u64,
    ) -> DriveOutcome {
        let mut outcome = machine.drive(kind, trigger, now);
        let duration = machine.config().discrete_duration_ms;
        while machine.is_discrete() {
            let next = machine.advance_discrete(now + duration);
            outcome.events.extend(next.events);
            if next.commit.is_some() {
                outcome.commit = next.commit;
            }
        }
        outcome
    }

    #[test]
    fn every_trigger_kind_produces_the_same_curve_and_commit() {
        let triggers = [
            TriggerKind::Keyboard,
            TriggerKind::HotCorner(HotCorner::TopLeft),
            TriggerKind::Shell,
            TriggerKind::Gesture(GestureKind::Swipe { fingers: 4 }),
        ];
        let mut reference_machine = machine();
        let reference = drive_to_completion(
            &mut reference_machine,
            OverviewKind::MissionControl,
            triggers[0],
            0,
        );
        for trigger in triggers {
            let mut machine = machine();
            let outcome =
                drive_to_completion(&mut machine, OverviewKind::MissionControl, trigger, 0);
            assert_eq!(
                curve(&outcome),
                curve(&reference),
                "trigger {trigger:?} must share the one progress curve"
            );
            assert_eq!(outcome.commit, reference.commit);
        }
    }

    #[test]
    fn workspace_switch_commits_the_direction() {
        let mut next = machine();
        assert_eq!(
            drive_to_completion(
                &mut next,
                OverviewKind::WorkspaceNext,
                TriggerKind::Keyboard,
                0
            )
            .commit,
            Some(TransitionCommit::SwitchWorkspace(1))
        );
        let mut prev = machine();
        assert_eq!(
            drive_to_completion(
                &mut prev,
                OverviewKind::WorkspacePrev,
                TriggerKind::Keyboard,
                0
            )
            .commit,
            Some(TransitionCommit::SwitchWorkspace(-1))
        );
    }

    #[test]
    fn mission_control_toggles_and_tracks_the_overview() {
        let mut machine = machine();
        let open = drive_to_completion(
            &mut machine,
            OverviewKind::MissionControl,
            TriggerKind::Keyboard,
            0,
        );
        assert_eq!(open.commit, Some(TransitionCommit::SetOverview(true)));
        machine.apply_commit(open.commit.unwrap());
        assert!(machine.overview_active());

        let close = drive_to_completion(
            &mut machine,
            OverviewKind::MissionControl,
            TriggerKind::Keyboard,
            1000,
        );
        assert_eq!(close.commit, Some(TransitionCommit::SetOverview(false)));
        machine.apply_commit(close.commit.unwrap());
        assert!(!machine.overview_active());
    }

    #[test]
    fn mid_transition_input_reverses_without_waiting() {
        let mut machine = machine();
        // Open Mission Control by gesture, halfway.
        machine.begin_gesture(
            OverviewKind::MissionControl,
            TriggerKind::Gesture(GestureKind::Pinch { fingers: 4 }),
            0,
        );
        machine.update_gesture(0.5, 10);
        assert!(machine.is_active());

        // A discrete reverse arrives: it must cancel immediately, then begin
        // the opposite transition (the animation clock commits it later).
        let outcome = machine.drive(OverviewKind::WorkspacePrev, TriggerKind::Keyboard, 20);
        assert_eq!(outcome.events[0].phase, ProgressPhase::End);
        assert!(
            outcome.events[0].cancelled,
            "the old transition is cancelled"
        );
        assert_eq!(outcome.events[1].phase, ProgressPhase::Begin);
        assert_eq!(outcome.commit, None, "the reverse animates on the clock");
        let duration = machine.config().discrete_duration_ms;
        let mut commit = outcome.commit;
        while machine.is_discrete() {
            let next = machine.advance_discrete(20 + duration);
            if next.commit.is_some() {
                commit = next.commit;
            }
        }
        assert_eq!(commit, Some(TransitionCommit::SwitchWorkspace(-1)));
    }

    #[test]
    fn discrete_trigger_advances_on_the_clock_then_commits() {
        let mut machine = machine();
        let started = machine.drive(OverviewKind::WorkspaceNext, TriggerKind::Keyboard, 0);
        assert_eq!(started.events.len(), 1, "only the Begin is synchronous");
        assert_eq!(started.commit, None);
        assert!(machine.is_discrete());

        // Halfway: a progress sample, no commit yet.
        let half = machine.advance_discrete(machine.config().discrete_duration_ms / 2);
        assert_eq!(half.events.last().unwrap().phase, ProgressPhase::Update);
        assert!(half.commit.is_none());
        assert!(machine.is_active());
        assert!(machine.is_discrete());

        // At the duration: the curve reaches 1.0 and commits.
        let end = machine.advance_discrete(machine.config().discrete_duration_ms);
        assert_eq!(end.events.last().unwrap().phase, ProgressPhase::End);
        assert!(end.events.last().unwrap().committed);
        assert_eq!(end.commit, Some(TransitionCommit::SwitchWorkspace(1)));
        assert!(!machine.is_discrete());
    }

    #[test]
    fn gesture_release_below_the_threshold_does_not_commit() {
        let mut machine = machine();
        machine.begin_gesture(
            OverviewKind::WorkspaceNext,
            TriggerKind::Gesture(GestureKind::Swipe { fingers: 3 }),
            0,
        );
        machine.update_gesture(0.05, 1000);
        let outcome = machine.end_gesture(1100, false);
        assert!(!outcome.events.last().unwrap().committed);
        assert_eq!(outcome.commit, None, "animate back, apply nothing");
    }

    #[test]
    fn rubber_band_overshoots_but_clamps_progress() {
        let mut machine = machine();
        machine.begin_gesture(
            OverviewKind::WorkspaceNext,
            TriggerKind::Gesture(GestureKind::Swipe { fingers: 3 }),
            0,
        );
        let event = machine.update_gesture(2.0, 100).unwrap();
        assert_eq!(event.progress, 1.0);
        assert!(
            event.raw_progress > 1.0,
            "past the last Space it rubber-bands"
        );
    }

    #[test]
    fn hit_testing_transfers_explicitly_to_the_overview() {
        let mut machine = machine();
        assert_eq!(machine.input_owner(), InputOwner::Normal);

        machine.begin_gesture(
            OverviewKind::MissionControl,
            TriggerKind::Gesture(GestureKind::Pinch { fingers: 4 }),
            0,
        );
        assert_eq!(
            machine.input_owner(),
            InputOwner::Overview,
            "the overview owns input as soon as it starts opening"
        );

        let outcome = machine.end_gesture(100, true);
        assert_eq!(outcome.commit, None);
        assert_eq!(
            machine.input_owner(),
            InputOwner::Normal,
            "ownership returns when the transition ends"
        );

        // A workspace slide never takes pointer ownership.
        machine.begin_gesture(
            OverviewKind::WorkspaceNext,
            TriggerKind::Gesture(GestureKind::Swipe { fingers: 3 }),
            0,
        );
        assert_eq!(machine.input_owner(), InputOwner::Normal);
    }

    #[test]
    fn selection_round_trip_clears_on_close() {
        let mut machine = machine();
        machine.set_overview(true);
        machine.select(WindowId(7));
        assert_eq!(machine.selection(), Some(WindowId(7)));
        assert!(!machine.overview_active());
        machine.apply_commit(TransitionCommit::SetOverview(false));
        assert_eq!(machine.selection(), None);
    }

    #[test]
    fn reduced_motion_still_commits_through_the_same_rule() {
        let mut machine = machine();
        machine.set_reduced_motion(true);
        let outcome = machine.drive(
            OverviewKind::MissionControl,
            TriggerKind::HotCorner(HotCorner::TopLeft),
            0,
        );
        assert_eq!(outcome.commit, Some(TransitionCommit::SetOverview(true)));
        // One Begin, one Update, one committed End — no animation steps.
        assert_eq!(outcome.events.len(), 3);
        assert!(outcome.events.last().unwrap().committed);
    }

    /// U-7: the shell's explicit enter/leave drives the same pipeline but
    /// commits the *requested* state instead of toggling, so a repeated
    /// enter while open does not close the overview.
    #[test]
    fn shell_request_drives_directionally_through_the_same_curve() {
        let mut machine = machine();
        // The explicit enter uses the same discrete animation clock.
        let enter = machine.drive_overview(true, TriggerKind::Shell, 0);
        assert_eq!(enter.commit, None, "it animates on the clock");
        assert!(machine.is_discrete());
        let duration = machine.config().discrete_duration_ms;
        let mut commit = enter.commit;
        while machine.is_discrete() {
            let next = machine.advance_discrete(duration);
            if next.commit.is_some() {
                commit = next.commit;
            }
        }
        assert_eq!(commit, Some(TransitionCommit::SetOverview(true)));
        machine.apply_commit(commit.unwrap());
        assert!(machine.overview_active());

        // An explicit enter while already open still targets `true` (no
        // toggle), so the overview stays open.
        let repeat = machine.drive_overview(true, TriggerKind::Shell, 1000);
        assert_eq!(repeat.commit, None);
        let mut commit = None;
        while machine.is_discrete() {
            let next = machine.advance_discrete(2000);
            if next.commit.is_some() {
                commit = next.commit;
            }
        }
        assert_eq!(commit, Some(TransitionCommit::SetOverview(true)));

        // And the explicit exit targets `false`.
        let exit = machine.drive_overview(false, TriggerKind::Shell, 3000);
        let mut commit = exit.commit;
        while machine.is_discrete() {
            let next = machine.advance_discrete(4000);
            if next.commit.is_some() {
                commit = next.commit;
            }
        }
        assert_eq!(commit, Some(TransitionCommit::SetOverview(false)));
    }

    /// U-1/FR-9: every transition kind takes the single-step path under
    /// reduced motion while still applying the same commit rule.
    #[test]
    fn reduced_motion_single_steps_every_transition_kind() {
        let cases = [
            (
                OverviewKind::WorkspaceNext,
                TransitionCommit::SwitchWorkspace(1),
            ),
            (
                OverviewKind::WorkspacePrev,
                TransitionCommit::SwitchWorkspace(-1),
            ),
            (
                OverviewKind::MissionControl,
                TransitionCommit::SetOverview(true),
            ),
            (
                OverviewKind::DesktopReveal,
                TransitionCommit::SetDesktopReveal(true),
            ),
        ];
        for (kind, expected) in cases {
            let mut machine = machine();
            machine.set_reduced_motion(true);
            let outcome = machine.drive(kind, TriggerKind::Shell, 0);
            assert_eq!(
                outcome.commit,
                Some(expected),
                "{kind:?} must commit in one step"
            );
            assert_eq!(outcome.events.len(), 3, "{kind:?}: Begin/Update/End only");
            assert!(!machine.is_discrete(), "{kind:?} must not arm the clock");
        }
    }
}
