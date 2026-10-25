#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

"""Convert LaserGRBL .resx files to flat per-culture JSON for the Rust port.

Key scheme:
  Strings.<Name>                     from Strings*.resx
  <Folder>.<Form>.<ctl>.<Property>   from form resx (folder prefix only for sub-folders)
  <Folder>.<Form>.$this.Text         form title
Invariant culture is written as en.json.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

DEFAULT_SRC = Path("/home/mechaot/workspace/external/Laser/LaserGRBL/LaserGRBL")
DEFAULT_OUT = Path(__file__).resolve().parent.parent / "resources" / "i18n"
SKIP_DIRS = {"SharpGL", "SvgLibrary"}
CULTURE_RE = re.compile(r"^[a-z]{2,3}(-[A-Za-z]{2,4})?$")
INVARIANT = "en"

# Menu order, display names, flag icon ids (MainForm.Designer.cs + AssignIcons).
CULTURES = [
    ("en", "English", "flags-gb"),
    ("it", "Italian", "flags-it"),
    ("es", "Spanish", "flags-es"),
    ("fr", "French", "flags-fr"),
    ("de", "German", "flags-de"),
    ("da", "Danish", "flags-dk"),
    ("pt-BR", "Portuguese", "flags-br"),
    ("ru", "Russian", "flags-ru"),
    ("zh-CN", "Chinese (simplified)", "flags-cn"),
    ("zh-TW", "Chinese (traditional)", "flags-cn"),
    ("sk-SK", "Slovak", "flags-sk"),
    ("hu-HU", "Hungarian", "flags-hu"),
    ("cs-CZ", "Czech", "flags-cz"),
    ("pl-PL", "Polish", "flags-pl"),
    ("el-GR", "Greek", "flags-gr"),
    ("tr-TR", "Turkish", "flags-tr"),
    ("ro-RO", "Romanian", "flags-ro"),
    ("nl-NL", "Dutch", "flags-nl"),
    ("uk", "Ukrainian", "flags-ua"),
    ("ja-JP", "Japanese", "flags-jp"),
]


def split_culture(stem: str) -> tuple[str, str]:
    """Split a file stem into base name and culture.

    Args:
        stem: file name without ".resx", e.g. "MainForm.it".
    Returns:
        (base, culture); culture is INVARIANT when no suffix present.
    """
    base, dot, suffix = stem.rpartition(".")
    if dot and CULTURE_RE.match(suffix):
        return base, suffix
    return stem, INVARIANT


def form_prefix(rel_dir: Path, base: str) -> str:
    """Build key prefix for a form.

    Args:
        rel_dir: directory of the file relative to source root.
        base: form name without culture.
    Returns:
        "Folder.Form" for sub-folders, plain "Form" for root files.
    """
    parts = [p for p in rel_dir.parts if p not in (".", "")]
    return ".".join(parts + [base])


def parse_resx(path: Path) -> dict[str, str]:
    """Parse one resx, keep plain string entries only.

    Args:
        path: resx file.
    Returns:
        name -> value, with typed/mimetype/metadata entries dropped, CRLF -> LF.
    """
    out: dict[str, str] = {}
    root = ET.parse(path).getroot()
    for data in root.iter("data"):
        # typed or binary entries are not plain strings
        if "type" in data.attrib or "mimetype" in data.attrib:
            continue
        name = data.get("name")
        # ">>ctl.Name" etc. is designer metadata
        if not name or name.startswith(">>"):
            continue
        val = data.find("value")
        text = (val.text or "") if val is not None else ""
        out[name] = text.replace("\r\n", "\n").replace("\r", "\n")
    return out


def collect(src: Path) -> dict[str, dict[str, str]]:
    """Walk source tree and build per-culture flat dicts.

    Args:
        src: root dir containing resx files.
    Returns:
        culture -> {key: value}.
    """
    result: dict[str, dict[str, str]] = {}
    for path in sorted(src.rglob("*.resx")):
        rel = path.relative_to(src)
        if rel.parts[0] in SKIP_DIRS:
            continue
        base, culture = split_culture(path.stem)
        prefix = "Strings" if base == "Strings" else form_prefix(rel.parent, base)
        bucket = result.setdefault(culture, {})
        for name, value in parse_resx(path).items():
            bucket[f"{prefix}.{name}"] = value
    return result


def write_all(src: Path, out: Path) -> dict[str, int]:
    """Write culture JSON files and cultures.json.

    Args:
        src: resx root.
        out: output directory (created if missing).
    Returns:
        culture -> key count.
    """
    out.mkdir(parents=True, exist_ok=True)
    data = collect(src)
    counts: dict[str, int] = {}
    for culture, entries in data.items():
        (out / f"{culture}.json").write_text(
            json.dumps(dict(sorted(entries.items())), ensure_ascii=False, indent=1) + "\n",
            encoding="utf-8",
        )
        counts[culture] = len(entries)
    menu = [{"culture": c, "name": n, "flag": f} for c, n, f in CULTURES]
    (out / "cultures.json").write_text(
        json.dumps(menu, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )
    return counts


def main() -> int:
    """CLI entry.

    Returns:
        process exit code.
    """
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--src", type=Path, default=DEFAULT_SRC)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()
    for culture, n in sorted(write_all(args.src, args.out).items()):
        print(f"{culture}: {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
