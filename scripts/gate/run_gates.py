#!/usr/bin/env python3
"""三态门禁 runner —— 计划(P0-2)。

设计要点（每条都可被 scripts/gate/test_gate_run_tristate.py 反向变异核对）：

1. plan 与 execute 分离。`plan` 先落一份 plan.jsonl（权威期望集合的快照 + 计数 + git head），
   `execute` 只往 results.jsonl 追加**已完成**的门禁结果。
   => runner 中途崩溃/被杀时，plan 里没跑完的项在汇总时必然按"缺项"处理成 INCOMPLETE，
      不会留下"看起来跑过"的假 PASS。
2. 三态映射（AGENTS §2.3：SKIP/INCOMPLETE 不得写成 PASS）：
   exit 0 => PASS；exit 1 => FAIL；其它（2、124、信号、脚本缺失、无法启动）=> INCOMPLETE。
3. 汇总器 gate_report.py **独立重新解析**权威期望集合（同一 stage 清单 / 登记表），
   再与 plan 比对。plan 少一项、重复一项、名字不在登记表里 => INCOMPLETE。
   => runner 不能靠"缩小期望集合"骗到全绿。
4. 本工具不是门禁（不叫 check_*，不进 STAGE_CONFIG 阶段清单，不改 168 的分母），
   它是门禁的**执行与记账面**。

退出码：0 仅当 verdict=PASS；1=FAIL；2=INCOMPLETE。
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

import gate_executability as GE  # noqa: E402  (P0-7 起：矩阵必须显式 build，否则抛 MatrixNotBuilt)

STATE_PASS = "PASS"
STATE_FAIL = "FAIL"
STATE_INCOMPLETE = "INCOMPLETE"

EXIT_BY_VERDICT = {STATE_PASS: 0, STATE_FAIL: 1, STATE_INCOMPLETE: 2}


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    for ln in path.read_text(encoding="utf-8").splitlines():
        ln = ln.strip()
        if ln:
            rows.append(json.loads(ln))
    return rows


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows), encoding="utf-8")


def authoritative_gates(stage: str, version: str, repo: Path) -> list[dict]:
    """权威期望集合 = STAGE_CONFIG 里该阶段的每一条 required_gates。

    脚本与命令**都算**（只挑脚本等于悄悄缩小阶段清单）。
    G 号/别名走 check_gate_executability 用的同一个 resolve_gate_path，避免第二套真值。
    """
    import yaml
    cfg_path = repo / "docs" / "governance" / "STAGE_CONFIG.yaml"
    cfg = yaml.safe_load(cfg_path.read_text(encoding="utf-8"))
    stages = cfg.get("stages") or {}
    if stage not in stages:
        return []
    out: list[dict] = []
    for raw in (stages[stage].get("required_gates") or []):
        entry = str(raw).strip()
        if not entry:
            continue
        resolved = GE.resolve_gate_path(entry)
        if resolved:
            row = {"gate": resolved, "kind": "script"}
            if resolved != entry:
                row["alias"] = entry
        elif entry.startswith("scripts/"):
            row = {"gate": entry, "kind": "script"}
        else:
            row = {"gate": entry, "kind": "command"}
        if row not in out:
            out.append(row)
    return out


def resolve_script(gate: str, repo: Path) -> Path | None:
    p = repo / gate
    return p if p.is_file() else None


def cmd_plan(args: argparse.Namespace) -> int:
    repo = Path(args.repo_root).resolve()
    gates = authoritative_gates(args.stage, args.version, repo)
    if not gates:
        print(f"run_gates: EMPTY expected set for stage={args.stage} version={args.version}", file=sys.stderr)
        return EXIT_BY_VERDICT[STATE_INCOMPLETE]
    n_cmd = sum(1 for g in gates if g["kind"] == "command")
    print(f"run_gates: stage list = script {len(gates)-n_cmd} + command {n_cmd} (命令也是清单的一部分)")
    head = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"],
                          capture_output=True, text=True).stdout.strip()
    out = Path(args.plan)
    write_jsonl(out, gates)
    stamp = {
        "run_id": args.run_id,
        "stage": args.stage,
        "version": args.version,
        "git_head": head,
        "planned_at": utc_now(),
        "expected_n": len(gates),
        "plan": str(out),
    }
    Path(str(out) + ".meta.json").write_text(json.dumps(stamp, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"run_gates: planned {len(gates)} gates for {args.stage}/{args.version} -> {out}")
    return 0


def cmd_execute(args: argparse.Namespace) -> int:
    repo = Path(args.repo_root).resolve()
    plan_path = Path(args.plan)
    if not plan_path.is_file():
        print(f"run_gates: plan file missing: {plan_path} (run `plan` first)", file=sys.stderr)
        return EXIT_BY_VERDICT[STATE_INCOMPLETE]
    plan = load_jsonl(plan_path)
    kinds = {r.get("gate"): r.get("kind", "script") for r in plan}
    results_path = Path(args.results)
    results_path.parent.mkdir(parents=True, exist_ok=True)
    results_path.write_text("", encoding="utf-8")

    n_pass = n_fail = n_inc = 0
    for i, row in enumerate(plan, start=1):
        gate = row.get("gate", "")
        kind = kinds.get(gate, "script")
        script = resolve_script(gate, repo) if kind == "script" else None
        rec = {"gate": gate, "seq": i, "kind": kind}
        started = utc_now()
        if kind == "script" and script is None:
            rc, why = None, "script-missing"
        else:
            t0 = time.time()
            try:
                argv = ["bash", str(script)] if kind == "script" else ["bash", "-c", gate]
                p = subprocess.run(argv, cwd=str(repo),
                                   capture_output=True, timeout=args.timeout_sec)
                rc = p.returncode
                (results_path.parent / f"{i:03d}-{Path(gate).name}.stdout").write_bytes(p.stdout[-20000:])
                (results_path.parent / f"{i:03d}-{Path(gate).name}.stderr").write_bytes(p.stderr[-8000:])
                if rc not in (0, 1):
                    why = f"exit-code-outside-pass-fail({rc})"
            except subprocess.TimeoutExpired:
                rc, why = 124, "timeout"
            except OSError as e:
                rc, why = None, f"exec-failed:{type(e).__name__}"
            rec["duration_s"] = round(time.time() - t0, 1)
        if rc == 0:
            state = STATE_PASS
        elif rc == 1:
            state = STATE_FAIL
        else:
            state = STATE_INCOMPLETE
            rec["reason"] = why or "unknown-exit"
        rec.update({"exit_code": rc, "state": state, "started": started, "finished": utc_now()})
        with results_path.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(rec, ensure_ascii=False) + "\n")
        n_pass += state == STATE_PASS
        n_fail += state == STATE_FAIL
        n_inc += state == STATE_INCOMPLETE
        print(f"[{state}] {gate}" + (f" ({rec.get('reason')})" if rec.get("reason") else ""))
    print(f"run_gates: executed n={len(plan)} pass={n_pass} fail={n_fail} incomplete={n_inc} -> {results_path}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--repo-root", default=str(REPO_ROOT))
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("plan", help="把权威期望集合写成 plan.jsonl（不执行任何门禁）")
    p.add_argument("--stage", required=True)
    p.add_argument("--version", required=True)
    p.add_argument("--plan", required=True)
    p.add_argument("--run-id", required=True)
    p.set_defaults(fn=cmd_plan)

    e = sub.add_parser("execute", help="按 plan.jsonl 执行并追加三态结果")
    e.add_argument("--plan", required=True)
    e.add_argument("--results", required=True)
    e.add_argument("--timeout-sec", type=int, default=900)
    e.set_defaults(fn=cmd_execute)

    a = ap.parse_args()
    return a.fn(a)


if __name__ == "__main__":
    sys.exit(main())
