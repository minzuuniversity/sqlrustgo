#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""soak_data_size_sampler.py — SOAK 的「表规模」证据轴采样器。

为什么需要它
------------
`docs/releases/v4.1.0/GA_GATE_REPORT.md` §7.25（`GA-SOAK-21`）写明，
168h SOAK 的合格证据需要「独占宿主 + 长时跑满 + 监控 **RSS/表规模/单查询 p99**」
三者一起。RSS 与单查询延迟分别由 `metrics.csv` 与 `driver.log` 覆盖，
**表规模**这一轴此前没有任何采集器。

本采样器补上它，且刻意做到**对被测系统零侵入**：

- **数据目录占用**用 `os.walk` 累加文件大小 ⇒ 只读文件系统，不碰数据库；
- **行数**改为**事后从驱动的 `driver.log` 推导**（`Q,…,insert,0` 行数），
  **不在采样时查库**；
- 采样结果**逐行 flush** 写 jsonl ⇒ 任何时刻读到都能数出已发生的账
  （与驱动 Q/E 行的逐行 flush 同一条纪律）。

⚠️ **为什么不在采样时查 `COUNT(*)`**（2026-11-19 实测踩到）：
被测服务端是 `--server-threads 16`，而臂 A 的驱动本身就是 `--threads 16`
⇒ **驱动已经把服务端的执行器占满了**。此时再插一条 `SELECT COUNT(*)`
就是第 17 条并发重查询：实测它在自己的 `read_timeout=30` 内**超时**，
而同一时刻服务端本身完全健康（q 稳定 23/s、panic 00、RSS 平稳）。
⇒ **采样器查库会污染被测系统本身**。行数轴改由 `driver.log` 事后推导，
采样器退化为**纯文件系统采样**，对被测系统零侵入。

它**不改变任何门禁判据**，也不输出 PASS/FAIL —— 只产出事实。

用法
----
    python3 scripts/soak/soak_data_size_sampler.py \
        --data-dir /tmp/sqlrustgo-v410-168h-<run_id> \
        --out /home/liying/soak-artifacts/<run_id>/data_size.jsonl \
        --interval 60

没有 pymysql 依赖 ⇒ 纯标准库。
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from typing import Optional


def dir_stats(path: str):
    """目录统计 ⇒ (占用字节, 文件数)。目录不存在返回 (None, None)（不猜）。"""
    if not path or not os.path.isdir(path):
        return None, None
    total = 0
    nfiles = 0
    try:
        for root, _dirs, files in os.walk(path):
            for fn in files:
                fp = os.path.join(root, fn)
                try:
                    total += os.stat(fp).st_size
                    nfiles += 1
                except OSError:
                    continue
    except OSError:
        return None, None
    return total, nfiles


def main(argv) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-dir", required=True, help="被测服务端的数据目录")
    ap.add_argument("--out", required=True, help="输出 jsonl 路径")
    ap.add_argument("--interval", type=float, default=60.0, help="采样间隔（秒）")
    args = ap.parse_args(argv)

    outdir = os.path.dirname(os.path.abspath(args.out))
    if outdir:
        os.makedirs(outdir, exist_ok=True)

    n = 0
    while True:
        now = time.time()
        n += 1
        size, nfiles = dir_stats(args.data_dir)
        rec = {
            "ts_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now)),
            "sample": n,
            "bytes": size,
            "files": nfiles,
            "rows": None,
            "rows_note": "not-collected-here（行数由 driver.log 的 insert 行事后推导）",
            "data_dir": args.data_dir,
        }
        with open(args.out, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
            f.flush()   # 逐行 flush：任何时刻读到都能数出已发生的账
        time.sleep(max(1.0, args.interval))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
