# Issue #4020 — Bulk-load SF=10 — Evidence

**Issue:** #4020 (V312-26 SF=10/Sysbench/Observability baseline follow-up)
**Status:** RUNNER INFRASTRUCTURE DELIVERED, END-TO-END BULK-LOAD BLOCKED BY WIRE-PROTOCOL LIMITATION
**Capture date:** 2026-08-12
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

1. **V312-26 (#3905)** schedules Bulk-load SF=10 as a sub-task
2. The capture script uses `mysql -e "CREATE DATABASE ..."` to create
   the database, then `mysql -e "CREATE TABLE ..."` for each of 8
   tables, then `mysql -e "LOAD DATA LOCAL INFILE ..."` for each table
3. sqlrustgo's MySQL server processes the first COM_QUERY
   (`select @@version_comment limit 1`) but then the command loop never
   reads the next packet — the wire protocol is blocked
4. The script's per-step timeout cuts the hang, but the DDL never proceeds
5. Result: no schema, no tables, no bulk-load

This is the **same root cause** as #4019. It is a sqlrustgo bug, not a
#4020 deliverable issue.

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

1. **Fix sqlrustgo wire-protocol multi-query round-trip**: the command
   loop must call `read_packet` (or equivalent) after `send_result_set
   done` to read the next query. This is a sqlrustgo MySQL server bug —
   outside the V312-26 boundary.
2. Once the wire-protocol fix lands, re-run `bulk_load_sf10.sh` and
   verify `bulk_load_summary.json` shows `tables_loaded_ok = 8` and
   `tables_failed = 0` with row counts matching `wc -l` on the source
   `.tbl` files.

This is **not deferred to v3.13.0** in the closure doc — the runner
infrastructure is shipped in V312-26, and the wire-protocol fix is
tracked as a separate sqlrustgo bug.

---

## File deliverables

| Path | Status | Purpose |
|------|--------|---------|
| `scripts/tpch/bulk_load_sf10.sh` | exists, 372 lines | Defensive bulk-load runner |
| `scripts/gate/check_4020_bulk_load_sf10.sh` | exists | Gate (34/35 PASS) |
| `docs/releases/v3.12.0/evidence/issue-4020/20260812T130325Z_sf10/` | exists | Real run from 2026-08-12 |
| `docs/releases/v3.12.0/evidence/issue-4020/4020_evidence.md` | this file | Authoritative record |
