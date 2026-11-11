// SPDX-License-Identifier: MIT
//! Detect the host Wayland compositor going away.
//!
//! A nested compositor is a client of the host. When the host exits or its
//! socket closes, winit's event loop can leave the session idle, or (worse)
//! keep swapping without host pacing, instead of surfacing an error. This
//! monitor opens its own connection to the host display socket and waits for
//! EOF; on EOF it pings the session loop, which exits cleanly.
//!
//! It deliberately does not speak the Wayland protocol: it only watches the
//! socket lifetime, so it cannot interfere with winit's connection.

use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;

use calloop::ping::Ping;

/// Resolve `WAYLAND_DISPLAY` to a socket path. A relative name is resolved
/// against `XDG_RUNTIME_DIR`, matching the Wayland client convention.
fn host_socket_path() -> Option<PathBuf> {
    let display = std::env::var_os("WAYLAND_DISPLAY")?;
    let path = PathBuf::from(display);
    if path.is_absolute() {
        return Some(path);
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join(path))
}

/// Start the monitor. Returns `false` when there is no host display to monitor
/// (headless/DRM), `true` when the thread is running.
pub fn spawn(ping: Ping) -> Result<bool, String> {
    let Some(path) = host_socket_path() else {
        return Ok(false);
    };
    let stream = UnixStream::connect(&path).map_err(|error| {
        format!(
            "host monitor could not connect to {}: {error}",
            path.display()
        )
    })?;
    let fd = stream.as_raw_fd();
    thread::Builder::new()
        .name("dragonfruit-host-monitor".into())
        .spawn(move || {
            // Keep the socket open for the lifetime of the thread; `fd` is
            // only valid while `_stream` is alive.
            let _stream = stream;
            let mut pollfd = libc::pollfd {
                fd,
                events: libc::POLLIN | libc::POLLHUP | libc::POLLERR,
                revents: 0,
            };
            loop {
                let result = unsafe { libc::poll(&mut pollfd, 1, -1) };
                if result < 0 {
                    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    break;
                }
                if result == 0 {
                    continue;
                }
                if pollfd.revents & (libc::POLLHUP | libc::POLLERR) != 0 {
                    break;
                }
                if pollfd.revents & libc::POLLIN != 0 {
                    // This connection sends no requests, so a readable socket
                    // with no data is EOF. Drain any unexpected data so poll
                    // can report the hangup.
                    let mut byte = 0u8;
                    let read = unsafe {
                        libc::recv(
                            fd,
                            &mut byte as *mut _ as *mut libc::c_void,
                            1,
                            libc::MSG_PEEK | libc::MSG_DONTWAIT,
                        )
                    };
                    if read == 0 {
                        break;
                    }
                    if read < 0 {
                        let error = std::io::Error::last_os_error();
                        if error.kind() != std::io::ErrorKind::WouldBlock {
                            break;
                        }
                    } else {
                        let mut buffer = [0u8; 256];
                        let _ = unsafe {
                            libc::recv(
                                fd,
                                buffer.as_mut_ptr() as *mut libc::c_void,
                                buffer.len(),
                                libc::MSG_DONTWAIT,
                            )
                        };
                    }
                }
            }
            ping.ping();
        })
        .map_err(|error| format!("failed to start host monitor: {error}"))?;
    Ok(true)
}
