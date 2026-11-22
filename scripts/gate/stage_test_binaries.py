#!/usr/bin/env python3
"""Stage the test binaries cargo just built to the paths the test-runner manifest names.

Why this exists
---------------
`check_anti_fabrication.sh` (CHECK 1.5, V312-24) runs a handful of test
binaries through `test-runner` and reports PASS/FAIL. Those binaries must be
built from **the tree under test**. The previous staging step did not
guarantee that:

1. it guessed which artifact was the right one — "scan `deps/`, take the
   one with the newest mtime";
2. and when the destination path already existed it **skipped the copy
   entirely** and used the old file as the evidence.

Together those two behaviours let the gate **certify a stale binary**: the
same source tree was tested, but with a binary built from an earlier
revision.

**Measured 2026-11-17.** The VtuGuard transaction-passthrough fix
(`1f9425134d`, 04:14) landed, and the gate still reported

    vtu_guard::tests::test_vtu_guard_tx_state_via_set_current_tx_id ... FAILED
    assertion failed: guarded.in_transaction()

The binary it ran was the 02:30 one — built **before** the fix. cargo had
already produced a fixed binary at 05:31; it just was never copied over.
The fix itself was fine (the fresh binary runs that test green); the gate
was measuring the previous revision.

What this does instead
-----------------------
cargo is asked for the answer (`--message-format=json`), not guessed:

- every `compiler-artifact` message with `profile.test == true` gives the
  **exact** executable path cargo just produced;
- that executable is copied to the hashless `target/release/<name>` path the
  manifest names — **always overwriting**;
- and the copy is refused when the artifact is **older than its own
  sources** (the package's `Cargo.toml`, its `src/**`, and the test target's
  own file). Refusing means the gate fails closed instead of certifying
  the previous revision.

Usage
-----
    stage_test_binaries.py <cargo-build-report.json> <manifest.toml>

Prints the staged binary names on stdout (space separated). Exits non-zero
— with one `STAGING_ERROR:` line per problem on stderr — when anything
could not be staged from this build.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import sys


def _mtime(path: str) -> float:
    try:
        return os.path.getmtime(path)
    except OSError:
        return 0.0


def newest_source_mtime(artifact: dict) -> float:
    """Newest mtime among the sources this artifact was built from."""
    newest = 0.0
    manifest_path = artifact.get("manifest_path")
    if manifest_path:
        newest = max(newest, _mtime(manifest_path))
        srcdir = os.path.join(os.path.dirname(manifest_path), "src")
        for root, _dirs, files in os.walk(srcdir):
            for name in files:
                if name.endswith(".rs"):
                    newest = max(newest, _mtime(os.path.join(root, name)))
    target = artifact.get("target") or {}
    if target.get("src_path"):
        newest = max(newest, _mtime(target["src_path"]))
    return newest


def built_test_binaries(build_json: str) -> dict[str, dict]:
    """Map target name -> the `compiler-artifact` message cargo produced for it."""
    built: dict[str, dict] = {}
    with open(build_json) as fh:
        for line in fh:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                msg = json.loads(line)
            except ValueError:
                continue
            if msg.get("reason") != "compiler-artifact":
                continue
            if not msg.get("executable"):
                continue
            if not (msg.get("profile") or {}).get("test"):
                continue
            name = ((msg.get("target") or {}).get("name") or "").replace("-", "_")
            if name:
                built[name] = msg
    return built


def manifest_binaries(manifest: str) -> list[str]:
    wanted: list[str] = []
    try:
        with open(manifest) as fh:
            for line in fh:
                m = re.search(r'binary\s*=\s*"([^"]+)"', line)
                if m:
                    wanted.append(m.group(1))
    except OSError:
        pass
    return wanted


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(
            "usage: stage_test_binaries.py <cargo-build-report.json> <manifest.toml>",
            file=sys.stderr,
        )
        return 2

    build_json, manifest = argv[1], argv[2]
    repo = os.getcwd()

    try:
        built = built_test_binaries(build_json)
    except OSError as exc:
        print(f"cannot read the cargo build report: {exc}", file=sys.stderr)
        return 1

    staged: list[str] = []
    errors: list[str] = []

    for rel in manifest_binaries(manifest):
        dst = os.path.join(repo, rel)
        name = os.path.basename(rel)
        artifact = built.get(name.replace("-", "_"))
        if artifact is None:
            errors.append(
                f"{name}: cargo did not build a test binary for it in this run"
            )
            continue
        exe = artifact["executable"]
        if not (os.path.isfile(exe) and os.access(exe, os.X_OK)):
            errors.append(f"{name}: cargo-reported executable is missing: {exe}")
            continue
        art_mtime = _mtime(exe)
        src_mtime = newest_source_mtime(artifact)
        if src_mtime > art_mtime:
            errors.append(
                f"{name}: STALE artifact — built {src_mtime - art_mtime:.0f}s BEFORE "
                "its own newest source; certifying it would be certifying the "
                "previous revision"
            )
            continue
        # Always overwrite: an existing destination is not evidence of
        # anything, and reusing it is exactly how a stale binary got
        # certified in the first place.
        shutil.copy2(exe, dst)
        staged.append(name)

    print(" ".join(sorted(set(staged))))
    for e in errors:
        print("STAGING_ERROR: " + e, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
