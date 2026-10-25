#!/usr/bin/env python3
"""trusted_baseline_verdict.py — the deterministic part of the CI trusted baseline (D2).

WHAT THIS FILE IS
-----------------
The Owner directive of 2026-10-25 asks for a CI that produces a *minimum
trustworthy baseline*:

  * a pinned commit,
  * fmt / clippy / check / test over everything,
  * JUnit plus `discovered` / `executed` / `ignored` counts with a minimum
    expectation,
  * an **independent** coverage measurement (cargo-llvm-cov, not a number
    somebody typed into a document),
  * an Evidence Manifest binding the commit SHA, the run ID and the
    SHA256 of every artifact,
  * a negative injection that must make the pipeline FAIL,
  * a **tri-state** verdict: PASS / FAIL / INCONCLUSIVE, where a glob that
    matches nothing is INCONCLUSIVE and never a vacuous PASS,
  * an independent verdict step that does not certify its own homework.

This script is the *measurement* half of that: it turns raw run artifacts
into a tri-state verdict plus a manifest. It is deliberately a plain,
deterministic program:

  * no network,
  * no model call,
  * no wall-clock dependency,
  * every input is a file whose bytes are hashed,
  * every decision rule is a named rule with a reason code.

The AI Judge (if one is ever attached) may only *explain* the verdict this
script produced. It may not change it.

EXIT CODES
----------
  0  verdict PASS
  1  verdict FAIL
  2  verdict INCONCLUSIVE
  3  usage error

SUBCOMMANDS
-----------
  junit     --cargo-log <log> --out <xml>
      Convert a cargo test log into JUnit XML. Deterministic: the same log
      always produces byte-identical XML.

  verdict   --counts <json> [--coverage <json>] [--manifest <json>]
            [--mode baseline|negative] [--out <json>]
      Produce the tri-state verdict and, with --manifest, the Evidence
      Manifest.

  selftest
      Run the built-in fixtures. They cover every branch of the decision
      table, including the two that matter most: a glob that matched
      nothing, and a negative injection that must not be able to pass.

Every subcommand also accepts --selftest, which runs the same fixtures.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

# --- decision table ---------------------------------------------------------
#
# The rules are data, not control flow, so that a reader can see the whole
# contract in one place and a test can assert it.
#
# rule id      condition                                   outcome
# ------------------------------------------------------------------
# R1_EMPTY     declared test targets == 0                    INCONCLUSIVE
# R2_COVERAGE  coverage tool missing                        INCONCLUSIVE
# R3_COVERAGE  measured coverage < minimum                  FAIL
# R4_COUNT     executed < minimum expected                  FAIL
# R5_TESTFAIL  one or more test failures                    FAIL
# R6_NEGATIVE  negative injection mode and no failure       FAIL
# R7_PASS      everything else                              PASS
#
# R1 is the rule the whole D2 directive is about: a measurement that
# looked at nothing must not be able to report success. R2 is the same
# idea applied to coverage: the existing pipeline in ci.yml answers a
# missing cargo-llvm-cov with "NOT-RUN" and `exit 0`, which is a vacuous
# pass wearing a NOT-RUN label.

RULES: tuple[dict[str, Any], ...] = (
    {
        "id": "R1_EMPTY",
        "outcome": "INCONCLUSIVE",
        "reason": "no declared test target was discovered; the measurement looked at nothing",
    },
    {
        "id": "R2_COVERAGE",
        "outcome": "INCONCLUSIVE",
        "reason": "the independent coverage tool was unavailable; coverage could not be measured",
    },
    {
        "id": "R3_COVERAGE",
        "outcome": "FAIL",
        "reason": "the independently measured line coverage is below the configured minimum",
    },
    {
        "id": "R4_COUNT",
        "outcome": "FAIL",
        "reason": "fewer test targets were executed than the configured minimum expectation",
    },
    {
        "id": "R5_TESTFAIL",
        "outcome": "FAIL",
        "reason": "at least one executed test failed",
    },
    {
        "id": "R6_NEGATIVE",
        "outcome": "FAIL",
        "reason": "negative injection mode was requested but the injected defect did not fail the run",
    },
    {
        "id": "R7_PASS",
        "outcome": "PASS",
        "reason": "every configured check produced a measured result and none of them failed",
    },
)

ANCHOR = "http://192.168.0.252:3000/openclaw/sqlrustgo"

# cargo prints one summary line per test binary:
#   test result: ok. 16 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s
# The unit-test binaries use "ok." / "FAILED."; integration binaries use
# "test result: FAILED. ...".
RESULT_RE = re.compile(
    r"^test result:\s*(?P<status>ok|FAILED)\.\s*"
    r"(?P<passed>\d+)\s+passed;\s*"
    r"(?P<failed>\d+)\s+failed;\s*"
    r"(?P<ignored>\d+)\s+ignored;",
    re.MULTILINE,
)

# cargo prints one line per test:
#   test engine::tests::foo ... ok
#   test engine::tests::bar ... FAILED
#   test engine::tests::baz ... ignored
TESTCASE_RE = re.compile(
    r"^test\s+(?P<name>\S+)\s+\.\.\.\s+(?P<status>ok|FAILED|ignored)\s*$",
    re.MULTILINE,
)

# "     Running tests/foo.rs (target/debug/deps/foo-hash)" and
# "     Running unittests src/lib.rs (target/debug/deps/sqlrustgo-hash)"
RUNNING_RE = re.compile(r"^\s*Running\s+(?P<label>.+?)\s+\(target/", re.MULTILINE)


def sha256_file(path: Path) -> str:
    """SHA256 of a file, streamed so that a large artifact does not need to fit in memory."""
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


# --- junit ------------------------------------------------------------------


def cargo_log_to_junit(cargo_log: str) -> ET.ElementTree:
    """Convert a cargo test log into a JUnit XML tree.

    The conversion is textual and therefore reproducible: the same log
    always yields the same tree, and the tree never contains a timestamp
    or any other value that would make two runs differ. That matters
    because the manifest binds artifact SHA256 values — a JUnit file that
    changed on every run would make the manifest useless.
    """
    suites: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    seen_running: set[str] = set()

    for line in cargo_log.splitlines():
        m = RUNNING_RE.match(line)
        if m:
            label = m.group("label")
            if label.startswith("unittests "):
                label = "unittests " + label.split(" ", 1)[1]
            if label not in seen_running:
                seen_running.add(label)
                current = {"name": label, "cases": [], "summary": None}
                suites.append(current)
            continue
        m = TESTCASE_RE.match(line)
        if m and current is not None:
            current["cases"].append(
                {"name": m.group("name"), "status": m.group("status")}
            )

    summaries = list(RESULT_RE.finditer(cargo_log))
    # Pair each summary line with the suite that was running when it was
    # printed. cargo prints "Running <target>" then the test lines then
    # the summary, so the nth summary belongs to the nth suite.
    for idx, sm in enumerate(summaries):
        if idx >= len(suites):
            break
        suites[idx]["summary"] = {
            "passed": int(sm.group("passed")),
            "failed": int(sm.group("failed")),
            "ignored": int(sm.group("ignored")),
        }

    root = ET.Element("testsuites", {"name": "cargo-test"})
    for suite in suites:
        summary = suite["summary"] or {"passed": 0, "failed": 0, "ignored": 0}
        node = ET.SubElement(
            root,
            "testsuite",
            {
                "name": suite["name"],
                "tests": str(
                    summary["passed"] + summary["failed"] + summary["ignored"]
                ),
                "failures": str(summary["failed"]),
                "ignored": str(summary["ignored"]),
                "errors": "0",
            },
        )
        for case in suite["cases"]:
            cnode = ET.SubElement(node, "testcase", {"name": case["name"], "classname": suite["name"]})
            if case["status"] == "FAILED":
                ET.SubElement(cnode, "failure", {"message": "test failed"})
            elif case["status"] == "ignored":
                ET.SubElement(cnode, "skipped", {"message": "ignored by the test itself"})
    return ET.ElementTree(root)


# --- counts -----------------------------------------------------------------


def counts_from_cargo_log(cargo_log: str) -> dict[str, int]:
    """Discovered / executed / failed / ignored, counted from the log itself.

    * executed  = the number of test binaries cargo actually ran
    * passed    = the number of test cases the binaries reported as passed
    * failed    = the number of test cases the binaries reported as failed
    * ignored   = the number of test cases the binaries reported as ignored
    """
    executed = len({m.group("label") for m in RUNNING_RE.finditer(cargo_log)})
    passed = failed = ignored = 0
    for m in RESULT_RE.finditer(cargo_log):
        passed += int(m.group("passed"))
        failed += int(m.group("failed"))
        ignored += int(m.group("ignored"))
    return {
        "executed_binaries": executed,
        "passed": passed,
        "failed": failed,
        "ignored": ignored,
    }


# --- verdict ----------------------------------------------------------------


def decide(
    counts: dict[str, Any],
    coverage: dict[str, Any] | None,
    min_executed: int,
    min_coverage_percent: float,
    mode: str,
    declared: int | None = None,
) -> dict[str, Any]:
    """Apply the decision table. Returns the verdict, the rules that fired and the reasons.

    `declared` is the number of test targets cargo metadata reports for the
    scope. It is the R1 input: when the declaration set is empty, the
    measurement has nothing to look at and cannot conclude anything.
    """
    reasons: list[str] = []
    fired: list[str] = []
    outcome = "PASS"
    fired_rule = "R7_PASS"

    def apply(rule_id: str) -> None:
        nonlocal outcome, fired_rule
        rule = next(r for r in RULES if r["id"] == rule_id)
        reasons.append(f"{rule_id}: {rule['reason']}")
        fired.append(rule_id)
        # FAIL dominates INCONCLUSIVE dominates PASS.
        if rule["outcome"] == "FAIL" or outcome == "PASS":
            outcome = rule["outcome"]
            fired_rule = rule_id

    if declared is not None and declared == 0:
        apply("R1_EMPTY")
    if coverage is None or coverage.get("status") != "measured":
        apply("R2_COVERAGE")
    elif float(coverage.get("line_percent", 0.0)) < min_coverage_percent:
        apply("R3_COVERAGE")

    executed = int(counts.get("executed_binaries", counts.get("executed", 0)))
    if executed < min_executed:
        apply("R4_COUNT")
    if int(counts.get("failed", 0)) > 0:
        apply("R5_TESTFAIL")

    if mode == "negative" and int(counts.get("failed", 0)) == 0:
        apply("R6_NEGATIVE")

    if outcome == "PASS":
        reasons.append("R7_PASS: every configured check produced a measured result")
        fired.append("R7_PASS")

    return {
        "verdict": outcome,
        "fired_rule": fired_rule,
        "rules_fired": fired,
        "reasons": reasons,
        "inputs": {
            "declared": declared,
            "min_executed": min_executed,
            "min_coverage_percent": min_coverage_percent,
            "mode": mode,
            "counts": counts,
            "coverage": coverage,
        },
    }


def build_manifest(
    commit: str, run_id: str, artifacts: list[Path], mode: str = "baseline"
) -> dict[str, Any]:
    """The Evidence Manifest: commit SHA + run ID + SHA256 of every artifact."""
    return {
        "schema": "trusted-baseline-manifest/v1",
        "repository": ANCHOR,
        "commit": commit,
        "run_id": run_id,
        "mode": mode,
        "artifacts": [
            {
                "path": str(p),
                "sha256": sha256_file(p),
                "bytes": p.stat().st_size,
            }
            for p in artifacts
            if p.is_file()
        ],
    }


# --- self test --------------------------------------------------------------


def _selftest() -> int:
    """Every branch of the decision table, proven by a fixture.

    The two fixtures that carry the whole point of D2 are `empty_glob` and
    `negative_injection_must_fail`: a measurement that looked at nothing,
    and a deliberately broken commit, must not be able to report PASS.
    """
    failures: list[str] = []

    def check(name: str, got: Any, want: Any) -> None:
        if got != want:
            failures.append(f"{name}: got {got!r}, want {want!r}")
        else:
            print(f"[OK]   {name}: {got!r}")

    good_counts = {"executed_binaries": 12, "passed": 400, "failed": 0, "ignored": 116}
    good_cov = {"status": "measured", "line_percent": 78.28, "tool": "cargo-llvm-cov"}

    v = decide(good_counts, good_cov, 1, 0.0, "baseline", declared=449)
    check("baseline_healthy_is_PASS", v["verdict"], "PASS")

    v = decide(good_counts, good_cov, 1, 0.0, "baseline", declared=0)
    check("empty_glob_is_INCONCLUSIVE", v["verdict"], "INCONCLUSIVE")
    check("empty_glob_rule", v["fired_rule"], "R1_EMPTY")

    v = decide(good_counts, None, 1, 0.0, "baseline", declared=449)
    check("coverage_tool_missing_is_INCONCLUSIVE", v["verdict"], "INCONCLUSIVE")
    check("coverage_tool_missing_rule", v["fired_rule"], "R2_COVERAGE")

    v = decide(good_counts, {"status": "measured", "line_percent": 10.0}, 1, 70.0, "baseline", declared=449)
    check("coverage_below_minimum_is_FAIL", v["verdict"], "FAIL")
    check("coverage_below_minimum_rule", v["fired_rule"], "R3_COVERAGE")

    v = decide({"executed_binaries": 0, "passed": 0, "failed": 0, "ignored": 0}, good_cov, 1, 0.0, "baseline", declared=449)
    check("nothing_executed_is_FAIL", v["verdict"], "FAIL")
    check("nothing_executed_rule", v["fired_rule"], "R4_COUNT")

    v = decide({"executed_binaries": 12, "passed": 399, "failed": 1, "ignored": 116}, good_cov, 1, 0.0, "baseline", declared=449)
    check("one_failed_test_is_FAIL", v["verdict"], "FAIL")
    check("one_failed_test_rule", v["fired_rule"], "R5_TESTFAIL")

    v = decide(good_counts, good_cov, 1, 0.0, "negative", declared=449)
    check("negative_injection_must_fail", v["verdict"], "FAIL")
    check("negative_injection_rule", v["fired_rule"], "R6_NEGATIVE")

    v = decide({"executed_binaries": 12, "passed": 399, "failed": 1, "ignored": 116}, good_cov, 1, 0.0, "negative", declared=449)
    check("negative_injection_actually_failed", v["verdict"], "FAIL")

    # JUnit conversion: deterministic and faithful.
    log = (
        "     Running unittests src/lib.rs (target/debug/deps/sqlrustgo-hash)\n"
        "test engine::tests::alpha ... ok\n"
        "test engine::tests::beta ... ok\n"
        "test engine::tests::gamma ... ignored\n"
        "test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
        "     Running tests/integration/thing.rs (target/debug/deps/thing-hash)\n"
        "test thing::works ... FAILED\n"
        "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s\n"
    )
    tree1 = cargo_log_to_junit(log)
    tree2 = cargo_log_to_junit(log)
    xml1 = ET.tostring(tree1.getroot(), encoding="unicode")
    xml2 = ET.tostring(tree2.getroot(), encoding="unicode")
    check("junit_is_deterministic", xml1 == xml2, True)
    check("junit_has_two_suites", len(tree1.getroot().findall("testsuite")), 2)
    check("junit_records_the_failure", len(tree1.getroot().findall(".//failure")), 1)
    check("junit_records_the_skip", len(tree1.getroot().findall(".//skipped")), 1)

    counts = counts_from_cargo_log(log)
    check("counts_executed_binaries", counts["executed_binaries"], 2)
    check("counts_passed", counts["passed"], 2)
    check("counts_failed", counts["failed"], 1)
    check("counts_ignored", counts["ignored"], 1)

    # An empty log is the vacuous case: it must never be counted as a pass.
    empty = counts_from_cargo_log("")
    check("empty_log_executed_zero", empty["executed_binaries"], 0)

    if failures:
        for f in failures:
            print(f"[FAIL] {f}", file=sys.stderr)
        print(f"selftest: {len(failures)} failure(s)", file=sys.stderr)
        return 1
    print("selftest: PASS")
    return 0


# --- cli --------------------------------------------------------------------


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return _selftest()

    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p_junit = sub.add_parser("junit", help="convert a cargo test log to JUnit XML")
    p_junit.add_argument("--cargo-log", required=True)
    p_junit.add_argument("--out", required=True)

    p_counts = sub.add_parser(
        "counts", help="merge the execution gate's JSON with the cargo log into one counts file"
    )
    p_counts.add_argument("--cargo-log", required=True)
    p_counts.add_argument("--gate-json", required=True, help="the --json output of check_test_target_execution.sh")
    p_counts.add_argument("--out", required=True)

    p_verdict = sub.add_parser("verdict", help="produce the tri-state verdict")
    p_verdict.add_argument("--counts", required=True, help="JSON file with the test counts")
    p_verdict.add_argument("--coverage", help="JSON file with the coverage measurement")
    p_verdict.add_argument("--manifest", help="JSON file to write the Evidence Manifest into")
    p_verdict.add_argument("--commit", default="", help="commit SHA the run judged")
    p_verdict.add_argument("--run-id", default="", help="the CI run ID")
    p_verdict.add_argument("--artifact", action="append", default=[], help="artifact to hash (repeatable)")
    p_verdict.add_argument("--min-executed", type=int, default=1)
    p_verdict.add_argument("--min-coverage-percent", type=float, default=0.0)
    p_verdict.add_argument("--mode", choices=("baseline", "negative"), default="baseline")
    p_verdict.add_argument("--out", help="JSON file to write the verdict into")

    args = ap.parse_args(argv)

    if args.cmd == "junit":
        log = Path(args.cargo_log).read_text(encoding="utf-8", errors="replace")
        tree = cargo_log_to_junit(log)
        Path(args.out).write_bytes(
            ET.tostring(tree.getroot(), encoding="utf-8", xml_declaration=True) + b"\n"
        )
        print(f"junit: wrote {args.out}")
        return 0

    if args.cmd == "counts":
        log = Path(args.cargo_log).read_text(encoding="utf-8", errors="replace")
        gate = json.loads(Path(args.gate_json).read_text(encoding="utf-8"))
        parsed = counts_from_cargo_log(log)
        counts = {
            # The declared set comes from `cargo metadata` (the execution
            # gate), never from the log: a log can only report what ran.
            "declared": int(gate.get("declared_count", 0)),
            "workspace_declared": int(gate.get("workspace_declared_count", 0)),
            "executed": int(gate.get("executed_count", 0)),
            "executed_binaries": parsed["executed_binaries"],
            "passed": parsed["passed"],
            "failed": parsed["failed"],
            "ignored": parsed["ignored"],
            "extra_binaries": int(gate.get("extra_binaries_count", 0)),
            "missing": int(gate.get("missing_count", 0)),
            "scope": gate.get("scope"),
            "gate_status": gate.get("status"),
        }
        Path(args.out).write_text(
            json.dumps(counts, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        print(json.dumps(counts, indent=2, sort_keys=True))
        return 0

    # verdict
    counts: dict[str, Any] = json.loads(Path(args.counts).read_text(encoding="utf-8"))
    coverage = None
    if args.coverage:
        coverage = json.loads(Path(args.coverage).read_text(encoding="utf-8"))
    declared = counts.get("declared")
    result = decide(
        counts,
        coverage,
        args.min_executed,
        args.min_coverage_percent,
        args.mode,
        declared=declared,
    )
    if args.manifest:
        manifest = build_manifest(
            args.commit,
            args.run_id,
            [Path(p) for p in args.artifact],
            mode=args.mode,
        )
        Path(args.manifest).write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        result["manifest"] = manifest
    payload = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.out:
        Path(args.out).write_text(payload, encoding="utf-8")
    print(payload, end="")
    verdict = result["verdict"]
    print(f"VERDICT: {verdict}", file=sys.stderr)
    return {"PASS": 0, "FAIL": 1, "INCONCLUSIVE": 2}[verdict]


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
