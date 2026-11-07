#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Asset provenance gate (T220).

Validates ``docs/design/asset-manifest.json`` and proves that every file under
``assets/`` is covered by a shipped entry with a source, license, and license
text. Categories with no shipped files (sound, illustration, runtime provider)
stay explicit so the first new asset must add a provenance row in the same
change.

Run directly:

    python3 scripts/check-asset-manifest.py

or through ``make check-assets``.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_MANIFEST = ROOT / "docs" / "design" / "asset-manifest.json"
CATEGORIES = {"icon", "font", "sound", "cursor", "wallpaper", "illustration"}
STATUSES = {"shipped", "none-shipped", "runtime-provider"}


def load_manifest(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema") != 1:
        raise ValueError(f"{path}: unsupported schema {data.get('schema')!r}")
    if not isinstance(data.get("assets"), list):
        raise ValueError(f"{path}: assets must be a list")
    categories = data.get("categories")
    if not isinstance(categories, dict) or set(categories) != CATEGORIES:
        raise ValueError(
            f"{path}: categories must be exactly {sorted(CATEGORIES)}"
        )
    return data


def expand_entry(root: Path, entry: dict) -> tuple[set[Path], list[str]]:
    """Return the files matched by a shipped entry and validation errors."""
    errors: list[str] = []
    matched: set[Path] = set()
    raw_path = entry.get("path")
    if not raw_path:
        return matched, [f"{entry.get('id')}: shipped entry has no path"]
    path = root / raw_path
    if not path.exists():
        return matched, [f"{entry.get('id')}: path does not exist: {raw_path}"]
    if path.is_file():
        matched.add(path)
        return matched, errors

    include = entry.get("include") or []
    if not include:
        return matched, [f"{entry.get('id')}: directory entry needs include patterns"]
    for pattern in include:
        for found in path.glob(pattern):
            if found.is_file() or found.is_symlink():
                matched.add(found)
    if not matched:
        errors.append(f"{entry.get('id')}: no files matched under {raw_path}")
    return matched, errors


def check(root: Path, manifest_path: Path) -> tuple[list[str], dict]:
    errors: list[str] = []
    try:
        data = load_manifest(manifest_path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return [str(exc)], {}

    ids: set[str] = set()
    matched_all: set[Path] = set()
    shipped_files = 0
    counts = {category: 0 for category in sorted(CATEGORIES)}

    for entry in data["assets"]:
        if not isinstance(entry, dict):
            errors.append("asset entry is not an object")
            continue
        asset_id = entry.get("id")
        if not asset_id:
            errors.append("asset entry has no id")
            continue
        if asset_id in ids:
            errors.append(f"{asset_id}: duplicate id")
        ids.add(asset_id)

        category = entry.get("category")
        if category not in CATEGORIES:
            errors.append(f"{asset_id}: unknown category {category!r}")
            continue
        counts[category] += 1

        for field in ("name", "status", "source", "license", "notice"):
            if not entry.get(field):
                errors.append(f"{asset_id}: missing {field}")
        status = entry.get("status")
        if status not in STATUSES:
            errors.append(f"{asset_id}: unknown status {status!r}")
            continue

        if status == "shipped":
            matched, entry_errors = expand_entry(root, entry)
            errors.extend(entry_errors)
            matched_all |= matched
            shipped_files += len(matched)
            license_file = entry.get("license_file")
            if not license_file:
                errors.append(f"{asset_id}: shipped entry has no license_file")
            elif not (root / license_file).is_file():
                errors.append(f"{asset_id}: license_file does not exist: {license_file}")
        else:
            if entry.get("path") is not None:
                errors.append(f"{asset_id}: {status} entry must not have a path")
            if entry.get("include"):
                errors.append(f"{asset_id}: {status} entry must not have include patterns")
            if not entry.get("owner"):
                errors.append(f"{asset_id}: {status} entry needs an owner")
            if not entry.get("policy"):
                errors.append(f"{asset_id}: {status} entry needs a policy")

    assets_root = root / "assets"
    if assets_root.is_dir():
        for path in sorted(assets_root.rglob("*")):
            if not (path.is_file() or path.is_symlink()):
                continue
            if path not in matched_all:
                errors.append(
                    f"unlisted asset: {path.relative_to(root)} "
                    "(add it to docs/design/asset-manifest.json)"
                )

    summary = {
        "entries": len(ids),
        "shipped_files": shipped_files,
        "categories": counts,
    }
    return errors, summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root")
    parser.add_argument("--manifest", type=Path, default=None, help="asset manifest JSON")
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    manifest = args.manifest or (root / "docs" / "design" / "asset-manifest.json")
    errors, summary = check(root, manifest)
    if errors:
        print(f"check-asset-manifest: {len(errors)} error(s):")
        for error in errors:
            print(f"  ERROR: {error}")
        return 1
    if not args.quiet:
        counts = ", ".join(
            f"{name}={count}" for name, count in summary["categories"].items()
        )
        print(
            f"check-asset-manifest: OK — {summary['shipped_files']} shipped files "
            f"across {summary['entries']} entries ({counts})"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
