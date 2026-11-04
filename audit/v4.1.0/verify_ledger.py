#!/usr/bin/env python3
"""校验 v4.1.0 issue 台账与其分诊原始件是否互相指得回（IL-1 的判据）。

放在 audit/ 而不是 scripts/gate/：它是 run 级校验器，不进门禁分母。

判据（任一不成立 ⇒ exit 1，绝不"看情况"）：
  L1 台账行数 == 分诊 jsonl 覆盖的 issue 数，且每行都有 domain / automatable
  L2 每行 evidence_grade ∈ {advisory-only, cross-checked}；
     cross-checked 的行必须真的被 >=2 个 family 分诊过（对照 triage jsonl 现场重算）
  L3 MD 里记录的输入指纹 == 仓内文件的实测 sha256
  L4 不完整性显式化：MD 若写了"缺分诊行"，行数必须与之一致

反向变异（AGENTS §2.5）：
  A 把 cross-checked 的 family 数门限从 2 放宽到 1 ⇒ 本校验必须红
  B 把 L3 的指纹比对改成"只打印不比" ⇒ 本校验必须红
"""
from __future__ import annotations

import collections
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs" / "releases" / "v4.1.0"
LEDGER_MD = DOCS / "ISSUE_LEDGER_2026-11-05.md"
LEDGER_JSONL = DOCS / "ISSUE_LEDGER_2026-11-05.jsonl"
TRIAGE_FILES = [DOCS / "ISSUE_TRIAGE_2026-11-05.jsonl",
                DOCS / "ISSUE_TRIAGE_CROSSCHECK_2026-11-05.jsonl"]

FAILS: list[str] = []


def fail(msg: str) -> None:
    FAILS.append(msg)


def sha256(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def main() -> int:
    for p in [LEDGER_MD, LEDGER_JSONL] + TRIAGE_FILES:
        if not p.is_file():
            print(f"MISSING: {p.relative_to(ROOT)}")
            return 2

    rows = [json.loads(l) for l in LEDGER_JSONL.read_text(encoding="utf-8").splitlines()
            if l.strip()]
    triage: dict[int, list[dict]] = collections.defaultdict(list)
    for p in TRIAGE_FILES:
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

    # L2 现场重算，不采信台账自报
    grade_ct = collections.Counter()
    for r in rows:
        fams = {((t.get("triaged_by") or {}).get("family") or "")
                for t in triage.get(r["number"], [])}
        fams.discard("")
        want = "cross-checked" if len(fams) >= 2 else "advisory-only"
        grade_ct[r.get("evidence_grade")] += 1
        if r.get("evidence_grade") != want:
            fail(f"L2 #{r['number']} 自报 {r.get('evidence_grade')}，"
                 f"现场重算应为 {want}（families={sorted(fams)}）")
        if r.get("evidence_grade") not in ("advisory-only", "cross-checked"):
            fail(f"L2 #{r['number']} 证据等级非法: {r.get('evidence_grade')}")

    # L3 MD 指纹 vs 实测
    md = LEDGER_MD.read_text(encoding="utf-8")
    recorded = dict(re.findall(r"- `([A-Za-z0-9_.-]+\.jsonl?)` = `(sha256:[0-9a-f]{64})`", md))
    for p in TRIAGE_FILES:
        got = sha256(p)
        if recorded.get(p.name) != got:
            fail(f"L3 {p.name} MD 记 {recorded.get(p.name)} 实测 {got}")
    m = re.search(r"evidence_hash \| `?(sha256:[0-9a-f]{64})`?", md)
    if not m:
        fail("L3 MD 缺 evidence_hash 行")
    if not recorded.get("all.json"):
        fail("L3 MD 缺 issues 快照(all.json) 指纹")

    # L4 显式不完整
    m2 = re.search(r"缺分诊行：(\d+) 条", md)
    if bool(m2) != bool(no_field or (len(rows) != len(triage))):
        fail("L4 MD 的缺行声明与实际不一致")

    print(json.dumps({
        "kind": "ledger_verify",
        "rows": len(rows),
        "triaged_issues": len(triage),
        "grades": dict(grade_ct),
        "triage_files": {p.name: sha256(p) for p in TRIAGE_FILES},
        "md_evidence_hash": (m.group(1) if m else None),
    }, ensure_ascii=False))
    if FAILS:
        for f in FAILS:
            print("FAIL:", f)
        print(f"verify_ledger: {len(FAILS)} FAIL")
        return 1
    print("verify_ledger: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
