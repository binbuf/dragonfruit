// SPDX-License-Identifier: MIT OR Apache-2.0
//! The window state machine (T-04).
//!
//! A window is **floating, minimized, zoomed, or fullscreen**, and every
//! transition remembers the geometry to restore. Zoom and fullscreen are
//! distinct states (macOS-style): Zoom fills the usable area of the Space
//! (output minus the reserved menu-bar/Dock zones), fullscreen occupies a
//! dedicated Space (T-05). There is deliberately **no maximize state** —
//! a protocol maximize request maps to [`WindowEvent::Zoom`] (FR-2).
//!
//! The machine is pure: it stores geometries and answers transitions; the
//! compositor supplies the target rectangle for the new state and applies
//! the returned geometry to the [`Space`](smithay::desktop::Space).

use smithay::utils::{Logical, Rectangle};

/// The four reachable window states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowState {
    /// Ordinary window at its floating geometry.
    Floating,
    /// Removed from the layout; restores to the state it was minimized from.
    Minimized,
    /// Fills the usable area (Space minus menu bar and Dock) — not a Space.
    Zoomed,
    /// Occupies its own Space (T-05).
    Fullscreen,
}

impl WindowState {
    /// Whether the window participates in the normal scene layout.
    pub const fn is_visible(self) -> bool {
        !matches!(self, WindowState::Minimized)
    }

    /// A stable label for the shell protocol and logs.
    pub const fn name(self) -> &'static str {
        match self {
            WindowState::Floating => "floating",
            WindowState::Minimized => "minimized",
            WindowState::Zoomed => "zoomed",
            WindowState::Fullscreen => "fullscreen",
        }
    }
}

/// A state transition request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowEvent {
    /// Grow to fill the usable area (protocol `maximize` maps here).
    Zoom,
    /// Return a zoomed window to its floating geometry.
    Unzoom,
    /// Enter the dedicated fullscreen Space.
    EnterFullscreen,
    /// Leave fullscreen for the state it was entered from.
    ExitFullscreen,
    /// Remove from the layout, remembering the current state.
    Minimize,
    /// Return a minimized window to the state it was minimized from.
    Restore,
}

impl WindowEvent {
    /// A protocol maximize request maps to Zoom; there is no distinct
    /// maximize state (FR-2).
    pub const fn from_maximize() -> Self {
        WindowEvent::Zoom
    }
}

/// The outcome of [`WindowStateMachine::apply`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowTransition {
    /// State before the event.
    pub from: WindowState,
    /// State after the event.
    pub to: WindowState,
    /// Whether the state actually changed.
    pub changed: bool,
}

/// The state machine for one window. It owns the floating, zoomed, and
/// fullscreen geometries so restore is exact after any round-trip.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowStateMachine {
    state: WindowState,
    floating: Rectangle<i32, Logical>,
    zoomed: Rectangle<i32, Logical>,
    fullscreen: Rectangle<i32, Logical>,
    /// State to return to when leaving Fullscreen (Floating or Zoomed).
    fullscreen_restore: WindowState,
    /// State to return to when leaving Minimized.
    minimize_restore: WindowState,
}

impl WindowStateMachine {
    /// A new floating window at `floating`.
    pub fn new(floating: Rectangle<i32, Logical>) -> Self {
        WindowStateMachine {
            state: WindowState::Floating,
            floating,
            zoomed: floating,
            fullscreen: floating,
            fullscreen_restore: WindowState::Floating,
            minimize_restore: WindowState::Floating,
        }
    }

    /// The current state.
    pub fn state(&self) -> WindowState {
        self.state
    }

    /// Whether the window is part of the normal layout.
    pub fn is_visible(&self) -> bool {
        self.state.is_visible()
    }

    /// The geometry currently occupied (minimized reports its restore
    /// geometry so the shell can animate from it).
    pub fn geometry(&self) -> Rectangle<i32, Logical> {
        match self.state {
            WindowState::Floating => self.floating,
            WindowState::Zoomed => self.zoomed,
            WindowState::Fullscreen => self.fullscreen,
            WindowState::Minimized => self.restore_geometry(),
        }
    }

    /// The floating (restore) geometry.
    pub fn floating_geometry(&self) -> Rectangle<i32, Logical> {
        self.floating
    }

    /// The zoomed geometry.
    pub fn zoomed_geometry(&self) -> Rectangle<i32, Logical> {
        self.zoomed
    }

    /// The fullscreen geometry.
    pub fn fullscreen_geometry(&self) -> Rectangle<i32, Logical> {
        self.fullscreen
    }

    /// The geometry of the state a minimized window will restore to.
    pub fn restore_geometry(&self) -> Rectangle<i32, Logical> {
        match self.minimize_restore {
            WindowState::Floating => self.floating,
            WindowState::Zoomed => self.zoomed,
            WindowState::Fullscreen => self.fullscreen,
            // A minimized window is never minimized again, so this arm is
            // unreachable; fall back to floating for totality.
            WindowState::Minimized => self.floating,
        }
    }

    /// Apply `event`, using `target` as the geometry the destination state
    /// should occupy. `target` is ignored by `Minimize` and `Restore`.
    ///
    /// Returns the transition; the caller applies [`Self::geometry`] to the
    /// scene when `changed` is true.
    pub fn apply(
        &mut self,
        event: WindowEvent,
        target: Rectangle<i32, Logical>,
    ) -> WindowTransition {
        let from = self.state;
        match event {
            WindowEvent::Zoom => {
                // Zoom is only meaningful from a normal (non-fullscreen,
                // non-minimized) state; a repeated Zoom just re-fits.
                if matches!(self.state, WindowState::Floating | WindowState::Zoomed) {
                    self.zoomed = target;
                    self.state = WindowState::Zoomed;
                }
            }
            WindowEvent::Unzoom => {
                if self.state == WindowState::Zoomed {
                    self.state = WindowState::Floating;
                }
            }
            WindowEvent::EnterFullscreen => {
                if matches!(self.state, WindowState::Floating | WindowState::Zoomed) {
                    self.fullscreen = target;
                    self.fullscreen_restore = self.state;
                    self.state = WindowState::Fullscreen;
                }
            }
            WindowEvent::ExitFullscreen => {
                if self.state == WindowState::Fullscreen {
                    self.state = self.fullscreen_restore;
                }
            }
            WindowEvent::Minimize => {
                if self.state != WindowState::Minimized {
                    self.minimize_restore = self.state;
                    self.state = WindowState::Minimized;
                }
            }
            WindowEvent::Restore => {
                if self.state == WindowState::Minimized {
                    self.state = self.minimize_restore;
                }
            }
        }
        WindowTransition {
            from,
            to: self.state,
            changed: self.state != from,
        }
    }

    /// Record a new floating geometry after an interactive move/resize.
    ///
    /// The zoomed/fullscreen geometries are untouched, so leaving those
    /// states still restores the (new) floating geometry.
    pub fn set_floating_geometry(&mut self, geometry: Rectangle<i32, Logical>) {
        self.floating = geometry;
    }

    /// Update the geometry of `state` in place (e.g. the usable area
    /// changed because reserved zones or the output mode changed). Ignored
    /// for Floating (use [`Self::set_floating_geometry`]) and Minimized.
    pub fn set_state_geometry(&mut self, state: WindowState, geometry: Rectangle<i32, Logical>) {
        match state {
            WindowState::Floating => self.set_floating_geometry(geometry),
            WindowState::Zoomed => self.zoomed = geometry,
            WindowState::Fullscreen => self.fullscreen = geometry,
            WindowState::Minimized => {}
        }
    }

    /// Whether the state machine still holds `geometry` as its floating
    /// restore geometry.
    pub fn restores_to(&self, geometry: Rectangle<i32, Logical>) -> bool {
        self.floating == geometry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn machine() -> WindowStateMachine {
        WindowStateMachine::new(rect(100, 100, 800, 600))
    }

    #[test]
    fn starts_floating_at_its_geometry() {
        let m = machine();
        assert_eq!(m.state(), WindowState::Floating);
        assert_eq!(m.geometry(), rect(100, 100, 800, 600));
        assert!(m.is_visible());
    }

    #[test]
    fn maximize_is_zoom_not_a_distinct_state() {
        let mut m = machine();
        let transition = m.apply(WindowEvent::from_maximize(), rect(0, 24, 1920, 1056));
        assert_eq!(transition.to, WindowState::Zoomed);
        assert!(!WindowState::name(transition.to).contains("max"));
    }

    #[test]
    fn zoom_round_trip_restores_original_geometry() {
        let mut m = machine();
        m.apply(WindowEvent::Zoom, rect(0, 24, 1920, 1056));
        assert_eq!(m.geometry(), rect(0, 24, 1920, 1056));
        m.apply(WindowEvent::Unzoom, rect(0, 0, 0, 0));
        assert_eq!(m.state(), WindowState::Floating);
        assert_eq!(m.geometry(), rect(100, 100, 800, 600));
    }

    #[test]
    fn fullscreen_round_trip_restores_original_geometry() {
        let mut m = machine();
        m.apply(WindowEvent::EnterFullscreen, rect(0, 0, 1920, 1080));
        assert_eq!(m.geometry(), rect(0, 0, 1920, 1080));
        m.apply(WindowEvent::ExitFullscreen, rect(0, 0, 0, 0));
        assert_eq!(m.state(), WindowState::Floating);
        assert_eq!(m.geometry(), rect(100, 100, 800, 600));
    }

    #[test]
    fn zoom_then_fullscreen_returns_to_zoomed() {
        let mut m = machine();
        let zoomed = rect(0, 24, 1920, 1056);
        m.apply(WindowEvent::Zoom, zoomed);
        m.apply(WindowEvent::EnterFullscreen, rect(0, 0, 1920, 1080));
        m.apply(WindowEvent::ExitFullscreen, rect(0, 0, 0, 0));
        assert_eq!(m.state(), WindowState::Zoomed);
        assert_eq!(m.geometry(), zoomed);
        m.apply(WindowEvent::Unzoom, rect(0, 0, 0, 0));
        assert_eq!(m.geometry(), rect(100, 100, 800, 600));
    }

    #[test]
    fn minimize_restores_from_every_state() {
        for entry in [WindowEvent::Zoom, WindowEvent::EnterFullscreen] {
            let mut m = machine();
            if entry != WindowEvent::EnterFullscreen {
                m.apply(entry, rect(0, 24, 1920, 1056));
            } else {
                m.apply(entry, rect(0, 0, 1920, 1080));
            }
            let before = m.geometry();
            let before_state = m.state();
            m.apply(WindowEvent::Minimize, rect(0, 0, 0, 0));
            assert_eq!(m.state(), WindowState::Minimized);
            assert!(!m.is_visible());
            assert_eq!(m.geometry(), before);
            m.apply(WindowEvent::Restore, rect(0, 0, 0, 0));
            assert_eq!(m.state(), before_state);
            assert_eq!(m.geometry(), before);
        }
    }

    #[test]
    fn minimize_from_floating_restores_floating() {
        let mut m = machine();
        m.apply(WindowEvent::Minimize, rect(0, 0, 0, 0));
        m.apply(WindowEvent::Restore, rect(0, 0, 0, 0));
        assert_eq!(m.state(), WindowState::Floating);
        assert_eq!(m.geometry(), rect(100, 100, 800, 600));
    }

    #[test]
    fn repeated_events_are_idempotent() {
        let mut m = machine();
        let first = m.apply(WindowEvent::Zoom, rect(0, 24, 1920, 1056));
        let second = m.apply(WindowEvent::Zoom, rect(0, 24, 1920, 1056));
        assert!(first.changed);
        assert!(!second.changed);
        let min = m.apply(WindowEvent::Minimize, rect(0, 0, 0, 0));
        let min2 = m.apply(WindowEvent::Minimize, rect(0, 0, 0, 0));
        assert!(min.changed);
        assert!(!min2.changed);
    }

    #[test]
    fn floating_resize_survives_zoom_round_trip() {
        let mut m = machine();
        let moved = rect(250, 180, 640, 480);
        m.set_floating_geometry(moved);
        m.apply(WindowEvent::Zoom, rect(0, 24, 1920, 1056));
        m.apply(WindowEvent::Unzoom, rect(0, 0, 0, 0));
        assert_eq!(m.geometry(), moved);
    }

    /// A deterministic property test: any sequence of events over a random
    /// floating geometry preserves the invariant that leaving zoom or
    /// fullscreen returns to the floating geometry, and leaving minimize
    /// returns to the pre-minimize state.
    #[test]
    fn restore_geometry_is_preserved_across_random_sequences() {
        let mut seed = 0x1234_5678_9abc_def0u64;
        let mut rng = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as u32
        };
        let events = [
            WindowEvent::Zoom,
            WindowEvent::Unzoom,
            WindowEvent::EnterFullscreen,
            WindowEvent::ExitFullscreen,
            WindowEvent::Minimize,
            WindowEvent::Restore,
        ];
        for _ in 0..2000 {
            let floating = rect(
                (rng() % 2000) as i32,
                (rng() % 1200) as i32,
                (rng() % 800 + 1) as i32,
                (rng() % 600 + 1) as i32,
            );
            let mut m = WindowStateMachine::new(floating);
            for _ in 0..32 {
                let event = events[(rng() as usize) % events.len()];
                let before = m.state();
                m.apply(event, rect(0, 24, 1920, 1056));
                // A minimized window must report the geometry it will
                // restore to.
                if m.state() == WindowState::Minimized {
                    assert_eq!(m.geometry(), m.restore_geometry());
                }
                // Zoom/fullscreen must not clobber the floating restore.
                assert_eq!(m.floating_geometry(), floating);
                // Entering and immediately leaving the same state is a
                // no-op on the restore geometry.
                if event == WindowEvent::Zoom
                    && before == WindowState::Floating
                    && m.state() == WindowState::Zoomed
                {
                    m.apply(WindowEvent::Unzoom, rect(0, 0, 0, 0));
                    assert_eq!(m.geometry(), floating);
                }
            }
            assert!(m.restores_to(floating));
        }
    }
}
