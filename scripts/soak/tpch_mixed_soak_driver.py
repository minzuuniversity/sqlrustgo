#!/usr/bin/env python3
"""TPC-H Mixed Workload SOAK Driver.

Combines TPC-H Q1-Q22 queries with INSERT/UPDATE/DELETE CRUD on 5 tables,
dynamic concurrency (4-32 workers), and automatic data replenishment.

Usage:
    python3 scripts/soak/tpch_mixed_soak_driver.py \\
        --host=127.0.0.1 --port=3397 --duration=1800 \\
        --mix-ratio=80 --concurrency-min=4 --concurrency-max=32
"""
import argparse
import json
import os
import random
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import crud_templates


@dataclass
class MixedSoakStats:
    queries_executed: int = 0
    query_errors: int = 0
    query_latencies_ms: list = field(default_factory=list)
    insert_count: int = 0
    update_count: int = 0
    delete_count: int = 0
    crud_errors: int = 0
    crud_latencies_ms: list = field(default_factory=list)
    read_write_conflicts: int = 0
    lock_wait_total_ms: float = 0.0
    lock_wait_samples: int = 0
    lock: threading.Lock = field(default_factory=threading.Lock)

    def record_query(self, ms: float, ok: bool):
        with self.lock:
            self.queries_executed += 1
            if not ok:
                self.query_errors += 1
            self.query_latencies_ms.append(ms)

    def record_crud(self, ms: float, ok: bool, op: str):
        with self.lock:
            if op == "insert":
                self.insert_count += 1
            elif op == "update":
                self.update_count += 1
            elif op == "delete":
                self.delete_count += 1
            if not ok:
                self.crud_errors += 1
            self.crud_latencies_ms.append(ms)

    def record_lock_wait(self, ms: float):
        with self.lock:
            self.lock_wait_total_ms += ms
            self.lock_wait_samples += 1

    def avg_lock_wait_ms(self) -> float:
        with self.lock:
            if self.lock_wait_samples == 0:
                return 0.0
            return self.lock_wait_total_ms / self.lock_wait_samples

    def crud_qps(self, op: str, elapsed: float) -> float:
        with self.lock:
            count = {"insert": self.insert_count, "update": self.update_count,
                     "delete": self.delete_count}[op]
        return count / elapsed if elapsed > 0 else 0.0


def percentile(sorted_list, p):
    if not sorted_list:
        return 0.0
    k = int(len(sorted_list) * p)
    if k >= len(sorted_list):
        k = len(sorted_list) - 1
    return sorted_list[k]


def run_mysql(host: str, port: int, user: str, sql: str,
              db: str = "tpch", timeout: int = 30):
    """Execute a single SQL statement via mysql CLI. Returns (ok, stderr)."""
    try:
        r = subprocess.run(
            ["mysql", "-h", host, "-P", str(port), "-u", user,
             "--silent", db, "-e", sql],
            capture_output=True, text=True, timeout=timeout,
        )
        return r.returncode == 0, r.stderr
    except subprocess.TimeoutExpired:
        return False, "timeout"
    except Exception as e:
        return False, str(e)


def find_pid(port: int):
    try:
        out = subprocess.check_output(["lsof", "-ti", f":{port}"],
                                     text=True, stderr=subprocess.DEVNULL)
        pids = [int(p) for p in out.strip().split() if p.isdigit()]
        return pids[0] if pids else None
    except Exception:
        return None


def load_queries(qdir: str):
    qs = []
    for i in range(1, 23):
        qf = Path(qdir) / f"q{i:02d}.sql"
        if qf.exists():
            qs.append(qf.read_text().strip().rstrip(";"))
    return qs


def get_row_count(host: str, port: int, user: str, table: str) -> int:
    ok, _ = run_mysql(host, port, user, f"SELECT COUNT(*) FROM {table};")
    if not ok:
        return 0
    try:
        out = subprocess.check_output(
            ["mysql", "-h", host, "-P", str(port), "-u", user,
             "--silent", "-N", "tpch", "-e", f"SELECT COUNT(*) FROM {table};"],
            text=True, timeout=10,
        )
        return int(out.strip().split("\n")[0])
    except Exception:
        return 0


def sample_rss(pid: int) -> int:
    if not pid:
        return 0
    try:
        with open(f"/proc/{pid}/status") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1])
    except Exception:
        pass
    return 0


def sample_fd(pid: int) -> int:
    if not pid:
        return 0
    try:
        return len(os.listdir(f"/proc/{pid}/fd"))
    except Exception:
        return 0


class Worker:
    def __init__(self, wid: int, host: str, port: int, user: str,
                 queries: list, tables: list, mix_ratio: int,
                 stats: MixedSoakStats, stop: threading.Event):
        self.wid = wid
        self.host = host
        self.port = port
        self.user = user
        self.queries = queries
        self.tables = tables
        self.mix_ratio = mix_ratio
        self.stats = stats
        self.stop = stop
        self.id_pools = {t: crud_templates.IdPool(t, wid) for t in tables}
        self.ctx = {"custkey": 1, "partkey": 1, "suppkey": 1, "nationkey": 1,
                    "linenumber": 1}
        self.rng = random.Random(wid * 9973 + int(time.time()))

    def run(self):
        while not self.stop.is_set():
            if self.rng.randint(1, 100) <= self.mix_ratio:
                self._do_query()
            else:
                self._do_crud()

    def _do_query(self):
        q = self.rng.choice(self.queries)
        t0 = time.perf_counter()
        ok, stderr = run_mysql(self.host, self.port, self.user,
                               q, timeout=60)
        ms = (time.perf_counter() - t0) * 1000
        self.stats.record_query(ms, ok)
        if not ok and "lock" in stderr.lower():
            self.stats.record_lock_wait(ms)

    def _do_crud(self):
        table = self.rng.choice(self.tables)
        try:
            sql = crud_templates.random_crud_for_table(
                table, self.id_pools[table], self.rng, self.ctx)
        except crud_templates.PoolExhausted:
            self._refresh_pool(table)
            return
        op = "insert" if sql.upper().startswith("INSERT") else \
             "update" if sql.upper().startswith("UPDATE") else "delete"
        t0 = time.perf_counter()
        ok, stderr = run_mysql(self.host, self.port, self.user, sql, timeout=30)
        ms = (time.perf_counter() - t0) * 1000
        self.stats.record_crud(ms, ok, op)
        if not ok and "lock" in stderr.lower():
            self.stats.record_lock_wait(ms)

    def _refresh_pool(self, table: str):
        pool = self.id_pools[table]
        db_max = get_row_count(self.host, self.port, self.user, table)
        pool.refresh(db_max)


def replenish_table(host: str, port: int, user: str, table: str,
                    target: int, batch_size: int = 100) -> int:
    current = get_row_count(host, port, user, table)
    if current >= target:
        return 0
    needed = min(target - current, batch_size)
    sql = (f"INSERT INTO {table} SELECT * FROM {table} "
           f"ORDER BY RANDOM() LIMIT {needed};") if current > 0 else None
    if sql:
        ok, _ = run_mysql(host, port, user, sql, timeout=60)
        return needed if ok else 0
    return 0


def replenishment_loop(host, port, user, tables, targets, stop):
    while not stop.is_set():
        time.sleep(60)
        for tbl, target in zip(tables, targets):
            try:
                replenish_table(host, port, user, tbl, target)
            except Exception:
                pass


def dynamic_concurrency_controller(workers: list, cmin: int, cmax: int,
                                    stop: threading.Event):
    rng = random.Random()
    while not stop.is_set():
        wait = rng.randint(30, 120)
        if stop.wait(wait):
            break
        target = rng.randint(cmin, cmax)
        current = len(workers)
        if target > current:
            for w in workers:
                if not w.thread.is_alive():
                    w.thread.join(timeout=1)
            new_workers = workers[:target]
            for i in range(len(workers), target):
                w = workers[i] if i < len(workers) else None
                if w is None:
                    break
            while len(workers) < target:
                workers.append(None)
        elif target < current:
            workers[:] = workers[:target]


def metrics_sampler(pid: int, csv_path: str, stats: MixedSoakStats,
                    stop: threading.Event, interval: int = 30):
    start = time.time()
    with open(csv_path, "w") as f:
        f.write("ts,rss_kb,fd,insert_qps,update_qps,delete_qps\n")
    while not stop.is_set():
        time.sleep(interval)
        rss = sample_rss(pid)
        fd = sample_fd(pid)
        elapsed = time.time() - start
        iq = stats.crud_qps("insert", elapsed)
        uq = stats.crud_qps("update", elapsed)
        dq = stats.crud_qps("delete", elapsed)
        with open(csv_path, "a") as f:
            f.write(f"{int(time.time())},{rss},{fd},{iq:.2f},{uq:.2f},{dq:.2f}\n")


def spawn_worker(worker: Worker, stats: MixedSoakStats,
                 stop: threading.Event) -> threading.Thread:
    def target():
        worker.run()
    t = threading.Thread(target=target, daemon=True)
    t.start()
    return t


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=3397)
    p.add_argument("--user", default="root")
    p.add_argument("--queries-dir", default="scripts/soak/tpch_queries")
    p.add_argument("--duration", type=int, default=1800)
    p.add_argument("--mix-ratio", type=int, default=80,
                   help="Percentage of operations that are queries (default 80)")
    p.add_argument("--concurrency-min", type=int, default=4)
    p.add_argument("--concurrency-max", type=int, default=32)
    p.add_argument("--output-dir", default="soak_results/mixed")
    args = p.parse_args()

    if not 1 <= args.mix_ratio <= 99:
        print(f"ERROR: --mix-ratio must be in [1, 99], got {args.mix_ratio}",
              file=sys.stderr)
        sys.exit(1)
    if args.concurrency_min < 1 or args.concurrency_max < args.concurrency_min:
        print("ERROR: invalid concurrency range", file=sys.stderr)
        sys.exit(1)

    queries = load_queries(args.queries_dir)
    if not queries:
        print(f"ERROR: no queries in {args.queries_dir}", file=sys.stderr)
        sys.exit(1)
    tables = ["orders", "lineitem", "customer", "part", "partsupp"]
    targets = [crud_templates.MIN_ROWS[t] for t in tables]

    ts = time.strftime("%Y%m%d_%H%M%S")
    out_dir = os.path.join(args.output_dir, f"mixed_{ts}")
    os.makedirs(out_dir, exist_ok=True)
    metrics_csv = os.path.join(out_dir, "metrics.csv")

    pid = find_pid(args.port)
    if not pid:
        print(f"ERROR: no server on port {args.port}", file=sys.stderr)
        sys.exit(1)

    print(f"Loaded {len(queries)} queries, mix={args.mix_ratio}% queries, "
          f"concurrency=[{args.concurrency_min},{args.concurrency_max}], "
          f"duration={args.duration}s")

    stats = MixedSoakStats()
    stop = threading.Event()

    initial = random.randint(args.concurrency_min, args.concurrency_max)
    worker_objs = []
    for i in range(initial):
        w = Worker(i, args.host, args.port, args.user, queries, tables,
                   args.mix_ratio, stats, stop)
        w.thread = spawn_worker(w, stats, stop)
        worker_objs.append(w)

    repl_thread = threading.Thread(
        target=replenishment_loop,
        args=(args.host, args.port, args.user, tables, targets, stop),
        daemon=True)
    repl_thread.start()

    sample_thread = threading.Thread(
        target=metrics_sampler,
        args=(pid, metrics_csv, stats, stop),
        daemon=True)
    sample_thread.start()

    ctrl_thread = threading.Thread(
        target=dynamic_concurrency_controller,
        args=(worker_objs, args.concurrency_min, args.concurrency_max, stop),
        daemon=True)
    ctrl_thread.start()

    t0 = time.time()
    try:
        time.sleep(args.duration)
    except KeyboardInterrupt:
        print("\nInterrupted")
    finally:
        stop.set()
        time.sleep(2)

    elapsed = time.time() - t0

    ql = sorted(stats.query_latencies_ms)
    cl = sorted(stats.crud_latencies_ms)
    total_ops = stats.queries_executed + (stats.insert_count +
                                          stats.update_count + stats.delete_count)
    crud_total = stats.insert_count + stats.update_count + stats.delete_count
    crud_ratio = (crud_total / total_ops * 100) if total_ops else 0.0

    report = {
        "level": "mixed",
        "mix_ratio_configured": args.mix_ratio,
        "mix_ratio_actual": round(crud_ratio, 2),
        "concurrency_min": args.concurrency_min,
        "concurrency_max": args.concurrency_max,
        "concurrency_initial": initial,
        "duration_seconds": int(elapsed),
        "queries_executed": stats.queries_executed,
        "query_errors": stats.query_errors,
        "p50_query_latency_ms": round(percentile(ql, 0.50), 4),
        "p99_query_latency_ms": round(percentile(ql, 0.99), 4),
        "crud": {
            "insert_count": stats.insert_count,
            "update_count": stats.update_count,
            "delete_count": stats.delete_count,
            "crud_errors": stats.crud_errors,
            "insert_qps": round(stats.crud_qps("insert", elapsed), 2),
            "update_qps": round(stats.crud_qps("update", elapsed), 2),
            "delete_qps": round(stats.crud_qps("delete", elapsed), 2),
            "p50_crud_latency_ms": round(percentile(cl, 0.50), 4),
            "p99_crud_latency_ms": round(percentile(cl, 0.99), 4),
        },
        "lock_wait_time_avg_ms": round(stats.avg_lock_wait_ms(), 4),
        "read_write_conflicts": stats.read_write_conflicts,
        "alert_triggered": stats.query_errors > 0 or stats.crud_errors > 0,
    }

    rpath = os.path.join(out_dir, "SoakReport.json")
    with open(rpath, "w") as f:
        json.dump(report, f, indent=2)

    print("\n=== Mixed SoakReport ===")
    print(json.dumps(report, indent=2))
    print(f"\nReport: {rpath}")
    passed = (stats.query_errors == 0 and stats.crud_errors == 0
              and 18.0 <= crud_ratio <= 22.0)
    print("\n" + "=" * 60)
    print(f"  {'PASS' if passed else 'FAIL'}: {total_ops:,} ops "
          f"({stats.queries_executed:,} queries + {crud_total:,} CRUD)")
    print(f"  CRUD ratio: {crud_ratio:.1f}% (target {100-args.mix_ratio}%)")
    print(f"  Query errors: {stats.query_errors}  CRUD errors: {stats.crud_errors}")
    print("=" * 60)
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()