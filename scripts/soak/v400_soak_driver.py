#!/usr/bin/env python3
"""Mixed read/write driver for v4.0.0 server-read-perf SOAK validation.

Validates:
  - D.1 PK column from TableInfo (no more 'id' hard-coding)
  - D.3 single-flush wire encode (no_flush packet variants)
  - C.1 FileStorage write_lock refactor (17 inherent methods + internal lock)

Workload mix:
  - 50% PK point-lookup:  SELECT c, pad FROM sbtest1 WHERE id = ?
  - 20% PK point-update:  UPDATE sbtest1 SET k = k + 1, c = ? WHERE id = ?
  - 10% Range scan:       SELECT id, c FROM sbtest1 WHERE id BETWEEN ? AND ?+99
  - 10% Count aggregate:  SELECT COUNT(*), SUM(k) FROM sbtest1
  - 10% Insert new row:   INSERT INTO sbtest1 (id, k, c, pad) VALUES (?, ?, ?, ?)

Each worker thread opens a persistent pymysql connection and runs the mix
in a tight loop until duration expires. Errors trigger reconnect with
exponential backoff (50ms → 2s cap).

Emits one CSV line per query to --output (default stdout):
    Q,<elapsed_ms>,<thread_id>,<query_kind>,<row_count>
    E,<elapsed_ms>,<thread_id>,<query_kind>,<error_msg>

Tunable:
    --threads=N        default 16
    --duration=S       total run time in seconds (required)
    --host H           default 127.0.0.1
    --port P           default 3306
    --user U           default root
    --password W       default empty
    --table T          default sbtest1
    --max-id N         default 10000 (rows in table)
    --read-pct PCT     default 50 (% reads; remainder is writes)
    --output PATH      default ./driver.log
"""
import argparse
import random
import sys
import threading
import time
from typing import Optional, Tuple

try:
    import pymysql
except ImportError:
    print("FATAL: pymysql not installed. pip install pymysql", file=sys.stderr)
    sys.exit(2)


# ---- work templates (mixed read/write) ----

def _rand_id(max_id: int) -> int:
    return random.randint(1, max_id)


def q_point_select(table: str, max_id: int) -> Tuple[str, str]:
    rid = _rand_id(max_id)
    return (f"SELECT c, pad FROM {table} WHERE id = {rid}", "point_select")


def q_point_update(table: str, max_id: int) -> Tuple[str, str]:
    rid = _rand_id(max_id)
    new_c = "x" * random.randint(5, 30)
    return (f"UPDATE {table} SET k = k + 1, c = '{new_c}' WHERE id = {rid}", "point_update")


def q_range_scan(table: str, max_id: int) -> Tuple[str, str]:
    start = random.randint(1, max(max_id - 100, 1))
    end = start + 99
    return (f"SELECT id, c FROM {table} WHERE id BETWEEN {start} AND {end}", "range_scan")


def q_count_agg(table: str, max_id: int) -> Tuple[str, str]:
    return (f"SELECT COUNT(*), SUM(k) FROM {table}", "count_agg")


def q_insert(table: str, max_id: int, thread_id: int = 0, insert_seq: int = 0) -> Tuple[str, str]:
    # Each worker thread owns a disjoint id range:
    #   id = max_id + thread_id + (insert_seq * 1024)
    # We start insert_seq at 1 (not 0) so the very first id is max_id + thread_id + 1024,
    # which never collides with the preloaded rows 1..max_id.
    rid = max_id + thread_id + ((insert_seq + 1) * 1024)
    c_val = "y" * random.randint(5, 30)
    pad_val = "p" * random.randint(5, 30)
    return (
        f"INSERT INTO {table} (id, k, c, pad) VALUES ({rid}, 0, '{c_val}', '{pad_val}')",
        "insert",
    )


# ---- per-thread worker --------------------------------------------------

def _thread_main(thread_id: int, host: str, port: int, user: str, password: str,
                 duration_s: float, output_fp, table: str, max_id: int,
                 read_pct: int, insert_pct: int) -> None:
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

    insert_seq = 0
    count = 0
    errs = 0
    try:
        while time.monotonic() < end_ts:
            t0 = time.monotonic()
            try:
                roll = random.randint(1, 100)
                if roll <= read_pct:
                    if roll <= 10:
                        sql, kind = q_count_agg(table, max_id)
                    elif roll <= 30:
                        sql, kind = q_range_scan(table, max_id)
                    else:
                        sql, kind = q_point_select(table, max_id)
                elif roll <= read_pct + insert_pct:
                    sql, kind = q_insert(table, max_id, thread_id, insert_seq)
                    insert_seq += 1
                else:
                    sql, kind = q_point_update(table, max_id)

                with conn.cursor() as cur:
                    cur.execute(sql)
                    try:
                        rows = cur.fetchall()
                    except Exception:
                        rows = []
                ms = (time.monotonic() - t0) * 1000.0
                row_count = len(rows) if rows else 0
                output_fp.write(f"Q,{ms:.1f},{thread_id},{kind},{row_count}\n")
                count += 1
                if count % 500 == 0:
                    output_fp.flush()
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
        output_fp.flush()
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
    ap.add_argument("--table", default="sbtest1")
    ap.add_argument("--max-id", type=int, default=10000,
                    help="rows in table (PK range upper bound)")
    ap.add_argument("--read-pct", type=int, default=80,
                    help="percentage of read queries (remainder is writes)")
    ap.add_argument("--insert-pct", type=int, default=10,
                    help="percentage of writes that are INSERT (rest are UPDATE)")
    ap.add_argument("--output", default=None)
    args = ap.parse_args()

    out = open(args.output, "w") if args.output else sys.stdout

    print(f"v400 soak driver: {args.threads} threads, {args.duration}s, "
          f"host={args.host}:{args.port}, table={args.table}, "
          f"read_pct={args.read_pct}, insert_pct={args.insert_pct}, "
          f"max_id={args.max_id}",
          file=sys.stderr)
    out.flush()

    threads = []
    for tid in range(args.threads):
        t = threading.Thread(
            target=_thread_main,
            args=(tid, args.host, args.port, args.user, args.password,
                  float(args.duration), out, args.table, args.max_id,
                  args.read_pct, args.insert_pct),
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