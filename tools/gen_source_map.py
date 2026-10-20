#!/usr/bin/env python3
"""Regenerate the per-file table of docs/SOURCE_MAP.md from the `//!` module headers.

Usage: python3 -I tools/gen_source_map.py [--check]
  (run from the workspace root, or anywhere: paths are resolved from this file)
  --check: exit 1 if the file would change (for CI), write nothing.

Only the block between the BEGIN/END GENERATED markers is replaced; the hand-written
part (task index etc.) stays untouched. Description = first paragraph of `//!` lines.
"""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOC = os.path.join(ROOT, "docs", "SOURCE_MAP.md")
BEGIN = "<!-- BEGIN GENERATED FILE TABLE (tools/gen_source_map.py) -->"
END = "<!-- END GENERATED FILE TABLE -->"
SCAN = ["crates", "app/src"]


def header(path: str) -> str:
    """Return the first `//!` paragraph of a Rust file, joined to one line.

    path: absolute file path. Returns '' when the file has no module doc.
    """
    parts = []
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            s = line.strip()
            if not s.startswith("//!"):
                # inner attributes / blanks may precede the header
                if not parts and (not s or s.startswith("#![")):
                    continue
                # header ends at first non-doc line
                break
            text = s[3:].strip()
            if not text:
                # blank doc line ends the first paragraph
                if parts:
                    break
                continue
            parts.append(text)
    return " ".join(parts).replace("|", "\\|")


def crate_of(rel: str) -> str:
    """Group key for a relative path.

    rel: path relative to ROOT. Returns e.g. 'crates/core' or 'app'.
    """
    p = rel.split("/")
    return "app" if p[0] == "app" else "/".join(p[:2])


def table() -> str:
    """Build the markdown block for all .rs files.

    Returns the generated text including markers.
    """
    rows = []
    for base in SCAN:
        for d, _, fs in os.walk(os.path.join(ROOT, base)):
            if "/target" in d:
                continue
            for f in fs:
                if f.endswith(".rs"):
                    full = os.path.join(d, f)
                    rel = os.path.relpath(full, ROOT).replace(os.sep, "/")
                    with open(full, encoding="utf-8") as fh:
                        n = sum(1 for _ in fh)
                    rows.append((crate_of(rel), rel, n, header(full)))
    rows.sort()
    out = [BEGIN, ""]
    cur = None
    for crate, rel, n, desc in rows:
        if crate != cur:
            cur = crate
            out += ["", f"### `{crate}`", "", "| file | lines | purpose |", "|---|---:|---|"]
        out.append(f"| [`{rel}`](../{rel}) | {n} | {desc or '_(no module doc)_'} |")
    out += ["", END]
    return "\n".join(out)


def main() -> int:
    """Rewrite the generated block of docs/SOURCE_MAP.md.

    Returns process exit code.
    """
    with open(DOC, encoding="utf-8") as fh:
        doc = fh.read()
    if BEGIN not in doc or END not in doc:
        print(f"markers missing in {DOC}", file=sys.stderr)
        return 2
    head, rest = doc.split(BEGIN, 1)
    _, tail = rest.split(END, 1)
    new = head + table() + tail
    if "--check" in sys.argv:
        return 0 if new == doc else 1
    with open(DOC, "w", encoding="utf-8") as fh:
        fh.write(new)
    return 0


if __name__ == "__main__":
    sys.exit(main())
