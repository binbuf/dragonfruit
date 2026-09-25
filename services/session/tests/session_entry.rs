// SPDX-License-Identifier: MIT
//! T-12.2 acceptance: the display-manager session entry, its startup/logout
//! script, and the install layout.
//!
//! The entry script is exercised for real against a fake `systemctl` and a
//! fake `dragonfruit-session`, so its sequence (import -> start -> wait ->
//! stop/reset-failed -> remove hand-off files) is asserted, not just its
//! text. No systemd user manager is needed.

use std::path::{Path, PathBuf};

use dragonfruit_session::entry;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unique_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "dragonfruit-{label}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
}

/// Parse a `.desktop` file's `[Desktop Entry]` keys into a map.
fn desktop_entry(text: &str) -> std::collections::BTreeMap<String, String> {
    let mut section = String::new();
    let mut keys = std::collections::BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_string();
            continue;
        }
        if section != "Desktop Entry" {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            keys.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    keys
}

#[test]
fn the_shipped_files_match_the_embedded_contract() {
    let root = manifest_dir();
    let desktop = std::fs::read_to_string(root.join("dragonfruit.desktop"))
        .expect("the shipped session entry exists");
    let script = std::fs::read_to_string(root.join("dragonfruit-session-entry"))
        .expect("the shipped entry script exists");
    assert_eq!(
        desktop,
        entry::DESKTOP_ENTRY,
        "the shipped .desktop and the embedded copy must not drift"
    );
    assert_eq!(
        script,
        entry::ENTRY_SCRIPT,
        "the shipped entry script and the embedded copy must not drift"
    );

    for (name, contents) in entry::UNIT_FILES {
        let on_disk = std::fs::read_to_string(root.join("units").join(name))
            .unwrap_or_else(|e| panic!("cannot read units/{name}: {e}"));
        assert_eq!(
            &on_disk, contents,
            "the embedded unit {name} and the shipped file must not drift"
        );
    }
}

#[test]
fn the_desktop_entry_names_the_session_and_its_command() {
    let keys = desktop_entry(entry::DESKTOP_ENTRY);
    assert_eq!(keys.get("Type").map(String::as_str), Some("Application"));
    assert_eq!(
        keys.get("Name").map(String::as_str),
        Some(entry::SESSION_NAME)
    );
    assert_eq!(
        keys.get("Exec").map(String::as_str),
        Some(entry::ENTRY_COMMAND)
    );
    assert_eq!(
        keys.get("DesktopNames").map(String::as_str),
        Some(entry::DESKTOP_NAME)
    );

    // The `Exec` program is the one the installer writes.
    let exec = keys.get("Exec").expect("an Exec line");
    assert_eq!(
        Path::new(exec).file_name().and_then(|n| n.to_str()),
        Some(entry::ENTRY_PROGRAM)
    );
}

#[test]
fn install_into_lays_out_the_entry_and_the_units() {
    let prefix = unique_dir("session-install");
    let written = entry::install_into(&prefix).expect("install succeeds");

    let entry_path = prefix.join(entry::BIN_DIR).join(entry::ENTRY_PROGRAM);
    let desktop_path = prefix
        .join(entry::WAYLAND_SESSIONS_DIR)
        .join(entry::SESSION_FILE);
    assert!(entry_path.exists(), "the entry script is installed");
    assert!(desktop_path.exists(), "the .desktop entry is installed");
    assert!(written.contains(&entry_path) && written.contains(&desktop_path));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&entry_path)
            .expect("entry metadata")
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0, "the entry script is executable: {mode:o}");
    }

    let desktop = std::fs::read_to_string(&desktop_path).expect("read installed desktop");
    assert_eq!(desktop, entry::DESKTOP_ENTRY);

    for (name, contents) in entry::UNIT_FILES {
        let path = prefix.join(entry::SYSTEMD_USER_DIR).join(name);
        let installed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("units/{name} was not installed: {e}"));
        assert_eq!(&installed, contents, "{name} content");
    }

    assert!(prefix
        .join(entry::SYSTEMD_USER_DIR)
        .join(entry::SESSION_TARGET)
        .exists());

    std::fs::remove_dir_all(&prefix).ok();
}

#[test]
fn the_entry_script_imports_starts_waits_and_tears_down() {
    let root = unique_dir("session-entry-run");
    let runtime = root.join("runtime");
    std::fs::create_dir_all(&runtime).expect("runtime dir");

    // A socket name unique to this test, and a fake runtime hand-off set.
    let socket = "df-entry-test";
    let handoff = [
        runtime.join(socket),
        runtime.join(format!("{socket}.lock")),
        runtime.join(format!("{socket}.x11-display")),
        runtime.join(format!("{socket}.launch-token")),
    ];
    for path in &handoff {
        std::fs::write(path, b"stale").expect("seed a hand-off file");
    }

    // Fake `dragonfruit-session`: print the contract environment.
    let fake_session = root.join("fake-session");
    std::fs::write(
        &fake_session,
        "#!/bin/sh\nprintf '%s\\n' \
         'XDG_CURRENT_DESKTOP=dragonfruit' \
         'XDG_SESSION_TYPE=wayland' \
         \"WAYLAND_DISPLAY=$DF_SESSION_SOCKET\" \
         'DRAGONFRUIT_LAUNCH_TOKEN=deadbeef'\n",
    )
    .expect("write fake session");
    make_executable(&fake_session);

    // Fake `systemctl`: record calls; report the compositor active once, then
    // gone, so the wait loop runs and exits.
    let record = root.join("systemctl.log");
    let counter = root.join("is-active.count");
    let fake_systemctl = root.join("fake-systemctl");
    std::fs::write(
        &fake_systemctl,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{record}'\n\
             if [ \"$1\" = '--user' ] && [ \"$2\" = 'is-active' ]; then\n\
               count=0\n\
               [ -f '{counter}' ] && count=$(cat '{counter}')\n\
               count=$((count + 1))\n\
               printf '%s' \"$count\" > '{counter}'\n\
               [ \"$count\" -le 1 ] && exit 0\n\
               exit 3\n\
             fi\n\
             exit 0\n",
            record = record.display(),
            counter = counter.display(),
        ),
    )
    .expect("write fake systemctl");
    make_executable(&fake_systemctl);

    let status = std::process::Command::new(manifest_dir().join("dragonfruit-session-entry"))
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("DF_SESSION_BIN", &fake_session)
        .env("DF_SYSTEMCTL", &fake_systemctl)
        .env("DF_SESSION_SOCKET", socket)
        .status()
        .expect("run the entry script");
    assert!(
        status.success(),
        "the entry script exits cleanly: {status:?}"
    );

    let calls = std::fs::read_to_string(&record).expect("the fake systemctl recorded calls");
    assert!(
        calls.contains("import-environment")
            && calls.contains("XDG_CURRENT_DESKTOP")
            && calls.contains("DRAGONFRUIT_LAUNCH_TOKEN"),
        "the session environment is imported: {calls}"
    );
    assert!(
        calls.contains(&format!("--user start {}", entry::SESSION_TARGET)),
        "the target is started: {calls}"
    );
    assert!(
        calls.contains(&format!("--user stop {}", entry::SESSION_TARGET)),
        "the target is stopped at logout: {calls}"
    );
    assert!(
        calls.contains(&format!("--user reset-failed {}", entry::SESSION_TARGET)),
        "the failed state is cleared: {calls}"
    );
    assert!(
        calls.contains("is-active --quiet dragonfruit-compositor.service"),
        "the entry waits on the anchor: {calls}"
    );

    for path in &handoff {
        assert!(
            !path.exists(),
            "logout removed the leaked hand-off file {}",
            path.display()
        );
    }

    std::fs::remove_dir_all(&root).ok();
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}
