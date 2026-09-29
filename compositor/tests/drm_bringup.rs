// SPDX-License-Identifier: MIT
//! T-03.2 acceptance: the DRM first bring-up *decision* is explicit and
//! testable without hardware.
//!
//! The DRM/KMS backend has never run in this environment (the host has no
//! free logind seat — see `docs/SLICING-REVIEW.md`), so the unit is marked
//! **open, not skipped**. This test pins the classification the backend and
//! `scripts/drm-bringup.sh` share:
//!
//! 1. a refused session is OPEN with the libseat reason;
//! 2. a session without a device is OPEN (no GPU);
//! 3. a device that produced no output is OPEN (master busy / disconnected);
//! 4. a session + device + output is READY and reports the output count;
//! 5. the marker is a single greppable line in every case.

use dragonfruit_compositor::drm_bringup::{DrmBringup, DrmBringupBlocker};

#[test]
fn a_refused_session_marks_the_unit_open_with_the_reason() {
    let outcome = DrmBringup::classify(Err("cannot open seat".into()), None, 0);
    assert_eq!(
        outcome,
        DrmBringup::Open(DrmBringupBlocker::NoSeat("cannot open seat".into()))
    );
    assert!(!outcome.is_ready());
    assert_eq!(
        outcome.marker(),
        "DRM bring-up: OPEN (no seat: cannot open seat)"
    );
}

#[test]
fn a_session_without_a_device_marks_the_unit_open() {
    let outcome = DrmBringup::classify(Ok(()), None, 0);
    assert_eq!(outcome, DrmBringup::Open(DrmBringupBlocker::NoGpu));
    assert_eq!(
        outcome.marker(),
        "DRM bring-up: OPEN (no usable DRM GPU on the seat)"
    );
}

#[test]
fn a_device_with_no_output_marks_the_unit_open() {
    let outcome = DrmBringup::classify(Ok(()), Some("card0".into()), 0);
    assert_eq!(outcome, DrmBringup::Open(DrmBringupBlocker::NoOutput));
    assert!(outcome.marker().contains("no connected output"));
}

#[test]
fn a_session_device_and_output_is_ready() {
    let outcome = DrmBringup::classify(Ok(()), Some("card0".into()), 2);
    assert!(outcome.is_ready());
    assert_eq!(
        outcome.marker(),
        "DRM bring-up: READY device=card0 outputs=2"
    );
}

#[test]
fn the_marker_is_always_one_greppable_line() {
    for outcome in [
        DrmBringup::classify(Err("x".into()), None, 0),
        DrmBringup::classify(Ok(()), None, 0),
        DrmBringup::classify(Ok(()), Some("card0".into()), 0),
        DrmBringup::classify(Ok(()), Some("card0".into()), 1),
    ] {
        let marker = outcome.marker();
        assert!(marker.starts_with("DRM bring-up: "), "{marker}");
        assert!(!marker.contains('\n'), "{marker}");
    }
}
