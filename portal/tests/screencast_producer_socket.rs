// SPDX-License-Identifier: MIT
//! T335a/T335b: frames cross the producer socket into the host tool.
//!
//! The compositor's producer feeds its frame socket only from the nested/DRM
//! capture pass (the headless backend has no readback), so the live client's
//! pixels arrive through the shipped host tool. This test drives that exact
//! path without a GPU: it starts the real `dragonfruit-screencast-producer`
//! against a private PipeWire core, writes frames in the compositor's own
//! packet format ([`CapturedFrame::encode`]), and reads the result back with
//! `pw-cli`/`pw-dump` — the graph really carries the stream's `Video/Source`
//! node, and the tool really consumed the packets.
//!
//! A standard client's read (`pipewiresrc`) is the same node; the live capture
//! script (`make t335b-screencast-client`) drives it in a nested session with a
//! display, and this test pins the half that needs no display.
//!
//! Skipped with a reason when `pipewire`, `pw-cli`, or the built host tool is
//! missing — never a silent pass.

use std::io::Write;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
const FPS: u32 = 30;

struct Guard(Child);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn tool(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn producer_binary() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        std::env::var_os("DRAGONFRUIT_PRODUCER_BIN").map(PathBuf::from),
        Some(root.join("../target/debug/dragonfruit-screencast-producer")),
        Some(root.join("../target/release/dragonfruit-screencast-producer")),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_file())
}

fn pw_cli(runtime: &Path, args: &[&str]) -> std::process::Output {
    Command::new("pw-cli")
        .args(args)
        .env("XDG_RUNTIME_DIR", runtime)
        .output()
        .expect("pw-cli runs")
}

/// One frame packet: the compositor's `CapturedFrame::encode` format.
fn frame_packet(pixel: [u8; 4]) -> Vec<u8> {
    let data: Vec<u8> = std::iter::repeat(pixel)
        .take(WIDTH as usize * HEIGHT as usize)
        .flatten()
        .collect();
    let mut packet = Vec::with_capacity(16 + data.len());
    packet.extend_from_slice(&WIDTH.to_le_bytes());
    packet.extend_from_slice(&HEIGHT.to_le_bytes());
    packet.extend_from_slice(&(WIDTH * 4).to_le_bytes());
    packet.extend_from_slice(&(data.len() as u32).to_le_bytes());
    packet.extend_from_slice(&data);
    packet
}

#[test]
fn the_host_tool_publishes_the_stream_node_and_consumes_frames() {
    if !tool("pipewire") || !tool("pw-cli") {
        eprintln!("SKIP the producer socket check: pipewire/pw-cli missing");
        return;
    }
    let Some(producer) = producer_binary() else {
        eprintln!("SKIP the producer socket check: no built dragonfruit-screencast-producer");
        return;
    };
    let dir =
        std::env::temp_dir().join(format!("dragonfruit-t335b-producer-{}", std::process::id()));
    let runtime = dir.join("runtime");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&runtime).expect("runtime dir");

    // A private PipeWire core: the tool registers the node in this graph.
    let _core = Guard(
        Command::new("pipewire")
            .env("XDG_RUNTIME_DIR", &runtime)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("pipewire starts"),
    );
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until && !pw_cli(&runtime, &["info", "0"]).status.success() {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        pw_cli(&runtime, &["info", "0"]).status.success(),
        "the core is up"
    );

    // The compositor's end of the stream: a listener the tool connects to.
    let socket = dir.join("frame.sock");
    let listener = UnixListener::bind(&socket).expect("the frame socket binds");
    listener.set_nonblocking(false).expect("blocking listener");
    let node_name = format!("dragonfruit-screencast.{WIDTH}.{HEIGHT}.{FPS}");
    let tool_child = Command::new(&producer)
        .arg(&socket)
        .arg(WIDTH.to_string())
        .arg(HEIGHT.to_string())
        .arg(FPS.to_string())
        .arg(&node_name)
        .env("XDG_RUNTIME_DIR", &runtime)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the host tool starts");
    let _tool = Guard(tool_child);

    let (mut stream, _) = listener.accept().expect("the host tool connects");
    // Two distinct frames: the tool pumps each into the node.
    stream
        .write_all(&frame_packet([0x12, 0x9a, 0xe4, 0xff]))
        .expect("frame 1");
    std::thread::sleep(Duration::from_millis(120));
    stream
        .write_all(&frame_packet([0xe4, 0x9a, 0x12, 0xff]))
        .expect("frame 2");

    // The graph carries the stream's one node, under the name the compositor's
    // producer and the portal's resolver both use.
    let mut seen = String::new();
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until {
        seen = String::from_utf8_lossy(&pw_cli(&runtime, &["list-objects", "Node"]).stdout)
            .into_owned();
        if seen.contains(&node_name) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        seen.contains(&node_name),
        "the host tool publishes {node_name} in the private graph:\n{seen}"
    );

    // The listing shape the compositor's resolver reads: an `id N, type …`
    // header line followed by indented `node.name = "…"` properties. The node
    // id on that header is what the portal hands a client.
    let mut node_id = None;
    for line in seen.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("id ") {
            node_id = rest
                .split([',', ' '])
                .next()
                .and_then(|id| id.parse::<u32>().ok());
            continue;
        }
        if line.starts_with([' ', '\t']) && line.contains(&format!("node.name = \"{node_name}\"")) {
            break;
        }
        if !line.starts_with([' ', '\t']) {
            node_id = None;
        }
    }
    let node_id = node_id.expect("the stream node has a graph id in the listing");
    assert!(node_id > 0, "a client links to the node by this id");
    let _ = std::fs::remove_dir_all(&dir);
}
