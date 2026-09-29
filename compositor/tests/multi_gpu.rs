// SPDX-License-Identifier: MIT
//! T-03.4 acceptance: the multi-GPU import/fallback *decision* is explicit and
//! testable without hardware.
//!
//! Real multi-GPU hardware is unavailable on this host (the host Wayland
//! session owns DRM master — see `docs/SLICING-REVIEW.md`), so the unit is
//! marked **open, not skipped**. These tests pin the classification and the
//! dmabuf-format rule the backend shares with `scripts/multi-gpu-probe.py`.

use dragonfruit_compositor::multi_gpu::{format_allowed, GpuNode, MultiGpuOutcome};

fn gpu(name: &str, render: Option<&str>) -> GpuNode {
    GpuNode {
        name: name.into(),
        render_node: render.map(str::to_owned),
    }
}

#[test]
fn a_seat_with_no_card_is_none() {
    let outcome = MultiGpuOutcome::classify(&[]);
    assert_eq!(outcome, MultiGpuOutcome::NoGpu);
    assert!(!outcome.is_multi());
}

#[test]
fn one_card_is_single_and_multi_cards_are_import_or_fallback() {
    let single = MultiGpuOutcome::classify(&[gpu("card0", Some("renderD128"))]);
    assert_eq!(
        single,
        MultiGpuOutcome::Single {
            device: "card0".into()
        }
    );

    let imported = MultiGpuOutcome::classify(&[
        gpu("card0", Some("renderD128")),
        gpu("card1", Some("renderD129")),
    ]);
    assert!(imported.is_multi());
    assert_eq!(
        imported.marker(),
        "Multi-GPU: IMPORTED devices=2 secondaries=1 (per-device render nodes)"
    );

    let fallback =
        MultiGpuOutcome::classify(&[gpu("card0", Some("renderD128")), gpu("card1", None)]);
    assert!(fallback.is_multi());
    assert_eq!(
        fallback.marker(),
        "Multi-GPU: FALLBACK devices=2 secondaries=1 fallback=1 \
         (primary renderer, linear import)"
    );
}

#[test]
fn the_format_rule_is_linear_only_for_a_fallback_device() {
    assert!(format_allowed(true, true));
    assert!(format_allowed(true, false));
    assert!(format_allowed(false, true));
    assert!(!format_allowed(false, false));
}

#[test]
fn every_marker_is_one_line() {
    for outcome in [
        MultiGpuOutcome::classify(&[]),
        MultiGpuOutcome::classify(&[gpu("card0", None), gpu("card1", None)]),
        MultiGpuOutcome::classify(&[gpu("card0", None), gpu("card1", Some("renderD129"))]),
    ] {
        assert!(outcome.marker().starts_with("Multi-GPU: "));
        assert!(!outcome.marker().contains('\n'));
    }
}
