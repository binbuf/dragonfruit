// SPDX-License-Identifier: MIT
//! T-10.2a acceptance: the operation matrix — rename, new folder, copy, move,
//! and delete — runs against a real temp tree, and every failure is reported
//! as a typed [`OperationError`]. Headless: no display, no GIO, no daemon.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use dragonfruit_files_core::{FileOps, Location, OperationError, StdFsOps};

fn file(dir: &Path, name: &str) -> Location {
    Location::file(dir.join(name))
}

fn path(location: &Location) -> std::path::PathBuf {
    location.to_file_path().expect("file location")
}

#[test]
fn rename_a_file_keeps_its_contents_and_replaces_its_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::write(dir.path().join("old.txt"), b"payload").expect("write");

    let moved = ops
        .rename(&file(dir.path(), "old.txt"), OsStr::new("new.txt"))
        .expect("rename");
    assert_eq!(moved, file(dir.path(), "new.txt"));
    assert_eq!(path(&moved), dir.path().join("new.txt"));
    assert!(!dir.path().join("old.txt").exists());
    assert_eq!(fs::read(dir.path().join("new.txt")).unwrap(), b"payload");
    assert!(ops.exists(&moved));
}

#[test]
fn rename_reports_an_existing_target_and_a_missing_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::write(dir.path().join("a.txt"), b"a").expect("write a");
    fs::write(dir.path().join("b.txt"), b"b").expect("write b");

    let error = ops
        .rename(&file(dir.path(), "a.txt"), OsStr::new("b.txt"))
        .expect_err("target exists");
    assert!(
        matches!(error, OperationError::AlreadyExists(_)),
        "got {error:?}"
    );
    // Nothing was clobbered.
    assert_eq!(fs::read(dir.path().join("b.txt")).unwrap(), b"b");
    assert_eq!(fs::read(dir.path().join("a.txt")).unwrap(), b"a");

    let error = ops
        .rename(&file(dir.path(), "missing.txt"), OsStr::new("x"))
        .expect_err("source missing");
    assert!(
        matches!(error, OperationError::NotFound(_)),
        "got {error:?}"
    );
}

#[test]
fn rename_rejects_an_empty_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::write(dir.path().join("a.txt"), b"a").expect("write");
    let error = ops
        .rename(&file(dir.path(), "a.txt"), OsStr::new(""))
        .expect_err("empty name");
    assert!(
        matches!(error, OperationError::InvalidName(_)),
        "got {error:?}"
    );
    assert!(dir.path().join("a.txt").exists());
}

#[test]
fn new_folder_generates_numbered_untitled_names() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();

    let first = ops.new_folder(&Location::file(dir.path())).expect("first");
    let second = ops.new_folder(&Location::file(dir.path())).expect("second");
    let third = ops.new_folder(&Location::file(dir.path())).expect("third");

    assert_eq!(path(&first).file_name().unwrap(), "untitled folder");
    assert_eq!(path(&second).file_name().unwrap(), "untitled folder 2");
    assert_eq!(path(&third).file_name().unwrap(), "untitled folder 3");
    for folder in [&first, &second, &third] {
        assert!(
            path(folder).is_dir(),
            "{} is not a directory",
            path(folder).display()
        );
    }
}

#[test]
fn create_dir_reports_an_existing_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let parent = Location::file(dir.path());
    ops.create_dir(&parent, OsStr::new("docs")).expect("first");
    let error = ops
        .create_dir(&parent, OsStr::new("docs"))
        .expect_err("duplicate");
    assert!(
        matches!(error, OperationError::AlreadyExists(_)),
        "got {error:?}"
    );
}

#[test]
fn copy_a_file_into_a_directory_preserves_bytes_and_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let dest = dir.path().join("dest");
    fs::create_dir(&dest).expect("mkdir dest");
    fs::write(dir.path().join("note.txt"), b"hello").expect("write");

    let copied = ops
        .copy(&file(dir.path(), "note.txt"), &Location::file(&dest))
        .expect("copy");
    assert_eq!(path(&copied), dest.join("note.txt"));
    assert_eq!(fs::read(dest.join("note.txt")).unwrap(), b"hello");
    // The source is untouched: copy never moves.
    assert_eq!(fs::read(dir.path().join("note.txt")).unwrap(), b"hello");
}

#[test]
fn copy_a_directory_recursively_copies_nested_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let source = dir.path().join("tree");
    fs::create_dir_all(source.join("nested/deeper")).expect("mkdir tree");
    fs::write(source.join("top.txt"), b"top").expect("write top");
    fs::write(source.join("nested/deeper/leaf.txt"), b"leaf").expect("write leaf");
    let dest = dir.path().join("dest");
    fs::create_dir(&dest).expect("mkdir dest");

    let copied = ops
        .copy(&Location::file(&source), &Location::file(&dest))
        .expect("copy tree");
    assert_eq!(path(&copied), dest.join("tree"));
    assert_eq!(fs::read(dest.join("tree/top.txt")).unwrap(), b"top");
    assert_eq!(
        fs::read(dest.join("tree/nested/deeper/leaf.txt")).unwrap(),
        b"leaf"
    );
    assert!(source.exists(), "copy must leave the source in place");
}

#[test]
#[cfg(unix)]
fn copy_recreates_a_symlink_without_following_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::write(dir.path().join("target.txt"), b"target").expect("write target");
    std::os::unix::fs::symlink("target.txt", dir.path().join("link")).expect("symlink");
    let dest = dir.path().join("dest");
    fs::create_dir(&dest).expect("mkdir dest");

    let copied = ops
        .copy(&file(dir.path(), "link"), &Location::file(&dest))
        .expect("copy link");
    let metadata = fs::symlink_metadata(path(&copied)).expect("lstat");
    assert!(metadata.file_type().is_symlink(), "copy followed the link");
    assert_eq!(
        fs::read_link(path(&copied)).unwrap(),
        std::path::PathBuf::from("target.txt")
    );
    // The target itself was not copied along.
    assert!(!dest.join("target.txt").exists());
}

#[test]
#[cfg(unix)]
fn copy_preserves_a_non_utf8_file_name() {
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let raw = OsStr::from_bytes(b"odd\xffname");
    fs::write(dir.path().join(raw), b"x").expect("write odd name");
    let dest = dir.path().join("dest");
    fs::create_dir(&dest).expect("mkdir dest");

    let copied = ops
        .copy(
            &Location::file(dir.path()).child(raw),
            &Location::file(&dest),
        )
        .expect("copy odd name");
    let name = path(&copied).file_name().unwrap().to_os_string();
    assert_eq!(name.as_bytes(), b"odd\xffname");
}

#[test]
fn copy_reports_a_missing_source_a_bad_destination_and_a_conflict() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let dest = dir.path().join("dest");
    fs::create_dir(&dest).expect("mkdir dest");

    let error = ops
        .copy(&file(dir.path(), "missing"), &Location::file(&dest))
        .expect_err("missing source");
    assert!(
        matches!(error, OperationError::NotFound(_)),
        "got {error:?}"
    );

    fs::write(dir.path().join("note.txt"), b"a").expect("write");
    fs::write(dir.path().join("plain"), b"b").expect("write plain");
    let error = ops
        .copy(&file(dir.path(), "note.txt"), &file(dir.path(), "plain"))
        .expect_err("destination is a file");
    assert!(
        matches!(error, OperationError::NotADirectory(_)),
        "got {error:?}"
    );

    ops.copy(&file(dir.path(), "note.txt"), &Location::file(&dest))
        .expect("first copy");
    let error = ops
        .copy(&file(dir.path(), "note.txt"), &Location::file(&dest))
        .expect_err("conflict");
    assert!(
        matches!(error, OperationError::AlreadyExists(_)),
        "got {error:?}"
    );
}

#[test]
fn copy_refuses_to_place_a_directory_inside_itself() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let source = dir.path().join("tree");
    fs::create_dir(&source).expect("mkdir tree");
    let source_location = Location::file(&source);

    let error = ops
        .copy(&source_location, &source_location)
        .expect_err("self copy");
    assert!(
        matches!(error, OperationError::InvalidName(_)),
        "got {error:?}"
    );
    assert!(source.is_dir());
}

#[test]
fn move_relocates_a_file_and_a_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let dest = dir.path().join("dest");
    fs::create_dir(&dest).expect("mkdir dest");
    fs::write(dir.path().join("note.txt"), b"hello").expect("write");
    fs::create_dir(dir.path().join("tree")).expect("mkdir tree");
    fs::write(dir.path().join("tree/leaf.txt"), b"leaf").expect("write leaf");

    let moved_file = ops
        .move_to(&file(dir.path(), "note.txt"), &Location::file(&dest))
        .expect("move file");
    assert_eq!(path(&moved_file), dest.join("note.txt"));
    assert!(!dir.path().join("note.txt").exists());
    assert_eq!(fs::read(dest.join("note.txt")).unwrap(), b"hello");

    let moved_tree = ops
        .move_to(&file(dir.path(), "tree"), &Location::file(&dest))
        .expect("move tree");
    assert_eq!(path(&moved_tree), dest.join("tree"));
    assert!(!dir.path().join("tree").exists());
    assert_eq!(fs::read(dest.join("tree/leaf.txt")).unwrap(), b"leaf");
}

#[test]
fn move_refuses_into_itself_and_reports_a_conflict() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let source = dir.path().join("tree");
    fs::create_dir(&source).expect("mkdir tree");
    let source_location = Location::file(&source);
    let error = ops
        .move_to(&source_location, &source_location)
        .expect_err("self move");
    assert!(
        matches!(error, OperationError::InvalidName(_)),
        "got {error:?}"
    );

    let dest = dir.path().join("dest");
    fs::create_dir(&dest).expect("mkdir dest");
    fs::write(dir.path().join("note.txt"), b"a").expect("write");
    fs::write(dest.join("note.txt"), b"b").expect("write dest");
    let error = ops
        .move_to(&file(dir.path(), "note.txt"), &Location::file(&dest))
        .expect_err("conflict");
    assert!(
        matches!(error, OperationError::AlreadyExists(_)),
        "got {error:?}"
    );
}

#[test]
fn delete_removes_a_file_and_a_directory_tree_permanently() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    fs::write(dir.path().join("note.txt"), b"x").expect("write");
    fs::create_dir_all(dir.path().join("tree/nested")).expect("mkdir tree");
    fs::write(dir.path().join("tree/nested/leaf.txt"), b"y").expect("write leaf");

    ops.delete(&file(dir.path(), "note.txt"))
        .expect("delete file");
    assert!(!dir.path().join("note.txt").exists());
    ops.delete(&file(dir.path(), "tree")).expect("delete tree");
    assert!(!dir.path().join("tree").exists());
}

#[test]
fn delete_reports_a_missing_target() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let error = ops
        .delete(&file(dir.path(), "missing"))
        .expect_err("missing");
    assert!(
        matches!(error, OperationError::NotFound(_)),
        "got {error:?}"
    );
}

#[test]
fn exists_counts_a_broken_symlink() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    assert!(!ops.exists(&file(dir.path(), "nothing")));
    fs::write(dir.path().join("thing"), b"x").expect("write");
    assert!(ops.exists(&file(dir.path(), "thing")));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("gone", dir.path().join("dangling")).expect("symlink");
        assert!(ops.exists(&file(dir.path(), "dangling")));
    }
}

#[test]
fn foreign_schemes_are_refused_by_every_operation() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ops = StdFsOps::new();
    let trash = Location::parse("trash:///").expect("parses");
    let local = Location::file(dir.path());
    let unsupported = |error: OperationError| {
        assert!(
            matches!(error, OperationError::UnsupportedScheme(ref scheme) if scheme == "trash"),
            "got {error:?}"
        );
    };

    unsupported(ops.new_folder(&trash).expect_err("new folder"));
    unsupported(
        ops.create_dir(&trash, OsStr::new("x"))
            .expect_err("create dir"),
    );
    unsupported(ops.rename(&trash, OsStr::new("x")).expect_err("rename"));
    unsupported(ops.copy(&trash, &local).expect_err("copy"));
    unsupported(ops.move_to(&trash, &local).expect_err("move"));
    unsupported(ops.delete(&trash).expect_err("delete"));
    assert!(!ops.exists(&trash));
}
