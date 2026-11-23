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

  S1  REFACTOR_TASK_DAG.json      18 x tasks[].base_sha  == BASE
  S2  TEST_COVERAGE_MATRIX.json   75 x cases[].base_sha  == BASE
  S3  03_TASK_MANIFEST.jsonl      18 x base_commit       == BASE, and the
      task_id order matches the DAG's task_id order one-to-one
  S4  14_HANDOFF_MANIFEST.json    base_sha              == BASE, while
      adr_base_commit keeps the ADR text's own commit identity
  S5  audit/v4.1.0/commands_pin_*.yaml and audit/v4.1.0/runs/*/manifest.yaml
      keep their original bytes and their original frozen_commit bindings
  S6  the pilot rerun summary still points at the pre-refactor base_sha

`BASE` is read from BASELINE_FREEZE.json rather than hardcoded here, so this
gate follows the freeze record instead of keeping a second copy of it.

S5 compares the historical records against the frozen baseline (which is an
ancestor of HEAD and therefore holds the bytes as they stood before this
re-point), and additionally compares every `frozen_commit` binding against the
historical commit 45da6968ae.  The two references differ on purpose: a few run
manifests were corrected *after* 45da6968ae without touching their
`frozen_commit`, and six of them were added after it, so a byte comparison
against 45da6968ae alone would report a false drift.

The invariants are raised explicitly instead of being written as `assert`
statements, because `python -O` strips `assert` and would turn this gate into
a no-op that prints "6/6 PASS" for anything.
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

DAG_REL = "docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json"
MATRIX_REL = "docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json"
TASK_MANIFEST_REL = "docs/refactor/v4.1.0-c/handoff/03_TASK_MANIFEST.jsonl"
HANDOFF_REL = "docs/releases/v4.1.0/refactoring/test-factory-design/14_HANDOFF_MANIFEST.json"

HISTORICAL_COMMIT = "45da6968ae2baaade6a31206eeee5854a8514ee2"
ADR_BASE_COMMIT = "45da6968ae2baaade6a31206eeee5854a8514ee2"
PILOT_EVIDENCE = pathlib.Path(
    "/Volumes/workspace/dev/srg-test-factory-pilot/evidence/rerun-summary-20261120.json")
PILOT_BASE_SHA = "e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0"

DAG_TASKS = 18
MATRIX_CASES = 75
TASK_MANIFEST_LINES = 18

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


def read_json(rel):
    path = ROOT / rel
    require(path.is_file(), f"{rel} does not exist")
    try:
        return json.loads(path.read_text())
    except ValueError as exc:
        raise ScopeError(f"{rel} is not valid JSON: {exc}") from exc


def read_jsonl(rel):
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


def historical_files():
    """The audit records that carry historical bindings, as repo-relative paths."""
    audit = ROOT / "audit/v4.1.0"
    files = sorted(str(p.relative_to(ROOT)) for p in audit.glob("commands_pin_*.yaml"))
    files += sorted(str(p.relative_to(ROOT)) for p in (audit / "runs").glob("*/manifest.yaml"))
    return files


def s1(base):
    dag = read_json(DAG_REL)
    tasks = dag.get("tasks")
    require(isinstance(tasks, list), f"{DAG_REL} has no tasks[] list")
    require(len(tasks) == DAG_TASKS,
            f"{DAG_REL} holds {len(tasks)} task(s), expected {DAG_TASKS}")
    wrong = [t.get("task_id") for t in tasks if t.get("base_sha") != base]
    require(not wrong,
            f"{len(wrong)} task(s) do not point at the baseline {base[:10]}: {wrong[:5]}")
    return f"{len(tasks)} task(s), all base_sha == {base[:10]}"


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
    require(adr == ADR_BASE_COMMIT,
            f"{HANDOFF_REL} adr_base_commit is {adr!r}, expected the ADR text's own "
            f"commit {ADR_BASE_COMMIT!r}")
    return f"base_sha == {base[:10]}, adr_base_commit == {ADR_BASE_COMMIT[:10]} (unchanged)"


def s5(base):
    files = historical_files()
    require(files, "no historical binding file was found under audit/v4.1.0")
    if not have_commit(base):
        skip("S5/baseline-bytes", f"base_sha {base[:10]} is not a commit of this repository")
    else:
        for rel in files:
            frozen = git_blob(base, rel)
            require(frozen is not None,
                    f"historical record {rel} is absent from the frozen baseline {base[:10]}")
            current = (ROOT / rel).read_bytes()
            require(current == frozen,
                    f"historical record {rel} drifted away from the frozen baseline "
                    f"{base[:10]}; historical bindings must not be re-pointed")
    compared = 0
    if not have_commit(HISTORICAL_COMMIT):
        skip("S5/historical-commit", f"{HISTORICAL_COMMIT[:10]} is not reachable")
    else:
        added = []
        for rel in files:
            original = git_blob(HISTORICAL_COMMIT, rel)
            if original is None:
                added.append(rel)
                continue
            compared += 1
            before = frozen_commit_values(original.decode())
            after = frozen_commit_values((ROOT / rel).read_text())
            require(before == after,
                    f"historical record {rel} changed its frozen_commit binding: "
                    f"{before} -> {after}")
        if added:
            note(f"{len(added)} historical record(s) were added after "
                 f"{HISTORICAL_COMMIT[:10]}; their bytes are covered by the frozen-baseline "
                 f"comparison")
    return (f"{len(files)} historical record(s) unchanged; {compared} frozen_commit "
            f"binding(s) still equal to {HISTORICAL_COMMIT[:10]}")


def s6():
    if not PILOT_EVIDENCE.is_file():
        skip("S6/pilot-evidence", f"{PILOT_EVIDENCE} does not exist")
        return f"pilot rerun summary not present, the binding was not checked"
    try:
        summary = json.loads(PILOT_EVIDENCE.read_text())
    except ValueError as exc:
        raise ScopeError(f"{PILOT_EVIDENCE} is not valid JSON: {exc}") from exc
    pilot_base = summary.get("base_sha")
    require(pilot_base == PILOT_BASE_SHA,
            f"the pilot rerun summary points at {pilot_base!r}, expected the historical "
            f"{PILOT_BASE_SHA!r}")
    return f"pilot rerun summary still points at {PILOT_BASE_SHA[:10]}"


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
    except (ValueError, KeyError) as exc:
        print(f"FAIL  {FREEZE_REL} does not carry a base_sha: {exc}")
        print("0/6 PASS")
        return 1
    if not re.fullmatch(r"[0-9a-f]{40}", str(base)):
        print(f"FAIL  {FREEZE_REL} base_sha is not a full 40-char sha: {base!r}")
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
