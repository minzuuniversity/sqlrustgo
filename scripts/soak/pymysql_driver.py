#!/usr/bin/env python3
"""pymysql driver for the wired SOAK harness (read-only).

N worker threads, each opens a persistent pymysql connection to the
sqlrustgo-mysql-server running at host:port. Each worker runs SELECT
workloads in a tight loop until duration expires.

sqlrustgo --storage binary does not support INSERT in this build, so
this driver is read-only by design. Combined with concurrent connections
this still exercises:
  * TLS handshake under churn (reconnect on errors)
  * SELECT result-set transmission (column counts, types, rows)
  * Worker-pool dispatch (#3657 bounded backpressure)
  * WAL/log churn

Emits one CSV line per query to --output (default stdout):
    Q,<elapsed_ms>,<thread_id>,<query_kind>,<row_count>
    E,<elapsed_ms>,<thread_id>,query,<error_msg>

Tunable:
    --threads=N        default 16
    --duration=S       total run time in seconds (required)
    --host H           default 127.0.0.1
    --port P           default 3306
    --user U           default root
    --password W       default empty
    --output PATH      default ./driver.log
"""
import argparse
import random
import sys
import threading
import time
from typing import Optional

try:
    import pymysql
except ImportError:
    print("FATAL: pymysql not installed. pip install pymysql", file=sys.stderr)
    sys.exit(2)


# ---- work templates (READ-only; works against binary storage) ----

def q_count() -> str:
    return "SELECT COUNT(*) FROM lineitem"


def q_sample() -> str:
    return ("SELECT l_orderkey, l_partkey, l_quantity, l_extendedprice, l_discount "
            "FROM lineitem ORDER BY l_orderkey DESC LIMIT 5")


def q_aggregate() -> str:
    return ("SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, "
            "SUM(l_extendedprice) AS sum_base_price, COUNT(*) AS count_order "
            "FROM lineitem WHERE l_shipdate <= '1998-09-02' "
            "GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus")


def q_customer() -> str:
    return ("SELECT c_nationkey, COUNT(*) AS cnt, AVG(c_acctbal) AS avg_bal "
            "FROM customer GROUP BY c_nationkey ORDER BY c_nationkey")


def q_orders_date() -> str:
    return ("SELECT o_orderpriority, COUNT(*) AS order_count "
            "FROM orders WHERE o_orderdate >= '1993-07-01' "
            "AND o_orderdate < '1993-10-01' "
            "GROUP BY o_orderpriority ORDER BY o_orderpriority")


READ_QUERIES = [q_count, q_sample, q_aggregate, q_customer, q_orders_date]


# ---- per-thread worker --------------------------------------------------

def _thread_main(thread_id: int, host: str, port: int, user: str, password: str,
                 duration_s: float, output_fp) -> None:
    end_ts = time.monotonic() + duration_s
    backoff = 0.05
    max_backoff = 2.0

    def connect():
        return pymysql.connect(
            host=host, port=port, user=user, password=password,
            connect_timeout=5, read_timeout=15, write_timeout=15,
            autocommit=True, charset="utf8mb4",
        )

    conn: Optional[pymysql.connections.Connection] = None
    try:
        conn = connect()
    except Exception as e:
        output_fp.write(f"E,0,{thread_id},connect,{str(e)[:200]}\n")
        output_fp.flush()
        return

    count = 0
    errs = 0
    try:
        while time.monotonic() < end_ts:
            t0 = time.monotonic()
            try:
                q_fn = random.choice(READ_QUERIES)
                sql = q_fn()
                with conn.cursor() as cur:
                    cur.execute(sql)
                    rows = cur.fetchall()
                ms = (time.monotonic() - t0) * 1000.0
                row_count = len(rows) if rows else 0
                output_fp.write(f"Q,{ms:.1f},{thread_id},{q_fn.__name__},{row_count}\n")
                count += 1
                backoff = 0.05
            except Exception as e:
                ms = (time.monotonic() - t0) * 1000.0
                err_str = str(e).split("\n", 1)[0][:200]
                output_fp.write(f"E,{ms:.1f},{thread_id},query,{err_str}\n")
                errs += 1
                time.sleep(min(backoff, max_backoff))
                backoff = min(backoff * 2.0, max_backoff)
                try:
                    if conn is not None:
                        conn.close()
                except Exception:
                    pass
                try:
                    conn = connect()
                except Exception:
                    pass
        output_fp.write(f"# thread {thread_id} done: {count} Q, {errs} E\n")
    finally:
        try:
            if conn is not None:
                conn.close()
        except Exception:
            pass


# ---- main ---------------------------------------------------------------

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=3306)
    ap.add_argument("--user", default="root")
    ap.add_argument("--password", default="")
    ap.add_argument("--threads", type=int, default=16)
    ap.add_argument("--duration", type=int, required=True,
                    help="total run time in seconds")
    ap.add_argument("--output", default=None)
    args = ap.parse_args()

    out = open(args.output, "w") if args.output else sys.stdout

    print(f"pymysql driver: {args.threads} threads, {args.duration}s, "
          f"host={args.host}:{args.port}",
          file=sys.stderr)
    out.flush()

    threads = []
    for tid in range(args.threads):
        t = threading.Thread(
            target=_thread_main,
            args=(tid, args.host, args.port, args.user, args.password,
                  float(args.duration), out),
            daemon=True,
        )
        t.start()
        threads.append(t)

    for t in threads:
        t.join()
    out.flush()
    if out is not sys.stdout:
        out.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
