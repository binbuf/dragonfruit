// SPDX-License-Identifier: MIT
//! Shared render plumbing (T-02).
//!
//! Backends build their element list from the [`Space`] via Smithay's
//! `render_output`/`output_elements` helpers — effects are compositor
//! render passes over live surface buffers, never client re-renders
//! ([02-compositor.md](../docs/design/02-compositor.md)).

use std::time::Duration;

use smithay::backend::renderer::element::surface::{
    render_elements_from_surface_tree, WaylandSurfaceRenderElement,
};
use smithay::backend::renderer::element::{Kind, RenderElementStates};
use smithay::backend::renderer::{ImportAll, Renderer};
use smithay::desktop::utils::{
    surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
    OutputPresentationFeedback,
};
use smithay::output::Output;
use smithay::utils::Scale;
use smithay::wayland::fractional_scale::with_fractional_scale;

use crate::state::DfState;

/// Build render elements for the shell's chrome surfaces (menu bar, Dock,
/// overlays) on `output`, in layer order (T-09).
///
/// Chrome surfaces are composited *above* the window [`Space`]; Smithay's
/// `render_output`/`DrmCompositor` take these as the custom-element list.
/// Only the `top` and `overlay` layers are handled here — background/bottom
/// layer stacking (wallpaper, desktop reveal) is a T-10/T-11 concern.
pub fn chrome_render_elements<R, E>(
    renderer: &mut R,
    state: &DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<E>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
    E: From<WaylandSurfaceRenderElement<R>>,
{
    let Some(geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    let mut elements = Vec::new();
    for chrome in state.chrome_surfaces(output.name().as_str(), geometry) {
        if chrome.layer < 2 {
            continue;
        }
        let location = chrome.location.to_physical_precise_round(scale);
        elements.extend(render_elements_from_surface_tree::<R, E>(
            renderer,
            &chrome.surface,
            location,
            scale,
            1.0,
            Kind::Unspecified,
        ));
    }
    elements
}

/// Collect presentation feedback for everything composited on `output`.
pub fn take_presentation_feedback(
    state: &DfState,
    output: &Output,
    states: &RenderElementStates,
) -> OutputPresentationFeedback {
    let mut feedback = OutputPresentationFeedback::new(output);
    for window in state.space.elements() {
        window.take_presentation_feedback(
            &mut feedback,
            surface_primary_scanout_output,
            |surface, _| surface_presentation_feedback_flags_from_states(surface, states),
        );
    }
    feedback
}

/// Send frame callbacks for the live windows of `output` on a backend with no
/// render report (headless).
///
/// Nested derives each surface's primary scan-out output from the render
/// element states ([`post_repaint`]); headless has neither a renderer nor a
/// report, so its single output is authoritative. Every live window mapped on
/// it is treated as presented and gets its pending `wl_callback.frame`
/// callbacks, exactly as a real backend would. This sends no pixels and
/// presents nothing — it exists so a CI client (and the T-11 FR-2 "a playing
/// video keeps playing" conformance test) advances through the same frame
/// path.
pub fn post_repaint_headless(state: &DfState, output: &Output, time: Duration) {
    for window in state.space.elements() {
        if state.space.outputs_for_element(window).contains(output) {
            // Throttle zero and an explicit primary output: the single headless
            // output is always the presentation target.
            window.send_frame(output, time, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }
    }
}

/// After a successful frame: send frame callbacks (throttled) and update
/// the preferred fractional scale the surfaces were displayed at.
pub fn post_repaint(
    state: &mut DfState,
    output: &Output,
    time: Duration,
    _states: &RenderElementStates,
) {
    let throttle = Some(Duration::from_secs(1));

    for window in state.space.elements() {
        window.with_surfaces(|surface, surface_states| {
            let primary_scanout_output = surface_primary_scanout_output(surface, surface_states);
            if let Some(output) = primary_scanout_output.as_ref() {
                with_fractional_scale(surface_states, |fractional_scale| {
                    fractional_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });

        if state.space.outputs_for_element(window).contains(output) {
            window.send_frame(output, time, throttle, surface_primary_scanout_output);
        }
    }
}
