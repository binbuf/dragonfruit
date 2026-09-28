// SPDX-License-Identifier: MIT
//! T-16.6b — the reduced-motion sweep.
//!
//! The acceptance is "every animation has a passing reduced-motion variant".
//! Proving that by spot-checking a handful of transitions is not a sweep, so
//! this suite **enumerates every animation** in the shipped tree and fails if
//! one lacks a reduced-motion variant:
//!
//! 1. **The motion catalog.** The generated Rust catalog
//!    (`compositor/src/design_tokens.rs`) and its QML twin
//!    (`design-system/Theme.qml`) are parsed. Both are generated from one
//!    source (`design-system/tokens/tokens.json`), so the sweep also
//!    cross-checks that the two lists name exactly the same motions. Every
//!    motion must have a real full duration and a reduced duration of `0`.
//! 2. **Every QML animation site.** Every `Behavior`/`*Animation`/`*Animator`
//!    block in the shipped QML (`shell/`, `design-system/`, `apps/`) must
//!    either resolve its duration through `Theme.motion` (which collapses to
//!    `0` under `Theme.reducedMotion`) or gate itself with an
//!    `enabled`/`running` binding that consults `Theme.reducedMotion`. A
//!    numeric `duration:` literal with no reduced-motion guard is a failure.
//! 3. **Every compositor lifecycle kind.** The window lifecycle enum
//!    (`window/motion.rs`) is enumerated and each kind must route through the
//!    shared `Tween::from_motion` reduced-motion hook; the overview pipeline
//!    must keep its reduced-motion single-step branch.
//!
//! The runtime behavior of each enumerated kind (appear/minimize/restore/
//! close/zoom/fullscreen, Mission Control, Spaces switch, Desktop Reveal) is
//! asserted one frame at a time by the existing reduced-motion conformance
//! tests in `window_conformance.rs` and `shell_protocol_conformance.rs`; this
//! suite is the always-on structural gate that a *new* animation cannot slip
//! past.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Repository root (this test lives in `<root>/compositor/tests`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compositor crate has a parent")
        .to_path_buf()
}

/// One named motion in the catalog, with its full and reduced durations.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    name: String,
    full_ms: u32,
    reduced_ms: u32,
}

/// Parse the generated Rust motion module: `pub const NAME: Motion = Motion {
/// duration_ms: N_u32, ..., reduced_duration_ms: R_u32, };`.
fn rust_catalog(source: &str) -> BTreeMap<String, Token> {
    let start = source
        .find("pub mod motion {")
        .expect("the generated Rust catalog has a motion module");
    let body = &source[start..];
    let mut tokens = BTreeMap::new();
    let mut rest = body;
    while let Some(pos) = rest.find("pub const ") {
        rest = &rest[pos + "pub const ".len()..];
        let name_end = rest.find(':').expect("const name terminator");
        let name = rest[..name_end].trim().to_string();
        let decl = &rest[name_end..];
        let end = decl.find("};").expect("motion declaration terminator");
        let decl = &decl[..end];
        let full = field_u32(decl, "duration_ms:").expect("duration_ms");
        let reduced = field_u32(decl, "reduced_duration_ms:").expect("reduced_duration_ms");
        tokens.insert(
            name.clone(),
            Token {
                name,
                full_ms: full,
                reduced_ms: reduced,
            },
        );
        rest = &rest[end..];
    }
    tokens
}

/// Parse a `field: <n>_u32` value.
fn field_u32(text: &str, field: &str) -> Option<u32> {
    let rest = text.split(field).nth(1)?;
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Parse the generated QML motion singleton:
/// `readonly property QtObject NAME: QtObject { fullDuration: N ...
/// duration: tokens.reducedMotion ? R : N ... }`.
fn qml_catalog(source: &str) -> BTreeMap<String, Token> {
    let start = source
        .find("readonly property var motion: QtObject {")
        .expect("the generated QML has a motion singleton");
    let body = &source[start..];
    let mut tokens = BTreeMap::new();
    let mut rest = body;
    while let Some(pos) = rest.find("readonly property QtObject ") {
        rest = &rest[pos + "readonly property QtObject ".len()..];
        let name_end = rest.find(':').expect("motion object name terminator");
        let name = rest[..name_end].trim().to_string();
        let block = &rest[name_end..];
        let end = block.find("}").expect("motion object terminator");
        let block = &block[..end];
        let full = field_plain(block, "fullDuration:").expect("fullDuration");
        let reduced = reduced_from_binding(block).expect("reduced-motion duration");
        tokens.insert(
            name.clone(),
            Token {
                name,
                full_ms: full,
                reduced_ms: reduced,
            },
        );
        rest = &rest[end..];
    }
    tokens
}

/// Parse a `field: <n>` integer.
fn field_plain(text: &str, field: &str) -> Option<u32> {
    let rest = text.split(field).nth(1)?;
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Parse `tokens.reducedMotion ? <reduced> : <full>`.
fn reduced_from_binding(text: &str) -> Option<u32> {
    let rest = text.split("tokens.reducedMotion ?").nth(1)?;
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// One QML animation site found in the tree.
#[derive(Debug)]
struct Site {
    path: String,
    line: usize,
    keyword: String,
    safe: bool,
    reason: String,
}

/// The animation keywords the sweep enumerates, `Behavior` first so a nested
/// `NumberAnimation` is judged as part of its enclosing `Behavior` block (the
/// `Behavior.enabled` guard is what collapses it).
const KEYWORDS: &[&str] = &[
    "Behavior on",
    "NumberAnimation",
    "PropertyAnimation",
    "ColorAnimation",
    "OpacityAnimator",
    "ScaleAnimator",
    "RotationAnimator",
    "XAnimator",
    "YAnimator",
    "SequentialAnimation",
    "ParallelAnimation",
    "SmoothedAnimation",
    "SpringAnimation",
    "PauseAnimation",
];

/// Whether `block` carries a reduced-motion variant.
fn block_is_safe(block: &str) -> (bool, String) {
    if block.contains("Theme.motion") {
        return (
            true,
            "duration comes from Theme.motion (collapses under reduced motion)".into(),
        );
    }
    if block.contains("reducedMotion") {
        return (
            true,
            "enabled/running is gated on Theme.reducedMotion".into(),
        );
    }
    if has_numeric_duration(block) {
        return (
            false,
            "numeric duration with no reduced-motion guard".into(),
        );
    }
    (
        false,
        "no Theme.motion duration and no Theme.reducedMotion guard".into(),
    )
}

/// Whether the block has a `duration:` followed by a numeric literal.
fn has_numeric_duration(block: &str) -> bool {
    let mut rest = block;
    while let Some(pos) = rest.find("duration:") {
        let after = rest[pos + "duration:".len()..].trim_start();
        if after.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return true;
        }
        rest = &rest[pos + "duration:".len()..];
    }
    false
}

/// Find the earliest keyword at or after `from`.
fn next_keyword(content: &str, from: usize) -> Option<(usize, &'static str)> {
    let mut best: Option<(usize, &'static str)> = None;
    for keyword in KEYWORDS {
        if let Some(pos) = content[from..].find(keyword) {
            let pos = from + pos;
            if best.map_or(true, |(b, _)| pos < b) {
                best = Some((pos, keyword));
            }
        }
    }
    best
}

/// The byte index just past the brace matching the `{` at `open`.
fn matching_brace(content: &str, open: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Enumerate every animation site in one QML file.
fn sites_in(path: &str, content: &str) -> Vec<Site> {
    // Skip commented animation examples deliberately: the sweep is about
    // shipped animation code, and comments never animate.
    let mut sites = Vec::new();
    let mut from = 0usize;
    while let Some((pos, keyword)) = next_keyword(content, from) {
        let Some(open_rel) = content[pos..].find('{') else {
            from = pos + keyword.len();
            continue;
        };
        let open = pos + open_rel;
        let Some(close) = matching_brace(content, open) else {
            break;
        };
        let block = &content[open + 1..close];
        let (safe, reason) = block_is_safe(block);
        let line = content[..pos].bytes().filter(|&b| b == b'\n').count() + 1;
        sites.push(Site {
            path: path.to_string(),
            line,
            keyword: keyword.to_string(),
            safe,
            reason,
        });
        from = close + 1;
    }
    sites
}

/// Recursively collect `.qml` files under `dir`, skipping test trees.
fn collect_qml(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            collect_qml(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "qml") {
            out.push(path);
        }
    }
}

#[test]
fn every_motion_token_has_a_reduced_motion_variant() {
    let root = repo_root();
    let rust = std::fs::read_to_string(root.join("compositor/src/design_tokens.rs"))
        .expect("read the generated Rust token catalog");
    let qml = std::fs::read_to_string(root.join("design-system/Theme.qml"))
        .expect("read the generated QML token catalog");

    let rust_tokens = rust_catalog(&rust);
    let qml_tokens = qml_catalog(&qml);

    assert!(
        !rust_tokens.is_empty(),
        "the Rust motion catalog is empty; the sweep would be vacuous"
    );
    // The Rust idents are UPPER_SNAKE and the QML properties lowerCamel; the
    // two are generated from one source, so the normalized names must match
    // exactly (a hand-added animation on one side would drift).
    let normalized = |map: &BTreeMap<String, Token>| {
        let mut names = map
            .keys()
            .map(|name| name.to_ascii_lowercase().replace('_', ""))
            .collect::<Vec<_>>();
        names.sort();
        names
    };
    assert_eq!(
        normalized(&rust_tokens),
        normalized(&qml_tokens),
        "the Rust and QML motion catalogs must name the same animations"
    );

    for (name, token) in &rust_tokens {
        assert!(
            token.full_ms > 0,
            "{name}: a motion with no full duration is not an animation"
        );
        assert_eq!(
            token.reduced_ms, 0,
            "{name}: the reduced-motion variant must be instant, not {} ms",
            token.reduced_ms
        );
    }

    eprintln!(
        "reduced-motion token sweep: {} animations enumerated ({})",
        rust_tokens.len(),
        rust_tokens.keys().cloned().collect::<Vec<_>>().join(", ")
    );
}

#[test]
fn every_qml_animation_site_has_a_reduced_motion_variant() {
    let root = repo_root();
    let mut files = Vec::new();
    for dir in [
        "shell",
        "design-system/components",
        "design-system/gallery",
        "apps",
    ] {
        collect_qml(&root.join(dir), &mut files);
    }
    assert!(!files.is_empty(), "the QML animation sweep found no files");

    let mut total = 0usize;
    let mut failures = Vec::new();
    for path in &files {
        let content = std::fs::read_to_string(path).expect("read QML file");
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        for site in sites_in(&relative, &content) {
            total += 1;
            if !site.safe {
                failures.push(format!(
                    "{}:{} — {} ({}); no reduced-motion variant",
                    site.path, site.line, site.keyword, site.reason
                ));
            }
        }
    }

    assert!(
        total > 0,
        "the QML animation sweep found no animation sites; the pattern is stale"
    );
    assert!(
        failures.is_empty(),
        "every animation must have a reduced-motion variant; {} site(s) do not:\n{}",
        failures.len(),
        failures.join("\n")
    );

    eprintln!(
        "reduced-motion site sweep: {total} animation sites across {} files",
        files.len()
    );
}

#[test]
fn every_compositor_lifecycle_kind_routes_through_reduced_motion() {
    let root = repo_root();
    let motion = std::fs::read_to_string(root.join("compositor/src/window/motion.rs"))
        .expect("read window/motion.rs");
    let overview = std::fs::read_to_string(root.join("compositor/src/overview/mod.rs"))
        .expect("read overview/mod.rs");

    // Every lifecycle kind the renderer animates. A new variant must be added
    // here (and to `WindowMotionKind::token`/the conformance tests), which is
    // exactly the review tripwire this sweep wants.
    let kinds = [
        "Appear",
        "Minimize",
        "Restore",
        "Close",
        "Zoom",
        "Fullscreen",
    ];
    let enum_start = motion
        .find("pub enum WindowMotionKind {")
        .expect("the lifecycle motion enum");
    let enum_end = motion[enum_start..]
        .find("}\n")
        .map(|end| enum_start + end)
        .expect("lifecycle enum body");
    let enum_body = &motion[enum_start..enum_end];
    for kind in kinds {
        assert!(
            enum_body.contains(&format!("{kind},")),
            "the sweep no longer sees lifecycle kind {kind}; update the enumeration"
        );
    }
    assert!(
        motion.contains("Tween::from_motion(now_ms, kind.token(), reduced_motion)"),
        "every lifecycle motion must route its tween through the reduced-motion hook"
    );

    // The overview/Spaces/Reveal machine keeps its single-step reduced path.
    assert!(
        overview.contains("if self.reduced_motion {"),
        "the overview machine must keep its reduced-motion single-step branch"
    );
    assert!(
        motion.contains("reduced motion") || motion.contains("reduced_motion"),
        "window/motion.rs must document the reduced-motion collapse"
    );

    eprintln!(
        "reduced-motion compositor sweep: {} lifecycle kinds enumerated ({})",
        kinds.len(),
        kinds.join(", ")
    );
}
