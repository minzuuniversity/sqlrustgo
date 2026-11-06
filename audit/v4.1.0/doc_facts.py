#!/usr/bin/env python3
"""P2-1 文档事实面：doc_stamp / doc_freshness / doc_path_claim_check / gen_doc_index。

背景（整改方案 §P2-1）：把 BidSystem 那套「文档事实面」接过来，
目标两条：**悬空路径 0**、**生成物「写盘口径」==「核对口径」**。

口径教训（BidSystem SF-08 家族，也是本工具存在的理由）：
    生成物里放进墙钟时间 ⇒ 每次重新生成都不同 ⇒ 逐字节比对**必然永远红**，
    判据于是变成摆设。所以：

    * 索引里**没有任何时间戳**；
    * 文档的「版本」用**内容哈希**表示，且哈希**排除 stamp 行本身**
      （否则写入 stamp 会改变哈希 ⇒ 自指 ⇒ 永远红）；
    * 「新鲜度」判的是 **stamp 与内容是否一致**，不是「距今多久」——
      距今多久是墙钟，属于展示域，不属于复现比对域。

子命令：
    index            生成文档索引（写 docs/releases/v4.1.0/DOC_INDEX.jsonl）
    check           重生成索引到临时文件并与已提交索引**逐字节**比对；
                    同时校验 stamp 一致性；并把既有门禁
                    check_docs_pointer_paths.sh / check_docs_links.sh 接进来
    stamp           报告/写入文档 stamp（内容哈希，不含时间戳）
    freshness       报告新鲜度（stamp 是否与内容一致；另附 git 提交日期，仅供展示）

指针检查**不重新实现**：既有门禁 scripts/gate/check_docs_pointer_paths.sh
已经实现了「回引号路径 + 已知扩展名 + 基线记账」这套规则，
一个判据只能有一份实现 —— 本工具只负责调用它并汇总裁决。

退出码：0 = 全部成立；1 = 有不成立项；2 = 用法错
本工具放在 audit/v4.1.0/：探针，不是门禁，不进 STAGE_CONFIG，分母 168 不动。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCOPE = REPO / "docs" / "releases" / "v4.1.0"
INDEX = SCOPE / "DOC_INDEX.jsonl"

STAMP_RE = re.compile(r"^<!-- doc-stamp: sha256=([0-9a-f]{64}) -->\n?", re.M)
# 墙钟时间：任何「生成物里都不许有」的形状。判据本身要能抓住它。
WALLCLOCK_RES = [
    re.compile(r"\b(19|20)\d{2}-\d{2}-\d{2}T\d{2}:\d{2}"),
    re.compile(r"\b(19|20)\d{2}-\d{2}-\d{2}\b"),
    re.compile(r"\bDate:\s*(Mon|Tue|Wed|Thu|Fri|Sat|Sun)\b"),
    re.compile(r"\bupdated_at\b"),
]
LINK_GATES = [
    ("docs_pointer_paths", REPO / "scripts" / "gate" / "check_docs_pointer_paths.sh"),
    ("docs_links", REPO / "scripts" / "gate" / "check_docs_links.sh"),
]


def docs_in_scope() -> list[Path]:
    return sorted(SCOPE.glob("*.md"))


def strip_stamp(text: str) -> str:
    return STAMP_RE.sub("", text)


def doc_hash(path: Path) -> str:
    """内容哈希，**排除 stamp 行** —— 写盘口径与核对口径必须一致。"""
    raw = path.read_text(encoding="utf-8", errors="replace")
    return hashlib.sha256(strip_stamp(raw).encode("utf-8")).hexdigest()


def read_stamp(path: Path) -> str | None:
    m = STAMP_RE.search(path.read_text(encoding="utf-8", errors="replace"))
    return m.group(1) if m else None


def build_index() -> list[dict]:
    """索引条目。**故意不含任何时间戳**（SF-08 口径）。"""
    rows = []
    for p in docs_in_scope():
        raw = p.read_text(encoding="utf-8", errors="replace")
        stamp = read_stamp(p)
        h = doc_hash(p)
        rows.append({
            "file": p.name,
            "bytes": len(raw.encode("utf-8")),
            "lines": raw.count("\n"),
            "sha256": h,
            "stamped": stamp is not None,
            "stamp_ok": stamp == h,
        })
    return rows


def render_index(rows: list[dict]) -> str:
    return "".join(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n" for r in rows)


def cmd_index(argv: list[str]) -> int:
    rows = build_index()
    text = render_index(rows)
    out = Path(argv[0]) if argv else INDEX
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8")
    print(json.dumps({"kind": "doc_index", "path": str(out), "docs": len(rows),
                      "stamped": sum(1 for r in rows if r["stamped"]),
                      "stamp_ok": sum(1 for r in rows if r["stamp_ok"])},
                     ensure_ascii=False))
    return 0


def cmd_check(argv: list[str]) -> int:
    fails: list[str] = []
    # 1) 写盘口径 == 核对口径：重生成到临时文件，逐字节比对
    rows = build_index()
    fresh = render_index(rows)
    committed = INDEX.read_text(encoding="utf-8") if INDEX.is_file() else None
    if committed is None:
        fails.append(f"没有已提交索引 {INDEX} —— 先 index 生成")
    elif committed != fresh:
        fails.append(f"索引与重生成结果不一致（{INDEX.name}）："
                     "生成物「写盘口径」必须等于「核对口径」")
    # 2) SF-08 自护：索引里不许有墙钟时间
    for rx in WALLCLOCK_RES:
        if rx.search(fresh):
            fails.append(f"索引里出现墙钟时间形状 {rx.pattern!r} —— "
                         "墙钟不得进复现比对域")
    # 3) stamp 新鲜度：stamp 必须等于当前内容哈希
    stale = [r["file"] for r in rows if r["stamped"] and not r["stamp_ok"]]
    for f in stale:
        fails.append(f"stamp 与内容不一致（内容改了但没重新 stamp）：{f}")
    # 4) 接既有门禁：指针与链接两条既有判据必须成立
    gate_results = []
    for name, script in LINK_GATES:
        if not script.is_file():
            gate_results.append({"gate": name, "status": "NOT_VERIFIED",
                                 "detail": f"{script.name} 不存在"})
            continue
        p = subprocess.run(["bash", str(script)], cwd=str(REPO),
                           capture_output=True, text=True, timeout=900)
        gate_results.append({"gate": name, "status": "PASS" if p.returncode == 0 else "FAIL",
                             "exit_code": p.returncode})
        if p.returncode != 0:
            fails.append(f"既有门禁 {name} 不成立（exit {p.returncode}）")
    print(json.dumps({"kind": "doc_facts_check",
                      "docs": len(rows),
                      "stamped": sum(1 for r in rows if r["stamped"]),
                      "stamp_ok": sum(1 for r in rows if r["stamp_ok"]),
                      "index_bytes": len(fresh.encode("utf-8")),
                      "gates": gate_results,
                      "fails": len(fails)}, ensure_ascii=False))
    for f in fails:
        print("FAIL:", f)
    if fails:
        print(f"doc_facts: {len(fails)} FAIL")
        return 1
    print("doc_facts: OK")
    return 0


def cmd_stamp(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(prog="doc_facts.py stamp")
    ap.add_argument("--write", action="store_true",
                    help="把 stamp 写进文档（默认只报告）")
    ap.add_argument("--file", action="append", help="只处理指定文件（可重复）")
    args = ap.parse_args(argv)

    targets = docs_in_scope()
    if args.file:
        names = set(args.file)
        targets = [p for p in targets if p.name in names]
    written = 0
    rows = []
    for p in targets:
        h = doc_hash(p)
        cur = read_stamp(p)
        if cur == h:
            continue
        rows.append({"file": p.name, "current": cur, "expected": h,
                     "action": "写入" if args.write else "待写入"})
        if args.write:
            raw = p.read_text(encoding="utf-8", errors="replace")
            body = strip_stamp(raw)
            marker = f"<!-- doc-stamp: sha256={h} -->\n"
            p.write_text(marker + body, encoding="utf-8")
            written += 1
    print(json.dumps({"kind": "doc_stamp", "scanned": len(targets),
                      "need_stamp": len(rows), "written": written,
                      "mode": "write" if args.write else "report",
                      "detail": rows[:20]}, ensure_ascii=False))
    return 0


def cmd_freshness(argv: list[str]) -> int:
    """新鲜度：判的是「stamp 与内容是否一致」（复现域），不是「距今多久」（展示域）。"""
    rows = []
    for p in docs_in_scope():
        cur = read_stamp(p)
        h = doc_hash(p)
        # git 提交日期只作展示，不进比对域
        try:
            d = subprocess.run(["git", "-C", str(REPO), "log", "-1", "--format=%cI",
                                "--", str(p.relative_to(REPO))],
                               capture_output=True, text=True, timeout=30).stdout.strip()
        except Exception:
            d = ""
        rows.append({"file": p.name, "stamped": cur is not None,
                     "fresh": cur == h if cur else None,
                     "last_commit": d})
    unstamped = [r["file"] for r in rows if not r["stamped"]]
    stale = [r["file"] for r in rows if r["stamped"] and not r["fresh"]]
    print(json.dumps({"kind": "doc_freshness", "docs": len(rows),
                      "unstamped": len(unstamped), "stale": len(stale),
                      "unstamped_files": unstamped[:20],
                      "stale_files": stale[:20]}, ensure_ascii=False))
    return 1 if stale else 0


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0 if argv else 2
    fn = {"index": cmd_index, "check": cmd_check, "stamp": cmd_stamp,
          "freshness": cmd_freshness}[argv[0]]
    return fn(argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
