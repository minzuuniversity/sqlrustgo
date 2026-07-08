#!/usr/bin/env python3
"""Standalone 72h soak status checker.  Run anytime to get current state.
Does NOT start/stop any processes — only reads state."""

import csv, json, os, re, subprocess, sys, tempfile, time, urllib.request

RESULTS_DIR = "test_results/72h_soak"

def find_server():
    """Find running sqlrustgo-mysql-server and extract ports."""
    try:
        out = subprocess.check_output(["ps", "aux"], text=True, timeout=5)
        for line in out.splitlines():
            if "sqlrustgo-mysql-server" in line and "serve" in line:
                parts = line.split()
                pid = parts[1]
                cpu = parts[2]
                rss = int(parts[5]) if len(parts) > 5 else 0
                cmd = " ".join(parts[10:])
                m_port = re.search(r"--monitor-port[= ](\d+)", cmd)
                m_srv  = re.search(r"--port[= ](\d+)", cmd)
                return {
                    "pid": pid, "port": m_srv.group(1) if m_srv else "?",
                    "mon_port": m_port.group(1) if m_port else "?",
                    "rss_kb": rss, "cpu": cpu
                }
    except Exception:
        return None
    return None

def http_stats(mon_port):
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{mon_port}/stats", timeout=5)
        return json.loads(r.read())
    except Exception:
        return None

def wal_size_mb():
    """Find the current soak72 WAL file and return (size_mb, dirname)."""
    td = tempfile.gettempdir()
    best = (0, "")
    try:
        for name in os.listdir(td):
            if name.startswith("soak72_run"):
                p = os.path.join(td, name, "sqlrustgo.wal")
                if os.path.exists(p):
                    sz = os.path.getsize(p) / (1024 * 1024)
                    # Most recent (highest run number) wins
                    if name > best[1]:
                        best = (sz, name)
    except PermissionError:
        pass
    return best

def latest_csv():
    """Return path + last N rows of most recent CSV."""
    if not os.path.isdir(RESULTS_DIR):
        return None, []
    csvs = [f for f in os.listdir(RESULTS_DIR) if f.startswith("72h_soak_") and f.endswith(".csv")]
    if not csvs:
        return None, []
    latest = max(csvs, key=lambda f: os.path.getmtime(os.path.join(RESULTS_DIR, f)))
    path = os.path.join(RESULTS_DIR, latest)
    with open(path) as f:
        reader = csv.reader(f)
        rows = list(reader)
    return path, rows

def screen_status():
    try:
        out = subprocess.check_output(["/opt/homebrew/bin/screen", "-ls"], text=True, timeout=5)
        if "soak72h" in out:
            return "Alive (Detached)" if "Detached" in out.split("soak72h")[1][:40] else "Alive"
        return "Not found"
    except Exception:
        return "N/A"

def main():
    print("=" * 68)
    print(f"  72H SOAK STATUS  —  {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 68)

    # Screen
    sc = screen_status()
    print(f"\n  Screen soak72h : {sc}")

    # Server
    sv = find_server()
    if sv:
        print(f"  Server          : PID={sv['pid']}  port={sv['port']}  "
              f"monitor={sv['mon_port']}  CPU={sv['cpu']}%  "
              f"RSS={int(sv['rss_kb'])/1024:.1f}MB")
        stats = http_stats(sv['mon_port'])
        if stats:
            q = stats.get("queries", {})
            c = stats.get("connections", {})
            print(f"  ── HTTP /stats ──")
            print(f"  Queries total   : {q.get('total', 0):,}")
            print(f"  Avg duration    : {q.get('avg_duration_seconds', 0)*1000:.1f} ms")
            print(f"  Slow (>1s)      : {q.get('slow', 0)}")
            print(f"  Active conns    : {c.get('active', 0)}")
            print(f"  Total conns     : {c.get('total', 0)}")
            print(f"  Peak conns      : {c.get('peak', 0)}")
        else:
            print(f"  HTTP /stats     : ❌ Unreachable")
    else:
        sv_info = "No server process found"
        print(f"  Server          : ❌ {sv_info}")

    # WAL
    wal, wal_dir = wal_size_mb()
    if wal > 0:
        print(f"  WAL             : {wal:.1f} MB  ({wal_dir})")
    else:
        print(f"  WAL             : not found (no active data dir)")

    # CSV
    csv_path, rows = latest_csv()
    if csv_path and len(rows) > 1:
        n = len(rows) - 1
        print(f"\n  ── CSV data ({n} points) ──")
        print(f"  File: {csv_path}")
        print(f"  {'elapsed':>8} {'WAL':>6} {'RSS':>6} {'Q_total':>8} {'avg_ms':>6} {'slow':>5} {'QPS':>7} {'conn':>4}")
        print(f"  {'─'*8} {'─'*6} {'─'*6} {'─'*8} {'─'*6} {'─'*5} {'─'*7} {'─'*4}")
        # Show last 5 data points
        for row in rows[-5:]:
            if row[0] == "timestamp":
                continue
            print(f"  {row[1]:>8} {row[2]:>6} {row[3]:>6} {row[4]:>8} {row[5]:>6} {row[6]:>5} {row[7]:>7} {row[8]:>4}")
        print(f"\n  Total samples: {n}")
    else:
        print(f"\n  CSV: no data points yet (first sample ~10min after start)")

    # Elapsed time
    if sv:
        try:
            start = os.path.getctime(csv_path) if csv_path else time.time()
            elapsed = (time.time() - start) / 60
            remaining = 4320 - elapsed
            print(f"\n  Elapsed : {elapsed/60:.1f}h / 72h")
            if remaining > 0:
                print(f"  Remaining : {remaining/60:.1f}h")
        except:
            pass

    print()

if __name__ == "__main__":
    main()
