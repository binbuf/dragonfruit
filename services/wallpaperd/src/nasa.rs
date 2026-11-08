// SPDX-License-Identifier: MIT
//! The NASA Image and Video Library source (ADR 0240).
//!
//! NASA is the additive Space provider. Its search endpoint is keyless; the
//! parser keeps only landscape, NASA-provenance images of at least
//! 1920×1080 px, filters obvious logo/poster/graphic records, and returns the
//! medium thumbnail plus the original URL. The catalogue fetch downloads only
//! the thumbnail; [`ContentSource::download_full`] fetches the original when
//! the user actually applies the picture.
//!
//! NASA content is generally public domain under the NASA media guidelines,
//! but the agency insignia/logos and some third-party material are not. The
//! provenance and text filters here keep those out of the catalogue, and the
//! pane credits NASA/center and links the media guidelines.

use std::collections::HashSet;

use serde::Deserialize;

use crate::model::Category;
use crate::source::{ContentSource, HttpClient};
use crate::wikipedia::{encode_component, strip_html, ParseError, ParsedItem};

/// The NASA Image and Video Library search endpoint.
pub const NASA_API: &str = "https://images-api.nasa.gov/search";
/// The NASA media-usage guidelines shown as the license link.
pub const NASA_MEDIA_GUIDELINES: &str = "https://www.nasa.gov/nasa-brand-center/images-and-media/";
/// The public detail page prefix.
pub const NASA_DETAILS: &str = "https://images.nasa.gov/details/";
/// The search page size requested per query.
pub const PAGE_SIZE: u32 = 20;
/// The most items kept for the Space category.
pub const PER_CATEGORY_LIMIT: usize = 10;
/// The minimum accepted full-resolution width.
pub const MIN_WIDTH: u32 = 1920;
/// The minimum accepted full-resolution height.
pub const MIN_HEIGHT: u32 = 1080;
/// The curated queries; the catalogue takes a few from each for variety.
pub const SPACE_QUERIES: [&str; 4] = ["nebula", "galaxy", "aurora", "star cluster"];

/// Text that marks a record as a logo, poster, chart, or other non-wallpaper
/// graphic. Matching is case-insensitive across title + description.
const FORBIDDEN_WORDS: [&str; 9] = [
    "logo",
    "insignia",
    "meatball",
    "worm logo",
    "poster",
    "chart",
    "diagram",
    "infographic",
    "text graphic",
];

/// Build the URL for one search page.
pub fn build_query(query: &str) -> String {
    format!(
        "{NASA_API}?q={}&media_type=image&page_size={PAGE_SIZE}&page=1",
        encode_component(query)
    )
}

/// A stable positive 31-bit id for a NASA `nasa_id`.
///
/// The cache layout and the D-Bus `FetchItem` method use a numeric key, while
/// NASA ids are strings. FNV-1a over the id is stable across releases; masking
/// the sign bit keeps the value inside QML's positive `int` range.
pub fn nasa_pageid(nasa_id: &str) -> u64 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in nasa_id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    u64::from(hash & 0x7fff_ffff)
}

/// Parse one NASA search response into filtered catalogue items.
pub fn parse_response(json: &str) -> Result<Vec<ParsedItem>, ParseError> {
    let response: Response =
        serde_json::from_str(json).map_err(|error| ParseError(error.to_string()))?;
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    if let Some(collection) = response.collection {
        for item in collection.items {
            if items.len() >= PAGE_SIZE as usize {
                break;
            }
            let Some(data) = item.data.into_iter().next() else {
                continue;
            };
            if data.media_type != "image" || data.nasa_id.is_empty() {
                continue;
            }
            if !nasa_provenance(&data) {
                continue;
            }
            let text = format!("{} {}", data.title, data.description).to_lowercase();
            if FORBIDDEN_WORDS.iter().any(|word| text.contains(word)) {
                continue;
            }
            let Some((full_url, width, height)) = best_full_link(&item.links) else {
                continue;
            };
            let Some(thumb_url) = best_thumb_link(&item.links) else {
                continue;
            };
            let pageid = nasa_pageid(&data.nasa_id);
            if !seen.insert(pageid) {
                continue;
            }
            let artist = if !data.secondary_creator.trim().is_empty() {
                strip_html(&data.secondary_creator)
            } else if !data.center.trim().is_empty() {
                format!("{} / NASA", strip_html(&data.center))
            } else {
                "NASA".to_owned()
            };
            items.push(ParsedItem {
                pageid,
                title: strip_html(&data.title),
                artist,
                license_short_name: "Public domain (NASA)".to_owned(),
                license_url: NASA_MEDIA_GUIDELINES.to_owned(),
                page_url: format!("{NASA_DETAILS}{}", data.nasa_id),
                description: strip_html(&data.description),
                category: Category::Space,
                width,
                height,
                mime: "image/jpeg".to_owned(),
                source_url: thumb_url,
                full_url,
                provider: "nasa".to_owned(),
                defer_full: true,
            });
        }
    }
    Ok(items)
}

/// Whether the record is NASA-produced (or from a NASA center) rather than a
/// third-party contribution.
fn nasa_provenance(data: &Data) -> bool {
    let creator = data.secondary_creator.trim();
    if !creator.is_empty() {
        return creator.to_lowercase().contains("nasa");
    }
    !data.center.trim().is_empty()
}

/// The largest landscape image link of at least 1920×1080, normalized to
/// https.
fn best_full_link(links: &[Link]) -> Option<(String, u32, u32)> {
    let mut best: Option<(&Link, u32, u32)> = None;
    for link in links {
        if link.render != "image" && !is_image_href(&link.href) {
            continue;
        }
        if link.width < MIN_WIDTH || link.height < MIN_HEIGHT || link.width <= link.height {
            continue;
        }
        let area = link.width.saturating_mul(link.height);
        let best_area = best.map_or(0, |(_, w, h)| w.saturating_mul(h));
        if area > best_area {
            best = Some((link, link.width, link.height));
        }
    }
    best.map(|(link, width, height)| (https_url(&link.href), width, height))
}

/// The best grid thumbnail: a landscape image smaller than the full link.
fn best_thumb_link(links: &[Link]) -> Option<String> {
    let mut best: Option<(&Link, u32)> = None;
    for link in links {
        if link.render != "image" && !is_image_href(&link.href) {
            continue;
        }
        if link.width == 0 || link.height == 0 || link.width <= link.height {
            continue;
        }
        if link.width > 1920 {
            continue;
        }
        let area = link.width.saturating_mul(link.height);
        if best.is_none() || area > best.map_or(0, |(_, a)| a) {
            best = Some((link, area));
        }
    }
    best.map(|(link, _)| https_url(&link.href))
}

/// Whether a link href looks like a raster image.
fn is_image_href(href: &str) -> bool {
    let path = href.split(['?', '#']).next().unwrap_or(href).to_lowercase();
    path.ends_with(".jpg") || path.ends_with(".jpeg") || path.ends_with(".png")
}

/// Normalize a NASA asset URL to https.
fn https_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("https://{rest}")
    } else if let Some(rest) = url.strip_prefix("//") {
        format!("https://{rest}")
    } else {
        url.to_owned()
    }
}

/// NASA Image and Video Library over an [`HttpClient`].
pub struct NasaSource<H> {
    http: H,
}

impl<H> NasaSource<H> {
    /// A source over `http`.
    pub fn new(http: H) -> Self {
        NasaSource { http }
    }

    /// The transport, for tests.
    pub fn http(&self) -> &H {
        &self.http
    }
}

impl<H: HttpClient> ContentSource for NasaSource<H> {
    fn catalogue(&self, category: Category) -> Result<Vec<ParsedItem>, String> {
        if category != Category::Space {
            return Err(format!("{} is not a NASA category", category.slug()));
        }
        let mut pages = Vec::new();
        for query in SPACE_QUERIES {
            let url = build_query(query);
            let bytes = self.http.get(&url)?;
            let text = String::from_utf8(bytes)
                .map_err(|error| format!("GET {url}: response was not UTF-8: {error}"))?;
            let page = parse_response(&text).map_err(|error| format!("GET {url}: {error}"))?;
            pages.push(page);
        }
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        // Round-robin a few from each query so one term cannot fill the row.
        for round in 0..3 {
            for page in &pages {
                if let Some(item) = page.get(round) {
                    if items.len() >= PER_CATEGORY_LIMIT {
                        break;
                    }
                    if seen.insert(item.pageid) {
                        items.push(item.clone());
                    }
                }
            }
        }
        for page in &pages {
            for item in page {
                if items.len() >= PER_CATEGORY_LIMIT {
                    break;
                }
                if seen.insert(item.pageid) {
                    items.push(item.clone());
                }
            }
        }
        items.truncate(PER_CATEGORY_LIMIT);
        Ok(items)
    }

    fn download_url(&self, url: &str) -> Result<Vec<u8>, String> {
        self.http.get(url)
    }
}

#[derive(Debug, Deserialize)]
struct Response {
    #[serde(default)]
    collection: Option<Collection>,
}

#[derive(Debug, Deserialize)]
struct Collection {
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    #[serde(default)]
    data: Vec<Data>,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Debug, Deserialize)]
struct Data {
    #[serde(default)]
    nasa_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    center: String,
    #[serde(default)]
    secondary_creator: String,
    #[serde(default)]
    media_type: String,
}

#[derive(Debug, Deserialize)]
struct Link {
    #[serde(default)]
    href: String,
    #[serde(default)]
    render: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/nasa-space.json");

    #[test]
    fn the_query_has_the_validated_shape() {
        let url = build_query("star cluster");
        for needle in [
            "q=star%20cluster",
            "media_type=image",
            "page_size=20",
            "page=1",
        ] {
            assert!(url.contains(needle), "query lacks {needle:?}: {url}");
        }
        assert!(url.starts_with(NASA_API));
    }

    #[test]
    fn the_fixture_parses_and_filters() {
        let items = parse_response(FIXTURE).expect("fixture");
        assert_eq!(
            items.len(),
            2,
            "portrait, logo, and third-party records are filtered"
        );

        let first = &items[0];
        assert_eq!(first.title, "Weighing in on the Dumbbell Nebula");
        assert_eq!(first.artist, "NASA/JPL-Caltech/Harvard-Smithsonian CfA");
        assert_eq!(first.license_short_name, "Public domain (NASA)");
        assert_eq!(first.license_url, NASA_MEDIA_GUIDELINES);
        assert!(first.page_url.ends_with("/PIA14417"));
        assert_eq!(first.category, Category::Space);
        assert_eq!(first.provider, "nasa");
        assert!(first.defer_full);
        assert_eq!(first.width, 2800);
        assert_eq!(first.height, 1400);
        assert!(first.full_url.ends_with("~orig.jpg"));
        assert!(first.source_url.ends_with("~medium.jpg"));

        let second = &items[1];
        assert_eq!(second.title, "Ring Nebula");
        assert_eq!(second.artist, "GSFC / NASA");
        assert_eq!(second.width, 3840);
        assert_eq!(second.height, 2160);
    }

    #[test]
    fn the_pageid_is_stable_positive_and_distinct() {
        let a = nasa_pageid("PIA14417");
        assert_eq!(a, nasa_pageid("PIA14417"));
        assert!(a <= 0x7fff_ffff);
        assert_ne!(a, nasa_pageid("GSFC_20171208_Archive_e001465"));
    }

    #[test]
    fn malformed_json_is_an_error_not_a_panic() {
        assert!(parse_response("not json").is_err());
    }
}
