// SPDX-License-Identifier: MIT
//! The Keyboard/Mouse/Trackpad half of the bridge host (T-15.4b).
//!
//! libinput is reached by the `dragonfruit-input` adapter; the shell and the
//! Settings app never link it. This module owns the one projection from the
//! typed [`InputSnapshot`] to the flat JSON view the two consumers draw. It
//! mirrors the Bluetooth and Storage halves ([`crate::bluetooth`],
//! [`crate::storage`]) but is **read-only**: libinput persists nothing and has
//! no setter, so the adapter exposes no writes (ADR 0124). The pane's
//! preferences (`input.pointerSpeed`, `input.naturalScroll`, …) are `settingsd`
//! keys the compositor applies, never writes through here.
//!
//! A refresh never invents a snapshot: the host calls
//! [`InputAdapter::refresh`](dragonfruit_input::InputAdapter::refresh) when
//! libinput answers (or when the session asks), and the read state stays the
//! single source of truth.

use dragonfruit_input::{DeviceKind, InputAdapter, InputDevice, InputSnapshot, InputSource};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_INPUT: &str = "input";

/// The bridge host for the input adapter: one read path and no writes.
#[derive(Debug, Clone, PartialEq)]
pub struct InputHost<S> {
    adapter: InputAdapter<S>,
}

impl<S: InputSource> InputHost<S> {
    /// A host over an input source.
    pub fn new(source: S) -> Self {
        InputHost {
            adapter: InputAdapter::new(source),
        }
    }

    /// Re-read libinput once. Called on startup and when the Settings pane or
    /// the shell opens; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The input view the pane and tile render.
    pub fn view(&self) -> Value {
        input_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &InputAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut InputAdapter<S> {
        &mut self.adapter
    }
}

/// Build the input view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the live
/// inventory. A session that runs libinput with **no recognized device** is
/// still `available` with `present: false`, and the consumer hides the item
/// then too — the battery's and Bluetooth's second hide rule.
pub fn input_view<S: InputSource>(adapter: &InputAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_INPUT, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_INPUT,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_INPUT, "state": "unavailable" });
    };
    input_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn input_snapshot_view(snapshot: &InputSnapshot) -> Value {
    let devices: Vec<Value> = snapshot.devices().iter().map(device_view).collect();
    json!({
        "kind": KIND_INPUT,
        "state": "available",
        "present": snapshot.present(),
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "deviceCount": snapshot.count(),
        "keyboardCount": snapshot.keyboards().len(),
        "pointerCount": snapshot.pointers().len(),
        "mouseCount": snapshot.mice().len(),
        "touchpadCount": snapshot.touchpads().len(),
        "devices": devices,
    })
}

fn device_view(device: &InputDevice) -> Value {
    json!({
        "name": device.name,
        "kernel": device.kernel,
        "identity": device.identity,
        "kind": kind_token(device.kind),
        "kindLabel": device.kind.label(),
        "glyph": device.kind.glyph(),
        "seat": device.seat,
        "capabilities": device.capabilities,
        "sizeMm": device.size_mm.map(|(w, h)| json!([w, h])),
        "tapToClick": device.tap_to_click,
        "naturalScroll": device.natural_scroll,
        "leftHanded": device.left_handed,
        "disableWhileTyping": device.disable_while_typing,
        "accelProfiles": device.accel_profiles,
        "defaultAccelProfile": device.default_accel_profile,
        "scrollMethods": device.scroll_methods,
        "defaultScrollMethod": device.default_scroll_method,
    })
}

/// The stable lower-case token for a device kind (the UI groups by it and the
/// tests assert it).
fn kind_token(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Keyboard => "keyboard",
        DeviceKind::Mouse => "mouse",
        DeviceKind::Touchpad => "touchpad",
        DeviceKind::Touchscreen => "touchscreen",
        DeviceKind::Tablet => "tablet",
        DeviceKind::Switch => "switch",
        DeviceKind::Other => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_input::{InputData, InputDeviceData, MockInput};

    fn data() -> InputData {
        InputData {
            devices: vec![
                InputDeviceData {
                    name: "AT Translated Set 2 keyboard".to_owned(),
                    kernel: "/dev/input/event3".to_owned(),
                    identity: "isa:0060".to_owned(),
                    capabilities: vec!["keyboard".to_owned()],
                    seat: "seat0, default".to_owned(),
                    ..InputDeviceData::default()
                },
                InputDeviceData {
                    name: "Synaptics TouchPad".to_owned(),
                    kernel: "/dev/input/event7".to_owned(),
                    identity: "i2c:06cb:7e7e".to_owned(),
                    capabilities: vec!["pointer".to_owned(), "gesture".to_owned()],
                    seat: "seat0, default".to_owned(),
                    size_mm: Some((97.0, 66.0)),
                    tap_to_click: Some(true),
                    natural_scroll: Some(true),
                    accel_profiles: vec!["adaptive".to_owned(), "flat".to_owned()],
                    default_accel_profile: Some("adaptive".to_owned()),
                    ..InputDeviceData::default()
                },
                InputDeviceData {
                    name: "Logitech USB Mouse".to_owned(),
                    kernel: "/dev/input/event9".to_owned(),
                    capabilities: vec!["pointer".to_owned()],
                    seat: "seat0, default".to_owned(),
                    ..InputDeviceData::default()
                },
            ],
        }
    }

    #[test]
    fn an_absent_stack_projects_a_hidden_slot() {
        let mut host = InputHost::new(MockInput::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "input");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_stack_projects_the_inventory() {
        let mut host = InputHost::new(MockInput::present(data()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], true);
        assert_eq!(view["glyph"], "keyboard");
        assert_eq!(view["label"], "1 keyboards, 2 pointing devices");
        assert_eq!(view["deviceCount"], 3);
        assert_eq!(view["keyboardCount"], 1);
        assert_eq!(view["pointerCount"], 2);
        assert_eq!(view["mouseCount"], 1);
        assert_eq!(view["touchpadCount"], 1);
        // Keyboards sort first, then the pointing devices by name.
        assert_eq!(view["devices"][0]["kind"], "keyboard");
        assert_eq!(view["devices"][1]["kind"], "mouse");
        assert_eq!(view["devices"][2]["kind"], "touchpad");
        assert_eq!(view["devices"][2]["kindLabel"], "Trackpad");
        assert_eq!(view["devices"][2]["tapToClick"], true);
        assert_eq!(view["devices"][2]["sizeMm"][0], 97.0);
    }

    #[test]
    fn a_running_stack_with_no_devices_is_available_but_not_present() {
        let mut host = InputHost::new(MockInput::present(InputData::default()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], false);
        assert_eq!(view["label"], "No input devices");
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = InputHost::new(MockInput::failing("libinput: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "libinput: timeout");
    }

    #[test]
    fn the_input_host_is_read_only() {
        // There is no write method: the pane's preferences are settingsd keys,
        // and libinput is an inventory (ADR 0124).
        let host = InputHost::new(MockInput::present(data()));
        assert_eq!(host.adapter().source().reads(), 0);
    }
}
