// SPDX-License-Identifier: MIT
//! Chrome-surface geometry: anchors, margins, and reserved zones (T-07 FR-1).
//!
//! A `df_layer_surface` is placed by anchor edges on its output, exactly
//! like `wlr-layer-shell`: zero, one, or both edges per axis; margins inset
//! from the anchored edges; a non-zero size request; and an exclusive zone
//! that reserves pixels on the anchored edge for the usable (Zoom) area.
//!
//! The computation is pure and unit-tested; the protocol layer owns the
//! [`LayerSurfaceState`] and applies the result to the scene.

use smithay::utils::{Logical, Point, Rectangle};

use crate::window::ReservedZones;

/// Anchor bit for the top edge.
pub const ANCHOR_TOP: u32 = 1;
/// Anchor bit for the bottom edge.
pub const ANCHOR_BOTTOM: u32 = 2;
/// Anchor bit for the left edge.
pub const ANCHOR_LEFT: u32 = 4;
/// Anchor bit for the right edge.
pub const ANCHOR_RIGHT: u32 = 8;

/// The `overlay` layer (`df_shell.layer` value 3): transient chrome such as
/// menus, popovers, and the OSD.
pub const LAYER_OVERLAY: u32 = 3;

/// Which edge a reserved zone belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Edge {
    /// The `df_output.edge` wire value.
    #[allow(dead_code)] // Used by the layer tests; the protocol layer maps to the generated enum.
    pub const fn wire(self) -> u32 {
        match self {
            Edge::Top => 0,
            Edge::Bottom => 1,
            Edge::Left => 2,
            Edge::Right => 3,
        }
    }
}

/// Margins inset from the anchored edges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Margins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

/// Keyboard-interaction policy (FR-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardInteraction {
    /// Never takes keyboard focus.
    #[default]
    None,
    /// Takes focus for as long as it is mapped (modal OSD).
    Exclusive,
    /// Takes focus when clicked.
    OnDemand,
}

impl KeyboardInteraction {
    /// Parse the wire value.
    pub const fn from_wire(value: u32) -> Self {
        match value {
            1 => KeyboardInteraction::Exclusive,
            2 => KeyboardInteraction::OnDemand,
            _ => KeyboardInteraction::None,
        }
    }
}

/// The compositor-side state of one chrome surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSurfaceState {
    pub anchor: u32,
    pub width: i32,
    pub height: i32,
    pub margin: Margins,
    pub exclusive_zone: i32,
    pub keyboard: KeyboardInteraction,
    pub layer: u32,
    /// The output name this surface anchors to.
    pub output: Option<String>,
    pub namespace: String,
    /// The size last sent in `configure`, if any.
    pub configured: Option<(i32, i32)>,
}

impl Default for LayerSurfaceState {
    fn default() -> Self {
        LayerSurfaceState {
            anchor: 0,
            width: 0,
            height: 0,
            margin: Margins::default(),
            exclusive_zone: 0,
            keyboard: KeyboardInteraction::None,
            layer: 2,
            output: None,
            namespace: String::new(),
            configured: None,
        }
    }
}

impl LayerSurfaceState {
    fn anchored(&self, edge: u32) -> bool {
        self.anchor & edge != 0
    }

    /// Whether this surface is placed on the output named `output_name`.
    ///
    /// `None` targets **every** output, matching `wlr-layer-shell`: the menu
    /// bar and Dock are created without an explicit output, so they anchor on
    /// each display — including one attached by hotplug (FR-1).
    pub fn matches_output(&self, output_name: &str) -> bool {
        self.output
            .as_deref()
            .map_or(true, |name| name == output_name)
    }

    /// Whether this surface is visible on `output_name`, given the output the
    /// last chrome interaction happened on (`chrome_focus_output`).
    ///
    /// An explicit `output` always wins. Otherwise, an `overlay` surface with
    /// no explicit output — a popover, menu, or OSD — is **per-output**: it is
    /// shown only on the output the interaction happened on, so a Dock or menu
    /// popover opened on one display never floats across all of them (T-10
    /// section 18). Before any chrome surface has taken focus there is no
    /// capture, and overlays fall back to every output (their pre-T-10
    /// behavior, which keeps a directly-mapped popup testable).
    pub fn visible_on_output(&self, output_name: &str, chrome_focus_output: Option<&str>) -> bool {
        if !self.matches_output(output_name) {
            return false;
        }
        if self.layer != LAYER_OVERLAY || self.output.is_some() {
            return true;
        }
        // A full-output overlay (anchored on all four edges) is a session-wide
        // layer such as Mission Control, not a per-output popover: it spans
        // every display (T-11 U-4). Per-output popovers anchor to fewer edges.
        if self.is_full_output() {
            return true;
        }
        chrome_focus_output.map_or(true, |name| name == output_name)
    }

    /// Whether the surface covers the whole output on both axes (anchored to
    /// all four edges). Used to distinguish session-wide overlays (Mission
    /// Control) from per-output popovers.
    pub fn is_full_output(&self) -> bool {
        self.anchored(ANCHOR_TOP)
            && self.anchored(ANCHOR_BOTTOM)
            && self.anchored(ANCHOR_LEFT)
            && self.anchored(ANCHOR_RIGHT)
    }

    /// Resolve the surface rectangle for `output` geometry.
    ///
    /// * A zero size on an axis stretches between both anchors (or the whole
    ///   output when unanchored); a positive size is clamped to the output.
    /// * An unanchored axis is centred.
    pub fn geometry(&self, output: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        let both_h = self.anchored(ANCHOR_LEFT) && self.anchored(ANCHOR_RIGHT);
        let both_v = self.anchored(ANCHOR_TOP) && self.anchored(ANCHOR_BOTTOM);

        let mut width = if self.width > 0 {
            self.width
        } else if both_h {
            output.size.w - self.margin.left - self.margin.right
        } else {
            output.size.w
        };
        let mut height = if self.height > 0 {
            self.height
        } else if both_v {
            output.size.h - self.margin.top - self.margin.bottom
        } else {
            output.size.h
        };
        width = width.max(1).min(output.size.w.max(1));
        height = height.max(1).min(output.size.h.max(1));

        let x = if both_h || self.anchored(ANCHOR_LEFT) {
            output.loc.x + self.margin.left
        } else if self.anchored(ANCHOR_RIGHT) {
            output.loc.x + output.size.w - width - self.margin.right
        } else {
            output.loc.x + (output.size.w - width) / 2
        };

        let y = if both_v || self.anchored(ANCHOR_TOP) {
            output.loc.y + self.margin.top
        } else if self.anchored(ANCHOR_BOTTOM) {
            output.loc.y + output.size.h - height - self.margin.bottom
        } else {
            output.loc.y + (output.size.h - height) / 2
        };

        Rectangle::new(Point::from((x, y)), (width, height).into())
    }

    /// The reserved zone this surface contributes, if any.
    ///
    /// A positive exclusive zone on a single edge reserves that edge. The
    /// vertical edges (top/bottom) are considered first, then the horizontal
    /// edges (left/right) so a vertical Dock anchored `left|top|bottom`
    /// reserves its left edge (T-10 section 2). Ambiguous anchors (both edges
    /// on the same axis, or no anchor) contribute nothing; a negative zone
    /// explicitly opts out.
    pub fn reserved(&self) -> Option<(Edge, i32)> {
        if self.exclusive_zone <= 0 {
            return None;
        }
        match (self.anchored(ANCHOR_TOP), self.anchored(ANCHOR_BOTTOM)) {
            (true, false) => return Some((Edge::Top, self.exclusive_zone)),
            (false, true) => return Some((Edge::Bottom, self.exclusive_zone)),
            _ => {}
        }
        match (self.anchored(ANCHOR_LEFT), self.anchored(ANCHOR_RIGHT)) {
            (true, false) => Some((Edge::Left, self.exclusive_zone)),
            (false, true) => Some((Edge::Right, self.exclusive_zone)),
            _ => None,
        }
    }

    /// The visible strip this surface reserves on its anchored edge, within
    /// `geometry` (the menu bar's bar, the Dock's bar), or `None` when it
    /// reserves nothing (popovers, overlays, auto-hidden chrome).
    ///
    /// The Dock's layer surface is larger than its bar: the extra
    /// `magnifyBand` above/beside the bar is transparent room for magnified
    /// icons. The reserved zone reports the bar only, so this is the visible
    /// chrome the backdrop material belongs behind.
    pub fn reserved_rect(
        &self,
        geometry: Rectangle<i32, Logical>,
    ) -> Option<Rectangle<i32, Logical>> {
        let (edge, thickness) = self.reserved()?;
        if thickness <= 0 {
            return None;
        }
        Some(match edge {
            Edge::Top => Rectangle::new(
                geometry.loc,
                (geometry.size.w, thickness.min(geometry.size.h)).into(),
            ),
            Edge::Bottom => {
                let h = thickness.min(geometry.size.h);
                Rectangle::new(
                    (geometry.loc.x, geometry.loc.y + geometry.size.h - h).into(),
                    (geometry.size.w, h).into(),
                )
            }
            Edge::Left => Rectangle::new(
                geometry.loc,
                (thickness.min(geometry.size.w), geometry.size.h).into(),
            ),
            Edge::Right => {
                let w = thickness.min(geometry.size.w);
                Rectangle::new(
                    (geometry.loc.x + geometry.size.w - w, geometry.loc.y).into(),
                    (w, geometry.size.h).into(),
                )
            }
        })
    }
}

/// Fold a set of chrome surfaces into the compositor's global reserved
/// zones (the union of every edge, so Zoom never overlaps any chrome).
///
/// Multi-monitor reserved zones are per-output in the design; the current
/// `DfState.reserved_zones` is a single struct, so this takes the union and
/// is a documented follow-up for T-11/T-16.
pub fn aggregate_reserved<'a>(
    layers: impl Iterator<Item = &'a LayerSurfaceState>,
    mut zones: ReservedZones,
) -> ReservedZones {
    for layer in layers {
        if let Some((edge, thickness)) = layer.reserved() {
            match edge {
                Edge::Top => zones.top = zones.top.max(thickness),
                Edge::Bottom => zones.bottom = zones.bottom.max(thickness),
                Edge::Left => zones.left = zones.left.max(thickness),
                Edge::Right => zones.right = zones.right.max(thickness),
            }
        }
    }
    zones
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output() -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((0, 0)), (1920, 1080).into())
    }

    fn anchored_top_bar() -> LayerSurfaceState {
        LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT,
            height: 24,
            ..Default::default()
        }
    }

    #[test]
    fn full_width_top_bar_spans_and_reserves_top() {
        let bar = anchored_top_bar();
        let geometry = bar.geometry(output());
        assert_eq!(
            geometry,
            Rectangle::new(Point::from((0, 0)), (1920, 24).into())
        );
        assert_eq!(
            bar.reserved(),
            None,
            "a zero exclusive zone reserves nothing"
        );
        let bar = LayerSurfaceState {
            exclusive_zone: 24,
            ..bar
        };
        assert_eq!(bar.reserved(), Some((Edge::Top, 24)));
    }

    #[test]
    fn bottom_dock_reserves_bottom() {
        let dock = LayerSurfaceState {
            anchor: ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT,
            height: 80,
            exclusive_zone: 80,
            ..Default::default()
        };
        assert_eq!(
            dock.geometry(output()),
            Rectangle::new(Point::from((0, 1000)), (1920, 80).into())
        );
        assert_eq!(dock.reserved(), Some((Edge::Bottom, 80)));
    }

    #[test]
    fn reserved_rect_is_the_visible_bar_not_the_whole_surface() {
        // A bottom Dock surface is taller than its bar: the transparent
        // magnification band sits above the reserved bar.
        let dock = LayerSurfaceState {
            anchor: ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT,
            height: 131,
            exclusive_zone: 67,
            ..Default::default()
        };
        let surface = dock.geometry(output());
        assert_eq!(
            dock.reserved_rect(surface),
            Some(Rectangle::new(Point::from((0, 1013)), (1920, 67).into())),
            "the panel is the anchored bar strip, band excluded"
        );

        // A menu bar reserves its whole surface.
        let bar = LayerSurfaceState {
            exclusive_zone: 24,
            ..anchored_top_bar()
        };
        assert_eq!(
            bar.reserved_rect(bar.geometry(output())),
            Some(bar.geometry(output()))
        );

        // A popover (no reserve) has no separate panel rect.
        let popover = LayerSurfaceState {
            width: 320,
            height: 200,
            ..Default::default()
        };
        assert_eq!(popover.reserved_rect(popover.geometry(output())), None);

        // A left/right Dock rotates the strip with the anchored edge.
        let right = LayerSurfaceState {
            anchor: ANCHOR_RIGHT | ANCHOR_TOP | ANCHOR_BOTTOM,
            width: 131,
            exclusive_zone: 67,
            ..Default::default()
        };
        assert_eq!(
            right.reserved_rect(right.geometry(output())),
            Some(Rectangle::new(
                Point::from((1920 - 67, 0)),
                (67, 1080).into()
            ))
        );
    }

    #[test]
    fn centered_osd_is_centred_without_anchors() {
        let osd = LayerSurfaceState {
            width: 400,
            height: 200,
            ..Default::default()
        };
        assert_eq!(
            osd.geometry(output()),
            Rectangle::new(Point::from((760, 440)), (400, 200).into())
        );
        assert_eq!(osd.reserved(), None);
    }

    #[test]
    fn margins_inset_from_anchored_edges() {
        let panel = LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_RIGHT,
            width: 320,
            height: 200,
            margin: Margins {
                top: 24,
                right: 8,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            panel.geometry(output()),
            Rectangle::new(Point::from((1592, 24)), (320, 200).into())
        );
    }

    #[test]
    fn oversize_is_clamped_to_the_output() {
        let panel = LayerSurfaceState {
            width: 5000,
            height: 5000,
            ..Default::default()
        };
        assert_eq!(panel.geometry(output()).size, (1920, 1080).into());
    }

    #[test]
    fn ambiguous_anchor_does_not_reserve() {
        let panel = LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_BOTTOM,
            exclusive_zone: 40,
            ..Default::default()
        };
        assert_eq!(panel.reserved(), None);
    }

    #[test]
    fn vertical_dock_reserves_left_or_right() {
        // A left Dock spans the output height (top|bottom) and reserves its
        // left edge; the right Dock mirrors it (T-10 section 2).
        let left = LayerSurfaceState {
            anchor: ANCHOR_LEFT | ANCHOR_TOP | ANCHOR_BOTTOM,
            width: 80,
            exclusive_zone: 80,
            ..Default::default()
        };
        assert_eq!(left.reserved(), Some((Edge::Left, 80)));
        assert_eq!(
            left.geometry(output()),
            Rectangle::new(Point::from((0, 0)), (80, 1080).into())
        );

        let right = LayerSurfaceState {
            anchor: ANCHOR_RIGHT | ANCHOR_TOP | ANCHOR_BOTTOM,
            width: 80,
            exclusive_zone: 80,
            ..Default::default()
        };
        assert_eq!(right.reserved(), Some((Edge::Right, 80)));
        assert_eq!(
            right.geometry(output()),
            Rectangle::new(Point::from((1840, 0)), (80, 1080).into())
        );
    }

    #[test]
    fn bottom_dock_does_not_leak_into_a_horizontal_reserve() {
        // A bottom Dock also anchors left|right; the vertical edge must win
        // so it never reserves both a bottom and a side strip.
        let dock = LayerSurfaceState {
            anchor: ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT,
            exclusive_zone: 60,
            ..Default::default()
        };
        assert_eq!(dock.reserved(), Some((Edge::Bottom, 60)));
    }

    #[test]
    fn negative_exclusive_zone_opts_out() {
        let panel = LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT,
            exclusive_zone: -1,
            ..Default::default()
        };
        assert_eq!(panel.reserved(), None);
    }

    #[test]
    fn aggregate_takes_the_union_of_edges() {
        let top = anchored_top_bar();
        let dock = LayerSurfaceState {
            anchor: ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT,
            exclusive_zone: 80,
            ..Default::default()
        };
        let zones = aggregate_reserved([&top, &dock].into_iter(), ReservedZones::default());
        assert_eq!(zones.top, 0);
        assert_eq!(zones.bottom, 80);
    }

    #[test]
    fn keyboard_interaction_wire_values() {
        assert_eq!(KeyboardInteraction::from_wire(0), KeyboardInteraction::None);
        assert_eq!(
            KeyboardInteraction::from_wire(1),
            KeyboardInteraction::Exclusive
        );
        assert_eq!(
            KeyboardInteraction::from_wire(2),
            KeyboardInteraction::OnDemand
        );
        assert_eq!(KeyboardInteraction::from_wire(9), KeyboardInteraction::None);
    }

    #[test]
    fn edge_wire_values_match_the_xml() {
        assert_eq!(Edge::Top.wire(), 0);
        assert_eq!(Edge::Bottom.wire(), 1);
        assert_eq!(Edge::Left.wire(), 2);
        assert_eq!(Edge::Right.wire(), 3);
    }

    #[test]
    fn a_surface_without_an_output_targets_every_output() {
        // The menu bar is created with no output, so it must anchor on the
        // primary and on any output attached later (FR-1 hotplug).
        let bar = LayerSurfaceState::default();
        assert!(bar.matches_output("HEADLESS-1"));
        assert!(bar.matches_output("HDMI-A-1"));

        let pinned = LayerSurfaceState {
            output: Some("HDMI-A-1".into()),
            ..Default::default()
        };
        assert!(pinned.matches_output("HDMI-A-1"));
        assert!(!pinned.matches_output("HEADLESS-1"));
    }

    #[test]
    fn overlay_popovers_are_per_output_when_chrome_is_focused() {
        // A popover is an `overlay` surface created without an explicit
        // output. Once a chrome surface has taken focus on one display it
        // must render only there, not on every output (T-10 section 18).
        let popup = LayerSurfaceState {
            layer: LAYER_OVERLAY,
            ..Default::default()
        };
        assert!(popup.visible_on_output("HEADLESS-1", Some("HEADLESS-1")));
        assert!(!popup.visible_on_output("HDMI-A-1", Some("HEADLESS-1")));
        // No capture yet: fall back to every output (pre-T-10 behavior).
        assert!(popup.visible_on_output("HEADLESS-1", None));
        assert!(popup.visible_on_output("HDMI-A-1", None));
        // An output that detached while focused matches no remaining output,
        // so the popover disappears instead of moving to another display.
        assert!(!popup.visible_on_output("HEADLESS-1", Some("HDMI-A-1")));
    }

    #[test]
    fn full_output_overlays_span_every_output() {
        // Mission Control is an `overlay` anchored to all four edges: it must
        // cover every display even when chrome focus was captured on one
        // (T-11 U-4), unlike a per-output popover.
        let overview = LayerSurfaceState {
            layer: LAYER_OVERLAY,
            anchor: ANCHOR_TOP | ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT,
            exclusive_zone: -1,
            ..Default::default()
        };
        assert!(overview.is_full_output());
        assert!(overview.visible_on_output("HEADLESS-1", Some("HDMI-A-1")));
        assert!(overview.visible_on_output("HDMI-A-1", Some("HDMI-A-1")));

        // A three-edge overlay (a popover spanning the width but not height)
        // stays per-output.
        let popover = LayerSurfaceState {
            layer: LAYER_OVERLAY,
            anchor: ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT,
            ..Default::default()
        };
        assert!(!popover.is_full_output());
        assert!(!popover.visible_on_output("HDMI-A-1", Some("HEADLESS-1")));
    }

    #[test]
    fn persistent_chrome_still_spans_every_output() {
        // The menu bar and Dock are `top` chrome with no output: they must
        // stay on every display regardless of where chrome focus is.
        let bar = anchored_top_bar();
        assert!(bar.visible_on_output("HEADLESS-1", Some("HDMI-A-1")));
        assert!(bar.visible_on_output("HDMI-A-1", Some("HDMI-A-1")));

        // An explicit-output overlay ignores the focus capture and obeys its
        // own output.
        let pinned_popup = LayerSurfaceState {
            layer: LAYER_OVERLAY,
            output: Some("HDMI-A-1".into()),
            ..Default::default()
        };
        assert!(!pinned_popup.visible_on_output("HEADLESS-1", Some("HDMI-A-1")));
        assert!(pinned_popup.visible_on_output("HDMI-A-1", Some("HEADLESS-1")));
    }
}
