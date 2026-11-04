#!/usr/bin/env python3
# scripts/gate/expert_triage.py
#
# 用专家梯对 Gitea open issues 做**廉价分诊**（非权威），产出 Issue 台账的机器行。
#
# 边界（重要）：
#   - 分诊输出是 Soft Gate：它只回答"这条 issue 属于哪个功能域、能不能自动化、
#     它破坏了哪条声称的功能"，**不回答**"这条 issue 是否已修好"。
#   - 判定 issue 是否解决仍走 ISSUE_CLOSING_VERIFICATION.md 五步 + PR 关联。
#   - 席位缺席不补猜：分诊失败的行记 triage_error，台账里标 needs_triage_retry。
#
# Usage:
#   python3 scripts/gate/expert_triage.py --input /tmp/srg-issues/full/all.json \
#       --seat deepseek-chat --out /tmp/srg-issues/triage.jsonl [--limit N] [--only <ids>]
#
# Exit 0 = 每条待分诊 issue 都得到可解析结论; 2 = 至少一条未得到结论(INCOMPLETE)。

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def load_ladder():
    path = REPO_ROOT / "scripts" / "gate" / "expert_ladder.py"
    spec = importlib.util.spec_from_file_location("expert_ladder", path)
    mod = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod

TRIAGE_CONTRACT = (
    "只输出**单个 JSON 对象**，不要其他文字：\n"
    "{"
    '"domain":"gate|ci-infra|test-validity|coverage|perf-tpch|storage-wal-crash|backup-restore|'
    'sql-semantics|vector-index|conn-lifecycle|docs-claims|release-stage|security|tooling|other",'
    '"breaks_claim":"该 issue 破坏了哪一条被声称的功能/契约，<=40字",'
    '"automatable":"auto|auto-needs-infra|manual-host|needs-human-decision",'
    '"blast":"small|medium|large",'
    '"fix_shape":"one-line|localized|cross-module|new-fixture|infra-config",'
    '"dup_of":整数或null'
    "}"
)

SYS = ("你在为一个 Rust 数据库项目 SQLRustGo 的 v4.1.0 开发线做缺陷分诊。"
       "该项目已发布 v4.0.0 但 GA 被撤销，v4.1.0 处于 ALPHA。"
       "分诊目标：判断这条 issue 影响哪条对外声称的能力、修复需要什么条件。"
       "不要提出补丁方案，不要臆测未在正文中出现的事实。")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", required=True)
    ap.add_argument("--seat", default="deepseek-chat")
    ap.add_argument("--out", required=True)
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--only", default="", help="comma separated issue numbers")
    ap.add_argument("--max-chars", type=int, default=2600)
    ap.add_argument("--max-tokens", type=int, default=300)
    ap.add_argument("--resume", action="store_true", help="skip ids already present in --out")
    args = ap.parse_args()

    L = load_ladder()
    seats = L.load_seats()
    seat = seats.get(args.seat)
    if not seat:
        print(f"unknown seat {args.seat}", file=sys.stderr)
        return 1

    issues = json.loads(Path(args.input).read_text(encoding="utf-8"))
    if args.only:
        keep = {int(x) for x in args.only.split(",") if x.strip()}
        issues = [i for i in issues if i["number"] in keep]
    if args.limit:
        issues = issues[: args.limit]

    outp = Path(args.out)
    outp.parent.mkdir(parents=True, exist_ok=True)
    done: set[int] = set()
    if args.resume and outp.exists():
        for line in outp.read_text(encoding="utf-8").splitlines():
            try:
                done.add(json.loads(line)["number"])
            except Exception:
                pass

    fh = open(outp, "a", encoding="utf-8")
    n_ok = n_bad = 0
    for idx, iss in enumerate(issues, 1):
        if iss["number"] in done:
            continue
        body = (iss.get("body") or "")[: args.max_chars]
        prompt = (
            f"Issue #{iss['number']}\n标题：{iss['title']}\n"
            f"标签：{','.join(iss.get('labels') or [])}\n正文（截断）：\n{body}\n\n{TRIAGE_CONTRACT}"
        )
        rec = L.ask(seat, prompt, max_tokens=args.max_tokens, system=SYS)
        parsed = None
        if rec.get("ok"):
            try:
                parsed = json.loads(rec["content"].strip().strip("`").removeprefix("json").strip())
            except Exception:
                fence = __import__("re").search(r"\{.*\}", rec["content"], __import__("re").S)
                if fence:
                    try:
                        parsed = json.loads(fence.group(0))
                    except Exception:
                        parsed = None
        row = {
            "number": iss["number"],
            "title": iss["title"],
            "labels": iss.get("labels") or [],
            "triaged_by": {"seat": seat.id, "family": seat.family, "model": seat.model},
            "prompt_sha256": rec.get("prompt_sha256"),
            "output_sha256": rec.get("output_sha256"),
            "latency_s": rec.get("latency_s"),
            "triaged_at": rec.get("requested_at"),
            "triage": parsed,
            "triage_ok": parsed is not None,
        }
        if parsed is None:
            row["triage_error"] = rec.get("error") or "unparseable"
            n_bad += 1
        else:
            n_ok += 1
        fh.write(json.dumps(row, ensure_ascii=False) + "\n")
        fh.flush()
        print(f"[{idx}/{len(issues)}] #{iss['number']} "
              f"{'ok' if parsed else 'ABSTAIN'} domain={(parsed or {}).get('domain','-')} "
              f"auto={(parsed or {}).get('automatable','-')} {rec.get('latency_s')}s", flush=True)
    fh.close()
    print(f"\nTRIAGE: ok={n_ok} abstain={n_bad} -> {outp}")
    return 0 if n_bad == 0 else 2


if __name__ == "__main__":
    sys.exit(main())
