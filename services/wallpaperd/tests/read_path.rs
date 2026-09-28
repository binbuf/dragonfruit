// SPDX-License-Identifier: MIT
//! T-18.1a acceptance (headless): catalogue fetch/cache layout, Featured-default
//! selection, the offline path, bundled-default resolution/precedence, and
//! lazy-vs-`Preload` behavior. No bus and no network are involved.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dragonfruit_wallpaperd::cache::{CacheLayout, WEEK_SECS};
use dragonfruit_wallpaperd::defaults::{install_default_from, DefaultResolver, SHARE_SUBDIR};
use dragonfruit_wallpaperd::model::{Category, Status};
use dragonfruit_wallpaperd::provider::{Provider, ServiceState};
use dragonfruit_wallpaperd::source::MockSource;

/// A provider with nothing cached and a shipped default path.
fn cold_provider(cache: &CacheLayout, builtin: Option<PathBuf>) -> Provider {
    Provider::load(CacheLayout::clone(cache), builtin)
}

/// Serve `pageid`s per category with download bytes for each.
fn seed(source: &MockSource, category: Category, pageids: &[u64]) {
    let items: Vec<_> = pageids
        .iter()
        .map(|&pageid| MockSource::parsed_item(category, pageid))
        .collect();
    for &pageid in pageids {
        source.with_download(
            &format!("https://upload.wikimedia.org/{pageid}.jpg"),
            b"image-bytes",
        );
    }
    source.with_items(category, items);
}

#[test]
fn preload_fetches_fills_the_cache_and_selects_the_featured_default() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let source = Arc::new(MockSource::online());
    seed(&source, Category::Nature, &[5, 6]);
    seed(&source, Category::Scenery, &[7]);
    let state = ServiceState::new(source.clone(), cold_provider(&cache, None));

    // Lazy: loading the provider touched neither the network nor the disk.
    assert!(source.catalogue_calls().is_empty());
    assert_eq!(state.provider().lock().unwrap().status(), Status::Idle);

    // Eager Preload performs exactly one refresh.
    let report = state.refresh_if_needed(1_000);
    assert!(report.fetched && report.items_changed);
    assert_eq!(report.status, Status::Ready);

    let provider = state.provider().lock().unwrap();
    assert_eq!(provider.items().len(), 3);
    // Deterministic Featured default: the first Nature entry (pageid 5).
    let default = provider.default_source().expect("a featured default");
    assert!(
        default.ends_with("nature/5.jpg"),
        "got {}",
        default.display()
    );
    // Categories were read in ADR 0055 order; only seeded ones returned items.
    assert_eq!(
        source.catalogue_calls(),
        Category::ALL.to_vec(),
        "every category is queried"
    );
    drop(provider);

    // Cache layout: `<category>/<pageid>.<ext>` plus the index.
    assert!(cache.root().join("nature/5.jpg").is_file());
    assert!(cache.root().join("nature/6.jpg").is_file());
    assert!(cache.root().join("scenery/7.jpg").is_file());
    let index = cache.load_index();
    assert_eq!(index.last_fetch, 1_000);
    assert_eq!(index.items.len(), 3);
    let first = index.items.iter().find(|item| item.pageid == 5).unwrap();
    assert_eq!(first.artist, "Test Artist");
    assert_eq!(first.license_short_name, "CC BY-SA 4.0");
    assert_eq!(first.category, Category::Nature);
    assert!(!first.page_url.is_empty() && !first.description.is_empty());
}

#[test]
fn an_idle_provider_serves_the_shipped_default_without_fetching() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let builtin = dir.path().join("Default.jpg");
    std::fs::write(&builtin, b"shipped").unwrap();

    let source = Arc::new(MockSource::offline());
    let state = ServiceState::new(source.clone(), cold_provider(&cache, Some(builtin.clone())));

    // Before any warm: the shipped default resolves, nothing was fetched.
    assert_eq!(
        state.provider().lock().unwrap().builtin_default_source(),
        Some(builtin.clone())
    );
    assert!(source.catalogue_calls().is_empty());
    assert!(source.download_calls().is_empty());

    // A warm with no network does not crash; it keeps the cache empty and goes
    // offline, and the shipped default still resolves.
    let report = state.refresh_if_needed(1_000);
    assert!(report.fetched);
    assert_eq!(report.status, Status::Offline);
    let provider = state.provider().lock().unwrap();
    assert!(provider.items().is_empty());
    assert_eq!(provider.builtin_default_source(), Some(builtin));
}

#[test]
fn a_fresh_cache_is_not_refetched() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let source = Arc::new(MockSource::online());
    seed(&source, Category::Nature, &[5]);
    let state = ServiceState::new(source.clone(), cold_provider(&cache, None));

    assert!(state.refresh_if_needed(1_000).fetched);
    let calls_after_first = source.catalogue_calls().len();
    assert_eq!(calls_after_first, Category::ALL.len());

    // Within the week: no second fetch.
    let again = state.refresh_if_needed(1_000 + 10);
    assert!(!again.fetched);
    assert_eq!(source.catalogue_calls().len(), calls_after_first);

    // Past the week: a re-check runs.
    let stale = state.refresh_if_needed(1_000 + WEEK_SECS);
    assert!(stale.fetched);
    assert_eq!(source.catalogue_calls().len(), calls_after_first * 2);
}

#[test]
fn a_cold_start_rebuilds_the_catalogue_from_the_cache_without_network() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let source = Arc::new(MockSource::online());
    seed(&source, Category::Nature, &[5]);
    let state = ServiceState::new(source.clone(), cold_provider(&cache, None));
    state.refresh_if_needed(1_000);

    // Restart: a new provider over the same cache, with the network absent.
    let offline = Arc::new(MockSource::offline());
    let restarted = Provider::load(cache, None);
    assert_eq!(restarted.status(), Status::Ready);
    assert_eq!(restarted.items().len(), 1);
    assert_eq!(restarted.last_fetch(), 1_000);
    assert!(
        offline.catalogue_calls().is_empty(),
        "cold start is offline"
    );
}

#[test]
fn an_offline_refresh_keeps_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let source = Arc::new(MockSource::online());
    seed(&source, Category::Nature, &[5]);
    let state = ServiceState::new(source.clone(), cold_provider(&cache, None));
    state.refresh_if_needed(1_000);

    // The network dies and the cache ages out; the refresh fails but keeps it.
    source.set_offline(true);
    let report = state.refresh_if_needed(1_000 + WEEK_SECS + 1);
    assert!(report.fetched);
    assert_eq!(report.status, Status::Offline);
    let provider = state.provider().lock().unwrap();
    assert_eq!(provider.items().len(), 1, "the cached item survives");
    assert_eq!(
        provider.last_fetch(),
        1_000,
        "a failed fetch is not a fetch"
    );
    assert!(provider.default_source().is_some());
}

#[test]
fn the_cache_never_contains_the_bundled_default() {
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let builtin = dir.path().join("Default.jpg");
    std::fs::write(&builtin, b"shipped").unwrap();

    let source = Arc::new(MockSource::online());
    seed(&source, Category::Nature, &[5]);
    let state = ServiceState::new(source, cold_provider(&cache, Some(builtin.clone())));
    state.refresh_if_needed(1_000);

    // The shipped default lives outside the cache and is read in place.
    let provider = state.provider().lock().unwrap();
    assert_eq!(provider.builtin_default_source(), Some(builtin.clone()));
    for item in provider.items() {
        let local = item.local_path.as_ref().unwrap();
        assert!(
            local.starts_with(cache.root()),
            "fetched items live in the cache"
        );
        assert_ne!(local, &builtin, "the shipped asset is never cached");
    }
    // And the cache root holds only fetched content: the categories + index.
    let mut entries: Vec<String> = std::fs::read_dir(cache.root())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, vec!["index.json", "nature"]);
}

#[test]
fn bundled_default_resolution_order_is_override_then_share_then_in_tree() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("asset.jpg");
    std::fs::write(&source, b"asset").unwrap();
    let prefix = dir.path().join("prefix");
    let installed = install_default_from(&source, &prefix).unwrap();
    assert_eq!(
        installed,
        prefix.join("share").join(SHARE_SUBDIR).join("Default.jpg")
    );

    let override_path = dir.path().join("override.jpg");
    std::fs::write(&override_path, b"override").unwrap();
    let data_dir = prefix.join("share");

    // Override wins.
    let resolver = DefaultResolver {
        override_path: Some(override_path.clone()),
        data_dirs: vec![data_dir.clone()],
        in_tree: source.clone(),
    };
    assert_eq!(resolver.resolve(), Some(override_path));

    // No override: the installed share path wins.
    let resolver = DefaultResolver {
        override_path: None,
        data_dirs: vec![data_dir],
        in_tree: source.clone(),
    };
    assert_eq!(resolver.resolve(), Some(installed));

    // No share path: the in-tree copy is the last resort.
    let resolver = DefaultResolver {
        override_path: None,
        data_dirs: vec![dir.path().join("missing")],
        in_tree: source.clone(),
    };
    assert_eq!(resolver.resolve(), Some(source));

    // Nothing: a clean `None`, never an error.
    let resolver = DefaultResolver {
        override_path: None,
        data_dirs: vec![dir.path().join("missing")],
        in_tree: dir.path().join("missing.jpg"),
    };
    assert_eq!(resolver.resolve(), None);
}

#[test]
fn the_in_tree_shipped_asset_is_present() {
    let in_tree =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/graphics/wallpapers/Default.jpg");
    assert!(in_tree.is_file(), "missing {}", in_tree.display());
}
