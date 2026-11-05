#!/usr/bin/env python3
"""P0-2 三态 runner 的**真实门禁**端到端探针（本脚本不是门禁）。

三个模式各自断言一件可复现的事实，全部成立才退出 0：

  empty  真实 ALPHA 阶段清单（10 项）+ 空 results
         => gate_report 必须判 INCOMPLETE、退出码 2，并给出 10 条 missing-result。
         意义：一次没跑的执行没有资格给结论，也不会"默认通过"。

  mini   3 项真实门禁子集（2 项当前确实绿 + 1 项当前确实红），真跑 execute
         => 必须判 FAIL、退出码 1、counts 为 pass 2 / fail 1。
         意义：真失败不会被三态口径抹平；命令通道（非脚本）也能被执行与记账。
         注意：这是 **3 项子集**，不是 ALPHA/GA 阶段清单全量，不构成门禁通过声明。

  mini-green  反向变异对照：把 mini 里那条真红的门禁摘掉，
              execute 的退出码必须从 1 变回 0（证明新退出码不是"永远非零"）。

  stale  用 ALPHA 清单生成的 plan 去对 GA 的权威集合
         => 必须报 stale-plan（plan 指纹与当前权威集合指纹不符）=> INCOMPLETE。
         意义：防止拿旧清单生成的 plan 给新树记账。

退出码：0 = 断言全部成立（复现成功）；1 = 断言不符。
不叫 check_*、不进 STAGE_CONFIG 阶段清单、不改门禁分母 168。
"""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RUN = REPO / "scripts" / "gate" / "run_gates.py"
REPORT = REPO / "scripts" / "gate" / "gate_report.py"

# mini 子集：故意含一条当前确实红的门禁（check_audit_manifest.sh 在冻结树上红 7 条）
MINI = [
    {"gate": "scripts/gate/check_gate_executability.sh", "kind": "script"},
    {"gate": "bash scripts/gate/check_stage.sh --version v4.1.0 --dry-run", "kind": "command"},
    {"gate": "scripts/gate/check_audit_manifest.sh", "kind": "script"},
]


def wj(path: Path, rows) -> None:
    path.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                    encoding="utf-8")


def last_json(stdout: str) -> dict:
    line = [l for l in stdout.splitlines() if l.startswith("{")]
    return json.loads(line[-1]) if line else {"verdict": "NO-OUTPUT", "reasons": [stdout[:300]]}


def make_plan(tmp: Path, stage: str) -> Path:
    p = tmp / ("plan-" + stage + ".jsonl")
    r = subprocess.run([sys.executable, str(RUN), "plan", "--stage", stage,
                        "--version", "v4.1.0", "--plan", str(p),
                        "--run-id", "probe-" + stage.lower()],
                       capture_output=True, text=True, cwd=str(REPO))
    if r.returncode != 0:
        raise SystemExit("probe: plan failed rc=%d %s" % (r.returncode, r.stderr[:400]))
    return p


def do_report(tmp: Path, plan_p: Path, res_p: Path, extra=()) -> tuple[int, dict]:
    cmd = [sys.executable, str(REPORT), "--plan", str(plan_p), "--results", str(res_p),
           "--out", str(tmp / "report.json")] + list(extra)
    r = subprocess.run(cmd, capture_output=True, text=True, cwd=str(REPO))
    return r.returncode, last_json(r.stdout)


def read_plan(p: Path) -> list[dict]:
    return [json.loads(l) for l in p.read_text(encoding="utf-8").splitlines() if l.strip()]


def mode_empty(tmp: Path) -> list[tuple[str, bool, str]]:
    plan_p = make_plan(tmp, "ALPHA")
    rows = read_plan(plan_p)
    kinds = [r.get("kind") for r in rows]
    res = tmp / "res-empty.jsonl"
    res.write_text("", encoding="utf-8")
    rc, j = do_report(tmp, plan_p, res, ["--stage", "ALPHA", "--version", "v4.1.0"])
    n = len([x for x in j.get("reasons", []) if x.startswith("missing-result")])
    return [
        ("ALPHA plan rows == 10", len(rows) == 10, "rows=%d" % len(rows)),
        ("kind split script 7 / command 3",
         kinds.count("script") == 7 and kinds.count("command") == 3,
         "script=%d command=%d" % (kinds.count("script"), kinds.count("command"))),
        ("report rc == 2 (INCOMPLETE)", rc == 2, "rc=%d" % rc),
        ("verdict INCOMPLETE (not a silent default)", j.get("verdict") == "INCOMPLETE",
         "verdict=%s" % j.get("verdict")),
        ("missing-result count == 10 (每项都点名)", n == 10, "n=%d" % n),
    ]


def mode_mini(tmp: Path) -> list[tuple[str, bool, str]]:
    auth = tmp / "auth-mini.jsonl"
    catalog = tmp / "catalog-mini.jsonl"
    plan_p = tmp / "plan-mini.jsonl"
    wj(auth, [{"gate": r["gate"]} for r in MINI])
    wj(catalog, [{"gate": r["gate"]} for r in MINI])
    wj(plan_p, MINI)
    res = tmp / "res-mini.jsonl"
    ex = subprocess.run([sys.executable, str(RUN), "execute", "--plan", str(plan_p),
                         "--results", str(res), "--timeout-sec", "180"],
                        capture_output=True, text=True, cwd=str(REPO))
    rows = [json.loads(l) for l in res.read_text(encoding="utf-8").splitlines() if l.strip()]
    states = {r["gate"]: r.get("state") for r in rows}
    rc, j = do_report(tmp, plan_p, res,
                      ["--authoritative-file", str(auth), "--catalog", str(catalog)])
    c = j.get("counts", {})
    return [
        ("execute produced 3 results", len(rows) == 3, "n=%d (execute rc=%d)" % (len(rows), ex.returncode)),
        ("execute self exit code follows verdict (FAIL => 1, not 0)", ex.returncode == 1,
         "execute rc=%d" % ex.returncode),
        ("command-channel gate actually ran",
         states.get(MINI[1]["gate"]) in ("PASS", "FAIL", "INCOMPLETE"),
         "state=%s" % states.get(MINI[1]["gate"])),
        ("the currently-red gate reported as FAIL (not smoothed away)",
         states.get("scripts/gate/check_audit_manifest.sh") == "FAIL",
         "state=%s" % states.get("scripts/gate/check_audit_manifest.sh")),
        ("verdict FAIL", j.get("verdict") == "FAIL", "verdict=%s" % j.get("verdict")),
        ("report rc == 1", rc == 1, "rc=%d" % rc),
        ("counts pass 2 / fail 1 / incomplete 0",
         c.get("PASS") == 2 and c.get("FAIL") == 1 and c.get("INCOMPLETE") == 0,
         "counts=%s" % json.dumps(c)),
    ]


def mode_stale(tmp: Path) -> list[tuple[str, bool, str]]:
    plan_p = make_plan(tmp, "ALPHA")          # meta.json 里存的是 ALPHA 的指纹
    res = tmp / "res-empty.jsonl"
    res.write_text("", encoding="utf-8")
    rc, j = do_report(tmp, plan_p, res, ["--stage", "GA", "--version", "v4.1.0"])
    reasons = j.get("reasons", [])
    return [
        ("stale-plan reason present", any(x.startswith("stale-plan") for x in reasons),
         "reasons0=%s" % (reasons[0][:120] if reasons else "none")),
        ("verdict INCOMPLETE", j.get("verdict") == "INCOMPLETE", "verdict=%s" % j.get("verdict")),
        ("report rc == 2", rc == 2, "rc=%d" % rc),
    ]


def mode_mini_green(tmp: Path) -> list[tuple[str, bool, str]]:
    """反向变异对照（AGENTS 2.5）：把 mini 子集里那条真红的门禁摘掉，
    同一个 execute 的退出码必须从 1 变回 0 —— 证明新加的退出码不是"永远非零"。"""
    subset = [r for r in MINI if r["gate"] != "scripts/gate/check_audit_manifest.sh"]
    auth = tmp / "auth-green.jsonl"
    catalog = tmp / "catalog-green.jsonl"
    plan_p = tmp / "plan-green.jsonl"
    wj(auth, [{"gate": r["gate"]} for r in subset])
    wj(catalog, [{"gate": r["gate"]} for r in subset])
    wj(plan_p, subset)
    res = tmp / "res-green.jsonl"
    ex = subprocess.run([sys.executable, str(RUN), "execute", "--plan", str(plan_p),
                         "--results", str(res), "--timeout-sec", "180"],
                        capture_output=True, text=True, cwd=str(REPO))
    rc, j = do_report(tmp, plan_p, res,
                      ["--authoritative-file", str(auth), "--catalog", str(catalog)])
    return [
        ("subset without the red gate: report verdict PASS", j.get("verdict") == "PASS",
         "verdict=%s" % j.get("verdict")),
        ("report rc == 0", rc == 0, "rc=%d" % rc),
        ("execute self exit code back to 0 (mutation control)", ex.returncode == 0,
         "execute rc=%d" % ex.returncode),
    ]


MODES = {"empty": mode_empty, "mini": mode_mini, "mini-green": mode_mini_green,
         "stale": mode_stale}


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in MODES:
        print("usage: probe_tristate_e2e.py {empty|mini|mini-green|stale}", file=sys.stderr)
        return 1
    mode = sys.argv[1]
    bad = 0
    with tempfile.TemporaryDirectory(prefix="srg-tristate-probe-") as td:
        for name, ok, detail in MODES[mode](Path(td)):
            print("[%s] %s :: %s" % ("OK " if ok else "BAD", name, detail))
            if not ok:
                bad += 1
    print("probe_tristate_e2e[%s]: %s" % (mode, "断言全部成立" if bad == 0 else "有 %d 条不符" % bad))
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
