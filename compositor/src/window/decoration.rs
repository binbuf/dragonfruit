// SPDX-License-Identifier: MIT
//! Server-side-decoration titlebar element (T-01.1).
//!
//! This is the compositor's SSD titlebar: pure geometry and drawing state,
//! sized from the generated design-system tokens so a compositor-drawn
//! titlebar cannot drift from the first-party QML `TitleBar`/`TrafficLights`
//! (FR-3). It is deliberately a *model plus a solid-fill renderer*:
//!
//! * geometry/insets are deterministic and unit-testable without a GPU;
//! * the fill is a token-opaque color: T-04.1a/1b added the shadow and
//!   rounded corners, and T-20's sampled material is a chrome-panel pass, so
//!   the SSD titlebar deliberately remains opaque rather than becoming glass;
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
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Color32F;
use smithay::utils::{Logical, Physical, Point, Rectangle, Scale};

use crate::design_tokens::{
    component::{traffic_lights, window_style},
    semantic,
};
use crate::window::corner::CornerMask;
use crate::window::motion::MotionFrame;
use crate::window::{DecorationTier, WindowId, WindowState};

/// The measured window-chrome style taxonomy (T230b). Chrome height is a
/// property of the window style, not one universal height. The 66 pt
/// icon-and-label toolbar is `Unified` plus [`WindowChrome::labelled_toolbar`];
/// an app-specific toolbar row can be added on top of any style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowStyle {
    Borderless,
    /// Titled, no toolbar: the minimum real chrome (32 pt).
    #[default]
    Titlebar,
    UnifiedCompact,
    Expanded,
    Unified,
    Preference,
}

impl WindowStyle {
    pub const ALL: [WindowStyle; 6] = [
        WindowStyle::Borderless,
        WindowStyle::Titlebar,
        WindowStyle::UnifiedCompact,
        WindowStyle::Expanded,
        WindowStyle::Unified,
        WindowStyle::Preference,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            WindowStyle::Borderless => "borderless",
            WindowStyle::Titlebar => "titlebar",
            WindowStyle::UnifiedCompact => "unifiedCompact",
            WindowStyle::Expanded => "expanded",
            WindowStyle::Unified => "unified",
            WindowStyle::Preference => "preference",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|style| style.name() == value)
    }

    /// The measured baseline chrome height, from the generated tokens.
    pub const fn height(self) -> i32 {
        match self {
            WindowStyle::Borderless => window_style::borderless::HEIGHT as i32,
            WindowStyle::Titlebar => window_style::titlebar::HEIGHT as i32,
            WindowStyle::UnifiedCompact => window_style::unified_compact::HEIGHT as i32,
            WindowStyle::Expanded => window_style::expanded::HEIGHT as i32,
            WindowStyle::Unified => window_style::unified::HEIGHT as i32,
            WindowStyle::Preference => window_style::preference::HEIGHT as i32,
        }
    }

    pub const fn title_row(self) -> i32 {
        match self {
            WindowStyle::Borderless => window_style::borderless::TITLE_ROW as i32,
            WindowStyle::Titlebar => window_style::titlebar::TITLE_ROW as i32,
            WindowStyle::UnifiedCompact => window_style::unified_compact::TITLE_ROW as i32,
            WindowStyle::Expanded => window_style::expanded::TITLE_ROW as i32,
            WindowStyle::Unified => window_style::unified::TITLE_ROW as i32,
            WindowStyle::Preference => window_style::preference::TITLE_ROW as i32,
        }
    }

    pub const fn toolbar_row(self) -> i32 {
        match self {
            WindowStyle::Borderless => window_style::borderless::TOOLBAR_ROW as i32,
            WindowStyle::Titlebar => window_style::titlebar::TOOLBAR_ROW as i32,
            WindowStyle::UnifiedCompact => window_style::unified_compact::TOOLBAR_ROW as i32,
            WindowStyle::Expanded => window_style::expanded::TOOLBAR_ROW as i32,
            WindowStyle::Unified => window_style::unified::TOOLBAR_ROW as i32,
            WindowStyle::Preference => window_style::preference::TOOLBAR_ROW as i32,
        }
    }
}

/// The final chrome configuration: a named style plus an optional labelled
/// toolbar row (the 66 pt variant) and an app-specific toolbar row (e.g. a
/// Pages-style document at 48 + 22 = 70 pt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowChrome {
    pub style: WindowStyle,
    pub labelled_toolbar: bool,
    pub toolbar_row: i32,
}

impl WindowChrome {
    pub const fn new(style: WindowStyle) -> Self {
        WindowChrome {
            style,
            labelled_toolbar: false,
            toolbar_row: 0,
        }
    }

    pub const fn height(self) -> i32 {
        let extra = if self.toolbar_row > 0 {
            self.toolbar_row
        } else {
            0
        };
        self.style.height()
            + extra
            + if self.labelled_toolbar {
                window_style::LABELLED_TOOLBAR_ROW as i32
            } else {
                0
            }
    }

    /// The row the lights live in: the explicit title row for
    /// `expanded`/`preference`, or the whole merged row for the unified
    /// styles. Using the final chrome height would push a preference window's
    /// lights down into its toolbar row.
    pub const fn title_row_height(self) -> i32 {
        if self.style.title_row() > 0 {
            self.style.title_row()
        } else {
            self.height()
        }
    }

    /// The measured 8/18 pt inset is to the 16 pt AppKit accessibility frame,
    /// and it applies on both axes. The drawn 14 pt circle is centred inside
    /// that frame, so its visible edge sits 1 pt further in.
    pub const fn traffic_inset(self) -> i32 {
        let inset = (self.title_row_height() - traffic_lights::BUTTON_SIZE as i32) / 2;
        if inset > 0 {
            inset
        } else {
            0
        }
    }
}

/// The compositor's default style for a server-decorated client. It draws a
/// titlebar without a toolbar, so the minimum titled style is the honest
/// default; first-party CSD apps declare their own style in QML.
pub const DEFAULT_WINDOW_STYLE: WindowStyle = WindowStyle::Titlebar;

/// Two titlebar presses within this many milliseconds count as a
/// double-click (T-01.3).
pub const TITLEBAR_DOUBLE_CLICK_MS: u64 = 400;

/// A double-click must land within this many logical pixels of the first
/// press (T-01.3); the window may have shifted a little under the move grab.
pub const TITLEBAR_DOUBLE_CLICK_SLOP: i32 = 4;

/// The top strip of a fullscreen window's content whose hover reveals the
/// titlebar (T-01.3). When revealed the titlebar overlays exactly this
/// strip; fullscreen reserves no inset. T230b sizes the strip from the
/// window's style rather than the old single titlebar token.
pub fn fullscreen_reveal_rect(
    content: Rectangle<i32, Logical>,
    chrome: WindowChrome,
) -> Rectangle<i32, Logical> {
    Rectangle::new(content.loc, (content.size.w, chrome.height()).into())
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
    /// The design-system spelling (`light` | `dark`), used by the settings
    /// seam (T-08) and the synthetic-input material commands (T-04.4b).
    pub const fn name(self) -> &'static str {
        match self {
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        }
    }

    /// Parse the design-system spelling (`light` | `dark`).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(ColorScheme::Light),
            "dark" => Some(ColorScheme::Dark),
            _ => None,
        }
    }

    /// Titlebar background (flat fill until the T-04 material pass).
    pub const fn chrome(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::CHROME,
            ColorScheme::Dark => semantic::dark::color::CHROME,
        }
    }

    /// The Dock plate's backdrop tone (T-14.7j).
    pub const fn dock_fill(self) -> [u8; 4] {
        match self {
            ColorScheme::Light => semantic::light::color::DOCK_FILL,
            ColorScheme::Dark => semantic::dark::color::DOCK_FILL,
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

    /// The insets a window with this decoration tier reserves, using the
    /// default compositor style (a titlebar-only window).
    pub const fn for_tier(tier: DecorationTier) -> Self {
        Self::for_tier_with_chrome(tier, WindowChrome::new(DEFAULT_WINDOW_STYLE))
    }

    /// The insets a window with this decoration tier and chrome style
    /// reserves (T230b).
    pub const fn for_tier_with_chrome(tier: DecorationTier, chrome: WindowChrome) -> Self {
        match tier {
            DecorationTier::ServerSide => WindowInsets {
                top: chrome.height(),
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

    /// Grow and offset `rect` back out by these insets (the whole decorated
    /// window, titlebar included). The inverse of [`Self::inset`].
    pub fn outset(self, rect: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        let w = (rect.size.w + self.left + self.right).max(0);
        let h = (rect.size.h + self.top + self.bottom).max(0);
        Rectangle::new(
            (rect.loc.x - self.left, rect.loc.y - self.top).into(),
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
    /// The named style and final chrome height that produced this titlebar.
    pub chrome: WindowChrome,
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
        chrome: WindowChrome,
        focused: bool,
        hovered: bool,
    ) -> Option<Self> {
        if tier != DecorationTier::ServerSide {
            return None;
        }
        if content.size.w <= 0 || content.size.h <= 0 || chrome.height() <= 0 {
            return None;
        }
        let (insets, titlebar) = match state {
            WindowState::Floating | WindowState::Zoomed => {
                let insets = WindowInsets::for_tier_with_chrome(tier, chrome);
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
                (WindowInsets::NONE, fullscreen_reveal_rect(content, chrome))
            }
            WindowState::Minimized => return None,
        };
        let buttons = Self::layout_buttons(titlebar, chrome);
        Some(TitlebarElement {
            window,
            content,
            chrome,
            insets,
            titlebar,
            focused,
            hovered,
            scheme: ColorScheme::default(),
            buttons,
        })
    }

    fn layout_buttons(
        titlebar: Rectangle<i32, Logical>,
        chrome: WindowChrome,
    ) -> Vec<TrafficLightButton> {
        // The 16 pt token is the AX/hit frame; the visible circle is 14 pt
        // and is drawn centred inside it.
        let size = traffic_lights::BUTTON_SIZE as i32;
        // The token is centre-to-centre; advance by exactly that much.
        let spacing = traffic_lights::SPACING as i32;
        let inset = chrome.traffic_inset();
        let y = titlebar.loc.y + inset;
        let mut x = titlebar.loc.x + inset;
        TrafficLightKind::ALL
            .iter()
            .map(|kind| {
                let rect = Rectangle::new((x, y).into(), (size, size).into());
                x += spacing;
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
    /// `motion` carries the window's in-flight lifecycle frame (T-02.1b/
    /// T-02.2): the titlebar is drawn with the same scale/fade as the client
    /// surface, so the decoration does not detach from a window that is still
    /// scaling in, restoring, or minimizing.
    ///
    /// The lights are rasterized as antialiased circles at the output's
    /// **physical** pixel resolution; the chrome is a flat token fill with the
    /// top corners clipped to the token-derived radius (T-04.1b). T-04.2 adds
    /// the material blur on the same geometry.
    pub fn render_elements(
        &self,
        scale: Scale<f64>,
        output_origin: Point<i32, Logical>,
        motion: Option<MotionFrame>,
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
        // Track the scale/fade of the lifecycle transform on the titlebar
        // rects (the rects are already relative to the window's target
        // geometry).
        let target = self.content;

        const TITLEBAR_SALT: u64 = 0x7469_746c_6562_6172; // "titlebar"
        let mut push = |physical: Rectangle<i32, Physical>, color: Color32F| {
            elements.push(solid_element(TITLEBAR_SALT, physical, color));
        };

        // Build back-to-front (background → lights → glyphs); reverse at the
        // end for Smithay's front-to-back element order.
        //
        // The chrome fill is clipped to the token radius on its top corners
        // (T-04.1b): the same decomposition the QML `TitleBar` produces with a
        // full-radius rectangle plus a square bottom patch, so the compositor
        // SSD and the first-party titlebar cannot drift (FR-3). The bottom
        // edge stays square and joins the content below.
        for span in CornerMask::titlebar().spans(self.titlebar) {
            let (rect, color) =
                motion_transform(span, color_from_rgba(self.scheme.chrome()), target, motion);
            push(to_physical(local(rect)), color);
        }

        let reveal = self.revealed();
        // The 16 pt button rect is the AX/hit frame; AppKit draws a 14 pt
        // circle centred inside it. Draw the inner circle, not the frame.
        for button in &self.buttons {
            let fill = self.fill_color(button);
            let visual = traffic_light_visual_rect(button.rect);
            // Apply the lifecycle transform to the button once, then
            // rasterize the transformed circle in physical pixels. Scaling
            // individual logical strips (as this used to) snapped every edge
            // to the logical grid, which read as a low-resolution circle.
            let (button_rect, button_fill) = motion_transform(visual, fill, target, motion);
            for span in circle_spans(button_rect, scale, origin) {
                push(span.rect, button_fill * span.coverage);
            }
            if button.enabled && reveal {
                // Scale the glyph with the same lifecycle frame as the
                // circle, then rasterize its antialiased strokes.
                let (glyph_frame, glyph_color) = motion_transform(
                    visual,
                    color_from_rgba(self.scheme.traffic_glyph()),
                    target,
                    motion,
                );
                let glyph_scale = f64::from(glyph_frame.size.w) / f64::from(visual.size.w);
                let glyph_size = traffic_lights::GLYPH_SIZE as f64 * glyph_scale;
                for span in glyph_spans(button.kind, glyph_frame, glyph_size, scale, origin) {
                    push(span.rect, glyph_color * span.coverage);
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

/// Apply a window lifecycle [`MotionFrame`] to one chrome rect and its color
/// (T-02.1b/T-02.2). `target` is the window's committed geometry the frame's
/// scale/offset are relative to; the rect is scaled about it, translated to
/// the interpolated location, and faded by the frame's alpha. Shared by the
/// SSD titlebar and the window shadow (T-04.1a) so both carry the same
/// transform as the client surface.
pub(crate) fn motion_transform(
    rect: Rectangle<i32, Logical>,
    color: Color32F,
    target: Rectangle<i32, Logical>,
    motion: Option<MotionFrame>,
) -> (Rectangle<i32, Logical>, Color32F) {
    match motion {
        Some(frame) => {
            let rel = rect.loc - target.loc;
            let scaled_loc = frame.rect.loc
                + Point::from((
                    (f64::from(rel.x) * frame.scale.x).round() as i32,
                    (f64::from(rel.y) * frame.scale.y).round() as i32,
                ));
            let scaled_w = (f64::from(rect.size.w) * frame.scale.x).round() as i32;
            let scaled_h = (f64::from(rect.size.h) * frame.scale.y).round() as i32;
            (
                Rectangle::new(scaled_loc, (scaled_w.max(1), scaled_h.max(1)).into()),
                color * frame.alpha,
            )
        }
        None => (rect, color),
    }
}

/// One physical pixel span of a rasterized circle and how much of that span
/// the circle covers (`0..=1`). The renderer multiplies the fill's alpha by
/// this coverage, so the edge pixels blend instead of stair-stepping.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CircleSpan {
    rect: Rectangle<i32, Physical>,
    coverage: f32,
}

/// Build one solid render element with a stable id for `salt`/geometry/color.
fn solid_element(
    salt: u64,
    rect: Rectangle<i32, Physical>,
    color: Color32F,
) -> SolidColorRenderElement {
    SolidColorRenderElement::new(
        crate::window::stable_solid_id(salt, rect, color),
        rect,
        CommitCounter::default(),
        color,
        smithay::backend::renderer::element::Kind::Unspecified,
    )
}

/// The visible circle rect inside a traffic light's 16 pt AX/hit frame.
fn traffic_light_visual_rect(button: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
    let diameter = traffic_lights::DIAMETER as i32;
    let pad = ((button.size.w - diameter) / 2).max(0);
    Rectangle::new(
        (button.loc.x + pad, button.loc.y + pad).into(),
        (diameter, diameter).into(),
    )
}

/// Rasterize the filled circle inscribed in `button` as antialiased physical
/// pixel spans.
///
/// The compositor has no vector rasterizer yet, so the macOS-style round
/// traffic lights are built from per-pixel spans. The old implementation
/// produced one *logical* row per strip; at 1x that is a 12-row staircase,
/// and at fractional/HiDPI scales each strip snapped to the logical grid, so
/// the circles read as low-resolution blocks. This rasterizer works in
/// physical pixels: it emits one row per physical pixel (so the circle gains
/// detail with the output scale), and gives the partial edge pixels a
/// coverage alpha so the silhouette is smooth.
///
/// `button` is already in output-local logical coordinates (the caller applies
/// any lifecycle motion), and the returned rects are output-local physical
/// coordinates. T-04's material pass may eventually replace this with a
/// texture.
fn circle_spans(
    button: Rectangle<i32, Logical>,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
) -> Vec<CircleSpan> {
    let diameter_x = f64::from(button.size.w) * scale.x;
    let diameter_y = f64::from(button.size.h) * scale.y;
    let diameter = diameter_x.min(diameter_y);
    if diameter <= 0.0 {
        return Vec::new();
    }

    let left = (f64::from(button.loc.x) - f64::from(output_origin.x)) * scale.x;
    let top = (f64::from(button.loc.y) - f64::from(output_origin.y)) * scale.y;
    let radius = diameter / 2.0;
    let center_x = left + diameter_x / 2.0;
    let center_y = top + diameter_y / 2.0;

    let first_row = (center_y - radius).floor() as i32;
    let last_row = (center_y + radius).ceil() as i32;
    let mut spans = Vec::with_capacity((last_row - first_row).max(0) as usize);

    for row in first_row..last_row {
        let row_top = f64::from(row);
        let row_bottom = row_top + 1.0;
        // How much of this physical pixel row the circle covers vertically.
        let overlap = row_bottom.min(center_y + radius) - row_top.max(center_y - radius);
        if overlap <= 0.0 {
            continue;
        }
        let coverage_y = overlap.min(1.0) as f32;

        // Sample the chord at the middle of the row's covered part, so the
        // partial pole rows still get their average width rather than a
        // zero-width sample at the mathematical tangent.
        let sample_y = row_top.max(center_y - radius) + overlap / 2.0;
        let dy = sample_y - center_y;
        let half = (radius * radius - dy * dy).max(0.0).sqrt();
        let x_left = center_x - half;
        let x_right = center_x + half;
        if x_right <= x_left {
            continue;
        }

        let first_full = x_left.ceil() as i32;
        let last_full = x_right.floor() as i32;
        let mut push_span = |x: i32, width: i32, coverage: f32| {
            if width > 0 && coverage > 0.0 {
                spans.push(CircleSpan {
                    rect: Rectangle::new((x, row).into(), (width, 1).into()),
                    coverage: (coverage * coverage_y).clamp(0.0, 1.0),
                });
            }
        };

        if first_full < last_full {
            // A fully covered centre span plus at most one partial pixel on
            // each side.
            push_span(first_full, last_full - first_full, 1.0);
            if x_left < f64::from(first_full) {
                push_span(first_full - 1, 1, (f64::from(first_full) - x_left) as f32);
            }
            if x_right > f64::from(last_full) {
                push_span(last_full, 1, (x_right - f64::from(last_full)) as f32);
            }
        } else {
            // The chord is narrower than a pixel: split its coverage across
            // the one or two pixels it touches.
            let first = x_left.floor() as i32;
            let last = x_right.floor() as i32;
            for x in first..=last {
                let covered = x_right.min(f64::from(x + 1)) - x_left.max(f64::from(x));
                push_span(x, 1, covered as f32);
            }
        }
    }
    spans
}

/// Rasterize one of the traffic-light glyphs as antialiased physical pixel
/// spans. The marks follow Tabler's 24-unit geometry: the path spans 12 units
/// and the stroke is 4 units, so a 7 pt box is a ~4.7 pt mark with a ~1.2 pt
/// stroke. The old implementation drew 2 pt rectangles over the full glyph
/// box, which read as oversized and blocky next to the QML Tabler icons.
fn glyph_spans(
    kind: TrafficLightKind,
    frame: Rectangle<i32, Logical>,
    glyph_size: f64,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
) -> Vec<CircleSpan> {
    let size = glyph_size.max(1.0);
    // Tabler's path spans 12 of 24 units; its stroke is 4 of 24.
    let half = size / 4.0;
    let stroke = (size * 4.0 / 24.0).max(0.5);

    let center_x = (f64::from(frame.loc.x) + f64::from(frame.size.w) / 2.0
        - f64::from(output_origin.x))
        * scale.x;
    let center_y = (f64::from(frame.loc.y) + f64::from(frame.size.h) / 2.0
        - f64::from(output_origin.y))
        * scale.y;

    let half_x = half * scale.x;
    let half_y = half * scale.y;
    let stroke_px = (stroke * scale.x + stroke * scale.y) / 2.0;

    // The Tabler `maximize` glyph is four corner brackets whose outer
    // corners sit at 4/24 and 20/24 of the box, so it reaches size/3 from
    // the centre. The minus and cross reach size/4.
    let bracket_x = size / 3.0 * scale.x;
    let bracket_y = size / 3.0 * scale.y;
    let arm_x = size / 6.0 * scale.x;
    let arm_y = size / 6.0 * scale.y;
    let zoom = matches!(kind, TrafficLightKind::Zoom);
    let reach_half_x = if zoom { bracket_x } else { half_x };
    let reach_half_y = if zoom { bracket_y } else { half_y };
    let reach_x = reach_half_x + stroke_px / 2.0 + 1.0;
    let reach_y = reach_half_y + stroke_px / 2.0 + 1.0;

    // Each glyph is one or more line segments in frame-local physical space.
    let mut segments: Vec<(f64, f64, f64, f64)> = Vec::new();
    match kind {
        TrafficLightKind::Minimize => segments.push((-half_x, 0.0, half_x, 0.0)),
        TrafficLightKind::Zoom => {
            // Four L-shaped corner brackets, matching Tabler's `maximize`.
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let x = sx * bracket_x;
                let y = sy * bracket_y;
                segments.push((x, y, x, y - sy * arm_y));
                segments.push((x, y, x - sx * arm_x, y));
            }
        }
        TrafficLightKind::Close => {
            segments.push((-half_x, -half_y, half_x, half_y));
            segments.push((-half_x, half_y, half_x, -half_y));
        }
    }

    let first_x = (center_x - reach_x).floor() as i32;
    let last_x = (center_x + reach_x).ceil() as i32;
    let first_y = (center_y - reach_y).floor() as i32;
    let last_y = (center_y + reach_y).ceil() as i32;
    let mut spans = Vec::new();
    for y in first_y..last_y {
        for x in first_x..last_x {
            let px = f64::from(x) + 0.5;
            let py = f64::from(y) + 0.5;
            let distance = segments
                .iter()
                .map(|&(ax, ay, bx, by)| {
                    distance_to_segment(px - center_x, py - center_y, ax, ay, bx, by)
                })
                .fold(f64::INFINITY, f64::min);
            let coverage = (stroke_px / 2.0 - distance + 0.5).clamp(0.0, 1.0);
            if coverage > 0.0 {
                spans.push(CircleSpan {
                    rect: Rectangle::new((x, y).into(), (1, 1).into()),
                    coverage: coverage as f32,
                });
            }
        }
    }
    spans
}

/// Shortest distance from a point to a line segment, in physical pixels.
fn distance_to_segment(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f64::EPSILON {
        return (px * px + py * py).sqrt();
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0);
    let qx = ax + t * dx;
    let qy = ay + t * dy;
    ((px - qx).powi(2) + (py - qy).powi(2)).sqrt()
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
        ssd_with_chrome(content, WindowChrome::new(DEFAULT_WINDOW_STYLE))
    }

    fn ssd_with_chrome(content: Rectangle<i32, Logical>, chrome: WindowChrome) -> TitlebarElement {
        TitlebarElement::for_window(
            window_id(),
            content,
            DecorationTier::ServerSide,
            WindowState::Floating,
            chrome,
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
        assert_eq!(titlebar.insets.top, DEFAULT_WINDOW_STYLE.height());
        assert_eq!(
            titlebar.titlebar,
            rect(
                100,
                200 - DEFAULT_WINDOW_STYLE.height(),
                800,
                DEFAULT_WINDOW_STYLE.height()
            )
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
    fn window_style_table_covers_the_measured_baselines() {
        let expected = [
            (WindowStyle::Borderless, "borderless", 0),
            (WindowStyle::Titlebar, "titlebar", 32),
            (WindowStyle::UnifiedCompact, "unifiedCompact", 40),
            (WindowStyle::Expanded, "expanded", 48),
            (WindowStyle::Unified, "unified", 52),
            (WindowStyle::Preference, "preference", 88),
        ];
        assert_eq!(WindowStyle::ALL.len(), expected.len());
        for (style, name, height) in expected {
            assert_eq!(style.name(), name);
            assert_eq!(WindowStyle::parse(name), Some(style));
            assert_eq!(style.height(), height, "{name}");
            assert_eq!(
                style.title_row() + style.toolbar_row(),
                style.height(),
                "{name}: title row + toolbar row must be the baseline"
            );
        }
        assert_eq!(WindowStyle::parse("labelled"), None);
    }

    #[test]
    fn labelled_unified_is_66_and_a_document_row_can_make_70() {
        let labelled = WindowChrome {
            style: WindowStyle::Unified,
            labelled_toolbar: true,
            toolbar_row: 0,
        };
        assert_eq!(labelled.height(), 66);
        let document = WindowChrome {
            style: WindowStyle::Expanded,
            labelled_toolbar: false,
            toolbar_row: 22,
        };
        assert_eq!(document.height(), 70);
    }

    #[test]
    fn traffic_light_geometry_is_style_derived() {
        // The 16 pt AX frame and 14 pt drawn circle are separate tokens; the
        // reference measured the frame, and drawing it made the lights too
        // large.
        assert_eq!(traffic_lights::BUTTON_SIZE as i32, 16);
        assert_eq!(traffic_lights::DIAMETER as i32, 14);
        assert_eq!(traffic_lights::SPACING as i32, 23);
        assert_eq!(WindowChrome::new(WindowStyle::Titlebar).traffic_inset(), 8);
        assert_eq!(WindowChrome::new(WindowStyle::Unified).traffic_inset(), 18);
        // Expanded and preference keep the lights in their title row, not in
        // the middle of the full chrome.
        assert_eq!(WindowChrome::new(WindowStyle::Expanded).traffic_inset(), 6);
        assert_eq!(
            WindowChrome::new(WindowStyle::Preference).traffic_inset(),
            8
        );
        let content = rect(0, 100, 400, 300);
        let titlebar = ssd_with_chrome(content, WindowChrome::new(WindowStyle::Unified));
        assert_eq!(titlebar.titlebar.size.h, 52);
        assert_eq!(titlebar.buttons[0].rect.loc.x, titlebar.titlebar.loc.x + 18);
        assert_eq!(titlebar.buttons[0].rect.size.w, 16);
        assert_eq!(
            titlebar.buttons[1].rect.loc.x - titlebar.buttons[0].rect.loc.x,
            23
        );
        assert_eq!(titlebar.buttons[0].rect.loc.y - titlebar.titlebar.loc.y, 18);
        for button in &titlebar.buttons {
            assert!(titlebar.titlebar.contains_rect(button.rect));
        }
    }

    #[test]
    fn csd_never_gets_a_compositor_titlebar() {
        assert_eq!(
            TitlebarElement::for_window(
                window_id(),
                rect(0, 0, 200, 150),
                DecorationTier::ClientSide,
                WindowState::Floating,
                WindowChrome::new(DEFAULT_WINDOW_STYLE),
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
                WindowChrome::new(DEFAULT_WINDOW_STYLE),
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
                WindowChrome::new(DEFAULT_WINDOW_STYLE),
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
            WindowChrome::new(DEFAULT_WINDOW_STYLE),
            true,
            true,
        )
        .expect("hovered fullscreen titlebar");
        assert_eq!(revealed.insets, WindowInsets::NONE);
        assert_eq!(
            revealed.titlebar,
            rect(0, 0, 1280, DEFAULT_WINDOW_STYLE.height())
        );
        assert_eq!(revealed.titlebar.loc, revealed.content.loc);
        assert!(revealed.revealed());
    }

    #[test]
    fn fullscreen_reveal_rect_is_the_top_strip() {
        let content = rect(10, 20, 800, 600);
        let chrome = WindowChrome::new(WindowStyle::Unified);
        assert_eq!(
            fullscreen_reveal_rect(content, chrome),
            rect(10, 20, 800, 52)
        );
        assert_eq!(
            fullscreen_reveal_rect(content, WindowChrome::new(DEFAULT_WINDOW_STYLE)),
            rect(10, 20, 800, 32)
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
    fn color_scheme_names_and_parses_the_settings_spelling() {
        assert_eq!(ColorScheme::Light.name(), "light");
        assert_eq!(ColorScheme::Dark.name(), "dark");
        assert_eq!(ColorScheme::parse("light"), Some(ColorScheme::Light));
        assert_eq!(ColorScheme::parse("dark"), Some(ColorScheme::Dark));
        assert_eq!(ColorScheme::parse("sepia"), None);
        // The compositor default reads over arbitrary client pixels until the
        // live setting lands (T-08).
        assert_eq!(ColorScheme::default(), ColorScheme::Dark);
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
        let inset = WindowChrome::new(DEFAULT_WINDOW_STYLE).traffic_inset();
        assert_eq!(
            titlebar.buttons[0].rect.loc.x,
            titlebar.titlebar.loc.x + inset
        );
        // The measured spacing is centre-to-centre.
        let spacing = traffic_lights::SPACING as i32;
        assert_eq!(
            titlebar.buttons[1].rect.loc.x,
            titlebar.buttons[0].rect.loc.x + spacing
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

    /// The horizontal extent covered by each physical row of a rasterized
    /// circle (its spans are split into a centre and partial edge pixels).
    fn row_extents(spans: &[CircleSpan]) -> std::collections::BTreeMap<i32, i32> {
        let mut extents: std::collections::BTreeMap<i32, (i32, i32)> =
            std::collections::BTreeMap::new();
        for span in spans {
            let entry = extents
                .entry(span.rect.loc.y)
                .or_insert((span.rect.loc.x, span.rect.loc.x));
            entry.0 = entry.0.min(span.rect.loc.x);
            entry.1 = entry.1.max(span.rect.loc.x + span.rect.size.w);
        }
        extents
            .into_iter()
            .map(|(y, (left, right))| (y, right - left))
            .collect()
    }

    /// The number of solid spans the three rasterized lights contribute.
    fn light_span_count(titlebar: &TitlebarElement, scale: Scale<f64>) -> usize {
        titlebar
            .buttons
            .iter()
            .map(|button| {
                circle_spans(
                    traffic_light_visual_rect(button.rect),
                    scale,
                    Point::from((0, 0)),
                )
                .len()
            })
            .sum()
    }

    /// The number of solid spans the rounded chrome fill contributes.
    fn titlebar_fill_count(titlebar: &TitlebarElement) -> usize {
        CornerMask::titlebar().spans(titlebar.titlebar).len()
    }

    #[test]
    fn idle_titlebar_draws_fill_and_lights_but_no_glyphs() {
        let mut titlebar = ssd(rect(0, 40, 200, 150));
        titlebar.hovered = false;
        titlebar.focused = true;
        let elements = titlebar.render_elements(1.0.into(), Point::from((0, 0)), None);
        // The rounded fill spans + the light spans, no glyphs while not
        // hovered.
        assert_eq!(
            elements.len(),
            titlebar_fill_count(&titlebar) + light_span_count(&titlebar, 1.0.into())
        );
    }

    #[test]
    fn traffic_light_draws_a_14_pt_circle_inside_the_16_pt_frame() {
        let titlebar = ssd(rect(0, 40, 200, 150));
        let close = titlebar.buttons[0].rect;
        assert_eq!(close.size.w, traffic_lights::BUTTON_SIZE as i32);
        assert_eq!(close.size.h, traffic_lights::BUTTON_SIZE as i32);

        let elements = titlebar.render_elements(1.0.into(), Point::from((0, 0)), None);
        let fill = titlebar_fill_count(&titlebar);
        let close_physical = close.to_physical_precise_round(Scale::from(1.0));
        let rects: Vec<_> = elements[..elements.len() - fill]
            .iter()
            .map(|element| element.geometry(1.0.into()))
            .filter(|rect| close_physical.contains_rect(*rect))
            .collect();

        let left = rects.iter().map(|rect| rect.loc.x).min().unwrap();
        let right = rects
            .iter()
            .map(|rect| rect.loc.x + rect.size.w)
            .max()
            .unwrap();
        assert_eq!(
            right - left,
            traffic_lights::DIAMETER as i32,
            "the drawn circle is 14 pt, not the 16 pt AX frame"
        );
        assert_eq!(
            left,
            close.loc.x + 1,
            "the circle is centred 1 pt inside the frame"
        );
        assert_eq!(
            rects.iter().map(|rect| rect.loc.y).min().unwrap(),
            close.loc.y + 1,
            "the circle is centred 1 pt inside the frame"
        );
    }

    #[test]
    fn traffic_light_glyphs_are_small_and_antialiased() {
        let titlebar = ssd(rect(0, 40, 200, 150));
        let visual = traffic_light_visual_rect(titlebar.buttons[0].rect);
        let spans = glyph_spans(
            TrafficLightKind::Close,
            visual,
            traffic_lights::GLYPH_SIZE as f64,
            1.0.into(),
            Point::from((0, 0)),
        );
        let solid: Vec<_> = spans.iter().filter(|span| span.coverage >= 0.5).collect();
        let left = solid.iter().map(|span| span.rect.loc.x).min().unwrap();
        let right = solid
            .iter()
            .map(|span| span.rect.loc.x + span.rect.size.w)
            .max()
            .unwrap();
        // Tabler's 12-of-24 path plus its 4-of-24 stroke is ~4.7 pt, not the
        // full 7 pt box the old rectangle marks used.
        assert!(right - left <= 5, "glyph must be ~4.7 pt wide");
        assert!(
            spans
                .iter()
                .any(|span| span.coverage > 0.0 && span.coverage < 1.0),
            "glyph edges must be antialiased"
        );
    }

    #[test]
    fn zoom_glyph_draws_corner_brackets_not_a_plus() {
        let titlebar = ssd(rect(0, 40, 200, 150));
        let visual = traffic_light_visual_rect(titlebar.buttons[2].rect);
        let spans = glyph_spans(
            TrafficLightKind::Zoom,
            visual,
            traffic_lights::GLYPH_SIZE as f64,
            1.0.into(),
            Point::from((0, 0)),
        );
        let center_x = visual.loc.x + visual.size.w / 2;
        let center_y = visual.loc.y + visual.size.h / 2;
        assert!(
            !spans.iter().any(|span| span.rect.loc.x == center_x
                && span.rect.loc.y == center_y
                && span.coverage > 0.5),
            "the zoom glyph must be Tabler's corner brackets, not a plus"
        );
    }

    #[test]
    fn traffic_lights_are_rasterized_as_circles() {
        let button = rect(10, 10, 12, 12);
        let spans = circle_spans(button, 1.0.into(), Point::from((0, 0)));
        let widths = row_extents(&spans);
        // The widest row reaches the full diameter at the vertical centre;
        // the pole rows are narrower, so the silhouette is round, not square.
        assert_eq!(*widths.values().max().unwrap(), 12);
        assert!(
            widths[&button.loc.y] < 12,
            "the top row must be narrower than the middle"
        );
        for span in &spans {
            assert_eq!(span.rect.size.h, 1);
            assert!(span.rect.loc.x >= button.loc.x);
            assert!(span.rect.loc.x + span.rect.size.w <= button.loc.x + button.size.w);
            assert!((0.0..=1.0).contains(&span.coverage));
        }
    }

    #[test]
    fn traffic_lights_are_antialiased_at_physical_resolution() {
        let button = rect(10, 10, 12, 12);

        // At 2x the circle is rasterized into twice as many physical rows
        // and spans the full 24 physical pixels, so it gains detail instead
        // of snapping to the logical grid.
        let spans = circle_spans(button, 2.0.into(), Point::from((0, 0)));
        let widths = row_extents(&spans);
        assert_eq!(widths.len(), 24, "one physical row per physical pixel");
        assert_eq!(*widths.values().max().unwrap(), 24);

        // Edge pixels are partially covered, which is what removes the
        // staircase. Interior pixels stay fully opaque.
        assert!(
            spans
                .iter()
                .any(|span| span.coverage > 0.0 && span.coverage < 1.0),
            "the circle must have antialiased edge spans"
        );
        assert!(
            spans.iter().any(|span| span.coverage == 1.0),
            "the circle must have fully covered interior spans"
        );

        // No two spans overlap: each physical pixel is painted at most once.
        let mut covered = std::collections::BTreeSet::new();
        for span in &spans {
            for x in span.rect.loc.x..span.rect.loc.x + span.rect.size.w {
                assert!(
                    covered.insert((x, span.rect.loc.y)),
                    "overlapping spans at ({x}, {})",
                    span.rect.loc.y
                );
            }
        }

        // The covered area is close to the analytic circle area (coverage is
        // an area fraction, so summing it approximates the disc).
        let area: f32 = spans
            .iter()
            .map(|span| span.rect.size.w as f32 * span.coverage)
            .sum();
        let expected = std::f32::consts::PI * 12.0 * 12.0;
        assert!(
            (area - expected).abs() < expected * 0.05,
            "rasterized area {area} should be close to {expected}"
        );
    }

    #[test]
    fn hovered_titlebar_reveals_glyphs() {
        let mut titlebar = ssd(rect(0, 40, 200, 150));
        titlebar.hovered = true;
        titlebar.focused = true;
        let unhovered = {
            let mut t = titlebar.clone();
            t.hovered = false;
            t.render_elements(1.0.into(), Point::from((0, 0)), None)
                .len()
        };
        let hovered = titlebar
            .render_elements(1.0.into(), Point::from((0, 0)), None)
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
        // The disabled light contributes no glyph spans.
        let glyphs: usize = titlebar
            .buttons
            .iter()
            .map(|b| {
                if b.enabled {
                    glyph_spans(
                        b.kind,
                        traffic_light_visual_rect(b.rect),
                        traffic_lights::GLYPH_SIZE as f64,
                        1.0.into(),
                        Point::from((0, 0)),
                    )
                    .len()
                } else {
                    0
                }
            })
            .sum();
        let total = titlebar
            .render_elements(1.0.into(), Point::from((0, 0)), None)
            .len();
        assert_eq!(
            total,
            titlebar_fill_count(&titlebar) + light_span_count(&titlebar, 1.0.into()) + glyphs
        );
    }

    #[test]
    fn render_elements_are_offset_by_the_output_origin() {
        let titlebar = ssd(rect(100, 200, 200, 150));
        let local = titlebar.render_elements(1.0.into(), Point::from((0, 0)), None);
        let shifted = titlebar.render_elements(1.0.into(), Point::from((100, 200)), None);
        // The fill is the backmost group; its last element is the top rounded
        // row, inset by the token radius. It moves with the output origin.
        let local_bg = local.last().expect("background").geometry(1.0.into()).loc;
        let shifted_bg = shifted.last().expect("background").geometry(1.0.into()).loc;
        let inset =
            CornerMask::titlebar().spans(titlebar.titlebar)[0].loc.x - titlebar.titlebar.loc.x;
        let height = DEFAULT_WINDOW_STYLE.height();
        assert_eq!(local_bg, (100 + inset, 200 - height).into());
        assert_eq!(shifted_bg, (inset, -height).into());
    }

    #[test]
    fn render_elements_are_front_to_back_with_the_opaque_fill_last() {
        let titlebar = ssd(rect(0, 40, 400, 300));
        let elements = titlebar.render_elements(1.0.into(), Point::from((0, 0)), None);
        let fill = titlebar_fill_count(&titlebar);
        assert!(fill > 1, "the chrome fill must be a rounded decomposition");

        // The backmost `fill` elements are the chrome, and their union is the
        // whole titlebar rect (the shape is clipped, not shrunk).
        let fill_elements = &elements[elements.len() - fill..];
        let union = fill_elements
            .iter()
            .map(|element| element.geometry(1.0.into()))
            .reduce(|a, b| a.merge(b))
            .expect("fill");
        assert_eq!(union, titlebar.titlebar.to_physical_precise_round(1.0));
        // The top edge is inset on both sides (rounded).
        let top = fill_elements.last().unwrap().geometry(1.0.into());
        assert_eq!(top.loc.y, titlebar.titlebar.loc.y);
        assert!(top.size.w < titlebar.titlebar.size.w);

        // The lights come first (front) so the opaque fill, which is last
        // (back), cannot cull them: an opaque element hides everything that
        // follows it in Smithay's front-to-back list.
        let cluster = titlebar.cluster_rect();
        let diameter = traffic_lights::DIAMETER as i32;
        for light in &elements[..elements.len() - fill] {
            let geo = light.geometry(1.0.into());
            assert_eq!(geo.size.h, 1, "a light span is one physical pixel tall");
            assert!(geo.size.w <= diameter);
            assert!(geo.loc.x >= cluster.loc.x);
            assert!(geo.loc.x + geo.size.w <= cluster.loc.x + cluster.size.w);
            assert!(geo.loc.y >= cluster.loc.y);
            assert!(geo.loc.y < cluster.loc.y + cluster.size.h);
        }
    }

    #[test]
    fn insets_shrink_and_offset_a_rectangle() {
        let insets = WindowInsets::for_tier(DecorationTier::ServerSide);
        let height = DEFAULT_WINDOW_STYLE.height();
        assert_eq!(
            insets.inset(rect(0, 0, 200, 190)),
            rect(0, height, 200, 190 - height)
        );
    }

    #[test]
    fn outset_is_the_inverse_of_inset() {
        let insets = WindowInsets::for_tier(DecorationTier::ServerSide);
        let height = DEFAULT_WINDOW_STYLE.height();
        let content = rect(10, 50, 400, 300);
        let window = insets.outset(content);
        // The whole decorated window grows upward by the titlebar strip.
        assert_eq!(window, rect(10, 50 - height, 400, 300 + height));
        // Round-trips back to the client area.
        assert_eq!(insets.inset(window), content);
    }
}
