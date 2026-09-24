// SPDX-License-Identifier: MIT
//! T-03.1b acceptance: the input-to-photon latency instrument and the
//! direct-scanout counter template.
//!
//! The instrument is backend-agnostic: routing stamps the input
//! (`input::process_input_event`) and the presenting backend samples it
//! (`render::post_repaint*` nested/headless, `backend::drm::render_surface`
//! on DRM). Headless drives the same code path, so this test can assert on
//! the documented render-stats contract without a display:
//!
//! 1. start the headless compositor, let startup settle;
//! 2. sample a baseline, then inject a real synthetic shortcut (Ctrl+Up,
//!    which requests a redraw through the normal input router);
//! 3. sample again and assert a latency sample was emitted and that it is
//!    within one 60 Hz frame (the T-03 budget);
//! 4. assert the direct-scanout counter template reads the same counters the
//!    compositor reports (`direct_scanouts=0` headless, `considered` = the
//!    sum of rendered + skipped), then;
//! 5. assert an idle window adds no samples — the instrument is flat when no
//!    input arrives.
//!
//! The render-stats line is positional and append-only ([ADR 0009]); the
//! latency fields are appended after the T-03.1a `client_wakeups`.

use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const SIGUSR1: i32 = 10;
const SIGTERM: i32 = 15;

/// One 60 Hz frame, the T-03 input-to-photon budget.
const ONE_FRAME_US: u64 = 1_000_000 / 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderStats {
    frames_rendered: u64,
    latency_us_last: u64,
    latency_samples: u64,
    latency_dropped: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScanoutStats {
    direct_scanouts: u64,
    frames_rendered: u64,
    frames_skipped_no_damage: u64,
    considered: u64,
}

#[derive(Debug, Clone, Copy)]
struct Trace {
    render: RenderStats,
    scanout: ScanoutStats,
}

/// Read one `key=value` numeric field from a trace line. The preceding-space
/// check stops `samples=` matching the suffix of `latency_samples=`.
fn field(line: &str, key: &str) -> Option<u64> {
    let needle = format!("{key}=");
    for (index, _) in line.match_indices(&needle) {
        if index == 0 || line.as_bytes()[index - 1] == b' ' {
            let rest = &line[index + needle.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            return rest[..end].parse().ok();
        }
    }
    None
}

fn parse_render(line: &str) -> Option<RenderStats> {
    if !line.contains("render stats") {
        return None;
    }
    Some(RenderStats {
        frames_rendered: field(line, "frames_rendered")?,
        latency_us_last: field(line, "latency_us_last")?,
        latency_samples: field(line, "latency_samples")?,
        latency_dropped: field(line, "latency_dropped")?,
    })
}

fn parse_scanout(line: &str) -> Option<ScanoutStats> {
    if !line.contains("scanout stats") {
        return None;
    }
    Some(ScanoutStats {
        direct_scanouts: field(line, "direct_scanouts")?,
        frames_rendered: field(line, "frames_rendered")?,
        frames_skipped_no_damage: field(line, "frames_skipped_no_damage")?,
        considered: field(line, "considered")?,
    })
}

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    synthetic_path: PathBuf,
    sender: UnixDatagram,
    sender_path: PathBuf,
    trace_rx: Receiver<Trace>,
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

        // The reader pairs each SIGUSR1's `render stats` line with the
        // `scanout stats` line that follows it into one `Trace`.
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            let mut pending_render: Option<RenderStats> = None;
            for line in reader.lines().map_while(Result::ok) {
                if let Some(render) = parse_render(&line) {
                    pending_render = Some(render);
                } else if let Some(scanout) = parse_scanout(&line) {
                    if let Some(render) = pending_render.take() {
                        if tx.send(Trace { render, scanout }).is_err() {
                            break;
                        }
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
            trace_rx: rx,
            reader: Some(reader),
        }
    }

    fn signal(&self, sig: i32) {
        unsafe {
            kill(self.child.id() as i32, sig);
        }
    }

    fn sample(&self) -> Trace {
        self.trace_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("compositor did not emit a render+scanout pair after SIGUSR1")
    }

    fn send(&self, command: &str) {
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
fn latency_instrument_emits_one_frame_samples_and_the_scanout_template_reads_stats() {
    let proc = CompositorProcess::start("dragonfruit-test-latency");

    // Startup, seat/output creation, Xwayland settle.
    std::thread::sleep(Duration::from_millis(1500));
    proc.signal(SIGUSR1);
    let _warmup = proc.sample();

    std::thread::sleep(Duration::from_millis(500));
    proc.signal(SIGUSR1);
    let first = proc.sample();

    // A real input through the normal router: Ctrl+Up (Mission Control)
    // requests a redraw, so the next presented frame must emit a sample.
    proc.send("key 29 down\nkey 103 down\nkey 29 up\nkey 103 up\n");
    std::thread::sleep(Duration::from_millis(400));
    proc.signal(SIGUSR1);
    let second = proc.sample();

    assert!(
        second.render.latency_samples > first.render.latency_samples,
        "input did not emit a latency sample: {first:?} -> {second:?}"
    );
    assert!(
        second.render.latency_us_last > 0,
        "a sample of zero microseconds is not a real measurement: {second:?}"
    );
    assert!(
        second.render.latency_us_last <= ONE_FRAME_US,
        "input-to-photon latency {} us exceeds one 60 Hz frame ({ONE_FRAME_US} us): {second:?}",
        second.render.latency_us_last,
    );
    assert_eq!(
        second.render.latency_dropped, first.render.latency_dropped,
        "a fresh input was dropped as stale: {first:?} -> {second:?}"
    );

    // The direct-scanout counter template reads the same counters the
    // compositor reports: headless never direct-scans-out, and `considered`
    // is rendered + skipped.
    assert_eq!(
        second.scanout.direct_scanouts, 0,
        "headless must never direct-scanout: {second:?}"
    );
    assert_eq!(
        second.scanout.considered,
        second.scanout.frames_rendered + second.scanout.frames_skipped_no_damage,
        "scanout template mismatch: {second:?}"
    );

    eprintln!(
        "latency instrument: {} -> sample {} us (max {} us), within one 60 Hz frame ({} us); \
         scanout template: {} (direct_scanouts flat at 0)",
        first.render.latency_samples,
        second.render.latency_us_last,
        second.render.latency_us_last,
        ONE_FRAME_US,
        second.scanout.considered,
    );

    // Idle: no input, so no new samples and no new drops.
    std::thread::sleep(Duration::from_millis(800));
    proc.signal(SIGUSR1);
    let third = proc.sample();
    assert_eq!(
        third.render.latency_samples, second.render.latency_samples,
        "the latency instrument sampled while idle: {second:?} -> {third:?}"
    );
    assert_eq!(
        third.render.latency_dropped, second.render.latency_dropped,
        "the latency instrument dropped inputs while idle: {second:?} -> {third:?}"
    );

    proc.shutdown();
}
