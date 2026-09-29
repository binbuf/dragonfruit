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

use smithay::backend::egl::EGLDevice;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::utils::{
    Relocate, RelocateRenderElement, RescaleRenderElement,
};
use smithay::backend::renderer::glow::GlowRenderer;
use smithay::backend::renderer::{ImportAll, ImportDma, ImportMem, ImportMemWl};
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::output::{Mode, PhysicalProperties, Subpixel};
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use smithay::render_elements;
use smithay::wayland::presentation::Refresh;

use crate::backend::{add_output, add_seat_capabilities};
use crate::session::{run_session, BackendHooks};
use crate::{render, LOCKSTEP_VERSION};

// Custom elements for the nested output: shell chrome (menu bar, overlays)
// and the compositor-drawn SSD titlebars (T-01.1). One enum so both kinds
// can composite above the window space in a single render pass.
render_elements! {
    pub NestedOutputElements<R> where R: ImportAll + ImportMem + ImportMemWl;
    Chrome=WaylandSurfaceRenderElement<R>,
    Decoration=SolidColorRenderElement,
    // A window mid-appear: scaled about its target and translated from the
    // Dock tile origin (T-02.1b).
    Appear=RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>,
    // The per-Space wallpaper (T-05.4): behind the windows.
    Wallpaper=crate::render::WallpaperRenderElement<R>,
}

// The frame element list after the magnifier pass (T-16.6b): the untransformed
// elements (`Plain`) or each element scaled/translated about the output centre
// (`Magnified`). One enum so the live frame and the offscreen capture render
// the same list.
render_elements! {
    pub NestedFrameElements<R> where R: ImportAll + ImportMem + ImportMemWl;
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
                            if let Some(output) = state.space.outputs().next().cloned() {
                                let mode = Mode {
                                    size,
                                    refresh: 60_000,
                                };
                                output.change_current_state(Some(mode), None, None, None);
                                output.set_preferred(mode);
                                current_scale = output.current_scale().fractional_scale();
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

                *shared_init.borrow_mut() = Some(NestedData {
                    backend,
                    damage_tracker: OutputDamageTracker::new(
                        window_size,
                        smithay::utils::Scale::from(scale),
                        smithay::utils::Transform::Flipped180,
                    ),
                    tracker_scale: scale,
                    tracker_size: (window_size.w, window_size.h),
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

struct NestedData {
    backend: WinitGraphicsBackend<GlowRenderer>,
    damage_tracker: OutputDamageTracker,
    /// The output scale the tracker was built with. The damage tracker's
    /// static mode supplies the scale every render element's geometry is
    /// resolved with, so it must track the output scale: at a fractional
    /// scale a stale `1.0` sizes chrome and window elements in logical pixels
    /// while placing them in physical pixels (T-16.3b).
    tracker_scale: f64,
    /// The physical mode size the tracker was built with.
    tracker_size: (i32, i32),
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
    if !state.needs_redraw {
        return Ok(());
    }
    state.needs_redraw = false;
    // Open this frame's material pass, so the chrome backdrop is applied at
    // most once per output (T-04.2).
    state.begin_render_frame();

    let Some(output) = state.space.outputs().next().cloned() else {
        return Ok(());
    };

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
            // A locked session composites only the lock surface: it is
            // prepended so it sits above every window and chrome surface
            // (T-12.3a).
            let mut custom_elements: Vec<NestedOutputElements<GlowRenderer>> =
                crate::lock::lock_render_elements(renderer, state, &output, scale);
            // Chrome surfaces (menu bar, overlays) composite above the
            // window space (T-09); SSD titlebars composite above their own
            // client surface (T-01.1).
            custom_elements.extend(crate::render::chrome_render_elements(
                renderer, state, &output, scale,
            ));
            // Backdrop blur under the chrome (T-04.2): appended after the
            // chrome surfaces so it composites below them and in front of the
            // windows it stands in for.
            custom_elements.extend(
                crate::render::chrome_backdrop_render_elements(state, &output, scale)
                    .into_iter()
                    .map(NestedOutputElements::Decoration),
            );
            custom_elements.extend(
                crate::render::titlebar_render_elements(state, &output, scale)
                    .into_iter()
                    .map(NestedOutputElements::Decoration),
            );
            custom_elements.extend(
                crate::render::window_menu_render_elements(state, &output, scale)
                    .into_iter()
                    .map(NestedOutputElements::Decoration),
            );
            // Window surfaces composite below the chrome/titlebar custom
            // elements; the appear transform is applied per window (T-02.1b).
            custom_elements.extend(crate::render::window_render_elements(
                renderer, state, &output, scale,
            ));
            // Elevation-token-driven shadows composite below the window
            // surfaces (T-04.1a), so the ring beyond a window is visible.
            custom_elements.extend(
                crate::render::window_shadow_render_elements(state, &output, scale)
                    .into_iter()
                    .map(NestedOutputElements::Decoration),
            );
            // The Files-owned desktop layer (T-19.3) composites below every
            // window and above the per-Space wallpaper, so desktop icons sit
            // behind open windows.
            custom_elements.extend(crate::render::desktop_render_elements(
                renderer, state, &output, scale,
            ));
            // The per-Space wallpaper is the bottom-most layer (T-05.4): it
            // fills the clear color and slides with its Space during a switch.
            custom_elements.extend(
                crate::render::wallpaper_render_elements(renderer, state, &output, scale)
                    .into_iter()
                    .map(NestedOutputElements::Wallpaper),
            );
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
            data.damage_tracker.render_output(
                renderer,
                &mut framebuffer,
                age,
                &frame_elements,
                state.wallpaper_color_for(&output),
            )
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

            let now = state.clock.now();
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
            let x = rect.loc.x.clamp(0, size.w);
            let y = rect.loc.y.clamp(0, size.h);
            let width = rect.size.w.clamp(1, size.w - x) as u32;
            let height = rect.size.h.clamp(1, size.h - y) as u32;
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
