#!/usr/bin/env python3
"""TPC-H Soak Driver using persistent MySQL connections."""

import argparse, json, os, subprocess, sys, threading, time
from dataclasses import dataclass, field
from pathlib import Path

def load_queries(qdir):
    qs = []
    for i in range(1, 23):
        qf = Path(qdir) / ("q%02d.sql" % i)
        if qf.exists():
            qs.append(qf.read_text().strip().rstrip(";"))
    return qs

def make_batch(sqls, db="tpch"):
    stmts = ["USE %s;" % db] + list(sqls)
    return (";\n".join(stmts) + ";").encode()

@dataclass
class SoakStats:
    queries: int = 0
    errors: int = 0
    latencies_ms: list = field(default_factory=list)
    lock: threading.Lock = field(default_factory=threading.Lock)
    def record(self, ms, ok, n=1):
        with self.lock:
            self.queries += n
            if not ok: self.errors += 1
            self.latencies_ms.append(ms)

def worker(tid, host, port, user, queries, batch_size, stop, stats):
    batch = make_batch(queries[:batch_size])
    while time.time() < stop:
        t0 = time.perf_counter()
        try:
            r = subprocess.run([
                "mysql", "-h", host, "-P", str(port), "-u", user,
                "--silent", "-N"
            ], input=batch, capture_output=True, text=True, timeout=30)
            ms = (time.perf_counter()-t0)*1000
            ok = r.returncode == 0 and "ERROR" not in r.stderr
            stats.record(ms, ok, batch_size)
        except Exception:
            ms = (time.perf_counter()-t0)*1000
            stats.record(ms, False, batch_size)

def run_soak(host, port, user, queries, concurrency, duration, batch_size):
    stats = SoakStats()
    stop = time.time() + duration
    threads = []
    for tid in range(concurrency):
        t = threading.Thread(target=worker,
            args=(tid, host, port, user, queries, batch_size, stop, stats),
            daemon=True)
        t.start(); threads.append(t)
    for t in threads: t.join(timeout=duration+30)
    lats = sorted(stats.latencies_ms)
    n = len(lats)
    p50 = lats[int(n*0.50)] if n>0 else 0.0
    p99 = lats[int(n*0.99)] if n>0 else 0.0
    return dict(queries_executed=stats.queries, errors=stats.errors,
                p50_latency_ms=round(p50,4), p99_latency_ms=round(p99,4))

def find_pid(port):
    try:
        out = subprocess.check_output(["lsof","-ti",":%d"%port],
                                 text=True, stderr=subprocess.DEVNULL)
        pids = [int(p) for p in out.strip().split() if p.isdigit()]
        return pids[0] if pids else None
    except: return None

def sample_metrics(pid, csv_path, interval, stop):
    while not stop.is_set():
        rss = fd = 0
        if pid:
            try:
                with open("/proc/%d/status"%pid) as f:
                    for line in f:
                        if line.startswith("VmRSS:"):
                            rss = int(line.split()[1]); break
            except: pass
            try: fd = len(os.listdir("/proc/%d/fd"%pid))
            except: pass
        with open(csv_path, "a") as f:
            f.write("%d,%d,%d\n" % (int(time.time()), rss, fd))
        for _ in range(interval*10):
            if stop.is_set(): break
            time.sleep(0.1)

def main():
    a = argparse.ArgumentParser()
    a.add_argument("--host", default="127.0.0.1")
    a.add_argument("--port", type=int, default=3396)
    a.add_argument("--user", default="ai")
    a.add_argument("--queries-dir", default="scripts/soak/tpch_queries")
    a.add_argument("--concurrency", type=int, default=16)
    a.add_argument("--duration", type=int, default=1800)
    a.add_argument("--batch-size", type=int, default=22)
    a.add_argument("--output-dir", default="soak_results")
    a.add_argument("--level", default="30m")
    args = a.parse_args()

    queries = load_queries(args.queries_dir)
    if not queries:
        print("ERROR: No queries in " + args.queries_dir, file=sys.stderr)
        sys.exit(1)
    print("Loaded %d queries, %d threads, batch=%d" % (
          len(queries), args.concurrency, args.batch_size))

    ts = time.strftime("%Y%m%d_%H%M%S")
    out = os.path.join(args.output_dir, "tpch_%s_%s" % (args.level, ts))
    os.makedirs(out, exist_ok=True)

    pid = find_pid(args.port)
    mcsv = os.path.join(out, "metrics.csv")
    baseline_rss = 0
    with open(mcsv, "w") as f:
        f.write("ts,rss_kb,fd\n")
        if pid:
            try:
                with open("/proc/%d/status"%pid) as pf:
                    for line in pf:
                        if line.startswith("VmRSS:"):
                            baseline_rss = int(line.split()[1])
                            f.write("%d,%d,0\n" % (int(time.time()), baseline_rss))
                            break
            except: pass

    stop_ev = threading.Event()
    smpl = threading.Thread(target=sample_metrics,
        args=(pid, mcsv, 30, stop_ev), daemon=True)
    smpl.start()

    t0 = time.time()
    rep = run_soak(args.host, args.port, args.user, queries,
                   args.concurrency, args.duration, args.batch_size)
    elapsed = int(time.time() - t0)
    stop_ev.set(); smpl.join(timeout=5)

    rss1 = 0
    if pid:
        try:
            with open("/proc/%d/status"%pid) as f:
                for line in f:
                    if line.startswith("VmRSS:"):
                        rss1 = int(line.split()[1]); break
        except: pass

    mg = ((rss1-baseline_rss)/float(baseline_rss or 1))*100
    rep.update({
        "level": args.level,
        "duration_seconds": elapsed,
        "concurrency": args.concurrency,
        "batch_size": args.batch_size,
        "memory_baseline_bytes": baseline_rss*1024,
        "memory_final_bytes": rss1*1024,
        "memory_growth_pct": round(mg, 4),
        "alert_triggered": bool(mg >= 10.0),
        "queries_per_second": round(rep["queries_executed"]/float(elapsed),2) if elapsed else 0,
    })

    rpath = os.path.join(out, "SoakReport.json")
    with open(rpath, "w") as f: json.dump(rep, f, indent=2)

    print("\n=== SoakReport ===")
    print(json.dumps(rep, indent=2))
    print("\nReport: " + rpath)

    v = "PASS" if rep["errors"]==0 and not rep["alert_triggered"] else "FAIL"
    print("\n" + "="*50)
    print("  %s: %d q (%.1f q/s)" % (v, rep["queries_executed"], rep["queries_per_second"]))
    print("  Errors: %d" % rep["errors"])
    print("  Memory: %.2f%%" % rep["memory_growth_pct"])
    print("  P50: %.2fms  P99: %.2fms" % (rep["p50_latency_ms"], rep["p99_latency_ms"]))
    print("="*50)
    sys.exit(0 if v=="PASS" else 1)

if __name__ == "__main__": main()
