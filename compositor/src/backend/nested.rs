// SPDX-License-Identifier: MIT OR Apache-2.0
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
use smithay::backend::renderer::glow::GlowRenderer;
use smithay::backend::renderer::{Color32F, ImportDma, ImportMemWl};
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::desktop::space::render_output;
use smithay::output::{Mode, PhysicalProperties, Subpixel};
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use smithay::wayland::presentation::Refresh;

use crate::backend::{add_output, add_seat_capabilities};
use crate::session::{run_session, BackendHooks};
use crate::{render, LOCKSTEP_VERSION};

/// Clear color: a flat dragonfruit tint; wallpaper rendering (per-Space)
/// arrives with the workspace model in T-05.
const CLEAR_COLOR: Color32F = Color32F::new(0.13, 0.05, 0.16, 1.0);

pub fn run(socket_name: &str) -> Result<(), String> {
    println!("dragonfruit-compositor: starting (backend=nested, lockstep-ipc=v{LOCKSTEP_VERSION})");

    let (mut backend, winit_loop) = winit::init::<GlowRenderer>().map_err(|e| {
        format!("failed to open a nested window (is there a Wayland session?): {e}")
    })?;
    backend.window().set_title("Dragonfruit");

    // Filled by the init hook; read by the render hook.
    let shared: Rc<RefCell<Option<NestedData>>> = Rc::new(RefCell::new(None));

    let shared_init = shared.clone();
    let shared_render = shared.clone();

    run_session(
        socket_name,
        BackendHooks {
            init: Box::new(move |state| {
                let output = add_output(
                    state,
                    "NESTED-1",
                    PhysicalProperties {
                        size: (0, 0).into(),
                        subpixel: Subpixel::Unknown,
                        make: "Dragonfruit".into(),
                        model: "Winit".into(),
                    },
                    Mode {
                        size: backend.window_size(),
                        refresh: 60_000,
                    },
                    (0, 0),
                    1.0,
                );
                add_seat_capabilities(state);

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
                    damage_tracker: OutputDamageTracker::from_output(&output),
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
    )
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
    let custom_elements: Vec<
        smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement<GlowRenderer>,
    > = Vec::new();
    let render_result = match data.backend.bind() {
        Ok((renderer, mut framebuffer)) => render_output(
            &output,
            renderer,
            &mut framebuffer,
            1.0,
            age,
            [&state.space],
            &custom_elements,
            &mut data.damage_tracker,
            CLEAR_COLOR,
        ),
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
