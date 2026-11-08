// SPDX-License-Identifier: MIT
//! The provider state machine: lazy warm, eager `Preload`, and the offline
//! path.
//!
//! [`Provider`] is pure state. [`fetch`] does the network work with no lock
//! held; [`ServiceState`] sequences the two so a session launch warms in the
//! background while the desktop renders the shipped default and no D-Bus read
//! blocks on the network. Exactly one refresh is in flight at a time: once a
//! thread has called [`Provider::begin_fetch`], another caller's
//! [`Provider::begin_fetch`] returns false and it serves the current state.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cache::{is_stale, CacheLayout, Index};
use crate::model::{Catalogue, Category, Status, WallpaperItem};
use crate::source::ContentSource;
use crate::wikipedia::ParsedItem;

/// Why a warm decided to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmReason {
    /// The cache is empty (first run).
    Empty,
    /// The cache exists but the last fetch is at least a week old.
    Stale,
    /// An explicit user retry forced a refresh even though the cache is fresh.
    Forced,
}

/// What a warm should do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmDecision {
    /// Start a fetch for this reason.
    Fetch(WarmReason),
    /// The cache is populated and current; nothing to do.
    Fresh,
    /// A fetch is already in flight; do not start another.
    Busy,
}

impl WarmDecision {
    /// Whether this decision starts a fetch.
    pub const fn should_fetch(self) -> bool {
        matches!(self, WarmDecision::Fetch(_))
    }
}

/// The result of one catalogue refresh.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchResult {
    /// The downloaded items (only successfully cached images appear).
    pub items: Vec<WallpaperItem>,
    /// Unix seconds the refresh completed.
    pub fetched_at: u64,
    /// One message per failed category or download (diagnostics, offline).
    pub failed: Vec<String>,
}

impl FetchResult {
    /// Whether the refresh produced any usable content.
    pub fn has_items(&self) -> bool {
        !self.items.is_empty()
    }
}

/// Download every category's catalogue and cache the thumbnails.
///
/// This is the whole network phase and takes no lock; the caller owns the
/// sequencing. Failures are collected, not fatal: one dead category never
/// discards the rest, and an entirely dead network yields an empty result (the
/// caller marks the service offline and keeps the old cache).
pub fn fetch<C: ContentSource + ?Sized>(source: &C, cache: &CacheLayout, now: u64) -> FetchResult {
    let mut items = Vec::new();
    let mut failed = Vec::new();
    for category in Category::ALL {
        let parsed = match source.catalogue(category) {
            Ok(parsed) => parsed,
            Err(error) => {
                failed.push(format!("{}: {error}", category.slug()));
                continue;
            }
        };
        for parsed in parsed {
            match source.download(&parsed) {
                Ok(bytes) => {
                    if parsed.defer_full {
                        // Deferred sources (NASA) cache only the grid image;
                        // the full-resolution original is fetched on demand.
                        match cache.write_thumb(
                            category,
                            parsed.pageid,
                            &parsed.mime,
                            &parsed.source_url,
                            &bytes,
                        ) {
                            Ok(thumb_path) => {
                                items.push(item_from(parsed, None, Some(thumb_path), now))
                            }
                            Err(error) => failed.push(format!(
                                "{} page {}: thumb cache write: {error}",
                                category.slug(),
                                parsed.pageid
                            )),
                        }
                    } else {
                        match cache.write_image(
                            category,
                            parsed.pageid,
                            &parsed.mime,
                            &parsed.source_url,
                            &bytes,
                        ) {
                            Ok(path) => {
                                items.push(item_from(parsed, Some(path.clone()), Some(path), now))
                            }
                            Err(error) => failed.push(format!(
                                "{} page {}: cache write: {error}",
                                category.slug(),
                                parsed.pageid
                            )),
                        }
                    }
                }
                Err(error) => failed.push(format!(
                    "{} page {}: download: {error}",
                    category.slug(),
                    parsed.pageid
                )),
            }
        }
    }
    ensure_default_full(source, cache, &mut items, &mut failed);
    FetchResult {
        items,
        fetched_at: now,
        failed,
    }
}

/// The Featured default/fallback must always have a full-resolution local
/// file. Wikimedia already downloads it during the catalogue fetch; a future
/// deferred source that owns Nature would need one explicit full download.
fn ensure_default_full<C: ContentSource + ?Sized>(
    source: &C,
    cache: &CacheLayout,
    items: &mut [WallpaperItem],
    failed: &mut Vec<String>,
) {
    let Some(index) = items
        .iter()
        .position(|item| item.category == Category::Nature)
    else {
        return;
    };
    if items[index].local_path.is_some() {
        return;
    }
    let item = items[index].clone();
    match source.download_full(&item) {
        Ok(bytes) => match cache.write_image(
            item.category,
            item.pageid,
            &item.mime,
            &item.full_url,
            &bytes,
        ) {
            Ok(path) => items[index].local_path = Some(path),
            Err(error) => failed.push(format!(
                "{} page {}: default cache write: {error}",
                item.category.slug(),
                item.pageid
            )),
        },
        Err(error) => failed.push(format!(
            "{} page {}: default download: {error}",
            item.category.slug(),
            item.pageid
        )),
    }
}

/// Turn a parsed item into a cached catalogue item.
fn item_from(
    parsed: ParsedItem,
    local_path: Option<PathBuf>,
    thumb_path: Option<PathBuf>,
    fetched_at: u64,
) -> WallpaperItem {
    WallpaperItem {
        pageid: parsed.pageid,
        provider: parsed.provider,
        title: parsed.title,
        artist: parsed.artist,
        license_short_name: parsed.license_short_name,
        license_url: parsed.license_url,
        page_url: parsed.page_url,
        description: parsed.description,
        category: parsed.category,
        width: parsed.width,
        height: parsed.height,
        mime: parsed.mime,
        source_url: parsed.source_url,
        full_url: parsed.full_url,
        thumb_path,
        local_path,
        fetched_at,
    }
}

/// The provider's pure state: the catalogue, the status, and the shipped
/// default path.
#[derive(Debug, Clone)]
pub struct Provider {
    cache: CacheLayout,
    catalogue: Catalogue,
    status: Status,
    last_fetch: u64,
    builtin: Option<PathBuf>,
}

impl Provider {
    /// A provider over `cache`, resolved against the shipped `builtin` default.
    /// A cold start rebuilds the catalogue from `index.json` with no network.
    pub fn load(cache: CacheLayout, builtin: Option<PathBuf>) -> Self {
        let index = cache.load_index();
        let status = if index.items.is_empty() {
            Status::Idle
        } else {
            Status::Ready
        };
        Provider {
            cache,
            catalogue: Catalogue { items: index.items },
            status,
            last_fetch: index.last_fetch,
            builtin,
        }
    }

    /// A provider over an explicit index (tests and cache seeding).
    pub fn from_index(cache: CacheLayout, builtin: Option<PathBuf>, index: Index) -> Self {
        let status = if index.items.is_empty() {
            Status::Idle
        } else {
            Status::Ready
        };
        Provider {
            cache,
            catalogue: Catalogue { items: index.items },
            status,
            last_fetch: index.last_fetch,
            builtin,
        }
    }

    /// The cache layout the provider writes.
    pub fn cache(&self) -> &CacheLayout {
        &self.cache
    }

    /// The service status.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Unix seconds of the last successful refresh (0 = never).
    pub fn last_fetch(&self) -> u64 {
        self.last_fetch
    }

    /// The resolved shipped `Default.jpg` path (`BuiltinDefaultSource`).
    pub fn builtin_default_source(&self) -> Option<PathBuf> {
        self.builtin.clone()
    }

    /// The fetched Featured default/fallback (`DefaultSource`): the first
    /// Nature entry under timestamp-desc that downloaded, else `None`.
    pub fn default_source(&self) -> Option<PathBuf> {
        self.catalogue
            .featured_default()
            .and_then(|item| item.local_path.clone())
    }

    /// The cached items.
    pub fn items(&self) -> &[WallpaperItem] {
        self.catalogue.items()
    }

    /// The catalogue.
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    /// Decide whether a warm should fetch. Pure; no lock, no network.
    pub fn warm_decision(&self, now: u64) -> WarmDecision {
        if self.status == Status::Fetching {
            return WarmDecision::Busy;
        }
        if self.catalogue.is_empty() {
            return WarmDecision::Fetch(WarmReason::Empty);
        }
        if is_stale(self.last_fetch, now) {
            return WarmDecision::Fetch(WarmReason::Stale);
        }
        WarmDecision::Fresh
    }

    /// Claim the single in-flight refresh. Returns false when one is already
    /// in flight, so a second caller serves the current state instead.
    pub fn begin_fetch(&mut self) -> bool {
        if self.status == Status::Fetching {
            return false;
        }
        self.status = Status::Fetching;
        true
    }

    /// Install a completed fetch. Non-empty content becomes `Ready`; an empty
    /// result keeps the cache and becomes `Offline`. The index is persisted
    /// either way so `lastFetch` survives a restart.
    ///
    /// Full-resolution files downloaded on demand survive a weekly refresh:
    /// when a refreshed item still has the same id and the file is present,
    /// its `local_path` is carried over instead of forcing a re-download.
    pub fn apply_fetch(&mut self, mut result: FetchResult) {
        if result.has_items() {
            let old_full: std::collections::HashMap<u64, PathBuf> = self
                .catalogue
                .items
                .iter()
                .filter_map(|item| {
                    item.local_path
                        .as_ref()
                        .filter(|path| path.is_file())
                        .map(|path| (item.pageid, path.clone()))
                })
                .collect();
            for item in &mut result.items {
                if item.local_path.is_none() {
                    if let Some(path) = old_full.get(&item.pageid) {
                        item.local_path = Some(path.clone());
                    }
                }
            }
            self.catalogue = Catalogue {
                items: result.items,
            };
            self.last_fetch = result.fetched_at;
            self.status = Status::Ready;
        } else {
            self.status = Status::Offline;
        }
        self.save_index();
    }

    /// Replace one item's full-resolution path after an on-demand download.
    pub fn set_item_local_path(&mut self, pageid: u64, path: PathBuf) -> bool {
        if let Some(item) = self
            .catalogue
            .items
            .iter_mut()
            .find(|item| item.pageid == pageid)
        {
            item.local_path = Some(path);
            true
        } else {
            false
        }
    }

    /// Persist the current catalogue index.
    pub fn save_index(&self) {
        let _ = self.cache.save_index(&Index {
            last_fetch: self.last_fetch,
            items: self.catalogue.items.clone(),
        });
    }

    /// Mark the service offline without replacing the cache (a fetch that
    /// could not even start).
    pub fn mark_offline(&mut self) {
        self.status = Status::Offline;
    }
}

/// The result of a service-level refresh attempt, for signal emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshReport {
    /// Whether a fetch actually ran.
    pub fetched: bool,
    /// The status after the attempt.
    pub status: Status,
    /// Whether the item list changed (a first fill).
    pub items_changed: bool,
    /// Whether the status changed.
    pub status_changed: bool,
}

impl RefreshReport {
    /// No work was done; the status is unchanged.
    pub fn none(status: Status) -> Self {
        RefreshReport {
            fetched: false,
            status,
            items_changed: false,
            status_changed: false,
        }
    }
}

/// The service-level orchestrator: pure state behind one lock, network work
/// outside it.
pub struct ServiceState {
    provider: Arc<Mutex<Provider>>,
    source: Arc<dyn ContentSource>,
}

impl ServiceState {
    /// A state over `source` and `provider`.
    pub fn new(source: Arc<dyn ContentSource>, provider: Provider) -> Self {
        ServiceState {
            provider: Arc::new(Mutex::new(provider)),
            source,
        }
    }

    /// The shared provider, for the D-Bus surface and tests.
    pub fn provider(&self) -> &Arc<Mutex<Provider>> {
        &self.provider
    }

    /// The content source.
    pub fn source(&self) -> &Arc<dyn ContentSource> {
        &self.source
    }

    /// Refresh if the cache is empty or stale; otherwise leave the state
    /// alone. This is both the lazy launch warm and the eager `Preload`: the
    /// only difference is the caller and whether it waits.
    ///
    /// The lock is held only for the state transitions; the network runs with
    /// it released, so a concurrent `Status`/`Items` read never blocks on the
    /// fetch.
    pub fn refresh_if_needed(&self, now: u64) -> RefreshReport {
        self.refresh(false, now)
    }

    /// Refresh the catalogue. `force` is the explicit user retry: it fetches
    /// even when the cache is fresh, while `Preload` and the launch warm keep
    /// the empty/stale rule.
    pub fn refresh(&self, force: bool, now: u64) -> RefreshReport {
        let (decision, cache) = {
            let provider = lock(&self.provider);
            let decision = if force && provider.status() != Status::Fetching {
                WarmDecision::Fetch(WarmReason::Forced)
            } else {
                provider.warm_decision(now)
            };
            (decision, provider.cache().clone())
        };
        if !decision.should_fetch() {
            let status = lock(&self.provider).status();
            return RefreshReport::none(status);
        }
        {
            let mut provider = lock(&self.provider);
            if !provider.begin_fetch() {
                let status = provider.status();
                return RefreshReport::none(status);
            }
        }

        let result = fetch(self.source.as_ref(), &cache, now);

        let mut provider = lock(&self.provider);
        let had_items = !provider.items().is_empty();
        provider.apply_fetch(result);
        let has_items = !provider.items().is_empty();
        RefreshReport {
            fetched: true,
            status: provider.status(),
            items_changed: !had_items && has_items,
            status_changed: true,
        }
    }

    /// Download one deferred item's full-resolution image and persist the new
    /// local path. Returns the cached path.
    ///
    /// The network runs with the provider lock released. A missing item and a
    /// failed download are ordinary errors returned to the caller; the
    /// catalogue status stays `ready`.
    pub fn fetch_item(&self, pageid: u64) -> Result<PathBuf, String> {
        let (item, cache) = {
            let provider = lock(&self.provider);
            let item = provider
                .items()
                .iter()
                .find(|item| item.pageid == pageid)
                .cloned()
                .ok_or_else(|| format!("wallpaper {pageid} not found"))?;
            if let Some(path) = &item.local_path {
                if path.is_file() {
                    return Ok(path.clone());
                }
            }
            (item, provider.cache().clone())
        };
        let bytes = self.source.download_full(&item)?;
        let path = cache
            .write_image(
                item.category,
                item.pageid,
                &item.mime,
                &item.full_url,
                &bytes,
            )
            .map_err(|error| format!("cache write: {error}"))?;
        {
            let mut provider = lock(&self.provider);
            if !provider.set_item_local_path(pageid, path.clone()) {
                return Err(format!("wallpaper {pageid} disappeared during download"));
            }
            provider.save_index();
        }
        Ok(path)
    }

    /// The `FetchItem` method reply: a small JSON object the bridge can parse.
    pub fn fetch_item_json(&self, pageid: u64) -> String {
        match self.fetch_item(pageid) {
            Ok(path) => serde_json::json!({
                "ok": true,
                "pageid": pageid,
                "source": path_string(Some(path)),
            })
            .to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "pageid": pageid,
                "error": error,
            })
            .to_string(),
        }
    }

    /// The `Status` property spelling.
    pub fn status_str(&self) -> String {
        lock(&self.provider).status().as_str().to_owned()
    }

    /// The `LastFetch` property (Unix seconds; 0 = never).
    pub fn last_fetch(&self) -> u64 {
        lock(&self.provider).last_fetch()
    }

    /// The `DefaultSource` property: the fetched Featured default/fallback
    /// path, or the empty string.
    pub fn default_source(&self) -> String {
        path_string(lock(&self.provider).default_source())
    }

    /// The `BuiltinDefaultSource` property: the resolved shipped default path,
    /// or the empty string.
    pub fn builtin_default_source(&self) -> String {
        path_string(lock(&self.provider).builtin_default_source())
    }

    /// The `Items` property: the cached items as a JSON array.
    pub fn items_json(&self) -> String {
        serde_json::to_string(lock(&self.provider).items()).unwrap_or_else(|_| "[]".to_owned())
    }

    /// The whole catalogue as a flat JSON snapshot (the method reply shape).
    pub fn snapshot_json(&self) -> String {
        let provider = lock(&self.provider);
        serde_json::json!({
            "status": provider.status().as_str(),
            "lastFetch": provider.last_fetch(),
            "defaultSource": path_string(provider.default_source()),
            "builtinDefaultSource": path_string(provider.builtin_default_source()),
            "items": provider.items(),
        })
        .to_string()
    }
}

/// A path as a wire string, empty when absent.
fn path_string(path: Option<PathBuf>) -> String {
    path.map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Lock a provider, recovering from poisoning (a panic must not wedge the
/// service).
pub fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Unix seconds now.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MockSource;

    fn provider(cache: &CacheLayout) -> Provider {
        Provider::load(cache.clone(), Some(PathBuf::from("/usr/share/Default.jpg")))
    }

    #[test]
    fn a_cold_provider_is_idle_and_wants_an_empty_fetch() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let provider = provider(&cache);
        assert_eq!(provider.status(), Status::Idle);
        assert!(provider.items().is_empty());
        assert_eq!(
            provider.warm_decision(1000),
            WarmDecision::Fetch(WarmReason::Empty)
        );
        assert_eq!(
            provider.builtin_default_source(),
            Some(PathBuf::from("/usr/share/Default.jpg"))
        );
        assert_eq!(provider.default_source(), None);
    }

    #[test]
    fn a_stale_populated_provider_wants_a_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let index = Index {
            last_fetch: 1000,
            items: vec![item(1)],
        };
        let provider = Provider::from_index(cache, None, index);
        assert_eq!(provider.status(), Status::Ready);
        assert_eq!(provider.warm_decision(1000 + 10), WarmDecision::Fresh);
        assert_eq!(
            provider.warm_decision(1000 + crate::cache::WEEK_SECS),
            WarmDecision::Fetch(WarmReason::Stale)
        );
    }

    #[test]
    fn only_one_fetch_is_in_flight() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let mut provider = provider(&cache);
        assert!(provider.begin_fetch());
        assert_eq!(provider.status(), Status::Fetching);
        assert!(!provider.begin_fetch(), "a second claim is refused");
        assert_eq!(provider.warm_decision(2000), WarmDecision::Busy);
    }

    #[test]
    fn an_empty_fetch_result_keeps_the_cache_and_goes_offline() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let index = Index {
            last_fetch: 1000,
            items: vec![item(1)],
        };
        let mut provider = Provider::from_index(cache, None, index);
        provider.begin_fetch();
        provider.apply_fetch(FetchResult {
            items: Vec::new(),
            fetched_at: 2000,
            failed: vec!["nature: network unavailable".to_owned()],
        });
        assert_eq!(provider.status(), Status::Offline);
        assert_eq!(provider.items().len(), 1, "the cache is kept");
        assert_eq!(provider.last_fetch(), 1000, "a failed fetch is not a fetch");
    }

    #[test]
    fn a_successful_fetch_replaces_the_catalogue_and_persists_the_index() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let mut provider = provider(&cache);
        provider.begin_fetch();
        let path = cache
            .write_image(Category::Nature, 5, "image/jpeg", "https://u/5.jpg", b"x")
            .unwrap();
        provider.apply_fetch(FetchResult {
            items: vec![item(5).with_path(path)],
            fetched_at: 2000,
            failed: Vec::new(),
        });
        assert_eq!(provider.status(), Status::Ready);
        assert_eq!(provider.last_fetch(), 2000);
        assert_eq!(
            provider.default_source().unwrap().file_name().unwrap(),
            "5.jpg"
        );

        let reloaded = Provider::load(cache, None);
        assert_eq!(reloaded.status(), Status::Ready);
        assert_eq!(reloaded.last_fetch(), 2000);
        assert_eq!(reloaded.items().len(), 1);
    }

    #[test]
    fn the_service_state_warm_is_offline_safe() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let source = Arc::new(MockSource::offline());
        let state = ServiceState::new(source.clone(), provider(&cache));
        let report = state.refresh_if_needed(1000);
        assert!(report.fetched);
        assert_eq!(report.status, Status::Offline);
        assert!(state.provider().lock().unwrap().items().is_empty());
    }

    #[test]
    fn the_service_state_preload_fetches_and_serves_items() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let source = Arc::new(MockSource::online());
        source.with_items(
            Category::Nature,
            vec![MockSource::parsed_item(Category::Nature, 11)],
        );
        source.with_download("https://upload.wikimedia.org/11.jpg", b"pic");
        let state = ServiceState::new(source.clone(), provider(&cache));

        let report = state.refresh_if_needed(1000);
        assert!(report.fetched && report.items_changed);
        assert_eq!(report.status, Status::Ready);
        assert_eq!(state.provider().lock().unwrap().items().len(), 1);
        assert!(source.catalogue_calls().contains(&Category::Nature));

        // A second warm now sees a fresh cache and does nothing.
        let again = state.refresh_if_needed(1000 + 10);
        assert!(!again.fetched);
        assert_eq!(again.status, Status::Ready);
    }

    #[test]
    fn an_explicit_refresh_forces_a_fresh_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let source = Arc::new(MockSource::online());
        source.with_items(
            Category::Nature,
            vec![MockSource::parsed_item(Category::Nature, 11)],
        );
        source.with_download("https://upload.wikimedia.org/11.jpg", b"pic");
        let state = ServiceState::new(source.clone(), provider(&cache));

        assert!(state.refresh_if_needed(1000).fetched);
        let calls = source.catalogue_calls().len();
        assert!(!state.refresh_if_needed(1000 + 10).fetched);
        assert!(state.refresh(true, 1000 + 20).fetched);
        assert_eq!(source.catalogue_calls().len(), calls * 2);
    }

    #[test]
    fn a_deferred_item_fetches_its_full_image_on_demand() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let source = Arc::new(MockSource::online());
        let deferred = MockSource::parsed_deferred_item(
            Category::Space,
            21,
            "https://images-assets.nasa.gov/21-thumb.jpg",
            "https://images-assets.nasa.gov/21-orig.jpg",
        );
        source.with_items(Category::Space, vec![deferred]);
        source.with_download("https://images-assets.nasa.gov/21-thumb.jpg", b"thumb");
        source.with_download("https://images-assets.nasa.gov/21-orig.jpg", b"full");
        let state = ServiceState::new(source.clone(), provider(&cache));

        assert!(state.refresh_if_needed(1000).fetched);
        {
            let provider = state.provider().lock().unwrap();
            let item = provider
                .items()
                .iter()
                .find(|item| item.pageid == 21)
                .unwrap();
            assert!(item.thumb_path.is_some());
            assert!(item.local_path.is_none(), "full image is deferred");
        }

        let path = state.fetch_item(21).expect("full download");
        assert_eq!(std::fs::read(&path).unwrap(), b"full");
        {
            let provider = state.provider().lock().unwrap();
            let item = provider
                .items()
                .iter()
                .find(|item| item.pageid == 21)
                .unwrap();
            assert_eq!(item.local_path.as_deref(), Some(path.as_path()));
        }

        // A restart reads the persisted full path from the index.
        let reloaded = Provider::load(cache, None);
        let item = reloaded
            .items()
            .iter()
            .find(|item| item.pageid == 21)
            .unwrap();
        assert_eq!(item.local_path.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn a_refresh_keeps_an_on_demand_full_image_for_the_same_item() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheLayout::new(dir.path());
        let full = cache
            .write_image(
                Category::Space,
                21,
                "image/jpeg",
                "https://u/21.jpg",
                b"full",
            )
            .unwrap();
        let mut provider = Provider::from_index(
            cache,
            None,
            Index {
                last_fetch: 1000,
                items: vec![WallpaperItem {
                    category: Category::Space,
                    local_path: Some(full.clone()),
                    ..item(21)
                }],
            },
        );
        provider.apply_fetch(FetchResult {
            items: vec![WallpaperItem {
                category: Category::Space,
                ..item(21)
            }],
            fetched_at: 2000,
            failed: Vec::new(),
        });
        let item = provider
            .items()
            .iter()
            .find(|item| item.pageid == 21)
            .unwrap();
        assert_eq!(item.local_path.as_deref(), Some(full.as_path()));
    }

    fn item(pageid: u64) -> WallpaperItem {
        WallpaperItem {
            pageid,
            provider: "wikimedia".to_owned(),
            title: String::new(),
            artist: String::new(),
            license_short_name: String::new(),
            license_url: String::new(),
            page_url: String::new(),
            description: String::new(),
            category: Category::Nature,
            width: 1,
            height: 1,
            mime: "image/jpeg".to_owned(),
            source_url: "https://u/x.jpg".to_owned(),
            full_url: "https://u/x.jpg".to_owned(),
            thumb_path: None,
            local_path: None,
            fetched_at: 0,
        }
    }

    trait WithPath {
        fn with_path(self, path: PathBuf) -> Self;
    }

    impl WithPath for WallpaperItem {
        fn with_path(mut self, path: PathBuf) -> Self {
            self.local_path = Some(path);
            self
        }
    }
}
