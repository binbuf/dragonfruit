// SPDX-License-Identifier: MIT
//! Teardown soak test: run the compositor N times and verify that every
//! cycle leaves nothing behind — no stray processes, no stray sockets.
//!
//! This is the scripted form of the Foundation phase exit criterion
//! ("`dragonfruit dev --nested` runs and exits cleanly 100 consecutive
//! times with zero stray processes or sockets"). CI runs the headless
//! backend, which needs no display; the nested backend is soak-tested on
//! a developer machine with `make soak`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Every file a clean compositor exit must remove for one session. The
/// socket and its lock, plus the shell and desktop launch-token hand-off
/// files (T-07/T-19.3). A leaked token is as much a teardown failure as a
/// leaked socket: it hands a later client a trusted role.
pub fn teardown_artifacts(socket: &Path) -> Vec<PathBuf> {
    vec![
        socket.to_path_buf(),
        socket.with_extension("lock"),
        socket.with_extension("launch-token"),
        socket.with_extension("desktop-launch-token"),
    ]
}

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

/// Run `cycles` compositor sessions on `backend`; fail on the first dirty
/// teardown. The default gate is headless (CI needs no display); `drm` is one
/// real session cycle on the hardware rail (`dragonfruit dev --soak 1 --drm`),
/// and `nested` runs the same gate against a host Wayland session.
pub fn run(
    compositor: &Path,
    runtime_dir: &Path,
    cycles: usize,
    backend: &str,
) -> Result<(), String> {
    for cycle in 1..=cycles {
        let socket_name = format!("dragonfruit-soak-{cycle}");
        let socket = runtime_dir.join(&socket_name);

        let mut child = Command::new(compositor)
            .arg("--backend")
            .arg(backend)
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

        // Verify teardown: no leaked socket, lock, token, or orphan.
        let leaked: Vec<PathBuf> = teardown_artifacts(&socket)
            .into_iter()
            .filter(|path| path.exists())
            .collect();
        if !leaked.is_empty() {
            for path in &leaked {
                let _ = std::fs::remove_file(path);
            }
            return Err(format!("cycle {cycle}: stray artifacts {leaked:?}"));
        }
        let strays = dragonfruit_processes();
        if !strays.is_empty() {
            return Err(format!("cycle {cycle}: stray processes {strays:?}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teardown_covers_the_socket_lock_and_both_tokens() {
        let socket = Path::new("/run/user/1000/dragonfruit-soak-3");
        let names: Vec<String> = teardown_artifacts(socket)
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        assert_eq!(
            names,
            [
                "/run/user/1000/dragonfruit-soak-3",
                "/run/user/1000/dragonfruit-soak-3.lock",
                "/run/user/1000/dragonfruit-soak-3.launch-token",
                "/run/user/1000/dragonfruit-soak-3.desktop-launch-token",
            ]
        );
    }
}
