// SPDX-License-Identifier: MIT
//! The content-source seam: the live Wikimedia client and the CI mock.
//!
//! [`ContentSource`] is the provider interface described in the design notes:
//! a second content source can be added later without changing the D-Bus
//! surface. [`WikipediaSource`] implements it over an [`HttpClient`];
//! [`MockSource`] serves canned catalogues and bytes so no test touches the
//! network.
//!
//! Downloads are sequential and per-request timeouts come from the HTTP agent,
//! so the concurrency and time caps in the design notes hold by construction.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::model::{Category, SourceKind, WallpaperItem};
use crate::nasa::NasaSource;
pub use crate::wikipedia::ParsedItem;
use crate::wikipedia::{build_query, parse_response, PER_CATEGORY_LIMIT, USER_AGENT};

/// The one thing that reaches over the network: a raw GET returning the body
/// bytes. The live implementation is [`UreqHttp`]; tests use a mock or the
/// [`MockSource`] above it.
pub trait HttpClient: Send + Sync {
    /// Fetch `url`, returning the response body on a 2xx status.
    fn get(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// The live HTTP client: one connection pool, one global timeout, one
/// descriptive `User-Agent`.
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    /// A client with the provider's timeout and user agent.
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(Duration::from_secs(20)))
            .build();
        UreqHttp {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        UreqHttp::new()
    }
}

impl std::fmt::Debug for UreqHttp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UreqHttp").finish_non_exhaustive()
    }
}

impl HttpClient for UreqHttp {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| format!("GET {url}: {error}"))?;
        response
            .body_mut()
            .read_to_vec()
            .map_err(|error| format!("GET {url} body: {error}"))
    }
}

/// The provider interface: one catalogue read per category, one URL download
/// primitive, and the grid/full-resolution convenience wrappers.
///
/// Implemented by [`WikipediaSource`] and [`NasaSource`] (live),
/// [`LiveSource`] (routing), and [`MockSource`] (tests).
pub trait ContentSource: Send + Sync {
    /// Fetch one category's catalogue, following `continue`, filtered, deduped,
    /// and capped at [`PER_CATEGORY_LIMIT`].
    fn catalogue(&self, category: Category) -> Result<Vec<ParsedItem>, String>;

    /// Download one URL.
    fn download_url(&self, url: &str) -> Result<Vec<u8>, String>;

    /// Download the grid-thumbnail bytes for a parsed item.
    fn download(&self, item: &ParsedItem) -> Result<Vec<u8>, String> {
        self.download_url(&item.source_url)
    }

    /// Download the full-resolution bytes for a catalogue item. Wikimedia
    /// items are already local, so this is only used for deferred sources.
    fn download_full(&self, item: &WallpaperItem) -> Result<Vec<u8>, String> {
        let url = if item.full_url.is_empty() {
            &item.source_url
        } else {
            &item.full_url
        };
        self.download_url(url)
    }
}

/// Wikimedia Commons over an [`HttpClient`].
pub struct WikipediaSource<H> {
    http: H,
}

impl<H> WikipediaSource<H> {
    /// A source over `http`.
    pub fn new(http: H) -> Self {
        WikipediaSource { http }
    }

    /// The transport, for tests.
    pub fn http(&self) -> &H {
        &self.http
    }
}

impl<H: HttpClient> ContentSource for WikipediaSource<H> {
    fn catalogue(&self, category: Category) -> Result<Vec<ParsedItem>, String> {
        if category.source() != SourceKind::Wikimedia {
            return Err(format!("{} is not a Wikimedia category", category.slug()));
        }
        let mut items = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let url = build_query(category, continuation.as_deref());
            let bytes = self.http.get(&url)?;
            let text = String::from_utf8(bytes)
                .map_err(|error| format!("GET {url}: response was not UTF-8: {error}"))?;
            let (page, next) =
                parse_response(&text, category).map_err(|error| format!("GET {url}: {error}"))?;
            items.extend(page);
            if items.len() >= PER_CATEGORY_LIMIT {
                break;
            }
            match next {
                Some(token) => continuation = Some(token),
                None => break,
            }
        }
        items.truncate(PER_CATEGORY_LIMIT);
        Ok(items)
    }

    fn download_url(&self, url: &str) -> Result<Vec<u8>, String> {
        self.http.get(url)
    }
}

/// The live source: Wikimedia for the Commons categories and NASA for Space.
///
/// The catalogue route is chosen by [`Category::source`]. Asset downloads are
/// routed by host, because [`ContentSource::download_url`] receives only the
/// URL; NASA assets live on `images-assets.nasa.gov`.
pub struct LiveSource {
    wikipedia: WikipediaSource<UreqHttp>,
    nasa: NasaSource<UreqHttp>,
}

impl LiveSource {
    /// A live source with the provider's HTTP client settings.
    pub fn new() -> Self {
        LiveSource {
            wikipedia: WikipediaSource::new(UreqHttp::new()),
            nasa: NasaSource::new(UreqHttp::new()),
        }
    }
}

impl Default for LiveSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentSource for LiveSource {
    fn catalogue(&self, category: Category) -> Result<Vec<ParsedItem>, String> {
        match category.source() {
            SourceKind::Wikimedia => self.wikipedia.catalogue(category),
            SourceKind::Nasa => self.nasa.catalogue(category),
        }
    }

    fn download_url(&self, url: &str) -> Result<Vec<u8>, String> {
        if url.contains("images-assets.nasa.gov") {
            self.nasa.download_url(url)
        } else {
            self.wikipedia.download_url(url)
        }
    }
}

/// A canned source for tests and CI: no network, fully deterministic.
#[derive(Default)]
pub struct MockSource {
    inner: Mutex<MockInner>,
}

#[derive(Default)]
struct MockInner {
    catalogues: HashMap<Category, Vec<ParsedItem>>,
    downloads: HashMap<String, Vec<u8>>,
    offline: bool,
    catalogue_calls: Vec<Category>,
    download_calls: Vec<String>,
}

impl MockSource {
    /// An online mock with no content.
    pub fn online() -> Self {
        MockSource::default()
    }

    /// An offline mock: every catalogue read fails, simulating no network.
    pub fn offline() -> Self {
        let mock = MockSource::default();
        mock.inner.lock().unwrap().offline = true;
        mock
    }

    /// Turn the network off after construction.
    pub fn set_offline(&self, offline: bool) {
        self.inner.lock().unwrap().offline = offline;
    }

    /// Serve `items` for `category`.
    pub fn with_items(&self, category: Category, items: Vec<ParsedItem>) -> &Self {
        self.inner
            .lock()
            .unwrap()
            .catalogues
            .insert(category, items);
        self
    }

    /// Serve `bytes` for a download URL.
    pub fn with_download(&self, url: &str, bytes: &[u8]) -> &Self {
        self.inner
            .lock()
            .unwrap()
            .downloads
            .insert(url.to_owned(), bytes.to_vec());
        self
    }

    /// The categories whose catalogue was read, in call order.
    pub fn catalogue_calls(&self) -> Vec<Category> {
        self.inner.lock().unwrap().catalogue_calls.clone()
    }

    /// The download URLs requested, in call order.
    pub fn download_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().download_calls.clone()
    }

    /// A minimal parsed item for fixtures.
    pub fn parsed_item(category: Category, pageid: u64) -> ParsedItem {
        ParsedItem {
            pageid,
            title: format!("File:{pageid}.jpg"),
            artist: "Test Artist".to_owned(),
            license_short_name: "CC BY-SA 4.0".to_owned(),
            license_url: "https://creativecommons.org/licenses/by-sa/4.0".to_owned(),
            page_url: format!("https://commons.wikimedia.org/wiki/File:{pageid}.jpg"),
            description: "A test picture".to_owned(),
            category,
            width: 6000,
            height: 4000,
            mime: "image/jpeg".to_owned(),
            source_url: format!("https://upload.wikimedia.org/{pageid}.jpg"),
            full_url: format!("https://upload.wikimedia.org/{pageid}.jpg"),
            provider: category.source().as_str().to_owned(),
            defer_full: false,
        }
    }

    /// A deferred item: the catalogue downloads `thumb_url`, the full image is
    /// fetched on demand from `full_url`.
    pub fn parsed_deferred_item(
        category: Category,
        pageid: u64,
        thumb_url: &str,
        full_url: &str,
    ) -> ParsedItem {
        let mut item = Self::parsed_item(category, pageid);
        item.source_url = thumb_url.to_owned();
        item.full_url = full_url.to_owned();
        item.defer_full = true;
        item
    }
}

impl ContentSource for MockSource {
    fn catalogue(&self, category: Category) -> Result<Vec<ParsedItem>, String> {
        let mut inner = self.inner.lock().unwrap();
        inner.catalogue_calls.push(category);
        if inner.offline {
            return Err(format!("{category:?}: network unavailable"));
        }
        Ok(inner.catalogues.get(&category).cloned().unwrap_or_default())
    }

    fn download_url(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut inner = self.inner.lock().unwrap();
        inner.download_calls.push(url.to_owned());
        if inner.offline {
            return Err(format!("{url}: network unavailable"));
        }
        inner
            .downloads
            .get(url)
            .cloned()
            .ok_or_else(|| format!("{url}: no canned bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mock_online_serves_items_and_downloads() {
        let source = MockSource::online();
        source.with_items(
            Category::Nature,
            vec![MockSource::parsed_item(Category::Nature, 1)],
        );
        source.with_download("https://upload.wikimedia.org/1.jpg", b"bytes");
        let item = MockSource::parsed_item(Category::Nature, 1);
        assert_eq!(
            source.catalogue(Category::Nature).unwrap(),
            vec![item.clone()]
        );
        assert_eq!(source.download(&item).unwrap(), b"bytes");
        assert_eq!(source.catalogue_calls(), vec![Category::Nature]);
        assert_eq!(
            source.download_calls(),
            vec!["https://upload.wikimedia.org/1.jpg"]
        );
    }

    #[test]
    fn an_offline_mock_fails_the_catalogue_read() {
        let source = MockSource::offline();
        assert!(source.catalogue(Category::Nature).is_err());
        assert!(source
            .download(&MockSource::parsed_item(Category::Nature, 1))
            .is_err());
    }

    #[test]
    fn a_deferred_item_downloads_the_thumb_and_full_urls_separately() {
        let source = MockSource::online();
        let item = MockSource::parsed_deferred_item(
            Category::Space,
            7,
            "https://images-assets.nasa.gov/7-thumb.jpg",
            "https://images-assets.nasa.gov/7-orig.jpg",
        );
        source.with_download("https://images-assets.nasa.gov/7-thumb.jpg", b"thumb");
        source.with_download("https://images-assets.nasa.gov/7-orig.jpg", b"full");
        assert_eq!(source.download(&item).unwrap(), b"thumb");

        let catalogue_item = WallpaperItem {
            pageid: 7,
            provider: "nasa".to_owned(),
            title: item.title.clone(),
            artist: item.artist.clone(),
            license_short_name: item.license_short_name.clone(),
            license_url: item.license_url.clone(),
            page_url: item.page_url.clone(),
            description: item.description.clone(),
            category: Category::Space,
            width: 3840,
            height: 2160,
            mime: "image/jpeg".to_owned(),
            source_url: item.source_url.clone(),
            full_url: item.full_url.clone(),
            thumb_path: None,
            local_path: None,
            fetched_at: 0,
        };
        assert_eq!(source.download_full(&catalogue_item).unwrap(), b"full");
        assert_eq!(
            source.download_calls(),
            vec![
                "https://images-assets.nasa.gov/7-thumb.jpg",
                "https://images-assets.nasa.gov/7-orig.jpg"
            ]
        );
    }

    #[test]
    fn the_live_client_is_free_to_construct() {
        // Constructing the agent must not open a connection.
        let _http = UreqHttp::new();
        let _live = LiveSource::new();
    }
}
