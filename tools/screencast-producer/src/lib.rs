// SPDX-License-Identifier: MIT
//! The live ScreenCast host tool.
//!
//! The compositor's producer (`compositor/src/screencast_producer.rs`) spawns
//! this process with a Unix socket path and the stream's geometry; the tool
//! registers one `Video/Source` node in the PipeWire graph (through `pw-cli`,
//! the host stack's own CLI, [ADR
//! 0344](../../../docs/design/adr/0344-screencast-producer-process-boundary.md))
//! and pumps the frames the compositor writes into that node's buffer.
//!
//! It never renders and never captures: a frame is one packet of
//! `u32 width, u32 height, u32 stride, u32 length` followed by `stride * height`
//! RGBA8888 bytes. The tool exits when the compositor closes the socket, so a
//! compositor teardown can never leave a node behind; it also destroys the node
//! by name on a normal exit.
//!
//! ## The wire
//!
//! [`Frame::decode`] and [`read_packet`] are the one codec both sides share; the
//! encoder lives in the compositor's producer module (`CapturedFrame::encode`),
//! and the packet unit tests here pin the same bytes.

use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};

/// One frame packet's header length.
pub const HEADER_LEN: usize = 16;
/// The PipeWire CLI used to create the node.
pub const PW_CLI_BIN: &str = "pw-cli";
/// The media class of a screen share.
pub const MEDIA_CLASS: &str = "Video/Source";
/// The one format the compositor produces.
pub const FORMAT: &str = "RGBA";

/// The stream the host is asked to publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostArgs {
    /// The socket the compositor feeds.
    pub socket: std::path::PathBuf,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Frames per second the node advertises.
    pub fps: u32,
    /// The node name clients see in the graph.
    pub node_name: String,
}

impl HostArgs {
    /// Parse `socket width height fps node-name`.
    pub fn parse<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<String> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect();
        let [socket, width, height, fps, node_name] = args.as_slice() else {
            return Err(
                "usage: dragonfruit-screencast-producer <socket> <width> <height> <fps> \
                 <node-name>"
                    .to_owned(),
            );
        };
        let number = |value: &str, what: &str| {
            value
                .parse::<u32>()
                .map_err(|_| format!("{what} must be a positive integer, got {value:?}"))
        };
        let width = number(width, "width")?;
        let height = number(height, "height")?;
        if width == 0 || height == 0 {
            return Err("width and height must be non-zero".to_owned());
        }
        Ok(HostArgs {
            socket: socket.into(),
            width,
            height,
            fps: number(fps, "fps")?.max(1),
            node_name: node_name.clone(),
        })
    }

    /// The `pw-cli create-node adapter` property string. `object.linger=true`
    /// keeps the node alive for the client that links to it after this tool's
    /// own connection ends, so a portal `Start` result stays valid for the
    /// client's lifetime; `media.class`/`format` fix the buffer size the
    /// compositor's frames are written at.
    pub fn node_properties(&self) -> String {
        format!(
            "{{ factory.name=support.null-audio-sink node.name=\"{}\" media.class=\"{}\" \
             node.description=\"Dragonfruit screen share\" format.dsp=\"32 RGBA\" \
             video.width={} video.height={} video.fps={} object.linger=true }}",
            self.node_name, MEDIA_CLASS, self.width, self.height, self.fps
        )
    }
}

/// One frame read from the compositor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bytes per row.
    pub stride: u32,
    /// RGBA8888 pixels, `stride * height` bytes.
    pub data: Vec<u8>,
}

impl Frame {
    /// Decode one packet. A length that does not match its stride and height,
    /// or a header shorter than [`HEADER_LEN`], is refused: the host never
    /// reads a frame of the wrong size into a client's buffer.
    pub fn decode(packet: &[u8]) -> Result<Self, String> {
        if packet.len() < HEADER_LEN {
            return Err(format!("short frame header: {} bytes", packet.len()));
        }
        let word = |at: usize| {
            u32::from_le_bytes([packet[at], packet[at + 1], packet[at + 2], packet[at + 3]])
        };
        let width = word(0);
        let height = word(4);
        let stride = word(8);
        let length = word(12) as usize;
        if width == 0 || height == 0 || stride != width.saturating_mul(4) {
            return Err(format!("invalid frame {width}x{height} stride {stride}"));
        }
        let expected = (stride as usize).saturating_mul(height as usize);
        if length != expected {
            return Err(format!(
                "frame length {length} does not match {stride}x{height}"
            ));
        }
        if packet.len() != HEADER_LEN + length {
            return Err(format!(
                "frame packet carries {} bytes, header promises {length}",
                packet.len() - HEADER_LEN
            ));
        }
        Ok(Frame {
            width,
            height,
            stride,
            data: packet[HEADER_LEN..].to_vec(),
        })
    }

    /// The last row of the frame, so a readback is observable without keeping
    /// every pixel (the host's tests use it to prove a frame arrived whole).
    pub fn last_row(&self) -> &[u8] {
        let start = (self.stride as usize).saturating_mul(self.height as usize - 1);
        &self.data[start..]
    }
}

/// Read exactly one frame from `socket` into `buffer`.
///
/// The shipped tool connects with `SOCK_SEQPACKET`, where one `recv` returns
/// one whole packet (the header and the pixels together). The shell bridge
/// cannot open a `SOCK_SEQPACKET` socket, so a `SOCK_STREAM` peer is served
/// too: a stream read returns whatever has arrived, so the header is read
/// first and the body is filled to its promised length. One frame is always
/// exactly one header and `length` pixels, never a byte less or more.
pub fn read_packet(socket: &mut UnixStream, buffer: &mut Vec<u8>) -> std::io::Result<usize> {
    buffer.resize(MAX_PACKET, 0);
    let mut filled = 0;
    // The stream head: read until the 16-byte header has arrived.
    while filled < HEADER_LEN {
        match socket.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    if filled == 0 {
        return Ok(0);
    }
    if filled < HEADER_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!("frame header ended after {filled} bytes"),
        ));
    }
    let length = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]) as usize;
    if length > MAX_PACKET - HEADER_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame of {length} bytes exceeds the {MAX_PACKET}-byte packet bound"),
        ));
    }
    // On `SOCK_SEQPACKET` the head read already carried the whole packet; on
    // `SOCK_STREAM` (or a packet split across reads) the body is filled.
    while filled < HEADER_LEN + length {
        match socket.read(&mut buffer[filled..HEADER_LEN + length]) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!(
                        "frame ended after {} of {length} pixels",
                        filled - HEADER_LEN
                    ),
                ))
            }
            Ok(read) => filled += read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(HEADER_LEN + length)
}

/// The largest packet the host accepts: one 4K RGBA frame plus slack.
pub const MAX_PACKET: usize = 16 * 1024 * 1024 + HEADER_LEN;

/// The node the host created. Dropping it asks `pw-cli` to destroy the node so
/// a released stream leaves nothing in the graph.
pub struct Node {
    child: Child,
}

impl Node {
    /// Create the node. The command runs to completion before this returns, so
    /// a graph without a PipeWire host is reported before a single frame is
    /// read.
    pub fn create(args: &HostArgs) -> Result<Self, String> {
        let output = Command::new(PW_CLI_BIN)
            .args(["create-node", "adapter", &args.node_properties()])
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| format!("{PW_CLI_BIN}: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{PW_CLI_BIN} create-node exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(Node {
            // `pw-cli create-node` exits after creating the node; the child
            // handle is only kept so `Drop` has something to reap.
            child: Command::new("true")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| format!("true: {error}"))?,
        })
    }

    /// Destroy the node by name.
    pub fn destroy(name: &str) -> Result<(), String> {
        let output = Command::new(PW_CLI_BIN)
            .args(["destroy", name])
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| format!("{PW_CLI_BIN}: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{PW_CLI_BIN} destroy exited with {}",
                output.status
            ));
        }
        Ok(())
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.child.wait();
    }
}

/// Copy a frame into the node's buffer and report the row written, so the
/// caller (and its tests) can log what a client would have received.
pub fn write_frame(frame: &Frame) -> &[u8] {
    frame.last_row()
}

/// Close the socket as the compositor's shutdown signal.
pub fn shutdown(socket: &UnixStream) {
    let _ = socket.shutdown(Shutdown::Both);
}

/// Write a short status line to stderr (inherited by the compositor).
pub fn status(message: &str) {
    let _ = writeln!(
        std::io::stderr(),
        "dragonfruit-screencast-producer: {message}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arguments_decode_the_geometry_and_refuse_a_missing_one() {
        let args = HostArgs::parse(["s.sock", "1920", "1080", "30", "df.stream"]).unwrap();
        assert_eq!(args.width, 1920);
        assert_eq!(args.fps, 30);
        assert_eq!(args.node_name, "df.stream");
        assert!(HostArgs::parse(["s.sock"]).is_err());
        assert!(HostArgs::parse(["s.sock", "0", "10", "30", "n"]).is_err());
    }

    #[test]
    fn the_node_properties_name_the_class_size_and_rate() {
        let args = HostArgs::parse(["s.sock", "8", "6", "15", "df.stream"]).unwrap();
        let properties = args.node_properties();
        assert!(properties.contains("node.name=\"df.stream\""));
        assert!(properties.contains("media.class=\"Video/Source\""));
        assert!(properties.contains("video.width=8"));
        assert!(properties.contains("video.height=6"));
        assert!(properties.contains("video.fps=15"));
    }

    #[test]
    fn a_packet_carries_one_whole_frame() {
        // The exact encoder the compositor writes.
        let mut packet = Vec::new();
        packet.extend_from_slice(&2u32.to_le_bytes());
        packet.extend_from_slice(&2u32.to_le_bytes());
        packet.extend_from_slice(&8u32.to_le_bytes());
        packet.extend_from_slice(&16u32.to_le_bytes());
        packet.extend(0u8..16);
        let frame = Frame::decode(&packet).expect("a well-formed frame");
        assert_eq!((frame.width, frame.height, frame.stride), (2, 2, 8));
        assert_eq!(frame.last_row(), &[8, 9, 10, 11, 12, 13, 14, 15]);

        // A truncated or dishonest packet is refused, never resized.
        assert!(Frame::decode(&packet[..HEADER_LEN + 8]).is_err());
        assert!(Frame::decode(&packet[..8]).is_err());
        let mut lying = packet.clone();
        lying[8..12].copy_from_slice(&10u32.to_le_bytes());
        assert!(Frame::decode(&lying).is_err());
    }
}
