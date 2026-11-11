// SPDX-License-Identifier: MIT
//! Off-loop presentation for the nested backend.
//!
//! `eglSwapBuffers` on a winit/Wayland surface waits for the host compositor's
//! frame callback. When the host stops presenting the nested window (the user
//! minimizes it, switches workspace, or another window fully covers it) that
//! wait never returns. A main-thread swap then starves everything the session
//! loop owns: Wayland client dispatch, synthetic input, and `SIGUSR1`.
//!
//! This module moves only the swap to a dedicated thread. The compositor keeps
//! the EGL context for drawing; before handing a frame over it releases the
//! context with `eglMakeCurrent(NO_SURFACE, NO_SURFACE, NO_CONTEXT)`. The
//! presenter makes the same context current on the same surface, swaps, and
//! releases it again. A calloop ping wakes the session loop when the swap
//! completes, so the main loop can spend the wait dispatching instead of
//! blocking inside the driver.
//!
//! The presenter owns no compositor state and no renderer: it only holds the
//! raw EGL display/context handles and the per-frame surface handle. That keeps
//! graphics-context ownership explicit and testable.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use smithay::backend::egl::ffi;
use smithay::utils::{Clock, Monotonic};

/// Which EGL damage extension the display supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageMode {
    Khr,
    Ext,
    None,
}

impl DamageMode {
    /// Match Smithay's `DamageSupport`: buffer age plus a damage extension.
    pub fn from_extensions(extensions: &[String]) -> Self {
        let has = |name: &str| extensions.iter().any(|extension| extension == name);
        if !has("EGL_EXT_buffer_age") {
            return DamageMode::None;
        }
        if has("EGL_KHR_swap_buffers_with_damage") {
            DamageMode::Khr
        } else if has("EGL_EXT_swap_buffers_with_damage") {
            DamageMode::Ext
        } else {
            DamageMode::None
        }
    }
}

/// Why a swap failed. `BadSurface` is recoverable: Smithay recreates the EGL
/// surface on the next bind, so the session keeps running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentError {
    BadSurface,
    Other(String),
}

impl std::fmt::Display for PresentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresentError::BadSurface => f.write_str("EGL_BAD_SURFACE"),
            PresentError::Other(message) => f.write_str(message),
        }
    }
}

/// A one-shot fault injected by the synthetic-input test harness. Production
/// runs never set this; it lets the lifecycle test exercise the recovery paths
/// without provoking a real driver error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentFault {
    BadSurface,
    Other,
}

/// One frame handed to the presenter thread.
pub struct PresentRequest {
    pub frame: u64,
    /// The EGL surface handle as an integer. Raw pointers are not `Send`.
    pub surface: usize,
    /// Owned damage rectangles in EGL (bottom-left origin) coordinates.
    pub damage: Option<Vec<[i32; 4]>>,
    /// Test-only fault injection; consumed by the presenter.
    pub fault: Option<PresentFault>,
}

/// The result of one swap, delivered back to the session loop.
pub struct PresentDone {
    pub frame: u64,
    /// Time the presenter spent inside `eglSwapBuffers`, including the host
    /// frame-callback wait.
    pub duration: Duration,
    /// Time spent making the context current on the presenter thread.
    pub make_current: Duration,
    /// Time spent inside `eglSwapBuffers` (the host wait).
    pub swap: Duration,
    /// Monotonic presentation time sampled immediately after the swap.
    pub presented_at: Duration,
    /// Wall-clock completion time, used for the frame-time sample.
    pub completed: Instant,
    pub result: Result<(), PresentError>,
}

/// The session-loop handle to the presenter thread.
pub struct Presenter {
    tx: Option<Sender<PresentRequest>>,
    rx: Receiver<PresentDone>,
    join: Option<JoinHandle<()>>,
}

impl Drop for Presenter {
    fn drop(&mut self) {
        // Safety net for paths that drop the backend without an explicit
        // `stop()`. The nested backend stops the presenter before dropping the
        // EGL objects; if this one is still blocked, the caller must keep the
        // EGL objects alive (the nested shutdown leaks them in that case).
        if self.join.is_some() && !self.stop() {
            eprintln!(
                "dragonfruit-compositor: present thread still blocked while dropping \
                 the presenter; the EGL backend must outlive it"
            );
        }
    }
}

impl Presenter {
    /// Start the presenter. `display`/`context` are raw EGL handles; the
    /// caller keeps the owning Smithay objects alive until [`Self::stop`].
    pub fn spawn(
        display: usize,
        context: usize,
        damage: DamageMode,
        ping: calloop::ping::Ping,
    ) -> Result<Self, String> {
        let (tx, requests) = mpsc::channel();
        let (done_tx, rx) = mpsc::channel();
        let join = thread::Builder::new()
            .name("dragonfruit-present".into())
            .spawn(move || present_loop(display, context, damage, requests, done_tx, ping))
            .map_err(|error| format!("failed to start present thread: {error}"))?;
        Ok(Self {
            tx: Some(tx),
            rx,
            join: Some(join),
        })
    }

    /// Hand one frame to the presenter. The caller must have released the EGL
    /// context and must not touch the surface until the matching
    /// [`PresentDone`] arrives.
    pub fn submit(&self, request: PresentRequest) -> Result<(), String> {
        self.tx
            .as_ref()
            .ok_or_else(|| "presenter is stopped".to_owned())?
            .send(request)
            .map_err(|_| "present thread exited".to_owned())
    }

    /// Non-blocking completion poll.
    pub fn try_recv(&self) -> Option<PresentDone> {
        self.rx.try_recv().ok()
    }

    /// Stop the worker. Returns `true` when the thread exited (and was joined).
    /// `false` means it is still blocked in the host swap; the caller must keep
    /// the EGL objects alive (leak them) rather than destroy objects the
    /// worker may still use.
    ///
    /// This is a **process-exit** path, not an in-process recovery primitive.
    /// Recovery from a bad surface is a synchronous rebind/retry on the
    /// session loop while the presenter is idle. Calling `stop` during a live
    /// session would orphan the worker and leave its EGL handles unowned.
    pub fn stop(&mut self) -> bool {
        self.tx.take();
        let Some(join) = self.join.take() else {
            return true;
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let now = Instant::now();
            if now >= deadline {
                // Detach: the worker is stuck in eglSwapBuffers.
                return false;
            }
            match self.rx.recv_timeout(deadline - now) {
                Ok(_) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    let _ = join.join();
                    return true;
                }
                Err(RecvTimeoutError::Timeout) => return false,
            }
        }
    }
}

fn present_loop(
    display: usize,
    context: usize,
    damage: DamageMode,
    requests: Receiver<PresentRequest>,
    done: Sender<PresentDone>,
    ping: calloop::ping::Ping,
) {
    let clock = Clock::<Monotonic>::new();
    while let Ok(request) = requests.recv() {
        let start = Instant::now();
        let outcome = match request.fault {
            Some(fault) => SwapOutcome {
                result: Err(match fault {
                    PresentFault::BadSurface => PresentError::BadSurface,
                    PresentFault::Other => PresentError::Other("injected present fault".to_owned()),
                }),
                make_current: Duration::ZERO,
                swap: Duration::ZERO,
            },
            None => swap(
                display,
                context,
                request.surface,
                request.damage.as_deref(),
                damage,
            ),
        };
        let duration = start.elapsed();
        let completed = Instant::now();
        let presented_at = clock.now().into();
        if done
            .send(PresentDone {
                frame: request.frame,
                duration,
                make_current: outcome.make_current,
                swap: outcome.swap,
                presented_at,
                completed,
                result: outcome.result,
            })
            .is_err()
        {
            break;
        }
        // Wake the session loop: it may be blocked in poll with no other
        // source pending (the host is the only thing that just changed).
        ping.ping();
    }
}

struct SwapOutcome {
    result: Result<(), PresentError>,
    make_current: Duration,
    swap: Duration,
}

fn swap(
    display: usize,
    context: usize,
    surface: usize,
    damage: Option<&[[i32; 4]]>,
    mode: DamageMode,
) -> SwapOutcome {
    // SAFETY: the caller guarantees the EGL display, context, and surface are
    // alive for the lifetime of the presenter, and that the context is not
    // current on another thread when this request runs.
    unsafe {
        let display = display as ffi::egl::types::EGLDisplay;
        let context = context as ffi::egl::types::EGLContext;
        let surface = surface as ffi::egl::types::EGLSurface;
        let make_current_start = Instant::now();
        if ffi::egl::MakeCurrent(display, surface, surface, context) == ffi::egl::FALSE {
            let error = ffi::egl::GetError();
            return SwapOutcome {
                result: Err(classify(error, "eglMakeCurrent")),
                make_current: make_current_start.elapsed(),
                swap: Duration::ZERO,
            };
        }
        let make_current = make_current_start.elapsed();
        let swap_start = Instant::now();
        let swapped = match (damage, mode) {
            (Some(rects), DamageMode::Khr) if !rects.is_empty() => {
                ffi::egl::SwapBuffersWithDamageKHR(
                    display,
                    surface,
                    rects.as_ptr() as *mut ffi::egl::types::EGLint,
                    rects.len() as ffi::egl::types::EGLint,
                )
            }
            (Some(rects), DamageMode::Ext) if !rects.is_empty() => {
                ffi::egl::SwapBuffersWithDamageEXT(
                    display,
                    surface,
                    rects.as_ptr() as *mut ffi::egl::types::EGLint,
                    rects.len() as ffi::egl::types::EGLint,
                )
            }
            _ => ffi::egl::SwapBuffers(display, surface),
        };
        let error = if swapped == ffi::egl::FALSE {
            ffi::egl::GetError()
        } else {
            0
        };
        // Release the context even when the swap failed, so the main thread can
        // make it current for the next frame.
        let _ = ffi::egl::MakeCurrent(
            display,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_CONTEXT,
        );
        let swap = swap_start.elapsed();
        if swapped == ffi::egl::FALSE {
            return SwapOutcome {
                result: Err(classify(error, "eglSwapBuffers")),
                make_current,
                swap,
            };
        }
        SwapOutcome {
            result: Ok(()),
            make_current,
            swap,
        }
    }
}

fn classify(error: ffi::egl::types::EGLint, call: &str) -> PresentError {
    // EGL_BAD_SURFACE is the one error Smithay can recover from by recreating
    // the surface; everything else is reported as-is.
    if error as u32 == ffi::egl::BAD_SURFACE {
        PresentError::BadSurface
    } else {
        PresentError::Other(format!("{call} failed (EGL error 0x{error:x})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_mode_requires_buffer_age() {
        assert_eq!(
            DamageMode::from_extensions(&["EGL_KHR_swap_buffers_with_damage".to_owned()]),
            DamageMode::None
        );
        assert_eq!(
            DamageMode::from_extensions(&[
                "EGL_EXT_buffer_age".to_owned(),
                "EGL_KHR_swap_buffers_with_damage".to_owned(),
            ]),
            DamageMode::Khr
        );
        assert_eq!(
            DamageMode::from_extensions(&[
                "EGL_EXT_buffer_age".to_owned(),
                "EGL_EXT_swap_buffers_with_damage".to_owned(),
            ]),
            DamageMode::Ext
        );
        assert_eq!(
            DamageMode::from_extensions(&["EGL_EXT_buffer_age".to_owned()]),
            DamageMode::None
        );
    }

    #[test]
    fn error_classification_marks_bad_surface_recoverable() {
        assert_eq!(
            classify(ffi::egl::BAD_SURFACE as i32, "eglSwapBuffers"),
            PresentError::BadSurface
        );
        assert!(matches!(
            classify(ffi::egl::CONTEXT_LOST as i32, "eglSwapBuffers"),
            PresentError::Other(_)
        ));
    }

    #[test]
    fn idle_presenter_stops_cleanly() {
        // No request is ever submitted, so the worker only exercises the
        // channel/join lifecycle; the raw EGL handles are never dereferenced.
        let (ping, _source) = calloop::ping::make_ping().expect("ping");
        let mut presenter = Presenter::spawn(0, 0, DamageMode::None, ping).expect("presenter");
        assert!(presenter.stop());
    }
}
