#!/usr/bin/env python3
"""Generate TPC-H SF=1.0 fixture data for tests/data/tpch-sf1/.

Scale factors:
- SF=1.0 (SF1): ~100 MB total
  region=5, nation=25, supplier=1000, customer=150000, part=20000,
  partsupp=80000, orders=150000, lineitem=600000

Output: pipe-delimited .tbl files matching TPC-H spec format
(terminated by newline, no trailing |, ISO 8601 dates, etc.).

Determinism: a fixed seed ensures the same bytes every run.
"""

import argparse
import random
import sys
from datetime import date, timedelta
from pathlib import Path

# SF=1.0 row counts (standard TPC-H full scale)
ROW_COUNTS = {
    "region": 5,
    "nation": 25,
    "supplier": 1000,
    "customer": 150000,
    "part": 20000,
    "partsupp": 80000,
    "orders": 150000,
    "lineitem": 600000,
}

SEGMENTS = ["BUILDING", "AUTOMOBILE", "MACHINERY", "HOUSEHOLD", "FURNITURE"]
SHIP_MODES = ["TRUCK", "MAIL", "SHIP", "AIR", "REG AIR", "RAIL", "FOB"]
RETURN_FLAGS = ["R", "A"]
LINE_STATUSES = ["O", "F"]
ORDER_STATUSES = ["O", "F", "P"]

NATION_NAMES = [
    "ALGERIA", "ETHIOPIA", "KENYA", "MOROCCO", "MOZAMBIQUE",
    "ARGENTINA", "BRAZIL", "CANADA", "PERU", "UNITED STATES",
    "INDIA", "INDONESIA", "JAPAN", "CHINA", "VIETNAM",
    "FRANCE", "GERMANY", "ROMANIA", "RUSSIA", "UNITED KINGDOM",
    "EGYPT", "IRAN", "IRAQ", "JORDAN", "SAUDI ARABIA",
]

REGION_NAMES = ["AFRICA", "AMERICA", "ASIA", "EUROPE", "MIDDLE EAST"]

PART_TYPES = [
    "STANDARD POLISHED TIN", "SMALL PLATED COPPER", "LARGE BRUSHED BRASS",
    "ECONOMY ANODIZED STEEL", "PROMO BURNISHED COPPER", "STANDARD PLATED TIN",
    "MEDIUM BURNISHED STEEL", "PROMO POLISHED BRASS",
]

PART_SIZES = list(range(1, 51))
PART_CONTAINERS = [
    "SM CASE", "LG BOX", "MED BAG", "MED BOX", "LG CASE", "SM BOX",
    "SM DRUM", "MED PACK", "LG PACK", "SM PACK", "SM JAR", "LG JAR",
]
SHIP_INSTRUCTS = ["DELIVER IN PERSON", "COLLECT COD", "TAKE BACK RETURN", "NONE", "DELLECT IN PERSON"]
ORDER_PRIORITIES = ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"]


def gen_region(rng: random.Random) -> list[str]:
    return [f"{i}|{name}|lar deposits. Special asymptotes" for i, name in enumerate(REGION_NAMES)]


def gen_nation(rng: random.Random) -> list[str]:
    rows = []
    for i, name in enumerate(NATION_NAMES):
        regionkey = i // 5
        rows.append(f"{i}|{name}|{regionkey}|special Tiresias about the furiously regular ideas")
    return rows


def gen_supplier(rng: random.Random, n: int) -> list[str]:
    rows = []
    for i in range(n):
        suppkey = i + 1
        nationkey = rng.randint(0, 24)
        name = f"Supplier#{suppkey:09d}"
        addr = f" Address {rng.randint(0, 99999):05d}"
        phone = f"{rng.randint(10, 99)}-{rng.randint(100, 999)}-{rng.randint(1000, 9999)}"
        acctbal = rng.uniform(-999.99, 9999.99)
        rows.append(f"{suppkey}|{name}|{addr}|{nationkey}|{phone}|{acctbal:.2f}|slyly final deposits cajole")
    return rows


def gen_customer(rng: random.Random, n: int) -> list[str]:
    rows = []
    for i in range(n):
        custkey = i + 1
        nationkey = rng.randint(0, 24)
        name = f"Customer#{custkey:09d}"
        addr = f" Address {rng.randint(0, 99999):05d}"
        phone = f"{rng.randint(10, 99)}-{rng.randint(100, 999)}-{rng.randint(1000, 9999)}"
        acctbal = rng.uniform(-999.99, 9999.99)
        segment = SEGMENTS[i % len(SEGMENTS)]
        rows.append(f"{custkey}|{name}|{addr}|{nationkey}|{phone}|{acctbal:.2f}|{segment}|final accounts wake carefully")
    return rows


def gen_part(rng: random.Random, n: int) -> list[str]:
    rows = []
    for i in range(n):
        partkey = i + 1
        name = f"{rng.choice(['green', 'red', 'blue', 'white', 'black', 'yellow'])} " \
               f"{rng.choice(['almond', 'antique', 'aquamarine', 'beige', 'bisque'])} " \
               f"{rng.choice(['blush', 'burnished', 'cornflower', 'cornsilk'])}"
        mfgr = f"Manufacturer#{rng.randint(1, 5)}"
        brand = f"Brand#{rng.randint(1, 5)}{rng.randint(1, 5)}"
        ptype = rng.choice(PART_TYPES)
        size = rng.choice(PART_SIZES)
        container = rng.choice(PART_CONTAINERS)
        price = rng.uniform(900.00, 2100.00)
        rows.append(f"{partkey}|{name}|{mfgr}|{brand}|{ptype}|{size}|{container}|{price:.2f}|final theodolites along the fluffily ironic")
    return rows


def gen_partsupp(rng: random.Random, n: int, part_count: int, supp_count: int) -> list[str]:
    rows = []
    seen = set()
    i = 0
    while i < n:
        ps_partkey = rng.randint(1, part_count)
        ps_suppkey = rng.randint(1, supp_count)
        key = (ps_partkey, ps_suppkey)
        if key in seen:
            continue
        seen.add(key)
        availqty = rng.randint(1, 9999)
        supplycost = rng.uniform(1.00, 1000.00)
        rows.append(f"{ps_partkey}|{ps_suppkey}|{availqty}|{supplycost:.2f}|fluffily regular requests")
        i += 1
    return rows


def gen_orders(rng: random.Random, n: int, cust_count: int) -> list[str]:
    base_date = date(1992, 1, 1)
    rows = []
    for i in range(n):
        orderkey = i + 1
        custkey = rng.randint(1, cust_count)
        status = rng.choice(ORDER_STATUSES)
        totalprice = rng.uniform(1000.00, 300000.00)
        orderdate = base_date + timedelta(days=rng.randint(0, 2500))
        priority = rng.choice(ORDER_PRIORITIES)
        clerk = f"Clerk#{rng.randint(1, 1000):09d}"
        shippriority = rng.randint(0, 9)
        rows.append(f"{orderkey}|{custkey}|{status}|{totalprice:.2f}|{orderdate}|{priority}|{clerk}|{shippriority}|slyly final requests")
    return rows


def gen_lineitem(rng: random.Random, n: int, order_count: int, part_count: int, supp_count: int) -> list[str]:
    base_date = date(1992, 1, 1)
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
        shipinstruct = rng.choice(SHIP_INSTRUCTS)
        shipmode = rng.choice(SHIP_MODES)
        rows.append(f"{orderkey}|{partkey}|{suppkey}|{linenumber}|{quantity}|{extendedprice:.2f}|{discount:.2f}|{tax:.2f}|{returnflag}|{linestatus}|{shipdate}|{commitdate}|{receiptdate}|{shipinstruct}|{shipmode}|slyly regular instructions")
    return rows


def write_tbl(path: Path, rows: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="") as f:
        for r in rows:
            f.write(r + "\n")
    size = path.stat().st_size
    print(f"  {path.name}: {len(rows)} rows, {size:,} bytes")


def main() -> int:
    ap = argparse.ArgumentParser(description="Generate TPC-H SF=1.0 fixture .tbl data")
    ap.add_argument("--output", required=True, help="Output directory (e.g., tests/data/tpch-sf1)")
    ap.add_argument("--seed", type=int, default=42, help="RNG seed (default: 42 for determinism)")
    args = ap.parse_args()

    out_dir = Path(args.output)
    rng = random.Random(args.seed)

    print(f"=== TPC-H SF=1.0 fixture generator (seed={args.seed}) ===")
    print(f"  output: {out_dir}")
    print()

    write_tbl(out_dir / "region.tbl", gen_region(rng))
    write_tbl(out_dir / "nation.tbl", gen_nation(rng))
    write_tbl(out_dir / "supplier.tbl", gen_supplier(rng, ROW_COUNTS["supplier"]))
    write_tbl(out_dir / "customer.tbl", gen_customer(rng, ROW_COUNTS["customer"]))
    write_tbl(out_dir / "part.tbl", gen_part(rng, ROW_COUNTS["part"]))
    write_tbl(out_dir / "partsupp.tbl", gen_partsupp(rng, ROW_COUNTS["partsupp"], ROW_COUNTS["part"], ROW_COUNTS["supplier"]))
    write_tbl(out_dir / "orders.tbl", gen_orders(rng, ROW_COUNTS["orders"], ROW_COUNTS["customer"]))
    write_tbl(out_dir / "lineitem.tbl", gen_lineitem(rng, ROW_COUNTS["lineitem"], ROW_COUNTS["orders"], ROW_COUNTS["part"], ROW_COUNTS["supplier"]))

    total_size = sum((out_dir / f).stat().st_size for f in [
        "region.tbl", "nation.tbl", "supplier.tbl", "customer.tbl",
        "part.tbl", "partsupp.tbl", "orders.tbl", "lineitem.tbl"
    ])
    print()
    print(f"Total: {total_size:,} bytes ({total_size / 1024 / 1024:.2f} MB)")

    print("Done.")
    return 0


if __name__ == "__main__":
    sys.exit(main())