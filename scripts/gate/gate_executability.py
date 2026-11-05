#!/usr/bin/env python3
"""gate_executability.py —— 门禁**执行点矩阵**（一份实现）。

#5461 / #5450 —— 「GA 清单里的门禁到底有没有人跑」这件事，此前只能靠人读
workflow 文件去数；本模块把它变成**可测量、可 fail-closed** 的判据。

## 问题（实测，2026-10-24）

`docs/governance/STAGE_CONFIG.yaml` 的 `stages.<STAGE>.required_gates` 是
**声明式**清单。GA 一共 **31 项**（其中 **27 项**是脚本门禁），但：

* CI 里唯一调用阶段驱动的地方是

      bash scripts/gate/check_stage.sh --version v4.1.0 --dry-run

  —— **`--dry-run` 只列出、不执行**（`check_stage.sh:403` 的 `[DRY]` 分支）；
* 于是清单里的门禁**大多在 CI 里一次都没跑过**。

实测（改动前）：GA 的 27 个脚本门禁里 **22 个**在
`.gitea/workflows/*.yml` 的任何 step 里**都没有出现**。

## 判定模型

一个门禁 G「有执行点」当且仅当：

1. **文件存在** —— `required_gates` 里写的路径（`{VER}` 按仓库既有的
   拼写变体解析）能落到一个真实文件；**且**
2. **可达** —— 从 CI 里**真正被调用**的脚本出发，沿「门禁调用门禁」的
   边做**传递闭包**，G 在闭包里。

「真正被调用」的根节点 = `.gitea/workflows/*.yml` 的 `jobs.*.steps[].run`
里出现的 `scripts/gate/*.sh` / `*.py`（用 YAML 解析 ⇒ **注释天然被排除**）。

**`--dry-run` 的例外**：调用 `check_stage.sh --dry-run` 的 step **不算**
阶段驱动清单的执行点 —— 它逐条打印 `[DRY] <cmd>` 后 `continue`，
门禁一个都没跑。这正是本次要堵的洞。

## 用法

    python3 scripts/gate/gate_executability.py            # 打印矩阵 + 退出码
    python3 scripts/gate/gate_executability.py --json     # 机器可读
    python3 scripts/gate/gate_executability.py --selftest # 变异自测（A/B/C 三臂）

退出码：0 = 全部有执行点；1 = 有缺失/不可达的门禁。

**调用方契约（P0-7）**：矩阵全局未构建时是 `None`，读取必须经
`required_script_paths()` / `required_scripts_by_stage()`，未 build 即抛
`MatrixNotBuilt`。`--selftest` 的 E 臂就是这条契约的反向夹具。
"""

from __future__ import annotations

import json
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
STAGE_CONFIG = os.path.join(REPO, "docs/governance/STAGE_CONFIG.yaml")
WORKFLOW_DIR = os.path.join(REPO, ".gitea/workflows")

# `scripts/gate/xxx.sh` —— 在门禁脚本之间与 CI 里都按这个形状出现。
GATE_PATH_RE = re.compile(r"scripts/gate/([A-Za-z0-9_.-]+\.(?:sh|py))")

# `{VER}` 的候选拼写。仓库里同一版本号的写法并不统一：
# `check_alpha_v410.sh` / `check_alpha_v3.12.0.sh` / `check_alpha_v312.sh`。
VER_FORMS = ["v4.1.0", "v4.1", "4.1.0", "4.1", "v410", "410"]


def _yaml():
    try:
        import yaml  # noqa: PLC0415
    except ImportError:  # pragma: no cover - 运行环境缺 PyYAML 时直接失败
        sys.stderr.write("FATAL: PyYAML is required (pip install pyyaml)\n")
        raise
    return yaml


def strip_shell_comments(run_text: str) -> str:
    """剥掉 `run:` 块里的 **bash 注释行**。

    YAML 注释在解析 workflow 时就被丢掉了，但 `run: |` 块里的 bash 注释
    （`# ...`）**仍在文本里** —— 例如 ci.yml 的 `test` job 里有一段 bash 注释
    提到 `check_stage.sh`。把「提到」当成「调用」会让执行点矩阵**假通过**
    （这正是本门禁第一次跑出来时踩到的坑）。
    """
    kept = []
    for line in run_text.split("\n"):
        if line.lstrip().startswith("#"):
            continue
        kept.append(line)
    return "\n".join(kept)


def resolve_gate_path(entry: str) -> str | None:
    """把 `required_gates` 的一条解析成本仓真实存在的路径。

    `entry` 可能是带参数的命令（`scripts/gate/check_rc_ga_gate.sh ga`），
    只取第一个词。返回**第一个存在**的候选路径；都不存在则返回 None。
    """
    token = entry.strip().split()[0] if entry.strip() else ""
    if not token.startswith("scripts/"):
        return None
    if "{VER}" not in token:
        return token if os.path.exists(os.path.join(REPO, token)) else None
    forms = []
    for v in VER_FORMS:
        forms.append(token.replace("{VER}", v))
        # 点号去点变体：v4.1.0 → v410
        forms.append(token.replace("{VER}", v.replace(".", "")))
    for cand in forms:
        if os.path.exists(os.path.join(REPO, cand)):
            return cand
    return None


def gate_dependencies(script_rel: str) -> list[str]:
    """一个门禁脚本**自己**调用了哪些门禁脚本。

    只看 `scripts/gate/xxx.sh` 这种形状 —— 门禁之间的调用一律走这个路径，
    所以文本扫描足够可靠（不需要解析 bash）。
    """
    abs_path = os.path.join(REPO, script_rel)
    if not os.path.exists(abs_path):
        return []
    try:
        with open(abs_path, encoding="utf-8", errors="replace") as fh:
            body = fh.read()
    except OSError:
        return []
    out = []
    for name in GATE_PATH_RE.findall(body):
        rel = "scripts/gate/" + name
        if rel != script_rel and rel not in out:
            out.append(rel)
    return out


def ci_roots() -> tuple[dict[str, list[str]], list[dict]]:
    """CI 里真正被调用的门禁脚本。

    返回 (根脚本 → 调用点描述列表, 每个 step 的元信息)。
    **用 YAML 解析 workflow** ⇒ 注释里的 `check_stage.sh` 不会被算进去。
    """
    yaml = _yaml()
    roots: dict[str, list[str]] = {}
    steps: list[dict] = []
    if not os.path.isdir(WORKFLOW_DIR):
        return roots, steps
    for wf in sorted(os.listdir(WORKFLOW_DIR)):
        if not wf.endswith((".yml", ".yaml")):
            continue
        try:
            with open(os.path.join(WORKFLOW_DIR, wf), encoding="utf-8") as fh:
                doc = yaml.safe_load(fh)
        except Exception:  # noqa: BLE001 - 一个坏 workflow 不该让体检崩掉
            continue
        if not isinstance(doc, dict):
            continue
        for job_name, job in (doc.get("jobs") or {}).items():
            if not isinstance(job, dict):
                continue
            for step in (job.get("steps") or []):
                if not isinstance(step, dict):
                    continue
                run = step.get("run")
                if not isinstance(run, str):
                    continue
                run_text = strip_shell_comments(run)
                meta = {
                    "workflow": wf,
                    "job": job_name,
                    "name": step.get("name") or "",
                    "run": run_text,
                }
                steps.append(meta)
                for name in GATE_PATH_RE.findall(run_text):
                    rel = "scripts/gate/" + name
                    roots.setdefault(rel, []).append(
                        f"{wf}:{job_name}" + (f"/{step.get('name')}" if step.get("name") else "")
                    )
    return roots, steps


def reachable_from_roots(
    roots: dict[str, list[str]],
    steps: list[dict],
    driver_stages: dict[str, list[str]],
    stage: str | None = None,
) -> set[str]:
    """从 CI 根节点出发的传递闭包。

    `--dry-run` 的阶段驱动**不贡献边** —— 它只列出清单、不执行。
    """
    # 阶段驱动的调用点：**必须显式钉住 --stage S**，且**不能**带 --dry-run
    # （--dry-run 只逐条打印 `[DRY] <cmd>` 后 continue ⇒ 一个门禁都没跑）。
    # 显式钉 stage 是为了「清单与执行者一一对应」：否则 develop 上的一次调用
    # 就会把 GA 的清单也算成有执行者。
    # 注意：调用方传进来的 driver_stages 会被这里**覆盖重算**（保留形参只为签名兼容）。
    driver_stages: dict[str, list[str]] = {}
    for meta in steps:
        run = meta["run"]
        if "check_stage.sh" not in run:
            continue
        if "--dry-run" in run:
            continue
        m = re.search(r"--stage[= ]+([A-Za-z0-9]+)", run)
        if not m:
            continue
        driver_stages.setdefault(m.group(1).upper(), []).append(
            f"{meta['workflow']}:{meta['job']}" + (f"/{meta['name']}" if meta["name"] else "")
        )

    seen: set[str] = set()
    stack = list(roots.keys())
    while stack:
        cur = stack.pop()
        if cur in seen:
            continue
        seen.add(cur)
        for dep in gate_dependencies(cur):
            if dep not in seen:
                stack.append(dep)

    if stage and stage in driver_stages:
        # 阶段驱动会执行「它自己那个 stage 的 required_gates 清单」——
        # 清单里的门禁因此也是可达的。只并入**该 stage** 的清单项。
        _require_built("reachable_from_roots()")
        for rel in ALL_REQUIRED_SCRIPTS_BY_STAGE.get(stage, set()):
            if rel not in seen:
                seen.add(rel)
    return seen


ALL_REQUIRED_SCRIPT_PATHS: set[str] | None = None
ALL_REQUIRED_SCRIPTS_BY_STAGE: dict[str, set[str]] | None = None


class MatrixNotBuilt(RuntimeError):
    """未调用 build_matrix() 就读矩阵 ⇒ 报错，而不是给一个空集。

    P0-7：这两个全局是 `build_matrix()` 里懒填充的。若哨兵是**空集**，
    漏掉 build 的调用方会「读到空清单」而毫无感觉 —— 于是
    「清单里每道门禁都有执行点」这类判据会在**空集上真空通过**
    （0 违例 = 通过），或者把孤儿数虚高成一个看起来像结论的数字。
    两种都不可接受：前者让门禁永远绿着，后者让度量说谎。
    """


def _require_built(where: str) -> None:
    if ALL_REQUIRED_SCRIPT_PATHS is None or ALL_REQUIRED_SCRIPTS_BY_STAGE is None:
        raise MatrixNotBuilt(
            f"gate_executability: {where} 读取矩阵前必须先调用 build_matrix()"
        )


def required_script_paths() -> set[str]:
    _require_built("required_script_paths()")
    return set(ALL_REQUIRED_SCRIPT_PATHS)


def required_scripts_by_stage() -> dict[str, set[str]]:
    _require_built("required_scripts_by_stage()")
    return {s: set(v) for s, v in ALL_REQUIRED_SCRIPTS_BY_STAGE.items()}


def build_matrix() -> dict:
    yaml = _yaml()
    with open(STAGE_CONFIG, encoding="utf-8") as fh:
        cfg = yaml.safe_load(fh)
    stages = cfg.get("stages") or {}
    roots, steps = ci_roots()

    global ALL_REQUIRED_SCRIPT_PATHS, ALL_REQUIRED_SCRIPTS_BY_STAGE
    ALL_REQUIRED_SCRIPT_PATHS = set()
    ALL_REQUIRED_SCRIPTS_BY_STAGE = {}

    raw_entries: list[tuple[str, str, str]] = []  # (stage, kind, entry)
    for stage, info in stages.items():
        ALL_REQUIRED_SCRIPTS_BY_STAGE[stage] = set()
        for entry in (info.get("required_gates") or []):
            raw_entries.append((stage, "script" if entry.strip().startswith("scripts/") else "command", entry))
            resolved = resolve_gate_path(entry)
            if resolved:
                ALL_REQUIRED_SCRIPT_PATHS.add(resolved)
                ALL_REQUIRED_SCRIPTS_BY_STAGE[stage].add(resolved)

    # 阶段驱动把哪些 stage 显式钉住了（`--stage S` 且不带 `--dry-run`）
    driver_stages: dict[str, list[str]] = {}
    for meta in steps:
        run = meta["run"]
        if "check_stage.sh" not in run or "--dry-run" in run:
            continue
        m = re.search(r"--stage[= ]+([A-Za-z0-9]+)", run)
        if m:
            driver_stages.setdefault(m.group(1).upper(), []).append(
                f"{meta['workflow']}:{meta['job']}"
            )

    reach_by_stage = {
        st: reachable_from_roots(roots, steps, driver_stages, st) for st in stages
    }

    rows = []
    for stage, kind, entry in raw_entries:
        reachable = reach_by_stage.get(stage, set())
        if kind == "command":
            rows.append(
                {
                    "stage": stage,
                    "kind": "command",
                    "entry": entry,
                    "path": None,
                    "exists": True,
                    "executor": "stage-driver" if reachable else None,
                    "verdict": "OK" if reachable else "NO-EXECUTOR",
                }
            )
            continue
        resolved = resolve_gate_path(entry)
        if resolved is None:
            rows.append(
                {
                    "stage": stage,
                    "kind": "script",
                    "entry": entry,
                    "path": None,
                    "exists": False,
                    "executor": None,
                    "verdict": "MISSING",
                }
            )
            continue
        if resolved in reachable:
            executor = ", ".join(roots.get(resolved, [])) or (
                "stage-driver:" + ",".join(driver_stages.get(stage, []))
            )
            rows.append(
                {
                    "stage": stage,
                    "kind": "script",
                    "entry": entry,
                    "path": resolved,
                    "exists": True,
                    "executor": executor,
                    "verdict": "OK",
                }
            )
        else:
            rows.append(
                {
                    "stage": stage,
                    "kind": "script",
                    "entry": entry,
                    "path": resolved,
                    "exists": True,
                    "executor": None,
                    "verdict": "NO-EXECUTOR",
                }
            )

    bad = [r for r in rows if r["verdict"] != "OK"]
    return {
        "roots": {k: v for k, v in roots.items()},
        "rows": rows,
        "bad": bad,
        "summary": {
            "total": len(rows),
            "ok": len(rows) - len(bad),
            "missing": len([r for r in rows if r["verdict"] == "MISSING"]),
            "no_executor": len([r for r in rows if r["verdict"] == "NO-EXECUTOR"]),
        },
    }


def render(matrix: dict) -> None:
    summary = matrix["summary"]
    print("=== 门禁执行点矩阵（STAGE_CONFIG.required_gates × CI 可达性）===")
    by_stage: dict[str, list] = {}
    for row in matrix["rows"]:
        by_stage.setdefault(row["stage"], []).append(row)
    for stage in sorted(by_stage):
        rows = by_stage[stage]
        bad = [r for r in rows if r["verdict"] != "OK"]
        print(f"\n--- {stage}: {len(rows)} 项, OK {len(rows) - len(bad)}, 有问题 {len(bad)}")
        for row in rows:
            mark = "OK  " if row["verdict"] == "OK" else "FAIL"
            path = row["path"] or row["entry"]
            ex = row["executor"] or "-"
            print(f"  [{mark}] {row['verdict']:<12} {path}")
            if row["verdict"] != "OK":
                print(f"         executor: {ex}")
    print(
        "\n=== 汇总 ===\n"
        f"  总计 {summary['total']} 项 | OK {summary['ok']} | "
        f"MISSING {summary['missing']} | NO-EXECUTOR {summary['no_executor']}"
    )


def selftest() -> int:
    """变异自测：A 基线绿 → B 注入没人覆盖的门禁 ⇒ 红 → C 补上执行点 ⇒ 绿。"""
    import shutil
    import tempfile

    tmp = tempfile.mkdtemp(prefix="gate_exec_selftest_")
    try:
        wf = os.path.join(tmp, ".gitea/workflows")
        os.makedirs(wf)
        cfg = os.path.join(tmp, "docs/governance")
        os.makedirs(cfg)
        with open(os.path.join(cfg, "STAGE_CONFIG.yaml"), "w", encoding="utf-8") as fh:
            fh.write(
                "stages:\n"
                "  GA:\n"
                "    order: 5\n"
                "    description: t\n"
                "    branch_pattern: [\"main\"]\n"
                "    carrier_enforced: true\n"
                "    required_files: []\n"
                "    required_gates:\n"
                "      - \"scripts/gate/check_alpha_gate.sh\"\n"
                "      - \"scripts/gate/check_orphan_gate.sh\"\n"
            )
        os.makedirs(os.path.join(tmp, "scripts/gate"), exist_ok=True)
        for name in ("check_alpha_gate.sh", "check_orphan_gate.sh"):
            with open(os.path.join(tmp, "scripts/gate", name), "w", encoding="utf-8") as fh:
                fh.write("#!/bin/bash\necho hi\n")
        with open(os.path.join(wf, "ci.yml"), "w", encoding="utf-8") as fh:
            fh.write(
                "name: t\n"
                "jobs:\n"
                "  j:\n"
                "    steps:\n"
                "      - name: run\n"
                "        run: bash scripts/gate/check_alpha_gate.sh\n"
            )

        global REPO, STAGE_CONFIG, WORKFLOW_DIR
        global ALL_REQUIRED_SCRIPT_PATHS, ALL_REQUIRED_SCRIPTS_BY_STAGE
        old = (REPO, STAGE_CONFIG, WORKFLOW_DIR)
        REPO = tmp
        STAGE_CONFIG = os.path.join(tmp, "docs/governance/STAGE_CONFIG.yaml")
        WORKFLOW_DIR = os.path.join(tmp, ".gitea/workflows")
        ALL_REQUIRED_SCRIPT_PATHS = None
        ALL_REQUIRED_SCRIPTS_BY_STAGE = None

        # E —— 未 build_matrix() 就读矩阵 ⇒ 必须报错（不能返回空集）
        try:
            required_script_paths()
        except MatrixNotBuilt:
            print("SELFTEST E: OK（未构建时读矩阵抛 MatrixNotBuilt，不返回空集）")
        else:
            print("SELFTEST FAIL: E 臂期望 MatrixNotBuilt，实际返回了空集（fail-open）")
            return 1
        try:
            required_scripts_by_stage()
        except MatrixNotBuilt:
            pass
        else:
            print("SELFTEST FAIL: E 臂 required_scripts_by_stage() 未报错（fail-open）")
            return 1
        # E2 —— 内部路径同样：阶段驱动展开清单前也必须先构建。
        # 注意本函数会**从 steps 自己重算** driver_stages（调用方传的参数被忽略），
        # 所以这里喂一条 `check_stage.sh --stage GA`（不带 --dry-run）的 step。
        try:
            reachable_from_roots(
                {},
                [{"run": "bash scripts/gate/check_stage.sh --stage GA",
                  "workflow": "wf", "job": "job", "name": "step"}],
                {},
                "GA",
            )
        except MatrixNotBuilt:
            print("SELFTEST E2: OK（reachable_from_roots 未构建时同样报错）")
        else:
            print("SELFTEST FAIL: E2 臂期望 MatrixNotBuilt，实际静默展开空清单")
            return 1

        # A —— 基线：check_orphan_gate.sh 没有任何执行点 ⇒ 红，且**点名它**
        m = build_matrix()
        a_fail = [r for r in m["bad"] if "check_orphan_gate.sh" in r["entry"]]
        if len(a_fail) != 1 or a_fail[0]["verdict"] != "NO-EXECUTOR":
            print(f"SELFTEST FAIL: A 臂期望 check_orphan_gate.sh 判 NO-EXECUTOR，实际 {a_fail}")
            return 1
        print("SELFTEST A: OK（没人执行的门禁被判 NO-EXECUTOR 并点名）")

        # B —— 补上执行点 ⇒ 绿
        with open(os.path.join(wf, "ci.yml"), "a", encoding="utf-8") as fh:
            fh.write(
                "      - name: run2\n"
                "        run: bash scripts/gate/check_orphan_gate.sh\n"
            )
        m = build_matrix()
        if m["bad"]:
            print(f"SELFTEST FAIL: B 臂期望全绿，实际 {m['bad']}")
            return 1
        print("SELFTEST B: OK（补上执行点 ⇒ 绿）")

        # C —— `--dry-run` 的阶段驱动**不算**执行点
        with open(os.path.join(wf, "ci.yml"), "w", encoding="utf-8") as fh:
            fh.write(
                "name: t\n"
                "jobs:\n"
                "  j:\n"
                "    steps:\n"
                "      - name: dry\n"
                "        run: bash scripts/gate/check_stage.sh --version v4.1.0 --dry-run\n"
            )
        m = build_matrix()
        c_fail = [r for r in m["bad"] if "check_orphan_gate.sh" in r["entry"]]
        if len(c_fail) != 1 or c_fail[0]["verdict"] != "NO-EXECUTOR":
            print(f"SELFTEST FAIL: C 臂期望 dry-run 不算执行点，实际 {c_fail}")
            return 1
        print("SELFTEST C: OK（--dry-run 不被当成执行点）")

        # D —— `run:` 块里的 **bash 注释**提到门禁 ⇒ **不算**调用
        with open(os.path.join(wf, "ci.yml"), "w", encoding="utf-8") as fh:
            fh.write(
                "name: t\n"
                "jobs:\n"
                "  j:\n"
                "    steps:\n"
                "      - name: dry\n"
                "        run: |\n"
                "          # bash 注释里提到 scripts/gate/check_orphan_gate.sh\n"
                "          # check_stage.sh was the release-promotion backstop\n"
                "          echo nothing-to-run\n"
            )
        m = build_matrix()
        d_fail = [r for r in m["bad"] if "check_orphan_gate.sh" in r["entry"]]
        if len(d_fail) != 1 or d_fail[0]["verdict"] != "NO-EXECUTOR":
            print(f"SELFTEST FAIL: D 臂期望 bash 注释不算调用，实际 {d_fail}")
            return 1
        print("SELFTEST D: OK（run 块里的 bash 注释不被当成调用）")

        print("SELFTEST: PASS")
        REPO, STAGE_CONFIG, WORKFLOW_DIR = old
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    matrix = build_matrix()
    if "--json" in argv:
        print(json.dumps(matrix, ensure_ascii=False, indent=2))
    else:
        render(matrix)
    return 1 if matrix["bad"] else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
