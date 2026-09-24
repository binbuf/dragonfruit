// SPDX-License-Identifier: MIT
//! Shared render plumbing (T-02).
//!
//! Backends build their element list from the [`Space`] via Smithay's
//! `render_output`/`output_elements` helpers — effects are compositor
//! render passes over live surface buffers, never client re-renders
//! ([02-compositor.md](../docs/design/02-compositor.md)).

use std::time::{Duration, Instant};

use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::{
    render_elements_from_surface_tree, WaylandSurfaceRenderElement,
};
use smithay::backend::renderer::element::utils::{
    Relocate, RelocateRenderElement, RescaleRenderElement,
};
use smithay::backend::renderer::element::{AsRenderElements, Id, Kind, RenderElementStates};
use smithay::backend::renderer::utils::{with_renderer_surface_state, CommitCounter};
use smithay::backend::renderer::{Color32F, ImportAll, ImportMem, Renderer};
use smithay::desktop::utils::{
    surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
    OutputPresentationFeedback,
};
use smithay::output::Output;
use smithay::render_elements;
use smithay::utils::{IsAlive, Logical, Rectangle, Scale};
use smithay::wayland::fractional_scale::with_fractional_scale;

use crate::state::DfState;
use crate::wallpaper::sample_wallpaper;
use crate::window::{
    backdrop_elements, is_backdrop_panel, shadow_elements, MaterialRole, SceneTransform,
    ShadowLevel, WindowState,
};

// The two kinds of wallpaper element a backend can composite (T-05.4): a
// decoded image sampled into its fitted destination, or the solid fallback /
// letterbox fill. Both are drawn behind every window (callers append them
// last in the front-to-back list, so they sit at the bottom).
render_elements! {
    pub WallpaperRenderElement<R> where R: ImportAll + ImportMem;
    Image=MemoryRenderBufferRenderElement<R>,
    Solid=SolidColorRenderElement,
}

/// Build the per-Space wallpaper elements for `output` (T-05.4), which
/// composite **below** every window and the chrome.
///
/// Each [`crate::wallpaper::WallpaperSlot`] the state reports is drawn: an
/// image wallpaper is sampled through [`sample_wallpaper`] and imported from
/// the decode cache (never decoded here), and a solid fill is drawn behind it
/// for the letterbox / no-image case. During a workspace switch the active and
/// incoming Spaces are both reported at the same horizontal offsets the live
/// window surfaces take, so the background slides with its Space.
///
/// The caller passes the same `scale` the rest of the frame uses; the image is
/// scaled by the GPU sampler from the one cached raster, so a large source is
/// never re-decoded per frame.
pub fn wallpaper_render_elements<R>(
    renderer: &mut R,
    state: &mut DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<WallpaperRenderElement<R>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + Send + 'static,
{
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    let mut elements = Vec::new();
    for slot in state.wallpaper_slots(output) {
        // The slot's rectangle: the whole output translated by the slide
        // offset, so the fallback fill moves with the Space too.
        let target = Rectangle::new((slot.offset_x, 0).into(), output_geometry.size);
        let color = slot.wallpaper.color;
        // Front-to-back: the image composites over its own solid fallback.
        let image = slot.wallpaper.source.as_deref().and_then(|source| {
            let decoded = state.wallpaper_cache.get(source)?;
            let sample = sample_wallpaper(decoded.size, slot.wallpaper.fit, target);
            if sample.dest.size.w <= 0 || sample.dest.size.h <= 0 {
                return None;
            }
            MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                sample.dest.loc.to_f64().to_physical(scale),
                decoded.buffer(),
                Some(1.0),
                Some(sample.src),
                Some(sample.dest.size),
                Kind::Unspecified,
            )
            .ok()
        });
        if let Some(image) = image {
            elements.push(WallpaperRenderElement::Image(image));
        }
        elements.push(WallpaperRenderElement::Solid(SolidColorRenderElement::new(
            Id::new(),
            target.to_physical_precise_round(scale),
            CommitCounter::default(),
            Color32F::new(color[0], color[1], color[2], color[3]),
            Kind::Unspecified,
        )));
    }
    elements
}

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

/// Build the token-driven backdrop blur for the chrome surfaces composited on
/// `output` (T-04.2), which composite **below** their chrome surface.
///
/// The backdrop is the material under a translucent chrome surface: a
/// token-derived frosted panel (see [`crate::window::backdrop`]) whose corners
/// come from the component radius. Callers append these **after**
/// [`chrome_render_elements`] and **before** the window surfaces, so the panel
/// sits behind its chrome and in front of the windows it stands in for.
///
/// The pass is opened once per rendered frame ([`DfState::begin_render_frame`])
/// and applied once per output; a second request for the same output in the
/// same frame draws nothing, which is the T-04.2 "one effect pass per frame"
/// invariant. Client translucent regions are honored: the chrome surface's own
/// buffer composites over the backdrop rather than being clipped out.
///
/// The resolved material is mapped through the active [`DegradeTier`]
/// (T-04.4a): `Reduced` shrinks the feather geometry, and `Minimal` turns the
/// blur off entirely (no elements, no damage). [`DegradeTier`]:
/// crate::window::DegradeTier
pub fn chrome_backdrop_render_elements(
    state: &mut DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<SolidColorRenderElement> {
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    // Only *panels that are actually on screen* get a backdrop:
    //
    // * A layer surface that has not committed a buffer is unmapped. The shell
    //   still gives it geometry (the overview is sized offscreen before its
    //   first open), but there are no pixels to sit behind, so frosting that
    //   rect would just wash the scene.
    // * A surface that fills the whole output is a scene-wide overlay (Mission
    //   Control, Desktop Reveal), not a translucent panel; it draws its own
    //   scrim over the live scene. Without this the T-05 overview overlay would
    //   frost the entire desktop at every degrade tier except `minimal`.
    let output_size = output_geometry.size;
    let surfaces: Vec<_> = state
        .chrome_surfaces(output.name().as_str(), output_geometry)
        .into_iter()
        .filter(|chrome| chrome.layer >= 2)
        .filter(|chrome| is_backdrop_panel(chrome.panel, output_size))
        .filter(|chrome| {
            with_renderer_surface_state(&chrome.surface, |surface| surface.buffer().is_some())
                .unwrap_or(false)
        })
        .collect();
    if surfaces.is_empty() {
        return Vec::new();
    }
    // The live scheme (T-04.4b) selects the per-scheme material tokens; the
    // pass renders in whatever light/dark the session currently is.
    let scheme = state.color_scheme;
    // Resolve the token material per surface and map it through the current
    // degrade tier (T-04.4a). `None` means "blur off" (the `Minimal` tier), so
    // the pass draws nothing and owns no damage: forcing the tier visibly
    // changes the pass.
    let specs: Vec<_> = surfaces
        .iter()
        .map(|chrome| {
            let role = MaterialRole::from_layer(chrome.layer);
            state.degrade.tier().backdrop(role.spec(scheme))
        })
        .collect();
    if specs.iter().all(Option::is_none) {
        return Vec::new();
    }
    // The chrome band: the union of the top/overlay surfaces on this output,
    // in output-local logical coordinates. This is the damage the pass owns.
    let region = surfaces
        .iter()
        .map(|chrome| chrome.panel)
        .reduce(|a, b| a.merge(b))
        .unwrap_or_default();
    if state
        .material_pass
        .apply(output.name().as_str(), region)
        .is_none()
    {
        return Vec::new();
    }
    let mut elements = Vec::new();
    for (chrome, spec) in surfaces.iter().zip(&specs) {
        if let Some(spec) = spec {
            elements.extend(backdrop_elements(chrome.panel, *spec, scale));
        }
    }
    elements
}

/// Build the client-surface render elements for the windows composited on
/// `output`, applying each window's lifecycle motion (T-02.1b/T-02.2).
///
/// This is the per-window replacement for the window half of
/// `smithay::desktop::space::render_output`: it walks the [`Space`] in the
/// same front-to-back order and resolves the same locations/scale, but wraps
/// a window that is mid-appear/restore/zoom/fullscreen in a
/// `Rescale`+`Relocate` pair and fades it through the surface-tree alpha when
/// the motion has a fade, so the window scales/fades from its origin. A window
/// with no in-flight motion produces byte-identical elements to the space
/// path.
///
/// A **minimizing** window is unmapped from the `Space` (input-inert) but its
/// surface is held as a ghost until the motion completes; those are rendered
/// from the window model after the space walk with the same transform.
///
/// Every in-flight window contributes to the reusable scene-transform pass
/// (T-04.3): the output composes at most one transform per frame, recorded via
/// [`DfState::scene_pass`], so the scene is never transformed twice.
///
/// [`Space`]: smithay::desktop::Space
pub fn window_render_elements<R, E>(
    renderer: &mut R,
    state: &mut DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<E>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
    E: From<WaylandSurfaceRenderElement<R>>
        + From<RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>>,
{
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    let now = state.now_msec();
    let mut elements: Vec<E> = Vec::new();
    // The union of the transformed window rects (global logical) this output
    // composed; `None` when every window draws untransformed.
    let mut scene_region: Option<Rectangle<i32, Logical>> = None;
    // `Space::elements` is bottom-to-top; the damage tracker consumes
    // front-to-back, so iterate in reverse (exactly like the space path).
    for window in state.space.elements().rev() {
        if !state.space.outputs_for_element(window).contains(output) {
            continue;
        }
        let Some(location) = state.space.element_location(window) else {
            continue;
        };
        // The surface origin is the element location minus the window's own
        // geometry offset (mirrors `InnerElement::render_location`). The bbox
        // is shifted the same way for the output-overlap test.
        let render_location = location - window.geometry().loc;
        let mut bbox = window.bbox_with_popups();
        bbox.loc += render_location;
        if !output_geometry.overlaps(bbox) {
            continue;
        }
        let location = render_location - output_geometry.loc;
        let location_phys = location.to_physical_precise_round(scale);
        match state.window_render_frame(window, now) {
            Some(frame) => {
                let transformed = push_motion_elements(
                    renderer,
                    &mut elements,
                    window,
                    location_phys,
                    scale,
                    frame,
                );
                merge_region(&mut scene_region, transformed);
            }
            None => {
                elements.extend(window.render_elements::<E>(renderer, location_phys, scale, 1.0));
            }
        }
    }
    // Minimizing/closing ghosts: unmapped from `Space` but still drawn until
    // the motion completes.
    for (window, motion) in state.windows.active_motions() {
        if !motion.kind.is_ghost() {
            continue;
        }
        if !window.alive() {
            continue;
        }
        if state.space.element_location(window).is_some() {
            continue;
        }
        // A window on an inactive Space is not visible at all, ghost included.
        if !state.window_on_active_space(window) {
            continue;
        }
        let Some(location) = state.windows.geometry(window) else {
            continue;
        };
        let render_location = location.loc - window.geometry().loc;
        let mut bbox = window.bbox_with_popups();
        bbox.loc += render_location;
        if !output_geometry.overlaps(bbox) {
            continue;
        }
        let location = render_location - output_geometry.loc;
        let location_phys = location.to_physical_precise_round(scale);
        if let Some(frame) = state.window_render_frame(window, now) {
            let transformed =
                push_motion_elements(renderer, &mut elements, window, location_phys, scale, frame);
            merge_region(&mut scene_region, transformed);
        }
    }
    // Open the reusable scene-transform pass for this output at most once per
    // frame (T-04.3). The damage is the union of the interpolated window
    // rects, converted to output-local logical coordinates.
    if let Some(mut region) = scene_region {
        region.loc -= output_geometry.loc;
        state.scene_pass.apply(output.name().as_str(), region);
    }
    elements
}

/// Extend `region` with `rect`, or set it when empty. The scene-transform
/// pass's damage is the union of every transformed window rect on an output.
fn merge_region(region: &mut Option<Rectangle<i32, Logical>>, rect: Rectangle<i32, Logical>) {
    *region = Some(match *region {
        Some(existing) => existing.merge(rect),
        None => rect,
    });
}

/// Push one window's surface elements wrapped in `Rescale`+`Relocate` with
/// the motion frame's scale/fade. Shared by the `Space` walk and the
/// minimizing-ghost walk.
///
/// The wrapping is expressed through the reusable [`SceneTransform`] (T-04.3):
/// the transform's scale/offset are exactly the `Rescale`+`Relocate` pair, so
/// the lifecycle motion and T-05's grid share one mapping. Returns the
/// transform's target rect (global logical), for the pass's damage union.
fn push_motion_elements<R, E>(
    renderer: &mut R,
    elements: &mut Vec<E>,
    window: &smithay::desktop::Window,
    location_phys: smithay::utils::Point<i32, smithay::utils::Physical>,
    scale: Scale<f64>,
    frame: crate::window::MotionFrame,
) -> Rectangle<i32, Logical>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
    E: From<WaylandSurfaceRenderElement<R>>
        + From<RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>>,
{
    // Fade through the surface-tree alpha, then scale about the window's
    // physical target origin and translate to the interpolated rect.
    //
    // The scale is relative to the surface's *current* size rather than the
    // motion target: appear/minimize draw a buffer already at the target size
    // (so the two are equal), while a zoom/fullscreen resizes the client
    // mid-flight and each committed buffer must map onto the interpolated
    // rect for the geometry to stay continuous.
    let transform = SceneTransform::from_motion(frame, window.geometry().size);
    let surface_scale = transform.scale();
    let fade = window.render_elements::<WaylandSurfaceRenderElement<R>>(
        renderer,
        location_phys,
        scale,
        frame.alpha,
    );
    for element in fade {
        let scaled = RescaleRenderElement::from_element(element, location_phys, surface_scale);
        let relocated = RelocateRenderElement::from_element(
            scaled,
            transform.offset().to_physical_precise_round(scale),
            Relocate::Relative,
        );
        elements.push(relocated.into());
    }
    transform.target
}

/// Build the elevation-token-driven drop shadows for the windows composited
/// on `output` (T-04.1a), which composite *below* their client surface.
///
/// A shadow is a custom element like the SSD titlebar, but it must sit behind
/// its window: callers append these **after** [`window_render_elements`] in
/// the front-to-back list. The whole decorated window (the SSD titlebar strip
/// included) is shadowed, from `component.elevation.high` and the active
/// scheme's `material.shadowOpacity`/`color.shadowColor`, and a window
/// mid-motion carries the same lifecycle transform as its content. Fullscreen
/// windows are not shadowed; a minimizing/closing ghost is.
pub fn window_shadow_render_elements(
    state: &DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<SolidColorRenderElement> {
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    let now = state.now_msec();
    // The live scheme follows desktop settings (T-08); dark is the default
    // that reads over arbitrary application pixels, matching the titlebar.
    // The elevation geometry is mapped through the material degrade tier
    // (T-04.4a): budget pressure tightens the spread/layers while keeping the
    // scheme tone, so windows stay legible at every tier. While Mission
    // Control is open the grid material (T-05.1b) is the single source, so the
    // tier the overview composes with is exactly the one `query grid` reports.
    let spec = match state.overview_grid_material() {
        Some(material) => material.shadow,
        None => state
            .degrade
            .tier()
            .shadow(ShadowLevel::High.spec(state.color_scheme)),
    };
    let mut elements = Vec::new();
    for window in state.space.elements() {
        if !state.space.outputs_for_element(window).contains(output) {
            continue;
        }
        // A fullscreen window is the whole output; it casts no shadow.
        if state.windows.state(window) == Some(WindowState::Fullscreen) {
            continue;
        }
        let Some(geometry) = state.windows.geometry(window) else {
            continue;
        };
        let window_rect = state.insets_for(window).outset(geometry);
        elements.extend(shadow_elements(
            window_rect,
            spec,
            scale,
            output_geometry.loc,
            state.window_render_frame(window, now),
        ));
    }
    // Minimizing/closing ghosts carry the same shadow as their titlebar and
    // surface, even though the window is already unmapped from `Space`.
    for (window, motion) in state.windows.active_motions() {
        if !motion.kind.is_ghost() {
            continue;
        }
        if !window.alive() {
            continue;
        }
        if state.space.element_location(window).is_some() {
            continue;
        }
        if !state.window_on_active_space(window) {
            continue;
        }
        let Some(geometry) = state.windows.geometry(window) else {
            continue;
        };
        if !output_geometry.overlaps(geometry) {
            continue;
        }
        let window_rect = state.insets_for(window).outset(geometry);
        elements.extend(shadow_elements(
            window_rect,
            spec,
            scale,
            output_geometry.loc,
            state.window_render_frame(window, now),
        ));
    }
    elements
}

/// Build the solid-fill SSD titlebar elements for the windows composited on
/// `output` (T-01.1). These are custom elements, so they composite *above*
/// the window [`Space`](smithay::desktop::Space) — the titlebar sits above
/// its own client surface. A window mid-motion carries the same scale/fade as
/// its content (T-02.1b/T-02.2), including a minimizing ghost whose window is
/// already unmapped.
pub fn titlebar_render_elements(
    state: &DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<SolidColorRenderElement> {
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    let now = state.now_msec();
    let mut elements = Vec::new();
    for window in state.space.elements() {
        if !state.space.outputs_for_element(window).contains(output) {
            continue;
        }
        if let Some(titlebar) = state.titlebar_element(window) {
            elements.extend(titlebar.render_elements(
                scale,
                output_geometry.loc,
                state.window_render_frame(window, now),
            ));
        }
    }
    // Titlebars for minimizing/closing ghosts (unmapped from `Space` but
    // still drawn).
    for (window, motion) in state.windows.active_motions() {
        if !motion.kind.is_ghost() {
            continue;
        }
        if !window.alive() {
            continue;
        }
        if state.space.element_location(window).is_some() {
            continue;
        }
        // A ghost on an inactive Space is not visible.
        if !state.window_on_active_space(window) {
            continue;
        }
        // A ghost is not in `Space`, so the per-output ownership test is the
        // model geometry's overlap with this output.
        let Some(geometry) = state.windows.geometry(window) else {
            continue;
        };
        if !output_geometry.overlaps(geometry) {
            continue;
        }
        if let Some(titlebar) = state.titlebar_element_for_motion(window) {
            elements.extend(titlebar.render_elements(
                scale,
                output_geometry.loc,
                state.window_render_frame(window, now),
            ));
        }
    }
    elements
}

/// Build the solid-fill elements for the open window menu (T-01.4), if any.
///
/// The menu is compositor chrome drawn above the titlebars on the output it
/// was opened on. The panel geometry is clamped to that output when it opens,
/// so the anchor's output is the one that draws it.
pub fn window_menu_render_elements(
    state: &DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<SolidColorRenderElement> {
    let Some(menu) = state.window_menu.as_ref() else {
        return Vec::new();
    };
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    if !output_geometry.contains(menu.anchor) {
        return Vec::new();
    }
    menu.render_elements(scale, output_geometry.loc)
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
pub fn post_repaint_headless(state: &mut DfState, output: &Output, time: Duration) {
    let mut wakeups = 0u64;
    for window in state.space.elements() {
        if state.space.outputs_for_element(window).contains(output) {
            // Throttle zero and an explicit primary output: the single headless
            // output is always the presentation target.
            window.send_frame(output, time, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
            wakeups += 1;
        }
    }
    // One frame-callback batch per live window: the idle budget's direct
    // "client wakeups" counter (T-03.1a).
    state.stats.client_wakeups += wakeups;
    // This is a presented frame (headless's stand-in for the photon): credit
    // any pending input with its input-to-photon round trip (T-03.1b).
    state.stats.latency.note_present(Instant::now());
}

/// Count the live windows composited on `output` — the frames a real backend
/// would wake with a frame-callback batch. Shared with the DRM backend, whose
/// frame callbacks are queued by Smithay's `DrmCompositor` rather than here.
pub fn count_output_windows(state: &DfState, output: &Output) -> u64 {
    state
        .space
        .elements()
        .filter(|window| state.space.outputs_for_element(window).contains(output))
        .count() as u64
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

    let mut wakeups = 0u64;
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
            wakeups += 1;
        }
    }
    // One frame-callback batch per live window: the idle budget's direct
    // "client wakeups" counter (T-03.1a).
    state.stats.client_wakeups += wakeups;
    // This frame was presented (nested `submit`): credit any pending input
    // with its input-to-photon round trip (T-03.1b).
    state.stats.latency.note_present(Instant::now());
}
