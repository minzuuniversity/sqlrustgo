#!/usr/bin/env python3
"""校验 v4.1.0 issue 台账与其分诊原始件是否互相指得回（IL-1 / IL-4 的判据）。

放在 audit/ 而不是 scripts/gate/：它是 run 级校验器，不进门禁分母。

台账按批次命名并存，不覆盖：
  A 代 = ..._2026-11-05.{jsonl,md}     当期 open=106 的快照（历史审计记录）
  B 代 = ..._2026-11-05B.{jsonl,md}    当期 open=110 的快照（含 #5594/#5595/#5596/#5597 的分诊）
默认校验 B 代；用 --ledger-md/--ledger-jsonl/--triage 可切回 A 代，两套都必须各自自洽。

判据（任一不成立 ⇒ exit 1，绝不"看情况"）：
  L1 台账行数 == 分诊 jsonl 覆盖的 issue 数，且每行都有 domain / automatable
  L2 每行 evidence_grade ∈ {advisory-only, cross-checked}；
     cross-checked 的行必须真的被 >=2 个 family 分诊过（对照 triage jsonl 现场重算）；
     且 ladder credited lineage 少于 2 个时，任何行都不许是 cross-checked（与 issue_ledger 同规则）
  L3a MD 里记录的输入指纹 == 仓内文件的实测 sha256；且必须显式记录 issues 快照指纹
  L3b MD 的 evidence_hash == 由 MD 自记输入指纹按名排序拼接后重算（生成端同式）
  L4 不完整性显式化：MD 若写了"缺分诊行"，行数必须与之一致

反向变异（AGENTS §2.5，由 verify_ledger_mutation.sh 机器复现，四臂）：
  CTRL  原样 B 代 ⇒ 必须绿（否则本校验永远红，等于没判据）
  A     把 cross-checked 的 family 门限从 2 放宽到 1 ⇒ 必须红
        （证明等级不是自报，而是真的受">=2 family"这条规则约束）
  B1    把 MD 里某条输入指纹改一个十六进制位 ⇒ 必须红（L3a 抓指纹漂移）
  B2    把台账 jsonl 里一行 advisory-only 改成 cross-checked ⇒ 必须红（L2 抓等级虚报）
  B3    把 MD 的 evidence_hash 改一个十六进制位 ⇒ 必须红（L3b 重算抓"指纹与总哈希脱钩"）
  B4    从 MD 删掉一条输入指纹 ⇒ 必须红（少报一个输入也过不去）
本文件不提供"关掉比对"的开关：能自己调弱的判据不算判据。
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs" / "releases" / "v4.1.0"

DEFAULT_LEDGER_MD = DOCS / "ISSUE_LEDGER_2026-11-05B.md"
DEFAULT_LEDGER_JSONL = DOCS / "ISSUE_LEDGER_2026-11-05B.jsonl"
DEFAULT_TRIAGE = [DOCS / "ISSUE_TRIAGE_2026-11-05.jsonl",
                  DOCS / "ISSUE_TRIAGE_CROSSCHECK_2026-11-05.jsonl",
                  DOCS / "ISSUE_TRIAGE_2026-11-05B.jsonl",
                  DOCS / "ISSUE_TRIAGE_CROSSCHECK_2026-11-05B.jsonl",
                  # IL-2：gpt 血统对 35 条 P0 的批量复核（第二个 family）
                  DOCS / "ISSUE_TRIAGE_CROSSCHECK_2026-11-05C.jsonl",
                  # IL-2：live 标签下新变成 P0 的 8 条（同为 gpt 血统批量复核）
                  DOCS / "ISSUE_TRIAGE_CROSSCHECK_2026-11-05D.jsonl"]

FAILS: list[str] = []


def fail(msg: str) -> None:
    FAILS.append(msg)


def sha256(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--label", default="B", help="台账批次标签（仅用于输出）")
    ap.add_argument("--ledger-md", default=str(DEFAULT_LEDGER_MD))
    ap.add_argument("--ledger-jsonl", default=str(DEFAULT_LEDGER_JSONL))
    ap.add_argument("--triage", action="append", default=None,
                    help="分诊 jsonl（可重复；缺省 = B 代四件套）")
    ap.add_argument("--family-min", type=int, default=2,
                    help="cross-checked 所需的最少 family 数（仅变异臂 A 用）")
    args = ap.parse_args(argv)

    ledger_md = Path(args.ledger_md)
    ledger_jsonl = Path(args.ledger_jsonl)
    triage_files = [Path(p) for p in (args.triage or DEFAULT_TRIAGE)]

    for p in [ledger_md, ledger_jsonl] + triage_files:
        if not p.is_file():
            print(f"MISSING: {p}")
            return 2

    rows = [json.loads(l) for l in ledger_jsonl.read_text(encoding="utf-8").splitlines()
            if l.strip()]
    triage: dict[int, list[dict]] = collections.defaultdict(list)
    for p in triage_files:
        for l in p.read_text(encoding="utf-8").splitlines():
            if not l.strip():
                continue
            rec = json.loads(l)
            if rec.get("triage_ok"):
                triage[int(rec["number"])].append(rec)

    # L1
    if len(rows) != len(triage):
        fail(f"L1 行数 {len(rows)} != 有分诊行的 issue 数 {len(triage)}")
    no_field = [r["number"] for r in rows if not r.get("domain") or not r.get("automatable")]
    if no_field:
        fail(f"L1 {len(no_field)} 行缺 domain/automatable: {no_field[:5]}")

    md = ledger_md.read_text(encoding="utf-8")

    # L2 现场重算，不采信台账自报。等级规则与生成端 issue_ledger.py 对齐：
    # grade = cross-checked 仅当 (credited lineage >= 2) 且 (该 issue 被 >=2 family 分诊)
    ml = re.search(r"ladder credited lineage=([^；;\n]+)", md)
    if not ml:
        fail("L2 MD 缺 ladder credited lineage 字段（等级规则无法核对）")
    lineages = {x.strip() for x in (ml.group(1).split(",") if ml else []) if x.strip()}
    ladder_ready = len(lineages) >= 2

    grade_ct = collections.Counter()
    for r in rows:
        fams = {((t.get("triaged_by") or {}).get("family") or "")
                for t in triage.get(r["number"], [])}
        fams.discard("")
        want = ("cross-checked"
                if (ladder_ready and len(fams) >= args.family_min) else "advisory-only")
        grade_ct[r.get("evidence_grade")] += 1
        if r.get("evidence_grade") != want:
            fail(f"L2 #{r['number']} 自报 {r.get('evidence_grade')}，"
                 f"现场重算应为 {want}（families={sorted(fams)}）")
        if r.get("evidence_grade") not in ("advisory-only", "cross-checked"):
            fail(f"L2 #{r['number']} 证据等级非法: {r.get('evidence_grade')}")

    # L3 MD 指纹 vs 实测
    recorded = dict(re.findall(r"- `([A-Za-z0-9_.-]+\.jsonl?)` = `(sha256:[0-9a-f]{64})`", md))
    for p in triage_files:
        got = sha256(p)
        if recorded.get(p.name) != got:
            fail(f"L3a {p.name} MD 记 {recorded.get(p.name)} 实测 {got}")
    m = re.search(r"evidence_hash \| `?(sha256:[0-9a-f]{64})`?", md)
    if not m:
        fail("L3a MD 缺 evidence_hash 行")
    # 「issues 快照」按**它不是分诊文件**来认，不按扩展名认。
    # 仓内禁 `*.json`，所以快照是 `.jsonl`；判据要的是「输入被记了指纹」，
    # 不是「扩展名长什么样」。
    triage_names = {p.name for p in triage_files}
    snapshots = [k for k in recorded if k not in triage_names]
    if not snapshots:
        fail("L3a MD 缺 issues 快照指纹（应有一条不属于分诊文件的输入指纹）")

    # L3b：总哈希必须能由 MD 自记的输入指纹重算出来（与 issue_ledger.py 同式）
    if m and recorded:
        recomputed = "sha256:" + hashlib.sha256(
            "".join(recorded[k] for k in sorted(recorded)).encode("utf-8")).hexdigest()
        if recomputed != m.group(1):
            fail(f"L3b evidence_hash MD 记 {m.group(1)} 重算 {recomputed}"
                 f"（自记输入 {len(recorded)} 条）")

    # L4 显式不完整
    m2 = re.search(r"缺分诊行：(\d+) 条", md)
    if bool(m2) != bool(no_field or (len(rows) != len(triage))):
        fail("L4 MD 的缺行声明与实际不一致")

    print(json.dumps({
        "kind": "ledger_verify",
        "batch": args.label,
        "credited_lineages": sorted(lineages),
        "ladder_ready": ladder_ready,
        "ledger_md": str(ledger_md.relative_to(ROOT)) if ledger_md.is_relative_to(ROOT) else str(ledger_md),
        "rows": len(rows),
        "triaged_issues": len(triage),
        "grades": dict(grade_ct),
        "triage_files": {p.name: sha256(p) for p in triage_files},
        "snapshot_fingerprints": {k: recorded[k] for k in sorted(snapshots)},
        "md_evidence_hash": (m.group(1) if m else None),
    }, ensure_ascii=False))
    if FAILS:
        for f in FAILS:
            print("FAIL:", f)
        print(f"verify_ledger[{args.label}]: {len(FAILS)} FAIL")
        return 1
    print(f"verify_ledger[{args.label}]: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
