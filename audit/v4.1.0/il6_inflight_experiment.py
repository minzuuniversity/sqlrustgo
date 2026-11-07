#!/usr/bin/env python3
"""IL-6 判别实验：同一个门禁命令在「有/无进行中的 run 目录」两种状态下的读数。

IL-6 的原始判据是「同一条命令在 run 内 rc=1 / run 后 rc=0」——
但今天这条**不可复现**，原因是基线本来就红（历史 hollow-manifest 那批，见看板 P0-6）。
所以实验改用**红数**（FAIL 行数）来隔离自指效应：

    状态 A（基线）    ：树里没有进行中的 run 目录            ⇒ 记 FAIL 行数
    状态 B（进行中）  ：造一个只有 manifest、logs 未写完的 run 目录 ⇒ 记 FAIL 行数
    状态 C（移除后）  ：删掉那个目录再跑一次                ⇒ 记 FAIL 行数

若自指效应存在，B 的红数应比 A 多 1（那个「正在执行的 run 目录」被算成了违规）。
实测（2026-11-08）：A = B = C = 8 ⇒ **现行实现没有把进行中的 run 目录计入**（该病灶已不成立）。

用法：python3.14 audit/v4.1.0/il6_inflight_experiment.py [--gate <脚本>] [--json]
本脚本**只读+临时造目录**（造完即删），不改任何已归档证据。
"""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
RUNS_DIR = REPO_ROOT / "audit/v4.1.0/runs"
SCRATCH_ID = "zz-il6-inflight-probe"


def run_gate(gate: str) -> dict:
    p = subprocess.run(["bash", gate], cwd=str(REPO_ROOT), capture_output=True, text=True, timeout=1800)
    out = (p.stdout or "") + (p.stderr or "")
    fails = len(re.findall(r"^FAIL:", out, re.M))
    return {"rc": p.returncode, "fail_lines": fails}


def main(argv: list[str]) -> int:
    gate = "scripts/gate/check_audit_manifest.sh"
    as_json = False
    i = 0
    while i < len(argv):
        if argv[i] == "--gate" and i + 1 < len(argv):
            gate = argv[i + 1]; i += 2
        elif argv[i] == "--json":
            as_json = True; i += 1
        else:
            i += 1

    scratch = RUNS_DIR / SCRATCH_ID
    if scratch.exists():
        shutil.rmtree(scratch)

    a = run_gate(gate)                                    # 状态 A：基线

    (scratch / "logs").mkdir(parents=True, exist_ok=True)  # 状态 B：造一个「正在执行」的 run
    (scratch / "manifest.yaml").write_text(
        "schema_version: audit-run/v1\nrun_id: %s\n" % SCRATCH_ID, encoding="utf-8")
    b = run_gate(gate)

    shutil.rmtree(scratch, ignore_errors=True)              # 状态 C：移除后
    c = run_gate(gate)

    out = {
        "kind": "il6_inflight_experiment",
        "gate": gate,
        "baseline": a,
        "inflight": b,
        "after_removal": c,
        "self_reference_effect": b["fail_lines"] - a["fail_lines"],
        "note": ("B 比 A 多 %d 条 ⇒ 进行中的 run 目录被计入判定（自指效应存在）"
                 % (b["fail_lines"] - a["fail_lines"]))
                if b["fail_lines"] != a["fail_lines"]
                else "A/B/C 红数相同 ⇒ 进行中的 run 目录**没有**被计入判定（该病灶在现行实现里不复现）",
        "baseline_red_reason": "基线本身即红（历史 hollow-manifest 那批，见看板 P0-6），"
                               "所以「run 内 rc=1 / run 后 rc=0」这条签名今天不可达 ⇒ 用红数隔离。",
    }
    if as_json:
        print(json.dumps(out, ensure_ascii=False, indent=2, sort_keys=True))
    else:
        for k in ("baseline", "inflight", "after_removal"):
            print("%-14s rc=%s fail_lines=%s" % (k, out[k]["rc"], out[k]["fail_lines"]))
        print("self_reference_effect =", out["self_reference_effect"])
        print(out["note"])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
