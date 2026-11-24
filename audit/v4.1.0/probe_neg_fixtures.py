#!/usr/bin/env python3
"""probe_neg_fixtures.py — 看板 P1-3 的取证探针（不是门禁）。

它把「每道阶段门禁有没有反向夹具」从口头承诺变成可对账的台账：

  docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl
      每一行 = 一道在阶段清单里的门禁（scope）+ 它的夹具状态。

三个子命令：

  check            对账台账本身（结构 + 语义 + 证据落盘）。
  fixtures         在**临时 scratch 工作树**里注入缺陷、跑真实门禁，
                   看门禁的判定会不会真的变。fixture 的期望写在脚本头部，
                   取值是下面四种之一（每个 fx*.sh 都必须声明，否则报
                   UNDECLARED-EXPECT）：
                     red                   门禁必须变红（夹具退出码 0）
                     green-characterization 门禁必须仍然绿（夹具退出码 0）
                     selftest-red          致残门禁自带的检测器，由 selftests 泳道判
                     inconclusive          本树里这条性质无法确立（夹具退出码 4，
                                            且必须说清为什么；用来登记「门禁本来就红、
                                            注入的缺陷不可归因」这类情形）
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
            sab = r.get("selftest_neg_fixture", "")
            if not sab or not os.path.isfile(os.path.join(REPO, sab)):
                bad.append("NO-SELFTEST-SABOTAGE-FIXTURE: %s (a selftest nobody can break is not "
                           "evidence; cite the fixture that cripples the detector)" % g)
            elif expect_of(sab) != "selftest-red":
                bad.append("SELFTEST-SABOTAGE-EXPECT-MISMATCH: %s -> %r" % (g, expect_of(sab)))
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
    ok = failed = errored = deferred = 0
    for s in scripts:
        name = os.path.basename(s)
        # A `selftest-red` fixture cripples a gate's own detector; its verdict
        # is the NEUTERED_SELFCHECK_RC attestation, which only `selftests`
        # parses. Re-running it here would cripple the same scratch tree again
        # without adding evidence, so `fixtures` defers it explicitly instead
        # of silently ignoring it.
        if expect_of(os.path.relpath(s, REPO)) == "selftest-red":
            deferred += 1
            print("\n===== %s -> DEFERRED (run by the selftests subcommand) =====" % name)
            continue
        proc = subprocess.run(["bash", s, tree], capture_output=True, text=True)
        out = proc.stdout + proc.stderr
        print("\n===== %s (script rc=%d) =====" % (name, proc.returncode))
        print(out.rstrip())
        expect = expect_of(os.path.relpath(s, REPO))
        if expect == "red":
            want = 0
        elif expect == "green-characterization":
            want = 0
        elif expect == "inconclusive":
            # The fixture declares that its property CANNOT be established in
            # this tree -- e.g. the gate it targets is already red for an
            # unrelated reason, so the injected defect cannot be attributed to
            # the red.  The fixture must then reach exactly that conclusion
            # (exit 4 = INCONCLUSIVE) and say why.  This is a declared
            # expectation that the lane CHECKS, not a way to skip a fixture:
            # a fixture that exits 0 or 3 here is a failure, and a fixture
            # that does not declare its expectation at all is still an error.
            want = 4
        else:
            want = None
        if want is None:
            errored += 1
            print("  -> UNDECLARED-EXPECT")
        elif proc.returncode == want:
            ok += 1
        else:
            failed += 1
    print("\nneg-fixtures: pass=%d fail=%d error=%d deferred-selftest-red=%d total=%d"
          % (ok, failed, errored, deferred, len(scripts)))
    return 0 if failed == 0 and errored == 0 else 1


def cmd_selftests(argv: list[str]) -> int:
    """Behavioural verdict for every gate whose ledger row says selftest-verified.

    Two conditions, both required. Neither is a guess about output shape:

      (1) the gate's own `--selftest` exits 0 on a clean tree;
      (2) the row's detector-crippling fixture makes **that same** `--selftest`
          go red. The probe parses the `NEUTERED_SELFCHECK_RC=` attestation
          itself; it does not trust the fixture's exit code, because a fixture
          that cripples nothing would otherwise exit 0 and be believed.

    A "selftest exists" claim that survives (2) means the selftest is
    load-bearing. One that survives only (1) proves nothing (AGENTS 2.5).
    """
    tree = ledger = None
    for i, a in enumerate(argv):
        if a == "--tree" and i + 1 < len(argv):
            tree = argv[i + 1]
        if a == "--ledger" and i + 1 < len(argv):
            ledger = argv[i + 1]
    if not tree:
        print("usage: probe_neg_fixtures.py selftests --tree <worktree> [--ledger <jsonl>]")
        return 2
    rows = [r for r in load_jsonl(ledger or LEDGER) if r.get("kind") == "selftest-verified"]
    if not rows:
        print("no selftest-verified rows in the ledger")
        return 1
    ok = bad = 0
    for r in sorted(rows, key=lambda x: x["gate"]):
        gate = r["gate"]
        proc = subprocess.run(["bash", os.path.join(tree, gate), "--selftest"],
                              capture_output=True, text=True, cwd=tree)
        clean_rc = proc.returncode
        sab = r.get("selftest_neg_fixture", "")
        sab_path = sab if os.path.isabs(sab) else os.path.join(tree, sab)
        if not os.path.isfile(sab_path):
            good, detail = False, "clean_rc=%d neutered_selftest_rc=None fixture_rc=None (fixture missing)" % clean_rc
        else:
            sp = subprocess.run(["bash", sab_path, tree], capture_output=True, text=True)
            m = re.search(r"^NEUTERED_SELFCHECK_RC=(\d+)$", sp.stdout + sp.stderr, re.M)
            nrc = int(m.group(1)) if m else None
            good = clean_rc == 0 and sp.returncode == 0 and nrc not in (None, 0)
            detail = "clean_rc=%d neutered_selftest_rc=%s fixture_rc=%d" % (clean_rc, nrc, sp.returncode)
        ok += 1 if good else 0
        bad += 0 if good else 1
        print("%-58s %s -> %s" % (gate, detail, "OK" if good else "NOT-OK"))
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


def _selftest_without_sabotage(rows):
    for r in rows:
        if r.get("kind") == "selftest-verified":
            r.pop("selftest_neg_fixture", None)
            return
    raise SystemExit("mutation setup failed: no selftest-verified row")


def _fake_sabotage_fixture(rows):
    for r in rows:
        if r.get("kind") == "selftest-verified":
            r["selftest_neg_fixture"] = "audit/v4.1.0/neg/sab_does_not_exist.sh"
            return
    raise SystemExit("mutation setup failed: no selftest-verified row")


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
    ("M9-selftest-claim-without-breakable-fixture", _selftest_without_sabotage),
    ("M10-sabotage-fixture-file-does-not-exist", _fake_sabotage_fixture),
]


def cmd_mutation_selftest(_argv: list[str]) -> int:
    scope = scope_gates()

    # Baseline first: if the committed ledger already has findings, an arm that
    # "finds something" proves nothing. Requiring a clean baseline is what makes
    # every finding below attributable to the injected mutation.
    base = validate(load_jsonl(LEDGER), scope, CATALOG_GATES)
    if base:
        print("BASELINE-NOT-CLEAN: the committed ledger has %d findings; "
              "mutation arms would be unverifiable" % len(base))
        for b in base[:5]:
            print("  " + b)
        return 1
    print("baseline: ledger findings=0 (arms are attributable)")

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

    # BA1 is a behavioural arm: it attacks the *selftests* verdict, not the
    # ledger schema. An inert fixture (prints rc=0, cripples nothing) must
    # make the verdict NOT-OK, otherwise the verdict is believing the fixture.
    inert = _inert_sabotage_fixture()
    try:
        path = _tmp_ledger(lambda rows: [
            r.__setitem__("selftest_neg_fixture", inert)
            for r in rows if r.get("kind") == "selftest-verified"])
        rc = cmd_selftests(["--tree", REPO, "--ledger", path])
    finally:
        os.unlink(path)
        os.unlink(inert)
    good = rc != 0
    ok += 1 if good else 0
    bad += 0 if good else 1
    print("ARM %-38s selftests_rc=%d want!=0 -> %s" % ("BA1-inert-sabotage-fixture", rc,
                                                       "OK" if good else "NOT-OK"))
    total = len(ARMS) + 1
    print("mutation-selftest total: ok=%d bad=%d arms=%d" % (ok, bad, total))
    return 0 if bad == 0 and ok == total else 1


def _inert_sabotage_fixture() -> str:
    """A fixture that claims to cripple a detector but touches nothing."""
    body = ("#!/usr/bin/env bash\n"
            "# expect: selftest-red\n"
            'echo "NEUTERED_SELFCHECK_RC=0"\n'
            "exit 0\n")
    fd, path = tempfile.mkstemp(prefix="inert_sab_", suffix=".sh")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(body)
    os.chmod(path, 0o755)
    return path


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
