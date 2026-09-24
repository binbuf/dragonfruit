// SPDX-License-Identifier: MIT
//! Token-driven backdrop blur pass for chrome surfaces (T-04.2).
//!
//! The compositor draws the material *under* a translucent chrome surface (the
//! menu bar, Dock, and popovers/menus) so the surface reads as frosted glass
//! instead of a flat rectangle. The flat GLES2 renderer has no texture
//! sampler in the element pipeline yet, so the blurred backdrop is expressed
//! the same way the elevation shadow is ([`shadow`](crate::window::shadow)):
//! a **token-derived stack of translucent rounded layers** that feather the
//! panel from its rounded edge inward. The panel is clipped with the same
//! [`CornerMask`](crate::window::corner::CornerMask) geometry as the titlebar
//! and shadow, so all three materials round from one source.
//!
//! The blur radii and panel opacities are read **only** from the generated
//! design tokens ([`design_tokens`](crate::design_tokens)):
//! `material.chromeBlur`/`chromeOpacity` for persistent chrome and
//! `material.popupBlur`/`popupOpacity` for popovers/overlays, per scheme. That
//! keeps a compositor-drawn material and the QML `Theme.material` group (which
//! `TitleBar.qml`/`Dock.qml` already consume) from drifting (FR-2).
//!
//! ## One effect pass per frame
//!
//! [`BackdropPass`] is the pass bookkeeping: the renderer opens a frame once,
//! then each output applies the backdrop exactly once. A second request for
//! the same `(frame, output)` is counted as `skipped` and draws nothing, so
//! the scene can never be blurred twice in one frame — the T-04.2 acceptance
//! and the invariant T-04.3/T-04.4 keep.
//!
//! A client's translucent regions are honored by construction: the backdrop is
//! a separate element drawn *below* the chrome's Wayland surface, so the
//! surface's own alpha blends over it instead of being punched out.

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Color32F;
use smithay::utils::{Logical, Rectangle, Scale, Size};

use crate::design_tokens::{component, semantic};
use crate::window::corner::{rounded_rect_spans, RoundedCorners};
use crate::window::decoration::{color_from_rgba, ColorScheme};
use crate::window::pass::FramePass;

/// The layer a chrome surface requested at which it stops being persistent
/// chrome and becomes transient overlay chrome (menus/popovers/OSD).
/// Mirrors `df_shell.layer` 3 / [`crate::shell::layer::LAYER_OVERLAY`].
const LAYER_OVERLAY: u32 = 3;

/// Logical pixels of blur per stacked layer: the number of feather layers is
/// derived from the token blur so a larger token blur is a softer panel. The
/// count is bounded so a hostile/oversized token cannot emit unbounded
/// elements.
const BLUR_PER_LAYER: f32 = 6.0;
/// Upper bound on the derived feather layer count.
const MAX_LAYERS: u32 = 8;

/// Which material role a chrome surface plays, selecting the token group the
/// backdrop consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialRole {
    /// Persistent chrome: the menu bar and the Dock (`material.chrome*`).
    #[default]
    Chrome,
    /// Transient overlay chrome: menus, popovers, and (T-11) the OSD
    /// (`material.popup*`).
    Popup,
}

impl MaterialRole {
    /// The role for a `df_shell.layer` value: `overlay` (3) and above is a
    /// popover, everything else is persistent chrome.
    pub const fn from_layer(layer: u32) -> Self {
        if layer >= LAYER_OVERLAY {
            MaterialRole::Popup
        } else {
            MaterialRole::Chrome
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            MaterialRole::Chrome => "chrome",
            MaterialRole::Popup => "popup",
        }
    }

    /// The backdrop geometry and tone for this role under `scheme`, resolved
    /// from the generated `material` tokens.
    pub fn spec(self, scheme: ColorScheme) -> BackdropSpec {
        let (blur, opacity) = match self {
            MaterialRole::Chrome => match scheme {
                ColorScheme::Light => (
                    semantic::light::material::CHROME_BLUR,
                    semantic::light::material::CHROME_OPACITY,
                ),
                ColorScheme::Dark => (
                    semantic::dark::material::CHROME_BLUR,
                    semantic::dark::material::CHROME_OPACITY,
                ),
            },
            MaterialRole::Popup => match scheme {
                ColorScheme::Light => (
                    semantic::light::material::POPUP_BLUR,
                    semantic::light::material::POPUP_OPACITY,
                ),
                ColorScheme::Dark => (
                    semantic::dark::material::POPUP_BLUR,
                    semantic::dark::material::POPUP_OPACITY,
                ),
            },
        };
        // The panel tone stands in for the scene sampled under the surface:
        // persistent chrome uses the chrome color, popovers the elevated
        // surface. The client surface's own alpha tints it further.
        let color = match self {
            MaterialRole::Chrome => scheme.chrome(),
            MaterialRole::Popup => scheme.surface_elevated(),
        };
        // The radius follows the component that draws that role, so the
        // backdrop corner matches the QML panel it sits behind. Both are
        // component tokens, never a hardcoded literal.
        let radius = match self {
            MaterialRole::Chrome => component::menu_bar::RADIUS,
            MaterialRole::Popup => component::popup::RADIUS,
        };
        BackdropSpec {
            blur,
            opacity,
            layers: ((blur / BLUR_PER_LAYER).round() as u32).clamp(1, MAX_LAYERS),
            radius,
            color,
        }
    }
}

/// A resolved backdrop: the token blur/opacity plus the derived feather layer
/// count, panel corner radius, and tone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackdropSpec {
    /// The softest feather spread in logical pixels
    /// (`material.{chrome,popup}Blur`).
    pub blur: f32,
    /// The panel opacity the feather layers accumulate to
    /// (`material.{chrome,popup}Opacity`).
    pub opacity: f32,
    /// Number of feather layers, derived from [`Self::blur`].
    pub layers: u32,
    /// The panel corner radius (`component.<role>.radius`).
    pub radius: f32,
    /// The panel tone with its token alpha.
    pub color: [u8; 4],
}

/// One feather layer of a backdrop panel. `rect` is in the same logical space
/// as the chrome surface; `radius` is its rounded corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackdropLayer {
    pub rect: Rectangle<i32, Logical>,
    pub radius: f32,
    pub color: Color32F,
}

/// The feather layers of `spec` for a chrome surface occupying `rect`, in the
/// **same logical space**. Returned back-to-front: index `0` is the full panel
/// and the last layer is the most inset. Every layer carries
/// `opacity / layers` of the panel color, so the panel edge is a single faint
/// layer and the center accumulates to the token opacity — a soft frosted
/// edge that never extends past the chrome surface.
pub fn backdrop_layers(rect: Rectangle<i32, Logical>, spec: BackdropSpec) -> Vec<BackdropLayer> {
    if rect.size.w <= 0 || rect.size.h <= 0 {
        return Vec::new();
    }
    let layers = spec.layers.max(1);
    let per_layer = spec.opacity / layers as f32;
    let base = color_from_rgba(spec.color);
    let mut result = Vec::with_capacity(layers as usize);
    for index in 0..layers {
        // index 0 → the whole panel; the last → inset by (nearly) the full
        // token blur, clamped so the panel never collapses.
        let inset = (f64::from(spec.blur) * f64::from(index) / f64::from(layers)).round() as i32;
        let w = (rect.size.w - 2 * inset).max(1);
        let h = (rect.size.h - 2 * inset).max(1);
        let max_inset_x = (rect.size.w - 1) / 2;
        let max_inset_y = (rect.size.h - 1) / 2;
        let inset_x = inset.min(max_inset_x.max(0));
        let inset_y = inset.min(max_inset_y.max(0));
        result.push(BackdropLayer {
            rect: Rectangle::new(
                (rect.loc.x + inset_x, rect.loc.y + inset_y).into(),
                (w, h).into(),
            ),
            radius: (spec.radius - inset as f32).max(0.0),
            color: base * per_layer,
        });
    }
    result
}

/// Whether the backdrop material applies to a chrome surface occupying
/// `geometry` on an output of `output_size` (both output-local logical).
///
/// A surface that fills the whole output is a **scene-wide overlay** — Mission
/// Control, Desktop Reveal — not a translucent panel: it composites the live
/// scene and draws its own scrim, so the compositor must not frost the entire
/// output for it. T-04.2's backdrop is for the menu bar, the Dock, and
/// popovers, which are all smaller than the output.
pub fn is_backdrop_panel(
    geometry: Rectangle<i32, Logical>,
    output_size: Size<i32, Logical>,
) -> bool {
    geometry.size != output_size
}

/// Resolve the rect a chrome surface actually paints — the panel the backdrop
/// material belongs behind.
///
/// `geometry` is the full output-local layer-surface rect. `reserved` is the
/// visible strip the surface reserves on its anchored edge (the menu bar / Dock
/// bar), and `region` is the bounding box of the part the client declares
/// interactive, in surface-local coordinates. The Dock sets its input region to
/// the visible bar, so the transparent magnification band is excluded; placing
/// `region` and intersecting it with `reserved` also stops a magnified or
/// bouncing icon that pokes into the band from growing the panel. With neither
/// signal the whole geometry is the panel (popovers, overlays).
pub fn panel_bounds(
    geometry: Rectangle<i32, Logical>,
    reserved: Option<Rectangle<i32, Logical>>,
    region: Option<Rectangle<i32, Logical>>,
) -> Rectangle<i32, Logical> {
    let region = region
        .filter(|rect| rect.size.w > 0 && rect.size.h > 0)
        .and_then(|rect| Rectangle::new(geometry.loc + rect.loc, rect.size).intersection(geometry));
    let panel = match (region, reserved) {
        (Some(region), Some(reserved)) => region.intersection(reserved),
        (Some(region), None) => Some(region),
        (None, reserved) => reserved,
    };
    panel.unwrap_or(geometry)
}

/// The bounding rectangle of a backdrop: the full chrome band (layer 0).
pub fn backdrop_bounds(
    rect: Rectangle<i32, Logical>,
    spec: BackdropSpec,
) -> Rectangle<i32, Logical> {
    backdrop_layers(rect, spec)
        .first()
        .map(|layer| layer.rect)
        .unwrap_or(rect)
}

/// Convert the backdrop layers into solid render elements for one chrome
/// surface, in output-local physical coordinates and front-to-back order
/// (Smithay's damage tracker consumes front-to-back).
///
/// Each layer is decomposed into the token-derived rounded-corner spans
/// (T-04.1b) so the panel edge follows the component radius.
pub fn backdrop_elements(
    rect: Rectangle<i32, Logical>,
    spec: BackdropSpec,
    scale: Scale<f64>,
) -> Vec<SolidColorRenderElement> {
    let mut elements = Vec::new();
    for layer in backdrop_layers(rect, spec) {
        let radius = layer.radius.round() as i32;
        for span in rounded_rect_spans(layer.rect, radius, RoundedCorners::All) {
            elements.push(SolidColorRenderElement::new(
                Id::new(),
                span.to_physical_precise_round(scale),
                CommitCounter::default(),
                layer.color,
                Kind::Unspecified,
            ));
        }
    }
    // `backdrop_layers` is back-to-front; Smithay consumes front-to-back.
    elements.reverse();
    elements
}

/// Per-frame bookkeeping for the backdrop pass (T-04.2 acceptance: one effect
/// pass per frame, no double-blur).
///
/// The renderer calls [`BackdropPass::begin_frame`] once per rendered frame,
/// then [`BackdropPass::apply`] once per output. A repeated request for the
/// same output in the same frame is counted as `skipped` and returns `None`,
/// so the pass can never draw twice.
#[derive(Debug, Default)]
pub struct BackdropPass(FramePass);

impl BackdropPass {
    pub fn new() -> Self {
        BackdropPass(FramePass::new())
    }

    /// Open a frame. Clears the per-output application set and the damage
    /// recorded for it. `frame` is the renderer's monotonic render serial.
    pub fn begin_frame(&mut self, frame: u64) {
        self.0.begin_frame(frame);
    }

    /// Apply the backdrop for `output` this frame. `region` is the union of
    /// the chrome bands on that output, in output-local logical coordinates.
    /// Returns the region when it applies now, or `None` when the backdrop
    /// already ran for this output in this frame (the double-blur guard).
    pub fn apply(
        &mut self,
        output: &str,
        region: Rectangle<i32, Logical>,
    ) -> Option<Rectangle<i32, Logical>> {
        self.0.apply(output, region)
    }

    /// The frame serial this pass is open for.
    pub fn frame(&self) -> u64 {
        self.0.frame()
    }

    /// Number of outputs the pass actually applied the backdrop for (one per
    /// output per frame; the effect-pass count).
    pub fn applications(&self) -> u64 {
        self.0.applications()
    }

    /// Number of requests skipped because the output already applied the
    /// backdrop this frame (the double-blur counter — must stay `0` in a
    /// correct render loop).
    pub fn skipped(&self) -> u64 {
        self.0.skipped()
    }

    /// The damage of the current frame's applications (output-local logical).
    pub fn damage(&self) -> &[Rectangle<i32, Logical>] {
        self.0.damage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::renderer::element::Element;
    use smithay::utils::{Physical, Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    /// The bounding rectangle of an element list at `scale`.
    fn merged_bounds(
        elements: &[SolidColorRenderElement],
        scale: Scale<f64>,
    ) -> Rectangle<i32, Physical> {
        elements
            .iter()
            .map(|element| element.geometry(scale))
            .reduce(|a, b| a.merge(b))
            .expect("at least one element")
    }

    #[test]
    fn roles_read_their_material_tokens_per_scheme() {
        let chrome_light = MaterialRole::Chrome.spec(ColorScheme::Light);
        assert_eq!(chrome_light.blur, semantic::light::material::CHROME_BLUR);
        assert_eq!(chrome_light.blur, 24.0);
        assert_eq!(
            chrome_light.opacity,
            semantic::light::material::CHROME_OPACITY
        );
        assert_eq!(chrome_light.opacity, 0.82);
        assert_eq!(chrome_light.radius, component::menu_bar::RADIUS);
        assert_eq!(chrome_light.color, ColorScheme::Light.chrome());

        let chrome_dark = MaterialRole::Chrome.spec(ColorScheme::Dark);
        assert_eq!(chrome_dark.blur, semantic::dark::material::CHROME_BLUR);
        assert_eq!(chrome_dark.blur, 28.0);
        assert_eq!(chrome_dark.opacity, 0.72);

        let popup_dark = MaterialRole::Popup.spec(ColorScheme::Dark);
        assert_eq!(popup_dark.blur, semantic::dark::material::POPUP_BLUR);
        assert_eq!(popup_dark.blur, 32.0);
        assert_eq!(popup_dark.opacity, semantic::dark::material::POPUP_OPACITY);
        assert_eq!(popup_dark.radius, component::popup::RADIUS);
        assert_eq!(popup_dark.color, ColorScheme::Dark.surface_elevated());

        // The feather layer count follows the blur token.
        assert_eq!(MaterialRole::Chrome.spec(ColorScheme::Light).layers, 4);
        assert_eq!(MaterialRole::Popup.spec(ColorScheme::Dark).layers, 5);
        assert_eq!(MaterialRole::default(), MaterialRole::Chrome);
    }

    #[test]
    fn overlay_layers_are_popups_and_persistent_layers_are_chrome() {
        assert_eq!(MaterialRole::from_layer(0), MaterialRole::Chrome);
        assert_eq!(MaterialRole::from_layer(2), MaterialRole::Chrome);
        assert_eq!(MaterialRole::from_layer(3), MaterialRole::Popup);
        assert_eq!(MaterialRole::from_layer(4), MaterialRole::Popup);
        assert_eq!(MaterialRole::Chrome.name(), "chrome");
        assert_eq!(MaterialRole::Popup.name(), "popup");
    }

    #[test]
    fn only_sub_output_panels_get_the_backdrop() {
        let output = Size::from((1920, 1200));
        // The menu bar and the Dock are panels: they get the material.
        assert!(is_backdrop_panel(rect(0, 0, 1920, 28), output));
        assert!(is_backdrop_panel(rect(0, 1069, 1920, 131), output));
        // A popover is a panel even when it is large.
        assert!(is_backdrop_panel(rect(700, 100, 520, 900), output));
        // A full-output overlay (Mission Control, Desktop Reveal) is a
        // scene-wide surface that draws its own scrim, never frosted whole.
        assert!(!is_backdrop_panel(rect(0, 0, 1920, 1200), output));
    }

    #[test]
    fn panel_bounds_lands_on_the_visible_bar_not_the_whole_surface() {
        // A bottom Dock: full surface 1920x131 at the output bottom; the
        // reserved bar is the bottom 67px; the input region (surface-local)
        // is the centred 400px bar slab at y=64.
        let dock = rect(0, 1069, 1920, 131);
        let bar = rect(0, 1133, 1920, 67);
        let region = rect(760, 64, 400, 67);
        assert_eq!(
            panel_bounds(dock, Some(bar), Some(region)),
            rect(760, 1133, 400, 67)
        );

        // A magnified icon pokes into the transparent band (surface-local
        // y=10..131); the panel must still stop at the reserved bar.
        let magnified = rect(700, 10, 520, 121);
        assert_eq!(
            panel_bounds(dock, Some(bar), Some(magnified)),
            rect(700, 1133, 520, 67)
        );

        // Without an input region the reserved bar is still the panel.
        assert_eq!(panel_bounds(dock, Some(bar), None), bar);

        // A menu bar reserves its whole surface; a popover reserves nothing
        // and is fully painted, so both keep their full geometry.
        let menubar = rect(0, 0, 1920, 28);
        assert_eq!(panel_bounds(menubar, Some(menubar), None), menubar);
        let popover = rect(300, 100, 320, 200);
        assert_eq!(panel_bounds(popover, None, None), popover);

        // A region wholly outside the surface is ignored rather than inventing
        // area.
        assert_eq!(
            panel_bounds(dock, Some(bar), Some(rect(5000, 5000, 10, 10))),
            bar
        );
    }

    #[test]
    fn layers_are_increasingly_inset_and_never_leave_the_panel() {
        let panel = rect(100, 20, 800, 24);
        let spec = MaterialRole::Chrome.spec(ColorScheme::Dark);
        let layers = backdrop_layers(panel, spec);
        assert_eq!(layers.len(), spec.layers as usize);

        // The first layer is the whole panel; each later layer is inset by a
        // non-decreasing amount, and every layer stays inside the panel with a
        // positive size.
        assert_eq!(layers[0].rect, panel);
        for pair in layers.windows(2) {
            assert!(pair[0].rect.contains_rect(pair[1].rect));
        }
        for layer in &layers {
            assert!(panel.contains_rect(layer.rect));
            assert!(layer.rect.size.w >= 1 && layer.rect.size.h >= 1);
            assert!(layer.radius >= 0.0 && layer.radius <= spec.radius);
        }

        // Each layer carries `opacity / layers`; the center accumulates to the
        // token opacity.
        let per = color_from_rgba(spec.color) * (spec.opacity / spec.layers as f32);
        assert_eq!(layers[0].color, per);
        assert_eq!(layers.last().unwrap().color, per);

        // The union of the feather layers is exactly the chrome band.
        assert_eq!(backdrop_bounds(panel, spec), panel);
    }

    #[test]
    fn elements_are_rounded_and_cover_the_chrome_band() {
        let panel = rect(100, 20, 800, 24);
        let spec = MaterialRole::Chrome.spec(ColorScheme::Light);
        let elements = backdrop_elements(panel, spec, 1.0.into());
        assert!(!elements.is_empty());

        // The union of the rounded decomposition is the whole panel.
        let union = merged_bounds(&elements, 1.0.into());
        assert_eq!(union, panel.to_physical_precise_round(1.0));

        // The corners are clipped: the outermost panel pixel is outside every
        // span (the mask is a subset of the bounding box).
        let corner = Point::from((panel.loc.x, panel.loc.y));
        assert!(!elements
            .iter()
            .any(|element| element.geometry(1.0.into()).contains(corner)));
    }

    #[test]
    fn elements_are_scaled_and_never_extend_outside_the_panel() {
        let panel = rect(10, 200, 400, 60);
        let spec = MaterialRole::Popup.spec(ColorScheme::Dark);
        let at_one = backdrop_elements(panel, spec, 1.0.into());
        let bound = merged_bounds(&at_one, 1.0.into());

        let at_two = backdrop_elements(panel, spec, 2.0.into());
        let doubled = merged_bounds(&at_two, 2.0.into());
        assert_eq!(doubled.size, bound.size * 2);

        // The panel is a material *under* the chrome surface: no feather layer
        // escapes the surface's physical bounds at either scale.
        let panel_phys = panel.to_physical_precise_round(2.0);
        for element in &at_two {
            assert!(
                panel_phys.contains_rect(element.geometry(2.0.into())),
                "a backdrop element escaped the panel"
            );
        }
    }

    #[test]
    fn a_degenerate_panel_produces_no_elements() {
        let spec = MaterialRole::Chrome.spec(ColorScheme::Dark);
        assert!(backdrop_elements(rect(0, 0, 0, 10), spec, 1.0.into()).is_empty());
        assert!(backdrop_elements(rect(0, 0, 10, 0), spec, 1.0.into()).is_empty());
    }

    #[test]
    fn pass_applies_once_per_output_and_frame() {
        let band = rect(0, 0, 1920, 24);
        let mut pass = BackdropPass::new();
        pass.begin_frame(1);

        assert_eq!(pass.apply("NESTED-1", band), Some(band));
        assert_eq!(pass.applications(), 1);
        assert_eq!(pass.skipped(), 0);
        assert_eq!(pass.damage(), &[band]);

        // A second request for the same output in the same frame is skipped:
        // no double-blur.
        assert_eq!(pass.apply("NESTED-1", band), None);
        assert_eq!(pass.applications(), 1);
        assert_eq!(pass.skipped(), 1);
        assert_eq!(pass.damage().len(), 1);

        // A different output applies independently.
        assert_eq!(pass.apply("HDMI-A-1", band), Some(band));
        assert_eq!(pass.applications(), 2);

        // A new frame re-opens the pass for the first output.
        pass.begin_frame(2);
        assert_eq!(pass.frame(), 2);
        assert!(pass.damage().is_empty());
        assert_eq!(pass.apply("NESTED-1", band), Some(band));
        assert_eq!(pass.applications(), 3);
        assert_eq!(pass.skipped(), 1);
    }

    #[test]
    fn a_zero_area_band_is_applied_but_records_no_damage() {
        let mut pass = BackdropPass::new();
        pass.begin_frame(7);
        assert_eq!(
            pass.apply("NESTED-1", rect(0, 0, 0, 0)),
            Some(rect(0, 0, 0, 0))
        );
        assert!(pass.damage().is_empty());
        assert_eq!(pass.applications(), 1);
    }
}
