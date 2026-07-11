#!/usr/bin/env python3
"""
soak_client.py - Multi-threaded soak test for sqlrustgo-mysql-server.
Uses raw MySQL text protocol over TCP sockets (no external deps).
Auth mode: none (skip password verification).
"""
import socket, argparse, time, threading, os, sys, json
from datetime import datetime, timezone
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=3307)
    p.add_argument("--database", default="sbtest")
    p.add_argument("--hours", type=float, default=1)
    p.add_argument("--threads", type=int, default=24)
    p.add_argument("--interval", type=int, default=30)
    p.add_argument("--table-size", type=int, default=40000)
    p.add_argument("--results-dir", default=None)
    p.add_argument("--hard-rss-mb", type=int, default=8192)
    p.add_argument("--soft-rss-mb", type=int, default=4096)
    args = p.parse_args()

    results_dir = args.results_dir or "test_results/soak_" + str(int(time.time()))
    os.makedirs(results_dir, exist_ok=True)
    metrics_file = os.path.join(results_dir, "metrics.csv")
    report_file = os.path.join(results_dir, "STABILITY_REPORT.md")
    log_file = os.path.join(results_dir, "soak.log")

    start_ts = time.time()
    end_ts = start_ts + args.hours * 3600

    lock = threading.Lock()
    queries = 0
    transactions = 0
    errors = 0
    latencies = []
    qt = defaultdict(int)
    et = defaultdict(int)

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
                except (FileNotFoundError, PermissionError):
                    pass
        except FileNotFoundError:
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
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            return 0, 0

    class MySQLConn:
        def __init__(self, host, port, database):
            self.host = host
            self.port = port
            self.database = database
            self.sock = None
            self.seq = 0
            self._connect()

        def _recv_full(self, n):
            data = b""
            while len(data) < n:
                chunk = self.sock.recv(n - len(data))
                if not chunk:
                    raise Exception("Connection closed")
                data += chunk
            return data

        def _send_packet(self, payload):
            length = len(payload)
            header = bytes([length & 0xFF, (length >> 8) & 0xFF, (length >> 16) & 0xFF, self.seq & 0xFF])
            self.sock.sendall(header + payload)
            self.seq = (self.seq + 1) & 0xFF

        def _recv_packet(self):
            header = self._recv_full(4)
            length = header[0] | (header[1] << 8) | (header[2] << 16)
            self.seq = header[3]
            return self._recv_full(length)

        def _connect(self):
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(15)
            self.sock.connect((self.host, self.port))
            self._recv_packet()  # server handshake
            # auth_mode=none: send handshake response with minimal fields
            # capability(4) + max_packet_size(4) + charset(1) + reserved(23) + username(null)
            cap = 0x00000a35 | 0x00000080 | 0x00000008
            pkt = cap.to_bytes(4, 'little') + (16777216).to_bytes(4, 'little')
            pkt += bytes([0x21]) + b"\x00" * 23 + b"root\x00\x00"
            self._send_packet(pkt)
            resp = self._recv_packet()
            if resp[0] == 0xFF:
                raise Exception("Auth failed")
            self.seq = 2
            self._send_packet(bytes([0x02]) + self.database.encode('utf-8'))
            self._recv_packet()

        def cmd_query(self, sql):
            self._send_packet(bytes([0x03]) + sql.encode('utf-8'))
            resp = self._recv_packet()
            if resp and resp[0] == 0xFF:
                raise Exception("MySQL error")
            return resp

        def reconnect(self):
            try:
                self.sock.close()
            except:
                pass
            self._connect()

        def close(self):
            if self.sock:
                try:
                    self.sock.close()
                except:
                    pass

    def worker(tid, table_size):
        nonlocal queries, transactions, errors, latencies, qt, et
        ops = 0
        recon_count = 0
        max_reconnects = 10

        try:
            conn = MySQLConn(args.host, args.port, args.database)
        except Exception as e:
            log("Worker %d connect error: %s" % (tid, e))
            with lock:
                errors += 1
                et["connect"] += 1
            return

        while time.time() < end_ts:
            op_start = time.time()

            # SELECT COUNT
            try:
                conn.cmd_query("SELECT COUNT(*) FROM sbtest")
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    et["SELECT_COUNT"] += 1
                try:
                    conn.reconnect()
                    recon_count += 1
                    if recon_count > max_reconnects:
                        log("Worker %d: too many reconnects" % tid)
                        break
                except:
                    pass
                time.sleep(0.1)
                continue

            # SELECT by k
            try:
                k_val = (tid + ops) % 100
                conn.cmd_query("SELECT id, k, c FROM sbtest WHERE k=%d LIMIT 3" % k_val)
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    et["SELECT_K"] += 1
                try:
                    conn.reconnect()
                    recon_count += 1
                except:
                    pass

            # SELECT range
            try:
                start_id = ((tid + ops) * 7) % 1000
                conn.cmd_query("SELECT id, k, c, pad FROM sbtest WHERE id BETWEEN %d AND %d LIMIT 5" % (
                    start_id, start_id + 10))
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    et["SELECT_RANGE"] += 1
                try:
                    conn.reconnect()
                    recon_count += 1
                except:
                    pass

            # SELECT by pk
            try:
                conn.cmd_query("SELECT id FROM sbtest WHERE id=%d" % (((tid + ops) * 17) % table_size))
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    et["SELECT_PK"] += 1
                try:
                    conn.reconnect()
                    recon_count += 1
                except:
                    pass

            latency_ms = (time.time() - op_start) * 1000
            with lock:
                transactions += 1
                latencies.append(latency_ms)
                if len(latencies) > 10000:
                    latencies = latencies[-5000:]

            ops += 1
            if ops % 20 == 0:
                time.sleep(0.02)

        conn.close()

    log("=" * 60)
    log("SQLRustGo Soak Test (raw TCP MySQL)")
    log("=" * 60)
    log("Host: %s:%d  DB: %s" % (args.host, args.port, args.database))
    log("Duration: %.1fh  Threads: %d  Table: %d" % (args.hours, args.threads, args.table_size))
    log("Results: %s" % results_dir)
    log("=" * 60)

    server_pid = find_pid()
    if server_pid:
        log("Server PID: %d" % server_pid)
    else:
        log("WARNING: Could not find server PID")

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
                    q = queries
                    t = transactions
                    e = errors
                    lat = list(latencies)

                rss_mb = 0
                fd_count = 0
                rss_kb = 0
                if server_pid:
                    rss_kb, fd_count = get_metrics(server_pid)
                    rss_mb = rss_kb / 1024.0
                    if sample == 0:
                        initial_rss_kb = rss_kb

                rss_delta_mb = (rss_kb - initial_rss_kb) / 1024.0 if server_pid else 0.0

                tps_val = t / elapsed if elapsed > 0 else 0
                qps_val = q / elapsed if elapsed > 0 else 0

                p50 = p95 = p99 = 0
                if lat:
                    lat.sort()
                    n = len(lat)
                    p50 = lat[int(n * 0.50)]
                    p95 = lat[int(n * 0.95)]
                    p99 = lat[int(n * 0.99)]

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

                if rss_mb > args.soft_rss_mb:
                    log("WARN: RSS %.0fMB > soft %dMB" % (rss_mb, args.soft_rss_mb))

                if sample % 4 == 0 and sample > 0:
                    log("[s%d %.2fh] RSS=%.0fMB(d%+.0f) FD=%d TPS=%.1f QPS=%.1f err=%d" % (
                        sample, elapsed/3600.0, rss_mb, rss_delta_mb, fd_count, tps_val, qps_val, e
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
        q = queries
        t = transactions
        e = errors
        lat = list(latencies)
        qt_copy = dict(qt)
        et_copy = dict(et)

    elapsed = time.time() - start_ts
    elapsed_h = elapsed / 3600.0
    lat.sort()
    n = len(lat)
    p50 = lat[int(n * 0.50)] if n > 0 else 0
    p95 = lat[int(n * 0.95)] if n > 0 else 0
    p99 = lat[int(n * 0.99)] if n > 0 else 0
    rss_mb_final = 0
    if server_pid:
        rss_kb, _ = get_metrics(server_pid)
        rss_mb_final = rss_kb / 1024.0
    tps_final = t / elapsed if elapsed > 0 else 0
    qps_final = q / elapsed if elapsed > 0 else 0
    verdict = "PASS" if e == 0 and status == "COMPLETED" else "REVIEW"

    with open(report_file, "w") as f:
        f.write("""# Soak Stability Report

**Run**: %s
**Duration**: %.3fh (target %.1fh)
**Status**: %s
**Host**: %s:%d
**Threads**: %d
**Table size**: %d

## Throughput

| Metric | Value |
|--------|-------|
| TPS | %.2f/s |
| QPS | %.2f/s |
| Total Transactions | %d |
| Total Queries | %d |
| Total Errors | %d |

## Query Mix

%s

## Latency (ms)

| Percentile | Value |
|------------|-------|
| p50 | %.2f |
| p95 | %.2f |
| p99 | %.2f |

## Error Breakdown

%s

## Resource Results

| Criterion | Threshold | Measured | Status |
|-----------|-----------|----------|--------|
| RSS hard limit | %d MB | %.0f MB | %s |
| Errors | 0 | %d | %s |
| Status | COMPLETED | %s | %s |

## Verdict

**%s** — %.3fh soak, TPS=%.1f, QPS=%.1f, errors=%d

Metrics: %s
""" % (
    datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    elapsed_h, args.hours, status, args.host, args.port, args.threads, args.table_size,
    tps_final, qps_final, t, q, e,
    json.dumps(qt_copy, indent=2),
    p50, p95, p99,
    json.dumps(et_copy, indent=2) if et_copy else "None",
    args.hard_rss_mb, rss_mb_final,
    "PASS" if rss_mb_final < args.hard_rss_mb else "FAIL",
    e, "PASS" if e == 0 else "FAIL",
    status, "PASS" if status == "COMPLETED" else "FAIL",
    verdict, elapsed_h, tps_final, qps_final, e,
    metrics_file,
))
    log("Report: %s" % report_file)
    print("DONE")

if __name__ == "__main__":
    main()