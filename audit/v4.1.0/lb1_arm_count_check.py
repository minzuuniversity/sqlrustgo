#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""LB-1 臂数的**外部**核对（不是门禁，不进 STAGE_CONFIG，不改门禁分母）。

  python3 audit/v4.1.0/lb1_arm_count_check.py

为什么需要外部核对：`issue_labels_mutations.py` 自己打印自己的臂数，
同一个程序无法证明自己的打印没被写死 —— 上一版就是把 `arms=7` 写死，
而实际只执行 6 条臂，报告据此声称的臂数是编的。

所以这里从外面重新数一遍：
  1) 跑变异跑法，数它实际打印的逐臂 verdict 行数（OK/BAD/DRIFT），
     与它自己声称的 arms= 数值比对；两个数必须相等。
  2) 反向变异：把臂数改回写死（arms=6），跑法必须判红。
     写死时 discriminating 仍是 7，与 arms=6 自相矛盾。
  3) 反向变异：去掉 verify 的「标签被删即判红」守卫后，
     有标签被删的夹具必须放行（rc=0），未变异必须判红。
     这条守卫此前**没有任何**反向变异覆盖。

退出码：0 = 三条核对全部成立；1 = 至少一条不成立（判据是摆设）；
        3 = 锚点漂了（证据过期，需要重新定位锚点）。
"""
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
TOOL = HERE / "issue_labels.py"
MUT = HERE / "issue_labels_mutations.py"
AFTER = HERE / "neg/snapshots/labelgap-open-after-20261105.jsonl"
LEDGER = REPO / "docs/releases/v4.1.0/ISSUE_LABEL_LEDGER_v4.1.0.jsonl"

VERDICT_RE = re.compile(r"^(OK  |BAD |DRIFT )")
ARMS_RE = re.compile(r"^arms=(\d+)\s+discriminating=(\d+)\s+"
                      r"not_discriminating=(\d+)\s+drift=(\d+)", re.M)


def run(cmd, cwd=None, timeout=300):
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=cwd)
    return p.returncode, p.stdout + p.stderr


def build_tree(tmp):
    """搭一棵能让 REPO 推导成立的最小仓：<tmp>/audit/v4.1.0 + <tmp>/docs/..."""
    (tmp / "audit" / "v4.1.0").mkdir(parents=True)
    (tmp / "docs" / "releases" / "v4.1.0").mkdir(parents=True)
    shutil.copy(TOOL, tmp / "audit" / "v4.1.0" / "issue_labels.py")
    shutil.copytree(HERE / "neg", tmp / "audit" / "v4.1.0" / "neg")
    shutil.copy(LEDGER, tmp / "docs" / "releases" / "v4.1.0" / LEDGER.name)
    return tmp


def shrunk_after(tmp):
    """造一份「比 after 少一个标签」的状态：模拟有人删了标签。"""
    rows = [json.loads(l) for l in AFTER.read_text(encoding="utf-8").splitlines() if l.strip()]
    victim = next(r for r in rows if r.get("labels"))
    dropped = victim["labels"][-1]
    victim["labels"] = victim["labels"][:-1]
    out = tmp / "shrunk-after.jsonl"
    out.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                   encoding="utf-8")
    return out, dropped


def check_arm_count_matches_verdicts():
    """核对 1：跑法声称的 arms= 必须等于它实际打印的逐臂 verdict 行数。

    变异跑法的 rc 不要求 0：rc=1 可能来自「有臂没判别」（NOT-DISCRIMINATING），
    那是另一个判据的职责；本核对只看「臂数声明 vs 实际 verdict 行数」是否一致。
    """
    rc, out = run([sys.executable, str(MUT)], timeout=600)
    m = ARMS_RE.search(out)
    if not m:
        print("BAD  arm-count-vs-verdicts: 输出里没有 arms= 汇总行")
        return False
    claimed = int(m.group(1))
    verdicts = sum(1 for l in out.splitlines() if VERDICT_RE.match(l))
    ok = claimed == verdicts
    print(f"{'OK  ' if ok else 'BAD '} arm-count-vs-verdicts: claimed arms={claimed} "
          f"verdict_lines={verdicts}")
    return ok


def check_counter_mutation_caught():
    """核对 2：把臂数写死（arms=6）必须被抓。

    抓它的不是脚本内部的不变式（同一个程序证明不了自己的打印没被写死），
    而是**外部**的核对 1 那条：写死后 claimed arms=6 与实际 verdict 行数 7 不等，
    外部核对必须因此判红。所以这里跑完变异体后，用核对 1 的同一把尺子量它。
    """
    base = MUT.read_text(encoding="utf-8")
    anchor = 'f"arms={arms_run} discriminating='
    if base.count(anchor) != 1:
        print(f"DRIFT counter-mutation: anchor count={base.count(anchor)} != 1")
        return None
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="lb1-counter-"))
    try:
        build_tree(tmp)
        mut = tmp / "audit" / "v4.1.0" / "mut.py"
        mut.write_text(base.replace(anchor, 'f"arms=6 discriminating='), encoding="utf-8")
        rc, out = run([sys.executable, str(mut)], cwd=str(tmp), timeout=600)
        line = next((l for l in out.splitlines() if l.startswith("arms=")), "")
        m = ARMS_RE.search(out)
        claimed = int(m.group(1)) if m else -1
        verdicts = sum(1 for l in out.splitlines() if VERDICT_RE.match(l))
        # 外部尺子：claimed arms 必须等于实际 verdict 行数。写死后 6 != 7 ⇒ 判红。
        caught = (claimed != verdicts) and claimed >= 0
        print(f"{'OK  ' if caught else 'BAD '} counter-mutation-caught: {line[:80]} "
              f"verdict_lines={verdicts} 外部尺子判红={caught}")
        if not caught:
            print("     写死臂数后外部尺子仍判绿 ⇒ 臂数可被编造，本核对失效")
        return caught
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def check_lost_guard_mutation():
    """核对 3：去掉「标签被删即判红」守卫后必须放行；未变异必须判红。"""
    base = TOOL.read_text(encoding="utf-8")
    anchor = "    if lost:\n        return 1"
    if base.count(anchor) != 1:
        print(f"DRIFT lost-guard-mutation: anchor count={base.count(anchor)} != 1")
        return None
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="lb1-lostguard-"))
    try:
        build_tree(tmp)
        shrunk, dropped = shrunk_after(tmp)
        # 空台账 ⇒ expected_ops=0 ⇒ added==expected 自然通过，
        # 唯一可能判红的就剩 `if lost: return 1` 这条守卫本身。
        # （真台账的 expected=39 来自它记的历史 ops，会让 added!=expected
        #  也判红，把「守卫是否被摘掉」这条判据淹没。）
        empty_ledger = tmp / "empty-ledger.jsonl"
        empty_ledger.write_text("", encoding="utf-8")
        # 未变异：用真工具对「少一个标签」的状态核对，必须判红
        rc_ctl, out_ctl = run([sys.executable, str(TOOL), "verify", str(AFTER),
                               "--after", str(shrunk), "--ledger", str(empty_ledger)], timeout=300)
        # 变异：把守卫摘掉，同一夹具必须放行
        mutant = tmp / "audit" / "v4.1.0" / "issue_labels.py"
        mutant.write_text(base.replace(anchor, "    if lost:\n        pass"), encoding="utf-8")
        rc_mut, out_mut = run([sys.executable, str(mutant), "verify", str(AFTER),
                               "--after", str(shrunk), "--ledger", str(empty_ledger)], timeout=300)
        ok = rc_ctl != 0 and rc_mut == 0
        print(f"{'OK  ' if ok else 'BAD '} lost-guard-mutation: control rc={rc_ctl} "
              f"mutated rc={rc_mut} dropped={dropped}")
        if rc_ctl == 0:
            print("     未变异没判红 ⇒ 夹具没造出「标签被删」")
        if rc_mut != 0:
            print(f"     摘掉守卫后仍判红 rc={rc_mut} ⇒ 守卫另有来源或本核对失效")
        return ok
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main() -> int:
    results = {
        "arm-count-vs-verdicts": check_arm_count_matches_verdicts(),
        "counter-mutation-caught": check_counter_mutation_caught(),
        "lost-guard-mutation": check_lost_guard_mutation(),
    }
    drift = [k for k, v in results.items() if v is None]
    bad = [k for k, v in results.items() if v is False]
    print(f"checks={len(results)} ok={len(results) - len(bad) - len(drift)} "
          f"not_ok={len(bad)} drift={len(drift)}")
    for k in bad:
        print("NOT-OK", k)
    for k in drift:
        print("DRIFT", k)
    if drift:
        return 3
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
