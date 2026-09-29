// SPDX-License-Identifier: MIT
//! Multi-GPU import/fallback classification (T-03.4).
//!
//! The DRM backend renders on the primary GPU and can drive several cards at
//! once (`GpuManager`). A secondary card that exposes its own render node
//! renders locally and its buffers are **imported** across devices; a card
//! with no usable render node (or a software EGL device) falls **back** to the
//! primary renderer and may only import linear buffers. Real multi-GPU
//! hardware is unavailable on this host, so this module owns the pure
//! decision — "which shape is this seat?" — that the backend logs, the
//! `scripts/multi-gpu-probe.py` probe mirrors, and the tests pin without a
//! device. The single-line marker is:
//!
//! ```text
//! Multi-GPU: NONE (no DRM device on the seat)
//! Multi-GPU: SINGLE device=<node>
//! Multi-GPU: IMPORTED devices=<n> secondaries=<m> (per-device render nodes)
//! Multi-GPU: FALLBACK devices=<n> secondaries=<m> fallback=<k> (primary renderer, linear import)
//! ```
//!
//! [03-real-session-bringup-perf.md]: ../../docs/design/tracks/03-real-session-bringup-perf.md

/// One DRM device as the import/fallback decision sees it: its card node and
/// the render node that will do its rendering, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuNode {
    pub name: String,
    pub render_node: Option<String>,
}

impl GpuNode {
    /// True when this device renders locally. A device without a render node
    /// falls back to the primary GPU's renderer and can only import linear
    /// buffers.
    pub fn renders_locally(&self) -> bool {
        self.render_node.is_some()
    }
}

/// The shape of the seat's GPU set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiGpuOutcome {
    /// No DRM device at all (no seat, or a seat with no card).
    NoGpu,
    /// One device: no cross-device import is required.
    Single { device: String },
    /// Every non-primary device has its own render node: buffers are imported
    /// across devices.
    Imported { devices: usize, secondaries: usize },
    /// At least one non-primary device has no render node and falls back to
    /// the primary renderer, importing only linear buffers.
    Fallback {
        devices: usize,
        secondaries: usize,
        fallback: usize,
    },
}

impl MultiGpuOutcome {
    /// Classify the seat's devices, primary first. `gpus` is in scan order:
    /// the backend puts the primary device at the head.
    pub fn classify(gpus: &[GpuNode]) -> MultiGpuOutcome {
        match gpus.split_first() {
            None => MultiGpuOutcome::NoGpu,
            Some((primary, [])) => MultiGpuOutcome::Single {
                device: primary.name.clone(),
            },
            Some((_primary, secondaries)) => {
                let fallback = secondaries
                    .iter()
                    .filter(|gpu| !gpu.renders_locally())
                    .count();
                if fallback == 0 {
                    MultiGpuOutcome::Imported {
                        devices: gpus.len(),
                        secondaries: secondaries.len(),
                    }
                } else {
                    MultiGpuOutcome::Fallback {
                        devices: gpus.len(),
                        secondaries: secondaries.len(),
                        fallback,
                    }
                }
            }
        }
    }

    /// True when the seat has more than one GPU (import or fallback shape).
    pub fn is_multi(&self) -> bool {
        matches!(
            self,
            MultiGpuOutcome::Imported { .. } | MultiGpuOutcome::Fallback { .. }
        )
    }

    /// The one-line marker the backend prints and the probe mirrors.
    pub fn marker(&self) -> String {
        match self {
            MultiGpuOutcome::NoGpu => "Multi-GPU: NONE (no DRM device on the seat)".into(),
            MultiGpuOutcome::Single { device } => format!("Multi-GPU: SINGLE device={device}"),
            MultiGpuOutcome::Imported {
                devices,
                secondaries,
            } => format!(
                "Multi-GPU: IMPORTED devices={devices} secondaries={secondaries} \
                 (per-device render nodes)"
            ),
            MultiGpuOutcome::Fallback {
                devices,
                secondaries,
                fallback,
            } => format!(
                "Multi-GPU: FALLBACK devices={devices} secondaries={secondaries} \
                 fallback={fallback} (primary renderer, linear import)"
            ),
        }
    }
}

/// Whether a dmabuf render format may be advertised for a device: a device
/// that renders locally offers all of them; a fallback device offers only
/// linear (the one modifier every importer can map).
pub fn format_allowed(renders_locally: bool, linear: bool) -> bool {
    renders_locally || linear
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu(name: &str, render: Option<&str>) -> GpuNode {
        GpuNode {
            name: name.into(),
            render_node: render.map(str::to_owned),
        }
    }

    #[test]
    fn no_device_is_none() {
        let outcome = MultiGpuOutcome::classify(&[]);
        assert_eq!(outcome, MultiGpuOutcome::NoGpu);
        assert!(!outcome.is_multi());
        assert_eq!(
            outcome.marker(),
            "Multi-GPU: NONE (no DRM device on the seat)"
        );
    }

    #[test]
    fn one_device_is_single_and_needs_no_import() {
        let outcome = MultiGpuOutcome::classify(&[gpu("card0", Some("renderD128"))]);
        assert_eq!(
            outcome,
            MultiGpuOutcome::Single {
                device: "card0".into()
            }
        );
        assert!(!outcome.is_multi());
        assert_eq!(outcome.marker(), "Multi-GPU: SINGLE device=card0");
    }

    #[test]
    fn every_secondary_with_a_render_node_is_imported() {
        let outcome = MultiGpuOutcome::classify(&[
            gpu("card0", Some("renderD128")),
            gpu("card1", Some("renderD129")),
        ]);
        assert_eq!(
            outcome,
            MultiGpuOutcome::Imported {
                devices: 2,
                secondaries: 1
            }
        );
        assert!(outcome.is_multi());
        assert_eq!(
            outcome.marker(),
            "Multi-GPU: IMPORTED devices=2 secondaries=1 (per-device render nodes)"
        );
    }

    #[test]
    fn a_secondary_without_a_render_node_falls_back() {
        let outcome = MultiGpuOutcome::classify(&[
            gpu("card0", Some("renderD128")),
            gpu("card1", None),
            gpu("card2", Some("renderD130")),
        ]);
        assert_eq!(
            outcome,
            MultiGpuOutcome::Fallback {
                devices: 3,
                secondaries: 2,
                fallback: 1
            }
        );
        assert_eq!(
            outcome.marker(),
            "Multi-GPU: FALLBACK devices=3 secondaries=2 fallback=1 \
             (primary renderer, linear import)"
        );
    }

    #[test]
    fn fallback_advertises_only_linear_formats() {
        assert!(format_allowed(true, true));
        assert!(format_allowed(true, false));
        assert!(format_allowed(false, true));
        assert!(!format_allowed(false, false));
    }

    #[test]
    fn markers_are_always_one_line() {
        for outcome in [
            MultiGpuOutcome::classify(&[]),
            MultiGpuOutcome::classify(&[gpu("card0", None)]),
            MultiGpuOutcome::classify(&[gpu("card0", None), gpu("card1", Some("renderD129"))]),
            MultiGpuOutcome::classify(&[gpu("card0", None), gpu("card1", None)]),
        ] {
            assert!(!outcome.marker().contains('\n'));
        }
    }
}
