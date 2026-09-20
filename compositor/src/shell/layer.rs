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
}
