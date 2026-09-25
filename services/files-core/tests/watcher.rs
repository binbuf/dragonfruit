// SPDX-License-Identifier: MIT
//! T-10.3b acceptance: an external create/delete updates the model within one
//! frame, folded in incrementally and never by re-listing. The scripted
//! `MockWatcher` fixture drives the seam headlessly; the `InotifyWatcher` cases
//! run the real fallback against a temp directory on Linux. No display, no GIO,
//! no daemon.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dragonfruit_files_core::{
    DirectoryModel, ListingHandle, Location, MockSource, MockWatcher, Node, NodeKind,
    OptimisticModel, SourceError, StdFsOps, WatchEvent, WatchEventKind,
};

fn file(name: &str) -> Node {
    Node::new(name, format!("file:///fixture/{name}"), NodeKind::File)
}

fn fixture(name: &str) -> Location {
    Location::file(format!("/fixture/{name}"))
}

fn drain_listing(model: &mut DirectoryModel, handle: &ListingHandle) {
    while !model.is_complete() {
        let Some(event) = handle.recv_timeout(Duration::from_secs(5)) else {
            break;
        };
        model.apply(event);
    }
}

fn listed(nodes: Vec<Node>) -> (DirectoryModel, Arc<MockSource>) {
    let source = Arc::new(MockSource::streaming(vec![nodes]));
    let mut model = DirectoryModel::new();
    let handle = model.begin(source.clone(), Location::file("/fixture"));
    drain_listing(&mut model, &handle);
    (model, source)
}

fn drive(model: &mut DirectoryModel, handle: &dragonfruit_files_core::WatchHandle, want: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut applied = 0;
    while applied < want && Instant::now() < deadline {
        applied += model.drain_watch(handle);
        if applied < want {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert_eq!(applied, want, "watcher events did not arrive");
}

#[test]
fn a_scripted_watcher_folds_create_and_delete_without_a_relist() {
    let (mut model, source) = listed(vec![file("keep"), file("remove")]);
    let keep = model.node_id_for_uri("file:///fixture/keep").expect("keep");
    let opens = source.opens();

    let watcher = Arc::new(MockWatcher::scripted(vec![vec![
        WatchEventKind::Created(file("added")),
        WatchEventKind::Removed {
            uri: "file:///fixture/remove".to_owned(),
        },
    ]]));
    let handle = model
        .begin_watch(watcher, &Location::file("/fixture"))
        .expect("watch");
    drive(&mut model, &handle, 2);

    // The external create appeared and the external delete vanished, within the
    // draining frame, with the untouched row's id preserved.
    assert!(model.node_id_for_uri("file:///fixture/added").is_some());
    assert!(model.node_id_for_uri("file:///fixture/remove").is_none());
    assert_eq!(model.node_id_for_uri("file:///fixture/keep"), Some(keep));
    assert_eq!(
        source.opens(),
        opens,
        "folding a watch event must not restart the listing"
    );
    model.check_consistent().expect("consistent");
}

#[test]
fn a_scripted_rename_keeps_the_node_id_and_selection() {
    let (mut model, _source) = listed(vec![file("old")]);
    let id = model.node_id_for_uri("file:///fixture/old").expect("old");
    model.check_consistent().expect("consistent");

    let watcher = Arc::new(MockWatcher::scripted(vec![vec![WatchEventKind::Renamed {
        from_uri: "file:///fixture/old".to_owned(),
        to: file("new"),
    }]]));
    let handle = model
        .begin_watch(watcher, &Location::file("/fixture"))
        .expect("watch");
    drive(&mut model, &handle, 1);

    assert_eq!(model.node_id_for_uri("file:///fixture/new"), Some(id));
    assert!(model.node_id_for_uri("file:///fixture/old").is_none());
    assert_eq!(
        model.node(id).map(|node| node.display_name().into_owned()),
        Some("new".to_owned())
    );
    model.check_consistent().expect("consistent");
}

#[test]
fn a_created_event_refreshes_a_row_instead_of_duplicating_it() {
    let (mut model, _source) = listed(vec![file("same")]);
    let id = model.node_id_for_uri("file:///fixture/same").expect("same");
    let refreshed = file("same").with_size(Some(99));

    let watcher = Arc::new(MockWatcher::scripted(vec![vec![WatchEventKind::Modified(
        refreshed,
    )]]));
    let handle = model
        .begin_watch(watcher, &Location::file("/fixture"))
        .expect("watch");
    drive(&mut model, &handle, 1);

    assert_eq!(model.len(), 1, "no duplicate row");
    assert_eq!(model.node_id_for_uri("file:///fixture/same"), Some(id));
    assert_eq!(model.node(id).and_then(Node::size), Some(99));
    model.check_consistent().expect("consistent");
}

#[test]
fn a_stale_watch_generation_is_ignored() {
    let (mut model, _source) = listed(vec![file("a")]);
    let event = WatchEvent::new(model.generation() + 1, WatchEventKind::Created(file("b")));
    assert!(!model.apply_watch(event));
    assert_eq!(model.len(), 1);
}

#[test]
fn a_watcher_event_confirms_a_pending_optimistic_create() {
    let mut model = OptimisticModel::new(DirectoryModel::new());
    let parent = Location::file("/fixture");
    let op = model.begin_new_folder(&parent);
    let node = model.model().nodes()[0].clone();
    assert!(model.is_pending(op));

    let generation = model.model().generation();
    assert!(model.apply_watch(WatchEvent::new(generation, WatchEventKind::Created(node))));
    assert!(!model.is_pending(op), "the create is reconciled");
    assert_eq!(model.len(), 1);
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_watcher_event_reverts_a_pending_create_that_vanished() {
    let mut model = OptimisticModel::new(DirectoryModel::new());
    let parent = Location::file("/fixture");
    let op = model.begin_new_folder(&parent);
    let uri = model.model().nodes()[0].uri().to_owned();

    let generation = model.model().generation();
    model.apply_watch(WatchEvent::new(generation, WatchEventKind::Removed { uri }));
    assert!(!model.is_pending(op));
    assert!(model.is_empty(), "the optimistic row snapped back");
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_watcher_event_confirms_a_pending_optimistic_delete() {
    let (base, _source) = listed(vec![file("gone")]);
    let id = base.node_id_for_uri("file:///fixture/gone").expect("gone");
    let mut model = OptimisticModel::new(base);
    model.selection_mut().select(id);
    let op = model.begin_delete(id).expect("pending delete");
    let generation = model.model().generation();

    model.apply_watch(WatchEvent::new(
        generation,
        WatchEventKind::Removed {
            uri: "file:///fixture/gone".to_owned(),
        },
    ));
    assert!(!model.is_pending(op));
    assert!(model.model().node(id).is_none());
    assert!(model.selection().is_empty());
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_watcher_event_reverts_a_pending_delete_that_came_back() {
    let (base, _source) = listed(vec![file("back")]);
    let id = base.node_id_for_uri("file:///fixture/back").expect("back");
    let mut model = OptimisticModel::new(base);
    model.selection_mut().select(id);
    let op = model.begin_delete(id).expect("pending delete");
    let generation = model.model().generation();

    model.apply_watch(WatchEvent::new(
        generation,
        WatchEventKind::Created(file("back")),
    ));
    assert!(!model.is_pending(op));
    assert!(model.model().node(id).is_some(), "the row returned");
    assert_eq!(model.selection().as_slice(), &[id]);
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_failing_watcher_open_reports_unsupported() {
    let mut model = DirectoryModel::new();
    let watcher = Arc::new(MockWatcher::failing(SourceError::UnsupportedScheme(
        "trash".to_owned(),
    )));
    let error = model
        .begin_watch(watcher, &fixture("x"))
        .err()
        .expect("unsupported");
    assert_eq!(error, SourceError::UnsupportedScheme("trash".to_owned()));
}

// The real fallback, over the kernel. Linux-only, because inotify is.
#[cfg(target_os = "linux")]
mod inotify {
    use super::*;
    use dragonfruit_files_core::{InotifyWatcher, StdFsSource};

    fn wait_for(
        model: &mut DirectoryModel,
        handle: &dragonfruit_files_core::WatchHandle,
        mut predicate: impl FnMut(&DirectoryModel) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            model.drain_watch(handle);
            if predicate(model) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the model never reflected the external change");
    }

    #[test]
    fn external_create_and_delete_update_the_model_without_a_relist() {
        let dir = tempfile::tempdir().expect("temp dir");
        let location = Location::file(dir.path());
        let source = Arc::new(StdFsSource::new());
        let mut model = DirectoryModel::new();
        let listing = model.begin(source, location.clone());
        drain_listing(&mut model, &listing);
        assert!(model.is_empty());

        let watcher = Arc::new(InotifyWatcher::new());
        let handle = model.begin_watch(watcher, &location).expect("watch");

        // An external create appears.
        let path = dir.path().join("external.txt");
        std::fs::write(&path, b"hello").expect("write");
        let uri = Location::file(&path).uri().to_owned();
        wait_for(&mut model, &handle, |model| {
            model.node_id_for_uri(&uri).is_some()
        });
        assert!(model.node_id_for_uri(&uri).is_some());
        model.check_consistent().expect("consistent");

        // An external delete vanishes, preserving no stale row.
        std::fs::remove_file(&path).expect("remove");
        wait_for(&mut model, &handle, |model| {
            model.node_id_for_uri(&uri).is_none()
        });
        assert!(model.node_id_for_uri(&uri).is_none());
        model.check_consistent().expect("consistent");
    }

    #[test]
    fn an_external_rename_keeps_the_node_id() {
        let dir = tempfile::tempdir().expect("temp dir");
        let before = dir.path().join("before.txt");
        std::fs::write(&before, b"x").expect("write");
        let location = Location::file(dir.path());
        let source = Arc::new(StdFsSource::new());
        let mut model = DirectoryModel::new();
        let listing = model.begin(source, location.clone());
        drain_listing(&mut model, &listing);

        let before_uri = Location::file(&before).uri().to_owned();
        let id = model.node_id_for_uri(&before_uri).expect("listed");

        let watcher = Arc::new(InotifyWatcher::new());
        let handle = model.begin_watch(watcher, &location).expect("watch");
        let after = dir.path().join("after.txt");
        std::fs::rename(&before, &after).expect("rename");
        let after_uri = Location::file(&after).uri().to_owned();
        wait_for(&mut model, &handle, |model| {
            model.node_id_for_uri(&after_uri).is_some()
        });
        assert_eq!(
            model.node_id_for_uri(&after_uri),
            Some(id),
            "an external rename keeps the stable id"
        );
        assert!(model.node_id_for_uri(&before_uri).is_none());
    }

    #[test]
    fn a_new_folder_via_stays_confirmed_against_a_live_watch() {
        // T-10.2b's synchronous `*_via` and T-10.3b's watcher must still agree:
        // the watcher sees the created folder and confirms nothing new, and the
        // model never duplicates the row.
        let dir = tempfile::tempdir().expect("temp dir");
        let location = Location::file(dir.path());
        let source = Arc::new(StdFsSource::new());
        let mut model = OptimisticModel::new(DirectoryModel::new());
        let listing = model.begin(source, location.clone());
        while !model.is_complete() {
            let Some(event) = listing.recv_timeout(Duration::from_secs(5)) else {
                break;
            };
            model.apply(event);
        }

        let watcher = Arc::new(InotifyWatcher::new());
        let handle = model.begin_watch(watcher, &location).expect("watch");
        let ops = StdFsOps::new();
        let op = model.new_folder_via(&ops, &location).expect("new folder");
        assert!(!model.is_pending(op));
        assert_eq!(model.len(), 1);

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            model.drain_watch(&handle);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(model.len(), 1, "the watcher must not duplicate the row");
        model.model().check_consistent().expect("consistent");
    }
}
