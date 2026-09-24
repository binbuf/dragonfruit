// SPDX-License-Identifier: MIT
//! T-02.1a acceptance: the shared animation clock's frame discipline.
//!
//! The idle trace (`idle_trace.rs`) asserts the steady state renders nothing.
//! This suite attaches the synthetic-input harness, starts a **dummy
//! animation** on the clock (`animate-dummy <ms>`), and asserts:
//!
//! 1. while the animation is live, the render-path frame count advances by
//!    exactly the clock's `animation_frames_stepped` — one compositor frame
//!    per animation frame;
//! 2. once it settles, both counters stay flat (the clock timer is disarmed,
//!    so the idle desktop costs nothing);
//! 3. a zero-duration animation (the reduced-motion collapse) takes a single
//!    step.
//!
//! `SIGUSR1` dumps the counters; the animation frame count travels on the
//! render-stats line so the two numbers can be compared directly.

use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const SIGUSR1: i32 = 10;
const SIGTERM: i32 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderStats {
    frames_rendered: u64,
    frames_skipped_no_damage: u64,
    direct_scanouts: u64,
    animation_frames_stepped: u64,
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
    Some(RenderStats {
        frames_rendered,
        frames_skipped_no_damage,
        direct_scanouts,
        animation_frames_stepped,
    })
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
fn one_compositor_frame_per_animation_frame_and_flat_when_idle() {
    let proc = CompositorProcess::start("dragonfruit-test-animation-clock");

    // Absorb startup activity (seat/output creation, Xwayland).
    std::thread::sleep(Duration::from_millis(1500));
    proc.signal(SIGUSR1);
    let _warmup = proc.sample();

    // Establish the idle baseline.
    std::thread::sleep(Duration::from_millis(300));
    proc.signal(SIGUSR1);
    let first = proc.sample();

    // Start a 200 ms dummy animation (≈12 frames at 16 ms).
    proc.animate("animate-dummy 200");

    // Let it run to completion and settle.
    std::thread::sleep(Duration::from_millis(400));
    proc.signal(SIGUSR1);
    let second = proc.sample();

    let frames = second.frames_rendered - first.frames_rendered;
    let stepped = second.animation_frames_stepped - first.animation_frames_stepped;
    assert!(
        stepped >= 5,
        "the 200 ms animation stepped only {stepped} frames: {first:?} -> {second:?}"
    );
    assert_eq!(
        frames, stepped,
        "one compositor frame per animation frame: rendered {frames}, stepped {stepped}"
    );
    assert_eq!(second.direct_scanouts, first.direct_scanouts);

    // Settled: the clock timer must be disarmed, so both counters are flat.
    std::thread::sleep(Duration::from_millis(600));
    proc.signal(SIGUSR1);
    let third = proc.sample();
    assert_eq!(
        third.frames_rendered, second.frames_rendered,
        "idle compositor rendered after the animation: {second:?} -> {third:?}"
    );
    assert_eq!(
        third.animation_frames_stepped, second.animation_frames_stepped,
        "the clock kept ticking after the animation settled: {second:?} -> {third:?}"
    );

    eprintln!(
        "animation clock trace: rendered +{frames}, stepped +{stepped}; \
         settled flat at frames_rendered={}, frames_stepped={}",
        third.frames_rendered, third.animation_frames_stepped,
    );

    proc.shutdown();
}

/// The reduced-motion collapse: a zero-duration animation takes a single
/// clock step, through the same commit path as a full one.
#[test]
fn a_zero_duration_animation_takes_one_step() {
    let proc = CompositorProcess::start("dragonfruit-test-animation-reduced");

    std::thread::sleep(Duration::from_millis(1500));
    proc.signal(SIGUSR1);
    let _warmup = proc.sample();

    std::thread::sleep(Duration::from_millis(300));
    proc.signal(SIGUSR1);
    let first = proc.sample();

    proc.animate("animate-dummy 0");
    std::thread::sleep(Duration::from_millis(300));
    proc.signal(SIGUSR1);
    let second = proc.sample();

    let stepped = second.animation_frames_stepped - first.animation_frames_stepped;
    assert!(
        (1..=2).contains(&stepped),
        "a zero-duration animation must take a single step, stepped {stepped}"
    );
    assert_eq!(
        second.frames_rendered - first.frames_rendered,
        stepped,
        "one compositor frame per animation frame"
    );

    proc.shutdown();
}
