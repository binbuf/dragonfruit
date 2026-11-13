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

/// Selects how the material scene pass is bounded (T238 follow-up).
///
/// `union` renders one scene texture over the padded union of every panel.
/// With a menu bar at the top and a Dock at the bottom that union spans almost
/// the whole output, so the scene pass renders far more than either panel
/// samples. `split` (the default) gives the top and bottom panel groups
/// independent regions/textures, so each pass is bounded to the strip that
/// actually feeds a blur or veil.
pub const ENV_MATERIAL_REGION_GROUPS: &str = "DRAGONFRUIT_MATERIAL_REGION_GROUPS";

/// Diagnostic switch that disables the sampled material pass entirely.
///
/// The control for the T238 three-way comparison: no scene texture, no Kawase
/// chain, no glass/veil compose, and no feather backdrop. Chrome surfaces are
/// still drawn, so the composite cost of the scene itself can be measured
/// without any material work.
pub const ENV_MATERIAL_OFF: &str = "DRAGONFRUIT_MATERIAL_OFF";

/// Opt-in fast path for the material scene cache (T238 follow-up).
///
/// When enabled, a frame whose scene-content epoch is unchanged reuses the
/// existing scene texture without walking the element list through the damage
/// tracker. The default stays off until the event-driven epoch hooks have been
/// validated on native and multi-monitor sessions.
pub const ENV_SCENE_CACHE_SKIP: &str = "DRAGONFRUIT_SCENE_CACHE_SKIP";

/// Whether the event-driven scene-cache skip is enabled.
pub fn scene_cache_skip_enabled() -> bool {
    static SKIP: OnceLock<bool> = OnceLock::new();
    *SKIP.get_or_init(|| env_flag_enabled(std::env::var(ENV_SCENE_CACHE_SKIP).ok().as_deref()))
}

/// Whether the diagnostic no-material arm is active.
pub fn material_disabled() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    *DISABLED.get_or_init(|| env_flag_enabled(std::env::var(ENV_MATERIAL_OFF).ok().as_deref()))
}

/// Whether an explicit environment value enables a default-off flag.
pub(crate) fn env_flag_enabled(value: Option<&str>) -> bool {
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

/// Whether the independently bounded top/bottom material regions are enabled.
///
/// The T238 comparison measured the split arm at roughly one fifth of the
/// union's scene CPU time with unchanged panel sources, so it is the default.
/// `DRAGONFRUIT_MATERIAL_REGION_GROUPS=union` keeps the previous single-union
/// implementation as a rollback/diagnostic arm.
pub fn split_regions_enabled() -> bool {
    static SPLIT: OnceLock<bool> = OnceLock::new();
    *SPLIT.get_or_init(|| {
        !matches!(
            std::env::var(ENV_MATERIAL_REGION_GROUPS)
                .ok()
                .map(|value| value.trim().to_ascii_lowercase())
                .as_deref(),
            Some("union" | "off" | "0" | "false")
        )
    })
}

/// One bounded material scene pass: the region to render, the panels whose
/// blur/veil samples it, and the stable scene-texture slot.
#[derive(Debug, Clone)]
pub struct MaterialGroup {
    pub region: MaterialRegion,
    pub panels: Vec<BlurPanel>,
    /// Scene-cache slot: 0 for the top group, 1 for the bottom group. Stable
    /// across frames so a menu bar appearing does not re-home the Dock's
    /// texture.
    pub slot: usize,
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
    /// is expanded by its own [`BlurPanel::sample_padding_px`] before the
    /// union, so a larger blur, lens, or declared overscan on one panel does
    /// not silently shrink another panel's margin.
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
            let padding = blur_panel.sample_padding_px();
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

    /// Resolve one or two independently bounded material groups.
    ///
    /// The control arm (`union`) and the full-output diagnostic return a
    /// single group over every panel. The `split` arm partitions panels by
    /// their centre against the output's vertical midpoint and resolves one
    /// padded union per half. A top menu bar and a bottom Dock therefore get
    /// separate textures instead of one nearly-full-output union.
    pub fn groups_for_material(
        panels: &[BlurPanel],
        output: Rectangle<i32, Logical>,
        scale: Scale<f64>,
    ) -> Vec<MaterialGroup> {
        Self::groups_for_material_mode(
            panels,
            output,
            scale,
            split_regions_enabled(),
            full_output_material_forced(),
        )
    }

    /// The renderer-free grouping decision. `split` and `full_output` are
    /// passed explicitly so the headless tests can exercise every arm without
    /// mutating process-wide environment flags.
    pub(crate) fn groups_for_material_mode(
        panels: &[BlurPanel],
        output: Rectangle<i32, Logical>,
        scale: Scale<f64>,
        split: bool,
        full_output: bool,
    ) -> Vec<MaterialGroup> {
        if panels.is_empty() {
            return Vec::new();
        }
        let single = |region: MaterialRegion, slot: usize| MaterialGroup {
            region,
            panels: panels.to_vec(),
            slot,
        };
        if full_output {
            return Self::full_output(output, scale)
                .map(|region| vec![single(region, 0)])
                .unwrap_or_default();
        }
        if !split {
            return Self::from_panels(panels, output, scale)
                .or_else(|| Self::full_output(output, scale))
                .map(|region| vec![single(region, 0)])
                .unwrap_or_default();
        }

        let midpoint = output.loc.y.saturating_add(output.size.h / 2);
        let mut top: Vec<BlurPanel> = Vec::new();
        let mut bottom: Vec<BlurPanel> = Vec::new();
        for panel in panels {
            let center = panel.panel.loc.y.saturating_add(panel.panel.size.h / 2);
            if center < midpoint {
                top.push(*panel);
            } else {
                bottom.push(*panel);
            }
        }

        let mut groups = Vec::with_capacity(2);
        for (slot, group) in [(0usize, top), (1usize, bottom)] {
            if group.is_empty() {
                continue;
            }
            if let Some(region) = Self::from_panels(&group, output, scale) {
                groups.push(MaterialGroup {
                    region,
                    panels: group,
                    slot,
                });
            }
        }
        if groups.is_empty() {
            // Safety net for a degenerate panel set: keep the old fallback.
            Self::full_output(output, scale)
                .map(|region| vec![single(region, 0)])
                .unwrap_or_default()
        } else {
            groups
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
        self.panel_source_padded(panel, 0, scale)
    }

    /// Like [`Self::panel_source`], but expands the sampled rect by `padding`
    /// logical pixels before clamping it to the material region.
    ///
    /// The measured Dock bevel and Gaussian need scene samples beyond the
    /// visible panel: the lens reaches outside the silhouette and the blur
    /// must not clamp at the rim. The region itself is already padded by
    /// [`crate::render::BlurPanel::sample_padding_px`]; this method exposes
    /// that margin to the blur pass and to the element's UV mapping.
    pub fn panel_source_padded(
        &self,
        panel: Rectangle<i32, Logical>,
        padding: i32,
        scale: Scale<f64>,
    ) -> Option<(Rectangle<i32, Logical>, Rectangle<f64, Buffer>)> {
        let padding = padding.max(0);
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
        let visible = expanded.intersection(self.rect)?;
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
    use crate::material_preset::{Preset, RenderMode};
    use crate::render::RegionPaint;
    use crate::window::blur::{BlurSpec, GlassSpec, RegionFilter};
    use crate::window::{ColorScheme, CornerRadii, DegradeTier};

    fn paint() -> RegionPaint {
        RegionPaint {
            scheme: ColorScheme::Light,
            preset: Preset::MenuDetached,
            tier: DegradeTier::Full,
            mode: RenderMode::SampledGlass,
            alpha: 0.5,
            tint: 0.9,
            edge: 0.0,
            edge_width: 0.0,
            foreground: 0.0,
            revision: 0,
        }
    }

    fn panel(rect: (i32, i32, i32, i32), radius: f32) -> BlurPanel {
        BlurPanel {
            key: crate::window::PanelKey::new("test", 2, 0),
            panel: Rectangle::new((rect.0, rect.1).into(), (rect.2, rect.3).into()),
            corners: CornerRadii::uniform(10),
            overscan: 0,
            spec: BlurSpec::new(radius, 10.0),
            glass: GlassSpec {
                refraction: 0.0,
                specular: 0.0,
                specular_width: 0.0,
                tint: 0.0,
                tint_color: [0.0; 3],
                ..GlassSpec::default()
            },
            veil: None,
            filter: RegionFilter::Filtered,
            paint: paint(),
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
    fn the_sample_padding_includes_refraction_and_declared_overscan() {
        let screen = output(1920, 1080);
        let mut region_panel = panel((100, 100, 200, 80), 8.0);
        // radius 8 -> 1 iteration -> blur padding 8; refraction 10 has a
        // 25 px lens falloff, plus one sample pixel; overscan adds 5.
        region_panel.glass.refraction = 10.0;
        region_panel.overscan = 5;
        let expected = region_panel.glass.sample_padding_px() + 5;
        assert!(expected > region_panel.spec.padding_px());
        let region = MaterialRegion::from_panels(&[region_panel], screen, 1.0.into())
            .expect("visible panel");
        assert_eq!(
            region.rect,
            Rectangle::new(
                (100 - expected, 100 - expected).into(),
                (200 + expected * 2, 80 + expected * 2).into()
            )
        );

        // A sharp/veil region does not reserve the blur/lens footprint; only
        // its declared overscan plus the one-pixel sample edge.
        let mut veil = panel((100, 100, 200, 80), 24.0);
        veil.filter = RegionFilter::Sharp;
        veil.overscan = 7;
        let region =
            MaterialRegion::from_panels(&[veil], screen, 1.0.into()).expect("visible veil");
        assert_eq!(
            region.rect,
            Rectangle::new((92, 92).into(), (216, 96).into()),
            "sharp/veil padding is 1 + overscan, not the blur radius"
        );
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
    fn a_padded_panel_source_expands_then_clamps_to_the_region() {
        let screen = output(1280, 800);
        let panel_rect = panel((101, 51, 333, 77), 12.0).panel;
        let region =
            MaterialRegion::from_panels(&[panel((101, 51, 333, 77), 12.0)], screen, 1.0.into())
                .expect("visible panel");
        // The region is the panel expanded by the panel's own sample padding
        // (12 px here), so a 5 px request expands inside that margin.
        let (visible, src) = region
            .panel_source_padded(panel_rect, 5, 1.0.into())
            .expect("padded source");
        assert_eq!(visible, Rectangle::new((96, 46).into(), (343, 87).into()));
        assert_eq!(src, Rectangle::new((7.0, 7.0).into(), (343.0, 87.0).into()));
        // A request larger than the region margin clamps to the region.
        let (visible, _) = region
            .panel_source_padded(panel_rect, 999, 1.0.into())
            .expect("clamped source");
        assert_eq!(visible, region.rect);
    }

    #[test]
    fn a_panel_source_clips_to_the_region() {
        let screen = output(1280, 800);
        let panel_rect = Rectangle::new((1200, 760).into(), (200, 100).into());
        let region = MaterialRegion::from_panels(
            &[panel(
                (
                    panel_rect.loc.x,
                    panel_rect.loc.y,
                    panel_rect.size.w,
                    panel_rect.size.h,
                ),
                8.0,
            )],
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
        let region =
            MaterialRegion::from_panels(&[panel((1199, 759, 81, 41), 12.0)], screen, 1.5.into())
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
    fn a_4k_output_reports_the_full_output_and_dock_only_region_areas() {
        // T-21.3 4K trace: the same helper the backends call. The menu-bar +
        // Dock union is the full output (the documented single-union shape);
        // a Dock-only frame is the region the sampled pass actually pays for.
        let screen = output(3840, 2160);
        let menu = panel((0, 0, 3840, 32), 24.0);
        let dock = panel((640, 2035, 2560, 125), 24.0);
        let full_px = 3840u64 * 2160;

        let both =
            MaterialRegion::from_panels(&[menu, dock], screen, 1.0.into()).expect("both panels");
        assert_eq!(both.texture_size, (3840, 2160).into());
        println!(
            "t21-region-4k: output=3840x2160 full_px={full_px} \
             both_panels_region={}x{} both_panels_px={} ratio={:.1}%",
            both.texture_size.w,
            both.texture_size.h,
            both.texture_size.w as u64 * both.texture_size.h as u64,
            100.0 * (both.texture_size.w as u64 * both.texture_size.h as u64) as f64
                / full_px as f64
        );

        let dock_only =
            MaterialRegion::from_panels(&[dock], screen, 1.0.into()).expect("dock panel");
        assert_eq!(dock_only.texture_size, (2608, 149).into());
        let dock_px = 2608u64 * 149;
        assert_eq!(dock_px, 388_592);
        println!(
            "t21-region-4k: dock_only_region={}x{} dock_only_px={dock_px} ratio={:.1}%",
            dock_only.texture_size.w,
            dock_only.texture_size.h,
            100.0 * dock_px as f64 / full_px as f64
        );

        let diagnostic = MaterialRegion::full_output(screen, 1.0.into()).expect("full output");
        assert_eq!(diagnostic.texture_size, (3840, 2160).into());
        println!(
            "t21-region-4k: full_output_diagnostic_region={}x{} px={}",
            diagnostic.texture_size.w,
            diagnostic.texture_size.h,
            diagnostic.texture_size.w as u64 * diagnostic.texture_size.h as u64
        );

        // At 2x the same arithmetic scales to physical pixels.
        let dock_only_2x =
            MaterialRegion::from_panels(&[dock], screen, 2.0.into()).expect("dock panel at 2x");
        assert_eq!(dock_only_2x.texture_size, (5216, 298).into());
        println!(
            "t21-region-4k: dock_only_region_2x={}x{} px={}",
            dock_only_2x.texture_size.w,
            dock_only_2x.texture_size.h,
            dock_only_2x.texture_size.w as u64 * dock_only_2x.texture_size.h as u64
        );
    }

    #[test]
    fn split_groups_bound_top_and_bottom_independently() {
        let screen = output(1920, 1200);
        let top = panel((0, 0, 1920, 30), 12.0);
        let bottom = panel((600, 1100, 720, 80), 12.0);
        let scale = 1.0.into();

        let union =
            MaterialRegion::groups_for_material_mode(&[top, bottom], screen, scale, false, false);
        assert_eq!(union.len(), 1);
        assert!(
            union[0].region.rect.size.h > 1000,
            "the union spans the top and bottom panels: {:?}",
            union[0].region.rect
        );

        let split =
            MaterialRegion::groups_for_material_mode(&[top, bottom], screen, scale, true, false);
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].slot, 0);
        assert_eq!(split[1].slot, 1);
        assert_eq!(split[0].panels.len(), 1);
        assert_eq!(split[1].panels.len(), 1);
        assert!(
            split[0].region.rect.size.h < 100,
            "the top strip is bounded: {:?}",
            split[0].region.rect
        );
        assert!(
            split[1].region.rect.size.h < 200,
            "the bottom strip is bounded: {:?}",
            split[1].region.rect
        );
        // The two regions do not overlap the empty middle of the output.
        assert!(!split[0]
            .region
            .rect
            .overlaps(Rectangle::new((0, 500).into(), (1920, 200).into())));
        assert!(!split[1]
            .region
            .rect
            .overlaps(Rectangle::new((0, 500).into(), (1920, 200).into())));
    }

    #[test]
    fn split_preserves_each_panels_absolute_scene_source() {
        let screen = output(1920, 1200);
        let top = panel((0, 0, 1920, 30), 12.0);
        let bottom = panel((600, 1100, 720, 80), 12.0);
        let scale = 1.0.into();
        let union =
            MaterialRegion::groups_for_material_mode(&[top, bottom], screen, scale, false, false);
        let split =
            MaterialRegion::groups_for_material_mode(&[top, bottom], screen, scale, true, false);

        let absolute = |region: &MaterialRegion, panel: &BlurPanel| {
            let (_, src) = region.panel_source(panel.panel, scale).expect("source");
            (
                region.physical.loc.x + src.loc.x as i32,
                region.physical.loc.y + src.loc.y as i32,
                src.size.w as i32,
                src.size.h as i32,
            )
        };

        // The bottom panel samples the same scene pixels in both arms even
        // though the region origin (and therefore the buffer coordinate)
        // differs. This is the visual-equivalence contract for the split.
        assert_eq!(
            absolute(&union[0].region, &bottom),
            absolute(&split[1].region, &bottom)
        );
        assert_eq!(
            absolute(&union[0].region, &top),
            absolute(&split[0].region, &top)
        );
    }

    #[test]
    fn split_preserves_absolute_sources_at_fractional_scale() {
        let screen = output(1920, 1200);
        let top = panel((0, 0, 1920, 30), 12.0);
        let bottom = panel((600, 1100, 720, 80), 12.0);
        let scale = Scale::from(1.5);
        let union =
            MaterialRegion::groups_for_material_mode(&[top, bottom], screen, scale, false, false);
        let split =
            MaterialRegion::groups_for_material_mode(&[top, bottom], screen, scale, true, false);
        assert_eq!(split.len(), 2);
        let absolute = |region: &MaterialRegion, panel: &BlurPanel| {
            let (_, src) = region.panel_source(panel.panel, scale).expect("source");
            (
                region.physical.loc.x + src.loc.x as i32,
                region.physical.loc.y + src.loc.y as i32,
                src.size.w as i32,
                src.size.h as i32,
            )
        };
        // Fractional rounding is applied once per region; the absolute source
        // must still match the union arm at the panel.
        assert_eq!(
            absolute(&union[0].region, &bottom),
            absolute(&split[1].region, &bottom)
        );
        assert_eq!(
            absolute(&union[0].region, &top),
            absolute(&split[0].region, &top)
        );
        assert!(split[0].region.texture_size.w > 0);
        assert!(split[1].region.texture_size.h > 0);
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
