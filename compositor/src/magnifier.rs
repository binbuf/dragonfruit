// SPDX-License-Identifier: MIT
//! The compositor magnifier (T-16.6b).
//!
//! Screen magnification is compositor-owned because it transforms the whole
//! scene, not one window ([02-compositor.md](../../docs/design/02-compositor.md)):
//! it is the one accessibility feature that cannot live in a client. This
//! module is the pure policy — zoom, the scene point kept at the output
//! centre, the follow mode, and the two coordinate mappings — deliberately
//! renderer-free so it is asserted headless.
//!
//! Three pieces compose into one magnifier:
//!
//! * **The view transform.** At zoom `z` the output shows the scene rectangle
//!   `size / z` centred on [`Magnifier::center`]. Every element is drawn
//!   scaled about the output centre, so a scene point `p` lands at
//!   `view = out_centre + (p - center) * z`.
//! * **Input mapping.** The inverse map turns the pointer's physical position
//!   back into the scene point under it, so hit-testing and the client's
//!   cursor position stay correct while magnified.
//! * **Follow modes.** `FollowFocus` re-centres on the focused window and
//!   `FollowCaret` re-centres on the focused window's text caret; this
//!   compositor has no caret-position source yet, so `FollowCaret` re-centres
//!   on the focused window (the honest Linux adaptation, documented in the
//!   ADR). Both clamp the centre so the visible window never leaves the
//!   output.
//!
//! The magnifier is a whole-output transform, not a lens: it is the macOS
//! "full screen" zoom, driven by the accessibility preferences, not a
//! magnifying-glass overlay.

use smithay::utils::{Logical, Point, Rectangle};

/// The smallest zoom the magnifier engages at (`1.0` is off).
pub const MIN_ZOOM: f64 = 1.0;
/// The largest zoom the magnifier accepts.
pub const MAX_ZOOM: f64 = 8.0;
/// The default zoom when the magnifier is first switched on.
pub const DEFAULT_ZOOM: f64 = 2.0;

/// What the magnifier pans to keep in view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MagnifierMode {
    /// Keep the focused window in view.
    #[default]
    FollowFocus,
    /// Keep the focused window's text caret in view. Falls back to the
    /// focused window until a caret-position source exists.
    FollowCaret,
}

impl MagnifierMode {
    /// Parse the settings/protocol spelling (`follow-focus` | `follow-caret`).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "follow-focus" | "focus" => Some(MagnifierMode::FollowFocus),
            "follow-caret" | "caret" => Some(MagnifierMode::FollowCaret),
            _ => None,
        }
    }

    /// The canonical spelling.
    pub const fn name(self) -> &'static str {
        match self {
            MagnifierMode::FollowFocus => "follow-focus",
            MagnifierMode::FollowCaret => "follow-caret",
        }
    }
}

/// The physical-space transform the render layer applies to every element of
/// an output: scale about `origin`, then translate by `translation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewTransform {
    /// The uniform zoom factor (`> 1.0`).
    pub scale: f64,
    /// The fixed point of the scale, in output-local physical pixels (the
    /// output centre).
    pub origin: Point<i32, smithay::utils::Physical>,
    /// The post-scale translation, in output-local physical pixels.
    pub translation: Point<i32, smithay::utils::Physical>,
}

impl ViewTransform {
    /// The physical result of drawing a physical output-local point.
    #[allow(dead_code)] // used by the magnifier unit tests.
    pub fn map_point(
        &self,
        point: Point<f64, smithay::utils::Physical>,
    ) -> Point<f64, smithay::utils::Physical> {
        let origin = self.origin.to_f64();
        let x = origin.x + (point.x - origin.x) * self.scale + f64::from(self.translation.x);
        let y = origin.y + (point.y - origin.y) * self.scale + f64::from(self.translation.y);
        (x, y).into()
    }

    /// Whether this is the identity (no change) transform.
    pub fn is_identity(&self) -> bool {
        (self.scale - 1.0).abs() < f64::EPSILON
            && self.translation.x == 0
            && self.translation.y == 0
    }
}

/// The single magnifier policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Magnifier {
    enabled: bool,
    zoom: f64,
    mode: MagnifierMode,
    /// The scene point kept at the output centre (global logical).
    center: Point<f64, Logical>,
    /// Cleared once a real centre has been seeded from an output/focus; until
    /// then the first output geometry becomes the centre.
    seeded: bool,
}

impl Default for Magnifier {
    fn default() -> Self {
        Magnifier {
            enabled: false,
            zoom: DEFAULT_ZOOM,
            mode: MagnifierMode::default(),
            center: (0.0, 0.0).into(),
            seeded: false,
        }
    }
}

impl Magnifier {
    pub fn new() -> Self {
        Magnifier::default()
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    pub fn mode(&self) -> MagnifierMode {
        self.mode
    }

    pub fn center(&self) -> Point<f64, Logical> {
        self.center
    }

    /// Switch the magnifier on or off. Enabling without a seeded centre leaves
    /// the centre to the first [`Self::follow`]/[`Self::seed_center`].
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set the zoom, clamped to `MIN_ZOOM..=MAX_ZOOM`. Selecting `MIN_ZOOM`
    /// disables the magnifier (there is nothing to magnify).
    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        if self.zoom <= MIN_ZOOM {
            self.enabled = false;
        }
    }

    pub fn set_mode(&mut self, mode: MagnifierMode) {
        self.mode = mode;
    }

    /// Whether the magnifier actually transforms the scene.
    pub fn is_active(&self) -> bool {
        self.enabled && self.zoom > MIN_ZOOM
    }

    /// Seed the centre from an output geometry (its centre) the first time an
    /// enabled magnifier needs one.
    pub fn seed_center(&mut self, geometry: Rectangle<i32, Logical>) {
        if !self.seeded {
            self.center = output_center(geometry);
            self.seeded = true;
        }
    }

    /// Set an explicit scene centre, clamped so the visible window stays
    /// inside `geometry`.
    pub fn set_center(&mut self, center: Point<f64, Logical>, geometry: Rectangle<i32, Logical>) {
        self.center = clamp_center(center, geometry, self.zoom);
        self.seeded = true;
    }

    /// Re-centre on a focus rectangle (global logical) through the current
    /// mode. `caret` is the focused window's caret rectangle when the client
    /// published one; `FollowCaret` uses it, and falls back to `focus` when it
    /// is absent (this compositor has no caret source yet).
    pub fn follow(
        &mut self,
        focus: Rectangle<i32, Logical>,
        caret: Option<Rectangle<i32, Logical>>,
        geometry: Rectangle<i32, Logical>,
    ) {
        if !self.is_active() {
            return;
        }
        let anchor = match (self.mode, caret) {
            (MagnifierMode::FollowCaret, Some(caret)) => caret,
            _ => focus,
        };
        let center = (
            f64::from(anchor.loc.x) + f64::from(anchor.size.w) / 2.0,
            f64::from(anchor.loc.y) + f64::from(anchor.size.h) / 2.0,
        )
            .into();
        self.center = clamp_center(center, geometry, self.zoom);
        self.seeded = true;
    }

    /// The scene point under a global-logical view point.
    pub fn map_from_view(
        &self,
        view: Point<f64, Logical>,
        geometry: Rectangle<i32, Logical>,
    ) -> Point<f64, Logical> {
        if !self.is_active() {
            return view;
        }
        let out_center = output_center(geometry);
        (
            self.center.x + (view.x - out_center.x) / self.zoom,
            self.center.y + (view.y - out_center.y) / self.zoom,
        )
            .into()
    }

    /// Where a global-logical scene point is drawn (global-logical view).
    ///
    /// Used by the DRM cursor rail once that rail magnifies its scene; the
    /// nested backend's scene is magnified by the element pass and draws no
    /// cursor of its own.
    #[allow(dead_code)]
    pub fn map_to_view(
        &self,
        scene: Point<f64, Logical>,
        geometry: Rectangle<i32, Logical>,
    ) -> Point<f64, Logical> {
        if !self.is_active() {
            return scene;
        }
        let out_center = output_center(geometry);
        (
            out_center.x + (scene.x - self.center.x) * self.zoom,
            out_center.y + (scene.y - self.center.y) * self.zoom,
        )
            .into()
    }

    /// The physical element transform for an output, or `None` when the
    /// magnifier is inactive (draw elements untransformed).
    pub fn view_transform(
        &self,
        geometry: Rectangle<i32, Logical>,
        output_scale: f64,
    ) -> Option<ViewTransform> {
        if !self.is_active() {
            return None;
        }
        let origin_logical = output_center(geometry);
        let origin = Point::<i32, smithay::utils::Physical>::from((
            (origin_logical.x * output_scale).round() as i32,
            (origin_logical.y * output_scale).round() as i32,
        ));
        // The fixed point is the output centre; translate so that `center`
        // lands exactly on it.
        let center_phys = (
            (self.center.x - f64::from(geometry.loc.x)) * output_scale,
            (self.center.y - f64::from(geometry.loc.y)) * output_scale,
        );
        let out_center_phys = (
            f64::from(geometry.size.w) / 2.0 * output_scale,
            f64::from(geometry.size.h) / 2.0 * output_scale,
        );
        let translation = Point::<i32, smithay::utils::Physical>::from((
            ((out_center_phys.0 - center_phys.0) * self.zoom).round() as i32,
            ((out_center_phys.1 - center_phys.1) * self.zoom).round() as i32,
        ));
        Some(ViewTransform {
            scale: self.zoom,
            origin,
            translation,
        })
    }
}

/// The global-logical centre of an output geometry.
fn output_center(geometry: Rectangle<i32, Logical>) -> Point<f64, Logical> {
    (
        f64::from(geometry.loc.x) + f64::from(geometry.size.w) / 2.0,
        f64::from(geometry.loc.y) + f64::from(geometry.size.h) / 2.0,
    )
        .into()
}

/// Clamp `center` so the visible `size/zoom` window stays inside `geometry`.
/// When the output is smaller than the window the output centre wins.
fn clamp_center(
    center: Point<f64, Logical>,
    geometry: Rectangle<i32, Logical>,
    zoom: f64,
) -> Point<f64, Logical> {
    let zoom = zoom.max(MIN_ZOOM);
    let half_w = f64::from(geometry.size.w) / (2.0 * zoom);
    let half_h = f64::from(geometry.size.h) / (2.0 * zoom);
    let min_x = f64::from(geometry.loc.x) + half_w;
    let max_x = f64::from(geometry.loc.x) + f64::from(geometry.size.w) - half_w;
    let min_y = f64::from(geometry.loc.y) + half_h;
    let max_y = f64::from(geometry.loc.y) + f64::from(geometry.size.h) - half_h;
    let x = if min_x > max_x {
        output_center(geometry).x
    } else {
        center.x.clamp(min_x, max_x)
    };
    let y = if min_y > max_y {
        output_center(geometry).y
    } else {
        center.y.clamp(min_y, max_y)
    };
    (x, y).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output() -> Rectangle<i32, Logical> {
        Rectangle::new((0, 0).into(), (1280, 720).into())
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn disabled_magnifier_is_the_identity() {
        let magnifier = Magnifier::new();
        assert!(!magnifier.is_active());
        assert!(magnifier.view_transform(output(), 1.0).is_none());
        let p = (400.0, 300.0).into();
        assert_eq!(magnifier.map_from_view(p, output()), p);
        assert_eq!(magnifier.map_to_view(p, output()), p);
    }

    #[test]
    fn zoom_maps_the_centre_to_the_output_centre() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.seed_center(output());
        // The seeded centre is the output centre, so the view transform is a
        // pure zoom about it: no translation.
        let transform = magnifier.view_transform(output(), 1.0).unwrap();
        assert_eq!(transform.scale, 2.0);
        assert_eq!(transform.translation, (0, 0).into());
        // The scene centre maps exactly onto the physical output centre.
        assert_eq!(
            transform.map_point((640.0, 360.0).into()),
            (640.0, 360.0).into()
        );

        // The centre is a fixed point of both logical mappings.
        let center = magnifier.center();
        assert_eq!(magnifier.map_to_view(center, output()), center);
        assert_eq!(magnifier.map_from_view(center, output()), center);
    }

    #[test]
    fn view_and_scene_mappings_are_inverses() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.set_zoom(3.0);
        magnifier.set_center((500.0, 300.0).into(), output());
        for point in [(0.0, 0.0), (640.0, 360.0), (1279.0, 719.0), (200.0, 90.0)] {
            let scene = magnifier.map_from_view(point.into(), output());
            let back = magnifier.map_to_view(scene, output());
            assert!((back.x - point.0).abs() < 1e-6 && (back.y - point.1).abs() < 1e-6);
        }
    }

    #[test]
    fn zoom_about_the_centre_keeps_the_centre_fixed_while_edges_expand() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.set_zoom(2.0);
        magnifier.set_center((640.0, 360.0).into(), output());
        // The output centre stays put.
        assert_eq!(
            magnifier.map_to_view((640.0, 360.0).into(), output()),
            (640.0, 360.0).into()
        );
        // A point 100 right of centre moves to 200 right (zoom 2).
        assert_eq!(
            magnifier.map_to_view((740.0, 360.0).into(), output()),
            (840.0, 360.0).into()
        );
    }

    #[test]
    fn the_centre_clamps_so_the_view_never_leaves_the_output() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.set_zoom(4.0);
        // Ask to centre far outside the output; the visible 320x180 window is
        // clamped to the output edge.
        magnifier.set_center((5000.0, -5000.0).into(), output());
        let center = magnifier.center();
        assert_eq!(center, (1280.0 - 160.0, 90.0).into());
        // The scene centre maps onto the physical output centre.
        let transform = magnifier.view_transform(output(), 1.0).unwrap();
        assert_eq!(
            transform.map_point((center.x, center.y).into()),
            (640.0, 360.0).into()
        );
    }

    #[test]
    fn a_window_larger_than_the_view_centres_the_output() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.set_zoom(1.0);
        assert!(!magnifier.is_active(), "min zoom disables");
        magnifier.set_zoom(8.0);
        magnifier.set_center((100.0, 100.0).into(), output());
        // 8x on a 1280x720 output shows 160x90; the clamp keeps it inside.
        assert_eq!(magnifier.center(), (100.0, 100.0).into());
    }

    #[test]
    fn follow_recentres_on_the_focus_rect() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.set_zoom(2.0);
        let focus = rect(900, 500, 200, 150);
        magnifier.follow(focus, None, output());
        // The focus centre is (1000,575); at zoom 2 the visible 640x360
        // window clamps to the output edge (960,540).
        assert_eq!(magnifier.center(), (960.0, 540.0).into());
    }

    #[test]
    fn follow_caret_prefers_the_caret_and_falls_back_to_the_window() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.set_zoom(2.0);
        magnifier.set_mode(MagnifierMode::FollowCaret);
        let focus = rect(200, 200, 400, 300);
        let caret = rect(500, 260, 2, 20);
        magnifier.follow(focus, Some(caret), output());
        assert_eq!(magnifier.center(), (501.0, 270.0).into());
        // No caret published: fall back to the window.
        magnifier.follow(focus, None, output());
        assert_eq!(magnifier.center(), (400.0, 350.0).into());
    }

    #[test]
    fn follow_is_a_no_op_when_the_magnifier_is_off() {
        let mut magnifier = Magnifier::new();
        magnifier.follow(rect(900, 500, 200, 150), None, output());
        assert_eq!(magnifier.center(), (0.0, 0.0).into());
    }

    #[test]
    fn the_physical_transform_places_the_centre_at_the_output_centre() {
        let mut magnifier = Magnifier::new();
        magnifier.set_enabled(true);
        magnifier.set_zoom(2.0);
        magnifier.set_center((400.0, 300.0).into(), output());
        let transform = magnifier.view_transform(output(), 2.0).unwrap();
        assert_eq!(transform.scale, 2.0);
        // The scene centre (400,300) is at physical (800,600); after the
        // transform it must land on the physical output centre (1280,720).
        let mapped = transform.map_point((800.0, 600.0).into());
        assert!((mapped.x - 1280.0).abs() < 1e-6 && (mapped.y - 720.0).abs() < 1e-6);
    }
}
