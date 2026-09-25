// SPDX-License-Identifier: MIT
//! App-switcher live-preview layout (T-06.2a).
//!
//! The Cmd-Tab switcher shows the **real** window surfaces of the running
//! apps, never thumbnails ([06-loop-v4-app-switcher.md]). The compositor lays
//! one live surface per recency entry out inside a preview area that leaves
//! the shell's app-card strip clear at the bottom; the shell draws the cards,
//! the accessibility names, and the selection highlight on its own overlay
//! surface.
//!
//! This module is pure geometry. The preview grid is the same T-04 scene
//! transform the Mission Control grid uses ([`grid_layout`]), so the switcher
//! adds no second mapping; the only switcher-specific decision is which part
//! of the output the previews fill and which window is the selected entry.
//!
//! [`grid_layout`]: super::grid::grid_layout

use smithay::utils::{Logical, Rectangle};

use crate::design_tokens::component::overview;

/// The vertical space reserved at the bottom of the output for the shell's
/// app-card strip: the card height plus the strip margin above and below,
/// reproduced from the shared `component.overview` tokens the shell draws
/// with, so the live previews never sit under the cards.
pub const CARD_STRIP: i32 = overview::CARD_HEIGHT as i32 + 2 * overview::STRIP_MARGIN as i32;

/// The output region the switcher's live previews fill: the whole output
/// minus the app-card strip at the bottom. A degenerate area never collapses
/// to zero.
pub fn preview_area(area: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
    let height = (area.size.h - CARD_STRIP).max(1);
    Rectangle::new(area.loc, (area.size.w, height).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::Point;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), (w, h).into())
    }

    #[test]
    fn preview_area_reserves_the_card_strip_at_the_bottom() {
        let area = rect(0, 0, 1280, 720);
        let preview = preview_area(area);
        assert_eq!(preview.loc, Point::from((0, 0)));
        assert_eq!(preview.size.w, 1280);
        assert_eq!(preview.size.h, 720 - CARD_STRIP);
        // The strip begins exactly where the previews end.
        assert_eq!(preview.loc.y + preview.size.h + CARD_STRIP, 720);
    }

    #[test]
    fn preview_area_never_collapses() {
        let preview = preview_area(rect(10, 20, 100, 10));
        assert!(preview.size.w >= 1 && preview.size.h >= 1);
    }
}
