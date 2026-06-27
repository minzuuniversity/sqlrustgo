#!/usr/bin/env python3
"""
raw_mysql_soak.py - Minimal MySQL text protocol client via raw sockets.
Bypasses mysql-connector/pymysql to avoid SET NAMES/AUTOCOMMIT issues.
Uses connection-per-operation model for simplicity.
"""

import argparse
import socket
import struct
import time
import threading
import os
import json
from datetime import datetime, timezone
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

def recv_full(sock, n):
    data = b""
    while len(data) < n:
        chunk = sock.recv(n - len(data))
        if not chunk:
            return None
        data += chunk
    return data

def send_packet(sock, seq, data):
    body = struct.pack("<I", len(data))[:3] + bytes([seq]) + data
    sock.sendall(body)

def mysql_exec(sql):
    """Connect, exec SQL, return (ok, error_msg). Closes connection always."""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(30)
    try:
        s.connect(("127.0.0.1", 3396))
    except Exception as e:
        return False, "CONNECT:" + str(e)

    # Read server handshake
    try:
        hdr = recv_full(s, 4)
        if not hdr:
            return False, "NO_HANDSHAKE"
        length = struct.unpack("<I", hdr[:3] + b'\x00')[0]
        seq = hdr[3]
        payload = recv_full(s, length)
        if not payload:
            return False, "NO_PAYLOAD"
    except Exception as e:
        return False, "HANDSHAKE:" + str(e)

    # Send auth: big-endian 0xa87b capability + username
    cap = struct.pack(">I", 0xa87b) + b"\x00" * 28
    auth = cap + b"root\x00"
    send_packet(s, seq + 1, auth)

    try:
        auth_hdr = recv_full(s, 4)
        if not auth_hdr:
            return False, "NO_AUTH_RESP"
        auth_len = struct.unpack("<I", auth_hdr[:3] + b'\x00')[0]
        auth_data = recv_full(s, auth_len)
        if not auth_data or auth_data[0] != 0x00:
            return False, "AUTH_FAILED"
    except Exception as e:
        return False, "AUTH:" + str(e)

    # Send query
    send_packet(s, seq + 2, sql.encode())

    try:
        responses = []
        old_timeout = s.gettimeout()
        s.settimeout(2.0)
        while True:
            try:
                hdr = s.recv(4)
                if not hdr:
                    break
                rlen = struct.unpack("<I", hdr[:3] + b'\x00')[0]
                rdata = recv_full(s, rlen)
                responses.append(rdata)
                if rlen == 5 and rdata and rdata[0] == 0xfe:
                    break
            except socket.timeout:
                break
        s.settimeout(old_timeout)
    except Exception as e:
        return False, "QUERY_RECV:" + str(e)

    if not responses:
        return False, "NO_RESPONSE"

    first = responses[0]
    if first[0] == 0x00:
        return True, None
    elif first[0] == 0xff:
        errno = struct.unpack("<H", first[1:3])[0]
        msg = first[3:].decode("utf-8", errors="replace")
        return False, "ERROR:%d:%s" % (errno, msg)
    else:
        # Result set = success for SELECT
        return True, None


parser = argparse.ArgumentParser()
parser.add_argument("--host", default="127.0.0.1")
parser.add_argument("--port", type=int, default=3396)
parser.add_argument("--hours", type=float, default=1)
parser.add_argument("--threads", type=int, default=8)
parser.add_argument("--interval", type=int, default=30)
parser.add_argument("--table-size", type=int, default=10000)
parser.add_argument("--results-dir", default=None)
parser.add_argument("--hard-rss-mb", type=int, default=8192)
parser.add_argument("--soft-rss-mb", type=int, default=4096)
args = parser.parse_args()

results_dir = args.results_dir or "test_results/raw_soak_" + str(int(time.time()))
os.makedirs(results_dir, exist_ok=True)
metrics_file = os.path.join(results_dir, "metrics.csv")
report_file = os.path.join(results_dir, "STABILITY_REPORT.md")
log_file = os.path.join(results_dir, "soak.log")
pid_file = os.path.join(results_dir, "soak.pid")

start_ts = time.time()
end_ts = start_ts + args.hours * 3600

lock = threading.Lock()
queries = 0
transactions = 0
errors = 0
latencies = []
queries_by_type = defaultdict(int)
errors_by_type = defaultdict(int)

def log(msg):
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    line = "[%s] %s" % (ts, msg)
    print(line)
    with open(log_file, "a") as f:
        f.write(line + "\n")

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

def worker(tid, table_size):
    global queries, transactions, errors, latencies, queries_by_type, errors_by_type
    ops = 0
    total_errors = 0

    while time.time() < end_ts:
        op_start = time.time()
        tid_val = tid * 1000000 + (ops % table_size)
        op_errors = 0

        # INSERT
        sql = "INSERT INTO sbtest VALUES (NULL, %d, REPEAT('c', 120), REPEAT('p', 60))" % tid_val
        ok, err = mysql_exec(sql)
        if ok:
            with lock:
                queries += 1
                queries_by_type["INSERT"] += 1
        else:
            op_errors += 1
            total_errors += 1
            with lock:
                errors += 1
                errors_by_type["INSERT"] += err

        # SELECT COUNT
        ok, err = mysql_exec("SELECT COUNT(*) FROM sbtest")
        if ok:
            with lock:
                queries += 1
                queries_by_type["SELECT"] += 1
        else:
            op_errors += 1
            total_errors += 1
            with lock:
                errors += 1

        # UPDATE
        sql = "UPDATE sbtest SET c=REPEAT('u', 120) WHERE id IN (SELECT id FROM (SELECT id FROM sbtest LIMIT 1) AS t)"
        ok, err = mysql_exec(sql)
        if ok:
            with lock:
                queries += 1
                queries_by_type["UPDATE"] += 1
        else:
            op_errors += 1
            total_errors += 1
            with lock:
                errors += 1
                errors_by_type["UPDATE"] += err

        # SELECT by index
        sql = "SELECT id, k FROM sbtest WHERE k=%d LIMIT 3" % (tid % 100)
        ok, err = mysql_exec(sql)
        if ok:
            with lock:
                queries += 1
                queries_by_type["SELECT"] += 1
        else:
            op_errors += 1
            total_errors += 1
            with lock:
                errors += 1

        # DELETE
        sql = "DELETE FROM sbtest WHERE id IN (SELECT id FROM (SELECT id FROM sbtest LIMIT 1) AS t)"
        ok, err = mysql_exec(sql)
        if ok:
            with lock:
                queries += 1
                queries_by_type["DELETE"] += 1
        else:
            op_errors += 1
            total_errors += 1
            with lock:
                errors += 1

        latency_ms = (time.time() - op_start) * 1000
        with lock:
            transactions += 1
            latencies.append(latency_ms)
            if len(latencies) > 10000:
                latencies = latencies[-5000:]

        ops += 1
        if ops % 20 == 0:
            time.sleep(0.05)

    log("Worker %d done: ops=%d errors=%d" % (tid, ops, total_errors))


log("=" * 60)
log("Raw MySQL text-protocol Soak Test (conn-per-op)")
log("=" * 60)
log("Host: 127.0.0.1:3396")
log("Duration: %.1fh  Threads: %d" % (args.hours, args.threads))
log("Results: %s" % results_dir)
log("=" * 60)

with open(pid_file, "w") as f:
    f.write(str(os.getpid()) + "\n")

server_pid = find_pid()
if server_pid:
    log("Server PID: %d" % server_pid)
else:
    log("WARNING: Could not find server PID")

log("Starting %d worker threads..." % args.threads)

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
                log("KILL: RSS %.0fMB > hard limit %dMB" % (rss_mb, args.hard_rss_mb))
                for f in futures:
                    f.cancel()
                executor.shutdown(wait=False)
                status = "KILLED"
                break
            if rss_mb > args.soft_rss_mb:
                log("WARN: RSS %.0fMB > soft limit %dMB" % (rss_mb, args.soft_rss_mb))
            if sample % 4 == 0 and sample > 0:
                remain = end_ts - time.time()
                log("  [s%d %.2fh remain=%.2fh] RSS=%.0fMB(d%+.0f) FD=%d TPS=%.1f QPS=%.1f err=%d" % (
                    sample, elapsed/3600.0, remain/3600.0,
                    rss_mb, rss_delta_mb, fd_count, tps_val, qps_val, e
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
    qt = dict(queries_by_type)
    et = dict(errors_by_type)

elapsed = time.time() - start_ts
lat.sort()
n = len(lat)
p50 = lat[int(n * 0.50)] if n > 0 else 0
p95 = lat[int(n * 0.95)] if n > 0 else 0
p99 = lat[int(n * 0.99)] if n > 0 else 0

rss_mb_final = 0
if server_pid:
    rss_kb, _ = get_metrics(server_pid)
    rss_mb_final = rss_kb / 1024.0

tps_val = t / elapsed if elapsed > 0 else 0
qps_val = q / elapsed if elapsed > 0 else 0
verdict = "PASS" if e == 0 and status == "COMPLETED" else "REVIEW"

with open(report_file, "w") as f:
    f.write("""# Soak Stability Report

**Run**: %s
**Duration**: %.3fh (target %.1fh)
**Status**: %s
**Host**: 127.0.0.1:3396
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
    elapsed/3600.0, args.hours, status, args.threads,
    tps_val, qps_val, t, q, e,
    json.dumps(qt, indent=2),
    p50, p95, p99,
    json.dumps(et, indent=2) if et else "None",
    args.hard_rss_mb, rss_mb_final,
    "PASS" if rss_mb_final < args.hard_rss_mb else "FAIL",
    e, "PASS" if e == 0 else "FAIL",
    status, "PASS" if status == "COMPLETED" else "FAIL",
    verdict, elapsed/3600.0, tps_val, qps_val, e,
    metrics_file,
))

log("Report: %s" % report_file)
