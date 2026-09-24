// SPDX-License-Identifier: MIT
//! Mission Control live-surface grid layout (T-05.1a).
//!
//! This is the documented Mission Control layout algorithm
//! ([03-workspaces.md](../../docs/design/03-workspaces.md#window-layout-and-occlusion-t-11-u-5)):
//! the real window surfaces are laid out as a **grid**, never a cascade and
//! never a thumbnail. The module is pure geometry — no Smithay scene, no GPU —
//! so the shape, membership order, uniform scale, and cell centering are
//! unit-testable headless.
//!
//! One [`GridLayout`] is computed for the windows of an output's active Space
//! (later slices reveal neighbouring Spaces as extra candidates, ordered by
//! their strip index); each window gets a [`GridPlacement`] mapping its
//! committed `source` rectangle onto a centered `target` inside its `cell`
//! with the grid's one uniform `scale`. The renderer turns the placement into
//! the reusable T-04 [`SceneTransform`](crate::window::SceneTransform)/
//! [`MotionFrame`] — there is no grid-specific transform.
//!
//! Membership excludes minimized windows (they live in the bottom strip) and
//! fullscreen windows (they own a dedicated Space card); ordering is active
//! Space first, then strip order, most-recently-used first within a Space, so
//! a window never swaps cells mid-gesture.

use smithay::utils::{Logical, Rectangle, Scale};

use crate::design_tokens::component::overview;
use crate::window::motion::MotionFrame;
use crate::window::WindowId;

/// The layout margin between a grid cell's edge and the window scaled into it,
/// from the design tokens (`component.overview.stripMargin`); the largest
/// window must fit its cell minus this margin.
pub const GRID_MARGIN: i32 = overview::STRIP_MARGIN as i32;

/// The readable-scale floor. `grid_layout` shrinks past it when there are many
/// windows; paging at the floor is a documented follow-up (the active page
/// contains the active window). Exposed so the future pager shares the number.
pub const MIN_GRID_SCALE: f64 = 0.1;

/// One window offered to the grid. `space_index` is the window's position in
/// the strip order (0 = active Space); `rank` is its recency rank *within* its
/// Space (0 = most recently used). `geometry` is the committed logical rect
/// (the transform's source).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridCandidate {
    pub window: WindowId,
    pub space_index: usize,
    pub rank: usize,
    pub geometry: Rectangle<i32, Logical>,
}

/// One window's place in the grid: its `cell` (the slot), the `target` rect it
/// is scaled+centered into, and the grid's shared uniform `scale`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPlacement {
    pub window: WindowId,
    /// The committed window rect the transform maps from.
    pub source: Rectangle<i32, Logical>,
    /// The grid slot, in the same logical space as the layout `area`.
    pub cell: Rectangle<i32, Logical>,
    /// The scaled, centered window rect, in the layout area's logical space.
    pub target: Rectangle<i32, Logical>,
    /// The uniform grid scale (target size over source size).
    pub scale: f64,
}

impl GridPlacement {
    /// The per-frame render transform at `progress` (`0.0` = the normal scene,
    /// `1.0` = fully in the grid). The rect lerps from the committed geometry
    /// to the grid target, so opening/closing the overview is continuous and
    /// reversible; alpha stays `1.0` (Mission Control never fades windows).
    pub fn frame(&self, progress: f64) -> MotionFrame {
        let t = progress.clamp(0.0, 1.0);
        let rect = lerp_rect(self.source, self.target, t);
        let scale = Scale::from((
            f64::from(rect.size.w) / f64::from(self.source.size.w.max(1)),
            f64::from(rect.size.h) / f64::from(self.source.size.h.max(1)),
        ));
        MotionFrame {
            rect,
            scale,
            offset: (
                rect.loc.x - self.source.loc.x,
                rect.loc.y - self.source.loc.y,
            )
                .into(),
            alpha: 1.0,
        }
    }
}

/// The resolved grid for one output.
#[derive(Debug, Clone, PartialEq)]
pub struct GridLayout {
    /// The area the grid fills (output geometry).
    pub area: Rectangle<i32, Logical>,
    pub columns: usize,
    pub rows: usize,
    /// The one uniform scale every window shares.
    pub scale: f64,
    /// Placements in membership order (active Space, then strip order,
    /// most-recently-used first within a Space).
    pub placements: Vec<GridPlacement>,
}

impl GridLayout {
    pub fn placement(&self, window: WindowId) -> Option<&GridPlacement> {
        self.placements
            .iter()
            .find(|placement| placement.window == window)
    }

    pub fn len(&self) -> usize {
        self.placements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }
}

/// Lay `candidates` out as a grid filling `area`.
///
/// * **Grid shape.** Columns start at `ceil(sqrt(n))` and are adjusted so the
///   grid arrangement's aspect ratio (`columns / rows`) is closest to the
///   area's; rows fill left-to-right, top-to-bottom.
/// * **Scaling.** One uniform scale for every window: the largest window (in
///   logical size) fits its cell minus [`GRID_MARGIN`].
/// * **Centering.** Every window is centered in its cell.
/// * **No occlusion.** Cells do not overlap by construction.
pub fn grid_layout(area: Rectangle<i32, Logical>, candidates: &[GridCandidate]) -> GridLayout {
    let mut ordered: Vec<&GridCandidate> = candidates.iter().collect();
    // Active Space first, then strip order; most-recently-used first within a
    // Space. Stable so a window does not swap cells mid-transition.
    ordered.sort_by_key(|candidate| (candidate.space_index, candidate.rank));

    let n = ordered.len();
    if n == 0 || area.size.w <= 0 || area.size.h <= 0 {
        return GridLayout {
            area,
            columns: 0,
            rows: 0,
            scale: 0.0,
            placements: Vec::new(),
        };
    }

    let columns = best_columns(n, area);
    let rows = n.div_ceil(columns);
    let cell_w = (area.size.w / columns as i32).max(1);
    let cell_h = (area.size.h / rows as i32).max(1);
    // The largest window must fit its cell minus the layout margin on each
    // side; all windows then share that scale.
    let content_w = (cell_w - 2 * GRID_MARGIN).max(1);
    let content_h = (cell_h - 2 * GRID_MARGIN).max(1);
    let scale = ordered
        .iter()
        .map(|candidate| {
            let size = candidate.geometry.size;
            (f64::from(content_w) / f64::from(size.w.max(1)))
                .min(f64::from(content_h) / f64::from(size.h.max(1)))
        })
        .fold(f64::INFINITY, f64::min);
    let scale = if scale.is_finite() { scale } else { 0.0 };

    let mut placements = Vec::with_capacity(n);
    for (index, candidate) in ordered.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let cell = Rectangle::new(
            (
                area.loc.x + column as i32 * cell_w,
                area.loc.y + row as i32 * cell_h,
            )
                .into(),
            (cell_w, cell_h).into(),
        );
        let target = fitted(candidate.geometry, cell, scale);
        placements.push(GridPlacement {
            window: candidate.window,
            source: candidate.geometry,
            cell,
            target,
            scale,
        });
    }

    GridLayout {
        area,
        columns,
        rows,
        scale,
        placements,
    }
}

/// The window rect scaled by the uniform `scale` and centered in `cell`.
fn fitted(
    source: Rectangle<i32, Logical>,
    cell: Rectangle<i32, Logical>,
    scale: f64,
) -> Rectangle<i32, Logical> {
    let w = ((f64::from(source.size.w) * scale).round() as i32).max(1);
    let h = ((f64::from(source.size.h) * scale).round() as i32).max(1);
    Rectangle::new(
        (
            cell.loc.x + (cell.size.w - w) / 2,
            cell.loc.y + (cell.size.h - h) / 2,
        )
            .into(),
        (w, h).into(),
    )
}

/// The column count whose arrangement aspect (`columns / rows`) is closest to
/// the area's, starting from `ceil(sqrt(n))` and preferring it on a tie.
fn best_columns(n: usize, area: Rectangle<i32, Logical>) -> usize {
    let area_aspect = f64::from(area.size.w) / f64::from(area.size.h.max(1));
    let root = (n as f64).sqrt();
    let floor = (root.floor() as usize).max(1);
    let ceil = (root.ceil() as usize).max(1);
    // The documented start is `ceil(sqrt(n))`; the only adjustment is between
    // the two counts that bracket `sqrt(n)`, so the grid stays near-square
    // (never trading a full row for an empty one) while still leaning toward
    // the workspace's aspect ratio.
    let mut best = ceil;
    let mut best_score = f64::INFINITY;
    for columns in [floor, ceil] {
        let rows = n.div_ceil(columns);
        let arrangement_aspect = columns as f64 / rows as f64;
        let mut score = (arrangement_aspect / area_aspect).ln().abs();
        if columns == ceil {
            // A hair of preference keeps `ceil(sqrt(n))` on a tie.
            score -= 1e-9;
        }
        if score < best_score {
            best_score = score;
            best = columns;
        }
    }
    best
}

/// Linear interpolation between two rectangles at `t`.
fn lerp_rect(
    from: Rectangle<i32, Logical>,
    to: Rectangle<i32, Logical>,
    t: f64,
) -> Rectangle<i32, Logical> {
    let lerp = |a: i32, b: i32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as i32;
    Rectangle::new(
        (lerp(from.loc.x, to.loc.x), lerp(from.loc.y, to.loc.y)).into(),
        (
            lerp(from.size.w, to.size.w).max(1),
            lerp(from.size.h, to.size.h).max(1),
        )
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::Point;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), (w, h).into())
    }

    fn candidate(window: u64, rank: usize, x: i32, y: i32, w: i32, h: i32) -> GridCandidate {
        GridCandidate {
            window: WindowId(window),
            space_index: 0,
            rank,
            geometry: rect(x, y, w, h),
        }
    }

    #[test]
    fn empty_candidates_produce_an_empty_grid() {
        let layout = grid_layout(rect(0, 0, 1280, 720), &[]);
        assert!(layout.is_empty());
        assert_eq!(layout.columns, 0);
        assert_eq!(layout.rows, 0);
        assert_eq!(layout.scale, 0.0);
    }

    #[test]
    fn one_window_fills_its_cell_centered() {
        let window = candidate(1, 0, 100, 100, 400, 300);
        let layout = grid_layout(rect(0, 0, 1200, 800), &[window]);
        assert_eq!((layout.columns, layout.rows), (1, 1));
        let placement = layout.placement(WindowId(1)).unwrap();
        assert_eq!(placement.cell, rect(0, 0, 1200, 800));
        // The window is centered in the cell.
        assert_eq!(
            (
                placement.target.loc.x + placement.target.size.w / 2,
                placement.target.loc.y + placement.target.size.h / 2,
            ),
            (600, 400)
        );
        // The largest window fits its cell minus the margin on both axes.
        assert!(placement.target.size.w <= 1200 - 2 * GRID_MARGIN);
        assert!(placement.target.size.h <= 800 - 2 * GRID_MARGIN);
    }

    #[test]
    fn four_windows_form_a_two_by_two_grid_with_one_scale() {
        let candidates = vec![
            candidate(1, 0, 0, 0, 400, 300),
            candidate(2, 1, 100, 100, 200, 150),
            candidate(3, 2, 200, 200, 800, 600),
            candidate(4, 3, 300, 300, 320, 240),
        ];
        let layout = grid_layout(rect(0, 0, 1200, 800), &candidates);
        assert_eq!((layout.columns, layout.rows), (2, 2));
        assert_eq!(layout.len(), 4);
        // The largest window (800x600) fits its 600x400 cell minus the margin
        // on the limiting (vertical) axis.
        let content_h = (800 / 2) as f64 - 2.0 * GRID_MARGIN as f64;
        assert!((layout.scale - content_h / 600.0).abs() < 1e-9);
        // Every placement shares the one scale and is centered in its cell.
        for placement in &layout.placements {
            assert!((placement.scale - layout.scale).abs() < 1e-9);
            let (cx, cy) = (
                placement.cell.loc.x + placement.cell.size.w / 2,
                placement.cell.loc.y + placement.cell.size.h / 2,
            );
            assert!(
                (placement.target.loc.x + placement.target.size.w / 2 - cx).abs() <= 1,
                "centered horizontally: {placement:?}"
            );
            assert!(
                (placement.target.loc.y + placement.target.size.h / 2 - cy).abs() <= 1,
                "centered vertically: {placement:?}"
            );
            // Cells never overlap (no occlusion by construction).
        }
        for (i, a) in layout.placements.iter().enumerate() {
            for b in layout.placements.iter().skip(i + 1) {
                assert!(
                    !a.cell.overlaps(b.cell),
                    "cells must not overlap: {a:?} / {b:?}"
                );
            }
        }
    }

    #[test]
    fn membership_is_ordered_by_space_then_recency() {
        let mut candidates = vec![
            GridCandidate {
                space_index: 1,
                ..candidate(10, 0, 0, 0, 100, 100)
            },
            GridCandidate {
                space_index: 0,
                ..candidate(20, 2, 0, 0, 100, 100)
            },
            GridCandidate {
                space_index: 0,
                ..candidate(30, 1, 0, 0, 100, 100)
            },
        ];
        let layout = grid_layout(rect(0, 0, 600, 600), &candidates);
        // Active Space (index 0) first, most-recently-used first, then the
        // neighbour Space.
        let order: Vec<u64> = layout
            .placements
            .iter()
            .map(|placement| placement.window.0)
            .collect();
        assert_eq!(order, vec![30, 20, 10]);

        // Shuffling the input must not change the order (stable).
        candidates.reverse();
        let shuffled = grid_layout(rect(0, 0, 600, 600), &candidates);
        let shuffled_order: Vec<u64> = shuffled
            .placements
            .iter()
            .map(|placement| placement.window.0)
            .collect();
        assert_eq!(shuffled_order, order);
    }

    #[test]
    fn relative_sizes_read_correctly_under_the_one_scale() {
        let candidates = vec![
            candidate(1, 0, 0, 0, 400, 300),
            candidate(2, 1, 0, 0, 200, 150),
        ];
        let layout = grid_layout(rect(0, 0, 1000, 600), &candidates);
        let big = layout.placement(WindowId(1)).unwrap();
        let small = layout.placement(WindowId(2)).unwrap();
        // The half-size window is half the target size: one shared scale.
        let ratio = f64::from(big.target.size.w) / f64::from(small.target.size.w);
        assert!((ratio - 2.0).abs() < 0.05, "ratio={ratio}");
    }

    #[test]
    fn frame_lerps_from_the_committed_geometry_to_the_grid_target() {
        let candidates = vec![candidate(1, 0, 100, 100, 400, 300)];
        let layout = grid_layout(rect(0, 0, 1200, 800), &candidates);
        let placement = layout.placement(WindowId(1)).unwrap();

        // At progress 0 the frame is the committed geometry (identity).
        let start = placement.frame(0.0);
        assert_eq!(start.rect, rect(100, 100, 400, 300));
        assert_eq!(start.alpha, 1.0);
        assert_eq!(start.scale, Scale::from((1.0, 1.0)));
        assert_eq!(start.offset, Point::from((0, 0)));

        // At progress 1 it is exactly the grid target.
        let end = placement.frame(1.0);
        assert_eq!(end.rect, placement.target);
        assert_eq!(end.alpha, 1.0);
        assert!((end.scale.x - placement.scale).abs() < 1e-9);

        // Mid-flight it is strictly between the two sizes (the scale may be
        // above or below 1, so compare against the ordered pair).
        let mid = placement.frame(0.5);
        let low = placement.source.size.w.min(placement.target.size.w);
        let high = placement.source.size.w.max(placement.target.size.w);
        assert!(mid.rect.size.w > low && mid.rect.size.w < high);
    }

    #[test]
    fn many_windows_still_share_one_scale_and_never_overlap() {
        let candidates: Vec<GridCandidate> = (0..12)
            .map(|index| candidate(index, index as usize, 0, 0, 320, 240))
            .collect();
        let layout = grid_layout(rect(0, 0, 1600, 900), &candidates);
        assert_eq!(layout.len(), 12);
        for placement in &layout.placements {
            assert!((placement.scale - layout.scale).abs() < 1e-9);
            assert!(layout.area.contains_rect(placement.cell));
        }
        for (i, a) in layout.placements.iter().enumerate() {
            for b in layout.placements.iter().skip(i + 1) {
                assert!(!a.cell.overlaps(b.cell));
            }
        }
    }
}
