#!/usr/bin/env python3
"""Matrix-driven adapter over the pilot test factory (v4.1.0, Task 5).

The pilot's `test_factory.py` is a *filter-based* executor: one invocation is
one package, one test target and N `--filter` strings.  The coverage matrix,
on the other hand, is a list of 75 scenarios, each with its own test name.
This module is the adapter between the two: it reads the matrix, groups the
selected cases by (package, target), drives the factory once per group and
turns the factory's result into one verdict per case.

What the adapter refuses to do
------------------------------
  * It never reports `PASS` for a run that produced no test.  The factory
    already refuses that (its zero-test counterexample fix) and the adapter
    keeps the refusal: a case whose test was not seen is `NOT_RUN`, never
    `PASS`.  The matrix's `cargo test --list` snapshot is 242 commits older
    than the baseline, so "the test still exists" cannot be assumed.
  * It never collapses a deliberate RED into a plain `FAIL`.  Two independent
    signals say a case is a deliberate RED -- the matrix's own registration
    (`source.claimed_status == "KNOWN_FAIL"`) and the pilot's rerun record
    (`FAILED_RED` / `COVERED_RED`, which map onto `KNOWN_FAIL`).  Either one
    is enough: a missing or stale external record must not be able to turn a
    registered RED into a plain `FAIL`, so a selected case with no pilot
    record aborts the slice instead of being judged.  A `KNOWN_FAIL` must
    carry the five-element RED identity (`test_fn`, `package_or_target`,
    `source_sha256`, `rc`, `attribution`); a registered RED that carries no
    identity cannot be judged, so it is reported `INFRA_ERROR` instead of
    being silently passed or silently failed.
  * It does not assume a scenario's test still exists.  A bespoke probe
    (kill-9 crash recovery, checkpoint torn write, ODKU rollback no-op) is
    reported `NOT_RUN` with the reason *until it is migrated*: a migrated
    probe is looked up in the probe registry (or by the convention that its
    source carries the label's slug) and is then executed like any other
    test.  A bespoke RED without the five-element identity may only be
    reported `NOT_RUN`.

The vocabulary
--------------
The matrix carries the mapping from the pilot's vocabulary onto the
seven-word contract vocabulary of `05_TEST_FACTORY_CONTRACT.json` in its
top-level `status_vocabulary_mapping` block.  The adapter reads that block,
and refuses to run when it differs from the copy the adapter ships with: a
drift between the two would let the matrix launder a RED (`FAILED_RED`
mapping onto `FAIL`), so the two must not diverge.

Usage
-----
    python3 matrix_driver.py --matrix <matrix.json> --slice 0,1,2 --out <dir>
    python3 matrix_driver.py --matrix <matrix.json> --slice T1,T4 --out <dir>

The command line is a thin wrapper over `run_slice()`; it does not repeat any
of the logic.  It exits 0 only when every case in the slice is `PASS` -- the
same convention as the factory it drives -- so a slice that still carries a
known RED or a case that did not run is a blocking state, not a pass.  Note
that this is a statement about the *slice*, not about the matrix: while
`required_for_certification` is 75/75 `false` (that is the owner's stage-exit
criterion), a slice with a registered RED always exits non-zero.  Consumers
such as Task 6 read the per-case verdicts, not the exit code.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[4]

MATRIX_REL = "docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json"
CASE_MAPPING_REL = "evidence/case-mapping-20261120.json"
FACTORY_REL = "docs/releases/v4.1.0/refactoring/tools/test_factory.py"

# The seven-word vocabulary of 05_TEST_FACTORY_CONTRACT.json.
CONTRACT_VOCABULARY = ("PASS", "FAIL", "KNOWN_FAIL", "UNSUPPORTED",
                       "NOT_RUN", "DIALECT_DIFFERENCE", "INFRA_ERROR")
# The pilot's own vocabulary (evidence/case-mapping-20261120.json).
PILOT_VOCABULARY = ("PASS", "FAILED_RED", "COVERED_PASS", "COVERED_RED")
# The words that stand for a deliberate RED.
RED_VOCABULARY = ("FAILED_RED", "COVERED_RED")
# The five elements of a RED identity.
RED_IDENTITY_FIELDS = ("test_fn", "package_or_target", "source_sha256",
                       "rc", "attribution")
# The key a RED identity uses to declare that one of its elements could not
# be recovered.
GAP_FIELD = "identity_gap"
# The fixed shape of a verdict: these five keys are what a consumer (Task 6)
# writes back into the matrix.
VERDICT_KEYS = ("case_id", "execution_status", "actual", "artifact_sha256",
                "red_identity")

# The adapter's own copy of the mapping.  The matrix carries the same block;
# the two must not diverge, and the adapter refuses to run when they do.
DEFAULT_STATUS_MAPPING = {
    "PASS": "PASS",
    "COVERED_PASS": "PASS",
    "FAILED_RED": "KNOWN_FAIL",
    "COVERED_RED": "KNOWN_FAIL",
    "FAIL": "FAIL",
    "NOT_RUN": "NOT_RUN",
    "INFRA_ERROR": "INFRA_ERROR",
    "UNSUPPORTED": "UNSUPPORTED",
    "DIALECT_DIFFERENCE": "DIALECT_DIFFERENCE",
}


class DriverError(Exception):
    """The adapter cannot judge the slice it was asked for."""


# --------------------------------------------------------------------------
# inputs
# --------------------------------------------------------------------------

def load_matrix(path: pathlib.Path) -> dict:
    path = pathlib.Path(path)
    if not path.is_file():
        raise DriverError(f"{path} does not exist")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except ValueError as exc:
        raise DriverError(f"{path} is not valid JSON: {exc}") from exc


def status_mapping(matrix: dict) -> dict:
    """The pilot -> contract vocabulary map, read from the matrix.

    The matrix's block must be the map the adapter ships with.  Two things
    are asserted, and both exist to stop a RED being laundered:

      * the pilot's RED words must map onto `KNOWN_FAIL` -- a corrupted
        matrix that sent `FAILED_RED` to `FAIL` would turn every deliberate
        RED into an ordinary failure;
      * the matrix's block and the adapter's own copy must not diverge -- a
        silent drift between the two would let the matrix decide on its own
        what a RED means.
    """
    mapping = matrix.get("status_vocabulary_mapping")
    if not isinstance(mapping, dict) or not mapping:
        raise DriverError(
            "the matrix carries no status_vocabulary_mapping block; the pilot "
            "vocabulary cannot be mapped onto the contract vocabulary")
    bad = sorted({t for t in mapping.values() if t not in CONTRACT_VOCABULARY})
    if bad:
        raise DriverError(f"the vocabulary mapping targets {bad}, which are "
                          f"not contract vocabulary")
    laundered = [word for word in RED_VOCABULARY
                 if mapping.get(word) != "KNOWN_FAIL"]
    if laundered:
        raise DriverError(f"the vocabulary mapping sends {laundered} to "
                          f"{[mapping.get(w) for w in laundered]}; a deliberate "
                          f"RED must map onto KNOWN_FAIL")
    if mapping != DEFAULT_STATUS_MAPPING:
        raise DriverError(
            f"the matrix's status_vocabulary_mapping differs from the "
            f"adapter's own copy: {sorted(set(mapping) ^ set(DEFAULT_STATUS_MAPPING))} "
            f"differ in membership, "
            f"{sorted(k for k in set(mapping) & set(DEFAULT_STATUS_MAPPING) if mapping[k] != DEFAULT_STATUS_MAPPING[k])} "
            f"differ in value; the two must not diverge")
    return mapping


def is_registered_red(case: dict, declared) -> bool:
    """Whether a case is registered as a deliberate RED.

    Two independent signals: the matrix's own registration
    (`source.claimed_status == "KNOWN_FAIL"`) and the pilot's rerun record
    (mapped onto `KNOWN_FAIL`).  Either one is enough -- an external record
    that is missing or out of date must not be able to turn a registered RED
    into a plain `FAIL`.
    """
    claimed = (case.get("source") or {}).get("claimed_status")
    return claimed == "KNOWN_FAIL" or declared == "KNOWN_FAIL"


def map_status(status, mapping: dict):
    """The contract-vocabulary status of a pilot (or contract) status.

    A contract word the map does not list maps onto itself: `KNOWN_FAIL` is a
    target of the mapping, never a key.
    """
    if status is None:
        return None
    if status in mapping:
        return mapping[status]
    if status in CONTRACT_VOCABULARY:
        return status
    raise DriverError(f"unknown status {status!r}")


def load_case_mapping(repo: pathlib.Path) -> dict:
    """{matrix_index: record} from the pilot's own rerun record."""
    path = pathlib.Path(repo) / CASE_MAPPING_REL
    if not path.is_file():
        raise DriverError(f"{CASE_MAPPING_REL} does not exist; the pilot's own "
                          f"rerun record is the only source of the pilot "
                          f"vocabulary")
    try:
        records = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as exc:
        raise DriverError(f"{CASE_MAPPING_REL} is not valid JSON: {exc}") from exc
    by_index = {}
    for record in records:
        by_index[record["matrix_index"]] = record
    return by_index


def resolve_indices(matrix: dict, cases) -> list:
    """The matrix indices a slice names.

    An element may be a matrix index, a `case_id` or a feature name (T1..T6);
    the adapter accepts all three so that a slice can be given in whichever
    way the caller has at hand.  A matrix index names exactly one case.
    """
    matrix_cases = matrix.get("cases") or []
    selected: list[int] = []
    for element in cases:
        token = str(element).strip()
        if not token:
            continue
        if token.isdigit():
            index = int(token)
            if not 0 <= index < len(matrix_cases):
                raise DriverError(f"matrix index {index} is out of range "
                                  f"(0..{len(matrix_cases) - 1})")
            selected.append(index)
            continue
        by_id = [i for i, c in enumerate(matrix_cases)
                 if c.get("case_id") == token]
        if by_id:
            selected.extend(by_id)
            continue
        by_feature = [i for i, c in enumerate(matrix_cases)
                      if c.get("feature") == token]
        if by_feature:
            selected.extend(by_feature)
            continue
        raise DriverError(f"{token!r} names neither a matrix index, a case_id "
                          f"nor a feature")
    seen = set()
    ordered = []
    for index in selected:
        if index not in seen:
            seen.add(index)
            ordered.append(index)
    return ordered


# --------------------------------------------------------------------------
# where a test lives
# --------------------------------------------------------------------------

def package_name(manifest: pathlib.Path) -> str:
    """The package name a Cargo.toml declares."""
    try:
        import tomllib
    except ModuleNotFoundError as exc:  # pragma: no cover - python < 3.11
        raise DriverError("python 3.11 or newer is required to read Cargo.toml"
                          ) from exc
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except Exception as exc:
        raise DriverError(f"{manifest} cannot be parsed: {exc}") from exc
    name = (data.get("package") or {}).get("name")
    if not name:
        raise DriverError(f"{manifest} declares no package name")
    return name


def defines_test(path: pathlib.Path, fn_name: str) -> bool:
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return False
    return re.search(rf"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+"
                     rf"{re.escape(fn_name)}\s*[(<]", text, re.M) is not None


def registered_test_targets(manifest: pathlib.Path) -> dict:
    """{test source path: target name} for a package's [[test]] blocks.

    The paths are the ones the manifest declares, relative to the package
    directory.  A test source that is not declared here is only reachable
    when cargo discovers it by itself: `<package>/tests/<stem>.rs`.
    """
    if not manifest.is_file():
        return {}
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover - python < 3.11
        return {}
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except Exception:
        return {}
    targets = {}
    for block in data.get("test") or []:
        path = block.get("path")
        name = block.get("name")
        if path and name:
            targets[str(pathlib.PurePosixPath(path))] = name
    return targets


def test_sources(repo: pathlib.Path):
    """Every runnable integration test source of the workspace.

    Yields (package, source path, cargo target name).  Only the sources
    cargo can actually run are yielded: `<package>/tests/<stem>.rs`, which
    cargo discovers by itself, and the sources a manifest declares in a
    [[test]] block.
    """
    repo = pathlib.Path(repo)
    root_manifest = repo / "Cargo.toml"
    if root_manifest.is_file():
        package = package_name(root_manifest)
        declared = registered_test_targets(root_manifest)
        tests = repo / "tests"
        if tests.is_dir():
            for path in sorted(tests.rglob("*.rs")):
                relative = path.relative_to(repo)
                key = relative.as_posix()
                if len(relative.parts) == 2:  # tests/<stem>.rs is discovered
                    yield package, path, path.stem
                elif key in declared:
                    yield package, path, declared[key]
    for crate_dir in sorted((repo / "crates").glob("*")):
        manifest = crate_dir / "Cargo.toml"
        tests = crate_dir / "tests"
        if not manifest.is_file() or not tests.is_dir():
            continue
        package = package_name(manifest)
        declared = registered_test_targets(manifest)
        for path in sorted(tests.rglob("*.rs")):
            relative = path.relative_to(crate_dir)
            key = relative.as_posix()
            if len(relative.parts) == 2:  # tests/<stem>.rs is discovered
                yield package, path, path.stem
            elif key in declared:
                yield package, path, declared[key]


def unregistered_test_source(repo: pathlib.Path, fn_name: str):
    """The source that defines `fn_name` but that cargo cannot run."""
    repo = pathlib.Path(repo)
    roots = []
    root_manifest = repo / "Cargo.toml"
    if root_manifest.is_file():
        roots.append((repo, package_name(root_manifest)))
    for crate_dir in sorted((repo / "crates").glob("*")):
        manifest = crate_dir / "Cargo.toml"
        if manifest.is_file():
            roots.append((crate_dir, package_name(manifest)))
    for base, _package in roots:
        tests = base / "tests"
        if not tests.is_dir():
            continue
        for path in sorted(tests.rglob("*.rs")):
            if defines_test(path, fn_name):
                relative = path.relative_to(repo)
                return (f"{relative} defines {fn_name} but is not registered "
                        f"as a cargo test target")
    return None


def resolve_target(repo: pathlib.Path, test_name: str):
    """(package, target, note) for a test name.

    `target` is None for a unit test (the crate's lib) and the cargo test
    target name for an integration test.  `package` is None when the test
    cannot be run at all; `note` then says why.
    """
    repo = pathlib.Path(repo)
    fn_name = test_name.split("::")[-1]

    if "::" in test_name:
        # A unit test: `module::path::fn` names a module inside a crate.
        module = test_name.split("::")[0]
        hits = []
        crates = repo / "crates"
        if crates.is_dir():
            for crate_dir in sorted(crates.iterdir()):
                manifest = crate_dir / "Cargo.toml"
                source = crate_dir / "src"
                if not manifest.is_file() or not source.is_dir():
                    continue
                for path in sorted(source.rglob("*.rs")):
                    if defines_test(path, fn_name):
                        hits.append((package_name(manifest), path))
        if not hits:
            return None, None, (f"no crate in the workspace defines {fn_name}; "
                                f"the test is not in the baseline any more")
        # The module path names the file the test lives in; when several
        # files define the function, the one the module path points at wins.
        for package, path in hits:
            if path.stem == module or path.parent.name == module:
                return package, None, ""
        return hits[0][0], None, ""

    # An integration test: the test name is the function itself.
    for package, path, target in test_sources(repo):
        if defines_test(path, fn_name):
            return package, target, ""
    note = unregistered_test_source(repo, fn_name)
    if note:
        return None, None, note
    return None, None, (f"no test source in the workspace defines {fn_name}; "
                        f"the test is not in the baseline any more")


# --------------------------------------------------------------------------
# bespoke probes
# --------------------------------------------------------------------------

def load_probe_registry(matrix: dict) -> dict:
    """{label: binding} for the bespoke probes that have been migrated.

    The bindings live in the matrix's own `probe_registry` block, next to the
    vocabulary mapping: they are part of the scenario catalogue, and the
    matrix is hash-covered by the handoff manifest.
    """
    registry = matrix.get("probe_registry")
    if registry is None:
        return {}
    if not isinstance(registry, dict):
        raise DriverError("the matrix's probe_registry is not an object")
    probes = registry.get("probes")
    if probes is None:
        return {}
    if not isinstance(probes, dict):
        raise DriverError("the matrix's probe_registry carries no probes object")
    return probes


def probe_slug(label: str) -> str:
    """The source stem a migrated probe of `label` would carry."""
    return re.sub(r"[^0-9A-Za-z]+", "_", label.strip().strip("()")).strip("_").lower()


def resolve_probe(matrix: dict, repo: pathlib.Path, label: str):
    """(package, target, filter, note) for a bespoke probe label.

    A bespoke probe is migrated as a cargo test target.  It is found either
    through the probe registry, which binds a label to a target explicitly,
    or by convention: the source whose stem is the label with its punctuation
    folded to underscores (`(crash-recovery-kill-9)` -> `crash_recovery_kill_9`).
    `package` is None when no probe has been migrated; `note` then says why.
    """
    registry = load_probe_registry(matrix)
    key = label.strip().strip("()")
    binding = registry.get(key) or registry.get(label)
    if isinstance(binding, dict) and binding.get("package"):
        return (binding["package"], binding.get("target"),
                binding.get("filter") or probe_slug(key), "")
    if binding is not None and not isinstance(binding, dict):
        raise DriverError(f"the probe registry binds {key!r} to {binding!r}, "
                          f"which is not an object")
    slug = probe_slug(key)
    for package, path, target in test_sources(repo):
        if path.stem.lower() == slug:
            return package, target, slug, ""
    return None, None, None, (
        f"no migrated probe for the label {label}: the matrix's probe_registry "
        f"binds no target for it and the baseline holds no test source named "
        f"{slug}")


# --------------------------------------------------------------------------
# the factory
# --------------------------------------------------------------------------

def sha256_file(path: pathlib.Path) -> str:
    try:
        return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()
    except OSError:
        return ""


def run_factory(repo: pathlib.Path, package: str, target, filters, suite: str,
                out_dir: pathlib.Path, timeout: int = 1800):
    """Drive the pilot factory once; return (result, artifact).

    `result` is the factory's structured result (or None when the factory
    itself could not run) and `artifact` describes the evidence it wrote.
    """
    factory = HERE / "test_factory.py"
    if not factory.is_file():
        raise DriverError(f"{factory} does not exist; the pilot's factory must "
                          f"be migrated into the baseline first")
    cmd = [sys.executable, str(factory), "run",
           "--repo", str(repo), "--suite", suite, "--package", package,
           "--out", str(out_dir), "--timeout", str(timeout)]
    if target:
        cmd += ["--test", target]
    for name in filters:
        cmd += ["--filter", name]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True,
                              timeout=timeout + 300)
    except subprocess.TimeoutExpired:
        return None, {"command": " ".join(cmd),
                      "path": None, "sha256": None, "digest": None}
    payload = None
    for line in reversed(proc.stdout.splitlines()):
        line = line.strip()
        if line.startswith("{"):
            try:
                payload = json.loads(line)
                break
            except ValueError:
                continue
    if payload is None:
        return None, {"command": " ".join(cmd),
                      "path": None, "sha256": None, "digest": None,
                      "stderr": proc.stderr.strip()[-400:]}
    evidence = pathlib.Path(payload.get("evidence", ""))
    if not evidence.is_file():
        return None, {"command": " ".join(cmd),
                      "path": None, "sha256": None, "digest": None}
    try:
        packed = json.loads(evidence.read_text(encoding="utf-8"))
    except ValueError:
        packed = {}
    raw_log = pathlib.Path(packed.get("raw_log", ""))
    artifact = {
        "command": " ".join(cmd),
        "path": str(raw_log) if raw_log.is_file() else None,
        "sha256": sha256_file(raw_log) if raw_log.is_file() else None,
        "digest": packed.get("artifact_digest"),
        "evidence": str(evidence),
    }
    return packed.get("result"), artifact


# --------------------------------------------------------------------------
# the verdict
# --------------------------------------------------------------------------

def coverage_source(case: dict, pilot_record) -> str:
    """Where the coverage claim of a case comes from."""
    parts = []
    if pilot_record:
        if pilot_record.get("evidence"):
            parts.append(str(pilot_record["evidence"]))
        if pilot_record.get("note"):
            parts.append(str(pilot_record["note"]))
    if not parts:
        declared = (case.get("source") or {}).get("declared_source")
        if declared:
            parts.append(str(declared))
    return "; ".join(parts)


def complete_identity(identity) -> bool:
    """Whether a RED identity carries all five elements, non-empty.

    An identity that declares a gap (`identity_gap`) is not complete: the
    element it cannot supply is still missing, and the reader is told which
    one it is.
    """
    if not isinstance(identity, dict):
        return False
    if identity.get(GAP_FIELD):
        return False
    for field in RED_IDENTITY_FIELDS:
        value = identity.get(field)
        if value is None:
            return False
        if isinstance(value, str) and not value.strip():
            return False
    return True


def identity_gap(identity) -> str:
    """The declared gap of a RED identity ("" when there is none)."""
    if not isinstance(identity, dict):
        return ""
    gap = identity.get(GAP_FIELD)
    return gap if isinstance(gap, str) else ""


def find_test(tests, expected: str):
    """The factory's test entry for `expected`, or None.

    An exact name wins; a substring match is only accepted when it is
    unambiguous, because the factory filters by substring and a filter that
    matches several tests cannot be attributed to one case.
    """
    exact = [t for t in tests if t.get("name") == expected]
    if exact:
        return exact[0]
    partial = [t for t in tests if expected in (t.get("name") or "")]
    return partial[0] if len(partial) == 1 else None


def evaluate_case(case: dict, index, pilot_record, mapping: dict,
                  factory_result=None, artifact=None, note: str = "",
                  expected_name: str = None, probe: bool = False) -> dict:
    """The verdict for one case.

    `factory_result` is the factory's structured result for the group the
    case belongs to; None means the case was not executed at all.
    `expected_name` is the name the factory knows the case by (the matrix's
    test name for a cargo test, the probe's filter for a bespoke probe).
    """
    pilot_status = (pilot_record or {}).get("rerun_status")
    declared = map_status(pilot_status, mapping)
    identity = case.get("red_identity")
    test_name = (case.get("source") or {}).get("test_name", "")
    expected = expected_name or test_name
    artifact = artifact or {}
    entry = {
        "case_id": case.get("case_id"),
        "matrix_index": index,
        "execution_status": "NOT_RUN",
        "actual": "",
        "artifact_sha256": artifact.get("sha256"),
        "red_identity": identity,
        "pilot_status": pilot_status,
        "declared_status": declared,
        "coverage_source": coverage_source(case, pilot_record),
        "artifact_path": artifact.get("path"),
        "command": artifact.get("command"),
    }

    if factory_result is None:
        if artifact.get("command"):
            # The factory was asked to run and produced no verdict: the run
            # could not be judged.  That is not the same as "the case did not
            # run" -- it is the runner that broke.
            entry["execution_status"] = "INFRA_ERROR"
            entry["actual"] = (f"{artifact['command']} → the factory produced "
                               f"no verdict: "
                               f"{artifact.get('stderr') or 'see the factory output'}")
        else:
            entry["execution_status"] = "NOT_RUN"
            entry["actual"] = note or "the case was not executed"
        return entry

    factory_class = factory_result.get("class")
    command = artifact.get("command", "")
    if factory_class == "INFRA_ERROR":
        entry["execution_status"] = "INFRA_ERROR"
        entry["actual"] = (f"{command} → INFRA_ERROR: "
                            f"{factory_result.get('note') or 'the run could not be judged'}")
        return entry

    found = find_test(factory_result.get("tests") or [], expected)

    if found is None:
        entry["execution_status"] = "NOT_RUN"
        if factory_class == "NOT_RUN":
            entry["actual"] = (f"{command} → the factory parsed no test line "
                               f"(zero tests); {expected} did not run")
        else:
            entry["actual"] = (f"{command} → {expected} was not among the "
                               f"tests the factory saw; the test may no longer "
                               f"exist")
        return entry

    test_class = found.get("class")
    if is_registered_red(case, declared):
        # A registered RED keeps its identity whether it reproduces or not:
        # a red that turned green is not a discharge, it is a trigger for
        # independent re-verification (02_TEST_EXECUTION_CONTRACT.md, step 8).
        if not complete_identity(identity):
            if probe:
                # A bespoke RED may only be reported NOT_RUN until its
                # identity is complete: the probe is not a cargo test, so
                # there is no other way to report what happened.
                entry["execution_status"] = "NOT_RUN"
                gap = identity_gap(identity) or "the case carries no RED identity"
                entry["actual"] = (f"{command} → {expected} is a registered RED "
                                   f"({pilot_status}) and {gap}; the case may "
                                   f"only be reported NOT_RUN")
                return entry
            entry["execution_status"] = "INFRA_ERROR"
            entry["actual"] = (f"{command} → {expected} is a registered RED "
                               f"({pilot_status}) but the case carries no "
                               f"complete RED identity; the verdict cannot be "
                               f"judged")
            return entry
        entry["execution_status"] = "KNOWN_FAIL"
        if test_class == "PASS":
            entry["actual"] = (f"{command} → {expected} ... ok — "
                               f"UNEXPECTED_GREEN: the case is a registered RED "
                               f"({pilot_status}); it may only be closed by "
                               f"independent re-verification")
        else:
            entry["actual"] = (f"{command} → {expected} ... FAILED — the "
                               f"registered RED ({pilot_status}) reproduced")
        return entry

    if test_class == "PASS":
        entry["execution_status"] = "PASS"
        entry["actual"] = f"{command} → {expected} ... ok"
    elif test_class == "FAIL":
        entry["execution_status"] = "FAIL"
        entry["actual"] = f"{command} → {expected} ... FAILED"
    elif test_class == "KNOWN_FAIL":
        # The factory classified the test as a known fail.  That is still a
        # deliberate RED, so it carries the same identity gate: a KNOWN_FAIL
        # without the five elements cannot be judged, and it must not reach
        # the consumer as if it could.
        if not complete_identity(identity):
            entry["execution_status"] = "INFRA_ERROR"
            entry["actual"] = (f"{command} → {expected} was classified as a "
                               f"known fail but the case carries no complete "
                               f"RED identity; the verdict cannot be judged")
            return entry
        entry["execution_status"] = "KNOWN_FAIL"
        entry["actual"] = f"{command} → {expected} ... FAILED (known fail)"
    elif test_class == "UNSUPPORTED":
        entry["execution_status"] = "UNSUPPORTED"
        entry["actual"] = f"{command} → {expected} ... ignored"
    else:
        entry["execution_status"] = "NOT_RUN"
        entry["actual"] = (f"{command} → {expected} was not judged "
                           f"(class {test_class!r})")
    return entry


# --------------------------------------------------------------------------
# the slice
# --------------------------------------------------------------------------

def run_slice(repo: pathlib.Path, matrix: pathlib.Path, cases: list,
               out_dir: pathlib.Path) -> list:
    """Execute the matrix_index set `cases` names; return one verdict each.

    Every verdict carries `case_id`, `execution_status`, `actual`,
    `artifact_sha256` and `red_identity`; `execution_status` is one of the
    seven contract words, and a `KNOWN_FAIL` carries the five-element RED
    identity.
    """
    repo = pathlib.Path(repo).resolve()
    out_dir = pathlib.Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    data = load_matrix(pathlib.Path(matrix))
    mapping = status_mapping(data)
    matrix_cases = data.get("cases") or []
    indices = resolve_indices(data, cases)
    pilot = load_case_mapping(repo)

    verdicts: dict[int, dict] = {}
    groups: dict = {}
    for index in indices:
        case = matrix_cases[index]
        record = pilot.get(index)
        if record is None:
            # The pilot's rerun record is one of the two signals that say a
            # case is a deliberate RED.  A selected case without a record is
            # a broken input, not a case without a RED: judging it would let
            # a missing file turn a registered RED into a plain FAIL.
            raise DriverError(
                f"matrix index {index} ({case.get('case_id')}) has no record "
                f"in {CASE_MAPPING_REL}; the pilot's rerun status is the RED "
                f"signal, and a missing record must not silently change a "
                f"registered RED into a FAIL")
        test_name = (case.get("source") or {}).get("test_name", "")
        if test_name.startswith("("):
            package, target, probe_filter, note = resolve_probe(data, repo, test_name)
            if package is None:
                verdicts[index] = evaluate_case(
                    case, index, record, mapping, None, None, note=note)
                continue
            groups.setdefault((package, target), []).append(
                (index, probe_filter, True))
            continue
        package, target, note = resolve_target(repo, test_name)
        if package is None:
            verdicts[index] = evaluate_case(
                case, index, record, mapping, None, None, note=note)
            continue
        groups.setdefault((package, target), []).append((index, test_name, False))

    for (package, target), group in groups.items():
        suite = "MATRIX-" + package + ("-" + target if target else "-lib")
        filters = [item[1] for item in group]
        result, artifact = run_factory(repo, package, target, filters, suite,
                                       out_dir)
        for index, expected, is_probe in group:
            verdicts[index] = evaluate_case(matrix_cases[index], index,
                                            pilot.get(index), mapping, result,
                                            artifact, expected_name=expected,
                                            probe=is_probe)

    results = [verdicts[index] for index in indices]
    stamp = time.strftime("%Y%m%dT%H%M%S")
    summary = out_dir / f"matrix-slice-{stamp}.json"
    summary.write_text(json.dumps(results, ensure_ascii=False, indent=2),
                       encoding="utf-8")
    return results


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Execute a slice of the v4.1.0 coverage matrix through the "
                    "pilot test factory.")
    parser.add_argument("--matrix", default=str(ROOT / MATRIX_REL),
                        help="the coverage matrix (default: the baseline's)")
    parser.add_argument("--slice", dest="slice_", default="",
                        help="comma separated matrix indices, case ids or "
                             "feature names (e.g. 0,1,2 or T1,T4)")
    parser.add_argument("--out", default="evidence/matrix-slice",
                        help="where the factory writes its evidence")
    parser.add_argument("--repo", default=str(ROOT),
                        help="the worktree root (default: the tool's own root)")
    args = parser.parse_args(argv)

    cases = [token for token in args.slice_.split(",") if token.strip()]
    if not cases:
        parser.error("nothing to do: --slice names the matrix indices to run")
    try:
        results = run_slice(pathlib.Path(args.repo), pathlib.Path(args.matrix),
                            cases, pathlib.Path(args.out))
    except DriverError as exc:
        print(f"FAIL  {exc}")
        return 1
    for entry in results:
        print(f"{entry['case_id']:16s} {entry['execution_status']:18s} "
              f"{entry['actual']}")
    counts: dict[str, int] = {}
    for entry in results:
        counts[entry["execution_status"]] = counts.get(entry["execution_status"], 0) + 1
    summary = ", ".join(f"{status}={count}" for status, count in sorted(counts.items()))
    print(f"{len(results)} case(s): {summary}")
    # Fail closed, like the factory it drives: a slice is green only when
    # every case in it is green.  A KNOWN_FAIL or a NOT_RUN is a blocking
    # state (INV-17), not a pass.
    return 0 if all(entry["execution_status"] == "PASS" for entry in results) else 1


if __name__ == "__main__":
    sys.exit(main())
