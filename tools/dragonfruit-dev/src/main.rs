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
//!
//! A nested session is either **thin** (`--services core`, the loop demo's
//! Dock identity + global menu) or **full** (`--services full`, adding
//! settingsd, system-status, notifications, and wallpaperd). `--private-bus`
//! plus `--fixtures` make a full session safe and populated next to another
//! desktop; see ADR 0123 and `make dev-full`.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use df_ipc::{DESKTOP_NAME, LOCKSTEP_VERSION};

mod demo;
mod session_selector;
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

/// The session-service set a nested dev session starts.
///
/// `None` is the headless/CI path (no services, no session-bus dependency).
/// `Core` is the thin loop demo: `app-index` (Dock identity) and `menu-broker`
/// (global menu), which the shell's chrome needs. `Full` adds the real session
/// services so Settings, Control Center, notifications, and the wallpaper
/// provider are live. `wallpaperd` is `Full`-only: it is a network-capable
/// content provider and does not belong on the thin/CI path.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ServiceSet {
    None,
    Core,
    Full,
}

impl ServiceSet {
    fn parse(value: &str) -> Option<ServiceSet> {
        match value {
            "none" => Some(ServiceSet::None),
            "core" => Some(ServiceSet::Core),
            "full" => Some(ServiceSet::Full),
            _ => None,
        }
    }

    /// Service binary names in launch order. `settingsd` is first so it owns
    /// its bus name before the shell binds Theme/Dock to it.
    fn names(self) -> Vec<&'static str> {
        match self {
            ServiceSet::None => Vec::new(),
            ServiceSet::Core => vec!["dragonfruit-app-index", "dragonfruit-menu-broker"],
            ServiceSet::Full => vec![
                "dragonfruit-settingsd",
                "dragonfruit-app-index",
                "dragonfruit-menu-broker",
                "dragonfruit-system-status",
                "dragonfruit-notifications",
                "dragonfruit-wallpaperd",
            ],
        }
    }

    /// The shell-critical bus names to wait for before launching the shell.
    /// `wallpaperd` and `notifications` are intentionally absent: the shell
    /// does not block on them.
    fn bus_names(self) -> Vec<&'static str> {
        match self {
            ServiceSet::None => Vec::new(),
            ServiceSet::Core => vec!["org.dragonfruit.AppIndex1", "org.dragonfruit.MenuBroker1"],
            ServiceSet::Full => vec![
                "org.dragonfruit.Settings1",
                "org.dragonfruit.AppIndex1",
                "org.dragonfruit.MenuBroker1",
                "org.dragonfruit.SystemStatus1",
            ],
        }
    }
}

#[derive(Debug)]
struct DevArgs {
    backend: &'static str,
    backend_explicit: bool,
    socket_name: String,
    launch: Vec<Vec<String>>,
    soak_cycles: Option<usize>,
    shell: bool,
    demo: bool,
    /// `None` means "auto": `Core` for a live session, `None` for the
    /// headless scripted half.
    services: Option<ServiceSet>,
    private_bus: bool,
    fixtures: bool,
}

fn usage() -> String {
    format!(
        "dragonfruit dev tool (lockstep-ipc=v{LOCKSTEP_VERSION}, desktop={DESKTOP_NAME})

USAGE:
    dragonfruit dev --nested [--socket-name NAME] [--shell] [--launch CMD...]
    dragonfruit dev --headless [--socket-name NAME] [--shell] [--launch CMD...]
    dragonfruit dev --demo [--nested|--headless] [--socket-name NAME]
    dragonfruit dev --soak [N] [--nested|--headless|--drm]
                                      # teardown soak (default 100 headless cycles)
    dragonfruit version

--shell launches the built shell process (build/shell/src/dragonfruit-shell,
or DF_SHELL_BIN) against the private socket.

--services none|core|full selects the session services to start. `core` is
`dragonfruit-app-index` and `dragonfruit-menu-broker` (the shell chrome the
thin loop needs); `full` also starts `dragonfruit-settingsd`,
`dragonfruit-system-status`, `dragonfruit-notifications`, and
`dragonfruit-wallpaperd`. The headless scripted demo always uses `none`.

--private-bus runs a private `dbus-daemon --session` and points
XDG_CONFIG_HOME/XDG_CACHE_HOME/XDG_STATE_HOME at a scratch directory under
the runtime dir, so an isolated session never touches the host desktop's
daemons or the developer's real settings. Falls back to the inherited bus
(with a warning) when `dbus-daemon` is missing.

--fixtures exports the deterministic `DF_*_FIXTURE` pane fixtures to the
session, so Settings and Control Center render populated without host
hardware or daemons.

--demo builds nothing (use `make demo`), but launches the shell, a Qt/Wayland
app, and an X11 app against a private socket and prints the T-01 checklist.
With a host Wayland session it runs nested for the human walkthrough; with
none (CI) it runs the headless scripted half: launch, settle, verify every
child is alive, tear down, and assert no socket leaked. `--launch` may be
repeated to add extra programs.

--soak runs the teardown gate: N compositor sessions, each asserting no
leaked socket, lock, launch token, or orphan. It defaults to the headless
backend; `--nested` and `--drm` run the same gate on those backends (one
`--soak 1 --drm` is the DRM session cycle in the T-03.4 runbook)."
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
        services: None,
        private_bus: false,
        fixtures: false,
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
            "--drm" => {
                args.backend = "drm";
                args.backend_explicit = true;
            }
            "--shell" => args.shell = true,
            "--demo" => args.demo = true,
            "--private-bus" => args.private_bus = true,
            "--fixtures" => args.fixtures = true,
            "--services" => {
                let Some(value) = it.next() else {
                    return Err("--services requires one of none|core|full".into());
                };
                let Some(set) = ServiceSet::parse(&value) else {
                    return Err(format!(
                        "unknown --services value {value:?}: expected none|core|full"
                    ));
                };
                args.services = Some(set);
            }
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
                Some(cycles) => run_soak(cycles, &args),
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
    /// Best-effort session services (app-index, menu-broker, wallpaperd). Unlike
    /// `launched`, their early exit is not a demo failure: the shell degrades
    /// gracefully when one is absent. They are still owned here so teardown
    /// can never leak them into the host session.
    services: Vec<(String, std::process::Child)>,
    /// A private session bus started by `--private-bus`, when one was used.
    /// Owned so an isolated session can never leak its daemon.
    bus: Option<std::process::Child>,
}

impl ChildGuard {
    fn new(compositor: std::process::Child) -> Self {
        ChildGuard {
            compositor: Some(compositor),
            launched: Vec::new(),
            services: Vec::new(),
            bus: None,
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
        for (_, mut child) in self.services.drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(mut child) = self.bus.take() {
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
    // Provision the Files-owned desktop's one-time token (T-19.3) so the
    // compositor writes `<socket>.desktop-launch-token`; the desktop process
    // then reads the hand-off file, exactly like the shell reads its own.
    let child = Command::new(&compositor)
        .arg("--backend")
        .arg(backend)
        .arg("--socket-name")
        .arg(socket_name)
        .env("DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN", random_token_hex())
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

/// The extra environment the dev-tree shell and app-index need so the staged
/// first-party `.desktop` entries resolve: the built app directories on `PATH`
/// (a launched first-party app must start) and a scratch `XDG_DATA_DIRS` with
/// the in-repo `.desktop` entries copied in (nothing is installed). A
/// production install provides both. Returns `(key, value)` pairs.
fn dev_share_env(runtime_dir: &Path) -> Vec<(&'static str, String)> {
    let mut envs = Vec::new();
    let app_dirs = demo::built_app_dirs();
    if !app_dirs.is_empty() {
        let mut parts = app_dirs;
        parts.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        if let Ok(path) = std::env::join_paths(parts) {
            envs.push(("PATH", path.to_string_lossy().into_owned()));
        }
    }
    let share = runtime_dir.join("df-dev-share");
    if demo::stage_first_party_desktop_entries(&share) {
        let existing = std::env::var_os("XDG_DATA_DIRS")
            .unwrap_or_else(|| std::ffi::OsString::from("/usr/local/share:/usr/share"));
        envs.push((
            "XDG_DATA_DIRS",
            format!("{}:{}", share.to_string_lossy(), existing.to_string_lossy()),
        ));
    }
    envs
}

/// Path to one of the session services the shell talks to, as a sibling of the
/// dev tool in the cargo target directory (both are built by the workspace).
/// `None` when it is not built, in which case the shell degrades gracefully.
fn service_path(name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let sibling = exe.parent()?.join(name);
    sibling.is_file().then_some(sibling)
}

/// The deterministic pane fixtures the session can opt into with `--fixtures`.
/// They are read by the shell (`DF_STATUS_FIXTURE`) and the Settings app, and
/// are inherited by apps the shell launches.
fn fixture_envs() -> Vec<(&'static str, &'static str)> {
    [
        "DF_STATUS_FIXTURE",
        "DF_SETTINGS_FIXTURE",
        "DF_BLUETOOTH_FIXTURE",
        "DF_STORAGE_FIXTURE",
        "DF_SOUND_FIXTURE",
        "DF_INPUT_FIXTURE",
        "DF_BATTERY_FIXTURE",
        "DF_NOTIFICATIONS_FIXTURE",
        "DF_UPDATES_FIXTURE",
        "DF_ACCOUNTS_FIXTURE",
        "DF_PRINTERS_FIXTURE",
        "DF_PRIVACY_FIXTURE",
        "DF_WALLPAPER_FIXTURE",
    ]
    .into_iter()
    .map(|key| (key, "1"))
    .collect()
}

/// Point the session's XDG config/cache/state at a scratch directory under the
/// runtime dir, so an isolated session never reads or writes the developer's
/// real desktop settings (`settingsd`) or wallpaper cache (`wallpaperd`).
fn apply_scratch_xdg(runtime_dir: &Path, socket_name: &str) {
    let base = runtime_dir.join(format!("{socket_name}.session"));
    let config = base.join("config");
    let cache = base.join("cache");
    let state = base.join("state");
    for dir in [&config, &cache, &state] {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("dragonfruit dev: could not create {}: {e}", dir.display());
            return;
        }
    }
    std::env::set_var("XDG_CONFIG_HOME", &config);
    std::env::set_var("XDG_CACHE_HOME", &cache);
    std::env::set_var("XDG_STATE_HOME", &state);
    println!("dragonfruit dev: scratch XDG dirs under {}", base.display());
}

/// Start a private session bus for an isolated full session and export its
/// address to every child spawned afterwards. Returns `true` when a private
/// bus is now in use; `false` falls back to the inherited bus (with a warning,
/// still using the scratch XDG dirs). Best-effort: a missing `dbus-daemon`
/// never blocks the session.
fn start_private_bus(guard: &mut ChildGuard, runtime_dir: &Path, socket_name: &str) -> bool {
    apply_scratch_xdg(runtime_dir, socket_name);
    if demo::which("dbus-daemon").is_none() {
        eprintln!("dragonfruit dev: dbus-daemon not found; using the inherited session bus");
        return false;
    }
    let mut child = match Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!("dragonfruit dev: failed to start a private bus: {e}");
            return false;
        }
    };
    let address = child.stdout.take().and_then(|stdout| {
        use std::io::{BufRead, BufReader};
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).ok()?;
        let line = line.trim().to_string();
        (!line.is_empty()).then_some(line)
    });
    let Some(address) = address else {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!("dragonfruit dev: private bus printed no address; using the inherited bus");
        return false;
    };
    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &address);
    println!("dragonfruit dev: private session bus at {address}");
    guard.bus = Some(child);
    true
}

/// One `NameHasOwner` probe against the session bus through `gdbus` (the dev
/// tool links no D-Bus crate). `None` when `gdbus` is unavailable.
fn bus_name_has_owner(name: &str) -> Option<bool> {
    let gdbus = demo::which("gdbus")?;
    let output = Command::new(gdbus)
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.NameHasOwner",
            name,
        ])
        .output()
        .ok()?;
    Some(output.status.success() && String::from_utf8_lossy(&output.stdout).contains("true"))
}

/// Wait for the shell-critical service names to appear so the shell's
/// one-shot startup identity load and Theme/Dock bind see them. Falls back to
/// a short settle when `gdbus` is unavailable.
fn wait_for_services_ready(set: ServiceSet, timeout: Duration) {
    let names = set.bus_names();
    if names.is_empty() {
        return;
    }
    if demo::which("gdbus").is_none() {
        std::thread::sleep(Duration::from_millis(500));
        return;
    }
    for name in names {
        let deadline = Instant::now() + timeout;
        let mut ready = false;
        while Instant::now() < deadline {
            if bus_name_has_owner(name) == Some(true) {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if !ready {
            eprintln!(
                "dragonfruit dev: warning: {name} did not appear within {}s",
                timeout.as_secs()
            );
        }
    }
}

/// Start the session services selected by `set` and wait for the ones the
/// shell depends on. All are best-effort: a missing binary or a missing
/// session bus is reported and skipped, and the shell falls back (an empty
/// Dock / its own fixed application menu / defaults).
fn launch_services(guard: &mut ChildGuard, socket_name: &str, runtime_dir: &Path, set: ServiceSet) {
    if set == ServiceSet::None {
        return;
    }
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        eprintln!("dragonfruit dev: no session bus; skipping session services");
        return;
    }
    let share = dev_share_env(runtime_dir);
    let mut base: Vec<(&str, String)> = vec![
        ("WAYLAND_DISPLAY", socket_name.to_string()),
        ("XDG_CURRENT_DESKTOP", DESKTOP_NAME.to_string()),
    ];
    for (key, value) in &share {
        base.push((*key, value.clone()));
    }
    let mut launched = false;
    for name in set.names() {
        let Some(path) = service_path(name) else {
            eprintln!("dragonfruit dev: {name} not built; the shell runs without it");
            continue;
        };
        let mut command = Command::new(&path);
        for (key, value) in &base {
            command.env(*key, value);
        }
        match command.spawn() {
            Ok(child) => {
                println!("dragonfruit dev: launched {name}");
                guard.services.push((name.to_string(), child));
                launched = true;
            }
            Err(e) => eprintln!("dragonfruit dev: failed to launch {name}: {e}"),
        }
    }
    if launched {
        wait_for_services_ready(set, Duration::from_secs(5));
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
    ];
    let mut envs = vec![
        ("WAYLAND_DISPLAY", socket_name.to_string()),
        ("XDG_CURRENT_DESKTOP", DESKTOP_NAME.to_string()),
        ("DRAGONFRUIT_LAUNCH_TOKEN", token),
        // The shell manages its own Wayland connection and renders QML
        // offscreen into the chrome surface.
        ("QT_QPA_PLATFORM", "offscreen".to_string()),
    ];
    // The lock screen authenticates through the small PAM helper (T-12.3b).
    // It is a sibling of the dev tool in the cargo target directory; without
    // it the shell falls back to searching `PATH`.
    if let Some(helper) = pam_helper_path() {
        envs.push(("DF_PAM_HELPER", helper.to_string_lossy().into_owned()));
    }
    // The shell launches apps through their `.desktop` Exec (e.g.
    // `dragonfruit-settings`). The dev tree installs nothing, so expose the
    // built app directories and the build's QML modules to the shell so a
    // launched first-party app can start. A production install provides both.
    for (key, value) in dev_share_env(runtime_dir) {
        envs.push((key, value));
    }
    envs.push((
        "QML_IMPORT_PATH",
        std::fs::canonicalize(demo::qml_import_path())
            .unwrap_or_else(|_| demo::qml_import_path())
            .to_string_lossy()
            .into_owned(),
    ));
    launch_program(guard, "shell", &shell, &args, &envs)
}

/// 32 random bytes as hex, for the compositor's `desktop:` launch token. The
/// token is a dev/demo convenience; a production session mints it through the
/// session manager. Falls back to a time-seeded value when `/dev/urandom` is
/// unavailable.
fn random_token_hex() -> String {
    use std::io::Read;
    let mut bytes = [0u8; 32];
    let random = std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .is_ok();
    if !random {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
            ^ (std::process::id() as u128);
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = ((seed >> ((i % 16) * 8)) & 0xff) as u8;
        }
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Path to the built Files binary, which also implements the desktop process
/// (`dragonfruit-files --desktop`). `DF_DESKTOP_APP` overrides. `None` when it
/// is not built, in which case the desktop half is skipped.
fn desktop_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("DF_DESKTOP_APP") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    let path = PathBuf::from("build/apps/files/dragonfruit-files");
    path.is_file().then_some(path)
}

/// Launch the Files-owned desktop process (T-19.3): its own process on the
/// compositor's background layer, provisioned with the desktop launch token.
/// Its own crash domain — its death never stops the shell or the browser.
fn launch_desktop(guard: &mut ChildGuard, socket_name: &str, runtime_dir: &Path) -> bool {
    let Some(desktop) = desktop_path() else {
        eprintln!("dragonfruit dev: dragonfruit-files not built; desktop not started");
        return false;
    };
    let token_path = runtime_dir.join(format!("{socket_name}.desktop-launch-token"));
    let Some(token) = wait_for_token(&token_path, Duration::from_secs(5)) else {
        eprintln!(
            "dragonfruit dev: no desktop launch token at {} — desktop not started",
            token_path.display()
        );
        return false;
    };
    let args = vec!["--desktop".to_string()];
    let mut envs = vec![
        ("WAYLAND_DISPLAY", socket_name.to_string()),
        ("XDG_CURRENT_DESKTOP", DESKTOP_NAME.to_string()),
        ("DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN", token),
        // The desktop speaks the private protocol itself and renders QML
        // offscreen into its buffer, exactly like the shell chrome.
        ("QT_QPA_PLATFORM", "offscreen".to_string()),
        ("QT_QUICK_BACKEND", "software".to_string()),
    ];
    for (key, value) in dev_share_env(runtime_dir) {
        envs.push((key, value));
    }
    envs.push((
        "QML_IMPORT_PATH",
        std::fs::canonicalize(demo::qml_import_path())
            .unwrap_or_else(|_| demo::qml_import_path())
            .to_string_lossy()
            .into_owned(),
    ));
    launch_program(guard, "files-desktop", &desktop, &args, &envs)
}

/// Path to `dragonfruit-pam-helper`: `DF_PAM_HELPER` if set, otherwise a
/// sibling of this binary (both are built by the cargo workspace). `None`
/// when neither exists, in which case the shell resolves it on `PATH`.
fn pam_helper_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("DF_PAM_HELPER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let sibling = exe.parent()?.join("dragonfruit-pam-helper");
    sibling.is_file().then_some(sibling)
}

/// Tear every child down, verify no socket or process leaked, and return
/// `true` when the teardown was dirty.
fn teardown_session(session: &mut Session) -> bool {
    // Only a *new* signal during teardown counts as "user wants out now";
    // the signal that triggered the shutdown must not skip the grace period.
    SIGNALLED.store(false, Ordering::SeqCst);

    let mut dirty = false;
    // Best-effort services first: their exit is never a failure, but they must
    // not outlive the session.
    for (name, mut child) in std::mem::take(&mut session.guard.services) {
        if shutdown_child(&mut child).is_err() {
            eprintln!("dragonfruit dev: service {name:?} refused to die");
            dirty = true;
        }
    }
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
    // The private bus, if any, goes last: every child has been signalled.
    if let Some(mut bus) = session.guard.bus.take() {
        let _ = shutdown_child(&mut bus);
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

/// Export the deterministic pane fixtures to this process so every child
/// (shell, demo apps, services) inherits them.
fn apply_fixtures(args: &DevArgs) {
    if !args.fixtures {
        return;
    }
    for (key, value) in fixture_envs() {
        std::env::set_var(key, value);
    }
    println!("dragonfruit dev: deterministic pane fixtures enabled");
}

/// Run one development session: compositor + optional launched programs,
/// all against a private socket, all torn down on exit.
fn run_dev_session(args: &DevArgs) -> ExitCode {
    let Ok(runtime_dir) = runtime_dir() else {
        return ExitCode::from(EXIT_USAGE);
    };
    apply_fixtures(args);
    let mut session = match start_session(args.backend, &args.socket_name, &runtime_dir) {
        Ok(session) => session,
        Err(SessionError::Interrupted) => return ExitCode::from(130),
        Err(SessionError::Failed(message)) => {
            eprintln!("dragonfruit dev: {message}");
            return ExitCode::FAILURE;
        }
    };

    if args.shell {
        if args.private_bus {
            start_private_bus(&mut session.guard, &runtime_dir, &args.socket_name);
        }
        let set = args.services.unwrap_or(ServiceSet::Core);
        launch_services(&mut session.guard, &args.socket_name, &runtime_dir, set);
        launch_shell(&mut session.guard, &args.socket_name, &runtime_dir);
        launch_desktop(&mut session.guard, &args.socket_name, &runtime_dir);
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
    apply_fixtures(args);

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

    // The thin loop starts only the core services (Dock identity + global
    // menu); the headless scripted half starts none so CI never depends on a
    // session bus, and `--services full` (the `make dev-full` session) opts
    // into settingsd, system-status, notifications, and the wallpaper
    // provider. Start them before the shell, which loads the app corpus once
    // at startup and binds Theme/Dock to settingsd.
    if args.private_bus && !scripted {
        start_private_bus(&mut session.guard, &runtime_dir, &args.socket_name);
    }
    let set = if scripted {
        ServiceSet::None
    } else {
        args.services.unwrap_or(ServiceSet::Core)
    };
    launch_services(&mut session.guard, &args.socket_name, &runtime_dir, set);
    if !launch_shell(&mut session.guard, &args.socket_name, &runtime_dir) {
        dirty = true;
    }
    // The Files-owned desktop (T-19.3) is its own process on the background
    // layer. A missing binary/token is a soft skip, not a demo failure.
    launch_desktop(&mut session.guard, &args.socket_name, &runtime_dir);

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

fn run_soak(cycles: usize, args: &DevArgs) -> ExitCode {
    let Ok(compositor) = compositor_path() else {
        return ExitCode::from(EXIT_USAGE);
    };
    let Ok(runtime_dir) = runtime_dir() else {
        return ExitCode::from(EXIT_USAGE);
    };
    // The gate is headless by default (CI needs no display); an explicit
    // `--nested`/`--drm` runs the same teardown check on that backend.
    let backend = if args.backend_explicit {
        args.backend
    } else {
        "headless"
    };
    match soak::run(&compositor, &runtime_dir, cycles, backend) {
        Ok(()) => {
            println!(
                "dragonfruit dev: soak passed — {cycles} clean {backend} cycles, \
                 zero strays, zero leaked sockets/tokens"
            );
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

    #[test]
    fn service_sets_are_none_core_and_full() {
        assert!(ServiceSet::None.names().is_empty());
        assert_eq!(
            ServiceSet::Core.names(),
            vec!["dragonfruit-app-index", "dragonfruit-menu-broker"]
        );
        let full = ServiceSet::Full.names();
        for name in [
            "dragonfruit-settingsd",
            "dragonfruit-app-index",
            "dragonfruit-menu-broker",
            "dragonfruit-system-status",
            "dragonfruit-notifications",
            "dragonfruit-wallpaperd",
        ] {
            assert!(full.contains(&name), "full set is missing {name}: {full:?}");
        }
    }

    #[test]
    fn wallpaperd_is_full_only_and_never_on_the_thin_or_ci_path() {
        assert!(!ServiceSet::Core.names().contains(&"dragonfruit-wallpaperd"));
        assert!(!ServiceSet::None.names().contains(&"dragonfruit-wallpaperd"));
        assert!(ServiceSet::Full.names().contains(&"dragonfruit-wallpaperd"));
    }

    #[test]
    fn shell_critical_bus_names_track_the_service_set() {
        assert!(ServiceSet::None.bus_names().is_empty());
        assert!(!ServiceSet::Core
            .bus_names()
            .contains(&"org.dragonfruit.Settings1"));
        assert!(ServiceSet::Full
            .bus_names()
            .contains(&"org.dragonfruit.Settings1"));
        assert!(ServiceSet::Full
            .bus_names()
            .contains(&"org.dragonfruit.SystemStatus1"));
    }

    #[test]
    fn parse_services_private_bus_and_fixtures() {
        let args = parse_dev_args(
            [
                "--demo",
                "--services",
                "full",
                "--private-bus",
                "--fixtures",
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .expect("parse");
        assert_eq!(args.services, Some(ServiceSet::Full));
        assert!(args.private_bus);
        assert!(args.fixtures);
    }

    #[test]
    fn parse_rejects_an_unknown_services_value() {
        let err = parse_dev_args(["--services", "everything"].iter().map(|s| s.to_string()))
            .expect_err("unknown services value must fail");
        assert!(err.contains("none|core|full"), "{err}");
    }

    #[test]
    fn parse_defaults_to_auto_services() {
        let args = parse_dev_args(["--nested"].iter().map(|s| s.to_string())).expect("parse");
        assert_eq!(args.services, None);
        assert!(!args.private_bus);
        assert!(!args.fixtures);
    }
}
