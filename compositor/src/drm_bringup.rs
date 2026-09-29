// SPDX-License-Identifier: MIT
//! DRM first bring-up outcome (T-03.2).
//!
//! The DRM/KMS backend is the one path that needs real hardware: a logind
//! seat (or a spare GPU / clean VM). On a host that has no *free* seat the
//! unit is **marked open, not skipped** ([03-real-session-bringup-perf.md]).
//! This module owns the pure classification of a bring-up attempt so the
//! decision — "did we get a seat, a device, and at least one connected
//! output?" — is unit-testable without a display, and so the backend and the
//! `scripts/drm-bringup.sh` probe agree on one stable, greppable marker.
//!
//! The marker is deliberately a single line:
//!
//! ```text
//! DRM bring-up: READY device=<node> outputs=<n>
//! DRM bring-up: OPEN (no seat: <reason>)
//! DRM bring-up: OPEN (no usable DRM GPU on the seat)
//! DRM bring-up: OPEN (no connected output: master busy or all disconnected)
//! ```
//!
//! [03-real-session-bringup-perf.md]: ../../docs/design/tracks/03-real-session-bringup-perf.md

/// The result of one DRM bring-up attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrmBringup {
    /// A session, a DRM device, and at least one connected output.
    Ready { device: String, outputs: usize },
    /// The unit is open on this host, with the precise blocker.
    Open(DrmBringupBlocker),
}

/// Why a DRM bring-up could not produce a display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrmBringupBlocker {
    /// `logind`/`libseat` refused a session: no free seat (the common case on
    /// a workstation already running a desktop).
    NoSeat(String),
    /// A session was acquired but no usable DRM device is on the seat.
    NoGpu,
    /// A device opened but the initial scan produced no output: another
    /// process holds DRM master, or every connector is disconnected.
    NoOutput,
}

impl DrmBringup {
    /// Classify an attempt from its three observations: the session result,
    /// the primary device name (when one resolved), and the number of
    /// outputs the initial connector scan produced.
    pub fn classify(
        session: Result<(), String>,
        device: Option<String>,
        outputs: usize,
    ) -> DrmBringup {
        match session {
            Err(reason) => DrmBringup::Open(DrmBringupBlocker::NoSeat(reason)),
            Ok(()) => match device {
                None => DrmBringup::Open(DrmBringupBlocker::NoGpu),
                Some(_) if outputs == 0 => DrmBringup::Open(DrmBringupBlocker::NoOutput),
                Some(device) => DrmBringup::Ready { device, outputs },
            },
        }
    }

    /// True only when a display was actually brought up.
    pub fn is_ready(&self) -> bool {
        matches!(self, DrmBringup::Ready { .. })
    }

    /// The one-line marker the backend prints and the hardware-rail probe
    /// greps for.
    pub fn marker(&self) -> String {
        match self {
            DrmBringup::Ready { device, outputs } => {
                format!("DRM bring-up: READY device={device} outputs={outputs}")
            }
            DrmBringup::Open(blocker) => format!("DRM bring-up: OPEN ({blocker})"),
        }
    }
}

impl std::fmt::Display for DrmBringupBlocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrmBringupBlocker::NoSeat(reason) => write!(f, "no seat: {reason}"),
            DrmBringupBlocker::NoGpu => write!(f, "no usable DRM GPU on the seat"),
            DrmBringupBlocker::NoOutput => write!(
                f,
                "no connected output: master busy or all connectors disconnected"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_session_is_open_with_the_reason() {
        let outcome = DrmBringup::classify(Err("cannot open seat".into()), None, 0);
        assert_eq!(
            outcome,
            DrmBringup::Open(DrmBringupBlocker::NoSeat("cannot open seat".into()))
        );
        assert!(!outcome.is_ready());
        let marker = outcome.marker();
        assert!(marker.starts_with("DRM bring-up: OPEN ("), "{marker}");
        assert!(marker.contains("no seat: cannot open seat"), "{marker}");
    }

    #[test]
    fn a_session_without_a_device_is_open_no_gpu() {
        let outcome = DrmBringup::classify(Ok(()), None, 0);
        assert_eq!(outcome, DrmBringup::Open(DrmBringupBlocker::NoGpu));
        assert!(outcome.marker().contains("no usable DRM GPU"));
    }

    #[test]
    fn a_device_with_no_output_is_open_no_output() {
        let outcome = DrmBringup::classify(Ok(()), Some("card0".into()), 0);
        assert_eq!(outcome, DrmBringup::Open(DrmBringupBlocker::NoOutput));
        assert!(outcome.marker().contains("no connected output"));
    }

    #[test]
    fn a_session_device_and_output_is_ready() {
        let outcome = DrmBringup::classify(Ok(()), Some("card0".into()), 2);
        assert!(outcome.is_ready());
        assert_eq!(
            outcome,
            DrmBringup::Ready {
                device: "card0".into(),
                outputs: 2
            }
        );
        assert_eq!(
            outcome.marker(),
            "DRM bring-up: READY device=card0 outputs=2"
        );
    }

    #[test]
    fn the_marker_is_always_one_line() {
        for outcome in [
            DrmBringup::classify(Err("x".into()), None, 0),
            DrmBringup::classify(Ok(()), None, 0),
            DrmBringup::classify(Ok(()), Some("card0".into()), 0),
            DrmBringup::classify(Ok(()), Some("card0".into()), 1),
        ] {
            assert!(!outcome.marker().contains('\n'));
        }
    }
}
