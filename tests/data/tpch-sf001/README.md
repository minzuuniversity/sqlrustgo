# ⚠️ DO NOT USE THIS FIXTURE ⚠️

> **Status**: CORRUPT — incompatible with TPC-H standard column order.
> **Discovered**: 2026-06-05, see
> `docs/discovery/2026-06-05-tpch-22-sf001-corrupt.md`.
> **Replaced by**: `/home/openclaw/sqlrustgo-tpch/data/` (canonical SF=0.01).

## What is wrong

The .tbl files in this directory have inconsistent per-row column counts
(mixing 8 / 9 / 10 fields) and the per-row column order does not match the
TPC-H standard schema. SQLite refuses to import them cleanly:

```
sqlite> .import tests/data/tpch-sf001/customer.tbl customer
tests/data/tpch-sf001/customer.tbl:1: expected 8 columns but found 9 - extras ignored
tests/data/tpch-sf001/customer.tbl:2: expected 8 columns but found 10 - extras ignored
...
```

## What you should do instead

Use the canonical SF=0.01 fixture at `/home/openclaw/sqlrustgo-tpch/data/`
(1500 customers / 15000 orders / 60000 lineitem rows = ~9 MB total on disk).
Its first row, e.g. `customer.tbl`:

```
1|Customer#0000001|0 Address|0|10-0000-0000|-500.00|AUTOMOBILE|Customer comment 0|
```

is 8 fields in standard TPC-H order
(`c_custkey, c_name, c_address, c_nationkey, c_phone, c_acctbal, c_mktsegment, c_comment`),
as expected.

## How this fixture was being used (and why it was wrong)

`tests/eval_22_vs_sqlite.rs` pointed at this directory and reported
"10/22 MATCHED" audit numbers. Because the .tbl files are corrupt, those
numbers were not measuring TPC-H query correctness — they were measuring
"engine behavior on garbage input". The numbers in PR #3125, #3126, #3127,
#3138 are all invalid as a result.

The canonical SF=0.01 audit (replacing this) will produce the real baseline.

## Cleanup

Once the canonical SF=0.01 audit is in place, this directory should be
**deleted** (or moved out of `tests/`) so no future session can accidentally
consume it again.
