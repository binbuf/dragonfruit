// SPDX-License-Identifier: MIT
//! Input routing (T-02) and the compositor input engine (T-03).
//!
//! Both the nested (winit) and DRM (libinput) backends feed their events
//! through [`process_input_event`]; there is no backend-specific input
//! path. T-02's routing forwards events to the focused client and
//! implements click-to-focus ([02-compositor.md](../../docs/design/02-compositor.md));
//! T-03 adds the pieces the compositor owns:
//!
//! * the xkb keymap and Cmd/Option mapping ([`keymap`]),
//! * the global shortcut engine ([`shortcuts`]) — the sole arbiter of
//!   bindings; clients never grab keys directly,
//! * gesture recognition feeding the shared progress pipeline
//!   ([`gestures`]),
//! * hot-corner dwell detection ([`hot_corners`]),
//! * the live input settings model ([`settings`]),
//! * one outbox/audit log for every trigger ([`dispatch`]).
//!
//! Window move/resize semantics remain T-04's job; the workspace and
//! Mission Control animations that consume these progress events are T-05
//! and T-11.

pub mod action;
pub mod constraint;
pub mod dispatch;
pub mod gestures;
pub mod hot_corners;
pub mod keymap;
pub mod settings;
pub mod shortcuts;
pub mod synthetic;

// The input vocabulary is re-exported for the rest of the crate and the
// tests; not every name is used by the binary itself.
#[allow(unused_imports)]
pub use action::{GestureKind, HotCorner, InputAction, TriggerKind};
#[allow(unused_imports)]
pub use dispatch::{AppAcceleratorEvent, DispatchedAction, InputDispatch, ShellInputEvent};
#[allow(unused_imports)]
pub use gestures::{
    GestureConfig, GestureRecognizer, GestureUpdate, ProgressConfig, ProgressEvent, ProgressPhase,
    ProgressPipeline,
};
#[allow(unused_imports)]
pub use hot_corners::{HotCornerAction, HotCornerConfig, HotCornerDetector, HotCornerTrigger};
#[allow(unused_imports)]
pub use keymap::{role_held, ModifierRole, RoleMods};
#[allow(unused_imports)]
pub use settings::{AccelProfile, InputSettings, KeyboardSettings, PointerSettings, ScrollMethod};
#[allow(unused_imports)]
pub use shortcuts::{
    AppAccelerator, GrabArbiter, GrabKind, Shortcut, ShortcutEngine, ShortcutOutcome,
};

use std::time::Instant;

use smithay::backend::input::{
    self as backend_input, AbsolutePositionEvent, ButtonState, Device as DeviceTrait, Event,
    GestureBeginEvent, GestureEndEvent, GesturePinchUpdateEvent as _, GestureSwipeUpdateEvent as _,
    InputBackend, InputEvent, KeyState, KeyboardKeyEvent as _, PointerAxisEvent,
    PointerButtonEvent, PointerMotionEvent as _, ProximityState, TabletToolButtonEvent as _,
    TabletToolDescriptor, TabletToolEvent as _, TabletToolProximityEvent as _,
    TabletToolTipEvent as _, TouchEvent as _,
};
use smithay::desktop::utils::under_from_surface_tree;
use smithay::desktop::WindowSurfaceType;
use smithay::input::keyboard::FilterResult;
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent};
use smithay::input::touch::{DownEvent, MotionEvent as TouchMotionEvent, UpEvent};
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Rectangle, SERIAL_COUNTER};
use smithay::wayland::tablet_manager::{
    TabletDescriptor, TabletHandle, TabletSeatTrait, TabletToolHandle,
};

use crate::overview::{InputOwner, OverviewKind};
use crate::shell::layer::KeyboardInteraction;
use crate::state::DfState;
use crate::window::{MenuKey, WindowState};

/// The chrome (shell layer) surface under a global-space point, with the
/// surface origin and its keyboard-interaction policy.
///
/// Chrome composites above the window space, so it is hit-tested before
/// windows, topmost layer first (`overlay` > `top` > ...). Returns `None`
/// when the point is over no mapped chrome surface.
pub fn chrome_under(
    state: &DfState,
    point: Point<f64, Logical>,
) -> Option<(WlSurface, Point<i32, Logical>, KeyboardInteraction)> {
    let output = state.space.outputs().find(|output| {
        state
            .space
            .output_geometry(output)
            .is_some_and(|geometry| geometry.to_f64().contains(point))
    })?;
    let geometry = state.space.output_geometry(output)?;
    // `chrome_surfaces` is ordered bottom-to-top; iterate it in reverse so
    // the topmost layer wins the hit test.
    for chrome in state.chrome_surfaces(&output.name(), geometry).iter().rev() {
        let origin = chrome.location + geometry.loc;
        // `under_from_surface_tree` takes the point relative to the surface
        // origin and returns the origin relative to `location`; pass (0,0)
        // and add `origin` back, exactly like `Window::surface_under`.
        let local = point - origin.to_f64();
        if let Some((surface, location)) =
            under_from_surface_tree(&chrome.surface, local, (0, 0), WindowSurfaceType::ALL)
        {
            return Some((surface, location + origin, chrome.keyboard));
        }
    }
    None
}

/// The surface under a global-space point, with the surface origin.
///
/// Chrome is hit first, then popups, then windows (the space keeps stacking
/// order). The returned origin is in global space so it can be handed to
/// Smithay's pointer/touch focus directly.
pub fn surface_under(
    state: &DfState,
    point: Point<f64, Logical>,
) -> Option<(WlSurface, Point<i32, Logical>)> {
    if let Some((surface, location, _)) = chrome_under(state, point) {
        return Some((surface, location));
    }
    // Hit-testing transfers explicitly while the overview owns input: the
    // window space is not hit-tested until ownership returns (T-11 FR-6).
    if state.overview_input_owner() == InputOwner::Overview {
        return None;
    }
    let (window, window_loc) = state.space.element_under(point)?;
    let (surface, location) =
        window.surface_under(point - window_loc.to_f64(), WindowSurfaceType::ALL)?;
    Some((surface, window_loc + location))
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
    B::TabletToolProximityEvent: backend_input::TabletToolProximityEvent<B>,
    B::TabletToolAxisEvent: backend_input::TabletToolAxisEvent<B>,
    B::TabletToolTipEvent: backend_input::TabletToolTipEvent<B>,
    B::TabletToolButtonEvent: backend_input::TabletToolButtonEvent<B>,
{
    match event {
        InputEvent::DeviceAdded { device } => {
            if device.has_capability(backend_input::DeviceCapability::TabletTool) {
                // Tablets are first-class from the start (FR-1): register
                // the device so clients can enumerate it.
                let _ = ensure_tablet(state, &TabletDescriptor::from(&device));
            }
        }
        InputEvent::DeviceRemoved { device } => {
            if device.has_capability(backend_input::DeviceCapability::TabletTool) {
                let tablet_seat = state.seat.tablet_seat();
                tablet_seat.remove_tablet(&TabletDescriptor::from(&device));
            }
        }
        InputEvent::Keyboard { event } => {
            let Some(keyboard) = state.seat.get_keyboard() else {
                return;
            };
            let keycode = event.key_code();
            let key_state = event.state();
            let serial = SERIAL_COUNTER.next_serial();
            let time = event.time_msec();
            let _: Option<()> = keyboard.input(
                state,
                keycode,
                key_state,
                serial,
                time,
                |data, mods, keysym| {
                    // Layout-agnostic base symbol so Cmd+Shift+3 matches
                    // the `3` binding, not `#`.
                    let sym = keysym
                        .raw_latin_sym_or_raw_current_sym()
                        .unwrap_or_else(|| keysym.modified_sym())
                        .raw();
                    // An open window menu is modal (T-01.4): it consumes
                    // every key so a shortcut cannot fire underneath it. The
                    // menu keys navigate/activate/dismiss it.
                    if data.window_menu.is_some() {
                        if key_state == KeyState::Pressed {
                            if let Some(menu_key) = MenuKey::from_keysym(sym) {
                                data.window_menu_key(menu_key);
                            }
                        }
                        return FilterResult::Intercept(());
                    }
                    if key_state == KeyState::Released {
                        if data.shortcuts.is_active(sym) {
                            data.shortcuts.note_release(sym);
                            return FilterResult::Intercept(());
                        }
                        return FilterResult::Forward;
                    }
                    // Ignore auto-repeat of an intercepted shortcut: hold
                    // must not re-trigger (T-12 FR-1).
                    if data.shortcuts.is_active(sym) {
                        return FilterResult::Intercept(());
                    }
                    match data.shortcuts.resolve(mods, sym) {
                        Some(ShortcutOutcome::System(action)) => {
                            data.shortcuts.note_press(sym);
                            data.dispatch_input_action(
                                action,
                                TriggerKind::Keyboard,
                                serial.into(),
                            );
                            FilterResult::Intercept(())
                        }
                        Some(ShortcutOutcome::Application {
                            app_id,
                            accelerator_id,
                        }) => {
                            data.shortcuts.note_press(sym);
                            data.input_dispatch.app_accelerator(
                                &app_id,
                                &accelerator_id,
                                TriggerKind::Keyboard,
                                serial.into(),
                            );
                            FilterResult::Intercept(())
                        }
                        None => FilterResult::Forward,
                    }
                },
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
                    serial: SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
            pointer.frame(state);
            update_hot_corners(state, location);
            if state.update_titlebar_hover(location) {
                state.needs_redraw = true;
            }
            if state.window_menu_open() {
                state.window_menu_hover(location);
            }
            state.notify_activity();
        }
        InputEvent::PointerMotionAbsolute { event } => {
            let Some(output) = state.space.outputs().next().cloned() else {
                return;
            };
            let Some(geometry) = state.space.output_geometry(&output) else {
                return;
            };
            // `position_transformed` is backend-agnostic: winit's
            // `CursorMoved` is window pixels, libinput reports
            // device-normalized coordinates, and each backend converts to
            // output pixels here. (Treating `position()` as normalized broke
            // nested pointer input: winit hands back pixels, so every motion
            // landed far outside the output and hit nothing.)
            let location = if geometry.size.w > 0 && geometry.size.h > 0 {
                let position = event.position_transformed(geometry.size);
                Point::from((
                    geometry.loc.x as f64 + position.x,
                    geometry.loc.y as f64 + position.y,
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
                    serial: SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
            pointer.frame(state);
            update_hot_corners(state, location);
            if state.update_titlebar_hover(location) {
                state.needs_redraw = true;
            }
            if state.window_menu_open() {
                state.window_menu_hover(location);
            }
            state.notify_activity();
        }
        InputEvent::PointerButton { event } => {
            let pointer = state.seat.get_pointer().unwrap();
            let serial = SERIAL_COUNTER.next_serial();
            let button_event = ButtonEvent {
                serial,
                time: event.time_msec(),
                button: event.button_code(),
                state: event.state(),
            };

            // Click-to-focus: focus never follows motion alone. A chrome
            // surface takes focus on any button press (a right-click context
            // menu, e.g. the Dock's, must be able to take keyboard focus so a
            // later click-away/focus loss can dismiss it); windows still focus
            // on left-click only (T-10 section 13).
            if button_event.state == ButtonState::Pressed {
                let location = pointer.current_location();
                // An open window menu owns pointer input (T-01.4): a press
                // inside activates a row and is consumed; a press outside
                // dismisses it and falls through to normal routing.
                if state.window_menu_open() && state.window_menu_click(location) {
                    state.notify_activity();
                    return;
                }
                if let Some((surface, _, _)) = chrome_under(state, location) {
                    // Chrome surfaces take keyboard focus only when their
                    // policy allows it (OnDemand/Exclusive); `None` ignores
                    // the click (T-07 FR-1).
                    state.focus_chrome_surface(&surface);
                } else {
                    // Right-click or Control-click on the titlebar opens the
                    // window menu (T-01.4). Like the left press, a
                    // floating/zoomed titlebar is consulted only when no
                    // client surface is under the point; a revealed
                    // fullscreen titlebar wins even when one is.
                    let is_left = button_event.button == 0x110; /* BTN_LEFT */
                    let is_right = button_event.button == 0x111; /* BTN_RIGHT */
                    let ctrl = state
                        .seat
                        .get_keyboard()
                        .map(|keyboard| keyboard.modifier_state().ctrl)
                        .unwrap_or(false);
                    if is_right || (is_left && ctrl) {
                        if let Some(window) = state.titlebar_window_at(location) {
                            let client_surface = surface_under(state, location);
                            let titlebar_wins = client_surface.is_none()
                                || state.windows.state(&window) == Some(WindowState::Fullscreen);
                            if titlebar_wins {
                                state.activate_window(&window, serial);
                                state.open_window_menu(&window, location);
                                state.needs_redraw = true;
                                state.notify_activity();
                                return;
                            }
                        }
                    }
                    if is_left {
                        let client_surface = surface_under(state, location);
                        // The SSD titlebar/controls are compositor chrome.
                        // A floating/zoomed titlebar sits above the client
                        // area, so it is consulted only when no client
                        // surface (toplevel, popup, or input region) is under
                        // the point — popups and client input keep priority.
                        // A revealed fullscreen titlebar overlays the client,
                        // so it wins even when a client surface is under
                        // (T-01.3).
                        let titlebar_window = state.titlebar_window_at(location);
                        let titlebar_wins = titlebar_window.as_ref().is_some_and(|window| {
                            client_surface.is_none()
                                || state.windows.state(window) == Some(WindowState::Fullscreen)
                        });
                        if titlebar_wins {
                            let window = titlebar_window.expect("titlebar window checked above");
                            state.activate_window(&window, serial);
                            if let Some((light, kind)) = state.titlebar_button_at(location) {
                                state.traffic_light_action(&light, kind);
                            } else if state.register_titlebar_click(&window, location) {
                                state.titlebar_double_click_action(&window);
                            } else if state.windows.state(&window) == Some(WindowState::Floating) {
                                state.begin_titlebar_move(&window, serial);
                            }
                            state.needs_redraw = true;
                            state.notify_activity();
                            return;
                        }
                        if let Some((surface, _)) = client_surface {
                            if let Some(keyboard) = state.seat.get_keyboard() {
                                keyboard.set_focus(state, Some(surface), serial);
                            }
                        }
                    }
                    // A click anywhere off the chrome (a window or empty
                    // desktop space, any button) dismisses an open chrome
                    // popup by dropping its keyboard focus, which the shell
                    // sees as `shellFocused=false` (FR-3 click-away).
                    if state.chrome_has_keyboard_focus() {
                        if let Some(keyboard) = state.seat.get_keyboard() {
                            keyboard.set_focus(state, None, serial);
                        }
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
        // Gestures are recognized by the compositor and fed into the
        // shared progress pipeline; they are not forwarded to clients
        // (the compositor owns multi-finger gestures). Unclaimed-gesture
        // pass-through is a T-05/T-11 policy refinement.
        InputEvent::GestureSwipeBegin { event } => {
            state
                .gestures
                .begin_swipe(event.fingers(), state.now_msec());
        }
        InputEvent::GestureSwipeUpdate { event } => {
            let time = state.now_msec();
            if let Some(update) = state.gestures.update_swipe(event.delta(), time) {
                drive_gesture(state, update);
            }
        }
        InputEvent::GestureSwipeEnd { event } => {
            end_gesture(state, event.cancelled());
        }
        InputEvent::GesturePinchBegin { event } => {
            state
                .gestures
                .begin_pinch(event.fingers(), state.now_msec());
        }
        InputEvent::GesturePinchUpdate { event } => {
            let time = state.now_msec();
            if let Some(update) = state
                .gestures
                .update_pinch(event.scale(), event.rotation(), time)
            {
                drive_gesture(state, update);
            }
        }
        InputEvent::GesturePinchEnd { event } => {
            end_gesture(state, event.cancelled());
        }
        // Hold gestures are not a system trigger today; drop them.
        InputEvent::GestureHoldBegin { .. } | InputEvent::GestureHoldEnd { .. } => {}
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
                    serial: SERIAL_COUNTER.next_serial(),
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
                    serial: SERIAL_COUNTER.next_serial(),
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
        InputEvent::TabletToolProximity { event } => {
            let tablet = ensure_tablet(state, &TabletDescriptor::from(&event.device()));
            let tool = ensure_tool(state, &event.tool());
            match event.state() {
                ProximityState::In => {
                    let location = touch_location(state, event.position());
                    if let Some((surface, loc)) = surface_under(state, location) {
                        tool.proximity_in(
                            location,
                            (surface, loc.to_f64()),
                            &tablet,
                            SERIAL_COUNTER.next_serial(),
                            event.time_msec(),
                        );
                    }
                }
                ProximityState::Out => tool.proximity_out(event.time_msec()),
            }
            state.notify_activity();
        }
        InputEvent::TabletToolAxis { event } => {
            let tablet = ensure_tablet(state, &TabletDescriptor::from(&event.device()));
            let tool = ensure_tool(state, &event.tool());
            let location = touch_location(state, event.position());
            let focus =
                surface_under(state, location).map(|(surface, loc)| (surface, loc.to_f64()));
            tool.motion(
                location,
                focus,
                &tablet,
                SERIAL_COUNTER.next_serial(),
                event.time_msec(),
            );
            // Pen pressure is first-class (FR-1): forward every changed
            // axis alongside the motion event.
            if event.pressure_has_changed() {
                tool.pressure(event.pressure());
            }
            if event.distance_has_changed() {
                tool.distance(event.distance());
            }
            if event.tilt_has_changed() {
                tool.tilt(event.tilt());
            }
            if event.rotation_has_changed() {
                tool.rotation(event.rotation());
            }
            if event.slider_has_changed() {
                tool.slider_position(event.slider_position());
            }
            if event.wheel_has_changed() {
                tool.wheel(event.wheel_delta(), event.wheel_delta_discrete());
            }
            state.notify_activity();
        }
        InputEvent::TabletToolTip { event } => {
            let tool = ensure_tool(state, &event.tool());
            match event.tip_state() {
                backend_input::TabletToolTipState::Down => {
                    tool.tip_down(SERIAL_COUNTER.next_serial(), event.time_msec());
                }
                backend_input::TabletToolTipState::Up => {
                    tool.tip_up(event.time_msec());
                }
            }
        }
        InputEvent::TabletToolButton { event } => {
            let tool = ensure_tool(state, &event.tool());
            tool.button(
                event.button(),
                event.button_state(),
                SERIAL_COUNTER.next_serial(),
                event.time_msec(),
            );
        }
        InputEvent::SwitchToggle { .. } => {
            state.notify_activity();
        }
        _ => {}
    }
}

/// Register (idempotently) a tablet device with the seat.
fn ensure_tablet(state: &mut DfState, descriptor: &TabletDescriptor) -> TabletHandle {
    let dh = state.display_handle.clone();
    state
        .seat
        .tablet_seat()
        .add_tablet::<DfState>(&dh, descriptor)
}

/// Register (idempotently) a tablet tool with the seat.
fn ensure_tool(state: &mut DfState, tool: &TabletToolDescriptor) -> TabletToolHandle {
    let dh = state.display_handle.clone();
    let tablet_seat = state.seat.tablet_seat();
    tablet_seat.add_tool::<DfState>(state, &dh, tool)
}

/// Feed a recognized gesture step into the single overview state machine.
fn drive_gesture(state: &mut DfState, update: GestureUpdate) {
    let Some(kind) = OverviewKind::from_action(update.action) else {
        return;
    };
    let trigger = TriggerKind::Gesture(update.kind);
    let time = state.now_msec();
    // Continue only an in-flight gesture of the same kind; a discrete
    // trigger's animation clock (or a different gesture) is cancelled so this
    // gesture owns the transition and its trigger is recorded correctly.
    let continuing = state.overview.active_is_gesture() && state.overview.kind() == Some(kind);
    if !continuing {
        for event in state.overview.begin_gesture(kind, trigger, time) {
            state.input_dispatch.progress(event);
        }
    }
    if let Some(event) = state.overview.update_gesture(update.progress_delta, time) {
        state.input_dispatch.progress(event);
    }
    // Translate the live surfaces with the gesture (T-11 Slice A).
    state.apply_overview_scene();
    state.needs_redraw = true;
}

/// Finish a gesture; on commit, apply the transition exactly once.
///
/// The recognizer is reset here unconditionally: a gesture that never
/// claimed a system action (an unclaimed finger/axis combination) has no
/// active transition, so it must still clear its recognizer state or the
/// next gesture would be evaluated against a stale one.
fn end_gesture(state: &mut DfState, cancelled: bool) {
    let time = state.now_msec();
    if cancelled {
        state.gestures.cancel();
    } else {
        let _ = state.gestures.end();
    }
    let outcome = state.overview.end_gesture(time, cancelled);
    let mut ended: Option<ProgressEvent> = None;
    for event in outcome.events {
        if event.phase == ProgressPhase::End {
            ended = Some(event);
        }
        state.input_dispatch.progress(event);
    }
    if let Some(commit) = outcome.commit {
        if let Some(event) = ended {
            state.input_dispatch.action(
                event.action,
                event.trigger,
                SERIAL_COUNTER.next_serial().into(),
            );
        }
        state.apply_overview_commit(commit);
    } else {
        // Not committed (or cancelled): the transition animates back, so the
        // scene returns to the settled active-Space layout.
        state.apply_workspace_layout();
    }
    state.needs_redraw = true;
}

/// The output geometry containing `location`, if any.
fn output_bounds_at(
    state: &DfState,
    location: Point<f64, Logical>,
) -> Option<Rectangle<i32, Logical>> {
    state.space.outputs().find_map(|output| {
        state
            .space
            .output_geometry(output)
            .filter(|geo| geo.to_f64().contains(location))
    })
}

/// Update hot-corner dwell state after pointer motion and schedule the
/// dwell timer when a corner is occupied.
fn update_hot_corners(state: &mut DfState, location: Point<f64, Logical>) {
    let Some(bounds) = output_bounds_at(state, location) else {
        state.hot_corners.reset();
        return;
    };
    if let Some(trigger) = state
        .hot_corners
        .pointer_moved(location, bounds, Instant::now())
    {
        state.dispatch_input_action(
            trigger.action,
            TriggerKind::HotCorner(trigger.corner),
            SERIAL_COUNTER.next_serial().into(),
        );
        return;
    }
    schedule_hot_corner_timer(state);
}

/// Ensure exactly one dwell timer is pending while a corner is occupied.
fn schedule_hot_corner_timer(state: &mut DfState) {
    if !state.hot_corners.is_armed() || state.hot_corner_timer.is_some() {
        return;
    }
    let dwell = state.hot_corners.dwell();
    match state
        .loop_handle
        .insert_source(Timer::from_duration(dwell), |_, _, state| {
            state.hot_corner_timer = None;
            poll_hot_corner(state);
            TimeoutAction::Drop
        }) {
        Ok(token) => state.hot_corner_timer = Some(token),
        Err(err) => eprintln!("dragonfruit-compositor: failed to arm hot-corner timer: {err}"),
    }
}

/// Re-evaluate the occupied corner when its dwell timer fires.
fn poll_hot_corner(state: &mut DfState) {
    if let Some(trigger) = state.hot_corners.poll(Instant::now()) {
        state.dispatch_input_action(
            trigger.action,
            TriggerKind::HotCorner(trigger.corner),
            SERIAL_COUNTER.next_serial().into(),
        );
    }
}

/// Arm the shared animation clock while any compositor transition is in
/// flight (T-02.1a). One reusable ~16 ms timer advances the overview's
/// discrete progress and every registered animation exactly one frame, then
/// re-arms until nothing is live. When nothing animates the timer is not
/// armed at all — zero damage, zero client wakeups (FR-2).
pub(crate) fn schedule_animation_timer(state: &mut DfState) {
    if state.animation_timer.is_some() || !state.animations_active() {
        return;
    }
    match state.loop_handle.insert_source(
        Timer::from_duration(crate::animation::FRAME_INTERVAL),
        |_, _, state| {
            state.animation_timer = None;
            poll_animations(state);
            TimeoutAction::Drop
        },
    ) {
        Ok(token) => state.animation_timer = Some(token),
        Err(err) => eprintln!("dragonfruit-compositor: failed to arm animation timer: {err}"),
    }
}

/// Advance every in-flight animation one frame and render exactly one
/// compositor frame for it.
///
/// The overview's discrete transition (keyboard/hot-corner/shell) shares the
/// pipeline a gesture drives and is advanced here; registered animations
/// (T-02.1b onward) are advanced through [`DfState::step_animations`]. One
/// tick always requests exactly one redraw, and the timer re-arms only while
/// something is still live.
fn poll_animations(state: &mut DfState) {
    let now = state.now_msec();
    if state.overview.is_discrete() {
        let outcome = state.overview.advance_discrete(now);
        for event in outcome.events {
            state.input_dispatch.progress(event);
        }
        // Translate the live surfaces with the same progress a gesture
        // produces, then apply a crossed threshold.
        state.apply_overview_scene();
        if let Some(commit) = outcome.commit {
            state.apply_overview_commit(commit);
        }
    }
    state.step_animations(now);
    state.needs_redraw = true;
    if state.animations_active() {
        schedule_animation_timer(state);
    }
}

/// libinput reports device-normalized coordinates for touch; winit reports
/// window pixels. `position()` is in Raw coordinates in both cases, so map
/// through the output geometry like absolute pointer motion.
fn touch_location(
    state: &DfState,
    position: Point<f64, smithay::utils::Raw>,
) -> Point<f64, Logical> {
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

#[cfg(test)]
mod matrix_tests {
    use super::*;
    use crate::input::gestures::ProgressPhase;
    use crate::input::hot_corners::HotCornerAction;
    use smithay::input::keyboard::xkb::keysyms;
    use smithay::input::keyboard::ModifiersState;
    use std::time::Duration;

    fn pipeline() -> ProgressPipeline {
        ProgressPipeline::new(ProgressConfig {
            rubber_band: 0.35,
            commit_progress: 0.4,
            commit_velocity: 0.8,
            discrete_duration_ms: 200,
            discrete_steps: 4,
        })
    }

    /// Keyboard and hot-corner triggers drive the discrete pipeline.
    fn keyboard_or_corner_curve(trigger: TriggerKind) -> Vec<ProgressEvent> {
        pipeline().drive_discrete(InputAction::MissionControl, trigger, 0)
    }

    /// A gesture that reaches the same progress at the same times.
    fn gesture_curve() -> Vec<ProgressEvent> {
        let mut pipeline = pipeline();
        let trigger = TriggerKind::Gesture(GestureKind::Swipe { fingers: 4 });
        let mut events = vec![pipeline.begin(InputAction::MissionControl, trigger, 0)];
        for step in 1..=4u64 {
            events.push(pipeline.set_progress(step as f64 / 4.0, step * 50).unwrap());
        }
        events.push(pipeline.end(200, false).unwrap());
        events
    }

    #[test]
    fn every_trigger_produces_the_same_progress_curve() {
        let keyboard = keyboard_or_corner_curve(TriggerKind::Keyboard);
        let corner = keyboard_or_corner_curve(TriggerKind::HotCorner(HotCorner::TopLeft));
        let gesture = gesture_curve();

        let shape = |events: &[ProgressEvent]| -> Vec<(ProgressPhase, f64, f64)> {
            events
                .iter()
                .map(|event| (event.phase, event.progress, event.velocity))
                .collect()
        };
        assert_eq!(shape(&keyboard), shape(&corner));
        assert_eq!(shape(&keyboard), shape(&gesture));

        // Every event names the same compositor action.
        for event in keyboard.iter().chain(&gesture) {
            assert_eq!(event.action, InputAction::MissionControl);
        }
    }

    #[test]
    fn keyboard_gesture_and_corner_all_resolve_to_mission_control() {
        // Keyboard: Ctrl+Up.
        let engine = ShortcutEngine::default();
        let mods = ModifiersState {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(
            engine.resolve(&mods, keysyms::KEY_Up),
            Some(ShortcutOutcome::System(InputAction::MissionControl))
        );

        // Gesture: four-finger swipe up.
        let mut recognizer = GestureRecognizer::default();
        recognizer.begin_swipe(4, 0);
        let gesture = recognizer.update_swipe((2.0, -80.0).into(), 10).unwrap();
        assert_eq!(gesture.action, InputAction::MissionControl);

        // Hot corner: dwell in the top-left.
        let mut detector = HotCornerDetector::new(HotCornerConfig {
            assignments: [
                HotCornerAction::MissionControl,
                HotCornerAction::None,
                HotCornerAction::None,
                HotCornerAction::None,
            ],
            dwell: Duration::ZERO,
            ..Default::default()
        });
        let now = Instant::now();
        let trigger = detector
            .pointer_moved(
                (1.0, 1.0).into(),
                Rectangle::new((0, 0).into(), (1920, 1080).into()),
                now,
            )
            .unwrap();
        assert_eq!(trigger.action, InputAction::MissionControl);
    }
}
