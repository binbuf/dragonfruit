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
use smithay::backend::renderer::glow::GlowRenderer;
use smithay::backend::renderer::{ImportAll, ImportDma, ImportMem, ImportMemWl};
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::desktop::space::render_output;
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
                    1.0,
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
                            if let Some(output) = state.space.outputs().next().cloned() {
                                let mode = Mode {
                                    size,
                                    refresh: 60_000,
                                };
                                output.change_current_state(Some(mode), None, None, None);
                                output.set_preferred(mode);
                            }
                            // The winit/EGL back buffer is bottom-up, so the
                            // damage tracker must render with Flipped180
                            // (smithay's `minimal.rs` winit example does the
                            // same). The output itself stays Normal so clients
                            // and input mapping are unaffected. The tracker's
                            // mode is static, so recreate it on resize.
                            if let Some(data) = shared_events.borrow_mut().as_mut() {
                                data.damage_tracker = OutputDamageTracker::new(
                                    size,
                                    smithay::utils::Scale::from(1.0),
                                    smithay::utils::Transform::Flipped180,
                                );
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
                        smithay::utils::Scale::from(1.0),
                        smithay::utils::Transform::Flipped180,
                    ),
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
}

fn render_frame(state: &mut crate::state::DfState, data: &mut NestedData) -> Result<(), String> {
    if !state.needs_redraw {
        return Ok(());
    }
    state.needs_redraw = false;

    let Some(output) = state.space.outputs().next().cloned() else {
        return Ok(());
    };

    let age = data.backend.buffer_age().unwrap_or(0);
    let render_result = match data.backend.bind() {
        Ok((renderer, mut framebuffer)) => {
            let scale = smithay::utils::Scale::from(output.current_scale().fractional_scale());
            // Chrome surfaces (menu bar, overlays) composite above the
            // window space (T-09); SSD titlebars composite above their own
            // client surface (T-01.1).
            let mut custom_elements: Vec<NestedOutputElements<GlowRenderer>> =
                crate::render::chrome_render_elements(renderer, state, &output, scale);
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
            render_output(
                &output,
                renderer,
                &mut framebuffer,
                1.0,
                age,
                [&state.space],
                &custom_elements,
                &mut data.damage_tracker,
                state.wallpaper_color_for(&output),
            )
        }
        Err(err) => return Err(format!("nested bind failed: {err}")),
    };

    let render_result = match render_result {
        Ok(result) => result,
        Err(err) => return Err(format!("nested render failed: {err:?}")),
    };

    match render_result.damage {
        Some(damage) => {
            // Partial-submit with the tracked damage region.
            if let Err(err) = data.backend.submit(Some(damage)) {
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
        }
        None => {
            // No damage, no render, no client wakeups (FR-2).
            state.stats.frames_skipped_no_damage += 1;
            if let Err(err) = data.backend.submit(None) {
                return Err(format!("nested submit failed: {err}"));
            }
        }
    }
    Ok(())
}
