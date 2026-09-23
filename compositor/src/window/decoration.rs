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
//! * hover and disabled *drawing* exist now; T-01.2 wires interaction: the
//!   compositor sets `hovered` from the pointer and routes button presses
//!   through [`TitlebarElement::button_at`] to the existing window state
//!   machine. This module still owns only geometry/drawing — it never calls
//!   a window action itself.
//!
//! T-01.3 adds drag-to-move (the press is handed to the existing interactive
//! move grab), double-click dispatch ([`TitlebarDoubleClick`] honoring
//! `dock.titlebarDoubleClick`), and the fullscreen hover reveal: a fullscreen
//! window's titlebar overlays the top of its content and is only laid out
//! while `hovered` is set from the reveal strip.
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

/// Two titlebar presses within this many milliseconds count as a
/// double-click (T-01.3).
pub const TITLEBAR_DOUBLE_CLICK_MS: u64 = 400;

/// A double-click must land within this many logical pixels of the first
/// press (T-01.3); the window may have shifted a little under the move grab.
pub const TITLEBAR_DOUBLE_CLICK_SLOP: i32 = 4;

/// The top strip of a fullscreen window's content whose hover reveals the
/// titlebar (T-01.3). When revealed the titlebar overlays exactly this
/// strip; fullscreen reserves no inset.
pub fn fullscreen_reveal_rect(content: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
    Rectangle::new(content.loc, (content.size.w, TITLEBAR_HEIGHT).into())
}

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

    /// Elevated surface fill (popovers, menus; T-01.4).
    pub const fn surface_elevated(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::SURFACE_ELEVATED,
            ColorScheme::Dark => semantic::dark::color::SURFACE_ELEVATED,
        }
    }

    /// Border color (menus, windows; T-01.4).
    pub const fn border(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::BORDER,
            ColorScheme::Dark => semantic::dark::color::BORDER,
        }
    }

    /// Accent fill (menu highlight; T-01.4).
    pub const fn accent(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::ACCENT,
            ColorScheme::Dark => semantic::dark::color::ACCENT,
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

/// What a titlebar double-click does (T-01.3), from the shell's
/// `dock.titlebarDoubleClick` setting. The compositor default is `Zoom`;
/// the live value follows settings in T-08.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitlebarDoubleClick {
    #[default]
    Zoom,
    Minimize,
    None,
}

impl TitlebarDoubleClick {
    /// Parse the shell setting spelling (`zoom` | `minimize` | `none`).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "zoom" => Some(TitlebarDoubleClick::Zoom),
            "minimize" => Some(TitlebarDoubleClick::Minimize),
            "none" => Some(TitlebarDoubleClick::None),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            TitlebarDoubleClick::Zoom => "zoom",
            TitlebarDoubleClick::Minimize => "minimize",
            TitlebarDoubleClick::None => "none",
        }
    }
}

/// One recorded titlebar press, the basis of double-click detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitlebarClick {
    pub window: WindowId,
    pub time_msec: u64,
    pub location: Point<i32, Logical>,
}

/// Double-click detection for titlebar presses (T-01.3). A press is the
/// second of a double-click when it lands on the same window, within
/// [`TITLEBAR_DOUBLE_CLICK_SLOP`] logical pixels and
/// [`TITLEBAR_DOUBLE_CLICK_MS`] milliseconds of the previous press.
#[derive(Debug, Clone, Copy, Default)]
pub struct DoubleClickTracker {
    last: Option<TitlebarClick>,
}

impl DoubleClickTracker {
    pub fn new() -> Self {
        DoubleClickTracker::default()
    }

    /// Record a press and report whether it is the second of a double-click.
    /// A double-click consumes the pair, so a third press starts fresh.
    pub fn register(
        &mut self,
        window: WindowId,
        location: Point<i32, Logical>,
        now_msec: u64,
    ) -> bool {
        let double = self.last.is_some_and(|last| {
            last.window == window
                && now_msec.saturating_sub(last.time_msec) <= TITLEBAR_DOUBLE_CLICK_MS
                && (last.location.x - location.x).abs() <= TITLEBAR_DOUBLE_CLICK_SLOP
                && (last.location.y - location.y).abs() <= TITLEBAR_DOUBLE_CLICK_SLOP
        });
        self.last = if double {
            None
        } else {
            Some(TitlebarClick {
                window,
                time_msec: now_msec,
                location,
            })
        };
        double
    }

    /// Forget any pending press (e.g. after a move starts or focus changes).
    pub fn reset(&mut self) {
        self.last = None;
    }
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
    /// * Fullscreen hides the titlebar until the pointer hovers the top
    ///   strip (`hovered`); when revealed it overlays the top of the
    ///   content and reserves no inset (T-01.3).
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
        if content.size.w <= 0 || content.size.h <= 0 {
            return None;
        }
        let (insets, titlebar) = match state {
            WindowState::Floating | WindowState::Zoomed => {
                let insets = WindowInsets::for_tier(tier);
                let titlebar = Rectangle::new(
                    (content.loc.x, content.loc.y - insets.top).into(),
                    (content.size.w, insets.top).into(),
                );
                (insets, titlebar)
            }
            WindowState::Fullscreen => {
                if !hovered {
                    return None;
                }
                (WindowInsets::NONE, fullscreen_reveal_rect(content))
            }
            WindowState::Minimized => return None,
        };
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

    /// The bounds of the button cluster: the hover target that reveals the
    /// glyphs (T-01.2). Hovering anywhere in the titlebar does not reveal
    /// them, only the cluster does (05-window-decorations.md behavior spec).
    pub fn cluster_rect(&self) -> Rectangle<i32, Logical> {
        let mut buttons = self.buttons.iter();
        let Some(first) = buttons.next() else {
            return Rectangle::new(self.titlebar.loc, (0, 0).into());
        };
        buttons.fold(first.rect, |union, button| union.merge(button.rect))
    }

    /// Whether `point` is inside the titlebar.
    pub fn hit(&self, point: Point<i32, Logical>) -> bool {
        self.titlebar.contains(point)
    }

    /// Whether the button cluster's glyphs are revealed. The design reveals
    /// them on cluster hover; a fullscreen titlebar only exists while its
    /// reveal strip is hovered, so it reveals too (T-01.3).
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
    /// physical coordinates, ordered front-to-back (glyphs → lights →
    /// background) as Smithay's damage tracker requires.
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

        // Build back-to-front (background → lights → glyphs); reverse at the
        // end for Smithay's front-to-back element order.
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

        // Smithay's `OutputDamageTracker::render_output` consumes elements
        // front-to-back and draws them in reverse, and an opaque element culls
        // everything that follows it. Without this reversal the opaque chrome
        // fill would cull its own lights and glyphs (they would never appear).
        elements.reverse();
        elements
    }
}

/// Convert an RGBA byte token to a renderer color.
pub(crate) fn color_from_rgba(rgba: [u8; 4]) -> Color32F {
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
    fn minimized_never_has_a_titlebar() {
        assert_eq!(
            TitlebarElement::for_window(
                window_id(),
                rect(0, 0, 200, 150),
                DecorationTier::ServerSide,
                WindowState::Minimized,
                true,
                true,
            ),
            None
        );
    }

    #[test]
    fn fullscreen_titlebar_is_hidden_until_the_reveal_strip_is_hovered() {
        let content = rect(0, 0, 1280, 720);
        // Not hovered: fullscreen draws nothing.
        assert_eq!(
            TitlebarElement::for_window(
                window_id(),
                content,
                DecorationTier::ServerSide,
                WindowState::Fullscreen,
                true,
                false,
            ),
            None
        );
        // Hovered: the titlebar overlays the top of the content and reserves
        // no inset.
        let revealed = TitlebarElement::for_window(
            window_id(),
            content,
            DecorationTier::ServerSide,
            WindowState::Fullscreen,
            true,
            true,
        )
        .expect("hovered fullscreen titlebar");
        assert_eq!(revealed.insets, WindowInsets::NONE);
        assert_eq!(revealed.titlebar, rect(0, 0, 1280, TITLEBAR_HEIGHT));
        assert_eq!(revealed.titlebar.loc, revealed.content.loc);
        assert!(revealed.revealed());
    }

    #[test]
    fn fullscreen_reveal_rect_is_the_top_strip() {
        let content = rect(10, 20, 800, 600);
        assert_eq!(
            fullscreen_reveal_rect(content),
            rect(10, 20, 800, TITLEBAR_HEIGHT)
        );
    }

    #[test]
    fn double_click_tracker_pairs_presses_on_the_same_window() {
        let mut tracker = DoubleClickTracker::new();
        let id = window_id();
        let at = Point::from((100, 20));
        assert!(!tracker.register(id, at, 1000));
        assert!(tracker.register(id, at, 1200), "second press is a double");
        // The pair is consumed: a third press starts a new pair.
        assert!(!tracker.register(id, at, 1300));
    }

    #[test]
    fn double_click_tracker_rejects_far_or_slow_or_other_window_presses() {
        let id = window_id();
        let other = WindowId(2);
        let at = Point::from((100, 20));

        let mut tracker = DoubleClickTracker::new();
        tracker.register(id, at, 1000);
        assert!(!tracker.register(id, at, 1000 + TITLEBAR_DOUBLE_CLICK_MS + 1));

        let mut tracker = DoubleClickTracker::new();
        tracker.register(id, at, 1000);
        assert!(!tracker.register(
            id,
            Point::from((100 + TITLEBAR_DOUBLE_CLICK_SLOP + 1, 20)),
            1100
        ));

        let mut tracker = DoubleClickTracker::new();
        tracker.register(id, at, 1000);
        assert!(!tracker.register(other, at, 1100));
    }

    #[test]
    fn titlebar_double_click_parses_the_shell_setting() {
        assert_eq!(
            TitlebarDoubleClick::parse("zoom"),
            Some(TitlebarDoubleClick::Zoom)
        );
        assert_eq!(
            TitlebarDoubleClick::parse("minimize"),
            Some(TitlebarDoubleClick::Minimize)
        );
        assert_eq!(
            TitlebarDoubleClick::parse("none"),
            Some(TitlebarDoubleClick::None)
        );
        assert_eq!(TitlebarDoubleClick::parse("fill"), None);
        assert_eq!(TitlebarDoubleClick::default(), TitlebarDoubleClick::Zoom);
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
    fn cluster_rect_spans_every_button_but_not_the_whole_titlebar() {
        let titlebar = ssd(rect(0, 40, 900, 300));
        let cluster = titlebar.cluster_rect();
        for button in &titlebar.buttons {
            assert!(
                cluster.contains_rect(button.rect),
                "cluster must cover each light"
            );
        }
        // The far end of a wide titlebar is outside the cluster.
        assert!(!cluster.contains(Point::from((880, cluster.loc.y + 1))));
        // A point above/below the lights is outside the vertically tight
        // cluster even though it is inside the titlebar.
        assert!(!cluster.contains(Point::from((
            cluster.loc.x + 1,
            titlebar.titlebar.loc.y + 1
        ))));
        assert!(titlebar.hit(Point::from((
            cluster.loc.x + 1,
            titlebar.titlebar.loc.y + 1
        ))));
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
        // The background is the last (backmost) element in front-to-back order.
        let local_bg = local.last().expect("background").geometry(1.0.into()).loc;
        let shifted_bg = shifted.last().expect("background").geometry(1.0.into()).loc;
        assert_eq!(local_bg, (100, 160).into());
        assert_eq!(shifted_bg, (0, -40).into());
    }

    #[test]
    fn render_elements_are_front_to_back_with_the_opaque_background_last() {
        let titlebar = ssd(rect(0, 40, 400, 300));
        let elements = titlebar.render_elements(1.0.into(), Point::from((0, 0)));
        assert_eq!(elements.len(), 4, "background + three lights");
        // The lights come first (front) so the opaque titlebar fill, which is
        // last (back), cannot cull them: an opaque element hides everything
        // that follows it in Smithay's front-to-back list.
        let diameter = traffic_lights::DIAMETER as i32;
        for light in &elements[..3] {
            assert_eq!(light.geometry(1.0.into()).size, (diameter, diameter).into());
        }
        assert_eq!(
            elements
                .last()
                .expect("background")
                .geometry(1.0.into())
                .size,
            (400, TITLEBAR_HEIGHT).into()
        );
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
