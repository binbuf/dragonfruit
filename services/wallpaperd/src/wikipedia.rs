// SPDX-License-Identifier: MIT
//! The Wikimedia Commons query shape and response parser (ADR 0055).
//!
//! One query per category, `generator=categorymembers` over namespace 6 with
//! `imageinfo` at 3840 px, formatversion 2, following `continue`. The response
//! is post-filtered (`ns==6`, `mime^=image/`), deduped by `pageid`, and capped
//! at ten per category. `utm_*` query parameters Wikimedia appends are
//! stripped from the cached URL, and HTML is stripped from the attribution.
//!
//! The parser is fixture-pinned: a live response shape change surfaces as a
//! failing test, never a runtime surprise.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::model::Category;

/// The Commons Action API endpoint.
pub const COMMONS_API: &str = "https://commons.wikimedia.org/w/api.php";
/// The descriptive `User-Agent` Wikimedia's policy asks for.
pub const USER_AGENT: &str =
    "Dragonfruit/0.1 (https://github.com/dragonfruit-desktop/dragonfruit) wallpaperd";
/// The requested thumbnail width.
pub const THUMB_WIDTH: u32 = 3840;
/// The most items kept per category.
pub const PER_CATEGORY_LIMIT: usize = 10;

/// Build the URL for one page of a category's catalogue.
///
/// `continuation` is the previous response's `gcmcontinue`, or `None` for the
/// first page. The parameter order is part of the validated shape and is
/// asserted by the tests.
pub fn build_query(category: Category, continuation: Option<&str>) -> String {
    let mut params = vec![
        ("action", "query".to_owned()),
        ("generator", "categorymembers".to_owned()),
        ("gcmtitle", category.commons_title().to_owned()),
        ("gcmnamespace", "6".to_owned()),
        ("gcmlimit", PER_CATEGORY_LIMIT.to_string()),
        ("gcmsort", "timestamp".to_owned()),
        ("gcmdir", "desc".to_owned()),
        ("prop", "imageinfo".to_owned()),
        ("iiprop", "url|size|mime|extmetadata".to_owned()),
        ("iiurlwidth", THUMB_WIDTH.to_string()),
        ("format", "json".to_owned()),
        ("formatversion", "2".to_owned()),
    ];
    if let Some(continuation) = continuation {
        params.push(("gcmcontinue", continuation.to_owned()));
    }
    let query: Vec<String> = params
        .into_iter()
        .map(|(key, value)| format!("{key}={}", encode_component(&value)))
        .collect();
    format!("{COMMONS_API}?{}", query.join("&"))
}

/// Percent-encode one query value (unreserved `A-Za-z0-9-_.~` untouched).
fn encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The normalized, utm-stripped form of a URL Wikimedia returns.
///
/// Wikimedia appends `utm_source`/`utm_campaign`/`utm_content` to `url` and
/// `thumburl`. Keeping them would make the cache key unstable and leak the
/// referrer, so every `utm_*` parameter is dropped; everything else (including
/// the fragment) is preserved.
pub fn strip_utm(url: &str) -> String {
    let (before_fragment, fragment) = match url.split_once('#') {
        Some((before, fragment)) => (before, Some(fragment)),
        None => (url, None),
    };
    let (base, query) = match before_fragment.split_once('?') {
        Some((base, query)) => (base, query),
        None => return url.to_owned(),
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| {
            let key = pair.split('=').next().unwrap_or_default();
            !key.starts_with("utm_")
        })
        .collect();
    let mut out = base.to_owned();
    if !kept.is_empty() {
        out.push('?');
        out.push_str(&kept.join("&"));
    }
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// Strip HTML tags and decode the handful of entities Commons emits, then
/// collapse whitespace. The result is display-ready attribution text.
pub fn strip_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One parsed, filtered catalogue item (before it has a local path).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedItem {
    /// The Commons `pageid`.
    pub pageid: u64,
    /// The file title.
    pub title: String,
    /// The HTML-stripped artist.
    pub artist: String,
    /// The license short name.
    pub license_short_name: String,
    /// The license URL.
    pub license_url: String,
    /// The file page URL.
    pub page_url: String,
    /// The HTML-stripped description.
    pub description: String,
    /// The category it was fetched under.
    pub category: Category,
    /// Original width.
    pub width: u32,
    /// Original height.
    pub height: u32,
    /// The MIME type.
    pub mime: String,
    /// The normalized thumbnail URL to download.
    pub source_url: String,
}

/// A parser failure (malformed JSON, or no usable item).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "commons response: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parse one formatversion=2 response page: the items and the next
/// `gcmcontinue`, when the API offered one.
pub fn parse_response(
    json: &str,
    category: Category,
) -> Result<(Vec<ParsedItem>, Option<String>), ParseError> {
    let response: Response =
        serde_json::from_str(json).map_err(|error| ParseError(error.to_string()))?;
    let continuation = response
        .r#continue
        .and_then(|value| value.get("gcmcontinue").map(|token| token.to_owned()));

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    if let Some(query) = response.query {
        for page in query.pages {
            if items.len() >= PER_CATEGORY_LIMIT {
                break;
            }
            if page.ns != 6 {
                continue;
            }
            if !seen.insert(page.pageid) {
                continue;
            }
            let Some(info) = page.imageinfo.into_iter().next() else {
                continue;
            };
            if !info.mime.starts_with("image/") {
                continue;
            }
            let source_url = info
                .thumburl
                .as_deref()
                .map(strip_utm)
                .unwrap_or_else(|| strip_utm(&info.url));
            if source_url.is_empty() {
                continue;
            }
            items.push(ParsedItem {
                pageid: page.pageid,
                title: page.title,
                artist: strip_html(metadata_value(&info.extmetadata, "Artist")),
                license_short_name: strip_html(metadata_value(
                    &info.extmetadata,
                    "LicenseShortName",
                )),
                license_url: metadata_value(&info.extmetadata, "LicenseUrl").to_owned(),
                page_url: info.descriptionurl,
                description: strip_html(metadata_value(&info.extmetadata, "ImageDescription")),
                category,
                width: info.width,
                height: info.height,
                mime: info.mime,
                source_url,
            });
        }
    }
    Ok((items, continuation))
}

/// Read one `extmetadata` value as a string (values are sometimes numbers).
fn metadata_value<'a>(metadata: &'a HashMap<String, MetaValue>, key: &str) -> &'a str {
    metadata
        .get(key)
        .and_then(|meta| meta.value.as_str())
        .unwrap_or_default()
}

#[derive(Debug, Deserialize)]
struct Response {
    #[serde(default)]
    query: Option<Query>,
    #[serde(default, rename = "continue")]
    r#continue: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct Query {
    #[serde(default)]
    pages: Vec<Page>,
}

#[derive(Debug, Deserialize)]
struct Page {
    pageid: u64,
    #[serde(default)]
    ns: i64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    imageinfo: Vec<ImageInfo>,
}

#[derive(Debug, Deserialize)]
struct ImageInfo {
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    mime: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    thumburl: Option<String>,
    #[serde(default)]
    descriptionurl: String,
    #[serde(default)]
    extmetadata: HashMap<String, MetaValue>,
}

#[derive(Debug, Deserialize)]
struct MetaValue {
    #[serde(default)]
    value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/commons-landscapes.json");

    #[test]
    fn the_query_has_the_validated_shape() {
        let url = build_query(Category::Scenery, None);
        for needle in [
            "action=query",
            "generator=categorymembers",
            "gcmtitle=Category%3AFeatured%20pictures%20of%20landscapes",
            "gcmnamespace=6",
            "gcmlimit=10",
            "gcmsort=timestamp",
            "gcmdir=desc",
            "prop=imageinfo",
            "iiprop=url%7Csize%7Cmime%7Cextmetadata",
            "iiurlwidth=3840",
            "format=json",
            "formatversion=2",
        ] {
            assert!(url.contains(needle), "query lacks {needle:?}: {url}");
        }
        assert!(!url.contains("gcmcontinue"));
        assert!(url.starts_with(COMMONS_API));
    }

    #[test]
    fn the_query_follows_continue() {
        let url = build_query(Category::Nature, Some("abc|def"));
        assert!(url.contains("gcmcontinue=abc%7Cdef"));
    }

    #[test]
    fn utm_parameters_are_stripped_and_everything_else_is_kept() {
        let url = "https://thumb.wikimedia.org/a/b.jpg/3840px-b.jpg?width=3840&utm_source=commons.wikimedia.org&utm_campaign=imageinfo&x=1#frag";
        assert_eq!(
            strip_utm(url),
            "https://thumb.wikimedia.org/a/b.jpg/3840px-b.jpg?width=3840&x=1#frag"
        );
        // No utm at all: untouched.
        assert_eq!(strip_utm("https://x/y.jpg?a=1"), "https://x/y.jpg?a=1");
        assert_eq!(strip_utm("https://x/y.jpg"), "https://x/y.jpg");
        // Only utm: the query is dropped entirely.
        assert_eq!(strip_utm("https://x/y.jpg?utm_source=a"), "https://x/y.jpg");
    }

    #[test]
    fn html_is_stripped_for_attribution_text() {
        assert_eq!(strip_html(r#"<a href="//x">Agnes</a>"#), "Agnes");
        assert_eq!(strip_html("<span>Own&nbsp;work</span>"), "Own work");
        assert_eq!(strip_html("a &amp; b"), "a & b");
        assert_eq!(strip_html("  multiple\n  spaces "), "multiple spaces");
    }

    #[test]
    fn the_fixture_parses_and_normalizes() {
        let (items, continuation) = parse_response(FIXTURE, Category::Scenery).expect("fixture");
        assert_eq!(items.len(), 2);
        assert!(
            continuation.is_some(),
            "the fixture carries a continue token"
        );
        let first = &items[0];
        assert_eq!(first.pageid, 145004660);
        assert_eq!(first.mime, "image/jpeg");
        assert_eq!(first.artist, "Agnes Monkelbaan");
        assert_eq!(first.license_short_name, "CC BY-SA 4.0");
        assert_eq!(
            first.license_url,
            "https://creativecommons.org/licenses/by-sa/4.0"
        );
        assert!(first.page_url.contains("commons.wikimedia.org/wiki/File:"));
        assert!(
            !first.source_url.contains("utm_"),
            "utm params are stripped: {}",
            first.source_url
        );
        assert!(first.source_url.contains("3840px-"));
        assert_eq!(first.width, 4116);
        assert_eq!(first.height, 1886);
    }

    #[test]
    fn non_file_namespaces_and_non_images_are_filtered() {
        let json = r#"{"query":{"pages":[
            {"pageid":1,"ns":14,"title":"Category:X","imageinfo":[{"mime":"image/jpeg","url":"https://x/1.jpg"}]},
            {"pageid":2,"ns":6,"title":"File:Doc.pdf","imageinfo":[{"mime":"application/pdf","url":"https://x/2.pdf"}]},
            {"pageid":3,"ns":6,"title":"File:Ok.jpg","imageinfo":[{"mime":"image/jpeg","url":"https://x/3.jpg","width":10,"height":20,"descriptionurl":"https://x/3"}]}
        ]}}"#;
        let (items, _) = parse_response(json, Category::Nature).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].pageid, 3);
    }

    #[test]
    fn duplicates_are_deduped_by_pageid() {
        let json = r#"{"query":{"pages":[
            {"pageid":7,"ns":6,"title":"File:A.jpg","imageinfo":[{"mime":"image/jpeg","url":"https://x/a.jpg"}]},
            {"pageid":7,"ns":6,"title":"File:A.jpg","imageinfo":[{"mime":"image/jpeg","url":"https://x/a.jpg"}]}
        ]}}"#;
        let (items, _) = parse_response(json, Category::Nature).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn a_response_is_capped_at_ten_per_category() {
        let pages: Vec<String> = (0..15)
            .map(|i| {
                format!(
                    r#"{{"pageid":{i},"ns":6,"title":"File:{i}.jpg","imageinfo":[{{"mime":"image/jpeg","url":"https://x/{i}.jpg"}}]}}"#
                )
            })
            .collect();
        let json = format!(r#"{{"query":{{"pages":[{}]}}}}"#, pages.join(","));
        let (items, _) = parse_response(&json, Category::Nature).unwrap();
        assert_eq!(items.len(), PER_CATEGORY_LIMIT);
    }

    #[test]
    fn malformed_json_is_an_error_not_a_panic() {
        assert!(parse_response("not json", Category::Nature).is_err());
    }
}
