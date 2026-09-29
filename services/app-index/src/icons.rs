// SPDX-License-Identifier: MIT
//! Themed icon lookup (T-14.1a).
//!
//! An application's `.desktop` `Icon` value is a *name* in the XDG icon theme
//! (or, occasionally, an absolute path). The shell needs a real file to draw,
//! so app-index resolves the name against the active icon theme following the
//! freedesktop icon-theme spec:
//!
//! 1. an absolute `Icon` path is passed through when it exists;
//! 2. the configured theme (`$DF_ICON_THEME`, else the GTK
//!    `gtk-icon-theme-name`, else `hicolor`) and its `Inherits` chain;
//! 3. `hicolor` is always the final theme fallback;
//! 4. the `pixmaps` directories are the last resort.
//!
//! Within a theme, a size directory closest to the requested size wins
//! (an exact match first, then `scalable`); `.svg` beats `.png` beats `.xpm`.
//! The lookup is pure over the filesystem so tests drive it against a fixture
//! theme tree.
//!
//! `Enumerate` resolves the icon for every installed entry, so the lookup is
//! memoized and its per-theme size directories and directory listings are
//! cached; without that, a cold enumeration of ~265 entries took ~8s.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The memoized result of one lookup.
type LookupCache = Arc<Mutex<HashMap<(String, i32), Option<PathBuf>>>>;
/// Per-theme candidate size directories: `(size base, directory path)`, where
/// the base is what `size_rank` parses (`48x48`, `scalable`, ...).
type SizeDirCache = Arc<Mutex<HashMap<PathBuf, Vec<(String, PathBuf)>>>>;
/// Immediate subdirectory listings, so repeated lookups do not re-scan.
type ListingCache = Arc<Mutex<HashMap<PathBuf, Vec<PathBuf>>>>;

/// The icon theme resolver: the search roots, the active theme chain, the
/// fallback `pixmaps` directories, and the memoization caches.
#[derive(Debug, Clone)]
pub struct IconTheme {
    /// `<data-dir>/icons` roots, most specific first.
    roots: Vec<PathBuf>,
    /// The active theme and its inheritance chain, in priority order.
    themes: Vec<String>,
    /// `pixmaps` directories (last resort).
    pixmaps: Vec<PathBuf>,
    size_dirs: SizeDirCache,
    listings: ListingCache,
    hits: LookupCache,
}

impl IconTheme {
    /// Build the resolver from the process environment.
    pub fn from_env() -> Self {
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
        let data_dirs = std::env::var_os("XDG_DATA_DIRS")
            .map(|v| v.to_string_lossy().into_owned())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());

        let mut roots = Vec::new();
        if let Some(home) = &data_home {
            roots.push(home.join("icons"));
        }
        for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
            roots.push(PathBuf::from(dir).join("icons"));
        }
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(home).join(".icons"));
        }

        let mut pixmaps = Vec::new();
        if let Some(home) = &data_home {
            pixmaps.push(home.join("pixmaps"));
        }
        for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
            pixmaps.push(PathBuf::from(dir).join("pixmaps"));
        }

        let active = active_theme();
        let themes = theme_chain(&active, &roots);
        IconTheme {
            roots,
            themes,
            pixmaps,
            size_dirs: Arc::new(Mutex::new(HashMap::new())),
            listings: Arc::new(Mutex::new(HashMap::new())),
            hits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Build a resolver from explicit roots and an explicit active theme
    /// (exposed for tests).
    pub fn from_roots(
        roots: impl IntoIterator<Item = PathBuf>,
        themes: impl IntoIterator<Item = String>,
        pixmaps: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        IconTheme {
            roots: roots.into_iter().collect(),
            themes: themes.into_iter().collect(),
            pixmaps: pixmaps.into_iter().collect(),
            size_dirs: Arc::new(Mutex::new(HashMap::new())),
            listings: Arc::new(Mutex::new(HashMap::new())),
            hits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Resolve `name` to a file, preferring an icon directory nearest `size`.
    /// Returns `None` when no theme or pixmap provides the name. Results are
    /// memoized for the life of the resolver.
    pub fn lookup(&self, name: &str, size: i32) -> Option<PathBuf> {
        let key = (name.to_owned(), size);
        if let Some(cached) = lock(&self.hits).get(&key) {
            return cached.clone();
        }
        let result = self.lookup_uncached(name, size);
        lock(&self.hits).insert(key, result.clone());
        result
    }

    fn lookup_uncached(&self, name: &str, size: i32) -> Option<PathBuf> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }

        // 1. Absolute path passthrough.
        let as_path = Path::new(name);
        if as_path.is_absolute() && as_path.is_file() {
            return Some(as_path.to_path_buf());
        }

        // 2. Theme search, in chain order.
        for theme in &self.themes {
            for root in &self.roots {
                let theme_dir = root.join(theme);
                if !theme_dir.is_dir() {
                    continue;
                }
                if let Some(path) = self.find_in_theme(&theme_dir, name, size) {
                    return Some(path);
                }
            }
        }

        // 3. Pixmaps fallback.
        for dir in &self.pixmaps {
            if let Some(path) = self.lookup_in_dir(dir, name) {
                return Some(path);
            }
        }
        None
    }

    fn find_in_theme(&self, theme_dir: &Path, name: &str, size: i32) -> Option<PathBuf> {
        let mut candidates: Vec<(i32, bool, PathBuf)> = self
            .size_dirs(theme_dir)
            .into_iter()
            .map(|(base, dir)| {
                let (distance, scalable) = size_rank(&base, size);
                (distance, scalable, dir)
            })
            .collect();
        // Closest size first; an exact raster match, then scalable, then
        // nearer raster directories.
        candidates.sort_by_key(|(distance, scalable, _)| (*distance, !*scalable));

        for (_, _, dir) in candidates {
            if let Some(path) = self.lookup_in_dir(&dir, name) {
                return Some(path);
            }
            // Icons also live one context level deeper (`48x48/apps/...`).
            if let Some(path) = self.lookup_in_child_dirs(&dir, name) {
                return Some(path);
            }
        }
        None
    }

    /// The candidate size directories of a theme, cached. An `index.theme`
    /// names the real subdirectories (e.g. `48x48/apps`); without one, the
    /// immediate subdirectories are scanned. The returned base is the size
    /// component the caller ranks against the requested size.
    fn size_dirs(&self, theme_dir: &Path) -> Vec<(String, PathBuf)> {
        if let Some(cached) = lock(&self.size_dirs).get(theme_dir) {
            return cached.clone();
        }
        let mut candidates: Vec<(String, PathBuf)> = Vec::new();
        match index_theme_directories(theme_dir) {
            Some(directories) => {
                for rel in directories {
                    let base = rel.split('/').next().unwrap_or(&rel).to_owned();
                    let dir = theme_dir.join(&rel);
                    candidates.push((base, dir));
                }
            }
            None => {
                if let Ok(read) = std::fs::read_dir(theme_dir) {
                    for item in read.flatten() {
                        if !item.path().is_dir() {
                            continue;
                        }
                        let name = item.file_name().to_string_lossy().into_owned();
                        candidates.push((name, item.path()));
                    }
                }
            }
        }
        lock(&self.size_dirs).insert(theme_dir.to_path_buf(), candidates.clone());
        candidates
    }

    fn lookup_in_dir(&self, dir: &Path, name: &str) -> Option<PathBuf> {
        for extension in ["svg", "png", "xpm"] {
            let candidate = dir.join(format!("{name}.{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    fn lookup_in_child_dirs(&self, dir: &Path, name: &str) -> Option<PathBuf> {
        let children = self.children(dir);
        for child in children {
            if let Some(path) = self.lookup_in_dir(&child, name) {
                return Some(path);
            }
        }
        None
    }

    /// The immediate subdirectories of `dir`, cached.
    fn children(&self, dir: &Path) -> Vec<PathBuf> {
        if let Some(cached) = lock(&self.listings).get(dir) {
            return cached.clone();
        }
        let mut children = Vec::new();
        if let Ok(read) = std::fs::read_dir(dir) {
            for item in read.flatten() {
                if item.path().is_dir() {
                    children.push(item.path());
                }
            }
        }
        lock(&self.listings).insert(dir.to_path_buf(), children.clone());
        children
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The active theme name: `$DF_ICON_THEME`, then the GTK
/// `gtk-icon-theme-name`, then `hicolor`.
pub fn active_theme() -> String {
    if let Ok(value) = std::env::var("DF_ICON_THEME") {
        if !value.trim().is_empty() {
            return value.trim().to_owned();
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let settings = PathBuf::from(home).join(".config/gtk-3.0/settings.ini");
        if let Some(name) = read_gtk_icon_theme(&settings) {
            return name;
        }
    }
    "hicolor".to_owned()
}

/// Resolve a theme's `Inherits` chain, always ending in `hicolor`. Cycles are
/// cut. Unknown themes are kept (their directories are simply absent).
fn theme_chain(active: &str, roots: &[PathBuf]) -> Vec<String> {
    let mut chain = vec![active.to_owned()];
    let mut cursor = 0;
    while cursor < chain.len() {
        let theme = chain[cursor].clone();
        cursor += 1;
        for root in roots {
            let index = root.join(&theme).join("index.theme");
            if let Some(inherits) = index_theme_inherits(&index) {
                for parent in inherits {
                    if !chain.iter().any(|existing| existing == &parent) {
                        chain.push(parent);
                    }
                }
            }
        }
    }
    if !chain.iter().any(|theme| theme == "hicolor") {
        chain.push("hicolor".to_owned());
    }
    chain
}

fn read_gtk_icon_theme(path: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("gtk-icon-theme-name=") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

/// The `Directories=` list of an `index.theme`, if present and non-empty.
fn index_theme_directories(theme_dir: &Path) -> Option<Vec<String>> {
    let contents = std::fs::read_to_string(theme_dir.join("index.theme")).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Directories=") {
            let dirs: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect();
            if !dirs.is_empty() {
                return Some(dirs);
            }
        }
    }
    None
}

/// The `Inherits=` list of an `index.theme`.
fn index_theme_inherits(index: &Path) -> Option<Vec<String>> {
    let contents = std::fs::read_to_string(index).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Inherits=") {
            let themes: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect();
            if !themes.is_empty() {
                return Some(themes);
            }
        }
    }
    None
}

/// Rank a size directory: exact matches first, then `scalable`, then the
/// nearest raster directory. `size` is the requested size; a `0` request only
/// orders exact-vs-scalable, and the caller re-sorts per request.
fn size_rank(dir_name: &str, size: i32) -> (i32, bool) {
    let base = dir_name.split('/').next().unwrap_or(dir_name);
    if base.eq_ignore_ascii_case("scalable") || base.eq_ignore_ascii_case("symbolic") {
        return (1, true);
    }
    let number: Option<i32> = base
        .split(|c: char| !c.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse().ok());
    match number {
        Some(value) => {
            let distance = (value - size).abs();
            if distance == 0 {
                (0, false)
            } else {
                (2 + distance, false)
            }
        }
        None => (i32::MAX / 2, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "df-icons-{}-{}-{}",
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

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"icon").unwrap();
    }

    fn write_text(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn finds_the_exact_size_in_the_active_theme() {
        let root = temp_dir("exact");
        touch(&root.join("hicolor/48x48/apps/firefox.png"));

        let theme = IconTheme::from_roots([root.clone()], ["hicolor".to_owned()], []);
        let path = theme.lookup("firefox", 48).unwrap();
        assert_eq!(path, root.join("hicolor/48x48/apps/firefox.png"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_exact_size_beats_scalable_and_the_nearest_raster_wins_without_one() {
        let root = temp_dir("near");
        touch(&root.join("hicolor/16x16/apps/firefox.png"));
        touch(&root.join("hicolor/64x64/apps/firefox.png"));
        touch(&root.join("hicolor/scalable/apps/firefox.svg"));

        let scalable_only = IconTheme::from_roots([root.clone()], ["hicolor".to_owned()], []);
        // No exact 48: scalable wins over the 64px raster.
        assert_eq!(
            scalable_only.lookup("firefox", 48).unwrap(),
            root.join("hicolor/scalable/apps/firefox.svg")
        );

        // With an exact 48px directory the raster exact match wins (a fresh
        // resolver; the live theme does not change during a session).
        touch(&root.join("hicolor/48x48/apps/firefox.png"));
        let with_exact = IconTheme::from_roots([root.clone()], ["hicolor".to_owned()], []);
        assert_eq!(
            with_exact.lookup("firefox", 48).unwrap(),
            root.join("hicolor/48x48/apps/firefox.png")
        );

        // Without any scalable icon, the nearest raster directory wins.
        let raster = temp_dir("raster");
        touch(&raster.join("hicolor/16x16/apps/thing.png"));
        touch(&raster.join("hicolor/64x64/apps/thing.png"));
        let theme = IconTheme::from_roots([raster.clone()], ["hicolor".to_owned()], []);
        assert_eq!(
            theme.lookup("thing", 48).unwrap(),
            raster.join("hicolor/64x64/apps/thing.png")
        );

        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&raster).ok();
    }

    #[test]
    fn honors_index_theme_directories_and_inherits() {
        let root = temp_dir("inherits");
        write_text(
            &root.join("MyTheme/index.theme"),
            "[Icon Theme]\nName=MyTheme\nInherits=Parent\nDirectories=48x48/apps\n",
        );
        touch(&root.join("MyTheme/48x48/apps/mine.png"));
        write_text(
            &root.join("Parent/index.theme"),
            "[Icon Theme]\nName=Parent\nDirectories=scalable/apps\n",
        );
        touch(&root.join("Parent/scalable/apps/inherited.svg"));

        let themes = theme_chain("MyTheme", std::slice::from_ref(&root));
        let theme = IconTheme::from_roots([root.clone()], themes, []);
        assert_eq!(
            theme.lookup("mine", 48).unwrap(),
            root.join("MyTheme/48x48/apps/mine.png")
        );
        assert_eq!(
            theme.lookup("inherited", 48).unwrap(),
            root.join("Parent/scalable/apps/inherited.svg")
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn absolute_paths_pass_through_and_missing_names_are_none() {
        let root = temp_dir("absolute");
        let absolute = root.join("custom.png");
        touch(&absolute);

        let theme = IconTheme::from_roots([root.clone()], ["hicolor".to_owned()], []);
        assert_eq!(theme.lookup(absolute.to_str().unwrap(), 48), Some(absolute));
        assert_eq!(theme.lookup("no-such-icon-xyz", 48), None);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn pixmaps_are_the_last_resort() {
        let root = temp_dir("pixmaps");
        let pixmaps = temp_dir("pixmaps-dir");
        touch(&pixmaps.join("legacy.xpm"));

        let theme =
            IconTheme::from_roots([root.clone()], ["hicolor".to_owned()], [pixmaps.clone()]);
        assert_eq!(theme.lookup("legacy", 48), Some(pixmaps.join("legacy.xpm")));

        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&pixmaps).ok();
    }

    #[test]
    fn repeated_lookups_are_memoized() {
        let root = temp_dir("memo");
        touch(&root.join("hicolor/48x48/apps/firefox.png"));
        let theme = IconTheme::from_roots([root.clone()], ["hicolor".to_owned()], []);
        let first = theme.lookup("firefox", 48).unwrap();
        // Remove the file: the memoized hit must still answer.
        fs::remove_file(root.join("hicolor/48x48/apps/firefox.png")).unwrap();
        assert_eq!(theme.lookup("firefox", 48), Some(first));
        // A miss is memoized too.
        assert_eq!(theme.lookup("absent", 48), None);
        assert_eq!(theme.lookup("absent", 48), None);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn shipped_first_party_app_icons_resolve_in_hicolor() {
        // T-19.1d: the shipped `.desktop` `Icon=` names must resolve against
        // the installed hicolor theme, and the installed payload is the same
        // asset the Dock bundles. A clean session resolves the launcher/menu
        // icon by name, so the name and the asset cannot drift.
        let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/apps");
        let root = temp_dir("first-party");
        for name in ["org.dragonfruit.Files.svg", "org.dragonfruit.Settings.svg"] {
            let src = assets.join(name);
            assert!(src.is_file(), "missing shipped asset {}", src.display());
            write_text(
                &root.join("hicolor/scalable/apps").join(name),
                &fs::read_to_string(&src).unwrap(),
            );
        }
        let theme = IconTheme::from_roots([root.clone()], ["hicolor".to_owned()], []);
        let apps = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps");
        for (desktop, name) in [
            (
                "files/org.dragonfruit.Files.desktop",
                "org.dragonfruit.Files",
            ),
            (
                "settings/org.dragonfruit.Settings.desktop",
                "org.dragonfruit.Settings",
            ),
        ] {
            let text = fs::read_to_string(apps.join(desktop)).expect("read shipped entry");
            let icon = text
                .lines()
                .find_map(|line| line.strip_prefix("Icon="))
                .expect("the shipped entry names an icon");
            assert_eq!(icon, name, "the .desktop Icon= matches the shipped asset");
            assert!(
                theme.lookup(icon, 128).is_some(),
                "{icon} resolves in hicolor"
            );
        }

        fs::remove_dir_all(&root).ok();
    }
}
