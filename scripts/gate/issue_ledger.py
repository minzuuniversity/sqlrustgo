#!/usr/bin/env python3
# scripts/gate/issue_ledger.py
#
# 把 Gitea open issues 与专家梯分诊结果合并成"issue 台账"（看板 IL-1 的产物）。
#
# 边界（写进产物本身，防止被误读成裁定）：
#   - 分诊是 Soft Gate：它回答"属于哪个功能域 / 破坏了哪条声称 / 能不能自动化 / 修复形状"，
#     **不回答**"是否已修好"。销账仍走 ISSUE_CLOSING_VERIFICATION.md + PR 关联。
#   - lineage 单一时产物标 `advisory-only`；只有 credited lineage >= 2 才允许写 cross-checked。
#
# 退出码：0=全部 issue 都有分诊行; 1=有 issue 缺分诊行(台账不完整); 2=输入不可用(INCOMPLETE)。

import argparse
import collections
import hashlib
import json
import os
import sys

STATUS_WORDS = ("advisory-only", "cross-checked")


def load_json(path):
    """读 issues 快照。**接受 JSON 数组，也接受 JSONL**。

    为什么要 JSONL：仓内禁 `*.json`（交接提示词 §4），而台账要记输入指纹
    —— 输入必须是**仓内的**文件，否则「重生成后指纹一致」这件事无法复现
    （旧台账的 issues 输入是个临时文件，指纹对不上任何仓内产物）。
    """
    if not os.path.isfile(path):
        raise SystemExit(2)
    with open(path, encoding="utf-8") as fh:
        raw = fh.read()
    stripped = raw.lstrip()
    if stripped.startswith("["):
        return json.loads(raw)
    return [json.loads(ln) for ln in raw.splitlines() if ln.strip()]


def load_jsonl(paths):
    """读入分诊 jsonl（可多份），返回 number -> [记录]。

    多份的意义：同一 issue 只有被 >=2 个不同 family 的血统分别分诊过，
    那条台账行才有资格写 cross-checked；单血统一律 advisory-only。
    """
    if isinstance(paths, str):
        paths = [paths]
    rows = {}
    for path in paths:
        if not os.path.isfile(path):
            raise SystemExit(2)
        for lineno, line in enumerate(open(path, encoding="utf-8"), 1):
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except ValueError as exc:
                print("INCOMPLETE: %s:%d bad json: %s" % (path, lineno, exc), file=sys.stderr)
                raise SystemExit(2)
            rows.setdefault(int(obj["number"]), []).append(obj)
    return rows


def family_of(rec):
    return ((rec or {}).get("triaged_by") or {}).get("family") or "?"


def names(labels):
    """Gitea 的 labels 既可能是字符串也可能是对象（取决于 API 变体）⇒ 统一成名字。"""
    return [l["name"] if isinstance(l, dict) else str(l) for l in (labels or [])]


def first(prefix, labels):
    v = [l for l in names(labels) if l.startswith(prefix)]
    return v[0] if v else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--issues", required=True, help="all.json from Gitea (type=issues)")
    ap.add_argument("--triage", nargs="+", required=True,
                    help="分诊 jsonl（expert_triage.py 产物），可多份；同一 issue 需 >=2 个不同 family 才算 cross-checked")
    ap.add_argument("--credited-lineages", default="",
                    help='comma separated, e.g. "gpt,qwen"；来自 expert_ladder health 实测')
    ap.add_argument("--source-run", default="", help="本次生成的 run id（AFP 5 字段之一）")
    ap.add_argument("--source-agent", default="", help="生成者身份（AFP 5 字段之一）")
    ap.add_argument("--out-jsonl", required=True)
    ap.add_argument("--out-md", required=True)
    args = ap.parse_args()

    try:
        issues = load_json(args.issues)
        triage = load_jsonl(args.triage)
    except SystemExit:
        print("INCOMPLETE: cannot read --issues/--triage inputs", file=sys.stderr)
        return 2
    if not issues:
        print("INCOMPLETE: zero issues loaded (refusing to render an empty ledger)",
              file=sys.stderr)
        return 2

    lineages = [x.strip() for x in args.credited_lineages.split(",") if x.strip()]
    ladder_ready = len(set(lineages)) >= 2

    rows, missing = [], []
    graded = 0
    for iss in issues:
        num = int(iss["number"])
        recs = [r for r in triage.get(num, []) if ((r.get("triage") or {}) and r.get("triage_ok") is not False)]
        famset = {family_of(r) for r in recs}
        # 行级证据等级：单血统分诊只能 advisory-only，即便 ladder 本身有两个 credited 血统
        grade = "cross-checked" if (ladder_ready and len(famset) >= 2) else "advisory-only"
        if grade == "cross-checked":
            graded += 1
        t = recs[0] if recs else None
        tp = (t or {}).get("triage") or {}
        if not tp:
            missing.append(num)
        if not famset:
            famset = {family_of(r) for r in triage.get(num, [])}
        rows.append({
            "number": num,
            "title": iss["title"],
            "url": iss.get("html_url") or iss.get("url"),
            "priority": first("priority/", iss.get("labels") or []),
            "cost": first("cost/", iss.get("labels") or []),
            "cap": (lambda v: v.split("/", 1)[1] if v and "/" in v else None)(
                first("cap/", iss.get("labels") or [])),
            "state_label": first("state/", iss.get("labels") or []),
            "created_at": iss.get("created_at"),
            "comments": iss.get("comments", 0),
            "domain": tp.get("domain"),
            "breaks_claim": tp.get("breaks_claim"),
            "automatable": tp.get("automatable"),
            "blast": tp.get("blast"),
            "fix_shape": tp.get("fix_shape"),
            "dup_of": tp.get("dup_of"),
            "triaged_by": (t or {}).get("triaged_by"),
            "evidence_grade": grade,
            "triage_families": sorted(famset),
            "credited_lineages": lineages,
        })

    inputs = [args.issues] + list(args.triage)
    input_hashes = {}
    for pth in inputs:
        blob = open(pth, "rb").read()
        input_hashes[os.path.basename(pth)] = "sha256:" + hashlib.sha256(blob).hexdigest()
    evidence_hash = "sha256:" + hashlib.sha256(
        "".join(input_hashes[k] for k in sorted(input_hashes)).encode("utf-8")
    ).hexdigest()

    with open(args.out_jsonl, "w", encoding="utf-8") as fh:
        for r in sorted(rows, key=lambda r: (r["number"])):
            r["ledger_run"] = args.source_run or "(unstamped)"
            fh.write(json.dumps(r, ensure_ascii=False) + "\n")

    # Markdown view
    import datetime as _dt
    now_iso = _dt.datetime.now(_dt.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    grade_ct = collections.Counter(r["evidence_grade"] for r in rows)
    fams = sorted({f for r in rows for f in r["triage_families"]})
    pr = collections.Counter(r["priority"] or "priority/(none)" for r in rows)
    dm = collections.Counter(r["domain"] or "(未分诊)" for r in rows)
    au = collections.Counter(r["automatable"] or "(未分诊)" for r in rows)
    dup = [r for r in rows if r["dup_of"]]
    lines = []
    lines.append("# v4.1.0 open issue 台账（IL-1 产物）\n")
    lines.append("- 生成：`scripts/gate/issue_ledger.py`（本文件是生成物，手改会被下次生成覆盖）")
    lines.append("- 输入：Gitea `type=issues` open=%d；分诊行=%d（`expert_triage.py`）"
                 % (len(issues), len(triage)))
    lines.append("- 证据等级（行级）：%s；ladder credited lineage=%s；实际分诊席位血统=%s"
                 % (" / ".join("%s %d" % kv for kv in grade_ct.most_common()),
                    ",".join(lineages) or "(none)", ",".join(fams)))
    lines.append("- 规则：ladder 有两个 credited 血统 ≠ 本条被两个血统分别核对过。"
                 "只有同一 issue 被 >=2 个 family 分诊才升级 cross-checked")
    lines.append("- 分诊只回答：域 / 破坏了哪条声称 / 可否自动化 / 修复形状 / 疑似重复。"
                 "**不回答\"是否已修好\"**，销账仍走 PR 关联与关闭验证")
    lines.append("")
    lines.append("## 0. 证据字段（AFP 5 字段，ADR-014）\n")
    lines.append("| 字段 | 值 |")
    lines.append("|---|---|")
    lines.append("| source_agent | %s |" % (args.source_agent or "(unstamped)"))
    lines.append("| source_run | %s |" % (args.source_run or "(unstamped)"))
    lines.append("| timestamp | %s |" % now_iso)
    lines.append("| evidence_hash | %s |" % evidence_hash)
    lines.append("| conflict_resolution | 单席位行一律 advisory-only；"
                 "同一 issue 被 >=2 family 分诊才升 cross-checked（本次 %d 行升级） |" % graded)
    lines.append("")
    lines.append("输入指纹（可用 `shasum -a 256` 复现 evidence_hash）：")
    lines.append("")
    for k in sorted(input_hashes):
        lines.append("- `%s` = `%s`" % (k, input_hashes[k]))
    lines.append("")
    lines.append("复现输入快照（重新拉取得到新指纹，因 issue 状态会变）：")
    lines.append("")
    lines.append("- issues：`python3 scripts/gate/gov_probe.py issues --dump <path>"
                 "`（open issues 逐页 limit=50，dump 行自报 sha256）")
    lines.append("- 分诊：`python3 scripts/gate/expert_triage.py --input <issues dump> "
                 "--seat <seat> --out <jsonl> --batch N`（席位与延迟逐行记录在 jsonl）")
    if missing:
        lines.append("- ⚠️ 缺分诊行：%d 条：%s" % (len(missing),
                     ",".join("#%d" % n for n in missing[:20])))
    lines.append("")
    lines.append("## 1. 分布\n")
    lines.append("| 维度 | 分布 |")
    lines.append("|---|---|")
    lines.append("| 优先级 | %s |" % " / ".join("%s %d" % kv for kv in pr.most_common()))
    lines.append("| 功能域 | %s |" % " / ".join("%s %d" % kv for kv in dm.most_common()))
    lines.append("| 可自动化 | %s |" % " / ".join("%s %d" % kv for kv in au.most_common()))
    lines.append("| 疑似重复 | %d 条 |" % len(dup))
    lines.append("")
    lines.append("## 2. 台账\n")
    lines.append("| # | 优先级 | 域 | 标题 | 破坏的声称 | 可自动化 | 修复形状 | blast | dup |")
    lines.append("|---|---|---|---|---|---|---|---|---|")

    def cell(x):
        s = "" if x is None else str(x)
        return (s.replace("|", "\\|").replace("\n", " "))[:90] or "-"

    def key(r):
        order = {"priority/p0": 0, "priority/p1": 1, "priority/p2": 2}
        return (order.get(r["priority"], 3), -int(r["comments"]), r["number"])

    for r in sorted(rows, key=key):
        title = r["title"][:70]
        lines.append("| [#%d](%s) | %s | %s | %s | %s | %s | %s | %s | %s |" % (
            r["number"], r["url"], (r["priority"] or "-").replace("priority/", ""),
            r["domain"] or "-", title, cell(r["breaks_claim"]),
            (r["automatable"] or "-").replace("needs-", ""), cell(r["fix_shape"]),
            r["blast"] or "-", ("#%s" % r["dup_of"]) if r["dup_of"] else "-"))
    with open(args.out_md, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")

    print("ledger rows=%d triaged=%d missing=%d graded_crosschecked=%d "
          "advisory_only=%d -> %s"
          % (len(rows), len(triage), len(missing), graded,
             len(rows) - graded, args.out_md))
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
