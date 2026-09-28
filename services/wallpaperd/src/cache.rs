// SPDX-License-Identifier: MIT
//! The lazy local cache: where fetched thumbnails and their attribution index
//! live (ADR 0094).
//!
//! Layout: `$XDG_CACHE_HOME/dragonfruit/wallpapers/` holds `index.json` and one
//! directory per category, `<category>/<pageid>.<ext>`. Only *fetched* content
//! is ever written here; the shipped default is read in place and never copied
//! into the cache.
//!
//! A cold start rebuilds the catalogue from `index.json` with no network
//! round-trip: [`CacheLayout::load_index`] is the whole cold path.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Category, WallpaperItem};

/// The refresh cadence: a re-check roughly once a week.
pub const WEEK_SECS: u64 = 7 * 24 * 60 * 60;
/// The provider's directory under `$XDG_CACHE_HOME`.
pub const APP_SUBDIR: &str = "dragonfruit/wallpapers";
/// The catalogue index filename.
pub const INDEX_FILE: &str = "index.json";

/// The persisted catalogue: the items and the last successful fetch time.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    /// Unix seconds of the last successful catalogue refresh; 0 means never.
    #[serde(default)]
    pub last_fetch: u64,
    /// Every cached item.
    #[serde(default)]
    pub items: Vec<WallpaperItem>,
}

impl Index {
    /// Whether nothing has been cached yet.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// The cache root, with the pure path math the provider and tests share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLayout {
    root: PathBuf,
}

impl CacheLayout {
    /// A layout rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        CacheLayout { root: root.into() }
    }

    /// The standard layout from the environment: `$XDG_CACHE_HOME` (or
    /// `$HOME/.cache`) plus [`APP_SUBDIR`]. `None` when neither is set.
    pub fn from_env() -> Option<Self> {
        let base = std::env::var_os("XDG_CACHE_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(|home| PathBuf::from(home).join(".cache"))
            })?;
        Some(CacheLayout::new(base.join(APP_SUBDIR)))
    }

    /// The cache root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The catalogue index path.
    pub fn index_path(&self) -> PathBuf {
        self.root.join(INDEX_FILE)
    }

    /// The directory holding one category's images.
    pub fn category_dir(&self, category: Category) -> PathBuf {
        self.root.join(category.slug())
    }

    /// The path for one image: `<root>/<slug>/<pageid>.<ext>`.
    pub fn image_path(&self, category: Category, pageid: u64, extension: &str) -> PathBuf {
        self.category_dir(category)
            .join(format!("{pageid}.{extension}"))
    }

    /// Read `index.json`, falling back to an empty index on a missing or
    /// malformed file (absence is normal, never an error).
    pub fn load_index(&self) -> Index {
        match std::fs::read_to_string(self.index_path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Index::default(),
        }
    }

    /// Write `index.json`, creating the root.
    pub fn save_index(&self, index: &Index) -> io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let text = serde_json::to_string_pretty(index).unwrap_or_else(|_| "{}".to_owned());
        std::fs::write(self.index_path(), text)
    }

    /// Write one downloaded image to `<category>/<pageid>.<ext>` and return the
    /// path written.
    pub fn write_image(
        &self,
        category: Category,
        pageid: u64,
        mime: &str,
        source_url: &str,
        bytes: &[u8],
    ) -> io::Result<PathBuf> {
        let dir = self.category_dir(category);
        std::fs::create_dir_all(&dir)?;
        let path = self.image_path(category, pageid, extension_for(mime, source_url));
        std::fs::write(&path, bytes)?;
        Ok(path)
    }
}

/// The file extension for a MIME type, falling back to the URL's own suffix and
/// finally `jpg`.
pub fn extension_for<'a>(mime: &str, source_url: &'a str) -> &'a str {
    match mime {
        "image/jpeg" | "image/jpg" => return "jpg",
        "image/png" => return "png",
        "image/webp" => return "webp",
        "image/gif" => return "gif",
        "image/tiff" => return "tif",
        _ => {}
    }
    if let Some(ext) = url_extension(source_url) {
        return ext;
    }
    "jpg"
}

/// The last path segment's extension, when it is a short ASCII slug.
fn url_extension(url: &str) -> Option<&str> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let ext = path.rsplit('.').next()?;
    if ext.len() <= 5 && !ext.contains('/') && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(ext)
    } else {
        None
    }
}

/// Whether the last fetch is old enough to re-check.
pub fn is_stale(last_fetch: u64, now: u64) -> bool {
    last_fetch == 0 || now.saturating_sub(last_fetch) >= WEEK_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(pageid: u64, category: Category, last_fetch: u64) -> WallpaperItem {
        WallpaperItem {
            pageid,
            title: "File:X.jpg".to_owned(),
            artist: "A".to_owned(),
            license_short_name: "CC BY-SA 4.0".to_owned(),
            license_url: "https://l".to_owned(),
            page_url: "https://p".to_owned(),
            description: "d".to_owned(),
            category,
            width: 1,
            height: 1,
            mime: "image/jpeg".to_owned(),
            source_url: "https://u/x.jpg".to_owned(),
            local_path: None,
            fetched_at: last_fetch,
        }
    }

    #[test]
    fn image_paths_follow_the_cache_layout() {
        let cache = CacheLayout::new("/cache/dragonfruit/wallpapers");
        assert_eq!(
            cache.category_dir(Category::Earth),
            Path::new("/cache/dragonfruit/wallpapers/earth")
        );
        assert_eq!(
            cache.image_path(Category::Underwater, 42, "jpg"),
            Path::new("/cache/dragonfruit/wallpapers/underwater/42.jpg")
        );
    }

    #[test]
    fn extensions_follow_the_mime_and_the_url() {
        assert_eq!(extension_for("image/jpeg", "https://u/x.png"), "jpg");
        assert_eq!(extension_for("image/png", "https://u/x"), "png");
        assert_eq!(extension_for("image/webp", "https://u/x"), "webp");
        assert_eq!(
            extension_for("application/octet-stream", "https://u/x.jpeg"),
            "jpeg"
        );
        assert_eq!(
            extension_for("application/octet-stream", "https://u/x"),
            "jpg"
        );
    }

    #[test]
    fn index_round_trips_and_a_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path().join("wallpapers"));
        assert!(cache.load_index().is_empty());

        let index = Index {
            last_fetch: 100,
            items: vec![item(1, Category::Nature, 100)],
        };
        cache.save_index(&index).unwrap();
        let loaded = cache.load_index();
        assert_eq!(loaded, index);
        assert_eq!(loaded.items[0].category, Category::Nature);
    }

    #[test]
    fn a_malformed_index_reads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        std::fs::write(cache.index_path(), "not json").unwrap();
        assert!(cache.load_index().is_empty());
    }

    #[test]
    fn write_image_creates_the_category_directory() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let path = cache
            .write_image(Category::Water, 9, "image/jpeg", "https://u/9.jpg", b"img")
            .unwrap();
        assert_eq!(path, dir.path().join("water/9.jpg"));
        assert_eq!(std::fs::read(&path).unwrap(), b"img");
    }

    #[test]
    fn staleness_is_weekly_and_never_fetched_is_stale() {
        assert!(is_stale(0, 1000));
        assert!(!is_stale(1000, 1000 + WEEK_SECS - 1));
        assert!(is_stale(1000, 1000 + WEEK_SECS));
        assert!(is_stale(1000, 1000 + WEEK_SECS * 3));
    }
}
