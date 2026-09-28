// SPDX-License-Identifier: MIT
//! The Accessibility adapter (T-15.14a).
//!
//! Accessibility does not wrap one daemon per feature: it projects the **host
//! stack**, the AT-SPI accessibility bus every toolkit and assistive client
//! already speaks. Its launcher owns `org.a11y.Bus` on the session bus and
//! publishes the live bridge status at `org.a11y.Status` ([`HostAccessibility`]):
//! `IsEnabled` and `ScreenReaderEnabled`. The adapter reuses that status and
//! never reimplements a screen reader, a toolkit, or a magnifier; the bus is
//! the single source of truth.
//!
//! # The read path
//!
//! 1. An [`AccessibilitySource`] returns the raw [`AccessibilityData`] once per
//!    [`AccessibilityAdapter::refresh`] (or absence/error).
//! 2. [`AccessibilitySnapshot::from_data`] types the two flags and derives the
//!    labels and glyph the pane and the Control Center tile draw.
//! 3. The adapter drives the shared subscription lifecycle, so a bus that comes
//!    and goes re-subscribes and re-syncs with no user-visible error.
//!
//! # Absence is normal
//!
//! The adapter is `Unavailable` only when the session bus is unreachable or no
//! `org.a11y.Bus` owns its name — a normal hidden state, never an error. A bus
//! that answers with every feature off is `Available` with
//! [`AccessibilitySnapshot::present`] false; a consumer that shows only an
//! active status hides the item then. A bus that owns its name but cannot be
//! read is `Error`, visible and inert with the message. Nothing blocks session
//! startup.
//!
//! # Read-only by design
//!
//! The host stack publishes `org.a11y.Status`; it offers no setter. The user's
//! durable accessibility preferences are `settingsd`'s and magnification is
//! compositor-owned ([07-system-integration.md], principle 3), so this adapter
//! has no writes — exactly as the input adapter is the inventory half. The
//! Settings pane and Control Center tile land with T-15.14b.
//!
//! # Testing
//!
//! CI has no AT-SPI, so the adapter is driven by [`MockAccessibility`] over the
//! source seam. `kill`/`restart` exercise absence and re-subscribe; `push`
//! drives the status. [`HostAccessibility`] reads the live bus, and
//! [`status_from_properties`] decodes a `GetAll` map.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md
//! [08-settings.md]: ../../../docs/design/08-settings.md

mod accessibility;
mod adapter;
mod model;
mod source;

pub use accessibility::{
    status_from_properties, HostAccessibility, A11Y_BUS_PATH, A11Y_BUS_SERVICE,
    A11Y_STATUS_INTERFACE,
};
pub use adapter::AccessibilityAdapter;
pub use model::{AccessibilityChange, AccessibilitySnapshot};
pub use source::{AccessibilityData, AccessibilitySource, MockAccessibility};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::{Adapter, AdapterId};

    #[test]
    fn the_adapter_reports_the_accessibility_slot() {
        let adapter = AccessibilityAdapter::new(MockAccessibility::absent());
        assert_eq!(
            <AccessibilityAdapter<MockAccessibility> as Adapter>::id(&adapter),
            AdapterId::ACCESSIBILITY
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = AccessibilityAdapter::new(MockAccessibility::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_glyph_and_labels_are_stable() {
        let off = AccessibilitySnapshot::from_data(&AccessibilityData::default());
        assert_eq!(off.glyph(), "accessibility");
        assert_eq!(off.label(), "Off");
    }
}
