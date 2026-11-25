#!/usr/bin/env python3
"""P1-4：每道门禁写 `effect`，并核对「声明」与「夹具」是否一致。

背景（看板 P1-4）：整改方案 §P1-4 的验收是
「**声称能抓 X 但没有 X 的夹具 ⇒ 红**」。

之前的状态：168 道门禁**没有一处**声明自己「能抓什么、抓不到什么」，
所以「声称」与「夹具」之间**没有任何可比对的东西** ——
`docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl` 记了 57 行夹具账，
但没有人说「这道门禁抓的是哪几类」，也就没人能发现
「它声称抓 X，夹具其实只碰了 Y」。

本工具做三件事：

  emit   给 GATE_CATALOG.jsonl 的每一行补上 `effect` 声明：
           有**可证伪**夹具的门禁 ⇒ `catches` 列出该夹具确实证明能抓的性质；
           其余门禁 ⇒ `catches` 为空，并写明为什么现在还不能声称。
           「还不能声称」本身也是一种诚实的声明。
  check  核对声明与夹具一致，违反即红（见下方四条判据）。
  report 打印分级汇总。

四条判据（`check`）：

  C1 每个门禁都必须有 `effect` 声明 —— 缺声明 = 不可核对 = 红。
  C2 `effect.catches` 非空 ⇒ 该门禁必须有一行**可证伪**夹具
     （kind ∈ {mutation-verified, selftest-verified}）。
     没有 ⇒ 「声称能抓 X 但没有 X 的夹具」 ⇒ 红。
  C3 `effect.catches` 里每个性质 id 必须能在该夹具的
     `fixture_id` / `note` 里**逐字找到**（归一化后子串匹配）。
     找不到 ⇒ 声称的性质没有落在任何夹具证据上 ⇒ 红。
  C4 `kind == unfalsifiable` 的门禁 ⇒ `catches` 必须为空
     （除非写了 reason）—— 不可证伪的门禁不许声称能抓任何东西。

归一化：转小写，非字母数字一律折成 `-`，于是
「registry drift」与「registry-drift」是同一个 token。

本工具放在 audit/v4.1.0/：探针，不是门禁，不进 STAGE_CONFIG，分母 168 不动。
用法：
    honesty.py emit  [catalog]
    honesty.py check [catalog]
    honesty.py report [catalog]
退出码：0 = 全部成立；1 = 有不成立项；2 = 用法错
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CATALOG = REPO / "docs" / "releases" / "v4.1.0" / "GATE_CATALOG.jsonl"
FIXTURES = REPO / "docs" / "releases" / "v4.1.0" / "NEG_FIXTURES_v4.1.0.jsonl"

FALSIFIABLE_KINDS = ("mutation-verified", "selftest-verified")

# 可证伪门禁各自「能抓的性质」—— 每个 id 都必须逐字出现在该夹具的
# fixture_id 或 note 里（C3 判据会核对）。这是**人工撰写**的声明，
# 不是从散文里刮出来的 —— 刮出来的 id 会在夹具改写时静默失配。
CATCHES = {
    "scripts/gate/check_anti_ignore_gate.sh": ["registry-drift"],
    "scripts/gate/check_cross_version_debt.sh": ["doc-missing"],
    "scripts/gate/check_docs_links.sh": ["links-broken"],
    "scripts/gate/check_drift_not_pass.sh": ["new-antipattern"],
    "scripts/gate/check_gate_executability.sh": ["missing-script"],
    "scripts/gate/check_int_debt.sh": ["doc-missing"],
    "scripts/gate/check_tests_dont_write_archived_evidence.sh": ["write-archived-evidence"],
    "scripts/gate/check_alpha_entry_v4.10.sh": ["doc-missing"],
    "scripts/gate/check_stage.sh": ["version-uninferable"],
    "scripts/gate/check_commit_message.sh": ["坏提交信息"],
    "scripts/gate/check_coverage_single_implementation.sh": ["scan_hits"],
    "scripts/gate/check_gate_test_integrity.sh": ["掩码检测器"],
    # The ALPHA stage gates whose negative fixture is established (v4.1.0
    # readiness plan, Task 8).  The property id of each one is a literal
    # substring of its fixture's fixture_id, so the C3 criterion can find it
    # on the evidence.
    #
    # check_coverage_v312.sh is deliberately NOT here: its fixture exists but
    # could not be established inside the ledger's 30-minute per-fixture
    # budget (the gate measures eight crates with `cargo llvm-cov`,
    # single-threaded and instrumented).  Its ledger row therefore stays
    # kind=pending-fixture, and claiming a property it cannot yet back would
    # be exactly the kind of over-claim this file exists to prevent.
    "scripts/gate/check_alpha_v410.sh": ["entry-doc-missing"],
    "scripts/gate/check_arch_invariants.sh": ["invariant-impl-removed"],
    "scripts/gate/check_arch3_no_bypass.sh": ["dml-guard-dropped"],
    "scripts/gate/check_sql_corpus_gate.sh": ["corpus-truncated"],
    "scripts/gate/check_g1_tpch_baseline.sh": ["hash-tampered"],
}

NO_CLAIM_YET = ("尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），"
                "因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。")


def norm(s: str) -> str:
    """归一化：小写，非字母数字折成 '-'，连续 '-' 压成一个。"""
    t = re.sub(r"[^0-9a-z\u4e00-\u9fff]+", "-", s.lower())
    return t.strip("-")


def load_jsonl(path: Path) -> list[dict]:
    if not path.is_file():
        return []
    return [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines() if l.strip()]


def fixture_index() -> dict[str, dict]:
    """门禁 → 夹具行。一道门禁可能有多行，取最强的那行。"""
    order = {"mutation-verified": 0, "selftest-verified": 1,
             "unfalsifiable": 2, "pending-fixture": 3}
    idx: dict[str, dict] = {}
    for r in load_jsonl(FIXTURES):
        g = r.get("gate")
        if not g:
            continue
        cur = idx.get(g)
        if cur is None or order.get(r.get("kind"), 9) < order.get(cur.get("kind"), 9):
            idx[g] = r
    return idx


def fixture_evidence_text(row: dict) -> str:
    return " ".join(str(row.get(k) or "") for k in ("fixture_id", "note", "fixture"))


def cmd_emit(argv: list[str]) -> int:
    catalog = Path(argv[0]) if argv else CATALOG
    rows = load_jsonl(catalog)
    fx = fixture_index()
    for r in rows:
        g = r.get("gate")
        f = fx.get(g)
        kind = (f or {}).get("kind")
        if g in CATCHES and kind in FALSIFIABLE_KINDS:
            effect = {"catches": list(CATCHES[g]),
                      "cannot_catch": [],
                      "reason": f"由夹具 {f.get('fixture_id') or f.get('fixture')} 证明"
                                f"（kind={kind}）"}
        elif kind == "unfalsifiable":
            effect = {"catches": [], "cannot_catch": [],
                      "reason": "不可证伪（kind=unfalsifiable）⇒ 不声称能抓任何东西；"
                                "已知问题见 ISSUE #5598 / NF-1"}
        else:
            effect = {"catches": [], "cannot_catch": [], "reason": NO_CLAIM_YET}
        r["effect"] = effect
    catalog.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                       encoding="utf-8")
    print(json.dumps({"kind": "effect_emit", "catalog": str(catalog), "gates": len(rows),
                      "with_catches": sum(1 for r in rows if r["effect"]["catches"]),
                      "declared_empty": sum(1 for r in rows if not r["effect"]["catches"])},
                     ensure_ascii=False))
    return 0


def cmd_check(argv: list[str]) -> int:
    catalog = Path(argv[0]) if argv else CATALOG
    rows = load_jsonl(catalog)
    fx = fixture_index()
    fails: list[str] = []

    for r in rows:
        g = r.get("gate") or "?"
        effect = r.get("effect")
        # C1 缺声明
        if not isinstance(effect, dict):
            fails.append(f"C1 {g} 缺 effect 声明 —— 不可核对")
            continue
        catches = effect.get("catches") or []
        f = fx.get(g)
        kind = (f or {}).get("kind")
        # C2 声称能抓 ⇒ 必须有可证伪夹具
        if catches and kind not in FALSIFIABLE_KINDS:
            fails.append(f"C2 {g} 声称能抓 {catches}，但夹具 kind="
                         f"{kind or '（无夹具行）'} —— 声称能抓 X 却没有 X 的夹具")
            continue
        # C3 每个声称的性质必须逐字落在夹具证据上
        if catches and f is not None:
            text = norm(fixture_evidence_text(f))
            for pid in catches:
                if norm(pid) not in text:
                    fails.append(f"C3 {g} 声称能抓 {pid!r}，但该夹具的 "
                                 f"fixture_id/note 里找不到它 —— 声明没落在证据上")
        # C4 不可证伪的门禁不许声称能抓
        if kind == "unfalsifiable" and catches and not (effect.get("reason") or "").strip():
            fails.append(f"C4 {g} 是不可证伪门禁，却声称能抓 {catches} 且没写 reason")

    # 反向：夹具账里声称可证伪、但目录里说「不声称」的矛盾
    for g, f in fx.items():
        if f.get("kind") in FALSIFIABLE_KINDS:
            hit = [r for r in rows if r.get("gate") == g]
            if not hit:
                fails.append(f"C5 夹具账有可证伪夹具 {g}，但门禁登记表里没有这一行")
            elif hit[0].get("effect", {}).get("catches"):
                pass  # 已声明，正常
            elif f.get("kind") == "mutation-verified":
                # 可证伪却没声明能抓什么 —— 漏声明，不是谎报，但仍会让判据变弱
                fails.append(f"C5 {g} 有可证伪夹具（{f.get('fixture_id')}）却没声明 effect.catches "
                             "—— 夹具白做了")

    print(json.dumps({"kind": "honesty_check", "catalog": str(catalog),
                      "gates": len(rows), "fixtures": len(fx),
                      "fails": len(fails)}, ensure_ascii=False))
    for x in fails[:40]:
        print("FAIL:", x)
    if fails:
        print(f"honesty: {len(fails)} FAIL")
        return 1
    print("honesty: OK")
    return 0


def cmd_report(argv: list[str]) -> int:
    catalog = Path(argv[0]) if argv else CATALOG
    rows = load_jsonl(catalog)
    fx = fixture_index()
    claimed = [r for r in rows if (r.get("effect") or {}).get("catches")]
    print(json.dumps({"kind": "honesty_report", "gates": len(rows),
                      "fixture_rows": len(fx),
                      "declared_catches": len(claimed),
                      "declared_empty": len(rows) - len(claimed),
                      "falsifiable_gates": sum(1 for g, f in fx.items()
                                               if f.get("kind") in FALSIFIABLE_KINDS)},
                     ensure_ascii=False))
    for r in claimed:
        e = r["effect"]
        f = fx.get(r["gate"]) or {}
        print(f"  {r['gate']}\n      catches={e['catches']}  fixture={f.get('fixture_id')}")
    return 0


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0 if argv else 2
    fn = {"emit": cmd_emit, "check": cmd_check, "report": cmd_report}[argv[0]]
    return fn(argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
