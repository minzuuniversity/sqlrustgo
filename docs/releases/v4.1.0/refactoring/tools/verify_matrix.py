#!/usr/bin/env python3
"""Coverage-matrix validator for the v4.1.0 75-scenario matrix (Task 5).

The matrix is the contract between "a scenario exists" and "a scenario is a
defined test".  Before this gate the 75 cases were a list of positions: every
one of them carried an empty `invariant_ids`, an `UNREVIEWED` mapping status, a
`NOT_RUN` execution status and no evidence, and two `case_id`s were used twice.
A list of positions cannot be executed, cannot be reviewed and cannot block a
release, so the matrix needs the same kind of gate the rest of the package
already has.

Seven assertions:

  M1  every case carries at least one invariant id, or declares -- in words --
      that it matches none, with a per-case basis for that declaration
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
  M7  the mapping is per case, not per bucket: every case carries an
      `invariant_mapping_basis`, no two cases carry a near-identical basis,
      every case that claims an invariant id names that id in its own basis
      and no single id is claimed by more than a third of the matrix, the
      `invariant_mapping_status` vocabulary is the one registered in
      `TRANSACTION_INVARIANTS.md`, and every case with an empty mapping is
      listed in the gap-proposal record's index table -- in both directions

WHAT "7/7 PASS" MEANS
----------------------
M7 checks the **form** of the mapping, not its substance.  A green M7 says:
every case carries its own sentence, no sentence is a copy of another, no one
invariant id is spread over the matrix, the status vocabulary is the
registered one, and the empty mappings are registered.  It does **not** say
that a mapping is *correct* -- that the sentence really describes what the
test asserts, and that the invariant id really corresponds to that assertion.
That is a human (Oracle / countersigner) review item, and it is what the
matrix's `assertion_review_verdict` fields exist for.  Treat a green M7 as
"the mapping is well-formed and per case", never as "the mapping is
verified".

The corpus of M4 is the *id column* of the two registry documents: an
invariant id is the first cell of a table row.  A token that only occurs in
prose is not an id -- `ADR-010`, which the storage capability contract
mentions in a sentence, must not pass for an invariant, and neither must a
model name in a provenance header.

An identity that cannot be completed is not silently dropped: it carries an
explicit `identity_gap` next to the element it cannot supply, so the hole is
visible in the matrix and auditable, rather than invisible behind a missing
key.  M6 accepts a declared gap only for a case that is not itself KNOWN_FAIL
(a KNOWN_FAIL must be able to be judged, so it must be complete).

Two properties are load-bearing and are asserted explicitly:

  * a deliberate RED is not a failure.  `FAILED_RED` / `COVERED_RED` map onto
    `KNOWN_FAIL`, never onto `PASS` and never onto a bare `FAIL`: collapsing a
    known RED into `FAIL` destroys the identity of the defect it stands for.

  * no assertion may pass vacuously.  M4 fails when no case references an
    invariant id at all (an empty reference list would compare against nothing
    and report success), and M6 fails when no case is KNOWN_FAIL (there would
    be no RED identity to check).

M1 and M7 are the pair that the Task 10 rework introduced.  The mapping used
to be attached per *feature*: every case that shared a `declared_invariant`
label was given the same id set, so 75 cases ended up with 13 labels and a
group of 28 unrelated tests carried one sentence about undo-log semantics.
Task 10 re-derived all 75 mappings from the test source, one case at a time,
and the honest result is that most of the pilot cases assert a property the
18 registered invariants do not talk about.  Those cases now carry an empty
id list -- which M1 used to forbid outright -- so M1's *purpose* (no case may
carry no invariant without saying so) is kept while its *form* now accepts a
declared empty mapping, and M7 makes the declaration auditable: a basis per
case, no copied basis, a shared id justified case by case, the status
vocabulary read back from the registry, and every empty mapping listed in the
index table of `INVARIANT-GAP-PROPOSALS.md` -- in both directions.

M7 verifies the *form* of that mapping.  Whether a mapping is substantively
right -- whether the sentence describes what the test really asserts and
whether the id really corresponds to that assertion -- is a human (Oracle /
countersigner) review item, and a green M7 is not a statement about it.

The invariants are raised explicitly instead of being written as `assert`
statements, because `python -O` strips `assert` and would turn this gate into
a no-op that prints "7/7 PASS" for anything.  The tool refuses to run under
`python -O` at all.
"""
from __future__ import annotations

import argparse
import difflib
import itertools
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[5]

MATRIX_REL = "docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json"
TX_REL = "docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md"
STORAGE_REL = "docs/releases/v4.1.0/refactoring/STORAGE_CAPABILITY_CONTRACT.md"
# The record that holds the cases whose assertions match no registered
# invariant.  M7 checks that every declared-empty case is listed there, so the
# hole is visible in a document a reviewer can read instead of only in the
# matrix.
GAP_REL = "docs/refactor/v4.1.0-c/takeover/INVARIANT-GAP-PROPOSALS.md"

# The mapping status a case must carry when it declares that its test asserts
# something no registered invariant talks about.
NO_MATCH_STATUS = "NO_INVARIANT_MATCH"
# The per-case field that says *why* a case carries the ids it carries.
BASIS_FIELD = "invariant_mapping_basis"
# The closed vocabulary of `invariant_mapping_status`.  It is *registered* in
# TRANSACTION_INVARIANTS.md (under the heading below) so that a consumer of the
# matrix can parse the field without reading this tool; M7 checks that the
# registry and this copy still agree.
MAPPING_STATUSES = ("MAPPED", "NO_INVARIANT_MATCH")
MAPPING_VOCAB_HEADING = "## 矩阵映射状态词表"

# Two per-case bases that are this similar (after normalisation) are the same
# sentence with cosmetic differences -- that is the copy-paste signature of a
# bucket mapping.  The comparison is on normalised text so that punctuation and
# whitespace cannot hide a copy.
BASIS_SIMILARITY_LIMIT = 0.90
# One invariant id may not be claimed by more than this share of the matrix.
# Past that point the mapping is a bucket, not a mapping: the original defect
# was one label covering 28 of 30 cases in one feature.
MAX_ID_CLAIM_SHARE = 1 / 3

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
# The key a RED identity uses to declare that one of its elements could not be
# recovered.  A declared gap is auditable; a missing element is not.
GAP_FIELD = "identity_gap"

SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
# An invariant id: the id cell of a registry table -- two to four capitals, a
# dash, an optional capital and one to three digits (INV-01, TF-I09, SC-12).
# A token that only occurs in prose is not an id: `ADR-010` in the storage
# contract's prose, or a model name in a provenance header, must not pass for
# an invariant.
ID_CELL_RE = re.compile(r"^[A-Z]{2,4}-[A-Z]?\d{1,3}$")

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

    A registry declares its ids in the id column of a table, so the ids are
    the first cells of the table rows.  Everything else -- prose mentions
    (`ADR-010` in the storage contract's prose), the blockquoted provenance
    header (author, model, evidence hash) -- is not part of the registry and
    must not be read as an invariant.
    """
    ids = set()
    for line in path.read_text().splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if not cells:
            continue
        if ID_CELL_RE.match(cells[0]):
            ids.add(cells[0])
    return ids


def corpus():
    """{invariant id: document that declares it} over the two registries."""
    found = {}
    for rel in (TX_REL, STORAGE_REL):
        path = ROOT / rel
        if not path.is_file():
            skip("M4/registry", f"{rel} does not exist")
            continue
        ids = declared_ids(path)
        if not ids:
            note(f"{rel} declares no invariant id -- the corpus is the id "
                 f"column of the other registry")
        for iid in ids:
            found.setdefault(iid, rel)
    return found


def basis_of(case):
    """The per-case mapping basis, or "" when it is missing or not a string."""
    value = case.get(BASIS_FIELD)
    if isinstance(value, str) and value.strip():
        return value.strip()
    return ""


def m1(matrix):
    cases = cases_of(matrix)
    empty = [c for c in cases if not c.get("invariant_ids")]
    # An empty id list is only allowed when the case says so: the mapping
    # status declares the empty mapping and the basis says why.  A case that
    # silently carries no invariant is the failure M1 exists to catch.
    undeclared = [c.get("case_id") for c in empty
                  if c.get("invariant_mapping_status") != NO_MATCH_STATUS]
    require(not undeclared,
            f"{len(undeclared)} case(s) carry no invariant id without declaring "
            f"the empty mapping ({NO_MATCH_STATUS}): {undeclared[:5]}")
    unexplained = [c.get("case_id") for c in empty if not basis_of(c)]
    require(not unexplained,
            f"{len(unexplained)} case(s) declare an empty mapping without a "
            f"{BASIS_FIELD}: {unexplained[:5]}")
    blank = [c.get("case_id") for c in cases
             for iid in c["invariant_ids"]
             if not isinstance(iid, str) or not iid.strip()]
    require(not blank,
            f"{len(blank)} invariant id(s) are blank or not a string: {blank[:5]}")
    return (f"{len(cases) - len(empty)} case(s) carry at least one invariant id, "
            f"{len(empty)} declare an empty mapping ({NO_MATCH_STATUS}) with a "
            f"per-case basis")


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


def identity_state(identity):
    """(state, problems) for a RED identity.

    `complete` -- all five elements are present and non-empty, the digest is a
    sha256, the exit code is an integer, and nothing is declared missing.

    `gap` -- all five elements are present, but `source_sha256` is null and an
    `identity_gap` says which element is missing and why.  A declared gap is
    auditable: the reader sees the hole instead of inferring it.

    `broken` -- anything else: a missing element, a malformed digest, an
    undeclared null.
    """
    if not isinstance(identity, dict):
        return "broken", ["no red_identity object"]
    problems = []
    gap = identity.get(GAP_FIELD)
    gap_ok = isinstance(gap, str) and bool(gap.strip())
    if gap is not None and not gap_ok:
        problems.append("identity_gap is present but empty")
    for field in RED_IDENTITY_FIELDS:
        value = identity.get(field)
        if value is None:
            if field == "source_sha256" and gap_ok:
                # The one element a declared gap may leave null: the hole is
                # declared, so it is visible instead of silently missing.
                continue
            problems.append(f"{field} is missing")
        elif isinstance(value, str) and not value.strip():
            problems.append(f"{field} is empty")
    digest = identity.get("source_sha256")
    if isinstance(digest, str) and digest.strip() and not SHA256_RE.match(digest):
        problems.append("source_sha256 is not a 64-character sha256")
    rc = identity.get("rc")
    if rc is not None and not isinstance(rc, int):
        problems.append("rc is not an integer exit code")
    if problems:
        return "broken", problems
    if digest is None:
        if not gap_ok:
            return "broken", ["source_sha256 is null and no identity_gap declares the gap"]
        return "gap", []
    if gap is not None:
        return "broken", ["identity_gap is declared but source_sha256 is present"]
    return "complete", []


def m6(matrix):
    cases = cases_of(matrix)
    reds = [c for c in cases
            if c.get("execution_status") == "KNOWN_FAIL"
            or (c.get("source") or {}).get("claimed_status") == "KNOWN_FAIL"]
    require(reds,
            "no case is KNOWN_FAIL; the RED identity check would be vacuous")
    broken = []
    for case in reds:
        state, problems = identity_state(case.get("red_identity"))
        if state != "complete":
            broken.append((case.get("case_id"), state, problems))
    require(not broken,
            f"{len(broken)} KNOWN_FAIL case(s) carry no complete RED identity: "
            f"{broken[:3]}")
    # A case that carries an identity but is not a KNOWN_FAIL may carry a
    # declared gap -- but the gap must be visible.  A half-written identity
    # that silently omits an element is not acceptable: it reads like a
    # complete one.
    red_ids = {c.get("case_id") for c in reds}
    malformed = []
    for case in cases:
        if case.get("case_id") in red_ids:
            continue
        identity = case.get("red_identity")
        if identity is None:
            continue
        state, problems = identity_state(identity)
        if state == "broken":
            malformed.append((case.get("case_id"), problems))
    require(not malformed,
            f"{len(malformed)} case(s) carry a malformed RED identity: "
            f"{malformed[:3]}")
    return (f"{len(reds)} KNOWN_FAIL case(s), every one carries the "
            f"five-element RED identity")


def normalise_basis(text):
    """Strip whitespace and punctuation so cosmetic edits cannot hide a copy."""
    return re.sub(r"[\s，。；：、（）()「」\"'`·]+", "", text)


def basis_similarity(a, b):
    """How similar two per-case bases are, on normalised text."""
    return difflib.SequenceMatcher(None, normalise_basis(a),
                                    normalise_basis(b)).ratio()


def mapping_status_vocabulary(path):
    """The `invariant_mapping_status` vocabulary, read from the registry.

    The vocabulary is registered in TRANSACTION_INVARIANTS.md so a consumer of
    the matrix can parse the field without reading this tool.  This function
    reads it back, so the registry and the tool cannot drift apart silently.
    """
    statuses = set()
    in_section = False
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("#"):
            in_section = stripped.startswith(MAPPING_VOCAB_HEADING)
            continue
        if not in_section or not stripped.startswith("|"):
            continue
        cells = [cell.strip().strip("`") for cell in stripped.strip("|").split("|")]
        token = cells[0] if cells else ""
        if token and re.fullmatch(r"[A-Z][A-Z_]*", token):
            statuses.add(token)
    return statuses


def gap_index_cases(path):
    """The case ids the gap record's index table lists.

    Only table rows count: a case id mentioned in prose is not a registration.
    """
    ids = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if not cells or not re.fullmatch(r"GAP-\d+", cells[0]):
            continue
        ids.update(re.findall(r"PILOT-[A-Za-z0-9\-]+", " ".join(cells[1:])))
    return ids


def m7(matrix):
    """The mapping must be per case, not per bucket.

    WHAT THIS CHECK IS, AND WHAT IT IS NOT
    ---------------------------------------
    M7 verifies the **form** of the mapping.  It can tell that every case
    carries its own sentence, that no sentence is a copy of another, that no
    single invariant id is spread over the whole matrix, and that every empty
    mapping is registered.  It cannot tell whether a mapping is *substantively*
    right -- whether the sentence really describes what the test asserts, and
    whether the invariant id really corresponds to that assertion.  That is a
    human (Oracle / countersigner) review item, and it is exactly what the
    `assertion_review_verdict` fields in the matrix are for.  A green M7 is
    therefore **not** a statement that the mapping is verified.

    Five properties:

      (a) every case carries a basis -- one sentence that says which part of
          which invariant its test actually asserts.  A bucket mapping has no
          per-case sentence, so this is the check that catches it.

      (b) no two cases carry near-identical bases.  The comparison is on
          normalised text (no punctuation, no whitespace) with a similarity
          threshold, so a copy cannot be hidden behind cosmetic rewording.

      (c) a shared invariant id must be justified per case: every case that
          claims an id has to name that id in its own basis, and no single id
          may be claimed by more than a third of the matrix -- past that point
          the mapping is a bucket, not a mapping.

      (d) the `invariant_mapping_status` vocabulary is the one registered in
          TRANSACTION_INVARIANTS.md, and the registry and this tool agree.

      (e) every case with an empty mapping is listed in the gap record's index
          table, and every case in that table really has an empty mapping --
          both directions, so the file cannot drift out of sync.
    """
    cases = cases_of(matrix)

    # (a) one basis per case
    missing = [c.get("case_id") for c in cases if not basis_of(c)]
    require(not missing,
            f"{len(missing)} case(s) carry no {BASIS_FIELD}: the mapping must "
            f"say per case why it holds the ids it holds: {missing[:5]}")

    # (b) no copy-pasted basis, cosmetic rewording included
    copies = []
    for left, right in itertools.combinations(cases, 2):
        ratio = basis_similarity(basis_of(left), basis_of(right))
        if ratio >= BASIS_SIMILARITY_LIMIT:
            copies.append((left.get("case_id"), right.get("case_id"),
                           round(ratio, 4)))
    require(not copies,
            f"{len(copies)} pair(s) of cases carry a near-identical basis "
            f"(similarity >= {BASIS_SIMILARITY_LIMIT}); a per-case basis that "
            f"is the same sentence twice is a bucket mapping: {copies[:3]}")

    # (c) a shared id must be justified per case
    by_id: dict[str, list] = {}
    for case in cases:
        for iid in case.get("invariant_ids") or []:
            by_id.setdefault(iid, []).append(case)
    ceiling = max(1, int(len(cases) * MAX_ID_CLAIM_SHARE))
    over = [(iid, len(members)) for iid, members in sorted(by_id.items())
            if len(members) > ceiling]
    require(not over,
            f"{len(over)} invariant id(s) are claimed by more than {ceiling} "
            f"case(s); one id may not carry more than a third of the matrix: "
            f"{over[:3]}")
    uncited = [(c.get("case_id"), iid) for iid, members in sorted(by_id.items())
               for c in members if iid not in basis_of(c)]
    require(not uncited,
            f"{len(uncited)} case(s) claim an invariant id their own basis "
            f"does not name; a shared id must be justified per case: "
            f"{uncited[:5]}")

    # (d) the mapping-status vocabulary is the registered one
    registry = ROOT / TX_REL
    if not registry.is_file():
        skip("M7/vocabulary", f"{TX_REL} does not exist")
        vocabulary = set(MAPPING_STATUSES)
    else:
        vocabulary = mapping_status_vocabulary(registry)
        require(vocabulary,
                f"{TX_REL} declares no invariant_mapping_status vocabulary "
                f"under the heading '{MAPPING_VOCAB_HEADING}'; the matrix's "
                f"mapping status cannot be parsed without it")
        require(set(MAPPING_STATUSES) == vocabulary,
                f"the tool's mapping-status vocabulary {sorted(MAPPING_STATUSES)} "
                f"does not match the registered one {sorted(vocabulary)} in "
                f"{TX_REL}")
    unknown = sorted({c.get("invariant_mapping_status") for c in cases}
                     - vocabulary)
    require(not unknown,
            f"{len(unknown)} case(s) carry an invariant_mapping_status outside "
            f"the registered vocabulary {sorted(vocabulary)}: {unknown}")

    # (e) the gap record and the matrix agree, in both directions
    empty = [c for c in cases if not c.get("invariant_ids")]
    listed = "no empty mapping to register"
    if empty:
        path = ROOT / GAP_REL
        if not path.is_file():
            skip("M7/gap-proposals", f"{GAP_REL} does not exist")
            listed = f"{len(empty)} empty mapping(s), the gap record is missing"
        else:
            registered = gap_index_cases(path)
            unlisted = sorted(c.get("case_id") for c in empty
                              if c.get("case_id") not in registered)
            require(not unlisted,
                    f"{len(unlisted)} case(s) declare an empty mapping but are "
                    f"not listed in the index table of {GAP_REL}: {unlisted[:5]}")
            stale = sorted(registered - {c.get("case_id") for c in empty})
            require(not stale,
                    f"{len(stale)} case(s) are listed in the index table of "
                    f"{GAP_REL} but do not declare an empty mapping: {stale[:5]}")
            listed = (f"{len(empty)} empty mapping(s), all listed in the index "
                      f"table of {GAP_REL}")

    labels: dict[str, list] = {}
    for case in cases:
        text = (case.get("source") or {}).get("declared_invariant")
        labels.setdefault(text, []).append(case)
    shared = {text: members for text, members in labels.items() if len(members) > 1}
    divergent = sum(1 for members in shared.values()
                    if len({tuple(m.get("invariant_ids") or []) for m in members}) > 1)
    claims = ", ".join(f"{iid}×{len(members)}" for iid, members in sorted(by_id.items()))
    return (f"{len(cases)} case(s) carry a per-case basis (max pairwise similarity "
            f"< {BASIS_SIMILARITY_LIMIT}); {listed}; "
            f"{len(by_id)} invariant id(s) claimed, each justified per case "
            f"[{claims}]; {len(shared)} declared-invariant label(s) still shared, "
            f"{divergent} of them with divergent id sets; "
            f"FORM ONLY -- substantive agreement with the test source is a "
            f"human/Oracle review item")


def main():
    parser = argparse.ArgumentParser(
        description="Verify the v4.1.0 75-scenario coverage matrix (M1-M7).")
    parser.add_argument("--strict", action="store_true",
                        help="fail when a cross-check could not be run")
    args = parser.parse_args()

    if sys.flags.optimize:
        print("refusing to run under python -O: assert statements are stripped, "
              "this gate would be a no-op")
        return 2
    if not (ROOT / MATRIX_REL).is_file():
        print(f"FAIL  {MATRIX_REL} does not exist")
        print("0/7 PASS")
        return 1
    try:
        matrix = read_json(MATRIX_REL)
    except MatrixError as exc:
        print(f"FAIL  {exc}")
        print("0/7 PASS")
        return 1

    checks = (("M1", m1), ("M2", m2), ("M3", m3),
              ("M4", m4), ("M5", m5), ("M6", m6), ("M7", m7))
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
