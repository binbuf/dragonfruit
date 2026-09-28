// SPDX-License-Identifier: MIT
//! The input snapshot the pane renders, decoded from one raw read.
//!
//! The model owns the classification a consumer should not repeat: it groups
//! the raw libinput devices into the kinds the Settings pane talks about
//! (keyboard, mouse, trackpad, touchscreen, tablet), orders them usefully, and
//! derives the glyph and label. It is a straight projection of the tool's
//! output — libinput reports strings and switches directly — so it maps no
//! libinput enum.
//!
//! A session that runs libinput but has no devices still answers `Available`,
//! with [`InputSnapshot::present`] false; the consumer hides the item then,
//! exactly as the battery item hides on a machine with no battery. Only the
//! adapter-level `Unavailable` (no libinput at all) is a different state.

use crate::source::{InputData, InputDeviceData};

/// The kind of input device, from its libinput capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceKind {
    /// A keyboard (or a device with keyboard capability, e.g. a button pad).
    Keyboard,
    /// A relative pointing device that is not a touchpad.
    Mouse,
    /// A touchpad: pointer plus gesture, with tapping.
    Touchpad,
    /// A direct-touch device.
    Touchscreen,
    /// A drawing tablet or pen.
    Tablet,
    /// A switch (lid, tablet-mode, keypad slide).
    Switch,
    /// Anything else libinput recognizes.
    Other,
}

impl DeviceKind {
    /// Classify one raw device from its capability tokens and feature set.
    ///
    /// A touchpad is a pointer with gesture support (or, on older tools that
    /// omit the `gesture` token, a pointer that exposes `Tap-to-click`); a
    /// mouse is any other pointer.
    pub fn from_capabilities(capabilities: &[String], tap_to_click: Option<bool>) -> Self {
        let has = |token: &str| capabilities.iter().any(|capability| capability == token);
        if has("keyboard") {
            DeviceKind::Keyboard
        } else if has("touch") {
            DeviceKind::Touchscreen
        } else if has("tablet") {
            DeviceKind::Tablet
        } else if has("pointer") {
            if has("gesture") || tap_to_click.is_some() {
                DeviceKind::Touchpad
            } else {
                DeviceKind::Mouse
            }
        } else if has("switch") {
            DeviceKind::Switch
        } else {
            DeviceKind::Other
        }
    }

    /// A human label for the device kind.
    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::Keyboard => "Keyboard",
            DeviceKind::Mouse => "Mouse",
            DeviceKind::Touchpad => "Trackpad",
            DeviceKind::Touchscreen => "Touchscreen",
            DeviceKind::Tablet => "Tablet",
            DeviceKind::Switch => "Switch",
            DeviceKind::Other => "Input device",
        }
    }

    /// The design-system glyph for the kind.
    pub fn glyph(self) -> &'static str {
        match self {
            DeviceKind::Keyboard => "keyboard",
            DeviceKind::Mouse => "mouse",
            DeviceKind::Touchpad => "trackpad",
            DeviceKind::Touchscreen => "touch",
            DeviceKind::Tablet => "tablet",
            DeviceKind::Switch | DeviceKind::Other => "input",
        }
    }

    /// The ordering rank the snapshot sorts by (keyboards first, as the pane
    /// lists them).
    fn rank(self) -> u8 {
        match self {
            DeviceKind::Keyboard => 0,
            DeviceKind::Mouse => 1,
            DeviceKind::Touchpad => 2,
            DeviceKind::Touchscreen => 3,
            DeviceKind::Tablet => 4,
            DeviceKind::Switch => 5,
            DeviceKind::Other => 6,
        }
    }
}

/// One input device the session has.
#[derive(Debug, Clone, PartialEq)]
pub struct InputDevice {
    /// The human name (`AT Translated Set 2 keyboard`, …).
    pub name: String,
    /// The event node (`/dev/input/event3`).
    pub kernel: String,
    /// The tool's identity string (`usb:046d:c077`, …).
    pub identity: String,
    /// The physical-group id libinput assigns.
    pub group: u32,
    /// The seat line (`seat0, default`).
    pub seat: String,
    /// The device's kind.
    pub kind: DeviceKind,
    /// The capability tokens.
    pub capabilities: Vec<String>,
    /// The size in millimeters, when reported.
    pub size_mm: Option<(f64, f64)>,
    /// libinput's built-in tap-to-click default.
    pub tap_to_click: Option<bool>,
    /// libinput's built-in tap-and-drag default.
    pub tap_and_drag: Option<bool>,
    /// libinput's built-in natural-scroll default.
    pub natural_scroll: Option<bool>,
    /// libinput's built-in left-handed default.
    pub left_handed: Option<bool>,
    /// libinput's built-in disable-while-typing default.
    pub disable_while_typing: Option<bool>,
    /// Every acceleration profile the device offers.
    pub accel_profiles: Vec<String>,
    /// The default acceleration profile.
    pub default_accel_profile: Option<String>,
    /// Every scroll method the device offers.
    pub scroll_methods: Vec<String>,
    /// The default scroll method.
    pub default_scroll_method: Option<String>,
    /// Every click method the device offers.
    pub click_methods: Vec<String>,
    /// The default click method.
    pub default_click_method: Option<String>,
}

impl InputDevice {
    fn from_data(data: &InputDeviceData) -> Self {
        let kind = DeviceKind::from_capabilities(&data.capabilities, data.tap_to_click);
        InputDevice {
            name: data.name.clone(),
            kernel: data.kernel.clone(),
            identity: data.identity.clone(),
            group: data.group,
            seat: data.seat.clone(),
            kind,
            capabilities: data.capabilities.clone(),
            size_mm: data.size_mm,
            tap_to_click: data.tap_to_click,
            tap_and_drag: data.tap_and_drag,
            natural_scroll: data.natural_scroll,
            left_handed: data.left_handed,
            disable_while_typing: data.disable_while_typing,
            accel_profiles: data.accel_profiles.clone(),
            default_accel_profile: data.default_accel_profile.clone(),
            scroll_methods: data.scroll_methods.clone(),
            default_scroll_method: data.default_scroll_method.clone(),
            click_methods: data.click_methods.clone(),
            default_click_method: data.default_click_method.clone(),
        }
    }

    /// Whether the device is a keyboard.
    pub fn is_keyboard(&self) -> bool {
        self.kind == DeviceKind::Keyboard
    }

    /// Whether the device is a relative pointer (mouse or touchpad).
    pub fn is_pointer(&self) -> bool {
        matches!(self.kind, DeviceKind::Mouse | DeviceKind::Touchpad)
    }

    /// Whether the device is a touchpad.
    pub fn is_touchpad(&self) -> bool {
        self.kind == DeviceKind::Touchpad
    }

    /// Whether the device is a mouse.
    pub fn is_mouse(&self) -> bool {
        self.kind == DeviceKind::Mouse
    }

    /// The seat without its `default` qualifier (`seat0`).
    pub fn seat_name(&self) -> &str {
        self.seat.split(',').next().unwrap_or("").trim()
    }
}

/// A device that appeared or disappeared between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceChange {
    /// A device the previous snapshot did not have.
    Added { name: String, kernel: String },
    /// A device the new snapshot no longer has.
    Removed { name: String, kernel: String },
}

impl DeviceChange {
    /// The device's name, for both change kinds.
    pub fn name(&self) -> &str {
        match self {
            DeviceChange::Added { name, .. } | DeviceChange::Removed { name, .. } => name,
        }
    }

    /// The device's kernel node, for both change kinds.
    pub fn kernel(&self) -> &str {
        match self {
            DeviceChange::Added { kernel, .. } | DeviceChange::Removed { kernel, .. } => kernel,
        }
    }

    /// Whether the device appeared.
    pub fn is_added(&self) -> bool {
        matches!(self, DeviceChange::Added { .. })
    }

    /// Whether the device disappeared.
    pub fn is_removed(&self) -> bool {
        matches!(self, DeviceChange::Removed { .. })
    }
}

/// The input inventory a pane or status slot renders.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputSnapshot {
    /// Every recognized device, keyboards then pointers then the rest, each
    /// group ordered by name.
    pub devices: Vec<InputDevice>,
}

impl InputSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &InputData) -> Self {
        let mut devices: Vec<InputDevice> =
            data.devices.iter().map(InputDevice::from_data).collect();
        devices.sort_by(|left, right| {
            left.kind
                .rank()
                .cmp(&right.kind.rank())
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.kernel.cmp(&right.kernel))
        });
        InputSnapshot { devices }
    }

    /// Every recognized device.
    pub fn devices(&self) -> &[InputDevice] {
        &self.devices
    }

    /// Whether the session has any recognized device. The item hides when
    /// there is none, even if libinput is present.
    pub fn present(&self) -> bool {
        !self.devices.is_empty()
    }

    /// The device count.
    pub fn count(&self) -> usize {
        self.devices.len()
    }

    /// The keyboards.
    pub fn keyboards(&self) -> Vec<&InputDevice> {
        self.devices
            .iter()
            .filter(|device| device.is_keyboard())
            .collect()
    }

    /// The relative pointers (mice and touchpads).
    pub fn pointers(&self) -> Vec<&InputDevice> {
        self.devices
            .iter()
            .filter(|device| device.is_pointer())
            .collect()
    }

    /// The touchpads.
    pub fn touchpads(&self) -> Vec<&InputDevice> {
        self.devices
            .iter()
            .filter(|device| device.is_touchpad())
            .collect()
    }

    /// The mice.
    pub fn mice(&self) -> Vec<&InputDevice> {
        self.devices
            .iter()
            .filter(|device| device.is_mouse())
            .collect()
    }

    /// The device with the kernel node `kernel`, when present.
    pub fn device_at(&self, kernel: &str) -> Option<&InputDevice> {
        self.devices.iter().find(|device| device.kernel == kernel)
    }

    /// The devices that appeared or disappeared relative to `previous`,
    /// added first. This is the "event" half of the adapter: the host can
    /// diff two reads to learn what changed without polling each device.
    pub fn device_changes(&self, previous: &InputSnapshot) -> Vec<DeviceChange> {
        let mut changes = Vec::new();
        for device in &self.devices {
            if previous.device_at(&device.kernel).is_none() {
                changes.push(DeviceChange::Added {
                    name: device.name.clone(),
                    kernel: device.kernel.clone(),
                });
            }
        }
        for device in &previous.devices {
            if self.device_at(&device.kernel).is_none() {
                changes.push(DeviceChange::Removed {
                    name: device.name.clone(),
                    kernel: device.kernel.clone(),
                });
            }
        }
        changes
    }

    /// The glyph the status slot draws: the keyboard when one exists, else the
    /// pointer half of the inventory, else a generic input glyph.
    pub fn glyph(&self) -> &'static str {
        if self.devices.iter().any(InputDevice::is_keyboard) {
            "keyboard"
        } else if self.devices.iter().any(InputDevice::is_pointer) {
            "mouse"
        } else {
            "input"
        }
    }

    /// A one-line label for the pane header / status item.
    pub fn label(&self) -> String {
        if !self.present() {
            return "No input devices".to_owned();
        }
        let keyboards = self.keyboards().len();
        let pointers = self.pointers().len();
        format!("{keyboards} keyboards, {pointers} pointing devices")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> InputData {
        InputData {
            devices: vec![
                InputDeviceData {
                    name: "Optical Mouse".to_owned(),
                    kernel: "/dev/input/event7".to_owned(),
                    capabilities: vec!["pointer".to_owned()],
                    ..InputDeviceData::default()
                },
                InputDeviceData {
                    name: "AT keyboard".to_owned(),
                    kernel: "/dev/input/event3".to_owned(),
                    capabilities: vec!["keyboard".to_owned()],
                    ..InputDeviceData::default()
                },
                InputDeviceData {
                    name: "Synaptics TouchPad".to_owned(),
                    kernel: "/dev/input/event5".to_owned(),
                    capabilities: vec!["pointer".to_owned(), "gesture".to_owned()],
                    tap_to_click: Some(true),
                    size_mm: Some((97.0, 66.0)),
                    ..InputDeviceData::default()
                },
            ],
        }
    }

    #[test]
    fn devices_are_classified_and_ordered_keyboards_first() {
        let snapshot = InputSnapshot::from_data(&data());
        assert_eq!(snapshot.count(), 3);
        assert_eq!(snapshot.devices[0].name, "AT keyboard");
        assert!(snapshot.devices[0].is_keyboard());
        assert_eq!(snapshot.devices[1].kind, DeviceKind::Mouse);
        assert_eq!(snapshot.devices[2].kind, DeviceKind::Touchpad);
        assert_eq!(snapshot.keyboards().len(), 1);
        assert_eq!(snapshot.mice().len(), 1);
        assert_eq!(snapshot.touchpads().len(), 1);
        assert_eq!(snapshot.pointers().len(), 2);
    }

    #[test]
    fn a_pointer_without_gesture_is_a_mouse() {
        let kind = DeviceKind::from_capabilities(&["pointer".to_owned()], None);
        assert_eq!(kind, DeviceKind::Mouse);
    }

    #[test]
    fn an_old_tool_classifies_a_pointer_with_tapping_as_a_touchpad() {
        // No `gesture` token, but the device exposes tap-to-click.
        let kind = DeviceKind::from_capabilities(&["pointer".to_owned()], Some(true));
        assert_eq!(kind, DeviceKind::Touchpad);
    }

    #[test]
    fn an_empty_inventory_is_available_but_not_present() {
        let snapshot = InputSnapshot::from_data(&InputData::default());
        assert!(!snapshot.present());
        assert_eq!(snapshot.count(), 0);
        assert_eq!(snapshot.label(), "No input devices");
        assert_eq!(snapshot.glyph(), "input");
    }

    #[test]
    fn the_label_counts_keyboards_and_pointing_devices() {
        let snapshot = InputSnapshot::from_data(&data());
        assert_eq!(snapshot.label(), "1 keyboards, 2 pointing devices");
        assert_eq!(snapshot.glyph(), "keyboard");
    }

    #[test]
    fn a_pointer_only_session_uses_the_mouse_glyph() {
        let data = InputData {
            devices: vec![InputDeviceData {
                name: "Mouse".to_owned(),
                kernel: "/dev/input/event0".to_owned(),
                capabilities: vec!["pointer".to_owned()],
                ..InputDeviceData::default()
            }],
        };
        assert_eq!(InputSnapshot::from_data(&data).glyph(), "mouse");
    }

    #[test]
    fn device_changes_report_additions_and_removals() {
        let previous = InputSnapshot::from_data(&data());
        let mut next_data = data();
        next_data.devices.remove(1); // drop the keyboard
        next_data.devices.push(InputDeviceData {
            name: "Pen Tablet".to_owned(),
            kernel: "/dev/input/event9".to_owned(),
            capabilities: vec!["tablet".to_owned()],
            ..InputDeviceData::default()
        });
        let next = InputSnapshot::from_data(&next_data);
        let changes = next.device_changes(&previous);
        assert_eq!(changes.len(), 2);
        assert!(changes[0].is_added());
        assert_eq!(changes[0].name(), "Pen Tablet");
        assert!(changes[1].is_removed());
        assert_eq!(changes[1].name(), "AT keyboard");
        // A device that stayed put is not a change.
        assert!(!changes
            .iter()
            .any(|change| change.name() == "Optical Mouse"));
    }

    #[test]
    fn seat_name_strips_the_default_qualifier() {
        let mut raw = data();
        raw.devices[0].seat = "seat0, default".to_owned();
        let snapshot = InputSnapshot::from_data(&raw);
        assert_eq!(
            snapshot.device_at("/dev/input/event7").unwrap().seat_name(),
            "seat0"
        );
    }
}
