#!/usr/bin/env python3
"""看板合并冲突的**唯一口径**与机器判据（看板 MERGE-1）。

口径（写在 docs/releases/v4.1.0/KANBAN_MERGE_POLICY_2026-11-08.md）：
    合并冲突时**两侧的证据都要留**。每条线的补充用**带身份前缀的 marker** 标注：
      `〔omp-macmini 线补记 ……〕`   /   `〔mcode-macmini 线补记 ……〕`
    若某个 PR **有意**删掉某一侧的行文本，必须在该行里**显式声明**
      （形如 `〔删去另一侧：<原因>〕`），否则机器判据判红。

机器判据（三条）：
    1. 行内只出现一侧 marker ⇒ 该行必须 (a) 属于**历史遗留清单**（GRANDFATHERED，来源见下），
       或 (b) 带显式删除声明；否则判红。
    2. 全表两侧 marker 的**总计数都必须非 0**（任一侧被整体丢掉 ⇒ 判红）。
    3. 历史遗留清单里的行**允许已被补齐**（补齐后不必从清单里删）。

历史遗留清单（GRANDFATHERED）的来源：2026-11-08 实测 —— 2026-11-06 的 develop 合并按侧丢文本，
本线 13 行里写的「合并保留两侧证据（未自行裁定）」标记当时**一处都没留下**，对方的 13 处却都留着。
今天的板子上仍可见 11 行「只有一侧 marker」：6 行只有本线（HO-1 / IL-4 / LB-1 / P1-5 / P2-2 / DEBT-2），
5 行只有对方（BIN-1 / ENG-11 / ENG-12 / EX-10 / WIRE-1）⇒ 逐个列名，不做静默豁免。

用法：
    python3 scripts/gate/kanban_merge_policy.py --selftest   # 反向变异自检
    python3 scripts/gate/kanban_merge_policy.py <board.jsonl> # 查某份板子

本模块**不新增门禁**、不改 STAGE_CONFIG ⇒ 门禁分母不变；判据由 `kanban_render.py check` 调用（既有工具）。
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# 本线 marker：〔omp-macmini 线补记 ……〕/〔omp 线……〕/〔2026-11-07 本线补记〕
MINE_RE = re.compile(r"〔\s*(?:omp-macmini|omp)\s*线补记|〔\s*omp\s*线|〔\s*2026-11-07\s*本线补记")
# 对方 marker：〔mcode-macmini 线补记 ……〕/〔另一侧补记……〕
THEIRS_RE = re.compile(r"〔\s*mcode-macmini\s*线补记|〔\s*另一侧补记")
# 显式声明「有意删去某一侧」
DECL_RE = re.compile(r"〔\s*删去另一侧[：:]")

# 历史遗留：2026-11-06 合并按侧丢文本导致的「只有一侧 marker」的行（逐个列名）。
GRANDFATHERED = {
    "HO-1", "IL-4", "LB-1", "P1-5", "P2-2", "DEBT-2",          # 只有本线
    "BIN-1", "ENG-11", "ENG-12", "EX-10", "WIRE-1",               # 只有对方
}


def _row_text(row: dict) -> str:
    return json.dumps(row, ensure_ascii=False)


def row_sides(row: dict) -> tuple[bool, bool, bool]:
    """返回 (has_mine, has_theirs, has_declaration)。"""
    s = _row_text(row)
    return bool(MINE_RE.search(s)), bool(THEIRS_RE.search(s)), bool(DECL_RE.search(s))


def check_rows(rows: list[dict]) -> list[str]:
    """返回问题列表；空列表 = 通过。"""
    problems: list[str] = []
    mine_total = theirs_total = 0
    for r in rows:
        rid = r.get("id") or "?"
        mine, theirs, decl = row_sides(r)
        mine_total += 1 if mine else 0
        theirs_total += 1 if theirs else 0
        if mine != theirs and not decl and rid not in GRANDFATHERED:
            side = "本线" if mine else "对方"
            problems.append(
                "row %s 只保留了%s的合并补记，另一侧的行文本不见了；"
                "要么补齐另一侧，要么在该行写明 `〔删去另一侧：<原因>〕`" % (rid, side)
            )
    if rows and mine_total == 0:
        problems.append("全表没有任何本线 marker ⇒ 合并把本线证据整体丢了")
    if rows and theirs_total == 0:
        problems.append("全表没有任何对方 marker ⇒ 合并把对方证据整体丢了")
    return problems


def _selftest() -> int:
    fails: list[str] = []

    def row(rid, text, status="完成"):
        return {"id": rid, "status": status, "task": text, "evidence": text, "acceptance": text}

    # A: 单侧 marker 且不在历史清单 ⇒ 必须被抓
    bad = [row("ZZ-A", "〔omp-macmini 线补记 —— 只有一侧〕")]
    if not check_rows(bad):
        fails.append("A: 单侧 marker 未被抓")
    else:
        print("[PASS] A: 单侧 marker（不在历史清单）被抓")
    # B: 单侧 + 显式声明 ⇒ 不得被抓
    #    （板子里同时要有另一侧的 marker，否则会触发「某侧整体消失」的聚合规则 —— 那是另一条判据）
    ok = [row("ZZ-B", "〔omp-macmini 线补记 —— 只有一侧〕〔删去另一侧：对方行文本在上游 PR 已重写〕"),
          row("ZZ-B2", "〔mcode-macmini 线补记 —— 对方〕〔omp-macmini 线补记 —— 本线也有〕")]
    if check_rows(ok):
        fails.append("B: 显式声明仍被判红")
    else:
        print("[PASS] B: 显式声明不误抓")
    # C: 两侧都有 ⇒ 不得被抓
    both = [row("ZZ-C", "〔omp-macmini 线补记 —— 本线〕〔mcode-macmini 线补记 —— 对方〕")]
    if check_rows(both):
        fails.append("C: 两侧都有仍被判红")
    else:
        print("[PASS] C: 两侧都有不误抓")
    # D: 历史清单里的单侧行 ⇒ 不得被抓（历史遗留，已逐个列名）
    legacy = [row("WIRE-1", "〔mcode-macmini 线补记 —— 历史遗留〕"),
              row("ZZ-D2", "〔omp-macmini 线补记 —— 本线〕〔mcode-macmini 线补记 —— 对方也有〕")]
    if check_rows(legacy):
        fails.append("D: 历史遗留行被误抓")
    else:
        print("[PASS] D: 历史遗留行不误抓（GRANDFATHERED 已列名）")
    # E: 某侧 marker 全部消失 ⇒ 必须被抓
    only_theirs = [row("ZZ-E1", "〔mcode-macmini 线补记 —— 对方〕"),
                   row("ZZ-E2", "〔mcode-macmini 线补记 —— 对方〕")]
    if not any("整体丢了" in p for p in check_rows(only_theirs)):
        fails.append("E: 一侧 marker 整体消失未被抓")
    else:
        print("[PASS] E: 一侧 marker 整体消失被抓")
    if fails:
        for f in fails:
            print("[FAIL] " + f)
        return 1
    print("kanban_merge_policy: OK (5 个反向变异臂全部符合期望)")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return _selftest()
    if not argv:
        print(__doc__)
        return 2
    path = Path(argv[0])
    rows = [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines() if l.strip()]
    problems = check_rows(rows)
    out = {
        "kind": "kanban_merge_policy",
        "board": str(path),
        "rows": len(rows),
        "mine_rows": sum(1 for r in rows if row_sides(r)[0]),
        "theirs_rows": sum(1 for r in rows if row_sides(r)[1]),
        "grandfathered": sorted(GRANDFATHERED),
        "problems": problems,
        "verdict": "FAIL" if problems else "OK",
    }
    print(json.dumps(out, ensure_ascii=False, indent=2, sort_keys=True))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
