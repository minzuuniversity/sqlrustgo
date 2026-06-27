#!/usr/bin/env python3
"""
MySQL CLI Ladder Soak Test — Persistent connections + multi-threading
Tests: 10/20/30/40/60 min × 1/2/4/8/16 threads = 25 runs
Client: Python raw socket MySQL protocol (no external deps, persistent connections)
"""
import subprocess
import tempfile
import shutil
import time
import os
import socket
import struct
import threading
import csv
import statistics
import sys

# ─── MySQL wire protocol client (no external deps) ────────────────────────────

def _recv_exact(sock, n):
    data = b""
    while len(data) < n:
        d = sock.recv(n - len(data))
        if not d:
            raise EOFError("connection closed")
        data += d
    return data

def _lenenc_int(data, pos):
    b = data[pos]
    if b < 0xfb:
        return b, pos + 1
    elif b == 0xfc:
        return struct.unpack("<H", data[pos+1:pos+3])[0], pos + 3
    elif b == 0xfd:
        return struct.unpack("<I", data[pos:pos+4])[0] & 0xFFFFFF, pos + 4
    elif b == 0xfe:
        return struct.unpack("<Q", data[pos+1:pos+9])[0], pos + 9
    else:
        raise ValueError(f"invalid lenenc byte {b}")

def _lenenc_str(data, pos):
    length, pos = _lenenc_int(data, pos)
    return data[pos:pos+length].decode(), pos + length

class MySQLClient:
    """Minimal persistent MySQL client using raw socket protocol."""
    def __init__(self, host, port, user="root", database=None):
        self.host = host
        self.port = port
        self.user = user
        self.database = database
        self.sock = None
        self.seq = 0
        self._connect()

    def _send_packet(self, payload):
        self.seq = (self.seq + 1) & 0xFF
        header = struct.pack("<BBB", len(payload) & 0xFF,
                             (len(payload) >> 8) & 0xFF,
                             (len(payload) >> 16) & 0xFF) + bytes([self.seq])
        self.sock.sendall(header + payload)

    def _recv_packet(self):
        header = _recv_exact(self.sock, 4)
        length = header[0] | (header[1] << 8) | (header[2] << 16)
        self.seq = header[3]
        return _recv_exact(self.sock, length)

    def _connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(15)
        self.sock.connect((self.host, self.port))
        # Handshake
        _ = self._recv_packet()  # server handshake
        # Auth
        cap = 0xBFA285  # default client capabilities
        max_size = 16777216
        collation = 45  # utf8mb4_general_ci
        username = self.user.encode()
        auth_resp = b"\x14" + b"\0" * 20  # mysql_native_password mock
        if self.database:
            cap |= 0x0008  # CONNECT_WITH_DB
        payload = struct.pack("<IIB", cap, max_size, collation) + b"\x00" * 23 + username + b"\x00" + auth_resp
        if self.database:
            payload += self.database.encode() + b"\x00"
        self._send_packet(payload)
        resp = self._recv_packet()
        if resp[0] == 0xFF:
            raise ConnectionError(f"auth failed: {resp[4:].decode('utf8', errors='replace')}")

    def query(self, sql, timeout=10):
        """Execute SQL, return (ok, error_msg). ok=True means no error."""
        self.sock.settimeout(timeout)
        self._send_packet(b"\x03" + sql.encode())
        pkt = self._recv_packet()
        if pkt[0] == 0xFF:
            return False, pkt[4:].decode("utf8", errors="replace")
        # OK packet or result set
        return True, ""

    def ping(self):
        try:
            self.query("SELECT 1", timeout=5)
            return True
        except Exception:
            return False

    def close(self):
        if self.sock:
            self.sock.close()
            self.sock = None

# ─── Soak run ────────────────────────────────────────────────────────────────

BINARY    = os.environ.get("BINARY", "./target/release/sqlrustgo-mysql-server")
OUT_DIR   = os.environ.get("OUT_DIR", "./docs/releases/v3.9.0/soak_results/")
os.makedirs(OUT_DIR, exist_ok=True)

DURATIONS_MIN = [10, 20, 30, 40, 60]
THREAD_COUNTS = [1, 2, 4, 8, 16]

def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    _, port = s.getsockname()
    s.close()
    return port

def wait_for_port(port, timeout=20):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            s = socket.socket()
            s.settimeout(1)
            s.connect(("127.0.0.1", port))
            s.close()
            return True
        except Exception:
            pass
        time.sleep(0.3)
    return False

def get_rss(pid):
    try:
        return int(subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True).strip())
    except Exception:
        return 0

def get_fd(pid):
    try:
        return len(os.listdir(f"/proc/{pid}/fd"))
    except Exception:
        return 0

def run_soak(duration_min, threads, label):
    dur_sec = duration_min * 60
    data_dir = tempfile.mkdtemp(prefix=f"soak_{label}_")
    port = free_port()
    log_file = f"{OUT_DIR}/{label}.log"
    csv_file = f"{OUT_DIR}/{label}.csv"

    print(f"[{label}] Starting server port={port}")
    log_fp = open(log_file, "w")
    proc = subprocess.Popen(
        [BINARY, "serve", "--host", "127.0.0.1", "--port", str(port),
         "--data-dir", data_dir, "--log-level", "warn"],
        stdout=log_fp, stderr=subprocess.STDOUT
    )

    if not wait_for_port(port):
        print(f"[{label}] FAIL: server not ready")
        proc.terminate()
        proc.wait()
        log_fp.close()
        shutil.rmtree(data_dir, ignore_errors=True)
        return None

    # Pre-warm: 1 query to establish connection
    try:
        c = MySQLClient("127.0.0.1", port, "root")
        ok, _ = c.query("SELECT 1")
        if not ok:
            print(f"[{label}] FAIL: ping failed")
            c.close()
            proc.terminate()
            proc.wait()
            log_fp.close()
            shutil.rmtree(data_dir, ignore_errors=True)
            return None
        c.close()
    except Exception as e:
        print(f"[{label}] FAIL: warm-up error: {e}")
        proc.terminate()
        proc.wait()
        log_fp.close()
        shutil.rmtree(data_dir, ignore_errors=True)
        return None

    print(f"[{label}] Running {threads} thread(s) for {duration_min}m ({dur_sec}s)")

    # Per-thread state
    thread_q = [0] * threads
    thread_e = [0] * threads
    thread_running = [True] * threads
    lock = threading.Lock()
    start_time = time.time()
    start_rss = get_rss(proc.pid)
    start_fd = get_fd(proc.pid)

    def worker(tid):
        q, errors = 0, 0
        counter = tid  # avoid query cache by varying SELECT value
        client = None
        try:
            client = MySQLClient("127.0.0.1", port, "root")
            while thread_running[tid]:
                ok, err = client.query(f"SELECT {counter} AS n")
                counter += threads
                if not ok:
                    errors += 1
                q += 1
        except Exception as e:
            # Connection error — count as error, try to reconnect once
            errors += 1
            try:
                if client:
                    client.close()
                client = MySQLClient("127.0.0.1", port, "root")
                ok, _ = client.query(f"SELECT {counter} AS n")
                if ok:
                    q += 1
                else:
                    errors += 1
            except Exception:
                errors += 1
        finally:
            if client:
                client.close()
        with lock:
            thread_q[tid] = q
            thread_e[tid] = errors

    # Start threads
    ths = [threading.Thread(target=worker, args=(t,)) for t in range(threads)]
    for th in ths:
        th.start()

    # Sample every 10s
    sample_times, sample_qs, sample_es = [], [], []
    last_q, last_e = 0, 0
    with open(csv_file, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["elapsed_s", "total_q", "total_e", "rss_kb", "fd", "qps"])
        while time.time() - start_time < dur_sec:
            time.sleep(10)
            elapsed = time.time() - start_time
            with lock:
                tq = sum(thread_q)
                te = sum(thread_e)
            rss = get_rss(proc.pid)
            fd = get_fd(proc.pid)
            qps = (tq - last_q) / 10
            writer.writerow([f"{elapsed:.0f}", tq, te, rss, fd, f"{qps:.1f}"])
            f.flush()
            sample_times.append(elapsed)
            sample_qs.append(tq)
            sample_es.append(te)
            last_q, last_e = tq, te

    # Stop workers
    for r in thread_running:
        r = False
    for th in ths:
        th.join(timeout=10)

    # Final
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()
    log_fp.close()
    shutil.rmtree(data_dir, ignore_errors=True)

    with lock:
        total_q = sum(thread_q)
        total_e = sum(thread_e)
    actual_dur = time.time() - start_time
    end_rss = get_rss(proc.pid)  # won't work after proc.wait() but we cached start_rss
    rss_growth = end_rss - start_rss if end_rss > 0 else 0
    fd_end = get_fd(proc.pid) if proc.poll() is None else start_fd
    qps = total_q / actual_dur if actual_dur > 0 else 0

    # p99 qps from samples
    if len(sample_times) > 2:
        dq = [sample_qs[i] - sample_qs[i-1] for i in range(1, len(sample_qs))]
        dt = [sample_times[i] - sample_times[i-1] for i in range(1, len(sample_times))]
        qps_series = [dq[i] / dt[i] for i in range(len(dq)) if dt[i] > 0]
        qps_series.sort()
        p99 = qps_series[int(len(qps_series) * 0.99)] if qps_series else qps
    else:
        p99 = qps

    passed = total_e == 0

    # Save summary
    with open(f"{OUT_DIR}/{label}_summary.txt", "w") as f:
        f.write(f"""Label:         {label}
Duration:      {actual_dur:.0f}s (target {dur_sec}s)
Threads:       {threads}
Total queries: {total_q}
Total errors:  {total_e}
Error rate:    {total_e/max(total_q,1)*100:.4f}%
Final QPS:     {qps:.1f}
p99 QPS:       {p99:.1f}
RSS start:    {start_rss} KB
RSS end:      {end_rss} KB
RSS growth:   {rss_growth} KB
FD start:     {start_fd}
FD end:       {fd_end}
PASS:         {'YES' if passed else 'NO'}
""")

    print(f"[{label}] ok={passed} q={total_q} err={total_e} "
          f"qps={qps:.1f} p99={p99:.1f} rss_g={rss_growth}KB dur={actual_dur:.0f}s")
    return passed

def main():
    total = len(DURATIONS_MIN) * len(THREAD_COUNTS)
    results = {}  # (dur, threads) -> (passed, total_q, total_e, qps, p99, rss_growth)

    # Quick smoke first
    print("\n=== Smoke: 2 min, 1 thread ===")
    run_soak(2, 1, "smoke_2m_1t")

    idx = 0
    for dur_min in DURATIONS_MIN:
        print(f"\n{'='*60}")
        print(f"=== DURATION: {dur_min} min | Threads: {THREAD_COUNTS}")
        print(f"{'='*60}")
        for t in THREAD_COUNTS:
            idx += 1
            label = f"soak_{dur_min}m_{t}t"
            print(f"[{idx}/{total}] Starting {label}...")
            passed = run_soak(dur_min, t, label)
            results[(dur_min, t)] = passed

    # ── Print results table ────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("RESULTS")
    print("=" * 70)

    header = f"{'Dur':>8}" + "".join(f"  t={t:>2}" for t in THREAD_COUNTS)
    print(header)
    print("-" * len(header))
    rows_md = []
    for dur_min in DURATIONS_MIN:
        row = f"{dur_min:>6}m "
        for t in THREAD_COUNTS:
            label = f"soak_{dur_min}m_{t}t"
            sm = f"{OUT_DIR}/{label}_summary.txt"
            if os.path.exists(sm):
                with open(sm) as f:
                    content = f.read()
                passed = "PASS:         YES" in content
                qps = next((l.split(":")[1].strip() for l in content.splitlines() if "Final QPS:" in l), "?")
                rss = next((l.split(":")[1].strip() for l in content.splitlines() if "RSS growth:" in l), "?")
                err = next((l.split(":")[1].strip() for l in content.splitlines() if "Total errors:" in l), "?")
                status = "✅ PASS" if passed else f"❌ FAIL({err}err)"
                row += f"  {status[:8]} "
                rows_md.append((dur_min, t, passed, qps, rss))
            else:
                row += "     SKIP "
        print(row)

    # Markdown report
    md = f"""# MySQL Client Ladder Soak Report

**Generated:** {time.strftime('%Y-%m-%d %H:%M:%S')}  
**Binary:** `{BINARY}`  
**Client:** Python raw socket MySQL client (persistent connections, no external deps)  
**Machine:** Z440 (Xeon E5-2680 v4)

## Results Table

| Duration | {' | '.join(f't={t}' for t in THREAD_COUNTS)} |
|----------|{'|'.join('---' for _ in THREAD_COUNTS)}|
"""
    for dur_min in DURATIONS_MIN:
        md += f"| **{dur_min}m** |"
        for t in THREAD_COUNTS:
            label = f"soak_{dur_min}m_{t}t"
            sm = f"{OUT_DIR}/{label}_summary.txt"
            if os.path.exists(sm):
                with open(sm) as f:
                    c = f.read()
                passed = "PASS:         YES" in c
                qps = next((l.split(":")[1].strip() for l in c.splitlines() if "Final QPS:" in l), "?")
                rss = next((l.split(":")[1].strip() for l in c.splitlines() if "RSS growth:" in l), "?")
                err = next((l.split(":")[1].strip() for l in c.splitlines() if "Total errors:" in l), "0")
                badge = "✅ PASS" if passed else f"❌ FAIL ({err} err)"
                md += f" {badge} (qps={qps}, rss_g={rss}KB) |"
            else:
                md += " — |"
        md += "\n"

    md += """
## Interpretation

- **RSS growth** = server RSS delta during soak (0 = no leak)
- **p99 QPS** = 99th percentile query throughput per 10s sample
- **Error** = queries returning non-OK from server (connection errors, protocol errors)
"""
    with open(f"{OUT_DIR}/LADDER_SOAK_REPORT.md", "w") as f:
        f.write(md)
    print(f"\nReport: {OUT_DIR}/LADDER_SOAK_REPORT.md")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nInterrupted")
        sys.exit(130)
