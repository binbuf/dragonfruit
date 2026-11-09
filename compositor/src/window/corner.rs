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

/// Per-corner radii for a material region (T197).
///
/// A menu attached to the menu bar keeps square top corners while its bottom
/// corners follow the popup radius; a detached card rounds all four. The
/// renderer clips with these radii, so the shape comes from the region
/// metadata rather than a single "all corners" literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CornerRadii {
    pub top_left: i32,
    pub top_right: i32,
    pub bottom_right: i32,
    pub bottom_left: i32,
}

impl CornerRadii {
    /// All four corners at the same radius.
    pub const fn uniform(radius: i32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// The radii for a [`CornerMask`]'s radius/mask pair.
    pub const fn from_mask(radius: i32, corners: RoundedCorners) -> Self {
        match corners {
            RoundedCorners::All => Self::uniform(radius),
            RoundedCorners::Top => Self {
                top_left: radius,
                top_right: radius,
                bottom_right: 0,
                bottom_left: 0,
            },
            RoundedCorners::Bottom => Self {
                top_left: 0,
                top_right: 0,
                bottom_right: radius,
                bottom_left: radius,
            },
        }
    }

    /// Clamp every radius to `max` and to non-negative values.
    pub fn clamped(self, max: i32) -> Self {
        let max = max.max(0);
        Self {
            top_left: self.top_left.clamp(0, max),
            top_right: self.top_right.clamp(0, max),
            bottom_right: self.bottom_right.clamp(0, max),
            bottom_left: self.bottom_left.clamp(0, max),
        }
    }

    /// The largest radius; used where a single radius is still required
    /// (the liquid-glass shader's SDF, the fallback feather).
    pub const fn max(self) -> i32 {
        let top = if self.top_left > self.top_right {
            self.top_left
        } else {
            self.top_right
        };
        let bottom = if self.bottom_right > self.bottom_left {
            self.bottom_right
        } else {
            self.bottom_left
        };
        if top > bottom {
            top
        } else {
            bottom
        }
    }

    /// Whether every corner is square.
    pub const fn is_square(self) -> bool {
        self.top_left == 0 && self.top_right == 0 && self.bottom_right == 0 && self.bottom_left == 0
    }
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
    rounded_rect_spans_with_radii(rect, CornerRadii::from_mask(radius, corners))
}

/// Decompose a rounded rectangle with independent per-corner radii into
/// non-overlapping horizontal spans (T197).
///
/// The row bands are as tall as the largest radius on each edge. A corner with
/// radius zero contributes no inset on its side, so an attached menu (square
/// top, rounded bottom) is a full-width strip at the top and a rounded card at
/// the bottom. Every radius is clamped to half the shorter side; the result
/// still covers exactly the same rectangle without gaps or overlap.
pub fn rounded_rect_spans_with_radii(
    rect: Rectangle<i32, Logical>,
    radii: CornerRadii,
) -> Vec<Rectangle<i32, Logical>> {
    let w = rect.size.w;
    let h = rect.size.h;
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let max_radius = w.min(h).max(0) / 2;
    let radii = radii.clamped(max_radius);
    if radii.is_square() {
        return vec![rect];
    }

    let top_band = radii.top_left.max(radii.top_right);
    let bottom_band = radii.bottom_left.max(radii.bottom_right);
    let mut spans = Vec::with_capacity((top_band + bottom_band + 1).max(1) as usize);

    // Top corner band: row 0 is the top edge (widest inset), the row adjacent
    // to the central band is nearly full width. Each side follows its own
    // radius; a square corner never insets.
    for index in 0..top_band {
        let left = if index < radii.top_left {
            corner_inset(radii.top_left, index)
        } else {
            0
        };
        let right = if index < radii.top_right {
            corner_inset(radii.top_right, index)
        } else {
            0
        };
        spans.push(span(rect, rect.loc.y + index, left, right));
    }

    // Central band: full width, from below the top band to above the bottom.
    let central_start = rect.loc.y + top_band;
    let central_end = rect.loc.y + h - bottom_band;
    if central_end > central_start {
        spans.push(Rectangle::new(
            (rect.loc.x, central_start).into(),
            (w, central_end - central_start).into(),
        ));
    }

    // Bottom corner band: mirror of the top, with the widest inset on the
    // bottom edge.
    for index in 0..bottom_band {
        let row = bottom_band - 1 - index;
        let left = if row < radii.bottom_left {
            corner_inset(radii.bottom_left, row)
        } else {
            0
        };
        let right = if row < radii.bottom_right {
            corner_inset(radii.bottom_right, row)
        } else {
            0
        };
        let y = rect.loc.y + h - bottom_band + index;
        spans.push(span(rect, y, left, right));
    }

    spans
}

/// One horizontal span of `rect` at row `y`, inset independently on each side.
fn span(rect: Rectangle<i32, Logical>, y: i32, left: i32, right: i32) -> Rectangle<i32, Logical> {
    let left = left.clamp(0, rect.size.w / 2);
    let right = right.clamp(0, rect.size.w / 2);
    let width = (rect.size.w - left - right).max(1);
    Rectangle::new((rect.loc.x + left, y).into(), (width, 1).into())
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
        let titlebar = rect(0, 40, 400, component::window_style::titlebar::HEIGHT as i32);
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

    #[test]
    fn per_corner_radii_keep_attached_corners_square() {
        // A menu attached to the menu bar: square top, rounded bottom.
        let attached = rect(10, 20, 200, 80);
        let spans = rounded_rect_spans_with_radii(
            attached,
            CornerRadii {
                top_left: 0,
                top_right: 0,
                bottom_right: 12,
                bottom_left: 12,
            },
        );
        assert_eq!(spans.first().unwrap().loc.y, attached.loc.y);
        assert_eq!(
            spans.first().unwrap().size.w,
            attached.size.w,
            "the top edge stays square"
        );
        let bottom = spans.last().unwrap();
        assert_eq!(bottom.loc.y, attached.loc.y + attached.size.h - 1);
        assert!(bottom.size.w < attached.size.w, "the bottom edge rounds");
        assert!(area(&spans) < i64::from(attached.size.w) * i64::from(attached.size.h));

        // Detached card: all four corners round, and each corner follows its
        // own radius. The left top corner is visibly narrower than the right
        // one on the first row.
        let detached = rect(10, 20, 200, 80);
        let spans = rounded_rect_spans_with_radii(
            detached,
            CornerRadii {
                top_left: 20,
                top_right: 4,
                bottom_right: 4,
                bottom_left: 20,
            },
        );
        let top = spans.first().unwrap();
        assert!(top.loc.x > detached.loc.x, "left top corner is rounded");
        assert!(
            top.loc.x + top.size.w < detached.loc.x + detached.size.w,
            "right top corner is rounded"
        );
        assert!(top.size.w < detached.size.w);
        for span in &spans {
            assert!(detached.contains_rect(*span));
        }
    }

    #[test]
    fn uniform_per_corner_radii_match_the_shared_mask() {
        let r = rect(3, 7, 201, 83);
        for radius in [0, 1, 4, 12, 40, 999] {
            assert_eq!(
                rounded_rect_spans_with_radii(r, CornerRadii::uniform(radius)),
                rounded_rect_spans(r, radius, RoundedCorners::All),
                "radius {radius}"
            );
        }
        let mask = CornerMask::titlebar();
        assert_eq!(
            rounded_rect_spans_with_radii(r, CornerRadii::from_mask(mask.radius, mask.corners)),
            rounded_rect_spans(r, mask.radius, mask.corners)
        );
    }
}
