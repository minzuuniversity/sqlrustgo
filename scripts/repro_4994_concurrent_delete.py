import pymysql
#!/usr/bin/env python3
"""
repro_4994.py — #4994 的最小分辨实验。

原 issue 的 8 worker × 12 轮复现出「352 次操作全部成功、ROWS=0」。
但它同时改了两个变量（并发数 4→8、增加 DELETE 分支），无法判断
哪个是触发条件。

本脚本按单一变量扫：

    4 worker × 12 轮（只加 DELETE，不加并发）  ← 验收标准 1
    8 worker × 12 轮（原 issue 条件）

**不要吞错误。** 原脚本用了 `2>/dev/null`，让「全失败」伪装成
「无异常」——这正是 352 次操作全绿却丢数据却没被发现的直接原因。
这里每个操作的退出码与错误都进计数器。

用法:
    repro_4994.py <port> [workers] [rounds]
"""
import subprocess
import sys
import threading
from collections import Counter

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 3306
WORKERS = int(sys.argv[2]) if len(sys.argv) > 2 else 4
ROUNDS = int(sys.argv[3]) if len(sys.argv) > 3 else 12

EXPECT_ROWS = WORKERS * 8 + 1  # 每 worker 存活 8 行 + 1 seed
STATS = Counter()
STATS_LOCK = threading.Lock()


def sql(stmt):
    """Run one statement. Returns (ok, error_text). Never swallows.

    `db=None` means no database selected — CREATE DATABASE cannot run
    with a `-D` pointing at the database it is about to create.
    """
    # NOTE: this server ignores mysql's `-D` (USE is a no-op), so every
    # statement runs in the default database context.
    cmd = ["mysql", "-h", "127.0.0.1", "-P", str(PORT), "-u", "root",
           "-N", "-B", "-e", stmt]
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode == 0, (r.stderr or "").strip()[:160]


def worker(wid):
    # One long-lived connection per worker. Spawning a fresh `mysql`
    # process per statement exhausted the server's connection limit
    # (ERROR 2002 (36)) and every later statement silently failed —
    # which looks exactly like "the server works but returns nothing".
    conn = pymysql.connect(host="127.0.0.1", port=PORT, user="root",
                           autocommit=True)
    cur = conn.cursor()
    local = Counter()

    def q(stmt):
        try:
            cur.execute(stmt)
            return True, "" if cur.description is None else None
        except Exception as e:  # noqa: BLE001
            return False, str(e)[:160]
    for i in range(1, ROUNDS + 1):
        rid = wid * 1000 + i

        # 1) INSERT ... ON DUPLICATE KEY UPDATE
        ok, err = sql(
            f"INSERT INTO t (id, v) VALUES ({rid}, {wid}) "
            f"ON DUPLICATE KEY UPDATE v = v + 1"
        )
        local["ok" if ok else "err"] += 1
        if not ok:
            local["err:" + err[:60]] += 1

        # 2) UPDATE
        ok, err = q(f"UPDATE t SET v = v + 10 WHERE id = {rid}")
        local["ok" if ok else "err"] += 1
        if not ok:
            local["err:" + err[:60]] += 1

        # 3) SELECT
        ok, err = q(f"SELECT v FROM t WHERE id = {rid}")
        local["ok" if ok else "err"] += 1
        if not ok:
            local["err:" + err[:60]] += 1

        # 4) DELETE — only from i > 8, matching the original script
        if i > 8:
            ok, err = q(f"DELETE FROM t WHERE id = {rid}")
            local["ok" if ok else "err"] += 1
            if not ok:
                local["err:" + err[:60]] += 1

        # 5) explicit transaction
        ok, err = q("BEGIN")
        local["ok" if ok else "err"] += 1
        ok, err = q(f"UPDATE t SET v = v + 100 WHERE id = {rid}")
        local["ok" if ok else "err"] += 1
        if i % 2:
            ok, err = q("ROLLBACK")
        else:
            ok, err = q("COMMIT")
        local["ok" if ok else "err"] += 1
        if not ok:
            local["err:" + err[:60]] += 1

    with STATS_LOCK:
        for k, v in local.items():
            STATS[k] += v
    cur.close()
    conn.close()


def main():
    # fresh state
    sql("DROP TABLE IF EXISTS t")
    ok, err = sql("CREATE TABLE t (id INT PRIMARY KEY, v BIGINT)")
    if not ok:
        print(f"FATAL: create table failed: {err}")
        sys.exit(2)
    sql("CREATE TABLE t (id INT PRIMARY KEY, v BIGINT)")
    sql("INSERT INTO t (id, v) VALUES (0, 0)")  # seed row

    ts = [threading.Thread(target=worker, args=(w,)) for w in range(1, WORKERS + 1)]
    for t in ts:
        t.start()
    for t in ts:
        t.join()

    conn = pymysql.connect(host="127.0.0.1", port=PORT, user="root",
                           autocommit=True)
    cur = conn.cursor()
    cur.execute("SELECT COUNT(*) FROM t")
    rows = int(cur.fetchone()[0])
    cur.close()
    conn.close()

    print(f"workers={WORKERS} rounds={ROUNDS}")
    print(f"  operations ok   : {STATS['ok']}")
    print(f"  operations err  : {STATS['err']}")
    for k, v in sorted(STATS.items()):
        if k.startswith("err:") and v:
            print(f"    {k}: {v}")
    print(f"  ROWS             : {rows}   (expected {EXPECT_ROWS})")
    verdict = "MATCHES" if rows == EXPECT_ROWS else "MISMATCH"
    print(f"  verdict         : {verdict}")
    return 0 if rows == EXPECT_ROWS else 1


if __name__ == "__main__":
    sys.exit(main())
