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
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::time::{Duration, Instant};

use smithay::backend::allocator::Fourcc;
use smithay::utils::{Buffer as BufferCoord, Physical, Point, Size, Size as BufferSize};

use smithay::backend::egl::{ffi, EGLDevice};
use smithay::backend::renderer::damage::{OutputDamageTracker, RenderOutputResult};
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::utils::{
    Relocate, RelocateRenderElement, RescaleRenderElement,
};
use smithay::backend::renderer::element::Element;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexProgram, GlesTexture};
use smithay::backend::renderer::glow::GlowRenderer;
use smithay::backend::renderer::{ImportAll, ImportDma, ImportMem, ImportMemWl, Renderer, Texture};
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::output::{Mode, PhysicalProperties, Subpixel};
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use smithay::render_elements;
use smithay::utils::Rectangle;
use smithay::wayland::presentation::Refresh;

use crate::backend::present::{DamageMode, PresentDone, PresentError, PresentRequest, Presenter};
use crate::backend::{add_output, add_seat_capabilities};
use crate::gpu_timer::{GpuPass, GpuTimer};
use crate::session::{run_session, BackendHooks};
use crate::state::PresentationProvenance;
use crate::window::{
    full_output_material_forced, BlurMissCounts, BlurMissReason, MaterialRegion, PanelKey,
    PanelPaintKey, PanelTextureSlots, RegionFilter, SceneTextureCache, SceneTextureKey,
    ShadowResources, TextureRenderer,
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
    // One-quad shader shadow (T238 decoration optimisation).
    Shadow=crate::window::ShadowElement,
    // One-quad SDF titlebar fill / traffic light (T238 decoration
    // optimisation).
    Titlebar=crate::window::TitlebarShapeElement,
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

/// The nested window attributes. The default matches Smithay's 1280x800;
/// `DRAGONFRUIT_NESTED_SIZE=WxH` is capture-only plumbing so the 1x/1.5x/2x
/// fixtures can request the physical output their scale maps to.
fn nested_window_attributes() -> smithay::reexports::winit::window::WindowAttributes {
    use smithay::reexports::winit::dpi::LogicalSize;
    use smithay::reexports::winit::window::Window as WinitWindow;
    let (width, height) = std::env::var("DRAGONFRUIT_NESTED_SIZE")
        .ok()
        .and_then(|value| {
            let (w, h) = value.split_once('x')?;
            Some((w.trim().parse::<f64>().ok()?, h.trim().parse::<f64>().ok()?))
        })
        .filter(|(w, h)| *w > 0.0 && *h > 0.0)
        .unwrap_or((1280.0, 800.0));
    WinitWindow::default_attributes()
        .with_inner_size(LogicalSize::new(width, height))
        .with_title("Dragonfruit")
        .with_visible(true)
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

    let (mut backend, winit_loop) =
        winit::init_from_attributes::<GlowRenderer>(nested_window_attributes()).map_err(|e| {
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
                // The nested rail's only presentation signal is the host
                // frame callback / swap completion. Physical vblank feedback
                // is not reachable through winit's Rust Wayland backend.
                state.stats.presentation_provenance = PresentationProvenance::NestedSwapCompletion;
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
                // Compile the measured separable Gaussian once, on the same
                // renderer. Failure is non-fatal: the Dock falls back to the
                // sharp panel slice (no frost) rather than a broken blur.
                let gaussian_program =
                    match std::borrow::BorrowMut::<GlesRenderer>::borrow_mut(backend.renderer())
                        .compile_custom_texture_shader(
                            crate::window::GAUSSIAN_BLUR_SHADER,
                            &crate::window::gaussian_uniform_names(),
                        ) {
                        Ok(program) => {
                            crate::trace::log("material", "gaussian-program=compiled");
                            Some(program)
                        }
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: measured Gaussian shader unavailable \
                                 ({err}); Dock frost disabled"
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
                // The sharp-scene bevel pass for the measured Dock. Failure
                // is non-fatal: the blurred body keeps its own edge treatment.
                let bevel_program =
                    match std::borrow::BorrowMut::<GlesRenderer>::borrow_mut(backend.renderer())
                        .compile_custom_texture_shader(
                            crate::window::BEVEL_SHADER,
                            &crate::window::glass_uniform_names(),
                        ) {
                        Ok(program) => {
                            crate::trace::log("material", "bevel-program=compiled");
                            Some(program)
                        }
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: Dock bevel shader unavailable \
                                 ({err}); using the blurred edge only"
                            );
                            None
                        }
                    };
                // T200: the menu-bar no-blur veil. Failure is non-fatal: the
                // sharp path falls back to the glass flat-band compose (a
                // scheme-appropriate plate) instead of the raw scene.
                let veil_program =
                    match std::borrow::BorrowMut::<GlesRenderer>::borrow_mut(backend.renderer())
                        .compile_custom_texture_shader(
                            crate::window::VEIL_SHADER,
                            &crate::window::veil_uniform_names(),
                        ) {
                        Ok(program) => {
                            crate::trace::log("material", "veil-program=compiled");
                            Some(program)
                        }
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: menu-bar veil shader unavailable \
                                 ({err}); using the glass flat-band fallback"
                            );
                            None
                        }
                    };
                // T238 decoration optimisation: compile the one-quad shadow
                // program and a 1x1 white probe texture on the same renderer.
                // Failure is non-fatal: the span decomposition remains.
                let shadow = {
                    let gles: &mut GlesRenderer =
                        std::borrow::BorrowMut::borrow_mut(backend.renderer());
                    match gles
                        .compile_custom_texture_shader(
                            crate::window::SHADOW_SHADER,
                            &crate::window::shadow_uniform_names(),
                        )
                        .and_then(|program| {
                            gles.import_memory(
                                &[255u8, 255, 255, 255],
                                Fourcc::Abgr8888,
                                Size::from((1, 1)),
                                false,
                            )
                            .map(|texture| ShadowResources { program, texture })
                        }) {
                        Ok(resources) => Some(resources),
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: shadow shader unavailable ({err}); \
                                 using span shadows"
                            );
                            None
                        }
                    }
                };
                if shadow.is_some() {
                    crate::trace::log("material", "shadow-program=compiled");
                }
                let titlebar = shadow.as_ref().and_then(|resources| {
                    let gles: &mut GlesRenderer =
                        std::borrow::BorrowMut::borrow_mut(backend.renderer());
                    match gles.compile_custom_texture_shader(
                        crate::window::TITLEBAR_SHADER,
                        &crate::window::titlebar_uniform_names(),
                    ) {
                        Ok(program) => Some(crate::window::TitlebarResources {
                            program,
                            texture: resources.texture.clone(),
                        }),
                        Err(err) => {
                            eprintln!(
                                "dragonfruit-compositor: titlebar shader unavailable ({err}); \
                                 using span traffic lights"
                            );
                            None
                        }
                    }
                });
                if titlebar.is_some() {
                    crate::trace::log("material", "titlebar-program=compiled");
                }
                // T238 P0: move the host swap off the session loop. The EGL
                // context is released before each handoff and made current
                // again on the next bind; the presenter owns only the raw
                // display/context handles and the per-frame surface.
                let (ping, ping_source) = calloop::ping::make_ping()
                    .map_err(|error| format!("failed to create present wakeup: {error}"))?;
                state
                    .loop_handle
                    .insert_source(ping_source, |_, _, _| ())
                    .map_err(|error| format!("failed to register present wakeup: {error}"))?;
                let context = backend.renderer().egl_context();
                let display = context.display().get_display_handle().handle as usize;
                let context_handle = context.get_context_handle() as usize;
                let damage_mode = DamageMode::from_extensions(context.display().extensions());
                let presenter = Presenter::spawn(display, context_handle, damage_mode, ping)?;
                // Host liveness: winit can leave the session idle (or
                // unpaced) when the host socket closes. A separate connection
                // to the host display wakes the loop on EOF so the nested
                // compositor exits instead of spinning.
                let (host_ping, host_source) = calloop::ping::make_ping()
                    .map_err(|error| format!("failed to create host-monitor wakeup: {error}"))?;
                state
                    .loop_handle
                    .insert_source(host_source, |_, _, state| {
                        eprintln!(
                            "dragonfruit-compositor: host Wayland connection closed; \
                             shutting down the nested session"
                        );
                        crate::trace::log("host-disconnected", "monitor");
                        state.running = false;
                    })
                    .map_err(|error| format!("failed to register host-monitor wakeup: {error}"))?;
                if let Err(error) = crate::backend::host_monitor::spawn(host_ping) {
                    eprintln!("dragonfruit-compositor: {error}");
                }
                *shared_init.borrow_mut() = Some(NestedData {
                    blur_program,
                    gaussian_program,
                    glass_program,
                    bevel_program,
                    veil_program,
                    shadow,
                    titlebar,
                    material_textures: MaterialTextureCache::default(),
                    backend: ManuallyDrop::new(backend),
                    damage_tracker: OutputDamageTracker::new(
                        window_size,
                        smithay::utils::Scale::from(scale),
                        smithay::utils::Transform::Flipped180,
                    ),
                    tracker_scale: scale,
                    tracker_size: (window_size.w, window_size.h),
                    gpu_timer: GpuTimer::new(),
                    presenter,
                    pending: None,
                    pending_dirty: false,
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
    // Stop the presenter before the EGL surface/context are dropped. If the
    // host has stalled the swap, the worker may still be inside
    // `eglSwapBuffers`; `shutdown` leaks the backend in that case instead of
    // freeing objects the worker still uses. This only happens on process
    // exit while the host is not presenting the window.
    if let Some(data) = shared.borrow_mut().take() {
        data.shutdown();
    }
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
    /// Scene textures for the independently bounded material groups: slot 0
    /// is the top group, slot 1 the bottom group. The union/control arm uses
    /// slot 0 only.
    scenes: Vec<SceneTextureCache<GlesTexture>>,
    /// One slot per chrome panel; each slot holds the textures for that
    /// panel's Kawase passes (the last one is the composed result).
    panels: PanelTextureSlots<GlesTexture>,
}

impl MaterialTextureCache {
    fn scene_slot(&mut self, slot: usize) -> &mut SceneTextureCache<GlesTexture> {
        while self.scenes.len() <= slot {
            self.scenes.push(SceneTextureCache::default());
        }
        &mut self.scenes[slot]
    }

    fn trim_panels(&mut self, keys: &[PanelKey]) {
        self.panels.trim_to(keys);
    }
}

struct NestedData {
    /// The EGL/GL owner. It is `ManuallyDrop` so [`NestedData::shutdown`] can
    /// leak it when the presenter is still blocked in the host swap; dropping
    /// it while the presenter thread might still use the surface would be a
    /// use-after-free.
    backend: ManuallyDrop<WinitGraphicsBackend<GlowRenderer>>,
    /// The compiled Kawase downsample program (T-20.1), or `None` when the
    /// driver refused it; the blur then falls back to a plain bilinear
    /// downsample chain, which still samples the live scene.
    blur_program: Option<GlesTexProgram>,
    /// The compiled measured separable Gaussian, or `None` when the driver
    /// refused it; the Dock then keeps the sharp panel slice without frost.
    gaussian_program: Option<GlesTexProgram>,
    /// The compiled Tahoe liquid-glass compose program (T-20.2), or `None`
    /// when the driver refused it; the panel then keeps the plain blurred
    /// compose with no lens, specular rim, or adaptive tint.
    glass_program: Option<GlesTexProgram>,
    /// The sharp-scene bevel pass for the measured Dock; `None` keeps the
    /// blurred body's edge treatment only.
    bevel_program: Option<GlesTexProgram>,
    /// The compiled menu-bar no-blur veil program (T200), or `None` when the
    /// driver refused it; the sharp veil then falls back to the flat-band
    /// glass compose.
    veil_program: Option<GlesTexProgram>,
    /// The compiled one-quad shadow program and its white probe texture
    /// (T238 decoration optimisation), or `None` when the driver refused it.
    shadow: Option<ShadowResources>,
    /// The compiled titlebar-shape program (T238 decoration optimisation),
    /// or `None` when the driver refused it.
    titlebar: Option<crate::window::TitlebarResources>,
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
    /// Off-loop host swap (T238 P0). The main loop builds and draws a frame,
    /// releases the EGL context, and hands the surface to this thread; the
    /// host's frame-callback wait can no longer starve client dispatch,
    /// synthetic input, or signals.
    presenter: Presenter,
    /// The frame currently owned by the presenter. At most one is in flight,
    /// which is also what makes the context handoff unambiguous.
    pending: Option<PendingPresent>,
    /// A new frame was requested while `pending` was in flight. Counted once
    /// per pending interval so `frames_coalesced` is not a loop-iteration
    /// count.
    pending_dirty: bool,
}

/// Post-swap work that must run on the session loop after the presenter
/// finishes a frame: presentation feedback, client frame callbacks, stats.
struct PendingPresent {
    frame: u64,
    /// When the frame's render began, so the recorded frame time includes the
    /// presenter's host wait.
    started: Instant,
    output: smithay::output::Output,
    states: smithay::backend::renderer::element::RenderElementStates,
    feedback: Option<smithay::desktop::utils::OutputPresentationFeedback>,
    /// A damage frame is a real presentation; the no-damage swap only wakes
    /// clients waiting on a frame callback.
    damage: bool,
    /// The live scene epoch when this frame's build started. If it advances
    /// before the swap completes, the presented pixels were already
    /// superseded by newer scene content.
    scene_epoch: u64,
    /// When this frame's scene content was built. Superseded ages are measured
    /// from here, not from the physical presentation time.
    scene_built_at: Instant,
    /// When the frame was handed to the presenter.
    submitted_at: Instant,
    /// When the scene first advanced past [`Self::scene_epoch`], if that
    /// happened before the handoff. `None` means the scene was still current
    /// at submission; a later advance is read from the live state.
    scene_changed_at: Option<Instant>,
}

impl NestedData {
    /// Stop the presenter and release the EGL backend.
    ///
    /// This is the **only** path that may drop the EGL objects, and it is only
    /// called after `run_session` returns (process exit). If the presenter is
    /// still blocked in `eglSwapBuffers` after the two-second grace period,
    /// the whole session is forgotten: the worker keeps using a live EGL
    /// surface/context and the process exits without freeing them. In-process
    /// recovery (a `BadSurface` swap) never calls this; it rebinds and retries
    /// on the session loop with the presenter idle.
    fn shutdown(mut self) {
        if !self.presenter.stop() {
            eprintln!(
                "dragonfruit-compositor: present thread still blocked at shutdown; \
                 leaking the EGL backend"
            );
            // The worker may still dereference the surface/context. Forget the
            // whole session instead of dropping `backend` underneath it.
            std::mem::forget(self);
            return;
        }
        // The presenter is joined, so no worker can touch EGL. `backend` is
        // `ManuallyDrop`; drop it explicitly before the remaining fields.
        // SAFETY: `shutdown` consumes `self` and drops the backend exactly
        // once, after the presenter has stopped.
        unsafe { ManuallyDrop::drop(&mut self.backend) };
    }

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

    /// Make the EGL context current on the window surface again after the
    /// presenter released it. `EGL_BUFFER_AGE` is only meaningful while the
    /// surface is current, and the damage tracker's no-damage skip depends on
    /// it.
    fn make_current(&mut self) -> bool {
        let (display, context) = {
            let context = self.backend.renderer().egl_context();
            (
                context.display().get_display_handle(),
                context.get_context_handle(),
            )
        };
        let surface = self.backend.egl_surface().get_surface_handle();
        // SAFETY: the display/context/surface are alive and owned by `backend`,
        // and the presenter is idle (`pending` is `None`) before this runs.
        unsafe {
            ffi::egl::MakeCurrent(display.handle, surface, surface, context) != ffi::egl::FALSE
        }
    }

    /// Finish a frame whose host swap the presenter completed. This runs on
    /// the session loop, so client dispatch and input kept running while the
    /// presenter waited for the host.
    fn finish_present(
        &mut self,
        state: &mut crate::state::DfState,
        done: PresentDone,
    ) -> Result<(), String> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };
        state.present_in_flight = false;
        debug_assert_eq!(pending.frame, done.frame);
        if crate::trace::enabled() {
            thread_local! {
                static PRESENT_TIMING_LOGS: std::cell::Cell<u32> =
                    const { std::cell::Cell::new(0) };
            }
            let logged = PRESENT_TIMING_LOGS.with(|count| {
                let value = count.get();
                count.set(value.saturating_add(1));
                value
            });
            if logged < 20 {
                crate::trace::log(
                    "present-timing",
                    &format!(
                        "frame={} make_current_us={} swap_us={} total_us={}",
                        pending.frame,
                        done.make_current.as_micros(),
                        done.swap.as_micros(),
                        done.duration.as_micros()
                    ),
                );
            }
        }
        state
            .stats
            .record_frame_stage(crate::state::FrameStage::Submit, done.duration);
        let refresh = pending
            .output
            .current_mode()
            .map(|mode| Duration::from_secs_f64(1_000f64 / mode.refresh as f64))
            .unwrap_or(Duration::from_secs_f64(1.0 / 60.0));
        if done.duration > refresh.saturating_mul(2) {
            state.stats.present_stalls = state.stats.present_stalls.saturating_add(1);
            crate::trace::log(
                "present-stall",
                &format!(
                    "frame={} output={} waited_us={}",
                    pending.frame,
                    pending.output.name(),
                    done.duration.as_micros()
                ),
            );
        }
        if state.scene_epoch() > pending.scene_epoch {
            // The scene changed while this frame was in flight. The host
            // presented the older pixels; the next frame will catch up. Record
            // the age of the presented content at handoff and at swap
            // completion, plus how long the newer scene has been waiting.
            let newer_scene_at = pending
                .scene_changed_at
                .unwrap_or_else(|| state.scene_epoch_changed_at());
            let submit_age = pending
                .submitted_at
                .saturating_duration_since(pending.scene_built_at);
            let completion_age = done
                .completed
                .saturating_duration_since(pending.scene_built_at);
            let newer_scene_age = done.completed.saturating_duration_since(newer_scene_at);
            let to_us = |duration: Duration| duration.as_micros().min(u128::from(u32::MAX)) as u32;
            state
                .stats
                .record_superseded(crate::state::SupersededSample {
                    submit_age_us: to_us(submit_age),
                    completion_age_us: to_us(completion_age),
                    newer_scene_age_us: to_us(newer_scene_age),
                    damage: pending.damage,
                });
        }
        match done.result {
            Ok(()) => {
                if pending.damage {
                    state.note_presentation(&pending.output.name(), done.presented_at);
                    crate::trace::log(
                        "present",
                        &format!(
                            "frame={} output={} nested",
                            pending.frame,
                            pending.output.name()
                        ),
                    );
                    if let Some(mut feedback) = pending.feedback {
                        feedback.presented::<_, smithay::utils::Monotonic>(
                            done.presented_at,
                            Refresh::fixed(refresh),
                            0,
                            wp_presentation_feedback::Kind::Vsync,
                        );
                        state.stats.presentation_feedback_sent =
                            state.stats.presentation_feedback_sent.saturating_add(1);
                    }
                    let callback_start = Instant::now();
                    render::post_repaint(
                        state,
                        &pending.output,
                        done.presented_at,
                        &pending.states,
                    );
                    state.stats.record_frame_stage(
                        crate::state::FrameStage::Callback,
                        callback_start.elapsed(),
                    );
                } else {
                    let callback_start = Instant::now();
                    render::send_frame_callbacks(
                        state,
                        &pending.output,
                        done.presented_at,
                        &pending.states,
                    );
                    state.stats.record_frame_stage(
                        crate::state::FrameStage::Callback,
                        callback_start.elapsed(),
                    );
                    crate::trace::log("wake-clients", "nested");
                }
                state.stats.frames_rendered += 1;
                let duration = done.completed.saturating_duration_since(pending.started);
                state.observe_rendered_frame(duration);
                state.stats.frame_observed_by_backend = true;
                Ok(())
            }
            Err(PresentError::BadSurface) => {
                state.stats.swaps_failed = state.stats.swaps_failed.saturating_add(1);
                // The host destroyed/recreated the surface. Only Smithay's
                // `EGLSurface::swap_buffers` path recreates it, so run one
                // blocking swap on the loop thread. This is rare (a surface
                // loss), and it is the only way to restore the presenter's
                // surface handle.
                eprintln!(
                    "dragonfruit-compositor: nested present lost the EGL surface; \
                     recreating through the synchronous path"
                );
                if self.backend.bind().is_ok() {
                    let _ = self.backend.submit(None);
                }
                state.needs_redraw = true;
                Ok(())
            }
            Err(PresentError::Other(message)) => {
                state.stats.swaps_failed = state.stats.swaps_failed.saturating_add(1);
                Err(format!("nested present failed: {message}"))
            }
        }
    }
}

fn render_frame(state: &mut crate::state::DfState, data: &mut NestedData) -> Result<(), String> {
    // Finish any swap the presenter completed while the loop was dispatching.
    // This must run before the output/deadline check so a completion is
    // processed even on an otherwise idle iteration.
    while let Some(done) = data.presenter.try_recv() {
        data.finish_present(state, done)?;
    }
    if data.pending.is_some() {
        // The host still owns the previous frame. Keep the loop free for
        // client dispatch, input, and signals; the presenter's ping wakes us
        // when the swap returns. Count one coalesced request per pending
        // interval, not one per loop iteration.
        if state.needs_redraw && !data.pending_dirty {
            data.pending_dirty = true;
            state.stats.frames_coalesced = state.stats.frames_coalesced.saturating_add(1);
        }
        return Ok(());
    }
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
    state.stats.frame_attempts = state.stats.frame_attempts.saturating_add(1);
    data.pending_dirty = false;
    // Frame time spans scene build, draw, and the presenter's host wait.
    let frame_start = Instant::now();
    // The scene content this frame renders is the state as of build start.
    // Superseded ages are measured from this point; if the epoch advances
    // during the build or the host wait, the presented pixels are already old.
    let scene_built_at = frame_start;
    let scene_epoch_at_build = state.scene_epoch();
    // T238 follow-up: every stage and GPU query in this frame carries this id.
    // The trace lets the shell/driver join a commit with the exact presented
    // frame instead of guessing from a millisecond window.
    let frame = state.stats.begin_frame();
    crate::trace::log(
        "frame-start",
        &format!("frame={frame} output={}", output.name()),
    );
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
        // The presenter released the context after the last swap. Make it
        // current on the surface again before asking for the back-buffer age;
        // otherwise the query returns no age and the damage tracker stops
        // skipping unchanged frames.
        data.make_current();
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
    let bind_start = Instant::now();
    let render_result = match data.backend.bind() {
        Ok((renderer, mut framebuffer)) => {
            // Buffer acquisition can wait for a free host swapchain buffer;
            // record it separately from the render work.
            state
                .stats
                .record_frame_stage(crate::state::FrameStage::Bind, bind_start.elapsed());
            let prepare_start = Instant::now();
            let scale = smithay::utils::Scale::from(output.current_scale().fractional_scale());
            let output_geometry = state.space.output_geometry(&output).unwrap_or_default();
            // Collect any GPU timer results from earlier frames before
            // starting this frame's passes (T-21.4b). Polling never waits on
            // the GPU; unavailable/disabled timers publish their reason.
            for sample in data.gpu_timer.poll(renderer) {
                state.stats.record_gpu_sample(sample);
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
            // T238 follow-up: source-level element counts for the final
            // composite breakdown.
            let mut scene_counts = crate::render::SceneElementCounts::default();
            let menu_count: u64;
            let desktop_count: u64;
            let wallpaper_count: u64;
            let lock_count: u64;
            if !blur_panels.is_empty() {
                // Two-pass composition (T-20.1/T-21.3). Pass one renders the
                // scene (wallpaper, desktop, windows, shadows, titlebars) into
                // a texture sized to the padded panel union; pass two composes
                // the output from the live scene elements + the per-panel
                // blurred samples + the chrome surfaces. The blur samples the
                // live scene texture inside this render pass — never a
                // screenshot or screencopy.
                let output_local = Rectangle::from_size(output_geometry.size);
                // The control arm renders one padded union; the split arm
                // gives the top and bottom panel groups independent textures.
                let groups = MaterialRegion::groups_for_material(&blur_panels, output_local, scale);
                if groups.is_empty() {
                    return Err("material region unavailable".to_owned());
                }
                let full_output = full_output_material_forced();
                // The Kawase chain's first destination stays full-output
                // sized: its texture size is part of the panel's visual
                // result, and T-21.3 must not change the blur at the panel.
                let chain_size = output
                    .current_mode()
                    .map(|mode| mode.size)
                    .unwrap_or_default();
                let scene_build_start = Instant::now();
                let mut scene_elements: Vec<NestedOutputElements<GlowRenderer>> = Vec::new();
                let menu = crate::render::window_menu_render_elements(state, &output, scale);
                menu_count = menu.len() as u64;
                scene_elements.extend(menu.into_iter().map(NestedOutputElements::Decoration));
                scene_elements.extend(crate::render::window_scene_render_elements(
                    renderer,
                    state,
                    &output,
                    scale,
                    &mut scene_counts,
                    data.shadow.as_ref(),
                    data.titlebar.as_ref(),
                ));
                let desktop =
                    crate::render::desktop_render_elements(renderer, state, &output, scale);
                desktop_count = desktop.len() as u64;
                scene_elements.extend(desktop);
                let wallpaper =
                    crate::render::wallpaper_render_elements(renderer, state, &output, scale);
                wallpaper_count = wallpaper.len() as u64;
                scene_elements.extend(wallpaper.into_iter().map(NestedOutputElements::Wallpaper));
                state.stats.record_frame_stage(
                    crate::state::FrameStage::SceneBuild,
                    scene_build_start.elapsed(),
                );
                state
                    .stats
                    .record_frame_stage(crate::state::FrameStage::Prepare, prepare_start.elapsed());
                // A locked session composites only the lock surface, prepended
                // so it sits above every window and chrome surface (T-12.3a).
                custom_elements =
                    crate::lock::lock_render_elements(renderer, state, &output, scale);
                lock_count = custom_elements.len() as u64;
                // Chrome surfaces (menu bar, drawer, overlays) composite above
                // the blur.
                custom_elements.extend(crate::render::chrome_render_elements(
                    renderer, state, &output, scale,
                ));

                // One scene pass per bounded group. The stage timing is the
                // whole frame's scene work, not one sample per group: the
                // advisor's per-frame totals must not be inflated by counting
                // the top and bottom passes as unrelated samples.
                let scene_start = Instant::now();
                let scene_epoch = state.scene_epoch();
                let allow_skip =
                    crate::window::scene_cache_skip_enabled() && !state.scene_is_animating();
                let mut scene_sync = Duration::ZERO;
                let mut scene_textures = Vec::with_capacity(groups.len());
                for group in &groups {
                    let region = group.region;
                    state
                        .stats
                        .record_material_region_for(output.name().as_str(), region.texture_size);
                    data.gpu_timer.begin(renderer, GpuPass::Scene, frame);
                    let cache = data.material_textures.scene_slot(group.slot);
                    let scene_texture_result = render_scene_offscreen(
                        renderer,
                        state,
                        &output,
                        scale,
                        region,
                        &scene_elements,
                        cache,
                        scene_epoch,
                        allow_skip,
                    );
                    data.gpu_timer.end(renderer);
                    if let Some(wait) = data.gpu_timer.finish(renderer) {
                        scene_sync = scene_sync.saturating_add(wait);
                    }
                    scene_textures.push((group.slot, region, scene_texture_result?));
                }
                if !scene_sync.is_zero() {
                    state
                        .stats
                        .record_frame_stage(crate::state::FrameStage::SceneSync, scene_sync);
                }
                let scene_bytes: u64 = groups
                    .iter()
                    .map(|group| {
                        data.material_textures
                            .scenes
                            .get(group.slot)
                            .map(|cache| cache.bytes())
                            .unwrap_or(0)
                    })
                    .sum();
                state.stats.record_material_scene_bytes(scene_bytes);
                state
                    .stats
                    .record_frame_stage(crate::state::FrameStage::Scene, scene_start.elapsed());

                let blur_start = Instant::now();
                let mut blur_allocations = 0u64;
                let mut blur_skips = 0u64;
                let mut blur_misses = BlurMissCounts::default();
                let mut blur_drawn = 0u64;
                let mut downsample_max = 0u32;
                for (slot, region, scene_texture) in &scene_textures {
                    let scene_cache = &data.material_textures.scenes[*slot];
                    let blur = build_blur_elements(
                        renderer,
                        BlurPrograms {
                            blur: data.blur_program.as_ref(),
                            gaussian: data.gaussian_program.as_ref(),
                            glass: data.glass_program.as_ref(),
                            bevel: data.bevel_program.as_ref(),
                            veil: data.veil_program.as_ref(),
                            chain_size,
                        },
                        &groups
                            .iter()
                            .find(|group| group.slot == *slot)
                            .map(|group| group.panels.as_slice())
                            .unwrap_or(&[]),
                        BlurScene {
                            texture: scene_texture,
                            cache: scene_cache,
                            region: *region,
                        },
                        scale,
                        BlurTargets {
                            panels: &mut data.material_textures.panels,
                            timer: &mut data.gpu_timer,
                            frame,
                        },
                    )?;
                    blur_allocations += blur.allocations;
                    blur_skips += blur.skips;
                    blur_misses.first += blur.misses.first;
                    blur_misses.scene += blur.misses.scene;
                    blur_misses.source += blur.misses.source;
                    blur_misses.spec += blur.misses.spec;
                    blur_misses.chain += blur.misses.chain;
                    custom_elements.extend(blur.elements);
                    let filtered: Vec<_> = groups
                        .iter()
                        .find(|group| group.slot == *slot)
                        .map(|group| {
                            group
                                .panels
                                .iter()
                                .filter(|panel| panel.filter.schedules_blur())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if !filtered.is_empty() {
                        blur_drawn += filtered.len() as u64;
                        downsample_max = downsample_max.max(
                            filtered
                                .iter()
                                .map(|panel| panel.spec.downsample())
                                .max()
                                .unwrap_or(0),
                        );
                    }
                }
                state
                    .stats
                    .record_frame_stage(crate::state::FrameStage::Blur, blur_start.elapsed());
                state.stats.record_material_panel_allocations_for(
                    output.name().as_str(),
                    blur_allocations,
                );
                state.stats.record_blur_skips(blur_skips);
                state.stats.record_blur_misses(blur_misses);
                if blur_drawn > 0 {
                    state.stats.record_blur_draw(downsample_max, blur_drawn);
                }
                state.stats.record_material_frame(output.name().as_str());
                if full_output {
                    // Diagnostic/rollback shape: the full-output scene texture
                    // is the output base, exactly like the pre-T-21.3 path.
                    // The scene cache owns the stable id and the commit that
                    // advances only when the offscreen scene changed (T-21.3).
                    let (slot, _, scene_texture) = &scene_textures[0];
                    let cache = &data.material_textures.scenes[*slot];
                    let scene_id = cache.id();
                    let scene_commit = cache.commit();
                    custom_elements.push(NestedOutputElements::Blur(
                        crate::window::BackdropBlurElement::new(
                            (*scene_texture).clone(),
                            Rectangle::new((0, 0).into(), output_geometry.size),
                            scene_texture.size(),
                            0,
                            scale,
                        )
                        .with_id(scene_id)
                        .with_commit(scene_commit),
                    ));
                } else {
                    // Region shape: the scene elements are the output base and
                    // the region textures are only the blur source. The list
                    // is already front-to-back (window menu, windows, desktop,
                    // wallpaper), so appending it below the blur preserves the
                    // old stacking order.
                    custom_elements.extend(scene_elements);
                }
                // Release the slots of panels that disappeared this frame.
                // This runs once after every group, not per group, so a panel
                // in the other half is not dropped.
                let panel_keys: Vec<PanelKey> = blur_panels.iter().map(|panel| panel.key).collect();
                data.material_textures.trim_panels(&panel_keys);
            } else {
                state
                    .stats
                    .record_frame_stage(crate::state::FrameStage::Prepare, prepare_start.elapsed());
                // No sampled material this frame: release the per-panel Kawase
                // textures (T-21.3 lifetime audit) and keep the pre-T-20
                // single-pass path: chrome + the feather backdrop (no GPU
                // readback). A locked session composites only the lock
                // surface, prepended so it sits above everything (T-12.3a).
                data.material_textures.trim_panels(&[]);
                custom_elements =
                    crate::lock::lock_render_elements(renderer, state, &output, scale);
                lock_count = custom_elements.len() as u64;
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
                let scene_build_start = Instant::now();
                let menu = crate::render::window_menu_render_elements(state, &output, scale);
                menu_count = menu.len() as u64;
                custom_elements.extend(menu.into_iter().map(NestedOutputElements::Decoration));
                custom_elements.extend(crate::render::window_scene_render_elements(
                    renderer,
                    state,
                    &output,
                    scale,
                    &mut scene_counts,
                    data.shadow.as_ref(),
                    data.titlebar.as_ref(),
                ));
                let desktop =
                    crate::render::desktop_render_elements(renderer, state, &output, scale);
                desktop_count = desktop.len() as u64;
                custom_elements.extend(desktop);
                let wallpaper =
                    crate::render::wallpaper_render_elements(renderer, state, &output, scale);
                wallpaper_count = wallpaper.len() as u64;
                custom_elements.extend(wallpaper.into_iter().map(NestedOutputElements::Wallpaper));
                state.stats.record_frame_stage(
                    crate::state::FrameStage::SceneBuild,
                    scene_build_start.elapsed(),
                );
            }
            // T238 follow-up: break the final composite list down by element
            // type. The source counters are a second view of the scene list;
            // in full-output mode that list is the offscreen pass, not the
            // final composite, so only the variant counts apply.
            let full_output = full_output_material_forced();
            let mut composite_counts = crate::state::CompositeElementCounts {
                lock: lock_count,
                ..Default::default()
            };
            if !full_output {
                composite_counts.titlebar = scene_counts.titlebar;
                composite_counts.titlebar_fill = scene_counts.titlebar_fill;
                composite_counts.titlebar_lights = scene_counts.titlebar_lights;
                composite_counts.titlebar_glyphs = scene_counts.titlebar_glyphs;
                composite_counts.window_content = scene_counts.window_content;
                composite_counts.window_shadow = scene_counts.window_shadow;
                composite_counts.shadow_area = scene_counts.shadow_area;
                composite_counts.shadow_span_area = scene_counts.shadow_span_area;
                composite_counts.titlebar_area = scene_counts.titlebar_area;
                composite_counts.motion = scene_counts.motion;
                composite_counts.menu = menu_count;
                composite_counts.desktop = desktop_count;
                composite_counts.wallpaper = wallpaper_count;
            }
            for element in &custom_elements {
                match element {
                    NestedOutputElements::Chrome(_) => composite_counts.chrome += 1,
                    NestedOutputElements::Decoration(_) => composite_counts.decoration += 1,
                    NestedOutputElements::Appear(_) => composite_counts.appear += 1,
                    NestedOutputElements::Wallpaper(_) => composite_counts.wallpaper += 1,
                    NestedOutputElements::Blur(_) => composite_counts.blur += 1,
                    // The macro's uninhabited generic-catcher variant.
                    _ => {}
                }
            }
            state
                .stats
                .record_composite_counts(composite_counts, !blur_panels.is_empty());
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
            let composite_start = Instant::now();
            data.gpu_timer.begin(renderer, GpuPass::Composite, frame);
            let render_result = data.damage_tracker.render_output(
                renderer,
                &mut framebuffer,
                age,
                &frame_elements,
                state.wallpaper_color_for(&output),
            );
            data.gpu_timer.end(renderer);
            if let Some(wait) = data.gpu_timer.finish(renderer) {
                state
                    .stats
                    .record_frame_stage(crate::state::FrameStage::CompositeSync, wait);
            }
            state.stats.record_frame_stage(
                crate::state::FrameStage::Composite,
                composite_start.elapsed(),
            );
            for sample in data.gpu_timer.poll(renderer) {
                state.stats.record_gpu_sample(sample);
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

    // The capture is a readback of the offscreen pass, independent of the
    // host swap; answer it before the frame leaves the main thread.
    if let Some(result) = captured.take() {
        resolve_capture(state, result);
    }

    // Destructure before the handoff: `damage` borrows the tracker, while
    // `states` is owned by this frame.
    let (damage, states) = {
        let RenderOutputResult { damage, states, .. } = render_result;
        (damage.cloned(), states)
    };
    let damage_frame = damage.is_some();
    let feedback =
        damage_frame.then(|| render::take_presentation_feedback(state, &output, &states));
    let window_size = data.backend.window_size();
    let damage_rects = damage.as_ref().map(|rects| {
        rects
            .iter()
            .map(|rect| {
                [
                    rect.loc.x,
                    window_size.h - rect.loc.y - rect.size.h,
                    rect.size.w,
                    rect.size.h,
                ]
            })
            .collect::<Vec<_>>()
    });
    // winit registers its own frame callback; keep that bookkeeping on the
    // loop thread before the surface is handed over.
    data.backend.window().pre_present_notify();
    let surface = data.backend.egl_surface().get_surface_handle() as usize;
    // Release the context so the presenter can make the same context current
    // on its own thread. The next bind makes it current here again.
    if let Err(error) = data.backend.renderer().egl_context().unbind() {
        return Err(format!(
            "nested present could not release the EGL context: {error:?}"
        ));
    }
    if let Err(error) = data.presenter.submit(PresentRequest {
        frame,
        surface,
        damage: damage_rects,
        fault: state.present_fault.take(),
    }) {
        return Err(format!("nested present handoff failed: {error}"));
    }
    state.stats.frames_submitted = state.stats.frames_submitted.saturating_add(1);
    if damage_frame {
        state.stats.damage_frames = state.stats.damage_frames.saturating_add(1);
    } else {
        // No damage, no render, no client wakeups (FR-2) — except that a woken
        // compositor must still reply to a client waiting on its frame
        // callback. The presenter defers that reply until the swap returns.
        state.stats.frames_skipped_no_damage += 1;
    }
    let submitted_at = Instant::now();
    let scene_changed_at =
        (state.scene_epoch() > scene_epoch_at_build).then(|| state.scene_epoch_changed_at());
    data.pending = Some(PendingPresent {
        frame,
        started: frame_start,
        output,
        states,
        feedback,
        damage: damage_frame,
        scene_epoch: scene_epoch_at_build,
        scene_built_at,
        submitted_at,
        scene_changed_at,
    });
    state.present_in_flight = true;
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
    // The live scene-content epoch and whether an unchanged epoch may reuse
    // the texture without walking the element list (T238 follow-up). The
    // caller disables the skip while a scene animation is live.
    epoch: u64,
    allow_skip: bool,
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
    if allow_skip && cache.can_skip(key, region.physical.loc, epoch) {
        state
            .stats
            .record_material_region_for(output.name().as_str(), region.texture_size);
        state.stats.record_scene_skip();
        return cache
            .texture()
            .cloned()
            .ok_or_else(|| "scene texture cache unexpectedly empty".to_owned());
    }
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
    // Relocate the output-local elements into the region's physical space.
    // `&E` implements `RenderElement`, so this borrows the caller's list
    // instead of consuming it; the same list is drawn live in region mode.
    //
    // Only elements that intersect the bounded region are submitted. Without
    // this the tracker still scissors them, but every off-region window,
    // titlebar, shadow, and wallpaper draw call is issued into a texture that
    // cannot contain it. The filter is also what makes the top/bottom split
    // pay off: each strip submits only the elements that can reach it.
    let offset = Point::<i32, Physical>::from((-region.physical.loc.x, -region.physical.loc.y));
    let total_elements = elements.len();
    let mut kept_elements = 0usize;
    let relocated: Vec<_> = elements
        .iter()
        .filter(|element| {
            let geometry = element.geometry(scale);
            if geometry.size.w <= 0 || geometry.size.h <= 0 {
                return false;
            }
            geometry.overlaps(region.physical)
        })
        .inspect(|_| kept_elements += 1)
        .map(|element| RelocateRenderElement::from_element(element, offset, Relocate::Relative))
        .collect();
    state
        .stats
        .record_scene_elements(total_elements, kept_elements);
    // The element filter above means a window that leaves the bounded region
    // is never submitted to the damage tracker. Its last pixels would stay in
    // the cached scene texture; the menu-bar veil samples that texture
    // directly, so it read as a ghost of the departed window. Detect the set
    // change and reset the tracker, forcing one full region repaint.
    let mut element_hasher = DefaultHasher::new();
    for element in &relocated {
        element.id().hash(&mut element_hasher);
    }
    if cache.note_region_elements(element_hasher.finish()) {
        cache.reset_tracker();
    }
    // Keep the scene damage tracker across frames (T-21.3). The first frame
    // (or the first frame after a reallocation/region change) renders the
    // whole region; later frames render only the damage the live elements
    // report, and an unchanged scene renders nothing at all. Age 1 keeps
    // Smithay from re-damaging the whole output on every call while still
    // retaining the last element states.
    if !cache.tracker_matches(key, region.physical.loc) {
        cache.set_tracker(
            key,
            region.physical.loc,
            OutputDamageTracker::new(tracker_size, scale, Transform::Normal),
        );
    }
    let damage = {
        let tracker = cache
            .tracker_mut()
            .ok_or_else(|| "scene damage tracker missing".to_owned())?;
        let mut target = renderer
            .bind(&mut texture)
            .map_err(|err| format!("scene bind failed: {err}"))?;
        let result = tracker
            .render_output(renderer, &mut target, 1, &relocated, clear)
            .map_err(|err| format!("scene render failed: {err:?}"))?;
        result.damage.cloned()
    };
    cache.note_scene_damage(damage);
    cache.set_last_epoch(epoch);
    Ok(texture)
}

/// The compiled programs and chain base for one blur build.
struct BlurPrograms<'a> {
    blur: Option<&'a GlesTexProgram>,
    gaussian: Option<&'a GlesTexProgram>,
    glass: Option<&'a GlesTexProgram>,
    bevel: Option<&'a GlesTexProgram>,
    veil: Option<&'a GlesTexProgram>,
    /// The full output's physical size: the Kawase chain keeps that as its
    /// first destination so the panel's visual blur is unchanged from the
    /// pre-region path.
    chain_size: Size<i32, Physical>,
}

/// The caches and timers the blur build owns. Bundled so the function keeps a
/// readable argument list.
struct BlurTargets<'a> {
    panels: &'a mut PanelTextureSlots<GlesTexture>,
    timer: &'a mut GpuTimer,
    /// Compositor frame id for the blur timer queries (T238 follow-up).
    frame: u64,
}

/// The scene texture and identity one blur build samples.
struct BlurScene<'a> {
    texture: &'a GlesTexture,
    cache: &'a SceneTextureCache<GlesTexture>,
    region: MaterialRegion,
}

/// The draw elements and counters produced by one blur build.
struct BlurBuild {
    elements: Vec<NestedOutputElements<GlowRenderer>>,
    allocations: u64,
    skips: u64,
    misses: BlurMissCounts,
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
) -> Result<BlurBuild, String> {
    let mut elements = Vec::new();
    let mut allocations = 0u64;
    let mut skips = 0u64;
    let mut misses = BlurMissCounts::default();
    for panel in panels {
        // The measured Dock is the only role whose blur texture is padded:
        // its bevel lens reaches beyond the visible silhouette and its
        // Gaussian must not clamp at the rim. Other roles keep the historical
        // panel-sized source so their full-output chain mapping is unchanged.
        let source_padding = if panel.spec.is_measured() {
            panel.sample_padding_px()
        } else {
            0
        };
        let Some((texture_panel, scene_src)) =
            scene
                .region
                .panel_source_padded(panel.source_panel(), source_padding, scale)
        else {
            continue;
        };
        let slot = targets.panels.slot(&panel.key);
        // Paint-only changes (scheme/tint/tier/radius/output generation) reuse
        // the filtered texture but must damage the element so the new paint is
        // composited (T197).
        slot.note_paint(PanelPaintKey::new(
            panel.paint,
            panel.corners,
            scene.cache.generation(),
        ));
        let id = slot.id().clone();
        if panel.filter == RegionFilter::Sharp {
            // Veil/sharp path: sample the compositor's scene texture directly.
            // No Kawase allocation and no blur pass is scheduled; the element
            // covers the whole padded region and clips to the region corners.
            // The scene commit advances only on real scene damage, so a moving
            // window under the bar damages the veil while a settled scene does
            // not (T200).
            slot.note_sharp_scene_commit(scene.cache.commit());
            let commit = slot.commit();
            let element = crate::window::BackdropBlurElement::new(
                scene.texture.clone(),
                panel.panel,
                scene.texture.size(),
                panel.corners.max(),
                scale,
            )
            .with_id(id)
            .with_commit(commit)
            .with_texture_region(scene.region.rect)
            .with_corners(panel.corners);
            // T200: a resolved veil uses its own depth-LUT shader. If the
            // driver refused that program, the flat-band glass compose keeps
            // a scheme-appropriate plate instead of exposing the raw scene.
            let element = if let Some(veil) = panel.veil {
                match programs.veil {
                    Some(program) => element.with_veil(veil, program.clone()),
                    None => match programs.glass {
                        Some(glass) => element.with_glass(panel.glass, glass.clone()),
                        None => element,
                    },
                }
            } else {
                element
            };
            elements.push(NestedOutputElements::Blur(element));
            continue;
        }
        let blur_key = slot.blur_key_for_scene(
            scene.cache,
            scene_src,
            crate::window::RegionSample {
                spec: panel.spec,
                padding: panel.sample_padding_px(),
                filter: panel.filter,
            },
            programs.chain_size,
            scale,
        );
        let texture = if slot.blur_cache_hit(&blur_key) {
            skips += 1;
            slot.last_texture()
                .cloned()
                .ok_or_else(|| "blur cache hit without a texture".to_owned())?
        } else {
            let reason = slot
                .blur_key()
                .and_then(|previous| blur_key.miss_reason(previous))
                .unwrap_or(BlurMissReason::First);
            misses.record(reason);
            // Clear the key before touching the chain: a failed render must
            // not let the next frame reuse a partially written texture.
            slot.invalidate_blur_cache();
            targets.timer.begin(renderer, GpuPass::Blur, targets.frame);
            let texture_result = blur_panel_texture(
                renderer,
                programs.blur,
                programs.gaussian,
                scene.texture,
                scene_src,
                texture_panel,
                &panel.spec,
                programs.chain_size,
                scale,
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
        // The body keeps the measured rim, flat band and corner lens. The
        // sharp overlay below only sharpens the refraction inside the rim.
        let element = crate::window::BackdropBlurElement::new(
            texture,
            panel.panel,
            texture_size,
            panel.corners.max(),
            scale,
        )
        .with_id(id)
        .with_commit(commit)
        .with_texture_panel(texture_panel)
        .with_corners(panel.corners);
        // The measured Dock uses the SDF as an antialiased coverage mask
        // instead of the integer spans, removing the 1x/1.5x stair-step.
        let element = if panel.spec.is_measured() && programs.glass.is_some() {
            element.with_full_panel_draw()
        } else {
            element
        };
        // Add the liquid-glass compose only when the driver compiled it; a
        // refused shader leaves the plain blurred backdrop (T-20.2).
        let element = match programs.glass {
            Some(glass) => element.with_glass(panel.glass, glass.clone()),
            None => element,
        };

        // The measured Dock gets a sharp-scene bevel over the blurred body.
        // It samples the scene texture directly, so the rim bends real
        // wallpaper detail instead of the already-frosted body. The element
        // draws only the bevel band; the body shows through the interior.
        // The list is front-to-back (Smithay draws it in reverse), so the
        // bevel must be pushed *before* the body to sit on top of it.
        let bevel = if panel.spec.is_measured() {
            programs.bevel.map(|bevel_program| {
                slot.note_sharp_scene_commit(scene.cache.commit());
                crate::window::BackdropBlurElement::new(
                    scene.texture.clone(),
                    panel.panel,
                    scene.texture.size(),
                    panel.corners.max(),
                    scale,
                )
                .with_id(slot.bevel_id().clone())
                .with_commit(slot.commit())
                .with_texture_region(scene.region.rect)
                .with_corners(panel.corners)
                .with_full_panel_draw()
                .with_glass(panel.glass, bevel_program.clone())
            })
        } else {
            None
        };
        if let Some(bevel) = bevel {
            elements.push(NestedOutputElements::Blur(bevel));
        }
        elements.push(NestedOutputElements::Blur(element));
    }
    // The caller trims the panel slots once after every material group, so a
    // panel in the other half is not dropped by this group's build.
    Ok(BlurBuild {
        elements,
        allocations,
        skips,
        misses,
    })
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
    gaussian: Option<&GlesTexProgram>,
    scene: &GlesTexture,
    scene_src: Rectangle<f64, BufferCoord>,
    texture_panel: Rectangle<i32, smithay::utils::Logical>,
    spec: &crate::window::BlurSpec,
    chain_size: Size<i32, Physical>,
    scale: smithay::utils::Scale<f64>,
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

    if spec.is_measured() {
        return gaussian_blur_panel_texture(
            renderer,
            gaussian,
            scene,
            scene_src,
            texture_panel,
            spec,
            scale,
            slot,
        );
    }

    let mut current: GlesTexture = scene.clone();
    let mut allocations = 0u64;
    // The region of `current` the next pass samples. The first pass samples
    // the panel's slice of the scene; later passes sample the whole texture.
    let mut current_src = scene_src;
    // The shared path keeps the chain base at the full output size (its
    // historical pixel-equivalence contract). The T198 Dock composite needs
    // the chain in panel space: a panel-sized source stretched to the output
    // base would make the blur radius effectively `panel/output` too small and
    // misalign every level with the sharp scene.
    let mut chain = if spec.sharp_weights.is_some() || spec.sigma.is_some() {
        texture_panel.to_physical_precise_round(scale).size
    } else {
        chain_size
    };
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
                Uniform::new(crate::window::BLUR_RADIUS_UNIFORM, spec.tap_offset(scale.x)),
            ];
            R::draw_texture_from_to(
                &mut frame,
                &current,
                current_src,
                dst_phys,
                &[dst_phys],
                program,
                &uniforms,
                1.0,
            )
            .map_err(|err| format!("blur draw failed: {err:?}"))?;
        }
        current = texture;
        current_src = Rectangle::from_size(current.size().to_f64());
        chain = Size::<i32, Physical>::from((dst_w, dst_h));
    }

    let Some(weights) = spec.sharp_weights else {
        // Drop any textures left by a previous, wider spec so the panel
        // cache's `last_texture()` is the result of this build.
        slot.truncate(spec.iterations as usize);
        return Ok((current, allocations));
    };

    // T198: compose the sharp scene and the selected Kawase levels into one
    // texture at the panel's visible physical size. Each draw is a normal
    // source-over pass with an alpha chosen so the running sum reaches the
    // convex target weight; the result is exactly `sum(weight_i * level_i)`.
    let composite_size = smithay::utils::Size::<i32, smithay::utils::Buffer>::from((
        (texture_panel.size.w.max(1) as f64 * scale.x)
            .round()
            .max(1.0) as i32,
        (texture_panel.size.h.max(1) as f64 * scale.y)
            .round()
            .max(1.0) as i32,
    ));
    let composite_index = spec.iterations as usize;
    while slot.len() <= composite_index {
        slot.push(
            renderer
                .create_buffer(Fourcc::Abgr8888, composite_size)
                .map_err(|err| format!("blur composite buffer failed: {err}"))?,
        );
        allocations += 1;
    }
    if slot[composite_index].size() != composite_size {
        slot[composite_index] = renderer
            .create_buffer(Fourcc::Abgr8888, composite_size)
            .map_err(|err| format!("blur composite buffer failed: {err}"))?;
        allocations += 1;
    }
    let mut composite = slot[composite_index].clone();
    let dst = Rectangle::from_size(Size::<i32, Physical>::from((
        composite_size.w,
        composite_size.h,
    )));
    // Level 0: the sharp scene slice, opaque.
    {
        let mut target = renderer
            .bind(&mut composite)
            .map_err(|err| format!("blur composite bind failed: {err}"))?;
        let mut frame = renderer
            .render(&mut target, dst.size, Transform::Normal)
            .map_err(|err| format!("blur composite render failed: {err}"))?;
        R::draw_texture_from_to(&mut frame, scene, scene_src, dst, &[dst], None, &[], 1.0)
            .map_err(|err| format!("blur composite sharp draw failed: {err:?}"))?;
    }
    // Levels 1..=5, in order. The alpha is the level weight over the weight
    // already painted; this makes the final pixel the weighted sum without
    // needing a multi-sampler shader.
    let mut painted = weights[0].max(0.0);
    for (level, weight) in weights.iter().enumerate().skip(1) {
        let weight = weight.max(0.0);
        if weight == 0.0 {
            continue;
        }
        let level_index = level.saturating_sub(1);
        let Some(texture) = slot.get(level_index).cloned() else {
            continue;
        };
        let total = painted + weight;
        let alpha = if total > 1e-6 { weight / total } else { 0.0 };
        let src = Rectangle::from_size(texture.size().to_f64());
        let mut target = renderer
            .bind(&mut composite)
            .map_err(|err| format!("blur composite bind failed: {err}"))?;
        let mut frame = renderer
            .render(&mut target, dst.size, Transform::Normal)
            .map_err(|err| format!("blur composite render failed: {err}"))?;
        R::draw_texture_from_to(&mut frame, &texture, src, dst, &[dst], None, &[], alpha)
            .map_err(|err| format!("blur composite level draw failed: {err:?}"))?;
        painted = total;
    }
    slot.truncate(composite_index + 1);
    Ok((composite, allocations))
}

/// The measured Dock blur: one sharp scene composite over a separable
/// Gaussian. This replaces the old five-level Kawase pyramid (T198 follow-up).
///
/// The panel path is small, so both passes run at the panel's physical
/// resolution. The first pass samples the scene texture directly, which lets
/// the Gaussian read the material region's padding beyond the visible panel;
/// the second pass is vertical and returns a panel-sized texture.
fn gaussian_blur_panel_texture<R>(
    renderer: &mut R,
    program: Option<&GlesTexProgram>,
    scene: &GlesTexture,
    scene_src: Rectangle<f64, BufferCoord>,
    texture_panel: Rectangle<i32, smithay::utils::Logical>,
    spec: &crate::window::BlurSpec,
    scale: smithay::utils::Scale<f64>,
    slot: &mut Vec<GlesTexture>,
) -> Result<(GlesTexture, u64), String>
where
    R: TextureRenderer
        + smithay::backend::renderer::Bind<GlesTexture>
        + smithay::backend::renderer::Offscreen<GlesTexture>,
    R::Error: std::fmt::Debug,
{
    use smithay::backend::renderer::gles::Uniform;

    let size = BufferSize::<i32, BufferCoord>::from((
        (texture_panel.size.w.max(1) as f64 * scale.x)
            .round()
            .max(1.0) as i32,
        (texture_panel.size.h.max(1) as f64 * scale.y)
            .round()
            .max(1.0) as i32,
    ));
    let sigma = spec.sigma.unwrap_or(0.0).max(0.01);

    // If the driver refused the Gaussian program, return the sharp panel
    // slice. It is still a valid panel texture; it simply has no frost.
    let Some(program) = program else {
        let (mut texture, allocations) = ensure_blur_texture(renderer, slot, 0, size)?;
        draw_blur_texture(
            renderer,
            &mut texture,
            scene,
            scene_src,
            size,
            None,
            &[],
            1.0,
        )?;
        slot.truncate(1);
        return Ok((texture, allocations));
    };

    let (mut horizontal, mut allocations) = ensure_blur_texture(renderer, slot, 0, size)?;
    let scene_size = scene.size();
    draw_blur_texture(
        renderer,
        &mut horizontal,
        scene,
        scene_src,
        size,
        Some(program),
        &[
            Uniform::new(
                crate::window::BLUR_TEXEL_UNIFORM,
                (
                    1.0f32 / scene_size.w.max(1) as f32,
                    1.0f32 / scene_size.h.max(1) as f32,
                ),
            ),
            Uniform::new(crate::window::GAUSSIAN_DIR_UNIFORM, (1.0f32, 0.0f32)),
            Uniform::new(
                crate::window::GAUSSIAN_SIGMA_UNIFORM,
                sigma * scale.x as f32,
            ),
        ],
        1.0,
    )?;

    let (mut vertical, more) = ensure_blur_texture(renderer, slot, 1, size)?;
    allocations += more;
    let horizontal_size = horizontal.size();
    draw_blur_texture(
        renderer,
        &mut vertical,
        &horizontal,
        Rectangle::from_size(horizontal_size.to_f64()),
        size,
        Some(program),
        &[
            Uniform::new(
                crate::window::BLUR_TEXEL_UNIFORM,
                (
                    1.0f32 / horizontal_size.w.max(1) as f32,
                    1.0f32 / horizontal_size.h.max(1) as f32,
                ),
            ),
            Uniform::new(crate::window::GAUSSIAN_DIR_UNIFORM, (0.0f32, 1.0f32)),
            Uniform::new(
                crate::window::GAUSSIAN_SIGMA_UNIFORM,
                sigma * scale.y as f32,
            ),
        ],
        1.0,
    )?;

    let Some(sharp) = spec.sharp_fraction else {
        slot.truncate(2);
        return Ok((vertical, allocations));
    };
    let sharp = sharp.clamp(0.0, 1.0);
    if sharp <= 0.0 {
        slot.truncate(2);
        return Ok((vertical, allocations));
    }

    let (mut composite, more) = ensure_blur_texture(renderer, slot, 2, size)?;
    allocations += more;
    // The measured mixture is `sharp * scene + (1 - sharp) * blurred`.
    // Paint the sharp scene opaque, then the blurred result with alpha
    // `1 - sharp`; source-over produces exactly that convex mix.
    draw_blur_texture(
        renderer,
        &mut composite,
        scene,
        scene_src,
        size,
        None,
        &[],
        1.0,
    )?;
    if sharp < 1.0 {
        draw_blur_texture(
            renderer,
            &mut composite,
            &vertical,
            Rectangle::from_size(vertical.size().to_f64()),
            size,
            None,
            &[],
            1.0 - sharp,
        )?;
    }
    slot.truncate(3);
    Ok((composite, allocations))
}

/// Reuse or allocate one panel-sized blur texture in `slot[index]`.
fn ensure_blur_texture<R>(
    renderer: &mut R,
    slot: &mut Vec<GlesTexture>,
    index: usize,
    size: BufferSize<i32, BufferCoord>,
) -> Result<(GlesTexture, u64), String>
where
    R: smithay::backend::renderer::Offscreen<GlesTexture>,
    R::Error: std::fmt::Debug,
{
    let mut allocations = 0u64;
    while slot.len() <= index {
        slot.push(
            renderer
                .create_buffer(Fourcc::Abgr8888, size)
                .map_err(|err| format!("blur buffer failed: {err}"))?,
        );
        allocations += 1;
    }
    if slot[index].size() != size {
        slot[index] = renderer
            .create_buffer(Fourcc::Abgr8888, size)
            .map_err(|err| format!("blur buffer failed: {err}"))?;
        allocations += 1;
    }
    Ok((slot[index].clone(), allocations))
}

/// Draw one source texture slice into a target blur texture.
fn draw_blur_texture<R>(
    renderer: &mut R,
    target_texture: &mut GlesTexture,
    source: &GlesTexture,
    source_rect: Rectangle<f64, BufferCoord>,
    size: BufferSize<i32, BufferCoord>,
    program: Option<&GlesTexProgram>,
    uniforms: &[smithay::backend::renderer::gles::Uniform<'_>],
    alpha: f32,
) -> Result<(), String>
where
    R: TextureRenderer + smithay::backend::renderer::Bind<GlesTexture>,
    R::Error: std::fmt::Debug,
{
    use smithay::utils::Transform;

    let dst = Rectangle::from_size(Size::<i32, Physical>::from((size.w, size.h)));
    let mut target = renderer
        .bind(target_texture)
        .map_err(|err| format!("blur bind failed: {err}"))?;
    let mut frame = renderer
        .render(&mut target, dst.size, Transform::Normal)
        .map_err(|err| format!("blur render failed: {err}"))?;
    R::draw_texture_from_to(
        &mut frame,
        source,
        source_rect,
        dst,
        &[dst],
        program,
        uniforms,
        alpha,
    )
    .map_err(|err| format!("blur draw failed: {err:?}"))?;
    Ok(())
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
