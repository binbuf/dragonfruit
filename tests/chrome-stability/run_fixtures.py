#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T349 chrome stability fixture gate.

Runs the offscreen QML fixture suite and compares each test function with
``tests/chrome-stability/expected-failures.json``. The four status-anchor
reproducers and the Dock chooser footprint reproducers are *expected to fail*
on the starting revision; their manifest rows name the owning task. When an
owner lands, that task flips its row to ``pass`` and this gate starts enforcing
the fix.

The gate is deterministic and needs no native seat. GPU/native evidence is a
separate class recorded by the smoke/capture rails.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

RESULT_RE = re.compile(
    r"^(PASS|FAIL!|SKIP|XFAIL|XPASS|BPASS|BFAIL)\s*:\s*"
    r"(?:\S+::)?(?P<case>[A-Za-z_][A-Za-z0-9_]*)::"
    r"(?P<test>[A-Za-z_][A-Za-z0-9_]*)(?:\(.*\))?"
)
STATUS = {
    "PASS": "pass",
    "FAIL!": "fail",
    "SKIP": "skip",
    "XFAIL": "xfail",
    "XPASS": "xpass",
    "BPASS": "bpass",
    "BFAIL": "bfail",
}
EXPECTED = {"pass", "fail"}


@dataclass(frozen=True)
class ManifestCase:
    test: str
    expected: str
    owner: str
    setup: str
    symptom: str


@dataclass(frozen=True)
class Problem:
    test: str
    message: str

    def __str__(self) -> str:
        return f"{self.test}: {self.message}"


def load_manifest(path: Path) -> list[ManifestCase]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema") != 1:
        raise ValueError(f"{path}: unsupported manifest schema {data.get('schema')!r}")
    raw_cases = data.get("cases")
    if not isinstance(raw_cases, list) or not raw_cases:
        raise ValueError(f"{path}: 'cases' must be a non-empty list")

    cases: list[ManifestCase] = []
    seen: set[str] = set()
    for index, raw in enumerate(raw_cases):
        if not isinstance(raw, dict):
            raise ValueError(f"{path}: case {index} is not an object")
        test = str(raw.get("test", ""))
        expected = str(raw.get("expected", ""))
        owner = str(raw.get("owner", ""))
        setup = str(raw.get("setup", ""))
        symptom = str(raw.get("symptom", ""))
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", test):
            raise ValueError(f"{path}: case {index} has an invalid test name {test!r}")
        if test in seen:
            raise ValueError(f"{path}: duplicate case {test!r}")
        if expected not in EXPECTED:
            raise ValueError(f"{path}: {test} expected must be pass or fail")
        if not re.fullmatch(r"T\d+", owner):
            raise ValueError(f"{path}: {test} owner must be a task id (Tnnn)")
        if not setup:
            raise ValueError(f"{path}: {test} must record its setup")
        if not symptom:
            raise ValueError(f"{path}: {test} must record its symptom")
        seen.add(test)
        cases.append(ManifestCase(test, expected, owner, setup, symptom))
    return cases


def parse_results(text: str) -> dict[str, str]:
    results: dict[str, str] = {}
    for line in text.splitlines():
        match = RESULT_RE.match(line.strip())
        if not match:
            continue
        test = match.group("test")
        if test in {"initTestCase", "cleanupTestCase"}:
            continue
        results[test] = STATUS[match.group(1)]
    return results


def evaluate(
    cases: Iterable[ManifestCase], results: dict[str, str]
) -> list[Problem]:
    problems: list[Problem] = []
    by_test = {case.test: case for case in cases}
    for test, case in by_test.items():
        actual = results.get(test)
        if actual is None:
            problems.append(Problem(test, "did not run"))
            continue
        if case.expected == "pass":
            if actual != "pass":
                problems.append(
                    Problem(test, f"expected pass, observed {actual} (owner {case.owner})")
                )
        elif actual != "fail":
            problems.append(
                Problem(
                    test,
                    f"expected fail until {case.owner} lands, observed {actual}; "
                    "if the fix landed, flip this manifest row to pass",
                )
            )
    for test in sorted(set(results) - set(by_test)):
        problems.append(
            Problem(test, "ran but has no manifest row; add expected pass/fail and owner")
        )
    return problems


def run_suite(
    binary: Path, build_dir: Path, source_dir: Path, capture_dir: Path | None
) -> tuple[int, str]:
    if not binary.is_file():
        return 127, f"{binary}: fixture binary is missing; run `make build`"
    env = dict(os.environ)
    env.update(
        {
            "QT_QPA_PLATFORM": "offscreen",
            "QT_QUICK_BACKEND": "software",
            "QML2_IMPORT_PATH": str(build_dir / "qml"),
        }
    )
    if capture_dir is not None:
        capture_dir.mkdir(parents=True, exist_ok=True)
        env["DF_CHROME_STABILITY_CAPTURE_DIR"] = str(capture_dir)
    command = [str(binary), "-input", "tst_chromestability.qml"]
    completed = subprocess.run(
        command,
        cwd=source_dir,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    return completed.returncode, completed.stdout


def write_report(
    path: Path,
    cases: Iterable[ManifestCase],
    results: dict[str, str],
    problems: list[Problem],
    raw: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "T349 chrome stability fixtures",
        "evidence class: Qt Quick Test, offscreen QPA + software scene graph",
        "",
        "case\texpected\tobserved\towner",
    ]
    for case in cases:
        lines.append(
            f"{case.test}\t{case.expected}\t{results.get(case.test, 'missing')}\t"
            f"{case.owner}"
        )
    lines.append("")
    if problems:
        lines.append("verdict: FAIL")
        for problem in problems:
            lines.append(f"problem: {problem}")
    else:
        lines.append("verdict: PASS")
    lines.extend(["", "## raw fixture output", raw.rstrip()])
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def default_paths() -> tuple[Path, Path, Path, Path]:
    root = Path(__file__).resolve().parents[2]
    return (
        root / "build",
        root / "shell" / "tests",
        root / "tests" / "chrome-stability" / "expected-failures.json",
        root / "docs" / "captures" / "stability2" / "t349-chrome-fixtures.txt",
    )


def main(argv: list[str] | None = None) -> int:
    build_default, source_default, manifest_default, report_default = default_paths()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", type=Path, default=build_default)
    parser.add_argument("--source-dir", type=Path, default=source_default)
    parser.add_argument("--manifest", type=Path, default=manifest_default)
    parser.add_argument("--report", type=Path, default=report_default)
    parser.add_argument(
        "--capture-dir",
        type=Path,
        default=build_default.parent / "docs" / "captures" / "stability2" / "fixtures",
        help="optional PNG output directory for the fixture cases",
    )
    parser.add_argument(
        "--no-capture",
        action="store_true",
        help="do not write fixture PNGs (report only)",
    )
    args = parser.parse_args(argv)

    try:
        cases = load_manifest(args.manifest)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"chrome-stability: {error}", file=sys.stderr)
        return 2

    binary = args.build_dir / "shell" / "tests" / "tst_chromestability"
    capture_dir = None if args.no_capture else args.capture_dir
    _status, raw = run_suite(binary, args.build_dir, args.source_dir, capture_dir)
    results = parse_results(raw)
    problems = evaluate(cases, results)
    # Qt Quick Test exits with the number of failed functions. A normal run has
    # one failure per expected-failure row, so the status is not a pass signal;
    # the parsed result set and the manifest are. A crash before the suite
    # finishes leaves missing rows, which evaluate() already reports.

    write_report(args.report, cases, results, problems, raw)
    print(f"chrome-stability: report {args.report}")
    for case in cases:
        print(
            f"  {results.get(case.test, 'missing'):8} {case.test:58} "
            f"expected {case.expected:4} owner {case.owner}"
        )
    if problems:
        print("chrome-stability: FAIL", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    print("chrome-stability: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
