// SPDX-License-Identifier: MIT
//! T-10.3b acceptance: an external create/delete updates the model within one
//! frame, folded in incrementally and never by re-listing. The scripted
//! `MockWatcher` fixture drives the seam headlessly; the `InotifyWatcher` cases
//! run the real fallback against a temp directory on Linux. No display, no GIO,
//! no daemon.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dragonfruit_files_core::{
    DirectoryModel, InotifyWatcher, ListingHandle, Location, MockSource, MockWatcher, Node,
    NodeKind, OptimisticModel, SourceError, StdFsOps, StdFsSource, WatchEvent, WatchEventKind,
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

// A model with no listing yet (generation 0): the optimistic layer's own
// inserts are the only rows, and they carry the current generation.
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
    fn an_external_hidden_create_is_not_listed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let location = Location::file(dir.path());
        let source = Arc::new(StdFsSource::new());
        let mut model = DirectoryModel::new();
        let listing = model.begin(source, location.clone());
        drain_listing(&mut model, &listing);
        assert!(model.is_empty());

        let watcher = Arc::new(InotifyWatcher::new());
        let handle = model.begin_watch(watcher, &location).expect("watch");

        std::fs::write(dir.path().join(".directory"), b"[Desktop Entry]\n").expect("write");
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            model.drain_watch(&handle);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(model.is_empty(), "a hidden create must not add a row");
        model.check_consistent().expect("consistent");
    }

    #[test]
    fn renaming_a_visible_file_hidden_removes_its_row() {
        let dir = tempfile::tempdir().expect("temp dir");
        let visible = dir.path().join("visible.txt");
        std::fs::write(&visible, b"x").expect("write");
        let location = Location::file(dir.path());
        let source = Arc::new(StdFsSource::new());
        let mut model = DirectoryModel::new();
        let listing = model.begin(source, location.clone());
        drain_listing(&mut model, &listing);
        let uri = Location::file(&visible).uri().to_owned();
        assert!(model.node_id_for_uri(&uri).is_some());

        let watcher = Arc::new(InotifyWatcher::new());
        let handle = model.begin_watch(watcher, &location).expect("watch");
        std::fs::rename(&visible, dir.path().join(".visible.txt")).expect("rename");
        wait_for(&mut model, &handle, |model| {
            model.node_id_for_uri(&uri).is_none()
        });
        assert!(model.node_id_for_uri(&uri).is_none());
        assert!(model.is_empty());
        model.check_consistent().expect("consistent");
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

// T332: the model-side half of the Files watcher. A burst folds through one
// batch and is described by one delta-worthy set of markers; a rename keeps
// the id the view selected by; and the C ABI session watches its own folder.
#[test]
fn a_coalesced_burst_is_one_model_update() {
    let dir = tempfile::tempdir().expect("temp dir");
    let location = Location::file(dir.path());
    let source = Arc::new(StdFsSource::new());
    let mut model = DirectoryModel::new();
    let listing = model.begin(source, location.clone());
    drain_listing(&mut model, &listing);
    assert_eq!(model.len(), 0);
    assert!(
        !model.take_removal(),
        "a fresh listing has no removal marker"
    );

    let watcher = Arc::new(InotifyWatcher::new());
    let handle = model.begin_watch(watcher, &location).expect("watch");

    // An extraction-sized burst: a folder and many files, written quickly.
    std::fs::create_dir(dir.path().join("Extracted")).expect("dir");
    for index in 0..24 {
        std::fs::write(dir.path().join(format!("file-{index:02}.txt")), b"payload").expect("write");
    }

    // Wait until the burst settles: every entry is listed and no more batch
    // is arriving. The creates stay additions however many batches the
    // kernel split them into; no held row is ever "changed".
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        model.drain_watch_batches(&handle);
        let quiet = model.len() >= 25
            && handle
                .recv_batch_timeout(Duration::from_millis(300))
                .is_none();
        if quiet || Instant::now() >= deadline {
            break;
        }
    }
    assert_eq!(
        model.len(),
        25,
        "every created entry is listed exactly once"
    );
    assert_eq!(
        model.added_ids().len(),
        25,
        "the creates are additions, not changes"
    );
    assert!(
        model.take_watch_changes().is_empty(),
        "no held row was replaced by the burst"
    );
    assert!(!model.take_removal());
    let folder = model
        .node_id_for_uri(Location::file(dir.path().join("Extracted")).uri())
        .expect("the created folder is listed");
    assert!(
        model.node(folder).expect("node").is_dir(),
        "IN_ISDIR honoured"
    );
    model.check_consistent().expect("consistent");
}

#[test]
fn a_batch_rename_keeps_the_selected_id_and_marks_the_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = dir.path().join("before.txt");
    std::fs::write(&before, b"x").expect("write");
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
    let before_uri = Location::file(&before).uri().to_owned();
    let id = model.model().node_id_for_uri(&before_uri).expect("listed");
    model.selection_mut().select(id);

    let watcher = Arc::new(InotifyWatcher::new());
    let handle = model.begin_watch(watcher, &location).expect("watch");
    let after = dir.path().join("after.txt");
    std::fs::rename(&before, &after).expect("rename");
    let after_uri = Location::file(&after).uri().to_owned();

    let deadline = Instant::now() + Duration::from_secs(5);
    while model.model().node_id_for_uri(&after_uri).is_none() && Instant::now() < deadline {
        model.drain_watch_batches(&handle);
        if model.model().node_id_for_uri(&after_uri).is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    // A rename is a change of a held row: the same id, the same selection.
    assert_eq!(model.model().node_id_for_uri(&after_uri), Some(id));
    assert_eq!(model.selection().as_slice(), &[id], "selection survives");
    assert_eq!(model.model_mut().take_watch_changes(), vec![id]);
    assert!(
        !model.model().pending_changes(),
        "the change was taken once"
    );
    assert!(!model.model().removal_pending(), "nothing vanished");
    model.model().check_consistent().expect("consistent");
}

#[cfg(target_os = "linux")]
#[test]
fn the_ffi_session_watches_its_folder_and_skips_its_own_writes() {
    use dragonfruit_files_core::ffi::{
        df_files_await_ops, df_files_begin, df_files_begin_new_folder, df_files_delta_free,
        df_files_free, df_files_poll_delta, df_files_watching, DF_FILES_DELTA_CHANGED,
        DF_FILES_DELTA_INSERT, DF_FILES_DELTA_REMOVED, DF_FILES_STATUS_BATCH, DF_FILES_STATUS_DONE,
        DF_FILES_STATUS_ERROR, DF_FILES_STATUS_TIMEOUT,
    };
    use std::ffi::CString;

    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("seed.txt"), b"seed").expect("write");
    let uri = Location::file(dir.path()).uri().to_owned();
    let uri = CString::new(uri).expect("uri");
    unsafe {
        let session = df_files_begin(uri.as_ptr());
        assert!(!session.is_null());
        assert_eq!(df_files_watching(session), 1, "a local folder is watched");

        // The listing has one file: keep asking until it is painted (a
        // streaming batch may trail the first poll, and the watch can split
        // a burst across polls).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let delta = df_files_poll_delta(session, 500);
            assert!(!delta.is_null(), "poll returned null");
            let status = (*delta).status;
            let kind = (*delta).kind;
            let total = (*delta).total;
            eprintln!("DBG poll status={status} kind={kind} total={total}");
            df_files_delta_free(delta);
            assert_ne!(status, DF_FILES_STATUS_ERROR, "listing failed");
            if (status == DF_FILES_STATUS_BATCH && total >= 1)
                || std::time::Instant::now() > deadline
            {
                assert!(
                    total >= 1,
                    "the seed file is painted before the watch is exercised"
                );
                let _ = kind;
                break;
            }
        }
        // Drain anything still queued (the listing's `DONE`).
        loop {
            let delta = df_files_poll_delta(session, 50);
            let status = (*delta).status;
            df_files_delta_free(delta);
            if status == DF_FILES_STATUS_DONE || status == DF_FILES_STATUS_TIMEOUT {
                break;
            }
        }
        // A settled watched session reports a quiet timeout, not a poll loop.
        let delta = df_files_poll_delta(session, 50);
        assert_eq!((*delta).status, DF_FILES_STATUS_TIMEOUT);
        df_files_delta_free(delta);

        // An external rename becomes in-place changes of the one held row
        // (the kernel reports the pair; a burst may split across polls).
        std::fs::rename(dir.path().join("seed.txt"), dir.path().join("renamed.txt"))
            .expect("rename");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut saw_renamed = false;
        let mut saw_removal = false;
        while !saw_renamed && std::time::Instant::now() < deadline {
            let delta = df_files_poll_delta(session, 2_000);
            assert_eq!(
                (*delta).status,
                DF_FILES_STATUS_BATCH,
                "a rename is a model change"
            );
            match (*delta).kind {
                DF_FILES_DELTA_CHANGED => {
                    // Every change is a held row: the receiver repaints it in
                    // place, it never rebuilds the list.
                    for index in 0..(*delta).change_count as usize {
                        let change = (*delta).changes.add(index).read();
                        let name = std::ffi::CStr::from_ptr(change.node.name).to_string_lossy();
                        assert_ne!(name, "seed.txt");
                        saw_renamed |= name == "renamed.txt";
                    }
                }
                DF_FILES_DELTA_REMOVED => saw_removal = true,
                other => panic!("an in-place rename must not reset/insert, got kind {other}"),
            }
            df_files_delta_free(delta);
        }
        assert!(saw_renamed, "the renamed row was reported");
        assert!(!saw_removal, "a rename never looks like a removal");

        // An external create is an insertion at its final rank.
        std::fs::write(dir.path().join("created.txt"), b"new").expect("write");
        let delta = df_files_poll_delta(session, 2_000);
        assert_eq!((*delta).status, DF_FILES_STATUS_BATCH);
        assert_eq!((*delta).kind, DF_FILES_DELTA_INSERT);
        assert_eq!((*delta).row_count, 1);
        df_files_delta_free(delta);

        // The session's own New Folder paints its row optimistically (a
        // reset) and its watcher echo folds into that same row: the listing
        // never reports a second row for the folder.
        assert!(df_files_begin_new_folder(session, uri.as_ptr()) > 0);
        df_files_await_ops(session, 2_000);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut total = 0;
        while std::time::Instant::now() < deadline {
            let delta = df_files_poll_delta(session, 50);
            let status = (*delta).status;
            if status == DF_FILES_STATUS_BATCH {
                total = (*delta).total;
            }
            df_files_delta_free(delta);
            if status == DF_FILES_STATUS_TIMEOUT {
                break;
            }
        }
        // The seed, the external create and the folder — once each.
        assert_eq!(total, 3, "the session's own write is not listed twice");
        assert!(dir.path().join("untitled folder").is_dir());
        df_files_free(session);
    }
}
