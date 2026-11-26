// SPDX-License-Identifier: GPL-3.0-only
//! The window menu's presentation row model (T329).
//!
//! The compositor owns the menu's geometry, hit-testing, and command routing
//! (`window/menu.rs`); the trusted shell renders the same rows as text and
//! icons in a popup surface. This module is the one shared row vocabulary: the
//! labels and glyph names the shell receives over the private protocol and the
//! names the fallback compositor draw and the conformance tests assert.
//!
//! Keeping it pure (no Smithay, no window state) lets `window_conformance`
//! pin the model without a display and lets the shell QML test use the same
//! labels/glyph names. The glyph names are Tabler resource ids; the design
//! system renders them through `TablerIcon`.

/// The four top-level rows, in order.
pub const ROW_COUNT: usize = 4;

/// One top-level row's presentation: the design-system label and the Tabler
/// glyph name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowSpec {
    pub label: &'static str,
    pub icon: &'static str,
}

/// The four top-level rows in menu order: Move to Space, Minimize, Zoom,
/// Close. The first row opens the Space submenu; the other three are
/// commands routed through the window state machine.
pub const ROWS: [RowSpec; ROW_COUNT] = [
    RowSpec {
        label: "Move to Space",
        icon: "layout-grid",
    },
    RowSpec {
        label: "Minimize",
        icon: "minus",
    },
    RowSpec {
        label: "Zoom",
        icon: "maximize",
    },
    RowSpec {
        label: "Close",
        icon: "x",
    },
];

/// The glyph used by every Move to Space submenu row. Spaces carry their
/// name as the label; one shared glyph keeps the submenu quiet.
pub const SPACE_ICON: &str = "layout-grid";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_rows_carry_the_design_labels_and_icons() {
        assert_eq!(ROW_COUNT, 4);
        let labels: Vec<_> = ROWS.iter().map(|row| row.label).collect();
        assert_eq!(
            labels,
            vec!["Move to Space", "Minimize", "Zoom", "Close"],
            "the menu labels are the readable design copy"
        );
        let icons: Vec<_> = ROWS.iter().map(|row| row.icon).collect();
        assert_eq!(
            icons,
            vec!["layout-grid", "minus", "maximize", "x"],
            "every row names a vendored Tabler glyph"
        );
        assert_eq!(SPACE_ICON, "layout-grid");
    }
}
