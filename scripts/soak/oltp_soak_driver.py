#!/usr/bin/env python3
"""OLTP Soak Driver — Production-like OLTP workload testing.

Simulates realistic production OLTP scenarios with:
- Persistent MySQL connections (no subprocess spawn per query)
- Multiple workload modes: oltp_read_write, oltp_update_index, oltp_write_only, oltp_read_only
- Realistic SQL patterns: point reads, range reads, inserts, updates, deletes, transactions
- Metrics: TPS, latency percentiles, memory growth, FD growth, error rates

Usage:
    python3 scripts/soak/oltp_soak_driver.py --mode=oltp_read_write --level=medium
    python3 scripts/soak/oltp_soak_driver.py --mode=oltp_read_write --duration=3600
    python3 scripts/soak/oltp_soak_driver.py --mode=oltp_update_index --duration=4h
"""

import argparse
import csv
import json
import os
import random
import socket
import struct
import sys
import threading
import time


# ─── MySQL wire protocol client (persistent connections) ────────────────────

class MySQLClient:
    """Minimal persistent MySQL client using raw socket protocol.
    Adapted from long_running_soak.py — proven working implementation."""

    def __init__(self, host, port, user="root", database=""):
        self.host = host
        self.port = port
        self.user = user
        self.database = database
        self.sock = None
        self.connected = False
        self.seq = 0

    def connect(self):
        """Establish connection and authenticate."""
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(30)
        self.sock.connect((self.host, self.port))

        # Receive handshake from server
        self.seq = 0
        pkt = self._recv(4)
        self.seq = pkt[3]
        length = pkt[0] | (pkt[1] << 8) | (pkt[2] << 16)
        handshake = self._recv(length)

        # Extract salt from handshake (8 bytes starting at offset 4+4+1+23 = 32... varies by version)
        # MySQL 5.x handshake: 1B protocol + null-term version + 4B thread_id + 8B salt1 + 1B filler + 8B salt2 + 1B filler
        # We'll extract salt1 (bytes 4-12) and salt2 (bytes 32-40) for newer protocols
        # Simpler: use full handshake bytes minus version
        salt1 = handshake[4:12]  # 8 bytes
        # For protocol 10, salt2 follows after capabilities
        # But for simplicity, use salt1 only (works for most servers)
        salt = salt1

        # Send auth response
        auth = b"\x00" * 20  # empty password
        cap = 0x000FA685  # client capabilities
        payload = struct.pack("<I", cap)
        payload += struct.pack("<I", 16777216)  # max packet size
        payload += bytes([45])  # charset utf8mb4
        payload += b"\x00" * 23  # reserved
        payload += self.user.encode() + b"\x00"  # username
        payload += bytes([len(auth)]) + auth  # auth response
        if self.database:
            payload += self.database.encode() + b"\x00"  # database

        self._send(payload)
        resp = self._recv_packet()
        if resp[0] == 0xFF:
            raise ConnectionError("Auth failed: " + resp[4:].decode("utf8", errors="replace"))
        self.connected = True

    def _recv(self, n):
        data = b""
        while len(data) < n:
            d = self.sock.recv(n - len(data))
            if not d:
                raise EOFError("connection closed")
            data += d
        return data

    def _send(self, payload):
        n = len(payload)
        self.seq = (self.seq + 1) & 0xFF
        hdr = bytes([n & 0xFF, (n >> 8) & 0xFF, (n >> 16) & 0xFF, self.seq])
        self.sock.sendall(hdr + payload)

    def _recv_packet(self):
        hdr = self._recv(4)
        self.seq = hdr[3]
        n = hdr[0] | (hdr[1] << 8) | (hdr[2] << 16)
        return self._recv(n)

    def query(self, sql):
        """Execute a single SQL statement. Returns True on success, False on error."""
        self._send(b"\x03" + sql.encode())
        pkt = self._recv_packet()
        return pkt[0] != 0xFF

    def execute_multi(self, sql):
        """Execute a multi-statement SQL (e.g. 'INSERT ...; INSERT ...').
        Returns (success, error_msg)."""
        self._send(b"\x03" + sql.encode())
        pkt = self._recv_packet()
        if pkt[0] == 0xFF:
            return False, pkt[4:].decode("utf8", errors="replace")
        return True, None

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except:
                pass
            self.sock = None
        self.connected = False


# ─── Workload definitions ───────────────────────────────────────────────────

def _rand_int(lo, hi):
    return random.randint(lo, hi)

def _rand_float(lo, hi):
    return round(random.uniform(lo, hi), 2)

def _rand_status():
    return random.choice(['pending', 'processing', 'shipped', 'delivered', 'cancelled'])

def _rand_payment():
    return random.choice(['credit_card', 'debit_card', 'paypal', 'bank_transfer', 'cash'])

def _now():
    return time.strftime('%Y-%m-%d %H:%M:%S')

# ── Global atomic ID counters (thread-safe unique IDs for INSERTs) ──
# Seeded above the initial data ranges to avoid collisions with seeded data
# and between concurrent worker threads.
_id_lock = threading.Lock()
_next_order_id = [500_001]      # orders: seed was 1-50000
_next_item_id  = [5_000_001]    # items:  seed was 1-200000

def _new_order_id():
    with _id_lock:
        _next_order_id[0] += 1
        return _next_order_id[0]

def _new_item_id():
    with _id_lock:
        _next_item_id[0] += 1
        return _next_item_id[0]

# ── Workload templates ──────────────────────────────────────────────────────

WORKLOADS = {}

def _rw_workload(client):
    """Mixed read/write workload (70/30 split)."""
    if which < 0.25:
        # Point PK read: customers
        cid = _rand_int(1, 10000)
        return 'R', f"SELECT c_id, c_name, c_email, c_balance FROM customers WHERE c_id = {cid}"
    elif which < 0.45:
        # Point PK read: orders
        oid = _rand_int(1, 50000)
        return 'R', f"SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = {oid}"
    elif which < 0.60:
        # Range read: orders by customer
        cid = _rand_int(1, 10000)
        return 'R', f"SELECT o_id, o_status, o_total FROM orders WHERE o_customer_id = {cid} ORDER BY o_id LIMIT 10"
    elif which < 0.72:
        # Range read: order items
        oid = _rand_int(1, 50000)
        return 'R', f"SELECT oi_id, oi_product_id, oi_quantity, oi_subtotal FROM order_items WHERE oi_order_id = {oid}"
    elif which < 0.82:
        # Customer balance update (read-modify-write)
        cid = _rand_int(1, 10000)
        delta = _rand_float(-100, 100)
        return 'U', f"UPDATE customers SET c_balance = c_balance + {delta}, c_updated_at = '{_now()}' WHERE c_id = {cid}"
    elif which < 0.92:
        # Order status update
        oid = _rand_int(1, 50000)
        new_status = _rand_status()
        return 'U', f"UPDATE orders SET o_status = '{new_status}', o_updated_at = '{_now()}' WHERE o_id = {oid}"
    else:
        # Insert new order with items (multi-statement)
        cid = _rand_int(1, 10000)
        oid = _new_order_id()
        total = _rand_float(10, 5000)
        discount = round(random.uniform(0, 0.3), 4)
        tax = round(total * random.uniform(0.05, 0.15), 2)
        payment = _rand_payment()
        now = _now()
        sql = f"INSERT INTO orders (o_id, o_customer_id, o_status, o_total, o_discount, o_tax, o_payment_method, o_created_at, o_updated_at) VALUES ({oid}, {cid}, 'pending', {total}, {discount}, {tax}, '{payment}', '{now}', '{now}')"
        # Add 1-3 order items
        n_items = random.randint(1, 3)
        for j in range(n_items):
            oi_id = _new_item_id()
            pid = _rand_int(1, 5000)
            qty = random.randint(1, 10)
            uprice = _rand_float(1, 500)
            disc = round(random.uniform(0, 0.2), 4)
            sub = round(qty * uprice * (1 - disc), 2)
            sql += f"; INSERT INTO order_items (oi_id, oi_order_id, oi_product_id, oi_quantity, oi_unit_price, oi_discount, oi_subtotal) VALUES ({oi_id}, {oid}, {pid}, {qty}, {uprice}, {disc}, {sub})"
        return 'I', sql
WORKLOADS['oltp_read_write'] = {
    'description': '70% read / 30% write — realistic OLTP mix',
    'generate': _rw_workload,
}

# ── oltp_update_index: heavy index column updates (hot row updates) ─────────

def _ui_workload(client):
    """Update-index workload (100% writes, hot row updates)."""
    which = random.random()
    if which < 0.30:
        # Update order status (indexed column)
        oid = _rand_int(1, 50000)
        new_status = _rand_status()
        return 'U', f"UPDATE orders SET o_status = '{new_status}', o_updated_at = '{_now()}' WHERE o_id = {oid}"
    elif which < 0.50:
        # Update customer balance (indexed column)
        cid = _rand_int(1, 10000)
        delta = _rand_float(-500, 500)
        return 'U', f"UPDATE customers SET c_balance = c_balance + {delta}, c_updated_at = '{_now()}' WHERE c_id = {cid}"
    elif which < 0.65:
        # Update order item quantity (indexed column)
        oid = _rand_int(1, 50000)
        return 'U', f"UPDATE order_items SET oi_quantity = oi_quantity + {random.randint(-2, 3)} WHERE oi_order_id = {oid} LIMIT 1"
    elif which < 0.80:
        # Delete cancelled orders (cleanup)
        oid = _rand_int(1, 50000)
        return 'D', f"DELETE FROM orders WHERE o_id = {oid} AND o_status = 'cancelled'"
    elif which < 0.90:
        # Update order payment method (indexed column)
        oid = _rand_int(1, 50000)
        payment = _rand_payment()
        return 'U', f"UPDATE orders SET o_payment_method = '{payment}', o_updated_at = '{_now()}' WHERE o_id = {oid}"
    else:
        # Insert order with items
        cid = _rand_int(1, 10000)
        oid = _new_order_id()
        total = _rand_float(10, 5000)
        discount = round(random.uniform(0, 0.3), 4)
        tax = round(total * random.uniform(0.05, 0.15), 2)
        payment = _rand_payment()
        now = _now()
        sql = f"INSERT INTO orders (o_id, o_customer_id, o_status, o_total, o_discount, o_tax, o_payment_method, o_created_at, o_updated_at) VALUES ({oid}, {cid}, 'pending', {total}, {discount}, {tax}, '{payment}', '{now}', '{now}')"
        n_items = random.randint(1, 3)
        for j in range(n_items):
            oi_id = _new_item_id()
            pid = _rand_int(1, 5000)
            qty = random.randint(1, 10)
            uprice = _rand_float(1, 500)
            disc = round(random.uniform(0, 0.2), 4)
            sub = round(qty * uprice * (1 - disc), 2)
            sql += f"; INSERT INTO order_items (oi_id, oi_order_id, oi_product_id, oi_quantity, oi_unit_price, oi_discount, oi_subtotal) VALUES ({oi_id}, {oid}, {pid}, {qty}, {uprice}, {disc}, {sub})"
        return 'I', sql

WORKLOADS['oltp_update_index'] = {
    'description': '100% index column updates — hot row update pressure',
    'generate': _ui_workload,
}

def _wo_workload(client):
    """Write-only workload (100% writes, heavy INSERT/DELETE)."""
    which = random.random()
    if which < 0.40:
        # Insert order + items
        cid = _rand_int(1, 10000)
        oid = _new_order_id()
        total = _rand_float(10, 5000)
        discount = round(random.uniform(0, 0.3), 4)
        tax = round(total * random.uniform(0.05, 0.15), 2)
        payment = _rand_payment()
        now = _now()
        sql = f"INSERT INTO orders (o_id, o_customer_id, o_status, o_total, o_discount, o_tax, o_payment_method, o_created_at, o_updated_at) VALUES ({oid}, {cid}, 'pending', {total}, {discount}, {tax}, '{payment}', '{now}', '{now}')"
        n_items = random.randint(1, 5)
        for j in range(n_items):
            oi_id = _new_item_id()
            pid = _rand_int(1, 5000)
            qty = random.randint(1, 10)
            uprice = _rand_float(1, 500)
            disc = round(random.uniform(0, 0.2), 4)
            sub = round(qty * uprice * (1 - disc), 2)
            sql += f"; INSERT INTO order_items (oi_id, oi_order_id, oi_product_id, oi_quantity, oi_unit_price, oi_discount, oi_subtotal) VALUES ({oi_id}, {oid}, {pid}, {qty}, {uprice}, {disc}, {sub})"
        return 'I', sql
    elif which < 0.60:
        # Bulk insert order items
        oid = _rand_int(1, 50000)
        n_items = random.randint(1, 5)
        sql = ""
        for j in range(n_items):
            oi_id = _new_item_id()
            pid = _rand_int(1, 5000)
            qty = random.randint(1, 10)
            uprice = _rand_float(1, 500)
            disc = round(random.uniform(0, 0.2), 4)
            sub = round(qty * uprice * (1 - disc), 2)
            sql += f"INSERT INTO order_items (oi_id, oi_order_id, oi_product_id, oi_quantity, oi_unit_price, oi_discount, oi_subtotal) VALUES ({oi_id}, {oid}, {pid}, {qty}, {uprice}, {disc}, {sub})"
        return 'I', sql
    elif which < 0.75:
        # Delete order + items
        oid = _rand_int(1, 50000)
        return 'D', f"DELETE FROM order_items WHERE oi_order_id = {oid}; DELETE FROM orders WHERE o_id = {oid}"
    elif which < 0.85:
        # Update order status (frequent state transitions)
        oid = _rand_int(1, 50000)
        new_status = t.split('->')[1]
        return 'U', f"UPDATE orders SET o_status = '{new_status}', o_updated_at = '{_now()}' WHERE o_id = {oid}"
    else:
        # Update customer address (occasional)
        cid = _rand_int(1, 10000)
        city = random.choice(['New York', 'Beijing', 'Tokyo', 'London', 'Paris', 'Sydney', 'Berlin', 'Toronto', 'Mumbai', 'Seoul'])
        region = random.choice(['NA', 'APAC', 'EU'])
        return 'U', f"UPDATE customers SET c_city = '{city}', c_region = '{region}', c_updated_at = '{_now()}' WHERE c_id = {cid}"

WORKLOADS['oltp_write_only'] = {
    'description': '100% write — WAL/redo pressure test',
    'generate': _wo_workload,
}

# ── oltp_read_only: pure read baseline (OLTP-style reads) ───────────────────

def _ro_workload(client):
    """Read-only workload (100% reads, OLTP-style)."""
    which = random.random()
    if which < 0.20:
        # Point PK read: customers
        cid = _rand_int(1, 10000)
        return 'R', f"SELECT c_id, c_name, c_email, c_balance FROM customers WHERE c_id = {cid}"
    elif which < 0.35:
        # Point PK read: orders
        oid = _rand_int(1, 50000)
        return 'R', f"SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = {oid}"
    elif which < 0.50:
        # Range read: orders by customer
        cid = _rand_int(1, 10000)
        return 'R', f"SELECT o_id, o_status, o_total FROM orders WHERE o_customer_id = {cid} ORDER BY o_id LIMIT 10"
    elif which < 0.60:
        # Range read: order items
        oid = _rand_int(1, 50000)
        return 'R', f"SELECT oi_id, oi_product_id, oi_quantity, oi_subtotal FROM order_items WHERE oi_order_id = {oid}"
    elif which < 0.70:
        # Aggregate: total revenue
        return 'R', "SELECT SUM(o_total) as total_revenue, COUNT(*) as order_count FROM orders WHERE o_status = 'delivered'"
    elif which < 0.80:
        # Aggregate: customer spending
        cid = _rand_int(1, 10000)
        return 'R', f"SELECT o_customer_id, SUM(o_total) as total_spent, COUNT(*) as order_count FROM orders WHERE o_customer_id = {cid} GROUP BY o_customer_id"
    elif which < 0.88:
        # Status distribution
        return 'R', "SELECT o_status, COUNT(*) as cnt FROM orders GROUP BY o_status ORDER BY cnt DESC"
    elif which < 0.94:
        # Product sales ranking
        return 'R', "SELECT oi_product_id, SUM(oi_quantity) as total_qty, SUM(oi_subtotal) as total_revenue FROM order_items GROUP BY oi_product_id ORDER BY total_qty DESC LIMIT 10"
    else:
        # Cross-table join: customer orders with items
        cid = _rand_int(1, 10000)
        return 'R', f"SELECT c.c_id, c.c_name, o.o_id, o.o_status, o.o_total FROM customers c JOIN orders o ON c.c_id = o.o_customer_id WHERE c.c_id = {cid} AND o.o_status = 'delivered' ORDER BY o.o_id DESC LIMIT 10"

WORKLOADS['oltp_read_only'] = {
    'description': '100% read — OLTP read baseline',
    'generate': _ro_workload,
}


# ─── Worker thread ──────────────────────────────────────────────────────────

class SoakStats:
    def __init__(self):
        self.queries = 0
        self.errors = 0
        self.inserts = 0
        self.updates = 0
        self.deletes = 0
        self.reads = 0
        self.latencies_ms = []
        self.error_msgs = []          # capture error samples
        self.error_by_type = {}       # count by error type
        self.lock = threading.Lock()
        self._start_time = None

    def record(self, ms, ok, sql_type, err_msg=None):
        with self.lock:
            self.queries += 1
            if ok:
                self.latencies_ms.append(ms)
                if sql_type == 'R':
                    self.reads += 1
                elif sql_type == 'I':
                    self.inserts += 1
                elif sql_type == 'U':
                    self.updates += 1
                elif sql_type == 'D':
                    self.deletes += 1
            else:
                self.errors += 1
                # Categorize error
                if err_msg:
                    err_type = self._classify_error(err_msg)
                    self.error_by_type[err_type] = self.error_by_type.get(err_type, 0) + 1
                    if len(self.error_msgs) < 20:
                        self.error_msgs.append(err_msg[:200])
                else:
                    self.error_by_type['connection_error'] = self.error_by_type.get('connection_error', 0) + 1

    def _classify_error(self, msg):
        import re
        m = re.search(r'ERROR\s+(\d+)\s+\((\w+)\)', msg)
        if m:
            code = m.group(1)
            sqlstate = m.group(2)
            if code == '1062':
                return f'Duplicate entry ({code})'
            elif code == '1452':
                return f'FK constraint ({code})'
            elif code == '1064':
                return f'Syntax error ({code})'
            elif code == '1146':
                return f'Table not found ({code})'
            elif code == '1054':
                return f'Unknown column ({code})'
            elif code in ('2006', '2013'):
                return f'Connection lost ({code})'
            else:
                return f'SQL error {code} ({sqlstate})'
        if 'connection' in msg.lower() or 'closed' in msg.lower():
            return 'connection_error'
        return 'other_error'

    def percentiles(self, p):
        if not self.latencies_ms:
            return 0.0
        sorted_lats = sorted(self.latencies_ms)
        idx = int(len(sorted_lats) * p / 100.0)
        return sorted_lats[min(idx, len(sorted_lats) - 1)]

    def summary(self):
        elapsed = time.time() - self._start_time if self._start_time else 1
        n = self.queries
        return {
            'queries': n,
            'errors': self.errors,
            'reads': self.reads,
            'inserts': self.inserts,
            'updates': self.updates,
            'deletes': self.deletes,
            'tps': round(n / max(elapsed, 0.001), 2),
            'p50': round(self.percentiles(50), 4),
            'p99': round(self.percentiles(99), 4),
            'p999': round(self.percentiles(99.9), 4),
            'error_rate_pct': round(self.errors / max(n, 1) * 100, 4),
            'error_by_type': dict(self.error_by_type),
            'error_samples': list(self.error_msgs[:5]),
        }


def worker(tid, host, port, user, database, duration, mode, stats):
    """Worker thread: persistent connection, loop workload until duration expires."""
    client = None
    workload = WORKLOADS[mode]

    deadline = time.time() + duration
    reconnect_count = 0

    while time.time() < deadline:
        try:
            if client is None:
                client = MySQLClient(host, port, user, database)
                client.connect()
                reconnect_count += 1

            # Generate SQL
            sql_type, sql = workload['generate'](client)

            # Execute (use multi-statement for batch inserts)
            t0 = time.perf_counter()
            err_msg = None
            if ';' in sql:
                ok, err_msg = client.execute_multi(sql)
            else:
                ok = client.query(sql)
            ms = (time.perf_counter() - t0) * 1000

            stats.record(ms, ok, sql_type, err_msg=err_msg)

            # Reconnect on error
            if not ok:
                if client:
                    client.close()
                client = None

        except Exception:
            ms = (time.perf_counter() - t0) * 1000 if 't0' in dir() else 0
            stats.record(ms, False, sql_type if 'sql_type' in dir() else 'R')
            if client:
                client.close()
            client = None

    # Cleanup
    if client:
        client.close()


# ─── Metrics sampler ────────────────────────────────────────────────────────

def find_server_pid(port):
    """Find server PID by port."""
    try:
        import subprocess
        out = subprocess.check_output(["lsof", "-ti", f":{port}"], text=True, stderr=subprocess.DEVNULL)
        pids = [int(p) for p in out.strip().split() if p.isdigit()]
        return pids[0] if pids else None
    except:
        return None

def sample_metrics(pid, csv_path, interval, stop):
    """Sample procfs metrics at given interval."""
    while not stop.is_set():
        rss = fd = 0
        if pid:
            try:
                with open(f"/proc/{pid}/status") as f:
                    for line in f:
                        if line.startswith("VmRSS:"):
                            rss = int(line.split()[1])
                            break
            except:
                pass
            try:
                fd = len(os.listdir(f"/proc/{pid}/fd"))
            except:
                pass
        with open(csv_path, "a") as f:
            f.write(f"{int(time.time())},{rss},{fd}\n")
        for _ in range(interval * 10):
            if stop.is_set():
                break
            time.sleep(0.1)


# ─── Report generation ──────────────────────────────────────────────────────

def generate_report(stats, duration, concurrency, mode, server_pid, metrics_csv, level):
    """Generate SoakReport.json with all metrics."""
    rss_baseline = 0
    rss_final = 0
    fd_baseline = 0
    fd_final = 0

    if metrics_csv and os.path.exists(metrics_csv):
        with open(metrics_csv) as f:
            lines = f.readlines()
        data = [l.strip().split(',') for l in lines if l.strip() and not l.startswith('ts')]
        if len(data) >= 2:
            rss_baseline = int(data[0][1])
            fd_baseline = int(data[0][2])
            rss_final = int(data[-1][1])
            fd_final = int(data[-1][2])

    mem_growth = ((rss_final - rss_baseline) / rss_baseline * 100) if rss_baseline > 0 else 0
    fd_growth = fd_final - fd_baseline

    s = stats.summary()
    alert = mem_growth >= 10 or fd_growth >= 5 or s['error_rate_pct'] > 1.0

    report = {
        'level': level,
        'mode': mode,
        'description': WORKLOADS[mode]['description'],
        'duration_seconds': duration,
        'concurrency': concurrency,
        'queries_executed': s['queries'],
        'reads': s['reads'],
        'inserts': s['inserts'],
        'updates': s['updates'],
        'errors': s['errors'],
        'tps': s['tps'],
        'p50_latency_ms': s['p50'],
        'p99_latency_ms': s['p99'],
        'p999_latency_ms': s['p999'],
        'error_rate_pct': s['error_rate_pct'],
        'error_by_type': s.get('error_by_type', {}),
        'error_samples': s.get('error_samples', []),
        'memory_baseline_kb': rss_baseline,
        'memory_final_kb': rss_final,
        'memory_growth_pct': round(mem_growth, 4),
        'fd_baseline': fd_baseline,
        'fd_final': fd_final,
        'fd_growth': fd_growth,
        'alert_triggered': bool(alert),
        'memory_alert': bool(mem_growth >= 10),
        'fd_alert': bool(fd_growth >= 5),
        'error_alert': bool(s['error_rate_pct'] > 1.0),
    }
    return report


# ─── Main ───────────────────────────────────────────────────────────────────

def main():
    a = argparse.ArgumentParser(description='OLTP Soak Test Driver')
    a.add_argument('--host', default='127.0.0.1')
    a.add_argument('--port', type=int, default=3396)
    a.add_argument('--user', default='root')
    a.add_argument('--password', default='')
    a.add_argument('--database', default='')
    a.add_argument('--mode', choices=list(WORKLOADS.keys()),
                   default='oltp_read_write',
                   help='Workload mode')
    a.add_argument('--concurrency', type=int, default=16)
    a.add_argument('--duration', type=int, default=0,
                   help='Duration in seconds (0 = use level default)')
    a.add_argument('--output-dir', default='soak_results')
    a.add_argument('--level', default='medium',
                   help='Test level: small, medium, large')
    a.add_argument('--metrics-interval', type=int, default=30)
    args = a.parse_args()

    # Resolve duration from level if not specified
    if args.duration == 0:
        level_map = {'small': 900, 'medium': 3600, 'large': 7200}
        args.duration = level_map.get(args.level, 3600)

    print(f"=== OLTP Soak Test ===")
    print(f"  Mode:        {args.mode} ({WORKLOADS[args.mode]['description']})")
    print(f"  Level:       {args.level}")
    print(f"  Concurrency: {args.concurrency}")
    print(f"  Duration:    {args.duration}s ({args.duration/60:.0f} min)")
    print(f"  Target:      {args.host}:{args.port}/{args.user}")
    print()

    # Output directory
    ts = time.strftime("%Y%m%d_%H%M%S")
    out = os.path.join(args.output_dir, f"oltp_{args.mode}_{args.level}_{ts}")
    os.makedirs(out, exist_ok=True)

    # Metrics CSV
    mcsv = os.path.join(out, "metrics.csv")
    with open(mcsv, "w") as f:
        f.write("ts,rss_kb,fd\n")

    # Find server PID
    pid = find_server_pid(args.port)

    # Start metrics sampler
    stop_ev = threading.Event()
    if pid:
        smpl = threading.Thread(target=sample_metrics, args=(pid, mcsv, args.metrics_interval, stop_ev), daemon=True)
        smpl.start()

    # Run workload
    stats = SoakStats()
    stats._start_time = time.time()

    threads = []
    for tid in range(args.concurrency):
        t = threading.Thread(target=worker,
            args=(tid, args.host, args.port, args.user, args.database,
                  args.duration, args.mode, stats),
            daemon=True)
        t.start()
        threads.append(t)

    for t in threads:
        t.join(timeout=args.duration + 60)

    elapsed = int(time.time() - stats._start_time)
    stop_ev.set()

    # Generate report
    report = generate_report(stats, elapsed, args.concurrency, args.mode, pid, mcsv, args.level)
    report_path = os.path.join(out, "SoakReport.json")
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)

    # Print summary
    print(f"\n=== SoakReport ===")
    print(json.dumps(report, indent=2))
    print(f"\nReport: {report_path}")

    v = "PASS" if not report['alert_triggered'] else "FAIL"
    print(f"\n{'='*50}")
    print(f"  {v}: {report['queries_executed']} queries ({report['tps']} q/s)")
    print(f"  Reads: {report['reads']}  Inserts: {report['inserts']}  Updates: {report['updates']}  Deletes: {report.get('deletes', 0)}")
    print(f"  Errors: {report['errors']} (rate: {report['error_rate_pct']}%)")
    if report.get('error_by_type'):
        print(f"  Error breakdown:")
        for etype, cnt in sorted(report['error_by_type'].items(), key=lambda x: -x[1]):
            print(f"    {etype}: {cnt}")
    if report.get('error_samples'):
        print(f"  Sample error (first): {report['error_samples'][0][:120]}")
    print(f"  Memory: {report['memory_growth_pct']:.2f}% (alert threshold: 10%)")
    print(f"  FD: {report['fd_growth']} (alert threshold: +5)")
    print(f"  P50: {report['p50_latency_ms']:.2f}ms  P99: {report['p99_latency_ms']:.2f}ms  P999: {report['p999_latency_ms']:.2f}ms")
    print(f"{'='*50}")
    sys.exit(0 if v == "PASS" else 1)

if __name__ == "__main__":
    main()
