// SPDX-License-Identifier: MIT
//! Synthetic input harness for the headless and nested backends (T-03).
//!
//! The headless backend has no seat devices, so protocol-level tests that
//! need input (T-03 shortcuts/gestures/hot corners, T-04 move/resize, the
//! T-07 input broadcasts) had no way to drive the seat. This module is a
//! real [`InputBackend`] implementation whose events are parsed from a
//! small line protocol delivered over a `UnixDatagram` socket; the socket
//! is bound only when `DRAGONFRUIT_SYNTHETIC_INPUT` names a path and only
//! on the headless or nested backend, so it is test plumbing, not a session
//! feature. On nested it additionally lets the T-01 capture script drive the
//! live walkthrough (Dock clicks, traffic lights, titlebar menu).
//!
//! Events are **libinput-equivalent**: key codes are raw evdev codes
//! (the backend adds the xkb +8 offset, exactly like libinput/winit),
//! absolute coordinates are device-normalized `0..=1` like libinput, and
//! relative motion is in logical pixels. Every event flows through the
//! one [`crate::input::process_input_event`] router, so a synthetic
//! shortcut, gesture, or hot corner exercises the same code path as a
//! real device.
//!
//! Wire format (one command per line; a datagram may batch lines):
//!
//! ```text
//! key <evdev-code> down|up
//! motion <dx> <dy>
//! motion-abs <x-norm> <y-norm>
//! button <evdev-code> down|up
//! axis <horizontal> <vertical>
//! swipe-begin <fingers> | swipe-update <dx> <dy> | swipe-end | swipe-cancel
//! pinch-begin <fingers> | pinch-update <scale> <rotation> | pinch-end | pinch-cancel
//! touch-down <slot> <x-norm> <y-norm> | touch-motion <slot> <x-norm> <y-norm>
//! touch-up <slot> | touch-frame
//! query decorations
//! query window-menu
//! set titlebar-double-click zoom|minimize|none
//! animate-dummy <duration-ms>
//! ```
//!
//! `query decorations` is a read-only introspection aid for the SSD
//! conformance tests (T-01.1/T-01.2): instead of injecting input, the
//! compositor replies to the sender with one `decoration` line per tracked
//! window (window id, server-side flag, titlebar rect, content rect, window
//! state, assigned Space id) followed by `end`. A datagram socket that never
//! receives replies is unaffected.
//!
//! `query window-menu` (T-01.4) replies with one `window-menu` line
//! describing the open menu (or `0` when closed) followed by `end`, so the
//! conformance test can aim at rows and assert dismissal.
//!
//! `set titlebar-double-click` sets the session's `dock.titlebarDoubleClick`
//! behavior (T-01.3) so the conformance test can prove the configured
//! action, not only the default `zoom`.
//!
//! `animate-dummy <ms>` starts a scene-free animation on the shared clock
//! (T-02.1a/T-03.1a) so a headless test can assert the frame discipline:
//! exactly one compositor frame per animation frame, and a flat counter once
//! it settles.

use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};

use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisRelativeDirection, AxisSource, ButtonState, Device,
    DeviceCapability, Event, GestureBeginEvent, GestureEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, InputBackend, InputEvent, KeyState, KeyboardKeyEvent, Keycode,
    PointerAxisEvent, PointerButtonEvent, PointerMotionAbsoluteEvent, PointerMotionEvent,
    TouchCancelEvent, TouchDownEvent, TouchEvent, TouchFrameEvent, TouchMotionEvent, TouchSlot,
    TouchUpEvent, UnusedEvent,
};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, Mode, PostAction};

use crate::state::DfState;
use crate::window::TitlebarDoubleClick;

/// Marker type defining the synthetic [`InputBackend`] types.
#[derive(Debug)]
pub struct SyntheticInputBackend;

/// The single virtual device every synthetic event is attributed to.
///
/// The seat capabilities are created by [`crate::backend::add_seat_capabilities`]
/// for every backend; this device only labels the event source and is
/// reported as capable of every class the harness can inject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntheticDevice;

impl Device for SyntheticDevice {
    fn id(&self) -> String {
        "dragonfruit-synthetic".into()
    }

    fn name(&self) -> String {
        "Dragonfruit synthetic input".into()
    }

    fn has_capability(&self, capability: DeviceCapability) -> bool {
        matches!(
            capability,
            DeviceCapability::Keyboard
                | DeviceCapability::Pointer
                | DeviceCapability::Touch
                | DeviceCapability::Gesture
                | DeviceCapability::TabletTool
        )
    }

    fn usb_id(&self) -> Option<(u32, u32)> {
        None
    }

    fn syspath(&self) -> Option<PathBuf> {
        None
    }
}

/// The synthetic event timestamp in microseconds (monotonic session clock).
fn micros(msec: u32) -> u64 {
    u64::from(msec) * 1_000
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticKeyboardEvent {
    time: u64,
    key: u32,
    state: KeyState,
    count: u32,
}

impl Event<SyntheticInputBackend> for SyntheticKeyboardEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl KeyboardKeyEvent<SyntheticInputBackend> for SyntheticKeyboardEvent {
    fn key_code(&self) -> Keycode {
        // libinput/winit both hand the xkb layer the evdev code + 8.
        (self.key + 8).into()
    }

    fn state(&self) -> KeyState {
        self.state
    }

    fn count(&self) -> u32 {
        self.count
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticPointerMotionEvent {
    time: u64,
    dx: f64,
    dy: f64,
}

impl Event<SyntheticInputBackend> for SyntheticPointerMotionEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl PointerMotionEvent<SyntheticInputBackend> for SyntheticPointerMotionEvent {
    fn delta_x(&self) -> f64 {
        self.dx
    }

    fn delta_y(&self) -> f64 {
        self.dy
    }

    fn delta_x_unaccel(&self) -> f64 {
        self.dx
    }

    fn delta_y_unaccel(&self) -> f64 {
        self.dy
    }
}

/// Absolute pointer motion with libinput's device-normalized coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticPointerMotionAbsoluteEvent {
    time: u64,
    x: f64,
    y: f64,
}

impl Event<SyntheticInputBackend> for SyntheticPointerMotionAbsoluteEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl PointerMotionAbsoluteEvent<SyntheticInputBackend> for SyntheticPointerMotionAbsoluteEvent {}

impl AbsolutePositionEvent<SyntheticInputBackend> for SyntheticPointerMotionAbsoluteEvent {
    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }

    fn x_transformed(&self, width: i32) -> f64 {
        (self.x * f64::from(width)).max(0.0)
    }

    fn y_transformed(&self, height: i32) -> f64 {
        (self.y * f64::from(height)).max(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticPointerButtonEvent {
    time: u64,
    button: u32,
    state: ButtonState,
}

impl Event<SyntheticInputBackend> for SyntheticPointerButtonEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl PointerButtonEvent<SyntheticInputBackend> for SyntheticPointerButtonEvent {
    fn button_code(&self) -> u32 {
        self.button
    }

    fn state(&self) -> ButtonState {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticPointerAxisEvent {
    time: u64,
    horizontal: Option<f64>,
    vertical: Option<f64>,
}

impl Event<SyntheticInputBackend> for SyntheticPointerAxisEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl PointerAxisEvent<SyntheticInputBackend> for SyntheticPointerAxisEvent {
    fn amount(&self, axis: Axis) -> Option<f64> {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }

    fn amount_v120(&self, _axis: Axis) -> Option<f64> {
        None
    }

    fn source(&self) -> AxisSource {
        AxisSource::Continuous
    }

    fn relative_direction(&self, _axis: Axis) -> AxisRelativeDirection {
        AxisRelativeDirection::Identical
    }
}

macro_rules! synthetic_gesture_begin {
    ($event:ident, $trait:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $event {
            time: u64,
            fingers: u32,
        }

        impl Event<SyntheticInputBackend> for $event {
            fn time(&self) -> u64 {
                self.time
            }

            fn device(&self) -> SyntheticDevice {
                SyntheticDevice
            }
        }

        impl GestureBeginEvent<SyntheticInputBackend> for $event {
            fn fingers(&self) -> u32 {
                self.fingers
            }
        }

        impl $trait<SyntheticInputBackend> for $event {}
    };
}

synthetic_gesture_begin!(SyntheticSwipeBeginEvent, GestureSwipeBeginEvent);
synthetic_gesture_begin!(SyntheticPinchBeginEvent, GesturePinchBeginEvent);

macro_rules! synthetic_gesture_end {
    ($event:ident, $trait:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $event {
            time: u64,
            cancelled: bool,
        }

        impl Event<SyntheticInputBackend> for $event {
            fn time(&self) -> u64 {
                self.time
            }

            fn device(&self) -> SyntheticDevice {
                SyntheticDevice
            }
        }

        impl GestureEndEvent<SyntheticInputBackend> for $event {
            fn cancelled(&self) -> bool {
                self.cancelled
            }
        }

        impl $trait<SyntheticInputBackend> for $event {}
    };
}

synthetic_gesture_end!(SyntheticSwipeEndEvent, GestureSwipeEndEvent);
synthetic_gesture_end!(SyntheticPinchEndEvent, GesturePinchEndEvent);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticSwipeUpdateEvent {
    time: u64,
    dx: f64,
    dy: f64,
}

impl Event<SyntheticInputBackend> for SyntheticSwipeUpdateEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl GestureSwipeUpdateEvent<SyntheticInputBackend> for SyntheticSwipeUpdateEvent {
    fn delta_x(&self) -> f64 {
        self.dx
    }

    fn delta_y(&self) -> f64 {
        self.dy
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticPinchUpdateEvent {
    time: u64,
    dx: f64,
    dy: f64,
    scale: f64,
    rotation: f64,
}

impl Event<SyntheticInputBackend> for SyntheticPinchUpdateEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl GesturePinchUpdateEvent<SyntheticInputBackend> for SyntheticPinchUpdateEvent {
    fn delta_x(&self) -> f64 {
        self.dx
    }

    fn delta_y(&self) -> f64 {
        self.dy
    }

    fn scale(&self) -> f64 {
        self.scale
    }

    fn rotation(&self) -> f64 {
        self.rotation
    }
}

/// A touch down/motion event (both share absolute coordinates and a slot).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticTouchEvent {
    time: u64,
    slot: u32,
    x: f64,
    y: f64,
}

impl Event<SyntheticInputBackend> for SyntheticTouchEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl TouchEvent<SyntheticInputBackend> for SyntheticTouchEvent {
    fn slot(&self) -> TouchSlot {
        Some(self.slot).into()
    }
}

impl AbsolutePositionEvent<SyntheticInputBackend> for SyntheticTouchEvent {
    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }

    fn x_transformed(&self, width: i32) -> f64 {
        (self.x * f64::from(width)).max(0.0)
    }

    fn y_transformed(&self, height: i32) -> f64 {
        (self.y * f64::from(height)).max(0.0)
    }
}

impl TouchDownEvent<SyntheticInputBackend> for SyntheticTouchEvent {}
impl TouchMotionEvent<SyntheticInputBackend> for SyntheticTouchEvent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticTouchUpEvent {
    time: u64,
    slot: u32,
}

impl Event<SyntheticInputBackend> for SyntheticTouchUpEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl TouchEvent<SyntheticInputBackend> for SyntheticTouchUpEvent {
    fn slot(&self) -> TouchSlot {
        Some(self.slot).into()
    }
}

impl TouchUpEvent<SyntheticInputBackend> for SyntheticTouchUpEvent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticTouchCancelEvent {
    time: u64,
    slot: u32,
}

impl Event<SyntheticInputBackend> for SyntheticTouchCancelEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl TouchEvent<SyntheticInputBackend> for SyntheticTouchCancelEvent {
    fn slot(&self) -> TouchSlot {
        Some(self.slot).into()
    }
}

impl TouchCancelEvent<SyntheticInputBackend> for SyntheticTouchCancelEvent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticTouchFrameEvent {
    time: u64,
}

impl Event<SyntheticInputBackend> for SyntheticTouchFrameEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> SyntheticDevice {
        SyntheticDevice
    }
}

impl TouchFrameEvent<SyntheticInputBackend> for SyntheticTouchFrameEvent {}

impl InputBackend for SyntheticInputBackend {
    type Device = SyntheticDevice;
    type KeyboardKeyEvent = SyntheticKeyboardEvent;
    type PointerAxisEvent = SyntheticPointerAxisEvent;
    type PointerButtonEvent = SyntheticPointerButtonEvent;
    type PointerMotionEvent = SyntheticPointerMotionEvent;
    type PointerMotionAbsoluteEvent = SyntheticPointerMotionAbsoluteEvent;

    type GestureSwipeBeginEvent = SyntheticSwipeBeginEvent;
    type GestureSwipeUpdateEvent = SyntheticSwipeUpdateEvent;
    type GestureSwipeEndEvent = SyntheticSwipeEndEvent;
    type GesturePinchBeginEvent = SyntheticPinchBeginEvent;
    type GesturePinchUpdateEvent = SyntheticPinchUpdateEvent;
    type GesturePinchEndEvent = SyntheticPinchEndEvent;
    type GestureHoldBeginEvent = UnusedEvent;
    type GestureHoldEndEvent = UnusedEvent;

    type TouchDownEvent = SyntheticTouchEvent;
    type TouchUpEvent = SyntheticTouchUpEvent;
    type TouchMotionEvent = SyntheticTouchEvent;
    type TouchCancelEvent = SyntheticTouchCancelEvent;
    type TouchFrameEvent = SyntheticTouchFrameEvent;

    type TabletToolAxisEvent = UnusedEvent;
    type TabletToolProximityEvent = UnusedEvent;
    type TabletToolTipEvent = UnusedEvent;
    type TabletToolButtonEvent = UnusedEvent;

    type SwitchToggleEvent = UnusedEvent;

    type SpecialEvent = UnusedEvent;
}

/// One parsed synthetic input command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyntheticCommand {
    Key {
        keycode: u32,
        state: KeyState,
    },
    PointerMotion {
        dx: f64,
        dy: f64,
    },
    PointerMotionAbsolute {
        x: f64,
        y: f64,
    },
    PointerButton {
        button: u32,
        state: ButtonState,
    },
    PointerAxis {
        horizontal: Option<f64>,
        vertical: Option<f64>,
    },
    SwipeBegin {
        fingers: u32,
    },
    SwipeUpdate {
        dx: f64,
        dy: f64,
    },
    SwipeEnd {
        cancelled: bool,
    },
    PinchBegin {
        fingers: u32,
    },
    PinchUpdate {
        scale: f64,
        rotation: f64,
    },
    PinchEnd {
        cancelled: bool,
    },
    TouchDown {
        slot: u32,
        x: f64,
        y: f64,
    },
    TouchMotion {
        slot: u32,
        x: f64,
        y: f64,
    },
    TouchUp {
        slot: u32,
    },
    TouchFrame,
    /// Read-only SSD titlebar introspection (T-01.1).
    QueryDecorations,
    /// Read-only window-menu introspection (T-01.4).
    QueryWindowMenu,
    /// Set `dock.titlebarDoubleClick` for the session (T-01.3 test plumbing).
    SetTitlebarDoubleClick(TitlebarDoubleClick),
    /// Start a scene-free animation on the shared clock (T-02.1a test
    /// plumbing; the frame-discipline calibration animation).
    AnimateDummy {
        duration_ms: u64,
    },
}

fn parse_state(token: &str) -> Result<KeyState, String> {
    match token {
        "down" | "pressed" => Ok(KeyState::Pressed),
        "up" | "released" => Ok(KeyState::Released),
        other => Err(format!("expected down|up, got {other:?}")),
    }
}

fn parse_button_state(token: &str) -> Result<ButtonState, String> {
    match parse_state(token)? {
        KeyState::Pressed => Ok(ButtonState::Pressed),
        KeyState::Released => Ok(ButtonState::Released),
    }
}

fn parse_f64(token: &str) -> Result<f64, String> {
    token
        .parse::<f64>()
        .map_err(|_| format!("expected a number, got {token:?}"))
}

fn parse_u32(token: &str) -> Result<u32, String> {
    token
        .parse::<u32>()
        .map_err(|_| format!("expected a non-negative integer, got {token:?}"))
}

/// Parse one command line (the inverse of the wire format in the module
/// docs). Unknown verbs and malformed arguments are errors, never panics:
/// a bad test line must not take down the compositor.
pub fn parse_command(line: &str) -> Result<SyntheticCommand, String> {
    let mut parts = line.split_whitespace();
    let Some(verb) = parts.next() else {
        return Err("empty command".into());
    };
    let cmd = match verb {
        "key" => {
            let keycode = parse_u32(parts.next().ok_or("key requires a keycode")?)?;
            let state = parse_state(parts.next().ok_or("key requires down|up")?)?;
            SyntheticCommand::Key { keycode, state }
        }
        "motion" => {
            let dx = parse_f64(parts.next().ok_or("motion requires dx")?)?;
            let dy = parse_f64(parts.next().ok_or("motion requires dy")?)?;
            SyntheticCommand::PointerMotion { dx, dy }
        }
        "motion-abs" => {
            let x = parse_f64(parts.next().ok_or("motion-abs requires x")?)?;
            let y = parse_f64(parts.next().ok_or("motion-abs requires y")?)?;
            SyntheticCommand::PointerMotionAbsolute { x, y }
        }
        "button" => {
            let button = parse_u32(parts.next().ok_or("button requires a code")?)?;
            let state = parse_button_state(parts.next().ok_or("button requires down|up")?)?;
            SyntheticCommand::PointerButton { button, state }
        }
        "axis" => {
            let horizontal = parse_f64(parts.next().ok_or("axis requires horizontal")?)?;
            let vertical = parse_f64(parts.next().ok_or("axis requires vertical")?)?;
            SyntheticCommand::PointerAxis {
                horizontal: Some(horizontal),
                vertical: Some(vertical),
            }
        }
        "swipe-begin" => SyntheticCommand::SwipeBegin {
            fingers: parse_u32(parts.next().ok_or("swipe-begin requires fingers")?)?,
        },
        "swipe-update" => SyntheticCommand::SwipeUpdate {
            dx: parse_f64(parts.next().ok_or("swipe-update requires dx")?)?,
            dy: parse_f64(parts.next().ok_or("swipe-update requires dy")?)?,
        },
        "swipe-end" => SyntheticCommand::SwipeEnd { cancelled: false },
        "swipe-cancel" => SyntheticCommand::SwipeEnd { cancelled: true },
        "pinch-begin" => SyntheticCommand::PinchBegin {
            fingers: parse_u32(parts.next().ok_or("pinch-begin requires fingers")?)?,
        },
        "pinch-update" => SyntheticCommand::PinchUpdate {
            scale: parse_f64(parts.next().ok_or("pinch-update requires scale")?)?,
            rotation: parts.next().map(parse_f64).transpose()?.unwrap_or(0.0),
        },
        "pinch-end" => SyntheticCommand::PinchEnd { cancelled: false },
        "pinch-cancel" => SyntheticCommand::PinchEnd { cancelled: true },
        "touch-down" => SyntheticCommand::TouchDown {
            slot: parse_u32(parts.next().ok_or("touch-down requires a slot")?)?,
            x: parse_f64(parts.next().ok_or("touch-down requires x")?)?,
            y: parse_f64(parts.next().ok_or("touch-down requires y")?)?,
        },
        "touch-motion" => SyntheticCommand::TouchMotion {
            slot: parse_u32(parts.next().ok_or("touch-motion requires a slot")?)?,
            x: parse_f64(parts.next().ok_or("touch-motion requires x")?)?,
            y: parse_f64(parts.next().ok_or("touch-motion requires y")?)?,
        },
        "touch-up" => SyntheticCommand::TouchUp {
            slot: parse_u32(parts.next().ok_or("touch-up requires a slot")?)?,
        },
        "touch-frame" => SyntheticCommand::TouchFrame,
        "animate-dummy" => SyntheticCommand::AnimateDummy {
            duration_ms: parts
                .next()
                .ok_or("animate-dummy requires a duration in ms")?
                .parse()
                .map_err(|_| "animate-dummy duration must be a non-negative integer".to_string())?,
        },
        "query" => match parts.next() {
            Some("decorations") => SyntheticCommand::QueryDecorations,
            Some("window-menu") => SyntheticCommand::QueryWindowMenu,
            _ => {
                return Err("query requires a known subject (decorations, window-menu)".into());
            }
        },
        "set" => match parts.next() {
            Some("titlebar-double-click") => {
                let value = parts
                    .next()
                    .ok_or("set titlebar-double-click requires a mode")?;
                let mode = TitlebarDoubleClick::parse(value)
                    .ok_or_else(|| format!("expected zoom|minimize|none, got {value:?}"))?;
                SyntheticCommand::SetTitlebarDoubleClick(mode)
            }
            _ => return Err("set requires a known subject (titlebar-double-click)".into()),
        },
        other => return Err(format!("unknown command {other:?}")),
    };
    if parts.next().is_some() {
        return Err(format!("trailing tokens after {verb:?}"));
    }
    Ok(cmd)
}

impl SyntheticCommand {
    /// Build the backend event for this command. `now_msec` is the session
    /// monotonic clock, used as the event timestamp.
    pub fn into_event(self, now_msec: u32) -> InputEvent<SyntheticInputBackend> {
        let time = micros(now_msec);
        match self {
            SyntheticCommand::Key { keycode, state } => InputEvent::Keyboard {
                event: SyntheticKeyboardEvent {
                    time,
                    key: keycode,
                    state,
                    count: 0,
                },
            },
            SyntheticCommand::PointerMotion { dx, dy } => InputEvent::PointerMotion {
                event: SyntheticPointerMotionEvent { time, dx, dy },
            },
            SyntheticCommand::PointerMotionAbsolute { x, y } => InputEvent::PointerMotionAbsolute {
                event: SyntheticPointerMotionAbsoluteEvent { time, x, y },
            },
            SyntheticCommand::PointerButton { button, state } => InputEvent::PointerButton {
                event: SyntheticPointerButtonEvent {
                    time,
                    button,
                    state,
                },
            },
            SyntheticCommand::PointerAxis {
                horizontal,
                vertical,
            } => InputEvent::PointerAxis {
                event: SyntheticPointerAxisEvent {
                    time,
                    horizontal,
                    vertical,
                },
            },
            SyntheticCommand::SwipeBegin { fingers } => InputEvent::GestureSwipeBegin {
                event: SyntheticSwipeBeginEvent { time, fingers },
            },
            SyntheticCommand::SwipeUpdate { dx, dy } => InputEvent::GestureSwipeUpdate {
                event: SyntheticSwipeUpdateEvent { time, dx, dy },
            },
            SyntheticCommand::SwipeEnd { cancelled } => InputEvent::GestureSwipeEnd {
                event: SyntheticSwipeEndEvent { time, cancelled },
            },
            SyntheticCommand::PinchBegin { fingers } => InputEvent::GesturePinchBegin {
                event: SyntheticPinchBeginEvent { time, fingers },
            },
            SyntheticCommand::PinchUpdate { scale, rotation } => InputEvent::GesturePinchUpdate {
                event: SyntheticPinchUpdateEvent {
                    time,
                    dx: 0.0,
                    dy: 0.0,
                    scale,
                    rotation,
                },
            },
            SyntheticCommand::PinchEnd { cancelled } => InputEvent::GesturePinchEnd {
                event: SyntheticPinchEndEvent { time, cancelled },
            },
            SyntheticCommand::TouchDown { slot, x, y } => InputEvent::TouchDown {
                event: SyntheticTouchEvent { time, slot, x, y },
            },
            SyntheticCommand::TouchMotion { slot, x, y } => InputEvent::TouchMotion {
                event: SyntheticTouchEvent { time, slot, x, y },
            },
            SyntheticCommand::TouchUp { slot } => InputEvent::TouchUp {
                event: SyntheticTouchUpEvent { time, slot },
            },
            SyntheticCommand::TouchFrame => InputEvent::TouchFrame {
                event: SyntheticTouchFrameEvent { time },
            },
            // Read-only query, never turned into an event (apply_datagram
            // intercepts it and replies to the sender).
            SyntheticCommand::QueryDecorations => {
                unreachable!("query decorations is handled by apply_datagram")
            }
            SyntheticCommand::QueryWindowMenu => {
                unreachable!("query window-menu is handled by apply_datagram")
            }
            // A settings command, not an input event.
            SyntheticCommand::SetTitlebarDoubleClick(_) => {
                unreachable!("set titlebar-double-click is handled by apply_datagram")
            }
            // A clock command, not an input event.
            SyntheticCommand::AnimateDummy { .. } => {
                unreachable!("animate-dummy is handled by apply_datagram")
            }
        }
    }
}

/// The environment variable naming the synthetic-input socket. Unset in a
/// normal session; set by headless protocol tests.
pub const ENV_SYNTHETIC_INPUT: &str = "DRAGONFRUIT_SYNTHETIC_INPUT";

/// Apply a datagram payload (one or more newline-separated commands).
///
/// Returns the number of commands applied. Malformed lines are logged and
/// skipped, never fatal.
fn apply_datagram_reply(
    state: &mut DfState,
    payload: &[u8],
    reply: Option<(&UnixDatagram, std::os::unix::net::SocketAddr)>,
) -> usize {
    let text = String::from_utf8_lossy(payload);
    let mut applied = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_command(line) {
            Ok(SyntheticCommand::QueryDecorations) => {
                if let Some((socket, peer)) = &reply {
                    let report = decoration_report(state);
                    if let Some(path) = peer.as_pathname() {
                        let _ = socket.send_to(report.as_bytes(), path);
                    }
                }
                applied += 1;
            }
            Ok(SyntheticCommand::QueryWindowMenu) => {
                if let Some((socket, peer)) = &reply {
                    let report = window_menu_report(state);
                    if let Some(path) = peer.as_pathname() {
                        let _ = socket.send_to(report.as_bytes(), path);
                    }
                }
                applied += 1;
            }
            Ok(SyntheticCommand::SetTitlebarDoubleClick(mode)) => {
                state.set_titlebar_double_click(mode);
                applied += 1;
            }
            Ok(SyntheticCommand::AnimateDummy { duration_ms }) => {
                // The clock arms its own timer; no redraw is requested here,
                // so the first rendered frame is the first animation frame.
                state.start_dummy_animation(duration_ms);
                applied += 1;
            }
            Ok(command) => {
                let now = state.now_msec() as u32;
                crate::input::process_input_event(state, command.into_event(now));
                applied += 1;
            }
            Err(err) => {
                eprintln!("dragonfruit-compositor: bad synthetic input {line:?}: {err}");
            }
        }
    }
    applied
}

/// The `query decorations` report: one line per tracked window.
///
/// `decoration <id> <server_side> <tbx> <tby> <tbw> <tbh> <cx> <cy> <cw> <ch>
/// <state> <space>` followed by `end`. The titlebar fields are zero when the
/// compositor draws no titlebar (CSD, hidden, or fullscreen); `<state>` is
/// `floating`, `zoomed`, `minimized`, or `fullscreen` (T-01.2 makes the
/// minimize/zoom transitions observable). `<space>` is the assigned Space id,
/// or `-1` when unassigned (T-01.4 Move to Space is observable through it).
fn decoration_report(state: &DfState) -> String {
    let mut out = String::new();
    for window in state.windows.windows() {
        let Some(id) = state.windows.id(window) else {
            continue;
        };
        let content = state.windows.geometry(window).unwrap_or_default();
        let element = state.titlebar_element(window);
        let ssd = element.is_some();
        let (tx, ty, tw, th) = element
            .map(|element| {
                (
                    element.titlebar.loc.x,
                    element.titlebar.loc.y,
                    element.titlebar.size.w,
                    element.titlebar.size.h,
                )
            })
            .unwrap_or((0, 0, 0, 0));
        let window_state = state
            .windows
            .state(window)
            .map(|state| state.name())
            .unwrap_or("unknown");
        let space = state
            .workspaces
            .window_space(id)
            .map(|space| space.0 as i64)
            .unwrap_or(-1);
        out.push_str(&format!(
            "decoration {} {} {tx} {ty} {tw} {th} {} {} {} {} {window_state} {space}\n",
            id.0, ssd as u32, content.loc.x, content.loc.y, content.size.w, content.size.h,
        ));
    }
    out.push_str("end\n");
    out
}

/// The `query window-menu` report (T-01.4).
///
/// `window-menu <open> <window> <x> <y> <w> <h> <highlighted> <submenu_open>
/// <submenu_highlighted> <sx> <sy> <sw> <sh>` followed by `end`. When closed
/// every numeric field is zero (highlight fields are `-1` when nothing is
/// highlighted). The `<window>` is the compositor window id the menu acts on;
/// the `s*` fields are the submenu panel rect (laid out even while closed).
fn window_menu_report(state: &DfState) -> String {
    let line = match state.window_menu.as_ref() {
        Some(menu) => format!(
            "window-menu 1 {} {} {} {} {} {} {} {} {} {} {} {}",
            menu.window.0,
            menu.rect.loc.x,
            menu.rect.loc.y,
            menu.rect.size.w,
            menu.rect.size.h,
            menu.highlighted.map(|index| index as i64).unwrap_or(-1),
            menu.open_submenu as u32,
            menu.submenu_highlighted
                .map(|index| index as i64)
                .unwrap_or(-1),
            menu.submenu_rect.loc.x,
            menu.submenu_rect.loc.y,
            menu.submenu_rect.size.w,
            menu.submenu_rect.size.h,
        ),
        None => "window-menu 0 0 0 0 0 0 -1 0 -1 0 0 0 0".to_string(),
    };
    format!("{line}\nend\n")
}

/// Bind the synthetic-input socket and insert its event source.
///
/// Only the headless and nested backends call this, and only when
/// [`ENV_SYNTHETIC_INPUT`] is set; the socket is removed by the caller at
/// teardown.
pub fn install(state: &mut DfState, path: &Path) -> Result<(), String> {
    // A stale socket from a crashed run would make bind fail; the tests
    // use a unique path per process, but be defensive.
    let _ = std::fs::remove_file(path);
    let socket = UnixDatagram::bind(path).map_err(|e| {
        format!(
            "failed to bind synthetic-input socket {}: {e}",
            path.display()
        )
    })?;
    socket
        .set_nonblocking(true)
        .map_err(|e| format!("failed to set synthetic-input socket nonblocking: {e}"))?;
    let _ = state
        .loop_handle
        .insert_source(
            Generic::new(socket, Interest::READ, Mode::Level),
            |_, socket, state| {
                let mut buf = [0u8; 4096];
                loop {
                    match socket.recv_from(&mut buf) {
                        Ok((len, peer)) => {
                            apply_datagram_reply(state, &buf[..len], Some((&*socket, peer)));
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: synthetic-input socket error: {err}"
                            );
                            break;
                        }
                    }
                }
                Ok(PostAction::Continue)
            },
        )
        .map_err(|e| format!("failed to register synthetic-input source: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_command_shape() {
        assert_eq!(
            parse_command("key 29 down").unwrap(),
            SyntheticCommand::Key {
                keycode: 29,
                state: KeyState::Pressed
            }
        );
        assert_eq!(
            parse_command("motion -4 12.5").unwrap(),
            SyntheticCommand::PointerMotion { dx: -4.0, dy: 12.5 }
        );
        assert_eq!(
            parse_command("motion-abs 0.5 0.25").unwrap(),
            SyntheticCommand::PointerMotionAbsolute { x: 0.5, y: 0.25 }
        );
        assert_eq!(
            parse_command("button 272 up").unwrap(),
            SyntheticCommand::PointerButton {
                button: 272,
                state: ButtonState::Released
            }
        );
        assert_eq!(
            parse_command("swipe-begin 4").unwrap(),
            SyntheticCommand::SwipeBegin { fingers: 4 }
        );
        assert_eq!(
            parse_command("swipe-update -20 3").unwrap(),
            SyntheticCommand::SwipeUpdate { dx: -20.0, dy: 3.0 }
        );
        assert_eq!(
            parse_command("swipe-end").unwrap(),
            SyntheticCommand::SwipeEnd { cancelled: false }
        );
        assert_eq!(
            parse_command("pinch-update 1.2 0").unwrap(),
            SyntheticCommand::PinchUpdate {
                scale: 1.2,
                rotation: 0.0
            }
        );
        assert_eq!(
            parse_command("touch-down 1 0.1 0.2").unwrap(),
            SyntheticCommand::TouchDown {
                slot: 1,
                x: 0.1,
                y: 0.2
            }
        );
        assert_eq!(
            parse_command("touch-frame").unwrap(),
            SyntheticCommand::TouchFrame
        );
        assert_eq!(
            parse_command("animate-dummy 160").unwrap(),
            SyntheticCommand::AnimateDummy { duration_ms: 160 }
        );
    }

    #[test]
    fn rejects_malformed_commands_without_panicking() {
        assert!(parse_command("").is_err());
        assert!(parse_command("key 29 sideways").is_err());
        assert!(parse_command("key").is_err());
        assert!(parse_command("motion 1").is_err());
        assert!(parse_command("motion 1 2 3").is_err());
        assert!(parse_command("teleport 1 2").is_err());
    }

    #[test]
    fn key_codes_get_the_xkb_offset() {
        let event = SyntheticCommand::Key {
            keycode: 29,
            state: KeyState::Pressed,
        }
        .into_event(0);
        let InputEvent::Keyboard { event } = event else {
            panic!("expected a keyboard event");
        };
        assert_eq!(u32::from(event.key_code()), 37);
        assert_eq!(event.state(), KeyState::Pressed);
    }
}
