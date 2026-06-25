#!/usr/bin/env python3
"""
light_soak.py - Minimal pure-SELECT wall-clock soak.
Uses subprocess mysql CLI, persistent connection per thread.
"""
import subprocess, argparse, time, os, sys, json
from datetime import datetime, timezone
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=3396)
    p.add_argument("--user", default="root")
    p.add_argument("--database", default="sbtest")
    p.add_argument("--hours", type=float, default=1)
    p.add_argument("--threads", type=int, default=4)
    p.add_argument("--interval", type=int, default=30)
    p.add_argument("--results-dir", default=None)
    p.add_argument("--hard-rss-mb", type=int, default=8192)
    p.add_argument("--soft-rss-mb", type=int, default=4096)
    args = p.parse_args()

    results_dir = args.results_dir or "test_results/soak_" + str(int(time.time()))
    os.makedirs(results_dir, exist_ok=True)
    metrics_file = os.path.join(results_dir, "metrics.csv")
    report_file = os.path.join(results_dir, "STABILITY_REPORT.md")
    log_file = os.path.join(results_dir, "soak.log")
    pid_file = os.path.join(results_dir, "soak.pid")

    start_ts = time.time()
    end_ts = start_ts + args.hours * 3600

    queries = [0]
    errors = [0]
    lock = __import__("threading").Lock()

    with open(pid_file, "w") as f:
        f.write(str(os.getpid()))

    def log(msg):
        ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        line = "[%s] %s" % (ts, msg)
        print(line)
        try:
            with open(log_file, "a") as f:
                f.write(line + "\n")
        except Exception:
            pass

    def mysql(sql):
        try:
            r = subprocess.run(
                ["mysql", "-h", args.host, "-P", str(args.port),
                 "-u", args.user, "--batch", "-N", "-e", sql, args.database],
                capture_output=True, text=True, timeout=10
            )
            return r.returncode == 0
        except Exception:
            return False

    def find_pid():
        try:
            for d in os.listdir("/proc"):
                if not d.isdigit():
                    continue
                try:
                    with open("/proc/" + d + "/cmdline", "rb") as f:
                        cmd = f.read().decode("utf-8", errors="ignore")
                        if "sqlrustgo-mysql-server" in cmd and "serve" in cmd:
                            return int(d)
                except Exception:
                    pass
        except Exception:
            pass
        return None

    def get_metrics(pid):
        try:
            with open("/proc/" + str(pid) + "/status") as f:
                content = f.read()
            rss_kb = 0
            for line in content.splitlines():
                if line.startswith("VmRSS:"):
                    rss_kb = int(line.split()[1])
                    break
            fd_count = len(os.listdir("/proc/" + str(pid) + "/fd"))
            return rss_kb, fd_count
        except Exception:
            return 0, 0

    def worker(tid):
        ops = 0
        while time.time() < end_ts:
            ok = True
            # Pure SELECT - no storage writes, no hangs
            for sql in [
                "SELECT COUNT(*) FROM sbtest",
                "SELECT id, k FROM sbtest WHERE k=%d LIMIT 3" % (tid % 100),
                "SELECT id FROM sbtest WHERE id BETWEEN %d AND %d LIMIT 5" % (tid * 7 % 1000, tid * 7 % 1000 + 10),
                "SELECT id, c, pad FROM sbtest LIMIT 3",
            ]:
                if not mysql(sql):
                    ok = False

            with lock:
                queries[0] += 4
                if not ok:
                    errors[0] += 1

            ops += 1
            if ops % 20 == 0:
                time.sleep(0.05)

    log("=" * 60)
    log("Light SELECT-only Soak")
    log("=" * 60)
    log("Host: %s:%d  DB: %s" % (args.host, args.port, args.database))
    log("Duration: %.1fh  Threads: %d" % (args.hours, args.threads))
    log("Results: %s" % results_dir)
    log("=" * 60)

    server_pid = find_pid()
    if server_pid:
        log("Server PID: %d" % server_pid)
    else:
        log("WARNING: Could not find server PID")

    with ThreadPoolExecutor(max_workers=args.threads) as executor:
        futures = [executor.submit(worker, i) for i in range(args.threads)]
        initial_rss_kb = 0
        sample = 0
        status = "COMPLETED"

        with open(metrics_file, "w") as mf:
            mf.write("ts,elapsed_h,rss_mb,rss_delta_mb,fd_count,tps,qps,errors\n")

            while time.time() < end_ts:
                elapsed = time.time() - start_ts
                ts_str = datetime.fromtimestamp(time.time(), tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")

                with lock:
                    q = queries[0]
                    e = errors[0]

                rss_mb = 0
                fd_count = 0
                if server_pid:
                    rss_kb, fd_count = get_metrics(server_pid)
                    rss_mb = rss_kb / 1024.0
                    if sample == 0:
                        initial_rss_kb = rss_kb

                rss_delta_mb = (rss_kb - initial_rss_kb) / 1024.0
                tps_val = q / elapsed / 4 if elapsed > 0 else 0  # 4 queries per tx
                qps_val = q / elapsed if elapsed > 0 else 0

                mf.write("%s,%.3f,%.1f,%.1f,%d,%.2f,%.2f,%d\n" % (
                    ts_str, elapsed / 3600.0, rss_mb, rss_delta_mb, fd_count,
                    tps_val, qps_val, e
                ))
                mf.flush()

                if rss_mb > args.hard_rss_mb:
                    log("KILL: RSS %.0fMB > hard %dMB" % (rss_mb, args.hard_rss_mb))
                    for f in futures:
                        f.cancel()
                    executor.shutdown(wait=False)
                    status = "KILLED"
                    break

                if sample % 4 == 0 and sample > 0:
                    remain = end_ts - time.time()
                    log("[s%d %.2fh] RSS=%.0fMB FD=%d QPS=%.1f err=%d" % (
                        sample, elapsed/3600.0, rss_mb, fd_count, qps_val, e
                    ))

                sample += 1
                time.sleep(args.interval)

        for f in futures:
            try:
                f.result(timeout=5)
            except Exception as exc:
                log("Worker error: %s" % exc)

    log("Soak complete. Status: %s" % status)

    with lock:
        q = queries[0]
        e = errors[0]

    elapsed = time.time() - start_ts
    elapsed_h = elapsed / 3600.0
    rss_mb_final = 0
    if server_pid:
        rss_kb, _ = get_metrics(server_pid)
        rss_mb_final = rss_kb / 1024.0

    qps_final = q / elapsed if elapsed > 0 else 0
    verdict = "PASS" if e == 0 and status == "COMPLETED" else "REVIEW"

    with open(report_file, "w") as f:
        f.write("""# Light Soak Report

**Run**: %s
**Duration**: %.3fh (target %.1fh)
**Status**: %s
**Host**: %s:%d
**Threads**: %d

## Results

| Metric | Value |
|--------|-------|
| Total Queries | %d |
| QPS | %.2f/s |
| Errors | %d |
| RSS Final | %.0f MB |
| Hard RSS Limit | %d MB |

## Verdict

**%s** — %.3fh, QPS=%.1f, errors=%d
""" % (
        datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        elapsed_h, args.hours, status, args.host, args.port, args.threads,
        q, qps_final, e, rss_mb_final, args.hard_rss_mb,
        verdict, elapsed_h, qps_final, e,
    ))

    log("Report: %s" % report_file)
    print("DONE")


if __name__ == "__main__":
    main()
