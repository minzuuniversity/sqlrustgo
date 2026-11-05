#!/usr/bin/env python3
"""三态门禁汇总器（P0-2）—— 独立重解析权威期望集合，再与 results.jsonl 比对。

判据（全部由 scripts/gate/test_gate_run_tristate.py 反向变异核对）：

  C1 期望集合不可缩小：plan/results 少一条权威清单里的门禁 => INCOMPLETE(coverage-short)。
  C2 缺项：plan 有、results 没有 => INCOMPLETE(missing-result)。
  C3 重复：同一 gate 在 results 出现两次 => INCOMPLETE(duplicate-result)。
  C4 退出码丢失/自相矛盾：state 与 exit_code 不一致，或 state 不在三态内 => INCOMPLETE(state-unverifiable)。
  C5 未登记名：results/plan 里的门禁不在登记表里 => INCOMPLETE(unregistered-gate)。
  C7 过期 plan：plan 指纹与当前权威集合指纹不符 => INCOMPLETE(stale-plan)。
     指纹由 run_gates.py 的 expected_sha256() 写在 <plan>.meta.json，汇总时自动读取；
     没有这一步，"按旧清单生成的 plan" 也能被判成覆盖齐全，等于给新树发旧绿。
  C6 优先级：只要有任何 INCOMPLETE，总结论就是 INCOMPLETE；否则有 FAIL 即 FAIL；全 PASS 才 PASS。
     理由：一次没跑全的执行**没有资格**给出结论；把"没核对"降级成"只挂了 X 条"，
     会让下一次只修那 X 条然后宣布绿 —— 这正是本仓反复出事的形状。

退出码：PASS=0 / FAIL=1 / INCOMPLETE=2（CI 里 1 与 2 都不是通过）。
本脚本不是门禁：不叫 check_*、不进 STAGE_CONFIG 阶段清单、不改 168 分母。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from run_gates import (  # noqa: E402
    STATE_PASS, STATE_FAIL, STATE_INCOMPLETE, EXIT_BY_VERDICT, load_jsonl, authoritative_gates,
)

VALID_STATES = (STATE_PASS, STATE_FAIL, STATE_INCOMPLETE)


def registered_names(catalog_path: Path) -> set[str]:
    names: set[str] = set()
    for row in load_jsonl(catalog_path):
        g = row.get("gate")
        if g:
            names.add(str(g))
    return names


def aggregate(plan_rows: list[dict], result_rows: list[dict], expected: list[str],
              catalog: set[str] | None, plan_sha: str | None = None) -> dict:
    reasons: list[str] = []
    from run_gates import expected_sha256
    live_sha = expected_sha256(expected)
    if plan_sha and plan_sha != live_sha:
        reasons.append(f"stale-plan: plan fingerprint {plan_sha[:18]} != authoritative {live_sha[:18]}"
                       " (plan 是在别的树上生成的，不得拿来给这棵树记账)")
    counts = {STATE_PASS: 0, STATE_FAIL: 0, STATE_INCOMPLETE: 0}

    # C5 / C1: 权威集合与 plan 的差集
    plan_names = [r.get("gate", "") for r in plan_rows]
    if not expected:
        reasons.append("coverage-short: authoritative expected set is EMPTY")
    for g in expected:
        if g not in plan_names:
            reasons.append(f"coverage-short: expected gate not in plan: {g}")
    exp_set = set(expected)
    kinds = {r.get("gate"): r.get("kind", "script") for r in plan_rows}
    for g in plan_names:
        if g not in exp_set:
            reasons.append(f"unregistered-gate: plan entry outside authoritative list: {g}")
        elif kinds.get(g, "script") == "script" and catalog is not None and g not in catalog:
            reasons.append(f"unregistered-gate: plan entry not in gate catalog: {g}")

    # C2 / C3: results 与 plan 对齐
    seen: dict[str, int] = {}
    for r in result_rows:
        g = r.get("gate", "")
        seen[g] = seen.get(g, 0) + 1
        if g not in plan_names:
            reasons.append(f"unregistered-gate: result for gate outside plan: {g}")
    for g, n in sorted(seen.items()):
        if n > 1:
            reasons.append(f"duplicate-result: {g} appears {n} times")
    for g in plan_names:
        if seen.get(g, 0) == 0:
            reasons.append(f"missing-result: planned gate never reported: {g}")

    # C4: 状态与退出码自洽
    for r in result_rows:
        g = r.get("gate", "?")
        st, rc = r.get("state"), r.get("exit_code")
        if st not in VALID_STATES:
            counts[STATE_INCOMPLETE] += 1
            reasons.append(f"state-unverifiable: {g} state={st!r} not one of {VALID_STATES}")
            continue
        if st == STATE_PASS and rc != 0:
            counts[STATE_INCOMPLETE] += 1
            reasons.append(f"state-unverifiable: {g} claims PASS but exit_code={rc!r}")
        elif st == STATE_FAIL and rc != 1:
            counts[STATE_INCOMPLETE] += 1
            reasons.append(f"state-unverifiable: {g} claims FAIL but exit_code={rc!r}")
        elif st == STATE_INCOMPLETE and rc in (0, 1) and not r.get("reason"):
            counts[STATE_INCOMPLETE] += 1
            reasons.append(f"state-unverifiable: {g} INCOMPLETE with exit_code={rc} and no reason")
        else:
            counts[st] += 1

    # C6: 优先级
    coverage_broken = any(r.startswith(("coverage-short", "missing-result", "duplicate-result",
                                        "unregistered-gate", "stale-plan")) for r in reasons)
    full_pass = (bool(expected) and not reasons and counts[STATE_PASS] == len(expected)
                 and len(result_rows) == len(expected))
    if coverage_broken or counts[STATE_INCOMPLETE] > 0:
        verdict = STATE_INCOMPLETE          # 没跑全/记不清 => 没有资格给结论
    elif full_pass:
        verdict = STATE_PASS                # 每项恰好一次且全 PASS
    elif counts[STATE_FAIL] > 0:
        verdict = STATE_FAIL
    else:
        verdict = STATE_INCOMPLETE          # 覆盖对不上但说不清原因 => 也只能是不完整

    return {"verdict": verdict, "counts": counts, "expected_n": len(expected),
            "planned_n": len(plan_names), "result_n": len(result_rows), "reasons": reasons}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--plan", required=True)
    ap.add_argument("--results", required=True)
    ap.add_argument("--repo-root", default=str(Path(__file__).resolve().parents[2]))
    ap.add_argument("--stage")
    ap.add_argument("--version")
    ap.add_argument("--authoritative-file", help="夹具/离线用：自己给出权威期望集合的 jsonl（每行 {gate}）")
    ap.add_argument("--catalog", help="登记表 jsonl（用于 C5 未登记名核对）")
    ap.add_argument("--plan-sha", help="plan 生成时记录的权威集合指纹（用于发现 plan 与新树错位）")
    ap.add_argument("--out", help="把汇总写成 jsonl")
    a = ap.parse_args()

    repo = Path(a.repo_root).resolve()
    if a.authoritative_file:
        expected = [r.get("gate", "") for r in load_jsonl(Path(a.authoritative_file))]
    else:
        if not (a.stage and a.version):
            print("gate_report: need --stage/--version or --authoritative-file", file=sys.stderr)
            return EXIT_BY_VERDICT[STATE_INCOMPLETE]
        expected = [g["gate"] for g in authoritative_gates(a.stage, a.version, repo)]

    catalog = registered_names(Path(a.catalog)) if a.catalog else None
    plan_sha = None
    meta = Path(str(a.plan) + ".meta.json")
    if a.plan_sha:
        plan_sha = a.plan_sha
    elif meta.is_file():
        plan_sha = json.loads(meta.read_text(encoding="utf-8")).get("expected_sha256")
    summary = aggregate(load_jsonl(Path(a.plan)), load_jsonl(Path(a.results)), expected, catalog, plan_sha)
    line = json.dumps(summary, ensure_ascii=False)
    print(line)
    if a.out:
        Path(a.out).parent.mkdir(parents=True, exist_ok=True)
        Path(a.out).write_text(line + "\n", encoding="utf-8")
    return EXIT_BY_VERDICT[summary["verdict"]]


if __name__ == "__main__":
    sys.exit(main())
