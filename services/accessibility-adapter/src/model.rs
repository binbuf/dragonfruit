// SPDX-License-Identifier: MIT
//! The Accessibility snapshot the pane renders, decoded from one raw AT-SPI
//! read.
//!
//! The model owns only the projection a consumer should not repeat: it types
//! the two `org.a11y.Status` booleans, derives the labels and glyph the pane
//! header and the Control Center tile draw, and computes the change stream
//! between two reads. It never enables a screen reader or turns on the
//! accessibility bridge — the host stack owns that state; this is what it
//! publishes.
//!
//! Both booleans are ordinary values, not presence: a bus that answers with
//! everything off is `Available` and the pane renders "Off". The second hide
//! rule is [`AccessibilitySnapshot::present`] — true once any accessibility
//! feature is on — for consumers that only show an active status.

use crate::source::AccessibilityData;

/// The Accessibility snapshot a pane or status slot renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccessibilitySnapshot {
    enabled: bool,
    screen_reader: bool,
}

impl AccessibilitySnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &AccessibilityData) -> Self {
        AccessibilitySnapshot {
            enabled: data.enabled,
            screen_reader: data.screen_reader,
        }
    }

    /// Whether the toolkit accessibility bridge is on.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Whether a screen reader is enabled.
    pub fn is_screen_reader_enabled(&self) -> bool {
        self.screen_reader
    }

    /// Whether any accessibility feature is on. This is the status slot's
    /// second hide rule: a bus that answers with everything off is
    /// `Available` but not present, exactly as an empty device inventory is.
    pub fn present(&self) -> bool {
        self.enabled || self.screen_reader
    }

    /// The bridge's on/off label.
    pub fn enabled_label(&self) -> &'static str {
        if self.enabled {
            "On"
        } else {
            "Off"
        }
    }

    /// The screen reader's on/off label.
    pub fn screen_reader_label(&self) -> &'static str {
        if self.screen_reader {
            "On"
        } else {
            "Off"
        }
    }

    /// The Control Center label: `Screen Reader On`, `On`, or `Off`.
    pub fn label(&self) -> &'static str {
        if self.screen_reader {
            "Screen Reader On"
        } else {
            self.enabled_label()
        }
    }

    /// The design-system glyph the pane header draws.
    pub fn glyph(&self) -> &'static str {
        "accessibility"
    }

    /// The status changes between `previous` and this snapshot, in order.
    pub fn changes(&self, previous: &AccessibilitySnapshot) -> Vec<AccessibilityChange> {
        let mut changes = Vec::new();
        if self.enabled != previous.enabled {
            changes.push(AccessibilityChange::Enabled {
                enabled: self.enabled,
            });
        }
        if self.screen_reader != previous.screen_reader {
            changes.push(AccessibilityChange::ScreenReader {
                enabled: self.screen_reader,
            });
        }
        changes
    }
}

/// One status change between two accessibility reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityChange {
    /// The accessibility bridge was turned on or off.
    Enabled {
        /// The new value.
        enabled: bool,
    },
    /// The screen reader was turned on or off.
    ScreenReader {
        /// The new value.
        enabled: bool,
    },
}

impl AccessibilityChange {
    /// Whether this change is about the accessibility bridge.
    pub fn is_enabled(&self) -> bool {
        matches!(self, AccessibilityChange::Enabled { .. })
    }

    /// Whether this change is about the screen reader.
    pub fn is_screen_reader(&self) -> bool {
        matches!(self, AccessibilityChange::ScreenReader { .. })
    }

    /// The new value carried by this change.
    pub fn enabled(&self) -> bool {
        match self {
            AccessibilityChange::Enabled { enabled }
            | AccessibilityChange::ScreenReader { enabled } => *enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_snapshot_types_the_read() {
        let snapshot = AccessibilitySnapshot::from_data(&AccessibilityData {
            enabled: true,
            screen_reader: true,
        });
        assert!(snapshot.is_enabled());
        assert!(snapshot.is_screen_reader_enabled());
        assert!(snapshot.present());
        assert_eq!(snapshot.glyph(), "accessibility");
        assert_eq!(snapshot.label(), "Screen Reader On");
        assert_eq!(snapshot.enabled_label(), "On");
        assert_eq!(snapshot.screen_reader_label(), "On");
    }

    #[test]
    fn a_bridge_only_session_is_on_but_not_a_screen_reader() {
        let snapshot = AccessibilitySnapshot::from_data(&AccessibilityData {
            enabled: true,
            screen_reader: false,
        });
        assert!(snapshot.present());
        assert_eq!(snapshot.label(), "On");
        assert_eq!(snapshot.screen_reader_label(), "Off");
    }

    #[test]
    fn an_all_off_bus_is_available_but_not_present() {
        let snapshot = AccessibilitySnapshot::from_data(&AccessibilityData::default());
        assert!(!snapshot.present());
        assert_eq!(snapshot.label(), "Off");
        assert_eq!(snapshot.glyph(), "accessibility");
    }

    #[test]
    fn each_flag_move_is_a_change() {
        let off = AccessibilitySnapshot::from_data(&AccessibilityData::default());
        let enabled = AccessibilitySnapshot::from_data(&AccessibilityData {
            enabled: true,
            screen_reader: false,
        });
        assert_eq!(
            enabled.changes(&off),
            vec![AccessibilityChange::Enabled { enabled: true }]
        );

        let both = AccessibilitySnapshot::from_data(&AccessibilityData {
            enabled: true,
            screen_reader: true,
        });
        assert_eq!(
            both.changes(&enabled),
            vec![AccessibilityChange::ScreenReader { enabled: true }]
        );

        assert_eq!(
            off.changes(&both),
            vec![
                AccessibilityChange::Enabled { enabled: false },
                AccessibilityChange::ScreenReader { enabled: false },
            ]
        );
        assert!(both.changes(&both).is_empty());
    }

    #[test]
    fn a_change_carries_its_new_value_and_kind() {
        let change = AccessibilityChange::ScreenReader { enabled: true };
        assert!(change.is_screen_reader());
        assert!(!change.is_enabled());
        assert!(change.enabled());
    }
}
