#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

"""Check that source files carry an SPDX license header.

Usage: python3 -I tools/check_headers.py
Scans *.rs, *.py, *.sh under app/, crates/, tools/, packaging/ (skips target/ and __pycache__).
Exit 0 when every file has `SPDX-License-Identifier` in its first 5 lines, else 1 and a file list.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIRS = ("app", "crates", "tools", "packaging")
SUFFIXES = {".rs", ".py", ".sh"}
SKIP_PARTS = {"target", "__pycache__", ".git"}
HEAD_LINES = 5


def source_files() -> list[Path]:
    """List source files to check.

    Returns sorted paths of *.rs/*.py/*.sh under the scanned directories.
    """
    out = []
    for d in DIRS:
        for p in (ROOT / d).rglob("*"):
            if p.is_file() and p.suffix in SUFFIXES and not SKIP_PARTS & set(p.relative_to(ROOT).parts):
                out.append(p)
    return sorted(out)


def has_spdx(path: Path) -> bool:
    """Look for the SPDX tag in the first lines of a file.

    path: file to read. Returns True when `SPDX-License-Identifier` is found.
    """
    with path.open(encoding="utf-8", errors="replace") as fh:
        for _, line in zip(range(HEAD_LINES), fh):
            if "SPDX-License-Identifier" in line:
                return True
    return False


def main() -> int:
    """CLI entry point.

    Returns 0 when all files have a header, 1 otherwise (missing files are printed).
    """
    files = source_files()
    missing = [p for p in files if not has_spdx(p)]
    if missing:
        print(f"{len(missing)} file(s) without SPDX-License-Identifier in the first {HEAD_LINES} lines:")
        for p in missing:
            print(f"  {p.relative_to(ROOT)}")
        return 1
    print(f"ok: {len(files)} files have an SPDX header")
    return 0


if __name__ == "__main__":
    sys.exit(main())
