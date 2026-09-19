// SPDX-License-Identifier: MIT OR Apache-2.0
//! Input routing (T-02).
//!
//! Both the nested (winit) and DRM (libinput) backends feed their events
//! through [`process_input_event`]; there is no backend-specific input
//! path. This layer forwards events to the focused client and implements
//! the click-to-focus rule ([02-compositor.md](../.docs/design/02-compositor.md)):
//! focus never follows pointer motion alone.
//!
//! Deliberately out of scope here: the global shortcut engine and keymap
//! conventions (T-03), and any window move/resize semantics (T-04).

use smithay::backend::input::{
    self as backend_input, AbsolutePositionEvent, ButtonState, Device as DeviceTrait, Event,
    GestureBeginEvent, GestureEndEvent, GesturePinchUpdateEvent as _, GestureSwipeUpdateEvent as _,
    InputBackend, InputEvent, KeyboardKeyEvent as _, PointerAxisEvent, PointerButtonEvent as _,
    PointerMotionEvent as _, TouchEvent as _,
};
use smithay::input::keyboard::FilterResult;
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, MotionEvent,
};
use smithay::input::touch::{DownEvent, MotionEvent as TouchMotionEvent, UpEvent};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Point;

use crate::state::DfState;

/// The surface under a global-space point, with the surface origin.
///
/// Popups are hit first, then windows (the space keeps stacking order).
pub fn surface_under(
    state: &DfState,
    point: Point<f64, smithay::utils::Logical>,
) -> Option<(WlSurface, Point<i32, smithay::utils::Logical>)> {
    let (window, window_loc) = state.space.element_under(point)?;
    window.surface_under(
        point - window_loc.to_f64(),
        smithay::desktop::WindowSurfaceType::ALL,
    )
}

/// Route one input event from any backend.
#[allow(clippy::too_many_lines)]
pub fn process_input_event<B: InputBackend>(state: &mut DfState, event: InputEvent<B>)
where
    B::Device: backend_input::Device,
    B::KeyboardKeyEvent: backend_input::KeyboardKeyEvent<B>,
    B::PointerMotionEvent: backend_input::PointerMotionEvent<B>,
    B::PointerMotionAbsoluteEvent: backend_input::PointerMotionAbsoluteEvent<B>,
    B::PointerButtonEvent: backend_input::PointerButtonEvent<B>,
    B::PointerAxisEvent: backend_input::PointerAxisEvent<B>,
    B::TouchDownEvent: backend_input::TouchDownEvent<B>,
    B::TouchMotionEvent: backend_input::TouchMotionEvent<B>,
    B::TouchUpEvent: backend_input::TouchUpEvent<B>,
    B::GestureSwipeBeginEvent: backend_input::GestureSwipeBeginEvent<B>,
    B::GestureSwipeUpdateEvent: backend_input::GestureSwipeUpdateEvent<B>,
    B::GestureSwipeEndEvent: backend_input::GestureSwipeEndEvent<B>,
    B::GesturePinchBeginEvent: backend_input::GesturePinchBeginEvent<B>,
    B::GesturePinchUpdateEvent: backend_input::GesturePinchUpdateEvent<B>,
    B::GesturePinchEndEvent: backend_input::GesturePinchEndEvent<B>,
    B::GestureHoldBeginEvent: backend_input::GestureHoldBeginEvent<B>,
    B::GestureHoldEndEvent: backend_input::GestureHoldEndEvent<B>,
{
    match event {
        InputEvent::DeviceAdded { device } => {
            // LED state propagation for keyboards lands with T-03.
            let _ = device.has_capability(backend_input::DeviceCapability::Keyboard);
        }
        InputEvent::DeviceRemoved { .. } => {}
        InputEvent::Keyboard { event } => {
            let Some(keyboard) = state.seat.get_keyboard() else {
                return;
            };
            let keycode = event.key_code();
            let key_state = event.state();
            // T-03 replaces this pass-through with the global shortcut
            // engine and the fixed Cmd/Option keymap conventions.
            let _: Option<()> = keyboard.input(
                state,
                keycode,
                key_state,
                smithay::utils::SERIAL_COUNTER.next_serial(),
                event.time_msec(),
                |_, _, _| FilterResult::Forward,
            );
            state.notify_activity();
        }
        InputEvent::PointerMotion { event } => {
            let pointer = state.seat.get_pointer().unwrap();
            let location = pointer.current_location() + event.delta();
            pointer.motion(
                state,
                surface_under(state, location).map(|(surface, loc)| (surface, loc.to_f64())),
                &MotionEvent {
                    location,
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
            pointer.frame(state);
            state.notify_activity();
        }
        InputEvent::PointerMotionAbsolute { event } => {
            let Some(output) = state.space.outputs().next().cloned() else {
                return;
            };
            let Some(geometry) = state.space.output_geometry(&output) else {
                return;
            };
            // winit reports window coordinates (the output starts at
            // (0,0) in nested mode); libinput reports device-normalized
            // coordinates, mapped onto the output here.
            let location = if geometry.size.w > 0 && geometry.size.h > 0 {
                let position = event.position();
                Point::from((
                    geometry.loc.x as f64 + position.x * geometry.size.w as f64,
                    geometry.loc.y as f64 + position.y * geometry.size.h as f64,
                ))
            } else {
                return;
            };
            let pointer = state.seat.get_pointer().unwrap();
            pointer.motion(
                state,
                surface_under(state, location).map(|(surface, loc)| (surface, loc.to_f64())),
                &MotionEvent {
                    location,
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
            pointer.frame(state);
            state.notify_activity();
        }
        InputEvent::PointerButton { event } => {
            let pointer = state.seat.get_pointer().unwrap();
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            let button_event = ButtonEvent {
                serial,
                time: event.time_msec(),
                button: event.button_code(),
                state: event.state(),
            };

            // Click-to-focus: focus never follows motion alone.
            if button_event.state == ButtonState::Pressed && button_event.button == 0x110
            /* BTN_LEFT */
            {
                let location = pointer.current_location();
                if let Some((surface, _)) = surface_under(state, location) {
                    if let Some(keyboard) = state.seat.get_keyboard() {
                        keyboard.set_focus(state, Some(surface), serial);
                    }
                }
            }

            pointer.button(state, &button_event);
            pointer.frame(state);
            state.notify_activity();
        }
        InputEvent::PointerAxis { event } => {
            let pointer = state.seat.get_pointer().unwrap();
            let horizontal = event.amount(backend_input::Axis::Horizontal);
            let vertical = event.amount(backend_input::Axis::Vertical);
            let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
            if let Some(amount) = horizontal {
                frame = frame.value(backend_input::Axis::Horizontal, amount);
                let discrete = event.amount_v120(backend_input::Axis::Horizontal);
                if let Some(discrete) = discrete {
                    frame = frame.v120(backend_input::Axis::Horizontal, discrete as i32);
                }
            } else {
                frame = frame.stop(backend_input::Axis::Horizontal);
            }
            if let Some(amount) = vertical {
                frame = frame.value(backend_input::Axis::Vertical, amount);
                let discrete = event.amount_v120(backend_input::Axis::Vertical);
                if let Some(discrete) = discrete {
                    frame = frame.v120(backend_input::Axis::Vertical, discrete as i32);
                }
            } else {
                frame = frame.stop(backend_input::Axis::Vertical);
            }
            pointer.axis(state, frame);
            pointer.frame(state);
            state.notify_activity();
        }
        InputEvent::GestureSwipeBegin { event } => {
            state.seat.get_pointer().unwrap().gesture_swipe_begin(
                state,
                &GestureSwipeBeginEvent {
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                    fingers: event.fingers(),
                },
            );
        }
        InputEvent::GestureSwipeUpdate { event } => {
            state.seat.get_pointer().unwrap().gesture_swipe_update(
                state,
                &GestureSwipeUpdateEvent {
                    time: event.time_msec(),
                    delta: event.delta(),
                },
            );
        }
        InputEvent::GestureSwipeEnd { event } => {
            state.seat.get_pointer().unwrap().gesture_swipe_end(
                state,
                &GestureSwipeEndEvent {
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                    cancelled: event.cancelled(),
                },
            );
        }
        InputEvent::GesturePinchBegin { event } => {
            state.seat.get_pointer().unwrap().gesture_pinch_begin(
                state,
                &GesturePinchBeginEvent {
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                    fingers: event.fingers(),
                },
            );
        }
        InputEvent::GesturePinchUpdate { event } => {
            state.seat.get_pointer().unwrap().gesture_pinch_update(
                state,
                &GesturePinchUpdateEvent {
                    time: event.time_msec(),
                    delta: event.delta(),
                    scale: event.scale(),
                    rotation: event.rotation(),
                },
            );
        }
        InputEvent::GesturePinchEnd { event } => {
            state.seat.get_pointer().unwrap().gesture_pinch_end(
                state,
                &GesturePinchEndEvent {
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                    cancelled: event.cancelled(),
                },
            );
        }
        InputEvent::GestureHoldBegin { event } => {
            state.seat.get_pointer().unwrap().gesture_hold_begin(
                state,
                &GestureHoldBeginEvent {
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                    fingers: event.fingers(),
                },
            );
        }
        InputEvent::GestureHoldEnd { event } => {
            state.seat.get_pointer().unwrap().gesture_hold_end(
                state,
                &GestureHoldEndEvent {
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                    cancelled: event.cancelled(),
                },
            );
        }
        InputEvent::TouchDown { event } => {
            let location = touch_location(state, event.position());
            let focus =
                surface_under(state, location).map(|(surface, loc)| (surface, loc.to_f64()));
            state.seat.get_touch().unwrap().down(
                state,
                focus,
                &DownEvent {
                    slot: event.slot(),
                    location,
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
            state.notify_activity();
        }
        InputEvent::TouchMotion { event } => {
            let location = touch_location(state, event.position());
            let focus =
                surface_under(state, location).map(|(surface, loc)| (surface, loc.to_f64()));
            state.seat.get_touch().unwrap().motion(
                state,
                focus,
                &TouchMotionEvent {
                    slot: event.slot(),
                    location,
                    time: event.time_msec(),
                },
            );
            state.notify_activity();
        }
        InputEvent::TouchUp { event } => {
            state.seat.get_touch().unwrap().up(
                state,
                &UpEvent {
                    slot: event.slot(),
                    serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
            state.notify_activity();
        }
        InputEvent::TouchCancel { .. } => {
            state.seat.get_touch().unwrap().cancel(state);
        }
        InputEvent::TouchFrame { .. } => {
            state.seat.get_touch().unwrap().frame(state);
        }
        // Tablet events flow through the same seat cursor for now; the
        // full tablet pipeline is part of the input settings work (T-03).
        InputEvent::TabletToolAxis { .. }
        | InputEvent::TabletToolProximity { .. }
        | InputEvent::TabletToolTip { .. }
        | InputEvent::TabletToolButton { .. }
        | InputEvent::SwitchToggle { .. } => {
            state.notify_activity();
        }
        _ => {}
    }
}

/// libinput reports device-normalized coordinates for touch; winit reports
/// window pixels. `position()` is in Raw coordinates in both cases, so map
/// through the output geometry like absolute pointer motion.
fn touch_location(
    state: &DfState,
    position: Point<f64, smithay::utils::Raw>,
) -> Point<f64, smithay::utils::Logical> {
    let Some(output) = state.space.outputs().next().cloned() else {
        return Point::from((0.0, 0.0));
    };
    let Some(geometry) = state.space.output_geometry(&output) else {
        return Point::from((0.0, 0.0));
    };
    Point::from((
        geometry.loc.x as f64 + position.x * geometry.size.w as f64,
        geometry.loc.y as f64 + position.y * geometry.size.h as f64,
    ))
}
