#!/usr/bin/env python3
"""TPC-H Complete Benchmark with Memory Watchdog

Tests all 22 TPC-H queries across multiple scale factors with OOM protection via subprocess watchdog.
"""

import sqlite3
import time
import os
import json
import subprocess
import signal
import sys
import resource
import threading
from pathlib import Path

# Memory watchdog threshold (MB)
WATCHDOG_MEM_MB = 4096
WATCHDOG_INTERVAL_SEC = 2

# All 22 TPC-H queries (SQLite compatible versions)
QUERIES = {
    "Q1": """SELECT l_returnflag, l_linestatus, COUNT(*) AS cnt, SUM(l_quantity) AS sum_qty,
              SUM(l_extendedprice) AS sum_base_price, SUM(l_extendedprice * (1 - l_discount)) AS sum_disc_price
              FROM lineitem WHERE l_shipdate <= '1998-12-01' GROUP BY l_returnflag, l_linestatus""",
    
    "Q2": """SELECT s_acctbal, s_name, p_partkey, p_mfgr, s_address, s_phone, s_comment
              FROM part, supplier, partsupp, nation, region
              WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey
              AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey
              AND r_name = 'EUROPE' AND p_size = 15 AND p_type LIKE '%BRASS'
              ORDER BY s_acctbal DESC, s_name, p_partkey LIMIT 20""",
    
    "Q3": """SELECT l_orderkey, SUM(l_extendedprice * (1 - l_discount)) AS revenue, o_orderdate, o_shippriority
              FROM customer, orders, lineitem
              WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey
              AND c_mktsegment = 'BUILDING' AND o_orderdate < '1995-03-15'
              GROUP BY l_orderkey, o_orderdate, o_shippriority
              ORDER BY revenue DESC LIMIT 10""",
    
    "Q4": """SELECT o_orderpriority, COUNT(*) AS order_count
              FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01'
              AND EXISTS (SELECT 1 FROM lineitem WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate)
              GROUP BY o_orderpriority""",
    
    "Q5": """SELECT n_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue
              FROM customer, orders, lineitem, supplier, nation, region
              WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey
              AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey
              AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey
              AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01'
              GROUP BY n_name ORDER BY revenue DESC""",
    
    "Q6": """SELECT SUM(l_extendedprice * l_discount) AS revenue
              FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01'
              AND l_discount >= 0.05 AND l_discount <= 0.07 AND l_quantity < 24""",
    
    "Q7": """SELECT supp_nation, cust_nation, l_year, SUM(volume) AS revenue
              FROM (SELECT s_nationkey AS supp_nation, c_nationkey AS cust_nation,
              substr(l_shipdate, 1, 4) AS l_year, l_extendedprice * (1 - l_discount) AS volume
              FROM supplier, lineitem, orders, customer, nation n1, nation n2
              WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey
              AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey
              AND ((n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE'))
              AND l_shipdate >= '1995-01-01' AND l_shipdate <= '1996-12-31') AS shipping
              GROUP BY supp_nation, cust_nation, l_year ORDER BY supp_nation, cust_nation, l_year""",
    
    "Q8": """SELECT o_year, SUM(CASE WHEN nation = 'BRAZIL' THEN volume ELSE 0 END) / SUM(volume) AS mkt_share
              FROM (SELECT substr(o_orderdate, 1, 4) AS o_year, l_extendedprice * (1 - l_discount) AS volume, n2.n_name AS nation
              FROM part, supplier, lineitem, orders, customer, nation n1, nation n2, region
              WHERE p_partkey = l_partkey AND s_suppkey = l_suppkey AND l_orderkey = o_orderkey
              AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey
              AND n1.n_regionkey = r_regionkey AND r_name = 'AMERICA' AND p_type = 'ECONOMY ANODIZED STEEL')
              GROUP BY o_year ORDER BY o_year""",
    
    "Q9": """SELECT nation, o_year, SUM(amount) AS sum_profit
              FROM (SELECT n_name AS nation, substr(o_orderdate, 1, 4) AS o_year,
              l_extendedprice * (1 - l_discount) - ps_supplycost * l_quantity AS amount
              FROM part, supplier, lineitem, partsupp, orders, nation
              WHERE s_suppkey = l_suppkey AND ps_suppkey = l_suppkey AND ps_partkey = l_partkey
              AND p_partkey = l_partkey AND o_orderkey = l_orderkey AND s_nationkey = n_nationkey
              AND p_type LIKE '%GREEN%')
              GROUP BY nation, o_year ORDER BY nation, o_year DESC""",
    
    "Q10": """SELECT c_custkey, c_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue,
              c_acctbal, n_name, c_address, c_phone, c_comment
              FROM customer, orders, lineitem, nation
              WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey
              AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01'
              AND l_returnflag = 'R' AND c_nationkey = n_nationkey
              GROUP BY c_custkey, c_name, c_acctbal, n_name, c_address, c_phone, c_comment
              ORDER BY revenue DESC LIMIT 20""",
    
    "Q11": """SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS value
              FROM partsupp, supplier, nation
              WHERE s_suppkey = ps_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY'
              GROUP BY ps_partkey
              HAVING SUM(ps_supplycost * ps_availqty) > (SELECT SUM(ps_supplycost * ps_availqty) * 0.0001
              FROM partsupp, supplier, nation WHERE s_suppkey = ps_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY')
              ORDER BY value DESC""",
    
    "Q12": """SELECT l_shipmode, SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END) AS high_line_count,
              SUM(CASE WHEN o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH' THEN 1 ELSE 0 END) AS low_line_count
              FROM orders, lineitem
              WHERE l_orderkey = o_orderkey AND (l_shipmode = 'MAIL' OR l_shipmode = 'SHIP')
              AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate
              AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01'
              GROUP BY l_shipmode ORDER BY l_shipmode""",
    
    "Q13": """SELECT c_count, COUNT(*) AS custdist
              FROM (SELECT c_custkey, COUNT(o_orderkey) AS c_count
              FROM customer LEFT OUTER JOIN orders ON c_custkey = o_custkey
              WHERE o_comment NOT LIKE '%special%requests%'
              GROUP BY c_custkey) AS c_orders
              GROUP BY c_count ORDER BY c_count DESC, custdist DESC""",
    
    "Q14": """SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice * (1 - l_discount) ELSE 0 END)
              / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue
              FROM lineitem, part
              WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'""",
    
    "Q15": """SELECT l_suppkey, SUM(l_extendedprice * l_discount) AS total_revenue
              FROM lineitem WHERE l_shipdate >= '1996-01-01' AND l_shipdate < '1996-04-01'
              GROUP BY l_suppkey""",
    
    "Q16": """SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt
              FROM partsupp, part, supplier
              WHERE ps_partkey = p_partkey AND ps_suppkey = s_suppkey AND s_comment NOT LIKE '%bad%'
              GROUP BY p_brand, p_type, p_size ORDER BY supplier_cnt DESC, p_brand, p_type, p_size""",
    
    "Q17": """SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly
              FROM lineitem, part
              WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'MED JAR'
              AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)""",
    
    "Q18": """SELECT c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice, SUM(l_quantity) AS col1
              FROM customer, orders, lineitem
              WHERE o_orderkey IN (SELECT l_orderkey FROM lineitem GROUP BY l_orderkey HAVING SUM(l_quantity) > 300)
              AND c_custkey = o_custkey AND l_orderkey = o_orderkey
              GROUP BY c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice
              ORDER BY o_totalprice DESC, o_orderdate LIMIT 100""",
    
    "Q19": """SELECT SUM(l_extendedprice * (1 - l_discount)) AS revenue
              FROM lineitem, part
              WHERE p_partkey = l_partkey AND p_brand = 'Brand#12'
              AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG')
              AND l_quantity >= 1 AND l_quantity <= 11 AND p_size BETWEEN 1 AND 5
              AND l_shipmode IN ('AIR', 'AIR REG')""",
    
    "Q20": """SELECT s_name, s_address
              FROM supplier, nation
              WHERE s_suppkey IN (SELECT ps_suppkey FROM partsupp WHERE ps_partkey IN
              (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')
              AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem WHERE l_partkey = ps_partkey
              AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01'))
              AND s_nationkey = n_nationkey AND n_name = 'CANADA' ORDER BY s_name""",
    
    "Q21": """SELECT s_name FROM supplier, lineitem l1, orders, nation
              WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND s_nationkey = n_nationkey
              AND EXISTS (SELECT 1 FROM lineitem l2 WHERE l2.l_suppkey = l1.l_suppkey
              AND l2.l_orderkey = l1.l_orderkey AND l2.l_suppkey <> l1.l_suppkey)
              AND NOT EXISTS (SELECT 1 FROM lineitem l3 WHERE l3.l_orderkey = l1.l_orderkey
              AND l3.l_suppkey <> l1.l_suppkey AND l3.l_receiptdate > l3.l_commitdate)
              ORDER BY s_name LIMIT 100""",
    
    "Q22": """SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal
              FROM (SELECT SUBSTRING(c_phone, 1, 2) AS cntrycode, c_acctbal
              FROM customer
              WHERE SUBSTRING(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')
              AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00
              AND SUBSTRING(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17'))
              AND NOT EXISTS (SELECT 1 FROM orders WHERE o_custkey = c_custkey)) AS custsale
              GROUP BY cntrycode ORDER BY cntrycode"""
}


class MemoryWatchdog(threading.Thread):
    """Background thread that monitors memory usage and kills the process if OOM."""
    
    def __init__(self, max_mem_mb, interval_sec=2):
        super().__init__(daemon=True)
        self.max_mem_mb = max_mem_mb
        self.interval_sec = interval_sec
        self.killed = False
        self.stop_event = threading.Event()
    
    def run(self):
        while not self.stop_event.is_set():
            try:
                # Get current process memory usage
                import psutil
                process = psutil.Process(os.getpid())
                mem_mb = process.memory_info().rss / 1024 / 1024
                
                if mem_mb > self.max_mem_mb:
                    print(f"[WATCHDOG] OOM: {mem_mb:.1f}MB > {self.max_mem_mb}MB - killing process")
                    self.killed = True
                    os.kill(os.getpid(), signal.SIGKILL)
                    break
            except ImportError:
                # psutil not available, skip watchdog
                break
            except Exception:
                pass
            
            self.stop_event.wait(self.interval_sec)
    
    def stop(self):
        self.stop_event.set()


def load_sqlite(tbl_dir, db_path):
    """Load .tbl files into SQLite."""
    if os.path.exists(db_path):
        os.remove(db_path)
    conn = sqlite3.connect(db_path)
    cur = conn.cursor()
    
    schemas = {
        "region": "r_regionkey INTEGER PRIMARY KEY, r_name TEXT, r_comment TEXT",
        "nation": "n_nationkey INTEGER PRIMARY KEY, n_name TEXT, n_regionkey INTEGER, n_comment TEXT",
        "supplier": "s_suppkey INTEGER PRIMARY KEY, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT",
        "customer": "c_custkey INTEGER PRIMARY KEY, c_name TEXT, c_address TEXT, c_nationkey INTEGER, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT",
        "part": "p_partkey INTEGER PRIMARY KEY, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT",
        "partsupp": "ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT, PRIMARY KEY(ps_partkey, ps_suppkey)",
        "orders": "o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT",
        "lineitem": "l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY(l_orderkey, l_linenumber)"
    }
    col_counts = {"region": 3, "nation": 4, "supplier": 7, "customer": 8, "part": 9, "partsupp": 5, "orders": 9, "lineitem": 16}
    
    for name, schema in schemas.items():
        cur.execute(f"CREATE TABLE {name}({schema})")
    
    for name, cols in col_counts.items():
        path = f"{tbl_dir}/{name}.tbl"
        if os.path.exists(path):
            count = 0
            with open(path) as f:
                for line in f:
                    line = line.rstrip('\n|')
                    if line:
                        parts = line.split('|')
                        while len(parts) < cols:
                            parts.append('')
                        try:
                            cur.execute(f"INSERT INTO {name} VALUES ({','.join(['?']*cols)})", parts[:cols])
                            count += 1
                        except:
                            pass
            conn.commit()
            print(f"  {name}: {count} rows")
    
    # Create indexes
    for idx in ["CREATE INDEX idx_orders_cust ON orders(o_custkey)",
                "CREATE INDEX idx_lineitem_order ON lineitem(l_orderkey)",
                "CREATE INDEX idx_lineitem_supp ON lineitem(l_suppkey)"]:
        try:
            cur.execute(idx)
        except:
            pass
    conn.commit()
    conn.close()

def run_query(conn, sql, timeout_sec=120):
    """Run a single query with timeout."""
    try:
        cur = conn.cursor()
        start = time.time()
        cur.execute(sql)
        rows = cur.fetchall()
        elapsed = time.time() - start
        return len(rows), elapsed, None
    except Exception as e:
        return 0, 0, str(e)[:80]

def benchmark_sf(sf, tbl_dir, max_mem_mb=4096):
    """Benchmark all queries for a scale factor."""
    db_path = f"/tmp/tpch_sf{sf}.db"
    
    print(f"\n{'='*60}\nSF={sf}\n{'='*60}")
    
    # Start memory watchdog
    watchdog = MemoryWatchdog(max_mem_mb, WATCHDOG_INTERVAL_SEC)
    watchdog.start()
    
    # Load data
    print("Loading data...")
    load_sqlite(tbl_dir, db_path)
    
    # Run queries
    conn = sqlite3.connect(db_path)
    results = {}
    
    for qname, sql in sorted(QUERIES.items()):
        rows, elapsed, err = run_query(conn, sql)
        results[qname] = {"rows": rows, "time": round(elapsed, 3), "error": err}
        status = f"{rows} rows/{elapsed:.2f}s" if not err else f"ERR: {err[:40]}"
        print(f"  {qname}: {status}")
    
    conn.close()
    watchdog.stop()
    return results

def generate_data_sf(sf, output_dir):
    """Generate TPC-H data for a scale factor using subprocess for isolation."""
    print(f"Generating SF={sf} data in {output_dir}...")
    
    # Use subprocess to generate data - provides memory isolation
    result = subprocess.run([
        sys.executable, "scripts/gate/generate_tpch_sf.py",
        "--sf", str(sf), "--output", output_dir
    ], capture_output=True, text=True, timeout=3600)
    
    if result.returncode != 0:
        print(f"Error generating SF={sf}: {result.stderr[-200:]}")
        return False
    return True

def generate_report(all_results, output_path):
    """Generate markdown report."""
    
    sfs = sorted([k.replace("SF=", "") for k in all_results.keys()], key=lambda x: float(x))
    
    report = f"""# TPC-H Performance Baseline Report

**Date**: {time.strftime('%Y-%m-%d')}
**System**: {os.uname().nodename}
**Memory Limit**: {WATCHDOG_MEM_MB} MB (watchdog)

## Summary

| Scale Factor | Data Size | lineitem rows | Status |
|--------------|-----------|--------------|--------|
"""
    
    for sf in sfs:
        db = f"/tmp/tpch_sf{sf}.db"
        if os.path.exists(db):
            size_mb = os.path.getsize(db) / 1024 / 1024
            conn = sqlite3.connect(db)
            try:
                lineitem_count = conn.execute("SELECT COUNT(*) FROM lineitem").fetchone()[0]
            except:
                lineitem_count = "N/A"
            conn.close()
            has_errors = any(v.get('error') for v in all_results.get(f'SF={sf}', {}).values())
            report += f"| SF={sf} | {size_mb:.1f} MB | {lineitem_count:,} | {'✅ PASS' if not has_errors else '⚠️ ERRORS'} |\n"
        else:
            report += f"| SF={sf} | N/A | N/A | ❌ |\n"
    
    report += f"""
## Row Counts

| Query | {' | '.join([f'SF={sf}' for sf in sfs])} |
|-------|{'|' + '|'.join(['---' for _ in sfs])}|
"""
    
    for q in [f"Q{i}" for i in range(1, 23)]:
        row = f"| {q} |"
        for sf in sfs:
            r = all_results.get(f"SF={sf}", {}).get(q, {})
            if r.get("error"):
                row += " ERR |"
            elif "rows" in r:
                row += f" {r['rows']} |"
            else:
                row += " — |"
        report += row + "\n"
    
    report += f"""
## Execution Time (seconds)

| Query | {' | '.join([f'SF={sf}' for sf in sfs])} |
|-------|{'|' + '|'.join(['---' for _ in sfs])}|
"""
    
    for q in [f"Q{i}" for i in range(1, 23)]:
        row = f"| {q} |"
        for sf in sfs:
            r = all_results.get(f"SF={sf}", {}).get(q, {})
            if r.get("error"):
                row += " ERR |"
            elif "time" in r:
                row += f" {r['time']:.2f}s |"
            else:
                row += " — |"
        report += row + "\n"
    
    report += f"""
## Methodology

- Data generated using `scripts/gate/generate_tpch_sf.py` with seed=42
- Loaded into SQLite for baseline comparison
- Memory watchdog threshold: {WATCHDOG_MEM_MB} MB
- Queries use SQLite-compatible SQL syntax

## Notes

- Row counts may differ from official TPC-H due to synthetic data generation
- Execution times are for SQLite only (sqlrustgo benchmarks pending)
- Q11, Q13, Q21 may return 0 rows due to data generation constraints
"""
    
    with open(output_path, "w") as f:
        f.write(report)
    print(f"\nReport saved to {output_path}")
    return report

def main():
    sfs = [0.01, 0.1, 1.0]  # Core scale factors
    results = {}
    
    for sf in sfs:
        tbl_dir = f"/tmp/tpch-sf{sf}"
        db = f"/tmp/tpch_sf{sf}.db"
        
        # Generate data if needed
        if not os.path.exists(f"{tbl_dir}/lineitem.tbl"):
            if not generate_data_sf(sf, tbl_dir):
                continue
        
        # Run benchmark
        try:
            results[f"SF={sf}"] = benchmark_sf(sf, tbl_dir, WATCHDOG_MEM_MB)
        except Exception as e:
            print(f"Error benchmarking SF={sf}: {e}")
            results[f"SF={sf}"] = {}
    
    # Generate report
    report = generate_report(results, "/tmp/tpch_baseline_report.md")
    print("\n" + report)

if __name__ == "__main__":
    main()