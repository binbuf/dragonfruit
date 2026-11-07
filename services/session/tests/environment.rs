// SPDX-License-Identifier: MIT
//! T-12.1b acceptance: the session environment reaches children, and the two
//! unit seams (`--print-env`, `--wait-socket`) behave.
//!
//! Headless: the "service" is a real `sh` probe that writes the variables it
//! sees to a file through the same [`Supervisor`] the session uses.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use dragonfruit_session::dev_session::{self, AutologinSnapshot, DevSessionState};
use dragonfruit_session::env::{self, SessionEnvironment};
use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor};

/// Run ticks until `predicate` holds or `timeout` elapses.
fn drive_until(
    supervisor: &mut Supervisor,
    timeout: Duration,
    mut predicate: impl FnMut(&Supervisor) -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        supervisor.tick();
        if predicate(supervisor) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

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

/// `sh` writes the six contract variables, one per line, to `$1`.
fn probe_args(out: &Path) -> Vec<String> {
    vec![
        "-c".to_string(),
        "printf '%s\\n' \"$XDG_CURRENT_DESKTOP\" \"$XDG_SESSION_TYPE\" \
         \"$WAYLAND_DISPLAY\" \"$DISPLAY\" \"$GTK_CSD\" \"$DRAGONFRUIT_LAUNCH_TOKEN\" > \"$1\""
            .to_string(),
        "sh".to_string(),
        out.to_string_lossy().into_owned(),
    ]
}

#[test]
fn a_trusted_child_sees_the_whole_session_environment() {
    let out = unique_path("env-trusted");
    let token = "ab".repeat(32);
    let environment = SessionEnvironment::new("df-env-test")
        .with_x11_display(":7")
        .with_launch_token(token.clone());
    let plan = SessionPlan::new(vec![ServiceSpec::new("probe", "sh")
        .args(probe_args(&out))
        .policy(RestartPolicy::Never)
        .trusted(true)])
    .with_environment(&environment);

    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("probe starts");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            matches!(s.service_state("probe"), Some(ServiceState::Exited))
        }),
        "the probe exits cleanly"
    );

    let contents = std::fs::read_to_string(&out).expect("the probe wrote its environment");
    let _ = std::fs::remove_file(&out);
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(
        lines,
        vec![
            df_ipc::DESKTOP_NAME,
            env::WAYLAND_SESSION_TYPE,
            "df-env-test",
            ":7",
            env::GTK_CSD_DISABLED,
            token.as_str(),
        ],
        "every contract variable reached the child: {contents:?}"
    );
    supervisor.shutdown();
}

#[test]
fn an_untrusted_child_does_not_get_the_token() {
    let out = unique_path("env-untrusted");
    let environment = SessionEnvironment::new("df-env-test").with_launch_token("ab".repeat(32));
    let plan = SessionPlan::new(vec![ServiceSpec::new("probe", "sh")
        .args(probe_args(&out))
        .policy(RestartPolicy::Never)])
    .with_environment(&environment);

    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("probe starts");
    assert!(drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.service_state("probe") == Some(ServiceState::Exited)
    }));

    let contents = std::fs::read_to_string(&out).expect("the probe wrote its environment");
    let _ = std::fs::remove_file(&out);
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(
        lines.last().copied(),
        Some(""),
        "the untrusted child sees no launch token: {contents:?}"
    );
    // XDG_CURRENT_DESKTOP and WAYLAND_DISPLAY still arrive.
    assert_eq!(lines.first().copied(), Some(df_ipc::DESKTOP_NAME));
    assert_eq!(lines.get(2).copied(), Some("df-env-test"));
    supervisor.shutdown();
}

#[test]
fn wait_socket_blocks_until_the_socket_appears() {
    let socket = unique_path("wait-socket");
    assert!(!socket.exists());
    let child = {
        let socket = socket.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            std::fs::write(socket, b"").expect("write a fake socket");
        })
    };

    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args(["--wait-socket"])
        .arg(&socket)
        .args(["--timeout", "5"])
        .output()
        .expect("the session binary runs");
    child.join().expect("the writer thread");
    let _ = std::fs::remove_file(&socket);

    assert!(
        output.status.success(),
        "wait-socket returns once the socket exists: {output:?}"
    );
}

#[test]
fn wait_socket_fails_when_the_timeout_elapses() {
    let socket = unique_path("wait-socket-missing");
    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args(["--wait-socket"])
        .arg(&socket)
        .args(["--timeout", "0"])
        .output()
        .expect("the session binary runs");
    assert!(!output.status.success(), "a missing socket times out");
}

#[test]
fn print_env_lists_the_contract_and_a_fresh_token() {
    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args([
            "--print-env",
            "--socket-name",
            "df-print-env",
            "--x11-display",
            ":9",
        ])
        .output()
        .expect("the session binary runs");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("env output is UTF-8");

    assert!(text.contains("XDG_CURRENT_DESKTOP=dragonfruit"), "{text}");
    assert!(text.contains("XDG_SESSION_TYPE=wayland"), "{text}");
    assert!(text.contains("WAYLAND_DISPLAY=df-print-env"), "{text}");
    assert!(text.contains("DISPLAY=:9"), "{text}");

    let token = text
        .lines()
        .find_map(|line| line.strip_prefix("DRAGONFRUIT_LAUNCH_TOKEN="))
        .expect("a launch token is exported");
    assert_eq!(token.len(), env::TOKEN_BYTES * 2, "{token}");
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()), "{token}");
}

#[test]
fn a_harness_armed_child_sees_the_return_desktop() {
    // T-12.6b: the session entry exports DRAGONFRUIT_DEV_RETURN into every
    // child; the shell reads it to show "Quit to <previous desktop>".
    let out = unique_path("env-dev-return");
    let environment = SessionEnvironment::new("df-env-test").with_dev_return("existing");
    let args = vec![
        "-c".to_string(),
        "printf '%s\\n' \"$DRAGONFRUIT_DEV_RETURN\" > \"$1\"".to_string(),
        "sh".to_string(),
        out.to_string_lossy().into_owned(),
    ];
    let plan = SessionPlan::new(vec![ServiceSpec::new("probe", "sh")
        .args(args)
        .policy(RestartPolicy::Never)])
    .with_environment(&environment);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("probe starts");
    assert!(drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.service_state("probe") == Some(ServiceState::Exited)
    }));

    let contents = std::fs::read_to_string(&out).expect("probe wrote its environment");
    let _ = std::fs::remove_file(&out);
    assert_eq!(contents, "existing\n");
    supervisor.shutdown();
}

#[test]
fn print_env_reads_the_armed_dev_session_state() {
    // The entry script derives DRAGONFRUIT_DEV_RETURN from the state file the
    // harness armed, without the caller passing it explicitly.
    let scratch = unique_path("env-state");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let state = DevSessionState::armed(
        "gdm",
        Some("existing".to_string()),
        true,
        AutologinSnapshot::default(),
    );
    dev_session::write(&scratch, &state).expect("write state");

    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args(["--print-env", "--socket-name", "df-print-env"])
        .env("XDG_STATE_HOME", &scratch)
        .output()
        .expect("the session binary runs");
    let _ = std::fs::remove_dir_all(&scratch);

    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("env output is UTF-8");
    assert!(
        text.contains("DRAGONFRUIT_DEV_RETURN=existing"),
        "print-env exports the return desktop: {text}"
    );
}

#[test]
fn print_env_without_armed_state_leaves_the_return_unset() {
    let scratch = unique_path("env-no-state");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .args(["--print-env", "--socket-name", "df-print-env"])
        .env("XDG_STATE_HOME", &scratch)
        .output()
        .expect("the session binary runs");
    let _ = std::fs::remove_dir_all(&scratch);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("env output is UTF-8");
    assert!(!text.contains("DRAGONFRUIT_DEV_RETURN"), "{text}");
    assert!(
        text.lines().any(|line| line == "GTK_CSD=0"),
        "the session must steer GTK to server-side decorations: {text}"
    );
}

#[test]
#[cfg(unix)]
fn sync_toolkit_writes_the_left_layout_and_calls_gsettings() {
    use std::os::unix::fs::PermissionsExt;

    let root = unique_path("toolkit-sync");
    let config = root.join("config");
    let bin = root.join("bin");
    std::fs::create_dir_all(&config).expect("config dir");
    std::fs::create_dir_all(&bin).expect("bin dir");

    // A fake `gsettings` records the live write instead of touching the
    // developer's dconf. The session binary invokes it through PATH.
    let record = root.join("gsettings.args");
    let fake = bin.join("gsettings");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
            record.display()
        ),
    )
    .expect("write fake gsettings");
    let mut permissions = std::fs::metadata(&fake).expect("metadata").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake, permissions).expect("chmod");

    // A Flatpak app that has run before has a sandbox config tree; the sync
    // must update it too, because the sandbox cannot see the host files.
    let flatpak_gtk3 = root.join(".var/app/com.example.Editor/config/gtk-3.0/settings.ini");
    std::fs::create_dir_all(flatpak_gtk3.parent().unwrap()).expect("flatpak config dir");
    std::fs::write(
        &flatpak_gtk3,
        "[Settings]\ngtk-theme-name=Breeze\ngtk-decoration-layout=appmenu:close\n",
    )
    .expect("seed flatpak settings");

    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .arg("--sync-toolkit")
        .env("XDG_CONFIG_HOME", &config)
        .env("HOME", &root)
        .env("PATH", &bin)
        .output()
        .expect("the session binary runs");
    assert!(output.status.success(), "{output:?}");

    let args = std::fs::read_to_string(&record).expect("gsettings recorded its arguments");
    assert!(args.contains("org.gnome.desktop.wm.preferences"), "{args}");
    assert!(args.contains("button-layout"), "{args}");
    assert!(args.contains("close,minimize,maximize:"), "{args}");

    for directory in ["gtk-3.0", "gtk-4.0"] {
        let path = config.join(directory).join("settings.ini");
        let text = std::fs::read_to_string(&path).expect("settings.ini exists");
        assert!(
            text.contains("gtk-decoration-layout=close,minimize,maximize:"),
            "{path:?}: {text}"
        );
    }
    let keyfile = std::fs::read_to_string(config.join("glib-2.0/settings/keyfile"))
        .expect("the GSettings keyfile exists");
    assert!(
        keyfile.contains("button-layout='close,minimize,maximize:'"),
        "{keyfile}"
    );

    let flatpak_text = std::fs::read_to_string(&flatpak_gtk3).expect("flatpak settings exist");
    assert!(
        flatpak_text.contains("gtk-theme-name=Breeze"),
        "the app's own keys survive: {flatpak_text}"
    );
    assert!(
        flatpak_text.contains("gtk-decoration-layout=close,minimize,maximize:"),
        "the sandbox gets the desktop layout: {flatpak_text}"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn shutting_a_session_down_ends_the_state() {
    let plan = SessionPlan::default_session_for(&SessionEnvironment::default_socket());
    plan.validate().expect("the environment plan is valid");
    let mut supervisor = Supervisor::new(plan);
    supervisor.shutdown();
    assert_eq!(supervisor.state(), SessionState::Ended);
}
