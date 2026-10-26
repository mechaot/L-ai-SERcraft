#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Andreas Pösch

"""Check that no test writes into the user's config dirs, the temp dir or the repository.

Usage: python3 -I tools/check_test_isolation.py [--jobs N] [--profile release|dev] [--skip-build]
                                                [--locked] [--timeout SEC] [-v]

How it works:
  1. Build all test binaries (`cargo test --workspace --release --no-run --message-format=json`).
  2. List the tests of every binary and run EACH test alone in its own process. cwd = package dir.
     HOME, XDG_CONFIG_HOME, XDG_DATA_HOME, XDG_CACHE_HOME, XDG_STATE_HOME, APPDATA and LOCALAPPDATA
     point into a fresh, empty scratch dir per process. TMPDIR, TEMP and TMP point into an empty `tmp`
     dir of that scratch dir.
  3. Anything that appears under the scratch dir is an isolation finding (binary + test name);
     anything left in the `tmp` dir after the process exited is a "temp leftover" finding.
  4. `git status --porcelain` is snapshotted before and after the whole run; any change is a finding.

A test passes the check when it calls `lg_base::paths::isolate_for_tests` before it touches
settings, logger, laser life, emulator config and friends, and makes its scratch files with
`lg_base::paths::test_dir` instead of `std::env::temp_dir()`; both are removed at process exit
(see docs/ARCHITECTURE.md, "Testing").

Exit codes: 0 clean, 1 isolation findings and/or failed tests, 2 setup error (build, listing, usage).
`--skip-build` reuses the binary list saved by the last run of this script (no cargo call at all).
`--profile dev` checks the debug binaries (CI: reuses the build of the `cargo test` step).
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# env vars that decide where an app keeps its config; all point into the scratch dir
SCRATCH_ENV = {
    "HOME": "home",
    "XDG_CONFIG_HOME": "config",
    "XDG_DATA_HOME": "data",
    "XDG_CACHE_HOME": "cache",
    "XDG_STATE_HOME": "state",
    "APPDATA": "appdata",
    "LOCALAPPDATA": "localappdata",
}
# temp dir vars (POSIX + Windows spellings) share one empty subdir of the scratch dir
TEMP_ENV = ("TMPDIR", "TEMP", "TMP")
TEMP_SUB = "tmp"
# debug switch of lg_base::test_support that keeps the scratch root: must not hide leftovers here
KEEP_ENV = "LG_KEEP_TEST_DIRS"
MAX_FILES_SHOWN = 8
TAIL_LINES = 25


@dataclass(frozen=True)
class TestBin:
    """One test executable.

    label: `package:target` for messages. exe: executable path. cwd: package dir (cargo runs tests there).
    """

    label: str
    exe: Path
    cwd: Path


@dataclass
class Outcome:
    """Result of one test process.

    binary: owning test binary. test: test name (`<list>` for the listing call).
    passed: exit code 0 (false on timeout). files: scratch entries left behind (relative paths).
    temp: entries left in the process's TMPDIR (relative to it). output: tail of stdout+stderr, kept for failures only.
    """

    binary: TestBin
    test: str
    passed: bool
    files: list[str] = field(default_factory=list)
    temp: list[str] = field(default_factory=list)
    output: str = ""


def target_dir() -> Path:
    """Cargo target dir.

    Returns `$CARGO_TARGET_DIR` or `<root>/target`.
    """
    env = os.environ.get("CARGO_TARGET_DIR")
    return Path(env).resolve() if env else ROOT / "target"


def package_label(package_id: str, manifest: Path) -> str:
    """Short package name from a cargo package id.

    package_id: `path+file:///x/crates/base#lg-base@1.3.0` or old `lg-base 1.3.0 (path+...)`.
    manifest: Cargo.toml path, used as fallback (dir name). Returns the package name.
    """
    m = re.search(r"#(?:[^@/]*/)?([^@/#]+)@", package_id) or re.match(r"([^ ]+) ", package_id)
    return m.group(1) if m else manifest.parent.name


def parse_cargo_json(lines: list[str]) -> list[TestBin]:
    """Pick test executables out of cargo's JSON messages.

    lines: stdout lines of `cargo test --no-run --message-format=json`.
    Returns the unique executables with `profile.test == true`, in cargo order.
    """
    bins: dict[Path, TestBin] = {}
    for line in lines:
        # non-JSON noise is skipped
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-artifact" or not msg.get("profile", {}).get("test"):
            continue
        exe = msg.get("executable")
        if not exe:
            continue
        manifest = Path(msg["manifest_path"])
        label = f"{package_label(msg.get('package_id', ''), manifest)}:{msg['target']['name']}"
        bins[Path(exe)] = TestBin(label, Path(exe), manifest.parent)
    return list(bins.values())


def cache_file() -> Path:
    """Where the binary list of the last build is saved.

    Returns the JSON path inside the cargo target dir.
    """
    return target_dir() / "check_test_isolation_bins.json"


def build(profile: str, locked: bool) -> list[TestBin]:
    """Build every test binary and remember the list.

    profile: `release` or `dev`. locked: pass `--locked` to cargo.
    Returns the test binaries. Exits with code 2 when the build fails.
    """
    cmd = ["cargo", "test", "--workspace", "--no-run"]
    if profile == "release":
        cmd.append("--release")
    if locked:
        cmd.append("--locked")
    print("build:", " ".join(cmd), flush=True)
    cmd.append("--message-format=json")
    # stderr (progress, warnings) goes straight to the terminal, stdout carries the JSON
    proc = subprocess.run(cmd, cwd=ROOT, stdout=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        print("error: cargo build failed", file=sys.stderr)
        sys.exit(2)
    bins = parse_cargo_json(proc.stdout.splitlines())
    cache = cache_file()
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps([[b.label, str(b.exe), str(b.cwd)] for b in bins]), encoding="utf-8")
    return bins


def load_cached() -> list[TestBin]:
    """Binary list of the last build (for `--skip-build`).

    Returns the test binaries. Exits with code 2 when there is no list or an executable is gone.
    """
    try:
        rows = json.loads(cache_file().read_text(encoding="utf-8"))
    except (OSError, ValueError):
        print(f"error: no saved build at {cache_file()}, run once without --skip-build", file=sys.stderr)
        sys.exit(2)
    bins = [TestBin(label, Path(exe), Path(cwd)) for label, exe, cwd in rows]
    gone = [b.label for b in bins if not b.exe.is_file()]
    if gone:
        print(f"error: executables missing ({', '.join(gone)}), run without --skip-build", file=sys.stderr)
        sys.exit(2)
    return bins


def scratch_env(scratch: Path) -> dict[str, str]:
    """Process env that hides the real config dirs.

    scratch: fresh dir; every config var gets its own empty subdir, the temp vars share the empty `tmp` subdir.
    Returns the full env mapping.
    """
    env = dict(os.environ)
    env.pop(KEEP_ENV, None)
    for var, sub in SCRATCH_ENV.items():
        d = scratch / sub
        d.mkdir(parents=True, exist_ok=True)
        env[var] = str(d)
    tmp = scratch / TEMP_SUB
    tmp.mkdir(parents=True, exist_ok=True)
    for var in TEMP_ENV:
        env[var] = str(tmp)
    return env


def scan(base: Path, skip: Path | None = None) -> list[str]:
    """Everything below a dir.

    base: dir to scan (not listed itself). skip: subdir that is not entered. Returns sorted relative paths:
    all files, symlinks and empty dirs (an empty dir counts too: `data_path()` creates the config dir even when
    nothing is written).
    """
    out: list[str] = []
    for dirpath, dirnames, filenames in os.walk(base):
        here = Path(dirpath)
        dirnames[:] = [d for d in dirnames if here / d != skip]
        if not dirnames and not filenames and here != base:
            out.append(str(here.relative_to(base)) + "/")
        out.extend(str((here / f).relative_to(base)) for f in filenames)
    return sorted(out)


def leftovers(scratch: Path) -> tuple[list[str], list[str]]:
    """Everything a test process created under its scratch dir.

    scratch: dir to scan. Returns (config findings, temp findings): the first lists entries outside `tmp`
    (relative to scratch; the pre-created, empty env dirs do not count), the second lists everything in the
    `tmp` dir (relative to it).
    """
    tmp = scratch / TEMP_SUB
    pre = {sub + "/" for sub in SCRATCH_ENV.values()}
    config = [p for p in scan(scratch, skip=tmp) if p not in pre]
    return config, scan(tmp)


def run_proc(argv: list[str], cwd: Path, env: dict[str, str], timeout: float, merge_stderr: bool) -> tuple[int | None, str, str]:
    """Run a child process.

    argv: command line. cwd: working dir. env: full environment. timeout: seconds until the child is killed.
    merge_stderr: True folds stderr into stdout. Returns (exit code or None on timeout, stdout, stderr).
    """
    err_mode = subprocess.STDOUT if merge_stderr else subprocess.PIPE
    with subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                          stderr=err_mode, text=True, errors="replace") as proc:
        try:
            out, err = proc.communicate(timeout=timeout)
            code: int | None = proc.returncode
        except subprocess.TimeoutExpired:
            proc.kill()
            out, err = proc.communicate()
            code = None
    return code, out or "", err or ""


def run_one(binary: TestBin, args: list[str], test: str, scratch_root: Path, timeout: float) -> Outcome:
    """Run one test binary invocation in a fresh scratch dir.

    binary: executable to run. args: its arguments. test: name for reports.
    scratch_root: parent for the per-process scratch dir. timeout: seconds before the process is killed.
    Returns the outcome (exit status + files left behind in the scratch dir and in its temp dir).
    """
    scratch = Path(tempfile.mkdtemp(prefix="p", dir=scratch_root))
    env = scratch_env(scratch)
    # cargo sets this for `cargo test`; a few tests read it at runtime
    env["CARGO_MANIFEST_DIR"] = str(binary.cwd)
    code, text, _ = run_proc([str(binary.exe), *args], binary.cwd, env, timeout, merge_stderr=True)
    passed = code == 0
    if code is None:
        text += f"\n[timeout after {timeout:.0f}s]"
    files, temp = leftovers(scratch)
    shutil.rmtree(scratch, ignore_errors=True)
    tail = "" if passed else "\n".join(text.splitlines()[-TAIL_LINES:])
    return Outcome(binary, test, passed, files, temp, tail)


def list_tests(binary: TestBin, scratch_root: Path, timeout: float) -> tuple[list[str], Outcome]:
    """Ask a test binary for its tests.

    binary: executable. scratch_root: scratch parent. timeout: seconds.
    Returns (test names, outcome of the listing process itself; carries findings if listing wrote files).
    """
    scratch = Path(tempfile.mkdtemp(prefix="l", dir=scratch_root))
    env = scratch_env(scratch)
    env["CARGO_MANIFEST_DIR"] = str(binary.cwd)
    code, out, err = run_proc([str(binary.exe), "--list", "--format", "terse"], binary.cwd, env, timeout, merge_stderr=False)
    files, temp = leftovers(scratch)
    shutil.rmtree(scratch, ignore_errors=True)
    names = [ln[: -len(": test")] for ln in out.splitlines() if ln.endswith(": test")]
    note = "" if code == 0 else (err[-2000:] or f"[listing timed out after {timeout:.0f}s]")
    return names, Outcome(binary, "<list>", code == 0, files, temp, note)


def repo_snapshot() -> dict[str, tuple[str, int, int]] | None:
    """State of the git working tree (what `git status` sees).

    Returns {path: (status code, size, mtime_ns)} so a rewrite of an already modified file shows up too,
    or None when git or the repo is not available. `target/` never shows (ignored by git); also skipped by name.
    """
    try:
        proc = subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=all", "-z"],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    snap: dict[str, tuple[str, int, int]] = {}
    entries = proc.stdout.decode(errors="replace").split("\0")
    i = 0
    while i < len(entries):
        e = entries[i]
        i += 1
        if len(e) < 4:
            continue
        status, path = e[:2], e[3:]
        # rename/copy entries carry the old path as an extra field
        if status[0] in "RC":
            i += 1
        if path.startswith("target/"):
            continue
        try:
            st = (ROOT / path).lstat()
            snap[path] = (status, st.st_size, st.st_mtime_ns)
        except OSError:
            snap[path] = (status, -1, -1)
    return snap


def diff_snapshots(before: dict[str, tuple[str, int, int]], after: dict[str, tuple[str, int, int]]) -> list[str]:
    """Compare two repo snapshots.

    before, after: results of `repo_snapshot`. Returns human readable lines for added, removed and changed paths.
    """
    out: list[str] = []
    for path in sorted(before.keys() | after.keys()):
        b, a = before.get(path), after.get(path)
        if b == a:
            continue
        if b is None:
            out.append(f"new       {path} [{a[0].strip() if a else ''}]")
        elif a is None:
            out.append(f"reverted  {path}")
        else:
            out.append(f"modified  {path}")
    return out


def print_findings(outcomes: list[Outcome], paths_of: Callable[[Outcome], list[str]]) -> None:
    """Print findings, one block per test process.

    outcomes: processes with findings. paths_of: picks the list of leftover paths out of an outcome.
    """
    for f in sorted(outcomes, key=lambda o: (o.binary.label, o.test)):
        paths = paths_of(f)
        print(f"  {f.binary.label}  {f.test}")
        for p in paths[:MAX_FILES_SHOWN]:
            print(f"      {p}")
        if len(paths) > MAX_FILES_SHOWN:
            print(f"      ... {len(paths) - MAX_FILES_SHOWN} more")


def main() -> int:
    """CLI entry point.

    Returns 0 when clean, 1 on isolation findings or failed tests, 2 on setup errors.
    """
    ap = argparse.ArgumentParser(description="Run every test alone with a scratch HOME/XDG and flag files it creates.")
    ap.add_argument("--jobs", "-j", type=int, default=os.cpu_count() or 4, help="parallel test processes (default: CPU count)")
    ap.add_argument("--profile", choices=("release", "dev"), default="release", help="cargo profile of the binaries (default: release)")
    ap.add_argument("--skip-build", action="store_true", help="reuse the binaries of the last run (no cargo call)")
    ap.add_argument("--locked", action="store_true", help="pass --locked to cargo")
    ap.add_argument("--timeout", type=float, default=600.0, help="seconds per test process (default: 600)")
    ap.add_argument("-v", "--verbose", action="store_true", help="print every test result")
    opts = ap.parse_args()
    if opts.jobs < 1:
        ap.error("--jobs must be >= 1")

    t0 = time.monotonic()
    bins = load_cached() if opts.skip_build else build(opts.profile, opts.locked)
    print(f"{len(bins)} test binaries, {opts.jobs} jobs", flush=True)

    before = repo_snapshot()
    scratch_root = Path(tempfile.mkdtemp(prefix="lg_iso_"))
    findings: list[Outcome] = []
    temp_findings: list[Outcome] = []
    failures: list[Outcome] = []
    total = 0
    try:
        with ThreadPoolExecutor(max_workers=opts.jobs) as pool:
            # list all binaries first (parallel), then run every test alone (parallel, one flat queue)
            listed = list(pool.map(lambda b: list_tests(b, scratch_root, opts.timeout), bins))
            futures = []
            for binary, (names, lst) in zip(bins, listed):
                if lst.files:
                    findings.append(lst)
                if lst.temp:
                    temp_findings.append(lst)
                if not lst.passed:
                    failures.append(lst)
                for name in names:
                    futures.append(pool.submit(run_one, binary, ["--exact", name, "-q", "--test-threads=1"], name, scratch_root, opts.timeout))
            for n, fut in enumerate(futures, 1):
                res = fut.result()
                total += 1
                if opts.verbose:
                    print(f"  {'ok  ' if res.passed else 'FAIL'} {'LEAK' if res.files or res.temp else '    '} {res.binary.label} {res.test}")
                elif sys.stderr.isatty() and n % 50 == 0:
                    print(f"\r  {n}/{len(futures)} tests", end="", file=sys.stderr, flush=True)
                if res.files:
                    findings.append(res)
                if res.temp:
                    temp_findings.append(res)
                if not res.passed:
                    failures.append(res)
        if sys.stderr.isatty():
            print("\r" + " " * 40 + "\r", end="", file=sys.stderr)
    finally:
        shutil.rmtree(scratch_root, ignore_errors=True)
    after = repo_snapshot()

    repo_changes: list[str] = []
    if before is None or after is None:
        print("warning: git status not available, repository check skipped")
    else:
        repo_changes = diff_snapshots(before, after)

    # report: isolation findings, repo changes and test failures are separate sections
    if findings:
        print(f"\nISOLATION FINDINGS: {len(findings)} test process(es) left files in the scratch HOME/XDG dirs")
        print_findings(findings, lambda o: o.files)
    if temp_findings:
        print(f"\nTEMP LEFTOVERS: {len(temp_findings)} test process(es) left files in their TMPDIR "
              "(use lg_base::paths::test_dir / isolate_for_tests, never std::env::temp_dir())")
        print_findings(temp_findings, lambda o: o.temp)
    if repo_changes:
        print(f"\nREPOSITORY CHANGED by the test run: {len(repo_changes)} path(s)")
        for line in repo_changes:
            print(f"  {line}")
    if failures:
        print(f"\nTEST FAILURES: {len(failures)}")
        for f in sorted(failures, key=lambda o: (o.binary.label, o.test)):
            print(f"  {f.binary.label}  {f.test}")
            for ln in f.output.splitlines():
                print(f"      | {ln}")

    elapsed = time.monotonic() - t0
    print(f"\n{total} tests in {len(bins)} binaries, {len(findings)} isolation finding(s), {len(temp_findings)} temp leftover(s), "
          f"{len(repo_changes)} repo change(s), {len(failures)} failure(s), {elapsed:.0f}s")
    return 1 if (findings or temp_findings or repo_changes or failures) else 0


if __name__ == "__main__":
    sys.exit(main())
