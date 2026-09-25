// SPDX-License-Identifier: MIT
//! `dragonfruit-session` — the T-12.1a session manager.
//!
//! The real entry point is the session planner; the environment, systemd
//! units, and second-VT workflow are T-12.1b. Two debug/ops surfaces ship
//! now:
//!
//! ```text
//! dragonfruit-session --print-plan            # the default composition, in order
//! dragonfruit-session --exec PROG [ARGS...] [--policy always|on-failure|never]
//! ```
//!
//! `--exec` supervises one process with the chosen restart policy until it
//! exits (a `never` service) or the session is signalled, which is the
//! smallest live demonstration of the restart seam.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor};

const POLL_INTERVAL: Duration = Duration::from_millis(50);

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
           --exec PROG [ARGS...] [--policy always|on-failure|never]\n\
                          supervise one process until it exits or is signalled\n\
           -h, --help     show this help"
    );
}
