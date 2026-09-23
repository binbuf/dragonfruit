// SPDX-License-Identifier: MIT
//! Server-side-decoration titlebar element (T-01.1).
//!
//! This is the compositor's SSD titlebar: pure geometry and drawing state,
//! sized from the generated design-system tokens so a compositor-drawn
//! titlebar cannot drift from the first-party QML `TitleBar`/`TrafficLights`
//! (FR-3). It is deliberately a *model plus a solid-fill renderer*:
//!
//! * geometry/insets are deterministic and unit-testable without a GPU;
//! * the fill is a flat token color (real materials — blur, shadow, rounded
//!   corners — are T-04 and replace [`TitlebarElement::render_elements`]);
//! * hover and disabled *drawing* exist now; interaction (which would set
//!   `hovered`) is T-01.2.
//!
//! The client's own input regions are untouched: nothing here routes input.

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Color32F;
use smithay::utils::{Logical, Point, Rectangle, Scale};

use crate::design_tokens::{
    component::{titlebar, traffic_lights},
    semantic,
};
use crate::window::{DecorationTier, WindowId, WindowState};

/// The logical height of the SSD titlebar, from the design tokens.
pub const TITLEBAR_HEIGHT: i32 = titlebar::HEIGHT as i32;

/// The design-system color scheme the titlebar draws from. The compositor's
/// live scheme follows desktop settings (T-08); until then the default is
/// dark, which reads correctly over arbitrary application pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorScheme {
    Light,
    #[default]
    Dark,
}

impl ColorScheme {
    /// Titlebar background (flat fill until the T-04 material pass).
    pub const fn chrome(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::CHROME,
            ColorScheme::Dark => semantic::dark::color::CHROME,
        }
    }

    /// The disabled/unfocused traffic-light fill.
    pub const fn control_active(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::CONTROL_ACTIVE,
            ColorScheme::Dark => semantic::dark::color::CONTROL_ACTIVE,
        }
    }

    /// Glyph ink.
    pub const fn traffic_glyph(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::TRAFFIC_GLYPH,
            ColorScheme::Dark => semantic::dark::color::TRAFFIC_GLYPH,
        }
    }

    /// The lit fill of a traffic light.
    pub const fn light(self, kind: TrafficLightKind) -> [u8; 4] {
        match (self, kind) {
            (ColorScheme::Light, TrafficLightKind::Close) => semantic::light::color::CLOSE,
            (ColorScheme::Light, TrafficLightKind::Minimize) => semantic::light::color::MINIMIZE,
            (ColorScheme::Light, TrafficLightKind::Zoom) => semantic::light::color::ZOOM,
            (ColorScheme::Dark, TrafficLightKind::Close) => semantic::dark::color::CLOSE,
            (ColorScheme::Dark, TrafficLightKind::Minimize) => semantic::dark::color::MINIMIZE,
            (ColorScheme::Dark, TrafficLightKind::Zoom) => semantic::dark::color::ZOOM,
        }
    }
}

/// Inset the titlebar reserves around a window's client area.
///
/// SSD reserves the top strip for the titlebar; the other edges are zero in
/// this slice (borders are T-04). CSD windows reserve nothing — the client
/// draws its own chrome, so the compositor must not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowInsets {
    pub top: i32,
    pub left: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WindowInsets {
    pub const NONE: WindowInsets = WindowInsets {
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
    };

    /// The insets a window with this decoration tier reserves.
    pub const fn for_tier(tier: DecorationTier) -> Self {
        match tier {
            DecorationTier::ServerSide => WindowInsets {
                top: TITLEBAR_HEIGHT,
                ..WindowInsets::NONE
            },
            DecorationTier::ClientSide => WindowInsets::NONE,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.top == 0 && self.left == 0 && self.right == 0 && self.bottom == 0
    }

    /// Shrink and offset `rect` by these insets (the client area inside the
    /// decorated window).
    pub fn inset(self, rect: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        let w = (rect.size.w - self.left - self.right).max(0);
        let h = (rect.size.h - self.top - self.bottom).max(0);
        Rectangle::new(
            (rect.loc.x + self.left, rect.loc.y + self.top).into(),
            (w, h).into(),
        )
    }
}

/// The three left-side window controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLightKind {
    Close,
    Minimize,
    Zoom,
}

impl TrafficLightKind {
    /// Left-to-right order (close, minimize, zoom).
    pub const ALL: [TrafficLightKind; 3] = [
        TrafficLightKind::Close,
        TrafficLightKind::Minimize,
        TrafficLightKind::Zoom,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            TrafficLightKind::Close => "close",
            TrafficLightKind::Minimize => "minimize",
            TrafficLightKind::Zoom => "zoom",
        }
    }
}

/// One laid-out traffic light. `rect` is in output-local logical coordinates
/// (titlebar coordinates), so T-01.2's hit-test and the renderer share it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrafficLightButton {
    pub kind: TrafficLightKind,
    pub rect: Rectangle<i32, Logical>,
    /// Disabled controls draw the muted fill with no glyph.
    pub enabled: bool,
}

/// A fully resolved SSD titlebar: geometry, insets, controls, and the
/// transient draw state (focus/hover) for one window in one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct TitlebarElement {
    pub window: WindowId,
    /// The client/content area the titlebar decorates.
    pub content: Rectangle<i32, Logical>,
    pub insets: WindowInsets,
    /// The titlebar rect, directly above `content`.
    pub titlebar: Rectangle<i32, Logical>,
    pub focused: bool,
    pub hovered: bool,
    pub scheme: ColorScheme,
    pub buttons: Vec<TrafficLightButton>,
}

impl TitlebarElement {
    /// Build the titlebar for `window`'s content geometry, or `None` when
    /// the compositor must not draw one:
    ///
    /// * Tier 3 (client-side decoration) never gets a compositor titlebar
    ///   (the dignity rule, [05-window-decorations.md]).
    /// * Fullscreen hides the titlebar (T-01.3 adds the hover reveal).
    /// * Minimized windows are unmapped.
    pub fn for_window(
        window: WindowId,
        content: Rectangle<i32, Logical>,
        tier: DecorationTier,
        state: WindowState,
        focused: bool,
        hovered: bool,
    ) -> Option<Self> {
        if tier != DecorationTier::ServerSide {
            return None;
        }
        if !matches!(state, WindowState::Floating | WindowState::Zoomed) {
            return None;
        }
        if content.size.w <= 0 || content.size.h <= 0 {
            return None;
        }
        let insets = WindowInsets::for_tier(tier);
        let titlebar = Rectangle::new(
            (content.loc.x, content.loc.y - insets.top).into(),
            (content.size.w, insets.top).into(),
        );
        let buttons = Self::layout_buttons(titlebar);
        Some(TitlebarElement {
            window,
            content,
            insets,
            titlebar,
            focused,
            hovered,
            scheme: ColorScheme::default(),
            buttons,
        })
    }

    fn layout_buttons(titlebar: Rectangle<i32, Logical>) -> Vec<TrafficLightButton> {
        let diameter = traffic_lights::DIAMETER as i32;
        let gap = traffic_lights::GAP as i32;
        let inset = traffic_lights::INSET as i32;
        let y = titlebar.loc.y + (titlebar.size.h - diameter) / 2;
        let mut x = titlebar.loc.x + inset;
        TrafficLightKind::ALL
            .iter()
            .map(|kind| {
                let rect = Rectangle::new((x, y).into(), (diameter, diameter).into());
                x += diameter + gap;
                TrafficLightButton {
                    kind: *kind,
                    rect,
                    enabled: true,
                }
            })
            .collect()
    }

    /// The traffic light under `point`, if any (hit-testing is T-01.2).
    pub fn button_at(&self, point: Point<i32, Logical>) -> Option<TrafficLightKind> {
        self.buttons
            .iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.kind)
    }

    /// Whether `point` is inside the titlebar.
    pub fn hit(&self, point: Point<i32, Logical>) -> bool {
        self.titlebar.contains(point)
    }

    /// Whether the button cluster's glyphs are revealed. The design reveals
    /// them on cluster hover (fullscreen hover reveal is T-01.3), staying
    /// colorless otherwise.
    pub fn revealed(&self) -> bool {
        self.hovered
    }

    fn fill_color(&self, button: &TrafficLightButton) -> Color32F {
        // An inactive window's lights are muted, as are any disabled
        // controls; only an active, enabled light shows its semantic color.
        let rgba = if button.enabled && self.focused {
            self.scheme.light(button.kind)
        } else {
            self.scheme.control_active()
        };
        color_from_rgba(rgba)
    }

    /// The solid-fill render elements for this titlebar, in output-local
    /// physical coordinates. The list is ordered background → lights →
    /// glyphs, so a back-to-front renderer draws them correctly.
    ///
    /// This is the flat T-01.1 fill; T-04 replaces it with the material pass
    /// (translucency, blur, rounding) while keeping the geometry above.
    pub fn render_elements(
        &self,
        scale: Scale<f64>,
        output_origin: Point<i32, Logical>,
    ) -> Vec<SolidColorRenderElement> {
        let mut elements = Vec::new();
        let origin = output_origin;
        let to_physical = |rect: Rectangle<i32, Logical>| rect.to_physical_precise_round(scale);
        let local = |rect: Rectangle<i32, Logical>| {
            Rectangle::new(
                (rect.loc.x - origin.x, rect.loc.y - origin.y).into(),
                rect.size,
            )
        };

        let mut push = |rect: Rectangle<i32, Logical>, color: Color32F| {
            elements.push(SolidColorRenderElement::new(
                Id::new(),
                to_physical(local(rect)),
                CommitCounter::default(),
                color,
                smithay::backend::renderer::element::Kind::Unspecified,
            ));
        };

        // Background: the flat chrome fill.
        push(self.titlebar, color_from_rgba(self.scheme.chrome()));

        let reveal = self.revealed();
        for button in &self.buttons {
            push(button.rect, self.fill_color(button));
            if button.enabled && reveal {
                for glyph in glyph_rects(button.kind, button.rect) {
                    push(glyph, color_from_rgba(self.scheme.traffic_glyph()));
                }
            }
        }
        elements
    }
}

/// Convert an RGBA byte token to a renderer color.
fn color_from_rgba(rgba: [u8; 4]) -> Color32F {
    Color32F::new(
        rgba[0] as f32 / 255.0,
        rgba[1] as f32 / 255.0,
        rgba[2] as f32 / 255.0,
        rgba[3] as f32 / 255.0,
    )
}

/// Axis-aligned marks approximating the control glyphs at token scale. The
/// compositor has no vector rasterizer yet, so glyphs are built from solid
/// rectangles; T-04's material pass may replace them with a real icon atlas.
fn glyph_rects(
    kind: TrafficLightKind,
    button: Rectangle<i32, Logical>,
) -> Vec<Rectangle<i32, Logical>> {
    let size = (traffic_lights::GLYPH_SIZE as i32).max(2);
    let thickness = (size / 3).max(1);
    let cx = button.loc.x + button.size.w / 2;
    let cy = button.loc.y + button.size.h / 2;
    let half = size / 2;
    let mark = |x: i32, y: i32, w: i32, h: i32| Rectangle::new((x, y).into(), (w, h).into());
    match kind {
        TrafficLightKind::Minimize => vec![mark(cx - half, cy - thickness / 2, size, thickness)],
        TrafficLightKind::Zoom => vec![
            mark(cx - half, cy - thickness / 2, size, thickness),
            mark(cx - thickness / 2, cy - half, thickness, size),
        ],
        // A staircase approximation of the diagonal cross.
        TrafficLightKind::Close => {
            let mut rects = Vec::new();
            for step in 0..3 {
                let offset = -half + step * thickness;
                rects.push(mark(cx + offset, cy + offset, thickness, thickness));
                rects.push(mark(
                    cx + offset,
                    cy - offset - thickness,
                    thickness,
                    thickness,
                ));
            }
            rects
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::renderer::element::Element;
    use smithay::utils::Size;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn window_id() -> WindowId {
        WindowId(1)
    }

    fn ssd(content: Rectangle<i32, Logical>) -> TitlebarElement {
        TitlebarElement::for_window(
            window_id(),
            content,
            DecorationTier::ServerSide,
            WindowState::Floating,
            true,
            false,
        )
        .expect("ssd titlebar")
    }

    #[test]
    fn ssd_titlebar_insets_the_client_area_from_token_height() {
        let content = rect(100, 200, 800, 600);
        let titlebar = ssd(content);
        assert_eq!(
            titlebar.insets,
            WindowInsets::for_tier(DecorationTier::ServerSide)
        );
        assert_eq!(titlebar.insets.top, TITLEBAR_HEIGHT);
        assert_eq!(
            titlebar.titlebar,
            rect(100, 200 - TITLEBAR_HEIGHT, 800, TITLEBAR_HEIGHT)
        );
        // The client area is directly below the titlebar and shares its width.
        assert_eq!(titlebar.titlebar.loc.x, titlebar.content.loc.x);
        assert_eq!(
            titlebar.titlebar.loc.y + titlebar.titlebar.size.h,
            titlebar.content.loc.y
        );
        assert_eq!(titlebar.titlebar.size.w, titlebar.content.size.w);
    }

    #[test]
    fn csd_never_gets_a_compositor_titlebar() {
        assert_eq!(
            TitlebarElement::for_window(
                window_id(),
                rect(0, 0, 200, 150),
                DecorationTier::ClientSide,
                WindowState::Floating,
                true,
                false,
            ),
            None
        );
        assert!(WindowInsets::for_tier(DecorationTier::ClientSide).is_empty());
    }

    #[test]
    fn fullscreen_and_minimized_have_no_titlebar() {
        for state in [WindowState::Fullscreen, WindowState::Minimized] {
            assert_eq!(
                TitlebarElement::for_window(
                    window_id(),
                    rect(0, 0, 200, 150),
                    DecorationTier::ServerSide,
                    state,
                    true,
                    false,
                ),
                None,
                "{state:?} must not carry a titlebar"
            );
        }
        // Zoomed windows do.
        assert!(TitlebarElement::for_window(
            window_id(),
            rect(0, 0, 200, 150),
            DecorationTier::ServerSide,
            WindowState::Zoomed,
            true,
            false,
        )
        .is_some());
    }

    #[test]
    fn traffic_lights_are_left_of_center_and_in_order() {
        let content = rect(0, 40, 400, 300);
        let titlebar = ssd(content);
        let kinds: Vec<_> = titlebar.buttons.iter().map(|b| b.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TrafficLightKind::Close,
                TrafficLightKind::Minimize,
                TrafficLightKind::Zoom
            ]
        );
        let inset = traffic_lights::INSET as i32;
        assert_eq!(
            titlebar.buttons[0].rect.loc.x,
            titlebar.titlebar.loc.x + inset
        );
        // Gap between adjacent lights.
        let gap = traffic_lights::GAP as i32;
        let diameter = traffic_lights::DIAMETER as i32;
        assert_eq!(
            titlebar.buttons[1].rect.loc.x,
            titlebar.buttons[0].rect.loc.x + diameter + gap
        );
        // Vertically centered and inside the titlebar.
        for button in &titlebar.buttons {
            assert!(titlebar.titlebar.contains_rect(button.rect));
        }
    }

    #[test]
    fn button_hit_test_matches_the_laid_out_rects() {
        let titlebar = ssd(rect(0, 40, 400, 300));
        let close = titlebar.buttons[0].rect;
        let center = Point::from((
            close.loc.x + close.size.w / 2,
            close.loc.y + close.size.h / 2,
        ));
        assert_eq!(titlebar.button_at(center), Some(TrafficLightKind::Close));
        assert_eq!(titlebar.button_at(Point::from((399, 60))), None);
        assert!(titlebar.hit(Point::from((200, 20))));
        assert!(!titlebar.hit(Point::from((200, 60))));
    }

    #[test]
    fn idle_titlebar_draws_fill_and_lights_but_no_glyphs() {
        let mut titlebar = ssd(rect(0, 40, 200, 150));
        titlebar.hovered = false;
        titlebar.focused = true;
        let elements = titlebar.render_elements(1.0.into(), Point::from((0, 0)));
        // 1 background + 3 lights, no glyphs while not hovered.
        assert_eq!(elements.len(), 1 + 3);
    }

    #[test]
    fn hovered_titlebar_reveals_glyphs() {
        let mut titlebar = ssd(rect(0, 40, 200, 150));
        titlebar.hovered = true;
        titlebar.focused = true;
        let unhovered = {
            let mut t = titlebar.clone();
            t.hovered = false;
            t.render_elements(1.0.into(), Point::from((0, 0))).len()
        };
        let hovered = titlebar
            .render_elements(1.0.into(), Point::from((0, 0)))
            .len();
        assert!(hovered > unhovered, "hover must reveal glyph elements");
    }

    #[test]
    fn inactive_window_lights_are_muted() {
        let mut titlebar = ssd(rect(0, 40, 200, 150));
        let muted = color_from_rgba(titlebar.scheme.control_active());
        titlebar.focused = false;
        for button in &titlebar.buttons {
            assert_eq!(titlebar.fill_color(button), muted);
        }
        titlebar.focused = true;
        assert_eq!(
            titlebar.fill_color(&titlebar.buttons[0]),
            color_from_rgba(titlebar.scheme.light(TrafficLightKind::Close))
        );
    }

    #[test]
    fn disabled_light_draws_the_muted_fill_without_a_glyph() {
        let mut titlebar = ssd(rect(0, 40, 200, 150));
        titlebar.hovered = true;
        titlebar.buttons[0].enabled = false;
        let lit = titlebar.fill_color(&titlebar.buttons[0]);
        let enabled = color_from_rgba(titlebar.scheme.light(TrafficLightKind::Close));
        assert_ne!(lit, enabled, "disabled light must not use its lit color");
        // The disabled light contributes no glyph rects.
        let glyphs: usize = titlebar
            .buttons
            .iter()
            .map(|b| {
                if b.enabled {
                    glyph_rects(b.kind, b.rect).len()
                } else {
                    0
                }
            })
            .sum();
        let total = titlebar
            .render_elements(1.0.into(), Point::from((0, 0)))
            .len();
        assert_eq!(total, 1 + 3 + glyphs);
    }

    #[test]
    fn render_elements_are_offset_by_the_output_origin() {
        let titlebar = ssd(rect(100, 200, 200, 150));
        let local = titlebar.render_elements(1.0.into(), Point::from((0, 0)));
        let shifted = titlebar.render_elements(1.0.into(), Point::from((100, 200)));
        let local_bg = local[0].geometry(1.0.into()).loc;
        let shifted_bg = shifted[0].geometry(1.0.into()).loc;
        assert_eq!(local_bg, (100, 160).into());
        assert_eq!(shifted_bg, (0, -40).into());
    }

    #[test]
    fn insets_shrink_and_offset_a_rectangle() {
        let insets = WindowInsets::for_tier(DecorationTier::ServerSide);
        assert_eq!(
            insets.inset(rect(0, 0, 200, 190)),
            rect(0, TITLEBAR_HEIGHT, 200, 190 - TITLEBAR_HEIGHT)
        );
    }
}
