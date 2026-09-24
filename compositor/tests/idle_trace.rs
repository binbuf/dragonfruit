// SPDX-License-Identifier: MIT
//! T-02 FR-2 / T-03.1a acceptance: the idle trace and the animation frame
//! budget.
//!
//! The idle budget is "zero damage, zero client wakeups from our shell".
//! With no clients attached, the compositor must settle into a steady state
//! where nothing asks for a frame. `dump_stats` already emits the render
//! counters on SIGUSR1; this test turns that into a scripted assertion on
//! the headless backend (which needs no display):
//!
//! 1. start the compositor, let startup/Xwayland settle,
//! 2. sample the render counters (SIGUSR1),
//! 3. sit idle for the trace window, sample again,
//! 4. assert `frames_rendered`, `animation_frames_stepped`,
//!    `direct_scanouts` and `client_wakeups` did not advance — a
//!    self-inflicted redraw while idle is a bug (T-02 FR-2);
//! 5. drive the clock's calibration animation (`animate-dummy`) and assert
//!    the render-path frame count advanced by exactly the clock's
//!    `animation_frames_stepped` — one compositor frame per animation frame
//!    (T-03.1a);
//! 6. assert both counters are flat again once the animation settles (the
//!    timer is disarmed, so the idle desktop costs nothing).
//!
//! The trace window is the T-03.1a acceptance duration — **60 s** — and is
//! configurable through `DF_IDLE_TRACE_SECS` (`scripts/idle-trace.sh` runs
//! the 60 s acceptance; the in-suite default is shorter so `make cargo-test`
//! stays quick).
//!
//! `frames_skipped_no_damage` may advance by one per SIGUSR1 (the signal
//! itself wakes the loop and the render pass records the no-damage frame),
//! so only the *damage* counters are asserted to be flat.
//!
//! `client_wakeups` counts the frame-callback batches handed to mapped
//! windows (`crate::render::post_repaint*`); with no window attached it is
//! zero, and the mapped-idle-client half of the budget is asserted by
//! `shell_idle_trace.rs`.

use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const SIGUSR1: i32 = 10;
const SIGTERM: i32 = 15;

/// In-suite trace window. The 60 s acceptance run sets `DF_IDLE_TRACE_SECS`.
const DEFAULT_IDLE_TRACE_SECS: u64 = 10;

/// The idle trace window, from `DF_IDLE_TRACE_SECS` (1..=600 s).
fn idle_trace_window() -> Duration {
    let secs = std::env::var("DF_IDLE_TRACE_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_IDLE_TRACE_SECS)
        .clamp(1, 600);
    Duration::from_secs(secs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderStats {
    frames_rendered: u64,
    frames_skipped_no_damage: u64,
    direct_scanouts: u64,
    animation_frames_stepped: u64,
    client_wakeups: u64,
}

fn parse_stats(line: &str) -> Option<RenderStats> {
    let rest = line.split("frames_rendered=").nth(1)?;
    let mut parts = rest.split_whitespace();
    let frames_rendered = parts.next()?.parse().ok()?;
    let frames_skipped_no_damage = parts
        .next()?
        .strip_prefix("frames_skipped_no_damage=")?
        .parse()
        .ok()?;
    let direct_scanouts = parts
        .next()?
        .strip_prefix("direct_scanouts=")?
        .parse()
        .ok()?;
    let animation_frames_stepped = parts
        .next()?
        .strip_prefix("animation_frames_stepped=")?
        .parse()
        .ok()?;
    let client_wakeups = parts
        .next()?
        .strip_prefix("client_wakeups=")?
        .parse()
        .ok()?;
    Some(RenderStats {
        frames_rendered,
        frames_skipped_no_damage,
        direct_scanouts,
        animation_frames_stepped,
        client_wakeups,
    })
}

/// Assert the four idle-budget counters did not move across the window.
#[track_caller]
fn assert_idle_flat(first: &RenderStats, second: &RenderStats, window: Duration) {
    assert_eq!(
        second.frames_rendered,
        first.frames_rendered,
        "idle compositor rendered {} new frame(s) over {window:?}: {first:?} -> {second:?}",
        second.frames_rendered - first.frames_rendered,
    );
    assert_eq!(
        second.direct_scanouts, first.direct_scanouts,
        "headless must never direct-scanout: {first:?} -> {second:?}",
    );
    assert_eq!(
        second.animation_frames_stepped, first.animation_frames_stepped,
        "the animation clock ticked while idle: {first:?} -> {second:?}",
    );
    assert_eq!(
        second.client_wakeups, first.client_wakeups,
        "the compositor woke a client while idle: {first:?} -> {second:?}",
    );
}

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    synthetic_path: PathBuf,
    sender: UnixDatagram,
    sender_path: PathBuf,
    stats_rx: Receiver<RenderStats>,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // The reader thread ends when the pipe closes (child killed).
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(self.socket_path.with_extension("lock"));
        let _ = std::fs::remove_file(&self.synthetic_path);
        let _ = std::fs::remove_file(&self.sender_path);
    }
}

impl CompositorProcess {
    fn start(socket_name: &str) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let synthetic_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.synthetic"));
        let sender_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.sender"));
        let _ = std::fs::remove_file(&synthetic_path);
        let _ = std::fs::remove_file(&sender_path);

        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_SYNTHETIC_INPUT", &synthetic_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start compositor");
        let stdout = child.stdout.take().expect("stdout piped");

        // Stream every render-stats line into a channel so the test can
        // wait for a specific sample with a timeout instead of racing.
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(stats) = parse_stats(&line) {
                    if tx.send(stats).is_err() {
                        break;
                    }
                }
            }
        });

        let deadline = Instant::now() + Duration::from_secs(10);
        while !socket_path.exists() || !synthetic_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before ready: {status}");
            }
            assert!(Instant::now() < deadline, "compositor never became ready");
            std::thread::sleep(Duration::from_millis(10));
        }

        let sender = UnixDatagram::bind(&sender_path).expect("bind sender socket");

        Self {
            child,
            socket_path,
            synthetic_path,
            sender,
            sender_path,
            stats_rx: rx,
            reader: Some(reader),
        }
    }

    fn signal(&self, sig: i32) {
        unsafe {
            kill(self.child.id() as i32, sig);
        }
    }

    fn sample(&self) -> RenderStats {
        self.stats_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("compositor did not emit render stats after SIGUSR1")
    }

    /// Send one synthetic-harness command to the compositor.
    fn animate(&self, command: &str) {
        self.sender
            .send_to(command.as_bytes(), &self.synthetic_path)
            .expect("send synthetic command");
    }

    fn shutdown(mut self) {
        self.signal(SIGTERM);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    assert!(status.success(), "compositor exit status: {status}");
                    break;
                }
                Ok(None) => {
                    assert!(
                        Instant::now() < deadline,
                        "compositor did not exit after SIGTERM"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("wait failed: {err}"),
            }
        }
        assert!(
            !self.socket_path.exists(),
            "teardown leak: socket {} survived exit",
            self.socket_path.display()
        );
    }
}

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

#[test]
fn idle_steady_state_renders_zero_frames() {
    let window = idle_trace_window();
    let proc = CompositorProcess::start("dragonfruit-test-idle");

    // Let startup, seat/output creation, and Xwayland settle. Everything
    // that legitimately wants a frame happens here; the warm-up sample
    // absorbs any late startup activity (Xwayland on a slow CI machine).
    std::thread::sleep(Duration::from_millis(1500));
    proc.signal(SIGUSR1);
    let _warmup = proc.sample();

    // A short baseline so the measured window contains only idle time.
    std::thread::sleep(Duration::from_millis(500));
    proc.signal(SIGUSR1);
    let first = proc.sample();

    // The idle window: a correct compositor has nothing to do — no timers,
    // no self-inflicted damage, no client wakeups, no shell attached.
    let started = Instant::now();
    std::thread::sleep(window);
    proc.signal(SIGUSR1);
    let second = proc.sample();
    let elapsed = started.elapsed();

    assert_idle_flat(&first, &second, elapsed);

    eprintln!(
        "idle trace ({:.1} s window): frames_rendered={} (flat, +0), \
         animation_frames_stepped={} (flat, +0), direct_scanouts={} (flat, +0), \
         client_wakeups={} (flat, +0), frames_skipped_no_damage {} -> {}",
        elapsed.as_secs_f64(),
        second.frames_rendered,
        second.animation_frames_stepped,
        second.direct_scanouts,
        second.client_wakeups,
        first.frames_skipped_no_damage,
        second.frames_skipped_no_damage,
    );

    // One compositor frame per animation frame: a 320 ms dummy animation is
    // ~20 frames at 16 ms. The clock arms its timer only while it runs.
    proc.animate("animate-dummy 320");
    std::thread::sleep(Duration::from_millis(600));
    proc.signal(SIGUSR1);
    let third = proc.sample();

    let frames = third.frames_rendered - second.frames_rendered;
    let stepped = third.animation_frames_stepped - second.animation_frames_stepped;
    assert!(
        stepped >= 5,
        "the 320 ms animation stepped only {stepped} frames: {second:?} -> {third:?}"
    );
    assert_eq!(
        frames, stepped,
        "one compositor frame per animation frame: rendered {frames}, stepped {stepped}"
    );
    assert_eq!(
        third.direct_scanouts, second.direct_scanouts,
        "headless must never direct-scanout during animation: {second:?} -> {third:?}",
    );

    eprintln!(
        "animation frame budget: rendered +{frames}, animation_frames_stepped +{stepped} \
         (one compositor frame per animation frame), client_wakeups +{}",
        third.client_wakeups - second.client_wakeups,
    );

    // Settled: the clock timer must be disarmed, so every counter is flat.
    std::thread::sleep(Duration::from_millis(600));
    proc.signal(SIGUSR1);
    let fourth = proc.sample();
    assert_eq!(
        fourth.frames_rendered, third.frames_rendered,
        "idle compositor rendered after the animation: {third:?} -> {fourth:?}"
    );
    assert_eq!(
        fourth.animation_frames_stepped, third.animation_frames_stepped,
        "the clock kept ticking after the animation settled: {third:?} -> {fourth:?}"
    );
    assert_eq!(
        fourth.client_wakeups, third.client_wakeups,
        "the compositor woke a client after the animation settled: {third:?} -> {fourth:?}"
    );

    eprintln!(
        "after the animation settled: frames_rendered={}, animation_frames_stepped={}, \
         client_wakeups={} (all flat)",
        fourth.frames_rendered, fourth.animation_frames_stepped, fourth.client_wakeups,
    );

    proc.shutdown();
}
