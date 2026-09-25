// SPDX-License-Identifier: MIT
//! T-10.1b acceptance: sorting is stable and incremental over a streamed
//! listing, and the sanctioned fallback is exercised through the same seam.
//! Headless — no display, no GIO, no daemon.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use dragonfruit_files_core::{
    DirectoryModel, DirectorySource, ListingHandle, Location, NodeKind, SortDirection, SortKey,
    SortSpec, SourceError, StdFsSource, SANCTIONED_FALLBACK_MARKER,
};

fn drain_to_completion(model: &mut DirectoryModel, handle: &ListingHandle) {
    let timeout = Duration::from_secs(30);
    while !model.is_complete() {
        match handle.recv_timeout(timeout) {
            Some(event) => {
                model.apply(event);
            }
            None => panic!("listing stalled in state {:?}", model.state()),
        }
    }
}

fn ordered_names(model: &DirectoryModel) -> Vec<String> {
    model
        .ordered()
        .map(|node| node.display_name().into_owned())
        .collect()
}

fn first_batch_names(model: &mut DirectoryModel, handle: &ListingHandle) -> Vec<String> {
    let first = handle
        .recv_timeout(Duration::from_secs(30))
        .expect("first batch");
    model.apply(first);
    ordered_names(model)
}

#[test]
fn a_real_tree_sorts_naturally_with_folders_first() {
    let dir = tempfile::tempdir().expect("temp dir");
    for name in ["file1", "file10", "file2", "report.txt"] {
        fs::write(dir.path().join(name), b"x").expect("write");
    }
    fs::create_dir(dir.path().join("subdir")).expect("mkdir");

    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::new();
    let handle = model.begin(source, Location::file(dir.path()));
    drain_to_completion(&mut model, &handle);

    // Default spec: name, ascending, folders first. Numeric order puts
    // file2 before file10; the directory leads.
    assert_eq!(
        ordered_names(&model),
        vec!["subdir", "file1", "file2", "file10", "report.txt"]
    );
    model.check_consistent().expect("consistent");
}

#[test]
fn the_sorted_projection_is_valid_after_every_batch() {
    const FILES: usize = 600;
    const BATCH: usize = 32;

    let dir = tempfile::tempdir().expect("temp dir");
    for index in 0..FILES {
        fs::write(dir.path().join(format!("file-{index:05}.txt")), b"x").expect("write");
    }

    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::with_batch_size(BATCH);
    let handle = model.begin(source, Location::file(dir.path()));

    let mut batches = 0;
    while !model.is_complete() {
        let Some(event) = handle.recv_timeout(Duration::from_secs(30)) else {
            panic!("listing stalled in state {:?}", model.state());
        };
        model.apply(event);
        batches += 1;
        // After every batch the projection is a sorted prefix and the
        // invariant (permutation + stable order) holds.
        let names = ordered_names(&model);
        assert!(
            names.windows(2).all(|pair| pair[0] <= pair[1]),
            "batch {batches} left the projection unsorted"
        );
        model.check_consistent().expect("consistent mid-stream");
    }

    assert!(batches > 1, "expected an incremental listing");
    assert_eq!(model.len(), FILES);
    assert_eq!(model.ordered().count(), FILES);
    assert_eq!(ordered_names(&model)[0], "file-00000.txt");
}

#[test]
fn changing_the_sort_keeps_node_ids_and_resorts_everything_loaded() {
    let dir = tempfile::tempdir().expect("temp dir");
    for name in ["a", "b", "c", "d"] {
        fs::write(dir.path().join(name), b"x").expect("write");
    }

    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::new();
    let handle = model.begin(source, Location::file(dir.path()));
    let prefix = first_batch_names(&mut model, &handle);
    let ids_before: Vec<_> = model.ordered().map(|node| node.id()).collect();

    let descending = SortSpec::new(SortKey::Name)
        .with_direction(SortDirection::Descending)
        .with_folders_first(false);
    model.set_sort(descending);
    assert_eq!(ordered_names(&model), vec!["d", "c", "b", "a"]);
    let mut ids_after: Vec<_> = model.ordered().map(|node| node.id()).collect();
    let mut ids_before = ids_before;
    ids_before.sort();
    ids_after.sort();
    assert_eq!(ids_before, ids_after);
    // The first batch was already a sorted prefix under the default spec.
    assert_eq!(prefix, vec!["a", "b", "c", "d"]);
    model.check_consistent().expect("consistent");
}

#[test]
fn folders_first_can_be_toggled_and_descending_still_leads_with_folders() {
    // A directory that sorts after a file by name: folders-first must still
    // lead with it.
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("aaa"), b"x").expect("write");
    fs::create_dir(dir.path().join("zzz")).expect("mkdir");

    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::new();
    let handle = model.begin(source, Location::file(dir.path()));
    drain_to_completion(&mut model, &handle);
    assert_eq!(ordered_names(&model), vec!["zzz", "aaa"]);

    let mixed = SortSpec::new(SortKey::Name).with_folders_first(false);
    model.set_sort(mixed);
    assert_eq!(ordered_names(&model), vec!["aaa", "zzz"]);

    // Descending does not flip the folders-first grouping: the directory
    // leads regardless of direction.
    let dir2 = tempfile::tempdir().expect("temp dir");
    fs::write(dir2.path().join("zzz"), b"x").expect("write");
    fs::create_dir(dir2.path().join("aaa")).expect("mkdir");
    let mut model2 = DirectoryModel::new();
    let handle2 = model2.begin(Arc::new(StdFsSource::new()), Location::file(dir2.path()));
    drain_to_completion(&mut model2, &handle2);
    let descending = SortSpec::new(SortKey::Name).with_direction(SortDirection::Descending);
    model2.set_sort(descending);
    assert_eq!(ordered_names(&model2), vec!["aaa", "zzz"]);
}

#[test]
fn the_fallback_is_exercised_for_kinds_and_refuses_foreign_schemes() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("note.txt"), b"hello").expect("write");
    fs::create_dir(dir.path().join("subdir")).expect("mkdir");

    let source = StdFsSource::new();
    let mut model = DirectoryModel::new();
    let handle = model.begin(Arc::new(source), Location::file(dir.path()));
    drain_to_completion(&mut model, &handle);
    assert!(model.is_complete());

    let types: Vec<NodeKind> = model.ordered().map(|node| node.kind()).collect();
    assert!(types.contains(&NodeKind::Directory));
    assert!(types.contains(&NodeKind::File));

    // The fallback resolves only `file://`; anything else is explicit.
    let trash = Location::parse("trash:///").expect("parses");
    let error = StdFsSource::new()
        .open(&trash)
        .err()
        .expect("unsupported scheme");
    assert_eq!(error, SourceError::UnsupportedScheme("trash".to_owned()));

    // The marker names the replacement, so a degraded build is visible.
    assert!(SANCTIONED_FALLBACK_MARKER.contains("GIO/GVfs"));
    assert!(SANCTIONED_FALLBACK_MARKER.contains("replace"));
}
