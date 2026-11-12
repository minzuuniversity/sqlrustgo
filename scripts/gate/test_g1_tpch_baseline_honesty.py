#!/usr/bin/env python3
"""G1 门禁的诚实性夹具（诚实三态：PASS / FAIL / NOT-RUN）。

WHY（2026-11-13 实测，见 docs/releases/v4.1.0/ALPHA_GATES_RERUN_2026-11-13.md §6）
--------------------------------------------------------------------------------
`check_g1_tpch_baseline.sh` 在本机（无 TPC-H 夹具）上：
  2/4 tpch_full_22_test → "40 passed, finished in 0.03s"
  3/4 tpch_hash_test    → "50 passed, finished in 0.04s"   ← 一条查询都没跑
  4/4 tpch_hash_compare → 测试自报 TPCH-NOT-RUN: reason=fixture_missing，
      比较器却报 "cannot find hash in test output"，门禁再译成 "hash mismatch"

两类缺陷，本夹具各钉一条：
  A. NOT-RUN 必须被识别为 NOT-RUN（带 reason），不得退化成"找不到哈希"或"hash mismatch"。
  B. 门禁在夹具缺失时必须判 INCOMPLETE（退出码 2），既不许报 PASS，也不许报 FAIL。

用法：python3 scripts/gate/test_g1_tpch_baseline_honesty.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
COMPARE_PY = HERE / "tpch_hash_compare.py"
GATE_SH = HERE / "check_g1_tpch_baseline.sh"


def load_compare():
    spec = importlib.util.spec_from_file_location("tpch_hash_compare", COMPARE_PY)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


FAILURES: list[str] = []


def check(name: str, cond: bool, detail: str = "") -> None:
    if cond:
        print(f"  [OK]   {name}")
    else:
        print(f"  [FAIL] {name} {detail}")
        FAILURES.append(name)


def main() -> int:
    mod = load_compare()

    # ---- A. 比较器侧：NOT-RUN 必须与 PASS / mismatch 分开 ----
    print("A. tpch_hash_compare 三态分类")

    not_run = (
        b"running 50 tests\n"
        b"TPCH-NOT-RUN: tpch_hash_test reason=fixture_missing "
        b"detail=no TPC-H fixture found (set TPCH_DATA_DIR to a directory with the 8 .tbl files)\n"
        b"test result: ok. 50 passed; 0 failed\n"
    )
    check("classify 存在（缺失即未实现三态）", hasattr(mod, "classify"))
    if hasattr(mod, "classify"):
        verdict, payload = mod.classify(not_run)[:2]
        check("夹具缺失 ⇒ NOT_RUN（不是 PASS）", verdict == "NOT_RUN", f"got {verdict!r}")
        check("reason 被保留为 fixture_missing", payload == "fixture_missing", f"got {payload!r}")

        good = b"G1 PASS: TPC-H 22/22 baseline hash matches (" + b"a" * 64 + b")\n"
        verdict, payload = mod.classify(good)[:2]
        check("PASS 行 ⇒ OK 且取出 64 位哈希", verdict == "OK" and payload == "a" * 64,
              f"got {verdict!r}/{payload!r}")

        drift = (
            b"G1 FAIL: TPC-H hash mismatch\n  expected: " + b"a" * 64
            + b"\n  actual:   " + b"b" * 64 + b"\n"
        )
        verdict, payload = mod.classify(drift)[:2]
        check("真漂移 ⇒ MISMATCH（不得混成 NOT_RUN）", verdict == "MISMATCH", f"got {verdict!r}")

        silent = b"test result: ok. 1 passed\n"
        verdict, payload = mod.classify(silent)[:2]
        check("既无 PASS 也无 NOT-RUN ⇒ UNDISCLOSED（不得报 PASS）",
              verdict == "UNDISCLOSED", f"got {verdict!r}")

    # ---- B. 门禁脚本侧：夹具缺失必须判 INCOMPLETE ----
    print("B. check_g1_tpch_baseline.sh 夹具缺失时的三态")
    env = dict(os.environ)
    env["TPCH_DATA_DIR"] = "/nonexistent-tpch-fixture-for-test"
    proc = subprocess.run(
        ["bash", str(GATE_SH)],
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=1800,
    )
    out = proc.stdout + proc.stderr
    check("退出码 = 2（INCOMPLETE，不是 0 也不是 1）", proc.returncode == 2,
          f"got {proc.returncode}")
    check("输出含 INCOMPLETE", "INCOMPLETE" in out)
    check("输出不再把 NOT-RUN 说成 hash mismatch", "hash mismatch" not in out)
    check("输出点名原因 fixture", "fixture" in out.lower())

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: G1 三态夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
