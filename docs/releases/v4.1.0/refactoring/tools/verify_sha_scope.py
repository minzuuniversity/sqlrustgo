#!/usr/bin/env python3
"""Scope validator for the v4.1.0 SHA re-point (Task 2).

The repository carries two kinds of SHA binding, and they must move
differently:

  *forward-looking* -- the bindings that describe work still to be done.  They
    must all point at the single frozen implementation baseline, i.e.
    `base_sha` in BASELINE_FREEZE.json.

  *historical* -- the bindings that record what already happened.  They must
    keep their original values; re-pointing them would rewrite history.

Six assertions:

  S1  REFACTOR_TASK_DAG.json      18 x tasks[].base_sha  == BASE, while
      artifact_context.source_code_base_sha keeps the pre-freeze base
  S2  TEST_COVERAGE_MATRIX.json   75 x cases[].base_sha  == BASE
  S3  03_TASK_MANIFEST.jsonl      18 x base_commit       == BASE, and the
      task_id order matches the DAG's task_id order one-to-one
  S4  14_HANDOFF_MANIFEST.json    base_sha              == BASE, while
      adr_base_commit keeps the ADR text's own commit identity
  S5  audit/v4.1.0/commands_pin_*.yaml and audit/v4.1.0/runs/*/manifest.yaml
      keep their original bytes and their original frozen_commit bindings
  S6  the tracked pilot rerun summary still points at the pre-freeze base

`BASE` is read from BASELINE_FREEZE.json rather than hardcoded here, so this
gate follows the freeze record instead of keeping a second copy of it.

S5 derives the historical set from the frozen baseline (`git ls-tree` at
BASE), not from a working-tree glob: a glob would silently shrink when a
historical record is deleted, and the shrunken set would then be reported as
if it were the expected one.  The set is frozen -- a *new* audit run manifest
must be added to the freeze record (or the gate must be re-ruled), it must
not silently widen the set.

S5 compares the historical records against the frozen baseline (which is an
ancestor of HEAD and therefore holds the bytes as they stood before this
re-point), and additionally compares every `frozen_commit` binding against the
commit the material was imported from.  The two references differ on purpose:
a few run manifests were corrected *after* that commit without touching their
`frozen_commit`, and six of them were added after it, so a byte comparison
against that commit alone would report a false drift.

The verdict is bound to a commit: the checks read the working tree, so the
working tree must be identical to HEAD.  A green run on a dirty worktree
would be a statement about uncommitted bytes, not about the commit a receipt
names (C5).  The tool therefore refuses to run on a worktree with pending
edits -- that is the intended fail-closed behaviour, not a limitation.

The invariants are raised explicitly instead of being written as `assert`
statements, because `python -O` strips `assert` and would turn this gate into
a no-op that prints "6/6 PASS" for anything.
"""
import argparse
import fnmatch
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[5]
FREEZE_REL = "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json"
FREEZE = ROOT / FREEZE_REL

DAG_REL = "docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json"
MATRIX_REL = "docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json"
TASK_MANIFEST_REL = "docs/refactor/v4.1.0-c/handoff/03_TASK_MANIFEST.jsonl"
HANDOFF_REL = "docs/releases/v4.1.0/refactoring/test-factory-design/14_HANDOFF_MANIFEST.json"

# The commit the imported material came from -- and the commit the ADR texts
# were authored in, so it doubles as the historical reference for the ADR
# provenance binding (S4).
MATERIAL_COMMIT = "45da6968ae2baaade6a31206eeee5854a8514ee2"

# The base every historical binding still points at: the develop tip as it was
# before the freeze.  S1 pins the test-factory package's provenance to it and
# S6 pins the pilot rerun's base to it; re-pointing either is an
# owner-ratified edit of this constant and of the field it guards.
PRE_FREEZE_BASE_SHA = "e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0"

# S6's subject: the tracked, in-repo copy of the pilot rerun summary.  It is
# the only copy every checkout has, so it is the copy that is asserted.
PILOT_EVIDENCE_REL = "evidence/rerun-summary-20261120.json"
# The pilot's own working copy, outside this repository.  It is a redundant
# witness: when it is there it must be byte-identical to the tracked one;
# when the pilot checkout does not exist at all the cross-check is reported as
# skipped (never as verified); when the checkout exists but the file is gone
# that is tampering and fails hard.
PILOT_OUT_OF_REPO_ROOT = pathlib.Path("/Volumes/workspace/dev/srg-test-factory-pilot")
PILOT_OUT_OF_REPO = PILOT_OUT_OF_REPO_ROOT / "evidence/rerun-summary-20261120.json"

DAG_TASKS = 18
MATRIX_CASES = 75
TASK_MANIFEST_LINES = 18

HISTORICAL_PATTERNS = ("audit/v4.1.0/commands_pin_*.yaml",
                        "audit/v4.1.0/runs/*/manifest.yaml")

FROZEN_COMMIT = re.compile(r'^\s*frozen_commit:\s*"?([0-9a-f]{40})"?\s*$', re.M)

SKIPPED = []
NOTES = []


class ScopeError(Exception):
    """A SHA scope invariant does not hold."""


def require(condition, message):
    """Assert-like helper that survives `python -O` (which strips `assert`)."""
    if not condition:
        raise ScopeError(message)


def skip(check, detail):
    """Record a cross-check that could not run (never reported as verified)."""
    SKIPPED.append(f"{check}: {detail}")


def note(message):
    """Record an informational remark, once per distinct message."""
    if message not in NOTES:
        NOTES.append(message)


def git_out(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def git_ok(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    ).returncode == 0


def have_commit(rev):
    return git_ok("cat-file", "-e", f"{rev}^{{commit}}")


def git_blob(rev, rel):
    """Bytes of `rel` at `rev`, or None when the path is not in that commit."""
    proc = subprocess.run(["git", "show", f"{rev}:{rel}"], cwd=ROOT,
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    return proc.stdout if proc.returncode == 0 else None


def dirty_paths():
    """The paths whose working-tree state differs from HEAD."""
    out = subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True)
    return [line for line in out.splitlines() if line.strip()]


def read_json(rel):
    """Parse a JSON file from the working tree (== HEAD: the tree is clean)."""
    path = ROOT / rel
    require(path.is_file(), f"{rel} does not exist")
    try:
        return json.loads(path.read_text())
    except ValueError as exc:
        raise ScopeError(f"{rel} is not valid JSON: {exc}") from exc


def read_jsonl(rel):
    """Parse a JSON-lines file from the working tree (== HEAD: tree is clean)."""
    path = ROOT / rel
    require(path.is_file(), f"{rel} does not exist")
    records = []
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if not line.strip():
            continue
        try:
            records.append(json.loads(line))
        except ValueError as exc:
            raise ScopeError(f"{rel}:{number} is not valid JSON: {exc}") from exc
    return records


def frozen_commit_values(text):
    """The frozen_commit bindings declared in a YAML manifest."""
    return FROZEN_COMMIT.findall(text)


def matches_historical_pattern(rel):
    return any(fnmatch.fnmatch(rel, pattern) for pattern in HISTORICAL_PATTERNS)


def frozen_historical_paths(base):
    """The historical record set as frozen in the baseline commit."""
    listing = git_out("ls-tree", "-r", "--name-only", base, "--", "audit/v4.1.0")
    return sorted(rel for rel in listing.splitlines() if matches_historical_pattern(rel))


def worktree_historical_paths():
    """The historical record set as it exists in the working tree."""
    found = set()
    for path in (ROOT / "audit/v4.1.0").rglob("*.yaml"):
        rel = str(path.relative_to(ROOT))
        if matches_historical_pattern(rel):
            found.add(rel)
    return sorted(found)


def s1(base):
    dag = read_json(DAG_REL)
    tasks = dag.get("tasks")
    require(isinstance(tasks, list), f"{DAG_REL} has no tasks[] list")
    require(len(tasks) == DAG_TASKS,
            f"{DAG_REL} holds {len(tasks)} task(s), expected {DAG_TASKS}")
    wrong = [t.get("task_id") for t in tasks if t.get("base_sha") != base]
    require(not wrong,
            f"{len(wrong)} task(s) do not point at the baseline {base[:10]}: {wrong[:5]}")
    context = dag.get("artifact_context")
    require(isinstance(context, dict), f"{DAG_REL} has no artifact_context object")
    source_base = context.get("source_code_base_sha")
    require(source_base == PRE_FREEZE_BASE_SHA,
            f"{DAG_REL} artifact_context.source_code_base_sha is {source_base!r}, "
            f"expected the pre-freeze base {PRE_FREEZE_BASE_SHA!r}; the provenance "
            f"binding of the test-factory package must not be re-pointed")
    return (f"{len(tasks)} task(s), all base_sha == {base[:10]}; "
            f"artifact_context.source_code_base_sha == {PRE_FREEZE_BASE_SHA[:10]} (unchanged)")


def s2(base):
    matrix = read_json(MATRIX_REL)
    cases = matrix.get("cases")
    require(isinstance(cases, list), f"{MATRIX_REL} has no cases[] list")
    require(len(cases) == MATRIX_CASES,
            f"{MATRIX_REL} holds {len(cases)} case(s), expected {MATRIX_CASES}")
    wrong = [c.get("case_id") for c in cases if c.get("base_sha") != base]
    require(not wrong,
            f"{len(wrong)} case(s) do not point at the baseline {base[:10]}: {wrong[:5]}")
    return f"{len(cases)} case(s), all base_sha == {base[:10]}"


def s3(base):
    records = read_jsonl(TASK_MANIFEST_REL)
    require(len(records) == TASK_MANIFEST_LINES,
            f"{TASK_MANIFEST_REL} holds {len(records)} record(s), expected "
            f"{TASK_MANIFEST_LINES}")
    wrong = [r.get("task_id") for r in records if r.get("base_commit") != base]
    require(not wrong,
            f"{len(wrong)} record(s) do not point at the baseline {base[:10]}: {wrong[:5]}")
    dag = read_json(DAG_REL)
    dag_ids = [t.get("task_id") for t in dag.get("tasks", [])]
    manifest_ids = [r.get("task_id") for r in records]
    require(dag_ids == manifest_ids,
            f"the task_id order of {TASK_MANIFEST_REL} does not match the DAG: "
            f"{manifest_ids[:5]} != {dag_ids[:5]}")
    return (f"{len(records)} record(s), all base_commit == {base[:10]}, "
            f"task_id order identical to the DAG")


def s4(base):
    handoff = read_json(HANDOFF_REL)
    require(handoff.get("base_sha") == base,
            f"{HANDOFF_REL} base_sha is {handoff.get('base_sha')!r}, expected {base!r}")
    adr = handoff.get("adr_base_commit")
    require(adr == MATERIAL_COMMIT,
            f"{HANDOFF_REL} adr_base_commit is {adr!r}, expected the ADR text's own "
            f"commit {MATERIAL_COMMIT!r}")
    return f"base_sha == {base[:10]}, adr_base_commit == {MATERIAL_COMMIT[:10]} (unchanged)"


def s5(base):
    require(have_commit(base),
            f"the frozen baseline {base[:10]} is not a commit of this repository; "
            f"the historical record set cannot be derived")
    expected = frozen_historical_paths(base)
    require(expected, "the frozen baseline holds no historical record")
    current = worktree_historical_paths()
    missing = sorted(set(expected) - set(current))
    extra = sorted(set(current) - set(expected))
    require(not missing,
            f"{len(missing)} historical record(s) are gone from the worktree: {missing[:5]}")
    require(not extra,
            f"{len(extra)} historical record(s) are not part of the frozen historical "
            f"set: {extra[:5]}")

    # The binding layer runs before the byte layer, and it may not pass
    # vacuously: a comparison of two empty binding lists proves nothing.
    bindings_now = {rel: frozen_commit_values((ROOT / rel).read_text()) for rel in expected}
    total_now = sum(len(values) for values in bindings_now.values())
    require(total_now > 0,
            "no historical record declares a frozen_commit binding; the binding "
            "comparison would be vacuous")

    compared = 0
    total_then = 0
    added = []
    if not have_commit(MATERIAL_COMMIT):
        skip("S5/historical-commit", f"{MATERIAL_COMMIT[:10]} is not reachable")
    else:
        for rel in expected:
            original = git_blob(MATERIAL_COMMIT, rel)
            if original is None:
                added.append(rel)
                continue
            compared += 1
            before = frozen_commit_values(original.decode())
            total_then += len(before)
            require(before == bindings_now[rel],
                    f"historical record {rel} changed its frozen_commit binding: "
                    f"{before} -> {bindings_now[rel]}")
        require(compared > 0,
                f"no historical record could be compared against {MATERIAL_COMMIT[:10]}; "
                f"the binding comparison would be vacuous")
        # The same file set on both sides: the records added after
        # MATERIAL_COMMIT have no counterpart there.
        total_now_compared = sum(len(bindings_now[rel]) for rel in expected
                                 if rel not in added)
        require(total_now_compared == total_then,
                f"the compared historical records declare {total_now_compared} "
                f"frozen_commit binding(s) now but {total_then} at "
                f"{MATERIAL_COMMIT[:10]}")
        if added:
            note(f"{len(added)} historical record(s) were added after "
                 f"{MATERIAL_COMMIT[:10]}; their bytes are covered by the frozen-baseline "
                 f"comparison")
    for rel in expected:
        frozen = git_blob(base, rel)
        require(frozen is not None,
                f"historical record {rel} is absent from the frozen baseline {base[:10]}")
        require((ROOT / rel).read_bytes() == frozen,
                f"historical record {rel} drifted away from the frozen baseline "
                f"{base[:10]}; historical bindings must not be re-pointed")
    return (f"{len(expected)} historical record(s) byte-identical to the frozen "
            f"baseline {base[:10]}; {compared} frozen_commit binding(s) compared "
            f"against {MATERIAL_COMMIT[:10]}, all unchanged")


def s6():
    tracked = ROOT / PILOT_EVIDENCE_REL
    require(tracked.is_file(),
            f"{PILOT_EVIDENCE_REL} does not exist; the tracked pilot rerun summary "
            f"is the only copy every checkout has")
    try:
        summary = json.loads(tracked.read_text())
    except ValueError as exc:
        raise ScopeError(f"{PILOT_EVIDENCE_REL} is not valid JSON: {exc}") from exc
    pilot_base = summary.get("base_sha")
    require(pilot_base == PRE_FREEZE_BASE_SHA,
            f"{PILOT_EVIDENCE_REL} points at {pilot_base!r}, expected the historical "
            f"{PRE_FREEZE_BASE_SHA!r}")
    detail = f"the tracked {PILOT_EVIDENCE_REL} still points at {PRE_FREEZE_BASE_SHA[:10]}"
    if PILOT_OUT_OF_REPO.is_file():
        require(PILOT_OUT_OF_REPO.read_bytes() == tracked.read_bytes(),
                f"the pilot's out-of-repo copy {PILOT_OUT_OF_REPO} differs from the "
                f"tracked {PILOT_EVIDENCE_REL}")
        detail += "; the out-of-repo pilot copy is byte-identical"
    elif PILOT_OUT_OF_REPO_ROOT.is_dir():
        raise ScopeError(
            f"the pilot checkout {PILOT_OUT_OF_REPO_ROOT} is present but "
            f"{PILOT_OUT_OF_REPO} is missing; the pilot evidence cannot have been deleted")
    else:
        skip("S6/pilot-out-of-repo",
             f"there is no pilot checkout at {PILOT_OUT_OF_REPO_ROOT}; only the tracked "
             f"copy was compared")
    return detail


def main():
    parser = argparse.ArgumentParser(
        description="Verify the forward/historical SHA scope of the v4.1.0 re-point (S1-S6).")
    parser.add_argument("--strict", action="store_true",
                        help="fail when a cross-check could not be run")
    args = parser.parse_args()

    if sys.flags.optimize:
        print("refusing to run under python -O: assert statements are stripped, "
              "this gate would be a no-op")
        return 2
    if not FREEZE.is_file():
        print(f"FAIL  {FREEZE_REL} does not exist")
        print("0/6 PASS")
        return 1
    try:
        base = json.loads(FREEZE.read_text())["base_sha"]
    except (ValueError, KeyError, TypeError) as exc:
        print(f"FAIL  {FREEZE_REL} does not carry a base_sha: {exc}")
        print("0/6 PASS")
        return 1
    if not re.fullmatch(r"[0-9a-f]{40}", str(base)):
        print(f"FAIL  {FREEZE_REL} base_sha is not a full 40-char sha: {base!r}")
        print("0/6 PASS")
        return 1

    # The verdict must be about a commit, not about uncommitted bytes.
    dirty = dirty_paths()
    if dirty:
        print("FAIL  the working tree is not clean; the verdict must be bound to a commit")
        for line in dirty[:20]:
            print(f"      {line}")
        if len(dirty) > 20:
            print(f"      ... and {len(dirty) - 20} more")
        print("0/6 PASS")
        return 1

    checks = (("S1", lambda: s1(base)), ("S2", lambda: s2(base)), ("S3", lambda: s3(base)),
              ("S4", lambda: s4(base)), ("S5", lambda: s5(base)), ("S6", s6))
    passed = 0
    for name, check in checks:
        try:
            detail = check()
        except ScopeError as exc:
            print(f"[{name}] FAIL  {exc}")
        except Exception as exc:  # a broken record must fail closed, not crash
            print(f"[{name}] FAIL  {type(exc).__name__}: {exc}")
        else:
            passed += 1
            print(f"[{name}] PASS  {detail}")
    for message in NOTES:
        print(f"note: {message}")
    for message in SKIPPED:
        print(f"skipped: {message}")
    verdict = f"{passed}/{len(checks)} PASS"
    if SKIPPED:
        verdict += f", {len(SKIPPED)} cross-check(s) skipped"
        if args.strict:
            verdict += " -- STRICT FAIL"
            print(verdict)
            return 1
    print(verdict)
    return 0 if passed == len(checks) else 1


if __name__ == "__main__":
    sys.exit(main())
