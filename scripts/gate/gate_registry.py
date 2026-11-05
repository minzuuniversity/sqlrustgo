#!/usr/bin/env python3
"""把「有哪些门禁、每道挂在哪个执行点上」变成一张可核对的登记表（P0-1）。

为什么放在 scripts/gate/ 但不叫 check_*：AGENTS 规定新增判据不得新增门禁分母。
本工具是 run 级登记/核对器，不进 STAGE_CONFIG 的阶段清单。

权威输入（都是既有 SSOT，不新建）：
  scripts/gate/check_*.sh|py            门禁全集（与 gov_probe.check_scripts 同口径）
  docs/governance/STAGE_CONFIG.yaml     阶段 required_gates
  .gitea/workflows + .github/workflows  CI 执行点（gate_executability.ci_roots）

登记表 docs/releases/v4.1.0/GATE_CATALOG.jsonl 每行：
  gate / class / stages / reason / owner_task / can_fail / swallow_hits / neg_fixture

class 四值（互斥，按优先级判定）：
  stage_required     被某阶段的 required_gates 解析到
  ci_reachable       CI 里从 root step 可达
  orphan_referenced  只在脚本正文（去注释）里被引用，没有执行点
  dead               既无执行点也无引用

反向变异（AGENTS §2.5）见 scripts/gate/test_gate_registry.py：
  删一行 / 改 class / 抹 reason / 翻 can_fail 都必须 rc!=0。
"""
from __future__ import annotations

import argparse
import glob
import json
import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CATALOG = REPO_ROOT / "docs" / "releases" / "v4.1.0" / "GATE_CATALOG.jsonl"
STAGE_ORDER = ("DRAFT", "ALPHA", "BETA", "RC", "GA")

sys.path.insert(0, str(REPO_ROOT / "scripts" / "gate"))


def check_scripts() -> list[str]:
    return sorted(
        glob.glob("scripts/gate/check_*.sh") + glob.glob("scripts/gate/check_*.py")
    )


def _src_without_comments(path: str) -> str:
    import gov_probe as gp
    return gp._src_without_comments(path)


def live_rows() -> list[dict]:
    """现场重算：每道门禁此刻真实挂在哪个执行点上。"""
    import yaml
    import gate_executability as ge
    import gov_probe as gp

    ge.build_matrix()
    by_stage = ge.required_scripts_by_stage()
    listed = ge.required_script_paths()
    roots, steps = ge.ci_roots()
    reach = set(roots) | set(ge.reachable_from_roots(roots, steps, {}, None))

    refs = set()
    for pat in ("scripts/**/*.sh", "scripts/**/*.py", "gate/*.sh"):
        for p in glob.glob(pat, recursive=True):
            body = Path(p).read_text(encoding="utf-8", errors="replace")
            refs |= gp.gate_refs_in(p, body, ge.GATE_PATH_RE)

    rows = []
    for f in check_scripts():
        src = _src_without_comments(f)
        stages = [s for s in STAGE_ORDER if f in by_stage.get(s, set())]
        if f in listed:
            cls = "stage_required"
        elif f in reach:
            cls = "ci_reachable"
        elif f in refs:
            cls = "orphan_referenced"
        else:
            cls = "dead"
        rows.append({
            "gate": f,
            "class": cls,
            "stages": stages,
            "reason": _reason(cls),
            "owner_task": _owner(cls),
            "can_fail": gp.can_fail_paths(src),
            "swallow_hits": sum(src.count(t) for t in gp.SWALLOW_TOKENS),
            "neg_fixture": bool(gp.NEG_FIXTURE_RE.search(src)),
        })
    return rows


CLASSES = ("stage_required", "ci_reachable", "orphan_referenced", "dead")
DISPOSITIONS = ("archive-candidate", "keep-rewire", "needs-execution-point",
                "needs-external-host", "superseded")
AUTHORED = ("disposition", "note", "evidence")

REASONS = {
    "stage_required": "在 STAGE_CONFIG 的阶段 required_gates 清单里",
    "ci_reachable": "CI 从 root step 可达，但不在任何阶段清单",
    "orphan_referenced": "只有脚本正文引用，没有任何阶段/CI 执行点（谁执行它不明）",
    "dead": "既无执行点也无正文引用（去注释口径）⇒ 永远不会运行",
}
OWNERS = {
    "stage_required": "-",
    "ci_reachable": "P0-2",
    "orphan_referenced": "P0-2",
    "dead": "P0-4",
}


def _reason(cls: str) -> str:
    return REASONS[cls]


def _owner(cls: str) -> str:
    return OWNERS[cls]


def cmd_emit(argv: list[str]) -> int:
    out = Path(argv[0]) if argv else CATALOG
    rows = live_rows()
    prev: dict[str, dict] = {}
    if out.is_file():
        prev = {json.loads(l)["gate"]: json.loads(l)
                for l in out.read_text(encoding="utf-8").splitlines() if l.strip()}
    for r in rows:  # 人工写就的处置与理由不派生，重生成时必须原样保留
        for k in AUTHORED:
            if k in prev.get(r["gate"], {}):
                r[k] = prev[r["gate"]][k]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                   encoding="utf-8")
    ct = {c: sum(1 for r in rows if r["class"] == c) for c in CLASSES}
    print(json.dumps({"kind": "gate_catalog_emit", "path": str(out),
                      "rows": len(rows), "by_class": ct,
                      "can_fail_n": sum(1 for r in rows if r["can_fail"]),
                      "neg_fixture_n": sum(1 for r in rows if r["neg_fixture"])},
                     ensure_ascii=False))
    return 0


def cmd_check(argv: list[str]) -> int:
    path = Path(argv[0]) if argv else CATALOG
    fails: list[str] = []
    if not path.is_file():
        print(f"MISSING catalog: {path}")
        return 2
    rec = [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines() if l.strip()]
    rec_by = {}
    for r in rec:
        g = r.get("gate")
        if g in rec_by:
            fails.append(f"C1 重复登记: {g}")
        rec_by[g] = r

    live = {r["gate"]: r for r in live_rows()}

    # C1 双向覆盖：登记表必须与门禁全集一一对应
    missing = sorted(set(live) - set(rec_by))
    extra = sorted(set(rec_by) - set(live))
    if missing:
        fails.append(f"C1 {len(missing)} 道门禁未登记: {missing[:6]}")
    if extra:
        fails.append(f"C1 {len(extra)} 行登记无对应文件: {extra[:6]}")

    # C2/C3/C5 现场重算必须与登记一致（登记表不许漂）
    for g, want in live.items():
        got = rec_by.get(g)
        if not got:
            continue
        for k in ("class", "stages", "can_fail", "swallow_hits", "neg_fixture"):
            if got.get(k) != want[k]:
                fails.append(f"C2 {g} 登记 {k}={got.get(k)!r} 现场重算 {want[k]!r}")
        # C4 不可运行的门禁必须显式交代
        if want["class"] in ("dead", "orphan_referenced"):
            if not (got.get("reason") or "").strip():
                fails.append(f"C4 {g} 缺 reason")
            if not (got.get("owner_task") or "").strip() or got.get("owner_task") == "-":
                fails.append(f"C4 {g} 缺 owner_task")

    # C6 不可运行的门禁必须"交代"，且理由必须逐字出自该门禁文件（反编造）
    for g, want in live.items():
        got = rec_by.get(g)
        if not got:
            continue
        if want["class"] in ("dead", "orphan_referenced"):
            dsp = got.get("disposition")
            if dsp not in DISPOSITIONS:
                fails.append(f"C6 {g} disposition={dsp!r} 不在词表 {list(DISPOSITIONS)}")
            ev = (got.get("evidence") or "").strip()
            if len(ev) < 8:
                fails.append(f"C6 {g} 缺 evidence（必须逐字引用门禁文件里的行）")
            elif ev not in Path(g).read_text(encoding="utf-8", errors="replace"):
                fails.append(f"C6 {g} evidence 在该门禁文件里找不到 ⇒ 疑似编造理由")
            if len((got.get("note") or "").strip()) < 12:
                fails.append(f"C6 {g} 缺 note（为什么这样处置）")
        # C7 处置与执行点状态不能自相矛盾
        if got.get("disposition") in ("archive-candidate", "superseded") \
                and want["class"] in ("stage_required", "ci_reachable"):
            fails.append(f"C7 {g} 已被标 {got.get('disposition')} 但此刻有执行点 ⇒ 登记表过期")

    ct = {c: sum(1 for g, r in live.items() if r["class"] == c) for c in CLASSES}
    print(json.dumps({"kind": "gate_registry_check", "catalog": str(path),
                      "check_n": len(live), "catalog_rows": len(rec),
                      "by_class": ct,
                      "by_disposition": {k: sum(1 for r in rec if r.get("disposition") == k)
                                         for k in DISPOSITIONS},
                      "fails": len(fails)}, ensure_ascii=False))
    for f in fails[:40]:
        print("FAIL:", f)
    if fails:
        print(f"gate_registry: {len(fails)} FAIL")
        return 1
    print("gate_registry: OK")
    return 0


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] not in ("emit", "check"):
        print("usage: gate_registry.py {emit|check} [catalog]", file=sys.stderr)
        return 2
    return cmd_emit(argv[1:]) if argv[0] == "emit" else cmd_check(argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
