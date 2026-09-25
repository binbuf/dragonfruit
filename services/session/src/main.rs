// SPDX-License-Identifier: MIT
//! `dragonfruit-session` — the T-12.1a session manager.
//!
//! The real entry point is the session planner; the environment, systemd
//! units, and second-VT workflow are T-12.1b. Three debug/ops surfaces ship:
//!
//! ```text
//! dragonfruit-session --print-plan            # the default composition, in order
//! dragonfruit-session --print-env             # the session environment to import
//! dragonfruit-session --wait-socket NAME      # block until the compositor's socket exists
//! dragonfruit-session --exec PROG [ARGS...] [--policy always|on-failure|never]
//! ```
//!
//! `--exec` supervises one process with the chosen restart policy until it
//! exits (a `never` service) or the session is signalled, which is the
//! smallest live demonstration of the restart seam. `--print-env` and
//! `--wait-socket` are the two seams the shipped systemd user units use
//! (T-12.1b): the session entry imports the environment, and each service
//! waits for the private socket before starting.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use dragonfruit_session::env::{self, SessionEnvironment};
use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const DEFAULT_SOCKET_WAIT: Duration = Duration::from_secs(10);

static TERMINATE: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_signal: libc::c_int) {
    TERMINATE.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("--print-plan") => {
            print_plan(&SessionPlan::default_session());
            ExitCode::SUCCESS
        }
        Some("--print-env") => print_env(&args[1..]),
        Some("--wait-socket") => wait_socket(&args[1..]),
        Some("-h") | Some("--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("--exec") => run_exec(&args[1..]),
        Some(other) => {
            eprintln!("dragonfruit-session: unknown argument {other:?}");
            print_help();
            ExitCode::from(2)
        }
    }
}

/// Print the plan in launch order: `stage<TAB>name<TAB>policy<TAB>command`.
fn print_plan(plan: &SessionPlan) {
    for stage in plan.stages() {
        for spec in plan.services_in_stage(stage) {
            let role = if spec.ends_session {
                "\tanchor"
            } else if spec.gate {
                "\tgate"
            } else {
                ""
            };
            println!(
                "{}\t{}\t{}\t{}{}",
                spec.stage,
                spec.name,
                spec.policy.as_str(),
                spec.command_line(),
                role
            );
        }
    }
}

/// The environment to import into the systemd user manager before starting
/// `dragonfruit-session.target`. Prints `KEY=VALUE` lines, including a freshly
/// minted `DRAGONFRUIT_LAUNCH_TOKEN`; the token is a session secret, so the
/// caller must import it rather than log it.
///
/// Options: `--socket-name NAME`, `--x11-display DISPLAY`.
fn print_env(args: &[String]) -> ExitCode {
    let mut socket_name = env::DEFAULT_SOCKET_NAME.to_string();
    let mut x11_display: Option<String> = None;
    let mut position = 0;
    while position < args.len() {
        match args[position].as_str() {
            "--socket-name" => {
                let Some(value) = args.get(position + 1) else {
                    eprintln!("dragonfruit-session: --socket-name needs a value");
                    return ExitCode::from(2);
                };
                socket_name = value.clone();
                position += 2;
            }
            "--x11-display" => {
                let Some(value) = args.get(position + 1) else {
                    eprintln!("dragonfruit-session: --x11-display needs a value");
                    return ExitCode::from(2);
                };
                x11_display = Some(value.clone());
                position += 2;
            }
            other => {
                eprintln!("dragonfruit-session: unknown --print-env argument {other:?}");
                return ExitCode::from(2);
            }
        }
    }

    let mut environment = SessionEnvironment::new(socket_name).with_generated_token();
    if let Some(display) = x11_display {
        environment = environment.with_x11_display(display);
    }
    // The trusted environment is the base plus the token; the base variables
    // are also what a non-trusted client sees.
    for (key, value) in environment.trusted() {
        println!("{key}={value}");
    }
    ExitCode::SUCCESS
}

/// Block until the compositor's private socket exists — the readiness gate
/// the shell and services need before they start (T-12.1a's `set_ready`, as a
/// unit `ExecStartPre`). `NAME` may be a bare socket name under
/// `$XDG_RUNTIME_DIR` or an absolute path. Options: `--timeout SECS` (default
/// 10).
fn wait_socket(args: &[String]) -> ExitCode {
    let mut name: Option<String> = None;
    let mut timeout = DEFAULT_SOCKET_WAIT;
    let mut position = 0;
    while position < args.len() {
        match args[position].as_str() {
            "--timeout" => {
                let Some(value) = args.get(position + 1).and_then(|v| v.parse().ok()) else {
                    eprintln!("dragonfruit-session: --timeout needs a number of seconds");
                    return ExitCode::from(2);
                };
                timeout = Duration::from_secs(value);
                position += 2;
            }
            other if name.is_none() => {
                name = Some(other.to_string());
                position += 1;
            }
            other => {
                eprintln!("dragonfruit-session: unexpected --wait-socket argument {other:?}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(name) = name else {
        eprintln!("dragonfruit-session: --wait-socket needs a socket name");
        return ExitCode::from(2);
    };

    let path = match socket_path(&name) {
        Some(path) => path,
        None => {
            eprintln!(
                "dragonfruit-session: XDG_RUNTIME_DIR is not set and {name:?} is not absolute"
            );
            return ExitCode::FAILURE;
        }
    };

    let deadline = Instant::now() + timeout;
    loop {
        if path.exists() {
            println!("dragonfruit-session: socket ready: {}", path.display());
            return ExitCode::SUCCESS;
        }
        if Instant::now() >= deadline {
            eprintln!(
                "dragonfruit-session: timed out waiting for socket {}",
                path.display()
            );
            return ExitCode::FAILURE;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Resolve a socket `name` to a path: an absolute path stands alone; a bare
/// name lives under `$XDG_RUNTIME_DIR`, which must be set.
fn socket_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name);
    if path.is_absolute() {
        return Some(path);
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join(path))
}

fn run_exec(args: &[String]) -> ExitCode {
    let mut policy = RestartPolicy::OnFailure;
    let mut command: Vec<String> = Vec::new();
    let mut position = 0;
    while position < args.len() {
        match args[position].as_str() {
            "--policy" => {
                let Some(value) = args.get(position + 1) else {
                    eprintln!("dragonfruit-session: --policy needs a value");
                    return ExitCode::from(2);
                };
                let Some(parsed) = RestartPolicy::parse(value) else {
                    eprintln!("dragonfruit-session: unknown policy {value:?}");
                    return ExitCode::from(2);
                };
                policy = parsed;
                position += 2;
            }
            other => {
                command.push(other.to_owned());
                position += 1;
            }
        }
    }

    let Some(program) = command.first().cloned() else {
        eprintln!("dragonfruit-session: --exec needs a program");
        return ExitCode::from(2);
    };
    let spec = ServiceSpec::new("exec", program)
        .args(command[1..].to_vec())
        .policy(policy);
    let mut supervisor = Supervisor::new(SessionPlan::new(vec![spec]));

    install_signal_handlers();
    if let Err(error) = supervisor.start() {
        eprintln!(
            "dragonfruit-session: cannot start {}: {error}",
            supervisor.plan().services[0].name
        );
        return ExitCode::FAILURE;
    }
    for line in supervisor.tick() {
        println!("{line}");
    }

    while supervisor.state() != SessionState::Ended && supervisor.any_running() {
        if TERMINATE.load(Ordering::SeqCst) {
            supervisor.shutdown();
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
        for line in supervisor.tick() {
            println!("{line}");
        }
    }
    for line in supervisor.tick() {
        println!("{line}");
    }
    if supervisor.service_state("exec") == Some(ServiceState::Failed) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_help() {
    println!(
        "dragonfruit-session — T-12.1a session manager\n\
         \n\
         Options:\n\
           --print-plan   print the default session composition in launch order\n\
           --print-env [--socket-name NAME] [--x11-display DISPLAY]\n\
                          print the session environment (base + launch token)\n\
                          for the systemd user manager to import\n\
           --wait-socket NAME [--timeout SECS]\n\
                          block until the compositor's private socket exists\n\
           --exec PROG [ARGS...] [--policy always|on-failure|never]\n\
                          supervise one process until it exits or is signalled\n\
           -h, --help     show this help"
    );
}
