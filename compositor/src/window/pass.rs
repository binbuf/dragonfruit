// SPDX-License-Identifier: MIT
//! Shared per-frame render-pass bookkeeping (T-04.3).
//!
//! The material track has one invariant: **one effect pass per rendered
//! frame**. [`FramePass`] is the reusable guard for it — the renderer opens a
//! frame once (with the monotonic render serial), then each pass applies at
//! most once per output in that frame. A repeated `(frame, output)` request is
//! counted as `skipped` and draws nothing, so an effect can never be applied
//! twice (the T-04.2 "no double-blur", the T-04.3 "no double-transform", and
//! the counter T-04.4a degrades on).
//!
//! [`crate::window::backdrop::BackdropPass`] and
//! [`crate::window::scene_transform::SceneTransformPass`] are the two passes
//! built on it; a new effect must not grow its own ad-hoc guard.

use smithay::utils::{Logical, Rectangle};

/// Per-frame, per-output pass bookkeeping: which outputs already applied, how
/// many applications/skips, and the damage the pass owns this frame.
#[derive(Debug, Default)]
pub struct FramePass {
    frame: u64,
    applied_outputs: Vec<String>,
    applications: u64,
    skipped: u64,
    damage: Vec<Rectangle<i32, Logical>>,
}

impl FramePass {
    pub fn new() -> Self {
        FramePass::default()
    }

    /// Open a frame. Clears the per-output application set and the damage
    /// recorded for it. `frame` is the renderer's monotonic render serial.
    pub fn begin_frame(&mut self, frame: u64) {
        self.frame = frame;
        self.applied_outputs.clear();
        self.damage.clear();
    }

    /// Apply the pass for `output` this frame. `region` is the effect's region
    /// in output-local logical coordinates. Returns the region when it applies
    /// now, or `None` when the pass already ran for this output in this frame
    /// (the double-application guard).
    pub fn apply(
        &mut self,
        output: &str,
        region: Rectangle<i32, Logical>,
    ) -> Option<Rectangle<i32, Logical>> {
        if self.applied_outputs.iter().any(|name| name == output) {
            self.skipped += 1;
            return None;
        }
        self.applied_outputs.push(output.to_string());
        self.applications += 1;
        if region.size.w > 0 && region.size.h > 0 {
            self.damage.push(region);
        }
        Some(region)
    }

    /// The frame serial this pass is open for.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Number of outputs the pass actually applied for (one per output per
    /// frame; the effect-pass count).
    pub fn applications(&self) -> u64 {
        self.applications
    }

    /// Number of requests skipped because the output already applied the pass
    /// this frame (the double-application counter — must stay `0` in a correct
    /// render loop).
    pub fn skipped(&self) -> u64 {
        self.skipped
    }

    /// The damage of the current frame's applications (output-local logical).
    pub fn damage(&self) -> &[Rectangle<i32, Logical>] {
        &self.damage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    #[test]
    fn a_frame_applies_once_per_output_and_counts_skips() {
        let region = rect(0, 0, 1920, 24);
        let mut pass = FramePass::new();
        pass.begin_frame(1);

        assert_eq!(pass.apply("NESTED-1", region), Some(region));
        assert_eq!(pass.applications(), 1);
        assert_eq!(pass.skipped(), 0);
        assert_eq!(pass.damage(), &[region]);

        assert_eq!(pass.apply("NESTED-1", region), None);
        assert_eq!(pass.applications(), 1);
        assert_eq!(pass.skipped(), 1);
        assert_eq!(pass.damage().len(), 1);

        assert_eq!(pass.apply("HDMI-A-1", region), Some(region));
        assert_eq!(pass.applications(), 2);

        pass.begin_frame(2);
        assert_eq!(pass.frame(), 2);
        assert!(pass.damage().is_empty());
        assert_eq!(pass.apply("NESTED-1", region), Some(region));
        assert_eq!(pass.applications(), 3);
        assert_eq!(pass.skipped(), 1);
    }

    #[test]
    fn a_zero_area_region_is_applied_but_records_no_damage() {
        let mut pass = FramePass::new();
        pass.begin_frame(7);
        let empty = rect(0, 0, 0, 0);
        assert_eq!(pass.apply("NESTED-1", empty), Some(empty));
        assert!(pass.damage().is_empty());
        assert_eq!(pass.applications(), 1);
    }
}
