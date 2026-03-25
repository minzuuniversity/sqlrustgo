#!/usr/bin/env python3
"""
Simple TPC-H Benchmark Comparison
SQLRustGo vs SQLite vs PostgreSQL
"""

import subprocess
import time
import sqlite3
import os
import json
import sys

SCALE = 1000  # SF 0.1 equivalent


def create_sqlite_db():
    """Create SQLite database with TPC-H schema"""
    conn = sqlite3.connect("/tmp/tpch_bench.db")
    c = conn.cursor()

    # Lineitem table (simplified)
    c.execute("""
        CREATE TABLE IF NOT EXISTS lineitem (
            l_orderkey INTEGER,
            l_partkey INTEGER,
            l_suppkey INTEGER,
            l_linenumber INTEGER,
            l_quantity REAL,
            l_extendedprice REAL,
            l_discount REAL,
            l_tax REAL,
            l_returnflag TEXT,
            l_linestatus TEXT,
            l_shipdate INTEGER,
            l_commitdate INTEGER,
            l_receiptdate INTEGER,
            l_shipinstruct TEXT,
            l_shipmode TEXT,
            l_comment TEXT
        )
    """)

    # Orders table
    c.execute("""
        CREATE TABLE IF NOT EXISTS orders (
            o_orderkey INTEGER PRIMARY KEY,
            o_custkey INTEGER,
            o_orderstatus TEXT,
            o_totalprice REAL,
            o_orderdate INTEGER,
            o_orderpriority TEXT,
            o_clerk TEXT,
            o_shippriority INTEGER,
            o_comment TEXT
        )
    """)

    # Customer table
    c.execute("""
        CREATE TABLE IF NOT EXISTS customer (
            c_custkey INTEGER PRIMARY KEY,
            c_name TEXT,
            c_address TEXT,
            c_nationkey INTEGER,
            c_phone TEXT,
            c_acctbal REAL,
            c_mktsegment TEXT,
            c_comment TEXT
        )
    """)

    conn.commit()
    return conn


def load_sqlite_data(conn, scale=SCALE):
    """Load sample data into SQLite"""
    c = conn.cursor()

    # Generate lineitem data
    print(f"Loading {scale} rows into SQLite...")
    start = time.time()

    data = []
    for i in range(1, scale + 1):
        data.append(
            (
                i % 10000 + 1,  # l_orderkey
                i % 200000 + 1,  # l_partkey
                i % 100 + 1,  # l_suppkey
                i % 5 + 1,  # l_linenumber
                (i % 50) + 1,  # l_quantity
                (i % 10000) / 100.0,  # l_extendedprice
                (i % 10) / 10.0,  # l_discount
                (i % 8 + 1) / 8.0,  # l_tax
                "N" if i % 3 != 0 else "R",  # l_returnflag
                "O" if i % 2 == 0 else "F",  # l_linestatus
                87600 + (i % 2000),  # l_shipdate
                87600 + (i % 2000),  # l_commitdate
                87600 + (i % 2000),  # l_receiptdate
                "DELIVER IN PERSON",  # l_shipinstruct
                "AIR",  # l_shipmode
                "comment",  # l_comment
            )
        )

        if i % 1000 == 0:
            c.executemany(
                "INSERT INTO lineitem VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", data
            )
            conn.commit()
            data = []

    if data:
        c.executemany(
            "INSERT INTO lineitem VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", data
        )
        conn.commit()

    print(f"SQLite data loaded in {time.time() - start:.2f}s")


def run_sqlite_query(conn, query, name):
    """Run a query and measure time"""
    c = conn.cursor()
    start = time.time()
    c.execute(query)
    _ = c.fetchall()
    elapsed = time.time() - start
    print(f"  {name}: {elapsed * 1000:.2f}ms")
    return elapsed * 1000


def run_sqlrustgo_benchmark():
    """Run SQLRustGo benchmark using the performance tests"""
    print("\n=== SQLRustGo Performance Test ===")

    # We'll use the test output directly
    result = subprocess.run(
        ["cargo", "test", "--test", "performance_test", "--", "--nocapture"],
        capture_output=True,
        text=True,
        cwd="/Users/liying/workspace/dev/heartopen/sqlrustgo",
    )

    # Parse output for timing
    import re

    times = {}
    for line in result.stdout.split("\n"):
        if "took:" in line:
            match = re.search(r"(test \w+).*took: ([\d.]+)ms", line)
            if match:
                times[match.group(1)] = float(match.group(2))

    return times


def main():
    print("=" * 60)
    print("TPC-H Benchmark Comparison: SF 0.1 (1K rows)")
    print("=" * 60)

    results = {"scale": 0.1, "rows": SCALE, "sqlite": {}, "sqlrustgo": {}}

    # SQLite Benchmark
    print("\n--- SQLite Benchmark ---")
    conn = create_sqlite_db()
    load_sqlite_data(conn, SCALE)

    # TPC-H Q1: Pricing Summary Report
    print("\nTPC-H Q1 (Pricing Summary):")
    q1 = "SELECT l_returnflag, SUM(l_quantity) as sum_qty, SUM(l_extendedprice) as sum_base_price FROM lineitem WHERE l_shipdate <= 87600 GROUP BY l_returnflag"
    results["sqlite"]["Q1"] = run_sqlite_query(conn, q1, "Q1")

    # TPC-H Q3: Shipping Priority
    print("\nTPC-H Q3 (Shipping Priority):")
    q3 = "SELECT l_orderkey, SUM(l_extendedprice * (1 - l_discount)) as revenue, o_orderdate, o_shippriority FROM lineitem, orders WHERE l_orderkey = o_orderkey AND l_shipdate > 87600 GROUP BY l_orderkey ORDER BY revenue DESC LIMIT 10"
    results["sqlite"]["Q3"] = run_sqlite_query(conn, q3, "Q3")

    # TPC-H Q6: Forecast Revenue
    print("\nTPC-H Q6 (Forecast Revenue):")
    q6 = "SELECT SUM(l_extendedprice * l_discount) as revenue FROM lineitem WHERE l_quantity < 25 AND l_shipdate >= 87600 AND l_shipdate < 89600"
    results["sqlite"]["Q6"] = run_sqlite_query(conn, q6, "Q6")

    # TPC-H Q10: Returned Item
    print("\nTPC-H Q10 (Returned Item):")
    q10 = "SELECT c_name, c_acctbal, SUM(l_extendedprice * (1 - l_discount)) as revenue, c_address, c_phone FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate >= 87600 AND o_orderdate < 87900 AND l_returnflag = 'R' GROUP BY c_name, c_acctbal, c_address, c_phone ORDER BY revenue DESC LIMIT 20"
    results["sqlite"]["Q10"] = run_sqlite_query(conn, q10, "Q10")

    conn.close()

    # SQLRustGo Benchmark
    print("\n--- SQLRustGo Benchmark ---")
    sqlrustgo_times = run_sqlrustgo_benchmark()
    results["sqlrustgo"] = sqlrustgo_times

    # Summary
    print("\n" + "=" * 60)
    print("BENCHMARK RESULTS SUMMARY")
    print("=" * 60)
    print(f"\nScale Factor: SF {results['scale']}")
    print(f"Data Size: {results['rows']} rows")

    print("\n| Query | SQLite (ms) | SQLRustGo (ms) |")
    print("|-------|--------------|----------------|")

    for q in ["Q1", "Q3", "Q6", "Q10"]:
        sq = results["sqlite"].get(q, "N/A")
        sr = results["sqlrustgo"].get(
            f"test_{'query' if q == 'Q1' else 'query'}_{q.lower()}", "N/A"
        )
        if sq != "N/A":
            sq = f"{sq:.2f}"
        print(f"| {q:5} | {sq:12} | {sr:14} |")

    # Save results
    with open("/tmp/benchmark_results.json", "w") as f:
        json.dump(results, f, indent=2)

    print("\nResults saved to /tmp/benchmark_results.json")


if __name__ == "__main__":
    main()
