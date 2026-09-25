// SPDX-License-Identifier: MIT
//! T-12.1b acceptance: the shipped systemd user units parse and agree with the
//! `SessionPlan` contract (stages, restart policy, session environment).
//!
//! This is deliberately a small, dependency-free unit parser (systemd's own
//! `systemd-analyze verify` needs a running user manager and is not available
//! in every CI image); it checks the invariants the composition depends on.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The session compositor's fixed socket name; the units and the environment
/// contract must agree on it.
const SOCKET: &str = "dragonfruit-wayland";
/// The static part of the session environment every unit carries.
const STATIC_ENV: [(&str, &str); 3] = [
    ("XDG_CURRENT_DESKTOP", "dragonfruit"),
    ("XDG_SESSION_TYPE", "wayland"),
    ("WAYLAND_DISPLAY", SOCKET),
];
/// Dynamic values the session entry imports into the user manager; every unit
/// passes them through.
const PASS_ENV: [&str; 2] = ["DISPLAY", "DRAGONFRUIT_LAUNCH_TOKEN"];

/// A parsed unit file: section -> repeated key entries.
type Sections = BTreeMap<String, Vec<(String, String)>>;

fn units_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("units")
}

fn parse(text: &str) -> Sections {
    let mut sections: Sections = BTreeMap::new();
    let mut current: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = Some(name.to_string());
            sections.entry(name.to_string()).or_default();
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("not a key=value line: {line:?}"));
        let section = current
            .clone()
            .unwrap_or_else(|| panic!("key outside a section: {line:?}"));
        sections
            .entry(section)
            .or_default()
            .push((key.trim().to_string(), value.trim().to_string()));
    }
    sections
}

fn load(name: &str) -> Sections {
    let path = units_dir().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    parse(&text)
}

fn values<'a>(sections: &'a Sections, section: &str, key: &str) -> Vec<&'a str> {
    sections
        .get(section)
        .into_iter()
        .flatten()
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .collect()
}

fn one<'a>(sections: &'a Sections, section: &str, key: &str) -> &'a str {
    let found = values(sections, section, key);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one {section}.{key}: {found:?}"
    );
    found[0]
}

fn unit_files() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(units_dir()).expect("the units directory exists") {
        let entry = entry.expect("readable directory entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".service") || name.ends_with(".target") {
            names.insert(name);
        }
    }
    names
}

#[test]
fn the_shipped_unit_set_is_complete() {
    let names = unit_files();
    let expected: BTreeSet<String> = [
        "dragonfruit-session.target",
        "dragonfruit-compositor.service",
        "dragonfruit-shell.service",
        "dragonfruit-settingsd.service",
        "dragonfruit-menu-broker.service",
        "dragonfruit-app-index.service",
        "dragonfruit-notifications.service",
        "dragonfruit-portal.service",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        names, expected,
        "the units directory holds the whole session"
    );
}

#[test]
fn the_target_pulls_the_whole_composition() {
    let target = load("dragonfruit-session.target");
    assert!(one(&target, "Unit", "Requires").contains("dragonfruit-compositor.service"));
    let wants = values(&target, "Unit", "Wants").join(" ");
    for service in [
        "dragonfruit-shell.service",
        "dragonfruit-settingsd.service",
        "dragonfruit-menu-broker.service",
        "dragonfruit-app-index.service",
        "dragonfruit-notifications.service",
        "dragonfruit-portal.service",
    ] {
        assert!(wants.contains(service), "{service} is wanted: {wants}");
    }
    assert_eq!(
        values(&target, "Install", "WantedBy"),
        vec!["graphical-session.target"]
    );
}

#[test]
fn the_compositor_is_the_session_anchor() {
    let compositor = load("dragonfruit-compositor.service");
    // Never restarted: a compositor crash ends the session.
    assert_eq!(one(&compositor, "Service", "Restart"), "no");
    let exec = one(&compositor, "Service", "ExecStart");
    assert!(exec.contains("--backend drm"), "{exec}");
    assert!(exec.contains(&format!("--socket-name {SOCKET}")), "{exec}");
}

#[test]
fn restart_policies_match_the_plan() {
    // services/session/src/plan.rs: shell Always; everything else OnFailure.
    let expectations = [
        ("dragonfruit-shell.service", "always"),
        ("dragonfruit-settingsd.service", "on-failure"),
        ("dragonfruit-menu-broker.service", "on-failure"),
        ("dragonfruit-app-index.service", "on-failure"),
        ("dragonfruit-notifications.service", "on-failure"),
        ("dragonfruit-portal.service", "on-failure"),
    ];
    for (file, policy) in expectations {
        let unit = load(file);
        assert_eq!(one(&unit, "Service", "Restart"), policy, "{file}");
    }
}

#[test]
fn every_service_carries_the_session_environment() {
    for file in unit_files() {
        if !file.ends_with(".service") {
            continue;
        }
        let unit = load(&file);
        let env: BTreeSet<String> = values(&unit, "Service", "Environment")
            .into_iter()
            .map(str::to_string)
            .collect();
        for (key, value) in STATIC_ENV {
            assert!(
                env.contains(&format!("{key}={value}")),
                "{file} carries {key}={value}"
            );
        }
        let pass = values(&unit, "Service", "PassEnvironment").join(" ");
        for key in PASS_ENV {
            assert!(
                pass.split_whitespace().any(|k| k == key),
                "{file} passes {key}"
            );
        }
        // The compositor's socket gate: the shell and the stage services wait
        // for the socket before starting (T-12.1a's `set_ready`).
        let pre = values(&unit, "Service", "ExecStartPre").join(" ");
        if file != "dragonfruit-compositor.service" {
            assert!(
                pre.contains(&format!("--wait-socket {SOCKET}")),
                "{file} waits for the socket: {pre}"
            );
        }
        let exec = one(&unit, "Service", "ExecStart");
        assert!(exec.starts_with("/usr/bin/"), "{file}: {exec}");
    }
}

#[test]
fn every_dependency_reference_resolves_to_a_shipped_unit() {
    // Only these non-Dragonfruit units may be referenced.
    let system_units: BTreeSet<&str> = ["graphical-session.target", "xdg-desktop-portal.service"]
        .into_iter()
        .collect();
    let shipped = unit_files();
    for file in &shipped {
        let unit = load(file);
        for key in ["Requires", "Wants", "After", "Before", "PartOf"] {
            for value in values(&unit, "Unit", key) {
                for reference in value.split_whitespace() {
                    if system_units.contains(reference) {
                        continue;
                    }
                    assert!(
                        shipped.contains(reference),
                        "{file} references {reference:?}, which is not shipped"
                    );
                }
            }
        }
        // Every service is part of the session target, so stopping the target
        // stops the session.
        if file.ends_with(".service") {
            assert!(
                values(&unit, "Unit", "PartOf").contains(&"dragonfruit-session.target"),
                "{file} is part of the session target"
            );
        }
    }
}
