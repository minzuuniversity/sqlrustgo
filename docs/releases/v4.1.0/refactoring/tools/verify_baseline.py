#!/usr/bin/env python3
"""Offline validator for the frozen v4.1.0 implementation baseline.

Asserts the four baseline invariants (B1-B4):

  B1  the develop tip is an ancestor of HEAD
  B2  BASELINE_FREEZE.json matches the repository it was frozen from
  B3  the baseline's delta over develop is exactly the imported material
  B4  the cherry_picked ledger is complete and truthful

`base_sha` / `tree_sha` describe the **frozen content baseline**, i.e. the
commit whose tree carries every imported artifact.  BASELINE_FREEZE.json is
committed *on top* of that commit, so `base_sha` is an ancestor of HEAD and
cannot be HEAD itself: a commit cannot contain its own SHA.

Everything asserted here is derived from commits reachable inside this
branch, so the tool also works in a shallow or single-branch checkout.  The
cross-checks against the original source commits (45da6968ae, b681b74ca3,
9a61cf59c0, cfd4306b7e) are extra corroboration: they are skipped, with a
note, when those commits have not been fetched.
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
DEVELOP_REF = "origin/develop/v4.1.0"
NOTES = []


def note(message):
    """Record a non-fatal remark, once per distinct message."""
    if message not in NOTES:
        NOTES.append(message)


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def git_ok(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    ).returncode == 0


def have_commit(rev):
    return git_ok("cat-file", "-e", f"{rev}^{{commit}}")


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


def diff_paths(left, right):
    """Paths that differ between two commits."""
    out = subprocess.check_output(
        ["git", "diff", "--name-only", left, right], cwd=ROOT, text=True)
    return [line for line in out.splitlines() if line]


def develop_base(parent):
    """The develop tip the 'already-present' commits are compared against."""
    if git_ok("rev-parse", "--verify", DEVELOP_REF):
        return DEVELOP_REF
    note(f"{DEVELOP_REF} unavailable locally; using the recorded parent {parent}")
    return parent


def b1():
    assert git_ok("merge-base", "--is-ancestor", DEVELOP_TIP, "HEAD"), \
        f"develop tip {DEVELOP_TIP} is not an ancestor of HEAD"
    return f"develop tip {DEVELOP_TIP[:10]} is an ancestor of HEAD"


def b2(record):
    base = record["base_sha"]
    assert re.fullmatch(r"[0-9a-f]{40}", base), \
        f"base_sha is not a full 40-char sha: {base!r}"
    assert have_commit(base), f"base_sha {base} is not a commit of this repository"
    tree = git("rev-parse", f"{base}^{{tree}}")
    assert tree == record["tree_sha"], \
        f"tree_sha {record['tree_sha']} != the tree of base_sha ({tree})"
    assert git_ok("merge-base", "--is-ancestor", base, "HEAD"), \
        f"base_sha {base} is not an ancestor of HEAD"
    assert git_ok("merge-base", "--is-ancestor", record["parent"], base), \
        f"parent {record['parent']} is not an ancestor of base_sha"
    return f"base_sha {base[:10]} and tree_sha {tree[:10]} match the repository"


def b3(record):
    parent, base = record["parent"], record["base_sha"]
    assert have_commit(base), f"base_sha {base} is not a commit of this repository"
    path_import = next(e for e in record["cherry_picked"] if e["state"] == "path-import")
    delta = set(diff_paths(parent, base))
    runtime = {p for p in delta if p.startswith(path_import["path"])}
    material = delta - runtime
    assert len(material) == record["material_import"]["files"], \
        f"the baseline changes {len(material)} file(s) outside the imported runtime " \
        f"contract, the record says {record['material_import']['files']}"
    assert len(runtime) == path_import["files"], \
        f"the imported runtime contract holds {len(runtime)} file(s), the record says " \
        f"{path_import['files']}"
    head = set(tree_files("HEAD"))
    missing = sorted(p for p in delta if p not in head)
    assert not missing, \
        f"{len(missing)} imported file(s) are missing from HEAD: {missing[:5]}"
    if have_commit(MATERIAL_COMMIT):
        assert material == set(changed_paths(MATERIAL_COMMIT)), \
            f"the imported material is not the file set of {MATERIAL_COMMIT[:10]}"
    else:
        note(f"{MATERIAL_COMMIT[:10]} not fetched; skipped the source-side cross-check")
    if have_commit(RUNTIME_COMMIT):
        assert runtime == set(tree_files(RUNTIME_COMMIT, path_import["path"])), \
            f"the imported runtime contract is not the file set of {RUNTIME_COMMIT[:10]}"
    else:
        note(f"{RUNTIME_COMMIT[:10]} not fetched; skipped the source-side cross-check")
    return (f"{len(material)} material file(s) + {len(runtime)} runtime-contract file(s) "
            f"imported over develop, all present in HEAD")


def b4(record):
    entries = record["cherry_picked"]
    states = [e["state"] for e in entries]
    assert len(entries) == 5, f"expected 5 cherry_picked entries, got {len(entries)}"
    for wanted in ("applied", "path-import", "already-present"):
        expected = {"applied": 2, "path-import": 1, "already-present": 2}[wanted]
        assert states.count(wanted) == expected, \
            f"expected {expected} '{wanted}' entries, ledger has {states.count(wanted)}"
    for entry in entries:
        assert re.fullmatch(r"[0-9a-f]{40}", entry["sha"]), \
            f"cherry_picked sha is not a full 40-char sha: {entry['sha']!r}"
    subjects = git("log", "--format=%s", "HEAD").splitlines()
    for entry in entries:
        if entry["state"] != "applied":
            continue
        if not have_commit(entry["sha"]):
            note(f"{entry['sha'][:10]} not fetched; skipped the source-side cross-check")
            continue
        subject = git("log", "-1", "--format=%s", entry["sha"])
        assert subject in subjects, \
            f"the cherry-pick of {entry['sha'][:10]} ({subject!r}) is not in HEAD's history"
    path_import = next(e for e in entries if e["state"] == "path-import")
    assert path_import["sha"] == RUNTIME_COMMIT, \
        f"path-import sha {path_import['sha']} is not {RUNTIME_COMMIT}"
    in_head = tree_files("HEAD", path_import["path"])
    assert len(in_head) == path_import["files"], \
        f"path-import directory holds {len(in_head)} file(s), the record says {path_import['files']}"
    base = develop_base(record["parent"])
    for entry in entries:
        if entry["state"] != "already-present":
            continue
        if not have_commit(entry["sha"]):
            note(f"{entry['sha'][:10]} not fetched; skipped the source-side cross-check")
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
    try:
        record = json.loads(FREEZE.read_text())
    except ValueError as exc:
        print(f"FAIL  {FREEZE_REL} is not valid JSON: {exc}")
        print("0/4 PASS")
        return 1

    checks = (("B1", b1), ("B2", lambda: b2(record)), ("B3", lambda: b3(record)),
              ("B4", lambda: b4(record)))
    passed = 0
    for name, check in checks:
        try:
            detail = check()
        except AssertionError as exc:
            print(f"[{name}] FAIL  {exc}")
        except Exception as exc:  # a broken record must fail closed, not crash
            print(f"[{name}] FAIL  {type(exc).__name__}: {exc}")
        else:
            passed += 1
            print(f"[{name}] PASS  {detail}")
    for message in NOTES:
        print(f"note: {message}")
    print(f"{passed}/{len(checks)} PASS")
    return 0 if passed == len(checks) else 1


if __name__ == "__main__":
    sys.exit(main())
