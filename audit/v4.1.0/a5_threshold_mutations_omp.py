#!/usr/bin/env python3.14
"""a5_coverage_decision 门槛判据的反向变异探针（#3844 × #5581）。

被检验的判据
------------
`scripts/gate/check_rc_ga_gate.sh` 里 `a5_coverage_decision` 的门槛来源：
调用方显式给定 → 其次 `COVERAGE_MIN`（`main()` 从 SSOT 读到）→ 最后现读
`docs/governance/STAGE_CONFIG.yaml`。三条路都给不出门槛 ⇒ 判红并写明
`NO_THRESHOLD_IN_SSOT`。

改前漏什么
----------
CI 血缘上 `COVERAGE_MIN=75` 是**顶层常量**，source 之后永远有值：
GA 门槛实际是 85，于是 84% 会被判 PASS ——「门槛从哪来」在该线上不可判定，
任何低于 GA 门槛的覆盖率只要 ≥75 就静默通过。

改后多报什么
------------
恰好等于 SSOT 门槛判 PASS，低 1 个百分点判 FAIL（`AT_MIN=PASS` /
`BELOW_MIN=FAIL` 这对断言就是差 1 个百分点的分界）。

探针双臂
--------
1. CONTROL：未变异 ⇒ `test_5581_a5_threshold_comes_from_ssot_when_caller_gives_none` 必须绿。
   （对照不绿说明用例本身坏了，探针没有意义。）
2. MUTANT：把 SSOT 兜底换成 CI 线的常量 `threshold=75` ⇒ 该用例必须变红，
   且失败签名必须是 `BELOW_MIN=PASS`（即「低于门槛却被判过」这个具体缺陷），
   不是编译错误之类的无关失败。

锚点必须命中且唯一，恢复必须核对；任一环节不成立 ⇒ 非零退出并说明原因，
绝不"跑完了就算过"。

命名：本文件不是 `check_*`，也不进 `STAGE_CONFIG.yaml` ⇒ 门禁分母仍是 168。
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
TARGET = REPO / "scripts" / "gate" / "check_rc_ga_gate.sh"
TEST_FILTER = "test_5581_a5_threshold_comes_from_ssot_when_caller_gives_none"

# 变异锚点：SSOT 兜底那一行（唯一）。
ANCHOR = (
    'threshold="$(coverage_min_for_mode "${GATE_TYPE:-all}" 2>/dev/null '
    "| awk '{print $2}')\""
)
MUTANT = "threshold=75  # MUTATION-PROBE-5581（刻意退回 CI 线的顶层常量语义）"


def run_cargo() -> tuple[int, str]:
    p = subprocess.run(
        ["cargo", "test", "-q", "--test", "coverage_notrun_honesty_test", TEST_FILTER],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    return p.returncode, p.stdout + p.stderr


def main() -> int:
    if not TARGET.is_file():
        print(f"FAIL: 找不到被测脚本 {TARGET}")
        return 2

    original = TARGET.read_text(encoding="utf-8")
    hits = original.count(ANCHOR)
    if hits != 1:
        print(f"FAIL: 变异锚点命中 {hits} 次（必须恰好 1 次）——判据实现已漂移，探针需更新")
        return 2

    # ---- 1. CONTROL ----
    rc_ctl, out_ctl = run_cargo()
    if rc_ctl != 0:
        print("FAIL: CONTROL 不绿 —— 未变异时该用例就失败，探针失去判别力")
        print(out_ctl[-1500:])
        return 1
    print("CONTROL: VERIFIED（未变异 ⇒ 用例绿，门槛判定确有实现支撑）")

    # ---- 2. MUTANT ----
    TARGET.write_text(original.replace(ANCHOR, MUTANT, 1), encoding="utf-8")
    try:
        rc_mut, out_mut = run_cargo()
    finally:
        # 无论如何都要恢复工作树；恢复结果在下面核对（不在 finally 里 return，
        # 那是会被后续异常吞掉的写法）。
        subprocess.run(
            ["git", "checkout", "HEAD", "--", "scripts/gate/check_rc_ga_gate.sh"],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        restored = TARGET.read_text(encoding="utf-8")

    if restored != original:
        print("FAIL: 恢复后文件与变异前不一致 —— 工作树被污染，本次结论不可信")
        return 3
    if ANCHOR not in restored:
        print("FAIL: 恢复后 SSOT 兜底不在位")
        return 3

    if rc_mut == 0:
        print("FAIL: 变异未被抓住 —— 把门槛退回顶层常量 75 之后用例仍然绿")
        print(out_mut[-1500:])
        return 1
    if "BELOW_MIN=PASS" not in out_mut:
        print("FAIL: 变异确实让用例变红，但失败签名不是 BELOW_MIN=PASS")
        print("      ⇒ 这只是一次无关失败，不能证明该判据有判别力")
        print(out_mut[-1500:])
        return 1
    print(
        "MUTANT: VERIFIED（84% < GA 门槛 85%，但沿用常量 75 ⇒ 判 PASS，用例变红 rc="
        f"{rc_mut}）"
    )
    print("VERDICT: 2/2 VERIFIED —— 门槛来源判据有判别力，工作树已核对恢复")
    return 0


if __name__ == "__main__":
    sys.exit(main())
