#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# LB-1 核对器的反向变异跑法（AGENTS 2.5）。
#
#   python3 audit/v4.1.0/issue_labels_mutations.py
#
# 每个臂都必须「判别成立」才算 load-bearing：
#   m1  把 has() 改成永远 True（= 缺标签全部豁免）  -> 整改前夹具必须变绿；未变异必须红
#   m2..m4 维度隔离：单维缺口夹具上，只查该维必须红；把该维摘掉必须绿
#   m5  去掉台账理由校验 -> 空理由台账必须被放过；未变异必须判 LEDGER-BAD
#   m6  台账少登记一条（under-count）-> verify 的 added 与 expected 必须不一致而判红；
#       用真台账做对照必须绿
# 退出码：0 = 全部判别成立；1 = 至少一个臂不判别（核对器是摆设）；3 = 锚点漂了（证据过期）。
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
TOOL = HERE / "issue_labels.py"
BEFORE = HERE / "neg/snapshots/labelgap-open-before-20261105.jsonl"
AFTER = HERE / "neg/snapshots/labelgap-open-after-20261105.jsonl"
LEDGER = HERE.parents[1] / "docs/releases/v4.1.0/ISSUE_LABEL_LEDGER_v4.1.0.jsonl"
DIMS = ("priority", "cost", "cap")

ARMS = [
    ("m1-has-always-true",
     'def has(labels, dim):\n    return any(str(x).startswith(dim + "/") for x in labels)',
     'def has(labels, dim):\n    return True',
     "BEFORE_FIXTURE", "green"),
    ("m5-ledger-rationale-check-removed",
     'if not str(r.get("rationale", "")).strip():',
     'if False:',
     "BAD_LEDGER", "green"),
]


def run(path, extra):
    p = subprocess.run([sys.executable, str(path)] + extra, capture_output=True, text=True, timeout=180)
    first = next((l for l in (p.stdout + "\n" + p.stderr).splitlines() if l.strip()), "")
    return p.returncode, first[:150]


def single_gap_fixture(tmp, dim, other_labels):
    """一个 issue：只缺 dim 这一维，其余两维都有标签。"""
    f = tmp / f"gap-{dim}.jsonl"
    labels = [f"{d}/p1" for d in DIMS if d != dim] + list(other_labels)
    f.write_text(json.dumps({"number": 1, "title": "synthetic", "labels": labels},
                            ensure_ascii=False) + "\n", encoding="utf-8")
    return f


def main() -> int:
    base = TOOL.read_text(encoding="utf-8")
    ledger_text = LEDGER.read_text(encoding="utf-8")
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="lb1-mut-"))
    bad_arms, drift = [], []
    try:
        rc0, s0 = run(TOOL, ["check", "--input", str(BEFORE), "--ledger", str(LEDGER)])
        print(f"BASELINE before-fixture: rc={rc0} {s0[:120]}")
        if rc0 == 0:
            print("BASELINE-NOT-RED: 整改前夹具本该判红，判红条件已失效")
            return 1
        rcA, sA = run(TOOL, ["check", "--input", str(AFTER), "--ledger", str(LEDGER)])
        print(f"BASELINE after-fixture: rc={rcA} {sA[:120]}")
        if rcA != 0:
            print("BASELINE-AFTER-RED: 整改后夹具必须绿（不是永远红的摆设）")
            return 1

        for name, old, new, case, want in ARMS:
            if base.count(old) != 1:
                drift.append(f"{name}: anchor={base.count(old)}")
                print(f"DRIFT {name}: anchor count={base.count(old)} != 1")
                continue
            copy = tmp / (name + ".py")
            copy.write_text(base.replace(old, new), encoding="utf-8")
            if case == "BEFORE_FIXTURE":
                rc, sighting = run(copy, ["check", "--input", str(BEFORE), "--ledger", str(LEDGER)])
            else:
                bad_ledger = tmp / "bad-ledger.jsonl"
                rows = [json.loads(l) for l in ledger_text.splitlines() if l.strip()]
                rows[0]["rationale"] = ""
                bad_ledger.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                                      encoding="utf-8")
                rc_un, _ = run(TOOL, ["check", "--input", str(AFTER), "--ledger", str(bad_ledger)])
                if rc_un == 0:
                    bad_arms.append(f"{name}: 未变异没抓空理由台账")
                    print(f"NOT-DISCRIMINATING unmutated says OK on bad ledger (want red)")
                rc, sighting = run(copy, ["check", "--input", str(AFTER), "--ledger", str(bad_ledger)])
            ok = (rc == 0) if want == "green" else (rc != 0)
            print(f"{'OK  ' if ok else 'BAD '} {name}: mutated rc={rc} want={want} sighting={sighting[:110]}")
            if not ok:
                bad_arms.append(name)

        for dim in DIMS:
            f = single_gap_fixture(tmp, dim, ["agent:synthetic"])
            rc_hit, s_hit = run(TOOL, ["check", "--input", str(f), "--ledger", str(LEDGER), "--only", dim])
            others = ",".join(d for d in DIMS if d != dim)
            rc_miss, s_miss = run(TOOL, ["check", "--input", str(f), "--ledger", str(LEDGER), "--only", others])
            ok = rc_hit != 0 and rc_miss == 0
            print(f"{'OK  ' if ok else 'BAD '} dim-{dim}: only-{dim} rc={rc_hit} only-{others} rc={rc_miss}")
            if not ok:
                bad_arms.append(f"dim-{dim}")

        # m6: 台账 under-count —— verify 必须因为 added != expected 判红
        short_ledger = tmp / "short-ledger.jsonl"
        rows = [json.loads(l) for l in ledger_text.splitlines() if l.strip()]
        dropped = rows[0]["assign"].popitem()
        short_ledger.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                                 encoding="utf-8")
        rc_ctl, s_ctl = run(TOOL, ["verify", str(BEFORE)])
        rc_m6, s_m6 = run(TOOL, ["verify", str(BEFORE), "--ledger", str(short_ledger)])
        ok6 = rc_ctl == 0 and rc_m6 != 0
        print(f"{'OK  ' if ok6 else 'BAD '} m6-ledger-undercount: control rc={rc_ctl} mutated rc={rc_m6} dropped={dropped[0]}={dropped[1]} sighting={s_m6[:100]}")
        if not ok6:
            bad_arms.append("m6-ledger-undercount")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print(f"arms=7 discriminating={7 - len(bad_arms) - len(drift)} not_discriminating={len(bad_arms)} drift={len(drift)}")
    for b in bad_arms:
        print("NOT-DISCRIMINATING", b)
    if drift:
        return 3
    return 1 if bad_arms else 0

if __name__ == "__main__":
    raise SystemExit(main())
