// SPDX-License-Identifier: MIT
//! Opt-in GPU timer queries for the presentation benchmark (T-21.4b).
//!
//! `DRAGONFRUIT_GPU_TIMING=1` brackets the scene pass, each panel's blur
//! chain, and the final output composite with `GL_TIME_ELAPSED` queries. The
//! results are read asynchronously (never a pipeline stall) and summarised as
//! p50/p95/p99 in the `gpu stats` line.
//!
//! Timer queries are an extension on GLES. When neither
//! `GL_EXT_disjoint_timer_query` nor `GL_ARB_timer_query` is present (or the
//! backend is headless/software), the counters report
//! `gpu_time=unavailable` with the reason. They are never fabricated from CPU
//! timings.
use std::borrow::BorrowMut;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::os::raw::c_void;

use smithay::backend::egl::ffi::egl::types::EGLSurface;
use smithay::backend::renderer::gles::{ffi, GlesRenderer};
use smithay::backend::renderer::glow::GlowRenderer;

/// Environment flag that turns the timer queries on.
pub const ENV_GPU_TIMING: &str = "DRAGONFRUIT_GPU_TIMING";

/// Queries kept in flight. A query result usually lands one or two frames
/// after it is issued; a bounded pool keeps a slow GPU from growing the ring.
const QUERY_POOL: usize = 24;

const GL_TIME_ELAPSED: u32 = 0x88BF;
const GL_QUERY_RESULT: u32 = 0x8866;
const GL_QUERY_RESULT_AVAILABLE: u32 = 0x8867;
const GL_GPU_DISJOINT: u32 = 0x8FBB;

type GlEnum = u32;
type GlUint = u32;
type GlSizei = i32;

type GenQueriesFn = unsafe extern "system" fn(GlSizei, *mut GlUint);
type BeginQueryFn = unsafe extern "system" fn(GlEnum, GlUint);
type EndQueryFn = unsafe extern "system" fn(GlEnum);
type GetQueryObjectFn = unsafe extern "system" fn(GlUint, GlEnum, *mut GlUint);

/// One timed render pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPass {
    /// The offscreen live-scene texture (wallpaper, desktop, windows).
    Scene,
    /// One panel's Kawase blur chain.
    Blur,
    /// The final output composite. The liquid-glass compose runs inside this
    /// pass; it is not a separately dispatchable GL pass.
    Composite,
}

impl GpuPass {
    /// Every pass, in the order the stats line prints them.
    pub const ALL: [GpuPass; 3] = [GpuPass::Scene, GpuPass::Blur, GpuPass::Composite];

    /// Stable field prefix (`scene_`, `blur_`, `composite_`).
    pub const fn name(self) -> &'static str {
        match self {
            GpuPass::Scene => "scene",
            GpuPass::Blur => "blur",
            GpuPass::Composite => "composite",
        }
    }

    const fn index(self) -> usize {
        match self {
            GpuPass::Scene => 0,
            GpuPass::Blur => 1,
            GpuPass::Composite => 2,
        }
    }
}

/// One completed GPU timer sample, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuSample {
    pub pass: GpuPass,
    pub nanos: u64,
}

/// Aggregated GPU timer samples for `dump_stats` (T-21.4b).
#[derive(Debug)]
pub struct GpuStats {
    enabled: bool,
    available: bool,
    reason: Option<String>,
    samples: [VecDeque<u32>; 3],
    totals: [u64; 3],
}

impl Default for GpuStats {
    fn default() -> Self {
        Self {
            enabled: false,
            available: false,
            reason: None,
            samples: std::array::from_fn(|_| VecDeque::new()),
            totals: [0; 3],
        }
    }
}

impl GpuStats {
    /// Samples kept per pass. Matches the frame-time window.
    pub const CAPACITY: usize = 4096;

    /// Publish the timer's current support state. Called once per rendered
    /// frame; the reason string is only replaced when it changes.
    pub fn set_status(&mut self, enabled: bool, available: bool, reason: &str) {
        self.enabled = enabled;
        self.available = available;
        let reason = if reason.is_empty() || reason == "none" {
            None
        } else {
            Some(reason.to_owned())
        };
        if self.reason != reason {
            self.reason = reason;
        }
    }

    /// Record one completed query. Nanoseconds are converted to microseconds
    /// for the trace; the percentile window matches `frame timing`.
    pub fn record(&mut self, pass: GpuPass, nanos: u64) {
        let micros = (nanos / 1_000).min(u64::from(u32::MAX)) as u32;
        let index = pass.index();
        self.totals[index] = self.totals[index].saturating_add(1);
        let ring = &mut self.samples[index];
        if ring.len() == Self::CAPACITY {
            ring.pop_front();
        }
        ring.push_back(micros);
    }

    /// One greppable line for `dump_stats`. `gpu_time=unavailable` is the
    /// honest result when the extension is absent or the feature is off.
    pub fn summary(&self) -> String {
        let gpu_time = if self.enabled && self.available {
            "available"
        } else {
            "unavailable"
        };
        let reason = match self.reason.as_deref() {
            Some(reason) => reason,
            None if self.enabled && self.available => "none",
            None if self.enabled => "uninitialised",
            None => "disabled",
        };
        let mut line = format!(
            "enabled={} available={} gpu_time={gpu_time} reason={reason}",
            self.enabled as u32, self.available as u32
        );
        for pass in GpuPass::ALL {
            let index = pass.index();
            let mut values: Vec<u32> = self.samples[index].iter().copied().collect();
            values.sort_unstable();
            let name = pass.name();
            line.push_str(&format!(
                " {name}_samples={} {name}_total={} {name}_p50_us={} \
                 {name}_p95_us={} {name}_p99_us={} {name}_max_us={}",
                values.len(),
                self.totals[index],
                percentile(&values, 50),
                percentile(&values, 95),
                percentile(&values, 99),
                values.last().copied().unwrap_or(0),
            ));
        }
        line
    }
}

/// The renderer types whose GL context owns the timer queries.
pub trait GpuTimerTarget {
    /// The underlying GLES renderer.
    fn gles_mut(&mut self) -> &mut GlesRenderer;
}

impl GpuTimerTarget for GlesRenderer {
    fn gles_mut(&mut self) -> &mut GlesRenderer {
        self
    }
}

impl GpuTimerTarget for GlowRenderer {
    fn gles_mut(&mut self) -> &mut GlesRenderer {
        BorrowMut::borrow_mut(self)
    }
}

/// One query object that has been issued but whose result is not read yet.
#[derive(Debug, Clone, Copy)]
struct PendingQuery {
    query: u32,
    pass: GpuPass,
}

/// The query entry points. GLES exposes the `EXT` names; desktop GL exposes
/// the core names behind `GL_ARB_timer_query`.
#[derive(Debug, Clone, Copy)]
struct ExtApi {
    gen: GenQueriesFn,
    begin: BeginQueryFn,
    end: EndQueryFn,
    get: GetQueryObjectFn,
}

#[derive(Debug, Clone, Copy)]
enum Api {
    Ext(ExtApi),
    Core,
}

impl Api {
    fn has_disjoint(self) -> bool {
        matches!(self, Api::Ext(_))
    }

    unsafe fn gen(self, gl: &ffi::Gles2, count: GlSizei, ids: *mut GlUint) {
        match self {
            Api::Ext(api) => (api.gen)(count, ids),
            Api::Core => gl.GenQueries(count, ids),
        }
    }

    unsafe fn begin(self, gl: &ffi::Gles2, target: GlEnum, query: GlUint) {
        match self {
            Api::Ext(api) => (api.begin)(target, query),
            Api::Core => gl.BeginQuery(target, query),
        }
    }

    unsafe fn end(self, gl: &ffi::Gles2, target: GlEnum) {
        match self {
            Api::Ext(api) => (api.end)(target),
            Api::Core => gl.EndQuery(target),
        }
    }

    unsafe fn result_available(self, gl: &ffi::Gles2, query: GlUint) -> bool {
        let mut available = 0u32;
        match self {
            Api::Ext(api) => (api.get)(query, GL_QUERY_RESULT_AVAILABLE, &mut available),
            Api::Core => gl.GetQueryObjectuiv(query, GL_QUERY_RESULT_AVAILABLE, &mut available),
        }
        available != 0
    }

    unsafe fn result(self, gl: &ffi::Gles2, query: GlUint) -> u32 {
        let mut value = 0u32;
        match self {
            Api::Ext(api) => (api.get)(query, GL_QUERY_RESULT, &mut value),
            Api::Core => gl.GetQueryObjectuiv(query, GL_QUERY_RESULT, &mut value),
        }
        value
    }
}

/// Per-renderer timer-query state.
#[derive(Debug)]
pub struct GpuTimer {
    enabled: bool,
    init_attempted: bool,
    api: Option<Api>,
    reason: Option<String>,
    free: Vec<u32>,
    pending: VecDeque<PendingQuery>,
    open: Option<PendingQuery>,
}

impl GpuTimer {
    /// Read the opt-in flag once. The queries are only created on the first
    /// rendered frame, on that frame's GL context.
    pub fn new() -> Self {
        let enabled = std::env::var_os(ENV_GPU_TIMING)
            .map(|value| value != "0")
            .unwrap_or(false);
        Self {
            enabled,
            init_attempted: false,
            api: None,
            reason: None,
            free: Vec::new(),
            pending: VecDeque::new(),
            open: None,
        }
    }

    /// Whether a query API was successfully initialised.
    pub fn available(&self) -> bool {
        self.enabled && self.api.is_some()
    }

    /// The `(enabled, available, reason)` triple published to [`GpuStats`].
    pub fn status(&self) -> (bool, bool, &str) {
        let available = self.available();
        let reason = if !self.enabled {
            "disabled"
        } else if available {
            "none"
        } else {
            self.reason.as_deref().unwrap_or("uninitialised")
        };
        (self.enabled, available, reason)
    }

    /// Start a timer query for `pass`. Nested queries and an exhausted pool
    /// are skipped rather than stalling the frame.
    pub fn begin<T: GpuTimerTarget + ?Sized>(&mut self, target: &mut T, pass: GpuPass) {
        if !self.enabled || self.open.is_some() {
            return;
        }
        self.ensure_initialised(target.gles_mut());
        let Some(api) = self.api else { return };
        let Some(query) = self.free.pop() else { return };
        let result = with_gl(target.gles_mut(), |gl| unsafe {
            api.begin(gl, GL_TIME_ELAPSED, query);
        });
        match result {
            Ok(()) => self.open = Some(PendingQuery { query, pass }),
            Err(_) => {
                self.free.push(query);
                self.mark_unavailable("context-error");
            }
        }
    }

    /// End the open query and queue it for asynchronous result collection.
    pub fn end<T: GpuTimerTarget + ?Sized>(&mut self, target: &mut T) {
        let Some(pending) = self.open.take() else {
            return;
        };
        let Some(api) = self.api else {
            self.free.push(pending.query);
            return;
        };
        let result = with_gl(target.gles_mut(), |gl| unsafe {
            api.end(gl, GL_TIME_ELAPSED);
        });
        if result.is_ok() {
            self.pending.push_back(pending);
        } else {
            self.free.push(pending.query);
            self.mark_unavailable("context-error");
        }
    }

    /// Collect every query whose result is ready. Never waits: a query that
    /// is still in flight stays queued for a later frame.
    pub fn poll<T: GpuTimerTarget + ?Sized>(&mut self, target: &mut T) -> Vec<GpuSample> {
        if !self.enabled {
            return Vec::new();
        }
        let Some(api) = self.api else {
            return Vec::new();
        };
        let mut samples = Vec::new();
        let result = with_gl(target.gles_mut(), |gl| unsafe {
            let mut disjoint = 0i32;
            if api.has_disjoint() {
                gl.GetIntegerv(GL_GPU_DISJOINT, &mut disjoint);
            }
            if disjoint != 0 {
                // The GPU was reset or the timer was disjoint: discard every
                // in-flight result instead of reporting a bogus interval.
                while let Some(pending) = self.pending.pop_front() {
                    self.free.push(pending.query);
                }
                return;
            }
            while let Some(pending) = self.pending.front().copied() {
                if !api.result_available(gl, pending.query) {
                    break;
                }
                let nanos = u64::from(api.result(gl, pending.query));
                self.free.push(pending.query);
                self.pending.pop_front();
                samples.push(GpuSample {
                    pass: pending.pass,
                    nanos,
                });
            }
        });
        if result.is_err() {
            self.mark_unavailable("context-error");
            return Vec::new();
        }
        samples
    }

    fn ensure_initialised(&mut self, gles: &mut GlesRenderer) {
        if self.init_attempted {
            return;
        }
        self.init_attempted = true;
        let outcome = with_gl(gles, |gl| unsafe {
            let Some(api) = detect_api(gl) else {
                return Err("no-timer-query-extension");
            };
            let mut ids = vec![0u32; QUERY_POOL];
            api.gen(gl, QUERY_POOL as GlSizei, ids.as_mut_ptr());
            ids.retain(|&id| id != 0);
            if ids.is_empty() {
                return Err("query-allocation-failed");
            }
            Ok((api, ids))
        });
        match outcome {
            Ok(Ok((api, ids))) => {
                self.api = Some(api);
                self.free = ids;
                self.reason = None;
            }
            Ok(Err(reason)) => self.reason = Some(reason.to_owned()),
            Err(_) => self.reason = Some("context-error".to_owned()),
        }
    }

    fn mark_unavailable(&mut self, reason: &str) {
        self.api = None;
        self.reason = Some(reason.to_owned());
        self.open = None;
        self.pending.clear();
        self.free.clear();
    }
}

impl Default for GpuTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Run `func` in the renderer's GL context, then restore the backend's EGL
/// window surface (nested). `GlesRenderer::with_context` makes the context
/// current with no draw surface; the winit backend does not rebind it before
/// `swap_buffers`, so leaving it unbound turns the next submit into a
/// context-loss error.
fn with_gl<R>(gles: &mut GlesRenderer, func: impl FnOnce(&ffi::Gles2) -> R) -> Result<R, ()> {
    // Capture the live draw surface before `with_context` detaches it. Reading
    // it here (rather than passing a handle from the caller) stays correct
    // when the nested backend resizes/recreates its surface during `bind()`.
    let surface = unsafe {
        smithay::backend::egl::ffi::egl::GetCurrentSurface(
            smithay::backend::egl::ffi::egl::DRAW as _,
        )
    };
    let result = gles.with_context(func).map_err(|_| ());
    if surface != smithay::backend::egl::ffi::egl::NO_SURFACE {
        restore_surface(gles, surface);
    }
    result
}

/// Rebind the EGL window surface that `GlesRenderer::with_context` detached.
fn restore_surface(gles: &GlesRenderer, surface: EGLSurface) {
    let context = gles.egl_context();
    let display = context.display().get_display_handle();
    unsafe {
        smithay::backend::egl::ffi::egl::MakeCurrent(
            **display,
            surface,
            surface,
            context.get_context_handle(),
        );
    }
}

/// Detect a usable timer-query API on the current GL context.
unsafe fn detect_api(gl: &ffi::Gles2) -> Option<Api> {
    let extensions = extensions(gl);
    let has = |name: &str| extensions.iter().any(|extension| extension == name);
    if has("GL_EXT_disjoint_timer_query") {
        return load_ext_api().map(Api::Ext);
    }
    if has("GL_ARB_timer_query") {
        return Some(Api::Core);
    }
    None
}

/// Read the GL extension list without assuming a GLES version. GL 3.0+ uses
/// the indexed form; older GLES exposes the space-separated string.
unsafe fn extensions(gl: &ffi::Gles2) -> Vec<String> {
    let mut major = 0i32;
    gl.GetIntegerv(ffi::MAJOR_VERSION, &mut major);
    if major >= 3 {
        let mut count = 0i32;
        gl.GetIntegerv(ffi::NUM_EXTENSIONS, &mut count);
        let mut names = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count.max(0) as u32 {
            let pointer = gl.GetStringi(ffi::EXTENSIONS, index);
            if let Some(name) = gl_string(pointer) {
                names.push(name);
            }
        }
        return names;
    }
    gl_string(gl.GetString(ffi::EXTENSIONS))
        .map(|value| value.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

unsafe fn gl_string(pointer: *const ffi::types::GLubyte) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    CStr::from_ptr(pointer as *const _)
        .to_str()
        .ok()
        .map(str::to_owned)
}

/// Load the `GL_EXT_disjoint_timer_query` entry points. `eglGetProcAddress`
/// is the only way to reach the `EXT` names; the core `Gles2` table does not
/// export them.
unsafe fn load_ext_api() -> Option<ExtApi> {
    Some(ExtApi {
        gen: load_proc("glGenQueriesEXT")?,
        begin: load_proc("glBeginQueryEXT")?,
        end: load_proc("glEndQueryEXT")?,
        get: load_proc("glGetQueryObjectuivEXT")?,
    })
}

unsafe fn load_proc<T: Copy>(name: &str) -> Option<T> {
    let pointer = smithay::backend::egl::get_proc_address(name);
    if pointer.is_null() {
        return None;
    }
    // The caller names the exact signature for the requested symbol; the
    // pointer comes from the active EGL context's driver.
    Some(std::mem::transmute_copy::<*const c_void, T>(&pointer))
}

fn percentile(sorted: &[u32], percent: usize) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let last = sorted.len() - 1;
    let rank = (percent * last) / 100;
    sorted[rank.min(last)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_stats_reports_unavailable_without_the_extension() {
        let mut stats = GpuStats::default();
        stats.set_status(true, false, "no-timer-query-extension");
        let summary = stats.summary();
        assert!(summary.contains("gpu_time=unavailable"));
        assert!(summary.contains("enabled=1"));
        assert!(summary.contains("available=0"));
        assert!(summary.contains("reason=no-timer-query-extension"));
        // Every pass stays parseable even before a sample exists.
        for pass in GpuPass::ALL {
            assert!(summary.contains(&format!("{}_samples=0", pass.name())));
            assert!(summary.contains(&format!("{}_p50_us=0", pass.name())));
        }
    }

    #[test]
    fn gpu_stats_are_windowed_and_percentile_based() {
        let mut stats = GpuStats::default();
        stats.set_status(true, true, "none");
        for micros in 1..=100u64 {
            stats.record(GpuPass::Scene, micros * 1_000);
        }
        stats.record(GpuPass::Blur, 250_000);
        stats.record(GpuPass::Composite, 500_000);

        let summary = stats.summary();
        assert!(summary.contains("gpu_time=available"));
        assert!(summary.contains("scene_samples=100"));
        assert!(summary.contains("scene_total=100"));
        assert!(summary.contains("scene_p50_us=50"));
        assert!(summary.contains("scene_p95_us=95"));
        assert!(summary.contains("scene_p99_us=99"));
        assert!(summary.contains("scene_max_us=100"));
        assert!(summary.contains("blur_p50_us=250"));
        assert!(summary.contains("composite_p50_us=500"));
    }

    #[test]
    fn gpu_stats_window_does_not_grow_after_capacity() {
        let mut stats = GpuStats::default();
        stats.set_status(true, true, "none");
        for _ in 0..GpuStats::CAPACITY {
            stats.record(GpuPass::Composite, 1_000);
        }
        for _ in 0..GpuStats::CAPACITY {
            stats.record(GpuPass::Composite, 2_000);
        }
        let summary = stats.summary();
        assert!(summary.contains(&format!("composite_samples={}", GpuStats::CAPACITY)));
        assert!(summary.contains(&format!("composite_total={}", GpuStats::CAPACITY * 2)));
        assert!(summary.contains("composite_p50_us=2"));
    }

    #[test]
    fn disabled_timer_is_unavailable_not_zero_gpu_time() {
        let timer = GpuTimer::new();
        // The test process does not set DRAGONFRUIT_GPU_TIMING, so the timer
        // must stay off and report a reason instead of a fabricated value.
        let (enabled, available, reason) = timer.status();
        assert!(!enabled);
        assert!(!available);
        assert_eq!(reason, "disabled");
        assert!(!timer.available());
    }
}
