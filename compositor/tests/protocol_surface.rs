// SPDX-License-Identifier: MIT OR Apache-2.0
//! T-02 acceptance: the protocol advertise list matches the ticket
//! exactly — no missing, no extras — and no capture protocol is ever
//! advertised (FR-7/FR-8).
//!
//! Spawns the compositor on the headless backend, connects as a Wayland
//! client, and asserts the exact set of advertised globals.

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wayland_client::protocol::{wl_registry, wl_surface};
use wayland_client::{Connection, Dispatch, QueueHandle};

/// The exact protocol surface from .docs/tasks/02-compositor-core.md.
///
/// `zwp_linux_dmabuf_v1` is backend-dependent: it is only advertised when
/// the backend has a renderer (headless deliberately does not), so it is
/// asserted separately in the "forbidden" list here.
const EXPECTED_GLOBALS: &[&str] = &[
    // core
    "wl_compositor",
    "wl_subcompositor",
    "wl_shm",
    "wl_seat",
    "wl_output",
    "wl_data_device_manager",
    // shells
    "xdg_wm_base",
    "zxdg_decoration_manager_v1",
    // outputs
    "zxdg_output_manager_v1",
    // buffers and scaling
    "wp_viewporter",
    "wp_fractional_scale_manager_v1",
    // presentation
    "wp_presentation",
    // pointer protocols
    "zwp_pointer_constraints_v1",
    "zwp_relative_pointer_manager_v1",
    "zwp_pointer_gestures_v1",
    "wp_cursor_shape_manager_v1",
    // idle
    "zwp_idle_inhibit_manager_v1",
    "ext_idle_notifier_v1",
    // content typing
    "wp_content_type_manager_v1",
    // clipboard managers (T-29)
    "zwlr_data_control_manager_v1",
    // sandboxing
    "wp_security_context_manager_v1",
    // launch feedback
    "xdg_activation_v1",
    // fail-secure locking (T-26)
    "ext_session_lock_manager_v1",
    // input methods
    "zwp_text_input_manager_v3",
    "zwp_input_method_manager_v2",
    // tablets (input layer: first-class from the start)
    "zwp_tablet_manager_v2",
];

/// Protocols that must never be advertised — "if a capture is not a
/// portal request, the answer is no" (FR-8), plus protocols reserved for
/// private replacements (T-07) or later tickets.
const FORBIDDEN_GLOBALS: &[&str] = &[
    "zwlr_screencopy_manager_v1",
    "zwlr_export_dmabuf_manager_v1",
    "ext_output_image_capture_source_manager_v1",
    "zwp_linux_dmabuf_v1", // headless has no renderer; asserted per backend
    "zwlr_layer_shell_v1", // private shell protocols in T-07
    "zwlr_foreign_toplevel_manager_v1", // ditto
    "ext_foreign_toplevel_list_v1", // ditto
];

#[derive(Default)]
struct RegistryState {
    globals: Vec<(u32, String)>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for RegistryState {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version: _,
        } = event
        {
            state.globals.push((name, interface));
        } else if let wl_registry::Event::GlobalRemove { name } = event {
            state.globals.retain(|(n, _)| *n != name);
        }
        let _ = qh;
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for RegistryState {
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

struct CompositorProcess {
    child: Child,
    socket_path: std::path::PathBuf,
    stdout: Option<std::process::ChildStdout>,
}

impl Drop for CompositorProcess {
    fn drop(&mut self) {
        // Never leak a compositor child, even when an assertion panics.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl CompositorProcess {
    fn start(socket_name: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"))
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(socket_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start compositor");
        let stdout = child.stdout.take();

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let socket_path = std::path::PathBuf::from(runtime_dir).join(socket_name);

        // Wait for the socket to appear.
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
            stdout,
        }
    }

    fn assert_clean_exit(mut self) {
        // Request shutdown like a session manager would.
        unsafe { libc_kill(self.child.id()) };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        if let Some(mut out) = self.stdout.take() {
                            use std::io::Read;
                            let mut buf = String::new();
                            let _ = out.read_to_string(&mut buf);
                            eprintln!("--- compositor stdout ---\n{buf}\n--- end ---");
                        }
                    }
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
                Err(e) => panic!("wait failed: {e}"),
            }
        }
        assert!(
            !self.socket_path.exists(),
            "teardown leak: socket {} survived exit",
            self.socket_path.display()
        );
    }
}

#[allow(non_snake_case)]
unsafe fn libc_kill(pid: u32) {
    // libc is already a workspace dependency of the compositor; use the
    // raw symbol to keep this test std-only otherwise.
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    const SIGTERM: i32 = 15;
    kill(pid as i32, SIGTERM);
}

fn collect_globals(socket_name: &str) -> (Vec<String>, CompositorProcess) {
    // Unique per process so leftovers from crashed runs cannot collide.
    let socket_name = format!("{socket_name}-{}", std::process::id());
    let socket_name = socket_name.as_str();
    let proc = CompositorProcess::start(socket_name);

    let stream = std::os::unix::net::UnixStream::connect(&proc.socket_path)
        .expect("failed to connect to compositor socket");
    let conn = Connection::from_socket(stream).expect("failed to create connection");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();

    let _display = conn.display();
    let registry_state = RegistryState::default();
    let _registry = conn.display().get_registry(&qh, ());

    let mut state = registry_state;
    // Two roundtrips: advertise + make sure nothing else arrives.
    queue.roundtrip(&mut state).expect("roundtrip failed");
    queue.roundtrip(&mut state).expect("roundtrip failed");

    let names: Vec<String> = state.globals.iter().map(|(_, i)| i.clone()).collect();
    (names, proc)
}

#[test]
fn protocol_surface_matches_ticket_exactly() {
    let (globals, proc) = collect_globals("dragonfruit-test-proto");

    let mut sorted = globals.clone();
    sorted.sort();
    let mut expected: Vec<String> = EXPECTED_GLOBALS.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(sorted, expected, "advertised protocol surface mismatch");

    proc.assert_clean_exit();
}

#[test]
fn no_capture_or_private_grab_protocols_advertised() {
    let (globals, proc) = collect_globals("dragonfruit-test-capture");

    for forbidden in FORBIDDEN_GLOBALS {
        assert!(
            !globals.iter().any(|g| g == forbidden),
            "forbidden protocol {forbidden} is advertised"
        );
    }

    proc.assert_clean_exit();
}
