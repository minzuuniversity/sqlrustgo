#!/usr/bin/env python3
"""binmode_probe 产物的「锁值」核对（tripwire，不是门禁，也不参与 168 分母）。

作用：把 #5601 的**判定面**钉在 run 证据里。
  - 观测面缺一块（probe.json 不在、步骤没记录、读回行没有）⇒ 2（INCOMPLETE，没能核对）
  - 判定面变了 ⇒ 3，意思是「#5601 的结论已经过时」，必须回去改 #5601 与本步，
    而不是让证据悄悄漂掉。

**2026-11-06：锁值从「缺陷存在」翻到「缺陷已修」**（BIN-1 修复，PR 见本仓库历史）。
旧锁值锁的是 `false_success_count == 3`（三处「报成功但没生效」）。引擎修好后旧锁值
只会报「判定过时」，**并没有钉住「缺陷不再出现」** —— 也就是说缺陷一旦回归，没有任何
东西会拦。现在锁的是修复后的面：

  - `create_database_rc == 1` / `create_trigger_rc == 1` / `drop_trigger_rc == 1`
    ⇒ DDL 在 binary 模式**明确报错**，不再返回 OK；
  - `false_success_count == 0` ⇒ 零处「报成功但没生效」；
  - `create_database_effective == False`（binary 模式本来就没有库命名空间）
    且 `trigger_actually_fired_rows == []`（触发器被明确拒绝，不存在“该触发却没触发”）。

缺陷回归时（rc 又变 0、false_success_count 又变 3）本步立刻报红 —— 这才是锁值的作用。
"""
import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
# #5601 修复后的判定面（2026-11-06）。
CURRENT_DEFECTS = {
    "create_database_rc": 1,             # CREATE DATABASE 明确报错（不再是假成功）
    "create_trigger_rc": 1,              # CREATE TRIGGER 明确报错（不再是假成功）
    "drop_trigger_rc": 1,                # DROP TRIGGER 明确报错
    "false_success_count": 0,            # 零处「报成功但没生效」
    "create_database_effective": False,  # binary 模式没有库命名空间（与修复前一致）
    "trigger_actually_fired_rows": [],   # 触发器被明确拒绝，不存在该触发未触发
}


def main(argv) -> int:
    ap = argparse.ArgumentParser(prog="binmode_lock.py")
    ap.add_argument("--json", default="test_results/binmode/probe.json")
    a = ap.parse_args(argv)
    p = pathlib.Path(a.json)
    if not p.is_absolute():
        p = REPO / p
    if not p.is_file():
        print("INCOMPLETE: 找不到 %s（binmode-probe 那一步没跑成）" % p, file=sys.stderr)
        return 2
    d = json.loads(p.read_text(encoding="utf-8"))

    incomplete = []
    if not d.get("trigger_self_admission_log_lines"):
        incomplete.append("服务端自认日志行为空：'silently skipped' 这一维没观测到（日志没落盘或级别不对）")
    if not d.get("steps"):
        incomplete.append("steps 为空：这一 run 根本没有逐条 SQL 的 rc/输出记录")
    if not d.get("final_rows"):
        incomplete.append("final_rows 为空：触发器是否生效无法核对")

    diff = []
    for key, want in CURRENT_DEFECTS.items():
        got = d.get(key)
        if key not in d:
            incomplete.append("json 缺字段 %s" % key)
        elif got != want:
            diff.append("%s: 锁的是 %r，现在是 %r" % (key, want, got))

    brief = {k: d.get(k) for k in list(CURRENT_DEFECTS) + [
        "create_database_rc", "show_databases_before", "show_databases_after",
        "create_trigger_rc", "trigger_self_admission_log_lines"]}
    brief["kind"] = "binmode_lock"
    brief["json"] = str(p.relative_to(REPO)) if p.is_relative_to(REPO) else str(p)
    print(json.dumps(brief, ensure_ascii=False, indent=1))

    if incomplete:
        print("NOT-OK: INCOMPLETE —— " + "; ".join(incomplete), file=sys.stderr)
        return 2
    if diff:
        print("NOT-OK: 缺陷面与 #5601 记录不一致（引擎行为变了）—— " + "; ".join(diff), file=sys.stderr)
        print("  ⇒ 回去更新 #5601 与 commands_cap_002.yaml 的本步锁值，别把证据漂掉。", file=sys.stderr)
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
