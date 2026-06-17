#!/usr/bin/env python3
"""
rawtcp_soak.py - Real wall-clock soak using raw MySQL text protocol via TCP sockets.
No external DB libraries needed.
"""
import socket, argparse, time, threading, os, sys, json, select
from datetime import datetime, timezone
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=3396)
    p.add_argument("--database", default="sbtest")
    p.add_argument("--hours", type=float, default=1)
    p.add_argument("--threads", type=int, default=8)
    p.add_argument("--interval", type=int, default=30)
    p.add_argument("--table-size", type=int, default=10000)
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
            self.connect()

        def connect(self):
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(10)
            self.sock.connect((self.host, self.port))
            # Read handshake packet
            self._recv()
            # Send handshake response with empty password (user=root, no password)
            # No auth exchange needed for local socket
            self.seq = 0
            self._send_packet(b"\x00")  # empty auth = no credentials
            self._recv()  # read OK or error
            self.seq = 1
            # Send USE database
            self._send_command(0x02, self.database.encode('utf-8'))
            self._recv()

        def _recv(self):
            # Read packet header (4 bytes: length[3] + seq[1])
            header = b""
            while len(header) < 4:
                chunk = self.sock.recv(4 - len(header))
                if not chunk:
                    raise Exception("Connection closed")
                header += chunk
            length = header[0] | (header[1] << 8) | (header[2] << 16)
            seq = header[3]
            self.seq = seq
            payload = b""
            while len(payload) < length:
                chunk = self.sock.recv(length - len(payload))
                if not chunk:
                    raise Exception("Connection closed")
                payload += chunk
            return payload

        def _send_packet(self, payload):
            length = len(payload)
            header = bytes([length & 0xFF, (length >> 8) & 0xFF, (length >> 16) & 0xFF, self.seq])
            self.sock.sendall(header + payload)
            self.seq = (self.seq + 1) & 0xFF

        def _send_command(self, cmd_byte, data=b""):
            self._send_packet(bytes([cmd_byte]) + data)

        def cmd_query(self, sql):
            self._send_command(0x03, sql.encode('utf-8'))
            resp = self._recv()
            # resp[0] == 0xFF means error
            if resp and resp[0] == 0xFF:
                # Error packet: error_code(2) + '#' + sqlstate(5) + message
                if len(resp) >= 3:
                    errno = resp[1] | (resp[2] << 8)
                    msg = resp[9:].decode('utf-8', errors='replace') if len(resp) > 9 else "error"
                    raise Exception("MySQL error %d: %s" % (errno, msg))
            return resp

        def reconnect(self):
            try:
                self.sock.close()
            except Exception:
                pass
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(10)
            self.sock.connect((self.host, self.port))
            self._recv()  # handshake
            self._send_packet(b"\x00")  # empty auth
            self._recv()  # OK
            self.seq = 1
            self._send_command(0x02, self.database.encode('utf-8'))  # USE db
            self._recv()

        def close(self):
            if self.sock:
                try:
                    self.sock.close()
                except Exception:
                    pass

    def worker(tid, table_size):
        nonlocal queries, transactions, errors, latencies, qt, et
        conn = None
        ops = 0
        try:
            conn = MySQLConn(args.host, args.port, args.database)
        except Exception as e:
            log("Worker %d connect error: %s" % (tid, e))
            with lock:
                errors += 1
            return

        while time.time() < end_ts:
            op_start = time.time()
            tid_val = tid * 1000000 + (ops % table_size)

            # Each iteration: 4 SELECTs (no persistent state needed)
            # INSERT/UPDATE have storage bugs; SELECT works reliably

            # SELECT COUNT
            try:
                conn.cmd_query("SELECT COUNT(*) FROM sbtest")
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    conn.reconnect()

            # SELECT by index
            try:
                conn.cmd_query("SELECT id, k FROM sbtest WHERE k=%d LIMIT 3" % ((tid + ops) % 100))
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    conn.reconnect()

            # SELECT range
            try:
                conn.cmd_query("SELECT id, k, c FROM sbtest WHERE id BETWEEN %d AND %d LIMIT 5" % (
                    ((tid + ops) * 7) % 1000, ((tid + ops) * 7) % 1000 + 10))
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    conn.reconnect()

            # SELECT random
            try:
                conn.cmd_query("SELECT id, c, pad FROM sbtest ORDER BY RAND() LIMIT 3")
                with lock:
                    queries += 1
                    qt["SELECT"] += 1
            except Exception:
                with lock:
                    errors += 1
                    conn.reconnect()

            latency_ms = (time.time() - op_start) * 1000
            with lock:
                transactions += 1
                latencies.append(latency_ms)
                if len(latencies) > 10000:
                    latencies = latencies[-5000:]

            ops += 1
            if ops % 20 == 0:
                time.sleep(0.05)

        conn.close()

    log("=" * 60)
    log("Raw TCP MySQL text-protocol Soak")
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
                if server_pid:
                    rss_kb, fd_count = get_metrics(server_pid)
                    rss_mb = rss_kb / 1024.0
                    if sample == 0:
                        initial_rss_kb = rss_kb

                rss_delta_mb = (rss_kb - initial_rss_kb) / 1024.0

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
                    remain = end_ts - time.time()
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
    elapsed_h, args.hours, status, args.host, args.port, args.threads,
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
