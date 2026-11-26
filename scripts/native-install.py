#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Transactional file install for the Dragonfruit native session (T343).

`scripts/native-session.sh` stages the complete session tree under a temporary
directory, then calls this helper to move it into the install prefix. The
helper validates every destination *before* writing anything, keeps a
rollback copy of every replaced file, and restores the previous install when
any write fails. It also records an install manifest so `native-session.sh
recover` can remove exactly what the install wrote.

Subcommands:

  install   --stage DIR --prefix DIR --manifest-out FILE
            [--manifest-in FILE] [--link NAME=SOURCE ...]
            [--require REL ...] [--force] [--fail-after-write]

  uninstall --prefix DIR --manifest FILE

Only paths below the given prefix are ever touched. Relative paths are
normalised and a `..` component or an absolute manifest path is refused, so a
tampered manifest cannot turn recovery into an arbitrary-file remover.
"""

import argparse
import json
import os
import shutil
import stat
import sys

PROGRAM = "native-install"


def note(message):
    print(f"{PROGRAM}: {message}")


def warn(message):
    print(f"{PROGRAM}: {message}", file=sys.stderr)


class Refused(Exception):
    """A validation failure before any write happened."""


def safe_relative(raw):
    """Return a normalised relative path, or None when it escapes the prefix."""
    if not isinstance(raw, str) or not raw:
        return None
    if os.path.isabs(raw):
        return None
    norm = os.path.normpath(raw)
    if norm in ("", ".") or norm == ".." or norm.startswith("../"):
        return None
    return norm


def under(prefix, rel):
    path = safe_relative(rel)
    if path is None:
        raise Refused(f"refusing unsafe path {rel!r}")
    return os.path.join(prefix, path)


# Every path the native install owns lives under one of these roots. Both the
# install and the uninstall enforce the list, so a tampered user-writable
# manifest cannot turn `recover` (which may run under sudo) into an
# arbitrary-file remover.
INSTALL_ROOTS = (
    "bin/",
    "lib/systemd/user/",
    "share/applications/",
    "share/dbus-1/services/",
    "share/dragonfruit/",
    "share/icons/",
    "share/metainfo/",
    "share/wayland-sessions/",
    "share/xdg-desktop-portal/",
)


def install_path(rel):
    norm = safe_relative(rel)
    if norm is None:
        return None
    for root in INSTALL_ROOTS:
        if norm == root.rstrip("/") or norm.startswith(root):
            return norm
    return None


def same_bytes(left, right):
    try:
        if os.path.getsize(left) != os.path.getsize(right):
            return False
        with open(left, "rb") as a, open(right, "rb") as b:
            while True:
                chunk_a = a.read(1 << 16)
                chunk_b = b.read(1 << 16)
                if chunk_a != chunk_b:
                    return False
                if not chunk_a:
                    return True
    except OSError:
        return False


def collect_stage(stage):
    """Map every staged relative path to (kind, value).

    kind is "file" (value is the permission bits) or "link" (value is the
    link target). Directories are implicit: they are created as needed.
    """
    entries = {}
    for dirpath, dirnames, filenames in os.walk(stage):
        dirnames.sort()
        for name in sorted(filenames):
            path = os.path.join(dirpath, name)
            rel = os.path.relpath(path, stage)
            if os.path.islink(path):
                entries[rel] = ("link", os.readlink(path))
            else:
                mode = stat.S_IMODE(os.stat(path).st_mode)
                entries[rel] = ("file", mode)
    return entries


def read_manifest(path):
    if not path or not os.path.isfile(path):
        return {}
    try:
        with open(path, encoding="utf-8") as handle:
            data = json.load(handle)
    except (OSError, ValueError) as error:
        warn(f"ignoring unreadable install manifest {path}: {error}")
        return {}
    return data if isinstance(data, dict) else {}


def manifest_files(data):
    files = data.get("files") if isinstance(data, dict) else None
    return files if isinstance(files, dict) else {}


def manifest_prefix(data):
    value = data.get("prefix") if isinstance(data, dict) else None
    return os.path.normpath(value) if isinstance(value, str) else None


class Action:
    __slots__ = ("rel", "dest", "kind", "value", "backup", "existed")

    def __init__(self, rel, dest, kind, value, backup=None):
        self.rel = rel
        self.dest = dest
        self.kind = kind
        self.value = value
        self.backup = backup
        self.existed = os.path.lexists(dest)


def plan(prefix, entries, sources, links, owned, force):
    """Validate every destination and return (actions, owned_now, conflicts).

    Nothing is written here. A conflict means the path exists, is not owned by
    a previous Dragonfruit install, and differs from what we would write.
    """
    actions = []
    owned_now = set(owned)
    conflicts = []

    for rel, (kind, value) in entries.items():
        staged = rel
        rel = install_path(rel)
        if rel is None:
            conflicts.append(f"staged path {staged!r} is outside the install roots")
            continue
        dest = under(prefix, rel)
        if os.path.islink(dest):
            actions.append(Action(rel, dest, kind, value))
            owned_now.add(rel)
        elif os.path.isdir(dest):
            conflicts.append(f"{dest} is a directory")
        elif os.path.exists(dest):
            if kind == "file" and same_bytes(dest, sources[rel]):
                # Already the exact file we would write: nothing to do, and no
                # reason to claim ownership of a file that was here first.
                if rel in owned:
                    owned_now.add(rel)
                continue
            if rel in owned:
                actions.append(Action(rel, dest, kind, value))
                owned_now.add(rel)
            elif force:
                actions.append(
                    Action(rel, dest, kind, value, backup=rel + ".pre-dfdev")
                )
                owned_now.add(rel)
            else:
                conflicts.append(
                    f"{dest} exists and was not written by this installer "
                    "(pass --force to replace it; a .pre-dfdev backup is kept)"
                )
        else:
            actions.append(Action(rel, dest, kind, value))
            owned_now.add(rel)

    for name, source in links:
        rel = install_path(os.path.join("bin", name))
        if rel is None:
            conflicts.append(f"refusing unsafe link name {name!r}")
            continue
        dest = under(prefix, rel)
        if os.path.islink(dest):
            owned_now.add(rel)
            if os.readlink(dest) != source:
                actions.append(Action(rel, dest, "link", source))
        elif os.path.isdir(dest):
            conflicts.append(f"{dest} is a directory")
        elif os.path.exists(dest):
            if force:
                actions.append(
                    Action(rel, dest, "link", source, backup=rel + ".pre-dfdev")
                )
                owned_now.add(rel)
            else:
                conflicts.append(
                    f"{dest} exists and is not a dev symlink "
                    "(pass --force to replace it; a .pre-dfdev backup is kept)"
                )
        else:
            actions.append(Action(rel, dest, "link", source))
            owned_now.add(rel)

    return actions, owned_now, conflicts


def ensure_parent(path, created=None, prefix=None):
    parent = os.path.dirname(path)
    if not parent:
        return
    if created is None:
        os.makedirs(parent, exist_ok=True)
        return
    missing = []
    current = parent
    stop = os.path.normpath(prefix) if prefix else None
    while current and os.path.normpath(current) != stop and not os.path.isdir(current):
        missing.append(current)
        current = os.path.dirname(current)
    for directory in reversed(missing):
        try:
            os.mkdir(directory)
            created.add(directory)
        except FileExistsError:
            pass


def persistent_backup(action, prefix):
    """Keep the pre-install file beside it for `recover` to restore."""
    if action.backup is None:
        return None
    backup = under(prefix, action.backup)
    if os.path.lexists(backup):
        # An older backup is the true original; never overwrite it.
        return action.backup
    ensure_parent(backup)
    if os.path.islink(action.dest):
        os.symlink(os.readlink(action.dest), backup)
    else:
        shutil.copy2(action.dest, backup)
    return action.backup


def write_one(action, source, rollback_dir, created_dirs, prefix):
    """Write one staged file/link, keeping a rollback copy when needed."""
    ensure_parent(action.dest, created_dirs, prefix)
    if action.existed:
        backup = os.path.join(rollback_dir, action.rel)
        ensure_parent(backup)
        if os.path.islink(action.dest):
            os.symlink(os.readlink(action.dest), backup)
        else:
            shutil.copy2(action.dest, backup)
    tmp = f"{action.dest}.df-native.{os.getpid()}"
    if os.path.lexists(tmp):
        os.unlink(tmp)
    if action.kind == "link":
        os.symlink(action.value, tmp)
    else:
        shutil.copyfile(source, tmp)
        os.chmod(tmp, action.value)
    os.replace(tmp, action.dest)


def remove_directories(directories, prefix):
    for directory in sorted(directories, key=len, reverse=True):
        try:
            os.rmdir(directory)
        except OSError:
            pass


def rollback(written, prefix, created_dirs):
    errors = 0
    for action, backup in reversed(written):
        try:
            if action.existed:
                if backup is None or not os.path.lexists(backup):
                    warn(f"cannot restore {action.dest}: rollback copy is gone")
                    errors += 1
                    continue
                ensure_parent(action.dest)
                if os.path.lexists(action.dest):
                    if os.path.isdir(action.dest) and not os.path.islink(action.dest):
                        warn(f"cannot restore {action.dest}: a directory appeared")
                        errors += 1
                        continue
                    os.unlink(action.dest)
                os.replace(backup, action.dest)
            else:
                if os.path.lexists(action.dest):
                    if os.path.isdir(action.dest) and not os.path.islink(action.dest):
                        continue
                    os.unlink(action.dest)
        except OSError as error:
            warn(f"cannot restore {action.dest}: {error}")
            errors += 1
    remove_directories(created_dirs, prefix)
    return errors


def verify(prefix, links, required):
    problems = []
    for rel in required:
        dest = under(prefix, rel)
        if not os.path.exists(dest):
            problems.append(f"required file {dest} is missing")
    for name, _source in links:
        dest = under(prefix, os.path.join("bin", name))
        if not os.path.islink(dest):
            problems.append(f"{dest} is not a symlink")
        elif not os.access(dest, os.X_OK):
            problems.append(f"{dest} does not point at an executable")
    return problems


def install(args):
    prefix = os.path.abspath(args.prefix)
    stage = os.path.abspath(args.stage)
    if not os.path.isdir(stage):
        raise Refused(f"staging tree {stage} does not exist")
    os.makedirs(prefix, exist_ok=True)

    entries = collect_stage(stage)
    if not entries:
        raise Refused(f"staging tree {stage} is empty")

    previous = read_manifest(args.manifest_in)
    owned = set()
    if manifest_prefix(previous) in (None, prefix):
        owned = {
            rel for rel in manifest_files(previous) if safe_relative(rel) is not None
        }

    links = []
    for spec in args.link or []:
        name, _, source = spec.partition("=")
        if not name or not source:
            raise Refused(f"malformed --link {spec!r} (expected NAME=SOURCE)")
        links.append((name, source))

    sources = {rel: os.path.join(stage, rel) for rel in entries}
    actions, owned_now, conflicts = plan(
        prefix, entries, sources, links, owned, args.force
    )
    if conflicts:
        warn("refusing to install; the previous install was not touched:")
        for conflict in conflicts:
            warn(f"  {conflict}")
        return 1

    rollback_dir = os.path.join(stage, ".rollback")
    shutil.rmtree(rollback_dir, ignore_errors=True)
    os.makedirs(rollback_dir, exist_ok=True)

    backups = {}
    written = []
    created_dirs = set()
    try:
        for action in actions:
            if action.backup:
                backups[action.rel] = persistent_backup(action, prefix)
            rollback_copy = (
                os.path.join(rollback_dir, action.rel) if action.existed else None
            )
            written.append((action, rollback_copy))
            write_one(
                action, sources.get(action.rel), rollback_dir, created_dirs, prefix
            )

        if args.fail_after_write:
            raise OSError("simulated write failure (--fail-after-write)")

        problems = verify(prefix, links, args.require or [])
        if problems:
            raise OSError("; ".join(problems))
    except (OSError, shutil.Error) as error:
        warn(f"install failed: {error}")
        if rollback(written, prefix, created_dirs) == 0:
            warn("rolled back to the previous install")
            shutil.rmtree(rollback_dir, ignore_errors=True)
            return 1
        warn(f"rollback is incomplete; the staging tree is kept at {stage}")
        return 3

    shutil.rmtree(rollback_dir, ignore_errors=True)

    files = {}
    directories = set()
    if manifest_prefix(previous) in (None, prefix):
        for rel, info in manifest_files(previous).items():
            if safe_relative(rel) is None:
                continue
            files[rel] = info if isinstance(info, dict) else {}
        for directory in previous.get("directories", []) or []:
            if isinstance(directory, str) and safe_relative(directory):
                directories.add(directory)
    for rel in owned_now:
        entry = dict(files.get(rel, {}))
        if backups.get(rel):
            entry["backup"] = backups[rel]
        files[rel] = entry
    for directory in created_dirs:
        rel = os.path.relpath(directory, prefix)
        if safe_relative(rel):
            directories.add(rel)

    payload = {
        "version": 1,
        "prefix": prefix,
        "files": files,
        "directories": sorted(directories),
    }
    out = args.manifest_out
    ensure_parent(out)
    tmp = f"{out}.tmp.{os.getpid()}"
    with open(tmp, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")
    # The caller (the user, not root) moves this into the user state dir, so
    # keep it readable after a sudo-run install.
    os.chmod(tmp, 0o644)
    os.replace(tmp, out)

    note(f"installed {len(actions)} path(s) under {prefix}")
    return 0


def uninstall(args):
    prefix = os.path.abspath(args.prefix)
    data = read_manifest(args.manifest)
    recorded = manifest_prefix(data)
    if recorded is not None and recorded != prefix:
        warn(
            f"{args.manifest} records prefix {recorded}, not {prefix}; refusing "
            "to remove a different tree"
        )
        return 1
    manifest = manifest_files(data)
    if not manifest:
        note(f"no installed files recorded in {args.manifest}")
        return 0
    removed = 0
    restored = 0
    errors = 0
    for raw_rel, _info in manifest.items():
        rel = install_path(raw_rel)
        if rel is None:
            warn(f"refusing manifest path outside the install roots: {raw_rel!r}")
            errors += 1
            continue
        dest = under(prefix, rel)
        # The backup name is derived, never trusted from the manifest: a
        # forged entry must not be able to move an arbitrary file.
        backup_rel = rel + ".pre-dfdev"
        backup = under(prefix, backup_rel)
        if os.path.lexists(backup):
            try:
                ensure_parent(dest)
                if os.path.isdir(dest) and not os.path.islink(dest):
                    warn(f"cannot restore {dest}: it is a directory")
                    errors += 1
                    continue
                if os.path.lexists(dest):
                    os.unlink(dest)
                os.replace(backup, dest)
                restored += 1
                continue
            except OSError as error:
                warn(f"cannot restore {dest}: {error}")
                errors += 1
                continue
        if not os.path.lexists(dest):
            continue
        try:
            if os.path.isdir(dest) and not os.path.islink(dest):
                continue
            os.unlink(dest)
            removed += 1
        except OSError as error:
            warn(f"cannot remove {dest}: {error}")
            errors += 1
    for raw_dir in data.get("directories", []) or []:
        directory = install_path(raw_dir) if isinstance(raw_dir, str) else None
        if directory is None:
            continue
        try:
            os.rmdir(under(prefix, directory))
        except OSError:
            pass
    note(f"removed {removed} installed path(s), restored {restored} backup(s)")
    return 1 if errors else 0


def main(argv):
    parser = argparse.ArgumentParser(prog=PROGRAM)
    sub = parser.add_subparsers(dest="command", required=True)

    install_parser = sub.add_parser("install")
    install_parser.add_argument("--stage", required=True)
    install_parser.add_argument("--prefix", required=True)
    install_parser.add_argument("--manifest-in", default="")
    install_parser.add_argument("--manifest-out", required=True)
    install_parser.add_argument("--link", action="append", default=[])
    install_parser.add_argument("--require", action="append", default=[])
    install_parser.add_argument("--force", action="store_true")
    install_parser.add_argument("--fail-after-write", action="store_true")

    uninstall_parser = sub.add_parser("uninstall")
    uninstall_parser.add_argument("--prefix", required=True)
    uninstall_parser.add_argument("--manifest", required=True)

    args = parser.parse_args(argv)
    try:
        if args.command == "install":
            return install(args)
        return uninstall(args)
    except Refused as error:
        warn(str(error))
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
