// SPDX-License-Identifier: MIT
//! The live ScreenCast producer (T335a).
//!
//! The portal backend must hand a sandboxed application a *live* PipeWire
//! video node. This module is the producer half of that: it feeds the
//! compositor's existing capture seam — the same element list the live frame
//! composites, rendered once into the screenshot path's offscreen texture —
//! to a `Video/Source` node by its node id, throttled to the requested
//! framerate, and names why it is absent when there is no PipeWire host.
//!
//! ## The process boundary
//!
//! A separate, stateless host tool, `dragonfruit-screencast-producer`
//! (`tools/screencast-producer`), owns the node. The compositor spawns one tool
//! per stream, hands it a Unix socket, and streams one encoded frame per
//! packet; the tool registers the `Video/Source` node with `pw-cli` — the
//! `pipewire` host stack's own CLI, the same host-stack-over-its-CLI
//! discipline the audio adapter uses for WirePlumber
//! ([ADR 0028](../../docs/design/adr/0028-audio-adapter-over-wireplumber-cli.md))
//! — and pumps each packet into the node's buffer. The tool never renders; it
//! only owns the node and the frame pump.
//!
//! The reason is blast radius. A compositor that hung in a frame path hangs the
//! session ([ADR 0158](../../docs/design/adr/0158-compositor-death-ends-session.md)),
//! and a node the compositor registered in the graph is a node it can never
//! outlive. Keeping the narrow, failure-prone half (node registration and the
//! frame pump) in a disposable process means a dead or stuck host costs a
//! stream, not the desktop. See
//! [ADR 0344](../../docs/design/adr/0344-screencast-producer-process-boundary.md).
//!
//! ## What the capture seam guarantees
//!
//! The producer adds no capture path. The backend hands
//! [`ScreencastProducer::offer_image_to_output`] the frame it read back from
//! [`crate::backend::nested::capture_frame`] for the stream's output; that is
//! the *same* offscreen render the screenshot path uses, so there is no second
//! render, no per-frame screencopy, and the trusted shell's own chrome
//! surfaces are already excluded.
//!
//! The stream is sized when the node is created and does not change: the
//! backend hands the whole-output readback to the producer, which re-crops it
//! to each stream's geometry, so the host's buffer and the frames agree
//! without a negotiation round trip per frame. An unchanged desktop costs no
//! socket write, and a stream never sees more frames than its rate.
//!
//! ## What T335a lands, and what T335b owns
//!
//! T335a lands this producer, the compositor's per-output feed in the nested
//! and DRM capture passes, the portal's live `PipeWireTransport`
//! ([`crate::stream`]), the host tool, and the live proof
//! (`make t335a-screencast`). T335b adds the private-protocol command that
//! creates and releases a stream by token (`create_screencast` /
//! `destroy_screencast`, v17) and makes the stream a per-client resource of
//! the compositor, and it makes the portal's registry own each session's
//! nodes. The shell bridge that would answer the portal's `CompositorWatch` is
//! still open ([ADR 0345](../../docs/design/adr/0345-screencast-bridge-and-portal-ownership.md)).

use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The host tool that owns the node and the frame pump, installed beside the
/// compositor (`scripts/stage-install.sh`) and resolved on `PATH`.
pub const PRODUCER_BIN: &str = "dragonfruit-screencast-producer";
/// The PipeWire CLI that asks the graph for a node id (ships with `pipewire`).
pub const PW_CLI_BIN: &str = "pw-cli";
/// The node name a stream publishes. The portal resolves it back to the node
/// id a client links to.
pub const NODE_NAME_PREFIX: &str = "dragonfruit-screencast";
/// One frame packet's header length: `u32` width, height, stride, length.
pub const FRAME_HEADER_LEN: usize = 16;
/// The longest the compositor waits for its host tool to connect at create.
pub const HOST_CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);

/// The negotiated stream size and framerate, fixed when the node is created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamGeometry {
    /// Width in physical output pixels.
    pub width: u32,
    /// Height in physical output pixels.
    pub height: u32,
    /// Requested frames per second.
    pub fps: u32,
}

#[allow(dead_code)] // `new` is the clamped constructor T335b calls; the tests pin it
impl StreamGeometry {
    /// A stream of `width` × `height` at `fps`. The rate is clamped to a usable
    /// band so a client asking for 0 or 10 000 still gets a live stream.
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        StreamGeometry {
            width: width.max(1),
            height: height.max(1),
            fps: fps.clamp(1, 240),
        }
    }

    /// The one legal frame length for this stream.
    #[allow(dead_code)] // read by the producer tests and the host tool's contract
    pub fn frame_len(&self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }

    /// The minimum gap between two offers at this rate.
    pub fn period(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.fps as f64)
    }

    /// Whether a frame may be offered `elapsed` after the last one.
    pub fn should_offer(&self, elapsed: Duration) -> bool {
        elapsed >= self.period()
    }
}

/// A frame read back from the capture seam: RGBA8888, tightly packed (the
/// stream format the node offers; a client's requested `RGBA`/`BGRx` is
/// served by the host tool from this one buffer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFrame {
    /// The frame's width in pixels.
    pub width: u32,
    /// The frame's height in pixels.
    pub height: u32,
    /// RGBA8888, `4 * width` bytes per row, `height` rows.
    pub data: Vec<u8>,
}

impl CapturedFrame {
    /// One row's byte length.
    pub fn stride(&self) -> u32 {
        self.width * 4
    }

    /// Encode the one-packet frame: `u32` width, `u32` height, `u32` stride,
    /// `u32` length, then the pixels.
    pub fn encode(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(FRAME_HEADER_LEN + self.data.len());
        packet.extend_from_slice(&self.width.to_le_bytes());
        packet.extend_from_slice(&self.height.to_le_bytes());
        packet.extend_from_slice(&self.stride().to_le_bytes());
        packet.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        packet.extend_from_slice(&self.data);
        packet
    }
}

/// Why a live stream could not be created or kept. Every variant is the
/// portal's named `pipewire-producer-unavailable` fallback: a client is never
/// handed a node id for a stream that does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerError {
    /// No PipeWire host: the graph has no node for this stream one creation
    /// later, or `pw-cli` could not be run.
    Unavailable(String),
    /// The host tool could not be started or did not connect in time.
    HostMissing(String),
    /// A frame did not fit the stream it was captured for.
    Geometry(String),
}

#[allow(dead_code)] // the fallback label is read by T335b's stream properties
impl ProducerError {
    /// The named fallback label the portal stream carries for this error.
    pub fn fallback_label(&self) -> &'static str {
        "pipewire-producer-unavailable"
    }

    /// A short, log-safe detail line.
    pub fn detail(&self) -> &str {
        match self {
            ProducerError::Unavailable(detail)
            | ProducerError::HostMissing(detail)
            | ProducerError::Geometry(detail) => detail,
        }
    }
}

impl std::fmt::Display for ProducerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self, self.detail())
    }
}

impl std::error::Error for ProducerError {}

/// One live stream: the host process, its frame socket, and the ledger.
struct LiveStream {
    geometry: StreamGeometry,
    node_name: String,
    output: String,
    child: Child,
    socket: UnixStream,
    last_offer: Option<Instant>,
    frames: u64,
    last_error: Option<ProducerError>,
    /// The last frame offered, so an unchanged desktop costs no socket write
    /// (a live stream of a still screen sends one frame, not 30 a second).
    last_frame: Option<CapturedFrame>,
}

/// The live producer over the compositor's one capture seam.
#[derive(Debug, Default)]
pub struct ScreencastProducer {
    streams: Vec<Option<LiveStream>>,
    host_program: Option<PathBuf>,
}

impl std::fmt::Debug for LiveStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveStream")
            .field("geometry", &self.geometry)
            .field("node_name", &self.node_name)
            .field("frames", &self.frames)
            .finish()
    }
}

// The producer's public contract is the whole surface below: the compositor's
// capture pass drives create/offer_image_to_output/destroy, the T335b
// integration drives export_node and the accessors, and the unit tests pin
// the rest. An item unused by today's binary is a seam, not dead code.
#[allow(dead_code)]
impl ScreencastProducer {
    /// A producer that spawns the shipped host tool from `PATH`.
    pub fn new() -> Self {
        ScreencastProducer::default()
    }

    /// A producer that spawns `program` instead of the shipped tool. The unit
    /// tests substitute a stand-in that reads the socket.
    #[allow(dead_code)] // test-only seam (the shipped path uses PATH)
    pub fn with_host_program(program: impl Into<PathBuf>) -> Self {
        ScreencastProducer {
            host_program: Some(program.into()),
            streams: Vec::new(),
        }
    }

    /// The number of live streams.
    pub fn len(&self) -> usize {
        self.streams
            .iter()
            .filter(|stream| stream.is_some())
            .count()
    }

    /// Whether no stream is live.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The frames offered to a stream (0 when it is not live).
    pub fn frames(&self, token: u64) -> u64 {
        self.get(token).map_or(0, |stream| stream.frames)
    }

    /// The geometry of a live stream.
    pub fn geometry(&self, token: u64) -> Option<StreamGeometry> {
        self.get(token).map(|stream| stream.geometry)
    }

    /// The node name a live stream publishes.
    pub fn node_name(&self, token: u64) -> Option<&str> {
        self.get(token).map(|stream| stream.node_name.as_str())
    }

    /// The output feeding a live stream.
    pub fn output(&self, token: u64) -> Option<&str> {
        self.get(token).map(|stream| stream.output.as_str())
    }

    /// The tokens of the streams one output feeds, in creation order.
    pub fn tokens_for_output(&self, output: &str) -> Vec<u64> {
        (0..self.streams.len() as u64)
            .filter(|token| {
                self.get(*token)
                    .is_some_and(|stream| stream.output == output)
            })
            .collect()
    }

    /// Offer one captured frame to every stream `output` feeds. The frame is
    /// the output's own; a stream on another output is untouched. Returns the
    /// number of frames written.
    pub fn offer_to_output(&mut self, output: &str, frame: &CapturedFrame, now: Instant) -> usize {
        self.tokens_for_output(output)
            .into_iter()
            .filter(|token| self.offer(*token, frame, now))
            .count()
    }

    /// Offer one full-output readback to every stream `output` feeds, each
    /// re-cropped to its own geometry. Returns the number of frames written.
    pub fn offer_image_to_output(
        &mut self,
        output: &str,
        image: &image::RgbaImage,
        now: Instant,
    ) -> usize {
        self.tokens_for_output(output)
            .into_iter()
            .filter(|token| self.offer_image(*token, image, now))
            .count()
    }

    /// Why a stream was dropped, when it was.
    pub fn last_error(&self, token: u64) -> Option<&ProducerError> {
        self.get(token)
            .and_then(|stream| stream.last_error.as_ref())
    }

    fn get(&self, token: u64) -> Option<&LiveStream> {
        self.streams
            .get(token as usize)
            .and_then(|stream| stream.as_ref())
    }

    fn get_mut(&mut self, token: u64) -> Option<&mut LiveStream> {
        self.streams
            .get_mut(token as usize)
            .and_then(|stream| stream.as_mut())
    }

    /// Create a stream: one socket, one host tool, one node request. `output`
    /// is the output whose capture pass feeds the stream.
    pub fn create(
        &mut self,
        output: &str,
        geometry: StreamGeometry,
        socket_path: &Path,
    ) -> Result<u64, ProducerError> {
        let _ = std::fs::remove_file(socket_path);
        let listener = UnixListener::bind(socket_path)
            .map_err(|error| ProducerError::HostMissing(format!("socket: {error}")))?;
        // Node ids in a graph are per client connection, so several streams
        // coexist only when each names itself; the socket carries the size.
        let node_name = format!(
            "{NODE_NAME_PREFIX}.{}.{}.{}",
            geometry.width, geometry.height, geometry.fps
        );
        let program = self
            .host_program
            .clone()
            .unwrap_or_else(|| PathBuf::from(PRODUCER_BIN));
        let child = Command::new(&program)
            .arg(socket_path)
            .arg(geometry.width.to_string())
            .arg(geometry.height.to_string())
            .arg(geometry.fps.to_string())
            .arg(&node_name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(if std::env::var_os("DF_SCREENCAST_DEBUG").is_some() {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .spawn()
            .map_err(|error| {
                ProducerError::HostMissing(format!("{}: {error}", program.display()))
            })?;
        let socket = match accept_in_time(listener, HOST_CONNECT_TIMEOUT) {
            Ok(socket) => socket,
            Err(error) => {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(socket_path);
                return Err(error);
            }
        };
        let _ = socket.set_nonblocking(false);
        let token = self.streams.len() as u64;
        self.streams.push(Some(LiveStream {
            geometry,
            node_name,
            output: output.to_owned(),
            child,
            socket,
            last_offer: None,
            frames: 0,
            last_error: None,
            last_frame: None,
        }));
        Ok(token)
    }

    /// Offer one captured frame at most once per stream period. Returns
    /// whether a frame was written. A frame of the wrong size is refused
    /// (never resized: the client's format is fixed); a peer that has gone
    /// drops the stream.
    pub fn offer(&mut self, token: u64, frame: &CapturedFrame, now: Instant) -> bool {
        let Some(stream) = self.get_mut(token) else {
            return false;
        };
        if frame.width != stream.geometry.width
            || frame.height != stream.geometry.height
            || frame.data.len() != stream.geometry.frame_len()
        {
            stream.last_error = Some(ProducerError::Geometry(format!(
                "captured {}x{} does not fit {}x{}",
                frame.width, frame.height, stream.geometry.width, stream.geometry.height
            )));
            return false;
        }
        self.write_due(token, frame, now)
    }

    /// Offer one full-output readback: re-crop it to the stream's geometry
    /// (the top-left anchor a screen share of the output uses) and throttle to
    /// the stream's rate. A readback smaller than the stream is refused and
    /// named, never padded.
    pub fn offer_image(&mut self, token: u64, image: &image::RgbaImage, now: Instant) -> bool {
        let Some(stream) = self.get(token) else {
            return false;
        };
        let geometry = stream.geometry;
        let due = stream
            .last_offer
            .map_or(true, |last| geometry.should_offer(now - last));
        if !due {
            return false;
        }
        if image.width() < geometry.width || image.height() < geometry.height {
            if let Some(stream) = self.get_mut(token) {
                stream.last_error = Some(ProducerError::Geometry(format!(
                    "readback {}x{} is smaller than the stream {}x{}",
                    image.width(),
                    image.height(),
                    geometry.width,
                    geometry.height
                )));
            }
            return false;
        }
        let cropped =
            image::imageops::crop_imm(image, 0, 0, geometry.width, geometry.height).to_image();
        // `RgbaImage` is RGBA8888, tightly packed: the format the node
        // negotiates and the format the wire header promises.
        let frame = CapturedFrame {
            width: geometry.width,
            height: geometry.height,
            data: cropped.into_raw(),
        };
        self.write_due(token, &frame, now)
    }

    fn write_due(&mut self, token: u64, frame: &CapturedFrame, now: Instant) -> bool {
        let Some(stream) = self.get_mut(token) else {
            return false;
        };
        let due = stream
            .last_offer
            .map_or(true, |last| stream.geometry.should_offer(now - last));
        if !due {
            return false;
        }
        // An idle desktop is one frame, not one per tick: the last frame on
        // the wire already shows exactly this picture. (The rate limit above
        // still applies, so a distinguishable frame never arrives faster than
        // the requested rate.)
        if stream.last_frame.as_ref() == Some(frame) {
            stream.last_offer = Some(now);
            return false;
        }
        match stream.socket.write_all(&frame.encode()) {
            Ok(()) => {
                stream.last_offer = Some(now);
                stream.frames += 1;
                stream.last_frame = Some(frame.clone());
                true
            }
            Err(error) => {
                let detail = format!("frame write failed: {error}");
                self.drop_stream(token, Some(ProducerError::HostMissing(detail)));
                false
            }
        }
    }

    /// Release a stream: close the socket (the tool's shutdown signal), stop
    /// the host, and forget it. Returns whether the stream was live.
    pub fn destroy(&mut self, token: u64) -> bool {
        self.drop_stream(token, None)
    }

    /// Release every stream (compositor teardown).
    pub fn destroy_all(&mut self) {
        for token in 0..self.streams.len() as u64 {
            self.drop_stream(token, None);
        }
    }

    fn drop_stream(&mut self, token: u64, error: Option<ProducerError>) -> bool {
        let Some(slot) = self.streams.get_mut(token as usize) else {
            return false;
        };
        let Some(mut stream) = slot.take() else {
            return false;
        };
        let _ = stream.socket.shutdown(Shutdown::Both);
        // A tool that ignored the close is bounded, not waited on forever.
        if stream.child.try_wait().ok().flatten().is_none() {
            let _ = stream.child.kill();
        }
        let _ = stream.child.wait();
        if let Some(error) = error {
            stream.last_error = Some(error);
            slot.replace(stream);
        }
        true
    }

    /// Resolve the live graph's node id for a stream, or name why there is
    /// none. A missing node is the named fallback; the caller decides whether
    /// the stream is released (the shell does, from the failed event).
    pub fn export_node(&mut self, token: u64) -> Result<u32, ProducerError> {
        let Some(node_name) = self.node_name(token).map(str::to_owned) else {
            self.destroy(token);
            return Err(ProducerError::Unavailable("no such stream".to_owned()));
        };
        let result = lookup_node_id(&node_name);
        if result.is_err() {
            self.destroy(token);
        }
        result
    }

    /// Resolve a live graph node id by name through `pw-cli`. Public so the
    /// T335b wiring and the diagnostics share one command and one parser.
    pub fn node_id(node_name: &str) -> Result<u32, ProducerError> {
        lookup_node_id(node_name)
    }
}

impl Drop for ScreencastProducer {
    fn drop(&mut self) {
        self.destroy_all();
    }
}

fn accept_in_time(listener: UnixListener, timeout: Duration) -> Result<UnixStream, ProducerError> {
    if listener.set_nonblocking(true).is_err() {
        return Err(ProducerError::HostMissing("listener".to_owned()));
    }
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((socket, _)) => return Ok(socket),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(ProducerError::HostMissing(
                        "the producer host did not connect".to_owned(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => {
                return Err(ProducerError::HostMissing(format!("accept: {error}")));
            }
        }
    }
}

/// Resolve a node id by name from the live graph through `pw-cli`. The JSON
/// spelling of `pw-dump` is avoided on purpose: the compact listing is stable
/// and small, and the parser is pinned by a captured fixture in the tests.
pub fn lookup_node_id(node_name: &str) -> Result<u32, ProducerError> {
    let output = Command::new(PW_CLI_BIN)
        .args(["list-objects", "Node"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| ProducerError::Unavailable(format!("{PW_CLI_BIN}: {error}")))?;
    if !output.status.success() {
        return Err(ProducerError::Unavailable(format!(
            "{PW_CLI_BIN} exited with {}",
            output.status
        )));
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    node_ids_from_listing(&listing, node_name)
        .pop()
        .ok_or_else(|| {
            ProducerError::Unavailable(format!("no {node_name} node in the PipeWire graph"))
        })
}

/// Every node id whose `node.name` is `node_name`, in listing order. Written
/// against `pw-cli list-objects Node`'s shape: an `id N, type …` line followed
/// by indented `key = "value"` properties (see the captured fixture in the
/// tests).
pub fn node_ids_from_listing(listing: &str, node_name: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    let mut current: Option<u32> = None;
    for line in listing.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("id ") {
            current = rest
                .split([',', ' '])
                .next()
                .and_then(|value| value.parse::<u32>().ok());
            if line_has_name(line, node_name) {
                if let Some(id) = current {
                    ids.push(id);
                }
            }
            continue;
        }
        if current.is_some() && line.starts_with([' ', '\t']) && line_has_name(line, node_name) {
            if let Some(id) = current {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
    ids
}

/// Whether a listing line carries `node.name = "<name>"`.
fn line_has_name(line: &str, node_name: &str) -> bool {
    line.contains(&format!("node.name = \"{node_name}\""))
        || line.contains(&format!("node.name=\"{node_name}\""))
        || line.contains(&format!("node.name = {node_name} "))
        || line.contains(&format!("node.name={node_name} "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The captured shape of `pw-cli list-objects Node` (2026-11 host,
    /// PipeWire 1.6), trimmed to the properties the resolver reads.
    const LISTING: &str = "\
	id 30, type PipeWire:Interface:Node/3
 		object.serial = \"30\"
 		node.name = \"Dummy-Driver\"
 	id 37, type PipeWire:Interface:Node/3
 		object.serial = \"37\"
 		node.name = \"dragonfruit-screencast.1920.1080.30\"
 	id 41, type PipeWire:Interface:Node/3
 		object.serial = \"41\"
 		node.name = \"other\"
";

    #[test]
    fn the_geometry_clamps_and_sizes_its_frame() {
        let geometry = StreamGeometry::new(1920, 1080, 0);
        assert_eq!(geometry.fps, 1, "a zero rate is one frame a second");
        assert_eq!(geometry.frame_len(), 1920 * 1080 * 4);
        assert_eq!(StreamGeometry::new(8, 8, 10_000).fps, 240);
        assert_eq!(StreamGeometry::new(0, 0, 30).width, 1);
    }

    #[test]
    fn the_rate_offers_at_most_one_frame_per_tick() {
        let geometry = StreamGeometry::new(8, 8, 30);
        assert!(geometry.should_offer(Duration::from_millis(34)));
        assert!(!geometry.should_offer(Duration::from_millis(20)));
        assert_eq!(geometry.period(), Duration::from_secs_f64(1.0 / 30.0));
    }

    #[test]
    fn the_node_listing_resolves_a_node_id_from_its_name() {
        assert_eq!(
            node_ids_from_listing(LISTING, "dragonfruit-screencast.1920.1080.30"),
            vec![37]
        );
        assert!(node_ids_from_listing(LISTING, "missing").is_empty());
        // A name that is a prefix of another must not match it.
        assert!(node_ids_from_listing(LISTING, "Dummy").is_empty());
    }

    #[test]
    fn a_frame_is_one_packet_with_its_own_geometry() {
        let frame = CapturedFrame {
            width: 2,
            height: 1,
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let packet = frame.encode();
        assert_eq!(packet.len(), FRAME_HEADER_LEN + 8);
        assert_eq!(u32::from_le_bytes(packet[0..4].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(packet[4..8].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(packet[8..12].try_into().unwrap()), 8);
        assert_eq!(u32::from_le_bytes(packet[12..16].try_into().unwrap()), 8);
    }

    /// A stand-in host the test drives, installed as `<name>.py` (the test's
    /// host program). It receives the production arguments
    /// `socket width height fps node-name`: `connected` opens a real
    /// connection and copies the first 112 bytes of the first
    /// frame packet (a 16-byte header and a 6x4 RGBA frame) to `capture`;
    /// `refused` exits without connecting. Behaviour lives in the script, not
    /// in the process environment, so parallel tests cannot disturb each
    /// other. The stand-in uses a `SOCK_STREAM` socket because SEQPACKET
    /// sockets cannot be connected from a portable script runtime; the shipped
    /// tool uses `SOCK_SEQPACKET`, and the compositor's write path is the same
    /// for both (one `write` per frame, read back in one go here).
    const STAND_IN_HOST: &str = r#"import socket
import sys

mode, socket_path, capture = sys.argv[1], sys.argv[2], sys.argv[3]
if mode == "refused":
    sys.exit(7)
connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
connection.connect(socket_path)
data = connection.recv(1024)
open(capture, "wb").write(data[:112])
connection.close()
"#;

    /// Install a stand-in for `kind` that copies its frame to `capture` and
    /// return the program the producer spawns. The program is a shell wrapper
    /// because [`ScreencastProducer::create`] passes the production arguments
    /// (`socket width height fps node-name`) and the stand-in needs its
    /// behaviour first.
    fn stand_in(dir: &Path, kind: &str, capture: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let python = dir.join(format!("stand-in-{kind}.py"));
        std::fs::write(&python, STAND_IN_HOST).expect("write the stand-in");
        let wrapper = dir.join(format!("stand-in-{kind}"));
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nexec python3 {} {kind} \"$1\" {}\n",
                python.display(),
                capture.display()
            ),
        )
        .expect("write the stand-in wrapper");
        let _ = std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755));
        wrapper
    }

    fn image_of(width: u32, height: u32) -> image::RgbaImage {
        image::RgbaImage::from_pixel(width, height, image::Rgba([1, 2, 3, 4]))
    }

    #[test]
    fn a_stream_spawns_its_host_and_writes_one_frame_per_change() {
        let dir = std::env::temp_dir().join(format!(
            "df-screencast-producer-test-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let socket_path = dir.join("frame.sock");
        let capture_path = dir.join("frame.bin");
        let _ = std::fs::remove_file(&capture_path);

        let mut producer =
            ScreencastProducer::with_host_program(stand_in(&dir, "connected", &capture_path));
        let geometry = StreamGeometry::new(6, 4, 30);
        let token = producer
            .create("HEADLESS-1", geometry, &socket_path)
            .expect("spawn the host");
        assert_eq!(producer.tokens_for_output("HEADLESS-1"), vec![token]);
        assert!(producer.tokens_for_output("OTHER-1").is_empty());

        let now = Instant::now();
        // A frame for another output never reaches this stream.
        assert_eq!(
            producer.offer_image_to_output("OTHER-1", &image_of(6, 4), now),
            0
        );
        assert_eq!(
            producer.offer_image_to_output("HEADLESS-1", &image_of(6, 4), now),
            1
        );
        // The same picture inside the rate window is not re-sent; a new one
        // after the window is.
        assert_eq!(
            producer.offer_image_to_output(
                "HEADLESS-1",
                &image_of(6, 4),
                now + Duration::from_millis(5)
            ),
            0
        );
        let mut changed = image_of(6, 4);
        changed.put_pixel(5, 3, image::Rgba([9, 8, 7, 6]));
        assert_eq!(
            producer.offer_image_to_output("HEADLESS-1", &changed, now + Duration::from_millis(40)),
            1
        );
        assert_eq!(producer.frames(token), 2);

        // The stand-in writes the packet it read; wait for it before the
        // stream (and its socket) is torn down, then read what it received.
        let deadline = Instant::now() + Duration::from_secs(5);
        let bytes = loop {
            let bytes = std::fs::read(&capture_path).unwrap_or_default();
            if !bytes.is_empty() || Instant::now() >= deadline {
                break bytes;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(producer.destroy(token));
        assert!(producer.is_empty());
        assert_eq!(
            bytes.len(),
            FRAME_HEADER_LEN + 6 * 4 * 4,
            "the host received one whole frame packet"
        );
        assert_eq!(&bytes[0..4], &6u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &4u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &24u32.to_le_bytes());
        assert_eq!(&bytes[12..16], &96u32.to_le_bytes());
        let last_row = FRAME_HEADER_LEN + 3 * 6 * 4;
        assert_eq!(
            &bytes[last_row..last_row + 4],
            &[1, 2, 3, 4],
            "the frame's last row crossed the socket unchanged"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A host that never connects is bounded by [`HOST_CONNECT_TIMEOUT`] and
    /// names the failure, so one broken stream can never wedge the compositor.
    #[test]
    fn the_host_tool_is_bounded_when_it_never_connects() {
        let dir = std::env::temp_dir().join(format!(
            "df-screencast-producer-refused-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let mut producer =
            ScreencastProducer::with_host_program(stand_in(&dir, "refused", &dir.join("unused")));
        let started = Instant::now();
        let result = producer.create(
            "HEADLESS-1",
            StreamGeometry::new(6, 4, 30),
            &dir.join("r.sock"),
        );
        assert!(
            matches!(result, Err(ProducerError::HostMissing(_))),
            "{result:?}"
        );
        assert!(
            started.elapsed() < HOST_CONNECT_TIMEOUT + Duration::from_secs(1),
            "a host that never connects is bounded by the connect timeout"
        );
        assert!(producer.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_error_names_its_fallback() {
        let error = ProducerError::Unavailable("no pipewire".to_owned());
        assert_eq!(error.fallback_label(), "pipewire-producer-unavailable");
        assert!(error.to_string().contains("no pipewire"));
    }
}
