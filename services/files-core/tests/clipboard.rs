// SPDX-License-Identifier: GPL-3.0-or-later
//! T330 acceptance: the clipboard model and the paste conflict policies run
//! against a real temp tree. Headless: no display, no GIO, no daemon.

use std::fs;
use std::path::Path;

use dragonfruit_files_core::{
    Clipboard, ClipboardItem, ClipboardMode, ConflictPolicy, Location, NodeKind, PasteStatus,
    StdFsOps,
};

fn file(dir: &Path, name: &str) -> Location {
    Location::file(dir.join(name))
}

fn path(location: &Location) -> std::path::PathBuf {
    location.to_file_path().expect("file location")
}

fn item(location: Location, kind: NodeKind) -> ClipboardItem {
    ClipboardItem::new(location, kind)
}

fn paste(
    clipboard: &Clipboard,
    ops: &StdFsOps,
    target: &Location,
    policy: ConflictPolicy,
) -> dragonfruit_files_core::PasteOutcome {
    clipboard.paste_with(ops, target, policy, |_| {})
}

#[test]
fn copy_pastes_a_file_and_a_folder_and_leaves_the_sources() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    fs::write(dir.path().join("source/report.txt"), b"report").expect("file");
    fs::create_dir(dir.path().join("source/Folder")).expect("folder");
    fs::write(dir.path().join("source/Folder/leaf.txt"), b"leaf").expect("leaf");

    let clipboard = Clipboard::copy([
        item(
            file(&dir.path().join("source"), "report.txt"),
            NodeKind::File,
        ),
        item(
            file(&dir.path().join("source"), "Folder"),
            NodeKind::Directory,
        ),
    ]);
    let target = Location::file(dir.path().join("dest"));
    let outcome = paste(&clipboard, &ops, &target, ConflictPolicy::KeepBoth);

    assert_eq!(outcome.copied, 2);
    assert_eq!(outcome.failed, 0);
    assert_eq!(
        fs::read(dir.path().join("dest/report.txt")).unwrap(),
        b"report"
    );
    assert_eq!(
        fs::read(dir.path().join("dest/Folder/leaf.txt")).unwrap(),
        b"leaf"
    );
    // Copy never moves.
    assert!(dir.path().join("source/report.txt").exists());
    assert!(dir.path().join("source/Folder/leaf.txt").exists());
}

#[test]
fn keep_both_numbers_every_conflicting_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    fs::write(dir.path().join("source/report.txt"), b"new").expect("source file");
    fs::write(dir.path().join("dest/report.txt"), b"old").expect("dest file");
    fs::write(dir.path().join("dest/report.txt 2"), b"older").expect("dest file 2");
    fs::create_dir(dir.path().join("source/Folder")).expect("source folder");
    fs::create_dir(dir.path().join("dest/Folder")).expect("dest folder");

    let clipboard = Clipboard::copy([
        item(
            file(&dir.path().join("source"), "report.txt"),
            NodeKind::File,
        ),
        item(
            file(&dir.path().join("source"), "Folder"),
            NodeKind::Directory,
        ),
    ]);
    let outcome = paste(
        &clipboard,
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::KeepBoth,
    );

    assert_eq!(outcome.copied, 2);
    // The one generated-name helper appends the counter to the whole name
    // (`report.txt`, `report.txt 2`, …), exactly as New Folder and Duplicate.
    assert_eq!(outcome.items[0].name, "report.txt 3");
    assert_eq!(outcome.items[1].name, "Folder 2");
    assert_eq!(
        fs::read(dir.path().join("dest/report.txt")).unwrap(),
        b"old"
    );
    assert_eq!(
        fs::read(dir.path().join("dest/report.txt 3")).unwrap(),
        b"new"
    );
    assert!(dir.path().join("dest/Folder 2").is_dir());
}

#[test]
fn skip_leaves_the_destination_and_reports_the_item() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    fs::write(dir.path().join("source/report.txt"), b"new").expect("source");
    fs::write(dir.path().join("dest/report.txt"), b"old").expect("dest");

    let clipboard = Clipboard::copy([item(
        file(&dir.path().join("source"), "report.txt"),
        NodeKind::File,
    )]);
    let outcome = paste(
        &clipboard,
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::Skip,
    );

    assert_eq!(outcome.skipped, 1);
    assert_eq!(outcome.items[0].status, PasteStatus::Skipped);
    assert_eq!(
        fs::read(dir.path().join("dest/report.txt")).unwrap(),
        b"old"
    );
    assert_eq!(
        fs::read(dir.path().join("source/report.txt")).unwrap(),
        b"new"
    );
}

#[test]
fn replace_removes_the_existing_destination() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    fs::write(dir.path().join("source/report.txt"), b"new").expect("source");
    fs::write(dir.path().join("dest/report.txt"), b"old").expect("dest");

    let clipboard = Clipboard::copy([item(
        file(&dir.path().join("source"), "report.txt"),
        NodeKind::File,
    )]);
    let outcome = paste(
        &clipboard,
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::Replace,
    );

    assert_eq!(outcome.replaced, 1);
    assert_eq!(outcome.items[0].status, PasteStatus::Replaced);
    assert_eq!(
        fs::read(dir.path().join("dest/report.txt")).unwrap(),
        b"new"
    );
    assert!(dir.path().join("source/report.txt").exists());
}

#[test]
fn cut_moves_the_source_and_a_same_folder_cut_is_a_skip() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    fs::write(dir.path().join("source/note.txt"), b"note").expect("file");
    fs::write(dir.path().join("source/stay.txt"), b"stay").expect("file");

    let clipboard = Clipboard::cut([
        item(file(&dir.path().join("source"), "note.txt"), NodeKind::File),
        item(file(&dir.path().join("source"), "stay.txt"), NodeKind::File),
    ]);
    // Paste one into the destination, then the other into its own folder.
    let outcome = paste(
        &clipboard,
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::KeepBoth,
    );
    assert_eq!(outcome.moved, 2);
    assert!(!dir.path().join("source/note.txt").exists());
    assert!(!dir.path().join("source/stay.txt").exists());
    assert_eq!(fs::read(dir.path().join("dest/note.txt")).unwrap(), b"note");

    let same = paste(
        &Clipboard::cut([item(
            file(&dir.path().join("dest"), "note.txt"),
            NodeKind::File,
        )]),
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::KeepBoth,
    );
    assert_eq!(same.skipped, 1);
    assert_eq!(same.items[0].status, PasteStatus::Skipped);
    assert!(dir.path().join("dest/note.txt").exists());
}

#[test]
fn a_paste_reports_each_item_through_the_progress_callback() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(dir.path().join("source").join(name), name.as_bytes()).expect("file");
    }
    let clipboard = Clipboard::copy(
        ["a.txt", "b.txt", "c.txt"]
            .into_iter()
            .map(|name| item(file(&dir.path().join("source"), name), NodeKind::File)),
    );

    let mut seen = Vec::new();
    let outcome = clipboard.paste_with(
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::KeepBoth,
        |item| seen.push(item.name.to_string_lossy().into_owned()),
    );

    assert_eq!(seen, vec!["a.txt", "b.txt", "c.txt"]);
    assert_eq!(outcome.items.len(), 3);
    assert_eq!(outcome.copied, 3);
    assert_eq!(outcome.succeeded(), 3);
}

#[test]
fn one_failed_item_does_not_abort_the_rest_of_the_batch() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    fs::write(dir.path().join("source/ok.txt"), b"ok").expect("file");

    let clipboard = Clipboard::copy([
        item(
            file(&dir.path().join("source"), "missing.txt"),
            NodeKind::File,
        ),
        item(file(&dir.path().join("source"), "ok.txt"), NodeKind::File),
    ]);
    let outcome = paste(
        &clipboard,
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::KeepBoth,
    );

    assert_eq!(outcome.failed, 1);
    assert_eq!(outcome.copied, 1);
    assert_eq!(outcome.items[0].status, PasteStatus::Failed);
    assert_eq!(outcome.items[1].status, PasteStatus::Copied);
    assert!(dir.path().join("dest/ok.txt").exists());
}

#[test]
#[cfg(unix)]
fn copying_a_symlink_keeps_it_a_link() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::create_dir(dir.path().join("source")).expect("source");
    fs::create_dir(dir.path().join("dest")).expect("dest");
    fs::write(dir.path().join("target.txt"), b"target").expect("target");
    std::os::unix::fs::symlink("../target.txt", dir.path().join("source/link")).expect("symlink");

    let clipboard = Clipboard::copy([item(
        file(&dir.path().join("source"), "link"),
        NodeKind::Symlink,
    )]);
    let outcome = paste(
        &clipboard,
        &ops,
        &Location::file(dir.path().join("dest")),
        ConflictPolicy::KeepBoth,
    );

    assert_eq!(outcome.copied, 1);
    let metadata = fs::symlink_metadata(path(&outcome.items[0].destination)).expect("lstat");
    assert!(metadata.file_type().is_symlink(), "copy followed the link");
    assert!(!dir.path().join("dest/target.txt").exists());
}

#[test]
fn clipboard_mode_round_trips() {
    assert_eq!(ClipboardMode::parse("copy"), Some(ClipboardMode::Copy));
    assert_eq!(ClipboardMode::parse("cut"), Some(ClipboardMode::Cut));
}
