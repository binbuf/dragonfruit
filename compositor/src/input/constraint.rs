// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer-constraint grabs (T-03 FR-5).
//!
//! `zwp_pointer_constraints_v1` is advertised in the T-02 protocol surface,
//! but until now `PointerConstraintsHandler::new_constraint` was a no-op: a
//! client could request a lock or a confine and get no enforcement. This
//! module supplies the grab that makes a constraint real.
//!
//! Constraints are gated by [`crate::input::shortcuts::GrabArbiter`]: the
//! compositor is the sole arbiter of grabs, and only a sanctioned session
//! client (the shell/Files, authenticated with a launch token) may install
//! one. An unsanctioned request is logged and refused (FR-5) — the client
//! simply keeps moving normally.
//!
//! The grab is self-healing: on every event it re-checks that its
//! constraint still exists and is active, and unsets itself if the client
//! destroyed it or the surface went away. That avoids a stuck pointer when
//! a constraint resource is dropped without an explicit deactivate.

use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerHandle,
    PointerInnerHandle, RelativeMotionEvent,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Rectangle};
use smithay::wayland::compositor::{RectangleKind, RegionAttributes};
use smithay::wayland::pointer_constraints::{with_pointer_constraint, PointerConstraint};

use crate::state::DfState;

/// The enforced shape of a constraint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstraintGeometry {
    /// The pointer is frozen at this global location.
    Locked { location: Point<f64, Logical> },
    /// The pointer is clamped to this global rectangle.
    Confined { region: Rectangle<i32, Logical> },
}

/// A pointer grab that enforces a lock or confine constraint.
pub struct PointerConstraintGrab {
    start_data: GrabStartData<DfState>,
    surface: WlSurface,
    pointer: PointerHandle<DfState>,
    geometry: ConstraintGeometry,
}

impl PointerConstraintGrab {
    pub fn new(
        start_data: GrabStartData<DfState>,
        surface: WlSurface,
        pointer: PointerHandle<DfState>,
        geometry: ConstraintGeometry,
    ) -> Self {
        PointerConstraintGrab {
            start_data,
            surface,
            pointer,
            geometry,
        }
    }

    /// Whether the constraint this grab enforces is still alive and active.
    fn constraint_active(&self) -> bool {
        with_pointer_constraint(&self.surface, &self.pointer, |constraint| {
            constraint.is_some_and(|constraint| constraint.is_active())
        })
    }

    /// Apply the constraint to a motion event's target location.
    fn constrained_location(&self, location: Point<f64, Logical>) -> Point<f64, Logical> {
        match self.geometry {
            ConstraintGeometry::Locked { location } => location,
            ConstraintGeometry::Confined { region } => {
                let min_x = f64::from(region.loc.x);
                let min_y = f64::from(region.loc.y);
                // `Rectangle` sizes are exclusive; keep the pointer on the
                // last pixel inside the region.
                let max_x = f64::from(region.loc.x + region.size.w.max(1) - 1);
                let max_y = f64::from(region.loc.y + region.size.h.max(1) - 1);
                Point::from((
                    location.x.clamp(min_x, max_x),
                    location.y.clamp(min_y, max_y),
                ))
            }
        }
    }

    /// Drop the grab if its constraint has gone away.
    fn unset_if_stale(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        serial: smithay::utils::Serial,
        time: u32,
    ) -> bool {
        if self.constraint_active() {
            return false;
        }
        handle.unset_grab(self, data, serial, time, false);
        true
    }
}

impl PointerGrab<DfState> for PointerConstraintGrab {
    fn motion(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        _focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        if self.unset_if_stale(data, handle, event.serial, event.time) {
            let focus = handle.current_focus();
            handle.motion(data, focus, event);
            return;
        }
        let location = self.constrained_location(event.location);
        let focus = handle.current_focus();
        handle.motion(
            data,
            focus,
            &MotionEvent {
                location,
                serial: event.serial,
                time: event.time,
            },
        );
    }

    fn relative_motion(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &ButtonEvent,
    ) {
        if self.unset_if_stale(data, handle, event.serial, event.time) {
            handle.button(data, event);
            return;
        }
        handle.button(data, event);
    }

    fn axis(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn frame(&mut self, data: &mut DfState, handle: &mut PointerInnerHandle<'_, DfState>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<DfState> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut DfState) {
        // Deactivate so the client receives `unlocked`/`unconfined`. If the
        // client already destroyed the resource, the lookup is `None`.
        with_pointer_constraint(&self.surface, &self.pointer, |constraint| {
            if let Some(constraint) = constraint {
                if constraint.is_active() {
                    constraint.deactivate();
                }
            }
        });
    }
}

/// The bounding box of a region's additive rectangles, if any.
///
/// Pointer-constraint regions are simple in practice (one add rectangle);
/// subtractive rectangles are ignored, which is conservative.
pub fn region_bounds(region: &RegionAttributes) -> Option<Rectangle<i32, Logical>> {
    let mut bounds: Option<Rectangle<i32, Logical>> = None;
    for (kind, rect) in &region.rects {
        if matches!(kind, RectangleKind::Add) {
            bounds = Some(match bounds {
                Some(existing) => existing.merge(*rect),
                None => *rect,
            });
        }
    }
    bounds
}

/// The constraint geometry for `surface`, if it has one.
pub fn constraint_geometry(
    surface: &WlSurface,
    pointer: &PointerHandle<DfState>,
    location: Point<f64, Logical>,
    surface_origin: Point<i32, Logical>,
) -> Option<ConstraintGeometry> {
    with_pointer_constraint(surface, pointer, |constraint| {
        let constraint = constraint?;
        match &*constraint {
            PointerConstraint::Locked(_) => Some(ConstraintGeometry::Locked { location }),
            PointerConstraint::Confined(confined) => {
                let region = confined
                    .region()
                    .and_then(region_bounds)
                    .map(|rect| Rectangle::new(rect.loc + surface_origin, rect.size));
                region.map(|region| ConstraintGeometry::Confined { region })
            }
        }
    })
}
