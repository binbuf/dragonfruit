// SPDX-License-Identifier: MIT
//! The transport seam: the raw AT-SPI status read and its mock.
//!
//! An [`AccessibilitySource`] is the only thing that talks to the host
//! accessibility stack, and the host stack here is the **AT-SPI accessibility
//! bus** ([`crate::HostAccessibility`]): the toolkit bridge and assistive
//! technology registry that every desktop already speaks. The bus publishes
//! its own status at `org.a11y.Status` —
//! `IsEnabled` (the accessibility bridge is on) and `ScreenReaderEnabled` (a
//! screen reader is asking for events). This adapter projects that status and
//! never reimplements a screen reader, a toolkit, or a magnifier.
//!
//! The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — the accessibility bus answered; `data` is the live
//!   status.
//! * `Ok(None)` — the bus is **absent** (no session bus, or no `org.a11y.Bus`
//!   owner). A normal state; the item hides.
//! * `Err(error)` — the bus is present but could not be read; the item shows
//!   visible and inert with the message.
//!
//! The adapter is **read-only**: `org.a11y.Status` is published by the host
//! stack, and the desktop's own accessibility switches are durable preferences
//! (`settingsd`) plus compositor-owned magnification. This adapter is the
//! projection of the live bridge state, nothing more.

use dragonfruit_system_adapters::AdapterError;

/// The raw result of one AT-SPI status read.
///
/// Both fields are booleans the host stack publishes; there is no third state
/// to invent. `enabled` is the accessibility bridge (`org.a11y.Status
/// .IsEnabled`) and `screen_reader` is the assistive client's request flag
/// (`ScreenReaderEnabled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccessibilityData {
    /// The toolkit accessibility bridge is on.
    pub enabled: bool,
    /// A screen reader is enabled.
    pub screen_reader: bool,
}

/// Reads the host accessibility stack.
///
/// The read result is a three-way answer, exactly as the adapter contract needs
/// it:
///
/// * `Ok(Some(data))` — the accessibility bus answered.
/// * `Ok(None)` — the bus is absent. A normal state; the item hides.
/// * `Err(error)` — the bus is present but the read failed; the item shows
///   visible and inert with the message.
pub trait AccessibilitySource {
    /// One read of the host accessibility stack.
    fn read(&mut self) -> Result<Option<AccessibilityData>, AdapterError>;
}

/// A fixture-backed source with a simulated accessibility-bus lifecycle.
///
/// The mock is the CI path: it serves [`AccessibilityData`] with no AT-SPI on
/// the machine, and `kill`/`restart` exercise absence and re-subscribe. `push`
/// drives the status so a test observes the change stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockAccessibility {
    present: bool,
    data: AccessibilityData,
    failure: Option<AdapterError>,
    reads: u32,
}

impl MockAccessibility {
    /// No accessibility bus is reachable.
    pub fn absent() -> Self {
        MockAccessibility {
            present: false,
            data: AccessibilityData::default(),
            failure: None,
            reads: 0,
        }
    }

    /// A bus that answers with `data`.
    pub fn present(data: AccessibilityData) -> Self {
        MockAccessibility {
            present: true,
            data,
            failure: None,
            reads: 0,
        }
    }

    /// A bus that fails every read (e.g. the daemon went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockAccessibility {
            present: true,
            data: AccessibilityData::default(),
            failure: Some(AdapterError::new(message)),
            reads: 0,
        }
    }

    /// The bus publishes fresh status.
    pub fn push(&mut self, data: AccessibilityData) {
        self.present = true;
        self.failure = None;
        self.data = data;
    }

    /// The bus goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The bus comes back (serving the last status, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated bus is reachable.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }
}

impl AccessibilitySource for MockAccessibility {
    fn read(&mut self) -> Result<Option<AccessibilityData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> AccessibilityData {
        AccessibilityData {
            enabled: true,
            screen_reader: true,
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockAccessibility::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.is_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockAccessibility::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockAccessibility::failing("at-spi: timeout");
        assert_eq!(mock.read().unwrap_err().message(), "at-spi: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn the_bus_can_be_killed_and_restarted() {
        let mut mock = MockAccessibility::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn pushes_replace_the_served_status_and_presence() {
        let mut mock = MockAccessibility::present(data());
        mock.push(AccessibilityData::default());
        assert_eq!(mock.read().unwrap(), Some(AccessibilityData::default()));
        assert!(mock.is_present());
    }
}
