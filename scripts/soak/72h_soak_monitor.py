#!/usr/bin/env python3
"""
72-hour soak test with continuous 10-minute reporting.
Uses sysbench oltp_read_write with HTTP monitoring API.
Auto-restarts server on crash; logs all data to CSV.
"""
import argparse, csv, json, os, re, shutil, signal, socket, subprocess, sys, tempfile, time, urllib.request

MONITOR_INTERVAL = 600  # 10 min

# ── Helpers ────────────────────────────────────────────────────────────────

def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    _, p = s.getsockname()
    s.close()
    return p

def wait_tcp(port, timeout=60):
    for _ in range(int(timeout / 0.5)):
        try:
            ss = socket.socket()
            ss.settimeout(1)
            ss.connect(("127.0.0.1", port))
            ss.close()
            return True
        except Exception:
            pass
        time.sleep(0.5)
    return False

def get_rss(pid):
    try:
        out = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True, timeout=5)
        return int(out.strip())
    except Exception:
        return 0

def wal_size_mb(data_dir):
    path = os.path.join(data_dir, "sqlrustgo.wal")
    try:
        return os.path.getsize(path) / (1024 * 1024)
    except Exception:
        return 0.0

def http_stats(port):
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{port}/stats", timeout=5)
        return json.loads(r.read())
    except Exception:
        return {}

def disk_usage(path="/"):
    s = os.statvfs(path)
    return (1 - s.f_bfree / s.f_blocks) * 100

def total_ram_mb():
    try:
        out = subprocess.check_output(["sysctl", "-n", "hw.memsize"], text=True, timeout=5)
        return int(out.strip()) / (1024 * 1024)
    except Exception:
        return 16384  # fallback M4 default

def kill_stale_servers():
    try:
        out = subprocess.check_output(["pgrep", "-f", "sqlrustgo-mysql-server"], text=True, stderr=subprocess.DEVNULL)
        for line in out.splitlines():
            try:
                subprocess.run(["kill", line.strip()], timeout=3)
            except Exception:
                pass
    except Exception:
        pass
    time.sleep(2)

def signal_handler(sig, frame):
    print(f"\n=== Caught signal {sig}, saving final state ===")
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

# ── Main ───────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="72-hour soak test with monitoring")
    parser.add_argument("--binary", default="./target/release/sqlrustgo-mysql-server")
    parser.add_argument("--server-threads", type=int, default=20)
    parser.add_argument("--sysbench-threads", type=int, default=8)
    parser.add_argument("--table-size", type=int, default=10000)
    parser.add_argument("--duration-min", type=int, default=72*60)
    parser.add_argument("--out-dir", default="./test_results/72h_soak")
    parser.add_argument("--max-wal-gb", type=float, default=10.0, help="warn threshold")
    parser.add_argument("--max-rss-mb", type=float, default=0, help="0=80%% of total RAM")
    args = parser.parse_args()

    os.makedirs(args.out_dir, exist_ok=True)
    total_ram = total_ram_mb()
    max_rss_mb = args.max_rss_mb or (total_ram * 0.8)
    ts = int(time.time())

    csv_path = os.path.join(args.out_dir, f"72h_soak_{ts}.csv")
    log_dir = os.path.join(args.out_dir, f"logs_{ts}")
    os.makedirs(log_dir, exist_ok=True)

    with open(csv_path, "w", newline="") as cf:
        writer = csv.writer(cf)
        writer.writerow([
            "timestamp", "elapsed_min", "wal_mb", "rss_mb",
            "total_q", "avg_dur_ms", "slow_q", "qps_from_last",
            "active_conn", "total_conn", "run_number",
            "server_alive", "sysbench_alive",
        ])

    print("=" * 70)
    print(f"  72H SOAK TEST — {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"  Binary : {args.binary}")
    print(f"  Server threads : {args.server_threads}")
    print(f"  Sysbench threads : {args.sysbench_threads}")
    print(f"  Table size : {args.table_size}")
    print(f"  Duration : {args.duration_min} min ({args.duration_min/60:.0f}h)")
    print(f"  Output : {args.out_dir}")
    print(f"  Max RSS : {max_rss_mb:.0f} MB ({total_ram:.0f} MB total)")
    print(f"  Max WAL : {args.max_wal_gb} GB")
    print("=" * 70)

    run_number = 0
    t_test_start = time.time()
    last_total_q = 0
    last_q_time = time.time()

    while True:
        total_elapsed = (time.time() - t_test_start) / 60
        if total_elapsed >= args.duration_min:
            print(f"\n=== Target duration reached ({args.duration_min} min) ===")
            break

        run_number += 1
        data_dir = tempfile.mkdtemp(prefix=f"soak72_run{run_number}_")
        port = free_port()
        mon_port = free_port()
        log_path = os.path.join(log_dir, f"run{run_number}_server.log")

        # ── Start server ────────────────────────────────────
        log_file = open(log_path, "w")
        srv_proc = subprocess.Popen(
            [args.binary, "serve",
             "--host", "127.0.0.1",
             "--port", str(port),
             "--data-dir", data_dir,
             f"--server-threads={args.server_threads}",
             "--log-level=error",
             f"--monitor-port={mon_port}"],
            stdout=log_file, stderr=subprocess.STDOUT,
            env={**os.environ, "RUST_LOG": "error"},
        )

        if not wait_tcp(port):
            print(f"[run{run_number}] Server failed to start, retrying...")
            srv_proc.terminate()
            log_file.close()
            shutil.rmtree(data_dir, ignore_errors=True)
            time.sleep(5)
            continue

        print(f"\n[run{run_number}] Server PID={srv_proc.pid} port={port} monitor={mon_port}")

        # ── Sysbench prepare ────────────────────────────────
        prep = subprocess.run([
            "sysbench", "oltp_read_write",
            "--db-driver=mysql",
            f"--mysql-host=127.0.0.1",
            f"--mysql-port={port}",
            "--mysql-user=root",
            f"--table-size={args.table_size}",
            "prepare",
        ], capture_output=True, text=True, timeout=300)
        if prep.returncode != 0:
            print(f"  prepare failed: {prep.stderr[:200]}")
            srv_proc.terminate()
            log_file.close()
            shutil.rmtree(data_dir, ignore_errors=True)
            time.sleep(5)
            continue
        print(f"  Sysbench prepared ({args.table_size} rows)")

        # ── Start sysbench run ─────────────────────────────
        sb_log = open(os.path.join(log_dir, f"run{run_number}_sysbench.out"), "w")
        sb_proc = subprocess.Popen([
            "sysbench", "oltp_read_write",
            "--db-driver=mysql",
            f"--mysql-host=127.0.0.1",
            f"--mysql-port={port}",
            "--mysql-user=root",
            f"--threads={args.sysbench_threads}",
            f"--time={args.duration_min * 60}",
            "--report-interval=30",
            "--percentile=99",
            "run",
        ], stdout=sb_log, stderr=subprocess.STDOUT)

        # ── Monitoring loop ────────────────────────────────
        t_run_start = time.time()
        run_samples = 0
        server_crashed = False

        while True:
            time.sleep(MONITOR_INTERVAL)

            # Check if server still alive
            srv_alive = srv_proc.poll() is None
            sb_alive = sb_proc.poll() is None

            if not srv_alive:
                print(f"  [run{run_number}] SERVER CRASHED (exit={srv_proc.returncode})")
                server_crashed = True
                break

            if not sb_alive:
                # Sysbench finished (or crashed)
                sb_rc = sb_proc.returncode
                print(f"  [run{run_number}] Sysbench exited (rc={sb_rc})")
                if sb_rc != 0:
                    print(f"  Sysbench error, restarting run")
                    server_crashed = True
                break

            # Collect metrics
            elapsed = time.time() - t_run_start
            elapsed_min = (time.time() - t_test_start) / 60
            rss_kb = get_rss(srv_proc.pid)
            wal_mb = wal_size_mb(data_dir)
            stats = http_stats(mon_port)

            total_q = stats.get("queries", {}).get("total", 0)
            avg_dur_ms = stats.get("queries", {}).get("avg_duration_seconds", 0) * 1000
            slow_q = stats.get("queries", {}).get("slow", 0)
            active_conn = stats.get("connections", {}).get("active", 0)
            total_conn = stats.get("connections", {}).get("total", 0)

            # QPS since last sample
            dt = time.time() - last_q_time
            qps_this = (total_q - last_total_q) / dt if dt > 0 else 0
            last_total_q = total_q
            last_q_time = time.time()

            rss_mb = rss_kb / 1024.0
            run_samples += 1

            # Write CSV
            with open(csv_path, "a", newline="") as cf:
                w = csv.writer(cf)
                w.writerow([
                    int(time.time()), f"{elapsed_min:.1f}", f"{wal_mb:.1f}", f"{rss_mb:.1f}",
                    total_q, f"{avg_dur_ms:.1f}", slow_q, f"{qps_this:.1f}",
                    active_conn, total_conn, run_number,
                    int(srv_alive), int(sb_alive),
                ])

            # Safety checks
            disk_pct = disk_usage("/")
            warnings = []
            if wal_mb > args.max_wal_gb * 1024:
                warnings.append(f"WAL={wal_mb:.0f}MB > {args.max_wal_gb}GB")
            if rss_mb > max_rss_mb:
                warnings.append(f"RSS={rss_mb:.0f}MB > {max_rss_mb:.0f}MB")
            if disk_pct > 90:
                warnings.append(f"DISK {disk_pct:.0f}%")

            # Print 10-min report
            warn_str = " ⚠️ " + " | ".join(warnings) if warnings else ""
            print(
                f"[T+{elapsed_min:6.1f}m run{run_number}] "
                f"QPS={qps_this:6.1f}  "
                f"dur={avg_dur_ms:6.1f}ms  "
                f"slow={slow_q:4d}  "
                f"RSS={rss_mb:6.1f}MB  "
                f"WAL={wal_mb:6.1f}MB  "
                f"conn={active_conn}/{total_conn}"
                f"{warn_str}"
            )

            # Check if total elapsed exceeds target
            total_elapsed_check = (time.time() - t_test_start) / 60
            if total_elapsed_check >= args.duration_min:
                print(f"\n=== Target duration reached ===")
                sb_proc.terminate()
                break

        # ── Cleanup after run ──────────────────────────────
        sb_log.close()
        log_file.close()

        # Kill sysbench if still running
        try:
            sb_proc.terminate()
            sb_proc.wait(timeout=5)
        except Exception:
            try:
                sb_proc.kill()
            except Exception:
                pass

        # Kill server
        try:
            srv_proc.terminate()
            srv_proc.wait(timeout=10)
        except Exception:
            try:
                srv_proc.kill()
            except Exception:
                pass

        shutil.rmtree(data_dir, ignore_errors=True)

        if server_crashed:
            print(f"  [run{run_number}] Server crashed after {run_samples} samples. Restarting in 10s...")
            time.sleep(10)
        else:
            # Clean exit (duration reached)
            print(f"\n=== Test complete ({run_number} runs) ===")
            break

    # ── Final summary ────────────────────────────────────
    print(f"\n{'=' * 70}")
    print(f"  72H SOAK COMPLETE — {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"  CSV: {csv_path}")
    print(f"  Logs: {log_dir}")
    print(f"{'=' * 70}")

    # Print CSV tail
    with open(csv_path) as f:
        lines = f.readlines()
        print(f"  {len(lines)-1} data points written")
        if len(lines) > 1:
            print(f"\n  Last 5 rows:")
            for line in lines[-5:]:
                print(f"    {line.strip()}")

if __name__ == "__main__":
    main()
