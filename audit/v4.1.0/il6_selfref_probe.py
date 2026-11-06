#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""IL-6 证据门禁自指的判别实验与反向变异（不是门禁，不进 STAGE_CONFIG）。

  python3 audit/v4.1.0/il6_selfref_probe.py

问题（IL-6）：scripts/gate/check_audit_manifest.sh 原先扫**工作区**目录，
于是「正在执行的 run」自己的目录也被算进判定 —— 它有 manifest.yaml、
但日志还没写完，verify 必红 ⇒ run 内红数 7->8，归档完再扫又回到 7。
判据自己数自己，红绿随执行时刻漂移。

本探针做三件事，每件都要有可复现证据：

  1) 现状核对：造一个**未提交**的在飞 run 目录（有 manifest、无 results），
     门禁必须**不**把它算成红（在飞 run 不进判定范围）。
  2) 反向变异：把门禁的枚举改回「扫工作区目录」，门禁必须**重新**把它算成红
     —— 证明这条判据是可被反向变异抓住的，不是恒绿摆设。
  3) 历史判据不被放宽：门禁必须仍然把已知的历史不可复核记录判红，
     且不可复核记录条数与改动前一致（不允许「为了让门禁变绿」而放宽历史判据）。

退出码：0 = 三条全部成立；1 = 至少一条不成立；3 = 锚点漂了（证据过期）。
"""
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
AUDIT = REPO / "audit" / "v4.1.0"
GATE = REPO / "scripts" / "gate" / "check_audit_manifest.sh"
INFLIGHT_ID = "20260905-il6-selfref-probe"

# 已知的历史不可复核记录（IL-5 / ISSUE #5596）：这些是历史判据的红，
# 修自指不允许把它们变绿。清单 = 改动前实测的 7 条（见归档 run 的对照）。
HIST_RED = [
    "iaia410-5112-evidence-20261015",
    "iaia410-a01-20261011T062229Z-609a8e81",
    "iaia410-a02-20261011T070228Z-609a8e81",
    "iaia410-a03-20261011T070845Z-omp",
    "iaia410-a04-20261011T180837Z-609a8e81",
    "iaia410-a05-20261012T004948Z-bf48818c",
    "iaia410-a06-r1-20261011T073325Z-01df92a5",
]


def run_gate(gate_path=None):
    """跑门禁，返回 (rc, stdout+stderr)。"""
    cmd = ["bash", str(gate_path or GATE)]
    p = subprocess.run(cmd, cwd=str(REPO), capture_output=True, text=True, timeout=600)
    return p.returncode, p.stdout + p.stderr


def failed_runs(output):
    """从门禁输出里取出被判红的 run_id 集合。

    门禁有两种红：
      FAIL: audit run <id> failed evidence verify
      FAIL: run dir without manifest: <id>
    """
    ids = set()
    for m in re.finditer(r"^FAIL: audit run (\S+) failed evidence verify", output, re.M):
        ids.add(m.group(1))
    for m in re.finditer(r"^FAIL: run dir without manifest: (\S+)", output, re.M):
        ids.add(m.group(1))
    return ids


def make_inflight_run(run_dir):
    """造一个「正在执行中」的 run：有 manifest、无 results/evidence_hash。"""
    run_dir.mkdir(parents=True, exist_ok=True)
    m = {
        "schema_version": 1,
        "run_id": run_dir.name,
        "source_agent": "il6-selfref-probe",
        "frozen_commit": "0" * 40,
        "environment": {},
        "artifacts": {},
        "commands": [{"id": "probe", "name": "probe", "cmd": "true",
                      "cwd": ".", "timeout_sec": 1}],
    }
    import yaml
    (run_dir / "manifest.yaml").write_text(
        yaml.safe_dump(m, sort_keys=False, allow_unicode=True, width=100),
        encoding="utf-8")
    return run_dir


def check_inflight_not_red():
    """1) 未提交的在飞 run 不得被判红。"""
    run_dir = AUDIT / "runs" / INFLIGHT_ID
    try:
        make_inflight_run(run_dir)
        rc, out = run_gate()
        reds = failed_runs(out)
        ok = INFLIGHT_ID not in reds
        print(f"{'OK  ' if ok else 'BAD '} inflight-not-red: rc={rc} "
              f"in-flight run red={INFLIGHT_ID in reds} "
              f"(total reds={len(reds)})")
        if not ok:
            print("     在飞 run 仍被判红 ⇒ 自指未消除")
        return ok
    finally:
        shutil.rmtree(run_dir, ignore_errors=True)


def check_worktree_scan_mutation():
    """2) 反向变异：把枚举改回扫工作区目录 ⇒ 在飞 run 必须重新被判红。"""
    base = GATE.read_text(encoding="utf-8")
    # 锚点：已提交树枚举的那一行
    anchor = 'if ! tracked="$(git -C "${REPO_ROOT}" ls-tree -r --name-only HEAD -- audit/v4.1.0 2>/dev/null)"; then'
    if base.count(anchor) != 1:
        print(f"DRIFT worktree-scan-mutation: anchor count={base.count(anchor)} != 1")
        return None
    # 变异：把已提交树换回工作区文件枚举（还原修复前的行为：
    # 工作区里未提交的在飞 run 也会被算进判定）
    mutant = base.replace(
        anchor,
        'if ! tracked="$(cd "${REPO_ROOT}" && find audit/v4.1.0 -type f '
        '-not -path "*/__pycache__/*" | sort)"; then')
    # 变异体必须放在仓内 scripts/gate/ 下：门禁用 BASH_SOURCE 推导 REPO_ROOT，
    # 放别处会推错仓根。文件名不用 check_ 前缀，免得被门禁清单的 glob 收进去。
    mgate = REPO / "scripts" / "gate" / ".il6_mutant_gate.sh"
    try:
        mgate.write_text(mutant, encoding="utf-8")
        run_dir = AUDIT / "runs" / INFLIGHT_ID
        try:
            make_inflight_run(run_dir)
            rc, out = run_gate(mgate)
            reds = failed_runs(out)
            caught = INFLIGHT_ID in reds
            print(f"{'OK  ' if caught else 'BAD '} worktree-scan-mutation: rc={rc} "
                  f"in-flight run red={caught} (total reds={len(reds)})")
            if not caught:
                print("     变异体没把在飞 run 算成红 ⇒ 本判据不可反向变异")
            return caught
        finally:
            shutil.rmtree(run_dir, ignore_errors=True)
    finally:
        try:
            mgate.unlink()
        except OSError:
            pass


def check_history_not_relaxed():
    """3) 历史判据不得被放宽：已知的历史不可复核记录仍须判红。"""
    rc, out = run_gate()
    reds = failed_runs(out)
    still_red = [r for r in HIST_RED if r in reds]
    ok = len(still_red) == len(HIST_RED)
    print(f"{'OK  ' if ok else 'BAD '} history-not-relaxed: rc={rc} "
          f"历史红 {len(still_red)}/{len(HIST_RED)} (total reds={len(reds)})")
    if not ok:
        missing = sorted(set(HIST_RED) - set(reds))
        print(f"     历史判据被放宽，这些不再判红: {missing}")
    return ok


def check_exclude_run_option():
    """4) --exclude-run 处置方案：排除必须**显式生效**，且不能顺手放过别的 run。

    断言：排除一个 run ⇒ 只有那一条从红变 SKIP，其余红的条数不变。
    （排除不是「让门禁变绿」的后门。）
    """
    rc_all, out_all = run_gate()
    reds_all = failed_runs(out_all)
    if not reds_all:
        print("BAD  exclude-run-option: 没有红可排除，判据无判别力")
        return False
    victim = sorted(reds_all)[0]
    p = subprocess.run(["bash", str(GATE), "--exclude-run", victim],
                       cwd=str(REPO), capture_output=True, text=True, timeout=600)
    out = p.stdout + p.stderr
    reds_excl = failed_runs(out)
    skipped = victim in out and "SKIP" in out
    # 排除的必须变成不红；其余红的必须一条不少
    ok = (victim not in reds_excl) and skipped and (reds_excl == reds_all - {victim})
    print(f"{'OK  ' if ok else 'BAD '} exclude-run-option: victim={victim} "
          f"reds {len(reds_all)} -> {len(reds_excl)} skipped={skipped}")
    if not ok:
        if victim in reds_excl:
            print("     排除后仍然判红 ⇒ --exclude-run 没生效")
        if reds_excl != reds_all - {victim}:
            print(f"     排除影响了别的 run: {sorted(reds_all - {victim} - reds_excl)}")
    return ok


def main() -> int:
    results = {
        "inflight-not-red": check_inflight_not_red(),
        "worktree-scan-mutation": check_worktree_scan_mutation(),
        "history-not-relaxed": check_history_not_relaxed(),
        "exclude-run-option": check_exclude_run_option(),
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
