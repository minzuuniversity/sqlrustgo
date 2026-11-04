#!/usr/bin/env python3
"""expert_triage 批量解析夹具（离线，无席位、无网络）。

存在的意义：批量分诊把"一次问一条"换成"一次问十条"，
最容易出的假象是**席位漏答而工具替它补上**——那正是反虚假执行要防的形态。
反向变异：
  M1  删掉返回数组里的一个对象 ⇒ missing 必须非空（工具不得凭空补出该编号）
  M2  让工具对范围外编号照单全收 ⇒ 本夹具必须红（extra 必须被报告且丢弃）
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from expert_triage import parse_batch_output  # noqa: E402

FAILS = []


def check(cond, msg):
    if not cond:
        FAILS.append(msg)


OBJ = ('{{"number":{n},"domain":"gate","breaks_claim":"c{n}","automatable":"auto",'
       '"blast":"small","fix_shape":"localized","dup_of":null}}')


def arr(nums):
    return "[" + ",".join(OBJ.format(n=n) for n in nums) + "]"


def main() -> int:
    nums = [5590, 5591, 5592]

    got, missing, extra = parse_batch_output(arr(nums), nums)
    check(sorted(got) == nums, f"完整批量应答必须全部解析，实际 {sorted(got)}")
    check(missing == [] and extra == [], f"完整应答不该有缺失/多余: {missing} {extra}")
    check(got[5591]["domain"] == "gate", "字段必须原样保留")

    # M1：少一个对象 ⇒ 必须报缺，且不得出现在 got
    truncated = arr(nums[:2])
    got2, missing2, _ = parse_batch_output(truncated, nums)
    check(5592 in missing2, "M1: 漏答的编号必须进 missing")
    check(5592 not in got2, "M1: 漏答的编号不得被补齐")

    # M2：范围外编号 ⇒ 必须进 extra 且不进 got
    polluted = arr(nums + [999999])
    got3, missing3, extra3 = parse_batch_output(polluted, nums)
    check(999999 in extra3, f"M2: 范围外编号必须报 extra，实际 {extra3}")
    check(999999 not in got3, "M2: 范围外编号不得进入台账")

    # 席位带解释文字 / ```json 包裹 ⇒ 仍需恢复
    noisy = "好的：\n```json\n" + arr(nums) + "\n```\n以上。"
    got4, missing4, _ = parse_batch_output(noisy, nums)
    check(sorted(got4) == nums, f"带围栏/前后缀必须恢复，实际 {sorted(got4)}")

    # 完全不可解析 ⇒ 全部缺，绝不产出任何 triage
    got5, missing5, _ = parse_batch_output("我不知道，先这样吧。", nums)
    check(got5 == {} and missing5 == nums, f"不可解析时不得编造: {got5}")

    # 单个对象但期望多条 ⇒ 仍按缺处理
    got6, missing6, _ = parse_batch_output(OBJ.format(n=5590), nums)
    check(sorted(got6) == [5590] and 5591 in missing6, "dict 单对象也要能解析且不补数")

    if FAILS:
        for f in FAILS:
            print("FAIL:", f)
        print(f"expert_triage fixtures: {len(FAILS)} FAIL")
        return 1
    print("expert_triage fixtures: OK (6 组断言 + M1/M2 反向变异语义)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
