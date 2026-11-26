// SPDX-License-Identifier: MIT
//! Chrome presentation correlation (T353).
//!
//! The compositor's frame trace separates three things that used to be
//! conflated:
//!
//! 1. a chrome **commit** — the compositor accepted pixels and assigned a
//!    per-surface generation;
//! 2. a `wl_surface.frame` **callback** — a pacing event; it tells the shell
//!    it may render the next QML frame, not that the callback's content was
//!    scanned out;
//! 3. a **present** — the backend reported a frame at the output, with the
//!    chrome generations that frame actually composited.
//!
//! The native acceptance metric ([`docs/design/evidence-and-acceptance.md`
//! §5.4](../../docs/design/evidence-and-acceptance.md)) is request-to-present
//! for the accepted chrome content. A callback alone must never satisfy it.
//! This module is the pure join/classification used by the compositor tests;
//! the T238 Python driver implements the same rules over the emitted trace.

/// Where a presentation timestamp came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSource {
    /// Kernel vblank timestamp on the DRM backend.
    Hardware,
    /// Compositor-clock fallback when the kernel event carried no timestamp.
    Estimate,
    /// Host swap completion on the nested backend.
    Host,
    /// The headless stand-in; it presents no pixels.
    Headless,
}

impl ClockSource {
    /// Whether a present from this source can satisfy the native metric.
    pub fn is_native(self) -> bool {
        matches!(self, Self::Hardware | Self::Estimate | Self::Host)
    }
}

/// One accepted chrome commit in the compositor trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeCommit {
    pub surface: String,
    pub generation: u64,
    pub has_buffer: bool,
    /// Process-monotonic microseconds at the commit handler.
    pub mono_us: u128,
}

/// One presented frame and the chrome content it composited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramePresentation {
    pub frame: u64,
    pub output: String,
    pub clock: ClockSource,
    /// `(surface, generation)` pairs that were part of the frame.
    pub chrome: Vec<(String, u64)>,
    /// Process-monotonic microseconds at the presentation endpoint.
    pub mono_us: u128,
}

impl FramePresentation {
    /// Whether this frame carried the accepted chrome content.
    pub fn carries(&self, surface: &str, generation: u64) -> bool {
        self.chrome
            .iter()
            .any(|(name, gen)| name == surface && *gen == generation)
    }
}

/// One `wl_surface.frame` callback in the compositor trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameCallback {
    pub surface: String,
    pub frame: u64,
    /// Whether the callback followed a real presented frame (not a
    /// no-damage/wake-only pass).
    pub presented: bool,
    pub mono_us: u128,
}

/// The shell's accepted content commit for one interaction target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionCommit {
    pub interaction: u64,
    pub target: String,
    /// The output the interaction was requested on, when the shell knows it.
    pub output: Option<String>,
    /// Process-monotonic microseconds at the shell's `commit-done`.
    pub mono_us: u128,
    /// A newer interaction for the same target, if one started before the
    /// presentation endpoint.
    pub superseded_at_us: Option<u128>,
}

/// The final outcome for one interaction target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A native present carried the accepted generation on the intended
    /// output.
    Presented,
    /// A native present carried the content, but only after a newer
    /// interaction for the same target had already been requested.
    LatePresented,
    /// The callback arrived, but no present ever carried the accepted
    /// content. This is a pacing event, not a presentation.
    CallbackWithoutContent,
    /// The shell never accepted a content commit (failed/deferred/skipped).
    Discarded,
    /// A newer commit for the same target replaced this one before a present
    /// carried it.
    Superseded,
    /// No present and no callback before the trace ended.
    Missed,
}

impl Outcome {
    /// Whether this outcome counts toward the native request-to-present
    /// metric. Only a real content presentation does.
    pub fn is_native(self) -> bool {
        matches!(self, Self::Presented | Self::LatePresented)
    }
}

/// Classify one interaction target from the compositor trace.
///
/// `commits`, `presents`, and `callbacks` may be in any order; the join is by
/// `(surface, generation)` and frame id, not by wall-clock proximity.
pub fn classify(
    interaction: &InteractionCommit,
    commits: &[ChromeCommit],
    presents: &[FramePresentation],
    callbacks: &[FrameCallback],
) -> Outcome {
    // The first content commit at or after the shell's accepted commit. A
    // commit without a buffer (geometry-only) is not content.
    let Some(commit) = commits
        .iter()
        .filter(|commit| commit.surface == interaction.target)
        .filter(|commit| commit.mono_us >= interaction.mono_us)
        .min_by_key(|commit| commit.mono_us)
    else {
        return Outcome::Discarded;
    };
    if !commit.has_buffer {
        return Outcome::Discarded;
    }

    // A present must name this exact surface generation on the intended
    // output. A callback's frame id alone is not evidence: the callback can
    // arrive before the frame that contains the new content, and a no-damage
    // callback has no frame at all.
    let mut matches: Vec<&FramePresentation> = presents
        .iter()
        .filter(|present| present.clock.is_native())
        .filter(|present| present.mono_us >= commit.mono_us)
        .filter(|present| present.carries(&interaction.target, commit.generation))
        .filter(|present| {
            interaction
                .output
                .as_ref()
                .map_or(true, |output| present.output == *output)
        })
        .collect();
    matches.sort_by_key(|present| present.mono_us);
    if let Some(present) = matches.first() {
        if let Some(superseded_at) = interaction.superseded_at_us {
            if present.mono_us > superseded_at {
                return Outcome::LatePresented;
            }
        }
        return Outcome::Presented;
    }

    // No present carried the content. If a newer commit for the same surface
    // arrived first, the frame was superseded; otherwise the callback (if any)
    // is a contentless pacing event.
    if let Some(superseded_at) = interaction.superseded_at_us {
        if superseded_at >= commit.mono_us {
            return Outcome::Superseded;
        }
    }
    let callback = callbacks.iter().any(|callback| {
        callback.surface == interaction.target && callback.mono_us >= interaction.mono_us
    });
    if callback {
        return Outcome::CallbackWithoutContent;
    }
    Outcome::Missed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(surface: &str, generation: u64, mono_us: u128) -> ChromeCommit {
        ChromeCommit {
            surface: surface.to_owned(),
            generation,
            has_buffer: true,
            mono_us,
        }
    }

    fn present(frame: u64, chrome: &[(&str, u64)], mono_us: u128) -> FramePresentation {
        FramePresentation {
            frame,
            output: "eDP-1".to_owned(),
            clock: ClockSource::Hardware,
            chrome: chrome
                .iter()
                .map(|(surface, generation)| ((*surface).to_owned(), *generation))
                .collect(),
            mono_us,
        }
    }

    fn interaction(target: &str, mono_us: u128) -> InteractionCommit {
        InteractionCommit {
            interaction: 7,
            target: target.to_owned(),
            output: Some("eDP-1".to_owned()),
            mono_us,
            superseded_at_us: None,
        }
    }

    #[test]
    fn callback_without_accepted_content_is_not_a_presentation() {
        let interaction = interaction("dock-popup", 1_000);
        let commits = [commit("dock-popup", 4, 1_200)];
        let presents = [present(9, &[("dock", 3)], 1_800)];
        let callbacks = [FrameCallback {
            surface: "dock-popup".to_owned(),
            frame: 9,
            presented: true,
            mono_us: 1_700,
        }];
        assert_eq!(
            classify(&interaction, &commits, &presents, &callbacks),
            Outcome::CallbackWithoutContent
        );
    }

    #[test]
    fn present_with_the_accepted_generation_is_native() {
        let interaction = interaction("dock-popup", 1_000);
        let commits = [commit("dock-popup", 4, 1_200)];
        let presents = [
            present(9, &[("dock-popup", 3)], 1_700),
            present(10, &[("dock-popup", 4)], 2_000),
        ];
        let callbacks = [FrameCallback {
            surface: "dock-popup".to_owned(),
            frame: 9,
            presented: true,
            mono_us: 1_650,
        }];
        let outcome = classify(&interaction, &commits, &presents, &callbacks);
        assert_eq!(outcome, Outcome::Presented);
        assert!(outcome.is_native());
    }

    #[test]
    fn present_on_another_output_does_not_satisfy_the_interaction() {
        let interaction = interaction("menubar-popup", 1_000);
        let commits = [commit("menubar-popup", 2, 1_100)];
        let mut other = present(4, &[("menubar-popup", 2)], 1_400);
        other.output = "HDMI-A-1".to_owned();
        assert_eq!(
            classify(&interaction, &commits, &[other], &[]),
            Outcome::Missed
        );
    }

    #[test]
    fn a_newer_commit_supersedes_an_unpresented_one() {
        let mut interaction = interaction("dock-popup", 1_000);
        interaction.superseded_at_us = Some(1_500);
        let commits = [commit("dock-popup", 1, 1_100)];
        let presents = [present(3, &[("dock-popup", 2)], 1_800)];
        assert_eq!(
            classify(&interaction, &commits, &presents, &[]),
            Outcome::Superseded
        );
    }

    #[test]
    fn headless_present_is_not_native() {
        let interaction = interaction("dock", 1_000);
        let commits = [commit("dock", 1, 1_100)];
        let mut headless = present(2, &[("dock", 1)], 1_300);
        headless.clock = ClockSource::Headless;
        let outcome = classify(&interaction, &commits, &[headless], &[]);
        assert_eq!(outcome, Outcome::Missed);
        assert!(!outcome.is_native());
    }

    #[test]
    fn geometry_only_commit_is_discarded() {
        let interaction = interaction("dock", 1_000);
        let commits = [ChromeCommit {
            surface: "dock".to_owned(),
            generation: 1,
            has_buffer: false,
            mono_us: 1_100,
        }];
        assert_eq!(
            classify(&interaction, &commits, &[], &[]),
            Outcome::Discarded
        );
    }
}
