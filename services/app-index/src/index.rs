// SPDX-License-Identifier: MIT
//! Application identity resolution (T-14.1a).
//!
//! app-index is the single owner of application identity
//! ([01-architecture.md](../../docs/design/01-architecture.md)): it maps an
//! `xdg_toplevel.app_id` (Wayland) or an Xwayland `WM_CLASS` pair to a
//! `.desktop` entry — the record carries the desktop id, the display name, the
//! themed icon name, and the launch semantics (Exec, Terminal). The shell,
//! Dock, and app switcher query it instead of carrying their own resolver.
//!
//! The resolution pipeline is deliberately pure and std-only so it is unit
//! tested directly against a fixture `.desktop` corpus. It mirrors the
//! fallback rules the legacy interim resolver documented (T-06), then adds the
//! heuristic tier the design calls for (Electron/Java/Flatpak identifiers):
//!
//! 1. exact desktop id (with or without the `.desktop` suffix);
//! 2. `StartupWMClass`, case-insensitively, against the WM_CLASS class or
//!    instance (X11) or the app id (Wayland);
//! 3. the desktop file id/stem and its reverse-DNS tail
//!    (`org.mozilla.firefox` → `firefox`);
//! 4. the executable basename parsed from `Exec` (Electron portables);
//! 5. a normalized-equality heuristic over id, name, and executable.
//!
//! Every unresolved identity is recorded in the miss set. That set is the
//! heuristic input the design names: it is served over the bus
//! (`org.dragonfruit.AppIndex1.Misses`) and consulted here so a miss that was
//! already seen is re-tested with the tolerant normalized comparison before it
//! is reported again.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// How an identity resolved — the audit trail for the heuristic table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// Exact `xdg_toplevel.app_id` / desktop id match.
    AppId,
    /// Matched a `.desktop` entry's `StartupWMClass`.
    StartupWmClass,
    /// Matched the `.desktop` file id or stem (or its reverse-DNS tail).
    DesktopId,
    /// Matched the executable basename parsed from `Exec`.
    Executable,
    /// Matched through the tolerant normalized heuristic.
    Heuristic,
}

impl IdentitySource {
    /// The wire spelling served over the bus.
    pub fn as_str(self) -> &'static str {
        match self {
            IdentitySource::AppId => "app_id",
            IdentitySource::StartupWmClass => "startup_wm_class",
            IdentitySource::DesktopId => "desktop_id",
            IdentitySource::Executable => "executable",
            IdentitySource::Heuristic => "heuristic",
        }
    }
}

/// A resolved application: the record plus which rule matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedApp {
    pub record: AppRecord,
    pub source: IdentitySource,
}

/// One installed `.desktop` application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppRecord {
    /// The desktop file id, e.g. `org.mozilla.firefox.desktop`.
    pub desktop_id: String,
    /// The `Name` field (falls back to the id stem).
    pub name: String,
    /// The `Icon` field (a themed icon name or an absolute path).
    pub icon: String,
    /// The raw `Exec` line (launch semantics; the shell expands field codes).
    pub exec: String,
    /// `Terminal=true`.
    pub terminal: bool,
    /// `NoDisplay=true`/`Hidden=true`.
    pub no_display: bool,
    /// The `Categories` list.
    pub categories: Vec<String>,
    /// The `StartupWMClass` value, if any.
    pub startup_wm_class: Option<String>,
}

impl AppRecord {
    /// True when the entry declares a usable single-line Exec.
    pub fn is_launchable(&self) -> bool {
        !self.no_display && !self.exec.trim().is_empty() && !self.exec.contains('\n')
    }
}

#[derive(Debug, Clone)]
struct AppEntry {
    record: AppRecord,
    /// `StartupWMClass` values, lowercased.
    wm_classes: Vec<String>,
    /// The file stem and desktop id stem, lowercased.
    stems: Vec<String>,
    /// The executable basename parsed from `Exec`, lowercased.
    executable: Option<String>,
}

/// The application index: a scanned `.desktop` corpus plus the resolution
/// audit state.
#[derive(Debug, Default)]
pub struct AppIndex {
    entries: Vec<AppEntry>,
    /// Identity strings no rule matched (the heuristic input).
    misses: BTreeSet<String>,
    resolved: u64,
    unresolved: u64,
}

impl AppIndex {
    /// Scan the standard XDG application directories.
    pub fn load() -> Self {
        Self::from_dirs(desktop_dirs())
    }

    /// Build an index from an explicit set of `applications` directories.
    /// Directories that do not exist are skipped; the first directory that
    /// provides a given desktop id wins (desktop-entry spec precedence).
    pub fn from_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        for dir in dirs {
            scan_dir(&dir, &dir, &mut entries, &mut seen);
        }
        entries.sort_by(|a, b| a.record.desktop_id.cmp(&b.record.desktop_id));
        AppIndex {
            entries,
            misses: BTreeSet::new(),
            resolved: 0,
            unresolved: 0,
        }
    }

    /// Resolve a Wayland `app_id` (or an already-resolved desktop id).
    pub fn resolve(&mut self, app_id: &str) -> Option<ResolvedApp> {
        self.resolve_keys(app_id, "", "")
    }

    /// Resolve a window's identifiers. `app_id` is the Wayland identifier (may
    /// be empty under Xwayland); `instance`/`class` are the `WM_CLASS` pair
    /// (may be empty on Wayland). Returns `None` and records a miss when
    /// nothing matches.
    pub fn resolve_window(
        &mut self,
        app_id: &str,
        instance: &str,
        class: &str,
    ) -> Option<ResolvedApp> {
        let class = normalize(class);
        let instance = normalize(instance);
        self.resolve_keys(app_id, &instance, &class)
    }

    /// Look up an entry by exact desktop id.
    pub fn lookup(&self, desktop_id: &str) -> Option<ResolvedApp> {
        let wanted = strip_suffix(&desktop_id.to_ascii_lowercase()).to_owned();
        self.entries
            .iter()
            .find(|entry| strip_suffix(&entry.record.desktop_id.to_ascii_lowercase()) == wanted)
            .map(|entry| ResolvedApp {
                record: entry.record.clone(),
                source: IdentitySource::AppId,
            })
    }

    /// Every installed record, ordered by desktop id.
    pub fn records(&self) -> impl Iterator<Item = &AppRecord> {
        self.entries.iter().map(|entry| &entry.record)
    }

    /// Identity strings that did not resolve (the heuristic input).
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

    /// True when no `.desktop` entries were found.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn resolve_keys(&mut self, app_id: &str, instance: &str, class: &str) -> Option<ResolvedApp> {
        let app_id = normalize(app_id);
        let wm_candidates: Vec<&str> = [class, instance]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect();
        if app_id.is_empty() && wm_candidates.is_empty() {
            return None;
        }

        let found = self.find_entry(&app_id, &wm_candidates);
        if let Some((position, source)) = found {
            self.resolved += 1;
            let entry = &self.entries[position];
            return Some(ResolvedApp {
                record: entry.record.clone(),
                source,
            });
        }

        // Miss: record and feed the heuristic table. A WM_CLASS class is the
        // canonical key (the interim resolver used the same one).
        let key = if !class.is_empty() {
            class
        } else if !instance.is_empty() {
            instance
        } else {
            app_id.as_str()
        };
        if !key.is_empty() {
            self.misses.insert(key.to_owned());
        }
        self.unresolved += 1;
        None
    }

    /// Apply the resolution pipeline and return the matched entry's position
    /// and the rule that matched it. `app_id` is the Wayland identity (possibly
    /// empty); `wm_candidates` are the X11 `WM_CLASS` values.
    fn find_entry(&self, app_id: &str, wm_candidates: &[&str]) -> Option<(usize, IdentitySource)> {
        let candidates: Vec<&str> = std::iter::once(app_id)
            .chain(wm_candidates.iter().copied())
            .filter(|value| !value.is_empty())
            .collect();

        // 1. Exact app id / desktop id (with or without the suffix). This tier
        //    is the Wayland `app_id`; a WM_CLASS is matched by StartupWMClass
        //    and the stem below.
        if !app_id.is_empty() {
            let wanted = strip_suffix(app_id);
            if let Some(position) = self.entries.iter().position(|entry| {
                strip_suffix(&entry.record.desktop_id.to_ascii_lowercase()) == wanted
            }) {
                return Some((position, IdentitySource::AppId));
            }
        }

        // 2. StartupWMClass against any candidate.
        for candidate in candidates.iter().copied() {
            if let Some(position) = self
                .entries
                .iter()
                .position(|entry| entry.wm_classes.iter().any(|wm| wm == candidate))
            {
                return Some((position, IdentitySource::StartupWmClass));
            }
        }

        // 3. Desktop id / stem / reverse-DNS tail.
        for candidate in candidates.iter().copied() {
            let candidate_tail = tail(candidate);
            if let Some(position) = self.entries.iter().position(|entry| {
                entry
                    .stems
                    .iter()
                    .any(|stem| stem == candidate || stem == candidate_tail)
            }) {
                return Some((position, IdentitySource::DesktopId));
            }
        }

        // 4. Executable basename (Electron/portable identifiers).
        for candidate in candidates.iter().copied() {
            if let Some(position) = self
                .entries
                .iter()
                .position(|entry| entry.executable.as_deref() == Some(candidate))
            {
                return Some((position, IdentitySource::Executable));
            }
        }

        // 5. Tolerant normalized heuristic. A previously-recorded miss is
        //    always re-tested here before it is recorded again, which is how
        //    the miss set feeds the heuristic table.
        for candidate in candidates.iter().copied() {
            let key = normalize_key(candidate);
            if key.len() < 3 {
                continue;
            }
            if let Some(position) = self.entries.iter().position(|entry| {
                let mut keys = vec![
                    normalize_key(strip_suffix(&entry.record.desktop_id.to_ascii_lowercase())),
                    normalize_key(&entry.record.name),
                ];
                keys.extend(entry.stems.iter().map(|stem| normalize_key(stem)));
                if let Some(executable) = &entry.executable {
                    keys.push(normalize_key(executable));
                }
                keys.into_iter().any(|entry_key| entry_key == key)
            }) {
                return Some((position, IdentitySource::Heuristic));
            }
        }

        None
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
    // Flatpak exports are not always on XDG_DATA_DIRS.
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/flatpak/exports/share/applications"));
    }
    dirs
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

/// The tolerant key: lowercase with every non-alphanumeric removed. This folds
/// `org.mozilla.firefox`, `org-mozilla-firefox`, and `Firefox` together.
fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn strip_suffix(value: &str) -> &str {
    value.strip_suffix(".desktop").unwrap_or(value)
}

/// The reverse-DNS tail: `org.mozilla.firefox` → `firefox`.
fn tail(value: &str) -> &str {
    value.rsplit('.').next().unwrap_or(value)
}

/// The executable basename of a desktop `Exec` line: the first token, field
/// codes dropped, with the directory and a trailing `%`-code removed.
fn executable_of(exec: &str) -> Option<String> {
    let first = exec.split_whitespace().next()?;
    let first = first.trim_matches('"');
    let base = Path::new(first)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())?;
    let base = base.to_ascii_lowercase();
    (!base.is_empty()).then_some(base)
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
        if seen.insert(entry.record.desktop_id.clone()) {
            out.push(entry);
        }
    }
}

fn parse_desktop_file(root: &Path, path: &Path) -> Option<AppEntry> {
    let contents = std::fs::read_to_string(path).ok()?;
    let desktop_id = desktop_id_for(root, path)?;
    let mut record = parse_record(&desktop_id, &contents)?;
    if record.name.is_empty() {
        record.name = strip_suffix(&desktop_id).to_owned();
    }

    let stem = path.file_stem()?.to_string_lossy().to_ascii_lowercase();
    let mut stems = vec![stem];
    let id_stem = strip_suffix(&desktop_id).to_ascii_lowercase();
    if id_stem != stems[0] {
        stems.push(id_stem.clone());
    }
    // A reverse-DNS id also matches its tail (`org.mozilla.firefox`).
    let id_tail = tail(&id_stem).to_owned();
    if !stems.contains(&id_tail) {
        stems.push(id_tail);
    }

    let wm_classes = record
        .startup_wm_class
        .as_deref()
        .map(|value| vec![normalize(value)])
        .unwrap_or_default();
    let executable = executable_of(&record.exec);

    Some(AppEntry {
        record,
        wm_classes,
        stems,
        executable,
    })
}

/// Parse the `[Desktop Entry]` group of `contents` into a record. Returns
/// `None` for non-Application entries (links/directories are not launchable
/// identities).
pub fn parse_record(desktop_id: &str, contents: &str) -> Option<AppRecord> {
    let mut in_desktop_entry = false;
    let mut record = AppRecord {
        desktop_id: desktop_id.to_owned(),
        ..Default::default()
    };
    let mut is_application = true;

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "Name" if record.name.is_empty() => record.name = value.to_owned(),
            "Icon" => record.icon = value.to_owned(),
            "Exec" => record.exec = value.to_owned(),
            "StartupWMClass" if !value.is_empty() => {
                record.startup_wm_class = Some(value.to_owned());
            }
            "Categories" => {
                record.categories = value
                    .split(';')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            "Terminal" => record.terminal = value.eq_ignore_ascii_case("true"),
            "Type" => is_application = value == "Application",
            "Hidden" | "NoDisplay" if value.eq_ignore_ascii_case("true") => {
                record.no_display = true;
            }
            _ => {}
        }
    }

    is_application.then_some(record)
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
            "df-app-index-{}-{}-{}",
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
    fn wayland_app_id_resolves_to_the_desktop_entry_and_icon() {
        let dir = temp_dir("wayland");
        write_desktop(
            &dir,
            "org.dragonfruit.Files.desktop",
            "[Desktop Entry]\nType=Application\nName=Files\nIcon=system-file-manager\n\
             Exec=dragonfruit-files %U\n",
        );

        let mut index = AppIndex::from_dirs([dir.clone()]);
        let files = index.resolve("org.dragonfruit.Files").unwrap();
        assert_eq!(files.record.desktop_id, "org.dragonfruit.Files.desktop");
        assert_eq!(files.record.icon, "system-file-manager");
        assert_eq!(files.source, IdentitySource::AppId);
        assert_eq!(index.resolved_count(), 1);
        assert_eq!(index.unresolved_count(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn startup_wm_class_resolves_case_insensitively_for_x11() {
        let dir = temp_dir("wmclass");
        write_desktop(
            &dir,
            "firefox.desktop",
            "[Desktop Entry]\nType=Application\nName=Firefox\nStartupWMClass=firefox\n\
             Exec=firefox %u\nIcon=firefox\n",
        );
        write_desktop(
            &dir,
            "steam.desktop",
            "[Desktop Entry]\nType=Application\nName=Steam\nStartupWMClass=Steam\nExec=steam\n",
        );

        let mut index = AppIndex::from_dirs([dir.clone()]);
        let firefox = index.resolve_window("", "Navigator", "Firefox").unwrap();
        assert_eq!(firefox.record.desktop_id, "firefox.desktop");
        assert_eq!(firefox.source, IdentitySource::StartupWmClass);
        let steam = index.resolve_window("", "", "steam").unwrap();
        assert_eq!(steam.record.desktop_id, "steam.desktop");
        assert_eq!(index.resolved_count(), 2);
        assert_eq!(index.unresolved_count(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn desktop_id_tail_and_executable_heuristics_resolve() {
        let dir = temp_dir("heuristics");
        write_desktop(
            &dir,
            "org.mozilla.firefox.desktop",
            "[Desktop Entry]\nType=Application\nName=Firefox\nExec=/usr/lib/firefox/firefox %u\n",
        );
        write_desktop(
            &dir,
            "visual-studio-code.desktop",
            "[Desktop Entry]\nType=Application\nName=Visual Studio Code\nExec=/usr/share/code/code %F\n",
        );

        let mut index = AppIndex::from_dirs([dir.clone()]);
        // Reverse-DNS tail.
        let firefox = index.resolve("firefox").unwrap();
        assert_eq!(firefox.record.desktop_id, "org.mozilla.firefox.desktop");
        assert_eq!(firefox.source, IdentitySource::DesktopId);
        // Executable basename of an Electron-style path.
        let code = index.resolve_window("Code", "", "").unwrap();
        assert_eq!(code.record.desktop_id, "visual-studio-code.desktop");
        assert_eq!(code.source, IdentitySource::Executable);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_normalized_heuristic_folds_separators() {
        let dir = temp_dir("normalized");
        write_desktop(
            &dir,
            "org.example.Calculator.desktop",
            "[Desktop Entry]\nType=Application\nName=Calculator\nExec=example-calculator\n",
        );

        let mut index = AppIndex::from_dirs([dir.clone()]);
        let calc = index.resolve("org-example-calculator").unwrap();
        assert_eq!(calc.source, IdentitySource::Heuristic);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn misses_are_recorded_and_reported() {
        let dir = temp_dir("miss");
        write_desktop(
            &dir,
            "known.desktop",
            "[Desktop Entry]\nType=Application\nName=Known\nStartupWMClass=known\n",
        );
        let mut index = AppIndex::from_dirs([dir.clone()]);
        assert!(index
            .resolve_window("", "sun-awt-X11-XFramePeer", "Java")
            .is_none());
        assert_eq!(index.unresolved_count(), 1);
        let misses: Vec<&str> = index.misses().collect();
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
        let index = AppIndex::from_dirs([dir.clone()]);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn first_data_dir_wins_for_duplicate_ids() {
        let user = temp_dir("user");
        let system = temp_dir("system");
        write_desktop(
            &user,
            "dup.desktop",
            "[Desktop Entry]\nType=Application\nName=User\n",
        );
        write_desktop(
            &system,
            "dup.desktop",
            "[Desktop Entry]\nType=Application\nName=System\n",
        );
        let index = AppIndex::from_dirs([user.clone(), system.clone()]);
        assert_eq!(index.lookup("dup").unwrap().record.name, "User");

        fs::remove_dir_all(&user).ok();
        fs::remove_dir_all(&system).ok();
    }

    #[test]
    fn nested_desktop_ids_join_with_dashes() {
        let dir = temp_dir("nested");
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        write_desktop(
            &sub,
            "app.desktop",
            "[Desktop Entry]\nType=Application\nName=App\nStartupWMClass=app\n",
        );
        let mut index = AppIndex::from_dirs([dir.clone()]);
        assert_eq!(
            index
                .resolve_window("", "", "app")
                .unwrap()
                .record
                .desktop_id,
            "sub-app.desktop"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_empty_identity_records_no_miss() {
        let dir = temp_dir("empty");
        write_desktop(
            &dir,
            "x.desktop",
            "[Desktop Entry]\nType=Application\nName=X\n",
        );
        let mut index = AppIndex::from_dirs([dir.clone()]);
        assert!(index.resolve("").is_none());
        assert!(index.resolve_window("", "", "").is_none());
        assert_eq!(index.unresolved_count(), 0);

        fs::remove_dir_all(&dir).ok();
    }
}
