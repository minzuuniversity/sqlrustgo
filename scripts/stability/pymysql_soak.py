#!/usr/bin/env python3
"""
pymysql_soak.py - Real wall-clock soak using pymysql.
Avoids TLS flush issue by using pymysql (known to work).
"""
import pymysql, argparse, time, os, sys, json
from datetime import datetime, timezone
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=3396)
    p.add_argument("--user", default="root")
    p.add_argument("--database", default="default")
    p.add_argument("--hours", type=float, default=1)
    p.add_argument("--threads", type=int, default=4)
    p.add_argument("--interval", type=int, default=30)
    p.add_argument("--table-size", type=int, default=10000)
    p.add_argument("--results-dir", default=None)
    p.add_argument("--hard-rss-mb", type=int, default=8192)
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
    latencies = []
    qt = defaultdict(int)
    et = defaultdict(int)
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

    def worker(tid, table_size):
        ops = 0
        # Each thread creates ONE persistent pymysql connection.
        # This avoids creating a new connection per query (which would
        # exhaust FD limits on high-concurrency soak runs).
        conn = None
        try:
            conn = pymysql.connect(
                host=args.host,
                port=args.port,
                user=args.user,
                database=args.database,
                charset=None,
                connect_timeout=10,
                autocommit=True,
            )
        except Exception as e:
            log("worker %d: initial connect failed: %s" % (tid, e))
            return

        while time.time() < end_ts:
            try:
                # Connection health check
                conn.ping(reconnect=True)

                op_start = time.time()
                ok = True
                # 4 queries per "transaction"
                for sql in [
                    "SELECT COUNT(*) FROM sbtest",
                    "SELECT id, k FROM sbtest WHERE k=%d LIMIT 3" % (tid % 100),
                    "SELECT id FROM sbtest WHERE id BETWEEN %d AND %d LIMIT 5"
                    % ((tid * 7) % 1000, (tid * 7) % 1000 + 10),
                    "SELECT id, c, pad FROM sbtest LIMIT 3",
                ]:
                    cur = conn.cursor()
                    try:
                        cur.execute(sql)
                        cur.fetchall()
                        with lock:
                            queries[0] += 1
                            qt[sql.split()[0]] += 1
                    except Exception:
                        ok = False
                        with lock:
                            errors[0] += 1
                            et[sql.split()[0]] += 1
                    finally:
                        cur.close()
                latency_ms = (time.time() - op_start) * 1000
                with lock:
                    latencies.append(latency_ms)
                    if len(latencies) > 10000:
                        del latencies[:5000]
                ops += 1
                if ops % 20 == 0:
                    time.sleep(0.05)
            except Exception as e:
                # Connection died — try to reconnect once
                with lock:
                    errors[0] += 1
                try:
                    if conn is not None:
                        conn.close()
                except Exception:
                    pass
                try:
                    conn = pymysql.connect(
                        host=args.host,
                        port=args.port,
                        user=args.user,
                        database=args.database,
                        charset=None,
                        connect_timeout=10,
                        autocommit=True,
                    )
                except Exception:
                    time.sleep(1)  # back off if reconnect fails

        # Cleanup
        if conn is not None:
            try:
                conn.close()
            except Exception:
                pass

    log("=" * 60)
    log("pymysql Soak Test")
    log("=" * 60)
    log("Host: %s:%d  DB: %s" % (args.host, args.port, args.database))
    log("Duration: %.1fh  Threads: %d" % (args.hours, args.threads))
    log("Results: %s" % results_dir)

    server_pid = find_pid()
    if server_pid:
        log("Server PID: %d" % server_pid)
    with ThreadPoolExecutor(max_workers=args.threads) as executor:
        futures = [executor.submit(worker, i, args.table_size) for i in range(args.threads)]
        initial_rss_kb = 0
        sample = 0
        status = "COMPLETED"

        with open(metrics_file, "w") as mf:
            mf.write("ts,elapsed_h,rss_mb,rss_delta_mb,fd_count,tps,qps,errors,p50_ms,p95_ms,p99_ms\n")
            while time.time() < end_ts:
                elapsed = time.time() - start_ts
                ts_str = datetime.fromtimestamp(time.time(), tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
                with lock:
                    q = queries[0]
                    e = errors[0]
                    lat = list(latencies)
                rss_mb = 0
                fd_count = 0
                if server_pid:
                    rss_kb, fd_count = get_metrics(server_pid)
                    rss_mb = rss_kb / 1024.0
                    if sample == 0:
                        initial_rss_kb = rss_kb
                rss_delta_mb = (rss_kb - initial_rss_kb) / 1024.0
                qps_val = q / elapsed if elapsed > 0 else 0
                tps_val = q / elapsed / 4 if elapsed > 0 else 0
                p50 = p95 = p99 = 0
                if lat:
                    lat_sorted = sorted(lat)
                    n = len(lat_sorted)
                    p50 = lat_sorted[int(n * 0.50)]
                    p95 = lat_sorted[int(n * 0.95)]
                    p99 = lat_sorted[int(n * 0.99)]
                mf.write("%s,%.3f,%.1f,%.1f,%d,%.2f,%.2f,%d,%.2f,%.2f,%.2f\n" % (
                    ts_str, elapsed / 3600.0, rss_mb, rss_delta_mb, fd_count,
                    tps_val, qps_val, e, p50, p95, p99
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
                    log("[s%d %.2fh] RSS=%.0fMB FD=%d QPS=%.1f err=%d" % (
                        sample, elapsed / 3600.0, rss_mb, fd_count, qps_val, e
                    ))
                sample += 1
                time.sleep(args.interval)
        for f in futures:
            try:
                f.result(timeout=5)
            except Exception as exc:
                log("Worker error: %s" % exc)
    log("Soak complete. Status: %s" % status)


if __name__ == "__main__":
    main()
