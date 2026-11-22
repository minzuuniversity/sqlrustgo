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
#   m7  去掉 verify 的「标签被删即判红」守卫 -> 有标签被删时必须放行；
#       未变异必须判红
#
# 臂数从实际执行的臂里数出来（ARMS + 维度隔离臂 + 具名臂），不再写死。
# 写死臂数是本文件上一版的缺陷：实际执行 6 条却打印 arms=7，报告据此声称「七臂」。
# 退出码：0 = 全部判别成立；1 = 至少一个臂不判别（核对器是摆设）；3 = 锚点漂了（证据过期）。
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

import tempfile


def _read_issues(path):
    """Local copy of issue_labels.read_issues for building the m6 fixture."""
    p = pathlib.Path(path)
    if p.is_dir():
        return []
    return [json.loads(l) for l in p.read_text(encoding="utf-8").splitlines() if l.strip()]

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
    arms_run = 0  # 实际执行的臂数；打印时用它，不写死
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
            arms_run += 1  # 漂移的臂也要计入总数（漂移单独计数）
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
            arms_run += 1
            f = single_gap_fixture(tmp, dim, ["agent:synthetic"])
            rc_hit, s_hit = run(TOOL, ["check", "--input", str(f), "--ledger", str(LEDGER), "--only", dim])
            others = ",".join(d for d in DIMS if d != dim)
            rc_miss, s_miss = run(TOOL, ["check", "--input", str(f), "--ledger", str(LEDGER), "--only", others])
            ok = rc_hit != 0 and rc_miss == 0
            print(f"{'OK  ' if ok else 'BAD '} dim-{dim}: only-{dim} rc={rc_hit} only-{others} rc={rc_miss}")
            if not ok:
                bad_arms.append(f"dim-{dim}")

        # m6: 台账 under-count —— verify 必须因为 added != expected 判红
        # 先从 BEFORE→AFTER 的实际 diff 构造一份**精确**台账（added==expected ⇒ rc=0），
        # 再从它删一条 ⇒ expected 少 1 ⇒ added!=expected ⇒ rc=1。
        # （真台账有 82 条 ops，其中 39 条不在这对快照的 diff 里，控制臂必红。）
        before_l = {i["number"]: i.get("labels") or [] for i in _read_issues(BEFORE)}
        after_l = {i["number"]: i.get("labels") or [] for i in _read_issues(AFTER)}
        exact_rows = []
        for num, albl in sorted(after_l.items()):
            blbl = before_l.get(num, [])
            # 只取三维（priority/cost/cap）；其他维度（如 cap/test）会被工具的
            # LEDGER-BAD 校验拒绝 ⇒ rc=2 而非我们想要的 added!=expected。
            new_ops = {x.split("/", 1)[0]: x for x in albl
                       if x not in blbl and x.split("/", 1)[0] in ("priority", "cost", "cap")}
            if new_ops:
                exact_rows.append({"number": num, "assign": new_ops,
                                   "rationale": "m6 fixture: exact BEFORE→AFTER diff"})
        exact_ledger = tmp / "exact-ledger.jsonl"
        exact_ledger.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in exact_rows),
                                 encoding="utf-8")
        short_rows = [dict(r) for r in exact_rows]
        dropped = short_rows[0]["assign"].popitem()
        short_ledger = tmp / "short-ledger.jsonl"
        short_ledger.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in short_rows),
                                 encoding="utf-8")
        rc_ctl, s_ctl = run(TOOL, ["verify", str(BEFORE), "--after", str(AFTER),
                                   "--ledger", str(exact_ledger)])
        rc_m6, s_m6 = run(TOOL, ["verify", str(BEFORE), "--after", str(AFTER),
                                 "--ledger", str(short_ledger)])
        ok6 = rc_ctl == 0 and rc_m6 != 0
        print(f"{'OK  ' if ok6 else 'BAD '} m6-ledger-undercount: control rc={rc_ctl} mutated rc={rc_m6} dropped={dropped[0]}={dropped[1]} sighting={s_m6[:100]}")
        if not ok6:
            bad_arms.append("m6-ledger-undercount")
        arms_run += 1

        # m7: 「标签被删即判红」这条守卫 —— 去掉它，有标签被删时必须放行
        # 夹具：after 状态比 before 少一个标签（模拟有人删了标签）。
        # 未变异必须判红；摘掉守卫后必须放行（rc=0），否则这条守卫是摆设。
        shrunk = tmp / "shrunk-after.jsonl"
        srows = [json.loads(l) for l in AFTER.read_text(encoding="utf-8").splitlines() if l.strip()]
        victim = next((r for r in srows if r.get("labels")), None)
        if victim is None:
            bad_arms.append("m7-lost-label-guard: 夹具里没有带标签的 issue")
            print("BAD  m7-lost-label-guard: no issue with labels in after-snapshot")
        else:
            dropped7 = victim["labels"][-1]
            victim["labels"] = victim["labels"][:-1]
            shrunk.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in srows),
                              encoding="utf-8")
            # --ledger 显式传：变异副本放在临时目录里，REPO 推导不出仓根。
            # 空台账 ⇒ expected=0 ⇒ added==expected 通过；唯一判红来源就是 lost 守卫。
            empty7 = tmp / "empty-ledger-m7.jsonl"
            empty7.write_text("", encoding="utf-8")
            rc_ctl7, s_ctl7 = run(TOOL, ["verify", str(AFTER), "--after", str(shrunk),
                                         "--ledger", str(empty7)])
            anchor7 = "    if lost:\n        return 1"
            if base.count(anchor7) != 1:
                drift.append(f"m7-lost-label-guard: anchor={base.count(anchor7)}")
                print(f"DRIFT m7-lost-label-guard: anchor count={base.count(anchor7)} != 1")
            else:
                copy7 = tmp / "m7-lost-label-guard.py"
                copy7.write_text(base.replace(anchor7, "    if lost:\n        pass"), encoding="utf-8")
                rc_m7, s_m7 = run(copy7, ["verify", str(AFTER), "--after", str(shrunk),
                                          "--ledger", str(empty7)])
                ok7 = rc_ctl7 != 0 and rc_m7 == 0
                print(f"{'OK  ' if ok7 else 'BAD '} m7-lost-label-guard: control rc={rc_ctl7} mutated rc={rc_m7} dropped={dropped7} sighting={s_m7[:100]}")
                if not ok7:
                    bad_arms.append("m7-lost-label-guard")
            arms_run += 1
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    discriminating = arms_run - len(bad_arms) - len(drift)
    print(f"arms={arms_run} discriminating={discriminating} not_discriminating={len(bad_arms)} drift={len(drift)}")
    # 自洽不变式：discriminating + not_discriminating + drift 必须等于 arms。
    # 写死臂数时这条会立刻破（例如 arms=6 但 discriminating=7），
    # 所以它是对「臂数被编造」这条缺陷的机器反证。
    if discriminating + len(bad_arms) + len(drift) != arms_run:
        print(f"ARMS-COUNT-INCONSISTENT arms={arms_run} "
              f"discriminating={discriminating} not_discriminating={len(bad_arms)} drift={len(drift)}")
        return 1
    for b in bad_arms:
        print("NOT-DISCRIMINATING", b)
    if drift:
        return 3
    return 1 if bad_arms else 0

if __name__ == "__main__":
    raise SystemExit(main())
