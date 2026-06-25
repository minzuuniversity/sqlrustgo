#!/usr/bin/env python3
"""Generate TPC-H SF=0.001 .tbl fixture data for tests/data/tpch-sf001/.

Row counts match scripts/generate_tpch_data.sh::expected_rows_for() for SF=0.001:
  region=5, nation=25, supplier=10, customer=15, part=20,
  partsupp=80, orders=150, lineitem=501

Output: pipe-delimited .tbl files matching TPC-H spec format
(terminated by newline, no trailing |, ISO 8601 dates, etc.).

Used by G1 hash baseline generation (tests/tpch_hashes_v380.json).
This is NOT the official TPC-H dbgen — it produces a minimal, deterministic
fixture suitable for engine correctness verification, not benchmark numbers.

Determinism: a fixed seed ensures the same bytes every run, so the
SHA-256 hash captured by tpch_hash_compare.py --capture is stable.
"""

import argparse
import random
import sys
from datetime import date, timedelta
from pathlib import Path

# SF=0.001 row counts (from scripts/generate_tpch_data.sh)
ROW_COUNTS = {
    "region": 5,
    "nation": 25,
    "supplier": 10,
    "customer": 15,
    "part": 20,
    "partsupp": 80,
    "orders": 150,
    "lineitem": 501,
}

# TPC-H segment names (5 standard MKTSEGMENT values)
SEGMENTS = ["BUILDING", "AUTOMOBILE", "MACHINERY", "HOUSEHOLD", "FURNITURE"]

# TPC-H ship modes (7 standard SHIPMODE values)
SHIP_MODES = ["TRUCK", "MAIL", "SHIP", "AIR", "REG AIR", "RAIL", "FOB"]

# Return flags (2 values)
RETURN_FLAGS = ["R", "A"]
LINE_STATUSES = ["O", "F"]

# Order statuses (3 values, F=finished, O=open, P=pending)
ORDER_STATUSES = ["O", "F", "P"]


def gen_region(rng: random.Random) -> list[str]:
    """5 rows: r_regionkey, r_name, r_comment"""
    names = ["AFRICA", "AMERICA", "ASIA", "EUROPE", "MIDDLE EAST"]
    return [f"{i}|{n}|lar deposits. Special asymptotes" for i, n in enumerate(names)]


def gen_nation(rng: random.Random) -> list[str]:
    """25 rows: n_nationkey, n_name, n_regionkey, n_comment"""
    nation_names = [
        "ALGERIA", "ETHIOPIA", "KENYA", "MOROCCO", "MOZAMBIQUE",  # AFRICA
        "ARGENTINA", "BRAZIL", "CANADA", "PERU", "UNITED STATES",  # AMERICA
        "INDIA", "INDONESIA", "JAPAN", "CHINA", "VIETNAM",  # ASIA
        "FRANCE", "GERMANY", "ROMANIA", "RUSSIA", "UNITED KINGDOM",  # EUROPE
        "EGYPT", "IRAN", "IRAQ", "JORDAN", "SAUDI ARABIA",  # MIDDLE EAST
    ]
    rows = []
    for i, name in enumerate(nation_names):
        regionkey = i // 5
        rows.append(f"{i}|{name}|{regionkey}|special Tiresias about the furiously regular ideas")
    return rows


def gen_supplier(rng: random.Random, n: int) -> list[str]:
    """n rows: s_suppkey, s_name, s_address, s_nationkey, s_phone, s_acctbal, s_comment"""
    rows = []
    for i in range(n):
        suppkey = i + 1
        nationkey = i % 25
        name = f"Supplier#{suppkey:09d}"
        addr = f" Address {rng.randint(0, 99999)}"
        phone = f"{rng.randint(10, 99)}-{rng.randint(100, 999)}-{rng.randint(1000, 9999)}"
        acctbal = rng.uniform(-999.99, 9999.99)
        rows.append(f"{suppkey}|{name}|{addr}|{nationkey}|{phone}|{acctbal:.2f}|slyly final deposits cajole")
    return rows


def gen_customer(rng: random.Random, n: int) -> list[str]:
    """n rows: c_custkey, c_name, c_address, c_nationkey, c_phone, c_acctbal, c_mktsegment, c_comment"""
    rows = []
    for i in range(n):
        custkey = i + 1
        nationkey = i % 25
        name = f"Customer#{custkey:09d}"
        addr = f" Address {rng.randint(0, 99999)}"
        phone = f"{rng.randint(10, 99)}-{rng.randint(100, 999)}-{rng.randint(1000, 9999)}"
        acctbal = rng.uniform(-999.99, 9999.99)
        segment = SEGMENTS[i % len(SEGMENTS)]
        rows.append(f"{custkey}|{name}|{addr}|{nationkey}|{phone}|{acctbal:.2f}|{segment}|final accounts wake carefully")
    return rows


def gen_part(rng: random.Random, n: int) -> list[str]:
    """n rows: p_partkey, p_name, p_mfgr, p_brand, p_type, p_size, p_container, p_retailprice, p_comment"""
    types = ["STANDARD POLISHED TIN", "SMALL PLATED COPPER", "LARGE BRUSHED BRASS",
             "ECONOMY ANODIZED STEEL", "PROMO BURNISHED COPPER", "STANDARD PLATED TIN",
             "MEDIUM BURNISHED STEEL", "PROMO POLISHED BRASS"]
    containers = ["SM CASE", "LG BOX", "MED BAG", "MED BOX", "LG CASE", "SM BOX",
                 "SM DRUM", "MED PACK", "LG PACK", "SM PACK", "SM JAR", "LG JAR"]
    rows = []
    for i in range(n):
        partkey = i + 1
        name = f"{rng.choice(['green', 'red', 'blue', 'white', 'black', 'yellow'])} " \
               f"{rng.choice(['almond', 'antique', 'aquamarine', 'beige', 'bisque'])} " \
               f"{rng.choice(['blush', 'burnished', 'cornflower', 'cornsilk'])}"
        mfgr = f"Manufacturer#{rng.randint(1, 5)}"
        brand = f"Brand#{rng.randint(1, 5)}{rng.randint(1, 5)}"
        ptype = rng.choice(types)
        size = rng.randint(1, 50)
        container = rng.choice(containers)
        price = rng.uniform(900.00, 2100.00)
        rows.append(f"{partkey}|{name}|{mfgr}|{brand}|{ptype}|{size}|{container}|{price:.2f}|final theodolites"
                   f" along the fluffily ironic")
    return rows


def gen_partsupp(rng: random.Random, n: int, part_count: int, supp_count: int) -> list[str]:
    """n rows: ps_partkey, ps_suppkey, ps_availqty, ps_supplycost, ps_comment"""
    rows = []
    i = 0
    while i < n:
        ps_partkey = rng.randint(1, part_count)
        ps_suppkey = rng.randint(1, supp_count)
        if any(r.startswith(f"{ps_partkey}|{ps_suppkey}|") for r in rows):
            continue  # avoid duplicate (partkey, suppkey) pairs
        availqty = rng.randint(1, 9999)
        supplycost = rng.uniform(1.00, 1000.00)
        rows.append(f"{ps_partkey}|{ps_suppkey}|{availqty}|{supplycost:.2f}|fluffily regular requests")
        i += 1
    return rows


def gen_orders(rng: random.Random, n: int, cust_count: int) -> list[str]:
    """n rows: o_orderkey, o_custkey, o_orderstatus, o_totalprice, o_orderdate, o_orderpriority,
    o_clerk, o_shippriority, o_comment"""
    base_date = date(1992, 1, 1)
    rows = []
    for i in range(n):
        orderkey = i + 1
        custkey = rng.randint(1, cust_count)
        status = rng.choice(ORDER_STATUSES)
        totalprice = rng.uniform(1000.00, 300000.00)
        orderdate = base_date + timedelta(days=rng.randint(0, 2500))
        priority = f"{rng.choice(['1-URGENT', '2-HIGH', '3-MEDIUM', '4-NOT SPECIFIED', '5-LOW'])}"
        clerk = f"Clerk#{rng.randint(1, 1000):09d}"
        shippriority = rng.randint(0, 9)
        rows.append(f"{orderkey}|{custkey}|{status}|{totalprice:.2f}|{orderdate}|{priority}|{clerk}|{shippriority}|slyly final requests")
    return rows


def gen_lineitem(rng: random.Random, n: int, order_count: int, part_count: int, supp_count: int) -> list[str]:
    """n rows: l_orderkey, l_partkey, l_suppkey, l_linenumber, l_quantity, l_extendedprice,
    l_discount, l_tax, l_returnflag, l_linestatus, l_shipdate, l_commitdate, l_receiptdate,
    l_shipinstruct, l_shipmode, l_comment"""
    base_date = date(1992, 1, 1)
    ship_instructs = ["DELIVER IN PERSON", "COLLECT COD", "TAKE BACK RETURN", "NONE", "DELIVER IN PERSON"]
    rows = []
    for i in range(n):
        orderkey = rng.randint(1, order_count)
        partkey = rng.randint(1, part_count)
        suppkey = rng.randint(1, supp_count)
        linenumber = rng.randint(1, 7)
        quantity = rng.randint(1, 50)
        extendedprice = quantity * rng.uniform(10.00, 1000.00)
        discount = round(rng.uniform(0.00, 0.10), 2)
        tax = round(rng.uniform(0.00, 0.08), 2)
        returnflag = rng.choice(RETURN_FLAGS)
        linestatus = rng.choice(LINE_STATUSES)
        shipdate = base_date + timedelta(days=rng.randint(0, 2500))
        commitdate = shipdate + timedelta(days=rng.randint(0, 30))
        receiptdate = commitdate + timedelta(days=rng.randint(0, 30))
        shipinstruct = rng.choice(ship_instructs)
        shipmode = rng.choice(SHIP_MODES)
        rows.append(f"{orderkey}|{partkey}|{suppkey}|{linenumber}|{quantity}|{extendedprice:.2f}|{discount}|{tax}|{returnflag}|{linestatus}|{shipdate}|{commitdate}|{receiptdate}|{shipinstruct}|{shipmode}|slyly regular instructions")
    return rows


def write_tbl(path: Path, rows: list[str]) -> None:
    """Write .tbl file (LF terminated, no trailing pipe)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="") as f:
        for r in rows:
            f.write(r + "\n")
    print(f"  {path.name}: {len(rows)} rows")


def main() -> int:
    ap = argparse.ArgumentParser(description="Generate TPC-H SF=0.001 fixture .tbl data")
    ap.add_argument("--output", required=True, help="Output directory (e.g., tests/data/tpch-sf001)")
    ap.add_argument("--seed", type=int, default=42, help="RNG seed (default: 42 for determinism)")
    args = ap.parse_args()

    out_dir = Path(args.output)
    rng = random.Random(args.seed)

    print(f"=== TPC-H SF=0.001 fixture generator (seed={args.seed}) ===")
    print(f"  output: {out_dir}")

    # Generate in dependency order
    write_tbl(out_dir / "region.tbl", gen_region(rng))
    write_tbl(out_dir / "nation.tbl", gen_nation(rng))
    write_tbl(out_dir / "supplier.tbl", gen_supplier(rng, ROW_COUNTS["supplier"]))
    write_tbl(out_dir / "customer.tbl", gen_customer(rng, ROW_COUNTS["customer"]))
    write_tbl(out_dir / "part.tbl", gen_part(rng, ROW_COUNTS["part"]))
    write_tbl(out_dir / "partsupp.tbl", gen_partsupp(rng, ROW_COUNTS["partsupp"],
                                                      ROW_COUNTS["part"], ROW_COUNTS["supplier"]))
    write_tbl(out_dir / "orders.tbl", gen_orders(rng, ROW_COUNTS["orders"], ROW_COUNTS["customer"]))
    write_tbl(out_dir / "lineitem.tbl", gen_lineitem(rng, ROW_COUNTS["lineitem"],
                                                      ROW_COUNTS["orders"], ROW_COUNTS["part"], ROW_COUNTS["supplier"]))

    print("Done.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
