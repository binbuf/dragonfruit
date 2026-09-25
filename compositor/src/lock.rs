// SPDX-License-Identifier: MIT
//! Fail-secure session locking (T-12.3a).
//!
//! The compositor advertises `ext-session-lock-v1` (Smithay's
//! [`smithay::wayland::session_lock`] module) and owns the enforcement side:
//! while a lock is active, user input never reaches a client, no client
//! receives keyboard focus, and the lock surfaces the lock client created
//! are composited above every other surface on every output. A compositor
//! crash ends the session, so no path returns to an unlocked desktop.
//!
//! The first-party lock UI is a shell Wayland client over this protocol:
//! the shell binds `ext_session_lock_manager_v1`, requests the lock when the
//! compositor broadcasts the `lock-screen` input action, and paints a lock
//! surface on each output. Authentication (PAM) is T-12.3b; input capture and
//! kill-resistance are T-12.3c.

use std::collections::HashMap;

use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::{ImportAll, Renderer};
use smithay::desktop::Window;
use smithay::output::Output;
use smithay::utils::{IsAlive, Logical, Scale, Size};
use smithay::wayland::session_lock::LockSurface;

use crate::state::DfState;

/// The compositor's session-lock state.
///
/// `locked` is the one fail-secure flag: it is set as soon as a client
/// requests a lock (before the confirmation is sent), it survives a lock
/// client disconnect, and only `ext_session_lock_v1.unlock_and_destroy`
/// clears it.
#[derive(Debug, Default)]
pub struct LockModel {
    locked: bool,
    /// The window that held keyboard focus when the lock engaged, so it can
    /// regain focus on unlock. `None` when no window was focused.
    restore_focus: Option<Window>,
    /// Live lock surfaces keyed by the output name they cover.
    surfaces: HashMap<String, LockSurface>,
}

impl LockModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the session is currently locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Enter the locked state, remembering `focused` so the desktop can be
    /// restored on unlock. Idempotent.
    pub fn lock(&mut self, focused: Option<Window>) {
        if !self.locked {
            self.restore_focus = focused;
        }
        self.locked = true;
    }

    /// Leave the locked state and forget every lock surface. The client owns
    /// the surface resources; the compositor only stops compositing them.
    /// Returns the window to restore focus to, if it is still alive.
    pub fn unlock(&mut self) -> Option<Window> {
        self.locked = false;
        self.surfaces.clear();
        self.restore_focus.take().filter(|window| window.alive())
    }

    /// Register the lock surface covering `output`.
    pub fn insert_surface(&mut self, output: impl Into<String>, surface: LockSurface) {
        self.surfaces.insert(output.into(), surface);
    }

    /// The live lock surface covering `output`, if any.
    pub fn surface(&self, output: &str) -> Option<&LockSurface> {
        self.surfaces.get(output).filter(|surface| surface.alive())
    }

    /// Drop surfaces whose output no longer exists or whose surface is dead.
    pub fn retain_live(&mut self, outputs: &[String]) {
        self.surfaces
            .retain(|output, surface| surface.alive() && outputs.iter().any(|name| name == output));
    }

    /// The number of live lock surfaces.
    pub fn surface_count(&self) -> usize {
        self.surfaces
            .values()
            .filter(|surface| surface.alive())
            .count()
    }

    /// Whether every output has a live lock surface. An empty output list is
    /// never covered (there is nothing to lock yet).
    pub fn covers(&self, outputs: &[String]) -> bool {
        let mapped: Vec<&str> = self
            .surfaces
            .iter()
            .filter(|(_, surface)| surface.alive())
            .map(|(name, _)| name.as_str())
            .collect();
        covered_by(outputs, &mapped)
    }
}

/// Pure coverage predicate: every `outputs` name appears in `mapped`. An
/// empty output list is never covered.
pub fn covered_by(outputs: &[String], mapped: &[&str]) -> bool {
    !outputs.is_empty() && outputs.iter().all(|name| mapped.contains(&name.as_str()))
}

/// Build the lock-surface render elements for `output` (T-12.3a).
///
/// The lock surface is configured to exactly the output's logical size, so it
/// composites at the output origin and covers the whole output. Callers
/// prepend these to the front-to-back element list, i.e. above every window
/// and every chrome surface. A missing or dead surface draws nothing: the
/// compositor keeps the clear color rather than leaking a client surface
/// underneath.
pub fn lock_render_elements<R, E>(
    renderer: &mut R,
    state: &DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<E>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
    E: From<smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement<R>>,
{
    if !state.lock.is_locked() {
        return Vec::new();
    }
    let Some(surface) = state.lock.surface(output.name().as_str()) else {
        return Vec::new();
    };
    render_elements_from_surface_tree::<R, E>(
        renderer,
        surface.wl_surface(),
        (0, 0),
        scale,
        1.0,
        Kind::Unspecified,
    )
}

/// The logical size a lock surface covering `output` must be configured to.
pub fn lock_surface_size(state: &DfState, output: &Output) -> Option<Size<u32, Logical>> {
    let geometry = state.space.output_geometry(output)?;
    Some(Size::from((geometry.size.w as u32, geometry.size.h as u32)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn lock_and_unlock_toggle_the_fail_secure_flag() {
        let mut model = LockModel::new();
        assert!(!model.is_locked());
        model.lock(None);
        assert!(model.is_locked());
        // Locking again is idempotent and keeps the session locked.
        model.lock(None);
        assert!(model.is_locked());
        let restore = model.unlock();
        assert!(!model.is_locked());
        assert!(restore.is_none());
    }

    #[test]
    fn coverage_requires_every_output() {
        let outputs = names(&["HEADLESS-1", "HEADLESS-2"]);
        assert!(covered_by(&outputs, &["HEADLESS-1", "HEADLESS-2"]));
        // A missing output is not covered.
        assert!(!covered_by(&outputs, &["HEADLESS-1"]));
        // No outputs at all is never covered.
        assert!(!covered_by(&[], &["HEADLESS-1"]));
    }
}
