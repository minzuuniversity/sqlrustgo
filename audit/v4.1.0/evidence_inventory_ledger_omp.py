# 台账侧证据清点器（live_rows / ledger / report）。
#
# 2026-11-06 合并说明：本文件原名 evidence_inventory.py，与另一条线
# （mcode-macmini/governance-kanban-expert-ladder）独立实现的同名 P1-5 工具撞名。
# 原名让给那条线的实现：它有配套的 evidence_inventory_mutations.py 与归档 run
# 20261106-evidinv-001，改名会让那次归档的 command 不可复现。本版本没有任何
# 归档 run 引用，故改名保留而非删除。两套实现互补：本版按台账对账，
# 彼版按文档声明分级（grade/verify）。原版正文未改动。
#!/usr/bin/env python3
"""P1-5: 把 v4.1.0 文档里的声明与证据 run 绑定，并让「未绑定的新声明」变红。

背景（整改方案 §1.2 / §1.4、看板 Q3）：
    `docs/releases/v4.1.0/*.md` 里有上千处 `PASS` / `全绿` / `✅`，
    但其中绝大多数**指不回任何已归档 run**。这就是「报告好看、一用就崩」的直接机制：
    文档写「通过」，却没有任何可复现的执行记录支撑它。

本工具做的事：
    1) 把每处声明按**所在小节**（markdown 标题分块）分级：
         bound   —— 同一小节内引用了已归档 run id 或 evidence 文档 ⇒ 可追到执行记录
         unbound —— 小节内没有任何 run/evidence 出处 ⇒ 「文档声称」
    2) 记账到基线台账（JSONL），并让**基线之上多出来的未绑定声明**变红。

设计口径（重要，别误读）：
    - 本工具**不追溯**历史存量：1073 处存量未绑定声明是既成事实，
      由 Owner 决定怎么补（ISSUE 侧），不靠改历史文档来「变绿」。
    - 本工具只卡**增量**：任何人新写一处没有出处的声明 ⇒ 红。
      这条判据的作用是止血，不是清算。
    - 减少声明（删掉吹过的话）永远不判红；只有增加未绑定声明才判红。

用法：
    evidence_inventory.py emit   [台账]            写出当前基线
    evidence_inventory.py check  [台账]            与基线比对；增量未绑定 ⇒ rc=1
    evidence_inventory.py report [台账]            打印分级汇总（不改任何东西）

台账默认 docs/releases/v4.1.0/CLAIMS_INVENTORY.jsonl。
本工具放在 audit/v4.1.0/ 下：它是探针/夹具，**不是门禁**，不进 STAGE_CONFIG。
"""
from __future__ import annotations

import argparse
import glob
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LEDGER = REPO / "docs" / "releases" / "v4.1.0" / "CLAIMS_INVENTORY.jsonl"
BINDINGS = REPO / "docs" / "releases" / "v4.1.0" / "CLAIMS_BINDINGS.jsonl"

# 与 gov_probe.py 完全同口径的声明记号 —— 必须一致，
# 否则本工具的 total 与 gov_probe 的 claims_hits 会对不上，两边都不可信。
CLAIM_RE = re.compile(r"PASS|全绿|✅")

# run id 的命名口径与 scripts/gate/check_audit_manifest.sh 的 RUN_RE 一致：
# iaia410-* 或 YYYYMMDD-*（归档 run 目录名），二者都在 audit/v4.1.0[/runs]/ 下。
# 注意后缀是 `-<slug>-<NNN>`（三位流水号，如 20261105-lb1-006）：
# 早先写成 `[0-9]{4}` 会把这类真 run 引用漏掉，bound 因此被系统性少算。
RUNID_RE = re.compile(
    r"(?:iaia410-[0-9A-Za-z._-]+|[0-9]{8}-[0-9A-Za-z._-]+)")
EVDOC_RE = re.compile(r"docs/releases/v4\.1\.0/evidence/[0-9A-Za-z._/-]+\.md")
# 必须指向**真正的 run 目录名**（iaia410-* 或 YYYYMMDD-*），
# 否则 `audit/v4.1.0/evidence_inventory.py` 这种工具路径会被误判成出处。
RUNDIR_RE = re.compile(
    r"audit/v4\.1\.0/(?:runs/)?(?:iaia410-[0-9A-Za-z._-]+|[0-9]{8}-[0-9A-Za-z._-]+)")

# ATX 标题：1~6 个 # 加空格。文档全部用 ATX 风格，setext 不做处理。
HEADING_RE = re.compile(r"(?m)^(?=#{1,6}\s)")


def sections(text: str) -> list[str]:
    """按 markdown 标题（含 setext 风格 `===`/`---` 下划线）切小节。"""
    return HEADING_RE.split(text)


def classify(path: Path) -> tuple[int, int, int]:
    """返回 (total, bound, unbound)；bound 要求同小节内有可指回的出处。"""
    text = path.read_text(encoding="utf-8", errors="replace")
    total = bound = 0
    for sec in sections(text):
        n = len(CLAIM_RE.findall(sec))
        if not n:
            continue
        total += n
        cited = bool(RUNID_RE.search(sec) or EVDOC_RE.search(sec)
                     or RUNDIR_RE.search(sec))
        if cited:
            bound += n
    return total, bound, total - bound


def live_rows() -> dict[str, dict]:
    rows: dict[str, dict] = {}
    for p in sorted(glob.glob("docs/releases/v4.1.0/*.md")):
        path = Path(p)
        total, bound, unbound = classify(path)
        if not total:
            continue
        rows[p] = {"file": p, "claims": total, "bound": bound, "unbound": unbound}
    return rows


def load_ledger(path: Path) -> dict[str, dict]:
    if not path.is_file():
        return {}
    out: dict[str, dict] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            r = json.loads(line)
            out[r["file"]] = r
    return out


def cmd_emit(argv: list[str]) -> int:
    out = Path(argv[0]) if argv else LEDGER
    rows = live_rows()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("".join(json.dumps(rows[k], ensure_ascii=False, sort_keys=True) + "\n"
                           for k in sorted(rows)), encoding="utf-8")
    print(json.dumps({"kind": "claims_inventory_emit", "path": str(out),
                      "files": len(rows),
                      "claims": sum(r["claims"] for r in rows.values()),
                      "bound": sum(r["bound"] for r in rows.values()),
                      "unbound": sum(r["unbound"] for r in rows.values())},
                     ensure_ascii=False))
    return 0


def cmd_check(argv: list[str]) -> int:
    path = Path(argv[0]) if argv else LEDGER
    base = load_ledger(path)
    if not base:
        print(f"FAIL: 没有台账 {path} —— 先 emit 基线，否则「新增」无从判定", file=sys.stderr)
        return 1
    live = live_rows()
    fails: list[str] = []

    for f, r in sorted(live.items()):
        b = base.get(f)
        if b is None:
            # 新文档：任何未绑定声明都不允许
            if r["unbound"]:
                fails.append(f"NEW-FILE {f} 带 {r['unbound']} 处未绑定声明"
                             f"（新写的声明必须同小节引用 run id 或 evidence 文档）")
            continue
        if r["unbound"] > b["unbound"]:
            fails.append(f"{f} 未绑定声明 {b['unbound']} -> {r['unbound']}"
                         f"（新增 {r['unbound'] - b['unbound']} 处无 run/evidence 出处）")

    print(json.dumps({"kind": "claims_inventory_check", "ledger": str(path),
                      "live_files": len(live), "ledger_files": len(base),
                      "live_claims": sum(r["claims"] for r in live.values()),
                      "live_bound": sum(r["bound"] for r in live.values()),
                      "live_unbound": sum(r["unbound"] for r in live.values()),
                      "ledger_unbound": sum(r["unbound"] for r in base.values()),
                      "fails": len(fails)}, ensure_ascii=False))
    for f in fails[:40]:
        print("FAIL:", f)
    if fails:
        print(f"claims_inventory: {len(fails)} FAIL")
        return 1
    print("claims_inventory: OK")
    return 0


def cmd_ledger(argv: list[str]) -> int:
    """写出「声明 ↔ run」绑定台账，给出 claims_ref（看板 P1-5 要的就是这个数）。

    落点是 docs/releases/v4.1.0/ 下的**单个 .jsonl 文件**，不新建目录：
    `audit/v4.1.0/*` 这个 glob 会把新增目录计进 gov_probe 的 audit_runs 指标，
    凭空多一个目录等于污染别人对账的分母；.jsonl 也不会被 docs410 的 *.md 计数吃进去。
    """
    out = Path(argv[0]) if argv else BINDINGS
    out.parent.mkdir(parents=True, exist_ok=True)
    rows = live_rows()
    all_recs: list[dict] = []
    hits = bound = sections_bound = 0
    for f in sorted(rows):
        text = Path(f).read_text(encoding="utf-8", errors="replace")
        recs = []
        for i, sec in enumerate(sections(text)):
            n = len(CLAIM_RE.findall(sec))
            if not n:
                continue
            hits += n
            runs = sorted({m.group(0) for m in RUNID_RE.finditer(sec)}
                          | {m.group(0) for m in RUNDIR_RE.finditer(sec)})
            evs = sorted({m.group(0) for m in EVDOC_RE.finditer(sec)})
            if not runs and not evs:
                continue
            bound += n
            sections_bound += 1
            recs.append({"file": f, "section": i, "claims": n,
                         "run_refs": runs[:20], "evidence_refs": evs[:20],
                         "excerpt": re.sub(r"\s+", " ", sec.strip())[:200]})
        if recs:
            all_recs.extend(recs)
    out.write_text("".join(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n"
                           for r in all_recs), encoding="utf-8")
    print(json.dumps({"kind": "claims_ledger", "path": str(out),
                      "claims_hits": hits, "claims_bound": bound,
                      "claims_unbound": hits - bound, "claims_ref": bound,
                      "bound_sections": sections_bound}, ensure_ascii=False))
    return 0


def cmd_report(argv: list[str]) -> int:
    path = Path(argv[0]) if argv else LEDGER
    rows = live_rows()
    tot = sum(r["claims"] for r in rows.values())
    bnd = sum(r["bound"] for r in rows.values())
    print(json.dumps({"kind": "claims_inventory_report", "files": len(rows),
                      "claims": tot, "bound": bnd, "unbound": tot - bnd,
                      "bound_pct": round(100.0 * bnd / tot, 2) if tot else 0.0},
                     ensure_ascii=False))
    worst = sorted(rows.values(), key=lambda r: -r["unbound"])[:12]
    for r in worst:
        print(f"{r['unbound']:5d} 未绑定 / {r['claims']:5d}  {r['file']}")
    return 0


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] not in ("emit", "check", "report", "ledger"):
        print("usage: evidence_inventory.py {emit|check|report|ledger} [path]", file=sys.stderr)
        return 2
    fn = {"emit": cmd_emit, "check": cmd_check,
          "report": cmd_report, "ledger": cmd_ledger}[argv[0]]
    return fn(argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
