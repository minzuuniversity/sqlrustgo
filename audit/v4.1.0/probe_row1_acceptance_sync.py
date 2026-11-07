#!/usr/bin/env python3
"""ROW-1 探针：`acceptance` 与 `status`/`evidence` 是否同步。

判据（看板 ROW-1 的验收原文）：
    「撤销/改判据必须同一提交改 `acceptance` 并留复现命令；
      探针判据：status=完成 且 acceptance 含「必须变绿」的行，
      其引用臂必须在最近归档 run 为绿，或 acceptance 已改写。」

为什么需要探针：
    判据被撤销（产品其实没缺陷，是探针期望值写错）时，很容易只改 `task`/`evidence`，
    忘了同步 `acceptance` —— 于是同一行里出现「完成」和「必须变绿」并存的自相矛盾，
    而看板的既有检查（render/counts/doc_facts）**看不出**这种矛盾：它们只校验结构。

三种判定：
    OFFENDER   —— status=完成 且 acceptance 仍含「必须变绿」，且其引用的**归档臂是红的**
                  （引用得到 ⇒ 判据没兑现，却已经标完成）。
    UNVERIFIED —— status=完成 且 acceptance 仍含「必须变绿」，但**引用不到**任何归档臂
                  （没有 run 引用 / run 不在树里 / 臂名对不上）⇒ 无法核对，不等于绿。
    OK         —— 其余（含 acceptance 已被改写、不再要求变绿的行）。

用法：
    python3.14 audit/v4.1.0/probe_row1_acceptance_sync.py            # 查当前看板
    python3.14 audit/v4.1.0/probe_row1_acceptance_sync.py --selftest  # 自检（反向变异）

退出码：0 = 没有 OFFENDER/UNVERIFIED；1 = 有；2 = 用法/环境错。
本探针**不新增门禁**、不进 STAGE_CONFIG ⇒ 门禁分母不变。
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
BOARD = REPO_ROOT / "docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl"
RUNS_DIR = REPO_ROOT / "audit/v4.1.0/runs"

MUST_BE_GREEN = "必须变绿"
# 归档 run id 的形状：2026MMDD-<slug>
RUN_ID_RE = re.compile(r"20\d{6}-[a-z0-9][a-z0-9._-]*")
# 引用臂：优先取反引号里的标识符（形如 `capability_smoke` / `null_ambiguous_on_wire`）
ARM_RE = re.compile(r"`([A-Za-z0-9_./-]{3,})`")


def load_board(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            rows.append(json.loads(line))
    return rows


def run_ids_in(text: str) -> list[str]:
    seen: list[str] = []
    for m in RUN_ID_RE.findall(text or ""):
        if m not in seen:
            seen.append(m)
    return seen


def arm_candidates(text: str) -> list[str]:
    """从行文本里猜「被引用的臂」标识符。

    取反引号标识符（最可靠），去掉明显的路径/文档名；再退回取「判据/臂」前的那段词。
    """
    out: list[str] = []
    for a in ARM_RE.findall(text or ""):
        if "/" in a and a.endswith((".rs", ".sh", ".py", ".md", ".jsonl", ".yaml")):
            continue
        if a in out:
            continue
        out.append(a)
    for m in re.finditer(r"([A-Za-z0-9_]{4,})\s*(?:判据|臂)", text or ""):
        w = m.group(1)
        if w not in out:
            out.append(w)
    return out


def manifest_results(run_id: str) -> list[dict] | None:
    p = RUNS_DIR / run_id / "manifest.yaml"
    if not p.is_file():
        return None
    try:
        import yaml  # type: ignore
        m = yaml.safe_load(p.read_text(encoding="utf-8")) or {}
    except Exception:
        return None
    return m.get("results") or []


def check_row(row: dict) -> tuple[str, str]:
    """返回 (verdict, detail)。verdict ∈ {OK, OFFENDER, UNVERIFIED}。"""
    if row.get("status") != "完成":
        return "OK", "非完成行"
    acc = row.get("acceptance") or ""
    if MUST_BE_GREEN not in acc:
        return "OK", "acceptance 已改写（不再要求变绿）"
    text = (row.get("acceptance") or "") + "\n" + (row.get("evidence") or "") + "\n" + (row.get("task") or "")
    ids = run_ids_in(text)
    if not ids:
        return "UNVERIFIED", "acceptance 仍要求变绿，但行里没有任何归档 run 引用 ⇒ 无法核对"
    # 取**最近**的、在树里存在的归档 run（run id 以日期开头 ⇒ 字典序即时间序）
    for rid in sorted(ids, reverse=True):
        results = manifest_results(rid)
        if results is None:
            continue
        arms = arm_candidates(text)
        matched = []
        for r in results:
            rid_field = str(r.get("id", ""))
            if any(a and a in rid_field for a in arms):
                matched.append((rid_field, r.get("exit_code")))
        if not matched:
            continue
        bad = [(i, c) for i, c in matched if c != 0]
        if bad:
            return "OFFENDER", f"引用的臂是红的：{bad}（run {rid}）"
        return "OK", f"引用的臂为绿：{matched}（run {rid}）"
    return "UNVERIFIED", f"引用的 run 不在树里/读不出：{ids}"


def probe(board_path: Path) -> tuple[int, dict]:
    rows = load_board(board_path)
    offenders, unver, ok = [], [], 0
    for r in rows:
        verdict, detail = check_row(r)
        if verdict == "OFFENDER":
            offenders.append((r.get("id"), detail))
        elif verdict == "UNVERIFIED":
            unver.append((r.get("id"), detail))
        else:
            ok += 1
    out = {
        "kind": "row1_acceptance_sync",
        "board": str(board_path.relative_to(REPO_ROOT)) if board_path.is_relative_to(REPO_ROOT) else str(board_path),
        "rows": len(rows),
        "ok": ok,
        "offenders": offenders,
        "unverified": unver,
        "verdict": "FAIL" if (offenders or unver) else "OK",
    }
    return (1 if (offenders or unver) else 0), out


def selftest() -> int:
    """反向变异：注入三类合成行，断言探针只抓该抓的。

    用的真实归档臂（都已在树里）：
      - 绿臂：20261107-crash-002 的 `crash-commit-fsync-b1`（rc=0）
      - 红臂：20261107-crash-002 的 `crash-wal-index-b1`（rc=101）
    """
    import tempfile

    base = load_board(BOARD)
    fails = []

    def synth(rid, status, acceptance, evidence=""):
        return {
            "id": rid,
            "task": "合成夹具行",
            "status": status,
            "priority": "P2",
            "stage": "ALPHA",
            "version": "v4.1.0",
            "domain": "test",
            "acceptance": acceptance,
            "evidence": evidence,
            "origin": "probe selftest",
            "board": "v4.1.0",
            "blocked_by": "-",
        }

    green_arm = "crash-commit-fsync-b1"
    red_arm = "crash-wal-index-b1"
    run_id = "20261107-crash-002"

    cases = [
        # A: 完成 + 仍要求变绿 + 引用红臂 ⇒ 必须被抓
        ("A-offender", synth("ZZ-A", "完成", f"该臂 `{red_arm}` 必须变绿", f"run {run_id}"), True),
        # B: 同上但 acceptance 已改写 ⇒ 不得被抓
        ("B-rewritten", synth("ZZ-B", "完成", "原判据已撤销，改由别的行承载", f"run {run_id} `{red_arm}`"), False),
        # C: 完成 + 要求变绿 + 引用绿臂 ⇒ 不得被抓
        ("C-green", synth("ZZ-C", "完成", f"该臂 `{green_arm}` 必须变绿", f"run {run_id}"), False),
        # D: 完成 + 要求变绿但无任何 run 引用 ⇒ UNVERIFIED（要报出来）
        ("D-unverified", synth("ZZ-D", "完成", "某个没有归档引用的判据必须变绿"), True),
    ]

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td) / "board.jsonl"
        for label, row, should_flag in cases:
            rows = base + [row]
            tmp.write_text("\n".join(json.dumps(r, ensure_ascii=False) for r in rows) + "\n", encoding="utf-8")
            rc, out = probe(tmp)
            flagged = bool(out["offenders"] or out["unverified"])
            if flagged != should_flag:
                fails.append(f"{label}: 期望 flagged={should_flag}，实际 {flagged}（{out}）")
            else:
                print(f"[PASS] {label}: flagged={flagged} as expected")
    if fails:
        for f in fails:
            print(f"[FAIL] {f}")
        return 1
    print("probe_row1_acceptance_sync: OK (4 个反向变异臂全部符合期望)")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    if any(a in ("-h", "--help") for a in argv):
        print(__doc__)
        return 0
    rc, out = probe(BOARD)
    print(json.dumps(out, ensure_ascii=False, indent=2, sort_keys=True))
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
