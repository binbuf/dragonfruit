// SPDX-License-Identifier: MIT
//! A configurable adapter for tests.

use crate::state::{AdapterId, AdapterState};
use crate::Adapter;

/// A test adapter whose state a test sets directly.
///
/// The mock is the mock half of FR-1: a consumer can be driven through all
/// three [`AdapterState`]s with no daemon on the bus. A new mock starts
/// [`AdapterState::Unavailable`] — the safe "daemon absent" default, so a
/// test that forgets to set a state exercises the degradation path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockAdapter<T> {
    id: AdapterId,
    state: AdapterState<T>,
}

impl<T> MockAdapter<T> {
    /// An unavailable mock for `id`.
    pub const fn new(id: AdapterId) -> Self {
        MockAdapter {
            id,
            state: AdapterState::Unavailable,
        }
    }

    /// A mock that starts in the available state.
    pub const fn with_available(id: AdapterId, snapshot: T) -> Self {
        MockAdapter {
            id,
            state: AdapterState::Available(snapshot),
        }
    }

    /// Move to the available state with a new snapshot.
    pub fn set_available(&mut self, snapshot: T) {
        self.state = AdapterState::available(snapshot);
    }

    /// Move to the unavailable state (daemon absent).
    pub fn set_unavailable(&mut self) {
        self.state = AdapterState::unavailable();
    }

    /// Move to the error state.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.state = AdapterState::with_error(message);
    }
}

impl<T> Adapter for MockAdapter<T> {
    type Snapshot = T;

    fn id(&self) -> AdapterId {
        self.id
    }

    fn state(&self) -> &AdapterState<T> {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatusSource;

    #[test]
    fn a_new_mock_is_unavailable() {
        let mock = MockAdapter::<u8>::new(AdapterId::WIFI);
        assert!(mock.state().is_unavailable());
        assert!(!mock.slot().visible);
    }

    #[test]
    fn a_mock_can_be_driven_through_every_state() {
        let mut mock = MockAdapter::new(AdapterId::AUDIO);

        mock.set_available(0.5_f32);
        assert!(mock.state().is_available());
        assert_eq!(mock.state().snapshot(), Some(&0.5));
        assert!(mock.slot().visible && mock.slot().enabled);

        mock.set_error("PipeWire: connection refused");
        assert!(mock.state().is_error());
        assert!(mock.slot().visible && !mock.slot().enabled);
        assert_eq!(
            mock.slot().error.as_deref(),
            Some("PipeWire: connection refused")
        );

        mock.set_unavailable();
        assert!(mock.state().is_unavailable());
        assert!(!mock.slot().visible);
    }

    #[test]
    fn an_initially_available_mock_carries_its_snapshot() {
        let mock = MockAdapter::with_available(AdapterId::POWER, 0.42_f64);
        assert_eq!(mock.state().snapshot(), Some(&0.42));
        assert_eq!(Adapter::id(&mock), AdapterId::POWER);
    }
}
