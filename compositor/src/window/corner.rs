// SPDX-License-Identifier: MIT
//! Token-driven rounded-corner clipping geometry (T-04.1b).
//!
//! The compositor's flat renderer has no vector rasterizer and no shader, so a
//! rounded rectangle is expressed as a stack of **non-overlapping horizontal
//! spans** — exactly the same trick the traffic lights use in
//! [`decoration`](crate::window::decoration). The radii are read from the
//! generated design tokens (`component.window.radius`,
//! `component.titlebar.cornerRadius`) so a compositor-drawn SSD surface and the
//! first-party QML `AppWindow`/`TitleBar` round from one source (FR-2/FR-3).
//!
//! The mask is the shared shape for the material pass: the SSD titlebar fill,
//! the layered window shadow, and (T-04.2) the backdrop blur all clip to the
//! same token-derived corners. The decomposition is deliberately pure geometry
//! so it is asserted headless, pixel-free, against the tokens.

use smithay::utils::{Logical, Rectangle};

use crate::design_tokens::component;

/// The logical corner radius of a floating window surface
/// (`component.window.radius`).
pub const WINDOW_RADIUS: i32 = component::window::RADIUS as i32;

/// The logical radius of the SSD titlebar's top corners
/// (`component.titlebar.cornerRadius`).
pub const TITLEBAR_RADIUS: i32 = component::titlebar::CORNER_RADIUS as i32;

/// Which corners of a rectangle are rounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoundedCorners {
    /// All four corners (a floating window).
    #[default]
    All,
    /// Only the top two (an SSD titlebar, which joins square to its content).
    Top,
    /// Only the bottom two (the compositor's rounded window bottom edge).
    Bottom,
}

/// A reusable token-derived corner mask: a radius plus the corners it rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerMask {
    pub radius: i32,
    pub corners: RoundedCorners,
}

impl CornerMask {
    /// The mask for a floating window's whole decorated rectangle.
    pub const fn window() -> Self {
        CornerMask {
            radius: WINDOW_RADIUS,
            corners: RoundedCorners::All,
        }
    }

    /// The mask for the SSD titlebar: its top corners follow the window, the
    /// bottom edge stays square so the chrome reads as one sheet with the
    /// content below (matching the QML `TitleBar`).
    pub const fn titlebar() -> Self {
        CornerMask {
            radius: TITLEBAR_RADIUS,
            corners: RoundedCorners::Top,
        }
    }

    /// The non-overlapping horizontal spans that cover `rect` with these
    /// corner radii, in the same logical coordinate space. Returned
    /// top-to-bottom.
    pub fn spans(self, rect: Rectangle<i32, Logical>) -> Vec<Rectangle<i32, Logical>> {
        rounded_rect_spans(rect, self.radius, self.corners)
    }
}

impl Default for CornerMask {
    fn default() -> Self {
        CornerMask::window()
    }
}

/// Decompose the rounded rectangle `rect` into non-overlapping horizontal
/// rectangles (spans) that together cover it exactly. `radius` is clamped to
/// half the shorter side, so a tiny or zero radius degrades to `[rect]` and
/// never produces a zero/negative width.
///
/// The corner bands are sampled like a filled circle: each row's inset is the
/// horizontal gap between the row's chord and the bounding corner, so the
/// silhouette reads round rather than chamfered. The central band between the
/// two corner radii is a single full-width span.
pub fn rounded_rect_spans(
    rect: Rectangle<i32, Logical>,
    radius: i32,
    corners: RoundedCorners,
) -> Vec<Rectangle<i32, Logical>> {
    let w = rect.size.w;
    let h = rect.size.h;
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let max_radius = w.min(h).max(0) / 2;
    let radius = radius.clamp(0, max_radius);
    if radius == 0 {
        return vec![rect];
    }

    let top = matches!(corners, RoundedCorners::All | RoundedCorners::Top);
    let bottom = matches!(corners, RoundedCorners::All | RoundedCorners::Bottom);
    let mut spans = Vec::with_capacity((2 * radius + 1) as usize);

    // Top corner band: row 0 is the top edge (widest inset), the row adjacent
    // to the central band is nearly full width.
    if top {
        for index in 0..radius {
            let inset = corner_inset(radius, index);
            spans.push(span(rect, rect.loc.y + index, inset));
        }
    }

    // Central band: full width, from below the top radius to above the bottom.
    let central_start = rect.loc.y + if top { radius } else { 0 };
    let central_end = rect.loc.y + h - if bottom { radius } else { 0 };
    if central_end > central_start {
        spans.push(Rectangle::new(
            (rect.loc.x, central_start).into(),
            (w, central_end - central_start).into(),
        ));
    }

    // Bottom corner band: mirror of the top, with the widest inset on the
    // bottom edge.
    if bottom {
        for index in 0..radius {
            let inset = corner_inset(radius, radius - 1 - index);
            let y = rect.loc.y + h - radius + index;
            spans.push(span(rect, y, inset));
        }
    }

    spans
}

/// One horizontal span of `rect` at row `y`, inset by `inset` on both sides.
fn span(rect: Rectangle<i32, Logical>, y: i32, inset: i32) -> Rectangle<i32, Logical> {
    let inset = inset.clamp(0, rect.size.w / 2);
    Rectangle::new(
        (rect.loc.x + inset, y).into(),
        ((rect.size.w - 2 * inset).max(1), 1).into(),
    )
}

/// How far a corner row is inset from the edge, for a circle of `radius`.
/// `edge_index` is the row's distance from the rounded edge (`0` = the edge
/// itself), so the row's centre sits `radius - (edge_index + 0.5)` from the
/// corner circle's centre.
fn corner_inset(radius: i32, edge_index: i32) -> i32 {
    let r = radius as f64;
    let dy = r - (edge_index as f64 + 0.5);
    let half = (r * r - dy * dy).max(0.0).sqrt();
    (r - half).round() as i32
}

/// The four square corner regions of `rect`, as `(top_left, top_right,
/// bottom_left, bottom_right)`, each `radius × radius`. This is the raw
/// "corner mask" the headless test asserts against the tokens: the rounded
/// silhouette touches each square only along its arc, and every square's size
/// is exactly the token radius.
pub fn corner_squares(rect: Rectangle<i32, Logical>, radius: i32) -> [Rectangle<i32, Logical>; 4] {
    let max_radius = rect.size.w.min(rect.size.h).max(0) / 2;
    let radius = radius.clamp(0, max_radius);
    [
        Rectangle::new(rect.loc, (radius, radius).into()),
        Rectangle::new(
            (rect.loc.x + rect.size.w - radius, rect.loc.y).into(),
            (radius, radius).into(),
        ),
        Rectangle::new(
            (rect.loc.x, rect.loc.y + rect.size.h - radius).into(),
            (radius, radius).into(),
        ),
        Rectangle::new(
            (
                rect.loc.x + rect.size.w - radius,
                rect.loc.y + rect.size.h - radius,
            )
                .into(),
            (radius, radius).into(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    /// Total area covered by a span set, in logical pixels.
    fn area(spans: &[Rectangle<i32, Logical>]) -> i64 {
        spans
            .iter()
            .map(|span| i64::from(span.size.w) * i64::from(span.size.h))
            .sum()
    }

    #[test]
    fn radii_come_from_the_window_and_titlebar_tokens() {
        assert_eq!(WINDOW_RADIUS, component::window::RADIUS as i32);
        assert_eq!(WINDOW_RADIUS, 14);
        assert_eq!(TITLEBAR_RADIUS, component::titlebar::CORNER_RADIUS as i32);
        assert_eq!(TITLEBAR_RADIUS, 14);
        assert_eq!(CornerMask::window().radius, WINDOW_RADIUS);
        assert_eq!(CornerMask::titlebar().radius, TITLEBAR_RADIUS);
    }

    #[test]
    fn spans_cover_the_rect_exactly_without_gaps_or_overlap() {
        let window = rect(100, 200, 400, 300);
        let spans = CornerMask::window().spans(window);
        assert!(spans.len() > 1, "the corners must be rounded");

        // Every span is inside the rect.
        for span in &spans {
            assert!(
                window.contains_rect(*span),
                "span escapes the rect: {span:?}"
            );
        }
        // The rounded shape is a subset of its bounding box: the corners are
        // clipped, so the area is smaller (but never empty).
        let rounded_area = area(&spans);
        let full_area = i64::from(window.size.w) * i64::from(window.size.h);
        assert!(rounded_area > 0 && rounded_area < full_area);

        // Exactly one span covers each logical row, and it is a horizontal
        // slice of the rect: no gaps and no overlap.
        let mut per_row: Vec<(i32, i32, i32)> = Vec::new();
        for span in &spans {
            for y in span.loc.y..span.loc.y + span.size.h {
                per_row.push((y, span.loc.x, span.loc.x + span.size.w));
            }
        }
        per_row.sort();
        for (index, row) in per_row.iter().enumerate() {
            assert_eq!(row.0, window.loc.y + index as i32, "every row covered once");
            assert!(row.1 >= window.loc.x && row.2 <= window.loc.x + window.size.w);
        }
        assert_eq!(per_row.len(), window.size.h as usize);
    }

    #[test]
    fn the_silhouette_is_round_at_the_corners() {
        let window = rect(0, 0, 400, 300);
        let radius = WINDOW_RADIUS;
        let spans = CornerMask::window().spans(window);

        let top_rows: Vec<_> = spans.iter().filter(|span| span.loc.y < radius).collect();
        assert_eq!(top_rows.len(), radius as usize);
        // The top edge is the narrowest row; widths grow toward the centre.
        let top_edge = top_rows[0];
        assert_eq!(top_edge.loc.y, window.loc.y);
        assert!(
            top_edge.size.w < window.size.w,
            "the top edge must be inset at the corners"
        );
        assert!(
            top_edge.loc.x > window.loc.x,
            "the top edge must start after the corner"
        );
        for pair in top_rows.windows(2) {
            assert!(
                pair[1].size.w >= pair[0].size.w,
                "the corner must widen toward the centre"
            );
        }
        // The last top row is back to (nearly) full width.
        assert_eq!(top_rows.last().unwrap().size.w, window.size.w);

        // Bottom corners mirror the top: the bottom edge is inset.
        let bottom_edge = spans.last().unwrap();
        assert_eq!(bottom_edge.loc.y, window.loc.y + window.size.h - 1);
        assert!(bottom_edge.size.w < window.size.w);
        assert_eq!(bottom_edge.loc.x, top_edge.loc.x);
    }

    #[test]
    fn titlebar_rounds_only_the_top_corners_matching_the_qml_titlebar() {
        let titlebar = rect(0, 40, 400, component::titlebar::HEIGHT as i32);
        let spans = CornerMask::titlebar().spans(titlebar);
        let radius = TITLEBAR_RADIUS;

        // Top edge inset, and no row at or below the top radius is narrowed:
        // the bottom edge stays square, exactly like the QML `TitleBar`'s
        // full-radius rectangle plus square bottom patch.
        let top_edge = spans.first().unwrap();
        assert_eq!(top_edge.loc.y, titlebar.loc.y);
        assert!(top_edge.size.w < titlebar.size.w);
        for span in &spans {
            if span.loc.y >= titlebar.loc.y + radius {
                assert_eq!(span.size.w, titlebar.size.w);
            }
        }
        let rounded_area = area(&spans);
        let full_area = i64::from(titlebar.size.w) * i64::from(titlebar.size.h);
        assert!(rounded_area > 0 && rounded_area < full_area);
    }

    #[test]
    fn corner_squares_are_the_token_radius_squares() {
        let window = rect(10, 20, 400, 300);
        let squares = corner_squares(window, WINDOW_RADIUS);
        let radius = WINDOW_RADIUS;
        assert_eq!(squares[0], rect(10, 20, radius, radius));
        assert_eq!(squares[1], rect(10 + 400 - radius, 20, radius, radius));
        assert_eq!(squares[2], rect(10, 20 + 300 - radius, radius, radius));
        assert_eq!(
            squares[3],
            rect(10 + 400 - radius, 20 + 300 - radius, radius, radius)
        );

        // The mask touches each square only along its arc: the outermost
        // corner pixel is outside every span.
        let spans = CornerMask::window().spans(window);
        let corner_pixel = Point::from((window.loc.x, window.loc.y));
        assert!(!spans.iter().any(|span| span.contains(corner_pixel)));
    }

    #[test]
    fn zero_or_tiny_radius_degrades_to_the_plain_rect() {
        let r = rect(0, 0, 10, 10);
        assert_eq!(rounded_rect_spans(r, 0, RoundedCorners::All), vec![r]);
        // A radius larger than half the side clamps rather than overflowing.
        let spans = rounded_rect_spans(r, 999, RoundedCorners::All);
        assert!(area(&spans) > 0 && area(&spans) <= 100);
        for span in &spans {
            assert!(r.contains_rect(*span));
            assert!(span.size.w >= 1);
        }
    }

    #[test]
    fn a_degenerate_rect_produces_no_spans() {
        assert!(rounded_rect_spans(rect(0, 0, 0, 10), 4, RoundedCorners::All).is_empty());
        assert!(rounded_rect_spans(rect(0, 0, 10, 0), 4, RoundedCorners::All).is_empty());
    }
}
