#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# est_recompute.py 的反向变异（AGENTS 2.5 / 本仓「每条新判据必须反向变异」）。
#
#   python3 audit/v4.1.0/est_recompute_mutations.py
#   python3 audit/v4.1.0/est_recompute_mutations.py --live-census   # 严格模式：绑活普查
#
# 默认绑**仓内普查快照** `audit/v4.1.0/census/20261105-est2-008.json`（从该 run 的 logs 取回），
# 并把「今天再数一遍」的差异作为对照行打出来；`--live-census` 才让活普查进入判据
# （那时语料漂移会让正臂红 —— 那是「该重做估算」的信号，不是「文档坏了」）。
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


# 正臂 P0 判的是「文档在它自己那次普查下是否自洽」。绑活普查 ⇒ 语料一动（新开/关 issue）
# 正臂就红，跟文档对不对无关：2026-11-08 实测活普查已漂到 open_total=102（快照是 112），
# 两条正臂当场红，而 12 条反臂照旧全红 ⇒ 红的是绑定，不是判据。
# 归档 run 008 当年把 census 写进 /tmp（macOS 会清），今天已不可复现 ⇒ 快照从该 run 的
# logs 里取回，落在仓内，随证据一起进版本库。
SNAPSHOT = HERE / "census" / f"{RUN_ID}.json"


def census_input(use_live: bool) -> str:
    if use_live:
        print("census vintage = LIVE（严格模式：语料漂移会让正臂红，这是预期，见 §6）")
        return live_census()
    if not SNAPSHOT.is_file():
        raise SystemExit(
            f"est_recompute_mutations: 缺仓内普查快照 {SNAPSHOT.relative_to(REPO)}"
            f" —— 应从 run {RUN_ID} 的 logs/001-est2-census-print.stdout 取回，别再退回 /tmp"
        )
    text = SNAPSHOT.read_text(encoding="utf-8").strip()
    print(f"census vintage = 快照 {SNAPSHOT.relative_to(REPO)}"
          f"（open_total={json.loads(text)['open_total']}）")
    return text


def report_live_drift(snapshot_text: str) -> None:
    """「今天再数一遍」的差异必须打出来：它不该让正臂红（文档是有日期的估算），
    但也不能不说 —— 语料漂了就意味着该重做估算，而不是让旧估算继续背书。"""
    try:
        live = json.loads(live_census())
    except SystemExit:
        print("live-census 对照：取不到活普查 ⇒ 这一行不给结论（不是「未漂」）")
        return
    snap = json.loads(snapshot_text)
    if snap.get("open_total") == live.get("open_total") and snap.get("cost") == live.get("cost"):
        print("live-census 对照：与快照一致 ⇒ 估算语料未漂")
    else:
        print(f"live-census 对照：语料已漂 —— 快照 open_total={snap.get('open_total')} "
              f"cost={snap.get('cost')} ⇒ 现值 open_total={live.get('open_total')} "
              f"cost={live.get('cost')}。本文属历史估算；要更新就按 §6 重跑普查并另立一节，"
              "别改本文的读数冒充现值。")


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
    use_live = "--live-census" in sys.argv
    census = census_input(use_live)
    if not use_live:
        report_live_drift(census)
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
