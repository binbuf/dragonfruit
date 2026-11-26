// SPDX-License-Identifier: GPL-3.0-or-later
//! The live ScreenCast host entry point (T335a).
//!
//! ```text
//! dragonfruit-screencast-producer <socket> <width> <height> <fps> <node-name>
//! ```
//!
//! The compositor starts this process with a live `SOCK_SEQPACKET` listener;
//! the tool connects, registers the node, and reads one frame per packet until
//! the compositor closes the socket. A missing PipeWire host is reported on
//! stderr and exits non-zero, which the compositor turns into the named stills
//! fallback.

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

use dragonfruit_screencast_producer::{status, write_frame, Frame, HostArgs, Node};

fn main() -> ExitCode {
    let args = match HostArgs::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(usage) => {
            status(&usage);
            return ExitCode::from(2);
        }
    };
    let mut socket = match UnixStream::connect(&args.socket) {
        Ok(socket) => socket,
        Err(error) => {
            status(&format!(
                "cannot connect to the compositor socket {}: {error}",
                args.socket.display()
            ));
            return ExitCode::from(3);
        }
    };
    let node = match Node::create(&args) {
        Ok(node) => node,
        Err(error) => {
            status(&error);
            return ExitCode::from(4);
        }
    };

    let mut buffer = Vec::new();
    let mut frames = 0u64;
    loop {
        let read = match dragonfruit_screencast_producer::read_packet(&mut socket, &mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) => {
                status(&format!("frame read failed: {error}"));
                break;
            }
        };
        match Frame::decode(&buffer[..read]) {
            Ok(frame) => {
                // The pump is the whole job: the frame is already the node's
                // buffer (the compositor wrote RGBA8888 at the negotiated
                // size). `write_frame` is the one place a future shared-memory
                // path replaces this copy.
                let row = write_frame(&frame);
                frames += 1;
                if frames == 1 {
                    status(&format!(
                        "first frame {0}x{1} for node {2} (last row {3} bytes)",
                        frame.width,
                        frame.height,
                        args.node_name,
                        row.len()
                    ));
                    let _ = std::io::stdout().flush();
                }
            }
            Err(reason) => status(&format!("dropped frame: {reason}")),
        }
    }

    drop(node);
    if let Err(error) = Node::destroy(&args.node_name) {
        status(&error);
    }
    status(&format!("{frames} frame(s) pumped; {}", "stream closed"));
    ExitCode::SUCCESS
}
