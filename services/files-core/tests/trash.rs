// SPDX-License-Identifier: MIT
//! T-10.3a acceptance: trash, restore, and empty round-trip against a temp
//! tree through the sanctioned freedesktop-spec fallback, with the real
//! `.trashinfo` store format. Headless: no display, no GIO, no daemon.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use dragonfruit_files_core::{
    parse_trash_info, DirectoryModel, FreedesktopTrash, Location, MockSource, Node, NodeKind,
    OperationError, OptimisticModel, TrashOps,
};

/// A trash store rooted inside the test's temp dir, so the round-trip never
/// touches the real user trash.
fn trash_for(dir: &Path) -> FreedesktopTrash {
    FreedesktopTrash::with_home_trash(dir.join("Trash"))
}

#[test]
fn trash_then_restore_round_trip() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let original = dir.path().join("report.txt");
    fs::write(&original, b"payload").expect("write");

    let item = trash
        .trash(&Location::file(&original))
        .expect("trash the file");
    assert!(!original.exists(), "moved out of its original home");
    assert!(item.file_path().is_file());
    assert_eq!(fs::read(item.file_path()).unwrap(), b"payload");
    assert_eq!(item.original_path(), original.as_path());
    assert!(item.info_path().is_file());

    let text = fs::read_to_string(item.info_path()).expect("read info");
    assert!(text.starts_with("[Trash Info]\n"), "info: {text:?}");
    let (recorded, date) = parse_trash_info(&text).expect("parse info");
    assert_eq!(recorded, original);
    assert_eq!(date.len(), 19, "ISO 8601 date: {date:?}");

    let entries = trash.entries().expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name(), OsStr::new("report.txt"));
    assert_eq!(entries[0].original_path(), original.as_path());

    let restored = trash.restore(&item).expect("put back");
    assert_eq!(restored, Location::file(&original));
    assert_eq!(fs::read(&original).unwrap(), b"payload");
    assert!(!item.file_path().exists());
    assert!(!item.info_path().exists());
    assert!(trash.entries().unwrap().is_empty());
}

#[test]
fn trash_moves_a_directory_recursively_and_restores_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let folder = dir.path().join("project");
    fs::create_dir_all(folder.join("nested")).expect("mkdir");
    fs::write(folder.join("nested/deep.txt"), b"deep").expect("write");

    let item = trash.trash(&Location::file(&folder)).expect("trash folder");
    assert!(!folder.exists());
    assert_eq!(
        fs::read(item.file_path().join("nested/deep.txt")).unwrap(),
        b"deep"
    );

    trash.restore(&item).expect("restore folder");
    assert_eq!(fs::read(folder.join("nested/deep.txt")).unwrap(), b"deep");
}

#[test]
fn a_duplicate_name_is_de_duplicated_in_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let original = dir.path().join("same.txt");

    fs::write(&original, b"first").expect("write first");
    trash
        .trash(&Location::file(&original))
        .expect("trash first");
    fs::write(&original, b"second").expect("write second");
    trash
        .trash(&Location::file(&original))
        .expect("trash second");

    let names: BTreeSet<OsString> = trash
        .entries()
        .unwrap()
        .into_iter()
        .map(|item| item.name().to_os_string())
        .collect();
    assert_eq!(
        names,
        BTreeSet::from([OsString::from("same.txt"), OsString::from("same.txt 2")])
    );
}

#[test]
fn empty_removes_every_trashed_item() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    for name in ["a.txt", "b.txt", "c.txt"] {
        let path = dir.path().join(name);
        fs::write(&path, b"x").expect("write");
        trash.trash(&Location::file(&path)).expect("trash");
    }

    assert_eq!(trash.empty().expect("empty"), 3);
    assert!(trash.entries().unwrap().is_empty());
    assert_eq!(
        fs::read_dir(trash.home_trash().join("files"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        fs::read_dir(trash.home_trash().join("info"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(trash.empty().expect("empty again"), 0);
}

#[test]
fn empty_on_a_store_that_never_existed_is_zero() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    assert_eq!(trash.empty().expect("empty"), 0);
    assert!(trash.entries().unwrap().is_empty());
}

#[test]
fn trash_refuses_a_foreign_scheme() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let error = trash
        .trash(&Location::parse("trash:///already-there").expect("parse"))
        .expect_err("unsupported");
    assert!(
        matches!(error, OperationError::UnsupportedScheme(ref scheme) if scheme == "trash"),
        "got {error:?}"
    );
}

#[test]
fn trash_reports_a_missing_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let error = trash
        .trash(&Location::file(dir.path().join("missing.txt")))
        .expect_err("missing");
    assert!(
        matches!(error, OperationError::NotFound(_)),
        "got {error:?}"
    );
}

#[test]
fn restore_refuses_when_the_original_path_is_occupied() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let original = dir.path().join("keep.txt");
    fs::write(&original, b"old").expect("write");
    let item = trash.trash(&Location::file(&original)).expect("trash");

    fs::write(&original, b"new").expect("recreate");
    let error = trash.restore(&item).expect_err("occupied");
    assert!(
        matches!(error, OperationError::AlreadyExists(_)),
        "got {error:?}"
    );
    // The trashed copy is untouched.
    assert!(item.file_path().is_file());
}

#[test]
fn restore_reports_a_missing_original_parent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let folder = dir.path().join("sub");
    fs::create_dir(&folder).expect("mkdir");
    let original = folder.join("note.txt");
    fs::write(&original, b"note").expect("write");
    let item = trash.trash(&Location::file(&original)).expect("trash");
    fs::remove_dir(&folder).expect("remove parent");

    let error = trash.restore(&item).expect_err("parent gone");
    assert!(
        matches!(error, OperationError::NotFound(_)),
        "got {error:?}"
    );
}

#[test]
fn malformed_trashinfo_entries_are_skipped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let original = dir.path().join("good.txt");
    fs::write(&original, b"good").expect("write");
    trash.trash(&Location::file(&original)).expect("trash");

    fs::write(
        trash.home_trash().join("info/broken.trashinfo"),
        b"not a trash info file\n",
    )
    .expect("write broken");
    fs::write(trash.home_trash().join("info/ignored.txt"), b"x").expect("write ignored");

    let entries = trash.entries().expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name(), OsStr::new("good.txt"));
}

#[test]
#[cfg(unix)]
fn trashing_and_restoring_preserves_non_utf8_names() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let name = OsStr::from_bytes(b"odd\xffname.txt");
    let original = dir.path().join(name);
    fs::write(&original, b"bytes").expect("write");

    let item = trash.trash(&Location::file(&original)).expect("trash");
    assert_eq!(item.name().as_bytes(), b"odd\xffname.txt");
    let entries = trash.entries().expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name().as_bytes(), b"odd\xffname.txt");

    trash.restore(&item).expect("restore");
    assert_eq!(fs::read(dir.path().join(name)).unwrap(), b"bytes");
}

fn loaded_model(dir: &Path, file: &Path) -> OptimisticModel {
    let node = Node::new(
        file.file_name().unwrap(),
        Location::file(file).uri().to_owned(),
        NodeKind::File,
    );
    let source = Arc::new(MockSource::streaming(vec![vec![node]]));
    let mut model = OptimisticModel::new(DirectoryModel::new());
    let handle = model.begin(source, Location::file(dir));
    while !model.is_complete() {
        let Some(event) = handle.recv_timeout(Duration::from_secs(5)) else {
            break;
        };
        model.apply(event);
    }
    model
}

#[test]
fn trash_via_hides_the_row_and_lands_in_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let file = dir.path().join("gone.txt");
    fs::write(&file, b"x").expect("write");
    let mut model = loaded_model(dir.path(), &file);
    let id = model.model().nodes()[0].id();

    let op = model.trash_via(&trash, id).expect("trash_via");
    assert!(!model.is_pending(op));
    assert!(model.model().node(id).is_none());
    assert!(!file.exists());
    assert_eq!(trash.entries().unwrap().len(), 1);
}

#[test]
fn trash_via_reverts_when_the_source_is_already_gone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trash = trash_for(dir.path());
    let file = dir.path().join("vanished.txt");
    fs::write(&file, b"x").expect("write");
    let mut model = loaded_model(dir.path(), &file);
    let id = model.model().nodes()[0].id();
    fs::remove_file(&file).expect("remove behind the model's back");

    let error = model.trash_via(&trash, id).expect_err("gone");
    assert!(
        matches!(error, OperationError::NotFound(_)),
        "got {error:?}"
    );
    assert!(model.model().node(id).is_some(), "row snapped back");
    assert_eq!(model.pending_len(), 0);
}
