// SPDX-License-Identifier: MIT
//! The SSD titlebar's window menu (T-01.4).
//!
//! A right-click or Control-click on the titlebar opens a small menu over
//! the window: **Move to Space** (a submenu of the window output's Spaces),
//! **Minimize**, **Zoom**, and **Close**. The menu is deliberately
//! *policy-free*: it resolves pointer and keyboard input to a
//! [`WindowMenuCommand`] and hands that to the existing window state machine.
//! It never changes window state itself, so the compositor keeps one owner
//! of truth and the four commands behave identically to the traffic lights
//! and the shell protocol.
//!
//! Geometry and dismissal mirror the design-system `ContextMenu`
//! (`design-system/components/ContextMenu.qml`): rows are
//! `component.contextMenu.rowHeight` tall inside `padding`, the panel is at
//! least `minWidth` wide, Escape closes an open submenu first and then the
//! menu, Up/Down/Home/End move the highlight, Right/Left open/close the
//! submenu, and Enter/Space activates the highlighted row. Clicking a row
//! activates it; clicking away (or losing focus) dismisses.
//!
//! The visual pass is intentionally flat and label-less in this slice: the
//! compositor has no text/icon renderer yet, so rows draw as token-colored
//! fills with a highlight, exactly as the T-01.1 traffic-light glyphs are
//! placeholder marks. T-04's material pass replaces the fill and draws the
//! labels; the geometry, hit-testing, and command routing here do not change.

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Color32F;
use smithay::input::keyboard::xkb::keysyms;
use smithay::utils::{Logical, Point, Rectangle, Scale, Size};

use crate::design_tokens::component::{context_menu, window as window_tokens};
use crate::window::decoration::{color_from_rgba, ColorScheme};
use crate::window::{WindowId, WindowMenuCommand};

/// The logical height of one menu row (`component.contextMenu.rowHeight`).
pub const MENU_ROW_HEIGHT: i32 = context_menu::ROW_HEIGHT as i32;
/// The panel padding (`component.contextMenu.padding`).
pub const MENU_PADDING: i32 = context_menu::PADDING as i32;
/// The minimum panel width (`component.contextMenu.minWidth`).
pub const MENU_MIN_WIDTH: i32 = context_menu::MIN_WIDTH as i32;
/// The gap between the menu and its submenu (`primitive.spacing.xxs`).
pub const MENU_SUBMENU_GAP: i32 = 2;
/// The panel border width (`component.window.borderWidth`).
pub const MENU_BORDER_WIDTH: i32 = window_tokens::BORDER_WIDTH as i32;

/// The number of top-level rows (Move to Space, Minimize, Zoom, Close).
const TOP_ROW_COUNT: i32 = 4;

/// One top-level window-menu entry. The submenu rows are always
/// [`WindowMenuRow::Command`] with [`WindowMenuCommand::MoveToSpace`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMenuRow {
    /// The "Move to Space" submenu opener.
    MoveToSpace,
    /// A command that routes through the window state machine.
    Command(WindowMenuCommand),
}

impl WindowMenuRow {
    /// The design-system label (drawn by T-04; used by tests and docs).
    pub const fn label(self) -> &'static str {
        match self {
            WindowMenuRow::MoveToSpace => "Move to Space",
            WindowMenuRow::Command(WindowMenuCommand::Minimize) => "Minimize",
            WindowMenuRow::Command(WindowMenuCommand::Zoom) => "Zoom",
            WindowMenuRow::Command(WindowMenuCommand::Close) => "Close",
            WindowMenuRow::Command(WindowMenuCommand::MoveToSpace(_)) => "Move to Space",
        }
    }

    /// The command this row resolves to, if it is not a submenu opener.
    pub const fn command(self) -> Option<WindowMenuCommand> {
        match self {
            WindowMenuRow::MoveToSpace => None,
            WindowMenuRow::Command(command) => Some(command),
        }
    }
}

/// A laid-out menu row with its global-space rect (the row's interactive
/// area, already inset by the panel padding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuRow {
    pub row: WindowMenuRow,
    pub label: String,
    pub rect: Rectangle<i32, Logical>,
}

/// A fully laid-out window menu for one window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowMenu {
    /// The window the menu acts on.
    pub window: WindowId,
    /// The global-space point the menu was opened at (the titlebar press).
    pub anchor: Point<i32, Logical>,
    /// The top-level panel rect in global space.
    pub rect: Rectangle<i32, Logical>,
    pub rows: Vec<MenuRow>,
    /// The submenu panel rect (valid whether or not it is open).
    pub submenu_rect: Rectangle<i32, Logical>,
    pub submenu: Vec<MenuRow>,
    pub open_submenu: bool,
    pub highlighted: Option<usize>,
    pub submenu_highlighted: Option<usize>,
    pub scheme: ColorScheme,
}

/// What a pointer activation resolved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuActivation {
    /// Activate this command and dismiss the menu.
    Command(WindowMenuCommand),
    /// The submenu opened; the menu stays open.
    OpenSubmenu,
    /// The pointer hit the menu but no actionable row.
    None,
}

/// A keyboard key the menu understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Activate,
    Escape,
}

impl MenuKey {
    /// Map an xkb keysym to a menu key, if it is one the menu handles.
    pub fn from_keysym(raw: u32) -> Option<Self> {
        match raw {
            keysyms::KEY_Up => Some(MenuKey::Up),
            keysyms::KEY_Down => Some(MenuKey::Down),
            keysyms::KEY_Left => Some(MenuKey::Left),
            keysyms::KEY_Right => Some(MenuKey::Right),
            keysyms::KEY_Home => Some(MenuKey::Home),
            keysyms::KEY_End => Some(MenuKey::End),
            keysyms::KEY_Return | keysyms::KEY_KP_Enter | keysyms::KEY_space => {
                Some(MenuKey::Activate)
            }
            keysyms::KEY_Escape => Some(MenuKey::Escape),
            _ => None,
        }
    }
}

/// What a key resolved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKeyOutcome {
    /// Activate this command and dismiss the menu.
    Command(WindowMenuCommand),
    /// The highlight/submenu changed; redraw.
    Changed,
    /// Dismiss the whole menu.
    Dismiss,
    /// The key was not meaningful; still owned by the menu.
    Ignored,
}

/// Place a panel of `size` at `anchor`, flipping and clamping so it stays
/// inside `bounds` (the design-system flip rule).
fn place(
    anchor: Point<i32, Logical>,
    size: Size<i32, Logical>,
    bounds: Rectangle<i32, Logical>,
) -> Point<i32, Logical> {
    let max_x = (bounds.loc.x + bounds.size.w - size.w).max(bounds.loc.x);
    let max_y = (bounds.loc.y + bounds.size.h - size.h).max(bounds.loc.y);
    let mut x = anchor.x;
    let mut y = anchor.y;
    if x + size.w > bounds.loc.x + bounds.size.w {
        x = anchor.x - size.w;
    }
    if y + size.h > bounds.loc.y + bounds.size.h {
        y = anchor.y - size.h;
    }
    (x.clamp(bounds.loc.x, max_x), y.clamp(bounds.loc.y, max_y)).into()
}

fn row_rect(panel: Rectangle<i32, Logical>, index: i32) -> Rectangle<i32, Logical> {
    Rectangle::new(
        (
            panel.loc.x + MENU_PADDING,
            panel.loc.y + MENU_PADDING + index * MENU_ROW_HEIGHT,
        )
            .into(),
        (panel.size.w - 2 * MENU_PADDING, MENU_ROW_HEIGHT).into(),
    )
}

impl WindowMenu {
    /// Open the menu for `window` at `anchor`, listing `spaces` (index, name)
    /// on the window's output. `bounds` is the output geometry the panel must
    /// stay inside.
    pub fn open(
        window: WindowId,
        anchor: Point<i32, Logical>,
        spaces: &[(usize, String)],
        bounds: Rectangle<i32, Logical>,
        scheme: ColorScheme,
    ) -> Self {
        let size = Size::from((
            MENU_MIN_WIDTH,
            2 * MENU_PADDING + TOP_ROW_COUNT * MENU_ROW_HEIGHT,
        ));
        let rect = Rectangle::new(place(anchor, size, bounds), size);
        let rows: Vec<MenuRow> = [
            WindowMenuRow::MoveToSpace,
            WindowMenuRow::Command(WindowMenuCommand::Minimize),
            WindowMenuRow::Command(WindowMenuCommand::Zoom),
            WindowMenuRow::Command(WindowMenuCommand::Close),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, row)| MenuRow {
            row,
            label: row.label().to_string(),
            rect: row_rect(rect, index as i32),
        })
        .collect();

        // The submenu hangs beside the "Move to Space" row (row 0), flipping
        // to the panel's left when it would leave the output.
        let submenu_size = Size::from((
            MENU_MIN_WIDTH,
            2 * MENU_PADDING + spaces.len().max(1) as i32 * MENU_ROW_HEIGHT,
        ));
        let mut sub_x = rect.loc.x + rect.size.w + MENU_SUBMENU_GAP;
        if sub_x + submenu_size.w > bounds.loc.x + bounds.size.w {
            sub_x = rect.loc.x - submenu_size.w - MENU_SUBMENU_GAP;
        }
        let max_sub_x = (bounds.loc.x + bounds.size.w - submenu_size.w).max(bounds.loc.x);
        let sub_x = sub_x.clamp(bounds.loc.x, max_sub_x);
        let max_sub_y = (bounds.loc.y + bounds.size.h - submenu_size.h).max(bounds.loc.y);
        let sub_y = rows[0].rect.loc.y.clamp(bounds.loc.y, max_sub_y);
        let submenu_rect = Rectangle::new((sub_x, sub_y).into(), submenu_size);
        let submenu = spaces
            .iter()
            .enumerate()
            .map(|(index, (space, name))| MenuRow {
                row: WindowMenuRow::Command(WindowMenuCommand::MoveToSpace(*space)),
                label: name.clone(),
                rect: row_rect(submenu_rect, index as i32),
            })
            .collect();

        WindowMenu {
            window,
            anchor,
            rect,
            rows,
            submenu_rect,
            submenu,
            open_submenu: false,
            // The design-system ContextMenu highlights the first row on open.
            highlighted: Some(0),
            submenu_highlighted: None,
            scheme,
        }
    }

    /// Whether `point` is inside the top-level panel.
    pub fn contains_panel(&self, point: Point<i32, Logical>) -> bool {
        self.rect.contains(point)
    }

    /// Whether `point` is inside the currently open submenu.
    pub fn contains_submenu(&self, point: Point<i32, Logical>) -> bool {
        self.open_submenu && self.submenu_rect.contains(point)
    }

    /// Whether `point` is inside the menu at all (panel or open submenu).
    pub fn hit(&self, point: Point<i32, Logical>) -> bool {
        self.contains_panel(point) || self.contains_submenu(point)
    }

    /// The top-level row under `point`.
    pub fn row_at(&self, point: Point<i32, Logical>) -> Option<usize> {
        self.rows.iter().position(|row| row.rect.contains(point))
    }

    /// The submenu row under `point`.
    pub fn submenu_row_at(&self, point: Point<i32, Logical>) -> Option<usize> {
        self.submenu.iter().position(|row| row.rect.contains(point))
    }

    /// Update the highlight to the row under `point` (design-system hover
    /// reveal). Returns whether the highlight changed. A point in the panel
    /// padding clears the highlight.
    pub fn hover(&mut self, point: Point<i32, Logical>) -> bool {
        if self.open_submenu && self.submenu_rect.contains(point) {
            let next = self.submenu_row_at(point);
            if next != self.submenu_highlighted {
                self.submenu_highlighted = next;
                return true;
            }
            return false;
        }
        if self.rect.contains(point) {
            let next = self.row_at(point);
            if next != self.highlighted {
                self.highlighted = next;
                return true;
            }
        }
        false
    }

    /// Resolve a pointer press inside the menu.
    pub fn activate(&mut self, point: Point<i32, Logical>) -> MenuActivation {
        if self.contains_submenu(point) {
            if let Some(index) = self.submenu_row_at(point) {
                if let Some(command) = self.submenu[index].row.command() {
                    return MenuActivation::Command(command);
                }
            }
            return MenuActivation::None;
        }
        if self.contains_panel(point) {
            if let Some(index) = self.row_at(point) {
                match self.rows[index].row {
                    WindowMenuRow::MoveToSpace => {
                        self.open_submenu = true;
                        self.submenu_highlighted = None;
                        self.move_submenu(1);
                        return MenuActivation::OpenSubmenu;
                    }
                    WindowMenuRow::Command(command) => {
                        return MenuActivation::Command(command);
                    }
                }
            }
        }
        MenuActivation::None
    }

    /// Move the top-level highlight by `delta`, wrapping.
    pub fn move_highlight(&mut self, delta: i32) {
        let count = self.rows.len() as i32;
        if count == 0 {
            return;
        }
        let start = self
            .highlighted
            .map(|index| index as i32)
            .unwrap_or(if delta > 0 { -1 } else { 0 });
        self.highlighted = Some((start + delta).rem_euclid(count) as usize);
    }

    /// Move the submenu highlight by `delta`, wrapping.
    pub fn move_submenu(&mut self, delta: i32) {
        let count = self.submenu.len() as i32;
        if count == 0 {
            return;
        }
        let start = self
            .submenu_highlighted
            .map(|index| index as i32)
            .unwrap_or(if delta > 0 { -1 } else { 0 });
        self.submenu_highlighted = Some((start + delta).rem_euclid(count) as usize);
    }

    /// Handle a keyboard key, mirroring the design-system `ContextMenu`.
    pub fn handle_key(&mut self, key: MenuKey) -> MenuKeyOutcome {
        match key {
            MenuKey::Escape => {
                if self.open_submenu {
                    self.open_submenu = false;
                    self.submenu_highlighted = None;
                    MenuKeyOutcome::Changed
                } else {
                    MenuKeyOutcome::Dismiss
                }
            }
            MenuKey::Down => {
                if self.open_submenu {
                    self.move_submenu(1);
                } else {
                    self.move_highlight(1);
                }
                MenuKeyOutcome::Changed
            }
            MenuKey::Up => {
                if self.open_submenu {
                    self.move_submenu(-1);
                } else {
                    self.move_highlight(-1);
                }
                MenuKeyOutcome::Changed
            }
            MenuKey::Right => {
                if !self.open_submenu && self.highlighted == Some(0) {
                    self.open_submenu = true;
                    self.submenu_highlighted = None;
                    self.move_submenu(1);
                    MenuKeyOutcome::Changed
                } else {
                    MenuKeyOutcome::Ignored
                }
            }
            MenuKey::Left => {
                if self.open_submenu {
                    self.open_submenu = false;
                    self.submenu_highlighted = None;
                    MenuKeyOutcome::Changed
                } else {
                    MenuKeyOutcome::Ignored
                }
            }
            MenuKey::Home => {
                if self.open_submenu {
                    self.submenu_highlighted = None;
                    self.move_submenu(1);
                } else {
                    self.highlighted = None;
                    self.move_highlight(1);
                }
                MenuKeyOutcome::Changed
            }
            MenuKey::End => {
                if self.open_submenu {
                    self.submenu_highlighted = None;
                    self.move_submenu(-1);
                } else {
                    self.highlighted = None;
                    self.move_highlight(-1);
                }
                MenuKeyOutcome::Changed
            }
            MenuKey::Activate => {
                if self.open_submenu {
                    match self
                        .submenu_highlighted
                        .and_then(|index| self.submenu[index].row.command())
                    {
                        Some(command) => MenuKeyOutcome::Command(command),
                        None => MenuKeyOutcome::Ignored,
                    }
                } else if self.highlighted == Some(0) {
                    self.open_submenu = true;
                    self.submenu_highlighted = None;
                    self.move_submenu(1);
                    MenuKeyOutcome::Changed
                } else {
                    match self
                        .highlighted
                        .and_then(|index| self.rows[index].row.command())
                    {
                        Some(command) => MenuKeyOutcome::Command(command),
                        None => MenuKeyOutcome::Ignored,
                    }
                }
            }
        }
    }

    /// The solid-fill render elements for the menu, in output-local physical
    /// coordinates. Ordered panel border → panel → highlight → submenu, so a
    /// back-to-front renderer draws them correctly.
    pub fn render_elements(
        &self,
        scale: Scale<f64>,
        output_origin: Point<i32, Logical>,
    ) -> Vec<SolidColorRenderElement> {
        let mut elements = Vec::new();
        let to_physical = |rect: Rectangle<i32, Logical>| rect.to_physical_precise_round(scale);
        let local = |rect: Rectangle<i32, Logical>| {
            Rectangle::new(
                (rect.loc.x - output_origin.x, rect.loc.y - output_origin.y).into(),
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

        let border_color = color_from_rgba(self.scheme.border());
        let surface = color_from_rgba(self.scheme.surface_elevated());
        let accent = color_from_rgba(self.scheme.accent());

        let mut push_panel =
            |panel: Rectangle<i32, Logical>, highlighted: Option<usize>, rows: &[MenuRow]| {
                let border = Rectangle::new(
                    (
                        panel.loc.x - MENU_BORDER_WIDTH,
                        panel.loc.y - MENU_BORDER_WIDTH,
                    )
                        .into(),
                    (
                        panel.size.w + 2 * MENU_BORDER_WIDTH,
                        panel.size.h + 2 * MENU_BORDER_WIDTH,
                    )
                        .into(),
                );
                push(border, border_color);
                push(panel, surface);
                if let Some(index) = highlighted {
                    if let Some(row) = rows.get(index) {
                        push(row.rect, accent);
                    }
                }
            };

        push_panel(self.rect, self.highlighted, &self.rows);
        if self.open_submenu {
            push_panel(self.submenu_rect, self.submenu_highlighted, &self.submenu);
        }
        elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rectangle<i32, Logical> {
        Rectangle::new((0, 0).into(), (1280, 720).into())
    }

    fn spaces() -> Vec<(usize, String)> {
        vec![(0, "Space 1".into()), (1, "Space 2".into())]
    }

    fn menu_at(anchor: (i32, i32)) -> WindowMenu {
        WindowMenu::open(
            WindowId(1),
            anchor.into(),
            &spaces(),
            bounds(),
            ColorScheme::Dark,
        )
    }

    fn center(rect: Rectangle<i32, Logical>) -> Point<i32, Logical> {
        (rect.loc.x + rect.size.w / 2, rect.loc.y + rect.size.h / 2).into()
    }

    #[test]
    fn rows_carry_the_four_design_commands_in_order() {
        let menu = menu_at((100, 100));
        let labels: Vec<_> = menu.rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(labels, vec!["Move to Space", "Minimize", "Zoom", "Close"]);
        assert_eq!(
            menu.rows[0].row,
            WindowMenuRow::MoveToSpace,
            "Move to Space is the submenu opener"
        );
        assert_eq!(
            menu.rows[1].row.command(),
            Some(WindowMenuCommand::Minimize)
        );
        assert_eq!(menu.rows[2].row.command(), Some(WindowMenuCommand::Zoom));
        assert_eq!(menu.rows[3].row.command(), Some(WindowMenuCommand::Close));
    }

    #[test]
    fn geometry_comes_from_the_context_menu_tokens() {
        let menu = menu_at((100, 100));
        assert_eq!(menu.rect.size.w, MENU_MIN_WIDTH);
        assert_eq!(
            menu.rect.size.h,
            2 * MENU_PADDING + TOP_ROW_COUNT * MENU_ROW_HEIGHT
        );
        assert_eq!(menu.rows[0].rect.loc.x, menu.rect.loc.x + MENU_PADDING);
        assert_eq!(menu.rows[0].rect.loc.y, menu.rect.loc.y + MENU_PADDING);
        assert_eq!(
            menu.rows[1].rect.loc.y - menu.rows[0].rect.loc.y,
            MENU_ROW_HEIGHT
        );
        assert_eq!(menu.rows[0].rect.size.w, MENU_MIN_WIDTH - 2 * MENU_PADDING);
    }

    #[test]
    fn panel_flips_and_clamps_to_stay_inside_the_output() {
        // Near the bottom-right: the panel flips up/left and stays inside.
        let menu = menu_at((1270, 710));
        assert!(menu.rect.loc.x + menu.rect.size.w <= 1280);
        assert!(menu.rect.loc.y + menu.rect.size.h <= 720);
        assert!(
            menu.rect.loc.x < 1270,
            "the panel flipped left of the anchor"
        );
        assert!(menu.rect.loc.y < 710, "the panel flipped above the anchor");
    }

    #[test]
    fn clicking_a_command_row_resolves_to_that_command() {
        let mut menu = menu_at((100, 100));
        // Row 2 is Zoom.
        assert_eq!(
            menu.activate(center(menu.rows[2].rect)),
            MenuActivation::Command(WindowMenuCommand::Zoom)
        );
        assert_eq!(
            menu.activate(center(menu.rows[3].rect)),
            MenuActivation::Command(WindowMenuCommand::Close)
        );
    }

    #[test]
    fn clicking_move_to_space_opens_the_submenu_then_a_space_row_commands() {
        let mut menu = menu_at((100, 100));
        assert_eq!(
            menu.activate(center(menu.rows[0].rect)),
            MenuActivation::OpenSubmenu
        );
        assert!(menu.open_submenu);
        assert_eq!(menu.submenu.len(), 2);
        assert_eq!(
            menu.activate(center(menu.submenu[1].rect)),
            MenuActivation::Command(WindowMenuCommand::MoveToSpace(1))
        );
    }

    #[test]
    fn keyboard_mirrors_the_design_system_context_menu() {
        let mut menu = menu_at((100, 100));
        // Open highlights the first row; Down twice reaches Zoom (row 2).
        assert_eq!(menu.highlighted, Some(0));
        assert_eq!(menu.handle_key(MenuKey::Down), MenuKeyOutcome::Changed);
        assert_eq!(menu.handle_key(MenuKey::Down), MenuKeyOutcome::Changed);
        assert_eq!(menu.highlighted, Some(2));
        assert_eq!(
            menu.handle_key(MenuKey::Activate),
            MenuKeyOutcome::Command(WindowMenuCommand::Zoom)
        );

        // Escape dismisses the top level.
        let mut menu = menu_at((100, 100));
        assert_eq!(menu.handle_key(MenuKey::Escape), MenuKeyOutcome::Dismiss);

        // Right opens the submenu; Escape closes it before the menu.
        let mut menu = menu_at((100, 100));
        assert_eq!(menu.handle_key(MenuKey::Right), MenuKeyOutcome::Changed);
        assert!(menu.open_submenu);
        assert_eq!(menu.handle_key(MenuKey::Escape), MenuKeyOutcome::Changed);
        assert!(!menu.open_submenu);
        assert_eq!(menu.handle_key(MenuKey::Escape), MenuKeyOutcome::Dismiss);

        // End then Activate selects the last row (Close).
        let mut menu = menu_at((100, 100));
        assert_eq!(menu.handle_key(MenuKey::End), MenuKeyOutcome::Changed);
        assert_eq!(menu.highlighted, Some(3));
        assert_eq!(
            menu.handle_key(MenuKey::Activate),
            MenuKeyOutcome::Command(WindowMenuCommand::Close)
        );
    }

    #[test]
    fn keysym_mapping_covers_the_menu_keys() {
        assert_eq!(MenuKey::from_keysym(keysyms::KEY_Up), Some(MenuKey::Up));
        assert_eq!(MenuKey::from_keysym(keysyms::KEY_Down), Some(MenuKey::Down));
        assert_eq!(
            MenuKey::from_keysym(keysyms::KEY_Escape),
            Some(MenuKey::Escape)
        );
        assert_eq!(
            MenuKey::from_keysym(keysyms::KEY_Return),
            Some(MenuKey::Activate)
        );
        assert_eq!(
            MenuKey::from_keysym(keysyms::KEY_space),
            Some(MenuKey::Activate)
        );
        assert_eq!(MenuKey::from_keysym(keysyms::KEY_a), None);
    }

    #[test]
    fn hover_highlights_the_row_under_the_pointer() {
        let mut menu = menu_at((100, 100));
        assert!(
            menu.hover(center(menu.rows[3].rect)),
            "hovering Close changes the highlight"
        );
        assert_eq!(menu.highlighted, Some(3));
        // Hovering the panel padding clears the highlight.
        assert!(menu.hover(menu.rect.loc + Point::from((2, 2))));
        assert_eq!(menu.highlighted, None);
        // A point outside the panel is ignored.
        assert!(!menu.hover(Point::from((0, 0))));
    }

    #[test]
    fn render_elements_draw_a_panel_and_a_highlight() {
        let mut menu = menu_at((100, 100));
        menu.highlighted = Some(2);
        let elements = menu.render_elements(1.0.into(), Point::from((0, 0)));
        // Border + panel + one highlight.
        assert_eq!(elements.len(), 3);
        let open = {
            menu.open_submenu = true;
            menu.submenu_highlighted = Some(0);
            menu.render_elements(1.0.into(), Point::from((0, 0))).len()
        };
        assert_eq!(open, 3 + 3, "the submenu adds border + panel + highlight");
    }
}
