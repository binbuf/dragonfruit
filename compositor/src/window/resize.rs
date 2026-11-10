// SPDX-License-Identifier: MIT
//! Interactive resize math: edge handling, client constraints, and output
//! clamping (T-04 FR-10).
//!
//! The same pure functions serve SSD edge input (T-13) and CSD clients'
//! `xdg_toplevel` resize requests. Client min/max-size hints clamp the
//! result, and a resized window never leaves its output.

use smithay::input::pointer::CursorIcon;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::utils::{Logical, Point, Rectangle, Size};

use super::decoration::WindowInsets;

/// Width of the interactive resize border around an SSD window, in logical
/// pixels. Matches the shared first-party `WindowResizeGrips` thickness so a
/// compositor-drawn frame and a client-drawn frame feel the same.
pub const RESIZE_BORDER: i32 = crate::design_tokens::component::window::RESIZE_BORDER as i32;

/// Side length of a corner resize handle. Corners are deliberately larger
/// than the edge strip so a diagonal drag is easy to start; the shared QML
/// grips use the same 12 px square.
pub const RESIZE_CORNER: i32 = crate::design_tokens::component::window::RESIZE_CORNER as i32;

/// One of the eight resize edges (the four corners are combinations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    /// Map an `xdg_toplevel.resize` edge; `None` and unknown values are
    /// rejected rather than treated as a resize.
    pub const fn from_xdg(edge: xdg_toplevel::ResizeEdge) -> Option<Self> {
        use xdg_toplevel::ResizeEdge as E;
        match edge {
            E::Top => Some(ResizeEdge::Top),
            E::Bottom => Some(ResizeEdge::Bottom),
            E::Left => Some(ResizeEdge::Left),
            E::Right => Some(ResizeEdge::Right),
            E::TopLeft => Some(ResizeEdge::TopLeft),
            E::TopRight => Some(ResizeEdge::TopRight),
            E::BottomLeft => Some(ResizeEdge::BottomLeft),
            E::BottomRight => Some(ResizeEdge::BottomRight),
            _ => None,
        }
    }

    /// Map an X11 `_NET_WM_MOVERESIZE` edge (T-06).
    pub const fn from_x11(edge: smithay::xwayland::xwm::ResizeEdge) -> Option<Self> {
        use smithay::xwayland::xwm::ResizeEdge as E;
        match edge {
            E::Top => Some(ResizeEdge::Top),
            E::Bottom => Some(ResizeEdge::Bottom),
            E::Left => Some(ResizeEdge::Left),
            E::Right => Some(ResizeEdge::Right),
            E::TopLeft => Some(ResizeEdge::TopLeft),
            E::TopRight => Some(ResizeEdge::TopRight),
            E::BottomLeft => Some(ResizeEdge::BottomLeft),
            E::BottomRight => Some(ResizeEdge::BottomRight),
        }
    }

    /// Whether dragging this edge moves the left side.
    pub const fn moves_left(self) -> bool {
        matches!(
            self,
            ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft
        )
    }

    /// Whether dragging this edge moves the right side.
    pub const fn moves_right(self) -> bool {
        matches!(
            self,
            ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight
        )
    }

    /// Whether dragging this edge moves the top side.
    pub const fn moves_top(self) -> bool {
        matches!(
            self,
            ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight
        )
    }

    /// Whether dragging this edge moves the bottom side.
    pub const fn moves_bottom(self) -> bool {
        matches!(
            self,
            ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight
        )
    }

    /// The pointer cursor that advertises this resize direction.
    pub const fn cursor_icon(self) -> CursorIcon {
        match self {
            ResizeEdge::Top => CursorIcon::NResize,
            ResizeEdge::Bottom => CursorIcon::SResize,
            ResizeEdge::Left => CursorIcon::WResize,
            ResizeEdge::Right => CursorIcon::EResize,
            ResizeEdge::TopLeft => CursorIcon::NwResize,
            ResizeEdge::TopRight => CursorIcon::NeResize,
            ResizeEdge::BottomLeft => CursorIcon::SwResize,
            ResizeEdge::BottomRight => CursorIcon::SeResize,
        }
    }
}

/// Which resize edge (if any) a point falls on inside a decorated window's
/// `frame` (the titlebar and the client area together).
///
/// The edge strip is [`RESIZE_BORDER`] wide and the corner handles are
/// [`RESIZE_CORNER`] square, matching `WindowResizeGrips.qml`. A point in the
/// middle of the frame is not a resize target. Very small windows are handled
/// by choosing the nearest side, so a narrow window still has a left/right
/// half rather than a contradictory both-sides edge.
pub fn resize_edge_at(
    frame: Rectangle<i32, Logical>,
    point: Point<i32, Logical>,
) -> Option<ResizeEdge> {
    if !frame.contains(point) {
        return None;
    }
    let left = point.x - frame.loc.x;
    let right = frame.loc.x + frame.size.w - 1 - point.x;
    let top = point.y - frame.loc.y;
    let bottom = frame.loc.y + frame.size.h - 1 - point.y;

    // Corners are larger than edges; pick the nearest horizontal/vertical
    // side so a tiny window cannot report both left and right.
    let horizontal = if left <= right {
        (left < RESIZE_CORNER).then_some(true)
    } else {
        (right < RESIZE_CORNER).then_some(false)
    };
    let vertical = if top <= bottom {
        (top < RESIZE_CORNER).then_some(true)
    } else {
        (bottom < RESIZE_CORNER).then_some(false)
    };
    if let (Some(left_side), Some(top_side)) = (horizontal, vertical) {
        return Some(match (left_side, top_side) {
            (true, true) => ResizeEdge::TopLeft,
            (true, false) => ResizeEdge::BottomLeft,
            (false, true) => ResizeEdge::TopRight,
            (false, false) => ResizeEdge::BottomRight,
        });
    }

    let horizontal = if left <= right {
        (left < RESIZE_BORDER).then_some(true)
    } else {
        (right < RESIZE_BORDER).then_some(false)
    };
    let vertical = if top <= bottom {
        (top < RESIZE_BORDER).then_some(true)
    } else {
        (bottom < RESIZE_BORDER).then_some(false)
    };
    match (horizontal, vertical) {
        (Some(true), _) => Some(ResizeEdge::Left),
        (Some(false), _) => Some(ResizeEdge::Right),
        (None, Some(true)) => Some(ResizeEdge::Top),
        (None, Some(false)) => Some(ResizeEdge::Bottom),
        (None, None) => None,
    }
}

/// Client size hints. A zero component on an axis means "unconstrained",
/// matching `xdg_surface.set_min_size`/`set_max_size`. `aspect` carries the
/// X11 `WM_NORMAL_HINTS` aspect ratio (T-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SizeConstraints {
    pub min: Size<i32, Logical>,
    pub max: Size<i32, Logical>,
    pub aspect: Option<(u32, u32)>,
}

impl SizeConstraints {
    /// Clamp `size` to the hints, never producing a zero/negative size.
    pub fn clamp(&self, size: Size<i32, Logical>) -> Size<i32, Logical> {
        let clamped = Size::from((
            clamp_axis(size.w, self.min.w, self.max.w),
            clamp_axis(size.h, self.min.h, self.max.h),
        ));
        let with_aspect = apply_aspect(clamped, self.aspect);
        // Aspect adjustment can exceed the max hint; re-clamp so the
        // contract "never larger than max" still holds.
        Size::from((
            clamp_axis(with_aspect.w, self.min.w, self.max.w),
            clamp_axis(with_aspect.h, self.min.h, self.max.h),
        ))
    }
}

fn clamp_axis(value: i32, min: i32, max: i32) -> i32 {
    let mut value = value.max(1);
    if min > 0 {
        value = value.max(min);
    }
    if max > 0 {
        value = value.min(max);
    }
    value
}

/// Apply an optional aspect ratio (X11 `WM_NORMAL_HINTS`), keeping the
/// size at least as large as the requested one on each axis.
pub fn apply_aspect(size: Size<i32, Logical>, aspect: Option<(u32, u32)>) -> Size<i32, Logical> {
    let Some((num, den)) = aspect else {
        return size;
    };
    if num == 0 || den == 0 {
        return size;
    }
    let target_w = size.w.max(1);
    let height_for_width = (target_w as i64 * den as i64 / num as i64).max(1) as i32;
    if height_for_width >= size.h {
        Size::from((target_w, height_for_width))
    } else {
        let width_for_height = (size.h as i64 * num as i64 / den as i64).max(1) as i32;
        Size::from((width_for_height, size.h.max(1)))
    }
}

/// Keep `rect` fully inside `output`, shrinking it if it is larger.
pub fn clamp_within_output(rect: &mut Rectangle<i32, Logical>, output: Rectangle<i32, Logical>) {
    rect.size.w = rect.size.w.min(output.size.w).max(1);
    rect.size.h = rect.size.h.min(output.size.h).max(1);
    if rect.loc.x < output.loc.x {
        rect.loc.x = output.loc.x;
    }
    if rect.loc.y < output.loc.y {
        rect.loc.y = output.loc.y;
    }
    if rect.loc.x + rect.size.w > output.loc.x + output.size.w {
        rect.loc.x = output.loc.x + output.size.w - rect.size.w;
    }
    if rect.loc.y + rect.size.h > output.loc.y + output.size.h {
        rect.loc.y = output.loc.y + output.size.h - rect.size.h;
    }
}

/// Clamp a move so the window never leaves `output`.
pub fn clamp_move(
    location: Point<i32, Logical>,
    window: Size<i32, Logical>,
    output: Rectangle<i32, Logical>,
) -> Point<i32, Logical> {
    let max_x = (output.loc.x + (output.size.w - window.w).max(0)).max(output.loc.x);
    let max_y = (output.loc.y + (output.size.h - window.h).max(0)).max(output.loc.y);
    Point::from((
        location.x.clamp(output.loc.x, max_x),
        location.y.clamp(output.loc.y, max_y),
    ))
}

/// Clamp a move so the *whole decorated frame* stays inside `bounds`.
///
/// `location` and `content_size` describe the client area, while `insets`
/// grow it into the frame the compositor actually draws (an SSD titlebar sits
/// above the content). Passing the usable bounds — the output minus the
/// reserved menu-bar/Dock strips — keeps a dragged window's titlebar out of
/// the chrome, not just its client area.
pub fn clamp_frame_move(
    location: Point<i32, Logical>,
    content_size: Size<i32, Logical>,
    insets: WindowInsets,
    bounds: Rectangle<i32, Logical>,
) -> Point<i32, Logical> {
    let frame = insets.outset(Rectangle::new(location, content_size));
    let frame_location = clamp_move(frame.loc, frame.size, bounds);
    insets.inset(Rectangle::new(frame_location, frame.size)).loc
}

/// How much of a dragged window must remain inside the output. macOS lets a
/// window hang off the left, right, or bottom edge, but keeps a grab strip on
/// screen so it can always be pulled back. The top edge is different: the menu
/// bar reserve is a hard floor.
pub const MIN_VISIBLE_EDGE: i32 = 48;

/// Clamp an interactive move so the window may hang partially off the left,
/// right, or bottom of `bounds`, but never crosses the top menu-bar reserve.
///
/// `location` and `content_size` describe the client area; `insets` grow it
/// into the decorated frame. The frame keeps at least [`MIN_VISIBLE_EDGE`] on
/// screen on the horizontal and bottom edges (or its whole extent when it is
/// smaller), and its top is clamped to `bounds.loc.y`.
pub fn clamp_frame_move_offscreen(
    location: Point<i32, Logical>,
    content_size: Size<i32, Logical>,
    insets: WindowInsets,
    bounds: Rectangle<i32, Logical>,
) -> Point<i32, Logical> {
    let frame = insets.outset(Rectangle::new(location, content_size));
    let visible_x = frame.size.w.max(1).min(MIN_VISIBLE_EDGE);
    let visible_y = frame.size.h.max(1).min(MIN_VISIBLE_EDGE);
    let min_x = bounds.loc.x - frame.size.w + visible_x;
    let max_x = bounds.loc.x + bounds.size.w - visible_x;
    let max_y = (bounds.loc.y + bounds.size.h - visible_y).max(bounds.loc.y);
    let frame_location = Point::from((
        frame.loc.x.clamp(min_x, max_x.max(min_x)),
        frame.loc.y.clamp(bounds.loc.y, max_y),
    ));
    insets.inset(Rectangle::new(frame_location, frame.size)).loc
}

/// Compute the geometry for an interactive resize of `start` along `edge`
/// by `delta`, honoring client constraints and staying inside `output`
/// (the caller passes the interactive content bounds).
pub fn resize_geometry(
    start: Rectangle<i32, Logical>,
    edge: ResizeEdge,
    delta: Point<f64, Logical>,
    constraints: SizeConstraints,
    output: Rectangle<i32, Logical>,
) -> Rectangle<i32, Logical> {
    let dx = delta.x.round() as i32;
    let dy = delta.y.round() as i32;

    let mut x = start.loc.x;
    let mut y = start.loc.y;
    let mut w = start.size.w;
    let mut h = start.size.h;

    if edge.moves_left() {
        x += dx;
        w -= dx;
    } else if edge.moves_right() {
        w += dx;
    }
    if edge.moves_top() {
        y += dy;
        h -= dy;
    } else if edge.moves_bottom() {
        h += dy;
    }

    let clamped = constraints.clamp(Size::from((w.max(1), h.max(1))));

    // Re-anchor the side opposite the dragged one after clamping.
    if edge.moves_left() {
        x = start.loc.x + start.size.w - clamped.w;
    }
    if edge.moves_top() {
        y = start.loc.y + start.size.h - clamped.h;
    }

    let mut rect = Rectangle::new(Point::from((x, y)), clamped);
    clamp_within_output(&mut rect, output);
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn output() -> Rectangle<i32, Logical> {
        rect(0, 0, 1920, 1080)
    }

    fn no_constraints() -> SizeConstraints {
        SizeConstraints::default()
    }

    #[test]
    fn resize_edge_at_maps_edges_and_corners() {
        let frame = rect(100, 100, 400, 300);
        // Edge strips.
        assert_eq!(
            resize_edge_at(frame, Point::from((300, 102))),
            Some(ResizeEdge::Top)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((300, 397))),
            Some(ResizeEdge::Bottom)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((102, 250))),
            Some(ResizeEdge::Left)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((497, 250))),
            Some(ResizeEdge::Right)
        );
        // Corner handles are larger than the edge strips.
        assert_eq!(
            resize_edge_at(frame, Point::from((105, 105))),
            Some(ResizeEdge::TopLeft)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((495, 105))),
            Some(ResizeEdge::TopRight)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((105, 395))),
            Some(ResizeEdge::BottomLeft)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((495, 395))),
            Some(ResizeEdge::BottomRight)
        );
        // The middle of the frame is content, not a resize target.
        assert_eq!(resize_edge_at(frame, Point::from((300, 250))), None);
        // Outside the frame.
        assert_eq!(resize_edge_at(frame, Point::from((99, 250))), None);
        assert_eq!(resize_edge_at(frame, Point::from((300, 400))), None);
    }

    #[test]
    fn resize_edge_at_handles_windows_narrower_than_the_corner() {
        let frame = rect(0, 0, 10, 10);
        // Every point is within a corner; the nearest half wins instead of
        // producing an impossible left+right combination.
        assert_eq!(
            resize_edge_at(frame, Point::from((1, 1))),
            Some(ResizeEdge::TopLeft)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((8, 1))),
            Some(ResizeEdge::TopRight)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((1, 8))),
            Some(ResizeEdge::BottomLeft)
        );
        assert_eq!(
            resize_edge_at(frame, Point::from((8, 8))),
            Some(ResizeEdge::BottomRight)
        );
    }

    #[test]
    fn resize_cursors_match_the_edge_direction() {
        assert_eq!(ResizeEdge::Top.cursor_icon(), CursorIcon::NResize);
        assert_eq!(ResizeEdge::Bottom.cursor_icon(), CursorIcon::SResize);
        assert_eq!(ResizeEdge::Left.cursor_icon(), CursorIcon::WResize);
        assert_eq!(ResizeEdge::Right.cursor_icon(), CursorIcon::EResize);
        assert_eq!(ResizeEdge::TopLeft.cursor_icon(), CursorIcon::NwResize);
        assert_eq!(ResizeEdge::TopRight.cursor_icon(), CursorIcon::NeResize);
        assert_eq!(ResizeEdge::BottomLeft.cursor_icon(), CursorIcon::SwResize);
        assert_eq!(ResizeEdge::BottomRight.cursor_icon(), CursorIcon::SeResize);
    }

    #[test]
    fn right_edge_grows_width_only() {
        let start = rect(100, 100, 400, 300);
        let resized = resize_geometry(
            start,
            ResizeEdge::Right,
            (60.0, 40.0).into(),
            no_constraints(),
            output(),
        );
        assert_eq!(resized, rect(100, 100, 460, 300));
    }

    #[test]
    fn left_edge_keeps_the_right_side_fixed() {
        let start = rect(100, 100, 400, 300);
        let resized = resize_geometry(
            start,
            ResizeEdge::Left,
            (-50.0, 0.0).into(),
            no_constraints(),
            output(),
        );
        assert_eq!(resized.loc.x + resized.size.w, start.loc.x + start.size.w);
        assert_eq!(resized.size.w, 450);
    }

    #[test]
    fn top_left_corner_keeps_the_bottom_right_fixed() {
        let start = rect(100, 100, 400, 300);
        let resized = resize_geometry(
            start,
            ResizeEdge::TopLeft,
            (-40.0, -30.0).into(),
            no_constraints(),
            output(),
        );
        assert_eq!(resized.loc.x + resized.size.w, start.loc.x + start.size.w);
        assert_eq!(resized.loc.y + resized.size.h, start.loc.y + start.size.h);
        assert_eq!(resized.size, Size::from((440, 330)));
    }

    #[test]
    fn min_size_clamps_the_resize() {
        let start = rect(100, 100, 400, 300);
        let constraints = SizeConstraints {
            min: Size::from((200, 150)),
            max: Size::from((0, 0)),
            aspect: None,
        };
        let resized = resize_geometry(
            start,
            ResizeEdge::BottomRight,
            (-350.0, -250.0).into(),
            constraints,
            output(),
        );
        assert_eq!(resized.size, Size::from((200, 150)));
    }

    #[test]
    fn max_size_clamps_the_resize() {
        let start = rect(100, 100, 400, 300);
        let constraints = SizeConstraints {
            min: Size::from((0, 0)),
            max: Size::from((500, 400)),
            aspect: None,
        };
        let resized = resize_geometry(
            start,
            ResizeEdge::BottomRight,
            (900.0, 900.0).into(),
            constraints,
            output(),
        );
        assert_eq!(resized.size, Size::from((500, 400)));
    }

    #[test]
    fn a_resize_never_leaves_the_output() {
        let start = rect(1500, 900, 400, 160);
        let resized = resize_geometry(
            start,
            ResizeEdge::BottomRight,
            (900.0, 900.0).into(),
            no_constraints(),
            output(),
        );
        assert!(resized.loc.x + resized.size.w <= output().size.w);
        assert!(resized.loc.y + resized.size.h <= output().size.h);
    }

    #[test]
    fn move_is_clamped_to_the_output() {
        let window = Size::from((400, 300));
        let clamped = clamp_move(Point::from((1800, 1000)), window, output());
        assert_eq!(clamped, Point::from((1520, 780)));
        let clamped = clamp_move(Point::from((-50, -50)), window, output());
        assert_eq!(clamped, Point::from((0, 0)));
    }

    /// Placement and Zoom subtract both the menu bar (top) and the Dock
    /// (bottom) reserves.
    fn usable() -> Rectangle<i32, Logical> {
        rect(0, 28, 1920, 1080 - 28 - 60)
    }

    /// Interactive move/resize subtracts only the menu bar; the Dock's
    /// reserve remains part of the draggable area.
    fn interactive() -> Rectangle<i32, Logical> {
        rect(0, 28, 1920, 1080 - 28)
    }

    #[test]
    fn move_is_clamped_to_the_given_bounds() {
        let window = Size::from((400, 300));
        let clamped = clamp_move(Point::from((100, -50)), window, usable());
        assert_eq!(clamped, Point::from((100, 28)));
        let clamped = clamp_move(Point::from((100, 2000)), window, usable());
        assert_eq!(clamped.y + window.h, usable().loc.y + usable().size.h);
    }

    #[test]
    fn decorated_frame_move_keeps_the_titlebar_out_of_reserved_chrome() {
        let window = Size::from((400, 300));
        let insets = WindowInsets {
            top: 28,
            ..WindowInsets::NONE
        };
        // The content would sit exactly below the menu bar, but the titlebar
        // is drawn above it: the whole frame must stay below the menu bar.
        let clamped = clamp_frame_move(Point::from((100, 28)), window, insets, interactive());
        assert_eq!(clamped, Point::from((100, 56)));
        // A client-side-decorated window reserves no titlebar inset.
        let clamped = clamp_frame_move(
            Point::from((100, 28)),
            window,
            WindowInsets::NONE,
            interactive(),
        );
        assert_eq!(clamped, Point::from((100, 28)));
        // The Dock does not block a move: the frame may sit over it.
        let clamped = clamp_frame_move(
            Point::from((100, 2000)),
            window,
            WindowInsets::NONE,
            interactive(),
        );
        assert_eq!(clamped, Point::from((100, 780)));
    }

    #[test]
    fn offscreen_move_keeps_the_menu_bar_clear_and_a_grab_strip_visible() {
        let window = Size::from((400, 300));
        // Up is the one hard stop: the frame top never crosses the menu bar.
        let clamped = clamp_frame_move_offscreen(
            Point::from((100, -500)),
            window,
            WindowInsets::NONE,
            interactive(),
        );
        assert_eq!(clamped.y, interactive().loc.y);
        // The titlebar inset is part of the frame, so the content sits below
        // the bar by the same amount.
        let insets = WindowInsets {
            top: 28,
            ..WindowInsets::NONE
        };
        let clamped =
            clamp_frame_move_offscreen(Point::from((100, -500)), window, insets, interactive());
        assert_eq!(clamped.y, interactive().loc.y + insets.top);

        // Left/right/bottom may hang off the output, keeping 48 px on screen.
        let clamped = clamp_frame_move_offscreen(
            Point::from((-500, 100)),
            window,
            WindowInsets::NONE,
            interactive(),
        );
        assert_eq!(clamped.x, -window.w + MIN_VISIBLE_EDGE);
        let clamped = clamp_frame_move_offscreen(
            Point::from((5000, 100)),
            window,
            WindowInsets::NONE,
            interactive(),
        );
        assert_eq!(
            clamped.x,
            interactive().loc.x + interactive().size.w - MIN_VISIBLE_EDGE
        );
        let clamped = clamp_frame_move_offscreen(
            Point::from((100, 5000)),
            window,
            WindowInsets::NONE,
            interactive(),
        );
        assert_eq!(
            clamped.y,
            interactive().loc.y + interactive().size.h - MIN_VISIBLE_EDGE
        );

        // A window that still fits is not moved.
        let clamped = clamp_frame_move_offscreen(
            Point::from((100, 100)),
            window,
            WindowInsets::NONE,
            interactive(),
        );
        assert_eq!(clamped, Point::from((100, 100)));
    }

    #[test]
    fn resize_stops_at_the_menu_bar_but_may_overlap_the_dock() {
        let start = rect(100, 200, 400, 300);
        let insets = WindowInsets {
            top: 28,
            ..WindowInsets::NONE
        };
        // `resize_geometry` works on the client area; the caller passes the
        // content bounds inside the interactive bounds (insets removed).
        let bounds = insets.inset(interactive());
        // Dragging the top edge up stops below the reserved menu-bar strip
        // and leaves room for the titlebar drawn above the content.
        let resized = resize_geometry(
            start,
            ResizeEdge::Top,
            (0.0, -500.0).into(),
            no_constraints(),
            bounds,
        );
        assert_eq!(resized.loc.y, interactive().loc.y + insets.top);
        // Dragging the bottom edge down can extend over the Dock, but not
        // past the output.
        let resized = resize_geometry(
            start,
            ResizeEdge::Bottom,
            (0.0, 2000.0).into(),
            no_constraints(),
            bounds,
        );
        assert_eq!(
            resized.loc.y + resized.size.h,
            interactive().loc.y + interactive().size.h
        );
        assert!(resized.loc.y + resized.size.h > usable().loc.y + usable().size.h);
    }

    #[test]
    fn aspect_ratio_is_preserved() {
        let sized = apply_aspect(Size::from((400, 100)), Some((4, 3)));
        assert_eq!(sized, Size::from((400, 300)));
        let sized = apply_aspect(Size::from((100, 400)), Some((4, 3)));
        assert_eq!(sized, Size::from((533, 400)));
    }

    #[test]
    fn xdg_edges_map_and_none_is_rejected() {
        use xdg_toplevel::ResizeEdge as E;
        assert_eq!(ResizeEdge::from_xdg(E::TopLeft), Some(ResizeEdge::TopLeft));
        assert_eq!(ResizeEdge::from_xdg(E::Bottom), Some(ResizeEdge::Bottom));
        assert_eq!(ResizeEdge::from_xdg(E::None), None);
    }

    /// Property: for any start/delta/constraints, the result respects the
    /// constraints and stays within the output.
    #[test]
    fn resized_geometry_always_respects_constraints_and_output() {
        let mut seed = 0xdead_beef_1234_5678u64;
        let mut rng = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as i32
        };
        let edges = [
            ResizeEdge::Top,
            ResizeEdge::Bottom,
            ResizeEdge::Left,
            ResizeEdge::Right,
            ResizeEdge::TopLeft,
            ResizeEdge::TopRight,
            ResizeEdge::BottomLeft,
            ResizeEdge::BottomRight,
        ];
        for _ in 0..2000 {
            let start = rect(
                rng().rem_euclid(1600),
                rng().rem_euclid(900),
                rng().rem_euclid(700) + 1,
                rng().rem_euclid(500) + 1,
            );
            let delta = Point::from((
                f64::from(rng().rem_euclid(1200) - 600),
                f64::from(rng().rem_euclid(1200) - 600),
            ));
            let constraints = SizeConstraints {
                min: Size::from((rng().rem_euclid(200), rng().rem_euclid(200))),
                max: Size::from((rng().rem_euclid(900) + 200, rng().rem_euclid(700) + 200)),
                aspect: None,
            };
            let edge = edges[(rng().unsigned_abs() as usize) % edges.len()];
            let resized = resize_geometry(start, edge, delta, constraints, output());
            assert!(resized.size.w >= 1 && resized.size.h >= 1);
            assert!(resized.size.w <= constraints.max.w);
            assert!(resized.size.h <= constraints.max.h);
            if constraints.min.w > 0 {
                assert!(resized.size.w >= constraints.min.w);
            }
            if constraints.min.h > 0 {
                assert!(resized.size.h >= constraints.min.h);
            }
            assert!(resized.loc.x >= output().loc.x);
            assert!(resized.loc.y >= output().loc.y);
            assert!(resized.loc.x + resized.size.w <= output().size.w);
            assert!(resized.loc.y + resized.size.h <= output().size.h);
        }
    }
}
