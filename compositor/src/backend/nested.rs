// SPDX-License-Identifier: MIT
//! Wayland nested backend (T-02): runs as a window on an existing
//! Wayland session — the daily development workflow.
//!
//! Rendering goes through Smithay's glow (GL/EGL) renderer with a damage
//! tracker: idle produces no frames (FR-2), and every redraw request
//! produces exactly one render pass (FR-4). The winit event loop is a
//! calloop source in smithay 0.7, so input, resize, and redraw wake the
//! shared session loop fd-driven — there is no polling thread.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use smithay::backend::allocator::Fourcc;
use smithay::utils::{Buffer as BufferCoord, Physical, Point, Size, Size as BufferSize};

use smithay::backend::egl::EGLDevice;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::utils::{
    Relocate, RelocateRenderElement, RescaleRenderElement,
};
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexProgram, GlesTexture};
use smithay::backend::renderer::glow::GlowRenderer;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::{ImportAll, ImportDma, ImportMem, ImportMemWl, Renderer, Texture};
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::output::{Mode, PhysicalProperties, Subpixel};
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use smithay::render_elements;
use smithay::utils::Rectangle;
use smithay::wayland::presentation::Refresh;

use crate::backend::{add_output, add_seat_capabilities};
use crate::gpu_timer::{GpuPass, GpuTimer};
use crate::session::{run_session, BackendHooks};
use crate::window::{
    full_output_material_forced, MaterialRegion, PanelBlurKey, PanelKey, PanelTextureSlots,
    SceneTextureCache, SceneTextureKey, TextureRenderer,
};
use crate::{render, LOCKSTEP_VERSION};

// Custom elements for the nested output: shell chrome (menu bar, overlays)
// and the compositor-drawn SSD titlebars (T-01.1). One enum so both kinds
// can composite above the window space in a single render pass.
render_elements! {
    pub NestedOutputElements<R> where R: ImportAll + ImportMem + ImportMemWl + TextureRenderer;
    Chrome=WaylandSurfaceRenderElement<R>,
    Decoration=SolidColorRenderElement,
    // A window mid-appear: scaled about its target and translated from the
    // Dock tile origin (T-02.1b).
    Appear=RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>,
    // The per-Space wallpaper (T-05.4): behind the windows.
    Wallpaper=crate::render::WallpaperRenderElement<R>,
    // The GPU-sampled backdrop blur (T-20.1): a Kawase-downsampled scene
    // region drawn under a chrome panel, clipped to its rounded corners.
    Blur=crate::window::BackdropBlurElement,
}

// The frame element list after the magnifier pass (T-16.6b): the untransformed
// elements (`Plain`) or each element scaled/translated about the output centre
// (`Magnified`). One enum so the live frame and the offscreen capture render
// the same list.
render_elements! {
    pub NestedFrameElements<R> where R: ImportAll + ImportMem + ImportMemWl + TextureRenderer;
    Plain=NestedOutputElements<R>,
    Magnified=RelocateRenderElement<RescaleRenderElement<NestedOutputElements<R>>>,
}

/// Apply the compositor magnifier (T-16.6b) to an output's element list. When
/// the magnifier is inactive the elements are returned untouched, so an
/// unmagnified frame is byte-identical to the pre-magnifier path.
fn magnify_elements(
    state: &crate::state::DfState,
    output: &smithay::output::Output,
    scale: smithay::utils::Scale<f64>,
    elements: Vec<NestedOutputElements<GlowRenderer>>,
) -> Vec<NestedFrameElements<GlowRenderer>> {
    let transform = state
        .space
        .output_geometry(output)
        .and_then(|geometry| state.magnifier().view_transform(geometry, scale.x));
    match transform {
        Some(transform) if !transform.is_identity() => {
            let factor = smithay::utils::Scale::from((transform.scale, transform.scale));
            elements
                .into_iter()
                .map(|element| {
                    let scaled =
                        RescaleRenderElement::from_element(element, transform.origin, factor);
                    let relocated = RelocateRenderElement::from_element(
                        scaled,
                        transform.translation,
                        Relocate::Relative,
                    );
                    NestedFrameElements::Magnified(relocated)
                })
                .collect()
        }
        _ => elements
            .into_iter()
            .map(NestedFrameElements::Plain)
            .collect(),
    }
}

pub fn run(socket_name: &str) -> Result<(), String> {
    println!("dragonfruit-compositor: starting (backend=nested, lockstep-ipc=v{LOCKSTEP_VERSION})");

    // T-03 synthetic-input harness: opt-in, test-only plumbing, exactly as
    // the headless backend installs it. On nested it exists so a capture
    // script can drive the live walkthrough (Dock clicks, traffic lights,
    // titlebar menu) and screenshot the result — CI never sets this. The
    // socket path is a full path so the caller controls naming/cleanup.
    let synthetic_path = std::env::var_os(crate::input::synthetic::ENV_SYNTHETIC_INPUT)
        .map(std::path::PathBuf::from);
    let install_path = synthetic_path.clone();

    let (mut backend, winit_loop) = winit::init::<GlowRenderer>().map_err(|e| {
        format!("failed to open a nested window (is there a Wayland session?): {e}")
    })?;
    backend.window().set_title("Dragonfruit");

    // Filled by the init hook; read by the render hook.
    let shared: Rc<RefCell<Option<NestedData>>> = Rc::new(RefCell::new(None));

    let shared_init = shared.clone();
    let shared_render = shared.clone();

    let result = run_session(
        socket_name,
        BackendHooks {
            init: Box::new(move |state| {
                // The nested GlowRenderer implements the two-pass scene
                // texture + Kawase/glass path, so the effective material can
                // report blur as active. The DRM rail leaves this false until
                // P2 lands there.
                state.material_blur_supported = true;
                let window_size = backend.window_size();
                // T-16.3b: the nested output's scale. Fractional values let a
                // live session exercise chrome sizing and the per-surface
                // downscale (`DRAGONFRUIT_NESTED_SCALE`); the default is 1.0.
                let scale = crate::backend::parse_output_scale(
                    std::env::var(crate::backend::ENV_NESTED_SCALE)
                        .ok()
                        .as_deref(),
                    1.0,
                );
                add_output(
                    state,
                    "NESTED-1",
                    PhysicalProperties {
                        size: (0, 0).into(),
                        subpixel: Subpixel::Unknown,
                        make: "Dragonfruit".into(),
                        model: "Winit".into(),
                    },
                    Mode {
                        size: window_size,
                        refresh: 60_000,
                    },
                    (0, 0),
                    scale,
                );
                add_seat_capabilities(state);
                if let Some(path) = &install_path {
                    crate::input::synthetic::install(state, path)?;
                }

                // shm buffer formats we can import.
                state
                    .shm_state
                    .update_formats(backend.renderer().shm_formats());

                // linux-dmabuf with the EGL render node of the nested window.
                let render_node =
                    EGLDevice::device_for_display(backend.renderer().egl_context().display())
                        .and_then(|device| device.try_get_render_node())
                        .ok()
                        .flatten();
                if let Some(node) = render_node {
                    let formats = backend.renderer().dmabuf_formats();
                    state.init_dmabuf(formats.iter().copied(), Some(node.dev_id()));
                }

                // Input, resize, redraw: fd-driven through calloop.
                let shared_events = shared_init.clone();
                state
                    .loop_handle
                    .insert_source(winit_loop, move |event, _, state| match event {
                        WinitEvent::CloseRequested => {
                            println!("dragonfruit-compositor: nested window closed");
                            state.running = false;
                        }
                        WinitEvent::Resized { size, .. } => {
                            let mut current_scale = 1.0;
                            let output = state.space.outputs().next().cloned();
                            if let Some(output) = output {
                                let mode = Mode {
                                    size,
                                    refresh: 60_000,
                                };
                                output.change_current_state(Some(mode), None, None, None);
                                output.set_preferred(mode);
                                current_scale = output.current_scale().fractional_scale();
                                // A resize changes the output mode; keep the
                                // animation tick and degrade budget on the
                                // current fastest period (T-21.2a).
                                state.on_output_mode_changed(&output);
                            }
                            // The winit/EGL back buffer is bottom-up, so the
                            // damage tracker must render with Flipped180. The
                            // tracker's static mode also carries the output
                            // scale, so rebuild it on resize or a scale change
                            // (T-16.3b).
                            if let Some(data) = shared_events.borrow_mut().as_mut() {
                                data.ensure_tracker((size.w, size.h), current_scale);
                            }
                            state.needs_redraw = true;
                        }
                        WinitEvent::Redraw | WinitEvent::Focus(_) => {
                            state.needs_redraw = true;
                        }
                        WinitEvent::Input(event) => crate::input::process_input_event(state, event),
                    })
                    .map_err(|e| format!("failed to register winit source: {e}"))?;

                // Compile the backdrop blur program once, on the renderer that
                // owns the output (T-20.1). Failure is non-fatal: the blur
                // chain then uses the built-in bilinear downsample.
                let blur_program =
                    match std::borrow::BorrowMut::<GlesRenderer>::borrow_mut(backend.renderer())
                        .compile_custom_texture_shader(
                            crate::window::BLUR_SHADER,
                            &crate::window::blur_uniform_names(),
                        ) {
                        Ok(program) => {
                            crate::trace::log("material", "blur-program=compiled");
                            Some(program)
                        }
                        Err(err) => {
                            // Non-fatal: the blur chain falls back to the
                            // built-in bilinear downsample.
                            eprintln!(
                                "dragonfruit-compositor: backdrop blur shader unavailable \
                                 ({err}); using bilinear downsample"
                            );
                            None
                        }
                    };
                // Compile the liquid-glass compose program once, on the same
                // renderer (T-20.2). Failure is non-fatal: the panel keeps the
                // plain blurred compose (no lens/specular/tint).
                let glass_program =
                    match std::borrow::BorrowMut::<GlesRenderer>::borrow_mut(backend.renderer())
                        .compile_custom_texture_shader(
                            crate::window::GLASS_SHADER,
                            &crate::window::glass_uniform_names(),
                        ) {
                        Ok(program) => {
                            crate::trace::log("material", "glass-program=compiled");
                            Some(program)
                        }
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: liquid-glass shader unavailable \
                                 ({err}); using plain blurred compose"
                            );
                            None
                        }
                    };
                *shared_init.borrow_mut() = Some(NestedData {
                    blur_program,
                    glass_program,
                    material_textures: MaterialTextureCache::default(),
                    backend,
                    damage_tracker: OutputDamageTracker::new(
                        window_size,
                        smithay::utils::Scale::from(scale),
                        smithay::utils::Transform::Flipped180,
                    ),
                    tracker_scale: scale,
                    tracker_size: (window_size.w, window_size.h),
                    gpu_timer: GpuTimer::new(),
                });
                Ok(())
            }),
            render: Box::new(move |state| {
                let mut guard = shared_render.borrow_mut();
                match guard.as_mut() {
                    Some(data) => render_frame(state, data),
                    None => {
                        state.needs_redraw = false;
                        Ok(())
                    }
                }
            }),
        },
    );
    // The synthetic socket node is session state, not a leak; remove it even
    // when `run_session` returns an error.
    if let Some(path) = &synthetic_path {
        let _ = std::fs::remove_file(path);
    }
    result
}

/// Reusable GPU storage for the two-pass material pass.
///
/// The scene texture and every Kawase chain texture used to be allocated with
/// `create_buffer` on each rendered frame. At output resolution that is tens of
/// megabytes of GPU allocation per frame; over a long session it fragments the
/// driver and can end in an allocation failure (or an OOM kill). The textures
/// are reference-counted, so the render elements can keep a clone while the
/// cache reuses the backing storage on the next frame.
///
/// The scene texture is keyed by region size/output mode/scale (T-21.3), and
/// panel slots are trimmed when the panel set shrinks, so a removed popover
/// releases its `GlesTexture` handles instead of holding them for the rest of
/// the session.
#[derive(Default)]
struct MaterialTextureCache {
    scene: SceneTextureCache<GlesTexture>,
    /// One slot per chrome panel; each slot holds the textures for that
    /// panel's Kawase passes (the last one is the composed result).
    panels: PanelTextureSlots<GlesTexture>,
}

impl MaterialTextureCache {
    fn trim_panels(&mut self, keys: &[PanelKey]) {
        self.panels.trim_to(keys);
    }
}

struct NestedData {
    backend: WinitGraphicsBackend<GlowRenderer>,
    /// The compiled Kawase downsample program (T-20.1), or `None` when the
    /// driver refused it; the blur then falls back to a plain bilinear
    /// downsample chain, which still samples the live scene.
    blur_program: Option<GlesTexProgram>,
    /// The compiled Tahoe liquid-glass compose program (T-20.2), or `None`
    /// when the driver refused it; the panel then keeps the plain blurred
    /// compose with no lens, specular rim, or adaptive tint.
    glass_program: Option<GlesTexProgram>,
    /// Reused scene/blur textures for the two-pass material pass.
    material_textures: MaterialTextureCache,
    damage_tracker: OutputDamageTracker,
    /// The output scale the tracker was built with. The damage tracker's
    /// static mode supplies the scale every render element's geometry is
    /// resolved with, so it must track the output scale: at a fractional
    /// scale a stale `1.0` sizes chrome and window elements in logical pixels
    /// while placing them in physical pixels (T-16.3b).
    tracker_scale: f64,
    /// The physical mode size the tracker was built with.
    tracker_size: (i32, i32),
    /// Opt-in GPU timer queries for the benchmark (T-21.4b).
    gpu_timer: GpuTimer,
}

impl NestedData {
    /// Rebuild the damage tracker when its static output mode (physical size
    /// or scale) no longer matches the live output.
    fn ensure_tracker(&mut self, size: (i32, i32), scale: f64) {
        if self.tracker_size == size && (self.tracker_scale - scale).abs() <= f64::EPSILON {
            return;
        }
        // The winit/EGL back buffer is bottom-up, so the damage tracker must
        // render with Flipped180 (smithay's `minimal.rs` winit example does
        // the same). The output itself stays Normal so clients and input
        // mapping are unaffected.
        self.damage_tracker = OutputDamageTracker::new(
            size,
            smithay::utils::Scale::from(scale),
            smithay::utils::Transform::Flipped180,
        );
        self.tracker_size = size;
        self.tracker_scale = scale;
    }
}

fn render_frame(state: &mut crate::state::DfState, data: &mut NestedData) -> Result<(), String> {
    let Some(output) = state.space.outputs().next().cloned() else {
        state.needs_redraw = false;
        return Ok(());
    };
    // Per-output frame requests (T-21.2b): urgent work (input/commit/resize)
    // presents immediately; animation work waits for this output's own
    // presentation deadline. An output that is not requested is not rendered,
    // even while another output's animation ticks.
    let now = Duration::from(state.clock.now());
    if !state.output_should_render(&output.name(), now) {
        return Ok(());
    }
    state.needs_redraw = false;
    state.note_output_frame(&output.name());
    // Open this frame's material pass, so the chrome backdrop is applied at
    // most once per output (T-04.2).
    state.begin_render_frame();

    // The damage tracker's static mode carries the output's physical size and
    // scale; its scale is what every render element's geometry is resolved
    // with. Reconcile it before building the frame so a runtime scale change
    // (the Displays pane's `df_output.set_scale`) re-sizes chrome and window
    // elements into physical pixels instead of mixing logical and physical
    // pixels (T-16.3b).
    let mode_size = output
        .current_mode()
        .map(|mode| mode.size)
        .unwrap_or_default();
    data.ensure_tracker(
        (mode_size.w, mode_size.h),
        output.current_scale().fractional_scale(),
    );

    let age = if state.magnifier().is_active() {
        // The magnifier moves every element; a partial buffer from the
        // pre-magnifier frame would leave stale pixels, so the pass forces a
        // full damage frame while it is active (T-16.6b).
        0
    } else {
        data.backend.buffer_age().unwrap_or(0)
    };
    // T-13.3b: a capture requested by the trusted shell is produced from a
    // second, offscreen render pass of the same element list (so the live
    // frame is untouched) and resolved after this frame is submitted.
    let capture_requested = state.pending_capture.is_some();
    let capture_rect = state
        .pending_capture
        .as_ref()
        .and_then(|capture| capture.rect);
    let mut captured: Option<Result<(image::RgbaImage, String), String>> = None;
    let render_result = match data.backend.bind() {
        Ok((renderer, mut framebuffer)) => {
            let scale = smithay::utils::Scale::from(output.current_scale().fractional_scale());
            let output_geometry = state.space.output_geometry(&output).unwrap_or_default();
            // Collect any GPU timer results from earlier frames before
            // starting this frame's passes (T-21.4b). Polling never waits on
            // the GPU; unavailable/disabled timers publish their reason.
            for sample in data.gpu_timer.poll(renderer) {
                state.stats.gpu.record(sample.pass, sample.nanos);
            }
            let (gpu_enabled, gpu_available, gpu_reason) = data.gpu_timer.status();
            state
                .stats
                .gpu
                .set_status(gpu_enabled, gpu_available, gpu_reason);
            // GPU sampled backdrop blur (T-20.1): resolve the panels whose
            // material keeps blur on and take the material pass for this
            // output. Empty means "no GPU blur this frame" — the tier is
            // `Minimal`, no chrome is mapped, or another path already took the
            // pass — and the pre-T-20 feather path below applies instead.
            let blur_panels = crate::render::chrome_blur_panels(state, &output);
            // Direct-scanout policy (T-21.3): a fullscreen client that covers
            // the output with no visible material panel skips the sampled
            // pass. The persistent top chrome is already hidden by
            // `visible_chrome_surfaces`; an overlay (OSD/menu) keeps the
            // material. Nested records the candidate but never advances the
            // real `direct_scanouts` counter — only DRM can scan out.
            let output_local = Rectangle::from_size(output_geometry.size);
            let direct_candidate = crate::window::direct_scanout_candidate(
                output_local,
                state.fullscreen_client_rect(&output),
                &blur_panels,
            );
            if direct_candidate {
                state
                    .stats
                    .record_direct_scanout_candidate(output.name().as_str());
            }
            let mut custom_elements: Vec<NestedOutputElements<GlowRenderer>>;
            if !blur_panels.is_empty() {
                // Two-pass composition (T-20.1/T-21.3). Pass one renders the
                // scene (wallpaper, desktop, windows, shadows, titlebars) into
                // a texture sized to the padded panel union; pass two composes
                // the output from the live scene elements + the per-panel
                // blurred samples + the chrome surfaces. The blur samples the
                // live scene texture inside this render pass — never a
                // screenshot or screencopy.
                let output_local = Rectangle::from_size(output_geometry.size);
                let region = MaterialRegion::for_material(&blur_panels, output_local, scale)
                    .ok_or_else(|| "material region unavailable".to_owned())?;
                let full_output = full_output_material_forced();
                // The Kawase chain's first destination stays full-output
                // sized: its texture size is part of the panel's visual
                // result, and T-21.3 must not change the blur at the panel.
                let chain_size = output
                    .current_mode()
                    .map(|mode| mode.size)
                    .unwrap_or_default();
                let mut scene_elements: Vec<NestedOutputElements<GlowRenderer>> = Vec::new();
                scene_elements.extend(
                    crate::render::window_menu_render_elements(state, &output, scale)
                        .into_iter()
                        .map(NestedOutputElements::Decoration),
                );
                scene_elements.extend(crate::render::window_scene_render_elements(
                    renderer, state, &output, scale,
                ));
                scene_elements.extend(crate::render::desktop_render_elements(
                    renderer, state, &output, scale,
                ));
                scene_elements.extend(
                    crate::render::wallpaper_render_elements(renderer, state, &output, scale)
                        .into_iter()
                        .map(NestedOutputElements::Wallpaper),
                );
                data.gpu_timer.begin(renderer, GpuPass::Scene);
                let scene_texture_result = render_scene_offscreen(
                    renderer,
                    state,
                    &output,
                    scale,
                    region,
                    &scene_elements,
                    &mut data.material_textures.scene,
                );
                data.gpu_timer.end(renderer);
                let scene_texture = scene_texture_result?;
                let scene_size = scene_texture.size();
                // A locked session composites only the lock surface, prepended
                // so it sits above every window and chrome surface (T-12.3a).
                custom_elements =
                    crate::lock::lock_render_elements(renderer, state, &output, scale);
                // Chrome surfaces (menu bar, drawer, overlays) composite above
                // the blur.
                custom_elements.extend(crate::render::chrome_render_elements(
                    renderer, state, &output, scale,
                ));
                let scene_generation = data.material_textures.scene.generation();
                let scene_commit = data.material_textures.scene.commit();
                let (blur_elements, blur_allocations, blur_skips) = build_blur_elements(
                    renderer,
                    BlurPrograms {
                        blur: data.blur_program.as_ref(),
                        glass: data.glass_program.as_ref(),
                        chain_size,
                    },
                    &blur_panels,
                    BlurScene {
                        texture: &scene_texture,
                        generation: scene_generation,
                        commit: scene_commit,
                        region,
                    },
                    scale,
                    BlurTargets {
                        cache: &mut data.material_textures,
                        timer: &mut data.gpu_timer,
                    },
                )?;
                state.stats.record_material_panel_allocations_for(
                    output.name().as_str(),
                    blur_allocations,
                );
                state.stats.record_blur_skips(blur_skips);
                custom_elements.extend(blur_elements);
                // The blur elements exist and are in the draw list; this is
                // the counter to trust over `blur_passes` (which only says the
                // panels resolved).
                let downsample_max = blur_panels
                    .iter()
                    .map(|panel| panel.spec.downsample())
                    .max()
                    .unwrap_or(0);
                state
                    .stats
                    .record_blur_draw(downsample_max, blur_panels.len() as u64);
                state.stats.record_material_frame(output.name().as_str());
                if full_output {
                    // Diagnostic/rollback shape: the full-output scene texture
                    // is the output base, exactly like the pre-T-21.3 path.
                    // The scene cache owns the stable id and the commit that
                    // advances only when the offscreen scene changed (T-21.3).
                    let scene_id = data.material_textures.scene.id();
                    let scene_commit = data.material_textures.scene.commit();
                    custom_elements.push(NestedOutputElements::Blur(
                        crate::window::BackdropBlurElement::new(
                            scene_texture,
                            Rectangle::new((0, 0).into(), output_geometry.size),
                            scene_size,
                            0,
                            scale,
                        )
                        .with_id(scene_id)
                        .with_commit(scene_commit),
                    ));
                } else {
                    // Region shape: the scene elements are the output base and
                    // the region texture is only the blur source. The list is
                    // already front-to-back (window menu, windows, desktop,
                    // wallpaper), so appending it below the blur preserves the
                    // old stacking order.
                    custom_elements.extend(scene_elements);
                }
            } else {
                // No sampled material this frame: release the per-panel Kawase
                // textures (T-21.3 lifetime audit) and keep the pre-T-20
                // single-pass path: chrome + the feather backdrop (no GPU
                // readback). A locked session composites only the lock
                // surface, prepended so it sits above everything (T-12.3a).
                data.material_textures.trim_panels(&[]);
                custom_elements =
                    crate::lock::lock_render_elements(renderer, state, &output, scale);
                custom_elements.extend(crate::render::chrome_render_elements(
                    renderer, state, &output, scale,
                ));
                // Backdrop under the chrome (T-04.2): appended after the
                // chrome surfaces so it composites below them and in front of
                // the windows it stands in for.
                custom_elements.extend(
                    crate::render::chrome_backdrop_render_elements(state, &output, scale)
                        .into_iter()
                        .map(NestedOutputElements::Decoration),
                );
                custom_elements.extend(
                    crate::render::window_menu_render_elements(state, &output, scale)
                        .into_iter()
                        .map(NestedOutputElements::Decoration),
                );
                custom_elements.extend(crate::render::window_scene_render_elements(
                    renderer, state, &output, scale,
                ));
                custom_elements.extend(crate::render::desktop_render_elements(
                    renderer, state, &output, scale,
                ));
                custom_elements.extend(
                    crate::render::wallpaper_render_elements(renderer, state, &output, scale)
                        .into_iter()
                        .map(NestedOutputElements::Wallpaper),
                );
            }
            // The compositor magnifier (T-16.6b) scales and pans the whole
            // scene about the output centre. Inactive, this is the identity;
            // active, every element is rescale+relocate wrapped. The live
            // frame and the offscreen capture render the same wrapped list.
            let frame_elements = magnify_elements(state, &output, scale, custom_elements);
            // T-13.3b: produce a requested capture first, into an offscreen
            // texture. Doing it before the window render lets the render pass
            // below restore the winit EGL surface (the offscreen bind makes
            // the context current without a surface), so the submit after
            // this frame still has a valid draw surface.
            if capture_requested {
                captured = Some(capture_frame(
                    renderer,
                    &frame_elements,
                    state,
                    &output,
                    scale,
                    capture_rect,
                ));
            }
            data.gpu_timer.begin(renderer, GpuPass::Composite);
            let render_result = data.damage_tracker.render_output(
                renderer,
                &mut framebuffer,
                age,
                &frame_elements,
                state.wallpaper_color_for(&output),
            );
            data.gpu_timer.end(renderer);
            for sample in data.gpu_timer.poll(renderer) {
                state.stats.gpu.record(sample.pass, sample.nanos);
            }
            render_result
        }
        Err(err) => return Err(format!("nested bind failed: {err}")),
    };

    let render_result = match render_result {
        Ok(result) => result,
        Err(err) => {
            // Answer a pending capture even though the live frame failed, so
            // the shell's portal presenter never hangs.
            if let Some(result) = captured.take() {
                resolve_capture(state, result);
            }
            return Err(format!("nested render failed: {err:?}"));
        }
    };

    match render_result.damage {
        Some(damage) => {
            // Partial-submit with the tracked damage region.
            if let Err(err) = data.backend.submit(Some(damage)) {
                if let Some(result) = captured.take() {
                    resolve_capture(state, result);
                }
                return Err(format!("nested submit failed: {err}"));
            }
            state.stats.frames_rendered += 1;

            // T-21.2b presentation timestamp source: Smithay's winit
            // `submit()` swaps the buffer and returns only
            // `Result<(), SwapBuffersError>`; it exposes no platform vblank
            // timestamp. The completion of `submit()` is therefore the
            // closest presentation time the nested rail has. Capture it
            // immediately after the successful swap, before frame-callback
            // work, and feed it to the per-output presentation clock. Only a
            // damage frame is a presentation: the no-damage branch below
            // swaps without new pixels and must not create a clock sample.
            // The fixed mode period remains the deadline source; T188b/c must
            // not infer VRR from this interval.
            let now = state.clock.now();
            state.note_presentation(&output.name(), now.into());
            let frame_duration = output
                .current_mode()
                .map(|mode| Duration::from_secs_f64(1_000f64 / mode.refresh as f64))
                .unwrap_or(Duration::from_secs_f64(1.0 / 60.0));
            let mut feedback =
                render::take_presentation_feedback(state, &output, &render_result.states);
            feedback.presented(
                now,
                Refresh::fixed(frame_duration),
                0,
                wp_presentation_feedback::Kind::Vsync,
            );
            render::post_repaint(state, &output, now.into(), &render_result.states);
            crate::trace::log("present", "nested");
        }
        None => {
            // No damage, no render, no client wakeups (FR-2) — except that a
            // woken compositor must still reply to a client that is waiting on
            // its frame callback (an input-driven frame usually has no damage
            // yet; the client's repaint is what produces it). An idle
            // compositor is never woken, so this adds nothing to the idle
            // budget.
            state.stats.frames_skipped_no_damage += 1;
            if let Err(err) = data.backend.submit(None) {
                if let Some(result) = captured.take() {
                    resolve_capture(state, result);
                }
                return Err(format!("nested submit failed: {err}"));
            }
            let now = state.clock.now();
            render::send_frame_callbacks(state, &output, now.into(), &render_result.states);
            crate::trace::log("wake-clients", "nested");
        }
    }
    if let Some(result) = captured {
        resolve_capture(state, result);
    }
    Ok(())
}

/// Render the scene `elements` into the material region's offscreen texture
/// once per frame (T-20.1/T-21.3). This is the texture the chrome backdrop
/// blur samples; it holds the live wallpaper/desktop/window/shadow pixels,
/// never a screenshot.
///
/// The elements are output-local physical, so each one is relocated by
/// `-region.physical.loc` before the tracker sees it. The texture itself stays
/// in the region's coordinate space; panel sources are translated by
/// [`MaterialRegion::panel_source`].
///
/// Synchronization: the DRM material path is single-GPU (multi-GPU sampled
/// material is disabled in `drm.rs`), so the scene texture and the Kawase
/// chain share one GL context. `render_output` finishes its frame before it
/// returns, and the following blur bind is ordered after it by GL. The nested
/// `GlesFrame` additionally waits on its fence when dropped. No cross-GPU
/// copy or fence hand-off is needed on this path.
pub(crate) fn render_scene_offscreen<R, E>(
    renderer: &mut R,
    state: &mut crate::state::DfState,
    output: &smithay::output::Output,
    scale: smithay::utils::Scale<f64>,
    region: MaterialRegion,
    elements: &[E],
    cache: &mut SceneTextureCache<GlesTexture>,
) -> Result<GlesTexture, String>
where
    R: Renderer
        + smithay::backend::renderer::Bind<GlesTexture>
        + smithay::backend::renderer::Offscreen<GlesTexture>,
    R::TextureId: Texture,
    R::Error: std::fmt::Debug,
    E: smithay::backend::renderer::element::RenderElement<R>,
{
    use smithay::utils::Transform;

    let output_size = output
        .current_mode()
        .map(|mode| mode.size)
        .ok_or_else(|| "output has no mode".to_owned())?;
    let buffer_size =
        BufferSize::<i32, BufferCoord>::from((region.texture_size.w, region.texture_size.h));
    let key = SceneTextureKey::new(buffer_size, output_size, scale);
    if cache.needs_alloc(key) {
        let texture = renderer
            .create_buffer(Fourcc::Abgr8888, buffer_size)
            .map_err(|err| format!("scene buffer failed: {err}"))?;
        cache.install(texture, key);
        state
            .stats
            .record_material_scene_allocation_for(output.name().as_str());
    }
    state
        .stats
        .record_material_region_for(output.name().as_str(), region.texture_size);
    let mut texture = cache
        .texture()
        .cloned()
        .ok_or_else(|| "scene texture cache unexpectedly empty".to_owned())?;
    let clear = state.wallpaper_color_for(output);
    let tracker_size = Size::<i32, Physical>::from((region.texture_size.w, region.texture_size.h));
    // Keep the scene damage tracker across frames (T-21.3). The first frame
    // (or the first frame after a reallocation/region change) renders the
    // whole region; later frames render only the damage the live elements
    // report, and an unchanged scene renders nothing at all. Age 1 keeps
    // Smithay from re-damaging the whole output on every call while still
    // retaining the last element states.
    if !cache.tracker_matches(key) {
        cache.set_tracker(
            key,
            OutputDamageTracker::new(tracker_size, scale, Transform::Normal),
        );
    }
    // Relocate the output-local elements into the region's physical space.
    // `&E` implements `RenderElement`, so this borrows the caller's list
    // instead of consuming it; the same list is drawn live in region mode.
    let offset = Point::<i32, Physical>::from((-region.physical.loc.x, -region.physical.loc.y));
    let relocated: Vec<_> = elements
        .iter()
        .map(|element| RelocateRenderElement::from_element(element, offset, Relocate::Relative))
        .collect();
    let changed = {
        let tracker = cache
            .tracker_mut()
            .ok_or_else(|| "scene damage tracker missing".to_owned())?;
        let mut target = renderer
            .bind(&mut texture)
            .map_err(|err| format!("scene bind failed: {err}"))?;
        let result = tracker
            .render_output(renderer, &mut target, 1, &relocated, clear)
            .map_err(|err| format!("scene render failed: {err:?}"))?;
        result.damage.is_some()
    };
    cache.note_scene_change(changed);
    Ok(texture)
}

/// The compiled programs and chain base for one blur build.
struct BlurPrograms<'a> {
    blur: Option<&'a GlesTexProgram>,
    glass: Option<&'a GlesTexProgram>,
    /// The full output's physical size: the Kawase chain keeps that as its
    /// first destination so the panel's visual blur is unchanged from the
    /// pre-region path.
    chain_size: Size<i32, Physical>,
}

/// The caches and timers the blur build owns. Bundled so the function keeps a
/// readable argument list.
struct BlurTargets<'a> {
    cache: &'a mut MaterialTextureCache,
    timer: &'a mut GpuTimer,
}

/// The scene texture and identity one blur build samples.
struct BlurScene<'a> {
    texture: &'a GlesTexture,
    generation: u64,
    commit: CommitCounter,
    region: MaterialRegion,
}

/// Build the per-panel blurred backdrop elements from the scene texture
/// (T-20.1/T-21.3). Each panel's region is downsampled through
/// `spec.iterations` Kawase passes — the custom [`crate::window::BLUR_SHADER`]
/// when the renderer compiled it, otherwise the built-in bilinear downsample —
/// and drawn back over the panel clipped to its token corners.
///
/// `region` translates each panel's output-coordinate source into the scene
/// texture.
fn build_blur_elements(
    renderer: &mut GlowRenderer,
    programs: BlurPrograms<'_>,
    panels: &[crate::render::BlurPanel],
    scene: BlurScene<'_>,
    scale: smithay::utils::Scale<f64>,
    targets: BlurTargets<'_>,
) -> Result<(Vec<NestedOutputElements<GlowRenderer>>, u64, u64), String> {
    let mut elements = Vec::new();
    let mut allocations = 0u64;
    let mut skips = 0u64;
    let keys: Vec<PanelKey> = panels.iter().map(|panel| panel.key).collect();
    for panel in panels {
        let Some((texture_panel, scene_src)) = scene.region.panel_source(panel.panel, scale) else {
            continue;
        };
        let blur_key = PanelBlurKey::new(
            scene.generation,
            scene.commit,
            scene_src,
            panel.spec,
            programs.chain_size,
            scale,
        );
        let slot = targets.cache.panels.slot(&panel.key);
        let id = slot.id().clone();
        let texture = if slot.blur_cache_hit(&blur_key) {
            skips += 1;
            slot.last_texture()
                .cloned()
                .ok_or_else(|| "blur cache hit without a texture".to_owned())?
        } else {
            // Clear the key before touching the chain: a failed render must
            // not let the next frame reuse a partially written texture.
            slot.invalidate_blur_cache();
            targets.timer.begin(renderer, GpuPass::Blur);
            let texture_result = blur_panel_texture(
                renderer,
                programs.blur,
                scene.texture,
                scene_src,
                &panel.spec,
                programs.chain_size,
                slot.textures_mut(),
            );
            targets.timer.end(renderer);
            let (texture, panel_allocations) = texture_result?;
            allocations += panel_allocations;
            slot.set_blur_key(blur_key);
            texture
        };
        let commit = slot.commit();
        let texture_size = texture.size();
        let element = crate::window::BackdropBlurElement::new(
            texture,
            panel.panel,
            texture_size,
            panel.spec.corner_radius.round().max(0.0) as i32,
            scale,
        )
        .with_id(id)
        .with_commit(commit)
        .with_texture_panel(texture_panel);
        // Add the liquid-glass compose only when the driver compiled it; a
        // refused shader leaves the plain blurred backdrop (T-20.2).
        let element = match programs.glass {
            Some(glass) => element.with_glass(panel.glass, glass.clone()),
            None => element,
        };
        elements.push(NestedOutputElements::Blur(element));
    }
    // Release the slots of panels that disappeared this frame. A panel that
    // merely moved or changed stacking order keeps its key, id, and textures.
    targets.cache.trim_panels(&keys);
    Ok((elements, allocations, skips))
}

/// Downsample `scene_src` of `scene` through a small Kawase chain, returning
/// the blurred texture (T-20.1). Each pass halves the resolution and samples
/// the previous texture with the custom program when available.
///
/// `scene_src` is in the scene texture's buffer coordinates (the caller has
/// already subtracted the material region origin). `chain_size` is the full
/// output's physical size; the first destination stays output-sized so the
/// panel's visual blur is pixel-equivalent to the full-output path.
pub(crate) fn blur_panel_texture<R>(
    renderer: &mut R,
    program: Option<&GlesTexProgram>,
    scene: &GlesTexture,
    scene_src: Rectangle<f64, BufferCoord>,
    spec: &crate::window::BlurSpec,
    chain_size: Size<i32, Physical>,
    slot: &mut Vec<GlesTexture>,
) -> Result<(GlesTexture, u64), String>
where
    R: TextureRenderer
        + smithay::backend::renderer::Bind<GlesTexture>
        + smithay::backend::renderer::Offscreen<GlesTexture>,
    R::Error: std::fmt::Debug,
{
    use smithay::backend::renderer::gles::Uniform;
    use smithay::utils::Transform;

    let mut current: GlesTexture = scene.clone();
    let mut allocations = 0u64;
    // The region of `current` the next pass samples. The first pass samples
    // the panel's slice of the scene; later passes sample the whole texture.
    let mut current_src = scene_src;
    let mut chain = chain_size;
    for pass in 0..spec.iterations as usize {
        let src_size = current.size();
        // The chain base is the output size, not the scene texture size: a
        // region-sized scene texture must not change the panel's blur result.
        let dst_w = (chain.w / 2).max(1);
        let dst_h = (chain.h / 2).max(1);
        let buffer_size = BufferSize::<i32, BufferCoord>::from((dst_w, dst_h));
        // Reuse this pass's texture from the previous frame; only allocate
        // when the panel (or output scale) changed size. The slot keeps the
        // texture alive across frames; a removed panel is trimmed by the
        // caller, which releases the handle.
        while slot.len() <= pass {
            slot.push(
                renderer
                    .create_buffer(Fourcc::Abgr8888, buffer_size)
                    .map_err(|err| format!("blur buffer failed: {err}"))?,
            );
            allocations += 1;
        }
        if slot[pass].size() != buffer_size {
            slot[pass] = renderer
                .create_buffer(Fourcc::Abgr8888, buffer_size)
                .map_err(|err| format!("blur buffer failed: {err}"))?;
            allocations += 1;
        }
        let mut texture: GlesTexture = slot[pass].clone();
        let dst_phys = Rectangle::from_size(Size::<i32, Physical>::from((dst_w, dst_h)));
        {
            let mut target = renderer
                .bind(&mut texture)
                .map_err(|err| format!("blur bind failed: {err}"))?;
            let mut frame = renderer
                .render(&mut target, dst_phys.size, Transform::Normal)
                .map_err(|err| format!("blur render failed: {err}"))?;
            let uniforms = [
                Uniform::new(
                    crate::window::BLUR_TEXEL_UNIFORM,
                    (1.0f32 / src_size.w as f32, 1.0f32 / src_size.h as f32),
                ),
                Uniform::new(crate::window::BLUR_RADIUS_UNIFORM, 1.0f32),
            ];
            R::draw_texture_from_to(
                &mut frame,
                &current,
                current_src,
                dst_phys,
                &[dst_phys],
                program,
                &uniforms,
            )
            .map_err(|err| format!("blur draw failed: {err:?}"))?;
        }
        current = texture;
        current_src = Rectangle::from_size(current.size().to_f64());
        chain = Size::<i32, Physical>::from((dst_w, dst_h));
    }
    Ok((current, allocations))
}

/// Render `elements` (the same list the live frame drew) into an offscreen
/// RGBA texture and read it back, cropped to `rect` (or the whole output).
/// Returns the image and the path-independent PNG payload; `render_frame`
/// writes it and replies to the shell.
fn capture_frame<E>(
    renderer: &mut GlowRenderer,
    elements: &[E],
    state: &crate::state::DfState,
    output: &smithay::output::Output,
    scale: smithay::utils::Scale<f64>,
    rect: Option<smithay::utils::Rectangle<i32, smithay::utils::Physical>>,
) -> Result<(image::RgbaImage, String), String>
where
    E: smithay::backend::renderer::element::RenderElement<GlowRenderer>,
{
    use smithay::backend::allocator::Fourcc;
    use smithay::backend::renderer::gles::GlesTexture;
    use smithay::backend::renderer::{Bind, ExportMem, Offscreen};
    use smithay::utils::{Buffer as BufferCoord, Transform};

    let size = output
        .current_mode()
        .map(|mode| mode.size)
        .ok_or_else(|| "output has no mode".to_owned())?;
    let buffer_size = smithay::utils::Size::<i32, BufferCoord>::from((size.w, size.h));
    let mut texture: GlesTexture = renderer
        .create_buffer(Fourcc::Abgr8888, buffer_size)
        .map_err(|err| format!("offscreen buffer failed: {err}"))?;
    let clear = state.wallpaper_color_for(output);
    let mut tracker = smithay::backend::renderer::damage::OutputDamageTracker::new(
        size,
        scale,
        Transform::Normal,
    );
    let mut target = renderer
        .bind(&mut texture)
        .map_err(|err| format!("offscreen bind failed: {err}"))?;
    tracker
        .render_output(renderer, &mut target, 0, elements, clear)
        .map_err(|err| format!("capture render failed: {err:?}"))?;
    let mapping = renderer
        .copy_framebuffer(
            &target,
            smithay::utils::Rectangle::from_size(buffer_size),
            Fourcc::Abgr8888,
        )
        .map_err(|err| format!("capture read failed: {err}"))?;
    let data = renderer
        .map_texture(&mapping)
        .map_err(|err| format!("capture map failed: {err}"))?
        .to_vec();
    let full = image::RgbaImage::from_raw(size.w as u32, size.h as u32, data)
        .ok_or_else(|| "capture dimensions invalid".to_owned())?;
    let image = match rect {
        None => full,
        Some(rect) => {
            // A capture rectangle can arrive at (or past) the exclusive
            // right/bottom edge (a drag selection, a window against the
            // screen edge). Clamp to the last drawable pixel before computing
            // the crop: `clamp(1, 0)` would panic and take the compositor
            // down on an otherwise valid screenshot request.
            let max_x = (size.w - 1).max(0);
            let max_y = (size.h - 1).max(0);
            let x = rect.loc.x.clamp(0, max_x);
            let y = rect.loc.y.clamp(0, max_y);
            let width = rect.size.w.clamp(1, (size.w - x).max(1)) as u32;
            let height = rect.size.h.clamp(1, (size.h - y).max(1)) as u32;
            image::imageops::crop_imm(&full, x as u32, y as u32, width, height).to_image()
        }
    };
    Ok((image, output.name()))
}

/// Write a produced capture to the path the shell supplied and reply, or
/// reply with the failure reason (T-13.3b).
fn resolve_capture(
    state: &mut crate::state::DfState,
    result: Result<(image::RgbaImage, String), String>,
) {
    let Some(pending) = state.take_capture() else {
        return;
    };
    match result {
        Ok((image, _output)) => {
            match image.save_with_format(&pending.path, image::ImageFormat::Png) {
                Ok(()) => {
                    let path = pending.path.to_string_lossy().into_owned();
                    state.broadcast_screenshot_saved(&path);
                }
                Err(err) => {
                    state.broadcast_screenshot_failed(&format!("write failed: {err}"));
                }
            }
        }
        Err(reason) => state.broadcast_screenshot_failed(&reason),
    }
}
