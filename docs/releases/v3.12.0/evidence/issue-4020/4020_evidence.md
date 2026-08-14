# Issue #4020 — Bulk-load SF=10 — Evidence

**Issue:** #4020 (V312-26 SF=10/Sysbench/Observability baseline follow-up)
**Status (2026-08-12):** RUNNER INFRASTRUCTURE DELIVERED; bulk-load blocked by wire-protocol multi-query round-trip (same root cause as #4019)
**Status (2026-08-13, current):** WIRE-PROTOCAL BLOCKER REMOVED — 3 of 8 TPC-H SF=10 tables now load with `parity=match`; the remaining 5 tables are blocked by a **throughput ceiling** in `FileStorage` (full-table re-serialization on every 100-row flush). Root cause located and reproduced with a 50.6× speedup micro-benchmark (see "Throughput root cause" below).
**Capture date range:** 2026-08-12 → 2026-08-13
**Related:** #3905 (V312-18 parent), #4018 (SF=10), #4019 (Sysbench)

---

## STRICT PROOF MODE audit

Per user's directive:
> "只以 252 Gitea 的 origin/develop/v3.12.0 当前 HEAD 为事实基线"
> "不允许根据报告标题、Issue 状态、PR 描述或脚本 exit=0 直接判断完成"
> "脚本 exit=0 不是 PASS"
> "不要证明你做过，要证明当前 develop 已经真实满足原始验收条件"

All evidence below is captured from a real
`bash scripts/tpch/bulk_load_sf10.sh` invocation on the current develop
HEAD. Files are written to
`docs/releases/v3.12.0/evidence/issue-4020/20260812T130325Z_sf10/`.

**The script exited non-zero (rc=1).** STRICT PROOF MODE: this is a
FAILURE — schema creation did not complete, no tables were loaded,
`bulk_load_summary.jsonl` was never created.

---

## What was delivered (verifiable)

1. **`scripts/tpch/bulk_load_sf10.sh`** — 372-line runner that:
   - 8 TPC-H schemas defined (region, nation, supplier, customer, part,
     partsupp, orders, lineitem) — matches the canonical
     `scripts/stability/load_tpch_fixture.sh` column order
   - `TABLES` array ordered small-to-large (FK-safe ordering)
   - Per-table LOAD DATA LOCAL INFILE with `FIELDS TERMINATED BY '|'
     LINES TERMINATED BY '\n'` (matches dbgen `.tbl` output exactly)
   - Per-table metrics: `elapsed_sec`, `rows_per_sec`, `row_count_parity`
     (match / mismatch / load_failed)
   - JSONL accumulator → `bulk_load_summary.json` via Python
   - Per-step `timeout` wrappers so wire-protocol hangs cannot stall
     the whole capture
   - Always writes `metadata.json` so the gate can see what was attempted
   - Server lifecycle managed via `trap` cleanup

2. **Gate** (`scripts/gate/check_4020_bulk_load_sf10.sh`) — verifies:
   - `scripts/tpch/bulk_load_sf10.sh` exists with valid bash syntax
   - All 8 `SCHEMA_*` constants + `TABLES` array present
   - LOAD DATA LOCAL INFILE pattern + dbgen terminators present
   - Per-step timeout (MYSQL_TIMEOUT_SEC, LOAD_TIMEOUT_SEC) present
   - Per-table metrics (`elapsed_sec`, `rows_per_sec`, `parity`,
     `bulk_load_summary.json`) present
   - Evidence directory + metadata.json + server.log present
   - **34/35 PASS** as of 2026-08-12 (the 1 FAIL is the evidence doc —
     see "Last refreshed" below)

3. **Runner end-to-end (real run)** — verified:
   ```
   $ bash scripts/tpch/bulk_load_sf10.sh
   [bulk_load_sf10] starting sqlrustgo-mysql-server on 127.0.0.1:23308 …
   [bulk_load_sf10] server listening (pid=18520)
   [bulk_load_sf10] creating database + 8 TPC-H schemas
   [bulk_load_sf10:ERROR] CREATE DATABASE failed (see schema_create.log)
   [bulk_load_sf10:ERROR] schema creation failed — see *_create.log
   [bulk_load_sf10] stopping server (pid=18520)
   # exit 1
   ```

## What was NOT delivered (honest gap)

**As of 2026-08-12** (`20260812T130325Z_sf10`):

1. **End-to-end bulk-load of all 8 TPC-H tables at SF=10** — `create_schemas`
   failed at the very first step (`mysql -e "CREATE DATABASE IF NOT EXISTS tpch_sf10;"`)
   due to the same sqlrustgo wire-protocol multi-query round-trip limitation
   documented in #4019. The server logs show:

   ```
   Connection from 127.0.0.1:33542
   Handshake response: cap=0x19bfaa85 ...
   Auth accepted, sending OK packet, seq=3
   Starting command loop, seq=4
   Query [127.0.0.1:33542]: select @@version_comment limit 1
   send_result_set: 3 cols, 1 rows, start_seq=1
   send_result_set done: final_seq=8
   [SILENCE — server never reads the next packet]
   ```

   The mysql client then blocks forever waiting for `CREATE DATABASE` to be
   processed. The script's `timeout 15` wrapper cuts the hang at 15s.

2. **No `bulk_load_summary.json`** — the JSONL accumulator was never created
   because the loop over `TABLES` was never reached.
3. **No `row_count_parity` results** — no rows loaded against which to
   compare `wc -l` on the source `.tbl` files.

**As of 2026-08-13** (`20260813T132615Z_sf10`): wire-protocol blocker removed
(committed as part of #4020 itself: `--load-infile-dir` flag). 3 of 8
TPC-H SF=10 tables load with `parity=match`. 2 tables (customer, part)
time out at the 1800 s LOAD_TIMEOUT_SEC. 3 tables (partsupp, orders,
lineitem) not attempted after the timeouts.

```jsonl
{"table":"region",   "src_lines":5,      "loaded_rows":5,       "elapsed_sec":0.049, "rows_per_sec":102,  "rc":0,   "parity":"match"}
{"table":"nation",   "src_lines":25,     "loaded_rows":25,      "elapsed_sec":0.043, "rows_per_sec":581,  "rc":0,   "parity":"match"}
{"table":"supplier", "src_lines":100000, "loaded_rows":100000,  "elapsed_sec":897.322,"rows_per_sec":111,  "rc":0,   "parity":"match"}
{"table":"customer", "src_lines":1500000,"loaded_rows":-1,      "elapsed_sec":1800.005,"rows_per_sec":0,  "rc":124, "parity":"load_failed"}
{"table":"part",     "src_lines":2000000,"loaded_rows":-1,      "elapsed_sec":1800.006,"rows_per_sec":0,  "rc":124, "parity":"load_failed"}
```

---

## Captured output (real run)

| Artifact | Path | Content |
|----------|------|---------|
| Bulk-load log | `20260812T130325Z_sf10/bulk_load_log.txt` | Shows start → schema fail → stop sequence |
| Metadata | `20260812T130325Z_sf10/metadata.json` | Config + sqlrustgo version + knobs |
| Schema create log | `20260812T130325Z_sf10/schema_create.log` | Empty (timeout stderr only) |
| Server log | `20260812T130325Z_sf10/server.log` | Wire-trace of the wire-protocol hang |
| Bulk-load summary | `20260812T130325Z_sf10/bulk_load_summary.jsonl` | **NOT CREATED** — schema creation failed |

### Metadata captured

```json
{
  "issue": "#4020",
  "run_id": "20260812T130325Z_sf10",
  "captured_at": "2026-08-12T13:03:41Z",
  "sqlrustgo_version": "sqlrustgo-mysql-server 0.1.0",
  "mysql_version": "mysql  Ver 8.0.46-0ubuntu0.24.04.3 for Linux on x86_64 ((Ubuntu))",
  "config": {
    "host": "127.0.0.1",
    "port": 23308,
    "db": "tpch_sf10",
    "user": "openclaw",
    "data_dir": "/tmp/tpch-sf10",
    "auth_mode": "none",
    "max_connections": 8,
    "server_threads": 4,
    "storage": "file",
    "wal_sync": "every"
  },
  "knobs": {
    "mysql_timeout_sec": 15,
    "load_timeout_sec": 60
  }
}
```

This proves the script started correctly, found the server binary, found
the SF=10 fixture, picked a free port, started the server, and got to
the first DDL — then failed honestly.

---

## Why this is blocked (causal chain)

**2026-08-12 root cause (now removed):** wire-protocol multi-query round-trip
limitation — same as #4019. Fixed by the `--load-infile-dir` flag (commit
`b263f4a97f`).

**2026-08-13 root cause (current):** `FileStorage` write amplification.

1. **V312-26 (#3905)** schedules Bulk-load SF=10 as a sub-task
2. The capture script uses `mysql -e "CREATE DATABASE ..."` to create
   the database, then `mysql -e "CREATE TABLE ..."` for each of 8
   tables, then `mysql -e "LOAD DATA LOCAL INFILE ..."` for each table
3. **Wire layer**: each `mysql -e` invocation now completes (post-fix);
   LOAD DATA LOCAL INFILE streams up to 16 MB of file bytes per packet
   and the inner loop in `crates/mysql-server/src/lib.rs`
   (`handle_load_local_infile`) accumulates complete lines into
   `pending_rows` before deciding to flush — effective batch is one
   packet, not `PERIODIC_FLUSH_ROWS=100`.
4. **Storage layer**: `Storage::insert` → `FileStorage::insert_buffered`
   appends each batch to `self.insert_buffer[table]`. Every time
   `buffered.len() >= buffer_threshold` (default **100** rows),
   `flush_buffer` calls `insert_direct`, which:
   - clones the entire `TableData` (`data.clone()`)
   - calls `save_table`, which **clones all rows again** into a
     `StoredTableData` and `serde_json::to_string_pretty`s the whole
     table, then `File::create` (truncate) + `write_all`.
5. Each flush is therefore O(rows_loaded). With a batch size of 100,
   total work to insert N rows is Σ_{k=1}^{N/100} k·row_size → **O(N²)**
   in rows, dominated by the largest late flushes.

For SF=10 supplier (100 k rows): 1000 flushes, each serializing a
growing JSON file (~40 MB final). For customer (1.5 M) and part
(2 M): 15 000 – 20 000 flushes each, with the table growing to
68 MB / 60 MB respectively. At supplier's observed 111 rows/s,
customer would need ~3.7 h and part ~5 h, which exceeds the
`LOAD_TIMEOUT_SEC=1800` (30 min) wrapper. lineitem (~60 M rows)
would need ~6 days.

### Throughput root cause (measured)

A focused micro-benchmark
(`crates/storage/tests/bulk_load_quadraticity.rs`, in this commit)
drives `FileStorage::insert` directly with N=30 000 synthetic TPC-H
shaped rows in batches of `buffer_threshold`. Per-batch wall time:

| rows_so_far | batch_us | json_bytes |
|---:|---:|---:|
| 200 | 5 662 | 111 061 |
| 2 000 | 48 595 | 1 101 774 |
| 4 000 | 93 934 | 2 204 294 |
| 6 000 | 145 706 | 3 307 091 |
| 8 000 | 199 433 | 4 409 184 |
| 10 000 | 237 464 | 5 511 320 |
| 12 000 | 299 207 | 6 614 808 |
| 14 000 | 342 368 | 7 717 050 |
| 16 000 | 389 835 | 8 819 396 |
| 18 000 | 439 936 | 9 921 558 |
| 20 000 | 488 819 | 11 023 774 |
| 22 000 | 560 007 | 12 125 990 |
| 24 000 | 593 952 | 13 228 206 |
| 26 000 | 647 204 | 14 330 422 |
| 28 000 | 688 602 | 15 432 638 |
| 30 000 | 740 330 | 16 574 429 |

Per-batch time grows linearly with `rows_so_far` — exactly the
O(N²) signature of "serialize the whole table on every flush".

Comparing the two thresholds head-to-head on identical 30 000-row
loads (identical final on-disk state):

| `buffer_threshold` | Total time | rows/s | Speedup |
|---:|---:|---:|---:|
| 100 (current default) | 109.87 s | 273 | 1× |
| 10 000 | 2.17 s | 13 825 | **50.6×** |

Hypothesis confirmed: raising the storage-layer flush threshold is
the dominant lever. (WAL fsync was independently ruled out: the
2026-08-13 run used `WAL_SYNC=off` and supplier still sustained
111 rows/s. The `WAL_SYNC` knob added to `bulk_load_sf10.sh` is the
evidence of that A/B.)

---

## Why not use `sqlrustgo-mysql-server exec`?

We investigated the `exec` subcommand as a workaround. Inspection showed
that `exec` uses `MemoryExecutionEngine` (ephemeral, in-memory, no
persistence, no `--data-dir` flag). It cannot satisfy the bulk-load
acceptance criteria because:

- Tables are dropped on process exit
- No WAL means no crash recovery
- A 600M-row lineitem table requires ~6.8 GB resident — single-process
  MemoryExecutionEngine would not survive even one query

So `exec` is **not** a viable bulk-load path. The wire-protocol fix is
the only path forward.

---

## Verification commands (all runnable today)

```bash
# 1. Gate (verifies infrastructure + evidence exists)
bash scripts/gate/check_4020_bulk_load_sf10.sh
# Expect: ✅ #4020 gate PASSED (34/35 — see infra above)

# 2. Bash syntax
bash -n scripts/tpch/bulk_load_sf10.sh
# Expect: (no output, exit 0)

# 3. Re-run the defensive capture (will fail at schema creation, but
#    captures the wire-protocol failure honestly)
bash scripts/tpch/bulk_load_sf10.sh
# Expect: schema creation fails, metadata.json written, exit 1

# 4. Inspect the captured metadata
cat docs/releases/v3.12.0/evidence/issue-4020/20260812T130325Z_sf10/metadata.json
# Expect: config knobs + sqlrustgo/mysql versions
```

---

## Path to full #4020 completion (post-V312-26)

To complete end-to-end bulk-load:

1. **(DONE)** Fix sqlrustgo wire-protocol multi-query round-trip:
   landed in commit `b263f4a97f` (`--load-infile-dir` flag).
2. **(NEXT)** Raise `FileStorage::buffer_threshold` from 100 to a much
   larger value (e.g. 10 000) so the storage layer batches bulk inserts
   the way the wire layer already does. The micro-bench above shows
   ~50× speedup with no behavioural change. This is a one-line default
   change in `FileStorage::new` / `FileStorage::new_with_wal`; the
   existing `new_with_buffer_config` constructor already supports
   arbitrary values.
3. **(AFTER)** Optionally switch `save_table` from pretty-printed JSON
   to compact JSON (or a row-append format) — the
   `serde_json::to_string_pretty` cost alone is several hundred ms per
   flush at the 16 MB-row table size, on top of the underlying
   serialization.
4. Once (2) lands, re-run `bulk_load_sf10.sh` and verify
   `bulk_load_summary.json` shows `tables_loaded_ok = 8` and
   `tables_failed = 0` with row counts matching `wc -l` on the source
   `.tbl` files.

Items 2 and 3 are sqlrustgo-storage bugs, not V312-26 boundary
issues — but they are the real blockers for the acceptance criterion
"bulk-load SF=10 succeeds end-to-end". They are tracked as part of the
#4020 follow-up, not deferred to v3.13.0.

---

## File deliverables

| Path | Status | Purpose |
|------|--------|---------|
| `scripts/tpch/bulk_load_sf10.sh` | exists, modified | Defensive bulk-load runner; `WAL_SYNC` env override added (no behavioural change at default) |
| `scripts/gate/check_4020_bulk_load_sf10.sh` | exists | Gate (34/35 PASS) |
| `docs/releases/v3.12.0/evidence/issue-4020/20260812T130325Z_sf10/` | exists | Real run from 2026-08-12 (wire-protocol blocker) |
| `docs/releases/v3.12.0/evidence/issue-4020/20260813T110046Z_sf10/` | exists | Run from 2026-08-13 11:00Z; pre-`--load-infile-dir` build → all tables rc=1 |
| `docs/releases/v3.12.0/evidence/issue-4020/20260813T125349Z_sf10/` | exists | Run from 2026-08-13 12:53Z; aborted — fixture dir missing 5 .tbl files |
| `docs/releases/v3.12.0/evidence/issue-4020/20260813T132615Z_sf10/` | exists | Run from 2026-08-13 13:26Z; `WAL_SYNC=off`, supplier 111 rows/s, customer/part timeout at 1800 s |
| `crates/storage/tests/bulk_load_quadraticity.rs` | exists | O(N²) write-amplification micro-bench (50.6× speedup at threshold=10 000) |
| `docs/releases/v3.12.0/evidence/issue-4020/4020_evidence.md` | this file | Authoritative record |
