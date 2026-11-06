#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# est_recompute.py 的反向变异（AGENTS 2.5 / 本仓「每条新判据必须反向变异」）。
#
#   python3 audit/v4.1.0/est_recompute_mutations.py
#
# 每条臂只在临时目录的副本里注入一个缺陷，仓库文件一律不动。
# 臂判据：注入后 est_recompute 必须变红；同时有两条「必须保持绿」的正臂
# （未变异基线、落在并集内的新读数），它们变红 ⇒ 判据本身过敏/源已漂。
#
# 退出码：0 = 全部反臂变红且正臂保持绿；1 = 有反臂仍绿（判据不承重）；
#         3 = 变异锚点命中次数 != 1（源已漂，证据过期）。
from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
TOOL = HERE / "est_recompute.py"
PLAN = REPO / "docs/releases/v4.1.0/GOVERNANCE_REMEDIATION_PLAN_2026-11-04.md"
DOC = REPO / "docs/releases/v4.1.0/ESTIMATE_DOUBLE_LINEAGE_2026-11-05.md"
RUN_ID = "20261105-est2-008"

# (name, 目标, 锚点, 替换, 期望抓到的 finding 前缀)
ARMS = [
    ("a1-tier-lower-drift", "doc", "86.0", "80.0", "doc-missing-tier-lower"),
    ("a2-blocksum-drift", "plan", "**9–14**", "**10–14**", "doc-missing-blocksum"),
    ("a3-total-mid-drift", "doc", "**213–234 人日**", "**213–240 人日**", "doc-missing-total-mid"),
    ("a4-union-drift", "doc", "**111–300 人日**", "**111–280 人日**", "doc-missing-union"),
    ("a5-seat-reading-dropped", "doc", "| **160–300 人日 / 20–40 周** |",
     "| 弃权（该行读数被删） |", "doc-missing-union"),
    ("a6-promise-out-of-band", "doc", "**≥190 人日**", "**≥390 人日**", "promise-out-of-baseline"),
    ("a7-calendar-band-regression", "doc", "**20–48 周**", "**20–37 周**", "doc-missing-calendar-band"),
    ("a8-promise-calendar-regression", "doc", "**29–38 周**", "**24–40 周**", "doc-missing-promise-calendar"),
    ("a9-drift-disclosure-removed", "doc", "不可复算", "可复算", "doc-missing-drift-disclosure"),
    ("a10-tool-weight-loosened", "tool", '"mid": (0.5, 1.25, 3.5)', '"mid": (0.5, 1.2, 3.5)', "doc-missing-total-mid"),
]


def live_census() -> str:
    p = subprocess.run([sys.executable, "scripts/gate/gov_probe.py", "issues"],
                       cwd=REPO, capture_output=True, text=True, timeout=300)
    for line in reversed(p.stdout.splitlines()):
        try:
            if json.loads(line).get("kind") == "issue_summary":
                return line
        except json.JSONDecodeError:
            continue
    raise SystemExit("est_recompute_mutations: 拿不到 live census")


def run_tool(tool: pathlib.Path, plan: pathlib.Path, doc: pathlib.Path,
             census: str, run_id: str = RUN_ID, fresh: str | None = None) -> subprocess.CompletedProcess:
    argv = [sys.executable, str(tool), "--plan", str(plan), "--doc", str(doc),
            "--require-run-id", run_id]
    if fresh:
        argv += ["--fresh-read", fresh]
    return subprocess.run(argv, input=census, capture_output=True, text=True, timeout=300)


def mutate(base: str, name: str, old: str, new: str, work: pathlib.Path, fname: str,
           all_hits: bool = False) -> pathlib.Path | None:
    n = base.count(old)
    if n == 0 or (n != 1 and not all_hits):
        print(f"DRIFT {name}: 锚点命中 {base.count(old)} 次 != 1")
        return None
    out = work / f"{name}-{fname}"
    out.write_text(base.replace(old, new), encoding="utf-8")
    return out


def main() -> int:
    census = live_census()
    tool_src = TOOL.read_text(encoding="utf-8")
    plan_src = PLAN.read_text(encoding="utf-8")
    doc_src = DOC.read_text(encoding="utf-8")

    red_flip = 0
    stayed_green: list[str] = []
    drift: list[str] = []
    positive_fail: list[str] = []

    work = pathlib.Path(tempfile.mkdtemp(prefix="est-mut-"))
    try:
        # 正臂 P0：未变异基线必须绿（否则源已漂，后面的结论都不成立）
        p = run_tool(TOOL, PLAN, DOC, census)
        if p.returncode != 0:
            positive_fail.append("p0-baseline")
            print(f"GREEN->RED p0-baseline: rc={p.returncode} {p.stdout.strip()[:200]}")
        else:
            print("OK    p0-baseline: 未变异基线保持绿")

        # 正臂 P1：新读数落在并集内 ⇒ 不得判过期
        fresh_in = work / "fresh-in.txt"
        fresh_in.write_text("按现状估计约 140–260 人日。\n", encoding="utf-8")
        p = run_tool(TOOL, PLAN, DOC, census, fresh=str(fresh_in))
        if p.returncode != 0:
            positive_fail.append("p1-fresh-inside-union")
            print(f"GREEN->RED p1: rc={p.returncode} {p.stdout.strip()[:200]}")
        else:
            print("OK    p1-fresh-inside-union: 并集内读数不误报")

        # 反臂
        for name, target, old, new, expect in ARMS:
            src = {"doc": doc_src, "plan": plan_src, "tool": tool_src}[target]
            fname = {"doc": "doc.md", "plan": "plan.md", "tool": "tool.py"}[target]
            # 「86.0」这类事实串在文档里出现多次（表里一次、结论里一次）：
            # 该臂要求把全部出现都改掉，锚点只要求至少命中一次；其余臂要求唯一。
            all_hits = name == "a1-tier-lower-drift"
            path = mutate(src, name, old, new, work, fname, all_hits=all_hits)
            if path is None:
                drift.append(name)
                continue
            tool_p = path if target == "tool" else TOOL
            plan_p = path if target == "plan" else PLAN
            doc_p = path if target == "doc" else DOC
            p = run_tool(tool_p, plan_p, doc_p, census)
            hit = expect in p.stdout
            if p.returncode != 0 and hit:
                red_flip += 1
                print(f"RED   {name}: rc={p.returncode} 抓到 {expect}")
            else:
                stayed_green.append(name)
                print(f"GREEN {name}: rc={p.returncode} 期望抓到 {expect} 未抓到")

        # 反臂 A11：并集外的新读数必须判过期
        fresh_out = work / "fresh-out.txt"
        fresh_out.write_text("按现状估计约 900–1500 人日。\n", encoding="utf-8")
        p = run_tool(TOOL, PLAN, DOC, census, fresh=str(fresh_out))
        if p.returncode != 0 and "fresh-read-drift-too-large" in p.stdout:
            red_flip += 1
            print("RED   a11-fresh-outside-union: 抓到 fresh-read-drift-too-large")
        else:
            stayed_green.append("a11-fresh-outside-union")
            print(f"GREEN a11: rc={p.returncode} 并集外读数未报警")

        # 反臂 A12：文档不指名证据 run ⇒ 必须红
        p = run_tool(TOOL, PLAN, DOC, census, run_id="20261105-none-999")
        if p.returncode != 0 and "doc-missing-run-ref" in p.stdout:
            red_flip += 1
            print("RED   a12-run-ref-dropped: 抓到 doc-missing-run-ref")
        else:
            stayed_green.append("a12-run-ref-dropped")
            print(f"GREEN a12: rc={p.returncode}")
    finally:
        shutil.rmtree(work, ignore_errors=True)

    arms = len(ARMS) + 2
    print(f"arms={arms} flipped_red={red_flip} stayed_green={len(stayed_green)} "
          f"drift={len(drift)} positive_fail={len(positive_fail)}")
    if drift:
        return 3
    return 1 if (stayed_green or positive_fail) else 0


if __name__ == "__main__":
    sys.exit(main())
