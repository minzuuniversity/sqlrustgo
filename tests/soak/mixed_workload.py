#!/usr/bin/env python3
# =============================================================================
# tests/soak/mixed_workload.py — v3.12.0 GA Mixed-Workload Python Driver (Issue #4387 / GA-2)
# =============================================================================
# Drives 5-class mixed workload against sqlrustgo-server:
#   W1 OLTP transactional (INSERT/UPDATE/DELETE + simple SELECT)         30%
#   W2 Read-heavy (point lookups + range scans)                            25%
#   W3 Aggregation (GROUP BY, COUNT, SUM, AVG)                           15%
#   W4 DDL/schemalight (CREATE/ALTER/DROP, index creation)                10%
#   W5 Long-running reports (heavy JOINs, subqueries, window funcs)       20%
#
# Usage:
#   python3 tests/soak/mixed_workload.py \\
#       --host 127.0.0.1 --port 3306 \\
#       --user root --database soak \\
#       --duration 3600 --ops-per-min 600 \\
#       --config tests/soak/mixed_workload_config.yaml \\
#       --output docs/releases/v3.12.0/evidence/v312-59/soak/mixed_workload_run.json
#
# Companion to tests/soak/v312_mixed_soak.rs (Rust harness).
# =============================================================================

import argparse
import json
import os
import random
import signal
import sys
import threading
import time
import yaml
from dataclasses import dataclass, field, asdict
from typing import List, Dict, Optional

try:
    import pymysql
except ImportError:
    print("[ERROR] pymysql required: pip3 install pymysql", file=sys.stderr)
    sys.exit(2)


# Workload classes and their fraction of total ops
WORKLOAD_CLASSES = ["W1", "W2", "W3", "W4", "W5"]
WORKLOAD_FRACTION = {"W1": 30, "W2": 25, "W3": 15, "W4": 10, "W5": 20}


@dataclass
class ClassMetrics:
    workload_class: str
    ops_attempted: int = 0
    ops_succeeded: int = 0
    ops_failed: int = 0
    total_latency_ms: int = 0
    latency_samples: List[int] = field(default_factory=list)
    errors: List[str] = field(default_factory=list)

    def avg_latency_ms(self) -> float:
        if not self.latency_samples:
            return 0.0
        return self.total_latency_ms / len(self.latency_samples)

    def p99_latency_ms(self) -> int:
        if not self.latency_samples:
            return 0
        sorted_samples = sorted(self.latency_samples)
        idx = min(int(len(sorted_samples) * 0.99), len(sorted_samples) - 1)
        return sorted_samples[idx]


@dataclass
class MixedWorkloadConfig:
    host: str = "127.0.0.1"
    port: int = 3306
    user: str = "root"
    password: str = ""
    database: str = "soak"
    duration_secs: int = 3600
    ops_per_min: int = 600
    threads_per_class: int = 4
    report_interval_secs: int = 30
    chaos_inject_prob: float = 0.0
    output_path: str = "docs/releases/v3.12.0/evidence/v312-59/soak/mixed_workload_run.json"

    @classmethod
    def from_yaml(cls, path: str) -> "MixedWorkloadConfig":
        with open(path) as f:
            data = yaml.safe_load(f)
        # Filter to only known dataclass fields (drop narrative sections like workload_classes)
        valid_fields = {f.name for f in cls.__dataclass_fields__.values()}
        filtered = {k: v for k, v in (data or {}).items() if k in valid_fields}
        return cls(**filtered)

    @classmethod
    def from_args(cls, args: argparse.Namespace) -> "MixedWorkloadConfig":
        return cls(
            host=args.host,
            port=args.port,
            user=args.user,
            password=args.password or "",
            database=args.database,
            duration_secs=args.duration,
            ops_per_min=args.ops_per_min,
            output_path=args.output,
        )


class WorkloadGenerator:
    """Generates SQL statements for one workload class."""

    def __init__(self, workload_class: str, rng: random.Random):
        self.workload_class = workload_class
        self.rng = rng

    def next_query(self) -> str:
        if self.workload_class == "W1":
            return self._oltp_query()
        elif self.workload_class == "W2":
            return self._read_heavy_query()
        elif self.workload_class == "W3":
            return self._aggregate_query()
        elif self.workload_class == "W4":
            return self._ddl_query()
        elif self.workload_class == "W5":
            return self._report_query()
        else:
            return "SELECT 1"

    def _oltp_query(self) -> str:
        # mix of INSERT / UPDATE / DELETE / SELECT
        op = self.rng.choice(["insert", "update", "delete", "select"])
        n = self.rng.randint(1, 10000)
        if op == "insert":
            return f"INSERT INTO orders (customer_id, total) VALUES ({n}, {n * 10})"
        elif op == "update":
            return f"UPDATE orders SET status='paid' WHERE id={n}"
        elif op == "delete":
            return f"DELETE FROM orders WHERE id={n}"
        else:
            return f"SELECT * FROM orders WHERE id={n}"

    def _read_heavy_query(self) -> str:
        n = self.rng.randint(1, 10000)
        kind = self.rng.choice(["point", "range"])
        if kind == "point":
            return f"SELECT * FROM customers WHERE id={n}"
        else:
            lo = n
            hi = n + 100
            return f"SELECT * FROM orders WHERE id BETWEEN {lo} AND {hi}"

    def _aggregate_query(self) -> str:
        return (
            "SELECT customer_id, COUNT(*) cnt, SUM(total) total_amt, AVG(total) avg_amt "
            "FROM orders GROUP BY customer_id ORDER BY cnt DESC LIMIT 100"
        )

    def _ddl_query(self) -> str:
        kind = self.rng.choice(["create_idx", "drop_idx", "alter"])
        idx = self.rng.randint(1, 1000)
        if kind == "create_idx":
            return f"CREATE INDEX idx_soak_{idx} ON orders (customer_id)"
        elif kind == "drop_idx":
            return f"DROP INDEX idx_soak_{max(1, idx - 5)} ON orders"
        else:
            return f"ALTER TABLE orders ADD COLUMN col_{idx} INT DEFAULT 0"

    def _report_query(self) -> str:
        return (
            "SELECT c.region, o.status, COUNT(*) cnt, SUM(o.total) amt "
            "FROM orders o JOIN customers c ON o.customer_id = c.id "
            "WHERE o.created_at > DATE_SUB(NOW(), INTERVAL 30 DAY) "
            "GROUP BY c.region, o.status "
            "ORDER BY amt DESC LIMIT 50"
        )


class WorkloadThread(threading.Thread):
    """One worker thread driving a single workload class."""

    def __init__(self, cls_name: str, cfg: MixedWorkloadConfig, metrics: Dict[str, ClassMetrics],
                 stop_event: threading.Event, lock: threading.Lock):
        super().__init__(daemon=True)
        self.cls_name = cls_name
        self.cfg = cfg
        self.metrics = metrics
        self.stop_event = stop_event
        self.lock = lock
        self.rng = random.Random(os.getpid() ^ hash(cls_name) ^ time.time_ns())
        self.gen = WorkloadGenerator(cls_name, self.rng)
        self.connection = None
        self._connect_with_retry()

    def _connect_with_retry(self, max_retries: int = 5) -> bool:
        for i in range(max_retries):
            try:
                self.connection = pymysql.connect(
                    host=self.cfg.host,
                    port=self.cfg.port,
                    user=self.cfg.user,
                    password=self.cfg.password,
                    database=self.cfg.database,
                    autocommit=True,
                    connect_timeout=5,
                )
                return True
            except Exception as e:
                if i == max_retries - 1:
                    self._record_error(f"connect failed: {e}")
                    return False
                time.sleep(1.0)
        return False

    def _record_error(self, msg: str) -> None:
        with self.lock:
            m = self.metrics[self.cls_name]
            m.errors.append(msg)
            if len(m.errors) > 20:
                m.errors = m.errors[-20:]

    def run(self) -> None:
        # ops per second for this class
        ops_per_sec = max(1, (self.cfg.ops_per_min * WORKLOAD_FRACTION[self.cls_name] // 100) // 60)
        sleep_between_us = max(1, 1_000_000 // ops_per_sec)

        while not self.stop_event.is_set():
            start = time.monotonic()
            try:
                if self.connection is None:
                    if not self._connect_with_retry(max_retries=2):
                        time.sleep(1.0)
                        continue
                with self.connection.cursor() as cur:
                    cur.execute(self.gen.next_query())
                    cur.fetchall()
                latency_ms = int((time.monotonic() - start) * 1000)
                with self.lock:
                    m = self.metrics[self.cls_name]
                    m.ops_attempted += 1
                    m.ops_succeeded += 1
                    m.total_latency_ms += latency_ms
                    m.latency_samples.append(latency_ms)
                    if len(m.latency_samples) > 10000:
                        m.latency_samples = m.latency_samples[-10000:]
            except Exception as e:
                latency_ms = int((time.monotonic() - start) * 1000)
                self._record_error(f"query failed: {e}")
                with self.lock:
                    m = self.metrics[self.cls_name]
                    m.ops_attempted += 1
                    m.ops_failed += 1
                    if self.connection:
                        try:
                            self.connection.close()
                        except Exception:
                            pass
                    self.connection = None

            # Sleep to maintain ops/sec
            elapsed_us = int((time.monotonic() - start) * 1_000_000)
            sleep_us = max(0, sleep_between_us - elapsed_us)
            if sleep_us > 0:
                self.stop_event.wait(timeout=sleep_us / 1_000_000)


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="v3.12.0 GA Mixed-Workload SOAK Driver")
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=3306)
    p.add_argument("--user", default="root")
    p.add_argument("--password", default="")
    p.add_argument("--database", default="soak")
    p.add_argument("--duration", type=int, default=3600,
                   help="Duration in seconds (default 3600 = 1h demo)")
    p.add_argument("--ops-per-min", type=int, default=600)
    p.add_argument("--config", default="tests/soak/mixed_workload_config.yaml",
                   help="Optional YAML config file (overrides --host etc.)")
    p.add_argument("--output", default="docs/releases/v3.12.0/evidence/v312-59/soak/mixed_workload_run.json")
    p.add_argument("--dry-run", action="store_true", help="Validate config + exit without running")
    return p.parse_args()


def main() -> int:
    args = parse_args()

    if args.config and os.path.isfile(args.config):
        print(f"[INFO] loading config from {args.config}")
        cfg = MixedWorkloadConfig.from_yaml(args.config)
        # CLI args override YAML
        if args.host != "127.0.0.1":
            cfg.host = args.host
        if args.port != 3306:
            cfg.port = args.port
        if args.duration != 3600:
            cfg.duration_secs = args.duration
    else:
        cfg = MixedWorkloadConfig.from_args(args)

    print(f"[INFO] v3.12.0 GA Mixed-Workload SOAK Driver (Issue #4387 / GA-2)")
    print(f"[INFO] host={cfg.host}:{cfg.port} db={cfg.database}")
    print(f"[INFO] duration={cfg.duration_secs}s ops_per_min={cfg.ops_per_min}")
    print(f"[INFO] output={cfg.output_path}")

    if args.dry_run:
        print("[DRY-RUN] OK — config valid")
        return 0

    # Init metrics
    metrics = {c: ClassMetrics(workload_class=c) for c in WORKLOAD_CLASSES}
    lock = threading.Lock()
    stop_event = threading.Event()

    # Spawn one thread per workload class
    threads = []
    for cls_name in WORKLOAD_CLASSES:
        t = WorkloadThread(cls_name, cfg, metrics, stop_event, lock)
        t.start()
        threads.append(t)

    # SIGINT handler
    def _sigint(_signum, _frame):
        print("\n[INFO] SIGINT received, stopping...")
        stop_event.set()
    signal.signal(signal.SIGINT, _sigint)

    # Run for duration
    start_time = time.time()
    deadline = start_time + cfg.duration_secs
    next_report = start_time + cfg.report_interval_secs

    while time.time() < deadline and not stop_event.is_set():
        time.sleep(min(5.0, deadline - time.time()))
        if time.time() >= next_report:
            elapsed = int(time.time() - start_time)
            with lock:
                tot_att = sum(m.ops_attempted for m in metrics.values())
                tot_ok = sum(m.ops_succeeded for m in metrics.values())
                tot_fail = sum(m.ops_failed for m in metrics.values())
            print(f"[{elapsed}s] attempted={tot_att} ok={tot_ok} fail={tot_fail}")
            next_report = time.time() + cfg.report_interval_secs

    stop_event.set()
    for t in threads:
        t.join(timeout=5.0)

    # Write final report
    os.makedirs(os.path.dirname(cfg.output_path), exist_ok=True)
    report = {
        "v312_ga_mixed_workload_run": {
            "config": asdict(cfg),
            "started_at_epoch": int(start_time),
            "duration_secs": int(time.time() - start_time),
            "per_class": {
                cls: {
                    "ops_attempted": m.ops_attempted,
                    "ops_succeeded": m.ops_succeeded,
                    "ops_failed": m.ops_failed,
                    "avg_latency_ms": round(m.avg_latency_ms(), 2),
                    "p99_latency_ms": m.p99_latency_ms(),
                    "error_count": len(m.errors),
                    "sample_errors": m.errors[:5],
                }
                for cls, m in metrics.items()
            },
            "totals": {
                "ops_attempted": sum(m.ops_attempted for m in metrics.values()),
                "ops_succeeded": sum(m.ops_succeeded for m in metrics.values()),
                "ops_failed": sum(m.ops_failed for m in metrics.values()),
            },
        }
    }
    with open(cfg.output_path, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\n[INFO] Report written: {cfg.output_path}")
    tot_att = report["v312_ga_mixed_workload_run"]["totals"]["ops_attempted"]
    tot_fail = report["v312_ga_mixed_workload_run"]["totals"]["ops_failed"]
    if tot_att > 0:
        fail_rate = tot_fail / tot_att
        print(f"[INFO] Total: {tot_att} attempts, {tot_fail} fails, failure_rate={fail_rate:.4f}")
        if fail_rate >= 0.01:
            print(f"[FAIL] failure rate {fail_rate:.4f} >= 0.01 (1% threshold)")
            return 1
    print("[PASS] v3.12.0 GA Mixed-Workload SOAK run complete")
    return 0


if __name__ == "__main__":
    sys.exit(main())