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
- **行数**用一条 `SELECT COUNT(*)`，默认**每 15 分钟**一次，且
  **失败即跳过**（不改判据、不重试打爆、不影响被测负载）；
- 采样结果**逐行 flush** 写 jsonl ⇒ 任何时刻读到都能数出已发生的账
  （与驱动 Q/E 行的逐行 flush 同一条纪律）。

它**不改变任何门禁判据**，也不输出 PASS/FAIL —— 只产出事实。

用法
----
    python3 scripts/soak/soak_data_size_sampler.py \
        --data-dir /tmp/sqlrustgo-v410-168h-<run_id> \
        --out /home/liying/soak-artifacts/exclusive-window/data_size.jsonl \
        --interval 60 \
        [--host 127.0.0.1 --port 3412 --user root --table sbtest1 \
         --rows-every 900 --password-env SOAK_MYSQL_PASSWORD]

没有 pymysql 时只采数据目录占用，行数轴记为 ``null``（**如实报 NOT-COLLECTED**，
不猜、不填）。
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from typing import Optional


def dir_bytes(path: str) -> Optional[int]:
    """目录占用（bytes）。目录不存在返回 None（不猜）。"""
    if not path or not os.path.isdir(path):
        return None
    total = 0
    try:
        for root, _dirs, files in os.walk(path):
            for fn in files:
                fp = os.path.join(root, fn)
                try:
                    total += os.stat(fp).st_size
                except OSError:
                    continue
    except OSError:
        return None
    return total


def count_rows(host: str, port: int, user: str, password: str, table: str) -> Optional[int]:
    """SELECT COUNT(*) —— 失败返回 None（失败即跳过，不重试）。"""
    try:
        import pymysql  # type: ignore
    except Exception:
        return None
    try:
        conn = pymysql.connect(host=host, port=port, user=user,
                               password=password, database="default",
                               connect_timeout=10, read_timeout=30,
                               write_timeout=30)
        try:
            with conn.cursor() as cur:
                cur.execute("SELECT COUNT(*) FROM `%s`" % table.replace("`", ""))
                row = cur.fetchone()
                return int(row[0]) if row else None
        finally:
            conn.close()
    except Exception:
        return None


def main(argv) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-dir", required=True, help="被测服务端的数据目录")
    ap.add_argument("--out", required=True, help="输出 jsonl 路径")
    ap.add_argument("--interval", type=float, default=60.0, help="采样间隔（秒）")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=0)
    ap.add_argument("--user", default="root")
    ap.add_argument("--password", default="")
    ap.add_argument("--password-env", default="",
                    help="从该环境变量读密码（不把密码写进命令行/进程表）")
    ap.add_argument("--table", default="sbtest1")
    ap.add_argument("--rows-every", type=float, default=900.0,
                    help="行数查询间隔（秒）；0 = 不查行数")
    args = ap.parse_args(argv)

    password = args.password
    if args.password_env:
        password = os.environ.get(args.password_env, "")
    if not password and args.password_env:
        print("WARN: 环境变量 %s 为空 ⇒ 行数轴将记为 null（如实报 NOT-COLLECTED）"
              % args.password_env, file=sys.stderr)

    outdir = os.path.dirname(os.path.abspath(args.out))
    if outdir:
        os.makedirs(outdir, exist_ok=True)

    last_rows_at = 0.0
    rows: Optional[int] = None
    rows_note = "not-collected"
    n = 0
    while True:
        now = time.time()
        n += 1
        size = dir_bytes(args.data_dir)
        if args.port and args.rows_every > 0 and (now - last_rows_at) >= args.rows_every:
            last_rows_at = now
            r = count_rows(args.host, args.port, args.user, password, args.table)
            if r is None:
                rows = None
                rows_note = "query-failed-or-pymysql-missing"
            else:
                rows = r
                rows_note = "ok"
        rec = {
            "ts_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now)),
            "sample": n,
            "bytes": size,
            "rows": rows,
            "rows_note": rows_note,
            "data_dir": args.data_dir,
        }
        with open(args.out, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
            f.flush()   # 逐行 flush：任何时刻读到都能数出已发生的账
        time.sleep(max(1.0, args.interval))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
