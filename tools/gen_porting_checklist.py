#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

"""Generate PORTING.md: one section per original C# file, one row per declared member.

Usage: gen_porting_checklist.py <original LaserGRBL project dir> <output PORTING.md>
Rows start as TODO; porting/audit agents replace status + Rust location.
"""
import os
import re
import sys

VENDORED = ("SharpGL/", "SvgLibrary/", "ComWrapper/WebSocket/", "ComWrapper/RJCP/", "ComWrapper/MySerial/")

# work package per path prefix (first match wins)
WP = [
    ("Clipper/", "WP-B"), ("CsPotrace/", "WP-B2"), ("Tools/ExpressionEvaluator.cs", "WP-D"),
    ("Hershey/", "WP-D"), ("RasterConverter/Dithering/", "WP-F"), ("RasterConverter/ImageTransform.cs", "WP-F"),
    ("SvgConverter/GCodeFromSVG.cs", "WP-E"), ("SvgConverter/gcodeRelated.cs", "WP-E"),
    ("SvgConverter/BezierTools.cs", "WP-E"), ("Settings.cs", "WP-C1"), ("Logger/", "WP-C1"), ("CSV/", "WP-C1"),
    ("GrblCommand.cs", "WP-C1"), ("StateBuilder.cs", "WP-C1"), ("GrblFile.cs", "WP-C1"), ("ColorScheme.cs", "WP-C1"),
    ("Tools/HiResTimer.cs", "WP-C1"), ("Tools/ThreadClass.cs", "WP-C1"), ("Tools/Cronometro.cs", "WP-C1"),
    ("Tools/MathHelper.cs", "WP-C1"), ("Tools/MobileAverageCalculator.cs", "WP-C1"), ("Tools/WordWrap.cs", "WP-C1"),
    ("Tools/RetainedSetting.cs", "WP-C1"), ("Core/", "WP-C2"), ("ComWrapper/", "WP-C2"), ("GrblEmulator/Grblv11", "WP-C2"),
    ("GrblEmulator/Websocket", "WP-C2"), ("HotKeysManager.cs", "WP-C2"), ("CustomButton.cs", "WP-C2"),
    ("SincroStart.cs", "WP-C2"), ("Telegram.cs", "WP-C2"), ("Sound/", "WP-C2"), ("UsageStats.cs", "WP-C2"),
    ("Tools/Project.cs", "WP-C2"), ("Tools/Serializer.cs", "WP-C2"), ("PSHelper/MaterialDB.cs", "WP-C2"),
    ("WiFiDiscovery/IPAddressHelper.cs", "WP-C2"), ("Autotrace/Autotrace.cs", "WP-C2"),
    ("RasterConverter/ImageProcessor.cs", "WP-C2"), ("UrlManager.cs", "WP-C2"), ("Tools/Json.cs", "WP-C2"),
    ("MainForm.cs", "WP-U1"), ("ConnectLogForm.cs", "WP-U1"), ("JogForm.cs", "WP-U1"), ("UserControls/CommandLog.cs", "WP-U1"),
    ("UserControls/Move2DControl.cs", "WP-U1"), ("UserControls/ThemeMgr.cs", "WP-U1"), ("Icons/", "WP-U1"), ("SvgIcons/", "WP-U1"),
    ("SplashScreenForm.cs", "WP-U1"), ("LegalDisclaimer.cs", "WP-U1"), ("LicenseForm.cs", "WP-U1"), ("ExceptionManager.cs", "WP-U1"),
    ("IssueDetectorForm.cs", "WP-U1"), ("Program.cs", "WP-U1"), ("PreviewForm.cs", "WP-U2"), ("UserControls/GrblPanel", "WP-U2"),
    ("UserControls/IGrblPanel.cs", "WP-U2"), ("Obj3D/", "WP-U2"), ("CustomButtonForm.cs", "WP-U2"),
    ("RasterConverter/", "WP-U3"), ("UserControls/ImageTransform.cs", "WP-U3"), ("PSHelper/", "WP-U3"),
    ("UserControls/", "WP-U1"), ("Tools/", "WP-U4"), ("", "WP-U4"),
]

DECL = re.compile(
    r"^\s*(?:\[[^\]]*\]\s*)*(?:(?:public|private|protected|internal|static|virtual|override|abstract|sealed|"
    r"readonly|unsafe|extern|partial|new|const|event|async|implicit|explicit|operator)\s+)+"
    r"(?:(?P<kind>class|struct|enum|interface|delegate)\s+(?P<tname>\w+)|"
    r"(?P<ret>[\w<>\[\],.?\s]+?)\s+(?P<name>[\w.]+|operator\s*\S+)\s*(?P<tail>\(|\{|=>|=|;))"
)
CTOR = re.compile(r"^\s*(?:public|private|protected|internal|static)\s+(?P<name>\w+)\s*\(")


def wp_for(rel: str) -> str:
    """Pick work package for a relative path.

    rel: path relative to project dir. Returns WP id.
    """
    for prefix, wp in WP:
        if rel.startswith(prefix):
            return wp
    return "WP-U4"


def members(path: str) -> list[str]:
    """Extract declared members from a C# file with a forgiving regex.

    path: file path. Returns list of 'kind name' strings in file order.
    """
    out = []
    with open(path, encoding="utf-8-sig", errors="replace") as fh:
        for line in fh:
            if line.lstrip().startswith("//"):
                continue
            m = DECL.match(line)
            if m:
                if m.group("kind"):
                    out.append(f"{m.group('kind')} {m.group('tname')}")
                else:
                    tail = m.group("tail")
                    kind = "method" if tail == "(" else "property" if tail in ("{", "=>") else "field"
                    out.append(f"{kind} {m.group('name').strip()}")
                continue
            c = CTOR.match(line)
            if c:
                out.append(f"ctor {c.group('name')}")
    return out


def main() -> None:
    """Walk project, write markdown checklist.

    Returns nothing; writes argv[2].
    """
    root, dst = sys.argv[1], sys.argv[2]
    files = []
    for d, _, fs in os.walk(root):
        for f in fs:
            if f.endswith(".cs") and not f.lower().endswith(".designer.cs"):
                rel = os.path.relpath(os.path.join(d, f), root).replace(os.sep, "/")
                if not rel.startswith(VENDORED):
                    files.append(rel)
    files.sort()
    lines = ["# Porting checklist", "",
             "Status: `DONE` (ported, Rust location given), `N/A` (not needed, reason given), `TODO`.",
             "Designer files (`*.Designer.cs`) belong to the form row of the same name: layout/captions/defaults must be mirrored.",
             "Vendored libraries are not listed (see PLAN.md §6).", ""]
    for rel in files:
        lines.append(f"## `{rel}` ({wp_for(rel)})")
        lines.append("")
        lines.append("| member | status | Rust location / reason |")
        lines.append("|---|---|---|")
        for m in members(os.path.join(root, rel)):
            lines.append(f"| `{m}` | TODO | |")
        lines.append("")
    with open(dst, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines))


if __name__ == "__main__":
    main()
