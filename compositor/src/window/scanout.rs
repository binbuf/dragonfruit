// SPDX-License-Identifier: MIT
//! Direct-scanout policy for the material pass (T-21.3).
//!
//! The sampled material pass exists to frost chrome. A fullscreen client that
//! covers the whole output with no visible material panel above it must not pay
//! for that pass: the frame can keep the feather/no-material stack and let
//! `DrmCompositor` put the client buffer on the primary plane. This module is
//! the pure decision; the backends own the actual render and the real
//! [`crate::state::RenderStats::direct_scanouts`] counter stays DRM-only.
//!
//! The policy is deliberately conservative. A fullscreen window that does not
//! cover the output (mid-transition, a client that requested a smaller
//! fullscreen), or any visible material panel, keeps the sampled path.

use smithay::utils::{Logical, Rectangle};

use crate::render::BlurPanel;

/// Whether `client` covers the complete `output` rectangle.
pub fn covers_output(client: Rectangle<i32, Logical>, output: Rectangle<i32, Logical>) -> bool {
    output.size.w > 0 && output.size.h > 0 && client.contains_rect(output)
}

/// Whether this output-frame can skip the sampled material pass.
///
/// True when a fullscreen client covers the output and no visible material
/// panel intersects it. The panel list is the frame's resolved
/// [`BlurPanel`]s, so a mapped Dock/menu bar or a transient popover keeps the
/// sampled pass and the frost. `fullscreen` is the client's output-local
/// rectangle; both rectangles must be in the same coordinate space.
pub fn direct_scanout_candidate(
    output: Rectangle<i32, Logical>,
    fullscreen: Option<Rectangle<i32, Logical>>,
    panels: &[BlurPanel],
) -> bool {
    let Some(fullscreen) = fullscreen else {
        return false;
    };
    if !covers_output(fullscreen, output) {
        return false;
    }
    !panels.iter().any(|panel| {
        panel
            .panel
            .intersection(fullscreen)
            .is_some_and(|visible| visible.size.w > 0 && visible.size.h > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::blur::{BlurSpec, GlassSpec};
    use crate::window::PanelKey;

    fn panel(rect: (i32, i32, i32, i32)) -> BlurPanel {
        BlurPanel {
            key: PanelKey::new("test", 2, 0),
            panel: Rectangle::new((rect.0, rect.1).into(), (rect.2, rect.3).into()),
            spec: BlurSpec::new(12.0, 10.0),
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
    fn no_fullscreen_client_keeps_the_material_pass() {
        assert!(!direct_scanout_candidate(output(1920, 1080), None, &[]));
    }

    #[test]
    fn a_partial_fullscreen_client_keeps_the_material_pass() {
        let partial = Rectangle::new((0, 0).into(), (1920, 1000).into());
        assert!(!direct_scanout_candidate(
            output(1920, 1080),
            Some(partial),
            &[]
        ));
    }

    #[test]
    fn a_fullscreen_client_with_no_material_panel_scans_out() {
        assert!(direct_scanout_candidate(
            output(1920, 1080),
            Some(output(1920, 1080)),
            &[]
        ));
    }

    #[test]
    fn a_visible_material_panel_keeps_the_sampled_pass() {
        let dock = panel((0, 955, 1920, 125));
        assert!(!direct_scanout_candidate(
            output(1920, 1080),
            Some(output(1920, 1080)),
            &[dock]
        ));
    }

    #[test]
    fn zero_area_or_offscreen_panels_do_not_block_scanout() {
        let zero = panel((0, 955, 1920, 0));
        let offscreen = panel((1920, 0, 100, 100));
        assert!(direct_scanout_candidate(
            output(1920, 1080),
            Some(output(1920, 1080)),
            &[zero, offscreen]
        ));
    }

    #[test]
    fn any_one_of_several_visible_panels_blocks_scanout() {
        let menu = panel((0, 0, 1920, 32));
        let dock = panel((0, 955, 1920, 125));
        assert!(!direct_scanout_candidate(
            output(1920, 1080),
            Some(output(1920, 1080)),
            &[menu, dock]
        ));
    }

    #[test]
    fn covers_output_requires_a_positive_output_and_a_full_cover() {
        assert!(covers_output(output(4, 3), output(4, 3)));
        assert!(!covers_output(output(4, 2), output(4, 3)));
        assert!(!covers_output(output(4, 3), output(0, 3)));
        // A larger client still covers the output.
        let larger = Rectangle::new((-10, -10).into(), (1940, 1100).into());
        assert!(covers_output(larger, output(1920, 1080)));
    }
}
