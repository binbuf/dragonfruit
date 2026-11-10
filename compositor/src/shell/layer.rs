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

use crate::material_preset::Preset;
use crate::window::{CornerRadii, ReservedZones};

/// Upper bound on the independently keyed material regions one chrome surface
/// may declare (T197, raised by T203). A menu and a notification stack need a
/// handful; the Control Center declares its outer plate, 18 module capsules,
/// and up to two expanded lists, so the bound keeps the per-frame region list,
/// damage union, and panel-slot table bounded even for a hostile trusted
/// client.
pub const MAX_PANEL_REGIONS: usize = 32;

/// Maximum length of a region key (T197).
pub const MAX_PANEL_REGION_KEY: usize = 64;

/// Maximum rim/shadow/veil overscan beyond a region's visual bounds, in
/// logical pixels (T197).
pub const MAX_PANEL_REGION_OVERSCAN: i32 = 512;

/// Maximum per-corner radius accepted from a trusted client (T197). The
/// renderer clamps further to half the region's shorter side.
pub const MAX_PANEL_REGION_RADIUS: i32 = 4096;

/// One independently keyed material region declared by a chrome surface
/// (T197).
///
/// `rect` is the **visual** bounds in surface-local logical coordinates. It is
/// independent of the surface's input region and reserved zone: an attached
/// menu can extend its material above the input hit box, and a veil can reach
/// past the reserved strip without changing either. `overscan` is extra
/// rim/shadow/veil reach beyond the visual bounds. `corners` carries
/// per-corner radii so an attached menu keeps square top corners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelRegion {
    /// Stable identity within this surface, e.g. `attached`, `card-1`.
    pub key: String,
    /// The optical preset declared for this region, or `None` to derive it
    /// from the surface namespace/role.
    pub preset: Option<Preset>,
    /// Visual bounds, surface-local logical.
    pub rect: Rectangle<i32, Logical>,
    /// Per-corner clipping radii in logical pixels.
    pub corners: CornerRadii,
    /// Extra visual reach for rim, shadow, or veil beyond `rect`.
    pub overscan: i32,
}

impl PanelRegion {
    /// Whether this region has a visible area.
    pub fn is_visible(&self) -> bool {
        self.rect.size.w > 0 && self.rect.size.h > 0
    }
}

/// Validate and build one region from protocol metadata (T197).
///
/// Invalid keys, unknown presets, and empty keys are rejected (the request is
/// ignored, the previous committed regions stay intact). Geometry and radii are
/// clamped exactly like `set_panel_rect`: negative extents cannot panic the
/// geometry type.
#[allow(clippy::too_many_arguments)] // mirrors the 11 protocol arguments
pub fn panel_region(
    key: &str,
    preset: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    top_left: i32,
    top_right: i32,
    bottom_right: i32,
    bottom_left: i32,
    overscan: i32,
) -> Option<PanelRegion> {
    if !valid_region_key(key) {
        return None;
    }
    let preset = if preset.is_empty() {
        None
    } else {
        Some(Preset::from_name(preset)?)
    };
    let clamp_radius = |radius: i32| radius.clamp(0, MAX_PANEL_REGION_RADIUS);
    Some(PanelRegion {
        key: key.to_owned(),
        preset,
        rect: panel_rect(x, y, width, height),
        corners: CornerRadii {
            top_left: clamp_radius(top_left),
            top_right: clamp_radius(top_right),
            bottom_right: clamp_radius(bottom_right),
            bottom_left: clamp_radius(bottom_left),
        },
        overscan: overscan.clamp(0, MAX_PANEL_REGION_OVERSCAN),
    })
}

/// A region key is a short ASCII token: letters, digits, `.`, `_`, and `-`.
/// The stable identity must survive logging and a hash without escaping.
pub fn valid_region_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_PANEL_REGION_KEY
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// Anchor bit for the top edge.
pub const ANCHOR_TOP: u32 = 1;
/// Anchor bit for the bottom edge.
pub const ANCHOR_BOTTOM: u32 = 2;
/// Anchor bit for the left edge.
pub const ANCHOR_LEFT: u32 = 4;
/// Anchor bit for the right edge.
pub const ANCHOR_RIGHT: u32 = 8;

/// The `background` layer (`df_shell.layer` value 0): the Files-owned desktop
/// icons surface (T-19.3). It composites **above the per-Space wallpaper and
/// below every window**, so desktop items sit behind open windows.
pub const LAYER_BACKGROUND: u32 = 0;

/// The `top` layer (`df_shell.layer` value 2): persistent chrome such as the
/// menu bar and the Dock.
pub const LAYER_TOP: u32 = 2;

/// The `overlay` layer (`df_shell.layer` value 3): transient chrome such as
/// menus, popovers, and the OSD.
pub const LAYER_OVERLAY: u32 = 3;

/// Whether `layer` composites **above** normal windows (`top`/`overlay`).
/// The lower layers (`background`/`bottom`) composite below windows.
pub const fn above_windows(layer: u32) -> bool {
    layer >= LAYER_TOP
}

/// Whether `layer` is the Files-owned desktop band (`background`, T-19.3).
pub const fn is_desktop(layer: u32) -> bool {
    layer == LAYER_BACKGROUND
}

/// Whether an above-window chrome surface stays visible while a fullscreen
/// client owns the output (T-21.3).
///
/// Persistent `top` chrome (the menu bar and Dock) is hidden: the fullscreen
/// client owns the display, so the compositor does not paint the bars over it
/// and the client can take the direct-scanout path. Transient `overlay`
/// surfaces (OSD, menus, notification, popovers) still composite above the
/// client and keep the sampled material when they are visible.
pub const fn visible_in_fullscreen(layer: u32) -> bool {
    layer != LAYER_TOP
}

/// The compositing band a `df_shell.layer` value occupies, from the per-Space
/// wallpaper up. Windows are compositor-stacked between [`StackBand::Desktop`]
/// and [`StackBand::Chrome`], so the derived order is the layer contract the
/// backends implement (T-19.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StackBand {
    /// The compositor-owned per-Space wallpaper (bottom-most).
    Wallpaper,
    /// The Files desktop-icon surface (`background`).
    Desktop,
    /// A normal client window (compositor `Space`).
    Windows,
    /// Above-window chrome (`top`/`overlay`).
    Chrome,
}

/// The band a `df_shell.layer` value composites in.
///
/// `background` is the desktop band; `top`/`overlay` are chrome. The
/// unimplemented `bottom` layer is treated as desktop-band so it never leaks
/// above windows (T-10/T-11 own its eventual policy).
pub const fn stack_band(layer: u32) -> StackBand {
    if is_desktop(layer) || layer == 1 {
        StackBand::Desktop
    } else {
        StackBand::Chrome
    }
}

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
    /// The panel rect the surface explicitly declared via
    /// `df_layer_surface.set_panel_rect`, in surface-local logical
    /// coordinates (T-14.7b). `None` means the compositor derives the panel
    /// from the input region and reserved strip (the menu bar, popovers).
    pub panel_rect: Option<Rectangle<i32, Logical>>,
    /// Independently keyed material regions declared via
    /// `df_layer_surface.set_panel_region` (T197), in surface-local logical
    /// coordinates. `None` means no multi-region metadata was committed (use
    /// [`Self::panel_rect`] or the derived panel). `Some(empty)` is an
    /// explicit empty panel set: the surface draws no material.
    pub panel_regions: Option<Vec<PanelRegion>>,
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
            panel_rect: None,
            panel_regions: None,
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

/// Add one edge reserve to `zones`, keeping the deepest reserve per edge.
fn reserve(zones: &mut ReservedZones, edge: Edge, thickness: i32) {
    match edge {
        Edge::Top => zones.top = zones.top.max(thickness),
        Edge::Bottom => zones.bottom = zones.bottom.max(thickness),
        Edge::Left => zones.left = zones.left.max(thickness),
        Edge::Right => zones.right = zones.right.max(thickness),
    }
}

/// Build the declared panel rect for a chrome surface from client-supplied
/// geometry.
///
/// `set_panel_rect` is untrusted client input: the rect legitimately shrinks to
/// nothing while a surface animates, but a negative size would panic Smithay's
/// geometry type and take the whole session down. Clamp both extents.
pub fn panel_rect(x: i32, y: i32, width: i32, height: i32) -> Rectangle<i32, Logical> {
    Rectangle::new((x, y).into(), (width.max(0), height.max(0)).into())
}

/// Fold a set of chrome surfaces into the compositor's global reserved
/// zones (the union of every edge, so Zoom never overlaps any chrome).
///
/// This is the fallback for an output that has not been resolved per-output
/// yet; [`aggregate_reserved_for`] is the per-output path (T-16.1a).
pub fn aggregate_reserved<'a>(
    layers: impl Iterator<Item = &'a LayerSurfaceState>,
    mut zones: ReservedZones,
) -> ReservedZones {
    for layer in layers {
        if let Some((edge, thickness)) = layer.reserved() {
            reserve(&mut zones, edge, thickness);
        }
    }
    zones
}

/// Fold the chrome surfaces that target `output_name` into that output's
/// reserved zones (T-16.1a).
///
/// [`LayerSurfaceState::matches_output`] is the filter: a surface created
/// without an explicit output (the menu bar, the Dock) targets **every**
/// display and reserves on each of them; a surface pinned to one output
/// reserves only there. Each edge keeps the deepest reserve, so a pinned
/// Dock and an all-output bar can reserve different edges on the same
/// output without either hiding the other. Reserved thicknesses are logical
/// pixels, so an output's scale does not change them — only the geometry
/// they are subtracted from.
pub fn aggregate_reserved_for<'a>(
    output_name: &str,
    layers: impl Iterator<Item = &'a LayerSurfaceState>,
    mut zones: ReservedZones,
) -> ReservedZones {
    for layer in layers {
        if !layer.matches_output(output_name) {
            continue;
        }
        if let Some((edge, thickness)) = layer.reserved() {
            reserve(&mut zones, edge, thickness);
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
    fn auto_hide_dock_reserves_nothing_and_fullscreen_hides_it() {
        // T237: a hidden auto-hide Dock sends exclusive_zone 0, so it removes
        // no usable area on any output (the shell's reservedThickness is 0 for
        // every reveal state), and the persistent top layer is hidden while a
        // fullscreen client owns the output.
        let dock = LayerSurfaceState {
            anchor: ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT,
            height: 131,
            exclusive_zone: 0,
            ..Default::default()
        };
        assert!(dock.matches_output("HEADLESS-1"));
        assert!(dock.matches_output("HDMI-A-1"));
        assert_eq!(dock.reserved(), None);
        assert_eq!(
            aggregate_reserved_for("HEADLESS-1", [&dock].into_iter(), ReservedZones::default()),
            ReservedZones::default(),
            "the auto-hide Dock reserves nothing on the primary"
        );
        assert_eq!(
            aggregate_reserved_for("HDMI-A-1", [&dock].into_iter(), ReservedZones::default()),
            ReservedZones::default(),
            "the auto-hide Dock reserves nothing on a second output"
        );
        assert!(!visible_in_fullscreen(LAYER_TOP));
    }

    #[test]
    fn menu_bar_veil_extent_is_visual_only_beyond_the_reserved_strip() {
        // T200: the layer surface is tall enough for the 95 pt veil, but the
        // exclusive zone stays the 28 pt bar strip. The visual region extends
        // past the strip without changing input or reservation.
        let bar = LayerSurfaceState {
            height: 95,
            exclusive_zone: 28,
            ..anchored_top_bar()
        };
        let geometry = bar.geometry(output());
        assert_eq!(geometry.size, (1920, 95).into());
        assert_eq!(
            bar.reserved_rect(geometry),
            Some(Rectangle::new(Point::from((0, 0)), (1920, 28).into())),
            "the veil's lower 67 pt must not reserve usable area"
        );
        let veil = panel_region("veil", "menuBarVeil", 0, 0, 1920, 95, 0, 0, 0, 0, 0)
            .expect("valid veil region");
        assert_eq!(veil.rect.size, (1920, 95).into());
        assert!(veil.is_visible());
        // Output isolation: the same surface-local region is clipped to the
        // owning output, never drawn across a second display.
        let surface = Rectangle::new(Point::from((0, 0)), (1920, 95).into());
        let other_output = Rectangle::new(Point::from((1280, 0)), (960, 540).into());
        let bounds = crate::window::explicit_panel_bounds(surface, veil.rect)
            .expect("region intersects its surface")
            .intersection(other_output)
            .expect("the visible slice on the second output");
        assert_eq!(bounds.size, (640, 95).into());
        assert_eq!(bounds.loc, Point::from((1280, 0)));
        // The top layer is hidden for a fullscreen client; the veil region
        // therefore contributes no material on that output.
        assert!(!visible_in_fullscreen(LAYER_TOP));
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
    fn per_output_reserves_are_scoped_by_matches_output() {
        // A Dock pinned to the primary output reserves only there; the
        // all-output menu bar reserves on every display (T-16.1a).
        let primary_dock = LayerSurfaceState {
            anchor: ANCHOR_BOTTOM | ANCHOR_LEFT | ANCHOR_RIGHT,
            exclusive_zone: 60,
            output: Some("eDP-1".into()),
            ..Default::default()
        };
        let menu_bar = LayerSurfaceState {
            exclusive_zone: 28,
            ..anchored_top_bar()
        };
        let layers = [&primary_dock, &menu_bar];

        let primary =
            aggregate_reserved_for("eDP-1", layers.iter().copied(), ReservedZones::default());
        assert_eq!(primary.top, 28, "the menu bar reserves on the primary");
        assert_eq!(
            primary.bottom, 60,
            "the pinned Dock reserves on the primary"
        );

        let secondary =
            aggregate_reserved_for("HDMI-A-1", layers.iter().copied(), ReservedZones::default());
        assert_eq!(
            secondary.top, 28,
            "the menu bar also reserves on the second"
        );
        assert_eq!(
            secondary.bottom, 0,
            "the Dock pinned to eDP-1 must not reserve on HDMI-A-1"
        );
    }

    #[test]
    fn per_output_keeps_the_deepest_reserve_on_an_edge() {
        // Two chrome surfaces can reserve the same edge on one output; the
        // deeper reserve wins, not the sum.
        let a = LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT,
            exclusive_zone: 24,
            ..Default::default()
        };
        let b = LayerSurfaceState {
            anchor: ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT,
            exclusive_zone: 30,
            ..Default::default()
        };
        let zones =
            aggregate_reserved_for("HEADLESS-1", [&a, &b].into_iter(), ReservedZones::default());
        assert_eq!(zones.top, 30);
    }

    #[test]
    fn chrome_geometry_is_resolved_per_output_scale() {
        // The same bar state sizes to each output's logical width, so two
        // outputs at different scales both reserve the correct zone (T-16.1a).
        let bar = LayerSurfaceState {
            height: 28,
            exclusive_zone: 28,
            ..anchored_top_bar()
        };
        let scale_one = Rectangle::new(Point::from((0, 0)), (1280, 720).into());
        let scale_two = Rectangle::new(Point::from((1280, 0)), (960, 540).into());

        assert_eq!(bar.geometry(scale_one).size, (1280, 28).into());
        assert_eq!(bar.geometry(scale_two).size, (960, 28).into());
        assert_eq!(
            bar.reserved_rect(bar.geometry(scale_two)),
            Some(Rectangle::new(Point::from((1280, 0)), (960, 28).into())),
            "the reserve follows the second output's logical geometry"
        );
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
    fn panel_region_metadata_is_validated_and_bounded() {
        // Attached menu: square top, rounded bottom, a known preset.
        let attached = panel_region(
            "attached",
            "menuAttached",
            10,
            20,
            240,
            300,
            0,
            0,
            12,
            12,
            20,
        )
        .expect("valid attached region");
        assert_eq!(attached.preset, Some(Preset::MenuAttached));
        assert_eq!(attached.rect.size, (240, 300).into());
        assert_eq!(attached.corners.top_left, 0);
        assert_eq!(attached.corners.bottom_left, 12);
        assert_eq!(attached.overscan, 20);
        assert!(attached.is_visible());

        // Empty preset means derive from the namespace/role.
        assert_eq!(
            panel_region("card", "", 0, 0, 10, 10, 4, 4, 4, 4, 0)
                .unwrap()
                .preset,
            None
        );

        // Unknown presets and malformed keys are rejected, not defaulted.
        assert!(panel_region("card", "not-a-preset", 0, 0, 10, 10, 4, 4, 4, 4, 0).is_none());
        assert!(panel_region("", "", 0, 0, 10, 10, 4, 4, 4, 4, 0).is_none());
        assert!(panel_region("bad key", "", 0, 0, 10, 10, 4, 4, 4, 4, 0).is_none());
        assert!(panel_region(
            &"x".repeat(MAX_PANEL_REGION_KEY + 1),
            "",
            0,
            0,
            10,
            10,
            4,
            4,
            4,
            4,
            0
        )
        .is_none());

        // Negative extents clamp; radii and overscan are bounded.
        let clamped = panel_region("clamped", "", 1, 2, -8, -9, -4, 99_999, -1, 7, 99_999)
            .expect("valid clamped region");
        assert_eq!(clamped.rect.size, (0, 0).into());
        assert_eq!(clamped.corners.top_left, 0);
        assert_eq!(clamped.corners.top_right, MAX_PANEL_REGION_RADIUS);
        assert_eq!(clamped.corners.bottom_right, 0);
        assert_eq!(clamped.corners.bottom_left, 7);
        assert_eq!(clamped.overscan, MAX_PANEL_REGION_OVERSCAN);
        assert!(!clamped.is_visible());
        assert_eq!(MAX_PANEL_REGIONS, 32);
    }

    #[test]
    fn panel_rect_clamps_negative_client_extents() {
        // `set_panel_rect` is client input. A degenerate (zero-height) panel is
        // valid while a surface animates, but negative extents must never reach
        // Smithay's `Size` constructor, which panics.
        let clamped = panel_rect(10, 20, -5, -64);
        assert_eq!(clamped.loc.x, 10);
        assert_eq!(clamped.loc.y, 20);
        assert_eq!(clamped.size.w, 0);
        assert_eq!(clamped.size.h, 0);

        let normal = panel_rect(-3, 4, 860, 736);
        assert_eq!(normal.loc.x, -3);
        assert_eq!(normal.loc.y, 4);
        assert_eq!(normal.size.w, 860);
        assert_eq!(normal.size.h, 736);
    }

    #[test]
    fn layer_bands_composite_wallpaper_desktop_windows_chrome() {
        // T-19.3: the desktop layer sits above the wallpaper and below every
        // window; chrome sits above windows. The enum's derived order is the
        // compositing contract the backends append elements in.
        assert_eq!(stack_band(LAYER_BACKGROUND), StackBand::Desktop);
        assert_eq!(stack_band(LAYER_TOP), StackBand::Chrome);
        assert_eq!(stack_band(LAYER_OVERLAY), StackBand::Chrome);
        assert!(StackBand::Wallpaper < StackBand::Desktop);
        assert!(StackBand::Desktop < StackBand::Windows);
        assert!(StackBand::Windows < StackBand::Chrome);
        // The desktop band is *not* above windows; chrome is.
        assert!(!above_windows(LAYER_BACKGROUND));
        assert!(above_windows(LAYER_TOP));
        assert!(above_windows(LAYER_OVERLAY));
        assert!(is_desktop(LAYER_BACKGROUND));
        assert!(!is_desktop(LAYER_TOP));
    }

    #[test]
    fn fullscreen_hides_top_chrome_but_keeps_transient_overlays() {
        // T-21.3 direct scanout: persistent menu bar/Dock (`top`) cannot sit
        // over a fullscreen client or the client is never the last element on
        // the primary plane. Transient overlays still composite (and keep the
        // material pass).
        assert!(!visible_in_fullscreen(LAYER_TOP));
        assert!(visible_in_fullscreen(LAYER_OVERLAY));
        assert!(visible_in_fullscreen(LAYER_BACKGROUND));
        assert!(visible_in_fullscreen(1));
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
