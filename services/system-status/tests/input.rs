// SPDX-License-Identifier: MIT
//! Bridge tests for the input half of the host (T-15.4b): the inventory view
//! the Settings pane and Control Center tile decode. The adapter is
//! read-only, so there is no write to exercise.

use dragonfruit_input::{InputData, InputDeviceData, MockInput};
use dragonfruit_system_status::InputHost;

fn data() -> InputData {
    InputData {
        devices: vec![
            InputDeviceData {
                name: "AT Translated Set 2 keyboard".to_owned(),
                kernel: "/dev/input/event3".to_owned(),
                capabilities: vec!["keyboard".to_owned()],
                seat: "seat0, default".to_owned(),
                ..InputDeviceData::default()
            },
            InputDeviceData {
                name: "Synaptics TouchPad".to_owned(),
                kernel: "/dev/input/event7".to_owned(),
                capabilities: vec!["pointer".to_owned(), "gesture".to_owned()],
                seat: "seat0, default".to_owned(),
                tap_to_click: Some(true),
                ..InputDeviceData::default()
            },
        ],
    }
}

#[test]
fn the_input_view_exposes_the_device_inventory() {
    let mut host = InputHost::new(MockInput::present(data()));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "input");
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], true);
    assert_eq!(view["deviceCount"], 2);
    assert_eq!(view["keyboardCount"], 1);
    assert_eq!(view["touchpadCount"], 1);
    assert_eq!(view["devices"][0]["kind"], "keyboard");
    assert_eq!(view["devices"][1]["kind"], "touchpad");
    assert_eq!(view["devices"][1]["kindLabel"], "Trackpad");
    assert_eq!(view["devices"][1]["tapToClick"], true);
}

#[test]
fn a_running_stack_with_no_devices_hides_the_item() {
    let mut host = InputHost::new(MockInput::present(InputData::default()));
    host.refresh();
    assert_eq!(host.view()["state"], "available");
    assert_eq!(host.view()["present"], false);
}

#[test]
fn absence_hides_the_item() {
    let mut host = InputHost::new(MockInput::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = InputHost::new(MockInput::failing("libinput: timeout"));
    host.refresh();
    assert_eq!(host.view()["state"], "error");
    assert_eq!(host.view()["error"], "libinput: timeout");
}
