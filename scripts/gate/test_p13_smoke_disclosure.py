#!/usr/bin/env python3
"""check_p13_soak_test.sh 的诚实披露判据（2026-11-13，看板 VACUOUS-GREEN-1 轻档发现）。

背景（实测）：soak_test 的底座 soak_test_harness.rs:89 run_soak_smoke 是**模拟查询环**
（源码注释原文 "// Simulated query loop"；时延 0.5+(i%3)*0.5 造出、内存每 100 次加 1KB
常量）⇒ 门禁的 11 个子检查验的是 harness 计分算术，不是任何一次真实 soak。
而门禁最后一行 `=== G7 Gate: PASS ===` + "Soak Test: ... verified" 容易被读者
当成"soak 有证据"。真正的 168h SOAK 属看板 ING-1（known_gaps.V400_09_soak，未闭合）。

判据（可证伪）：门禁输出必须自带两条披露——
  1. 声明 soak_test 是 simulated harness smoke（不是真实 soak）；
  2. 指明真实 soak 的归属（ING-1 / V400-09 / run_soak_with_heartbeat.sh）。

用法：python3 scripts/gate/test_p13_smoke_disclosure.py
退出码：0 = 通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
GATE = HERE / "check_p13_soak_test.sh"

FAILURES: list[str] = []


def check(name: str, cond: bool, detail: str = "") -> None:
    if cond:
        print(f"  [OK]   {name}")
    else:
        print(f"  [FAIL] {name} {detail}")
        FAILURES.append(name)


def main() -> int:
    proc = subprocess.run(
        ["bash", str(GATE)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=1800,
    )
    out = (proc.stdout + proc.stderr)
    low = out.lower()
    check("门禁退出码 = 0（本机现有状态）", proc.returncode == 0, f"got {proc.returncode}")
    check("输出含 simulated harness smoke 披露", "simulated harness smoke" in low)
    check("披露指明非真实 soak", ("not a real soak" in low) or ("does not constitute" in low and "evidence of a real soak" in low))
    check("披露指向真实 soak 的归属（ING-1/V400-09）",
          ("ING-1" in out) or ("V400-09" in out),
          "未找到 ING-1/V400-09 指引")
    check("披露给出长跑入口（run_soak_with_heartbeat.sh）", "run_soak_with_heartbeat.sh" in out)

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: p13 披露判据全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
