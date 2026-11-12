#!/usr/bin/env python3
"""check_stage.sh 的覆盖率臂必须显式声明被判定阶段（S-3 判据）。

WHY（2026-11-13，docs/releases/v4.1.0/ALPHA_GATES_RERUN_2026-11-13.md §2 S-3）
--------------------------------------------------------------------------
`check_coverage_v312.sh` 在 `SQLRUSTGO_COVERAGE_STAGE` 未声明时按缺省 `ga`（85%）判——
这是 #5581 的刻意设计：漏声明的调用方应该吃到最严档，而不是悄悄放宽。
但 `check_stage.sh` 是"知道自己正在判哪个阶段"的调用方（它解析出了 STAGE），
它不该吃缺省：ALPHA 阶段门禁跑出来却写着 `stage=ga threshold=85%`，
读者会把 GA 口径的判定当成 ALPHA 口径的结论。

判据（可证伪）：
  A. `check_stage.sh --dry-run --version v4.1.0` 的 dry-run 输出里，
     覆盖率臂必须带 `SQLRUSTGO_COVERAGE_STAGE=alpha`（阶段解析自 STAGE.yaml）。
  B. `--stage GA` 覆盖时，同一行必须变成 `SQLRUSTGO_COVERAGE_STAGE=ga`。
  C. 声明必须只挂在覆盖率臂上，不得污染其它臂的 dry-run 行。

用法：python3 scripts/gate/test_coverage_stage_declaration.py
退出码：0 = 通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
STAGE_SH = HERE / "check_stage.sh"

FAILURES: list[str] = []


def check(name: str, cond: bool, detail: str = "") -> None:
    if cond:
        print(f"  [OK]   {name}")
    else:
        print(f"  [FAIL] {name} {detail}")
        FAILURES.append(name)


def dry_run(*args: str) -> str:
    proc = subprocess.run(
        ["bash", str(STAGE_SH), "--dry-run", *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=300,
    )
    return proc.stdout + proc.stderr


def main() -> int:
    print("A. ALPHA（阶段解析自 STAGE.yaml）")
    out = dry_run("--version", "v4.1.0")
    cov_lines = [l for l in out.splitlines() if "check_coverage_v312.sh" in l and "DRY" in l]
    check("dry-run 里有覆盖率臂行", bool(cov_lines), repr(cov_lines[:1]))
    check("覆盖率臂行带 SQLRUSTGO_COVERAGE_STAGE=alpha",
          any("SQLRUSTGO_COVERAGE_STAGE=alpha" in l for l in cov_lines),
          repr(cov_lines[:1]))
    check("声明只挂在覆盖率臂（其它 DRY 行不带该前缀）",
          all("SQLRUSTGO_COVERAGE_STAGE" not in l or "check_coverage_v312.sh" in l
              for l in out.splitlines() if "DRY" in l))

    print("B. --stage GA 覆盖")
    out = dry_run("--version", "v4.1.0", "--stage", "GA")
    cov_lines = [l for l in out.splitlines() if "check_coverage_v312.sh" in l and "DRY" in l]
    check("覆盖率臂行带 SQLRUSTGO_COVERAGE_STAGE=ga",
          any("SQLRUSTGO_COVERAGE_STAGE=ga" in l for l in cov_lines),
          repr(cov_lines[:1]))

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: 覆盖率臂的阶段声明判据全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
