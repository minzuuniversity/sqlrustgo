# Wired Soak Test — 4 Critical Bugs Found

> **Status**: RESOLVED
> **Resolution date**: 2026-07-02
> **Severity (when open)**: P0
> **Related**: #3225 (wired 24h soak), #3229 (wired 168h soak), TPC-H wire test `#[ignore]`
> **Closing PR**: [Gitea #3673](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/3673) — merged into `develop/v3.9.0` as commit `d162bc4ae` on 2026-07-02
> **Regression coverage**:
>   - `tests/wired_insert_payload_regression_test.rs` — 5 wire-protocol regression tests (Bug 1 + Bug 2)
>   - `crates/mysql-server/src/lib.rs::test_make_err_packet_null_byte_separator` — Bug 4 unit + integration
>   - `crates/mysql-server/src/lib.rs::test_make_err_packet` — Bug 4 enhanced with full ERR-packet layout assertion
>   - `tests/wire_protocol_smoke.rs` — pre-existing `INSERT INTO t VALUES (1, 'hello')` round-trip covers Bug 1 path
>   - `tests/ddl_e2e_test.rs` and executor unit tests cover `CREATE/DROP DATABASE` (Bug 3)
>
## Overview

During wired long-running soak testing (`scripts/stability/run_wired_soak.sh`), four distinct bugs were discovered that prevent the wired soak test from running successfully. These are **real protocol-level and DDL support issues**, not test flakiness.

---

## Resolution Summary

All four bugs were resolved prior to this issue being closed. The fixes
ship in `develop/v3.9.0` and `feature/issue-3423-tpch-sf1-baseline`.

| Bug | Root-cause fix | Regression test |
|-----|----------------|-----------------|
| Bug 1 — INSERT payload truncation | Wire protocol correctly forwards the full `Packet.payload[1..]` slice; `Packet::read_from` reads the `u24` length and consumes exactly that many bytes. Combined with `split_top_level_statements` (PR #3638) the multi-statement batch is now parsed and executed statement-by-statement. | `regression_insert_single_row_values_payload_not_truncated`, `regression_insert_single_row_various_types` |
| Bug 2 — bulk INSERT drops connection | Three converging protocol fixes — PR #3652 (charset 0x21, col-def fill byte, sequence reset on every new command), PR #3637 (TLS drain loop for multi-record TLS payloads), PR #3638 (`split_top_level_statements`) — keep the socket alive through multi-row INSERTs of any size. | `regression_insert_multi_row_200_rows_no_lost_connection`, `regression_insert_multi_row_1000_rows_25kb`, `regression_sysbench_prepare_pattern_composite` |
| Bug 3 — `CREATE DATABASE` / `DROP DATABASE` | Parser variants (`CreateDatabaseStatement`, `DropDatabaseStatement`) and executor handlers (`execute_create_database`, `execute_drop_database`, `execute_use_database`) ship in `crates/parser/src/parser.rs` and `src/execution_engine.rs`. `IF NOT EXISTS` / `IF EXISTS` are accepted. | `tests/ddl_e2e_test.rs` + `tests/wire_protocol_smoke.rs` round-trips |
| Bug 4 — error packet null-byte | `make_err_packet` pushes `0x00` between SQL state and message (commit `54403de9e`). | `test_make_err_packet_null_byte_separator` (integration), `test_make_err_packet` (unit, full layout assertion) |

---

## Verification

On `feature/issue-3423-tpch-sf1-baseline` at 2026-07-02:

```text
$ cargo test -p sqlrustgo --test wired_insert_payload_regression_test
running 21 tests
test common::tpch_cli_harness::tests::test_pick_free_port ... ok
test regression_insert_single_row_values_payload_not_truncated ... ok
test regression_insert_single_row_various_types ... ok
test regression_insert_multi_row_200_rows_no_lost_connection ... ok
test regression_insert_multi_row_1000_rows_25kb ... ok
test regression_sysbench_prepare_pattern_composite ... ok
... (16 mod tests) ... ok
test result: ok. 21 passed; 0 failed
```

5 of 21 tests are the new wire-protocol regression tests added for this
issue (covering Bug 1 single-row, Bug 1 value-variety, Bug 2 at 200/1000
rows and 25 KB payload, plus the sysbench-pattern composite). The
remaining 16 tests are the harness's own unit tests.

## Bug 1: INSERT Payload Truncation (COM_QUERY — VALUES content lost)

### Description

The `mysql` CLI sends INSERT as a COM_QUERY with the full SQL text, but the server only sees the truncated statement — the VALUES content is missing. This causes INSERT to fail with "Expected `(` after VALUES".

### Steps to Reproduce

1. Start sqlrustgo server
2. Use the `mysql` CLI to run:
   ```sql
   INSERT INTO region VALUES (1, 'Africa');
   ```
3. Server responds:
   ```
   ERROR 1064 (42000): Expected `(` after VALUES
   ```

### Expected vs Actual

- **Expected**: The INSERT succeeds, the row is written to the table.
- **Actual**: The server only receives `INSERT INTO region VALUES` (no VALUES content), causing a parse error.

### Root Cause Hypothesis

The server's COM_QUERY handler is truncating the payload before passing it to the parser. The VALUES clause content is being lost somewhere in the command dispatch pipeline.

### Development Tasks

- [x] **T1**: Trace the COM_QUERY payload through the server's `do_command_loop` — add debug logging at each hop to identify where the payload is truncated.
- [x] **T2**: Check if the issue is in packet framing (length calculation) or in the payload copy. Compare the raw packet received on the wire vs. the string passed to the parser.
- [x] **T3**: Verify the fix by running the INSERT with `mysql` CLI and confirming the row appears in the table.

---

## Bug 2: sysbench prepare — bulk INSERT fails with Lost Connection

### Description

The sysbench OLTP test bulk INSERT fails with "Lost connection to MySQL server during query" (error 2013). The server drops the connection during a multi-row INSERT.

### Steps to Reproduce

1. Start sqlrustgo server
2. Run sysbench prepare:
   ```bash
   sysbench oltp_read_write --db-driver=mysql --mysql-host=127.0.0.1 --mysql-port=3396 --mysql-user=root --mysql-password='' --db-name=test --tables=1 --table-size=100000 --threads=8 --report-interval=5 prepare
   ```
3. Observe the error:
   ```
   FATAL: mysql_stmt_execute() failed (error code: 2013)
   ERROR 2013 (HY000): Lost connection to MySQL server during query
   ```

### Expected vs Actual

- **Expected**: The bulk INSERT completes successfully and the table is populated.
- **Actual**: The server drops the connection mid-query, causing error 2013.

### Root Cause Hypothesis

Possible causes:
- The server's connection handling drops idle connections during long-running queries
- The server's query execution hits a panic or unhandled error during bulk INSERT
- The server's packet framing or payload size handling has a bug with large payloads

### Development Tasks

- [x] **T1**: Enable server-side logging at debug level and capture the exact moment the connection is dropped. Look for panics, assertion failures, or explicit `drop` in the connection handler.
- [x] **T2**: Check if the bulk INSERT payload exceeds the server's packet size limits. Compare the packet length header vs. the actual payload size.
- [x] **T3**: Test with progressively smaller `--table-size` values to find the threshold where the bug stops occurring.
- [x] **T4**: Verify the fix by running sysbench prepare and confirming all rows are inserted.

---

## Bug 3: CREATE DATABASE not supported

### Description

There is no `CreateDatabase` or `DropDatabase` variant in the `Statement` enum, so DDL for databases is not supported.

### Steps to Reproduce

1. Start sqlrustgo server
2. Use the `mysql` CLI to run:
   ```sql
   CREATE DATABASE tpch;
   ```
3. Server responds with a parse error.

### Expected vs Actual

- **Expected**: The database is created and can be used for subsequent queries.
- **Actual**: Parse error — the statement is not recognized.

### Root Cause

The `Statement` enum in `crates/parser/src/parser.rs` has table-level DDL variants (`CreateTable`, `DropTable`, `CreateIndex`, `DropIndex`, `CreateView`, `DropView`) but no database-level DDL variants.

### Development Tasks

- [x] **T1**: Add `CreateDatabase` and `DropDatabase` variants to the `Statement` enum in `crates/parser/src/parser.rs`.
- [x] **T2**: Add grammar rules in the SQL parser to recognize `CREATE DATABASE` and `DROP DATABASE` statements.
- [x] **T3**: Add execution logic — `CreateDatabase` should create the database directory in the data directory; `DropDatabase` should verify the database is empty and remove it.
- [x] **T4**: Add error handling — duplicate database, non-empty drop, etc.

---

## Bug 4: Error packet format bug — missing null-byte after SQL state

### Description

The error packet is missing the null-byte terminator after the SQL state string. The MySQL wire protocol requires the SQL state to be null-terminated before the error message.

### Steps to Reproduce

1. Trigger any server error (e.g., DROP a non-existent table).
2. The client receives an error packet.
3. The SQL state in the error packet is not null-terminated — the error message bytes immediately follow the SQL state.

### Expected vs Actual

- **Expected**: The error packet has the format: `0xFF + error_code (u16) + 0x23 + SQL_STATE (5 bytes) + 0x00 + ERROR_MESSAGE`.
- **Actual**: The error packet has the format: `0xFF + error_code (u16) + 0x23 + SQL_STATE (5 bytes) + ERROR_MESSAGE` — no null-byte between SQL state and message.

### Root Cause

In `crates/mysql-server/src/lib.rs`, the `make_err_packet` function at line 868:

```rust
fn make_err_packet(seq: u8, code: u16, state: &str, msg: &str) -> Packet {
    let mut p = Vec::new();
    p.push(0xff);
    p.write_u16::<LittleEndian>(code).unwrap();
    p.push(0x23);
    p.extend_from_slice(state.as_bytes());
    p.extend_from_slice(msg.as_bytes());  // ← Missing null-byte before msg
    Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }
}
```

The `0x00` null-byte between the SQL state and the error message is missing.

### Development Tasks

- [x] **T1**: Add `p.push(0x00);` after the SQL state in `make_err_packet` (line 874 of `crates/mysql-server/src/lib.rs`).
- [x] **T2**: Update the existing test `test_make_err_packet` to verify the null-byte is present.
- [x] **T3**: Add a dedicated test for error packet format — verify the null-byte separator between SQL state and message.

---

## Summary of Files to Modify

| Bug | File | Lines |
|-----|------|-------|
| Bug 1 (INSERT truncation) | `crates/mysql-server/src/lib.rs` | ~868+ (COM_QUERY handler) |
| Bug 2 (sysbench connection drop) | `crates/mysql-server/src/lib.rs` | ~868+ (COM_QUERY handler) |
| Bug 3 (CREATE DATABASE) | `crates/parser/src/parser.rs` | 57+ (Statement enum) |
| Bug 4 (error packet null-byte) | `crates/mysql-server/src/lib.rs` | 868-878 (make_err_packet) |

---

## Priority

1. **Bug 4** (error packet null-byte) — single-line fix, high impact on protocol correctness
2. **Bug 3** (CREATE DATABASE) — add DDL support, needed for TPC-H fixture loading
3. **Bug 1** (INSERT truncation) — protocol bug, may share root cause with Bug 2
4. **Bug 2** (sysbench connection drop) — may be the same root cause as Bug 1 (large payload truncation)

Bugs 1 and 2 may share a common root cause: the server's COM_QUERY payload handling has a bug with large or multi-part payloads. Fixing Bug 1 may automatically resolve Bug 2.
