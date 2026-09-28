// SPDX-License-Identifier: MIT
//! Bridge tests for the Accessibility half of the host (T-15.14b): the view
//! the Settings pane and Control Center tile decode. The adapter is read-only,
//! so there are no writes to test.

use dragonfruit_accessibility_adapter::{AccessibilityData, MockAccessibility};
use dragonfruit_system_status::AccessibilityHost;

#[test]
fn the_view_exposes_the_flags_and_labels() {
    let mut host = AccessibilityHost::new(MockAccessibility::present(AccessibilityData {
        enabled: true,
        screen_reader: true,
    }));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "accessibility");
    assert_eq!(view["state"], "available");
    assert_eq!(view["glyph"], "accessibility");
    assert_eq!(view["label"], "Screen Reader On");
    assert_eq!(view["present"], true);
    assert_eq!(view["enabled"], true);
    assert_eq!(view["enabledLabel"], "On");
    assert_eq!(view["screenReader"], true);
    assert_eq!(view["screenReaderLabel"], "On");
}

#[test]
fn a_bridge_only_session_is_available_and_present() {
    let mut host = AccessibilityHost::new(MockAccessibility::present(AccessibilityData {
        enabled: true,
        screen_reader: false,
    }));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], true);
    assert_eq!(view["label"], "On");
    assert_eq!(view["screenReaderLabel"], "Off");
}

#[test]
fn an_all_off_bus_is_available_but_not_present() {
    let mut host = AccessibilityHost::new(MockAccessibility::present(AccessibilityData::default()));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], false);
    assert_eq!(view["label"], "Off");
    // Read-only: the view has no write action to expose.
    assert!(view.get("outcome").is_none());
}

#[test]
fn absence_hides_the_item() {
    let mut host = AccessibilityHost::new(MockAccessibility::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert!(host.state().contains("unavailable"));
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = AccessibilityHost::new(MockAccessibility::failing("at-spi: timeout"));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "error");
    assert_eq!(view["error"], "at-spi: timeout");
}

#[test]
fn a_refresh_reads_once_and_never_invents_a_snapshot() {
    let mut host = AccessibilityHost::new(MockAccessibility::present(AccessibilityData {
        enabled: true,
        screen_reader: false,
    }));
    host.refresh();
    assert_eq!(host.adapter().source().reads(), 1);
    // The AT-SPI bus answers with a screen reader now; a re-read converges.
    host.adapter_mut().source_mut().push(AccessibilityData {
        enabled: true,
        screen_reader: true,
    });
    host.refresh();
    assert_eq!(host.view()["label"], "Screen Reader On");
    assert_eq!(host.adapter().source().reads(), 2);
}
