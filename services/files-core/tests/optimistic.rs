// SPDX-License-Identifier: MIT
//! T-10.2b acceptance: operations apply optimistically (visible before any
//! confirmation), then reconcile — confirm keeps the painted result, revert
//! snaps it back — while the sort spec and the selection survive. The `*_via`
//! cases run the real operation through `StdFsOps` against a temp tree, so
//! the model and the filesystem are checked to agree. Headless: no display,
//! no GIO, no daemon.

use std::ffi::OsStr;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use dragonfruit_files_core::{
    DirectoryModel, Location, MockSource, Node, NodeId, NodeKind, OperationError, OptimisticModel,
    SortDirection, SortKey, SortSpec, StdFsOps,
};

fn files(names: &[&str]) -> Vec<Node> {
    names
        .iter()
        .map(|name| Node::new(*name, format!("file:///fixture/{name}"), NodeKind::File))
        .collect()
}

fn loaded(nodes: Vec<Node>) -> OptimisticModel {
    let source = Arc::new(MockSource::streaming(vec![nodes]));
    let mut model = OptimisticModel::new(DirectoryModel::new());
    let handle = model.begin(source, Location::file("/fixture"));
    while !model.is_complete() {
        let Some(event) = handle.recv_timeout(Duration::from_secs(5)) else {
            break;
        };
        model.apply(event);
    }
    model
}

fn name_of(model: &OptimisticModel, id: NodeId) -> String {
    model
        .model()
        .node(id)
        .expect("node present")
        .display_name()
        .into_owned()
}

fn ordered_names(model: &OptimisticModel) -> Vec<String> {
    model
        .model()
        .ordered()
        .map(|node| node.display_name().into_owned())
        .collect()
}

fn node_id(model: &OptimisticModel, name: &str) -> NodeId {
    model
        .model()
        .nodes()
        .iter()
        .find(|node| node.display_name() == name)
        .expect("node by name")
        .id()
}

#[test]
fn an_optimistic_rename_is_visible_immediately_then_confirms() {
    let mut model = loaded(files(&["b", "a", "c"]));
    let a = node_id(&model, "a");
    let b = node_id(&model, "b");
    model.selection_mut().select(a);
    model.selection_mut().select(b);
    let sort_before = *model.sort_spec();

    let op = model.begin_rename(a, "z").expect("pending rename");
    // The row already shows the new name: no confirmation ran.
    assert_eq!(name_of(&model, a), "z");
    assert_eq!(ordered_names(&model), vec!["b", "c", "z"]);
    // Sort, selection, and ids are untouched.
    assert_eq!(*model.sort_spec(), sort_before);
    assert_eq!(model.selection().as_slice(), &[a, b]);
    assert!(model.is_pending(op));
    model.model().check_consistent().expect("consistent");

    assert!(model.confirm(op));
    assert!(!model.is_pending(op));
    assert_eq!(name_of(&model, a), "z");
    assert_eq!(model.selection().as_slice(), &[a, b]);
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_reverted_rename_snaps_back_and_keeps_state() {
    let mut model = loaded(files(&["a", "b"]));
    let a = node_id(&model, "a");
    let b = node_id(&model, "b");
    model.selection_mut().select(a);

    let op = model.begin_rename(a, "zzz").expect("pending rename");
    assert_eq!(ordered_names(&model), vec!["b", "zzz"]);
    assert!(model.revert(op));
    assert_eq!(name_of(&model, a), "a");
    assert_eq!(ordered_names(&model), vec!["a", "b"]);
    assert_eq!(model.selection().as_slice(), &[a]);
    assert_eq!(model.pending_len(), 0);
    // The restored node keeps its uri too.
    assert_eq!(
        model.model().node(a).expect("node").uri(),
        "file:///fixture/a"
    );
    let _ = b;
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_new_folder_appears_folders_first_and_reverts_cleanly() {
    let mut model = loaded(files(&["note.txt"]));
    let parent = Location::file("/fixture");

    let op = model.begin_new_folder(&parent);
    assert_eq!(model.len(), 2);
    let created = model
        .model()
        .nodes()
        .iter()
        .find(|node| node.name() == OsStr::new("untitled folder"))
        .expect("optimistic folder");
    assert!(created.is_dir());
    assert_eq!(created.uri(), "file:///fixture/untitled%20folder");
    // Folders-first default puts it at the top.
    assert_eq!(ordered_names(&model)[0], "untitled folder");
    model.model().check_consistent().expect("consistent");

    assert!(model.revert(op));
    assert_eq!(model.len(), 1);
    assert!(model
        .model()
        .nodes()
        .iter()
        .all(|node| node.name() != OsStr::new("untitled folder")));
    model.model().check_consistent().expect("consistent");
}

#[test]
fn new_folders_number_against_the_rows_already_in_the_model() {
    let mut model = loaded(files(&["untitled folder", "untitled folder 2"]));
    let parent = Location::file("/fixture");
    model.begin_new_folder(&parent);
    assert!(model
        .model()
        .nodes()
        .iter()
        .any(|node| node.name() == OsStr::new("untitled folder 3")));
    model.model().check_consistent().expect("consistent");
}

#[test]
fn selection_survives_delete_revert_and_resort() {
    let mut model = loaded(files(&["a", "b", "c"]));
    let a = node_id(&model, "a");
    let c = node_id(&model, "c");
    model.selection_mut().select(a);
    model.selection_mut().select(c);

    let op = model.begin_delete(a).expect("pending delete");
    assert!(model.model().node(a).is_none());
    assert_eq!(model.selection().as_slice(), &[c]);
    model.model().check_consistent().expect("consistent");

    assert!(model.revert(op));
    assert_eq!(model.selection().as_slice(), &[a, c]);
    model.model().check_consistent().expect("consistent");

    // A re-sort does not disturb the selection, and a confirmed delete drops
    // only the deleted id.
    model.set_sort(
        SortSpec::new(SortKey::Name)
            .with_direction(SortDirection::Descending)
            .with_folders_first(false),
    );
    assert_eq!(ordered_names(&model), vec!["c", "b", "a"]);
    assert_eq!(model.selection().as_slice(), &[a, c]);
    let op = model.begin_delete(c).expect("pending delete");
    assert!(model.confirm(op));
    assert_eq!(model.selection().as_slice(), &[a]);
    assert!(model.model().node(c).is_none());
    model.model().check_consistent().expect("consistent");
}

#[test]
fn rename_via_reconciles_the_model_with_the_filesystem() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("a.txt"), b"payload").expect("write");
    let source = Location::file(dir.path().join("a.txt"));
    let mut model = loaded(vec![Node::new(
        OsStr::new("a.txt"),
        source.uri(),
        NodeKind::File,
    )]);
    let id = node_id(&model, "a.txt");
    model.selection_mut().select(id);

    let ops = StdFsOps::new();
    let op = model
        .rename_via(&ops, id, OsStr::new("b.txt"))
        .expect("rename");
    assert_eq!(op.get(), 1);
    assert!(!model.is_pending(op));
    assert!(dir.path().join("b.txt").exists());
    assert!(!dir.path().join("a.txt").exists());
    assert_eq!(name_of(&model, id), "b.txt");
    assert_eq!(
        model.model().node(id).expect("node").uri(),
        Location::file(dir.path().join("b.txt")).uri()
    );
    assert_eq!(model.selection().as_slice(), &[id]);
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_failed_rename_reverts_the_optimistic_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("a.txt"), b"a").expect("write a");
    fs::write(dir.path().join("b.txt"), b"b").expect("write b");
    let mut model = loaded(vec![
        Node::new(
            OsStr::new("a.txt"),
            Location::file(dir.path().join("a.txt")).uri(),
            NodeKind::File,
        ),
        Node::new(
            OsStr::new("b.txt"),
            Location::file(dir.path().join("b.txt")).uri(),
            NodeKind::File,
        ),
    ]);
    let a = node_id(&model, "a.txt");
    model.selection_mut().select(a);

    let ops = StdFsOps::new();
    let error = model
        .rename_via(&ops, a, OsStr::new("b.txt"))
        .expect_err("conflict");
    assert!(
        matches!(error, OperationError::AlreadyExists(_)),
        "got {error:?}"
    );
    assert_eq!(name_of(&model, a), "a.txt");
    assert_eq!(model.pending_len(), 0);
    assert_eq!(model.selection().as_slice(), &[a]);
    assert!(dir.path().join("a.txt").exists());
    assert_eq!(fs::read(dir.path().join("b.txt")).unwrap(), b"b");
    model.model().check_consistent().expect("consistent");
}

#[test]
fn delete_via_removes_the_file_and_confirms() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("note.txt"), b"x").expect("write");
    let mut model = loaded(vec![Node::new(
        OsStr::new("note.txt"),
        Location::file(dir.path().join("note.txt")).uri(),
        NodeKind::File,
    )]);
    let id = node_id(&model, "note.txt");
    model.selection_mut().select(id);

    let ops = StdFsOps::new();
    model.delete_via(&ops, id).expect("delete");
    assert!(!dir.path().join("note.txt").exists());
    assert!(model.model().node(id).is_none());
    assert!(model.selection().is_empty());
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_failed_delete_restores_the_row_and_its_selection() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut model = loaded(vec![Node::new(
        OsStr::new("gone"),
        Location::file(dir.path().join("gone")).uri(),
        NodeKind::File,
    )]);
    let id = node_id(&model, "gone");
    model.selection_mut().select(id);

    let ops = StdFsOps::new();
    let error = model.delete_via(&ops, id).expect_err("missing");
    assert!(
        matches!(error, OperationError::NotFound(_)),
        "got {error:?}"
    );
    assert_eq!(name_of(&model, id), "gone");
    assert_eq!(model.selection().as_slice(), &[id]);
    assert_eq!(model.pending_len(), 0);
    model.model().check_consistent().expect("consistent");
}

#[test]
fn new_folder_via_creates_a_real_directory_the_model_agrees_with() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut model = loaded(Vec::new());
    let parent = Location::file(dir.path());
    let ops = StdFsOps::new();

    model.new_folder_via(&ops, &parent).expect("first folder");
    model.new_folder_via(&ops, &parent).expect("second folder");

    assert!(dir.path().join("untitled folder").is_dir());
    assert!(dir.path().join("untitled folder 2").is_dir());
    let names: Vec<String> = model
        .model()
        .ordered()
        .map(|node| node.display_name().into_owned())
        .collect();
    assert_eq!(names, vec!["untitled folder", "untitled folder 2"]);
    assert_eq!(
        model
            .model()
            .nodes()
            .iter()
            .find(|node| node.name() == OsStr::new("untitled folder 2"))
            .expect("node")
            .uri(),
        Location::file(dir.path().join("untitled folder 2")).uri()
    );
    assert_eq!(model.pending_len(), 0);
    model.model().check_consistent().expect("consistent");
}

#[test]
fn a_foreign_scheme_operation_reverts_and_reports_unsupported() {
    let mut model = loaded(vec![Node::new("x", "trash:///x", NodeKind::File)]);
    let id = node_id(&model, "x");

    let ops = StdFsOps::new();
    let error = model
        .rename_via(&ops, id, OsStr::new("y"))
        .expect_err("trash unsupported");
    assert!(
        matches!(error, OperationError::UnsupportedScheme(ref scheme) if scheme == "trash"),
        "got {error:?}"
    );
    assert_eq!(name_of(&model, id), "x");
    assert_eq!(model.pending_len(), 0);
    model.model().check_consistent().expect("consistent");
}
