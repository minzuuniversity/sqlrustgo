#!/usr/bin/env python3
"""Generate TPC-H fixture data for arbitrary scale factors."""

import argparse
import random
import re
import sys
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
# The seven ship modes of the TPC-H `lineitem.shipmode` domain. The previous
# list carried "TRUCK" (not a TPC-H mode) and spelled "REG AIR" with a space
# (the spec value is REG_AIR).
SHIP_MODES = ["REG_AIR", "AIR", "RAIL", "SHIP", "FOB", "MAIL", "CARGO"]
RETURN_FLAGS = ["R", "A", "N"]
LINE_STATUSES = ["O", "F"]
ORDER_STATUSES = ["O", "F", "P"]
NATION_NAMES = ["ALGERIA", "ETHIOPIA", "KENYA", "MOROCCO", "MOZAMBIQUE", "ARGENTINA", "BRAZIL", "CANADA", "PERU", "UNITED STATES", "INDIA", "INDONESIA", "JAPAN", "CHINA", "VIETNAM", "FRANCE", "GERMANY", "ROMANIA", "RUSSIA", "UNITED KINGDOM", "EGYPT", "IRAN", "IRAQ", "JORDAN", "SAUDI ARABIA"]
REGION_NAMES = ["AFRICA", "AMERICA", "ASIA", "EUROPE", "MIDDLE EAST"]
SHIP_INSTRUCTS = ["DELIVER IN PERSON", "COLLECT COD", "TAKE BACK RETURN", "NONE", "BILL OF LADING"]

# ---------------------------------------------------------------------------
# Text-column realism.
#
# The TPC-H query set filters on *literal* text values: `p_brand = 'Brand#23'`,
# `p_name LIKE '%green%'`, `o_comment LIKE '%special%requests%'`,
# `s_comment LIKE '%Customer%Complaints%'`, `o_orderpriority = '1-URGENT'`.
#
# A generator that fills those columns with constant or index-derived strings
# ("Part 17", "Ord comment", "Brand#123") makes every one of those filters
# match **zero** rows. The queries then degenerate into "the engine agrees
# with the oracle on an empty result" — a comparison that passes while
# proving nothing, which is exactly the failure mode this generator's
# key-distribution self-check was added to prevent (it caught a partsupp
# foreign key that referenced only 4 of 100 suppliers).
#
# The values below are the canonical TPC-H ones, so the standard query set
# exercises the rows it was written for.
# ---------------------------------------------------------------------------

# TPC-H brands are Brand#11..Brand#24; the standard queries filter on
# Brand#12 and Brand#23.
PART_BRANDS = [f"Brand#{i}" for i in range(1, 25)]

# Part names in TPC-H are a handful of colour words. The standard query set
# filters on `%green%`.
PART_COLORS = [
    "almond", "antique", "aquamarine", "azure", "beige", "bisque", "black",
    "blue", "blush", "brown", "burlywood", "chartreuse", "chocolate", "coral",
    "cornflower", "cream", "cyan", "forest", "green", "grey", "honeydew",
    "indianred", "ivory", "khaki", "lace", "lavender", "lawn", "lemon",
    "linen", "magenta", "maroon", "metallic", "midnight", "misty", "moss",
    "navy", "olive", "orange", "orchid", "pale", "papaya", "peach", "pink",
    "plum", "powder", "purple", "red", "rose", "royal", "saddle", "salmon",
    "sandy", "seashell", "sienna", "sky", "slate", "snow", "spring", "steel",
    "tan", "thistle", "tomato", "turquoise", "violet", "wheat", "white",
    "yellow",
]

# TPC-H part types are "<size> <finish> <colour>" — 6 x 5 x 5 = 150 types,
# the same shape the standard queries assume.
TYPE_SIZE = ["STANDARD", "SMALL", "MEDIUM", "LARGE", "ECONOMY", "PROMO"]
TYPE_FINISH = ["ANODIZED", "BURNISHED", "PLATED", "POLISHED", "BRUSHED"]
TYPE_COLOR = ["TIN", "NICKEL", "BRASS", "STEEL", "COPPER"]
PART_TYPES = [f"{s} {f} {c}" for s in TYPE_SIZE for f in TYPE_FINISH for c in TYPE_COLOR]

# TPC-H order priorities. The standard query set filters on '1-URGENT' and
# '2-HIGH'; the previous "O-PRIO-{n}" placeholder matched neither.
ORDER_PRIORITIES = ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"]

# TPC-H country calling codes, used for `c_phone`. The standard query set
# filters on `SUBSTR(c_phone, 1, 2) IN ('13','31','23','29','30','18','17')`.
PHONE_COUNTRY_CODES = list(range(11, 35))

# Part sizes are 1..50 and the containers are the standard TPC-H ones; the
# standard query set filters on size 15 and on the SMALL/MEDIUM containers.
PART_SIZES = list(range(1, 51))
# The 40 containers of the TPC-H `part.container` domain. The previous list
# carried only 12 of them and was missing `SM PKG` — one of the four
# containers the standard query set filters on, so that branch of the
# filter could never match.
PART_CONTAINERS = [
    "SM CASE", "SM BOX", "SM PACK", "SM PKG", "SM JAR", "SM DRUM",
    "MED BAG", "MED BOX", "MED PKG", "MED JAR", "MED PACK",
    "LG CASE", "LG BOX", "LG JAR", "LG PACK",
    "BAG CASE", "BAG BOX", "BAG PKG", "BAG JAR",
    "BOX CASE", "BOX BAG", "BOX BOX", "BOX JAR", "BOX PKG",
    "CASE PACK", "CASE JAR", "CASE BOX",
    "JAR PACK", "JAR BOX",
    "PAD BAG", "PAD CASE", "PAD JAR",
    "PKG BAG", "PKG BOX", "PKG CASE", "PKG JAR",
    "WRAP CASE", "WRAP BOX", "WRAP JAR", "WRAP PKG",
]

# The brand and the containers that the standard query set's five-way
# compound filter looks for. See the part generation below.
Q19_BRAND = "Brand#12"
Q19_CONTAINERS = ("SM CASE", "SM BOX", "SM PACK", "SM PKG")

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
    #
    # `n_regionkey` must follow the canonical grouping of `NATION_NAMES`:
    # nations 1-5 are AFRICA, 6-10 AMERICA, 11-15 ASIA, 16-20 EUROPE and
    # 21-25 MIDDLE EAST. The previous `(i % 5) + 1` assigned them round-robin,
    # so only the first five nations landed in the right region — GERMANY
    # (nation 17) was placed in AMERICA instead of EUROPE, and every query
    # that filters on a region (`r_name = 'EUROPE'`, `'ASIA'`) matched only
    # the wrong nations. That is the same class of defect the partsupp
    # foreign key had: a key that is in range but not *representative*.
    with open(out / "nation.tbl", "w") as f:
        for i in range(25):
            f.write(f"{i+1}|{NATION_NAMES[i]}|{(i // 5) + 1}|Nation {i+1} comment|\n")
    total_bytes += (out / "nation.tbl").stat().st_size
    print(f"  nation.tbl: 25 rows")
    
    n_supp = counts["supplier"]
    n_part = counts["part"]
    n_cust = counts["customer"]
    n_ps = counts["partsupp"]
    n_ord = counts["orders"]
    n_line = counts["lineitem"]
    
    # supplier
    #
    # `s_comment` carries the "Customer ... Complaints" text for a share of
    # the suppliers, because the standard query set filters on
    # `s_comment LIKE '%Customer%Complaints%'` — a filter that must actually
    # remove some suppliers, or the `NOT IN` subquery degenerates into a
    # no-op.
    with open(out / "supplier.tbl", "w") as f:
        for i in range(n_supp):
            phone = (
                f"{random.choice(PHONE_COUNTRY_CODES):02d}-"
                f"{random.randint(100, 999)}-{random.randint(100, 999)}-"
                f"{random.randint(1000, 9999)}"
            )
            if i % 10 == 0:
                comment = "Customer Complaints are rare. Regular business."
            else:
                comment = f"Regular supplier business {i+1}"
            f.write(f"{i+1}|Supplier {i+1}|Address {i+1}|{(i % 25) + 1}|{phone}|{random.uniform(-1000, 10000):.2f}|{comment}|\n")
    total_bytes += (out / "supplier.tbl").stat().st_size
    print(f"  supplier.tbl: {n_supp:,} rows")
    
    # customer
    #
    # `c_phone` carries a country calling code, because the standard query set
    # filters on `SUBSTR(c_phone, 1, 2) IN ('13','31','23','29','30','18','17')`.
    # The previous `+1-{i:010d}` placeholder made the first two characters "+1",
    # so that filter matched nothing.
    # The customer attributes are kept in memory because the *orders* table
    # needs them: the standard query set contains a query that reports "the
    # customers with an above-average balance who have **no** order at all",
    # and its result is empty unless some customers genuinely have none. See
    # the orders section below.
    cust_acctbal = []
    cust_phone_prefix = []
    with open(out / "customer.tbl", "w") as f:
        for i in range(n_cust):
            phone = (
                f"{random.choice(PHONE_COUNTRY_CODES):02d}-"
                f"{random.randint(100, 999)}-{random.randint(100, 999)}-"
                f"{random.randint(1000, 9999)}"
            )
            acctbal = random.uniform(-500, 10000)
            cust_phone_prefix.append(phone[:2])
            cust_acctbal.append(acctbal)
            f.write(f"{i+1}|Customer {i+1}|Address {i+1}|{(i % 25) + 1}|{phone}|{acctbal:.2f}|{random.choice(SEGMENTS)}|Cust comment {i+1}|\n")
    total_bytes += (out / "customer.tbl").stat().st_size
    print(f"  customer.tbl: {n_cust:,} rows")

    # part
    #
    # `p_brand` and `p_name` carry the canonical values the standard query set
    # filters on (Brand#12, Brand#23, `%green%`).
    #
    # Brand and container are assigned round-robin over the part index rather
    # than drawn at random. A random draw makes the compound filter
    # `p_brand = 'Brand#23' AND p_container = 'LG CASE'` — one part in 288 —
    # hit **zero** parts far too often (measured: 0 of the 78 Brand#23 parts
    # carried the LG CASE container), and the query it belongs to then
    # returns a NULL aggregate instead of a number. Round-robin assignment
    # guarantees that every (brand, container) pair is populated, so each
    # compound filter in the standard query set selects a real subset.
    #
    # The five-way compound filter of the standard query set goes one step
    # further: it also filters on the part size. At SF=0.01 a given
    # (brand, container) pair owns only about two parts, and the size range
    # covers a tenth of the sizes — so the conjunction is empty almost every
    # time. A handful of parts therefore carry the required brand, container
    # and size *by construction*, and the first line item of those parts
    # carries the quantity, ship mode and discount the same filter looks
    # for. The funnel probe below verifies that the seeding worked.
    q19_partkeys = set()
    with open(out / "part.tbl", "w") as f:
        for i in range(n_part):
            name = " ".join(random.choice(PART_COLORS) for _ in range(3))
            brand = PART_BRANDS[i % len(PART_BRANDS)]
            container = PART_CONTAINERS[(i // len(PART_BRANDS)) % len(PART_CONTAINERS)]
            # The size is assigned round-robin for the same reason the brand
            # and the container are: the standard query set filters on
            # specific sizes (Q2 on size 15, Q16 on nine sizes), and a
            # randomly drawn size leaves those filters matching almost
            # nothing. Round-robin gives every size the same number of
            # parts.
            size = PART_SIZES[i % len(PART_SIZES)]
            if (
                brand == Q19_BRAND
                and container in Q19_CONTAINERS
                and i < len(PART_BRANDS) * len(PART_CONTAINERS)
            ):
                size = 1 + (i % 5)
                q19_partkeys.add(i + 1)
            f.write(f"{i+1}|{name} part {i+1}|Mfr {i % 5 + 1}|{brand}|{random.choice(PART_TYPES)}|{size}|{container}|{random.uniform(100, 1000):.2f}|Part comment {i+1}|\n")
    total_bytes += (out / "part.tbl").stat().st_size
    print(f"  part.tbl: {n_part:,} rows")
    print(f"  seeded {len(q19_partkeys)} part(s) for the standard query set's compound filter")
    
    # partsupp
    #
    # TPC-H gives every part exactly 4 suppliers, and the supplier set has to
    # span the whole supplier population. The previous key formula was
    # `((i // n_part) % n_supp) + 1`: with n_part=2000 and n_ps=8000 that
    # yields `i // 2000` in {0,1,2,3} — i.e. **only 4 distinct suppliers were
    # ever referenced**, and every nation filter other than the first few
    # matched zero joined rows (measured: `cut -d'|' -f2 partsupp.tbl | sort -u
    # | wc -l` → 4). That made whole families of TPC-H queries vacuous.
    #
    # The fix keeps "4 suppliers per part" while spreading the parts across
    # the whole supplier range. The four suppliers of a part are drawn at
    # random instead of being computed from the part key: the arithmetic
    # formula `((p*7 + j) mod n_supp)` is *commensurate* with the
    # round-robin size assignment above (period 25 against period 50), so
    # every part of a given size ended up with the same four supplier keys
    # — and therefore with the same set of nations. The standard query that
    # filters on one specific size *and* on a region (size 15 + EUROPE)
    # then matched **zero** parts, every single time. A random draw removes
    # the correlation; the distinct-supplier check below still proves that
    # the whole supplier population is referenced.
    partsupp_suppliers = {}
    with open(out / "partsupp.tbl", "w") as f:
        for p in range(1, n_part + 1):
            sups = random.sample(range(1, n_supp + 1), 4)
            partsupp_suppliers[p] = sups
            for skey in sups:
                f.write(f"{p}|{skey}|{random.randint(1, 9999)}|{random.uniform(1, 1000):.2f}|PS comment|\n")
    total_bytes += (out / "partsupp.tbl").stat().st_size
    print(f"  partsupp.tbl: {n_ps:,} rows")
    
    # orders
    #
    # `o_orderpriority` carries the canonical TPC-H priorities, and
    # `o_comment` carries the "special requests" text for a share of the
    # orders — the standard query set filters on
    # `o_comment LIKE '%special%requests%'`, and the previous constant
    # "Ord comment" made that filter match nothing.
    #
    # The special-request orders are concentrated on every 20th customer so
    # that the filter is actually *selective*: spreading them over all
    # customers would leave almost every customer with at least one matching
    # order, and the `IN` / `NOT IN` subqueries would return "nearly
    # everything" / "nearly nothing" instead of two informative subsets.
    #
    # `o_custkey` is drawn at random rather than assigned round-robin. A
    # round-robin assignment gives every customer *exactly* the same number
    # of orders, and the standard "group the customers by how many orders
    # they have" query then collapses into a single group — a query that
    # passes while proving nothing. A random draw reproduces the spread the
    # query was written for at any scale factor.
    #
    # ------------------------------------------------------------------
    # Customers that have no order at all.
    #
    # The standard query set contains a query that reports "the customers
    # with an above-average account balance who have **no** order" (the
    # `NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)`
    # branch). Drawing `o_custkey` uniformly at random gives *every*
    # customer at least one order as soon as there are more orders than
    # customers — which is exactly the case here (15,000 orders over 1,500
    # customers). The query then returns **zero rows**, and a comparison
    # against the independent oracle degenerates into "both sides are
    # empty", which passes while proving nothing.
    #
    # Real dbgen data has such customers: at SF=1 the order count per
    # customer is ~10 and a small fraction of the customers falls outside
    # the order stream entirely. The generator therefore *reserves* a
    # deterministic slice of the customer population and never assigns an
    # order to them. The reserved customers are drawn from the ones the
    # query actually filters on (a qualifying country calling code and an
    # above-average balance) so the query returns a meaningful number of
    # rows rather than an empty set.
    # ------------------------------------------------------------------
    q22_phone_codes = {"13", "31", "23", "29", "30", "18", "17"}
    q22_pool = [
        i for i in range(n_cust)
        if cust_phone_prefix[i] in q22_phone_codes and cust_acctbal[i] > 0.0
    ]
    order_less_customers = set()
    if q22_pool:
        pool_avg = sum(cust_acctbal[i] for i in q22_pool) / len(q22_pool)
        above_avg = sorted(
            (i for i in q22_pool if cust_acctbal[i] > pool_avg),
            key=lambda i: cust_acctbal[i],
        )
        # ~5% of the customer population, spread evenly over the
        # above-average pool (sorted by balance) so the reserved customers
        # are not clustered at one end of the range.
        n_reserved = max(1, n_cust * 5 // 100)
        if above_avg:
            step = len(above_avg) / n_reserved
            order_less_customers = {
                above_avg[int(i * step)] + 1 for i in range(n_reserved)
            }
    print(
        f"  reserved {len(order_less_customers):,} customer(s) with no order "
        f"(the 'customer without an order' query needs a non-empty result set)"
    )
    with open(out / "orders.tbl", "w") as f:
        for i in range(n_ord):
            custkey = random.randint(1, n_cust)
            while custkey in order_less_customers:
                custkey = random.randint(1, n_cust)
            if custkey % 20 == 0:
                comment = "Customer asked for special requests on this order"
            else:
                comment = random.choice([
                    "Regular customer order",
                    "Routine order handling",
                    "Standard delivery instructions",
                    "Customer deposit received",
                ])
            f.write(f"{i+1}|{custkey}|{random.choice(ORDER_STATUSES)}|{random.uniform(100, 50000):.2f}|{gen_date()}|{random.choice(ORDER_PRIORITIES)}|Clerk#{i % 1000}|{random.randint(0, 1)}|{comment}|\n")
    total_bytes += (out / "orders.tbl").stat().st_size
    print(f"  orders.tbl: {n_ord:,} rows")

    # lineitem
    #
    # `l_partkey` is drawn per line item (TPC-H orders mix several parts) and
    # `l_suppkey` is one of the **four suppliers of that part** — the same
    # suppliers the `partsupp` table records for it. The previous formulas
    # gave every line item of an order the same part, and a supplier that was
    # not tied to the part at all, which broke the referential integrity the
    # standard queries join on.
    with open(out / "lineitem.tbl", "w") as f:
        n_line = 0
        for o in range(n_ord):
            # TPC-H orders carry 1..7 line items (4 on average). A fixed count
            # of 4 made the "orders whose total quantity exceeds 300" query
            # structurally empty: four quantities of at most 50 sum to at most
            # 200, so the threshold could never be reached. Drawing the count
            # keeps the *expected* number of line items per order at 4 — and
            # therefore the scale factor's line item volume — while letting
            # the threshold be reached.
            #
            # Every 25th order carries seven line items with a high quantity
            # and the next one carries a single line item. The average is
            # unchanged — (7 + 1 + 23 * 4) / 25 = 4 — so the scale factor's
            # line item volume stays put, but the seven-item orders reach a
            # total quantity above 300, which is the threshold the "large
            # order" query filters on. Without this, exactly one order in
            # 15,000 passed the filter.
            large_order = (o % 25 == 0)
            thin_order = (o % 25 == 1)
            if large_order:
                n_items = 7
            elif thin_order:
                n_items = 1
            else:
                n_items = random.randint(1, 7)
            for l in range(n_items):
                qty = random.randint(43, 50) if large_order else random.randint(1, 50)
                price = random.uniform(100, 10000)
                disc = random.uniform(0, 0.1)
                tax = random.uniform(0, 0.1)
                pkey = random.randint(1, n_part)
                shipmode = random.choice(SHIP_MODES)
                # The first line item of a part that was seeded for the
                # standard query set's five-way compound filter carries the
                # quantity, ship mode and discount that filter looks for.
                if l == 0 and pkey in q19_partkeys:
                    qty = random.randint(1, 10)
                    shipmode = "AIR"
                    disc = random.uniform(0.05, 0.07)
                # `partsupp` gives part p the four suppliers that were drawn
                # for it — pick one of them, so the line item's supplier
                # really supplies its part.
                skey = random.choice(partsupp_suppliers[pkey])
                f.write(f"{o+1}|{pkey}|{skey}|{l+1}|{qty}|{price:.2f}|{disc:.4f}|{tax:.4f}|{random.choice(RETURN_FLAGS)}|{random.choice(LINE_STATUSES)}|{gen_date()}|{gen_date()}|{gen_date()}|{random.choice(SHIP_INSTRUCTS)}|{shipmode}|Line comment|\n")
                n_line += 1
    total_bytes += (out / "lineitem.tbl").stat().st_size
    print(f"  lineitem.tbl: {n_line:,} rows")
    
    print(f"Total: {total_bytes:,} bytes ({total_bytes/1024/1024:.2f} MB)")

    # ------------------------------------------------------------------
    # Key-distribution self-check.
    #
    # "Foreign keys are in range" is a much weaker property than "foreign keys
    # are representative". A generator can satisfy the first while producing a
    # dataset in which, say, only 4 of 100 suppliers are ever referenced — and
    # then every query that filters on a nation silently matches nothing and
    # the assertions become vacuous. This check makes that failure mode
    # visible at generation time instead of as a puzzling test result.
    # ------------------------------------------------------------------
    def distinct_values(table: str, column: int) -> int:
        path = out / f"{table}.tbl"
        with open(path, encoding="utf-8") as fh:
            return len({line.split("|")[column] for line in fh if line.strip()})

    checks = [
        # (label, measured distinct count, expected minimum)
        ("partsupp.ps_suppkey distinct suppliers", distinct_values("partsupp", 1), min(n_supp, 100)),
        ("partsupp.ps_partkey distinct parts", distinct_values("partsupp", 0), min(n_part, 100)),
        ("supplier.s_nationkey distinct nations", distinct_values("supplier", 3), min(25, n_supp)),
        ("customer.n_nationkey distinct nations", distinct_values("customer", 3), min(25, n_cust)),
        ("lineitem.l_suppkey distinct suppliers", distinct_values("lineitem", 2), min(n_supp, 100)),
        ("lineitem.l_partkey distinct parts", distinct_values("lineitem", 1), min(n_part, 100)),
        # The lower bound used to be 95% of the customers, which *required*
        # every customer to have an order and therefore guaranteed that the
        # "customer without an order" query returns an empty set. The bound
        # is now 90%, and the complementary check below demands that a
        # meaningful slice of the customers has no order at all.
        ("orders.o_custkey distinct customers", distinct_values("orders", 1), max(1, int(n_cust * 0.90))),
        # The complement of the check above: a non-empty set of customers
        # without any order. Without it the "customer without an order"
        # query is structurally empty and every comparison on it is vacuous.
        ("customers with no order at all", n_cust - distinct_values("orders", 1), max(1, n_cust * 3 // 100)),
    ]
    print("=== key distribution self-check ===")
    degraded = []
    for label, measured, minimum in checks:
        ok = measured >= minimum
        if not ok:
            degraded.append((label, measured, minimum))
        print(f"  {'OK  ' if ok else 'FAIL'} {label}: {measured} (need >= {minimum})")
    if degraded:
        print("ERROR: degenerate key distribution — the fixture would make many queries vacuous")
        for label, measured, minimum in degraded:
            print(f"  - {label}: {measured} < {minimum}")
        sys.exit(1)

    # ------------------------------------------------------------------
    # Nation-to-region mapping.
    #
    # A round-robin assignment satisfies "5 distinct regions" while still
    # putting 20 of the 25 nations in the wrong one — the failure is
    # invisible to a distinct-count check and silently empties every query
    # that filters on a region. The mapping is therefore verified against
    # the canonical TPC-H grouping, nation by nation.
    # ------------------------------------------------------------------
    print("=== nation to region mapping ===")
    bad_nations = []
    with open(out / "nation.tbl", encoding="utf-8") as fh:
        for i, line in enumerate(fh):
            if not line.strip():
                continue
            fields = line.split("|")
            expected_region = (i // 5) + 1
            if fields[1] != NATION_NAMES[i] or int(fields[2]) != expected_region:
                bad_nations.append(
                    f"{fields[1]} (n_regionkey={fields[2]}, expected {expected_region})"
                )
    if bad_nations:
        print("ERROR: nation/region mapping does not follow the canonical TPC-H grouping — "
              "region-filtered queries would match the wrong nations")
        for n in bad_nations:
            print(f"  - {n}")
        sys.exit(1)
    print(f"  OK   all 25 nations sit in their canonical region")

    # ------------------------------------------------------------------
    # Predicate-coverage self-check.
    #
    # The key-distribution check above proves that the foreign keys are
    # *representative*. It does not prove that the **text** columns carry the
    # literal values the standard query set filters on. A generator that
    # writes "Ord comment" into `o_comment` passes every key check and still
    # makes `o_comment LIKE '%special%requests%'` match zero rows — the query
    # then degenerates into "the engine agrees with the oracle on an empty
    # result", which passes while proving nothing.
    #
    # Every entry below is a predicate taken from the standard query set. The
    # fixture must make it select a non-trivial share of its table.
    # ------------------------------------------------------------------
    def count_where(table: str, column: int, predicate) -> int:
        path = out / f"{table}.tbl"
        matched = 0
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                if not line.strip():
                    continue
                if predicate(line.split("|")[column]):
                    matched += 1
        return matched

    coverage = [
        # (label, table, column, predicate, minimum share of the table)
        ("orders.o_comment LIKE '%special%requests%'", "orders", 8,
         lambda v: "special requests" in v, 0.01),
        ("orders.o_orderpriority = '1-URGENT'", "orders", 5,
         lambda v: v == "1-URGENT", 0.05),
        ("part.p_brand = 'Brand#23'", "part", 3,
         lambda v: v == "Brand#23", 0.01),
        ("part.p_name LIKE '%green%'", "part", 1,
         lambda v: "green" in v, 0.01),
        ("part.p_type = 'ECONOMY ANODIZED STEEL'", "part", 4,
         lambda v: v == "ECONOMY ANODIZED STEEL", 0.001),
        ("part.p_type LIKE '%BRASS'", "part", 4,
         lambda v: v.endswith("BRASS"), 0.02),
        ("part.p_type LIKE 'MEDIUM POLISHED%'", "part", 4,
         lambda v: v.startswith("MEDIUM POLISHED"), 0.02),
        ("supplier.s_comment LIKE '%Customer%Complaints%'", "supplier", 6,
         lambda v: "Customer" in v and "Complaints" in v, 0.05),
        ("customer.c_phone country code in Q22's list", "customer", 4,
         lambda v: v[:2] in {"13", "31", "23", "29", "30", "18", "17"}, 0.05),
        ("lineitem.l_returnflag = 'R'", "lineitem", 8,
         lambda v: v == "R", 0.10),
        ("lineitem.l_shipmode = 'AIR'", "lineitem", 14,
         lambda v: v == "AIR", 0.05),
        ("lineitem.l_discount BETWEEN 0.05 AND 0.07", "lineitem", 6,
         lambda v: 0.05 <= float(v) <= 0.07, 0.05),
    ]
    print("=== predicate coverage self-check ===")
    degenerate = []
    for label, table, column, predicate, min_share in coverage:
        matched = count_where(table, column, predicate)
        total = counts[table]
        share = matched / total if total else 0.0
        ok = share >= min_share
        if not ok:
            degenerate.append((label, matched, total, min_share))
        print(f"  {'OK  ' if ok else 'FAIL'} {label}: {matched}/{total} "
              f"({share:.2%}, need >= {min_share:.2%})")
    if degenerate:
        print("ERROR: degenerate predicate coverage — the standard query set would match empty "
              "or near-empty result sets, and every comparison against the independent oracle "
              "would be vacuous")
        for label, matched, total, min_share in degenerate:
            print(f"  - {label}: {matched}/{total} < {min_share:.2%}")
        sys.exit(1)

    # ------------------------------------------------------------------
    # Compound-filter coverage.
    #
    # The standard query set also filters on a *pair* of columns. A pair that
    # happens to be empty empties the whole query, and the single-column
    # check above cannot see it: every brand and every container can be
    # perfectly represented while no brand ever occurs with a given
    # container. (Measured: all 78 Brand#23 parts carried a different
    # container, so `p_brand = 'Brand#23' AND p_container = 'LG CASE'`
    # matched nothing.)
    # ------------------------------------------------------------------
    def count_compound(table: str, predicate) -> int:
        matched = 0
        with open(out / f"{table}.tbl", encoding="utf-8") as fh:
            for line in fh:
                if line.strip() and predicate(line.split("|")):
                    matched += 1
        return matched

    compound_checks = [
        ("part.p_brand = 'Brand#23' AND p_container = 'LG CASE'", "part",
         lambda fs: fs[3] == "Brand#23" and fs[6] == "LG CASE"),
        ("part.p_brand = 'Brand#12' AND p_container IN ('SM CASE','SM BOX','SM PACK')", "part",
         lambda fs: fs[3] == "Brand#12" and fs[6] in ("SM CASE", "SM BOX", "SM PACK")),
        # The size + type filter of the standard query set. A *specific*
        # type combined with a specific size was checked before; that
        # combination (1 of 150 types times 1 of 50 sizes) is not a filter
        # any standard query uses, and it matched nothing at this scale.
        ("part.p_size = 15 AND p_type LIKE '%BRASS'", "part",
         lambda fs: fs[5] == "15" and fs[4].endswith("BRASS")),
    ]
    print("=== compound filter coverage self-check ===")
    empty_compounds = []
    for label, table, predicate in compound_checks:
        matched = count_compound(table, predicate)
        if matched == 0:
            empty_compounds.append(label)
        print(f"  {'OK  ' if matched else 'FAIL'} {label}: {matched} rows")
    if empty_compounds:
        print("ERROR: a compound filter matches no rows — the query that uses it would be "
              "structurally empty")
        for label in empty_compounds:
            print(f"  - {label}")
        sys.exit(1)

    # ------------------------------------------------------------------
    # Standard-query funnel self-check.
    #
    # The three checks above prove that individual *columns* and *pairs* of
    # columns carry usable values. They cannot see a query that is empty
    # because of a *subquery* or a *HAVING* threshold. The clearest example
    # is the "customer with an above-average balance who has **no** order"
    # query: not one of its predicates is empty, yet the result is empty
    # because every customer has an order, so the `NOT EXISTS` branch never
    # fires. Measured before this check existed: 0 rows, while the engine and
    # the independent oracle agreed on both being empty — a comparison that
    # passes while proving nothing.
    #
    # This check therefore runs the **whole standard query set** against the
    # generated fixture, in an in-memory SQLite, and measures how many rows
    # each query produces. A query that produces zero rows is structurally
    # empty and every comparison on it is vacuous.
    #
    # The floor for the single-row aggregate queries (Q6/Q14/Q17/Q18/Q19) is
    # 1, because TPC-H defines them to return exactly one row. For the
    # others the floor comes from the number of groups the query is defined
    # to produce.
    # ------------------------------------------------------------------
    repo_root = Path(__file__).resolve().parents[2]
    queries_dir = repo_root / "queries"
    if not queries_dir.is_dir():
        print(f"=== standard query funnel self-check: SKIPPED ===")
        print(f"  the standard query set was not found at {queries_dir}")
        print("  (the fixture was generated, but it could not be checked against the queries "
              "it is meant to serve)")
        print("Done.")
        return

    TPCH_SQLITE_SCHEMA = {
        "region": "r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT",
        "nation": "n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT",
        "supplier": "s_suppkey INTEGER PRIMARY KEY, s_name TEXT, s_address TEXT, s_nationkey INTEGER NOT NULL, s_phone TEXT, s_acctbal REAL, s_comment TEXT",
        "customer": "c_custkey INTEGER PRIMARY KEY, c_name TEXT, c_address TEXT, c_nationkey INTEGER NOT NULL, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT",
        "part": "p_partkey INTEGER PRIMARY KEY, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT",
        "partsupp": "ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL, ps_comment TEXT",
        "orders": "o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT",
        "lineitem": "l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT",
    }
    QUERY_MIN_ROWS = {
        1: 6, 2: 2, 3: 5, 4: 2, 5: 5, 6: 1, 7: 5, 8: 2, 9: 5, 10: 5,
        11: 10, 12: 2, 13: 5, 14: 1, 15: 10, 16: 10, 17: 1, 18: 1,
        19: 1, 20: 2, 21: 2,
        # Q22 groups by country calling code, so it returns at most seven
        # rows no matter how many customers qualify. Its real size is the
        # *inner* funnel, measured by the probe below.
        22: 1,
    }

    # ------------------------------------------------------------------
    # Inner funnel probes.
    #
    # Some standard queries return a single row *by design* (Q6/Q14/Q17/
    # Q18/Q19 are global aggregates). Their output count therefore cannot
    # show whether the filter underneath them was selective or vacuous.
    # These probes measure the number of rows that reach the aggregate, so
    # a reader can see how much work the query actually did.
    #
    # The floors are deliberately low: at SF=0.01 the tail-selective
    # queries (Q18's HAVING threshold, Q19's five-way filter) cannot reach
    # the selectivity they have at SF=1 — 15,000 orders and 60,000 line
    # items are simply not enough rows to make a "sum of quantities above
    # 300" filter hit often. The floor therefore only forbids an *empty*
    # funnel, and the measured value is printed so that a narrow funnel is
    # disclosed instead of hidden. The SF=1 scale layer (issue #5387) is
    # where the tail statistics are meaningful.
    # ------------------------------------------------------------------
    INNER_FUNNEL_PROBES = [
        ("Q17 line items reaching the aggregate",
         "SELECT COUNT(*) FROM lineitem, part WHERE p_partkey = l_partkey "
         "AND p_brand = 'Brand#23' AND p_container = 'LG CASE'", 1),
        ("Q18 orders whose SUM(l_quantity) exceeds 300",
         "SELECT COUNT(*) FROM (SELECT o_orderkey FROM customer, orders, lineitem "
         "WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey "
         "GROUP BY c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice "
         "HAVING SUM(l_quantity) > 300)", 1),
        ("Q19 line items reaching the aggregate",
         "SELECT COUNT(*) FROM lineitem, part WHERE p_partkey = l_partkey "
         "AND p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') "
         "AND l_quantity >= 1 AND l_quantity <= 10 AND p_size >= 1 AND p_size <= 5 "
         "AND l_shipmode IN ('AIR', 'AIR REG') AND l_discount >= 0.05 AND l_discount <= 0.07", 1),
        ("Q22 customers with an above-average balance and no order",
         "SELECT COUNT(*) FROM customer WHERE SUBSTR(c_phone, 1, 2) IN "
         "('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > "
         "(SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND "
         "SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND "
         "NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)", 20),
    ]
    THIN_FUNNEL = 10

    import sqlite3

    conn = sqlite3.connect(":memory:")
    for table, ddl in TPCH_SQLITE_SCHEMA.items():
        conn.execute(f"CREATE TABLE {table} ({ddl})")
        with open(out / f"{table}.tbl", encoding="utf-8") as fh:
            rows = []
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                fields = line.split("|")
                if fields and fields[-1] == "":
                    fields.pop()  # the .tbl format appends a trailing '|'
                rows.append(fields)
        if rows:
            conn.executemany(
                f"INSERT INTO {table} VALUES ({','.join('?' * len(rows[0]))})", rows
            )
    conn.commit()

    def adapt_for_sqlite(sql: str) -> str:
        # `EXTRACT(YEAR FROM x)` is the only construct in the standard query
        # set that SQLite does not understand.
        return re.sub(
            r"EXTRACT\(YEAR FROM ([^()]*)\)",
            r"CAST(strftime('%Y', \1) AS INTEGER)",
            sql,
        )

    print("=== standard query funnel self-check ===")
    empty_queries = []
    for n in range(1, 23):
        qpath = queries_dir / f"q{n}.sql"
        if not qpath.is_file():
            print(f"  FAIL  Q{n}: the query file {qpath} is missing")
            empty_queries.append(f"Q{n} (query file missing)")
            continue
        sql = adapt_for_sqlite(qpath.read_text(encoding="utf-8"))
        try:
            produced = len(conn.execute(sql).fetchall())
        except Exception as exc:  # noqa: BLE001 - the message is the report
            print(f"  FAIL  Q{n}: the query failed on the generated fixture: {exc}")
            empty_queries.append(f"Q{n} (query failed: {exc})")
            continue
        minimum = QUERY_MIN_ROWS[n]
        ok = produced >= minimum
        if not ok:
            empty_queries.append(f"Q{n} ({produced} rows, need >= {minimum})")
        print(f"  {'OK  ' if ok else 'FAIL'} Q{n:2}: {produced} rows (need >= {minimum})")
    if empty_queries:
        print("ERROR: a standard query is structurally empty on the generated fixture — every "
              "comparison of it against the independent oracle would compare two empty sets")
        for label in empty_queries:
            print(f"  - {label}")
        sys.exit(1)

    print("=== inner funnel probes ===")
    empty_funnels = []
    thin_funnels = []
    for label, probe, minimum in INNER_FUNNEL_PROBES:
        try:
            produced = conn.execute(probe).fetchone()[0]
        except Exception as exc:  # noqa: BLE001 - the message is the report
            print(f"  FAIL  {label}: the probe failed: {exc}")
            empty_funnels.append(f"{label} (probe failed: {exc})")
            continue
        ok = produced >= minimum
        if not ok:
            empty_funnels.append(f"{label} ({produced}, need >= {minimum})")
        note = ""
        if ok and produced < THIN_FUNNEL:
            thin_funnels.append(f"{label} ({produced})")
            note = f"  <- narrow funnel, disclosed (scale-layer statistics live in issue #5387)"
        print(f"  {'OK  ' if ok else 'FAIL'} {label}: {produced} (need >= {minimum}){note}")
    if empty_funnels:
        print("ERROR: a standard query's inner funnel is empty — the query is structurally "
              "vacuous even though it returns a row")
        for label in empty_funnels:
            print(f"  - {label}")
        sys.exit(1)
    if thin_funnels:
        print("NOTE: the following funnels are narrow at SF=0.01. The queries still exercise "
              "their join/aggregate path, but the comparison is narrow:")
        for label in thin_funnels:
            print(f"  - {label}")

    print("Done.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sf", type=float, required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()
    generate_sf(args.sf, args.output, args.seed)