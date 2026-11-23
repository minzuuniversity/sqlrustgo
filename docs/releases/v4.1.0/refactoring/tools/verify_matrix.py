#!/usr/bin/env python3
"""Coverage-matrix validator for the v4.1.0 75-scenario matrix (Task 5).

The matrix is the contract between "a scenario exists" and "a scenario is a
defined test".  Before this gate the 75 cases were a list of positions: every
one of them carried an empty `invariant_ids`, an `UNREVIEWED` mapping status, a
`NOT_RUN` execution status and no evidence, and two `case_id`s were used twice.
A list of positions cannot be executed, cannot be reviewed and cannot block a
release, so the matrix needs the same kind of gate the rest of the package
already has.

Six assertions:

  M1  every case carries at least one invariant id
  M2  no case is still `UNREVIEWED`
  M3  `case_id` is unique across the 75 cases
  M4  every referenced invariant id is declared in
      `TRANSACTION_INVARIANTS.md` or `STORAGE_CAPABILITY_CONTRACT.md`
  M5  `execution_status` uses the seven-word contract vocabulary (the pilot's
      own vocabulary is tolerated before the mapping is applied), and the
      top-level `status_vocabulary_mapping` is a well-formed map from the
      pilot vocabulary onto the contract vocabulary
  M6  every case that is KNOWN_FAIL carries the complete five-element RED
      identity (`test_fn`, `package_or_target`, `source_sha256`, `rc`,
      `attribution`)

Two properties are load-bearing and are asserted explicitly:

  * a deliberate RED is not a failure.  `FAILED_RED` / `COVERED_RED` map onto
    `KNOWN_FAIL`, never onto `PASS` and never onto a bare `FAIL`: collapsing a
    known RED into `FAIL` destroys the identity of the defect it stands for.

  * no assertion may pass vacuously.  M4 fails when no case references an
    invariant id at all (an empty reference list would compare against nothing
    and report success), and M6 fails when no case is KNOWN_FAIL (there would
    be no RED identity to check).

The invariants are raised explicitly instead of being written as `assert`
statements, because `python -O` strips `assert` and would turn this gate into
a no-op that prints "6/6 PASS" for anything.  The tool refuses to run under
`python -O` at all.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[5]

MATRIX_REL = "docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json"
TX_REL = "docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md"
STORAGE_REL = "docs/releases/v4.1.0/refactoring/STORAGE_CAPABILITY_CONTRACT.md"

EXPECTED_CASES = 75

# The seven-word vocabulary of 05_TEST_FACTORY_CONTRACT.json.
CONTRACT_VOCABULARY = ("PASS", "FAIL", "KNOWN_FAIL", "UNSUPPORTED",
                       "NOT_RUN", "DIALECT_DIFFERENCE", "INFRA_ERROR")
# The pilot's own vocabulary (evidence/case-mapping-20261120.json).
PILOT_VOCABULARY = ("PASS", "FAILED_RED", "COVERED_PASS", "COVERED_RED")
# The words that stand for a deliberate RED.
RED_VOCABULARY = ("FAILED_RED", "COVERED_RED")

RED_IDENTITY_FIELDS = ("test_fn", "package_or_target", "source_sha256",
                       "rc", "attribution")

SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
# An id-shaped token: two to four capitals, a dash, an optional capital and one
# to three digits (INV-01, TF-I09, SC-12).
ID_TOKEN_RE = re.compile(r"\b[A-Z]{2,4}-[A-Z]?\d{1,3}\b")

SKIPPED: list[str] = []
NOTES: list[str] = []


class MatrixError(Exception):
    """A matrix invariant does not hold."""


def require(condition, message):
    """Assert-like helper that survives `python -O` (which strips `assert`)."""
    if not condition:
        raise MatrixError(message)


def skip(check, detail):
    """Record a cross-check that could not run (never reported as verified)."""
    SKIPPED.append(f"{check}: {detail}")


def note(message):
    """Record an informational remark, once per distinct message."""
    if message not in NOTES:
        NOTES.append(message)


def read_json(rel):
    path = ROOT / rel
    require(path.is_file(), f"{rel} does not exist")
    try:
        return json.loads(path.read_text())
    except ValueError as exc:
        raise MatrixError(f"{rel} is not valid JSON: {exc}") from exc


def cases_of(matrix):
    cases = matrix.get("cases")
    require(isinstance(cases, list), f"{MATRIX_REL} has no cases[] list")
    require(len(cases) == EXPECTED_CASES,
            f"{MATRIX_REL} holds {len(cases)} case(s), expected {EXPECTED_CASES}")
    return cases


def declared_ids(path):
    """The invariant ids a document declares.

    Only the body of the document is read: the blockquoted header carries the
    provenance of the text (author, model, evidence hash), not the registry,
    and scanning it would pick up model names as if they were invariant ids.
    """
    ids = set()
    for line in path.read_text().splitlines():
        if line.lstrip().startswith(">"):
            continue
        ids.update(ID_TOKEN_RE.findall(line))
    return ids


def corpus():
    """{invariant id: document that declares it} over the two registries."""
    found = {}
    for rel in (TX_REL, STORAGE_REL):
        path = ROOT / rel
        if not path.is_file():
            skip("M4/registry", f"{rel} does not exist")
            continue
        for iid in declared_ids(path):
            found.setdefault(iid, rel)
    return found


def m1(matrix):
    cases = cases_of(matrix)
    empty = [c.get("case_id") for c in cases if not c.get("invariant_ids")]
    require(not empty,
            f"{len(empty)} case(s) carry no invariant id: {empty[:5]}")
    blank = [c.get("case_id") for c in cases
             for iid in c["invariant_ids"]
             if not isinstance(iid, str) or not iid.strip()]
    require(not blank,
            f"{len(blank)} invariant id(s) are blank or not a string: {blank[:5]}")
    return f"{len(cases)} case(s), every one carries at least one invariant id"


def m2(matrix):
    cases = cases_of(matrix)
    unreviewed = [c.get("case_id") for c in cases
                  if not isinstance(c.get("invariant_mapping_status"), str)
                  or not c["invariant_mapping_status"].strip()
                  or c["invariant_mapping_status"] == "UNREVIEWED"]
    require(not unreviewed,
            f"{len(unreviewed)} case(s) are still UNREVIEWED: {unreviewed[:5]}")
    states = sorted({c["invariant_mapping_status"] for c in cases})
    return f"{len(cases)} case(s), mapping status {'/'.join(states)}"


def m3(matrix):
    cases = cases_of(matrix)
    seen: dict[str, int] = {}
    for case in cases:
        cid = case.get("case_id")
        seen[cid] = seen.get(cid, 0) + 1
    duplicates = sorted(cid for cid, count in seen.items() if count > 1)
    require(not duplicates,
            f"{len(duplicates)} case_id(s) are used more than once: {duplicates[:5]}")
    return f"{len(cases)} case(s), {len(seen)} distinct case_id(s)"


def m4(matrix):
    cases = cases_of(matrix)
    ids = corpus()
    require(ids,
            f"neither {TX_REL} nor {STORAGE_REL} declares an invariant id; "
            f"there is no corpus to check the mapping against")
    referenced = set()
    unknown = []
    for case in cases:
        for iid in case.get("invariant_ids") or []:
            referenced.add(iid)
            if iid not in ids:
                unknown.append((case.get("case_id"), iid))
    require(referenced,
            "no case references an invariant id; the comparison against the "
            "registry would be vacuous")
    require(not unknown,
            f"{len(unknown)} invariant id(s) are declared in neither registry: "
            f"{unknown[:5]}")
    sources = sorted({ids[iid] for iid in referenced})
    return (f"{len(referenced)} distinct id(s) referenced, all declared in "
            f"{', '.join(sources)}")


def m5(matrix):
    cases = cases_of(matrix)
    allowed = CONTRACT_VOCABULARY + PILOT_VOCABULARY
    wrong = [(c.get("case_id"), c.get("execution_status")) for c in cases
             if c.get("execution_status") not in allowed]
    require(not wrong,
            f"{len(wrong)} case(s) carry a status outside the contract and "
            f"pilot vocabularies: {wrong[:5]}")
    mapping = matrix.get("status_vocabulary_mapping")
    require(isinstance(mapping, dict) and mapping,
            f"{MATRIX_REL} carries no status_vocabulary_mapping block; the "
            f"pilot vocabulary cannot be mapped onto the contract vocabulary")
    uncovered = [word for word in PILOT_VOCABULARY if word not in mapping]
    require(not uncovered,
            f"the vocabulary mapping does not cover the pilot word(s): "
            f"{uncovered}")
    off_contract = sorted({target for target in mapping.values()
                            if target not in CONTRACT_VOCABULARY})
    require(not off_contract,
            f"the vocabulary mapping targets {off_contract}, which are not "
            f"contract vocabulary")
    collapsed = [word for word in RED_VOCABULARY
                 if mapping.get(word) != "KNOWN_FAIL"]
    require(not collapsed,
            f"the deliberate RED word(s) {collapsed} must map onto KNOWN_FAIL; "
            f"a known RED is not a failure and must not be collapsed into one")
    not_identity = sorted(word for word, target in mapping.items()
                           if word in CONTRACT_VOCABULARY and target != word)
    require(not not_identity,
            f"the contract word(s) {not_identity} must map onto themselves")
    return (f"{len(cases)} case(s) use the contract/pilot vocabulary; the "
            f"mapping covers {len(mapping)} word(s) and keeps the RED words "
            f"on KNOWN_FAIL")


def m6(matrix):
    cases = cases_of(matrix)
    reds = [c for c in cases
            if c.get("execution_status") == "KNOWN_FAIL"
            or (c.get("source") or {}).get("claimed_status") == "KNOWN_FAIL"]
    require(reds,
            "no case is KNOWN_FAIL; the RED identity check would be vacuous")
    broken = []
    for case in reds:
        identity = case.get("red_identity")
        problems = []
        if not isinstance(identity, dict):
            problems.append("no red_identity object")
        else:
            for field in RED_IDENTITY_FIELDS:
                value = identity.get(field)
                if value is None:
                    problems.append(f"{field} is missing")
                elif isinstance(value, str) and not value.strip():
                    problems.append(f"{field} is empty")
            digest = identity.get("source_sha256")
            if isinstance(digest, str) and digest.strip() \
                    and not SHA256_RE.match(digest):
                problems.append("source_sha256 is not a 64-character sha256")
            if not isinstance(identity.get("rc"), int):
                problems.append("rc is not an integer exit code")
        if problems:
            broken.append((case.get("case_id"), problems))
    require(not broken,
            f"{len(broken)} KNOWN_FAIL case(s) carry no complete RED identity: "
            f"{broken[:3]}")
    return (f"{len(reds)} KNOWN_FAIL case(s), every one carries the "
            f"five-element RED identity")


def main():
    parser = argparse.ArgumentParser(
        description="Verify the v4.1.0 75-scenario coverage matrix (M1-M6).")
    parser.add_argument("--strict", action="store_true",
                        help="fail when a cross-check could not be run")
    args = parser.parse_args()

    if sys.flags.optimize:
        print("refusing to run under python -O: assert statements are stripped, "
              "this gate would be a no-op")
        return 2
    if not (ROOT / MATRIX_REL).is_file():
        print(f"FAIL  {MATRIX_REL} does not exist")
        print("0/6 PASS")
        return 1
    try:
        matrix = read_json(MATRIX_REL)
    except MatrixError as exc:
        print(f"FAIL  {exc}")
        print("0/6 PASS")
        return 1

    checks = (("M1", m1), ("M2", m2), ("M3", m3),
              ("M4", m4), ("M5", m5), ("M6", m6))
    passed = 0
    for name, check in checks:
        try:
            detail = check(matrix)
        except MatrixError as exc:
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
