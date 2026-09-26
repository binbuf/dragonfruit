// SPDX-License-Identifier: MIT
//! T-12.5a acceptance: one suspend/resume cycle recovers outputs, input, and
//! clients without a restart.
//!
//! Spawns the compositor on the headless backend with the T-03 synthetic-input
//! harness and drives a real `wayland-client` through a full cycle:
//!
//! * **quiesce.** `suspend` stops frames and drops user input; the scene and
//!   the client connection are untouched, and a frame callback requested while
//!   asleep is *not* delivered.
//! * **recover.** `resume` repaints every output and routes input again; the
//!   pending frame callback is delivered, the output and window counts are
//!   unchanged, and a second synthetic input is observed.
//! * **repeat.** A second cycle completes, so the counter proves the session
//!   stayed alive across cycles.
//!
//! The compositor's suspend state is read back over `query session`; the
//! client's survival is proved by its own protocol round trips.

use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

/// The headless backend's default output mode; the client window is smaller.
const WINDOW_W: i32 = 320;
const WINDOW_H: i32 = 240;

struct CompositorProcess {
    child: Child,
    socket_path: PathBuf,
    synthetic_path: PathBuf,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.synthetic_path);
        let _ = std::fs::remove_file(self.synthetic_path.with_extension("reply"));
    }
}

impl CompositorProcess {
    fn start(socket_name: &str, synthetic: &Path) -> Self {
        let socket_name = format!("{socket_name}-{}", std::process::id());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = PathBuf::from(&runtime_dir).join(&socket_name);

        let _ = std::fs::remove_file(synthetic);
        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .env("DRAGONFRUIT_SYNTHETIC_INPUT", synthetic)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start compositor");

        let deadline = Instant::now() + Duration::from_secs(10);
        while !socket_path.exists() || !synthetic.exists() {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("compositor exited before ready: {status}");
            }
            assert!(Instant::now() < deadline, "compositor never became ready");
            std::thread::sleep(Duration::from_millis(10));
        }
        Self {
            child,
            socket_path,
            synthetic_path: synthetic.to_path_buf(),
        }
    }
}

/// Client end of the compositor's synthetic-input harness.
struct SyntheticInput {
    socket: std::os::unix::net::UnixDatagram,
    path: PathBuf,
    reply_path: PathBuf,
}

impl SyntheticInput {
    fn connect(path: &Path) -> Self {
        let reply_path = path.with_extension("reply");
        let _ = std::fs::remove_file(&reply_path);
        let socket = std::os::unix::net::UnixDatagram::bind(&reply_path)
            .expect("failed to bind synthetic reply socket");
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set reply timeout");
        Self {
            socket,
            path: path.to_path_buf(),
            reply_path,
        }
    }

    fn send(&self, command: &str) {
        self.socket
            .send_to(command.as_bytes(), &self.path)
            .unwrap_or_else(|err| panic!("failed to send synthetic {command:?}: {err}"));
    }

    #[track_caller]
    fn query(&self, command: &str) -> String {
        self.send(command);
        let mut buf = [0u8; 16 * 1024];
        let len = self
            .socket
            .recv(&mut buf)
            .unwrap_or_else(|err| panic!("no reply to {command:?}: {err}"));
        String::from_utf8_lossy(&buf[..len]).into_owned()
    }
}

impl Drop for SyntheticInput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.reply_path);
    }
}

/// A minimal mapped toplevel that tracks its frame callbacks.
#[derive(Default)]
struct Client {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    outputs: usize,
    surface: Option<wl_surface::WlSurface>,
    /// Number of `wl_callback.done` events delivered (frames presented).
    frames: usize,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Client {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "xdg_wm_base" => {
                    state.wm_base = Some(registry.bind(name, version.min(2), qh, ()));
                }
                "wl_output" => {
                    let _output: wl_output::WlOutput = registry.bind(name, version.min(4), qh, ());
                    state.outputs += 1;
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(Client: ignore wl_compositor::WlCompositor);
delegate_noop!(Client: ignore wl_shm::WlShm);
delegate_noop!(Client: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Client: ignore wl_buffer::WlBuffer);
delegate_noop!(Client: ignore wl_output::WlOutput);

impl Dispatch<wl_surface::WlSurface, ()> for Client {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for Client {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.frames += 1;
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Client {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for Client {
    fn event(
        _: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for Client {
    fn event(
        _: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        _: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

fn connect(socket_path: &Path) -> (Connection, EventQueue<Client>, Client) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let stream = loop {
        match UnixStream::connect(socket_path) {
            Ok(stream) => break stream,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                ) =>
            {
                assert!(Instant::now() < deadline, "connect failed: {err}");
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("connect failed: {err}"),
        }
    };
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let registry = conn.display().get_registry(&qh, ());
    let mut state = Client::default();
    queue.roundtrip(&mut state).expect("roundtrip failed");
    drop(registry);
    (conn, queue, state)
}

fn shm_buffer(
    client: &Client,
    qh: &QueueHandle<Client>,
    width: i32,
    height: i32,
) -> (wl_buffer::WlBuffer, std::fs::File) {
    let shm = client.shm.clone().expect("wl_shm bound");
    let stride = width * 4;
    let size = (stride * height) as usize;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SHM_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SHM_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dragonfruit-suspend-conformance-{}-{seq}.shm",
        std::process::id()
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

/// Map a decorated-free toplevel with a real shm buffer.
fn map_toplevel(client: &mut Client, queue: &mut EventQueue<Client>) -> std::fs::File {
    let qh = queue.handle();
    let compositor = client.compositor.clone().expect("wl_compositor bound");
    let wm_base = client.wm_base.clone().expect("xdg_wm_base bound");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Suspend".into());
    surface.commit();
    queue.roundtrip(client).expect("initial configure");

    let (buffer, file) = shm_buffer(client, &qh, WINDOW_W, WINDOW_H);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, WINDOW_W, WINDOW_H);
    surface.commit();
    queue.roundtrip(client).expect("map commit");

    client.surface = Some(surface);
    file
}

/// The value of `key=value` in the first line that starts with `prefix`.
fn field(report: &str, prefix: &str, key: &str) -> u64 {
    report
        .lines()
        .find(|line| line.starts_with(prefix))
        .and_then(|line| {
            line.split_whitespace().find_map(|token| {
                token
                    .strip_prefix(&format!("{key}="))
                    .and_then(|value| value.parse().ok())
            })
        })
        .unwrap_or_else(|| panic!("no {key} in {report:?}"))
}

fn session_field(report: &str, key: &str) -> u64 {
    field(report, "session ", key)
}

/// User input events that reached the input router.
fn latency_received(report: &str) -> u64 {
    field(report, "latency ", "received")
}

/// Wait until `query session` reports `key == expected`.
#[track_caller]
fn wait_for_session(input: &SyntheticInput, key: &str, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let report = input.query("query session");
        if session_field(&report, key) == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {key}={expected}, last {report:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Dispatch until the client has received at least `target` frame callbacks.
///
/// Uses bounded `roundtrip`s rather than `blocking_dispatch` so a missing
/// callback fails on the deadline instead of hanging the test process.
#[track_caller]
fn wait_for_frames(queue: &mut EventQueue<Client>, client: &mut Client, target: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while client.frames < target {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {target} frame callbacks (got {})",
            client.frames
        );
        queue
            .roundtrip(client)
            .expect("roundtrip failed while waiting for a frame");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_suspend_resume_cycle_recovers_outputs_input_and_clients() {
    let socket_name = format!("dragonfruit-test-suspend-{}", std::process::id());
    let synthetic_path = std::env::temp_dir().join(format!("{socket_name}-synth"));
    let proc = CompositorProcess::start(&socket_name, &synthetic_path);
    let input = SyntheticInput::connect(&synthetic_path);

    let (_conn, mut queue, mut client) = connect(&proc.socket_path);
    let _file = map_toplevel(&mut client, &mut queue);
    assert_eq!(client.outputs, 1, "one headless output");

    // Awake baseline: the scene has its output and mapped client window.
    let before = input.query("query session");
    assert_eq!(session_field(&before, "suspended"), 0);
    assert_eq!(session_field(&before, "cycles"), 0);
    assert_eq!(session_field(&before, "outputs"), 1);
    assert_eq!(session_field(&before, "windows"), 1);

    let latency_before = latency_received(&input.query("query latency"));

    // --- suspend: quiesce -------------------------------------------------
    input.send("suspend");
    wait_for_session(&input, "suspended", 1);
    let suspended = input.query("query session");
    assert_eq!(
        session_field(&suspended, "outputs"),
        1,
        "suspend must not drop an output"
    );
    assert_eq!(
        session_field(&suspended, "windows"),
        1,
        "suspend must not unmap a client window"
    );

    // User input is dropped, so it never reaches the input router.
    input.send("key 30 down");
    input.send("key 30 up");
    let latency_during = latency_received(&input.query("query latency"));
    assert_eq!(
        latency_during, latency_before,
        "input must be dropped while suspended"
    );

    // A frame request queued while asleep stays pending: the client commits a
    // no-op update, the compositor keeps it, and no frame is presented.
    let _pending = client.surface.as_ref().unwrap().frame(&queue.handle(), ());
    client.surface.as_ref().unwrap().commit();
    queue
        .roundtrip(&mut client)
        .expect("roundtrip while suspended");
    assert_eq!(
        client.frames, 0,
        "the compositor must not present a frame while suspended"
    );

    // --- resume: recover --------------------------------------------------
    input.send("resume");
    wait_for_session(&input, "suspended", 0);
    wait_for_session(&input, "cycles", 1);

    // The pending callback from before the wake is delivered: the output was
    // repainted and the client was re-woken without a reconnect.
    wait_for_frames(&mut queue, &mut client, 1);

    // Input is routed again.
    input.send("key 30 down");
    let latency_after = latency_received(&input.query("query latency"));
    assert!(
        latency_after > latency_during,
        "input must reach the compositor after resume ({latency_after} vs {latency_during})"
    );

    // The client connection and the whole scene survived.
    queue.roundtrip(&mut client).expect("alive after resume");
    let after = input.query("query session");
    assert_eq!(session_field(&after, "suspended"), 0);
    assert_eq!(session_field(&after, "cycles"), 1);
    assert_eq!(session_field(&after, "outputs"), 1);
    assert_eq!(session_field(&after, "windows"), 1);

    // --- a second cycle proves the session stays alive --------------------
    input.send("suspend");
    wait_for_session(&input, "suspended", 1);
    input.send("resume");
    wait_for_session(&input, "cycles", 2);
    queue
        .roundtrip(&mut client)
        .expect("alive after the second cycle");
    let last = input.query("query session");
    assert_eq!(session_field(&last, "suspended"), 0);
    assert_eq!(session_field(&last, "cycles"), 2);
    assert_eq!(
        session_field(&last, "windows"),
        1,
        "the client survived both cycles"
    );
}
