// SPDX-License-Identifier: MIT
//! Interim application-identity resolver (T-06 FR-3).
//!
//! X11 clients identify themselves with `WM_CLASS` (an instance and a
//! class string); Wayland clients use `xdg_toplevel.app_id`. Mapping either
//! to a `.desktop` application is owned by the `app-index` service (T-23).
//! Until that lands, this is a pure-std interim resolver that scans the
//! XDG application directories for `.desktop` entries and matches:
//!
//! 1. `StartupWMClass` (case-insensitive) against the WM_CLASS class or
//!    instance,
//! 2. the desktop file id / file stem against the class or instance,
//! 3. otherwise records a miss that feeds the T-23 heuristics.
//!
//! It deliberately does **not** link GIO/`AppInfo`: the compositor is a thin
//! policy layer and must not pull the desktop stack in. T-23 replaces this
//! with an `app-index` query and owns the heuristic table. The resolver is
//! intentionally cheap (one directory walk at startup, no per-frame work)
//! and never blocks the event loop.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// How an identity was resolved — useful for the audit trail and for T-23
/// to know which heuristic already covers a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// Matched a `.desktop` entry's `StartupWMClass`.
    StartupWmClass,
    /// Matched the `.desktop` file id or stem.
    DesktopId,
}

/// A resolved application identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppIdentity {
    /// The desktop file id, e.g. `firefox.desktop`.
    pub desktop_id: String,
    /// The `Name` field of the desktop entry, if present.
    pub name: Option<String>,
    /// Which rule matched.
    pub source: IdentitySource,
}

#[derive(Debug, Clone)]
struct AppEntry {
    desktop_id: String,
    name: Option<String>,
    /// `StartupWMClass` values, lowercased.
    wm_classes: Vec<String>,
    /// The desktop file id and stem, lowercased, for fallback matching.
    stems: Vec<String>,
}

/// The interim identity resolver.
#[derive(Debug, Default)]
pub struct AppResolver {
    entries: Vec<AppEntry>,
    /// WM_CLASS strings no rule matched (T-23 input).
    misses: BTreeSet<String>,
    resolved: u64,
    unresolved: u64,
}

impl AppResolver {
    /// Scan the standard XDG application directories.
    pub fn load() -> Self {
        Self::from_dirs(desktop_dirs())
    }

    /// Build a resolver from an explicit set of `applications` directories.
    /// Directories that do not exist are skipped.
    pub fn from_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        for dir in dirs {
            scan_dir(&dir, &dir, &mut entries, &mut seen);
        }
        entries.sort_by(|a, b| a.desktop_id.cmp(&b.desktop_id));
        AppResolver {
            entries,
            misses: BTreeSet::new(),
            resolved: 0,
            unresolved: 0,
        }
    }

    /// Resolve an X11 `WM_CLASS` pair. `instance` is the resource name and
    /// `class` the resource class; either may be empty. Returns `None` and
    /// records a miss when nothing matches (the caller still falls back to
    /// the raw class for grouping).
    pub fn resolve_wm_class(&mut self, instance: &str, class: &str) -> Option<AppIdentity> {
        let instance = normalize(instance);
        let class = normalize(class);
        if instance.is_empty() && class.is_empty() {
            return None;
        }

        // 1. StartupWMClass is the authoritative mapping.
        for entry in &self.entries {
            if entry.wm_classes.iter().any(|wm| {
                (!class.is_empty() && *wm == class) || (!instance.is_empty() && *wm == instance)
            }) {
                self.resolved += 1;
                return Some(AppIdentity {
                    desktop_id: entry.desktop_id.clone(),
                    name: entry.name.clone(),
                    source: IdentitySource::StartupWmClass,
                });
            }
        }

        // 2. Fall back to the desktop file id/stem (many apps set
        //    WM_CLASS to their binary name, which matches the desktop id).
        for entry in &self.entries {
            if entry.stems.iter().any(|stem| {
                (!class.is_empty() && stem == &class) || (!instance.is_empty() && stem == &instance)
            }) {
                self.resolved += 1;
                return Some(AppIdentity {
                    desktop_id: entry.desktop_id.clone(),
                    name: entry.name.clone(),
                    source: IdentitySource::DesktopId,
                });
            }
        }

        // 3. Miss: feed the T-23 heuristics.
        let key = if !class.is_empty() { class } else { instance };
        self.misses.insert(key);
        self.unresolved += 1;
        None
    }

    /// WM_CLASS strings that did not resolve (T-23 input).
    pub fn misses(&self) -> impl Iterator<Item = &str> {
        self.misses.iter().map(String::as_str)
    }

    /// Number of successful resolutions.
    pub fn resolved_count(&self) -> u64 {
        self.resolved
    }

    /// Number of misses.
    pub fn unresolved_count(&self) -> u64 {
        self.unresolved
    }

    /// Number of loaded desktop entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// The standard XDG application directories, most specific first.
pub fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    if let Some(home) = data_home {
        dirs.push(home.join("applications"));
    }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|v| v.to_string_lossy().into_owned())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
        dirs.push(PathBuf::from(dir).join("applications"));
    }
    dirs
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

/// Recursively scan `dir` for `.desktop` entries; `root` is the top of the
/// applications tree, used to build spec-style desktop file ids.
fn scan_dir(root: &Path, dir: &Path, out: &mut Vec<AppEntry>, seen: &mut BTreeSet<String>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for item in read.flatten() {
        let path = item.path();
        if path.is_dir() {
            scan_dir(root, &path, out, seen);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        let Some(entry) = parse_desktop_file(root, &path) else {
            continue;
        };
        // Most specific data dir wins; XDG order puts $XDG_DATA_HOME first.
        if seen.insert(entry.desktop_id.clone()) {
            out.push(entry);
        }
    }
}

fn parse_desktop_file(root: &Path, path: &Path) -> Option<AppEntry> {
    let contents = std::fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut name = None;
    let mut wm_classes = Vec::new();
    let mut is_application = true;
    let mut hidden = false;

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "Name" if name.is_none() => name = Some(value.to_string()),
            "StartupWMClass" if !value.is_empty() => {
                wm_classes.push(normalize(value));
            }
            "Type" => is_application = value == "Application",
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    // Hidden entries are not launchable; still let them resolve so a
    // running app is not reported as unknown, but skip non-applications.
    if !is_application {
        return None;
    }
    let _ = hidden;

    let desktop_id = desktop_id_for(root, path)?;
    let stem = path.file_stem()?.to_string_lossy().to_ascii_lowercase();
    let mut stems = vec![stem];
    // The desktop id with `.desktop` stripped, dashes intact.
    let id_stem = desktop_id.trim_end_matches(".desktop").to_ascii_lowercase();
    if id_stem != stems[0] {
        stems.push(id_stem);
    }

    Some(AppEntry {
        desktop_id,
        name,
        wm_classes,
        stems,
    })
}

/// Build a desktop file id per the desktop-entry spec: the path relative to
/// the applications root, `/` replaced by `-`.
fn desktop_id_for(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut id = String::new();
    for component in rel.components() {
        if !id.is_empty() {
            id.push('-');
        }
        id.push_str(&component.as_os_str().to_string_lossy());
    }
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "df-identity-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_desktop(dir: &Path, name: &str, body: &str) {
        fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn startup_wm_class_resolves_case_insensitively() {
        let dir = temp_dir("wmclass");
        write_desktop(
            &dir,
            "firefox.desktop",
            "[Desktop Entry]\nType=Application\nName=Firefox\nStartupWMClass=firefox\n",
        );
        write_desktop(
            &dir,
            "steam.desktop",
            "[Desktop Entry]\nType=Application\nName=Steam\nStartupWMClass=Steam\n",
        );

        let mut resolver = AppResolver::from_dirs([dir.clone()]);
        let firefox = resolver.resolve_wm_class("Navigator", "Firefox").unwrap();
        assert_eq!(firefox.desktop_id, "firefox.desktop");
        assert_eq!(firefox.source, IdentitySource::StartupWmClass);
        let steam = resolver.resolve_wm_class("", "steam").unwrap();
        assert_eq!(steam.desktop_id, "steam.desktop");
        assert_eq!(resolver.resolved_count(), 2);
        assert_eq!(resolver.unresolved_count(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn desktop_id_stem_is_the_fallback() {
        let dir = temp_dir("stem");
        write_desktop(
            &dir,
            "org.example.Game.desktop",
            "[Desktop Entry]\nType=Application\nName=Game\n",
        );
        let mut resolver = AppResolver::from_dirs([dir.clone()]);
        let game = resolver
            .resolve_wm_class("game", "org.example.Game")
            .unwrap();
        assert_eq!(game.desktop_id, "org.example.Game.desktop");
        assert_eq!(game.source, IdentitySource::DesktopId);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn misses_are_recorded_for_t23() {
        let dir = temp_dir("miss");
        write_desktop(
            &dir,
            "known.desktop",
            "[Desktop Entry]\nType=Application\nStartupWMClass=known\n",
        );
        let mut resolver = AppResolver::from_dirs([dir.clone()]);
        assert!(resolver
            .resolve_wm_class("sun-awt-X11-XFramePeer", "Java")
            .is_none());
        assert_eq!(resolver.unresolved_count(), 1);
        let misses: Vec<&str> = resolver.misses().collect();
        assert_eq!(misses, vec!["java"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_applications_are_ignored() {
        let dir = temp_dir("link");
        write_desktop(
            &dir,
            "link.desktop",
            "[Desktop Entry]\nType=Link\nName=A Link\nStartupWMClass=link\n",
        );
        let resolver = AppResolver::from_dirs([dir.clone()]);
        assert_eq!(resolver.len(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nested_desktop_ids_use_dashes() {
        let dir = temp_dir("nested");
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        write_desktop(
            &sub,
            "app.desktop",
            "[Desktop Entry]\nType=Application\nStartupWMClass=app\n",
        );
        let mut resolver = AppResolver::from_dirs([dir.clone()]);
        let identity = resolver.resolve_wm_class("", "app").unwrap();
        assert_eq!(identity.desktop_id, "sub-app.desktop");

        fs::remove_dir_all(&dir).ok();
    }
}
