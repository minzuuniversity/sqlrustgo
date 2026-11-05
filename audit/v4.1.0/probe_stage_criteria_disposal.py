#!/usr/bin/env python3
"""probe_stage_criteria_disposal.py — 看板 P1-1 的取证探针（不是门禁）。

它做两件门禁 selftest 做不了的事：
  1. check                —— 对**真实** SSOT 跑 C9，断言 0 条 C9 判红，
     并断言 21 条 exit_criteria 恰好被 7 条 criteria_gates + 14 条 manual_only_criteria 覆盖；
  2. mutation-selftest    —— 在真实配置的临时副本上重新制造缺陷，断言门禁确实变红。
     门禁自带的 --selftest 用的是合成样本，这里证明的是「这个仓现在的这份文件」也能被证伪。

判据来自 scripts/gate/check_stage_config_consistency.sh 的规则 C9。
本文件不新增门禁、不改门禁清单与阈值（分母仍 168）。
"""
from __future__ import annotations

import copy
import os
import subprocess
import sys
import tempfile

import yaml

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CFG = os.path.join(REPO, "docs", "governance", "STAGE_CONFIG.yaml")
GATE = os.path.join(REPO, "scripts", "gate", "check_stage_config_consistency.sh")

EXPECT_CRIT = 21
EXPECT_GATES = 7
EXPECT_MANUAL = 14


def run_gate(cfg_path: str) -> tuple[int, list[str]]:
    proc = subprocess.run(
        ["bash", GATE, "--config", cfg_path, "--version", "v4.1.0"],
        capture_output=True, text=True,
    )
    c9 = [ln for ln in proc.stdout.splitlines() if ln.startswith("FAIL\tC9\t")]
    return proc.returncode, c9


def counts(doc) -> tuple[int, int, int]:
    crit = g = m = 0
    for info in (doc.get("stages") or {}).values():
        info = info or {}
        crit += len(info.get("exit_criteria") or [])
        g += len(info.get("criteria_gates") or {})
        m += len(info.get("manual_only_criteria") or {})
    return crit, g, m


def cmd_check(_argv: list[str]) -> int:
    doc = yaml.safe_load(open(CFG, encoding="utf-8"))
    crit, gates, manual = counts(doc)
    rc, c9 = run_gate(CFG)
    ok = rc == 0 and not c9 and crit == EXPECT_CRIT and gates == EXPECT_GATES and manual == EXPECT_MANUAL
    print("crit=%d gates=%d manual=%d gate_rc=%d c9_findings=%d -> %s"
          % (crit, gates, manual, rc, len(c9), "OK" if ok else "NOT-OK"))
    for line in c9:
        print("  " + line.replace("\t", " | "))
    if crit != gates + manual:
        print("  每条判据必须恰好被处置一次：gates+manual=%d vs crit=%d" % (gates + manual, crit))
    return 0 if ok else 1


def _tmp_with(mut) -> str:
    doc = yaml.safe_load(open(CFG, encoding="utf-8"))
    mut(doc)
    fd, path = tempfile.mkstemp(prefix="stagecfg_mut_", suffix=".yaml")
    with os.fdopen(fd, "w", encoding="utf-8") as fh:
        yaml.safe_dump(doc, fh, allow_unicode=True, sort_keys=False)
    return path


def _strip_all(doc):
    for info in (doc.get("stages") or {}).values():
        info.pop("criteria_gates", None)
        info.pop("manual_only_criteria", None)


def _strip_one(doc):
    # 只摘掉 ALPHA 的覆盖率映射：该判据变成无人判定。
    g = doc["stages"]["ALPHA"]["criteria_gates"]
    for key in list(g):
        if "COVERAGE_MIN_ALPHA" in key:
            del g[key]
            return
    raise SystemExit("mutation setup failed: no ALPHA coverage mapping")


def _empty_reason(doc):
    mo = doc["stages"]["RC"]["manual_only_criteria"]
    for key in mo:
        if key.startswith("Tag v"):
            mo[key] = "   "
            return
    raise SystemExit("mutation setup failed: no RC tag criterion")


def _dead_key(doc):
    g = doc["stages"]["BETA"]["criteria_gates"]
    for key in list(g):
        if "COVERAGE_MIN_BETA" in key:
            g[key + " (typo)"] = g.pop(key)
            return
    raise SystemExit("mutation setup failed: no BETA coverage mapping")


def _double_disposal(doc):
    g = doc["stages"]["GA"]["criteria_gates"]
    mo = doc["stages"]["GA"]["manual_only_criteria"]
    key = "GA tag cut: v{VER}"
    mo[key] = "同一条判据被处置两次"
    g[key] = ["scripts/gate/check_stage.sh"]


def _no_executor_gate(doc):
    g = doc["stages"]["ALPHA"]["criteria_gates"]
    for key in list(g):
        if "COVERAGE_MIN_ALPHA" in key:
            g[key] = ["scripts/gate/check_beta_gate.sh"]
            return
    raise SystemExit("mutation setup failed: no ALPHA coverage mapping")


ARMS = [
    ("M1-strip-all-mappings", _strip_all, EXPECT_CRIT),
    ("M2-strip-one-mapping", _strip_one, 1),
    ("M3-empty-manual-reason", _empty_reason, 1),
    ("M4-dead-mapping-key", _dead_key, 2),          # 死键 + 该判据无人判定
    ("M5-double-disposal", _double_disposal, 1),
    ("M6-gate-without-execution-point", _no_executor_gate, 1),
]


def cmd_mutation_selftest(_argv: list[str]) -> int:
    ok = bad = 0
    for name, mut, want_min in ARMS:
        path = _tmp_with(mut)
        try:
            rc, c9 = run_gate(path)
        finally:
            os.unlink(path)
        good = rc == 1 and len(c9) >= want_min
        if good:
            ok += 1
        else:
            bad += 1
        print("ARM %-34s rc=%d c9=%d want_rc1 want_min=%d -> %s"
              % (name, rc, len(c9), want_min, "OK" if good else "NOT-OK"))
    print("stage-criteria mutation-selftest: ok=%d bad=%d" % (ok, bad))
    return 0 if bad == 0 and ok == len(ARMS) else 1


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__)
        return 2
    sub, rest = argv[0], argv[1:]
    if sub == "check":
        return cmd_check(rest)
    if sub == "mutation-selftest":
        return cmd_mutation_selftest(rest)
    print("usage: probe_stage_criteria_disposal.py {check|mutation-selftest}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
