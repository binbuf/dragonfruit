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
use smithay::backend::renderer::element::{AsRenderElements, Kind, RenderElementStates};
use smithay::backend::renderer::utils::{with_renderer_surface_state, CommitCounter};
use smithay::backend::renderer::{Color32F, ImportAll, ImportMem, Renderer};
use smithay::desktop::utils::{
    send_frames_surface_tree, surface_presentation_feedback_flags_from_states,
    surface_primary_scanout_output, OutputPresentationFeedback,
};
use smithay::output::Output;
use smithay::render_elements;
use smithay::utils::{IsAlive, Logical, Rectangle, Scale};
use smithay::wayland::compositor::{with_states, SurfaceAttributes};
use smithay::wayland::fractional_scale::with_fractional_scale;
use wayland_server::Resource;

use crate::state::DfState;
use crate::wallpaper::sample_wallpaper;
use crate::window::{
    backdrop_elements, is_backdrop_panel, shadow_elements, BlurSpec, MaterialRole, PanelKey,
    SceneTransform, ShadowLevel, ShadowSpec, WindowState,
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
        let physical = target.to_physical_precise_round(scale);
        let fill = Color32F::new(color[0], color[1], color[2], color[3]);
        elements.push(WallpaperRenderElement::Solid(SolidColorRenderElement::new(
            crate::window::stable_solid_id(0x7761_6c6c_7061_7065, physical, fill),
            physical,
            CommitCounter::default(),
            fill,
            Kind::Unspecified,
        )));
    }
    elements
}

/// Build render elements for the shell's chrome surfaces (menu bar, Dock,
/// overlays) on `output`, topmost first (T-09).
///
/// Chrome surfaces are composited *above* the window [`Space`]; Smithay's
/// `render_output`/`DrmCompositor` take these as the custom-element list in
/// front-to-back order, so [`chrome_surfaces`](crate::shell::DfState::chrome_surfaces)'s
/// bottom-to-top list is reversed here. Only the above-window `top`/`overlay`
/// layers are handled; the `background` desktop layer is composited below
/// windows by [`desktop_render_elements`] (T-19.3).
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
    // `chrome_surfaces` is ordered bottom-to-top (layer 2 then 3), but this
    // list is consumed front-to-back: reverse it so overlays (menus, the
    // Applications drawer) render above the persistent menu bar and Dock.
    for chrome in state
        .visible_chrome_surfaces(output.name().as_str(), geometry)
        .into_iter()
        .rev()
    {
        if !crate::shell::layer::above_windows(chrome.layer) {
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

/// Build render elements for the Files-owned **desktop layer** surfaces on
/// `output` (T-19.3).
///
/// The desktop-icon surface is a `background`-layer `df_layer_surface`: it
/// composites **above the per-Space wallpaper and below every window**, so
/// desktop items sit behind open windows. Callers append these elements
/// between [`window_render_elements`] and [`wallpaper_render_elements`] in the
/// front-to-back list, which is exactly the `Wallpaper < Desktop < Windows <
/// Chrome` band order asserted by `shell::layer`'s unit test.
pub fn desktop_render_elements<R, E>(
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
    for chrome in state.visible_chrome_surfaces(output.name().as_str(), geometry) {
        if !crate::shell::layer::is_desktop(chrome.layer) {
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
        .visible_chrome_surfaces(output.name().as_str(), output_geometry)
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
    // Resolve the token material per surface through that role's **effective**
    // state (T-21.2c): persistent chrome keeps the Reduced floor while the
    // sampled path is available, transient overlays may reach Minimal, and a
    // role whose sampled pass is absent takes its milky fallback (no feather
    // elements). The software/headless path always resolves to Minimal, so a
    // backend without a GPU scene texture cannot drift from the deterministic
    // feather geometry.
    let visible: Vec<_> = surfaces
        .iter()
        .filter_map(|chrome| {
            let role = MaterialRole::from_layer_namespace(chrome.layer, &chrome.namespace);
            let material = state.effective_material_for_role(role);
            if !material.blur_active {
                return None;
            }
            material
                .tier
                .backdrop(role.spec_for(scheme, &chrome.namespace))
                .map(|spec| (chrome, spec))
        })
        .collect();
    if visible.is_empty() {
        return Vec::new();
    }
    // The chrome band: the union of the panels that actually draw, in
    // output-local logical coordinates. This is the damage the pass owns.
    let region = visible
        .iter()
        .map(|(chrome, _)| chrome.panel)
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
    // Backdrops follow the same front-to-back rule as the chrome surfaces:
    // the topmost overlay's panel tint is drawn before lower chrome's.
    for (chrome, spec) in visible.iter().rev() {
        elements.extend(backdrop_elements(chrome.panel, *spec, scale));
    }
    elements
}

/// One chrome panel that should receive the GPU-sampled backdrop blur
/// (T-20.1): the visible panel rect and its token-resolved blur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurPanel {
    /// Stable identity of the chrome surface this panel belongs to (T-21.3).
    /// The panel rect may move (Dock magnification) without changing the
    /// panel's render-element identity.
    pub key: PanelKey,
    pub panel: Rectangle<i32, Logical>,
    pub spec: BlurSpec,
    /// The Tahoe liquid-glass parameters for this panel (T-20.2).
    pub glass: crate::window::GlassSpec,
}

/// Resolve the GPU backdrop blur panels for `output` and apply the material
/// pass (T-20.1).
///
/// Returns the panels to blur when the active degrade tier keeps blur on and at
/// least one chrome panel is mapped and on screen. Returns empty when the blur
/// is off (`Minimal`), no panel is mapped, or the pass already applied for this
/// output in this frame; the caller then falls back to the feather render
/// elements ([`chrome_backdrop_render_elements`]), which owns the same
/// one-pass guard.
///
/// The blur samples the caller's offscreen **scene texture** inside the render
/// pass, never a screenshot or screencopy.
pub fn chrome_blur_panels(state: &mut DfState, output: &Output) -> Vec<BlurPanel> {
    // The software/headless path (T-20.4) has no GPU scene texture, so the
    // sampled-material pass never runs there: the caller falls back to the
    // deterministic feather render elements, with no readback.
    if !state.material_path.gpu_blur() {
        return Vec::new();
    }
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    let output_size = output_geometry.size;
    let surfaces: Vec<_> = state
        .visible_chrome_surfaces(output.name().as_str(), output_geometry)
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
    let scheme = state.color_scheme;
    let mut panels: Vec<BlurPanel> = surfaces
        .iter()
        .filter_map(|chrome| {
            let role = MaterialRole::from_layer_namespace(chrome.layer, &chrome.namespace);
            let material = state.effective_material_for_role(role);
            if !material.blur_active {
                return None;
            }
            let spec = material
                .tier
                .blur(role.blur_spec_for(scheme, &chrome.namespace))?;
            let glass = material.tier.glass(role.glass_spec(scheme))?;
            Some(BlurPanel {
                key: PanelKey::new(
                    &chrome.namespace,
                    chrome.layer,
                    chrome.surface.id().protocol_id(),
                ),
                panel: chrome.panel,
                spec,
                glass,
            })
        })
        .collect();
    if panels.is_empty() {
        return Vec::new();
    }
    // The blur/glass elements are appended front-to-back too: an overlay's
    // panel must sit above the persistent chrome it covers.
    panels.reverse();
    // The chrome band: the union of the panels (the damage the pass owns).
    let region = panels
        .iter()
        .map(|panel| panel.panel)
        .reduce(|a, b| a.merge(b))
        .unwrap_or_default();
    if state
        .material_pass
        .apply(output.name().as_str(), region)
        .is_none()
    {
        return Vec::new();
    }
    // T-20.4 instrumentation: the pass actually ran, so record the output-frame
    // and the panel count / largest downsample the trace reports.
    let downsample_max = panels
        .iter()
        .map(|panel| panel.spec.downsample())
        .max()
        .unwrap_or(0);
    state
        .stats
        .record_blur_pass(downsample_max, panels.len() as u64);
    panels
}

/// Build the **complete** window stack for `output`: each window's SSD
/// titlebar, client surface, and drop shadow, interleaved in the window
/// stacking order.
///
/// The compositor used to emit every titlebar as one global layer above every
/// client surface, and every shadow as another global layer below them. That
/// splits one decorated window across two z-planes: a background window's
/// titlebar painted over a foreground window's body, and a foreground window's
/// shadow painted under a background window. This walk keeps each window's
/// three pieces together (titlebar above its own content, shadow below it and
/// above the windows underneath), which is the stacking order the `Space`
/// itself defines.
///
/// The open window menu is still a separate layer above this stack: it is a
/// transient popup, not part of any window.
pub fn window_scene_render_elements<R, E>(
    renderer: &mut R,
    state: &mut DfState,
    output: &Output,
    scale: Scale<f64>,
) -> Vec<E>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
    E: From<WaylandSurfaceRenderElement<R>>
        + From<RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>>
        + From<SolidColorRenderElement>,
{
    let Some(output_geometry) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    let now = state.now_msec();
    let shadow_spec = resolved_shadow_spec(state);
    let mut elements: Vec<E> = Vec::new();
    let mut scene_region: Option<Rectangle<i32, Logical>> = None;
    // `Space::elements` is bottom-to-top; the damage tracker consumes
    // front-to-back, so iterate in reverse.
    for window in state.space.elements().rev() {
        if !state.space.outputs_for_element(window).contains(output) {
            continue;
        }
        let Some(location) = state.space.element_location(window) else {
            continue;
        };
        let render_location = location - window.geometry().loc;
        let mut bbox = window.bbox_with_popups();
        bbox.loc += render_location;
        if !output_geometry.overlaps(bbox) {
            continue;
        }
        let location = render_location - output_geometry.loc;
        let location_phys = location.to_physical_precise_round(scale);
        let motion = state.window_render_frame(window, now);

        // The titlebar belongs to this window and sits above its content.
        if let Some(titlebar) = state.titlebar_element(window) {
            elements.extend(
                titlebar
                    .render_elements(scale, output_geometry.loc, motion)
                    .into_iter()
                    .map(E::from),
            );
        }
        if let Some(transformed) = push_window_content(
            renderer,
            &mut elements,
            window,
            location_phys,
            scale,
            motion,
        ) {
            merge_region(&mut scene_region, transformed);
        }
        // The shadow sits below its own content, but above the windows it
        // covers — not in a global layer under the whole stack.
        push_window_shadow(
            &mut elements,
            state,
            window,
            output_geometry,
            scale,
            shadow_spec,
            motion,
        );
    }
    // Minimizing/closing ghosts keep their complete decoration stack too.
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
        let frame = state.window_render_frame(window, now);
        if let Some(titlebar) = state.titlebar_element_for_motion(window) {
            elements.extend(
                titlebar
                    .render_elements(scale, output_geometry.loc, frame)
                    .into_iter()
                    .map(E::from),
            );
        }
        if let Some(frame) = frame {
            if let Some(transformed) = push_window_content(
                renderer,
                &mut elements,
                window,
                location_phys,
                scale,
                Some(frame),
            ) {
                merge_region(&mut scene_region, transformed);
            }
        }
        push_window_shadow(
            &mut elements,
            state,
            window,
            output_geometry,
            scale,
            shadow_spec,
            frame,
        );
    }
    if let Some(mut region) = scene_region {
        region.loc -= output_geometry.loc;
        state.scene_pass.apply(output.name().as_str(), region);
    }
    elements
}

/// Push one window's client surface elements, transformed by its lifecycle
/// motion when one is live. Returns the transform's target rect (global
/// logical) for the scene pass's damage union.
fn push_window_content<R, E>(
    renderer: &mut R,
    elements: &mut Vec<E>,
    window: &smithay::desktop::Window,
    location_phys: smithay::utils::Point<i32, smithay::utils::Physical>,
    scale: Scale<f64>,
    motion: Option<crate::window::MotionFrame>,
) -> Option<Rectangle<i32, Logical>>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
    E: From<WaylandSurfaceRenderElement<R>>
        + From<RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>>,
{
    match motion {
        Some(frame) => Some(push_motion_elements(
            renderer,
            elements,
            window,
            location_phys,
            scale,
            frame,
        )),
        None => {
            elements.extend(window.render_elements::<E>(renderer, location_phys, scale, 1.0));
            None
        }
    }
}

/// Push one window's elevation-token shadow into `elements`.
fn push_window_shadow<E>(
    elements: &mut Vec<E>,
    state: &DfState,
    window: &smithay::desktop::Window,
    output_geometry: Rectangle<i32, Logical>,
    scale: Scale<f64>,
    spec: ShadowSpec,
    motion: Option<crate::window::MotionFrame>,
) where
    E: From<SolidColorRenderElement>,
{
    // A fullscreen window is the whole output; it casts no shadow.
    if state.windows.state(window) == Some(WindowState::Fullscreen) {
        return;
    }
    let Some(geometry) = state.windows.geometry(window) else {
        return;
    };
    let window_rect = state.insets_for(window).outset(geometry);
    elements.extend(
        shadow_elements(window_rect, spec, scale, output_geometry.loc, motion)
            .into_iter()
            .map(E::from),
    );
}

/// Resolve the active scheme/degrade shadow geometry once per output frame.
fn resolved_shadow_spec(state: &DfState) -> ShadowSpec {
    match state
        .overview_grid_material()
        .or_else(|| state.switcher_material())
    {
        Some(material) => material.shadow,
        None => state
            .degrade
            .tier()
            .shadow(ShadowLevel::High.spec(state.color_scheme)),
    }
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

/// Build the solid-fill elements for the open window menu (T-01.4), if any.
///
/// The menu is compositor chrome drawn above the complete window stack
/// (titlebars, client surfaces, and shadows) on the output it was opened on. The panel geometry is clamped to that output when it opens,
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

/// Send the pending `wl_surface.frame` callbacks of every chrome surface
/// composited on `output` and return how many were answered.
///
/// Chrome is paced by presentation just like a window: the shell requests a
/// callback before each commit, and only renders the next QML frame when this
/// callback arrives. There is deliberately no 1 s throttle here (the window
/// path's throttle is for occluded/background windows); a chrome surface that
/// is visible on the output is the shell's pacing source. The callback count
/// feeds the T-21.1a `chrome_frame_callbacks` counter.
fn send_chrome_frame_callbacks(state: &DfState, output: &Output, time: Duration) -> u64 {
    let Some(geometry) = state.space.output_geometry(output) else {
        return 0;
    };
    let mut sent = 0u64;
    // Callbacks are answered for every chrome surface, including the
    // persistent `top` chrome hidden while a fullscreen client owns the
    // output. Otherwise the shell would wait forever for a callback on a
    // surface the compositor is deliberately not drawing (T-21.3).
    for chrome in state.chrome_surfaces(output.name().as_str(), geometry) {
        let pending = with_states(&chrome.surface, |states| {
            states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .len()
        });
        if pending == 0 {
            continue;
        }
        send_frames_surface_tree(&chrome.surface, output, time, None, |_, _| {
            Some(output.clone())
        });
        sent += pending as u64;
    }
    sent
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
    // Chrome surfaces are paced by the same presented frame (T-21.1c).
    state.stats.chrome_frame_callbacks += send_chrome_frame_callbacks(state, output, time);
    // This is a presented frame (headless's stand-in for the photon): credit
    // any pending input with its input-to-photon round trip (T-03.1b).
    state.stats.latency.note_present(Instant::now());
    crate::trace::log("present", "headless");
}

/// Send frame callbacks for the live windows of `output` without presenting a
/// new frame.
///
/// The compositor is damage-driven and normally sends a window's `frame`
/// callback from [`post_repaint`] after it presents. A client that has a
/// pending update and is waiting on that callback (Qt Wayland, for one) can
/// otherwise stall for as long as the compositor stays idle: the compositor
/// has nothing to present, so it sends no callback, so the client never
/// commits, so the compositor has nothing to present. Waking the clients on an
/// input-driven frame (see the nested backend's no-damage branch) breaks that
/// cycle without changing the idle budget: an idle compositor is not woken, so
/// it still sends nothing.
pub fn send_frame_callbacks(
    state: &mut DfState,
    output: &Output,
    time: Duration,
    _states: &RenderElementStates,
) {
    let mut wakeups = 0u64;
    for window in state.space.elements() {
        if state.space.outputs_for_element(window).contains(output) {
            window.send_frame(
                output,
                time,
                Some(Duration::ZERO),
                surface_primary_scanout_output,
            );
            wakeups += 1;
        }
    }
    state.stats.client_wakeups += wakeups;
    // A no-damage frame still answers chrome callbacks, otherwise the shell
    // would wait for a presentation that the idle compositor never produces.
    state.stats.chrome_frame_callbacks += send_chrome_frame_callbacks(state, output, time);
}

/// Whether the compositor may advertise a fractional preferred scale to
/// `window`'s surfaces.
///
/// X11 windows (`window.x11_surface().is_some()`) are integer-scaled by
/// policy ([`docs/xwayland-scaling.md`], T-16.3a): Xwayland is a Wayland
/// client but represents X11 clients that cannot interpret fractional
/// metrics, so they only ever see the integer `wl_output.scale`. The
/// compositor downscales them itself in a later slice (T-16.3b).
pub fn accepts_fractional_scale(window: &smithay::desktop::Window) -> bool {
    window.x11_surface().is_none()
}

/// Advertise `output`'s fractional scale to the shell chrome surfaces
/// composited on it (T-16.3b).
///
/// Chrome surfaces are ordinary Wayland clients with a `wp_fractional_scale`
/// object, so a scale-aware shell can render its menu bar and Dock at the
/// output's real resolution instead of a 1x buffer the compositor then
/// stretches (the "blurry chrome" failure). The compositor never advertises a
/// fractional scale to Xwayland (`accepts_fractional_scale`); chrome is a
/// first-party Wayland client and may use it.
pub fn update_chrome_preferred_scale(state: &DfState, output: &Output) {
    let Some(geometry) = state.space.output_geometry(output) else {
        return;
    };
    let scale = output.current_scale().fractional_scale();
    for chrome in state.chrome_surfaces(output.name().as_str(), geometry) {
        smithay::wayland::compositor::with_states(&chrome.surface, |states| {
            with_fractional_scale(states, |fractional_scale| {
                fractional_scale.set_preferred_scale(scale);
            });
        });
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

    // Chrome surfaces on this output learn the output's fractional scale, so
    // the shell can size its buffers per output (T-16.3b).
    update_chrome_preferred_scale(state, output);

    let mut wakeups = 0u64;
    for window in state.space.elements() {
        // Integer-scaled Xwayland (T-16.3a): X11 clients never receive a
        // fractional preferred scale. They render at the integer
        // `wl_output.scale` the output advertises; T-16.3b adds the
        // compositor-side viewport downscale. Wayland windows keep the
        // fractional preferred scale.
        if accepts_fractional_scale(window) {
            window.with_surfaces(|surface, surface_states| {
                let primary_scanout_output =
                    surface_primary_scanout_output(surface, surface_states);
                if let Some(output) = primary_scanout_output.as_ref() {
                    with_fractional_scale(surface_states, |fractional_scale| {
                        fractional_scale
                            .set_preferred_scale(output.current_scale().fractional_scale());
                    });
                }
            });
        }

        if state.space.outputs_for_element(window).contains(output) {
            window.send_frame(output, time, throttle, surface_primary_scanout_output);
            wakeups += 1;
        }
    }
    // One frame-callback batch per live window: the idle budget's direct
    // "client wakeups" counter (T-03.1a).
    state.stats.client_wakeups += wakeups;
    // Chrome surfaces are paced by this presented frame (T-21.1c). No 1 s
    // throttle: a visible chrome surface is the shell's animation clock.
    state.stats.chrome_frame_callbacks += send_chrome_frame_callbacks(state, output, time);
    // This frame was presented (nested `submit`): credit any pending input
    // with its input-to-photon round trip (T-03.1b).
    state.stats.latency.note_present(Instant::now());
}
