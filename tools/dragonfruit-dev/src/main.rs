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

mod demo;
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
    backend_explicit: bool,
    socket_name: String,
    launch: Vec<Vec<String>>,
    soak_cycles: Option<usize>,
    shell: bool,
    demo: bool,
}

fn usage() -> String {
    format!(
        "dragonfruit dev tool (lockstep-ipc=v{LOCKSTEP_VERSION}, desktop={DESKTOP_NAME})

USAGE:
    dragonfruit dev --nested [--socket-name NAME] [--shell] [--launch CMD...]
    dragonfruit dev --headless [--socket-name NAME] [--shell] [--launch CMD...]
    dragonfruit dev --demo [--nested|--headless] [--socket-name NAME]
    dragonfruit dev --soak [N]        # teardown soak test (default 100 cycles)
    dragonfruit version

--shell launches the built shell process (build/shell/src/dragonfruit-shell,
or DF_SHELL_BIN) against the private socket.

--demo builds nothing (use `make demo`), but launches the shell, a Qt/Wayland
app, and an X11 app against a private socket and prints the T-01 checklist.
With a host Wayland session it runs nested for the human walkthrough; with
none (CI) it runs the headless scripted half: launch, settle, verify every
child is alive, tear down, and assert no socket leaked. `--launch` may be
repeated to add extra programs."
    )
}

fn parse_dev_args(mut it: impl Iterator<Item = String>) -> Result<DevArgs, String> {
    let mut args = DevArgs {
        backend: "nested",
        backend_explicit: false,
        socket_name: format!("dragonfruit-dev-{}", std::process::id()),
        launch: Vec::new(),
        soak_cycles: None,
        shell: false,
        demo: false,
    };
    let mut launch: Option<Vec<String>> = None;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--nested" => {
                args.backend = "nested";
                args.backend_explicit = true;
            }
            "--headless" => {
                args.backend = "headless";
                args.backend_explicit = true;
            }
            "--shell" => args.shell = true,
            "--demo" => args.demo = true,
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
            "--launch" => {
                // Each --launch starts a new program; the following
                // non-flag arguments are its argv.
                if let Some(cmd) = launch.take() {
                    if cmd.is_empty() {
                        return Err("--launch requires a command to run".into());
                    }
                    args.launch.push(cmd);
                }
                launch = Some(Vec::new());
            }
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
                None if args.demo => run_demo_session(&args),
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

    /// Names of launched children that have already exited, with their
    /// status. Used by the demo's scripted half to fail when a client dies
    /// instead of staying mapped.
    fn dead_children(&mut self) -> Vec<String> {
        let mut dead = Vec::new();
        for (name, child) in self.launched.iter_mut() {
            match child.try_wait() {
                Ok(Some(status)) => dead.push(format!("{name} exited early: {status}")),
                Ok(None) => {}
                Err(err) => dead.push(format!("{name}: {err}")),
            }
        }
        dead
    }

    /// Names of launched children that have already exited with a failure
    /// status. A client the human closes cleanly (exit 0) is not a failure;
    /// a crash mid-demo is.
    fn failed_children(&mut self) -> Vec<String> {
        let mut failed = Vec::new();
        for (name, child) in self.launched.iter_mut() {
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => {
                    failed.push(format!("{name} exited with {status}"));
                }
                _ => {}
            }
        }
        failed
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

/// A live session: the compositor child plus the private socket and the
/// Xwayland `DISPLAY` hand-off it provisioned. Owning this in one place is
/// what lets `run_dev_session` and the demo harness share teardown.
struct Session {
    guard: ChildGuard,
    socket: PathBuf,
    display_path: PathBuf,
    display: Option<String>,
}

/// Why a session could not start.
enum SessionError {
    /// The compositor never came up; the message is already user-facing.
    Failed(String),
    /// SIGINT/SIGTERM arrived before the session was ready.
    Interrupted,
}

/// Start the compositor on `backend`, wait for its private socket and (if
/// any) the Xwayland `DISPLAY` hand-off. From here on the returned `Session`
/// owns every child, so even a panic cannot leak one.
fn start_session(
    backend: &str,
    socket_name: &str,
    runtime_dir: &Path,
) -> Result<Session, SessionError> {
    let compositor = compositor_path().map_err(SessionError::Failed)?;
    let socket = socket_path(runtime_dir, socket_name);

    println!(
        "dragonfruit dev: backend={} lockstep-ipc=v{LOCKSTEP_VERSION}",
        backend
    );
    let child = Command::new(&compositor)
        .arg("--backend")
        .arg(backend)
        .arg("--socket-name")
        .arg(socket_name)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            SessionError::Failed(format!("failed to start {}: {e}", compositor.display()))
        })?;
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
                "dragonfruit dev: WAYLAND_DISPLAY={socket_name} XDG_CURRENT_DESKTOP={DESKTOP_NAME}"
            );
            break;
        }
        if guard
            .compositor()
            .try_wait()
            .map_or(true, |status| status.is_some())
        {
            return Err(SessionError::Failed(
                "compositor exited before the socket appeared".into(),
            ));
        }
        if started.elapsed() > SOCKET_WAIT {
            let _ = shutdown_child(guard.compositor());
            return Err(SessionError::Failed(format!(
                "timed out waiting for {}",
                socket.display()
            )));
        }
        if signalled() {
            let _ = shutdown_child(guard.compositor());
            return Err(SessionError::Interrupted);
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // Xwayland may still be starting when the Wayland socket appears; wait
    // briefly for its DISPLAY hand-off so X11 launched apps get it (T-06).
    // Absent means Xwayland is unavailable (Wayland-only session).
    let display_path = runtime_dir.join(format!("{socket_name}.x11-display"));
    let display = wait_for_x11_display(&display_path, Duration::from_secs(5));
    if let Some(display) = &display {
        println!("dragonfruit dev: DISPLAY={display}");
    }

    Ok(Session {
        guard,
        socket,
        display_path,
        display,
    })
}

/// Environment shared by every client launched against the private socket.
fn launch_envs(socket_name: &str, display: Option<&str>) -> Vec<(&'static str, String)> {
    let mut envs = vec![
        ("WAYLAND_DISPLAY", socket_name.to_string()),
        ("XDG_CURRENT_DESKTOP", DESKTOP_NAME.to_string()),
    ];
    if let Some(display) = display {
        envs.push(("DISPLAY", display.to_string()));
    }
    envs
}

/// Spawn one child, record it under `label` in the guard, and report a
/// launch failure to the caller. Returns `true` on success.
fn launch_program(
    guard: &mut ChildGuard,
    label: &str,
    program: &Path,
    args: &[String],
    envs: &[(&str, String)],
) -> bool {
    let mut command = Command::new(program);
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    match command.spawn() {
        Ok(child) => {
            println!("dragonfruit dev: launched {label}");
            guard.launched.push((label.to_string(), child));
            true
        }
        Err(e) => {
            eprintln!(
                "dragonfruit dev: failed to launch {label} ({}): {e}",
                program.display()
            );
            false
        }
    }
}

/// Launch the shell with the one-time token the compositor provisioned at
/// startup (T-09). Returns `false` if the shell could not be launched.
fn launch_shell(guard: &mut ChildGuard, socket_name: &str, runtime_dir: &Path) -> bool {
    let shell = match shell_path() {
        Ok(shell) => shell,
        Err(e) => {
            eprintln!("dragonfruit dev: {e}");
            return false;
        }
    };
    let token_path = runtime_dir.join(format!("{socket_name}.launch-token"));
    let Some(token) = wait_for_token(&token_path, Duration::from_secs(5)) else {
        eprintln!(
            "dragonfruit dev: no launch token at {} — shell not started",
            token_path.display()
        );
        return false;
    };
    let args = vec![
        "--socket-name".to_string(),
        socket_name.to_string(),
        "--menubar-height".to_string(),
        "28".to_string(),
        "--placeholders".to_string(),
    ];
    let envs = [
        ("WAYLAND_DISPLAY", socket_name.to_string()),
        ("XDG_CURRENT_DESKTOP", DESKTOP_NAME.to_string()),
        ("DRAGONFRUIT_LAUNCH_TOKEN", token),
        // The shell manages its own Wayland connection and renders QML
        // offscreen into the chrome surface.
        ("QT_QPA_PLATFORM", "offscreen".to_string()),
    ];
    launch_program(guard, "shell", &shell, &args, &envs)
}

/// Tear every child down, verify no socket or process leaked, and return
/// `true` when the teardown was dirty.
fn teardown_session(session: &mut Session) -> bool {
    // Only a *new* signal during teardown counts as "user wants out now";
    // the signal that triggered the shutdown must not skip the grace period.
    SIGNALLED.store(false, Ordering::SeqCst);

    let mut dirty = false;
    for (name, mut child) in std::mem::take(&mut session.guard.launched) {
        if shutdown_child(&mut child).is_err() {
            eprintln!("dragonfruit dev: {name:?} refused to die");
            dirty = true;
        }
    }

    {
        let child = session.guard.compositor();
        if child.try_wait().map_or(true, |s| s.is_none()) && shutdown_child(child).is_err() {
            eprintln!("dragonfruit dev: compositor refused to shut down cleanly");
            dirty = true;
        }
    }
    // Disarm the guard: teardown is complete, nothing left to kill.
    session.guard.compositor = None;

    // Teardown verification: no stray socket may survive (FR-5).
    if session.socket.exists() {
        let _ = std::fs::remove_file(&session.socket);
        let _ = std::fs::remove_file(session.socket.with_extension("lock"));
        eprintln!(
            "dragonfruit dev: DIRTY TEARDOWN — socket {} survived exit",
            session.socket.display()
        );
        dirty = true;
    }
    // The Xwayland DISPLAY hand-off file is session state; the compositor
    // removes it, but clean it up here too if a hard kill left it behind.
    let _ = std::fs::remove_file(&session.display_path);
    let strays = soak::dragonfruit_processes();
    if !strays.is_empty() {
        eprintln!("dragonfruit dev: DIRTY TEARDOWN — stray processes: {strays:?}");
        dirty = true;
    }
    dirty
}

/// Block until the compositor exits or a signal arrives.
fn wait_for_compositor_exit(session: &mut Session, on_signal: &str) {
    loop {
        if session
            .guard
            .compositor()
            .try_wait()
            .is_ok_and(|status| status.is_some())
        {
            break;
        }
        if signalled() {
            println!("{on_signal}");
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn report_teardown(dirty: bool, clean_message: &str) -> ExitCode {
    if dirty {
        ExitCode::from(EXIT_DIRTY)
    } else {
        println!("{clean_message}");
        ExitCode::SUCCESS
    }
}

/// Run one development session: compositor + optional launched programs,
/// all against a private socket, all torn down on exit.
fn run_dev_session(args: &DevArgs) -> ExitCode {
    let Ok(runtime_dir) = runtime_dir() else {
        return ExitCode::from(EXIT_USAGE);
    };
    let mut session = match start_session(args.backend, &args.socket_name, &runtime_dir) {
        Ok(session) => session,
        Err(SessionError::Interrupted) => return ExitCode::from(130),
        Err(SessionError::Failed(message)) => {
            eprintln!("dragonfruit dev: {message}");
            return ExitCode::FAILURE;
        }
    };

    if args.shell {
        launch_shell(&mut session.guard, &args.socket_name, &runtime_dir);
    }

    for cmd in &args.launch {
        let Some(program) = cmd.first() else { continue };
        let envs = launch_envs(&args.socket_name, session.display.as_deref());
        launch_program(
            &mut session.guard,
            &cmd.join(" "),
            Path::new(program),
            &cmd[1..],
            &envs,
        );
    }

    wait_for_compositor_exit(&mut session, "dragonfruit dev: shutting down");

    let dirty = teardown_session(&mut session);
    report_teardown(
        dirty,
        "dragonfruit dev: clean teardown — host session undisturbed",
    )
}

/// The `make demo` harness (T-01.6a). Builds nothing (the Makefile does),
/// but launches the shell, a Qt/Wayland app, and an X11 app, prints the
/// checklist, and runs either the nested human walkthrough or the headless
/// scripted half that CI uses.
fn run_demo_session(args: &DevArgs) -> ExitCode {
    let Ok(runtime_dir) = runtime_dir() else {
        return ExitCode::from(EXIT_USAGE);
    };

    // Pick the backend: an explicit flag wins; otherwise nested when a host
    // Wayland session exists, headless (the CI path) when it does not.
    let backend = if args.backend_explicit {
        args.backend
    } else if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
        "nested"
    } else {
        "headless"
    };
    let scripted = backend == "headless";

    let qt = demo::qt_app();
    let x11 = demo::x11_app();
    print!(
        "{}",
        demo::checklist(&args.socket_name, qt.as_ref(), x11.as_ref(), scripted)
    );

    let Some(qt) = qt else {
        eprintln!(
            "dragonfruit demo: Qt app not found under build/apps — run `make build` \
             or set DF_DEMO_QT_APP"
        );
        return ExitCode::FAILURE;
    };

    let mut session = match start_session(backend, &args.socket_name, &runtime_dir) {
        Ok(session) => session,
        Err(SessionError::Interrupted) => return ExitCode::from(130),
        Err(SessionError::Failed(message)) => {
            eprintln!("dragonfruit dev: {message}");
            return ExitCode::FAILURE;
        }
    };

    let mut dirty = false;

    if !launch_shell(&mut session.guard, &args.socket_name, &runtime_dir) {
        dirty = true;
    }

    // The Qt app is a normal Wayland client of the private socket. It needs
    // the build-tree QML import root (the shell bakes this in at compile
    // time; the apps do not). Software rendering keeps the headless CI path
    // independent of a GPU.
    let mut qt_envs = launch_envs(&args.socket_name, None);
    qt_envs.push(("QT_QPA_PLATFORM", "wayland".to_string()));
    qt_envs.push((
        "QML_IMPORT_PATH",
        demo::qml_import_path().display().to_string(),
    ));
    if scripted {
        qt_envs.push(("QT_QUICK_BACKEND", "software".to_string()));
        qt_envs.push(("LIBGL_ALWAYS_SOFTWARE", "1".to_string()));
    }
    if !launch_program(&mut session.guard, "Qt/Wayland app", &qt, &[], &qt_envs) {
        dirty = true;
    }

    // The X11 app only makes sense once Xwayland is up; a Wayland-only
    // session is valid, so skip it with a note rather than failing.
    match (session.display.clone(), &x11) {
        (Some(display), Some(app)) => {
            // Pin the X11 window to the top-right corner (negative offsets are
            // measured from the output's far edges, so this is independent of
            // the output size). The compositor honors the ICCCM
            // user-specified position, so the X11 demo window no longer lands
            // on top of the centered Settings window.
            let args = vec![
                "-geometry".to_string(),
                // content y=80 keeps the compositor's 40px SSD titlebar
                // (drawn above the content) clear of the 28px menu bar.
                "320x160-40+80".to_string(),
                "-title".to_string(),
                "Dragonfruit X11".to_string(),
                "Dragonfruit X11 demo window".to_string(),
            ];
            let envs = [
                ("DISPLAY", display),
                ("XDG_CURRENT_DESKTOP", DESKTOP_NAME.to_string()),
            ];
            if !launch_program(&mut session.guard, "X11 app", app, &args, &envs) {
                dirty = true;
            }
        }
        (None, _) => eprintln!(
            "dragonfruit demo: Xwayland is unavailable; skipping the X11 half \
             (a Wayland-only session is valid)"
        ),
        (Some(_), None) => eprintln!(
            "dragonfruit demo: no X11 app found (install xmessage or set \
             DF_DEMO_X11_APP); skipping the X11 half"
        ),
    }

    // Any extra `--launch` programs.
    for cmd in &args.launch {
        let Some(program) = cmd.first() else { continue };
        let envs = launch_envs(&args.socket_name, session.display.as_deref());
        if !launch_program(
            &mut session.guard,
            &cmd.join(" "),
            Path::new(program),
            &cmd[1..],
            &envs,
        ) {
            dirty = true;
        }
    }

    if scripted {
        // Scripted half (CI): let the shell and clients map, assert every
        // child is still alive, then tear down and check for leaks.
        println!(
            "dragonfruit demo: scripted half — settling {} ms",
            demo::SCRIPTED_SETTLE.as_millis()
        );
        std::thread::sleep(demo::SCRIPTED_SETTLE);
        if session
            .guard
            .compositor()
            .try_wait()
            .is_ok_and(|status| status.is_some())
        {
            eprintln!("dragonfruit demo: compositor exited during the scripted half");
            dirty = true;
        }
        let dead = session.guard.dead_children();
        for message in &dead {
            eprintln!("dragonfruit demo: {message}");
        }
        if !dead.is_empty() {
            dirty = true;
        } else if !dirty {
            println!("dragonfruit demo: all children alive after settle");
        }
    } else {
        // Nested human walkthrough: block until the Dragonfruit window is
        // closed (or Ctrl-C). A client that exits with a failure *while the
        // compositor is still running* is a crash; once the compositor is
        // gone, every client exit is just the lost connection — the
        // documented way to end the session — so those are not failures.
        let mut reported: std::collections::HashSet<String> = std::collections::HashSet::new();
        loop {
            if session
                .guard
                .compositor()
                .try_wait()
                .is_ok_and(|status| status.is_some())
            {
                break;
            }
            if signalled() {
                println!("dragonfruit demo: shutting down");
                break;
            }
            for message in session.guard.failed_children() {
                if reported.insert(message.clone()) {
                    eprintln!("dragonfruit demo: {message}");
                    dirty = true;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    if teardown_session(&mut session) {
        dirty = true;
    }

    if scripted {
        report_teardown(
            dirty,
            "dragonfruit demo: scripted half OK — shell + clients launched, clean teardown",
        )
    } else {
        report_teardown(
            dirty,
            "dragonfruit demo: clean teardown — host session undisturbed",
        )
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
    use super::*;

    #[test]
    fn socket_names_stay_within_unix_limits() {
        let name = format!("dragonfruit-dev-{}", std::process::id());
        assert!(name.len() < 100);
        assert!(name.starts_with(df_ipc::DESKTOP_NAME));
    }

    #[test]
    fn repeated_launch_flags_each_start_a_program() {
        let args = parse_dev_args(
            [
                "--demo",
                "--headless",
                "--launch",
                "app-a",
                "--flag",
                "--launch",
                "app-b",
                "arg",
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .expect("parse");
        assert!(args.demo);
        assert!(args.backend_explicit);
        assert_eq!(args.backend, "headless");
        assert_eq!(
            args.launch,
            vec![
                vec!["app-a".to_string(), "--flag".to_string()],
                vec!["app-b".to_string(), "arg".to_string()],
            ]
        );
    }

    #[test]
    fn demo_without_a_backend_flag_is_not_explicit() {
        let args = parse_dev_args(["--demo"].iter().map(|s| s.to_string())).expect("parse");
        assert!(args.demo);
        assert!(!args.backend_explicit);
    }
}
