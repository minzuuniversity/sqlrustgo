# Sysbench COM_STMT_PREPARE Compatibility + Row-Level Locking Design

**Status:** Draft (Brainstorming → Spec Self-Review)
**Iteration:** 2026-08-14
**Issues:** #4210 (V312-18b-blocker), #4211 (V312-18b-followup), #4019 (closure)
**Parent:** #4019 (Sysbench OLTP baseline metrics)

## Problem

#4019 (Sysbench OLTP baseline metrics) was closed on 2026-08-14 with `--db-ps-mode=disable` workaround for write_only/read_write workloads. Two follow-up issues block full closure:

1. **#4211 (V312-18b-followup):** sysbench defaults to prepared statements; without `--db-ps-mode=disable` it fails with `mysql_stmt_prepare() failed` and MySQL error 2027 "Malformed packet". This indicates the COM_STMT_PREPARE handler doesn't accept sysbench-generated payloads.

2. **#4210 (V312-18b-blocker):** `oltp_write_only` and `oltp_read_write` with 2 threads hit `Duplicate entry 'X' for key 'PRIMARY'` under concurrent DELETE+INSERT. SQLRustGo releases the storage write lock between statements within a transaction, so two transactions both observe the row as absent and race on INSERT.

Without fixing both, #4019's closure is incomplete — codex would reopen on subsequent audit.

## Goal

Make `sysbench oltp_read_only` / `oltp_write_only` / `oltp_read_write` execute successfully WITHOUT `--db-ps-mode=disable` on develop/v3.12.0, with metrics captured for all 4 OLTP workloads.

## Constraints

- **Wire protocol compatibility:** Must accept sysbench 1.0.20 generated payloads (LuaJIT-generated binary protocol).
- **Transaction correctness:** Must prevent duplicate PK under concurrent DELETE+INSERT transactions.
- **No regressions:** Existing tests (oltp_read_only without flag, COM_QUERY direct path) must continue to pass.
- **Backwards compatible:** Existing API surface (StorageEngine trait) must remain stable. New methods added with default implementations where possible.

## Architecture

### Phase 1: COM_STMT_PREPARE Compatibility (#4211)

**Target:** `crates/mysql-server/src/lib.rs` — `do_command_loop` COM_STMT_PREPARE branch (line 4456).

**Current state:** Handler returns OK packet with statement_id, column_count, param_count. The handler accepts the SQL text and stores it. Parameter types are inferred from the SQL via `infer_param_types_from_sql`.

**Bug:** When sysbench sends `SELECT c FROM sbtest1 WHERE id=?` with bind types, the response sequence (params definition packet after OK) gets malformed for sysbench's specific payload format.

**Fix:**
1. **Robust placeholder counting:** Replace `count_placeholders` (line 3009) with a parser-based count that respects string literals and comments.
2. **Type inference for `?` placeholders:** Improve `infer_param_types_from_sql` to handle sysbench's INTEGER bind types (currently all `?` are inferred as VARSTRING which MySQL clients reject).
3. **Param definition packet format:** Verify the binary format matches MySQL 5.6+ spec: `lenenc_str(name)`, `lenenc_str(org_name)`, `lenenc_str(table)`, `lenenc_str(org_table)`, `lenenc_str(db)`, `lenenc_str(catalog)`, `lenenc_int(length)`, `type_code`, `flags`, `decimals`, `filler`.
4. **OK packet:** Currently sends `column_count` as u16. MySQL spec: 0x00 (OK marker) + 4-byte stmt_id + 2-byte column_count + 2-byte param_count + 0x00 reserved + 2-byte warnings. Verify correct byte order.

**Phase 1 Tests:** `tests/integration/sysbench_prepared_test.rs`
- `test_com_stmt_prepare_select_with_bind`
- `test_com_stmt_prepare_insert_with_columns`
- `test_com_stmt_prepare_execute_then_close`
- `test_com_stmt_prepare_reset_connection`

### Phase 2: Row-Level Locking (#4210)

**Target:** `crates/storage/src/` — Add row-level lock abstraction.

**Strategy: Optimistic + Conflict-Detect (SQLite-style)**

SQLRustGo keeps storage under a single `parking_lot::RwLock<S>`. Within a transaction, multiple statements briefly hold the write lock then release. Two concurrent transactions see the same state.

**Approach:** Add a per-key lock map to `MemoryStorage` (and `FileStorage`) that tracks read/write intent on specific row keys.

```rust
// In StorageEngine trait:
trait StorageEngine {
    // ... existing methods ...

    /// Acquire a write intent on the given key for the transaction.
    /// Returns Ok(guard) if acquired; Conflict if another tx holds the lock.
    fn acquire_write_lock(&mut self, _tx_id: u64, _table: &str, _key: Value) -> SqlResult<LockGuard> {
        // Default: no-op (single-user)
    }

    /// Release the lock on commit/rollback.
    fn release_write_lock(&mut self, _guard: LockGuard) {}
}

// New type
pub struct LockGuard {
    tx_id: u64,
    table: String,
    key: Value,
}
```

**Phase 2 Implementation in `MemoryStorage`:**

```rust
struct MemoryStorage {
    // ... existing ...
    row_locks: HashMap<(String, Value), u64>, // (table, key) -> tx_id
}

impl MemoryStorage {
    fn acquire_write_lock(&mut self, tx_id: u64, table: &str, key: Value) -> SqlResult<LockGuard> {
        if let Some(&holder) = self.row_locks.get(&(table.to_string(), key.clone())) {
            if holder != tx_id {
                return Err(SqlError::ExecutionError(
                    format!("Lock conflict: tx {} holds key", holder)
                ));
            }
        }
        self.row_locks.insert((table.to_string(), key.clone()), tx_id);
        Ok(LockGuard { tx_id, table: table.to_string(), key })
    }

    fn release_write_lock(&mut self, guard: LockGuard) {
        if let Some(&holder) = self.row_locks.get(&(guard.table.clone(), guard.key.clone())) {
            if holder == guard.tx_id {
                self.row_locks.remove(&(guard.table, guard.key));
            }
        }
    }
}
```

**Integration in INSERT/DELETE/UPDATE:**

The integration point is in `StorageEngine::insert/delete/update` (when called from a transaction). The execution engine dispatcher needs to:
1. Detect the primary key (or unique key) from the table_info.
2. Acquire write lock on the key BEFORE the operation.
3. Release lock on commit or rollback.

For #4210, the simplest implementation is:
- In `MemoryStorage::insert`, if a primary key is defined, acquire the lock before insert.
- In `MemoryStorage::delete`, if a primary key is defined, acquire the lock before delete.
- Locks are released when the transaction commits/rolls back (via `TransactionManager`).

**Phase 2 Tests:** `tests/integration/sysbench_concurrent_test.rs`
- `test_concurrent_delete_insert_same_pk`
- `test_concurrent_two_transactions_update_same_row`
- `test_oltp_write_only_2_threads_no_conflict` (true bug repro)

### Phase 3: End-to-End Sysbench Validation

After Phase 1 + Phase 2 land:

```bash
# Run all 4 OLTP workloads WITHOUT --db-ps-mode=disable
for w in oltp_read_only oltp_write_only oltp_read_write oltp_insert; do
  sysbench $w --table-size=100 --threads=2 --time=10 \
    --mysql-db=$DB --mysql-host=127.0.0.1 --mysql-port=$PORT \
    --mysql-user=root run
done
```

Capture metrics for each workload. Save to `docs/releases/v3.12.0/evidence/issue-4019/20260814T<timestamp>_full_compat/`.

Update #4019 with cross-reference to #4210/#4211 closure.

## Data Flow

```
Client (sysbench)
    │
    ▼
COM_STMT_PREPARE packet  ──►  do_command_loop
    │                          │
    │                          ▼
    │                       parse SQL, count placeholders, infer types
    │                          │
    │                          ▼
    │                       ps_manager.add(sql, param_count, column_count, param_types)
    │                          │
    │                          ▼
    │                       OK packet (stmt_id, column_count, param_count)
    │                          │
    │                          ▼
    │                       Param def packets (if param_count > 0)
    │
    ▼
COM_STMT_EXECUTE packet  ──►  do_command_loop
    │                          │
    │                          ▼
    │                       parse_stmt_execute_params(payload, param_count, param_types)
    │                          │
    │                          ▼
    │                       replace_placeholders(sql, params) → final_sql
    │                          │
    │                          ▼
    │                       [BEGIN TRANSACTION] if autocommit=0
    │                          │
    │                          ▼
    │                       For INSERT/DELETE on PK:
    │                          │
    │                          ├─► acquire_write_lock(tx_id, table, key)
    │                          │      │
    │                          │      ├─► Conflict → return error to sysbench
    │                          │      │       (sysbench retries / load continues)
    │                          │      │
    │                          │      └─► Acquired → proceed
    │                          │
    │                          ▼
    │                       eng.execute(final_sql)
    │                          │
    │                          ▼
    │                       [COMMIT/ROLLBACK] → release_write_lock(guard)
    │
    ▼
COM_STMT_CLOSE packet  ──►  ps_manager.remove(stmt_id)
```

## Error Handling

| Error | Code | Trigger | Behavior |
|-------|------|---------|----------|
| Lock conflict | 1213 (ER_LOCK_DEADLOCK) | Two tx on same key | Return error to client; sysbench treats as retryable |
| Unknown statement | 1243 (ER_UNKNOWN_COM_ERROR) | Bad stmt_id | Return error; client re-prepares |
| Malformed packet | 1047 | Bad payload | Return error; client logs and skips |

## Testing Strategy

### Unit Tests
- `infer_param_types_from_sql` for sysbench query patterns
- `parse_stmt_execute_params` for various binary payload formats
- `count_placeholders` for SQL with literals/comments

### Integration Tests
- `tests/integration/sysbench_prepared_test.rs` — wire protocol tests
- `tests/integration/sysbench_concurrent_test.rs` — concurrent transaction tests

### End-to-End Tests
- Run actual sysbench against ephemeral server, capture metrics

### Acceptance Criteria

For #4211:
- `sysbench oltp_read_only` exit 0 WITHOUT `--db-ps-mode=disable`
- Prepare/Execute/Close/Reset round-trip works
- `cargo test --test sysbench_prepared_test`: 4/4 PASS

For #4210:
- `sysbench oltp_write_only --threads=2 --time=10` exit 0 WITHOUT `--db-ps-mode=disable`
- `sysbench oltp_read_write --threads=2 --time=10` exit 0 WITHOUT `--db-ps-mode=disable`
- 0 ignored errors, 0 duplicate PK
- `cargo test --test sysbench_concurrent_test`: 3/3 PASS

For #4019:
- All 4 OLTP workloads captured with metrics
- Existing #4019 evidence updated with new cross-references

## Out of Scope

- Server-side savepoints / nested transactions
- SERIALIZABLE isolation with predicate locks
- Adaptive query cache (separate issue)
- Async network I/O optimizations

## Risks

1. **Phase 2 row-lock scope:** Adding per-key locks is a public API change. Storage backend implementors need to provide the methods. Default implementations (no-op) keep backwards compatibility.

2. **sysbench protocol edge cases:** sysbench 1.0.20 may have specific payload formats. The fix may need to handle multiple variants. Mitigation: extensive integration tests.

3. **Performance:** Per-key locking adds Map operations. For 100-row tables, negligible. For larger tables, may need lock-free or RCU. Out of scope for #4210.

## Timeline

- Phase 1 (COM_STMT_PREPARE): 1-2 days
- Phase 2 (Row-level locking): 2-3 days
- Phase 3 (Validation): 1 day

Total: ~1 week.

## References

- Issue #4019: Sysbench OLTP baseline metrics
- Issue #4210: Row-level locking blocker
- Issue #4211: COM_STMT_PREPARE compatibility
- PR #4140: Wire protocol multi-query fix
- MySQL 5.6 Protocol Documentation: COM_STMT_PREPARE
- Sysbench 1.0.20 source: `oltp_common.lua:284`
