// SPDX-License-Identifier: MIT
//! T-12.2 acceptance: logout tears down a service's whole process group, so
//! no grandchild a wrapper started leaks past the session.
//!
//! Each "service" is a real `sh` that backgrounds a `sleep` grandchild and
//! waits. The supervisor spawns it as a process-group leader, so `shutdown`
//! and the anchor-exit path must reach the grandchild too. This is the
//! headless form of the nested startup/logout test plan.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor};

fn unique_path(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "dragonfruit-{label}-{}-{nanos}",
        std::process::id()
    ))
}

/// `sh` starts a `sleep` grandchild, writes its pid to `$1`, and waits.
fn wrapper(out: &Path) -> ServiceSpec {
    ServiceSpec::new("worker", "sh").args([
        "-c".to_string(),
        "sleep 30 & echo $! > \"$1\"; wait".to_string(),
        "sh".to_string(),
        out.to_string_lossy().into_owned(),
    ])
}

fn process_alive(pid: u32) -> bool {
    // Signal 0 checks existence without sending anything.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

fn wait_for_pid(path: &Path, timeout: Duration) -> Option<u32> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(pid) = text.trim().parse::<u32>() {
                return Some(pid);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

fn wait_until_gone(pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    !process_alive(pid)
}

#[test]
fn shutdown_tears_down_the_whole_process_group() {
    let pidfile = unique_path("logout-shutdown");
    let plan = SessionPlan::new(vec![wrapper(&pidfile).policy(RestartPolicy::Never)]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("the worker starts");

    let grandchild = wait_for_pid(&pidfile, Duration::from_secs(5))
        .expect("the wrapper wrote its grandchild pid");
    let worker = supervisor.running_pid("worker").expect("the worker pid");
    assert!(process_alive(worker) && process_alive(grandchild));

    supervisor.shutdown();

    assert_eq!(supervisor.state(), SessionState::Ended);
    assert_eq!(
        supervisor.service_state("worker"),
        Some(ServiceState::Stopped)
    );
    assert!(
        wait_until_gone(worker, Duration::from_secs(3)),
        "the direct child is gone"
    );
    assert!(
        wait_until_gone(grandchild, Duration::from_secs(3)),
        "the grandchild in the service's process group is gone"
    );

    let _ = std::fs::remove_file(&pidfile);
}

#[test]
fn an_anchor_exit_tears_down_the_rest_of_the_group() {
    let pidfile = unique_path("logout-anchor");
    let plan = SessionPlan::new(vec![
        ServiceSpec::new("compositor", "sh")
            .args([
                "-c".to_string(),
                "sleep 30 & echo $! > \"$1\"; wait".to_string(),
                "sh".to_string(),
                pidfile.to_string_lossy().into_owned(),
            ])
            .policy(RestartPolicy::Never)
            .ends_session(true),
        wrapper(Path::new("/dev/null")).policy(RestartPolicy::Always),
    ]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("stage 0 starts");

    let anchor_pid = supervisor
        .running_pid("compositor")
        .expect("the anchor pid");
    let grandchild = wait_for_pid(&pidfile, Duration::from_secs(5))
        .expect("the anchor wrote its grandchild pid");

    // A compositor crash ends the session and stops the shell.
    assert!(supervisor.kill("compositor"));
    let deadline = Instant::now() + Duration::from_secs(5);
    while supervisor.state() != SessionState::Ended && Instant::now() < deadline {
        supervisor.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(supervisor.state(), SessionState::Ended);
    assert_eq!(
        supervisor.service_state("compositor"),
        Some(ServiceState::Exited)
    );

    assert!(
        wait_until_gone(anchor_pid, Duration::from_secs(3)),
        "the anchor is gone"
    );
    assert!(
        wait_until_gone(grandchild, Duration::from_secs(3)),
        "the anchor's grandchild is gone"
    );

    let _ = std::fs::remove_file(&pidfile);
}

#[test]
fn kill_reaches_a_service_group() {
    let pidfile = unique_path("logout-kill");
    let plan = SessionPlan::new(vec![wrapper(&pidfile).policy(RestartPolicy::OnFailure)]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("the worker starts");

    let grandchild = wait_for_pid(&pidfile, Duration::from_secs(5))
        .expect("the wrapper wrote its grandchild pid");
    assert!(supervisor.kill("worker"), "the kill seam signals the group");
    assert!(
        wait_until_gone(grandchild, Duration::from_secs(3)),
        "a killed service does not leak its grandchild"
    );

    supervisor.shutdown();
    let _ = std::fs::remove_file(&pidfile);
}
