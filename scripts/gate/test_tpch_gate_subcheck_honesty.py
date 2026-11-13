#!/usr/bin/env python3
"""tpch_gate_subcheck.sh 的诚实性夹具（七个门禁共用实现的三态判据）。

WHY（2026-11-13，见 docs/releases/v4.1.0/ALPHA_GATES_RERUN_2026-11-13.md §8）
--------------------------------------------------------------------------
七道门禁内嵌同一段 `grep "test result.*ok"` 的 TPC-H 子检查；在没有 TPC-H 夹具的
宿主上，tpch_gate_test 自报 TPCH-NOT-RUN 后仍以 `test result: ok` 返回，
子检查于是被判 PASS，并且文案声称 "TPC-H gate (22/22) maintained"。
本夹具钉两条：
  A. 共用子检查在缺夹具时必须判 INCOMPLETE（rc=2），不许 PASS、不许说 mismatch；
  B. 调用方（以 check_p35_simd.sh 为端到端代表）不得再报
     "TPC-H gate (22/22) maintained" 的 PASS，必须出现 INCOMPLETE。

用法：python3 scripts/gate/test_tpch_gate_subcheck_honesty.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
SUBCHECK = HERE / "tpch_gate_subcheck.sh"
CONSUMER = HERE / "check_p35_simd.sh"

FAILURES: list[str] = []


def check(name: str, cond: bool, detail: str = "") -> None:
    if cond:
        print(f"  [OK]   {name}")
    else:
        print(f"  [FAIL] {name} {detail}")
        FAILURES.append(name)


def run(cmd, timeout=1800):
    env = dict(os.environ)
    env["TPCH_DATA_DIR"] = "/nonexistent-tpch-fixture-for-test"
    return subprocess.run(cmd, cwd=str(REPO_ROOT), env=env,
                          capture_output=True, text=True, timeout=timeout)


def main() -> int:
    print("A. 共用子检查（无夹具 ⇒ INCOMPLETE）")
    proc = run(["bash", str(SUBCHECK)])
    out = proc.stdout + proc.stderr
    check("退出码 = 2（不是 0 也不是 1）", proc.returncode == 2, f"got {proc.returncode}")
    check("判 INCOMPLETE", "INCOMPLETE" in out)
    check("reason=fixture_missing", "fixture_missing" in out)
    check("没有伪造的 PASS 行", "TPCH-SUBCHECK: PASS" not in out)

    print("B. 调用方端到端（check_p35_simd.sh 代表）")
    proc = run(["bash", str(CONSUMER)])
    out = proc.stdout + proc.stderr
    check("退出码 = 2（INCOMPLETE）", proc.returncode == 2, f"got {proc.returncode}")
    check("输出含 INCOMPLETE", "INCOMPLETE" in out)
    check("不再声称 TPC-H (22/22) maintained 的 PASS",
          "PASS: TPC-H gate (22/22) maintained" not in out)

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: tpch_gate_subcheck 三态夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
