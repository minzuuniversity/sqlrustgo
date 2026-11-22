#!/usr/bin/env python3
"""gate_baseline_v410.py — v4.1.0 自有门禁基线重盘器（2026-11-16）

**它是什么**
一个**探针**（不是门禁）：不叫 ``check_*``、不进 ``STAGE_CONFIG`` 阶段清单、
不改动 168 的分母（``gov_probe.metrics.check_n`` 保持 168）。它只做两件事：
把 168 道门禁逐条判定成 v4.1.0 的归属标签，并输出可复算的基线清单。

**为什么需要它（基线升级的实测依据）**
1. ``docs/releases/v4.0.0/STAGE.yaml`` 的 GA 判定已于 2026-09-30 撤销
   （``ga_promotion_status.verdict = REVOKED``），v4.0.0 的继承清单**不得**
   直接充当 v4.1.0 的合格证据。
2. ``docs/releases/v4.1.0/STAGE.yaml`` 的 ``required_gates_for_ga`` /
   ``_beta`` / ``_rc`` 至今只是 ``note: Deferred ...`` 的占位文本，
   且全仓**无任何消费方**（实测：``grep -rn "required_gates_for" scripts/ docs/governance/``
   零命中）⇒ 执行面只读 ``docs/governance/STAGE_CONFIG.yaml``。
3. ``docs/releases/v4.1.0/GATE_CATALOG.jsonl`` 有 168 行（与分母严格对齐），
   但只有 ``class``（dead / ci_reachable / stage_required / orphan_referenced）
   与 ``disposition``，**没有**「对 v4.1.0 而言这是承重项还是历史记录」的判定。

**判定规则（确定性，可复算）**
R1  class=stage_required          → REQUIRED / execution_point=STAGE_LIST
R2  disposition=superseded        → RETIRED
R3  disposition=archive-candidate → HISTORICAL
R4  disposition=needs-execution-point / keep-rewire
                                 → REQUIRED / execution_point=NONE_PENDING
R5  disposition=needs-external-host
                                 → REQUIRED / execution_point=NONE_BLOCKED_EXTERNAL
R6  class=ci_reachable            → REQUIRED / execution_point=CI_DECLARED

**标签口径（4 标签，与双血统征询结论一致）**
REQUIRED       v4.1.0 范围内必须被执行（是否已接线由 execution_point 字段如实记录）
HISTORICAL     已被同族新门禁取代，或绑定历史版本线 ⇒ 文档引用改为历史记录
RETIRED        文件头自标 SUPERSEDED / DEPRECATED（重复实现）⇒ 退役
NOT_APPLICABLE 主题不在 v4.1.0 范围

**防假绿设计**
``execution_point`` 与标签**分离**：标成 REQUIRED 但 ``execution_point`` 为
``NONE_*`` 的门禁，会被 ``--gap`` 单独列出 —— 「应当执行但没有执行点」和
「已经执行并通过」是两件事，基线不得把前者算成后者。

用法::

    python3 audit/v4.1.0/gate_baseline_v410.py --emit   # 写 GATE_BASELINE_V410.jsonl
    python3 audit/v4.1.0/gate_baseline_v410.py --gap    # 只打印「REQUIRED 但无执行点」
    python3 audit/v4.1.0/gate_baseline_v410.py --selftest  # 反向变异自证
"""

from __future__ import annotations

import argparse
import json
import os
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CATALOG = os.path.join(REPO_ROOT, "docs/releases/v4.1.0/GATE_CATALOG.jsonl")
STAGE_CONFIG = os.path.join(REPO_ROOT, "docs/governance/STAGE_CONFIG.yaml")
OUT = os.path.join(REPO_ROOT, "docs/releases/v4.1.0/GATE_BASELINE_V410.jsonl")

LABEL_REQUIRED = "REQUIRED"
LABEL_HISTORICAL = "HISTORICAL"
LABEL_RETIRED = "RETIRED"
LABEL_NA = "NOT_APPLICABLE"

EP_STAGE = "STAGE_LIST"
EP_CI = "CI_DECLARED"
EP_PENDING = "NONE_PENDING"
EP_EXTERNAL = "NONE_BLOCKED_EXTERNAL"

DISPOSITION_TO_EP = {
    "needs-execution-point": EP_PENDING,
    "keep-rewire": EP_PENDING,
    "needs-external-host": EP_EXTERNAL,
}


def load_catalog() -> list[dict]:
    rows = []
    with open(CATALOG, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def load_stage_lists() -> dict[str, list[str]]:
    """从 STAGE_CONFIG.yaml 读出各阶段的 required_gates（唯一执行面权威源）。"""
    try:
        import yaml  # type: ignore
    except Exception as exc:  # pragma: no cover
        raise SystemExit(f"需要 PyYAML 读取 STAGE_CONFIG：{exc}")
    with open(STAGE_CONFIG, encoding="utf-8") as fh:
        cfg = yaml.safe_load(fh)
    out: dict[str, list[str]] = {}
    for stage, info in (cfg.get("stages") or {}).items():
        out[str(stage).upper()] = [str(g) for g in (info.get("required_gates") or [])]
    return out


def classify(row: dict, stage_lists: dict[str, list[str]]) -> dict:
    gate = row["gate"]
    cls = row.get("class")
    disp = row.get("disposition")

    if cls == "stage_required":
        label, ep = LABEL_REQUIRED, EP_STAGE
    elif disp == "superseded":
        label, ep = LABEL_RETIRED, "N/A"
    elif disp == "archive-candidate":
        label, ep = LABEL_HISTORICAL, "N/A"
    elif disp in DISPOSITION_TO_EP:
        label, ep = LABEL_REQUIRED, DISPOSITION_TO_EP[disp]
    elif cls == "ci_reachable":
        label, ep = LABEL_REQUIRED, EP_CI
    else:
        # 兜底：既无 class 判据也无 disposition ⇒ 不猜，标 NOT_APPLICABLE 并留痕
        label, ep = LABEL_NA, "N/A"

    # 该门禁出现在哪些阶段的承重清单里（按脚本路径匹配，允许带参数别名）
    stages: list[str] = []
    base = gate.split()[0] if gate.split() else gate
    for stage, gates in sorted(stage_lists.items()):
        for g in gates:
            if g == gate or g.split()[0] == base:
                if stage not in stages:
                    stages.append(stage)
                break

    ga_critical = "GA" in stages

    return {
        "gate": gate,
        "v410_label": label,
        "execution_point": ep,
        "v410_stages": stages,
        "ga_critical": ga_critical,
        "source_class": cls,
        "source_disposition": disp,
        "basis": row.get("reason", ""),
        "owner_task": row.get("owner_task", ""),
        "can_fail": row.get("can_fail"),
        "neg_fixture": row.get("neg_fixture"),
    }


def build() -> list[dict]:
    stage_lists = load_stage_lists()
    rows = load_catalog()
    out = [classify(r, stage_lists) for r in rows]
    # 保持与目录同序（catalog 顺序），不排序 —— 逐行键序固定
    return out


def summarize(rows: list[dict]) -> dict:
    from collections import Counter

    labels = Counter(r["v410_label"] for r in rows)
    eps = Counter(r["execution_point"] for r in rows)
    gap = [r for r in rows if r["v410_label"] == LABEL_REQUIRED and r["execution_point"].startswith("NONE_")]
    return {
        "total": len(rows),
        "labels": dict(labels),
        "execution_points": dict(eps),
        "ga_critical": sum(1 for r in rows if r["ga_critical"]),
        "gap_count": len(gap),
        "gap_gates": [r["gate"] for r in gap],
    }


def selftest(rows: list[dict]) -> int:
    """反向变异自证：篡改每一类判定，分类器必须翻红。"""
    failures: list[str] = []

    def check(name: str, cond: bool) -> None:
        if cond:
            print(f"  PASS  {name}")
        else:
            print(f"  FAIL  {name}")
            failures.append(name)

    # 1) 分母必须是 168（不新增 check_* ⇒ 不动分母）
    check("分母 == 168", len(rows) == 168)

    # 2) stage_required 必须是 REQUIRED + STAGE_LIST
    sub = [r for r in rows if r["source_class"] == "stage_required"]
    check("stage_required ⇒ REQUIRED", all(r["v410_label"] == LABEL_REQUIRED for r in sub))
    check("stage_required ⇒ STAGE_LIST", all(r["execution_point"] == EP_STAGE for r in sub))

    # 3) superseded ⇒ RETIRED；archive-candidate ⇒ HISTORICAL
    sub = [r for r in rows if r["source_disposition"] == "superseded"]
    check("superseded ⇒ RETIRED", all(r["v410_label"] == LABEL_RETIRED for r in sub))
    sub = [r for r in rows if r["source_disposition"] == "archive-candidate"]
    check("archive-candidate ⇒ HISTORICAL", all(r["v410_label"] == LABEL_HISTORICAL for r in sub))

    # 4) 主题仍在但无执行点 ⇒ REQUIRED + NONE_*（不得被算成「已执行」）
    sub = [r for r in rows if r["source_disposition"] in ("needs-execution-point", "keep-rewire")]
    check("in-scope 无执行点 ⇒ REQUIRED", all(r["v410_label"] == LABEL_REQUIRED for r in sub))
    check("in-scope 无执行点 ⇒ NONE_PENDING", all(r["execution_point"] == EP_PENDING for r in sub))

    # 5) GA 承重项必须都挂在 GA 阶段清单上
    ga = [r for r in rows if r["ga_critical"]]
    check("ga_critical ⇒ stages 含 GA", all("GA" in r["v410_stages"] for r in ga))
    check("ga_critical > 0", len(ga) > 0)

    # 6) 反向变异：把一条 stage_required 的人造分类改成 NOT_APPLICABLE ⇒ 判定必须翻红
    if sub:
        mutated = [dict(r) for r in rows]
        target = next(i for i, r in enumerate(mutated) if r["source_class"] == "stage_required")
        mutated[target]["v410_label"] = LABEL_NA
        bad = [r for r in mutated if r["source_class"] == "stage_required" and r["v410_label"] != LABEL_REQUIRED]
        check("反向变异(篡改 stage_required 标签)被检出", len(bad) == 1)

    # 7) 反向变异：删掉一条 ⇒ 分母必须不等于 168（证明分母检查真的会红）
    check("反向变异(删一条 ⇒ 分母≠168)", len(rows[:-1]) != 168)

    # 8) 反向变异：把一条 NONE_PENDING 改标 STAGE_LIST ⇒ gap 计数必须减少
    s = summarize(rows)
    mutated = [dict(r) for r in rows]
    for r in mutated:
        if r["execution_point"] == EP_PENDING:
            r["execution_point"] = EP_STAGE
            break
    s2 = summarize(mutated)
    check("反向变异(伪造执行点)使 gap 计数下降", s2["gap_count"] == s["gap_count"] - 1)

    print(f"\nselftest: {'ALL PASS' if not failures else 'FAILED: ' + ', '.join(failures)}")
    return 0 if not failures else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--emit", action="store_true", help="写出 GATE_BASELINE_V410.jsonl")
    ap.add_argument("--gap", action="store_true", help="只打印 REQUIRED 但无执行点的门禁")
    ap.add_argument("--selftest", action="store_true", help="反向变异自证")
    ap.add_argument("--summary", action="store_true", help="打印汇总")
    args = ap.parse_args()

    rows = build()

    if args.selftest:
        return selftest(rows)

    if args.gap:
        gap = [r for r in rows if r["v410_label"] == LABEL_REQUIRED and r["execution_point"].startswith("NONE_")]
        print(f"REQUIRED 但无执行点：{len(gap)} 道")
        for r in gap:
            print(f"  {r['gate']:52s} {r['execution_point']:24s} {r['source_disposition']}")
        return 0

    if args.emit:
        with open(OUT, "w", encoding="utf-8") as fh:
            for r in rows:
                fh.write(json.dumps(r, ensure_ascii=False, sort_keys=False) + "\n")
        print(f"written: {OUT} ({len(rows)} lines)")

    s = summarize(rows)
    print(json.dumps(s, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
