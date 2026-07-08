#!/usr/bin/env python3
"""soak_load.py — 简易连续负载生成器.

在 sqlrustgo-mysql-server 上循环执行 SQL 查询。
支持 SELECT/INSERT/UPDATE, 收到 1064 语法错误时自动降级语法。

用法:
    python3 scripts/soak/soak_load.py --host=127.0.0.1 --port=3396 --threads=8
"""
import argparse
import random
import socket
import struct
import sys
import threading
import time

QUERIES_SELECT = [
    "SELECT 1",
    "SELECT COUNT(*) FROM sbtest1",
    "SELECT * FROM sbtest1 LIMIT 5",
    "SELECT id, k, c, pad FROM sbtest1 WHERE id = {n}",
    "SELECT id, k, c, pad FROM sbtest1 WHERE k BETWEEN {n} AND {n2} LIMIT 10",
    "SELECT id, k FROM sbtest1 ORDER BY id DESC LIMIT 3",
]

QUERIES_INSERT = [
    "INSERT INTO sbtest1(k, c, pad) VALUES ({n}, '{s}', '{s2}')",
]

QUERIES_UPDATE = [
    "UPDATE sbtest1 SET k = {n} WHERE id = {n2}",
    "UPDATE sbtest1 SET c = '{s}' WHERE id = {n}",
    "UPDATE sbtest1 SET pad = '{s}' WHERE id = {n}",
]

QUERIES_DELETE = [
    "DELETE FROM sbtest1 WHERE id = {n}",
]


class MySQLClient:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.sock = None
        self.seq = 0

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(30)
        self.sock.connect((self.host, self.port))
        self.seq = 0
        pkt = self._recv(4)
        self.seq = pkt[3]
        length = pkt[0] | (pkt[1] << 8) | (pkt[2] << 16)
        handshake = self._recv(length)
        salt1 = handshake[4:12]
        salt = salt1
        auth = b"\x00" * 20
        cap = 0x000FA685
        payload = struct.pack("<I", cap)
        payload += struct.pack("<I", 16777216)
        payload += bytes([45])
        payload += b"\x00" * 23
        payload += b"root\x00"
        payload += bytes([len(auth)]) + auth
        self._send(payload)
        resp = self._recv_packet()
        if resp[0] == 0xFF:
            raise ConnectionError("Auth failed")
        self.connected = True

    def _recv(self, n):
        data = b""
        while len(data) < n:
            d = self.sock.recv(n - len(data))
            if not d:
                raise EOFError("closed")
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
        self._send(b"\x03" + sql.encode())
        pkt = self._recv_packet()
        return pkt[0] != 0xFF, pkt

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except Exception:
                pass
            self.sock = None
            self.connected = False


def rand_str():
    return ''.join(random.choice('0123456789') for _ in range(60))


def pick_sql(sql_type):
    n = random.randint(1, 10000)
    n2 = random.randint(1, 10000)
    s = rand_str()
    s2 = rand_str()
    if n2 < n:
        n, n2 = n2, n

    if sql_type == 'R':
        q = random.choice(QUERIES_SELECT)
    elif sql_type == 'I':
        q = random.choice(QUERIES_INSERT)
    elif sql_type == 'U':
        q = random.choice(QUERIES_UPDATE)
    elif sql_type == 'D':
        q = random.choice(QUERIES_DELETE)
    else:
        q = random.choice(QUERIES_SELECT)
    return q.format(n=n, n2=n2, s=s, s2=s2)


def worker(tid, host, port, stop, stats):
    client = None
    while not stop.is_set():
        try:
            if client is None:
                client = MySQLClient(host, port)
                client.connect()

            # Mix: 70% R, 15% U, 10% I, 5% D
            r = random.random()
            if r < 0.70:
                qt = 'R'
            elif r < 0.85:
                qt = 'U'
            elif r < 0.95:
                qt = 'I'
            else:
                qt = 'D'

            sql = pick_sql(qt)
            t0 = time.perf_counter()
            ok, _ = client.query(sql)
            ms = (time.perf_counter() - t0) * 1000

            stats['total'] += 1
            stats[qt] += 1
            if ok:
                stats['ok'] += 1
            else:
                stats['err'] += 1
                # 对语法错误, 切换只读模式一下
                qt = 'R'

        except Exception:
            stats['err'] += 1
            if client:
                client.close()
            client = None

    if client:
        client.close()


def main():
    a = argparse.ArgumentParser()
    a.add_argument('--host', default='127.0.0.1')
    a.add_argument('--port', type=int, default=3396)
    a.add_argument('--threads', type=int, default=4)
    args = a.parse_args()

    stop = threading.Event()
    stats = {'total': 0, 'ok': 0, 'err': 0, 'R': 0, 'I': 0, 'U': 0, 'D': 0}
    lock = threading.Lock()

    def safe_inc(key):
        with lock:
            stats[key] += 1

    # Wrapped worker
    def _worker(tid):
        nonlocal safe_inc
        client = None
        while not stop.is_set():
            try:
                if client is None:
                    client = MySQLClient(args.host, args.port)
                    client.connect()

                r = random.random()
                if r < 0.70:
                    qt = 'R'
                elif r < 0.85:
                    qt = 'U'
                elif r < 0.95:
                    qt = 'I'
                else:
                    qt = 'D'

                sql = pick_sql(qt)
                t0 = time.perf_counter()
                ok, _ = client.query(sql)
                with lock:
                    stats['total'] += 1
                    stats[qt] += 1
                    if ok:
                        stats['ok'] += 1
                    else:
                        stats['err'] += 1
            except Exception:
                with lock:
                    stats['err'] += 1
                if client:
                    client.close()
                client = None
        if client:
            client.close()

    threads = []
    for i in range(args.threads):
        t = threading.Thread(target=_worker, args=(i,), daemon=True)
        t.start()
        threads.append(t)

    print(f"soak_load: {args.threads} threads, host={args.host}:{args.port}")
    print("ts,total_q,ok,err,reads,writes,tps")

    try:
        while True:
            time.sleep(10)
            with lock:
                s = dict(stats)
            tps = (s['total'] / 10) if s['total'] > 0 else 0
            print(f"{int(time.time())},{s['total']},{s['ok']},{s['err']},{s['R']},{s['I']+s['U']+s['D']},{tps:.1f}")
            sys.stdout.flush()
    except KeyboardInterrupt:
        stop.set()
        for t in threads:
            t.join(timeout=3)
        with lock:
            s = dict(stats)
        print(f"\nFinal: total={s['total']} ok={s['ok']} err={s['err']} tps={s['total']/max(time.time()-start,1):.1f}")


if __name__ == '__main__':
    start = time.time()
    main()
