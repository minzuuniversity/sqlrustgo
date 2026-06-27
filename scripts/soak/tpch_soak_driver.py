#!/usr/bin/env python3
"""TPC-H Soak Driver using mysql CLI subprocess."""

import argparse
import json
import os
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class SoakStats:
    queries: int = 0
    errors: int = 0
    latencies_ms: list = field(default_factory=list)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def record(self, latency_ms: float, ok: bool) -> None:
        with self.lock:
            self.queries += 1
            if not ok:
                self.errors += 1
            self.latencies_ms.append(latency_ms)


def run_query(host: str, port: int, user: str, sql: str) -> tuple[bool, float]:
    """Run SQL via mysql CLI. Returns (ok, latency_ms)."""
    start = time.perf_counter()
    try:
        r = subprocess.run(
            ["mysql", "-h", host, "-P", str(port), "-u", user,
             "--silent", "-N", "-e", sql],
            capture_output=True, text=True, timeout=30,
        )
        ok = r.returncode == 0
        elapsed = (time.perf_counter() - start) * 1000
        return ok, elapsed
    except Exception:
        elapsed = (time.perf_counter() - start) * 1000
        return False, elapsed


def worker_thread(tid: int, host: str, port: int, user: str,
                  queries: list, stop_time: float, stats: SoakStats) -> None:
    q_idx = tid % len(queries)
    while time.time() < stop_time:
        sql = queries[q_idx % len(queries)]
        q_idx += 1
        ok, lat = run_query(host, port, user, sql)
        stats.record(lat, ok)
        if q_idx % 50 == 0:
            time.sleep(0.001)


def run_soak(host: str, port: int, user: str, queries: list,
             concurrency: int, duration: int) -> dict:
    stats = SoakStats()
    stop = time.time() + duration
    threads = []
    for tid in range(concurrency):
        t = threading.Thread(target=worker_thread,
                           args=(tid, host, port, user, queries, stop, stats),
                           daemon=True)
        t.start()
        threads.append(t)
    for t in threads:
        t.join(timeout=duration + 30)
    lats = sorted(stats.latencies_ms) if stats.latencies_ms else [0.0]
    n = len(lats)
    p50 = lats[int(n * 0.50)] if n > 0 else 0.0
    p99 = lats[int(n * 0.99)] if n > 0 else 0.0
    return {"queries_executed": stats.queries, "errors": stats.errors,
            "p50_latency_ms": round(p50, 4), "p99_latency_ms": round(p99, 4)}


def find_pid(port: int):
    try:
        out = subprocess.check_output(["lsof", "-ti", f":{port}"],
                                     text=True, stderr=subprocess.DEVNULL)
        pids = [int(p) for p in out.strip().split() if p.isdigit()]
        return pids[0] if pids else None
    except Exception:
        pass
    try:
        for e in os.listdir("/proc"):
            if not e.isdigit():
                continue
            try:
                with open(f"/proc/{e}/cmdline", "rb") as f:
                    cmd = f.read().decode("utf-8", errors="ignore")
                    if "sqlrustgo" in cmd and f"--port {port}" in cmd:
                        return int(e)
            except Exception:
                pass
    except Exception:
        pass
    return None


def sample_metrics(pid):
    rss, fd = 0, 0
    if not pid:
        return rss, fd
    try:
        with open(f"/proc/{pid}/status") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    rss = int(line.split()[1])
                    break
    except Exception:
        pass
    try:
        fd = len(os.listdir(f"/proc/{pid}/fd"))
    except Exception:
        pass
    return rss, fd


def sampler(pid, csv_path, interval, stop):
    while not stop.is_set():
        rss, fd = sample_metrics(pid)
        with open(csv_path, "a") as f:
            f.write(f"{int(time.time())},{rss},{fd}\n")
        for _ in range(interval * 10):
            if stop.is_set():
                break
            time.sleep(0.1)


def load_queries(qdir):
    queries = []
    for i in range(1, 23):
        qf = Path(qdir) / f"q{i:02d}.sql"
        if qf.exists():
            queries.append(qf.read_text().strip())
    return queries


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=3396)
    p.add_argument("--user", default="ai")
    p.add_argument("--queries-dir", default="scripts/soak/tpch_queries")
    p.add_argument("--concurrency", type=int, default=16)
    p.add_argument("--duration", type=int, default=1800)
    p.add_argument("--output-dir", default="soak_results")
    p.add_argument("--level", default="30m")
    args = p.parse_args()

    queries = load_queries(args.queries_dir)
    if not queries:
        print(f"ERROR: No query files in {args.queries_dir}", file=sys.stderr)
        sys.exit(1)
    print(f"Loaded {len(queries)} TPC-H queries")

    ts = time.strftime("%Y%m%d_%H%M%S")
    out = f"{args.output_dir}/tpch_{args.level}_{ts}"
    os.makedirs(out, exist_ok=True)

    pid = find_pid(args.port)
    rss0, fd0 = sample_metrics(pid)
    mcsv = os.path.join(out, "metrics.csv")
    with open(mcsv, "w") as f:
        f.write("ts,rss_kb,fd\n")
        if pid:
            f.write(f"{int(time.time())},{rss0},{fd0}\n")

    stop_ev = threading.Event()
    smpl = threading.Thread(target=sampler,
                           args=(pid, mcsv, 30, stop_ev), daemon=True)
    smpl.start()

    print(f"Starting: {args.concurrency} threads, {args.duration}s")
    t0 = time.time()
    rep = run_soak(args.host, args.port, args.user,
                    queries, args.concurrency, args.duration)
    elapsed = int(time.time() - t0)

    stop_ev.set()
    smpl.join(timeout=5)

    rss1, fd1 = sample_metrics(pid)
    mg = ((rss1 - rss0) / rss0 * 100) if rss0 else 0.0
    fg = (fd1 - fd0) if (fd0 and fd1) else 0

    rep.update({
        "level": args.level,
        "duration_seconds": elapsed,
        "concurrency": args.concurrency,
        "memory_baseline_bytes": rss0 * 1024,
        "memory_final_bytes": rss1 * 1024,
        "memory_growth_pct": round(mg, 4),
        "fd_baseline": fd0 or 0,
        "fd_final": fd1 or 0,
        "fd_growth": fg,
        "alert_triggered": bool(mg >= 10.0 or fg >= 5),
        "queries_per_second": round(rep["queries_executed"] / elapsed, 2) if elapsed else 0,
    })

    rpath = os.path.join(out, "SoakReport.json")
    with open(rpath, "w") as f:
        json.dump(rep, f, indent=2)

    print(f"\n=== SoakReport ===")
    print(json.dumps(rep, indent=2))
    print(f"\nReport: {rpath}")

    v = "PASS" if rep["errors"] == 0 and not rep["alert_triggered"] else "FAIL"
    print(f"\n{'='*50}")
    print(f"  {v}: {rep['queries_executed']:,} q ({rep['queries_per_second']} q/s)")
    print(f"  Errors: {rep['errors']}")
    print(f"  Memory: {rep['memory_growth_pct']:.2f}% (alert: 10%)")
    print(f"  FD: {rep['fd_growth']} (alert: +5)")
    print(f"  P50: {rep['p50_latency_ms']:.2f}ms  P99: {rep['p99_latency_ms']:.2f}ms")
    print(f"{'='*50}")
    sys.exit(0 if v == "PASS" else 1)


if __name__ == "__main__":
    main()
