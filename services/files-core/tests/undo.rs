// SPDX-License-Identifier: GPL-3.0-only
//! T331 acceptance: the bounded undo/redo stack for rename, new folder, move,
//! copy, and Move to Trash. Every case runs the real operation through
//! `StdFsOps` / `FreedesktopTrash` against a temp tree, then undoes it and
//! checks the filesystem was restored exactly — and that a refused undo
//! reports a reason while leaving the current state untouched. Headless: no
//! display, no GIO, no daemon.

use std::fs;
use std::path::Path;

use dragonfruit_files_core::{
    FileOps, FreedesktopTrash, Location, OperationError, StdFsOps, TrashOps, UndoError, UndoStack,
    UNDO_LIMIT,
};

fn trash_for(dir: &Path) -> FreedesktopTrash {
    FreedesktopTrash::with_home_trash(dir.join("Trash"))
}

fn write(path: &Path, body: &[u8]) {
    fs::write(path, body).expect("write fixture");
}

/// Resolve `<dir>/<relative>` to a `file://` location.
fn at(dir: &Path, relative: &str) -> Location {
    Location::file(dir.join(relative))
}

#[test]
fn a_rename_undoes_and_redoes() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("a.txt"), b"payload");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();

    let renamed = ops
        .rename(&at(dir.path(), "a.txt"), "b.txt".as_ref())
        .expect("rename");
    stack.record_rename(at(dir.path(), "a.txt"), renamed);
    assert_eq!(stack.undo_label(), Some("Rename \"a.txt\""));
    assert!(dir.path().join("b.txt").exists());

    let outcome = stack.undo(&ops, &trash_for(dir.path())).expect("undo");
    assert_eq!(outcome.message(false), "Undid \"Rename \"a.txt\"\"");
    assert!(dir.path().join("a.txt").exists());
    assert!(!dir.path().join("b.txt").exists());
    assert_eq!(fs::read(dir.path().join("a.txt")).unwrap(), b"payload");
    assert_eq!(stack.undo_len(), 0);
    assert_eq!(stack.redo_len(), 1);

    let redone = stack.redo(&ops, &trash_for(dir.path())).expect("redo");
    assert_eq!(redone.message(true), "Redid \"Rename \"a.txt\"\"");
    assert!(dir.path().join("b.txt").exists());
    assert!(!dir.path().join("a.txt").exists());
    assert_eq!(stack.undo_len(), 1);
    assert_eq!(stack.redo_len(), 0);
}

#[test]
fn a_rename_undo_reports_a_taken_name_and_changes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("a.txt"), b"a");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let renamed = ops
        .rename(&at(dir.path(), "a.txt"), "b.txt".as_ref())
        .expect("rename");
    stack.record_rename(at(dir.path(), "a.txt"), renamed);

    // Something else took the old name while b.txt existed.
    write(&dir.path().join("a.txt"), b"newcomer");
    let error = stack.undo(&ops, &trash_for(dir.path())).unwrap_err();
    assert_eq!(error.reason(), "\"a.txt\" already exists");
    match &error {
        UndoError::Conflict(conflict) => {
            assert_eq!(conflict.kind, "exists");
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    // Both files are exactly as they were before the refused undo.
    assert_eq!(fs::read(dir.path().join("a.txt")).unwrap(), b"newcomer");
    assert_eq!(fs::read(dir.path().join("b.txt")).unwrap(), b"a");
    // The step is still undoable once the conflict is cleared.
    assert_eq!(stack.undo_len(), 1);
}

#[test]
fn a_rename_undo_of_a_vanished_item_reports_missing() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("a.txt"), b"a");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let renamed = ops
        .rename(&at(dir.path(), "a.txt"), "b.txt".as_ref())
        .expect("rename");
    stack.record_rename(at(dir.path(), "a.txt"), renamed);
    fs::remove_file(dir.path().join("b.txt")).unwrap();

    let error = stack.undo(&ops, &trash_for(dir.path())).unwrap_err();
    match &error {
        UndoError::Conflict(conflict) => assert_eq!(conflict.kind, "missing"),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert!(
        error.reason().contains("b.txt"),
        "reason: {}",
        error.reason()
    );
    assert_eq!(stack.undo_len(), 1, "a refused undo keeps the step");
}

#[test]
fn a_new_folder_undoes_and_a_non_empty_one_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let parent = Location::file(dir.path());

    let created = ops.new_folder(&parent).expect("new folder");
    assert!(dir.path().join("untitled folder").is_dir());
    stack.record_new_folder(created);
    stack.undo(&ops, &trash_for(dir.path())).expect("undo");
    assert!(!dir.path().join("untitled folder").exists());

    // A folder that has since gained a file is never deleted by undo.
    let created = ops.new_folder(&parent).expect("second folder");
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        1,
        "the first folder is gone, so the new one is `untitled folder`"
    );
    write(&created.to_file_path().unwrap().join("kept.txt"), b"keep");
    stack.record_new_folder(created);
    let error = stack.undo(&ops, &trash_for(dir.path())).unwrap_err();
    match &error {
        UndoError::Conflict(conflict) => assert_eq!(conflict.kind, "not-empty"),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert!(dir.path().join("untitled folder/kept.txt").exists());
    assert_eq!(stack.undo_len(), 1);
}

#[test]
fn a_copy_undo_removes_only_the_copy() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("source")).unwrap();
    fs::create_dir(dir.path().join("dest")).unwrap();
    write(&dir.path().join("source/report.txt"), b"report");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();

    let source = at(dir.path(), "source/report.txt");
    let copied = ops
        .copy_as(&source, &at(dir.path(), "dest"), "report.txt".as_ref())
        .expect("copy");
    stack.record_copy(source.clone(), copied.clone());
    assert!(dir.path().join("dest/report.txt").exists());

    stack.undo(&ops, &trash_for(dir.path())).expect("undo");
    assert!(!dir.path().join("dest/report.txt").exists());
    // The source is untouched: undo removed only what the copy made.
    assert_eq!(fs::read(source.to_file_path().unwrap()).unwrap(), b"report");
}

#[test]
fn a_copy_undo_of_a_vanished_copy_reports_missing() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("report.txt"), b"report");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let source = at(dir.path(), "report.txt");
    let copied = ops
        .copy_as(
            &source,
            &Location::file(dir.path()),
            "report 2.txt".as_ref(),
        )
        .expect("copy");
    stack.record_copy(source, copied);
    fs::remove_file(dir.path().join("report 2.txt")).unwrap();

    let error = stack.undo(&ops, &trash_for(dir.path())).unwrap_err();
    match &error {
        UndoError::Conflict(conflict) => assert_eq!(conflict.kind, "missing"),
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn a_move_undoes_back_to_the_source_folder() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("from")).unwrap();
    fs::create_dir(dir.path().join("to")).unwrap();
    write(&dir.path().join("from/move-me.txt"), b"move");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();

    let from = at(dir.path(), "from/move-me.txt");
    let moved = ops
        .move_as(&from, &at(dir.path(), "to"), "move-me.txt".as_ref())
        .expect("move");
    stack.record_move(from.clone(), moved);
    assert!(dir.path().join("to/move-me.txt").exists());

    stack.undo(&ops, &trash_for(dir.path())).expect("undo");
    assert!(dir.path().join("from/move-me.txt").exists());
    assert!(!dir.path().join("to/move-me.txt").exists());
    assert_eq!(fs::read(from.to_file_path().unwrap()).unwrap(), b"move");

    stack.redo(&ops, &trash_for(dir.path())).expect("redo");
    assert!(dir.path().join("to/move-me.txt").exists());
    assert!(!dir.path().join("from/move-me.txt").exists());
}

#[test]
fn a_move_undo_refuses_when_the_original_spot_is_taken() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("from")).unwrap();
    fs::create_dir(dir.path().join("to")).unwrap();
    write(&dir.path().join("from/move-me.txt"), b"move");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let from = at(dir.path(), "from/move-me.txt");
    let moved = ops
        .move_as(&from, &at(dir.path(), "to"), "move-me.txt".as_ref())
        .expect("move");
    stack.record_move(from, moved);
    write(&dir.path().join("from/move-me.txt"), b"squatter");

    let error = stack.undo(&ops, &trash_for(dir.path())).unwrap_err();
    match &error {
        UndoError::Conflict(conflict) => {
            assert_eq!(conflict.kind, "exists");
            assert_eq!(
                conflict.item.as_ref().map(Location::display),
                Some(at(dir.path(), "from/move-me.txt").display())
            );
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    // Neither file moved.
    assert_eq!(
        fs::read(dir.path().join("from/move-me.txt")).unwrap(),
        b"squatter"
    );
    assert_eq!(
        fs::read(dir.path().join("to/move-me.txt")).unwrap(),
        b"move"
    );
}

#[test]
fn a_move_undo_refuses_when_the_source_folder_is_gone() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("from")).unwrap();
    fs::create_dir(dir.path().join("to")).unwrap();
    write(&dir.path().join("from/move-me.txt"), b"move");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let from = at(dir.path(), "from/move-me.txt");
    let moved = ops
        .move_as(&from, &at(dir.path(), "to"), "move-me.txt".as_ref())
        .expect("move");
    stack.record_move(from, moved);
    fs::remove_dir(dir.path().join("from")).unwrap();

    let error = stack.undo(&ops, &trash_for(dir.path())).unwrap_err();
    match &error {
        UndoError::Conflict(conflict) => assert_eq!(conflict.kind, "parent-gone"),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert!(dir.path().join("to/move-me.txt").exists());
}

#[test]
fn a_trash_undo_puts_the_item_back_and_a_redo_trashes_it_again() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("note.txt"), b"hi");
    let trash = trash_for(dir.path());
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();

    let item = trash
        .trash(&at(dir.path(), "note.txt"))
        .expect("trash the file");
    assert!(!dir.path().join("note.txt").exists());
    stack.record_trash(item);
    assert_eq!(stack.undo_label(), Some("Move \"note.txt\" to Trash"));

    stack.undo(&ops, &trash).expect("undo trashes");
    assert_eq!(fs::read(dir.path().join("note.txt")).unwrap(), b"hi");
    assert!(trash.entries().unwrap().is_empty());
    assert_eq!(stack.redo_len(), 1);

    stack.redo(&ops, &trash).expect("redo trash");
    assert!(!dir.path().join("note.txt").exists());
    assert_eq!(trash.entries().unwrap().len(), 1);
}

#[test]
fn a_trash_undo_reports_a_taken_original_spot() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("note.txt"), b"hi");
    let trash = trash_for(dir.path());
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let item = trash
        .trash(&at(dir.path(), "note.txt"))
        .expect("trash the file");
    stack.record_trash(item);
    write(&dir.path().join("note.txt"), b"new");

    let error = stack.undo(&ops, &trash).unwrap_err();
    match &error {
        UndoError::Conflict(conflict) => assert_eq!(conflict.kind, "exists"),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert_eq!(fs::read(dir.path().join("note.txt")).unwrap(), b"new");
    assert_eq!(trash.entries().unwrap().len(), 1, "the Trash is untouched");
}

#[test]
fn a_trash_undo_reports_an_entry_emptied_from_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("note.txt"), b"hi");
    let trash = trash_for(dir.path());
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let item = trash
        .trash(&at(dir.path(), "note.txt"))
        .expect("trash the file");
    stack.record_trash(item);
    trash.empty().expect("empty trash");

    let error = stack.undo(&ops, &trash).unwrap_err();
    match &error {
        UndoError::Conflict(conflict) => assert_eq!(conflict.kind, "trash-gone"),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert!(!dir.path().join("note.txt").exists());
}

#[test]
fn a_paste_batch_undoes_as_one_step() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::create_dir(dir.path().join("dst")).unwrap();
    write(&dir.path().join("src/one.txt"), b"1");
    write(&dir.path().join("src/two.txt"), b"2");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();

    for name in ["one.txt", "two.txt"] {
        let source = at(dir.path(), &format!("src/{name}"));
        let destination = ops
            .copy_as(&source, &at(dir.path(), "dst"), name.as_ref())
            .expect("copy");
        stack.record_paste(7, source, destination, false);
    }
    assert_eq!(stack.undo_len(), 1, "one paste is one undo step");
    assert_eq!(stack.undo_label(), Some("Paste \"one.txt\""));
    assert!(dir.path().join("dst/one.txt").exists());
    assert!(dir.path().join("dst/two.txt").exists());

    stack
        .undo(&ops, &trash_for(dir.path()))
        .expect("undo the paste");
    assert!(!dir.path().join("dst/one.txt").exists());
    assert!(!dir.path().join("dst/two.txt").exists());
    assert!(dir.path().join("src/one.txt").exists());
    assert!(dir.path().join("src/two.txt").exists());
    assert_eq!(stack.undo_len(), 0);

    // A different batch token starts a new step.
    let source = at(dir.path(), "src/one.txt");
    let destination = ops
        .copy_as(&source, &at(dir.path(), "dst"), "one.txt".as_ref())
        .expect("copy again");
    stack.record_paste(8, source, destination, false);
    assert_eq!(stack.undo_len(), 1);
}

#[test]
fn a_multi_item_undo_refusal_changes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::create_dir(dir.path().join("dst")).unwrap();
    write(&dir.path().join("src/one.txt"), b"1");
    write(&dir.path().join("src/two.txt"), b"2");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    for name in ["one.txt", "two.txt"] {
        let source = at(dir.path(), &format!("src/{name}"));
        let destination = ops
            .copy_as(&source, &at(dir.path(), "dst"), name.as_ref())
            .expect("copy");
        stack.record_paste(3, source, destination, false);
    }
    // One of the copies vanished: the whole step is refused, and the copy
    // that is still there is not removed either.
    fs::remove_file(dir.path().join("dst/two.txt")).unwrap();
    let error = stack.undo(&ops, &trash_for(dir.path())).unwrap_err();
    assert!(matches!(error, UndoError::Conflict(_)), "got {error:?}");
    assert!(dir.path().join("dst/one.txt").exists());
    assert_eq!(stack.undo_len(), 1);
}

#[test]
fn the_stack_is_bounded_oldest_first_out() {
    assert_eq!(UNDO_LIMIT, 20);
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::with_limit(3);
    let parent = Location::file(dir.path());
    for _ in 0..5 {
        let created = ops.new_folder(&parent).expect("new folder");
        stack.record_new_folder(created);
    }
    assert_eq!(stack.undo_len(), 3);
    for expected in [
        "untitled folder 5",
        "untitled folder 4",
        "untitled folder 3",
    ] {
        assert_eq!(
            stack.undo_label().map(|label| label.to_owned()),
            Some(format!("New Folder \"{expected}\""))
        );
        stack.undo(&ops, &trash_for(dir.path())).expect("undo");
    }
    assert!(matches!(
        stack.undo(&ops, &trash_for(dir.path())),
        Err(UndoError::Empty)
    ));
    // The two oldest steps fell off the stack; their folders remain.
    assert!(dir.path().join("untitled folder").is_dir());
    assert!(dir.path().join("untitled folder 2").is_dir());
}

#[test]
fn a_new_operation_clears_the_redo_tail() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join("a.txt"), b"a");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let renamed = ops
        .rename(&at(dir.path(), "a.txt"), "b.txt".as_ref())
        .expect("rename");
    stack.record_rename(at(dir.path(), "a.txt"), renamed);
    stack.undo(&ops, &trash_for(dir.path())).expect("undo");
    assert_eq!(stack.redo_len(), 1);

    let created = ops.new_folder(&Location::file(dir.path())).expect("folder");
    stack.record_new_folder(created);
    assert_eq!(stack.redo_len(), 0, "a new action makes redo unreachable");
}

#[test]
fn navigation_clears_both_stacks() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let mut stack = UndoStack::new();
    let created = ops.new_folder(&Location::file(dir.path())).expect("folder");
    stack.record_new_folder(created);
    stack.undo(&ops, &trash_for(dir.path())).expect("undo");
    assert_eq!(stack.redo_len(), 1);
    stack.clear();
    assert_eq!(stack.undo_len(), 0);
    assert_eq!(stack.redo_len(), 0);
    assert!(matches!(
        stack.undo(&ops, &trash_for(dir.path())),
        Err(UndoError::Empty)
    ));
}

#[test]
fn a_zero_limit_stack_records_nothing() {
    let mut stack = UndoStack::with_limit(0);
    stack.record_rename(Location::file("/x/a"), Location::file("/x/b"));
    assert_eq!(stack.undo_len(), 0);
    assert_eq!(stack.undo_label(), None);
}

#[test]
fn an_undo_never_touches_a_foreign_scheme() {
    let mut stack = UndoStack::new();
    stack.record_move(
        Location::file("/x/a"),
        Location::parse("trash:///a").unwrap(),
    );
    let ops = StdFsOps::new();
    let error = stack
        .undo(&ops, &trash_for(Path::new("/nonexistent")))
        .unwrap_err();
    assert!(matches!(
        error,
        UndoError::Conflict(_) | UndoError::Failed(OperationError::UnsupportedScheme(_))
    ));
}
