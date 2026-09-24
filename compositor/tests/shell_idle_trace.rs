// SPDX-License-Identifier: MIT
//! T-09 acceptance: the menu bar contributes zero wakeups to the idle
//! desktop budget (FR-6).
//!
//! The T-02 idle trace (`idle_trace.rs`) asserts the steady state with no
//! clients attached. This suite attaches a **chrome surface** — a client
//! that authenticates through `df_core`, creates the menu-bar
//! `df_layer_surface`, commits one frame, and then sits completely idle —
//! and asserts the compositor renders no further frames. That is the
//! compositor-side half of the budget; the shell-side half is that the real
//! menu bar has no polling loop (its clock is a single minute-aligned
//! one-shot timer, `shell/menubar/MenuBarClock.qml`).
//!
//! The client here is a raw `wayland-client` stand-in, not the Qt shell, so
//! the test is deterministic and runs before the Qt build in CI. The real
//! shell's rendering path is covered by the live nested capture and the
//! dev workflow (`dragonfruit dev --nested --shell`).

use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};

/// Generated client bindings for the private protocols.
mod core_client {
    #![allow(unused_imports, clippy::single_component_path_imports)]
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-core.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("../protocols/dragonfruit-core.xml");
}

mod shell_client {
    #![allow(unused_imports, clippy::single_component_path_imports)]
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-shell.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("../protocols/dragonfruit-shell.xml");
}

use core_client::df_core;
use shell_client::{df_layer_surface, df_shell};

const SIGUSR1: i32 = 10;
const SIGTERM: i32 = 15;
const OUTPUT_W: i32 = 1280;
const BAR_H: i32 = 28;

#[derive(Default)]
struct ShellClient {
    registry: Option<wl_registry::WlRegistry>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    core_global: Option<(u32, u32)>,
    shell_global: Option<(u32, u32)>,
    authenticated: bool,
    layer_configures: Vec<(u32, i32, i32)>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for ShellClient {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        state.registry = Some(registry.clone());
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()))
                }
                "wl_shm" => state.shm = Some(registry.bind(name, version.min(1), qh, ())),
                "df_core" => state.core_global = Some((name, version)),
                "df_shell" => state.shell_global = Some((name, version)),
                _ => {}
            }
        }
    }
}

impl Dispatch<df_core::DfCore, ()> for ShellClient {
    fn event(
        state: &mut Self,
        _: &df_core::DfCore,
        event: df_core::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let df_core::Event::Authenticated { .. } = event {
            state.authenticated = true;
        }
    }
}

impl Dispatch<df_shell::DfShell, ()> for ShellClient {
    fn event(
        _: &mut Self,
        _: &df_shell::DfShell,
        _: df_shell::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<df_layer_surface::DfLayerSurface, ()> for ShellClient {
    fn event(
        state: &mut Self,
        _: &df_layer_surface::DfLayerSurface,
        event: df_layer_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let df_layer_surface::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            state.layer_configures.push((serial, width, height));
        }
    }
}

delegate_noop!(ShellClient: ignore wl_compositor::WlCompositor);
delegate_noop!(ShellClient: ignore wl_shm::WlShm);
delegate_noop!(ShellClient: ignore wl_shm_pool::WlShmPool);
delegate_noop!(ShellClient: ignore wl_buffer::WlBuffer);
delegate_noop!(ShellClient: ignore wl_surface::WlSurface);

// --- render-stats harness --------------------------------------------------

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
    token_path: PathBuf,
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
        let _ = std::fs::remove_file(&self.token_path);
    }
}

impl CompositorProcess {
    fn start(socket_name: &str, token: &str) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);
        let token_path = PathBuf::from(&runtime_dir).join(format!("{socket_name}.launch-token"));

        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_LAUNCH_TOKENS", token)
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
        while !socket_path.exists() || !token_path.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before ready: {status}");
            }
            assert!(Instant::now() < deadline, "compositor never became ready");
            std::thread::sleep(Duration::from_millis(10));
        }

        Self {
            child,
            socket_path,
            token_path,
            stats_rx: rx,
            reader: Some(reader),
        }
    }

    fn read_token(&self) -> String {
        std::fs::read_to_string(&self.token_path)
            .expect("token file readable")
            .trim()
            .to_string()
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

// --- client helpers --------------------------------------------------------

fn connect(socket_path: &Path) -> (Connection, EventQueue<ShellClient>, ShellClient) {
    // `CompositorProcess::start` returns as soon as the socket *node* exists,
    // but a stale node left by a killed run (or a window between the node
    // appearing and the listener accepting) can make a single connect race to
    // ECONNREFUSED (T-09 verify flake). Retry briefly so a startup race can
    // never fail the suite.
    let deadline = Instant::now() + Duration::from_secs(10);
    let stream = loop {
        match std::os::unix::net::UnixStream::connect(socket_path) {
            Ok(stream) => break stream,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                ) =>
            {
                assert!(
                    Instant::now() < deadline,
                    "connect failed after retrying for 10s: {err}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("connect failed: {err}"),
        }
    };
    let conn = Connection::from_socket(stream).expect("connection failed");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = ShellClient::default();
    queue.roundtrip(&mut state).expect("registry roundtrip");
    (conn, queue, state)
}

#[track_caller]
fn wait_for(
    conn: &Connection,
    queue: &mut EventQueue<ShellClient>,
    state: &mut ShellClient,
    timeout: Duration,
    mut pred: impl FnMut(&ShellClient) -> bool,
) {
    use std::os::fd::AsRawFd;
    let deadline = Instant::now() + timeout;
    loop {
        conn.flush().expect("flush while waiting");
        queue.dispatch_pending(state).expect("dispatch pending");
        if pred(state) {
            return;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out waiting for condition");
        let mut pollfd = libc::pollfd {
            fd: conn.as_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut pollfd, 1, millis) };
        assert!(ready >= 0, "poll failed");
        if ready > 0 {
            queue
                .blocking_dispatch(state)
                .expect("dispatch while waiting");
        } else {
            panic!("timed out waiting for condition");
        }
    }
}

/// Create an shm-backed buffer for the menu bar.
fn shm_buffer(
    state: &ShellClient,
    qh: &QueueHandle<ShellClient>,
    width: i32,
    height: i32,
) -> (wl_buffer::WlBuffer, std::fs::File) {
    let shm = state.shm.clone().expect("wl_shm bound");
    let stride = width * 4;
    let size = (stride * height) as usize;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SHM_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SHM_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-shell-idle-{}-{seq}-{:p}.shm",
        std::process::id(),
        &shm
    ));
    let file = std::fs::File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("failed to create shm backing file");
    file.set_len(size as u64).expect("failed to size shm file");
    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(0, width, height, stride, wl_shm::Format::Argb8888, qh, ());
    pool.destroy();
    (buffer, file)
}

/// T-09 FR-6: a mapped, idle menu-bar chrome surface must not cause the
/// compositor to render. The shell commits one frame, then stops; the
/// steady state renders zero frames across a second of inactivity.
#[test]
fn idle_menu_bar_contributes_zero_wakeups() {
    let token = "cc".repeat(32);
    let proc = CompositorProcess::start("dragonfruit-test-shell-idle", &token);

    let (conn, mut queue, mut state) = connect(&proc.socket_path);
    let (name, version) = state.core_global.expect("df_core advertised");
    let qh = queue.handle();
    let core = state
        .registry
        .clone()
        .unwrap()
        .bind::<df_core::DfCore, _, _>(name, version, &qh, ());
    core.authenticate(1, proc.read_token());
    wait_for(&conn, &mut queue, &mut state, Duration::from_secs(5), |s| {
        s.authenticated
    });

    let (shell_name, shell_version) = state.shell_global.expect("df_shell advertised");
    let shell = state
        .registry
        .clone()
        .unwrap()
        .bind::<df_shell::DfShell, _, _>(shell_name, shell_version, &qh, ());
    let surface = state.compositor.clone().unwrap().create_surface(&qh, ());
    let layer = shell.get_layer_surface(
        &surface,
        None,
        df_shell::Layer::Top,
        "menubar".to_string(),
        &qh,
        (),
    );
    layer.set_anchor(1 | 4 | 8); // top | left | right
    layer.set_size(0, BAR_H);
    layer.set_exclusive_zone(BAR_H);
    layer.set_keyboard_interaction(df_layer_surface::KeyboardInteraction::OnDemand);
    surface.commit();
    wait_for(&conn, &mut queue, &mut state, Duration::from_secs(5), |s| {
        s.layer_configures
            .iter()
            .any(|(_, width, height)| *width == OUTPUT_W && *height == BAR_H)
    });

    // Commit one frame of chrome, exactly as the shell does on configure.
    let (buffer, _file) = shm_buffer(&state, &qh, OUTPUT_W, BAR_H);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, OUTPUT_W, BAR_H);
    surface.commit();
    let _ = conn.flush();

    // Let the chrome commit and any startup activity settle.
    std::thread::sleep(Duration::from_millis(1500));
    proc.signal(SIGUSR1);
    let _warmup = proc.sample();

    std::thread::sleep(Duration::from_millis(750));
    proc.signal(SIGUSR1);
    let first = proc.sample();

    std::thread::sleep(Duration::from_millis(1000));
    proc.signal(SIGUSR1);
    let second = proc.sample();

    assert_eq!(
        second.frames_rendered,
        first.frames_rendered,
        "an idle menu bar caused {} new frame(s): {first:?} -> {second:?}",
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

    eprintln!(
        "shell idle trace: frames_rendered={} (flat with the menu bar mapped), \
         animation_frames_stepped={} (flat), \
         frames_skipped_no_damage {} -> {}",
        second.frames_rendered,
        second.animation_frames_stepped,
        first.frames_skipped_no_damage,
        second.frames_skipped_no_damage,
    );

    layer.destroy();
    surface.destroy();
    shell.destroy();
    core.destroy();
    let _ = conn.flush();
    proc.shutdown();
}
