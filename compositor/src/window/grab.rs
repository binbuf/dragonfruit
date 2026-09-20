// SPDX-License-Identifier: MIT
//! Pointer grabs for interactive move and resize (T-04 FR-10).
//!
//! These grabs serve both SSD titlebar/edge input (T-13) and CSD clients'
//! `xdg_toplevel.move`/`resize` requests. They operate on the floating
//! geometry only: zoomed and fullscreen windows are not interactively
//! moved or resized, matching macOS behavior.

use smithay::backend::input::ButtonState;
use smithay::desktop::Window;
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
    RelativeMotionEvent,
};
use smithay::utils::{Logical, Point, Rectangle};

use super::resize::{clamp_move, resize_geometry, ResizeEdge};
use crate::state::DfState;

/// An interactive window move. The window follows the pointer, clamped to
/// its output.
pub struct MoveGrab {
    start_data: GrabStartData<DfState>,
    window: Window,
    initial_location: Point<i32, Logical>,
}

impl MoveGrab {
    pub fn new(
        start_data: GrabStartData<DfState>,
        window: Window,
        initial_location: Point<i32, Logical>,
    ) -> Self {
        MoveGrab {
            start_data,
            window,
            initial_location,
        }
    }
}

impl PointerGrab<DfState> for MoveGrab {
    fn motion(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        _focus: Option<(
            smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
            Point<f64, Logical>,
        )>,
        event: &MotionEvent,
    ) {
        let delta = event.location - self.start_data.location;
        let mut location = self.initial_location + delta.to_i32_round();
        if let Some(output) = data.output_bounds_for(&self.window) {
            let size = data
                .windows
                .floating_geometry(&self.window)
                .map(|geo| geo.size)
                .unwrap_or_default();
            location = clamp_move(location, size, output);
        }
        data.move_window(&self.window, location);
        handle.motion(data, None, event);
    }

    fn relative_motion(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        focus: Option<(
            smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
            Point<f64, Logical>,
        )>,
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
        if event.state == ButtonState::Released && event.button == self.start_data.button {
            handle.unset_grab(self, data, event.serial, event.time, true);
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

    fn unset(&mut self, _data: &mut DfState) {}
}

/// An interactive window resize from one of the eight edges.
pub struct ResizeGrab {
    start_data: GrabStartData<DfState>,
    window: Window,
    edges: ResizeEdge,
    initial_geometry: Rectangle<i32, Logical>,
}

impl ResizeGrab {
    pub fn new(
        start_data: GrabStartData<DfState>,
        window: Window,
        edges: ResizeEdge,
        initial_geometry: Rectangle<i32, Logical>,
    ) -> Self {
        ResizeGrab {
            start_data,
            window,
            edges,
            initial_geometry,
        }
    }
}

impl PointerGrab<DfState> for ResizeGrab {
    fn motion(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        _focus: Option<(
            smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
            Point<f64, Logical>,
        )>,
        event: &MotionEvent,
    ) {
        let delta = event.location - self.start_data.location;
        let constraints = data.window_size_constraints(&self.window);
        let output = data
            .output_bounds_for(&self.window)
            .unwrap_or(self.initial_geometry);
        let geometry = resize_geometry(
            self.initial_geometry,
            self.edges,
            delta,
            constraints,
            output,
        );
        data.resize_window(&self.window, geometry);
        handle.motion(data, None, event);
    }

    fn relative_motion(
        &mut self,
        data: &mut DfState,
        handle: &mut PointerInnerHandle<'_, DfState>,
        focus: Option<(
            smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
            Point<f64, Logical>,
        )>,
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
        if event.state == ButtonState::Released && event.button == self.start_data.button {
            handle.unset_grab(self, data, event.serial, event.time, true);
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

    fn unset(&mut self, _data: &mut DfState) {}
}
