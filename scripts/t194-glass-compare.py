#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Command-line wrapper for the T194 glass comparison core.

Usage:
    python3 scripts/t194-glass-compare.py compare \
        --reference reference.png --candidate candidate.png --case case.json
    python3 scripts/t194-glass-compare.py motion \
        --reference-before a.png --reference-after b.png \
        --candidate-before c.png --candidate-after d.png --case motion.json
    python3 scripts/t194-glass-compare.py timing \
        --reference baseline.json --candidate current.json

The implementation lives in ``scripts/glass_compare.py`` so the self-test and
later T207 runs can import it directly.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from glass_compare import main  # noqa: E402


if __name__ == "__main__":
    sys.exit(main())
