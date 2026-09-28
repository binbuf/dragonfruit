// SPDX-License-Identifier: MIT
//! The live transport: libinput through the control tool it ships.
//!
//! The device handles belong to the compositor and the native libinput headers
//! are not part of the pinned toolchain, so the adapter enumerates devices the
//! same way the audio adapter reads WirePlumber: through the tool the project
//! ships, `libinput list-devices`. The parsing churn — a human-oriented output
//! that "may change at any time" per its man page — is isolated here;
//! [`InputData::from_list_devices`] is the pinned seam and is tested against a
//! captured fixture (see docs/design/adr/0124-input-device-adapter.md).
//!
//! The source constructs no process until it is called, so building an adapter
//! is free and a session without libinput still boots. A tool that cannot be
//! run, or that exits non-zero, is treated as absence; a read that returns an
//! empty device list is a present stack with no devices.

use std::process::Command;

use dragonfruit_system_adapters::AdapterError;

use crate::source::{InputData, InputDeviceData, InputSource};

/// The libinput control tool (ships with `libinput`).
pub const LIBINPUT_BIN: &str = "libinput";

impl InputData {
    /// Decode one `libinput list-devices` run into the raw input read.
    ///
    /// The output is a sequence of blank-line-separated records. Each starts
    /// with a `Device:` line and continues with `Key: value` lines. Unknown
    /// keys are ignored so a tool that grows a field keeps working; missing
    /// fields stay at their default (so `n/a` and absence are the same
    /// `None`).
    ///
    /// This is the one place the tool's spelling is known: it is pinned by the
    /// fixture and the churn stays behind the source seam.
    pub fn from_list_devices(output: &str) -> InputData {
        let mut devices = Vec::new();
        let mut current: Option<InputDeviceData> = None;

        for line in output.lines() {
            let line = line.trim_end();
            if line.trim().is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            if key == "Device" {
                if let Some(device) = current.take() {
                    devices.push(device);
                }
                current = Some(InputDeviceData {
                    name: value.to_owned(),
                    ..InputDeviceData::default()
                });
                continue;
            }
            let Some(device) = current.as_mut() else {
                continue;
            };
            match key {
                "Kernel" => device.kernel = value.to_owned(),
                "Id" => device.identity = value.to_owned(),
                "Group" => device.group = value.parse().unwrap_or(0),
                "Seat" => device.seat = value.to_owned(),
                "Size" => device.size_mm = parse_size(value),
                "Capabilities" => {
                    device.capabilities = value.split_whitespace().map(str::to_owned).collect()
                }
                "Tap-to-click" => device.tap_to_click = parse_switch(value),
                "Tap-and-drag" => device.tap_and_drag = parse_switch(value),
                "Tap drag lock" => device.tap_drag_lock = parse_switch(value),
                "Left-handed" => device.left_handed = parse_switch(value),
                "Nat.scrolling" => device.natural_scroll = parse_switch(value),
                "Middle emulation" => device.middle_emulation = parse_switch(value),
                "Disable-w-typing" => device.disable_while_typing = parse_switch(value),
                "Disable-w-trackpointing" => {
                    device.disable_while_trackpointing = parse_switch(value)
                }
                "Accel profiles" => {
                    let (options, default) = parse_options(value);
                    device.accel_profiles = options;
                    device.default_accel_profile = default;
                }
                "Scroll methods" => {
                    let (options, default) = parse_options(value);
                    device.scroll_methods = options;
                    device.default_scroll_method = default;
                }
                "Click methods" => {
                    let (options, default) = parse_options(value);
                    device.click_methods = options;
                    device.default_click_method = default;
                }
                "Rotation" => device.rotation = parse_text(value),
                _ => {}
            }
        }
        if let Some(device) = current.take() {
            devices.push(device);
        }
        InputData { devices }
    }
}

/// Parse a `97.33x66.86mm` size into millimeters, or `None` for `n/a`.
fn parse_size(value: &str) -> Option<(f64, f64)> {
    let value = value.strip_suffix("mm").unwrap_or(value).trim();
    let (width, height) = value.split_once('x')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

/// Parse an `enabled`/`disabled`/`n/a` switch. Anything else (including a
/// list, which belongs to `parse_options`) is `None`.
fn parse_switch(value: &str) -> Option<bool> {
    match value {
        "enabled" | "on" => Some(true),
        "disabled" | "off" => Some(false),
        _ => None,
    }
}

/// Parse a `n/a` text field: `None` for `n/a` or empty, else the text.
fn parse_text(value: &str) -> Option<String> {
    if value == "n/a" || value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

/// Parse an option list where the default is prefixed with `*`
/// (`*two-finger edge`, `flat *adaptive`).
///
/// Returns every option in order and the default's name, when one is marked.
fn parse_options(value: &str) -> (Vec<String>, Option<String>) {
    if value == "n/a" || value.is_empty() {
        return (Vec::new(), None);
    }
    let mut options = Vec::new();
    let mut default = None;
    for token in value.split_whitespace() {
        if let Some(name) = token.strip_prefix('*') {
            default = Some(name.to_owned());
            options.push(name.to_owned());
        } else {
            options.push(token.to_owned());
        }
    }
    (options, default)
}

/// The real libinput transport: one `libinput list-devices` run per read.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommandLibinput;

impl CommandLibinput {
    /// A source that runs the control tool on demand.
    pub const fn new() -> Self {
        CommandLibinput
    }
}

impl InputSource for CommandLibinput {
    fn read(&mut self) -> Result<Option<InputData>, AdapterError> {
        // A tool that cannot run is absence — not an error, and never a
        // startup blocker. A tool that runs but cannot open any device (the
        // common non-seat case) exits 0 with an empty list, which is a present
        // stack with no devices.
        let output = match Command::new(LIBINPUT_BIN).arg("list-devices").output() {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };
        if !output.status.success() {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(Some(InputData::from_list_devices(&text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/libinput-list-devices.txt");

    #[test]
    fn the_fixture_parses_to_the_device_inventory() {
        let data = InputData::from_list_devices(FIXTURE);
        assert_eq!(data.devices.len(), 4);
        assert_eq!(data.devices[0].name, "AT Translated Set 2 keyboard");
        assert_eq!(data.devices[0].kernel, "/dev/input/event3");
        assert_eq!(data.devices[0].group, 8);
        assert_eq!(data.devices[0].seat, "seat0, default");
        assert_eq!(data.devices[0].capabilities, vec!["keyboard".to_owned()]);
        assert_eq!(data.devices[0].tap_to_click, None);
    }

    #[test]
    fn a_touchpad_parses_its_capabilities_and_defaults() {
        let data = InputData::from_list_devices(FIXTURE);
        let touchpad = data
            .devices
            .iter()
            .find(|device| device.name.contains("Synaptics TouchPad"))
            .expect("touchpad is in the fixture");
        assert_eq!(touchpad.kernel, "/dev/input/event5");
        assert_eq!(
            touchpad.capabilities,
            vec!["pointer".to_owned(), "gesture".to_owned()]
        );
        assert_eq!(touchpad.size_mm, Some((97.33, 66.86)));
        assert_eq!(touchpad.tap_to_click, Some(true));
        assert_eq!(touchpad.tap_and_drag, Some(true));
        assert_eq!(touchpad.tap_drag_lock, Some(false));
        assert_eq!(touchpad.natural_scroll, Some(true));
        assert_eq!(touchpad.left_handed, Some(false));
        assert_eq!(touchpad.disable_while_typing, Some(true));
        assert_eq!(
            touchpad.accel_profiles,
            vec!["flat".to_owned(), "adaptive".to_owned()]
        );
        assert_eq!(touchpad.default_accel_profile.as_deref(), Some("adaptive"));
        assert_eq!(
            touchpad.scroll_methods,
            vec![
                "two-finger".to_owned(),
                "edge".to_owned(),
                "button".to_owned()
            ]
        );
        assert_eq!(
            touchpad.default_scroll_method.as_deref(),
            Some("two-finger")
        );
        assert_eq!(
            touchpad.click_methods,
            vec!["button-areas".to_owned(), "clickfinger".to_owned()]
        );
        assert_eq!(
            touchpad.default_click_method.as_deref(),
            Some("button-areas")
        );
        assert_eq!(touchpad.rotation, None);
    }

    #[test]
    fn a_mouse_has_a_single_scroll_method_and_no_tapping() {
        let data = InputData::from_list_devices(FIXTURE);
        let mouse = data
            .devices
            .iter()
            .find(|device| device.name.contains("Optical Mouse"))
            .expect("mouse is in the fixture");
        assert_eq!(mouse.capabilities, vec!["pointer".to_owned()]);
        assert_eq!(mouse.tap_to_click, None);
        assert_eq!(mouse.scroll_methods, vec!["button".to_owned()]);
        assert_eq!(mouse.default_scroll_method.as_deref(), Some("button"));
        assert_eq!(
            mouse.accel_profiles,
            vec!["flat".to_owned(), "adaptive".to_owned()]
        );
    }

    #[test]
    fn unknown_keys_are_ignored_and_blank_records_are_skipped() {
        let data = InputData::from_list_devices(
            "Device:           Test Pad\nKernel:           /dev/input/event9\nSome-future-key:  whatever\n\n",
        );
        assert_eq!(data.devices.len(), 1);
        assert_eq!(data.devices[0].name, "Test Pad");
        assert_eq!(data.devices[0].kernel, "/dev/input/event9");
    }

    #[test]
    fn an_empty_run_is_a_present_stack_with_no_devices() {
        let data = InputData::from_list_devices("");
        assert!(data.devices.is_empty());
    }

    #[test]
    fn a_line_before_any_device_is_ignored() {
        let data =
            InputData::from_list_devices("Kernel: stray\nDevice: A\nKernel: /dev/input/event1\n");
        assert_eq!(data.devices.len(), 1);
        assert_eq!(data.devices[0].name, "A");
    }

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not run any process; only a call does.
        let _source = CommandLibinput::new();
    }
}
