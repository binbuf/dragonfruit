// SPDX-License-Identifier: MIT
#![allow(dead_code)] // Forward-looking window API consumed by T-05/T-07/T-10/T-13.

//! Window model: states, focus, placement, and regions (T-04).
//!
//! This module is the compositor's window policy. It owns the per-window
//! state machine and geometry restore, the per-output cascade, transient
//! parent/child relationships, and the window lifecycle/focus broadcast
//! outbox. The shell never co-owns any of it: a shell crash or restart
//! leaves every entry untouched (FR-9).
//!
//! [`WindowModel`] keys metadata by Smithay [`Window`], which is cheap to
//! clone and hash by identity. The compositor remains the source of truth
//! for stacking (`Space`) and focus (`Seat`); this module adds the state
//! those layers do not carry.

pub mod corner;
pub mod decoration;
pub mod events;
pub mod grab;
pub mod menu;
pub mod motion;
pub mod placement;
pub mod popup;
pub mod resize;
pub mod shadow;
pub mod state;

#[allow(unused_imports)]
pub use corner::{corner_squares, rounded_rect_spans, CornerMask, RoundedCorners};
pub use decoration::{
    fullscreen_reveal_rect, ColorScheme, DoubleClickTracker, TitlebarDoubleClick, TitlebarElement,
    TrafficLightKind, WindowInsets,
};
pub use events::{ShellWindowEvent, WindowDispatch, WindowEventKind};
pub use menu::{MenuActivation, MenuKey, MenuKeyOutcome, WindowMenu};
pub use motion::{MotionFrame, WindowMotion, WindowMotionKind};
#[allow(unused_imports)]
pub use placement::{
    cascaded_geometry, centered_on, user_positioned_geometry, Cascade, CASCADE_SLOTS, CASCADE_STEP,
};
#[allow(unused_imports)]
pub use resize::{
    apply_aspect, clamp_move, clamp_within_output, resize_geometry, ResizeEdge, SizeConstraints,
};
#[allow(unused_imports)]
pub use shadow::{
    shadow_bounds, shadow_elements, shadow_layers, ShadowLayer, ShadowLevel, ShadowSpec,
};
pub use state::{WindowEvent, WindowState, WindowStateMachine, WindowTransition};

use std::collections::HashMap;

use smithay::desktop::Window;
use smithay::utils::{Logical, Rectangle};

/// A compositor-stable window identifier. Unlike a shell-side id, this
/// survives shell restarts and is the handle the private protocol uses
/// (T-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

impl std::fmt::Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "window-{}", self.0)
    }
}

/// Reserved zones around an output (menu bar, Dock), provided by the shell
/// over the private protocol (T-07). Zoom fills the remaining usable area.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReservedZones {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
}

impl ReservedZones {
    /// The usable area of `output` after subtracting the reserves, never
    /// collapsing to zero.
    pub fn usable(&self, output: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        let w = (output.size.w - self.left - self.right).max(1);
        let h = (output.size.h - self.top - self.bottom).max(1);
        Rectangle::new(
            (output.loc.x + self.left, output.loc.y + self.top).into(),
            (w, h).into(),
        )
    }
}

/// Which side draws a window's decorations (T-06/T-13).
///
/// X11 windows have no Wayland CSD and land in Tier 2 (compositor-drawn
/// SSD) unless their `_MOTIF_WM_HINTS` explicitly ask to be undecorated.
/// Wayland clients follow `xdg-decoration`, whose compositor default is
/// also SSD. T-13 owns actually rendering the titlebar; this is the
/// per-window tier the renderer consults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecorationTier {
    /// The compositor draws the titlebar (Tier 2).
    #[default]
    ServerSide,
    /// The client draws its own decorations (Tier 3).
    ClientSide,
}

/// A window-menu command (SSD titlebar right-click in T-13, private
/// protocol in T-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMenuCommand {
    /// Move the window to another Space (T-05 owns Spaces).
    MoveToSpace(usize),
    /// Minimize the window and its transients.
    Minimize,
    /// Toggle Zoom.
    Zoom,
    /// Ask the client to close.
    Close,
}

#[derive(Debug)]
struct WindowEntry {
    id: WindowId,
    machine: WindowStateMachine,
    parent: Option<Window>,
    children: Vec<Window>,
    app_id: Option<String>,
    title: Option<String>,
    decorations: DecorationTier,
    /// The in-flight (or last completed) lifecycle motion (T-02.1b/T-02.2):
    /// appear, minimize, or restore.
    motion: Option<WindowMotion>,
}

/// Compositor-owned window metadata, keyed by Smithay [`Window`].
#[derive(Debug, Default)]
pub struct WindowModel {
    entries: HashMap<Window, WindowEntry>,
    cascades: HashMap<String, Cascade>,
    /// Window ids ordered most-recent first: a newly mapped window enters at
    /// the front and focus moves a window to the front. Drives the app
    /// switcher's order (T-12) and the Dock's "activate most recent window"
    /// (`activate_app`); the compositor owns it because only the compositor
    /// sees every window and focus transition (T-07).
    recency: Vec<WindowId>,
    next_id: u64,
}

impl WindowModel {
    pub fn new() -> Self {
        WindowModel::default()
    }

    /// Register a newly mapped window with its initial floating geometry.
    pub fn insert(&mut self, window: Window, geometry: Rectangle<i32, Logical>) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;
        self.entries.insert(
            window,
            WindowEntry {
                id,
                machine: WindowStateMachine::new(geometry),
                parent: None,
                children: Vec::new(),
                app_id: None,
                title: None,
                decorations: DecorationTier::default(),
                motion: None,
            },
        );
        // A new window is the most recent until another window is focused.
        // The Dock projects an app entry from the window list, not from
        // focus, so `activate_app` must find a window that has never been
        // focused; the newest of several is the best default, and a window
        // that is then focused moves to the front via `touch_recency`.
        self.recency.insert(0, id);
        id
    }

    /// Forget a window. Children are detached rather than left dangling.
    pub fn remove(&mut self, window: &Window) -> Option<WindowId> {
        let entry = self.entries.remove(window)?;
        self.recency.retain(|id| *id != entry.id);
        if let Some(parent) = &entry.parent {
            if let Some(parent_entry) = self.entries.get_mut(parent) {
                parent_entry.children.retain(|child| child != window);
            }
        }
        for child in &entry.children {
            if let Some(child_entry) = self.entries.get_mut(child) {
                child_entry.parent = None;
            }
        }
        Some(entry.id)
    }

    /// Mark `window` as the most recently used window.
    pub fn touch_recency(&mut self, window: &Window) {
        let Some(id) = self.id(window) else {
            return;
        };
        self.recency.retain(|existing| *existing != id);
        self.recency.insert(0, id);
    }

    /// Window ids, most recently used first (the app switcher's order).
    pub fn recency(&self) -> &[WindowId] {
        &self.recency
    }

    /// Whether `window` is registered.
    pub fn contains(&self, window: &Window) -> bool {
        self.entries.contains_key(window)
    }

    /// The stable id for `window`.
    pub fn id(&self, window: &Window) -> Option<WindowId> {
        self.entries.get(window).map(|entry| entry.id)
    }

    /// The state machine for `window`.
    pub fn machine(&self, window: &Window) -> Option<&WindowStateMachine> {
        self.entries.get(window).map(|entry| &entry.machine)
    }

    /// The mutable state machine for `window`.
    pub fn machine_mut(&mut self, window: &Window) -> Option<&mut WindowStateMachine> {
        self.entries.get_mut(window).map(|entry| &mut entry.machine)
    }

    /// The current state of `window`.
    pub fn state(&self, window: &Window) -> Option<WindowState> {
        self.machine(window).map(WindowStateMachine::state)
    }

    /// The geometry `window` currently occupies.
    pub fn geometry(&self, window: &Window) -> Option<Rectangle<i32, Logical>> {
        self.machine(window).map(WindowStateMachine::geometry)
    }

    /// Apply a state event, returning the transition if the window is known.
    pub fn apply(
        &mut self,
        window: &Window,
        event: WindowEvent,
        target: Rectangle<i32, Logical>,
    ) -> Option<WindowTransition> {
        self.machine_mut(window)
            .map(|machine| machine.apply(event, target))
    }

    /// Record an interactive move/resize of a floating window.
    pub fn set_floating_geometry(&mut self, window: &Window, geometry: Rectangle<i32, Logical>) {
        if let Some(machine) = self.machine_mut(window) {
            machine.set_floating_geometry(geometry);
        }
    }

    /// The floating restore geometry of `window`.
    pub fn floating_geometry(&self, window: &Window) -> Option<Rectangle<i32, Logical>> {
        self.machine(window)
            .map(WindowStateMachine::floating_geometry)
    }

    /// The `app_id` last observed for `window`.
    pub fn app_id(&self, window: &Window) -> Option<&str> {
        self.entries
            .get(window)
            .and_then(|entry| entry.app_id.as_deref())
    }

    /// The title last observed for `window`.
    pub fn title(&self, window: &Window) -> Option<&str> {
        self.entries
            .get(window)
            .and_then(|entry| entry.title.as_deref())
    }

    /// Update the `app_id`; returns true if it changed.
    pub fn set_app_id(&mut self, window: &Window, app_id: Option<String>) -> bool {
        let Some(entry) = self.entries.get_mut(window) else {
            return false;
        };
        if entry.app_id == app_id {
            return false;
        }
        entry.app_id = app_id;
        true
    }

    /// Update the title; returns true if it changed.
    pub fn set_title(&mut self, window: &Window, title: Option<String>) -> bool {
        let Some(entry) = self.entries.get_mut(window) else {
            return false;
        };
        if entry.title == title {
            return false;
        }
        entry.title = title;
        true
    }

    /// The decoration tier for `window` (T-13 renders ServerSide).
    pub fn decorations(&self, window: &Window) -> DecorationTier {
        self.entries
            .get(window)
            .map(|entry| entry.decorations)
            .unwrap_or_default()
    }

    /// Set the decoration tier; returns true if it changed.
    pub fn set_decorations(&mut self, window: &Window, tier: DecorationTier) -> bool {
        let Some(entry) = self.entries.get_mut(window) else {
            return false;
        };
        if entry.decorations == tier {
            return false;
        }
        entry.decorations = tier;
        true
    }

    // --- lifecycle motion (T-02.1b appear; T-02.2 minimize/restore) --------

    /// Begin (or replace) `window`'s lifecycle motion. Replacing a live
    /// motion retargets it without waiting (the interruptibility rule).
    pub fn set_motion(&mut self, window: &Window, motion: WindowMotion) -> bool {
        let Some(entry) = self.entries.get_mut(window) else {
            return false;
        };
        entry.motion = Some(motion);
        true
    }

    /// `window`'s lifecycle motion, if one was ever recorded.
    pub fn motion(&self, window: &Window) -> Option<&WindowMotion> {
        self.entries
            .get(window)
            .and_then(|entry| entry.motion.as_ref())
    }

    /// `window`'s live render frame, or `None` when it has no motion (or has
    /// completed). Identity is never wrapped.
    pub fn motion_frame(&self, window: &Window, now_ms: u64) -> Option<MotionFrame> {
        self.motion(window)
            .filter(|motion| !motion.completed)
            .map(|motion| motion.frame(now_ms))
    }

    /// Whether `window` has a **live close** motion: it is input-inert, out of
    /// the layout, and owned by the close ghost until the motion commits its
    /// removal (T-02.4a). The window stays in the model until then.
    pub fn is_closing(&self, window: &Window) -> bool {
        self.motion(window)
            .is_some_and(|motion| motion.kind == WindowMotionKind::Close && !motion.completed)
    }

    /// Advance every lifecycle motion one clock frame. Returns the windows
    /// whose motion reached its end this frame and whether any motion is
    /// still live.
    pub fn step_motions(&mut self, now_ms: u64) -> (Vec<Window>, bool) {
        let mut done = Vec::new();
        let mut active = false;
        for (window, entry) in self.entries.iter_mut() {
            let Some(motion) = entry.motion.as_mut() else {
                continue;
            };
            if motion.completed {
                continue;
            }
            motion.frames += 1;
            if motion.is_done(now_ms) {
                motion.completed = true;
                done.push(window.clone());
            } else {
                active = true;
            }
        }
        (done, active)
    }

    /// Every recorded lifecycle motion (live and completed), with the window
    /// it belongs to, for the `query motion` test hook.
    pub fn motions(&self) -> impl Iterator<Item = (&Window, WindowId, &WindowMotion)> {
        self.entries.iter().filter_map(|(window, entry)| {
            entry
                .motion
                .as_ref()
                .map(|motion| (window, entry.id, motion))
        })
    }

    /// Every live (not yet completed) lifecycle motion with its window. The
    /// render layer draws an active minimize ghost from this even though the
    /// window is unmapped from the [`Space`](smithay::desktop::Space).
    pub fn active_motions(&self) -> impl Iterator<Item = (&Window, &WindowMotion)> {
        self.motions()
            .filter(|(_, _, motion)| !motion.completed)
            .map(|(window, _, motion)| (window, motion))
    }

    /// Make `child` a transient of `parent` (replacing any previous parent).
    pub fn set_parent(&mut self, child: &Window, parent: &Window) {
        if child == parent {
            return;
        }
        if let Some(old_parent) = self
            .entries
            .get(child)
            .and_then(|entry| entry.parent.clone())
        {
            if let Some(old_entry) = self.entries.get_mut(&old_parent) {
                old_entry.children.retain(|c| c != child);
            }
        }
        if let Some(parent_entry) = self.entries.get_mut(parent) {
            if !parent_entry.children.contains(child) {
                parent_entry.children.push(child.clone());
            }
        }
        if let Some(child_entry) = self.entries.get_mut(child) {
            child_entry.parent = Some(parent.clone());
        }
    }

    /// The transient parent of `window`, if any.
    pub fn parent(&self, window: &Window) -> Option<Window> {
        self.entries
            .get(window)
            .and_then(|entry| entry.parent.clone())
    }

    /// The direct transient children of `window`.
    pub fn children(&self, window: &Window) -> Vec<Window> {
        self.entries
            .get(window)
            .map(|entry| entry.children.clone())
            .unwrap_or_default()
    }

    /// `window` and all of its transitive transient children, parents first.
    pub fn transient_tree(&self, window: &Window) -> Vec<Window> {
        let mut result = vec![window.clone()];
        let mut index = 0;
        while index < result.len() {
            let current = result[index].clone();
            index += 1;
            for child in self.children(&current) {
                if !result.contains(&child) {
                    result.push(child);
                }
            }
        }
        result
    }

    /// The next cascade slot for `output`, advancing the per-output counter.
    pub fn cascade_index(&mut self, output: &str) -> i32 {
        self.cascades.entry(output.to_string()).or_default().next()
    }

    /// Reset the cascade for `output` (e.g. its last window closed).
    pub fn reset_cascade(&mut self, output: &str) {
        if let Some(cascade) = self.cascades.get_mut(output) {
            cascade.reset();
        }
    }

    /// Number of tracked windows.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every tracked window.
    pub fn windows(&self) -> impl Iterator<Item = &Window> {
        self.entries.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    // `WindowModel` is keyed by Smithay `Window`, which needs a live
    // surface to construct. These tests therefore exercise the pieces that
    // do not require a `Window` (reserved zones) plus the cascade counters
    // through a model with no entries. Transient behavior is covered by
    // the compositor-level integration path and the pure state machine.

    #[test]
    fn reserved_zones_shrink_the_usable_area() {
        let zones = ReservedZones {
            top: 24,
            bottom: 80,
            left: 0,
            right: 0,
        };
        assert_eq!(zones.usable(rect(0, 0, 1920, 1080)), rect(0, 24, 1920, 976));
    }

    #[test]
    fn reserved_zones_never_collapse_to_zero() {
        let zones = ReservedZones {
            top: 5000,
            bottom: 5000,
            left: 5000,
            right: 5000,
        };
        let usable = zones.usable(rect(0, 0, 1920, 1080));
        assert!(usable.size.w >= 1 && usable.size.h >= 1);
    }

    #[test]
    fn cascades_are_tracked_per_output() {
        let mut model = WindowModel::new();
        assert_eq!(model.cascade_index("DP-1"), 0);
        assert_eq!(model.cascade_index("DP-1"), 1);
        assert_eq!(model.cascade_index("HDMI-A-1"), 0);
        model.reset_cascade("DP-1");
        assert_eq!(model.cascade_index("DP-1"), 0);
    }
}
