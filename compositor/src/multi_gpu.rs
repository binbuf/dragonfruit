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

/// Where one output's T-20 material pass samples its scene texture from under
/// `renderer_multi` (T-20.4, [ADR 0182]).
///
/// The material needs an output-sized **scene texture** to blur. The texture is
/// owned by the GPU that rendered the scene for it; the pass must never sample
/// a texture that lives on another GPU. This decision is pure so it can be
/// asserted without two cards.
///
/// [ADR 0182]: ../../docs/design/adr/0182-tahoe-liquid-glass-material-pass.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneRoute {
    /// The output's GPU owns the scene texture: the blur/glass pass composes
    /// locally, with no cross-GPU work (the preferred path).
    Local,
    /// The output renders on a different GPU than the one that owns the scene
    /// texture: the texture is copied/imported onto the output's GPU before the
    /// pass samples it. The copy is the fallback for a panel straddling
    /// outputs; the material never samples across devices.
    Copy { from: String, to: String },
    /// No GPU scene texture exists (software/headless): the deterministic
    /// `Minimal` feather stack is used and nothing is read back.
    Software,
}

impl SceneRoute {
    /// The stable name used by the trace marker.
    pub const fn name(&self) -> &'static str {
        match self {
            SceneRoute::Local => "local",
            SceneRoute::Copy { .. } => "copy",
            SceneRoute::Software => "software",
        }
    }

    /// True when the route crosses devices and a copy is required.
    pub const fn is_cross_gpu(&self) -> bool {
        matches!(self, SceneRoute::Copy { .. })
    }

    /// The one-line marker the backend prints for one output.
    pub fn marker(&self, output: &str) -> String {
        match self {
            SceneRoute::Local => format!("Multi-GPU: SCENE output={output} route=local"),
            SceneRoute::Copy { from, to } => {
                format!("Multi-GPU: SCENE output={output} route=copy from={from} to={to}")
            }
            SceneRoute::Software => {
                format!("Multi-GPU: SCENE output={output} route=software (minimal feather)")
            }
        }
    }
}

/// Decide the scene-texture route for one output. `scene_texture_gpu` is the
/// GPU that owns (rendered) the scene texture, or `None` on the software path;
/// `output_gpu` is the GPU that renders the output.
pub fn scene_route(scene_texture_gpu: Option<&str>, output_gpu: &str) -> SceneRoute {
    match scene_texture_gpu {
        None => SceneRoute::Software,
        Some(owner) if owner == output_gpu => SceneRoute::Local,
        Some(owner) => SceneRoute::Copy {
            from: owner.to_string(),
            to: output_gpu.to_string(),
        },
    }
}

/// One output's routing decision, in the backend's output order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSceneRoute {
    pub output: String,
    pub route: SceneRoute,
}

/// Plan every output's scene-texture route against one scene-texture owner.
/// `outputs` are `(output_name, output_gpu)` pairs.
pub fn plan_scene_routes(
    scene_texture_gpu: Option<&str>,
    outputs: &[(String, String)],
) -> Vec<OutputSceneRoute> {
    outputs
        .iter()
        .map(|(output, gpu)| OutputSceneRoute {
            output: output.clone(),
            route: scene_route(scene_texture_gpu, gpu),
        })
        .collect()
}

/// A one-line plan summary for the trace, or the empty string when there are
/// no outputs.
pub fn scene_routes_marker(routes: &[OutputSceneRoute]) -> String {
    routes
        .iter()
        .map(|entry| entry.route.marker(&entry.output))
        .collect::<Vec<_>>()
        .join("\n")
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
    fn a_scene_texture_routes_locally_on_its_own_gpu() {
        let route = scene_route(Some("renderD128"), "renderD128");
        assert_eq!(route, SceneRoute::Local);
        assert!(!route.is_cross_gpu());
        assert_eq!(route.name(), "local");
        assert_eq!(
            route.marker("HDMI-A-1"),
            "Multi-GPU: SCENE output=HDMI-A-1 route=local"
        );
    }

    #[test]
    fn an_output_on_another_gpu_copies_the_scene_texture() {
        let route = scene_route(Some("renderD128"), "renderD129");
        assert_eq!(
            route,
            SceneRoute::Copy {
                from: "renderD128".into(),
                to: "renderD129".into()
            }
        );
        assert!(route.is_cross_gpu());
        assert_eq!(
            route.marker("DP-1"),
            "Multi-GPU: SCENE output=DP-1 route=copy from=renderD128 to=renderD129"
        );
    }

    #[test]
    fn the_software_path_has_no_scene_texture_and_stays_minimal() {
        let route = scene_route(None, "renderD128");
        assert_eq!(route, SceneRoute::Software);
        assert!(!route.is_cross_gpu());
        assert_eq!(
            route.marker("HEADLESS-1"),
            "Multi-GPU: SCENE output=HEADLESS-1 route=software (minimal feather)"
        );
    }

    #[test]
    fn a_plan_routes_every_output_independently() {
        let outputs = vec![
            ("eDP-1".to_string(), "renderD128".to_string()),
            ("DP-1".to_string(), "renderD129".to_string()),
        ];
        // The scene texture is owned by the primary: its own output is local,
        // the secondary copies across.
        let plan = plan_scene_routes(Some("renderD128"), &outputs);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].route, SceneRoute::Local);
        assert!(plan[1].route.is_cross_gpu());

        let marker = scene_routes_marker(&plan);
        assert!(marker.contains("output=eDP-1 route=local"), "{marker}");
        assert!(
            marker.contains("output=DP-1 route=copy from=renderD128 to=renderD129"),
            "{marker}"
        );
        assert!(!marker.contains('\n') || marker.lines().count() == 2);

        // No scene texture at all: every output falls back to software.
        let plan = plan_scene_routes(None, &outputs);
        assert!(plan.iter().all(|entry| entry.route == SceneRoute::Software));
        assert!(scene_routes_marker(&[]).is_empty());
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
