#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Validate the T219 experience schema, manifest, and thresholds.

The contract has three checked-in files:

* ``docs/captures/experience-schema.json`` — the JSON Schema for a manifest
  entry (surface id, state, axes, evidence class, trigger, acceptance metric,
  and verification).
* ``docs/captures/experience-manifest.json`` — the inventory rows and the
  capture/test/OPEN mapping for every surface state.
* ``docs/captures/experience-thresholds.json`` — the per-surface material,
  geometry, typography, motion, latency, contrast, and performance
  thresholds. Thresholds may be tightened, never silently loosened.

Commands:

    python3 scripts/t219-experience-schema.py --check
    python3 scripts/t219-experience-schema.py --coverage
    python3 scripts/t219-experience-schema.py --list-open
    python3 scripts/t219-experience-schema.py --validate-artifact PATH
    python3 scripts/t219-experience-schema.py --json

``--check`` is the hardware-independent gate used by ``make gates``. It
validates the manifest against the JSON Schema, resolves every axis set and
threshold profile, verifies captured file hashes, checks test paths, requires
OPEN rows to name a reason, a rerun command, and an owner, and prints the
captured/verified/OPEN coverage summary by surface.

The script deliberately uses only the Python standard library: the ordinary
CI suite must not require a Mac, a GPU, or a third-party JSON-Schema package.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import OrderedDict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCHEMA = REPO / "docs" / "captures" / "experience-schema.json"
MANIFEST = REPO / "docs" / "captures" / "experience-manifest.json"
THRESHOLDS = REPO / "docs" / "captures" / "experience-thresholds.json"
DIMENSIONS = (
    "material",
    "geometry",
    "typography",
    "motion",
    "latency",
    "contrast",
    "performance",
)
AXIS_KEYS = (
    "appearance",
    "scale",
    "refresh",
    "reduceMotion",
    "reduceTransparency",
    "increaseContrast",
    "display",
    "locale",
)
STATUSES = ("captured", "verified", "open")


class ValidationError(Exception):
    pass


def load_json(path: Path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)
    except FileNotFoundError as exc:
        raise ValidationError(f"{path}: missing") from exc
    except json.JSONDecodeError as exc:
        raise ValidationError(f"{path}: invalid JSON: {exc}") from exc


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


# ---------------------------------------------------------------------------
# Minimal JSON-Schema validator for the keywords this contract uses.
# ---------------------------------------------------------------------------

def _type_matches(value, expected: str) -> bool:
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "boolean":
        return isinstance(value, bool)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if expected == "null":
        return value is None
    return True


def _resolve_ref(schema: dict, root: dict) -> dict:
    ref = schema.get("$ref")
    if not ref:
        return schema
    if not ref.startswith("#/"):
        raise ValidationError(f"unsupported $ref {ref!r}")
    node = root
    for part in ref[2:].split("/"):
        node = node[part]
    return node


def validate_schema(instance, schema: dict, root: dict, path: str = "$") -> list[str]:
    """Validate ``instance`` against the small JSON-Schema subset we use."""
    schema = _resolve_ref(schema, root)
    errors: list[str] = []

    if "const" in schema and instance != schema["const"]:
        errors.append(f"{path}: expected {schema['const']!r}, got {instance!r}")
    if "enum" in schema and instance not in schema["enum"]:
        errors.append(f"{path}: {instance!r} is not one of {schema['enum']!r}")

    expected = schema.get("type")
    if expected is not None:
        allowed = expected if isinstance(expected, list) else [expected]
        if not any(_type_matches(instance, item) for item in allowed):
            errors.append(f"{path}: expected type {expected!r}, got {type(instance).__name__}")
            return errors

    if isinstance(instance, dict):
        for key in schema.get("required", []):
            if key not in instance:
                errors.append(f"{path}: missing required key {key!r}")
        properties = schema.get("properties", {})
        additional = schema.get("additionalProperties", True)
        for key, value in instance.items():
            if key in properties:
                errors.extend(validate_schema(value, properties[key], root, f"{path}.{key}"))
            elif additional is False:
                errors.append(f"{path}: unexpected key {key!r}")
            elif isinstance(additional, dict):
                errors.extend(validate_schema(value, additional, root, f"{path}.{key}"))
        if "minProperties" in schema and len(instance) < schema["minProperties"]:
            errors.append(f"{path}: needs at least {schema['minProperties']} properties")

    if isinstance(instance, list):
        if "minItems" in schema and len(instance) < schema["minItems"]:
            errors.append(f"{path}: needs at least {schema['minItems']} items")
        if schema.get("uniqueItems"):
            seen = set()
            for item in instance:
                key = json.dumps(item, sort_keys=True)
                if key in seen:
                    errors.append(f"{path}: duplicate item {item!r}")
                seen.add(key)
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for index, item in enumerate(instance):
                errors.extend(validate_schema(item, item_schema, root, f"{path}[{index}]"))

    if isinstance(instance, str):
        if "minLength" in schema and len(instance) < schema["minLength"]:
            errors.append(f"{path}: shorter than {schema['minLength']}")
        if "pattern" in schema:
            import re
            if re.search(schema["pattern"], instance) is None:
                errors.append(f"{path}: {instance!r} does not match {schema['pattern']!r}")

    if "allOf" in schema:
        for index, sub in enumerate(schema["allOf"]):
            errors.extend(validate_schema(instance, sub, root, f"{path}.allOf[{index}]"))

    if "oneOf" in schema:
        matches = 0
        sub_errors = []
        for sub in schema["oneOf"]:
            result = validate_schema(instance, sub, root, path)
            if not result:
                matches += 1
            sub_errors.append(result)
        if matches != 1:
            errors.append(f"{path}: expected exactly one oneOf match, got {matches}")

    if "if" in schema:
        condition_errors = validate_schema(instance, schema["if"], root, path)
        if not condition_errors and "then" in schema:
            errors.extend(validate_schema(instance, schema["then"], root, path))

    return errors


# ---------------------------------------------------------------------------
# Contract checks.
# ---------------------------------------------------------------------------

def deep_merge(base: dict, overrides: dict) -> dict:
    import copy
    result = copy.deepcopy(base)
    for key, value in overrides.items():
        if isinstance(value, dict) and isinstance(result.get(key), dict):
            result[key] = deep_merge(result[key], value)
        else:
            result[key] = copy.deepcopy(value)
    return result


def resolve_threshold(thresholds: dict, surface_id: str) -> dict:
    surfaces = thresholds.get("surfaces", {})
    if surface_id not in surfaces:
        raise ValidationError(f"thresholds: no entry for surface {surface_id!r}")
    entry = surfaces[surface_id]
    profile_name = entry.get("profile")
    profiles = thresholds.get("profiles", {})
    if profile_name not in profiles:
        raise ValidationError(
            f"thresholds: surface {surface_id!r} references unknown profile {profile_name!r}"
        )
    return deep_merge(profiles[profile_name], entry.get("overrides", {}))


def check_thresholds(thresholds: dict) -> list[str]:
    errors: list[str] = []
    if thresholds.get("schema") != "dragonfruit.experience.thresholds/1":
        errors.append("thresholds: schema must be dragonfruit.experience.thresholds/1")
    policy = thresholds.get("policy", {})
    if policy.get("loosen") != "forbidden":
        errors.append("thresholds: policy.loosen must be 'forbidden' (thresholds are tighten-only)")
    profiles = thresholds.get("profiles")
    if not isinstance(profiles, dict) or not profiles:
        errors.append("thresholds: profiles must be a non-empty object")
        return errors
    for name, profile in profiles.items():
        if not isinstance(profile, dict):
            errors.append(f"thresholds: profile {name!r} is not an object")
            continue
        for dimension in DIMENSIONS:
            if dimension not in profile:
                errors.append(f"thresholds: profile {name!r} missing {dimension!r}")
                continue
            value = profile[dimension]
            if not isinstance(value, dict) or "applicable" not in value:
                errors.append(f"thresholds: profile {name!r}.{dimension} needs 'applicable'")
                continue
            if value["applicable"] is False and not value.get("reason"):
                errors.append(f"thresholds: profile {name!r}.{dimension} is not applicable but has no reason")
    surfaces = thresholds.get("surfaces")
    if not isinstance(surfaces, dict) or not surfaces:
        errors.append("thresholds: surfaces must be a non-empty object")
        return errors
    for surface_id, entry in surfaces.items():
        if not isinstance(entry, dict) or "profile" not in entry:
            errors.append(f"thresholds: surface {surface_id!r} needs a profile")
            continue
        try:
            resolved = resolve_threshold(thresholds, surface_id)
        except ValidationError as exc:
            errors.append(str(exc))
            continue
        for dimension in DIMENSIONS:
            if dimension not in resolved:
                errors.append(f"thresholds: surface {surface_id!r} missing {dimension!r}")
    return errors


def check_manifest(manifest: dict, schema: dict, thresholds: dict) -> tuple[list[str], dict]:
    errors: list[str] = []
    errors.extend(validate_schema(manifest, schema, schema))

    surfaces = manifest.get("surfaces", [])
    entries = manifest.get("entries", [])
    surface_by_id: OrderedDict[str, dict] = OrderedDict()
    for surface in surfaces:
        sid = surface.get("id")
        if sid in surface_by_id:
            errors.append(f"manifest: duplicate surface id {sid!r}")
        surface_by_id[sid] = surface
        states = surface.get("states", [])
        seen_states = set()
        for state in states:
            state_id = state.get("id")
            if state_id in seen_states:
                errors.append(f"manifest: surface {sid!r} duplicate state {state_id!r}")
            seen_states.add(state_id)

    axis_sets = manifest.get("axis_sets", {})
    vocabulary = manifest.get("axes_vocabulary", {})
    for name, axes in axis_sets.items():
        for key in AXIS_KEYS:
            if key not in axes:
                errors.append(f"manifest: axis set {name!r} missing {key!r}")
        for key, value in axes.items():
            if key in vocabulary and value not in vocabulary[key]:
                errors.append(f"manifest: axis set {name!r}.{key}={value!r} is not in the vocabulary")

    entry_by_id: OrderedDict[str, dict] = OrderedDict()
    covered: dict[str, set[str]] = {sid: set() for sid in surface_by_id}
    for entry in entries:
        eid = entry.get("id")
        if eid in entry_by_id:
            errors.append(f"manifest: duplicate entry id {eid!r}")
        entry_by_id[eid] = entry
        sid = entry.get("surface")
        state = entry.get("state")
        if sid not in surface_by_id:
            errors.append(f"manifest: entry {eid!r} references unknown surface {sid!r}")
            continue
        states = {state["id"] for state in surface_by_id[sid]["states"]}
        if state not in states:
            errors.append(f"manifest: entry {eid!r} references unknown state {sid}/{state}")
            continue
        covered[sid].add(state)

        axes = entry.get("axes")
        if isinstance(axes, str) and axes not in axis_sets:
            errors.append(f"manifest: entry {eid!r} references unknown axis set {axes!r}")
        elif isinstance(axes, dict):
            for key in AXIS_KEYS:
                if key not in axes:
                    errors.append(f"manifest: entry {eid!r} inline axes missing {key!r}")

        try:
            resolved = resolve_threshold(thresholds, sid)
        except ValidationError as exc:
            errors.append(f"manifest: entry {eid!r}: {exc}")
            resolved = None
        if resolved is not None:
            for dimension in entry.get("acceptance", {}).get("dimensions", []):
                if dimension not in resolved:
                    errors.append(f"manifest: entry {eid!r} uses unknown dimension {dimension!r}")
                elif not resolved[dimension].get("applicable"):
                    errors.append(
                        f"manifest: entry {eid!r} uses non-applicable dimension "
                        f"{sid}.{dimension}"
                    )

        verification = entry.get("verification", {})
        status = entry.get("status")
        kind = verification.get("kind")
        if status == "captured":
            if kind != "capture":
                errors.append(f"manifest: entry {eid!r} captured but verification is {kind!r}")
                continue
            path = REPO / verification.get("path", "")
            if not path.is_file():
                errors.append(f"manifest: entry {eid!r} capture path missing: {path}")
                continue
            actual = sha256(path)
            expected = verification.get("sha256")
            if actual != expected:
                errors.append(
                    f"manifest: entry {eid!r} hash mismatch for {path}: "
                    f"expected {expected}, got {actual}"
                )
        elif status == "verified":
            if kind != "test":
                errors.append(f"manifest: entry {eid!r} verified but verification is {kind!r}")
                continue
            test_path = REPO / verification.get("test", "")
            if not test_path.is_file():
                errors.append(f"manifest: entry {eid!r} test path missing: {test_path}")
        elif status == "open":
            if kind != "open":
                errors.append(f"manifest: entry {eid!r} open but verification is {kind!r}")
            for key in ("reason", "rerun", "owner"):
                if not verification.get(key):
                    errors.append(f"manifest: entry {eid!r} open row missing {key!r}")
        else:
            errors.append(f"manifest: entry {eid!r} has unknown status {status!r}")

    for sid, surface in surface_by_id.items():
        for state in surface.get("states", []):
            if state["id"] not in covered[sid]:
                errors.append(f"manifest: surface {sid!r} state {state['id']!r} has no entry")

    return errors, {"surfaces": surface_by_id, "entries": entry_by_id, "covered": covered}


def check_artifact(artifact: dict, manifest: dict, schema: dict, thresholds: dict) -> list[str]:
    errors = validate_schema(artifact, schema["$defs"]["entry"], schema, "$")
    surfaces = {surface["id"]: surface for surface in manifest.get("surfaces", [])}
    sid = artifact.get("surface")
    if sid not in surfaces:
        errors.append(f"artifact: unknown surface {sid!r}")
        return errors
    states = {state["id"] for state in surfaces[sid]["states"]}
    if artifact.get("state") not in states:
        errors.append(f"artifact: unknown state {sid}/{artifact.get('state')}")
    try:
        resolved = resolve_threshold(thresholds, sid)
    except ValidationError as exc:
        errors.append(f"artifact: {exc}")
        return errors
    for dimension in artifact.get("acceptance", {}).get("dimensions", []):
        if dimension not in resolved:
            errors.append(f"artifact: unknown dimension {dimension!r}")
        elif not resolved[dimension].get("applicable"):
            errors.append(f"artifact: non-applicable dimension {sid}.{dimension}")
    verification = artifact.get("verification", {})
    if artifact.get("status") == "captured":
        path = REPO / verification.get("path", "")
        if not path.is_file():
            errors.append(f"artifact: capture path missing: {path}")
        else:
            actual = sha256(path)
            if actual != verification.get("sha256"):
                errors.append(f"artifact: hash mismatch for {path}")
    elif artifact.get("status") == "verified":
        test_path = REPO / verification.get("test", "")
        if not test_path.is_file():
            errors.append(f"artifact: test path missing: {test_path}")
    elif artifact.get("status") == "open":
        for key in ("reason", "rerun", "owner"):
            if not verification.get(key):
                errors.append(f"artifact: open row missing {key!r}")
    return errors


# ---------------------------------------------------------------------------
# Coverage report.
# ---------------------------------------------------------------------------

def coverage(manifest: dict) -> dict:
    surfaces = OrderedDict((surface["id"], surface["name"]) for surface in manifest["surfaces"])
    rows = OrderedDict(
        (sid, {"name": name, "captured": 0, "verified": 0, "open": 0, "total": 0})
        for sid, name in surfaces.items()
    )
    for entry in manifest["entries"]:
        row = rows[entry["surface"]]
        row[entry["status"]] += 1
        row["total"] += 1
    totals = {"captured": 0, "verified": 0, "open": 0, "total": 0}
    for row in rows.values():
        for key in totals:
            totals[key] += row[key]
    return {"rows": rows, "totals": totals}


def print_coverage(manifest: dict) -> None:
    report = coverage(manifest)
    rows = report["rows"]
    width = max(len(sid) for sid in rows) if rows else 8
    print(f"{'surface':<{width}}  {'name':<34}  {'captured':>8}  {'verified':>8}  {'open':>5}  {'total':>5}")
    print("-" * (width + 2 + 34 + 2 + 8 + 2 + 8 + 2 + 5 + 2 + 5))
    for sid, row in rows.items():
        name = row["name"][:34]
        print(
            f"{sid:<{width}}  {name:<34}  {row['captured']:>8}  {row['verified']:>8}  "
            f"{row['open']:>5}  {row['total']:>5}"
        )
    print("-" * (width + 2 + 34 + 2 + 8 + 2 + 8 + 2 + 5 + 2 + 5))
    totals = report["totals"]
    print(
        f"{'TOTAL':<{width}}  {'':<34}  {totals['captured']:>8}  {totals['verified']:>8}  "
        f"{totals['open']:>5}  {totals['total']:>5}"
    )
    print(
        f"\nCovered (captured + verified): "
        f"{totals['captured'] + totals['verified']}/{totals['total']} "
        f"({100.0 * (totals['captured'] + totals['verified']) / totals['total']:.1f}%)"
    )
    all_open = [sid for sid, row in rows.items() if row["captured"] + row["verified"] == 0]
    if all_open:
        print(f"All-OPEN surfaces: {', '.join(all_open)}")


def list_open(manifest: dict) -> None:
    for entry in manifest["entries"]:
        if entry["status"] != "open":
            continue
        verification = entry["verification"]
        print(
            f"{entry['id']}  {entry['surface']}/{entry['state']}  "
            f"[{verification['owner']}] {verification['reason']}  "
            f"rerun: {verification['rerun']}"
        )


# ---------------------------------------------------------------------------
# Main.
# ---------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate the full contract")
    parser.add_argument("--coverage", action="store_true", help="print the coverage summary")
    parser.add_argument("--list-open", action="store_true", help="print every OPEN row")
    parser.add_argument("--json", action="store_true", help="print coverage as JSON")
    parser.add_argument(
        "--validate-artifact",
        metavar="PATH",
        help="validate one standalone manifest entry against the contract",
    )
    args = parser.parse_args(argv)

    if not any((args.check, args.coverage, args.list_open, args.json, args.validate_artifact)):
        parser.error("choose --check, --coverage, --list-open, --json, or --validate-artifact")

    try:
        schema = load_json(SCHEMA)
        manifest = load_json(MANIFEST)
        thresholds = load_json(THRESHOLDS)
    except ValidationError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    errors: list[str] = []
    errors.extend(check_thresholds(thresholds))
    manifest_errors, _ = check_manifest(manifest, schema, thresholds)
    errors.extend(manifest_errors)

    if args.validate_artifact:
        try:
            artifact = load_json(Path(args.validate_artifact))
        except ValidationError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        errors.extend(check_artifact(artifact, manifest, schema, thresholds))

    if errors:
        print(f"FAIL: {len(errors)} contract error(s)", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1

    if args.json:
        report = coverage(manifest)
        print(json.dumps(report, indent=2))
        return 0

    if args.list_open:
        list_open(manifest)
        return 0

    if args.validate_artifact and not args.check and not args.coverage:
        print(f"PASS: artifact {args.validate_artifact}")
        return 0

    if args.check or args.coverage:
        if args.validate_artifact:
            print(f"PASS: artifact {args.validate_artifact}")
        print_coverage(manifest)
        if args.check:
            print(f"\nPASS: {len(manifest['entries'])} entries, "
                  f"{len(manifest['surfaces'])} surfaces, "
                  f"{len(thresholds['profiles'])} threshold profiles")
        return 0

    return 0


if __name__ == "__main__":
    sys.exit(main())
