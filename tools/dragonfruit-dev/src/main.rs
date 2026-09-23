// SPDX-License-Identifier: MIT
//! `dragonfruit` — the development command.
//!
//! `dragonfruit dev --nested` launches the compositor as a window on the
//! host session with its own Wayland socket, optionally launches extra
//! programs against that socket, and tears everything down on exit — the
//! host session is never disturbed
//! (docs/design/11-session-and-dev-workflow.md).
//!
//! `dragonfruit dev --soak N` is the teardown-hygiene gate: it runs N
//! headless sessions and verifies zero stray processes and zero stray
//! sockets after every cycle. This is the scripted form of the Foundation
//! phase exit criterion (100 consecutive clean exits).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use df_ipc::{DESKTOP_NAME, LOCKSTEP_VERSION};

mod soak;

const EXIT_USAGE: u8 = 64;
const EXIT_DIRTY: u8 = 71;
const SOCKET_WAIT: Duration = Duration::from_secs(10);
const TEARDOWN_WAIT: Duration = Duration::from_secs(5);

static SIGNALLED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_sig: i32) {
    SIGNALLED.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
    }
}

fn signalled() -> bool {
    SIGNALLED.load(Ordering::SeqCst)
}

fn send_sigterm(pid: u32) {
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
    }
}

struct DevArgs {
    backend: &'static str,
    socket_name: String,
    launch: Vec<Vec<String>>,
    soak_cycles: Option<usize>,
    shell: bool,
}

fn usage() -> String {
    format!(
        "dragonfruit dev tool (lockstep-ipc=v{LOCKSTEP_VERSION}, desktop={DESKTOP_NAME})

USAGE:
    dragonfruit dev --nested [--socket-name NAME] [--shell] [--launch CMD...]
    dragonfruit dev --headless [--socket-name NAME] [--shell] [--launch CMD...]
    dragonfruit dev --soak [N]        # teardown soak test (default 100 cycles)
    dragonfruit version

--shell launches the built shell process (build/shell/src/dragonfruit-shell,
or DF_SHELL_BIN) against the private socket."
    )
}

fn parse_dev_args(mut it: impl Iterator<Item = String>) -> Result<DevArgs, String> {
    let mut args = DevArgs {
        backend: "nested",
        socket_name: format!("dragonfruit-dev-{}", std::process::id()),
        launch: Vec::new(),
        soak_cycles: None,
        shell: false,
    };
    let mut launch: Option<Vec<String>> = None;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--nested" => args.backend = "nested",
            "--headless" => args.backend = "headless",
            "--shell" => args.shell = true,
            "--soak" => {
                args.soak_cycles = Some(it.next().and_then(|v| v.parse().ok()).unwrap_or(100));
            }
            "--socket-name" => {
                let Some(name) = it.next() else {
                    return Err("--socket-name requires a value".into());
                };
                if name.len() > 100 {
                    return Err("socket name too long for a Unix socket path".into());
                }
                args.socket_name = name;
            }
            "--launch" => launch = Some(Vec::new()),
            "--help" | "-h" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => {
                if let Some(cmd) = launch.as_mut() {
                    cmd.push(other.to_string());
                } else {
                    return Err(format!("unknown argument {other:?}\n\n{}", usage()));
                }
            }
        }
    }
    if let Some(cmd) = launch {
        if cmd.is_empty() {
            return Err("--launch requires a command to run".into());
        }
        args.launch.push(cmd);
    }
    Ok(args)
}

fn main() -> ExitCode {
    install_signal_handlers();
    let mut it = std::env::args().skip(1);
    match it.next().as_deref() {
        Some("version") => {
            println!("{DESKTOP_NAME} dev tool (lockstep-ipc=v{LOCKSTEP_VERSION})");
            ExitCode::SUCCESS
        }
        Some("dev") => match parse_dev_args(it) {
            Ok(args) => match args.soak_cycles {
                Some(cycles) => run_soak(cycles),
                None => run_dev_session(&args),
            },
            Err(err) => {
                eprintln!("dragonfruit: {err}");
                ExitCode::from(EXIT_USAGE)
            }
        },
        _ => {
            eprintln!("dragonfruit: expected `dev` subcommand\n\n{}", usage());
            ExitCode::from(EXIT_USAGE)
        }
    }
}

/// Path to the compositor binary: `DF_COMPOSITOR_BIN` if set, otherwise a
/// sibling of this binary (both live in the same cargo target directory).
fn compositor_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("DF_COMPOSITOR_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "DF_COMPOSITOR_BIN={} does not point to a file",
            path.display()
        ));
    }
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate own executable: {e}"))?;
    let sibling = exe.parent().unwrap().join("dragonfruit-compositor");
    if sibling.is_file() {
        Ok(sibling)
    } else {
        Err(format!(
            "compositor not found at {} — build the workspace first (`make build`) \
             or set DF_COMPOSITOR_BIN",
            sibling.display()
        ))
    }
}

/// Path to the shell binary: `DF_SHELL_BIN` if set, otherwise the CMake
/// build tree relative to the repository root (`make build`).
fn shell_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("DF_SHELL_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "DF_SHELL_BIN={} does not point to a file",
            path.display()
        ));
    }
    let path = PathBuf::from("build/shell/src/dragonfruit-shell");
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "shell not found at {} — build the Qt side (`make build`) \
             or set DF_SHELL_BIN",
            path.display()
        ))
    }
}

fn runtime_dir() -> Result<PathBuf, String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| "XDG_RUNTIME_DIR is not set".to_string())?;
    if !dir.exists() {
        return Err(format!("XDG_RUNTIME_DIR={} does not exist", dir.display()));
    }
    Ok(dir)
}

fn socket_path(runtime_dir: &Path, socket_name: &str) -> PathBuf {
    runtime_dir.join(socket_name)
}

/// Wait up to `timeout` for the compositor's one-time launch-token hand-off
/// file (T-07) and return its trimmed contents.
fn wait_for_token(path: &Path, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(contents) = std::fs::read_to_string(path) {
            let token = contents.trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}

/// Wait up to `timeout` for the compositor's Xwayland `DISPLAY` hand-off
/// file and return its trimmed contents (T-06).
fn wait_for_x11_display(path: &Path, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(contents) = std::fs::read_to_string(path) {
            let display = contents.trim().to_string();
            if !display.is_empty() {
                return Some(display);
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}

/// Owns the session's child processes so a panic or an early return can
/// never leak them into the host session (T-01 FR-5: quitting must leave
/// the host completely undisturbed). Normal teardown disarms the guard
/// after the graceful shutdown; `Drop` is the last-resort SIGKILL.
struct ChildGuard {
    compositor: Option<std::process::Child>,
    launched: Vec<(String, std::process::Child)>,
}

impl ChildGuard {
    fn new(compositor: std::process::Child) -> Self {
        ChildGuard {
            compositor: Some(compositor),
            launched: Vec::new(),
        }
    }

    fn compositor(&mut self) -> &mut std::process::Child {
        self.compositor
            .as_mut()
            .expect("compositor child is present until teardown")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.compositor.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        for (_, mut child) in self.launched.drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Run one development session: compositor + optional launched programs,
/// all against a private socket, all torn down on exit.
fn run_dev_session(args: &DevArgs) -> ExitCode {
    let Ok(compositor) = compositor_path() else {
        return ExitCode::from(EXIT_USAGE);
    };
    let Ok(runtime_dir) = runtime_dir() else {
        return ExitCode::from(EXIT_USAGE);
    };
    let socket = socket_path(&runtime_dir, &args.socket_name);

    println!(
        "dragonfruit dev: backend={} lockstep-ipc=v{LOCKSTEP_VERSION}",
        args.backend
    );
    let child = match Command::new(&compositor)
        .arg("--backend")
        .arg(args.backend)
        .arg("--socket-name")
        .arg(&args.socket_name)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "dragonfruit dev: failed to start {}: {e}",
                compositor.display()
            );
            return ExitCode::FAILURE;
        }
    };
    // From here on the guard owns every child, so even a panic cannot leak
    // a compositor or a launched app into the host session.
    let mut guard = ChildGuard::new(child);

    // Wait for the private socket, then surface the path (FR-5).
    let started = Instant::now();
    loop {
        if socket.exists() {
            println!(
                "dragonfruit dev: private socket ready: {}",
                socket.display()
            );
            println!(
                "dragonfruit dev: WAYLAND_DISPLAY={} XDG_CURRENT_DESKTOP={DESKTOP_NAME}",
                args.socket_name
            );
            break;
        }
        if guard
            .compositor()
            .try_wait()
            .map_or(true, |status| status.is_some())
        {
            eprintln!("dragonfruit dev: compositor exited before the socket appeared");
            return ExitCode::FAILURE;
        }
        if started.elapsed() > SOCKET_WAIT {
            eprintln!(
                "dragonfruit dev: timed out waiting for {}",
                socket.display()
            );
            let _ = shutdown_child(guard.compositor());
            return ExitCode::FAILURE;
        }
        if signalled() {
            let _ = shutdown_child(guard.compositor());
            return ExitCode::from(130);
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // Xwayland may still be starting when the Wayland socket appears; wait
    // briefly for its DISPLAY hand-off so X11 `--launch`ed apps get it
    // (T-06). Absent means Xwayland is unavailable (Wayland-only session).
    let display_path = runtime_dir.join(format!("{}.x11-display", args.socket_name));
    let display = wait_for_x11_display(&display_path, Duration::from_secs(5));
    if let Some(display) = &display {
        println!("dragonfruit dev: DISPLAY={display}");
    }

    // The shell is a private-protocol client of the compositor; launch it
    // with the one-time token the compositor provisioned at startup (T-09).
    if args.shell {
        match shell_path() {
            Ok(shell) => {
                let token_path = runtime_dir.join(format!("{}.launch-token", args.socket_name));
                match wait_for_token(&token_path, Duration::from_secs(5)) {
                    Some(token) => {
                        let mut command = Command::new(&shell);
                        command.arg("--socket-name").arg(&args.socket_name);
                        command.arg("--menubar-height").arg("28");
                        command.arg("--placeholders");
                        command.env("WAYLAND_DISPLAY", &args.socket_name);
                        command.env("XDG_CURRENT_DESKTOP", DESKTOP_NAME);
                        command.env("DRAGONFRUIT_LAUNCH_TOKEN", &token);
                        // The shell manages its own Wayland connection and
                        // renders QML offscreen into the chrome surface.
                        command.env("QT_QPA_PLATFORM", "offscreen");
                        match command.spawn() {
                            Ok(child) => {
                                println!("dragonfruit dev: launched shell");
                                guard.launched.push(("shell".to_string(), child));
                            }
                            Err(e) => eprintln!(
                                "dragonfruit dev: failed to launch shell {}: {e}",
                                shell.display()
                            ),
                        }
                    }
                    None => eprintln!(
                        "dragonfruit dev: no launch token at {} — shell not started",
                        token_path.display()
                    ),
                }
            }
            Err(e) => eprintln!("dragonfruit dev: {e}"),
        }
    }

    for cmd in &args.launch {
        let Some(program) = cmd.first() else { continue };
        let mut command = Command::new(program);
        command.args(&cmd[1..]);
        command.env("WAYLAND_DISPLAY", &args.socket_name);
        command.env("XDG_CURRENT_DESKTOP", DESKTOP_NAME);
        if let Some(display) = &display {
            command.env("DISPLAY", display);
        }
        match command.spawn() {
            Ok(child) => {
                println!("dragonfruit dev: launched {cmd:?}");
                guard.launched.push((cmd.join(" "), child));
            }
            Err(e) => eprintln!("dragonfruit dev: failed to launch {cmd:?}: {e}"),
        }
    }

    // Block until the compositor exits or SIGINT/SIGTERM arrives.
    loop {
        if guard
            .compositor()
            .try_wait()
            .is_ok_and(|status| status.is_some())
        {
            break;
        }
        if signalled() {
            println!("dragonfruit dev: shutting down");
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // Only a *new* signal during teardown counts as "user wants out now";
    // the signal that triggered the shutdown must not skip the grace
    // period below (it used to SIGKILL a compositor that was already
    // exiting cleanly and report a dirty teardown).
    SIGNALLED.store(false, Ordering::SeqCst);

    let mut dirty = false;
    for (name, mut child) in std::mem::take(&mut guard.launched) {
        if shutdown_child(&mut child).is_err() {
            eprintln!("dragonfruit dev: {name:?} refused to die");
            dirty = true;
        }
    }

    {
        let child = guard.compositor();
        if child.try_wait().map_or(true, |s| s.is_none()) && shutdown_child(child).is_err() {
            eprintln!("dragonfruit dev: compositor refused to shut down cleanly");
            dirty = true;
        }
    }
    // Disarm the guard: teardown is complete, nothing left to kill.
    guard.compositor = None;

    // Teardown verification: no stray socket may survive (FR-5).
    if socket.exists() {
        let _ = std::fs::remove_file(&socket);
        let _ = std::fs::remove_file(socket.with_extension("lock"));
        eprintln!(
            "dragonfruit dev: DIRTY TEARDOWN — socket {} survived exit",
            socket.display()
        );
        dirty = true;
    }
    // The Xwayland DISPLAY hand-off file is session state; the compositor
    // removes it, but clean it up here too if a hard kill left it behind.
    let _ = std::fs::remove_file(&display_path);
    let strays = soak::dragonfruit_processes();
    if !strays.is_empty() {
        eprintln!("dragonfruit dev: DIRTY TEARDOWN — stray processes: {strays:?}");
        dirty = true;
    }

    if dirty {
        ExitCode::from(EXIT_DIRTY)
    } else {
        println!("dragonfruit dev: clean teardown — host session undisturbed");
        ExitCode::SUCCESS
    }
}

/// SIGTERM first; SIGKILL if the child ignores the grace period (or a new
/// signal demands immediate exit). `Err(())` means the child exited with a
/// failure status or had to be killed mid-flight.
fn shutdown_child(child: &mut std::process::Child) -> Result<(), ()> {
    send_sigterm(child.id());
    let deadline = Instant::now();
    while deadline.elapsed() < TEARDOWN_WAIT {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return Ok(()),
        }
        if signalled() {
            break;
        }
    }
    let _ = child.kill();
    match child.wait() {
        Ok(status) if status.success() => Ok(()),
        _ => Err(()),
    }
}

fn run_soak(cycles: usize) -> ExitCode {
    let Ok(compositor) = compositor_path() else {
        return ExitCode::from(EXIT_USAGE);
    };
    let Ok(runtime_dir) = runtime_dir() else {
        return ExitCode::from(EXIT_USAGE);
    };
    match soak::run(&compositor, &runtime_dir, cycles) {
        Ok(()) => {
            println!("dragonfruit dev: soak passed — {cycles} clean cycles, zero strays");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("dragonfruit dev: soak FAILED: {err}");
            ExitCode::from(EXIT_DIRTY)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn socket_names_stay_within_unix_limits() {
        let name = format!("dragonfruit-dev-{}", std::process::id());
        assert!(name.len() < 100);
        assert!(name.starts_with(df_ipc::DESKTOP_NAME));
    }
}
