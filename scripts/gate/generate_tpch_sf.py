#!/usr/bin/env python3
"""Generate TPC-H fixture data for arbitrary scale factors."""

import argparse
import random
from datetime import date, timedelta
from pathlib import Path

SF1_ROW_COUNTS = {
    "region": 5,
    "nation": 25,
    "supplier": 10000,
    "customer": 150000,
    "part": 200000,
    "partsupp": 800000,
    "orders": 1500000,
    "lineitem": 6000000,
}

SEGMENTS = ["BUILDING", "AUTOMOBILE", "MACHINERY", "HOUSEHOLD", "FURNITURE"]
SHIP_MODES = ["TRUCK", "MAIL", "SHIP", "AIR", "REG AIR", "RAIL", "FOB"]
RETURN_FLAGS = ["R", "A"]
LINE_STATUSES = ["O", "F"]
ORDER_STATUSES = ["O", "F", "P"]
NATION_NAMES = ["ALGERIA", "ETHIOPIA", "KENYA", "MOROCCO", "MOZAMBIQUE", "ARGENTINA", "BRAZIL", "CANADA", "PERU", "UNITED STATES", "INDIA", "INDONESIA", "JAPAN", "CHINA", "VIETNAM", "FRANCE", "GERMANY", "ROMANIA", "RUSSIA", "UNITED KINGDOM", "EGYPT", "IRAN", "IRAQ", "JORDAN", "SAUDI ARABIA"]
REGION_NAMES = ["AFRICA", "AMERICA", "ASIA", "EUROPE", "MIDDLE EAST"]
PART_TYPES = ["STANDARD POLISHED TIN", "SMALL PLATED COPPER", "LARGE BRUSHED BRASS", "ECONOMY ANODIZED STEEL", "PROMO BURNISHED COPPER", "STANDARD PLATED TIN", "MEDIUM BURNISHED STEEL", "PROMO POLISHED BRASS"]
PART_SIZES = list(range(1, 51))
PART_CONTAINERS = ["SM CASE", "LG BOX", "MED BAG", "MED BOX", "LG CASE", "SM BOX", "SM DRUM", "MED PACK", "LG PACK", "SM PACK", "SM JAR", "LG JAR"]
SHIP_INSTRUCTS = ["DELIVER IN PERSON", "COLLECT COD", "TAKE BACK RETURN", "NONE", "BILL OF LADING"]

def scale_row_count(table, sf):
    if table in ("region", "nation"):
        return SF1_ROW_COUNTS[table]
    return int(SF1_ROW_COUNTS[table] * sf)

def gen_date():
    start = date(1992, 1, 1)
    end = date(1998, 12, 31)
    delta = (end - start).days
    return str(start + timedelta(days=random.randint(0, delta)))

def generate_sf(sf, output_dir, seed=42):
    random.seed(seed)
    counts = {t: scale_row_count(t, sf) for t in SF1_ROW_COUNTS}
    
    print(f"=== TPC-H SF={sf} (seed={seed}) ===")
    out = Path(output_dir)
    out.mkdir(parents=True, exist_ok=True)
    total_bytes = 0
    
    # region
    with open(out / "region.tbl", "w") as f:
        for i in range(5):
            f.write(f"{i+1}|{REGION_NAMES[i]}|Region {i+1} comment|\n")
    total_bytes += (out / "region.tbl").stat().st_size
    print(f"  region.tbl: 5 rows")
    
    # nation
    with open(out / "nation.tbl", "w") as f:
        for i in range(25):
            f.write(f"{i+1}|{NATION_NAMES[i]}|{(i % 5) + 1}|Nation {i+1} comment|\n")
    total_bytes += (out / "nation.tbl").stat().st_size
    print(f"  nation.tbl: 25 rows")
    
    n_supp = counts["supplier"]
    n_part = counts["part"]
    n_cust = counts["customer"]
    n_ps = counts["partsupp"]
    n_ord = counts["orders"]
    n_line = counts["lineitem"]
    
    # supplier
    with open(out / "supplier.tbl", "w") as f:
        for i in range(n_supp):
            f.write(f"{i+1}|Supplier {i+1}|Address {i+1}|{(i % 25) + 1}|+1-{i:010d}|{random.uniform(-1000, 10000):.2f}|Supplier comment {i+1}|\n")
    total_bytes += (out / "supplier.tbl").stat().st_size
    print(f"  supplier.tbl: {n_supp:,} rows")
    
    # customer
    with open(out / "customer.tbl", "w") as f:
        for i in range(n_cust):
            f.write(f"{i+1}|Customer {i+1}|Address {i+1}|{(i % 25) + 1}|+1-{i:010d}|{random.uniform(-500, 10000):.2f}|{random.choice(SEGMENTS)}|Cust comment {i+1}|\n")
    total_bytes += (out / "customer.tbl").stat().st_size
    print(f"  customer.tbl: {n_cust:,} rows")
    
    # part
    with open(out / "part.tbl", "w") as f:
        for i in range(n_part):
            f.write(f"{i+1}|Part {i+1}|Mfr {i+1 % 5 + 1}|Brand#{i % 5 + 1}23|{random.choice(PART_TYPES)}|{random.choice(PART_SIZES)}|{random.choice(PART_CONTAINERS)}|{random.uniform(100, 1000):.2f}|Part comment {i+1}|\n")
    total_bytes += (out / "part.tbl").stat().st_size
    print(f"  part.tbl: {n_part:,} rows")
    
    # partsupp
    with open(out / "partsupp.tbl", "w") as f:
        for i in range(n_ps):
            pkey = (i % n_part) + 1
            skey = ((i // n_part) % n_supp) + 1
            f.write(f"{pkey}|{skey}|{random.randint(1, 9999)}|{random.uniform(1, 1000):.2f}|PS comment|\n")
    total_bytes += (out / "partsupp.tbl").stat().st_size
    print(f"  partsupp.tbl: {n_ps:,} rows")
    
    # orders
    with open(out / "orders.tbl", "w") as f:
        for i in range(n_ord):
            custkey = (i % min(n_cust, 150000)) + 1
            f.write(f"{i+1}|{custkey}|{random.choice(ORDER_STATUSES)}|{random.uniform(100, 50000):.2f}|{gen_date()}|O-PRIO-{i % 5 + 1}|Clerk#{i % 1000}|0|Ord comment|\n")
    total_bytes += (out / "orders.tbl").stat().st_size
    print(f"  orders.tbl: {n_ord:,} rows")
    
    # lineitem
    with open(out / "lineitem.tbl", "w") as f:
        for o in range(n_ord):
            for l in range(4):  # 4 lineitems per order
                qty = random.randint(1, 50)
                price = random.uniform(100, 10000)
                disc = random.uniform(0, 0.1)
                tax = random.uniform(0, 0.1)
                f.write(f"{o+1}|{(o % n_part) + 1}|{(o * 7 + l) % n_supp + 1}|{l+1}|{qty}|{price:.2f}|{disc:.4f}|{tax:.4f}|{random.choice(RETURN_FLAGS)}|{random.choice(LINE_STATUSES)}|{gen_date()}|{gen_date()}|{gen_date()}|{random.choice(SHIP_INSTRUCTS)}|{random.choice(SHIP_MODES)}|Line comment|\n")
    total_bytes += (out / "lineitem.tbl").stat().st_size
    print(f"  lineitem.tbl: {n_ord * 4:,} rows")
    
    print(f"Total: {total_bytes:,} bytes ({total_bytes/1024/1024:.2f} MB)")
    print("Done.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sf", type=float, required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()
    generate_sf(args.sf, args.output, args.seed)