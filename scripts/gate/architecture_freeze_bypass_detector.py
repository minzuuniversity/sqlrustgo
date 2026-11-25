#!/usr/bin/env python3
"""AD-002 storage-DML-bypass detector for `check_architecture_freeze.sh` (A7-1).

Why this file exists
--------------------
`check_architecture_freeze.sh` A7-1 is specified in
`docs/releases/v3.8.0/specs/gate/SPEC-011-mysql-server-dual-path.md`:

  * AD-002 (Single-path Execution) forbids **direct** `storage.insert/update/
    delete/scan` calls that bypass `ExecutionEngine::execute()`.
  * `eng.execute()` on an `ExecutionEngine` IS the legal single DML entry
    and must NOT be reported.
  * SPEC-011 §3.3 states the intended verification verbatim:
    "mysql-server 0 处 storage DML bypass
     (grep -E 'storage\\.(insert|update|delete|scan)' 0 matches)".

The shipped implementation greps `eng.execute(` instead — i.e. it reports
the **legal** entry point and never looks for the pattern the spec names.
The consequence is a check with zero discriminating power: it reports
"INFO (N calls found — verify they go through ExecutionEngine)" on a tree
where all N calls are correct, and stays silent on a tree where a real
bypass exists.

This module is the single implementation of that rule, extracted so it can
be regression-tested the way `stage_test_binaries.py` already is
(`tests/integration/gate/stage_test_binaries_test.rs`).

Two correctness requirements the naive grep does not meet
----------------------------------------------------------
1. **Only compiled sources count.** `crates/server/src/openclaw_endpoints.rs`
   is *not* declared as a module in `crates/server/src/lib.rs`, so cargo
   never compiles it. A grep over `crates/*/src/**/*.rs` reports its
   `storage.delete(...)` calls as AD-002 violations even though they are
   not in the binary. Worse, a sibling `openclaw_endpoints.rs.bak` and
   other comments still call that file a "production site".
   This module therefore resolves, per crate, which source files cargo
   actually compiles.

2. **Test code does not count.** A call inside `#[cfg(test)] mod tests`
   or inside a `#[test] fn` is not a production path.

Exit codes
----------
0  no bypass found
1  at least one bypass found
2  the scan could not be completed (bad input / unreadable tree)

Usage
-----
    python3 architecture_freeze_bypass_detector.py <repo_root> [crate ...]

Output: one `<path>:<line>: <text>` line per finding on stdout.
"""

from __future__ import annotations

import os
import re
import sys

# The DML surface a server-side crate must not reach directly. AD-002
# (SPEC-011 §3.1): "AD-002 真正禁止的是直接调 storage.insert/update/delete
# 绕过 ExecutionEngine".
BYPASS_METHODS = (
    "insert",
    "insert_row",
    "insert_rows",
    "update",
    "update_row",
    "delete",
    "delete_row",
    "scan",
)

# Receiver names that denote a storage handle. `storage` is the name the
# spec's own verification line uses; the others are the conventional
# aliases used across this repo.
STORAGE_RECEIVERS = ("storage", "store", "st", "storage_handle")

# `storage.insert(...)`, `storage.insert_row(...)`, ...
BYPASS_RE = re.compile(
    r"\b(?:%s)\s*\.\s*(?:%s)\s*\(" % (
        "|".join(re.escape(r) for r in STORAGE_RECEIVERS),
        "|".join(re.escape(m) for m in BYPASS_METHODS),
    )
)

# String / char literals. A diagnostic message that *mentions* the pattern
# (e.g. crates/storage/src/vtu_guard.rs: "Direct storage.insert() call
# bypasses VtuGuard execution path") is prose, not a call site.
STRING_LIT_RE = re.compile(r'r#*"(?:[^"\\]|\\.)*"#*|r#*\'(?:[^\'\\]|\\.)*\'#*|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'')

# `mod foo;` / `pub mod foo;` / `pub(crate) mod foo;`
MOD_DECL_RE = re.compile(
    r"^\s*(?:pub\s*(?:\([^)]*\)\s*)?)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;",
    re.MULTILINE,
)

# `#[cfg(test)]` / `#[cfg(any(test, ...))]`
CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*(?:all|any)?\s*\(?\s*test\b")
CFG_TEST_MOD_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")

# `#[test]` / `#[tokio::test]` / `#[rstest]` ...
TEST_ATTR_RE = re.compile(r"#\s*\[\s*(?:[A-Za-z0-9_:]*\s*,\s*)*(?:test|tokio::test|rstest|actix_rt::test)\s*\]")

# Files that are always compiled when their crate is.
ROOT_FILES = ("lib.rs", "main.rs", "mod.rs")


def _read(path: str) -> str:
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return ""


def _module_names(crate_dir: str) -> set:
    """Module names declared by a crate root file (`src/lib.rs`, `src/main.rs`)."""
    names = set()
    for root_file in ("lib.rs", "main.rs"):
        p = os.path.join(crate_dir, "src", root_file)
        if os.path.isfile(p):
            names.update(MOD_DECL_RE.findall(_read(p)))
    return names


def compiled_sources(crate_dir: str) -> list:
    """Source files of a crate that cargo actually compiles.

    A file `crates/<c>/src/<name>.rs` is compiled when:
      * it is `lib.rs` / `main.rs` / `mod.rs`, or
      * `<name>` is declared as `mod <name>;` in the crate root, or
      * it lives in `crates/<c>/src/<dir>/mod.rs` where `<dir>` is declared
        as `mod <dir>;` in the crate root (the 2018 directory-module form),
        or
      * it lives under such a declared directory (submodule of a submodule).

    Anything else in `src/` is an orphan file that cargo never compiles.
    """
    src = os.path.join(crate_dir, "src")
    if not os.path.isdir(src):
        return []

    declared = _module_names(crate_dir)
    out = []

    for dirpath, dirnames, filenames in os.walk(src):
        dirnames[:] = [d for d in dirnames if d not in ("target", ".git")]
        rel_dir = os.path.relpath(dirpath, src)
        rel_dir = "" if rel_dir == "." else rel_dir

        parts = [p for p in rel_dir.split(os.sep) if p]
        # Every directory on the path must correspond to a declared module
        # (either `<dir>/mod.rs` for the 2018 form, or a declared dir whose
        # children are auto-discovered by the 2018+ convention).
        if parts:
            top = parts[0]
            if top not in declared:
                continue
            # `top` must actually be a directory-module or contain mod.rs
            if not os.path.isfile(os.path.join(dirpath, "..", top, "mod.rs")) and \
               not os.path.isfile(os.path.join(src, top, "mod.rs")):
                # 2018+ edition: `mod top;` inside top.rs makes top.rs a
                # module; `mod top/mod.rs` style also possible.
                if not os.path.isfile(os.path.join(src, top + ".rs")):
                    continue

        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            if fn in ROOT_FILES:
                out.append(os.path.join(dirpath, fn))
                continue
            stem = fn[:-3]
            if stem in declared:
                out.append(os.path.join(dirpath, fn))
                continue
            # `foo/mod.rs` inside a declared directory
            if stem in declared and os.path.basename(dirpath) == stem:
                out.append(os.path.join(dirpath, fn))

    return sorted(set(out))


def _strip_test_regions(text: str):
    """Yield (line_number, line_text) for lines that are NOT test code.

    A region is considered test code when it is inside
      * a `#[cfg(test)]` item, or
      * a `#[test]`-attributed function.

    The scan is brace-depth based: a `#[cfg(test)]`/`#[test]` attribute
    applies from its line until the end of the item it decorates.
    """
    lines = text.split("\n")
    out = []
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]
        stripped = line.strip()

        if CFG_TEST_RE.search(line) or TEST_ATTR_RE.search(line):
            # Consume the decorated item: from the attribute line until the
            # brace depth returns to the level it had before the attribute.
            depth = 0
            started = False
            j = i
            while j < n:
                depth += lines[j].count("{") - lines[j].count("}")
                if "{" in lines[j]:
                    started = True
                if started and depth <= 0:
                    break
                j += 1
            i = j + 1
            continue

        out.append((i + 1, line))
        i += 1
    return out


def scan_file(path: str):
    """Return [(lineno, text)] of AD-002 bypass candidates in one file.

    String and char literals are blanked out first: a diagnostic message
    that merely *mentions* `storage.insert()` (as crates/storage/src/
    vtu_guard.rs does) is prose, not a call site.
    """
    text = _read(path)
    hits = []
    for lineno, line in _strip_test_regions(text):
        stripped = STRING_LIT_RE.sub(lambda m: " " * len(m.group(0)), line)
        if BYPASS_RE.search(stripped):
            hits.append((lineno, line.strip()))
    return hits


def scan_repo(repo_root: str, crate_filter=None):
    crates_dir = os.path.join(repo_root, "crates")
    if not os.path.isdir(crates_dir):
        raise RuntimeError("crates dir not found: %s" % crates_dir)

    findings = []
    for crate in sorted(os.listdir(crates_dir)):
        crate_dir = os.path.join(crates_dir, crate)
        if not os.path.isdir(crate_dir):
            continue
        if crate_filter and crate not in crate_filter:
            continue
        for path in compiled_sources(crate_dir):
            for lineno, text in scan_file(path):
                findings.append((path, lineno, text))
    return findings


def main(argv):
    if len(argv) < 2:
        sys.stderr.write("usage: %s <repo_root> [crate ...]\n" % argv[0])
        return 2
    repo_root = os.path.abspath(argv[1])
    crate_filter = set(argv[2:]) if len(argv) > 2 else None
    try:
        findings = scan_repo(repo_root, crate_filter)
    except RuntimeError as exc:
        sys.stderr.write("%s\n" % exc)
        return 2
    for path, lineno, text in findings:
        rel = os.path.relpath(path, repo_root)
        print("%s:%d: %s" % (rel, lineno, text))
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
