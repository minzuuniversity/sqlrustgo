#!/usr/bin/env python3
"""gate_executability 的**反向夹具**（P0-7：懒初始化全局的 fail-open 陷阱）。

这不是门禁（名字不是 `check_*`，也不挂在 STAGE_CONFIG 的阶段清单里）。
它核对的是那条**调用方契约**：矩阵全局未构建时必须是「未构建」，
读取必须报错 —— 因为空集会同时制造两种假象：

* 「清单里每道门禁都有执行点」在**空清单上真空通过**（0 违例 = 通过）；
* 「孤儿门禁 N 道」变成无意义的数字。

夹具臂：

    N1/N2/N3  未 build_matrix() 时三个入口都必须抛 MatrixNotBuilt
    N4        build 之后访问器必须给出非空清单（否则"报错"只是永远红）
    N5        两个既有调用方必须在读访问器**之前**调用 build_matrix()
    N6        把守卫中和后，`--selftest` 必须变红（守卫是承重的，不是装饰）
"""

from __future__ import annotations

import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import gate_executability as ge  # noqa: E402


def n1_n2_n3() -> list[str]:
    fails: list[str] = []
    for label, call in (
        ("N1 required_script_paths()", ge.required_script_paths),
        ("N2 required_scripts_by_stage()", ge.required_scripts_by_stage),
        (
            "N3 reachable_from_roots()",
            lambda: ge.reachable_from_roots(
                {},
                [{"run": "bash scripts/gate/check_stage.sh --stage GA",
                  "workflow": "wf", "job": "job", "name": "step"}],
                {},
                "GA",
            ),
        ),
    ):
        try:
            call()
        except ge.MatrixNotBuilt:
            continue
        except BaseException as exc:  # 报错但报错了别的东西 ⇒ 契约仍然没守住
            fails.append(f"{label}: 未构建时抛的是 {type(exc).__name__}，不是 MatrixNotBuilt")
            continue
        fails.append(f"{label}: 未构建时没有抛 MatrixNotBuilt（fail-open）")
    return fails


def n4() -> list[str]:
    ge.build_matrix()
    paths = ge.required_script_paths()
    by_stage = ge.required_scripts_by_stage()
    if not paths or not by_stage:
        return [f"N4 build_matrix() 之后访问器仍为空: paths={len(paths)} stages={len(by_stage)}"]
    return []


def n5() -> list[str]:
    """两个既有调用方必须「先 build 再读」。

    判两条，一条运行时一条静态，互为反查：

    * **运行时**：跑 `gov_probe.py metrics` 与 `gate_registry.py check`，
      要求 exit 0 且 `listed_n` / `catalog_rows` 非零。若谁漏掉 build_matrix()，
      矩阵未构建 ⇒ 访问器抛 MatrixNotBuilt ⇒ 子命令直接失败（不再是空集假绿）。
    * **静态近邻**：每处读访问器**上方 12 行内**必须有 `build_matrix(`。
      （早先用「整个函数体内是否出现」判定，cmd_metrics 是几百行的大函数，
      另一段无关的 build_matrix() 会让检查假绿 ⇒ 改成按行近邻。）
    """
    fails: list[str] = []

    for args, key in ((["gov_probe.py", "metrics"], "listed_n"),
                      (["gate_registry.py", "check"], "catalog_rows")):
        r = subprocess.run([sys.executable, str(HERE / args[0])] + args[1:],
                           capture_output=True, text=True, timeout=300)
        if r.returncode != 0:
            fails.append(f"N5 {' '.join(args)} rc={r.returncode}: {r.stderr.strip()[:160]}")
            continue
        head = r.stdout.strip().splitlines()[0] if r.stdout.strip() else ""
        if f'"{key}"' not in head:
            fails.append(f"N5 {' '.join(args)} 输出里没有 {key}，夹具与实现脱钩")
            continue
        if f'"{key}": 0' in head:
            fails.append(f"N5 {' '.join(args)} 的 {key}=0（空清单假绿的典型形状）")

    for name in ("gate_registry.py", "gov_probe.py"):
        lines = (HERE / name).read_text(encoding="utf-8").splitlines()
        body = "\n".join(lines)
        if "required_script_paths(" not in body and "required_scripts_by_stage(" not in body:
            fails.append(f"N5 {name} 不再经访问器读矩阵（改回直读全局 ⇒ P0-7 的洞回来了）")
        for i, line in enumerate(lines):
            if "required_script_paths" not in line and "required_scripts_by_stage" not in line:
                continue
            window = lines[max(0, i - 12):i]
            if not any("build_matrix(" in w for w in window):
                fails.append(f"N5 {name}:{i + 1} 读访问器上方 12 行内没有 build_matrix()")
    return fails


def n6() -> list[str]:
    src = pathlib.Path(HERE, "gate_executability.py").read_text(encoding="utf-8")
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="ge_mut_"))
    try:
        (tmp / "scripts" / "gate").mkdir(parents=True)
        old = "    if ALL_REQUIRED_SCRIPT_PATHS is None or ALL_REQUIRED_SCRIPTS_BY_STAGE is None:"
        if src.count(old) != 1:
            return [f"N6 找不到守卫判据（count={src.count(old)}），夹具已与实现脱钩"]
        (tmp / "scripts" / "gate" / "gate_executability.py").write_text(
            src.replace(old, "    if False:"), encoding="utf-8"
        )
        r = subprocess.run(
            [sys.executable, "scripts/gate/gate_executability.py", "--selftest"],
            cwd=tmp, capture_output=True, text=True, timeout=120,
        )
        if r.returncode == 0:
            return ["N6 守卫被中和后 --selftest 仍然通过（夹具不承重）"]
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return []


def main() -> int:
    fails: list[str] = []
    for fn in (n1_n2_n3, n4, n5, n6):
        fails += fn()
    if fails:
        for f in fails:
            print("FIXTURE FAIL:", f)
        return 1
    print("test_gate_executability: OK (N1..N6 all held)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
