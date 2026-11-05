#!/usr/bin/env python3
"""issue 台账的新鲜度与可重放性探针（IL-4 的机器判据）。

放在 audit/ 而不是 scripts/gate/：run 级探针，不进门禁分母（不命名 check_*，
不在 STAGE_CONFIG 阶段清单里）。

两个模式：
  snapshot-count  现场重拉 open issues ⇒ 条数必须等于已提交台账行数。
                  不等 ⇒ 点名差集并 rc=1。含义：台账过期不是"记得刷就行"，
                  而是会被机器红着脸指出来。
  replay-rows     用现场快照 + 仓内四份分诊件重放 issue_ledger.py ⇒
                  行级字段必须与已提交台账逐字段相等。含义：分诊结论不能事后改写。

第三个模式是它自己的反向变异（AGENTS 2.5）：
  mutation-selftest  往台账副本里注入两处漂移，探针必须各自变红：
                     (1) 删一行 ⇒ snapshot-count 必须红（少一条当期 issue 就过不去）
                     (2) 改一行的 domain ⇒ replay-rows 必须红（事后改写分诊结论会被重放抓到）
                     原样副本必须绿。任一臂反着来 ⇒ 本模式 exit 1（判据可疑）。

退出码：0 一致 / 1 漂移（真发现）/ 2 输入不可用（INCOMPLETE，不等于通过）
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs" / "releases" / "v4.1.0"
GOV = ROOT / "scripts" / "gate" / "gov_probe.py"
LEDGER = ROOT / "scripts" / "gate" / "issue_ledger.py"

TRIAGE_FILES = [DOCS / "ISSUE_TRIAGE_2026-11-05.jsonl",
                DOCS / "ISSUE_TRIAGE_CROSSCHECK_2026-11-05.jsonl",
                DOCS / "ISSUE_TRIAGE_2026-11-05B.jsonl",
                DOCS / "ISSUE_TRIAGE_CROSSCHECK_2026-11-05B.jsonl"]

# comments/url 会随时间变（评论数、URL 前缀），比对这些字段只会制造假红。
COMPARE_FIELDS = ("number", "title", "priority", "cost", "cap", "state_label",
                  "domain", "breaks_claim", "automatable", "blast", "fix_shape",
                  "dup_of", "evidence_grade", "triage_families", "created_at")


def run_dump(tmp: pathlib.Path) -> tuple[pathlib.Path, dict] | tuple[None, dict]:
    dump = tmp / "fresh-issues.json"
    proc = subprocess.run([sys.executable, str(GOV), "issues", "--dump", str(dump)],
                          capture_output=True, text=True, cwd=str(ROOT))
    info: dict = {}
    for line in proc.stdout.splitlines():
        try:
            rec = json.loads(line)
        except ValueError:
            continue
        if rec.get("kind") == "issue_summary":
            info["open_total"] = rec.get("open_total")
        if rec.get("kind") == "issues_dump":
            info["sha256"] = rec.get("sha256")
            info["count"] = rec.get("count")
    if proc.returncode != 0 or not dump.is_file():
        return None, {"http_rc": proc.returncode,
                      "stderr": proc.stderr.strip().splitlines()[-1:]}
    return dump, info


def committed_ledger(jsonl: pathlib.Path) -> list[dict]:
    return [json.loads(l) for l in jsonl.read_text(encoding="utf-8").splitlines() if l.strip()]



def selftest(args) -> int:
    """反向变异：注入漂移，探针必须变红；原样副本必须绿。"""
    import copy
    import tempfile as _tf
    src_j = pathlib.Path(args.ledger_jsonl)
    rows = committed_ledger(src_j)
    n_ok = n_bad = 0

    def run(label: str, mode: str, ledger_jsonl: str) -> int:
        proc = subprocess.run([sys.executable, str(pathlib.Path(__file__).resolve()), mode,
                               "--ledger-jsonl", ledger_jsonl,
                               "--ledger-md", str(args.ledger_md)],
                              capture_output=True, text=True, cwd=str(ROOT))
        first = next((l for l in proc.stdout.splitlines() if l.startswith("{")), "")
        if not first:
            first = "NO-JSON-OUTPUT stderr=" + (proc.stderr.strip().splitlines()[-1:] or [""])[0][:160]
        print(f"ARM {label} mode={mode} rc={proc.returncode} {first[:200]}")
        return proc.returncode

    with _tf.TemporaryDirectory(prefix="srg-mut-") as td:
        td = pathlib.Path(td)
        # CTRL：原样副本 ⇒ 两模式都必须 rc=0
        ctrl = td / "ctrl.jsonl"
        ctrl.write_text(src_j.read_text(encoding="utf-8"), encoding="utf-8")
        for mode in ("snapshot-count", "replay-rows"):
            rc = run("CTRL", mode, str(ctrl))
            good = (rc == 0)
            print(f"  expect rc=0 got rc={rc} ok={1 if good else 0}")
            n_ok, n_bad = n_ok + (1 if good else 0), n_bad + (0 if good else 1)

        # 漂移 1：删一行（当期 open 比台账多一条）⇒ 必须 rc!=0
        drop = td / "drop.jsonl"
        drop.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows[1:]),
                        encoding="utf-8")
        rc = run("MUT-drop-row", "snapshot-count", str(drop))
        good = (rc != 0)
        print(f"  expect rc!=0 got rc={rc} ok={1 if good else 0}")
        n_ok, n_bad = n_ok + (1 if good else 0), n_bad + (0 if good else 1)

        # 漂移 2：改写一行 domain ⇒ replay-rows 必须 rc!=0
        tam = copy.deepcopy(rows)
        tam[0]["domain"] = "__mutated-after-the-fact__"
        tam_p = td / "tam.jsonl"
        tam_p.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in tam),
                         encoding="utf-8")
        rc = run("MUT-domain-rewrite", "replay-rows", str(tam_p))
        good = (rc != 0)
        print(f"  expect rc!=0 got rc={rc} ok={1 if good else 0}")
        n_ok, n_bad = n_ok + (1 if good else 0), n_bad + (0 if good else 1)

    print(f"mutation-selftest: ok={n_ok} bad={n_bad}")
    if n_bad:
        print("mutation-selftest: 判据可疑（有臂未按预期，不能据此声称探针有效）")
        return 1
    print("mutation-selftest: OK (CTRL 绿；删行 / 改 domain 均红)")
    return 0

def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("mode", choices=["snapshot-count", "replay-rows", "mutation-selftest"])
    ap.add_argument("--ledger-jsonl", default=str(DOCS / "ISSUE_LEDGER_2026-11-05B.jsonl"))
    ap.add_argument("--ledger-md", default=str(DOCS / "ISSUE_LEDGER_2026-11-05B.md"))
    args = ap.parse_args(argv)

    if args.mode == "mutation-selftest":
        return selftest(args)

    led_j, led_m = pathlib.Path(args.ledger_jsonl), pathlib.Path(args.ledger_md)
    if not (led_j.is_file() and led_m.is_file()) or not all(p.is_file() for p in TRIAGE_FILES):
        print(json.dumps({"kind": "ledger_freshness", "mode": args.mode,
                          "result": "INCOMPLETE", "why": "台账或分诊件缺失"},
                         ensure_ascii=False))
        return 2

    rows = committed_ledger(led_j)
    with tempfile.TemporaryDirectory(prefix="srg-ledger-") as td:
        tmp = pathlib.Path(td)
        dump, info = run_dump(tmp)
        if dump is None:
            print(json.dumps({"kind": "ledger_freshness", "mode": args.mode,
                              "result": "INCOMPLETE", "why": "现场取不到 open issues",
                              **info}, ensure_ascii=False))
            return 2

        fresh = json.loads(dump.read_text(encoding="utf-8"))
        fresh_nums = {int(i["number"]) for i in fresh}
        led_nums = {int(r["number"]) for r in rows}

        if args.mode == "snapshot-count":
            drift_missing = sorted(led_nums - fresh_nums)   # 台账有、当期没有（已关）
            drift_extra = sorted(fresh_nums - led_nums)     # 当期有、台账没有（新开）
            out = {"kind": "ledger_freshness", "mode": args.mode,
                   "fresh_open": info.get("count"), "ledger_rows": len(rows),
                   "snapshot_sha256": info.get("sha256"),
                   "closed_since_snapshot": drift_missing,
                   "opened_since_snapshot": drift_extra}
            out["result"] = "consistent" if not (drift_missing or drift_extra) else "DRIFT"
            print(json.dumps(out, ensure_ascii=False))
            return 0 if out["result"] == "consistent" else 1

        # replay-rows：同一输入必须得到同一行级结论
        lineages = ""
        ml = re.search(r"ladder credited lineage=([^；;\n]+)", led_m.read_text(encoding="utf-8"))
        if ml:
            lineages = ml.group(1).strip()
        out_j, out_m = tmp / "replay.jsonl", tmp / "replay.md"
        cmd = [sys.executable, str(LEDGER), "--issues", str(dump),
               "--triage", *[str(p) for p in TRIAGE_FILES],
               "--out-jsonl", str(out_j), "--out-md", str(out_m)]
        if lineages:
            cmd += ["--credited-lineages", lineages]
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=str(ROOT))
        if not out_j.is_file():
            print(json.dumps({"kind": "ledger_freshness", "mode": args.mode,
                              "result": "INCOMPLETE", "why": "重放未能产出台账",
                              "ledger_rc": proc.returncode,
                              "stderr": proc.stderr.strip().splitlines()[-1:]},
                             ensure_ascii=False))
            return 2
        replay = committed_ledger(out_j)
        rp = {int(r["number"]): r for r in replay}
        diffs: list[dict] = []
        for r in rows:
            n = int(r["number"])
            if n not in rp:
                diffs.append({"number": n, "field": "(row)", "why": "重放里没有这行"})
                continue
            for f in COMPARE_FIELDS:
                if r.get(f) != rp[n].get(f):
                    diffs.append({"number": n, "field": f,
                                  "committed": r.get(f), "replay": rp[n].get(f)})
        extra = sorted(set(rp) - led_nums)
        if extra:
            diffs += [{"number": n, "field": "(row)", "why": "重放多出的行"} for n in extra]

        out = {"kind": "ledger_freshness", "mode": args.mode,
               "committed_rows": len(rows), "replay_rows": len(replay),
               "ledger_rc": proc.returncode, "compared_fields": len(COMPARE_FIELDS),
               "row_diffs": len(diffs), "sample_diffs": diffs[:5],
               "replay_fingerprint": "sha256:" + hashlib.sha256(out_j.read_bytes()).hexdigest()}
        out["result"] = "consistent" if not diffs else "DRIFT"
        print(json.dumps(out, ensure_ascii=False))
        if diffs:
            return 1
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
