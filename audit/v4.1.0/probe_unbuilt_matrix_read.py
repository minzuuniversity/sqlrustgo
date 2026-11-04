#!/usr/bin/env python3
"""P0-7 的**进程外**探针：不 build_matrix() 就读取执行点矩阵必须被拒。

放在 audit/v4.1.0/（run 级校验器，不是 check_*，不进阶段清单）。
它刻意**另起进程**，因为在同一进程里 `import` 顺序会掩盖问题：
真正的生产事故是"调用方忘记 build_matrix() 却拿到空清单"，
只有干净进程才能复现。

exit 0 = 读取被拒（且报的是 MatrixNotBuilt）；
exit 1 = 读取成功（fail-open 回来了）或报错类型不对。
"""
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
CHILD = "import sys; sys.path.insert(0, 'scripts/gate'); import gate_executability as ge; ge.required_script_paths()"

p = subprocess.run([sys.executable, "-c", CHILD], cwd=REPO, capture_output=True, text=True, timeout=120)
last = (p.stderr.strip().splitlines() or [""])[-1]
if p.returncode == 0:
    print("PROBE FAIL: 未构建时读取矩阵成功返回（fail-open）")
    sys.exit(1)
if "MatrixNotBuilt" not in p.stderr:
    print(f"PROBE FAIL: 退出码 {p.returncode}，但报的不是 MatrixNotBuilt：{last[:160]}")
    sys.exit(1)
print(f"PROBE OK: 未构建即被拒 rc={p.returncode} :: {last[:120]}")
