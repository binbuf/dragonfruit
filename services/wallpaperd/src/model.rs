// SPDX-License-Identifier: MIT
//! The provider's typed model: the six categories, the cached item with its
//! attribution, the catalogue, and the service status.
//!
//! Nothing here touches the network or the bus; the model is the pure data the
//! parser fills, the cache stores, and the D-Bus surface projects.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The six Dragonfruit wallpaper categories, one Wikimedia Commons category
/// each (ADR [0055](../../../docs/design/adr/0055-online-wallpaper-content-provider.md)).
///
/// The slug is also the cache directory name, so the ordering is part of the
/// cache layout and must not be reordered casually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// `Category:Featured pictures of nature`.
    Nature,
    /// `Category:Featured pictures of bodies of water`.
    Water,
    /// `Category:Featured pictures of cityscapes`.
    Cityscapes,
    /// `Category:Featured pictures of Earth from space`.
    Earth,
    /// `Category:Featured pictures of landscapes`.
    Scenery,
    /// `Category:Featured pictures taken underwater`.
    Underwater,
}

impl Category {
    /// Every category, in cache/query order.
    pub const ALL: [Category; 6] = [
        Category::Nature,
        Category::Water,
        Category::Cityscapes,
        Category::Earth,
        Category::Scenery,
        Category::Underwater,
    ];

    /// The lowercase directory name and JSON discriminator.
    pub const fn slug(self) -> &'static str {
        match self {
            Category::Nature => "nature",
            Category::Water => "water",
            Category::Cityscapes => "cityscapes",
            Category::Earth => "earth",
            Category::Scenery => "scenery",
            Category::Underwater => "underwater",
        }
    }

    /// The exact Commons category title the query asks for.
    pub const fn commons_title(self) -> &'static str {
        match self {
            Category::Nature => "Category:Featured pictures of nature",
            Category::Water => "Category:Featured pictures of bodies of water",
            Category::Cityscapes => "Category:Featured pictures of cityscapes",
            Category::Earth => "Category:Featured pictures of Earth from space",
            Category::Scenery => "Category:Featured pictures of landscapes",
            Category::Underwater => "Category:Featured pictures taken underwater",
        }
    }

    /// The human label the pane shows.
    pub const fn label(self) -> &'static str {
        match self {
            Category::Nature => "Nature",
            Category::Water => "Water",
            Category::Cityscapes => "Cityscapes",
            Category::Earth => "Earth",
            Category::Scenery => "Scenery",
            Category::Underwater => "Underwater",
        }
    }

    /// Look a category up by slug, Commons title, or label (case-insensitive).
    pub fn parse(text: &str) -> Option<Category> {
        let text = text.trim();
        Category::ALL.into_iter().find(|category| {
            category.slug().eq_ignore_ascii_case(text)
                || category.commons_title().eq_ignore_ascii_case(text)
                || category.label().eq_ignore_ascii_case(text)
        })
    }
}

/// The provider's lifecycle status, the `Status` D-Bus property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// Loaded from cache (or fresh), nothing to do; the service is lazy.
    #[default]
    Idle,
    /// A catalogue refresh is in flight.
    Fetching,
    /// The catalogue is populated and current enough.
    Ready,
    /// The last refresh could not reach the network; the cache is served as-is.
    Offline,
}

impl Status {
    /// The wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Status::Idle => "idle",
            Status::Fetching => "fetching",
            Status::Ready => "ready",
            Status::Offline => "offline",
        }
    }
}

/// One cached Featured picture: the local file plus the attribution the pane
/// must display.
///
/// `source_url` is the normalized (utm-stripped) thumbnail URL the bytes came
/// from; `local_path` is where they were written, or `None` when the download
/// failed. `fetched_at` is Unix seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperItem {
    /// The Commons `pageid`, the dedupe key.
    pub pageid: u64,
    /// The file title (`File:…`).
    pub title: String,
    /// The artist/author, HTML stripped.
    pub artist: String,
    /// The license short name (`CC BY-SA 4.0`, `Public domain`, …).
    pub license_short_name: String,
    /// The license URL.
    pub license_url: String,
    /// The Commons file page URL.
    pub page_url: String,
    /// The description, HTML stripped.
    pub description: String,
    /// The Dragonfruit category this item was fetched under.
    pub category: Category,
    /// Original width in pixels.
    pub width: u32,
    /// Original height in pixels.
    pub height: u32,
    /// The MIME type (`image/jpeg`, …).
    pub mime: String,
    /// The normalized source (thumbnail) URL.
    pub source_url: String,
    /// The local cache path when the download succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<PathBuf>,
    /// When this item was fetched, Unix seconds.
    pub fetched_at: u64,
}

impl WallpaperItem {
    /// A file URL or path the compositor can consume, when the image is local.
    pub fn local(&self) -> Option<&std::path::Path> {
        self.local_path.as_deref()
    }
}

/// The whole fetched catalogue, in fetch order: categories in [`Category::ALL`]
/// order, each category's items in timestamp-descending order as the API
/// returned them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Catalogue {
    /// Every item, in fetch order.
    pub items: Vec<WallpaperItem>,
}

impl Catalogue {
    /// An empty catalogue.
    pub fn new() -> Self {
        Catalogue::default()
    }

    /// The items, in fetch order.
    pub fn items(&self) -> &[WallpaperItem] {
        &self.items
    }

    /// How many items are cached.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether nothing is cached yet.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items of one category, in fetch order.
    pub fn items_for(&self, category: Category) -> impl Iterator<Item = &WallpaperItem> {
        self.items
            .iter()
            .filter(move |item| item.category == category)
    }

    /// The first Nature entry under timestamp-desc: the Featured row's default
    /// and the fallback when the shipped asset is unavailable (ADR 0094).
    ///
    /// This is the deterministic rule from ADR 0055, not a heuristic: the API
    /// returns each category timestamp-descending, so the first Nature item in
    /// fetch order is the newest featured nature picture.
    pub fn featured_default(&self) -> Option<&WallpaperItem> {
        self.items
            .iter()
            .find(|item| item.category == Category::Nature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_round_trips_through_slug_title_and_label() {
        for category in Category::ALL {
            assert_eq!(Category::parse(category.slug()), Some(category));
            assert_eq!(Category::parse(category.commons_title()), Some(category));
            assert_eq!(Category::parse(category.label()), Some(category));
        }
        assert_eq!(Category::parse("NATURE"), Some(Category::Nature));
        assert_eq!(Category::parse("bogus"), None);
    }

    #[test]
    fn the_commons_titles_are_the_adr_0055_map() {
        assert_eq!(
            Category::Nature.commons_title(),
            "Category:Featured pictures of nature"
        );
        assert_eq!(
            Category::Underwater.commons_title(),
            "Category:Featured pictures taken underwater"
        );
    }

    #[test]
    fn featured_default_is_the_first_nature_entry_in_fetch_order() {
        let mk = |pageid, category| WallpaperItem {
            pageid,
            title: String::new(),
            artist: String::new(),
            license_short_name: String::new(),
            license_url: String::new(),
            page_url: String::new(),
            description: String::new(),
            category,
            width: 1,
            height: 1,
            mime: "image/jpeg".to_owned(),
            source_url: String::new(),
            local_path: None,
            fetched_at: 0,
        };
        let catalogue = Catalogue {
            items: vec![
                mk(1, Category::Scenery),
                mk(2, Category::Nature),
                mk(3, Category::Nature),
            ],
        };
        assert_eq!(catalogue.featured_default().unwrap().pageid, 2);
        assert!(Catalogue::new().featured_default().is_none());
    }

    #[test]
    fn statuses_have_stable_wire_spellings() {
        assert_eq!(Status::Idle.as_str(), "idle");
        assert_eq!(Status::Fetching.as_str(), "fetching");
        assert_eq!(Status::Ready.as_str(), "ready");
        assert_eq!(Status::Offline.as_str(), "offline");
    }
}
