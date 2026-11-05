// SPDX-License-Identifier: MIT
//! The pure panel-region geometry for the material pass (T-21.3).
//!
//! The GPU material pass samples the scene under the chrome panels. Rendering
//! the whole output for a menu bar and a Dock wastes the scene texture and the
//! blur bandwidth, so the pass is scoped to the union of the panels expanded
//! by their blur padding. This module owns that calculation; it is deliberately
//! renderer-free and is asserted headless.
//!
//! The padding is **not** a literal: [`crate::window::BlurSpec::padding`]
//! derives it from the token blur radius and the Kawase iteration count. Each
//! panel contributes its own padding before the union, because the menu bar
//! and a popup can run at different material tiers.

use std::sync::OnceLock;

use smithay::utils::{Buffer, Logical, Physical, Rectangle, Scale, Size};

use crate::render::BlurPanel;

/// Force the full-output material scene texture (diagnostic/rollback).
///
/// The region path is the default. Setting this to `1`/`true`/`on`/`yes`
/// renders the material source into the whole output, exactly as the pre-
/// T-21.3 path did, so a region-origin regression can be isolated without
/// changing the rest of the frame. It is read once per process.
pub const ENV_MATERIAL_FULL_OUTPUT: &str = "DRAGONFRUIT_MATERIAL_FULL_OUTPUT";

/// Whether an explicit environment value enables a default-off flag.
fn env_flag_enabled(value: Option<&str>) -> bool {
    matches!(
        value
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "on" | "yes" | "enabled")
    )
}

/// Whether the full-output material scene texture is forced.
pub fn full_output_material_forced() -> bool {
    static FORCED: OnceLock<bool> = OnceLock::new();
    *FORCED.get_or_init(|| {
        let value = std::env::var(ENV_MATERIAL_FULL_OUTPUT).ok();
        env_flag_enabled(value.as_deref())
    })
}

/// The output-local rectangle a material pass must render, plus the scene
/// texture size it needs.
///
/// `rect` is the canonical logical region (the one the material pass owns for
/// damage); `physical` is the same region after the renderer's precise scale
/// rounding and output clamp. A region-sized scene texture uses `physical` as
/// its geometry and `texture_size` as its allocation size. Panel `src`
/// rectangles are translated by `physical.loc` (the region origin) before
/// being mapped into the texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialRegion {
    /// The padded panel union, clamped to the output, in output-local logical
    /// coordinates.
    pub rect: Rectangle<i32, Logical>,
    /// The same region in output-local physical coordinates (precise rounded
    /// at the output scale and clamped to the physical output rect).
    pub physical: Rectangle<i32, Physical>,
    /// The buffer size for the region-sized scene texture.
    pub texture_size: Size<i32, Buffer>,
}

impl MaterialRegion {
    /// Resolve the material region for `panels` on an output.
    ///
    /// `output` and every `panel.panel` must be in the **same output-local**
    /// coordinate space (the compositor strips the output origin before
    /// calling this). `scale` is the output's fractional scale.
    ///
    /// Returns `None` when there is no panel with a visible area (empty input,
    /// degenerate panels, or panels entirely outside the output). Each panel
    /// is expanded by its own [`crate::window::BlurSpec::padding`] before the
    /// union, so a larger blur on one panel does not silently shrink another
    /// panel's margin.
    pub fn from_panels(
        panels: &[BlurPanel],
        output: Rectangle<i32, Logical>,
        scale: Scale<f64>,
    ) -> Option<Self> {
        if output.size.w <= 0
            || output.size.h <= 0
            || !scale.x.is_finite()
            || !scale.y.is_finite()
            || scale.x <= 0.0
            || scale.y <= 0.0
        {
            return None;
        }

        let mut union: Option<Rectangle<i32, Logical>> = None;
        for blur_panel in panels {
            let panel = blur_panel.panel;
            if panel.size.w <= 0 || panel.size.h <= 0 {
                continue;
            }
            let padding = blur_panel.spec.padding_px();
            let expanded = Rectangle::new(
                (
                    panel.loc.x.saturating_sub(padding),
                    panel.loc.y.saturating_sub(padding),
                )
                    .into(),
                (
                    panel.size.w.saturating_add(padding.saturating_mul(2)),
                    panel.size.h.saturating_add(padding.saturating_mul(2)),
                )
                    .into(),
            );
            let Some(clipped) = expanded.intersection(output) else {
                continue;
            };
            union = Some(match union {
                Some(current) => current.merge(clipped),
                None => clipped,
            });
        }

        let rect = union?;
        // The scene texture is allocated in physical pixels. Round the region
        // exactly like the panel `src` mapping does, then clamp the rounded
        // rectangle back to the physical output: independent location/size
        // rounding at a fractional scale can otherwise poke one pixel past the
        // output edge.
        let physical_output = output.to_physical_precise_round(scale);
        let physical = rect
            .to_physical_precise_round(scale)
            .intersection(physical_output)?;
        if physical.size.w <= 0 || physical.size.h <= 0 {
            return None;
        }
        Some(Self {
            rect,
            physical,
            texture_size: Size::from((physical.size.w, physical.size.h)),
        })
    }

    /// The full-output diagnostic/rollback region.
    ///
    /// `output` is output-local logical, exactly like [`Self::from_panels`].
    /// This is what [`Self::for_material`] uses when the rollback flag is set,
    /// and the safe fallback when a panel set has no visible area.
    pub fn full_output(output: Rectangle<i32, Logical>, scale: Scale<f64>) -> Option<Self> {
        if output.size.w <= 0
            || output.size.h <= 0
            || !scale.x.is_finite()
            || !scale.y.is_finite()
            || scale.x <= 0.0
            || scale.y <= 0.0
        {
            return None;
        }
        let physical = output.to_physical_precise_round(scale);
        if physical.size.w <= 0 || physical.size.h <= 0 {
            return None;
        }
        Some(Self {
            rect: output,
            physical,
            texture_size: Size::from((physical.size.w, physical.size.h)),
        })
    }

    /// Resolve the material region for a frame: the padded panel union by
    /// default, or the whole output when the diagnostic flag is set.
    ///
    /// A panel set with no visible area falls back to the full output rather
    /// than leaving the caller without a texture; the panel list is already
    /// filtered for visible panels, so this is a safety net.
    pub fn for_material(
        panels: &[BlurPanel],
        output: Rectangle<i32, Logical>,
        scale: Scale<f64>,
    ) -> Option<Self> {
        if full_output_material_forced() {
            Self::full_output(output, scale)
        } else {
            Self::from_panels(panels, output, scale).or_else(|| Self::full_output(output, scale))
        }
    }

    /// Map a panel into this region.
    ///
    /// Returns the part of `panel` visible in the region (output-local
    /// logical) and the source rectangle in the region texture's buffer
    /// coordinates. The source origin is translated by [`Self::physical`]'s
    /// origin, which is the key difference from the pre-region path where a
    /// panel's `src` was already an output-coordinate rectangle.
    pub fn panel_source(
        &self,
        panel: Rectangle<i32, Logical>,
        scale: Scale<f64>,
    ) -> Option<(Rectangle<i32, Logical>, Rectangle<f64, Buffer>)> {
        let visible = panel.intersection(self.rect)?;
        if visible.size.w <= 0 || visible.size.h <= 0 {
            return None;
        }
        let physical = visible
            .to_physical_precise_round(scale)
            .intersection(self.physical)?;
        if physical.size.w <= 0 || physical.size.h <= 0 {
            return None;
        }
        let src = Rectangle::new(
            (
                (physical.loc.x - self.physical.loc.x) as f64,
                (physical.loc.y - self.physical.loc.y) as f64,
            )
                .into(),
            (physical.size.w as f64, physical.size.h as f64).into(),
        );
        Some((visible, src))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::blur::{BlurSpec, GlassSpec};

    fn panel(rect: (i32, i32, i32, i32), radius: f32) -> BlurPanel {
        BlurPanel {
            key: crate::window::PanelKey::new("test", 2, 0),
            panel: Rectangle::new((rect.0, rect.1).into(), (rect.2, rect.3).into()),
            spec: BlurSpec::new(radius, 10.0),
            glass: GlassSpec {
                refraction: 0.0,
                specular: 0.0,
                specular_width: 0.0,
                tint: 0.0,
                tint_color: [0.0; 3],
            },
        }
    }

    fn output(w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::from_size((w, h).into())
    }

    #[test]
    fn an_empty_or_degenerate_input_has_no_region() {
        let screen = output(1280, 800);
        assert_eq!(MaterialRegion::from_panels(&[], screen, 1.0.into()), None);
        assert_eq!(
            MaterialRegion::from_panels(&[panel((10, 10, 0, 40), 12.0)], screen, 1.0.into()),
            None
        );
        assert_eq!(
            MaterialRegion::from_panels(&[panel((10, 10, 40, 0), 12.0)], screen, 1.0.into()),
            None
        );
        // Entirely outside the output, even after the blur padding.
        assert_eq!(
            MaterialRegion::from_panels(&[panel((1300, 10, 40, 40), 12.0)], screen, 1.0.into()),
            None
        );
        // A zero-sized output cannot host a texture.
        assert_eq!(
            MaterialRegion::from_panels(
                &[panel((10, 10, 40, 40), 12.0)],
                output(0, 800),
                1.0.into()
            ),
            None
        );
        // An invalid scale is a caller error, not a zero-sized texture.
        assert_eq!(
            MaterialRegion::from_panels(&[panel((10, 10, 40, 40), 12.0)], screen, 0.0.into()),
            None
        );
    }

    #[test]
    fn a_single_panel_is_expanded_by_its_own_blur_padding() {
        let screen = output(1920, 1080);
        // radius 24 -> 3 Kawase iterations -> max(24, 8) = 24 logical px.
        let region =
            MaterialRegion::from_panels(&[panel((100, 20, 800, 28), 24.0)], screen, 1.0.into())
                .expect("visible panel");
        // The top expands past the output and is clamped to y=0.
        assert_eq!(
            region.rect,
            Rectangle::new((76, 0).into(), (848, 72).into())
        );
        assert_eq!(
            region.physical,
            Rectangle::new((76, 0).into(), (848, 72).into())
        );
        assert_eq!(region.texture_size, (848, 72).into());
    }

    #[test]
    fn the_union_expands_each_panel_before_merging() {
        let screen = output(800, 600);
        let a = panel((100, 100, 100, 50), 8.0); // padding 8
        let b = panel((400, 300, 50, 50), 24.0); // padding 24
        let region = MaterialRegion::from_panels(&[a, b], screen, 1.0.into()).expect("two panels");
        // a -> (92,92,116,66); b -> (376,276,98,98); union -> (92,92,382,282).
        // Expanding the union by the larger padding instead would start at
        // (76,76) and prove the per-panel rule was not applied.
        assert_eq!(
            region.rect,
            Rectangle::new((92, 92).into(), (382, 282).into())
        );
        assert_eq!(region.texture_size, (382, 282).into());

        // The union is order-independent.
        let reversed =
            MaterialRegion::from_panels(&[b, a], screen, 1.0.into()).expect("two panels");
        assert_eq!(reversed, region);
    }

    #[test]
    fn the_region_clips_to_every_output_edge() {
        let screen = output(1280, 800);

        let top_left =
            MaterialRegion::from_panels(&[panel((-50, -20, 200, 100), 8.0)], screen, 1.0.into())
                .expect("partially visible");
        // (-58,-28,216,116) -> (0,0,158,88).
        assert_eq!(
            top_left.rect,
            Rectangle::new((0, 0).into(), (158, 88).into())
        );

        let bottom_right =
            MaterialRegion::from_panels(&[panel((1200, 760, 200, 100), 8.0)], screen, 1.0.into())
                .expect("partially visible");
        // (1192,752,216,116) -> (1192,752,88,48).
        assert_eq!(
            bottom_right.rect,
            Rectangle::new((1192, 752).into(), (88, 48).into())
        );

        // A panel whose padding still touches the output is visible.
        let touching =
            MaterialRegion::from_panels(&[panel((1280, 0, 10, 10), 8.0)], screen, 1.0.into())
                .expect("padding overlaps the output");
        assert_eq!(
            touching.rect,
            Rectangle::new((1272, 0).into(), (8, 18).into())
        );
    }

    #[test]
    fn scale_and_rounding_map_the_region_to_a_physical_texture() {
        let screen = output(1280, 800);
        // radius 12 -> 2 iterations -> max(12, 4) = 12 logical px padding.
        let region =
            MaterialRegion::from_panels(&[panel((101, 51, 333, 77), 12.0)], screen, 1.0.into())
                .expect("visible panel");
        assert_eq!(
            region.rect,
            Rectangle::new((89, 39).into(), (357, 101).into())
        );

        // At 2x every edge is exact.
        let at_two =
            MaterialRegion::from_panels(&[panel((101, 51, 333, 77), 12.0)], screen, 2.0.into())
                .expect("visible panel");
        assert_eq!(
            at_two.physical,
            Rectangle::new((178, 78).into(), (714, 202).into())
        );
        assert_eq!(at_two.texture_size, (714, 202).into());

        // At 1.5x the location and size round independently, exactly like
        // `Rectangle::to_physical_precise_round`.
        let at_one_and_a_half =
            MaterialRegion::from_panels(&[panel((101, 51, 333, 77), 12.0)], screen, 1.5.into())
                .expect("visible panel");
        assert_eq!(
            at_one_and_a_half.physical,
            Rectangle::new((134, 59).into(), (536, 152).into())
        );
        assert_eq!(at_one_and_a_half.texture_size, (536, 152).into());

        // A region clamped at the logical edge can still round one physical
        // pixel past the output; the physical rect is clamped again so the
        // texture never exceeds the output.
        let rounded_edge =
            MaterialRegion::from_panels(&[panel((1199, 759, 81, 41), 12.0)], screen, 1.5.into())
                .expect("visible panel");
        // Logical (1187,747,93,53) -> 1.5x (1780.5,1120.5,139.5,79.5) ->
        // (1781,1121,140,80), right/bottom clamped to 1920x1200.
        assert_eq!(
            rounded_edge.rect,
            Rectangle::new((1187, 747).into(), (93, 53).into())
        );
        assert_eq!(
            rounded_edge.physical,
            Rectangle::new((1781, 1121).into(), (139, 79).into())
        );
        assert_eq!(rounded_edge.texture_size, (139, 79).into());
    }

    #[test]
    fn the_full_output_region_covers_the_output_at_every_scale() {
        let screen = output(1280, 800);
        let at_one = MaterialRegion::full_output(screen, 1.0.into()).expect("full output");
        assert_eq!(at_one.rect, screen);
        assert_eq!(
            at_one.physical,
            Rectangle::new((0, 0).into(), (1280, 800).into())
        );
        assert_eq!(at_one.texture_size, (1280, 800).into());

        let at_one_and_a_half =
            MaterialRegion::full_output(screen, 1.5.into()).expect("full output");
        assert_eq!(
            at_one_and_a_half.physical,
            Rectangle::new((0, 0).into(), (1920, 1200).into())
        );
        assert_eq!(at_one_and_a_half.texture_size, (1920, 1200).into());

        let at_two = MaterialRegion::full_output(screen, 2.0.into()).expect("full output");
        assert_eq!(at_two.texture_size, (2560, 1600).into());

        // Invalid inputs are rejected, never a zero-sized texture.
        assert_eq!(
            MaterialRegion::full_output(output(0, 800), 1.0.into()),
            None
        );
        assert_eq!(MaterialRegion::full_output(screen, 0.0.into()), None);
    }

    #[test]
    fn a_panel_source_is_translated_by_the_region_origin() {
        let screen = output(1280, 800);
        let panel_rect = panel((101, 51, 333, 77), 12.0).panel;
        let region =
            MaterialRegion::from_panels(&[panel((101, 51, 333, 77), 12.0)], screen, 1.0.into())
                .expect("visible panel");
        assert_eq!(
            region.panel_source(panel_rect, 1.0.into()),
            Some((
                panel_rect,
                Rectangle::new((12.0, 12.0).into(), (333.0, 77.0).into())
            ))
        );

        // At 1.5x the panel physical origin is (152,77), the region origin is
        // (134,59), so the source starts at (18,18) inside the texture.
        let region =
            MaterialRegion::from_panels(&[panel((101, 51, 333, 77), 12.0)], screen, 1.5.into())
                .expect("visible panel");
        assert_eq!(
            region.panel_source(panel_rect, 1.5.into()),
            Some((
                panel_rect,
                Rectangle::new((18.0, 18.0).into(), (500.0, 116.0).into())
            ))
        );

        // A panel from a different region has no source here.
        assert_eq!(
            region.panel_source(Rectangle::new((10, 10).into(), (40, 40).into()), 1.5.into()),
            None
        );
    }

    #[test]
    fn a_panel_source_clips_to_the_region() {
        let screen = output(1280, 800);
        let panel_rect = Rectangle::new((1200, 760).into(), (200, 100).into());
        let region = MaterialRegion::from_panels(
            &[BlurPanel {
                key: crate::window::PanelKey::new("test", 2, 0),
                panel: panel_rect,
                spec: BlurSpec::new(8.0, 10.0),
                glass: GlassSpec {
                    refraction: 0.0,
                    specular: 0.0,
                    specular_width: 0.0,
                    tint: 0.0,
                    tint_color: [0.0; 3],
                },
            }],
            screen,
            1.0.into(),
        )
        .expect("visible panel");

        // The region is (1192,752,88,48); the panel's visible part is the
        // bottom-right 80x40, starting 8px into the texture.
        assert_eq!(
            region.panel_source(panel_rect, 1.0.into()),
            Some((
                Rectangle::new((1200, 760).into(), (80, 40).into()),
                Rectangle::new((8.0, 8.0).into(), (80.0, 40.0).into())
            ))
        );

        // The physical clamp at 1.5x wins over the panel's independently
        // rounded size: the source never reaches past the texture.
        let region = MaterialRegion::from_panels(
            &[BlurPanel {
                key: crate::window::PanelKey::new("test", 2, 0),
                panel: Rectangle::new((1199, 759).into(), (81, 41).into()),
                spec: BlurSpec::new(12.0, 10.0),
                glass: GlassSpec {
                    refraction: 0.0,
                    specular: 0.0,
                    specular_width: 0.0,
                    tint: 0.0,
                    tint_color: [0.0; 3],
                },
            }],
            screen,
            1.5.into(),
        )
        .expect("visible panel");
        let (visible, src) = region
            .panel_source(
                Rectangle::new((1199, 759).into(), (81, 41).into()),
                1.5.into(),
            )
            .expect("panel source");
        assert_eq!(visible, Rectangle::new((1199, 759).into(), (81, 41).into()));
        assert_eq!(src.loc, (18.0, 18.0).into());
        assert_eq!(src.size, (121.0, 61.0).into());
        // The source fits inside the region texture.
        assert!(src.loc.x + src.size.w <= region.texture_size.w as f64);
        assert!(src.loc.y + src.size.h <= region.texture_size.h as f64);
    }

    #[test]
    fn the_full_output_flag_accepts_only_explicit_truthy_values() {
        for value in ["1", "true", "TRUE", " on ", "yes", "enabled"] {
            assert!(env_flag_enabled(Some(value)), "{value:?}");
        }
        for value in ["", "0", "false", "off", "no", "disabled", "random"] {
            assert!(!env_flag_enabled(Some(value)), "{value:?}");
        }
        assert!(!env_flag_enabled(None));
    }
}
