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

Every counting and enumerating assertion is pinned to the frozen `base_sha`,
never to HEAD.  HEAD moves as later tasks build on the baseline, and a
validator that counts files in a moving ref goes red for reasons that have
nothing to do with baseline integrity.

The HEAD side of B3 asserts PATH PRESENCE, not blob identity.  The freeze
record freezes *identity* (which baseline, which material, which commit) --
content is supposed to evolve as later tasks legitimately re-point bindings.
Who owns which imported path:

  * Task 2 (前向/历史 SHA 重指) rewrites these imported files **in place**,
    so their path survives but their blob does not:
      - docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json
        (tasks[].base_sha)
      - docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json
        (cases[].base_sha)
      - docs/refactor/v4.1.0-c/handoff/03_TASK_MANIFEST.jsonl (base_commit)
      - docs/releases/v4.1.0/refactoring/test-factory-design/14_HANDOFF_MANIFEST.json
        (base_sha)
      - plus the handoff manifests that carry their hashes
        (01_ARCHITECTURE_BASELINE.json, 04_TASK_DEPENDENCY_DAG.json,
        10_HANDOFF_MANIFEST.json) and evidence/SOURCE_BINDINGS.json
  * Task 4 (ADR 批准落地) **relocates** ADR-410-011 out of
    runtime-contract/20261120-pr5915/ into refactoring/adrs/ and rewrites its
    Status / Approvals lines -- so neither the old path nor the frozen blob
    survives; see RELOCATABLE below.
  * Content-level guarantees for the rewritable files belong to the
    downstream per-task validators (verify_sha_scope.py, verify_matrix.py,
    verify_handoff.py), not here.

The blob-level comparison B3 does against the *source* commits is a
frozen-versus-frozen comparison (45da6968ae vs the frozen base_sha): it
proves the import itself was faithful and can never be affected by what a
later task rewrites.

Source-side cross-checks (comparing the baseline against the original source
commits 45da6968ae / b681b74ca3 / 9a61cf59c0 / cfd4306b7e) can only run when
those commits are reachable.  When they are not, the check is *skipped* and
reported as such: a cross-check that could not run is never reported as one
that was verified.  `--strict` turns any skipped cross-check into a failure.

The invariants are raised explicitly instead of being written as `assert`
statements, because `python -O` strips `assert` and would turn this gate into
a no-op that prints "4/4 PASS" for anything.
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys
from datetime import datetime

ROOT = pathlib.Path(__file__).resolve().parents[5]
FREEZE_REL = "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json"
FREEZE = ROOT / FREEZE_REL
DEVELOP_TIP = "8cbde5b6d33c876fa1197474c46d50142a9823bd"
MATERIAL_COMMIT = "45da6968ae2baaade6a31206eeee5854a8514ee2"
RUNTIME_COMMIT = "b681b74ca3c0373cb2b7c44ff2cb032cf1a1c540"

# Imported paths that a downstream task legitimately relocates, with the task
# that owns them.  Such a path may leave its original directory *and* be
# rewritten, so neither its path nor its frozen blob has to survive in HEAD;
# the relocation is recognised by the file name still being present somewhere.
# A plain deletion (the name gone everywhere) still turns B3 red.
RELOCATABLE = {
    "docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/"
    "ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md":
        "Task 4 moves it into docs/releases/v4.1.0/refactoring/adrs/ and "
        "rewrites its Status / Approvals lines",
}

SKIPPED = []
NOTES = []


class BaselineError(Exception):
    """A frozen-baseline invariant does not hold."""


def require(condition, message):
    """Assert-like helper that survives `python -O` (which strips `assert`)."""
    if not condition:
        raise BaselineError(message)


def skip(check, detail):
    """Record a cross-check that could not run (never reported as verified).

    `check` is the assertion id (B1..B4) the skipped cross-check belongs to, so
    the summary can say which check was only partially verified.
    """
    SKIPPED.append((check, detail))


def note(message):
    """Record an informational remark, once per distinct message."""
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


def tree_entries(rev, paths=None):
    """(mode, kind, blob) per path of `rev`'s tree, optionally restricted."""
    args = ["ls-tree", "-r", rev]
    if paths:
        args += ["--", *(paths if isinstance(paths, (list, tuple)) else [paths])]
    out = subprocess.check_output(["git", *args], cwd=ROOT, text=True)
    entries = {}
    for line in out.splitlines():
        meta, name = line.split("\t", 1)
        mode, kind, blob = meta.split()
        entries[name] = (mode, kind, blob)
    return entries


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


def path_import_entry(record):
    entries = [e for e in record["cherry_picked"] if e["state"] == "path-import"]
    require(len(entries) == 1, f"expected exactly one path-import entry, got {len(entries)}")
    return entries[0]


def compare_blobs(source_rev, base_rev, paths, label):
    """The baseline must carry the source commit's bytes, unchanged.

    Frozen-versus-frozen comparison (source commit vs the frozen base_sha):
    it proves the *import* was faithful and is unaffected by what a later
    task rewrites in HEAD.
    """
    source = tree_entries(source_rev, paths)
    landed = tree_entries(base_rev, paths)
    for path in paths:
        require(path in source, f"{label} file {path} is absent from {source_rev[:10]}")
        require(path in landed, f"{label} file {path} is absent from the frozen baseline {base_rev[:10]}")
        require(source[path] == landed[path],
                f"{label} file {path} was altered during import: "
                f"{source[path][2][:12]} != {landed[path][2][:12]}")


def missing_from_head(delta, frozen, head):
    """Imported paths that are no longer accounted for in HEAD's tree.

    Identity, not content: an imported path counts as present when

      1. the path is still in HEAD -- its content may have been legitimately
         rewritten in place (Task 2 re-points the forward bindings), so the
         blob is deliberately not compared here;
      2. its frozen blob is still in HEAD under another path -- the file was
         relocated but not rewritten;
      3. it is a declared relocation (RELOCATABLE) and the file name is still
         in HEAD -- relocated *and* rewritten (Task 4 moves ADR-410-011 into
         adrs/ and edits its Status / Approvals lines).

    Anything else is a genuine deletion.
    """
    by_blob = {}
    by_name = {}
    for path, entry in head.items():
        by_blob.setdefault(entry[2], set()).add(path)
        by_name.setdefault(path.rsplit("/", 1)[-1], set()).add(path)
    missing = []
    for path in delta:
        if path in head:
            continue
        if frozen[path][2] in by_blob:
            continue
        if path in RELOCATABLE and by_name.get(path.rsplit("/", 1)[-1]):
            continue
        missing.append(path)
    return sorted(missing)


def b1():
    require(git_ok("merge-base", "--is-ancestor", DEVELOP_TIP, "HEAD"),
            f"develop tip {DEVELOP_TIP} is not an ancestor of HEAD")
    return f"develop tip {DEVELOP_TIP[:10]} is an ancestor of HEAD"


def b2(record):
    base = record["base_sha"]
    require(re.fullmatch(r"[0-9a-f]{40}", base),
            f"base_sha is not a full 40-char sha: {base!r}")
    require(have_commit(base), f"base_sha {base} is not a commit of this repository")
    tree = git("rev-parse", f"{base}^{{tree}}")
    require(tree == record["tree_sha"],
            f"tree_sha {record['tree_sha']} != the tree of base_sha ({tree})")
    require(git_ok("merge-base", "--is-ancestor", base, "HEAD"),
            f"base_sha {base} is not an ancestor of HEAD")
    require(git_ok("merge-base", "--is-ancestor", record["parent"], base),
            f"parent {record['parent']} is not an ancestor of base_sha")
    ref = record["source_ref"]
    resolved = next((r for r in (ref, f"origin/{ref}")
                     if git_ok("rev-parse", "--verify", f"{r}^{{commit}}")), None)
    require(resolved is not None, f"source_ref {ref} does not resolve to a commit")
    if resolved != ref:
        note(f"source_ref {ref} is not a local ref; verified {resolved} instead")
    try:
        datetime.fromisoformat(record["frozen_at"])
    except (TypeError, ValueError) as exc:
        raise BaselineError(
            f"frozen_at {record['frozen_at']!r} is not an ISO-8601 timestamp: {exc}")
    return (f"base_sha {base[:10]}, tree_sha {tree[:10]}, source_ref {ref} and "
            f"frozen_at all match the repository")


def b3(record):
    parent, base = record["parent"], record["base_sha"]
    require(have_commit(base), f"base_sha {base} is not a commit of this repository")
    path_import = path_import_entry(record)
    delta = set(diff_paths(parent, base))
    runtime = {p for p in delta if p.startswith(path_import["path"])}
    material = delta - runtime
    require(len(material) == record["material_import"]["files"],
            f"the baseline changes {len(material)} file(s) outside the imported runtime "
            f"contract, the record says {record['material_import']['files']}")
    require(len(runtime) == path_import["files"],
            f"the imported runtime contract holds {len(runtime)} file(s), the record says "
            f"{path_import['files']}")
    if have_commit(MATERIAL_COMMIT):
        require(material == set(changed_paths(MATERIAL_COMMIT)),
                f"the imported material is not the file set of {MATERIAL_COMMIT[:10]}")
        compare_blobs(MATERIAL_COMMIT, base, sorted(material), "material")
    else:
        skip("B3", f"material: {MATERIAL_COMMIT[:10]} is not reachable")
    if have_commit(RUNTIME_COMMIT):
        require(runtime == set(tree_entries(RUNTIME_COMMIT, path_import["path"])),
                f"the imported runtime contract is not the file set of {RUNTIME_COMMIT[:10]}")
        compare_blobs(RUNTIME_COMMIT, base, sorted(runtime), "runtime contract")
    else:
        skip("B3", f"runtime-contract: {RUNTIME_COMMIT[:10]} is not reachable")
    missing = missing_from_head(delta, tree_entries(base), tree_entries("HEAD"))
    require(not missing,
            f"{len(missing)} imported path(s) are no longer present in HEAD: {missing[:5]}")
    return (f"{len(material)} material file(s) + {len(runtime)} runtime-contract file(s) "
            f"imported over develop, all still present in HEAD")


def check_applied(entry, base):
    """An 'applied' entry must have landed its content in the frozen baseline."""
    sha, cherry = entry["sha"], entry.get("cherry_pick")
    require(cherry, f"applied entry {sha[:10]} does not record the cherry-pick commit carrying it")
    require(re.fullmatch(r"[0-9a-f]{40}", cherry),
            f"cherry_pick {cherry!r} is not a full 40-char sha")
    require(have_commit(cherry), f"cherry-pick commit {cherry[:10]} is not a commit of this repository")
    require(git_ok("merge-base", "--is-ancestor", cherry, "HEAD"),
            f"cherry-pick commit {cherry[:10]} is not in HEAD's history")
    require(diff_paths(f"{cherry}^", cherry),
            f"cherry-pick commit {cherry[:10]} carries no change but its entry is 'applied'")
    if not have_commit(sha):
        skip("B4", f"applied: {sha[:10]} is not reachable; skipped the blob comparison")
        return
    paths = changed_paths(sha)
    require(paths, f"applied commit {sha[:10]} touches no file")
    compare_blobs(sha, base, paths, "applied")


def check_already_present(entry, record):
    """An 'already-present' entry must not have changed the baseline."""
    sha = entry["sha"]
    base, parent = record["base_sha"], record["parent"]
    if not have_commit(sha):
        skip("B4", f"already-present: {sha[:10]} is not reachable; skipped the comparison")
        return
    paths = changed_paths(sha)
    require(paths, f"commit {sha[:10]} touches no file")
    drift = git("diff", "HEAD", "--", *paths)
    require(not drift,
            f"the working tree drifted away from the frozen baseline for {sha[:10]}: "
            f"{drift.splitlines()[:5]}")
    frozen_parent = tree_entries(parent, paths)
    frozen_base = tree_entries(base, paths)
    for path in paths:
        require(path in frozen_parent, f"{path} is absent from the frozen parent {parent[:10]}")
        require(path in frozen_base, f"{path} is absent from the frozen baseline {base[:10]}")
        require(frozen_parent[path] == frozen_base[path],
                f"already-present commit {sha[:10]}: {path} differs between the frozen "
                f"parent and the baseline")
    cherry = entry.get("cherry_pick")
    if cherry:
        require(have_commit(cherry),
                f"cherry-pick commit {cherry[:10]} is not a commit of this repository")
        require(not diff_paths(f"{cherry}^", cherry),
                f"the cherry-pick commit {cherry[:10]} is not empty but the entry declares "
                f"net_delta=empty")
    elif entry.get("net_delta") == "empty":
        source = tree_entries(sha, paths)
        for path in paths:
            require(source[path] == frozen_parent[path],
                    f"already-present commit {sha[:10]}: {path} is not identical in the "
                    f"frozen parent, so skipping it was not a no-op")


def b4(record):
    entries = record["cherry_picked"]
    states = [e["state"] for e in entries]
    require(len(entries) == 5, f"expected 5 cherry_picked entries, got {len(entries)}")
    for wanted, expected in (("applied", 1), ("path-import", 1), ("already-present", 3)):
        require(states.count(wanted) == expected,
                f"expected {expected} '{wanted}' entries, ledger has {states.count(wanted)}")
    for entry in entries:
        require(re.fullmatch(r"[0-9a-f]{40}", entry["sha"]),
                f"cherry_picked sha is not a full 40-char sha: {entry['sha']!r}")
    base = record["base_sha"]
    for entry in entries:
        if entry["state"] == "applied":
            check_applied(entry, base)
        elif entry["state"] == "already-present":
            check_already_present(entry, record)
    path_import = path_import_entry(record)
    require(path_import["sha"] == RUNTIME_COMMIT,
            f"path-import sha {path_import['sha']} is not {RUNTIME_COMMIT}")
    frozen = tree_entries(base, [path_import["path"]])
    require(len(frozen) == path_import["files"],
            f"the frozen path-import directory holds {len(frozen)} file(s), the record says "
            f"{path_import['files']}")
    return ("ledger complete (applied 1 / path-import 1 / already-present 3); the frozen "
            f"path-import directory holds {len(frozen)} file(s)")


def main():
    parser = argparse.ArgumentParser(
        description="Verify the frozen v4.1.0 implementation baseline (B1-B4).")
    parser.add_argument("--strict", action="store_true",
                        help="fail when a source-side cross-check could not be run")
    args = parser.parse_args()

    if sys.flags.optimize:
        print("refusing to run under python -O: assert statements are stripped, "
              "this gate would be a no-op")
        return 2
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
    results = []
    for name, check in checks:
        try:
            detail = check()
        except BaselineError as exc:
            results.append((name, "FAIL", str(exc), ""))
        except Exception as exc:  # a broken record must fail closed, not crash
            results.append((name, "FAIL", f"{type(exc).__name__}: {exc}", ""))
        else:
            skipped_here = sum(1 for cid, _ in SKIPPED if cid == name)
            scope = (f"[{skipped_here} cross-check(s) skipped - not verified]"
                     if skipped_here else "")
            results.append((name, "PASS" if not skipped_here else "PARTIAL",
                            detail, scope))
    for name, status, detail, scope in results:
        print(f"[{name}] {status}  {detail}" + (f"  {scope}" if scope else ""))
    for message in NOTES:
        print(f"note: {message}")
    for check_id, detail in SKIPPED:
        print(f"skipped: {check_id} {detail}")
    tally = {}
    for _, status, _, _ in results:
        tally[status] = tally.get(status, 0) + 1
    verdict = ", ".join(f"{tally[status]}/{len(checks)} {status}"
                        for status in ("PASS", "PARTIAL", "FAIL") if tally.get(status))
    if SKIPPED:
        verdict += (f" -- {len(SKIPPED)} source-side cross-check(s) SKIPPED "
                    f"(not verified)")
        if args.strict:
            verdict += " -- STRICT FAIL"
            print(verdict)
            return 1
    else:
        verdict += " -- COMPLETE: every assertion ran"
    print(verdict)
    return 0 if not tally.get("FAIL") else 1


if __name__ == "__main__":
    sys.exit(main())
