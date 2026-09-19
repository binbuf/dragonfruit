// SPDX-License-Identifier: MIT OR Apache-2.0
//! T-02 acceptance: the FR-2 idle trace.
//!
//! The idle budget is "zero damage, zero client wakeups from our shell".
//! With no clients attached, the compositor must settle into a steady state
//! where nothing asks for a frame. `dump_stats` already emits the render
//! counters on SIGUSR1; this test turns that into a scripted assertion on
//! the headless backend (which needs no display):
//!
//! 1. start the compositor, let startup/Xwayland settle,
//! 2. sample the render counters (SIGUSR1),
//! 3. sit idle for a second, sample again,
//! 4. assert `frames_rendered` did not advance — a self-inflicted redraw
//!    while idle is a bug (T-02 FR-2).
//!
//! `frames_skipped_no_damage` may advance by one per SIGUSR1 (the signal
//! itself wakes the loop and the render pass records the no-damage frame),
//! so only the *damage* counter is asserted to be flat.

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
    Some(RenderStats {
        frames_rendered,
        frames_skipped_no_damage,
        direct_scanouts,
    })
}

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
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
    }
}

impl CompositorProcess {
    fn start(socket_name: &str) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start compositor");
        let stdout = child.stdout.take().expect("stdout piped");

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(runtime_dir).join(&socket_name);

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
        while !socket_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before socket appeared: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "compositor socket never appeared"
            );
            std::thread::sleep(Duration::from_millis(10));
        }

        Self {
            child,
            socket_path,
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
    let proc = CompositorProcess::start("dragonfruit-test-idle");

    // Let startup, seat/output creation, and Xwayland settle. Everything
    // that legitimately wants a frame happens here; the first sample is a
    // warm-up that absorbs any late startup activity (Xwayland on a slow
    // CI machine, etc.).
    std::thread::sleep(Duration::from_millis(1500));
    proc.signal(SIGUSR1);
    let _warmup = proc.sample();

    // Sit idle. A correct compositor has nothing to do: no timers, no
    // self-inflicted damage, no shell attached.
    std::thread::sleep(Duration::from_millis(750));
    proc.signal(SIGUSR1);
    let first = proc.sample();

    std::thread::sleep(Duration::from_millis(1000));
    proc.signal(SIGUSR1);
    let second = proc.sample();

    assert_eq!(
        second.frames_rendered,
        first.frames_rendered,
        "idle compositor rendered {} new frame(s): {first:?} -> {second:?}",
        second.frames_rendered - first.frames_rendered,
    );
    assert_eq!(
        second.direct_scanouts, first.direct_scanouts,
        "headless must never direct-scanout: {first:?} -> {second:?}",
    );

    eprintln!(
        "idle trace: frames_rendered={} (flat), frames_skipped_no_damage {} -> {}",
        second.frames_rendered, first.frames_skipped_no_damage, second.frames_skipped_no_damage,
    );

    proc.shutdown();
}
