#!/usr/bin/env python3
"""gov_probe 判据夹具（离线，无网络、无凭据）。

存在的意义：gov_probe.py 是"给文档数字当出处"的工具，它自己必须先可证伪。
两条反向变异（AGENTS §2.5）：
  M1  把优先级标签从 `priority/p0` 改回裸 `P0`（或删掉 priority/ 前缀匹配）
      ⇒ 本夹具必须变红（真实标签形态被钉死）。
  M2  把 summarize_runs 的成功判定从 `conclusion` 改回 `status`
      ⇒ 本夹具必须变红（终态 status 是 completed，成功会被记成 0）。
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gov_probe import gate_refs_in, issue_row, summarize_runs  # noqa: E402

FAILS: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILS.append(msg)


def main() -> int:
    # --- issue_row：真实标签形态（M1 的钉子） ---
    real = {
        "number": 5583, "title": "x",
        "labels": [{"name": "priority/p2"}, {"name": "cost/t1"},
                   {"name": "cap/governance"}, {"name": "state/claimed"}],
        "milestone": None, "assignees": [], "comments": 0,
    }
    row = issue_row(real)
    check(row["priority"] == ["priority/p2"],
          f"M1: priority/p2 必须被识别，实际 {row['priority']}")
    check(row["cost"] == ["cost/t1"], f"cost 口径漂移: {row['cost']}")
    check(row["cap"] == ["cap/governance"], f"cap 口径漂移: {row['cap']}")
    check("state/claimed" in row["labels"], "labels 必须保留全量标签")

    # 字符串形态标签（API 变体）也必须工作
    row2 = issue_row({"number": 1, "title": "t", "labels": ["priority/p0"]})
    check(row2["priority"] == ["priority/p0"],
          f"字符串标签形态未覆盖: {row2['priority']}")

    # 无优先级 ⇒ 必须是显式 (none)，不能是空列表（空列表会让汇总漏计）
    row3 = issue_row({"number": 2, "title": "t", "labels": ["cap/build"]})
    check(row3["priority"] == ["(none)"],
          f"缺优先级必须显式 (none)，实际 {row3['priority']}")

    # --- summarize_runs：conclusion 口径（M2 的钉子） ---
    def run(i, conclusion, started, completed):
        return {"id": i, "status": "completed", "conclusion": conclusion,
                "started_at": started, "completed_at": completed,
                "created_at": started}

    base = [
        run(1, "success", "2026-10-01T00:00:00Z", "2026-10-01T00:10:00Z"),
        run(2, "failure", "2026-10-01T01:00:00Z", "2026-10-01T01:30:00Z"),
        run(3, "success", "2026-10-01T02:00:00Z", "2026-10-01T02:20:00Z"),
        run(4, "cancelled", "2026-10-01T03:00:00Z", "2026-10-01T03:05:00Z"),
    ]
    s = summarize_runs(base)
    check(s["runs"] == 4, f"runs 计数错: {s['runs']}")
    check(s["conclusions"].get("success") == 2,
          f"M2: success 必须按 conclusion 计数，实际 {s['conclusions']}")
    check(s["success_rate_pct"] == 50.0, f"成功率错: {s['success_rate_pct']}")
    check(s["dur_median_s"] in (600, 1200, 900), f"中位数不可解释: {s['dur_median_s']}")

    # 反向变异 M2：只看 status 的实现会得到 0 成功 ⇒ 上面的断言必然红。
    # 这里显式构造 status-only 的输入，证明 status 本身不足以判定成功。
    status_only = [{"id": 1, "status": "completed"}]
    s2 = summarize_runs(status_only)
    check(s2["conclusions"].get("success", 0) == 0,
          "status=completed 不得被当成 success（终态语义）")

    # 挂死样本（>3h）不得进入分位
    hung = base + [run(9, "failure", "2026-10-01T04:00:00Z", "2026-10-01T12:00:00Z")]
    s3 = summarize_runs(hung)
    check(s3["dur_n"] == 4, f">3h 挂死样本必须剔除，dur_n={s3['dur_n']}")
    check(s3["runs"] == 5, f"剔除分位不影响 runs 分母，runs={s3['runs']}")

    # 空输入不得炸，也不得给出成功率
    s4 = summarize_runs([])
    check(s4["runs"] == 0 and s4["success_rate_pct"] is None,
          f"空输入语义错: {s4}")

    # 引用识别：散文里提到门禁名 ≠ 有代码调用它（实测过：本仓探针自己就因此
    # 把死门禁从 37 虚降到 36）。P1/P2 是可证伪的一对。
    from gate_executability import GATE_PATH_RE
    py_doc = (
        "#!/usr/bin/env python3\n"
        '"""usage: 参见 scripts/gate/check_prose_only_mention.sh '
        '与 scripts/gate/check_py_docstring_mention.sh"""\n'
        "print('hello')\n"
    )
    got = gate_refs_in("scripts/gate/tool.py", py_doc, GATE_PATH_RE)
    check(got == set(), f"P1: 文档字符串里的门禁名不得算引用，实际 {sorted(got)}")

    sh_real = (
        "#!/bin/bash\n"
        "# 上一版调用过 scripts/gate/check_sh_comment_mention.sh（已移除）\n"
        "bash scripts/gate/check_sh_real_call.sh\n"
    )
    got2 = gate_refs_in("scripts/gate/wrapper.sh", sh_real, GATE_PATH_RE)
    check(got2 == {"scripts/gate/check_sh_real_call.sh"},
          f"P2: 真调用必须计入、注释不得计入，实际 {sorted(got2)}")

    if FAILS:
        for f in FAILS:
            print("FAIL:", f)
        print(f"gov_probe fixtures: {len(FAILS)} FAIL")
        return 1
    print("gov_probe fixtures: OK (issue_row 标签口径 + summarize_runs conclusion/分位口径 + gate_refs_in 引用口径 + 2 条反向变异)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
