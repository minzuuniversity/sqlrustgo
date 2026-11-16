#!/usr/bin/env python3
"""SOAK 起跑复验用的**稳态窗口**统计量。

为什么从 metrics.csv 算，而不是 driver.log
------------------------------------------
驱动的 Q/E 行第 2 列是**单条查询自己的耗时**（毫秒），**不是**「驱动
启动以来的经过时间」：

    t0 = time.monotonic()                       # ← 取在循环体内，每条查询之前
    ms = (time.monotonic() - t0) * 1000.0
    output_fp.write(f"Q,{ms:.1f},...")

把它当成 elapsed 用，等于要求「单条查询跑了 30 秒」才把这一行算进稳态
—— 于是稳态区间恒为空集，复验恒判「0 q/s、错误率 100 %」。

**2026-11-17 实测**：Z6G4（192.168.0.252）上的 attempt-31 被这道仪器
误杀。那一轮其实健康（180 s 内 9194 条查询 ≈ 51 q/s、0 错误），却因为
读错时间基准被判成「0 q/s vs floor 10」，白跑一轮。

metrics.csv 才是带时间戳的权威口径：15 s 一采样，每行有 `elapsed_s`
（臂 A 启动以来的秒数）与**累计** `driver_q` / `driver_e`。

用法
----
    soak_steady_window.py <metrics.csv> [warmup_s]

输出（空格分隔）::

    <Q> <E> <SECS> <QPS> <ERR_PCT> <SUSTAINED>

- ``Q`` / ``E``   稳态窗口内**新增**的查询数 / 错误数
- ``SECS``        稳态窗口长度（秒）
- ``QPS``         稳态窗口内的查询吞吐
- ``ERR_PCT``     稳态窗口内的错误率（百分数）
- ``SUSTAINED``   1 = 稳态窗口够长（≥ 30 s，即驱动至少跑了 60 s）；0 = 不够

**读不出统计量时以非零退出码结束且不输出统计行** —— 「读不出来」不是
「达标」，调用方必须据此判红。
"""

from __future__ import annotations

import csv
import sys

# 稳态窗口至少要有这么长，判定才有意义。低于它 ⇒ 驱动多半还没跑热，
# 此时的吞吐数字不能代表「这台机器扛得住」。
MIN_STEADY_SECS = 30.0


def _read_rows(path: str, warmup: float) -> list[tuple[float, int, int]]:
    rows: list[tuple[float, int, int]] = []
    with open(path, newline="") as fh:
        for row in csv.DictReader(fh):
            try:
                elapsed = float(row.get("elapsed_s") or 0)
            except (TypeError, ValueError):
                continue
            if elapsed < warmup:
                continue
            try:
                q = int(float(row.get("driver_q") or 0))
                e = int(float(row.get("driver_e") or 0))
            except (TypeError, ValueError):
                continue
            rows.append((elapsed, q, e))
    return rows


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(
            "usage: soak_steady_window.py <metrics.csv> [warmup_s]",
            file=sys.stderr,
        )
        return 2
    path = argv[1]
    try:
        warmup = float(argv[2]) if len(argv) > 2 else 30.0
    except (TypeError, ValueError):
        print(f"FAIL: warmup is not a number: {argv[2]!r}", file=sys.stderr)
        return 2

    try:
        rows = _read_rows(path, warmup)
    except OSError as exc:
        print(f"FAIL: cannot read {path}: {exc}", file=sys.stderr)
        return 1

    if len(rows) < 2:
        print(
            f"FAIL: {path} has {len(rows)} steady-window sample(s) "
            f"(warmup={warmup}s) — need at least 2 to compute a rate",
            file=sys.stderr,
        )
        return 1

    t0, q0, e0 = rows[0]
    t1, q1, e1 = rows[-1]
    dq = q1 - q0
    de = e1 - e0
    dt = t1 - t0
    if dt <= 0:
        print(f"FAIL: steady window has zero length ({path})", file=sys.stderr)
        return 1

    total = dq + de
    qps = dq / dt
    err_pct = (100.0 * de / total) if total > 0 else 100.0
    sustained = 1 if dt >= MIN_STEADY_SECS else 0

    print(f"{dq} {de} {int(dt)} {qps:.1f} {err_pct:.3f} {sustained}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
