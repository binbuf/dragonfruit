// SPDX-License-Identifier: GPL-3.0-only
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
//!
//! # Install / uninstall / update events (T-14.1b)
//!
//! The index remembers the directories it was built from, so [`AppIndex::refresh`]
//! can re-scan them and diff against the current corpus. A new desktop id is an
//! install, a disappeared id an uninstall, and a changed record an update; the
//! diff is returned as [`IndexEvent`]s and retained (bounded) for
//! [`AppIndex::events`]/[`AppIndex::drain_events`]. The service watches the
//! directories with inotify and calls `refresh` on a monitor event, so an idle
//! service does no work ([watch](crate::watch)).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

/// The maximum number of [`IndexEvent`]s retained for `Events`.
pub const EVENT_CAPACITY: usize = 256;

/// What happened to one `.desktop` entry between two scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexEventKind {
    /// A desktop id that was not installed before.
    Installed,
    /// An installed desktop id whose record changed.
    Updated,
    /// A desktop id that is no longer present.
    Uninstalled,
}

impl IndexEventKind {
    /// The wire spelling served over the bus.
    pub fn as_str(self) -> &'static str {
        match self {
            IndexEventKind::Installed => "installed",
            IndexEventKind::Updated => "updated",
            IndexEventKind::Uninstalled => "uninstalled",
        }
    }
}

/// One install/uninstall/update observed between two scans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEvent {
    pub kind: IndexEventKind,
    /// The desktop file id, e.g. `org.mozilla.firefox.desktop`.
    pub desktop_id: String,
    /// The display name at the time of the scan (empty for an uninstall).
    pub name: String,
}

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
    /// `OnlyShowIn` desktop names (semicolon-separated in the file).
    pub only_show_in: Vec<String>,
    /// `NotShowIn` desktop names (semicolon-separated in the file).
    pub not_show_in: Vec<String>,
    /// The raw `TryExec` program, empty when the entry does not declare one.
    pub try_exec: String,
}

impl AppRecord {
    /// True when the entry declares a usable single-line Exec.
    pub fn is_launchable(&self) -> bool {
        !self.no_display && !self.exec.trim().is_empty() && !self.exec.contains('\n')
    }

    /// True when this entry is intended for at least one of `desktops`.
    ///
    /// `OnlyShowIn` is a whitelist: a non-empty list that names none of the
    /// current desktops hides the entry. `NotShowIn` is a blacklist. Names are
    /// compared case-insensitively because the desktop-name contract is
    /// lowercase while third-party files often spell names in capitals.
    ///
    /// `TryExec` is not checked here: whether the program exists is a
    /// filesystem fact resolved once at scan time (see [`AppIndex`]).
    pub fn shows_in(&self, desktops: &[String]) -> bool {
        let matches = |names: &[String]| {
            names.iter().any(|name| {
                desktops
                    .iter()
                    .any(|desktop| name.eq_ignore_ascii_case(desktop))
            })
        };
        if !self.only_show_in.is_empty() && !matches(&self.only_show_in) {
            return false;
        }
        if matches(&self.not_show_in) {
            return false;
        }
        true
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
    /// Whether a non-empty `TryExec` names an executable on this machine.
    /// Always true when the entry has no `TryExec`.
    try_exec_found: bool,
}

/// The application index: a scanned `.desktop` corpus plus the resolution
/// audit state.
#[derive(Debug, Default)]
pub struct AppIndex {
    entries: Vec<AppEntry>,
    /// The `applications` directories this index was built from, in
    /// precedence order. Kept so `refresh` can re-scan exactly the same set.
    dirs: Vec<PathBuf>,
    /// The desktop names this index filters against (`XDG_CURRENT_DESKTOP`,
    /// colon-separated and lowercased). Empty means no desktop is known.
    desktops: Vec<String>,
    /// The `PATH` used to resolve a non-absolute `TryExec`, kept so `refresh`
    /// re-evaluates the same search path.
    search_paths: Vec<PathBuf>,
    /// Identity strings no rule matched (the heuristic input).
    misses: BTreeSet<String>,
    /// Install/uninstall/update observations, oldest first, bounded by
    /// [`EVENT_CAPACITY`].
    events: VecDeque<IndexEvent>,
    /// Bumped once per non-empty refresh; consumers use it to detect change.
    revision: u64,
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
    ///
    /// Visibility is evaluated against the process `XDG_CURRENT_DESKTOP` and
    /// `PATH`; tests and nested runs use [`AppIndex::from_dirs_with`] for an
    /// explicit environment.
    pub fn from_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        Self::from_dirs_with(dirs, current_desktops(), executable_search_paths())
    }

    /// Build an index with an explicit desktop-name list and `PATH`.
    ///
    /// `desktops` are the names in `XDG_CURRENT_DESKTOP` (already split on
    /// `:`); `search_paths` resolves a non-absolute `TryExec`. Passing an empty
    /// `desktops` is the no-desktop-environment case: `OnlyShowIn` entries
    /// hide and `NotShowIn` entries stay visible.
    pub fn from_dirs_with(
        dirs: impl IntoIterator<Item = PathBuf>,
        desktops: impl IntoIterator<Item = String>,
        search_paths: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        let dirs: Vec<PathBuf> = dirs.into_iter().collect();
        let desktops: Vec<String> = desktops
            .into_iter()
            .map(|desktop| desktop.trim().to_ascii_lowercase())
            .filter(|desktop| !desktop.is_empty())
            .collect();
        let search_paths: Vec<PathBuf> = search_paths.into_iter().collect();
        let entries = scan(&dirs, &search_paths);
        AppIndex {
            entries,
            dirs,
            desktops,
            search_paths,
            misses: BTreeSet::new(),
            events: VecDeque::new(),
            revision: 0,
            resolved: 0,
            unresolved: 0,
        }
    }

    /// Re-scan the directories this index was built from and diff against the
    /// current corpus, updating the entries in place. Returns the
    /// install/uninstall/update events, ordered by desktop id; the events are
    /// also retained for [`AppIndex::events`]/[`AppIndex::drain_events`]. The
    /// resolution counters and the miss set are untouched: a refresh is
    /// maintenance, not a query.
    pub fn refresh(&mut self) -> Vec<IndexEvent> {
        let wanted = scan(&self.dirs, &self.search_paths);
        let previous: BTreeMap<&str, &AppEntry> = self
            .entries
            .iter()
            .map(|entry| (entry.record.desktop_id.as_str(), entry))
            .collect();
        let present: BTreeSet<&str> = wanted
            .iter()
            .map(|entry| entry.record.desktop_id.as_str())
            .collect();

        let mut events = Vec::new();
        for entry in &wanted {
            match previous.get(entry.record.desktop_id.as_str()) {
                None => events.push(IndexEvent {
                    kind: IndexEventKind::Installed,
                    desktop_id: entry.record.desktop_id.clone(),
                    name: entry.record.name.clone(),
                }),
                // `TryExec` availability is a filesystem fact outside the
                // parsed record; a program appearing or disappearing changes
                // the catalog's visibility and must still be an update.
                Some(existing)
                    if existing.record != entry.record
                        || existing.try_exec_found != entry.try_exec_found =>
                {
                    events.push(IndexEvent {
                        kind: IndexEventKind::Updated,
                        desktop_id: entry.record.desktop_id.clone(),
                        name: entry.record.name.clone(),
                    })
                }
                Some(_) => {}
            }
        }
        for entry in &self.entries {
            if !present.contains(entry.record.desktop_id.as_str()) {
                events.push(IndexEvent {
                    kind: IndexEventKind::Uninstalled,
                    desktop_id: entry.record.desktop_id.clone(),
                    name: entry.record.name.clone(),
                });
            }
        }

        if !events.is_empty() {
            events.sort_by(|a, b| a.desktop_id.cmp(&b.desktop_id));
            self.revision += 1;
            for event in &events {
                self.events.push_back(event.clone());
            }
            while self.events.len() > EVENT_CAPACITY {
                self.events.pop_front();
            }
        }
        self.entries = wanted;
        events
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

    /// Resolve a window identity without recording a miss or touching the
    /// resolution counters. The launch registry uses this: window activity is
    /// not a query, and a busy session must not inflate the audit counts.
    pub fn resolve_activity(
        &self,
        app_id: &str,
        instance: &str,
        class: &str,
    ) -> Option<ResolvedApp> {
        let app_id = normalize(app_id);
        let instance = normalize(instance);
        let class = normalize(class);
        let wm_candidates: Vec<&str> = [class.as_str(), instance.as_str()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect();
        if app_id.is_empty() && wm_candidates.is_empty() {
            return None;
        }
        self.find_entry(&app_id, &wm_candidates)
            .map(|(position, source)| ResolvedApp {
                record: self.entries[position].record.clone(),
                source,
            })
    }

    /// The `applications` directories this index was built from.
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// A counter bumped once per non-empty [`AppIndex::refresh`].
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The retained install/uninstall/update events, oldest first.
    pub fn events(&self) -> impl Iterator<Item = &IndexEvent> {
        self.events.iter()
    }

    /// Take and clear the retained index events.
    pub fn drain_events(&mut self) -> Vec<IndexEvent> {
        self.events.drain(..).collect()
    }

    /// Every installed record, ordered by desktop id. This is the identity
    /// corpus: an entry hidden from the drawer by `OnlyShowIn`, `NotShowIn`,
    /// or a missing `TryExec` is still resolvable here.
    pub fn records(&self) -> impl Iterator<Item = &AppRecord> {
        self.entries.iter().map(|entry| &entry.record)
    }

    /// The records the Applications drawer should offer: installed (a
    /// non-empty `TryExec` resolves) and intended for one of this index's
    /// desktops. `NoDisplay`/`Hidden` records stay here with their flag; the
    /// shell still filters them out of launcher lists.
    pub fn visible_records(&self) -> impl Iterator<Item = &AppRecord> {
        self.entries
            .iter()
            .filter(|entry| entry.try_exec_found && entry.record.shows_in(&self.desktops))
            .map(|entry| &entry.record)
    }

    /// The desktop names this index filters against.
    pub fn desktops(&self) -> &[String] {
        &self.desktops
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

/// Scan every directory in `dirs` into one ordered entry list. Directories
/// that do not exist are skipped; the first directory that provides a given
/// desktop id wins (desktop-entry spec precedence).
fn scan(dirs: &[PathBuf], search_paths: &[PathBuf]) -> Vec<AppEntry> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for dir in dirs {
        scan_dir(dir, dir, search_paths, &mut entries, &mut seen);
    }
    entries.sort_by(|a, b| a.record.desktop_id.cmp(&b.record.desktop_id));
    entries
}

/// Recursively scan `dir` for `.desktop` entries; `root` is the top of the
/// applications tree, used to build spec-style desktop file ids.
fn scan_dir(
    root: &Path,
    dir: &Path,
    search_paths: &[PathBuf],
    out: &mut Vec<AppEntry>,
    seen: &mut BTreeSet<String>,
) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for item in read.flatten() {
        let path = item.path();
        if path.is_dir() {
            scan_dir(root, &path, search_paths, out, seen);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        let Some(entry) = parse_desktop_file(root, &path, search_paths) else {
            continue;
        };
        // Most specific data dir wins; XDG order puts $XDG_DATA_HOME first.
        if seen.insert(entry.record.desktop_id.clone()) {
            out.push(entry);
        }
    }
}

fn parse_desktop_file(root: &Path, path: &Path, search_paths: &[PathBuf]) -> Option<AppEntry> {
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
    let try_exec_found = try_exec_found(&record.try_exec, search_paths);

    Some(AppEntry {
        record,
        wm_classes,
        stems,
        executable,
        try_exec_found,
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
            "OnlyShowIn" => record.only_show_in = desktop_names(value),
            "NotShowIn" => record.not_show_in = desktop_names(value),
            "TryExec" => record.try_exec = value.to_owned(),
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

/// Split a semicolon-separated desktop-name list, dropping empty parts.
fn desktop_names(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The desktop names in `XDG_CURRENT_DESKTOP`, which the spec defines as a
/// colon-separated list. An unset/empty value yields the no-desktop case.
fn current_desktops() -> Vec<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .map(|value| {
            value
                .split(':')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The process `PATH` entries used for a relative `TryExec`.
fn executable_search_paths() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default()
}

/// A non-empty `TryExec` is installed only when it names an executable. An
/// absolute value is checked directly; any other value is looked up in
/// `PATH`, as the desktop-entry spec defines. An empty `TryExec` is not a
/// filter.
fn try_exec_found(try_exec: &str, search_paths: &[PathBuf]) -> bool {
    let try_exec = try_exec.trim();
    if try_exec.is_empty() {
        return true;
    }
    if Path::new(try_exec).is_absolute() {
        return is_executable(Path::new(try_exec));
    }
    search_paths
        .iter()
        .any(|dir| is_executable(&dir.join(try_exec)))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
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

    fn write_executable(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn visible_ids(index: &AppIndex) -> Vec<String> {
        index
            .visible_records()
            .map(|record| record.desktop_id.clone())
            .collect()
    }

    // A second desktop name is fixture data only; the naming gate exempts it.
    const OTHER_DESKTOP: &str = "GNOME"; // df-allow-desktop-name: fixture desktop name

    #[test]
    fn shipped_first_party_entries_are_launchable() {
        // Migrated from the shell's retired `.desktop` resolver (T-14.7):
        // app-index is the only production parser now, so it verifies the
        // in-repo first-party entries (identity is a launch precondition).
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps");
        let index = AppIndex::from_dirs([root.join("settings"), root.join("files")]);
        for id in [
            "org.dragonfruit.Settings.desktop",
            "org.dragonfruit.Files.desktop",
        ] {
            let resolved = index
                .lookup(id)
                .unwrap_or_else(|| panic!("{id} is not in the index"));
            assert!(
                resolved.record.is_launchable(),
                "{id} is not launchable: {:?}",
                resolved.record.exec
            );
        }
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

    #[test]
    fn a_refresh_reports_install_update_and_uninstall() {
        let dir = temp_dir("refresh");
        write_desktop(
            &dir,
            "keep.desktop",
            "[Desktop Entry]\nType=Application\nName=Keep\nStartupWMClass=keep\n",
        );
        write_desktop(
            &dir,
            "gone.desktop",
            "[Desktop Entry]\nType=Application\nName=Gone\nStartupWMClass=gone\n",
        );
        let mut index = AppIndex::from_dirs([dir.clone()]);
        assert_eq!(index.revision(), 0);

        // A no-op refresh produces no events and no revision bump.
        assert!(index.refresh().is_empty());
        assert_eq!(index.revision(), 0);

        // Install: a new desktop id.
        write_desktop(
            &dir,
            "new.desktop",
            "[Desktop Entry]\nType=Application\nName=New\nStartupWMClass=new\n",
        );
        let events = index.refresh();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, IndexEventKind::Installed);
        assert_eq!(events[0].desktop_id, "new.desktop");
        assert_eq!(events[0].name, "New");
        assert_eq!(index.revision(), 1);
        assert!(index.lookup("new.desktop").is_some());

        // Update: an existing desktop id whose record changed.
        write_desktop(
            &dir,
            "keep.desktop",
            "[Desktop Entry]\nType=Application\nName=Kept\nStartupWMClass=keep\n",
        );
        let events = index.refresh();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, IndexEventKind::Updated);
        assert_eq!(events[0].name, "Kept");
        assert_eq!(index.lookup("keep.desktop").unwrap().record.name, "Kept");

        // Uninstall: a desktop id that disappeared.
        fs::remove_file(dir.join("gone.desktop")).unwrap();
        let events = index.refresh();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, IndexEventKind::Uninstalled);
        assert_eq!(events[0].desktop_id, "gone.desktop");
        assert!(index.lookup("gone.desktop").is_none());

        // The events are retained, oldest first, and can be drained.
        assert_eq!(index.events().count(), 3);
        let drained = index.drain_events();
        assert_eq!(drained.len(), 3);
        assert!(index.events().next().is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_activity_does_not_touch_the_audit_counters() {
        let dir = temp_dir("activity");
        write_desktop(
            &dir,
            "known.desktop",
            "[Desktop Entry]\nType=Application\nName=Known\nStartupWMClass=known\n",
        );
        let index = AppIndex::from_dirs([dir.clone()]);
        let resolved = index.resolve_activity("", "", "Known").unwrap();
        assert_eq!(resolved.record.desktop_id, "known.desktop");
        assert!(index.resolve_activity("", "java", "Java").is_none());
        assert_eq!(index.resolved_count(), 0);
        assert_eq!(index.unresolved_count(), 0);
        assert_eq!(index.misses().count(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn only_show_in_entries_are_hidden_outside_their_desktops() {
        let dir = temp_dir("only-show-in");
        write_desktop(
            &dir,
            "other-desktop-only.desktop",
            &format!(
                "[Desktop Entry]\nType=Application\nName=Other Desktop Only\n\
                 OnlyShowIn={OTHER_DESKTOP};\nExec=other-desktop-only\n"
            ),
        );
        write_desktop(
            &dir,
            "dragonfruit.desktop",
            &format!(
                "[Desktop Entry]\nType=Application\nName=Dragonfruit\n\
                 OnlyShowIn=dragonfruit;{OTHER_DESKTOP};\nExec=dragonfruit-app\n"
            ),
        );
        write_desktop(
            &dir,
            "normal.desktop",
            "[Desktop Entry]\nType=Application\nName=Normal\nExec=normal-app\n",
        );

        let index = AppIndex::from_dirs_with(
            [dir.clone()],
            ["dragonfruit".to_owned()],
            Vec::<PathBuf>::new(),
        );
        assert_eq!(
            visible_ids(&index),
            vec!["dragonfruit.desktop", "normal.desktop"]
        );
        // Identity is not removed: a running other-desktop app still resolves.
        assert!(index.lookup("other-desktop-only.desktop").is_some());

        // The same corpus under the other desktop hides the Dragonfruit-only
        // entry.
        let other = AppIndex::from_dirs_with(
            [dir.clone()],
            [OTHER_DESKTOP.to_owned()],
            Vec::<PathBuf>::new(),
        );
        assert_eq!(
            visible_ids(&other),
            vec![
                "dragonfruit.desktop",
                "normal.desktop",
                "other-desktop-only.desktop"
            ]
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn not_show_in_entries_are_hidden_in_the_listed_desktop() {
        let dir = temp_dir("not-show-in");
        write_desktop(
            &dir,
            "not-other.desktop",
            &format!(
                "[Desktop Entry]\nType=Application\nName=Not Other\n\
                 NotShowIn={OTHER_DESKTOP};\nExec=not-other\n"
            ),
        );
        write_desktop(
            &dir,
            "not-dragonfruit.desktop",
            "[Desktop Entry]\nType=Application\nName=Not Dragonfruit\n\
             NotShowIn=dragonfruit;\nExec=not-dragonfruit\n",
        );

        let index = AppIndex::from_dirs_with(
            [dir.clone()],
            ["dragonfruit".to_owned()],
            Vec::<PathBuf>::new(),
        );
        assert_eq!(visible_ids(&index), vec!["not-other.desktop"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_try_exec_hides_the_entry_and_an_empty_one_does_not() {
        let apps = temp_dir("try-exec-apps");
        let bin = temp_dir("try-exec-bin");
        write_executable(&bin, "installed-tool");
        let absolute = write_executable(&bin, "absolute-tool");

        write_desktop(
            &apps,
            "missing.desktop",
            "[Desktop Entry]\nType=Application\nName=Missing\n\
             TryExec=definitely-not-installed\nExec=missing-app\n",
        );
        write_desktop(
            &apps,
            "installed.desktop",
            "[Desktop Entry]\nType=Application\nName=Installed\n\
             TryExec=installed-tool\nExec=installed-tool\n",
        );
        write_desktop(
            &apps,
            "absolute.desktop",
            &format!(
                "[Desktop Entry]\nType=Application\nName=Absolute\nTryExec={}\n\
                 Exec=absolute-tool\n",
                absolute.display()
            ),
        );
        write_desktop(
            &apps,
            "empty.desktop",
            "[Desktop Entry]\nType=Application\nName=Empty\nTryExec=\nExec=empty-app\n",
        );

        let index =
            AppIndex::from_dirs_with([apps.clone()], ["dragonfruit".to_owned()], [bin.clone()]);
        assert_eq!(
            visible_ids(&index),
            vec!["absolute.desktop", "empty.desktop", "installed.desktop"]
        );

        // Removing the program makes the entry "not installed" on refresh,
        // and the visibility change is an update so consumers re-read.
        fs::remove_file(bin.join("installed-tool")).unwrap();
        let mut index = index;
        let events = index.refresh();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, IndexEventKind::Updated);
        assert_eq!(events[0].desktop_id, "installed.desktop");
        assert_eq!(
            visible_ids(&index),
            vec!["absolute.desktop", "empty.desktop"]
        );

        fs::remove_dir_all(&apps).ok();
        fs::remove_dir_all(&bin).ok();
    }

    #[test]
    fn no_desktop_environment_hides_only_show_in_and_keeps_not_show_in() {
        let dir = temp_dir("no-desktop");
        write_desktop(
            &dir,
            "only.desktop",
            "[Desktop Entry]\nType=Application\nName=Only\nOnlyShowIn=dragonfruit;\n\
             Exec=only\n",
        );
        write_desktop(
            &dir,
            "not.desktop",
            "[Desktop Entry]\nType=Application\nName=Not\nNotShowIn=dragonfruit;\n\
             Exec=not\n",
        );
        write_desktop(
            &dir,
            "normal.desktop",
            "[Desktop Entry]\nType=Application\nName=Normal\nExec=normal\n",
        );

        let index =
            AppIndex::from_dirs_with([dir.clone()], Vec::<String>::new(), Vec::<PathBuf>::new());
        assert_eq!(visible_ids(&index), vec!["normal.desktop", "not.desktop"]);
        assert_eq!(index.desktops(), &[] as &[String]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_display_and_hidden_records_stay_in_the_catalog() {
        let dir = temp_dir("no-display");
        write_desktop(
            &dir,
            "nodisplay.desktop",
            "[Desktop Entry]\nType=Application\nName=No Display\nNoDisplay=true\n\
             Exec=nodisplay\n",
        );
        write_desktop(
            &dir,
            "hidden.desktop",
            "[Desktop Entry]\nType=Application\nName=Hidden\nHidden=true\nExec=hidden\n",
        );

        let index = AppIndex::from_dirs_with(
            [dir.clone()],
            ["dragonfruit".to_owned()],
            Vec::<PathBuf>::new(),
        );
        // The visibility filter is only OnlyShowIn/NotShowIn/TryExec; the
        // shell keeps filtering NoDisplay/Hidden through the record flag.
        assert_eq!(
            visible_ids(&index),
            vec!["hidden.desktop", "nodisplay.desktop"]
        );
        assert!(index.records().all(|record| record.no_display));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn try_exec_lookup_checks_an_absolute_path_directly_and_others_on_path() {
        let bin = temp_dir("try-exec-helper");
        let tool = write_executable(&bin, "tool");
        assert!(try_exec_found("tool", std::slice::from_ref(&bin)));
        assert!(!try_exec_found("tool", &[]));
        assert!(try_exec_found(&tool.to_string_lossy(), &[]));
        assert!(!try_exec_found("/definitely/not/here", &[]));
        assert!(try_exec_found("", &[]));

        fs::remove_dir_all(&bin).ok();
    }
}
