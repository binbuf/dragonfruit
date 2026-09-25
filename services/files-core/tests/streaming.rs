// SPDX-License-Identifier: MIT
//! T-10.1a acceptance: a directory streams incrementally into the model in
//! arrival order, and the model stays consistent. Headless — no display, no
//! GTK, no daemon.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use dragonfruit_files_core::{
    DirectoryModel, ListingEvent, ListingEventKind, ListingHandle, Location, MockSource, Node,
    NodeKind, StdFsSource,
};

fn file(label: &str) -> Node {
    Node::new(label, format!("file:///fixture/{label}"), NodeKind::File)
}

/// Apply events until the model completes (or the timeout fires).
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

#[test]
fn a_large_directory_streams_incrementally() {
    const FILES: usize = 4_000;
    const BATCH: usize = 64;

    let dir = tempfile::tempdir().expect("temp dir");
    for index in 0..FILES {
        fs::write(dir.path().join(format!("file-{index:05}.txt")), b"x").expect("write");
    }

    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::with_batch_size(BATCH);
    let handle = model.begin(source, Location::file(dir.path()));

    // The first event must be a batch smaller than the whole directory, and
    // the model must already be renderable: that is the streaming contract.
    let first = handle
        .recv_timeout(Duration::from_secs(30))
        .expect("first batch");
    assert!(matches!(first.kind, ListingEventKind::Batch(_)));
    model.apply(first);
    assert!(model.is_streaming(), "state is {:?}", model.state());
    assert!(!model.is_empty());
    assert!(model.len() <= BATCH, "first batch was {}", model.len());
    assert!(model.len() < FILES);
    model.check_consistent().expect("consistent mid-stream");

    drain_to_completion(&mut model, &handle);

    assert!(model.is_complete());
    assert_eq!(model.len(), FILES);
    model.check_consistent().expect("consistent at completion");
    assert_eq!(
        model.location().and_then(Location::to_file_path),
        Some(dir.path().to_path_buf())
    );
}

#[test]
fn a_fixture_streams_in_arrival_order() {
    let source = Arc::new(MockSource::streaming(vec![
        vec![file("a"), file("b"), file("c")],
        vec![file("d"), file("e")],
    ]));
    let mut model = DirectoryModel::new();
    let handle = model.begin(source, Location::file("/fixture"));

    // Apply only the first batch: the model is a prefix, still streaming.
    let first = handle
        .recv_timeout(Duration::from_secs(5))
        .expect("batch 1");
    assert!(model.apply(first));
    let names: Vec<String> = model
        .nodes()
        .iter()
        .map(|node| node.display_name().into_owned())
        .collect();
    assert_eq!(names, vec!["a", "b", "c"]);
    assert!(model.is_streaming());
    model.check_consistent().expect("consistent mid-stream");

    drain_to_completion(&mut model, &handle);
    let names: Vec<String> = model
        .nodes()
        .iter()
        .map(|node| node.display_name().into_owned())
        .collect();
    assert_eq!(names, vec!["a", "b", "c", "d", "e"]);
    assert!(model.is_complete());
    model.check_consistent().expect("consistent at completion");
}

#[test]
fn the_fallback_lists_kinds_and_sizes_from_a_real_tree() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("note.txt"), b"hello").expect("write file");
    fs::create_dir(dir.path().join("subdir")).expect("mkdir");

    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::new();
    let handle = model.begin(source, Location::file(dir.path()));
    drain_to_completion(&mut model, &handle);

    let mut names: Vec<String> = model
        .nodes()
        .iter()
        .map(|node| node.display_name().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, vec!["note.txt", "subdir"]);

    let note = model
        .nodes()
        .iter()
        .find(|node| node.display_name() == "note.txt")
        .expect("note.txt listed");
    assert_eq!(note.kind(), NodeKind::File);
    assert_eq!(note.size(), Some(5));
    assert!(note.modified().is_some());

    let subdir = model
        .nodes()
        .iter()
        .find(|node| node.display_name() == "subdir")
        .expect("subdir listed");
    assert_eq!(subdir.kind(), NodeKind::Directory);
    assert!(subdir.is_dir());
}

#[test]
fn a_failed_open_reports_the_error_and_no_nodes() {
    use dragonfruit_files_core::SourceError;

    let source = Arc::new(MockSource::failing(SourceError::PermissionDenied(
        "/secret".to_owned(),
    )));
    let mut model = DirectoryModel::new();
    let handle = model.begin(source, Location::file("/secret"));
    let event = handle
        .recv_timeout(Duration::from_secs(5))
        .expect("error event");
    model.apply(event);

    assert!(matches!(
        model.error(),
        Some(SourceError::PermissionDenied(_))
    ));
    assert!(model.is_empty());
    model.check_consistent().expect("consistent");
}

#[cfg(unix)]
#[test]
fn raw_non_utf8_names_round_trip_through_the_listing() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().expect("temp dir");
    let raw = OsStr::from_bytes(b"odd\xffname");
    fs::write(dir.path().join(raw), b"x").expect("write odd name");

    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::new();
    let handle = model.begin(source, Location::file(dir.path()));
    drain_to_completion(&mut model, &handle);

    let listed = model
        .nodes()
        .iter()
        .find(|node| node.name() == raw)
        .expect("odd name listed with its bytes intact");
    assert!(listed.display_name().contains('\u{fffd}'));
    assert!(listed.uri().starts_with("file://"));
}

#[test]
fn applying_a_stale_generation_does_nothing() {
    let source = Arc::new(MockSource::streaming(vec![vec![file("a")]]));
    let mut model = DirectoryModel::new();
    let stale = model.begin(source.clone(), Location::file("/one"));
    let stale_generation = stale.generation();
    let _live = model.begin(source, Location::file("/two"));

    let stale_event = ListingEvent::batch(stale_generation, vec![file("nope")]);
    assert!(!model.apply(stale_event));
    assert!(model.is_empty());
}
