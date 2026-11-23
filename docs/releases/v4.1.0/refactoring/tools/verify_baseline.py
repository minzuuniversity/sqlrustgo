#!/usr/bin/env python3
"""Offline validator for the frozen v4.1.0 implementation baseline.

Asserts the four baseline invariants (B1-B4):

  B1  the develop tip is an ancestor of HEAD
  B2  BASELINE_FREEZE.json matches the repository it was frozen from
  B3  every imported material file is present in the HEAD tree
  B4  the cherry_picked ledger is complete and truthful

`base_sha` / `tree_sha` describe the **frozen content baseline**, i.e. the
commit whose tree carries every imported artifact.  BASELINE_FREEZE.json is
committed *on top* of that commit, so `base_sha` is an ancestor of HEAD and
cannot be HEAD itself: a commit cannot contain its own SHA.
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[5]
FREEZE_REL = "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json"
FREEZE = ROOT / FREEZE_REL
DEVELOP_TIP = "8cbde5b6d33c876fa1197474c46d50142a9823bd"
MATERIAL_COMMIT = "45da6968ae2baaade6a31206eeee5854a8514ee2"
RUNTIME_COMMIT = "b681b74ca3c0373cb2b7c44ff2cb032cf1a1c540"
RUNTIME_DIR = "docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/"
DEVELOP_REF = "origin/develop/v4.1.0"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def git_ok(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    ).returncode == 0


def tree_files(rev, path=None):
    """File names of `rev`'s tree, optionally restricted to `path`."""
    args = ["ls-tree", "-r", "--name-only", rev]
    if path:
        args += ["--", path]
    out = subprocess.check_output(["git", *args], cwd=ROOT, text=True)
    return [line for line in out.splitlines() if line]


def changed_paths(rev):
    """Paths touched by commit `rev`."""
    out = subprocess.check_output(
        ["git", "show", "--name-only", "--format=", rev], cwd=ROOT, text=True)
    return [line for line in out.splitlines() if line]


def develop_base(parent):
    """The develop tip the 'already-present' commits are compared against."""
    if git_ok("rev-parse", "--verify", DEVELOP_REF):
        return DEVELOP_REF
    print(f"note: {DEVELOP_REF} unavailable locally; using the recorded parent {parent}")
    return parent


def b1():
    assert git_ok("merge-base", "--is-ancestor", DEVELOP_TIP, "HEAD"), \
        f"develop tip {DEVELOP_TIP} is not an ancestor of HEAD"
    return f"develop tip {DEVELOP_TIP[:10]} is an ancestor of HEAD"


def b2(record):
    base = record["base_sha"]
    assert re.fullmatch(r"[0-9a-f]{40}", base), \
        f"base_sha is not a full 40-char sha: {base!r}"
    assert git_ok("cat-file", "-e", f"{base}^{{commit}}"), \
        f"base_sha {base} is not a commit of this repository"
    tree = git("rev-parse", f"{base}^{{tree}}")
    assert tree == record["tree_sha"], \
        f"tree_sha {record['tree_sha']} != the tree of base_sha ({tree})"
    assert git_ok("merge-base", "--is-ancestor", base, "HEAD"), \
        f"base_sha {base} is not an ancestor of HEAD"
    assert git_ok("merge-base", "--is-ancestor", record["parent"], base), \
        f"parent {record['parent']} is not an ancestor of base_sha"
    return f"base_sha {base[:10]} and tree_sha {tree[:10]} match the repository"


def b3(record):
    head = set(tree_files("HEAD"))
    material = changed_paths(MATERIAL_COMMIT)
    assert len(material) == record["material_import"]["files"], \
        f"{MATERIAL_COMMIT} touches {len(material)} file(s), the record says " \
        f"{record['material_import']['files']}"
    missing = sorted(p for p in material if p not in head)
    assert not missing, \
        f"{len(missing)} file(s) of {MATERIAL_COMMIT[:10]} are missing from HEAD: {missing[:5]}"
    runtime = tree_files(RUNTIME_COMMIT, RUNTIME_DIR)
    missing_runtime = sorted(p for p in runtime if p not in head)
    assert not missing_runtime, \
        f"{len(missing_runtime)} file(s) of {RUNTIME_COMMIT[:10]} are missing from HEAD: " \
        f"{missing_runtime[:5]}"
    return (f"{len(material)} material file(s) of {MATERIAL_COMMIT[:10]} + "
            f"{len(runtime)} runtime-contract file(s) of {RUNTIME_COMMIT[:10]} present in HEAD")


def b4(record):
    entries = record["cherry_picked"]
    states = [e["state"] for e in entries]
    assert len(entries) == 5, f"expected 5 cherry_picked entries, got {len(entries)}"
    for wanted in ("applied", "path-import", "already-present"):
        assert states.count(wanted) == {"applied": 2, "path-import": 1,
                                         "already-present": 2}[wanted], \
            f"expected {states.count(wanted)} '{wanted}' entries, ledger has {states.count(wanted)}"
    for entry in entries:
        assert re.fullmatch(r"[0-9a-f]{40}", entry["sha"]), \
            f"cherry_picked sha is not a full 40-char sha: {entry['sha']!r}"
    subjects = git("log", "--format=%s", "HEAD").splitlines()
    for entry in entries:
        if entry["state"] != "applied":
            continue
        subject = git("log", "-1", "--format=%s", entry["sha"])
        assert subject in subjects, \
            f"the cherry-pick of {entry['sha'][:10]} ({subject!r}) is not in HEAD's history"
    base = develop_base(record["parent"])
    for entry in entries:
        if entry["state"] != "already-present":
            continue
        paths = changed_paths(entry["sha"])
        assert paths, f"commit {entry['sha'][:10]} touches no file"
        diff = git("diff", base, "HEAD", "--", *paths)
        assert not diff, \
            f"already-present commit {entry['sha'][:10]} still differs from {base}: " \
            f"{diff.splitlines()[:5]}"
        drift = git("diff", "HEAD", "--", *paths)
        assert not drift, \
            f"the working tree drifted away from the frozen baseline for " \
            f"{entry['sha'][:10]}: {drift.splitlines()[:5]}"
    path_import = next(e for e in entries if e["state"] == "path-import")
    assert path_import["sha"] == RUNTIME_COMMIT, \
        f"path-import sha {path_import['sha']} is not {RUNTIME_COMMIT}"
    imported = set(tree_files(RUNTIME_COMMIT, path_import["path"]))
    in_head = set(tree_files("HEAD", path_import["path"]))
    assert imported == in_head, \
        f"imported directory differs from HEAD: {sorted(imported ^ in_head)[:5]}"
    assert len(in_head) == path_import["files"], \
        f"path-import directory holds {len(in_head)} file(s), the record says {path_import['files']}"
    return ("ledger complete (applied 2 / path-import 1 / already-present 2); "
            f"path-import directory holds {len(in_head)} file(s)")


def main():
    parser = argparse.ArgumentParser(
        description="Verify the frozen v4.1.0 implementation baseline (B1-B4).")
    parser.add_argument("--check", action="store_true",
                        help="run the B1-B4 assertions (default behaviour)")
    parser.parse_args()

    if not FREEZE.is_file():
        print(f"FAIL  {FREEZE_REL} does not exist")
        print("0/4 PASS")
        return 1
    record = json.loads(FREEZE.read_text())

    checks = (("B1", b1), ("B2", lambda: b2(record)), ("B3", lambda: b3(record)),
              ("B4", lambda: b4(record)))
    passed = 0
    for name, check in checks:
        try:
            detail = check()
        except AssertionError as exc:
            print(f"[{name}] FAIL  {exc}")
        else:
            passed += 1
            print(f"[{name}] PASS  {detail}")
    print(f"{passed}/{len(checks)} PASS")
    return 0 if passed == len(checks) else 1


if __name__ == "__main__":
    sys.exit(main())
