// SPDX-License-Identifier: MIT
//! Teardown soak test: run the compositor N times and verify that every
//! cycle leaves nothing behind — no stray processes, no stray sockets.
//!
//! This is the scripted form of the Foundation phase exit criterion
//! ("`dragonfruit dev --nested` runs and exits cleanly 100 consecutive
//! times with zero stray processes or sockets"). CI runs the headless
//! backend, which needs no display; the nested backend is soak-tested on
//! a developer machine with `make soak`.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Snapshot of dragonfruit processes currently running, by scanning
/// /proc directly (no procps dependency). Our own process is excluded —
/// the soak check runs from inside `dragonfruit dev`.
pub fn dragonfruit_processes() -> Vec<String> {
    let own_pid = std::process::id().to_string();
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return found;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.bytes().all(|b| b.is_ascii_digit()) || name == own_pid {
            continue;
        }
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{name}/comm")) {
            let comm = comm.trim_end();
            if comm.starts_with("dragonfruit") {
                found.push(comm.to_owned());
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Run `cycles` headless compositor sessions; fail on the first dirty
/// teardown.
pub fn run(compositor: &Path, runtime_dir: &Path, cycles: usize) -> Result<(), String> {
    for cycle in 1..=cycles {
        let socket_name = format!("dragonfruit-soak-{cycle}");
        let socket = runtime_dir.join(&socket_name);
        let lock = socket.with_extension("lock");

        let mut child = Command::new(compositor)
            .arg("--backend")
            .arg("headless")
            .arg("--socket-name")
            .arg(&socket_name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cycle {cycle}: failed to start compositor: {e}"))?;

        // Wait for socket readiness, then let the compositor run briefly.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if socket.exists() {
                break;
            }
            if child.try_wait().map_or(true, |status| status.is_some()) {
                return Err(format!(
                    "cycle {cycle}: compositor exited before socket appeared"
                ));
            }
            if std::time::Instant::now() > deadline {
                let _ = child.kill();
                return Err(format!(
                    "cycle {cycle}: socket {} never appeared",
                    socket.display()
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::sleep(Duration::from_millis(50));

        // Clean shutdown: SIGTERM, then wait.
        unsafe {
            libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut exited = false;
        while std::time::Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(status)) => {
                    exited = true;
                    if !status.success() {
                        return Err(format!("cycle {cycle}: compositor exit status: {status}"));
                    }
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(e) => return Err(format!("cycle {cycle}: {e}")),
            }
        }
        if !exited {
            let _ = child.kill();
            return Err(format!("cycle {cycle}: compositor ignored SIGTERM"));
        }

        // Verify teardown: socket, lock file, stray processes.
        if socket.exists() || lock.exists() {
            let _ = std::fs::remove_file(&socket);
            let _ = std::fs::remove_file(&lock);
            return Err(format!("cycle {cycle}: stray socket {}", socket.display()));
        }
        let strays = dragonfruit_processes();
        if !strays.is_empty() {
            return Err(format!("cycle {cycle}: stray processes {strays:?}"));
        }
    }
    Ok(())
}
