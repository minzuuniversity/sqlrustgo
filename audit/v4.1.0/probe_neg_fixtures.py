#!/usr/bin/env python3
"""probe_neg_fixtures.py — 看板 P1-3 的取证探针（不是门禁）。

它把「每道阶段门禁有没有反向夹具」从口头承诺变成可对账的台账：

  docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl
      每一行 = 一道在阶段清单里的门禁（scope）+ 它的夹具状态。

三个子命令：

  check            对账台账本身（结构 + 语义 + 证据落盘）。
  fixtures         在**临时 scratch 工作树**里注入缺陷、跑真实门禁，
                   看门禁的判定会不会真的变。fixture 的期望写在脚本头部。
  mutation-selftest 在台账的临时副本上制造缺陷，断言 check 会变红。

它不新增门禁、不改门禁清单/阈值/跳过开关（分母仍 168），
也不参与 STAGE_CONFIG 的阶段清单。
"""
from __future__ import annotations

import glob
import json
import os
import re
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CATALOG = os.path.join(REPO, "docs", "releases", "v4.1.0", "GATE_CATALOG.jsonl")
LEDGER = os.path.join(REPO, "docs", "releases", "v4.1.0", "NEG_FIXTURES_v4.1.0.jsonl")
NEGDIR = os.path.join(REPO, "audit", "v4.1.0", "neg")
RUNS = os.path.join(REPO, "audit", "v4.1.0", "runs")

KINDS = ("mutation-verified", "selftest-verified", "unfalsifiable", "pending-fixture")
NAME_RE = re.compile(r"SOAK|PERF|COMPAT|TPCH|STABILITY|CRASH|UPGRADE|CHAOS|QPS|SYSBENCH", re.I)
SELFTEST_RE = re.compile(r"--selftest|self_test|def selftest|run_selftest|selftest\(\)")
MIN_REASON = 40


def load_jsonl(path: str) -> list[dict]:
    out = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("//"):
                out.append(json.loads(line))
    return out


def scope_gates() -> list[str]:
    rows = [r for r in load_jsonl(CATALOG) if r.get("class") == "stage_required"
            or NAME_RE.search(r.get("gate", ""))]
    seen, out = set(), []
    for r in rows:
        g = r["gate"]
        if g not in seen:
            seen.add(g)
            out.append(g)
    return sorted(out)


def expect_of(script: str) -> str | None:
    """Read the '# expect:' declaration out of a fixture script."""
    path = os.path.join(REPO, script)
    if not os.path.isfile(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            m = re.match(r"^#\s*expect:\s*(\S+)", line)
            if m:
                return m.group(1)
            if not line.startswith("#") and line.strip():
                break
    return None


def evidence_ok(run_id: str) -> bool:
    return bool(run_id) and os.path.isdir(os.path.join(RUNS, run_id))


def validate(rows: list[dict], scope: list[str], catalog_gates: set[str]) -> list[str]:
    bad: list[str] = []
    by_gate: dict[str, list[dict]] = {}
    ids: set[str] = set()
    for r in rows:
        g = r.get("gate", "")
        by_gate.setdefault(g, []).append(r)
        fid = r.get("fixture_id") or ""
        if fid:
            if fid in ids:
                bad.append("DUPLICATE-FIXTURE-ID: %s" % fid)
            ids.add(fid)

    for g in scope:
        rs = by_gate.get(g, [])
        if not rs:
            bad.append("NO-LEDGER-ROW: %s (stage-required gate without a fixture row)" % g)
        elif len(rs) > 1:
            bad.append("DUPLICATE-ROW: %s has %d rows" % (g, len(rs)))
    scope_set = set(scope)
    for g, rs in by_gate.items():
        if g not in scope_set and g not in catalog_gates:
            bad.append("ROW-OUTSIDE-CATALOG: %s" % g)

    for r in rows:
        g = r.get("gate", "")
        kind = r.get("kind", "")
        if kind not in KINDS:
            bad.append("BAD-KIND: %s kind=%r" % (g, kind))
            continue
        if not os.path.exists(os.path.join(REPO, g)):
            bad.append("GATE-PATH-MISSING: %s" % g)
        if kind == "mutation-verified":
            script = r.get("fixture", "")
            exp = expect_of(script) if script else None
            if not script or not os.path.isfile(os.path.join(REPO, script)):
                bad.append("FIXTURE-FILE-MISSING: %s -> %r" % (g, script))
            elif exp != "red":
                bad.append("FIXTURE-EXPECT-MISMATCH: %s declares %s but row is mutation-verified"
                           % (g, exp))
            if not evidence_ok(r.get("evidence_run", "")):
                bad.append("NO-EVIDENCE-RUN: %s claims mutation-verified without a run" % g)
        elif kind == "selftest-verified":
            src = ""
            impl = r.get("selftest_impl") or g
            if os.path.exists(os.path.join(REPO, impl)):
                src = open(os.path.join(REPO, impl), encoding="utf-8", errors="replace").read()
            if not SELFTEST_RE.search(src):
                bad.append("NO-SELFTEST-IN-IMPL: %s (selftest_impl=%s)" % (g, impl))
            if not evidence_ok(r.get("evidence_run", "")):
                bad.append("NO-EVIDENCE-RUN: %s claims selftest-verified without a run" % g)
        elif kind == "unfalsifiable":
            reason = (r.get("reason") or "").strip()
            if len(reason) < MIN_REASON:
                bad.append("EMPTY-REASON: %s (unfalsifiable needs a >=%d char reason)"
                           % (g, MIN_REASON))
            script = r.get("falsification_attempt", "")
            exp = expect_of(script) if script else None
            if not script or not os.path.isfile(os.path.join(REPO, script)):
                bad.append("NO-FALSIFICATION-ATTEMPT: %s" % g)
            elif exp != "green-characterization":
                bad.append("FALSIFICATION-EXPECT-MISMATCH: %s -> %r" % (g, exp))
            if not evidence_ok(r.get("evidence_run", "")):
                bad.append("NO-EVIDENCE-RUN: %s claims unfalsifiable without a run" % g)
        hole = r.get("known_hole")
        if hole:
            hexp = expect_of(hole)
            if not os.path.isfile(os.path.join(REPO, hole)):
                bad.append("KNOWN-HOLE-FIXTURE-MISSING: %s -> %s" % (g, hole))
            elif hexp != "green-characterization":
                bad.append("KNOWN-HOLE-EXPECT-MISMATCH: %s -> %r" % (g, hexp))
            elif not evidence_ok(r.get("evidence_run", "")):
                bad.append("KNOWN-HOLE-NO-EVIDENCE: %s" % g)
        if kind == "pending-fixture":
            if not (r.get("target") or "").strip():
                bad.append("NO-TARGET: %s (pending-fixture must say what the fixture will do)" % g)
            if not (r.get("close_run") or "").strip():
                bad.append("NO-CLOSE-PLAN: %s (pending-fixture must name the closing run/issue)" % g)
            if r.get("evidence_run"):
                bad.append("EVIDENCE-ON-PENDING: %s must not cite evidence for a fixture it has no" % g)
    return bad


def cmd_check(argv: list[str]) -> int:
    ledger = argv[0] if argv else LEDGER
    scope = scope_gates()
    rows = load_jsonl(ledger)
    bad = validate(rows, scope, CATALOG_GATES)
    kinds = {}
    for r in rows:
        kinds[r.get("kind", "?")] = kinds.get(r.get("kind", "?"), 0) + 1
    verified = kinds.get("mutation-verified", 0) + kinds.get("selftest-verified", 0)
    print("scope=%d rows=%d kinds=%s falsifiable_now=%d/%d -> %s"
          % (len(scope), len(rows),
             ",".join("%s=%d" % (k, n) for k, n in sorted(kinds.items())),
             verified, len(scope), "OK" if not bad else "NOT-OK"))
    for b in bad:
        print("  " + b)
    return 0 if not bad else 1


# --- fixtures ----------------------------------------------------------------

def cmd_fixtures(argv: list[str]) -> int:
    tree = None
    for i, a in enumerate(argv):
        if a == "--tree" and i + 1 < len(argv):
            tree = argv[i + 1]
    if not tree or not os.path.isdir(os.path.join(tree, ".git"[:-1] or ".")) and not os.path.isdir(tree):
        print("usage: probe_neg_fixtures.py fixtures --tree <scratch-worktree>")
        return 2
    scripts = sorted(glob.glob(os.path.join(NEGDIR, "fx*.sh")))
    if not scripts:
        print("no fixture scripts under audit/v4.1.0/neg/")
        return 1
    ok = failed = errored = 0
    for s in scripts:
        name = os.path.basename(s)
        proc = subprocess.run(["bash", s, tree], capture_output=True, text=True)
        out = proc.stdout + proc.stderr
        print("\n===== %s (script rc=%d) =====" % (name, proc.returncode))
        print(out.rstrip())
        expect = expect_of(os.path.relpath(s, REPO))
        if expect == "red":
            want = 0
        elif expect == "green-characterization":
            want = 0
        else:
            want = None
        if want is None:
            errored += 1
            print("  -> UNDECLARED-EXPECT")
        elif proc.returncode == want:
            ok += 1
        else:
            failed += 1
    print("\nneg-fixtures: pass=%d fail=%d error=%d total=%d" % (ok, failed, errored, len(scripts)))
    return 0 if failed == 0 and errored == 0 else 1


def cmd_selftests(argv: list[str]) -> int:
    """Run the embedded --selftest of every gate whose ledger row says selftest-verified.

    The tree argument is a scratch worktree: some selftests touch files, so they
    are never run against the branch checkout.
    """
    tree = None
    for i, a in enumerate(argv):
        if a == "--tree" and i + 1 < len(argv):
            tree = argv[i + 1]
    if not tree:
        print("usage: probe_neg_fixtures.py selftests --tree <scratch-worktree>")
        return 2
    rows = [r for r in load_jsonl(LEDGER) if r.get("kind") == "selftest-verified"]
    if not rows:
        print("no selftest-verified rows in the ledger")
        return 1
    ok = bad = 0
    for r in sorted(rows, key=lambda x: x["gate"]):
        gate = r["gate"]
        proc = subprocess.run(["bash", os.path.join(tree, gate), "--selftest"],
                              capture_output=True, text=True, cwd=tree)
        out = proc.stdout + proc.stderr
        arms = len([ln for ln in out.splitlines()
                    if re.search(r"selftest|fixture|self-test", ln, re.I)])
        neg = len([ln for ln in out.splitlines() if re.search(r"reject|not detected|expected", ln, re.I)])
        good = proc.returncode == 0 and arms >= 3
        ok += 1 if good else 0
        bad += 0 if good else 1
        print("%-58s rc=%d selftest_lines=%d neg_assert_lines=%d -> %s"
              % (gate, proc.returncode, arms, neg, "OK" if good else "NOT-OK"))
        if not good:
            print(out.rstrip()[:1200])
    print("gate selftests: ok=%d bad=%d" % (ok, bad))
    return 0 if bad == 0 else 1


# --- mutation selftest -------------------------------------------------------

def _tmp_ledger(mutate) -> str:
    rows = load_jsonl(LEDGER)
    mutate(rows)
    fd, path = tempfile.mkstemp(prefix="neg_ledger_", suffix=".jsonl")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n")
    return path


def _drop_row(rows):
    rows[:] = [r for r in rows if r.get("kind") != "mutation-verified"][:max(1, len(rows) - 1)]


def _dup_row(rows):
    rows.append(dict(rows[0]))


def _empty_reason(rows):
    for r in rows:
        if r.get("kind") == "unfalsifiable":
            r["reason"] = "nope"
            return
    raise SystemExit("mutation setup failed: no unfalsifiable row")


def _fake_fixture(rows):
    for r in rows:
        if r.get("kind") == "mutation-verified":
            r["fixture"] = "audit/v4.1.0/neg/fx_does_not_exist.sh"
            return
    raise SystemExit("mutation setup failed: no mutation-verified row")


def _fake_selftest(rows):
    for r in rows:
        if r.get("kind") == "selftest-verified":
            r["selftest_impl"] = "audit/v4.1.0/neg/fx07-docs-links-broken.sh"
            return
    raise SystemExit("mutation setup failed: no selftest-verified row")


def _fake_evidence(rows):
    for r in rows:
        if r.get("kind") == "mutation-verified":
            r["evidence_run"] = "20991231-not-a-real-run"
            return
    raise SystemExit("mutation setup failed: no mutation-verified row")


def _pending_without_close(rows):
    for r in rows:
        if r.get("kind") == "pending-fixture":
            r.pop("close_run", None)
            return
    raise SystemExit("mutation setup failed: no pending-fixture row")


def _unfalsifiable_as_mutation(rows):
    for r in rows:
        if r.get("kind") == "unfalsifiable":
            r["kind"] = "mutation-verified"
            r["fixture"] = r.pop("falsification_attempt", "")
            return
    raise SystemExit("mutation setup failed: no unfalsifiable row")


CATALOG_GATES = {r["gate"] for r in load_jsonl(CATALOG)}

ARMS = [
    ("M1-drop-a-gate-row", _drop_row),
    ("M2-duplicate-row", _dup_row),
    ("M3-unfalsifiable-empty-reason", _empty_reason),
    ("M4-fixture-file-does-not-exist", _fake_fixture),
    ("M5-selftest-claim-without-selftest", _fake_selftest),
    ("M6-fake-evidence-run", _fake_evidence),
    ("M7-pending-without-close-plan", _pending_without_close),
    ("M8-unfalsifiable-relabeled-mutation", _unfalsifiable_as_mutation),
]


def cmd_mutation_selftest(_argv: list[str]) -> int:
    scope = scope_gates()
    ok = bad = 0
    for name, mut in ARMS:
        path = _tmp_ledger(mut)
        try:
            findings = validate(load_jsonl(path), scope, CATALOG_GATES)
        finally:
            os.unlink(path)
        good = len(findings) >= 1
        ok += 1 if good else 0
        bad += 0 if good else 1
        print("ARM %-38s findings=%d want>=1 -> %s" % (name, len(findings), "OK" if good else "NOT-OK"))
    print("neg-fixture ledger mutation-selftest: ok=%d bad=%d" % (ok, bad))
    return 0 if bad == 0 and ok == len(ARMS) else 1


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__)
        return 2
    sub, rest = argv[0], argv[1:]
    if sub == "check":
        return cmd_check(rest)
    if sub == "fixtures":
        return cmd_fixtures(rest)
    if sub == "selftests":
        return cmd_selftests(rest)
    if sub == "mutation-selftest":
        return cmd_mutation_selftest(rest)
    print("usage: probe_neg_fixtures.py {check|fixtures|selftests|mutation-selftest}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
