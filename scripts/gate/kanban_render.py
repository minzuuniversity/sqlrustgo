#!/usr/bin/env python3
"""kanban_render.py — 把 KANBAN_<STAGE>.jsonl 渲染进 KANBAN_<STAGE>.md 的台账段。

单一来源纪律（docs/governance/STAGE_KANBAN.md §3）：
  台账的权威副本是 JSONL；Markdown 里的台账表与计数行是**生成物**。

用法：
  kanban_render.py render <kanban.md>      # 把 JSONL 写进 md 的 LEDGER 标记段
  kanban_render.py check  <kanban.md>      # 只核对；漂移 -> 退出码 1
  kanban_render.py counts <kanban.md>      # 打印计数 JSON（供其它工具引用）

退出码只有三种（严格三态，fail-closed）：
  0 = 一致 / 渲染成功
  1 = 不一致（check 模式发现漂移），或输入不合法（缺文件、JSONL 结构坏、标记缺失）
  2 = 保留：本脚本不使用，避免与"无法判定"混淆
状态词汇被严格限定为五种，任何越界词汇都是错误，不是警告。
"""
import argparse
import json
import os
import sys

BEGIN = "<!-- LEDGER:BEGIN (generated from {src} — 不要手改本段) -->"
END = "<!-- LEDGER:END -->"
COUNT_PREFIX = "- 台账计数："
STATUSES = ("未开始", "进行中", "待复核", "完成", "阻塞")
REQUIRED = ("id", "task", "status", "acceptance", "evidence", "blocked_by")


class InputError(Exception):
    """输入不合法：等价于'没能核对'，绝不返回 0。"""


def load_jsonl(md_path):
    src = os.path.splitext(md_path)[0] + ".jsonl"
    if not os.path.isfile(md_path):
        raise InputError("missing markdown: %s" % md_path)
    if not os.path.isfile(src):
        raise InputError("missing machine-readable ledger: %s" % src)
    rows = []
    for lineno, line in enumerate(open(src, encoding="utf-8"), 1):
        if not line.strip():
            continue
        try:
            obj = json.loads(line)
        except ValueError as exc:
            raise InputError("%s:%d bad json: %s" % (src, lineno, exc))
        for key in REQUIRED:
            if key not in obj:
                raise InputError("%s:%d missing field %r" % (src, lineno, key))
        if obj["status"] not in STATUSES:
            raise InputError("%s:%d status %r not in %s"
                             % (src, lineno, obj["status"], list(STATUSES)))
        rows.append(obj)
    if not rows:
        raise InputError("%s has zero rows" % src)
    ids = [r["id"] for r in rows]
    if len(set(ids)) != len(ids):
        raise InputError("%s has duplicate ids" % src)
    # 无证据的"完成"不成立（AFP §1）：自动降级为待复核，而不是放行
    for r in rows:
        if r["status"] == "完成" and not (r["evidence"] or "").strip():
            r["status"] = "待复核"
            r["status_note"] = "auto-downgraded: 完成 without evidence"
    return os.path.basename(src), rows


def render_block(src, rows):
    has_origin = any(r.get("origin") for r in rows)
    head = ["ID", "任务", "状态", "验收判据（可证伪）", "证据", "阻塞方"]
    if has_origin:
        head.append("来源")

    def cell(text):
        return str(text).replace("|", "\\|").replace("\n", " ") or "-"

    lines = ["| " + " | ".join(head) + " |",
             "|" + "---|" * len(head)]
    for r in rows:
        row = [r["id"], r["task"], r["status"], r["acceptance"],
               r["evidence"], r["blocked_by"] or "-"]
        if has_origin:
            row.append(r.get("origin") or "-")
        lines.append("| " + " | ".join(cell(v) for v in row) + " |")
    counts = " / ".join("%s %d" % (s, sum(1 for r in rows if r["status"] == s))
                        for s in STATUSES if any(r["status"] == s for r in rows))
    count_line = "%s%s / 合计 %d" % (COUNT_PREFIX, counts, len(rows))
    return (BEGIN.format(src=src) + "\n\n" + count_line + "\n\n"
            + "\n".join(lines) + "\n" + END)


PLACEHOLDER = "<!-- LEDGER:PLACEHOLDER -->"


def _marker_span(md_text):
    """Return (start, end_after) of the LEDGER block, tolerating any src inside BEGIN."""
    import re
    ph = md_text.find(PLACEHOLDER)
    if ph >= 0:
        return ph, ph + len(PLACEHOLDER)
    m = re.search(r"^<!-- LEDGER:BEGIN .*?-->$", md_text, re.M)
    if not m:
        raise InputError("markdown has neither LEDGER:PLACEHOLDER nor a"
                         " '<!-- LEDGER:BEGIN ... -->' marker;"
                         " refusing to guess where the ledger goes")
    e = md_text.find(END, m.end())
    if e < 0:
        raise InputError("LEDGER:BEGIN marker has no matching LEDGER:END")
    return m.start(), e + len(END)


def splice(md_text, block):
    start, end = _marker_span(md_text)
    return md_text[:start] + block + md_text[end:]


def md_table_rows(md_text):
    """Extract the ledger table currently present in the markdown."""
    start, end = _marker_span(md_text)
    import re
    m = re.match(r"^<!-- LEDGER:BEGIN .*?-->$", md_text[start:], re.M)
    seg_start = start + (m.end() if m else 0)
    seg = md_text[seg_start:end]
    rows = {}
    for line in seg.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 6 or cells[0] in ("ID",) or set(cells[0]) <= set("-"):
            continue
        rows[cells[0]] = tuple(cells[:6])
    count_line = next((l for l in seg.splitlines()
                       if l.strip().startswith(COUNT_PREFIX)), None)
    return rows, count_line


def main(argv=None):
    ap = argparse.ArgumentParser(prog="kanban_render.py")
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("render", "check", "counts"):
        p = sub.add_parser(name)
        p.add_argument("md")
    args = ap.parse_args(argv)

    try:
        src, rows = load_jsonl(args.md)
    except InputError as exc:
        print("INCOMPLETE: %s" % exc, file=sys.stderr)
        return 1

    if args.cmd == "counts":
        print(json.dumps({"rows": len(rows),
                          "by_status": {s: sum(1 for r in rows if r["status"] == s)
                                        for s in STATUSES
                                        if any(r["status"] == s for r in rows)}},
                         ensure_ascii=False))
        return 0

    try:
        block = render_block(src, rows)
        md_text = open(args.md, encoding="utf-8").read()
    except (InputError, OSError) as exc:
        print("INCOMPLETE: %s" % exc, file=sys.stderr)
        return 1

    if args.cmd == "render":
        try:
            out = splice(md_text, block)
        except InputError as exc:
            print("INCOMPLETE: %s" % exc, file=sys.stderr)
            return 1
        with open(args.md, "w", encoding="utf-8") as fh:
            fh.write(out)
        print("rendered %d rows from %s into %s" % (len(rows), src, args.md))
        return 0

    # check
    try:
        want_rows, want_count = md_table_rows(block)
        have_rows, have_count = md_table_rows(md_text)
    except InputError as exc:
        print("INCOMPLETE: %s" % exc, file=sys.stderr)
        return 1
    problems = []
    if set(want_rows) != set(have_rows):
        problems.append("id set differs: jsonl-only=%s md-only=%s"
                        % (sorted(set(want_rows) - set(have_rows)),
                           sorted(set(have_rows) - set(want_rows))))
    for rid in sorted(set(want_rows) & set(have_rows)):
        if want_rows[rid] != have_rows[rid]:
            problems.append("row %s differs\n    jsonl: %s\n    md:    %s"
                            % (rid, want_rows[rid], have_rows[rid]))
    if want_count != have_count:
        problems.append("count line differs\n    jsonl: %r\n    md:    %r"
                        % (want_count, have_count))
    if problems:
        print("FAIL: %s drifted from %s (%d problems)"
              % (os.path.basename(args.md), src, len(problems)), file=sys.stderr)
        for p in problems:
            print("  - " + p, file=sys.stderr)
        print("fix: python3 scripts/gate/kanban_render.py render %s" % args.md,
              file=sys.stderr)
        return 1
    print("OK: %s ledger matches %s (%d rows)"
          % (os.path.basename(args.md), src, len(rows)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
