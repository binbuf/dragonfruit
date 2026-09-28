// SPDX-License-Identifier: MIT
//! The Accessibility half of the bridge host (T-15.14b).
//!
//! The host stack is reached by the `dragonfruit-accessibility-adapter` over
//! the AT-SPI accessibility bus; the shell and the Settings app never link it.
//! This module owns the one projection from the typed [`AccessibilitySnapshot`]
//! to the flat JSON view the two consumers draw. It mirrors the input half
//! ([`crate::input`]) and is **read-only**: `org.a11y.Status` has no setter
//! (ADR 0144). The durable accessibility preferences (currently
//! `accessibility.reduceMotion`) are `settingsd` keys the pane writes through
//! its own client; the compositor owns magnification. This host only projects
//! the live bridge state.
//!
//! Absence is single-layered, exactly as the adapter's is (ADR 0144). The view
//! is `unavailable` only when there is no session bus or no `org.a11y.Bus`
//! owner; the item hides then. A bus that answers with every feature off is
//! `available` with `present: false` (the tile's second hide rule). A bus that
//! owns its name but cannot be read is `error`, visible and inert with the
//! message.
//!
//! A refresh never invents a snapshot: the host calls
//! [`AccessibilityAdapter::refresh`](dragonfruit_accessibility_adapter::AccessibilityAdapter::refresh)
//! when the session asks, and the read state stays the single source of truth.

use dragonfruit_accessibility_adapter::{
    AccessibilityAdapter, AccessibilitySnapshot, AccessibilitySource,
};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_ACCESSIBILITY: &str = "accessibility";

/// The bridge host for the Accessibility adapter: one read path and no writes.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityHost<S> {
    adapter: AccessibilityAdapter<S>,
}

impl<S: AccessibilitySource> AccessibilityHost<S> {
    /// A host over an accessibility source.
    pub fn new(source: S) -> Self {
        AccessibilityHost {
            adapter: AccessibilityAdapter::new(source),
        }
    }

    /// Re-read the AT-SPI accessibility bus once. Called on startup and when
    /// the Settings pane or the shell opens; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The accessibility view the pane and tile render.
    pub fn view(&self) -> Value {
        accessibility_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &AccessibilityAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut AccessibilityAdapter<S> {
        &mut self.adapter
    }
}

/// Build the accessibility view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the two live
/// flags and their derived labels. A bus that answers with every feature off is
/// still `available` with `present: false`, the tile's second hide rule.
pub fn accessibility_view<S: AccessibilitySource>(adapter: &AccessibilityAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_ACCESSIBILITY, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_ACCESSIBILITY,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_ACCESSIBILITY, "state": "unavailable" });
    };
    accessibility_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn accessibility_snapshot_view(snapshot: &AccessibilitySnapshot) -> Value {
    json!({
        "kind": KIND_ACCESSIBILITY,
        "state": "available",
        "glyph": snapshot.glyph(),
        // `Screen Reader On` / `On` / `Off`; the tile's subtitle.
        "label": snapshot.label(),
        // The tile hides when the bus answers with every feature off.
        "present": snapshot.present(),
        "enabled": snapshot.is_enabled(),
        "enabledLabel": snapshot.enabled_label(),
        "screenReader": snapshot.is_screen_reader_enabled(),
        "screenReaderLabel": snapshot.screen_reader_label(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_accessibility_adapter::{AccessibilityData, MockAccessibility};

    #[test]
    fn an_absent_bus_projects_a_hidden_slot() {
        let mut host = AccessibilityHost::new(MockAccessibility::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "accessibility");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_bus_projects_the_flags_and_labels() {
        let mut host = AccessibilityHost::new(MockAccessibility::present(AccessibilityData {
            enabled: true,
            screen_reader: true,
        }));
        host.refresh();
        let view = host.view();
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
    fn an_all_off_bus_is_available_but_not_present() {
        let mut host =
            AccessibilityHost::new(MockAccessibility::present(AccessibilityData::default()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], false);
        assert_eq!(view["label"], "Off");
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = AccessibilityHost::new(MockAccessibility::failing("at-spi: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "at-spi: timeout");
    }

    #[test]
    fn a_refresh_never_invents_a_snapshot() {
        let mut host = AccessibilityHost::new(MockAccessibility::present(AccessibilityData {
            enabled: true,
            screen_reader: false,
        }));
        host.refresh();
        assert_eq!(host.view()["label"], "On");
        // A kill empties the view; there is no stale flag left behind.
        host.adapter_mut().source_mut().kill();
        host.refresh();
        assert_eq!(host.view()["state"], "unavailable");
        assert!(host.view().get("screenReader").is_none());
    }
}
