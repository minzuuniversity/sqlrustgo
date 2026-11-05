#!/usr/bin/env python3
"""P0-2 反向夹具：三态 runner + 汇总器必须能分辨"跑过 / 挂了 / 没核对"。

每一臂都写清它**证明什么**，且必须与期望一致；任何一臂不符 => 本夹具 rc=1。
CTRL 臂存在的原因（AGENTS §2.5）：证明这些判据不是"永远红"的坏门禁。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RUN = REPO / "scripts" / "gate" / "run_gates.py"
REPORT = REPO / "scripts" / "gate" / "gate_report.py"

PASS, FAIL, INC = "PASS", "FAIL", "INCOMPLETE"
WANT_EXIT = {PASS: 0, FAIL: 1, INC: 2}


def wj(path: Path, rows) -> None:
    path.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows), encoding="utf-8")


def report(tmp: Path, *, plan, results, auth, catalog=None, plan_sha=None) -> tuple[int, dict]:
    cmd = [sys.executable, str(REPORT), "--plan", str(plan), "--results", str(results),
           "--authoritative-file", str(auth)]
    if catalog:
        cmd += ["--catalog", str(catalog)]
    if plan_sha:
        cmd += ["--plan-sha", plan_sha]
    p = subprocess.run(cmd, capture_output=True, text=True)
    line = [l for l in p.stdout.splitlines() if l.startswith("{")]
    return p.returncode, json.loads(line[-1]) if line else {"verdict": "NO-OUTPUT", "reasons": [p.stderr[:300]]}


def arms(tmp: Path) -> list[tuple[str, str, str]]:
    """返回 (臂名, 期望结论, 说明)"""
    out: list[tuple[str, str, str]] = []
    auth = tmp / "auth.jsonl"
    wj(auth, [{"gate": g} for g in ("g1.sh", "g2.sh", "g3.sh")])
    catalog = tmp / "catalog.jsonl"
    wj(catalog, [{"gate": g} for g in ("g1.sh", "g2.sh", "g3.sh")])
    full_plan = tmp / "plan_full.jsonl"
    wj(full_plan, [{"gate": g} for g in ("g1.sh", "g2.sh", "g3.sh")])
    all_pass = tmp / "res_all_pass.jsonl"
    wj(all_pass, [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh", "g3.sh")])

    def one(name: str, plan: Path, res_name: str, rows, want: str, note: str) -> None:
        rp = tmp / res_name
        wj(rp, rows)
        rc, j = report(tmp, plan=plan, results=rp, auth=auth, catalog=catalog)
        ok = j.get("verdict") == want and rc == WANT_EXIT[want]
        out.append((name, want, "OK " + note if ok else f"MISMATCH verdict={j.get('verdict')} rc={rc} reasons={j.get('reasons')[:2]}"))

    # CTRL —— 全集全绿必须判 PASS（证明判据不是永远红）
    out.append(("CTRL", PASS, "pending"))
    rc, j = report(tmp, plan=full_plan, results=all_pass, auth=auth, catalog=catalog)
    out[-1] = ("CTRL", PASS, "OK 全集恰好一次且全 PASS => PASS" if (j.get("verdict") == PASS and rc == 0)
               else f"MISMATCH verdict={j.get('verdict')} rc={rc} reasons={j.get('reasons')}")

    one("M1-missing-result", full_plan, "res_m1.jsonl",
        [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh")], INC,
        "planned 3 只报 2 => 缺项判 INCOMPLETE")
    one("M2-duplicate-result", full_plan, "res_m2.jsonl",
        [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh", "g3.sh", "g3.sh")], INC,
        "同一门禁报两次 => INCOMPLETE（防重复计数刷满覆盖）")
    one("M3-exit-code-lost", full_plan, "res_m3.jsonl",
        [{"gate": "g1.sh", "state": PASS, "exit_code": 0}, {"gate": "g2.sh", "state": PASS},
         {"gate": "g3.sh", "state": PASS, "exit_code": 0}], INC,
        "声称 PASS 但没有 exit_code => INCOMPLETE（正是 iaia410 那批记录缺的字段）")
    one("M4-shrink-expectation", tmp / "plan_short.jsonl", "res_short.jsonl",
        [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh")], INC,
        "plan 只列 2 条但权威集合 3 条 => coverage-short")
    wj(tmp / "plan_short.jsonl", [{"gate": g} for g in ("g1.sh", "g2.sh")])
    wj(tmp / "res_short.jsonl", [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh")])
    # 上面 two one() 调用先于文件写入会读不到，这里补一次真正的核对
    rc4, j4 = report(tmp, plan=tmp / "plan_short.jsonl", results=tmp / "res_short.jsonl", auth=auth, catalog=catalog)
    out[-1] = ("M4-shrink-expectation", INC,
               "OK 缩小期望集合被权威集合反查抓住" if (j4.get("verdict") == INC and rc4 == 2)
               else f"MISMATCH verdict={j4.get('verdict')} rc={rc4} reasons={j4.get('reasons')}")

    one("M5-unregistered-gate", tmp / "plan_fake.jsonl", "res_fake.jsonl",
        [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh", "ghost.sh")], INC,
        "用登记表的 3 条对 plan 的 3 条 => ghost.sh 不在登记表 => INCOMPLETE")
    wj(tmp / "plan_fake.jsonl", [{"gate": g} for g in ("g1.sh", "g2.sh", "ghost.sh")])
    wj(tmp / "res_fake.jsonl", [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh", "ghost.sh")])
    rcf, jf = report(tmp, plan=tmp / "plan_fake.jsonl", results=tmp / "res_fake.jsonl", auth=auth, catalog=catalog)
    out[-1] = ("M5-unregistered-gate", INC,
               "OK 用假门禁名刷覆盖被抓" if (jf.get("verdict") == INC and rcf == 2)
               else f"MISMATCH verdict={jf.get('verdict')} rc={rcf} reasons={jf.get('reasons')}")

    # M11：plan 指纹与当前权威集合不符（换了阶段清单/换了树还拿旧 plan 记账）
    rp11 = tmp / "res_m11.jsonl"
    wj(rp11, [{"gate": g, "state": PASS, "exit_code": 0} for g in ("g1.sh", "g2.sh", "g3.sh")])
    rc11, j11 = report(tmp, plan=full_plan, results=rp11, auth=auth, catalog=catalog,
                       plan_sha="sha256:" + "0" * 64)
    out.append(("M11-stale-plan", INC,
                "OK plan 指纹与权威集合不符 => INCOMPLETE（防拿旧清单给新树发绿）"
                if (j11.get("verdict") == INC and rc11 == 2)
                else f"MISMATCH verdict={j11.get('verdict')} rc={rc11} reasons={j11.get('reasons')[:2]}"))

    one("M6-state-skip", full_plan, "res_skip.jsonl",
        [{"gate": "g1.sh", "state": PASS, "exit_code": 0},
         {"gate": "g2.sh", "state": "SKIP", "exit_code": 0},
         {"gate": "g3.sh", "state": PASS, "exit_code": 0}], INC,
        "把 SKIP 写成 state=SKIP 且 exit 0 => 不接受第四态 => INCOMPLETE")

    # 真实执行臂：runner 的退出码映射
    def exec_arm(name: str, script_bodies: dict[str, str], expect: dict[str, str], note: str,
                 plan_only: tuple[str, ...] = ()) -> None:
        root = tmp / f"exec_{name}"
        root.mkdir(parents=True, exist_ok=True)
        for g, body in script_bodies.items():
            f = root / g
            f.write_text("#!/usr/bin/env bash\n" + body + "\n", encoding="utf-8")
            f.chmod(0o755)
        plan = root / "plan.jsonl"
        wj(plan, [{"gate": g} for g in list(script_bodies) + list(plan_only)])
        res = root / "results.jsonl"
        p = subprocess.run([sys.executable, str(RUN), "--repo-root", str(root), "execute",
                            "--plan", str(plan), "--results", str(res)], capture_output=True, text=True)
        rows = [json.loads(l) for l in res.read_text(encoding="utf-8").splitlines() if l.strip()]
        states = {r["gate"]: r["state"] for r in rows}
        want = "mixed" if len(set(expect.values())) > 1 else next(iter(expect.values()))
        ok = len(rows) == len(script_bodies) + len(plan_only) and states == expect
        out.append((name, want, f"OK runner 映射：{note} => {states}" if ok
                    else f"MISMATCH states={states} 期望={expect} rc={p.returncode}"))

    exec_arm("M7-exit2", {"g1.sh": "exit 2"}, {"g1.sh": INC}, "exit 2 既不是 PASS 也不是 FAIL")
    exec_arm("M8-script-missing", {"present.sh": "exit 0"}, {"present.sh": PASS, "absent.sh": INC},
             "plan 点名一个仓里不存在的门禁 => 该项 INCOMPLETE，其余照常判定",
             plan_only=("absent.sh",))
    exec_arm("M9-real-fail", {"g1.sh": "exit 1"}, {"g1.sh": FAIL}, "exit 1 => FAIL（不会被当成 INCOMPLETE 抹平）")

    # C6 优先级：同时有 FAIL 与 INCOMPLETE 时总结论取 INCOMPLETE，但 FAIL 明细必须在
    mix = tmp / "res_mix.jsonl"
    wj(mix, [{"gate": "g1.sh", "state": FAIL, "exit_code": 1},
             {"gate": "g2.sh", "state": PASS, "exit_code": 0},
             {"gate": "g3.sh", "state": INC, "exit_code": None, "reason": "script-missing"}])
    rcm, jm = report(tmp, plan=full_plan, results=mix, auth=auth, catalog=catalog)
    counts = jm.get("counts", {})
    ok = jm.get("verdict") == INC and rcm == 2 and counts.get(FAIL, 0) == 1
    out.append(("M10-precedence", INC,
                "OK FAIL+INCOMPLETE => 总结论 INCOMPLETE，且 FAIL 明细仍被计数" if ok
                else f"MISMATCH verdict={jm.get('verdict')} rc={rcm} counts={counts}"))

    return out


def main() -> int:
    bad = 0
    with tempfile.TemporaryDirectory() as td:
        for name, want, note in arms(Path(td)):
            flag = "OK " if note.startswith("OK") else "BAD"
            if flag == "BAD":
                bad += 1
            print(f"[{flag}] {name}: 期望={want} :: {note}")
    if bad:
        print(f"test_gate_run_tristate: {bad} 臂不符合期望")
        return 1
    print("test_gate_run_tristate: OK (CTRL + M1..M11 全部符合期望)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
