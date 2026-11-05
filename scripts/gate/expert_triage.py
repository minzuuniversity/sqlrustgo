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


def label_names(labels) -> list:
    """Gitea labels 既可能是字符串也可能是对象（取决于 API 变体）⇒ 统一成名字。"""
    return [l["name"] if isinstance(l, dict) else str(l) for l in (labels or [])]


BATCH_CONTRACT = (
    "对上面**每一条** issue 各输出一个 JSON 对象，组成一个 JSON 数组；"
    "不得输出数组以外的任何文字，不得合并 issue，不得省略编号。\n"
    "数组长度必须等于 issue 条数；每个对象必须带 \"number\" 字段（整数，原样回显编号）。\n"
    "对象字段同单条契约：\n" + TRIAGE_CONTRACT.removeprefix("只输出**单个 JSON 对象**，不要其他文字：\n")
)


def parse_batch_output(text: str, expected_numbers: list) -> tuple:
    """把席位返回的文本解析成 {number: triage_obj}，并给出缺失编号。

    判据可证伪：少一个对象 ⇒ missing 必须非空；多一个不认识的编号 ⇒ 必须被丢弃且计入 extra。
    绝不"补齐"缺失项 —— 补齐就是编造（AGENTS 反虚假执行纪律）。
    """
    import json as _json
    import re as _re
    raw = (text or "").strip().strip("`")
    if raw.lower().startswith("json"):
        raw = raw[4:].strip()
    objs = None
    try:
        objs = _json.loads(raw)
    except ValueError:
        arr = _re.search(r"\[.*\]", raw, _re.S)
        if arr:
            try:
                objs = _json.loads(arr.group(0))
            except ValueError:
                objs = None
    if objs is None:
        # 退而求其次：逐个抓 {...} 对象（席位有时会带解释性前后缀）
        objs = []
        for m in _re.finditer(r"\{(?:[^{}]|\{[^{}]*\})*\}", raw, _re.S):
            try:
                objs.append(_json.loads(m.group(0)))
            except ValueError:
                continue
    if isinstance(objs, dict):
        objs = [objs]
    got, extra = {}, []
    for o in objs:
        if not isinstance(o, dict):
            continue
        num = o.get("number")
        try:
            num = int(num)
        except (TypeError, ValueError):
            continue
        if num not in set(expected_numbers):
            extra.append(num)
            continue
        payload = {k: v for k, v in o.items() if k != "number"}
        got[num] = payload
    missing = [n for n in expected_numbers if n not in got]
    return got, missing, extra


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
    ap.add_argument("--batch", type=int, default=0,
                    help=">0 时按 N 条/次分诊（省席位轮次；输出仍是每 issue 一行）")
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

    def write_row(num, title, labels, rec, parsed, mode):
        nonlocal n_ok, n_bad
        row = {
            "number": num,
            "title": title,
            "labels": labels,
            "triaged_by": {"seat": seat.id, "family": seat.family, "model": seat.model},
            "prompt_sha256": rec.get("prompt_sha256"),
            "output_sha256": rec.get("output_sha256"),
            "latency_s": rec.get("latency_s"),
            "triaged_at": rec.get("requested_at"),
            "mode": mode,
            "triage": parsed,
            "triage_ok": parsed is not None,
        }
        if parsed is None:
            row["triage_error"] = rec.get("error") or "unparseable_or_missing_from_batch"
            n_bad += 1
        else:
            n_ok += 1
        fh.write(json.dumps(row, ensure_ascii=False) + "\n")
        fh.flush()

    if args.batch > 0:
        pending = [i for i in issues if i["number"] not in done]
        by_num = {i["number"]: i for i in pending}
        for bstart in range(0, len(pending), args.batch):
            chunk = pending[bstart:bstart + args.batch]
            nums = [i["number"] for i in chunk]
            parts = []
            for i in chunk:
                body = (i.get("body") or "")[: args.max_chars]
                parts.append(f"Issue #{i['number']}\n标题：{i['title']}\n"
                             f"标签：{','.join(label_names(i.get('labels')))}\n正文（截断）：\n{body}")
            prompt = "\n\n---\n\n".join(parts) + "\n\n" + BATCH_CONTRACT
            rec = L.ask(seat, prompt, max_tokens=max(args.max_tokens, 160), system=SYS)
            got, missing, extra = ({}, nums, [])
            if rec.get("ok"):
                got, missing, extra = parse_batch_output(rec["content"], nums)
                if extra:
                    print(f"  note: 席位返回了范围外编号 {extra}（已丢弃）", flush=True)
            for num in nums:
                iss = by_num[num]
                write_row(num, iss["title"], label_names(iss.get("labels")),
                          rec, got.get(num), f"batch-{args.batch}")
                print(f"[{num}] {'ok' if got.get(num) else 'ABSTAIN'} "
                      f"domain={(got.get(num) or {}).get('domain','-')} "
                      f"auto={(got.get(num) or {}).get('automatable','-')}", flush=True)
            print(f"  batch {bstart//args.batch + 1}: 请求 {len(nums)} 得到 {len(got)} "
                  f"缺 {len(missing)} 延迟 {rec.get('latency_s')}s", flush=True)
        fh.close()
        print(f"\nTRIAGE(batch={args.batch}): ok={n_ok} abstain={n_bad} -> {outp}")
        return 0 if n_bad == 0 else 2

    for idx, iss in enumerate(issues, 1):
        if iss["number"] in done:
            continue
        body = (iss.get("body") or "")[: args.max_chars]
        prompt = (
            f"Issue #{iss['number']}\n标题：{iss['title']}\n"
            f"标签：{','.join(label_names(iss.get('labels')))}\n正文（截断）：\n{body}\n\n{TRIAGE_CONTRACT}"
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
            "labels": label_names(iss.get("labels")),
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
