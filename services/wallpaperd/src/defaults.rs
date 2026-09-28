// SPDX-License-Identifier: MIT
//! The shipped original default wallpaper and its resolution order (ADR 0094).
//!
//! `assets/graphics/wallpapers/Default.jpg` is the project's own MIT asset and
//! the out-of-box, first-run, and cold-cache/offline background. It is
//! installed to `$prefix/share/dragonfruit/wallpapers/Default.jpg` at package
//! time and read in place; it is never copied into the provider cache.
//!
//! Resolution order (first existing wins):
//!
//! 1. `DF_DEFAULT_WALLPAPER` — a dev/test override pointing at a file.
//! 2. The installed share path, searched over `XDG_DATA_DIRS`
//!    (`/usr/local/share:/usr/share` by default): `<dir>/dragonfruit/wallpapers/Default.jpg`.
//! 3. The in-tree `assets/graphics/wallpapers/Default.jpg` (so `make dev`
//!    works straight from the checkout).
//!
//! [`BuiltinDefaultSource`](crate::provider::Provider::builtin_default_source)
//! is this resolved path; it is distinct from the fetched `DefaultSource`.

use std::io;
use std::path::{Path, PathBuf};

/// The shipped asset's filename.
pub const DEFAULT_FILENAME: &str = "Default.jpg";
/// The share subdirectory the asset installs to (`share/` is implied by the
/// data dir): `<prefix>/share/dragonfruit/wallpapers/Default.jpg`.
pub const SHARE_SUBDIR: &str = "dragonfruit/wallpapers";
/// The environment variable that overrides the whole resolution.
pub const ENV_OVERRIDE: &str = "DF_DEFAULT_WALLPAPER";
/// The XDG data-dir list.
pub const ENV_DATA_DIRS: &str = "XDG_DATA_DIRS";
/// The POSIX default data dirs, used when `XDG_DATA_DIRS` is unset.
pub const DEFAULT_DATA_DIRS: [&str; 2] = ["/usr/local/share", "/usr/share"];

/// The in-tree asset path, resolved at compile time from the crate location.
pub fn in_tree_default() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/graphics/wallpapers")
        .join(DEFAULT_FILENAME)
}

/// A testable, environment-independent resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultResolver {
    /// The `DF_DEFAULT_WALLPAPER` override, when set.
    pub override_path: Option<PathBuf>,
    /// The data dirs to search for the installed asset.
    pub data_dirs: Vec<PathBuf>,
    /// The in-tree fallback.
    pub in_tree: PathBuf,
}

impl DefaultResolver {
    /// The resolver the service runs with, read from the environment.
    pub fn from_env() -> Self {
        let override_path = std::env::var_os(ENV_OVERRIDE)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let data_dirs = std::env::var(ENV_DATA_DIRS)
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| value.split(':').map(PathBuf::from).collect())
            .unwrap_or_else(|| DEFAULT_DATA_DIRS.iter().map(PathBuf::from).collect());
        DefaultResolver {
            override_path,
            data_dirs,
            in_tree: in_tree_default(),
        }
    }

    /// Resolve against the real filesystem.
    pub fn resolve(&self) -> Option<PathBuf> {
        self.resolve_with(|path| path.is_file())
    }

    /// Resolve against an arbitrary existence predicate (the test seam).
    pub fn resolve_with(&self, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
        if let Some(path) = &self.override_path {
            if exists(path) {
                return Some(path.clone());
            }
        }
        for dir in &self.data_dirs {
            let path = dir.join(SHARE_SUBDIR).join(DEFAULT_FILENAME);
            if exists(&path) {
                return Some(path);
            }
        }
        if exists(&self.in_tree) {
            return Some(self.in_tree.clone());
        }
        None
    }
}

/// Resolve the shipped default from the environment, or `None` when the asset
/// is nowhere to be found (a normal state; the desktop then falls back).
pub fn resolve_default() -> Option<PathBuf> {
    DefaultResolver::from_env().resolve()
}

/// Copy the shipped asset to `<prefix>/share/dragonfruit/wallpapers/Default.jpg`
/// for packaging (T-32 calls this at build/package time). Returns the installed
/// path.
pub fn install_default(prefix: &Path) -> io::Result<PathBuf> {
    install_default_from(&in_tree_default(), prefix)
}

/// [`install_default`] with an explicit source, for tests.
pub fn install_default_from(source: &Path, prefix: &Path) -> io::Result<PathBuf> {
    let dir = prefix.join("share").join(SHARE_SUBDIR);
    std::fs::create_dir_all(&dir)?;
    let destination = dir.join(DEFAULT_FILENAME);
    std::fs::copy(source, &destination)?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver(override_path: Option<&str>, data_dirs: &[&str], in_tree: &str) -> DefaultResolver {
        DefaultResolver {
            override_path: override_path.map(PathBuf::from),
            data_dirs: data_dirs.iter().map(PathBuf::from).collect(),
            in_tree: PathBuf::from(in_tree),
        }
    }

    #[test]
    fn the_override_wins_when_present() {
        let resolver = resolver(Some("/dev/Default.jpg"), &["/share"], "/tree/Default.jpg");
        let resolved = resolver.resolve_with(|path| path == Path::new("/dev/Default.jpg"));
        assert_eq!(resolved, Some(PathBuf::from("/dev/Default.jpg")));
    }

    #[test]
    fn the_installed_share_path_beats_the_in_tree_fallback() {
        let resolver = resolver(
            None,
            &["/usr/local/share", "/usr/share"],
            "/tree/Default.jpg",
        );
        let installed = PathBuf::from("/usr/share")
            .join(SHARE_SUBDIR)
            .join(DEFAULT_FILENAME);
        let probe = installed.clone();
        let resolved = resolver.resolve_with(move |path| path == probe);
        assert_eq!(resolved, Some(installed));
    }

    #[test]
    fn the_in_tree_copy_is_the_last_resort() {
        let resolver = resolver(None, &["/usr/share"], "/tree/Default.jpg");
        let resolved = resolver.resolve_with(|path| path == Path::new("/tree/Default.jpg"));
        assert_eq!(resolved, Some(PathBuf::from("/tree/Default.jpg")));
    }

    #[test]
    fn nothing_found_is_none_not_an_error() {
        let resolver = resolver(None, &["/usr/share"], "/tree/Default.jpg");
        assert_eq!(resolver.resolve_with(|_| false), None);
    }

    #[test]
    fn a_missing_override_falls_through_to_the_rest() {
        let resolver = resolver(Some("/missing.jpg"), &["/share"], "/tree/Default.jpg");
        let resolved = resolver.resolve_with(|path| path == Path::new("/tree/Default.jpg"));
        assert_eq!(resolved, Some(PathBuf::from("/tree/Default.jpg")));
    }

    #[test]
    fn install_copies_the_asset_to_the_share_path() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.jpg");
        std::fs::write(&source, b"default").unwrap();
        let prefix = dir.path().join("root");
        let installed = install_default_from(&source, &prefix).unwrap();
        assert_eq!(
            installed,
            prefix.join("share/dragonfruit/wallpapers/Default.jpg")
        );
        assert_eq!(std::fs::read(&installed).unwrap(), b"default");

        // The installed copy resolves via the share path with that prefix.
        let resolver = DefaultResolver {
            override_path: None,
            data_dirs: vec![prefix.join("share")],
            in_tree: source.clone(),
        };
        assert_eq!(resolver.resolve(), Some(installed));
    }

    #[test]
    fn the_in_tree_asset_exists_in_the_checkout() {
        // `make dev` relies on this: the checkout carries the shipped asset.
        assert!(
            in_tree_default().is_file(),
            "missing in-tree default: {}",
            in_tree_default().display()
        );
    }
}
