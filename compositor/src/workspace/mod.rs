// SPDX-License-Identifier: MIT OR Apache-2.0
//! Spaces: the compositor's workspace model (T-05).
//!
//! Workspace organization is an **internal compositor primitive**
//! ([03-workspaces.md](../../.docs/design/03-workspaces.md)): the
//! compositor owns creation, removal, ordering, and activation, and the
//! shell is a pure consumer of the event stream over the private protocol
//! (T-07). This module is deliberately pure — it deals in
//! [`SpaceId`]/[`WindowId`] and never touches the Smithay scene — so the
//! lockstep, fullscreen-Space, hotplug-migration, and app-memory rules are
//! unit-testable without a live compositor.
//!
//! Model decisions (all from the design doc):
//!
//! * **Spaces are per-display and ordered.** Each output has its own
//!   ordered list; a switch advances the active Space on *every* output in
//!   lockstep (macOS semantics). A fullscreen Space is inserted at the same
//!   index on every output so the lists stay aligned.
//! * **A fullscreen window occupies a dedicated Space** that exists only
//!   while the window is fullscreen and appears in the strip order right
//!   after the Space it was entered from.
//! * **Each Space carries its own wallpaper**, rendered by the compositor
//!   (the shell never draws the desktop background).
//! * **Windows belong to applications, not Spaces.** App Space memory is
//!   keyed by `app_id` (Wayland) / `WM_CLASS` (Xwayland, T-06) until
//!   app-index identity lands (T-23).
//! * **Display hotplug preserves the model.** A newly attached output gets
//!   a fresh Space list; detaching one migrates its windows to the current
//!   Space of the remaining primary output before its Spaces are
//!   destroyed.
//!
//! Everything the model does is recorded in [`WorkspaceDispatch`] before
//! the scene is updated, so the shell can never observe a frame where its
//! state and the compositor's disagree (FR-8).

#![allow(dead_code)] // Forward-looking workspace API consumed by T-07/T-11/T-16.

pub mod events;

#[allow(unused_imports)]
pub use events::{WorkspaceDispatch, WorkspaceEvent, WorkspaceEventKind};

use std::collections::HashMap;

use crate::window::WindowId;

/// The number of Spaces every output starts with (the vertical slice ships
/// three — [ROADMAP.md](../../.docs/ROADMAP.md)).
pub const INITIAL_SPACES: usize = 3;

/// A compositor-stable workspace identifier. Stable across shell restarts
/// and output hotplug, so the private protocol can address a Space without
/// the shell inventing an id (T-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SpaceId(pub u64);

impl std::fmt::Display for SpaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "space-{}", self.0)
    }
}

/// How a wallpaper source is mapped onto an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WallpaperFit {
    /// Scale to cover, cropping the overflow (the default).
    #[default]
    Fill,
    /// Scale to fit, letterboxing.
    Fit,
    /// Scale each axis independently.
    Stretch,
    /// Draw at native size, centered.
    Center,
}

/// A per-Space wallpaper. The compositor renders it as part of the
/// workspace scene; the shell never draws the desktop background.
#[derive(Debug, Clone, PartialEq)]
pub struct Wallpaper {
    /// Solid fallback / letterbox color (RGBA, linear-ish 0..1). Always
    /// present so a Space without an image source still renders.
    pub color: [f32; 4],
    /// Source image path, if a picture was chosen (Settings, T-16).
    pub source: Option<String>,
    pub fit: WallpaperFit,
}

impl Wallpaper {
    /// A solid-color wallpaper.
    pub fn solid(color: [f32; 4]) -> Self {
        Wallpaper {
            color,
            source: None,
            fit: WallpaperFit::Fill,
        }
    }

    /// The default wallpaper for Space `index`, cycling a small palette so
    /// switching Spaces is visually distinguishable before Settings can
    /// choose pictures.
    pub fn default_for(index: usize) -> Self {
        const PALETTE: [[f32; 4]; INITIAL_SPACES] = [
            [0.13, 0.05, 0.16, 1.0], // deep dragonfruit purple
            [0.05, 0.12, 0.18, 1.0], // deep teal
            [0.18, 0.09, 0.06, 1.0], // deep amber
        ];
        Wallpaper::solid(PALETTE[index % PALETTE.len()])
    }
}

/// One Space on one output.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub id: SpaceId,
    /// A human-readable name for logs and the strip fallback.
    pub name: String,
    pub wallpaper: Wallpaper,
    /// `Some(window)` when this Space exists only to host a fullscreen
    /// window; it is destroyed when that window leaves fullscreen.
    pub fullscreen_for: Option<WindowId>,
}

impl Space {
    /// Whether this is a transient fullscreen Space.
    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen_for.is_some()
    }
}

/// The ordered Space list of one output plus its active index.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputSpaces {
    pub output: String,
    pub spaces: Vec<Space>,
    pub active: usize,
}

impl OutputSpaces {
    /// The active Space, if the list is non-empty.
    pub fn active_space(&self) -> Option<&Space> {
        self.spaces.get(self.active)
    }

    /// The Space at `index`.
    pub fn space_at(&self, index: usize) -> Option<&Space> {
        self.spaces.get(index)
    }

    /// Clamp `active` into range after a structural change.
    fn clamp_active(&mut self) {
        if self.spaces.is_empty() {
            self.active = 0;
        } else if self.active >= self.spaces.len() {
            self.active = self.spaces.len() - 1;
        }
    }
}

/// The compositor's workspace truth. The shell keeps no second copy.
#[derive(Debug)]
pub struct WorkspaceModel {
    /// Outputs in output-management priority order (the first is primary).
    outputs: Vec<OutputSpaces>,
    next_space_id: u64,
    /// window -> Space assignment.
    assignments: HashMap<WindowId, SpaceId>,
    /// app identity (`app_id`/`WM_CLASS`) -> Space *index*. Storing the
    /// index rather than the id keeps memory valid across lockstep output
    /// changes and fullscreen insertions.
    app_memory: HashMap<String, usize>,
    dispatch: WorkspaceDispatch,
}

impl Default for WorkspaceModel {
    fn default() -> Self {
        WorkspaceModel::new()
    }
}

impl WorkspaceModel {
    pub fn new() -> Self {
        WorkspaceModel {
            outputs: Vec::new(),
            next_space_id: 0,
            assignments: HashMap::new(),
            app_memory: HashMap::new(),
            dispatch: WorkspaceDispatch::new(),
        }
    }

    /// Register an output with a fresh Space list (hotplug attach and
    /// initial backend setup). Idempotent: an already-known output is left
    /// untouched.
    pub fn add_output(&mut self, output: &str) {
        if self.outputs.iter().any(|o| o.output == output) {
            return;
        }
        let mut spaces = Vec::with_capacity(INITIAL_SPACES);
        for index in 0..INITIAL_SPACES {
            spaces.push(make_space(&mut self.next_space_id, index, None));
        }
        self.outputs.push(OutputSpaces {
            output: output.to_string(),
            spaces,
            active: 0,
        });
        self.dispatch.push(WorkspaceEvent::output_added(output));
    }

    /// Remove an output, migrating every window assigned to its Spaces to
    /// the current Space of `primary` (FR-7). `primary` must be a remaining
    /// output; when it is `None` (the last output went away) assignments are
    /// left intact so a later re-attach can re-home them.
    ///
    /// Returns the migrated `(window, new SpaceId)` pairs.
    pub fn remove_output(
        &mut self,
        output: &str,
        primary: Option<&str>,
    ) -> Vec<(WindowId, SpaceId)> {
        let Some(index) = self.outputs.iter().position(|o| o.output == output) else {
            return Vec::new();
        };
        let removed = self.outputs.remove(index);
        let removed_ids: Vec<SpaceId> = removed.spaces.iter().map(|space| space.id).collect();

        let target = primary.and_then(|name| {
            self.outputs
                .iter()
                .find(|o| o.output == name)
                .and_then(OutputSpaces::active_space)
                .map(|space| space.id)
        });

        let mut migrated = Vec::new();
        if let Some(target) = target {
            for (window, space) in self.assignments.iter_mut() {
                if removed_ids.contains(space) {
                    *space = target;
                    migrated.push((*window, target));
                    self.dispatch
                        .push(WorkspaceEvent::window_assigned(*window, target));
                }
            }
        }
        for id in &removed_ids {
            self.dispatch.push(WorkspaceEvent::removed(output, *id));
        }
        self.dispatch.push(WorkspaceEvent::output_removed(output));
        migrated
    }

    /// Whether `output` is known.
    pub fn has_output(&self, output: &str) -> bool {
        self.outputs.iter().any(|o| o.output == output)
    }

    /// The ordered Space ids of `output`.
    pub fn space_ids(&self, output: &str) -> Vec<SpaceId> {
        self.output(output)
            .map(|o| o.spaces.iter().map(|s| s.id).collect())
            .unwrap_or_default()
    }

    /// The ordered Space names of `output` (used to assert lockstep
    /// alignment, and as the strip fallback before T-07).
    pub fn space_names(&self, output: &str) -> Vec<String> {
        self.output(output)
            .map(|o| o.spaces.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default()
    }

    /// The number of Spaces on `output`.
    pub fn space_count(&self, output: &str) -> usize {
        self.output(output).map(|o| o.spaces.len()).unwrap_or(0)
    }

    /// The active Space id of `output`.
    pub fn active_space(&self, output: &str) -> Option<SpaceId> {
        self.output(output)
            .and_then(OutputSpaces::active_space)
            .map(|space| space.id)
    }

    /// The active Space index of `output`.
    pub fn active_index(&self, output: &str) -> Option<usize> {
        self.output(output).map(|o| o.active)
    }

    /// The Space id at `index` on `output`.
    pub fn space_at(&self, output: &str, index: usize) -> Option<SpaceId> {
        self.output(output)
            .and_then(|o| o.space_at(index))
            .map(|space| space.id)
    }

    /// The output that owns `space`.
    pub fn space_output(&self, space: SpaceId) -> Option<&str> {
        self.outputs
            .iter()
            .find(|o| o.spaces.iter().any(|s| s.id == space))
            .map(|o| o.output.as_str())
    }

    /// The Space `window` is assigned to.
    pub fn window_space(&self, window: WindowId) -> Option<SpaceId> {
        self.assignments.get(&window).copied()
    }

    /// Assign `window` to `space`. Returns true if the assignment changed.
    pub fn assign_window(&mut self, window: WindowId, space: SpaceId) -> bool {
        if self.assignments.get(&window) == Some(&space) {
            return false;
        }
        self.assignments.insert(window, space);
        self.dispatch
            .push(WorkspaceEvent::window_assigned(window, space));
        true
    }

    /// Assign `window` to the Space at `index` on `output`.
    pub fn assign_window_at(
        &mut self,
        window: WindowId,
        output: &str,
        index: usize,
    ) -> Option<SpaceId> {
        let space = self.space_at(output, index)?;
        self.assign_window(window, space);
        Some(space)
    }

    /// Forget a closed window's assignment.
    pub fn forget_window(&mut self, window: WindowId) {
        self.assignments.remove(&window);
    }

    /// Move `window` to the Space at `index` on `output` (the window-menu
    /// "Move to Space" primitive, FR-5). Returns the destination Space.
    pub fn move_window(&mut self, window: WindowId, output: &str, index: usize) -> Option<SpaceId> {
        self.assign_window_at(window, output, index)
    }

    /// Remember that `app_id` belongs on Space `index` (FR-5).
    pub fn remember_app(&mut self, app_id: &str, index: usize) {
        self.app_memory.insert(app_id.to_string(), index);
    }

    /// The remembered Space index for `app_id`.
    pub fn app_space_index(&self, app_id: &str) -> Option<usize> {
        self.app_memory.get(app_id).copied()
    }

    /// The wallpaper of the active Space on `output`.
    pub fn active_wallpaper(&self, output: &str) -> Option<&Wallpaper> {
        self.output(output)
            .and_then(OutputSpaces::active_space)
            .map(|space| &space.wallpaper)
    }

    /// The wallpaper at `index` on `output`.
    pub fn wallpaper_at(&self, output: &str, index: usize) -> Option<&Wallpaper> {
        self.output(output)
            .and_then(|o| o.space_at(index))
            .map(|space| &space.wallpaper)
    }

    /// Replace the wallpaper of the Space at `index` on `output`.
    pub fn set_wallpaper(&mut self, output: &str, index: usize, wallpaper: Wallpaper) -> bool {
        let Some(output) = self.output_mut(output) else {
            return false;
        };
        let Some(space) = output.spaces.get_mut(index) else {
            return false;
        };
        space.wallpaper = wallpaper;
        true
    }

    /// Advance the active Space on **every** output in lockstep (FR-2).
    ///
    /// Returns true if anything changed. Switches clamp at the ends; the
    /// rubber-banding that makes the ends feel physical lives in the
    /// gesture pipeline (T-11), not here.
    pub fn switch_all(&mut self, delta: i32) -> bool {
        let mut changed = false;
        let mut activated = Vec::new();
        for output in self.outputs.iter_mut() {
            if output.spaces.is_empty() {
                continue;
            }
            let len = output.spaces.len() as i32;
            let next = (output.active as i32 + delta).clamp(0, len - 1) as usize;
            if next != output.active {
                output.active = next;
                changed = true;
            }
            if let Some(space) = output.active_space() {
                activated.push((output.output.clone(), next, space.id));
            }
        }
        if changed {
            for (output, index, space) in activated {
                self.dispatch
                    .push(WorkspaceEvent::activated(&output, index, space));
            }
        }
        changed
    }

    /// Activate a specific Space index on **every** output in lockstep
    /// (FR-2), clamping out-of-range requests. Returns true if anything
    /// changed.
    pub fn activate_all(&mut self, index: usize) -> bool {
        let mut changed = false;
        let mut activated = Vec::new();
        for output in self.outputs.iter_mut() {
            if output.spaces.is_empty() {
                continue;
            }
            let next = index.min(output.spaces.len() - 1);
            if next != output.active {
                output.active = next;
                changed = true;
            }
            if let Some(space) = output.active_space() {
                activated.push((output.output.clone(), next, space.id));
            }
        }
        if changed {
            for (output, index, space) in activated {
                self.dispatch
                    .push(WorkspaceEvent::activated(&output, index, space));
            }
        }
        changed
    }

    /// Insert a new empty Space at the end of every output's list (FR-1).
    pub fn create_space(&mut self) -> Option<SpaceId> {
        if self.outputs.is_empty() {
            return None;
        }
        let index = self.outputs[0].spaces.len();
        let mut spaces = Vec::with_capacity(self.outputs.len());
        for _ in 0..self.outputs.len() {
            spaces.push(make_space(&mut self.next_space_id, index, None));
        }
        let created = spaces.first().map(|space| space.id);
        let mut events = Vec::with_capacity(self.outputs.len());
        for output in self.outputs.iter_mut() {
            let space = spaces.remove(0);
            output.spaces.push(space);
            events.push(WorkspaceEvent::created(
                &output.output,
                index,
                created.unwrap(),
            ));
        }
        for event in events {
            self.dispatch.push(event);
        }
        created
    }

    /// Remove the Space at `index` from every output (FR-1). Refuses to
    /// remove the last Space or a fullscreen Space. Returns true if removed.
    pub fn remove_space(&mut self, index: usize) -> bool {
        if self.outputs.is_empty()
            || self.outputs.iter().any(|o| {
                o.spaces.len() <= 1 || o.spaces.get(index).is_some_and(Space::is_fullscreen)
            })
        {
            return false;
        }
        let mut removed = None;
        let mut events = Vec::with_capacity(self.outputs.len());
        for output in self.outputs.iter_mut() {
            let space = output.spaces.remove(index);
            if removed.is_none() {
                removed = Some(space.id);
            }
            output.clamp_active();
            events.push(WorkspaceEvent::removed(&output.output, space.id));
        }
        for event in events {
            self.dispatch.push(event);
        }
        removed.is_some()
    }

    /// Reorder the Space at `from` to position `to` on every output
    /// (FR-1). Returns true if the order changed.
    pub fn reorder_space(&mut self, from: usize, to: usize) -> bool {
        if self.outputs.is_empty() || from == to {
            return false;
        }
        let valid = self
            .outputs
            .iter()
            .all(|o| from < o.spaces.len() && to < o.spaces.len());
        if !valid {
            return false;
        }
        let mut moved = None;
        let mut events = Vec::with_capacity(self.outputs.len());
        for output in self.outputs.iter_mut() {
            let space = output.spaces.remove(from);
            if moved.is_none() {
                moved = Some(space.id);
            }
            output.spaces.insert(to, space);
            events.push(WorkspaceEvent::reordered(
                &output.output,
                moved.unwrap(),
                to,
            ));
        }
        for event in events {
            self.dispatch.push(event);
        }
        moved.is_some()
    }

    /// Enter fullscreen: create a dedicated Space immediately after the
    /// origin Space on every output, make it active everywhere, and assign
    /// `window` to it (FR-3). Returns the new Space id.
    ///
    /// Inserting on *all* outputs (not just the window's) keeps the lockstep
    /// index alignment intact.
    pub fn enter_fullscreen(&mut self, window: WindowId, origin: SpaceId) -> Option<SpaceId> {
        if self.outputs.is_empty() {
            return None;
        }
        // The origin Space lives on exactly one output; that output owns the
        // window and therefore owns the fullscreen Space.
        let (owner, origin_index) =
            self.outputs
                .iter()
                .enumerate()
                .find_map(|(owner, output)| {
                    output
                        .spaces
                        .iter()
                        .position(|space| space.id == origin)
                        .map(|index| (owner, index))
                })?;
        let insert_at = origin_index + 1;
        let mut spaces = Vec::with_capacity(self.outputs.len());
        for _ in 0..self.outputs.len() {
            spaces.push(make_space(&mut self.next_space_id, insert_at, Some(window)));
        }
        let owner_id = spaces.get(owner).map(|space| space.id);
        let mut events = Vec::with_capacity(self.outputs.len() * 2);
        for output in self.outputs.iter_mut() {
            let space = spaces.remove(0);
            let id = space.id;
            output.spaces.insert(insert_at, space);
            output.active = insert_at;
            events.push(WorkspaceEvent::created(&output.output, insert_at, id));
            events.push(WorkspaceEvent::activated(&output.output, insert_at, id));
        }
        for event in events {
            self.dispatch.push(event);
        }
        let owner_id = owner_id?;
        self.assign_window(window, owner_id);
        Some(owner_id)
    }

    /// Leave fullscreen: remove the dedicated Space from every output and
    /// return to the origin Space (the one immediately before it), FR-3.
    /// Returns the origin Space id.
    pub fn exit_fullscreen(&mut self, window: WindowId) -> Option<SpaceId> {
        // Every output has a fullscreen Space for this window at the same
        // index; the window is assigned to the owning output's instance.
        let fs_index = self.outputs.iter().find_map(|output| {
            output
                .spaces
                .iter()
                .position(|space| space.fullscreen_for == Some(window))
        })?;
        let owner = self.assignments.get(&window).and_then(|space| {
            self.outputs
                .iter()
                .position(|output| output.spaces.iter().any(|s| s.id == *space))
        });
        let origin_index = fs_index.saturating_sub(1);
        let mut origin = None;
        let mut events = Vec::with_capacity(self.outputs.len() * 2);
        for (index, output) in self.outputs.iter_mut().enumerate() {
            if output
                .spaces
                .get(fs_index)
                .is_some_and(Space::is_fullscreen)
            {
                let space = output.spaces.remove(fs_index);
                events.push(WorkspaceEvent::removed(&output.output, space.id));
            }
            output.clamp_active();
            output.active = origin_index.min(output.spaces.len().saturating_sub(1));
            if owner == Some(index) {
                origin = output.active_space().map(|space| space.id);
            }
            if let Some(space) = output.active_space() {
                events.push(WorkspaceEvent::activated(
                    &output.output,
                    output.active,
                    space.id,
                ));
            }
        }
        for event in events {
            self.dispatch.push(event);
        }
        let origin = origin.or_else(|| {
            self.outputs
                .first()
                .and_then(OutputSpaces::active_space)
                .map(|space| space.id)
        })?;
        self.assign_window(window, origin);
        Some(origin)
    }

    /// Every Space id currently known, in output/order.
    pub fn all_space_ids(&self) -> Vec<SpaceId> {
        self.outputs
            .iter()
            .flat_map(|o| o.spaces.iter().map(|s| s.id))
            .collect()
    }

    /// The outbox of workspace events for the private protocol (T-07).
    pub fn dispatch(&mut self) -> &mut WorkspaceDispatch {
        &mut self.dispatch
    }

    /// Number of outputs.
    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    fn output(&self, name: &str) -> Option<&OutputSpaces> {
        self.outputs.iter().find(|o| o.output == name)
    }

    fn output_mut(&mut self, name: &str) -> Option<&mut OutputSpaces> {
        self.outputs.iter_mut().find(|o| o.output == name)
    }
}

/// Allocate a new [`Space`], advancing the model's id counter. A free
/// function so callers can create Spaces while holding a disjoint borrow of
/// the output list.
fn make_space(next_id: &mut u64, index: usize, fullscreen_for: Option<WindowId>) -> Space {
    let id = SpaceId(*next_id);
    *next_id += 1;
    let name = match fullscreen_for {
        Some(window) => format!("Fullscreen ({window})"),
        None => format!("Space {}", index + 1),
    };
    Space {
        id,
        name,
        wallpaper: Wallpaper::default_for(index),
        fullscreen_for,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with_two_outputs() -> WorkspaceModel {
        let mut model = WorkspaceModel::new();
        model.add_output("DP-1");
        model.add_output("HDMI-A-1");
        model.dispatch().drain();
        model
    }

    #[test]
    fn add_output_creates_three_spaces_and_is_idempotent() {
        let mut model = WorkspaceModel::new();
        model.add_output("DP-1");
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES);
        assert_eq!(model.active_index("DP-1"), Some(0));
        let ids = model.space_ids("DP-1");
        model.add_output("DP-1");
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES);
        assert_eq!(model.space_ids("DP-1"), ids, "ids must be stable");
    }

    #[test]
    fn switch_advances_every_output_in_lockstep() {
        let mut model = model_with_two_outputs();
        assert!(model.switch_all(1));
        assert_eq!(model.active_index("DP-1"), Some(1));
        assert_eq!(model.active_index("HDMI-A-1"), Some(1));
        assert!(model.switch_all(1));
        assert_eq!(model.active_index("DP-1"), Some(2));
        assert_eq!(model.active_index("HDMI-A-1"), Some(2));
    }

    #[test]
    fn switch_clamps_at_both_ends() {
        let mut model = model_with_two_outputs();
        assert!(!model.switch_all(-1), "already at the first Space");
        assert_eq!(model.active_index("DP-1"), Some(0));
        model.activate_all(INITIAL_SPACES - 1);
        assert!(!model.switch_all(1), "already at the last Space");
        assert_eq!(model.active_index("DP-1"), Some(INITIAL_SPACES - 1));
    }

    #[test]
    fn direct_activation_synchronizes_all_outputs() {
        let mut model = model_with_two_outputs();
        assert!(model.activate_all(2));
        assert_eq!(model.active_index("DP-1"), Some(2));
        assert_eq!(model.active_index("HDMI-A-1"), Some(2));
        // Out-of-range requests clamp to the last Space rather than fail.
        assert!(!model.activate_all(99), "already clamped to the last Space");
        assert_eq!(model.active_index("DP-1"), Some(INITIAL_SPACES - 1));
    }

    #[test]
    fn fullscreen_round_trip_returns_to_origin_space() {
        let mut model = model_with_two_outputs();
        model.activate_all(1);
        let origin = model.active_space("DP-1").unwrap();
        let window = WindowId(7);

        let fullscreen = model.enter_fullscreen(window, origin).unwrap();
        // A dedicated Space appears right after the origin on every output,
        // and each output's list stays the same length (lockstep alignment).
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES + 1);
        assert_eq!(model.space_count("HDMI-A-1"), INITIAL_SPACES + 1);
        assert_eq!(model.space_names("DP-1"), model.space_names("HDMI-A-1"));
        // The window's output owns the fullscreen Space and the window.
        assert_eq!(model.space_at("DP-1", 2), Some(fullscreen));
        assert_eq!(model.active_index("DP-1"), Some(2));
        assert_eq!(model.active_index("HDMI-A-1"), Some(2));
        assert_eq!(model.window_space(window), Some(fullscreen));

        let returned = model.exit_fullscreen(window).unwrap();
        assert_eq!(returned, origin);
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES);
        assert_eq!(model.space_count("HDMI-A-1"), INITIAL_SPACES);
        assert_eq!(model.active_space("DP-1"), Some(origin));
        assert_eq!(model.window_space(window), Some(origin));
    }

    #[test]
    fn fullscreen_space_can_be_removed_from_the_strip_but_not_by_remove_space() {
        let mut model = model_with_two_outputs();
        let origin = model.active_space("DP-1").unwrap();
        let fullscreen = model.enter_fullscreen(WindowId(1), origin).unwrap();
        let index = model
            .space_ids("DP-1")
            .iter()
            .position(|id| *id == fullscreen)
            .unwrap();
        assert!(
            !model.remove_space(index),
            "a fullscreen Space is lifecycle-owned"
        );
        model.exit_fullscreen(WindowId(1));
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES);
    }

    #[test]
    fn move_window_changes_assignment_and_emits_event() {
        let mut model = model_with_two_outputs();
        let window = WindowId(3);
        let first = model.active_space("DP-1").unwrap();
        model.assign_window(window, first);
        model.dispatch().drain();

        let moved = model.move_window(window, "DP-1", 2).unwrap();
        assert_eq!(model.window_space(window), Some(moved));
        assert_ne!(moved, first);
        let events = model.dispatch().drain();
        assert!(events.iter().any(|event| matches!(
            event.kind,
            WorkspaceEventKind::WindowAssigned { window: w, .. } if w == WindowId(3)
        )));
    }

    #[test]
    fn app_memory_is_keyed_by_index() {
        let mut model = model_with_two_outputs();
        model.remember_app("org.example.App", 2);
        assert_eq!(model.app_space_index("org.example.App"), Some(2));
        assert_eq!(model.app_space_index("org.example.Other"), None);
    }

    #[test]
    fn detaching_an_output_migrates_its_windows_to_the_primary_active_space() {
        let mut model = model_with_two_outputs();
        model.activate_all(1);
        let primary_active = model.active_space("DP-1").unwrap();
        let hdmi_space = model.active_space("HDMI-A-1").unwrap();

        let window = WindowId(9);
        model.assign_window(window, hdmi_space);
        model.dispatch().drain();

        let migrated = model.remove_output("HDMI-A-1", Some("DP-1"));
        assert_eq!(migrated, vec![(window, primary_active)]);
        assert_eq!(model.window_space(window), Some(primary_active));
        assert!(!model.has_output("HDMI-A-1"));
        assert_eq!(model.output_count(), 1);
        assert!(model
            .dispatch()
            .drain()
            .iter()
            .any(|e| e.kind == WorkspaceEventKind::OutputRemoved));
    }

    #[test]
    fn hotplug_attach_detach_matrix_loses_no_windows() {
        let mut model = WorkspaceModel::new();
        model.add_output("DP-1");
        model.add_output("HDMI-A-1");
        model.activate_all(1);
        let primary_active = model.active_space("DP-1").unwrap();
        let secondary_active = model.active_space("HDMI-A-1").unwrap();
        let a = WindowId(1);
        let b = WindowId(2);
        model.assign_window(a, primary_active);
        model.assign_window(b, secondary_active);

        // Detach the secondary: its window migrates to the primary's
        // current Space, none is lost.
        model.remove_output("HDMI-A-1", Some("DP-1"));
        assert_eq!(model.window_space(a), Some(primary_active));
        assert_eq!(model.window_space(b), Some(primary_active));

        // Re-attach: a fresh Space list, existing assignments untouched.
        model.add_output("HDMI-A-1");
        assert_eq!(model.space_count("HDMI-A-1"), INITIAL_SPACES);
        assert_eq!(model.active_index("HDMI-A-1"), Some(0));
        for window in [a, b] {
            let space = model.window_space(window).unwrap();
            assert!(
                model.space_output(space).is_some(),
                "window {window} must still belong to a live Space"
            );
        }
    }

    #[test]
    fn detaching_the_last_output_keeps_assignments_for_reattach() {
        let mut model = WorkspaceModel::new();
        model.add_output("DP-1");
        let space = model.active_space("DP-1").unwrap();
        let window = WindowId(4);
        model.assign_window(window, space);
        model.remove_output("DP-1", None);
        assert_eq!(model.output_count(), 0);
        // The assignment survives so a re-attached output can re-home it.
        assert_eq!(model.window_space(window), Some(space));
    }

    #[test]
    fn create_remove_and_reorder_keep_lockstep_lists_aligned() {
        let mut model = model_with_two_outputs();
        let created = model.create_space().unwrap();
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES + 1);
        assert_eq!(model.space_count("HDMI-A-1"), INITIAL_SPACES + 1);
        // The new Space is the owning output's id; every output's list is
        // the same length and in the same order (names align).
        assert_eq!(model.space_at("DP-1", INITIAL_SPACES), Some(created));
        assert_eq!(model.space_names("DP-1"), model.space_names("HDMI-A-1"));

        assert!(model.reorder_space(0, 2));
        assert_eq!(model.space_names("DP-1"), model.space_names("HDMI-A-1"));
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES + 1);

        assert!(model.remove_space(0));
        assert_eq!(model.space_count("DP-1"), INITIAL_SPACES);
        assert_eq!(model.space_count("HDMI-A-1"), INITIAL_SPACES);
        assert_eq!(model.space_names("DP-1"), model.space_names("HDMI-A-1"));
    }

    #[test]
    fn last_space_cannot_be_removed() {
        let mut model = WorkspaceModel::new();
        model.add_output("DP-1");
        assert!(model.remove_space(0));
        assert!(model.remove_space(0));
        assert!(!model.remove_space(0), "the last Space must remain");
        assert_eq!(model.space_count("DP-1"), 1);
    }

    #[test]
    fn each_space_carries_its_own_wallpaper() {
        let model = model_with_two_outputs();
        let first = model.wallpaper_at("DP-1", 0).unwrap().clone();
        let second = model.wallpaper_at("DP-1", 1).unwrap().clone();
        assert_ne!(first.color, second.color);
        assert_eq!(model.active_wallpaper("DP-1").unwrap().color, first.color);
    }
}
