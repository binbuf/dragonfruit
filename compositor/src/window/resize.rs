// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interactive resize math: edge handling, client constraints, and output
//! clamping (T-04 FR-10).
//!
//! The same pure functions serve SSD edge input (T-13) and CSD clients'
//! `xdg_toplevel` resize requests. Client min/max-size hints clamp the
//! result, and a resized window never leaves its output.

use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::utils::{Logical, Point, Rectangle, Size};

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
}

/// Client size hints. A zero component on an axis means "unconstrained",
/// matching `xdg_surface.set_min_size`/`set_max_size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SizeConstraints {
    pub min: Size<i32, Logical>,
    pub max: Size<i32, Logical>,
}

impl SizeConstraints {
    /// Clamp `size` to the hints, never producing a zero/negative size.
    pub fn clamp(&self, size: Size<i32, Logical>) -> Size<i32, Logical> {
        Size::from((
            clamp_axis(size.w, self.min.w, self.max.w),
            clamp_axis(size.h, self.min.h, self.max.h),
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

/// Compute the geometry for an interactive resize of `start` along `edge`
/// by `delta`, honoring client constraints and staying inside `output`.
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
