# Execution Semantics Contract (SEM-1)

**Status**: ACTIVE
**Owner**: SQLRustGo Execution WG
**Effective from**: v3.8.0
**Related**: #2975 (SEM-1), #2976 (SEM-2), #2973 (INT-4), #2974 (ARCH-2)

## Purpose

Establishes a single, binding execution semantics for every code path that
processes a SQL statement in SQLRustGo. Any code that violates this contract
is a release-blocker bug.

## Scope

Applies to every executor in the workspace:

| Executor | Path | Mod-tree status |
|----------|------|-----------------|
| `ExecutionEngine<S>` (root) | `src/execution_engine.rs` | in mod tree |
| `LocalExecutor` (in-tree) | `crates/executor/src/local_executor_dml.rs` | in mod tree |
| `SqlExecutor` | `crates/executor/src/sql_executor.rs` | in mod tree |
| `ParallelVolcanoExecutor` | `crates/executor/src/parallel_executor.rs` | **not in mod tree — historical** |
| `LocalExecutor` (legacy) | `crates/executor/src/local_executor.rs` | **not in mod tree — superseded by `local_executor_dml`** |

Files outside the mod tree are dead code; they are kept for reference only.

## Contract

### 1. Single DML Entry

All DML (INSERT / UPDATE / DELETE) MUST go through `ExecutionEngine::execute`
→ the dedicated `execute_insert` / `execute_update` / `execute_delete`
methods. No other executor may write directly to storage.

**Enforcement**: VtuGuard panic in `crates/storage/src/vtu_guard.rs` for any
bypass.

### 2. Single SELECT Entry

All SELECT execution (including CTEs, unions, subqueries) MUST go through
`ExecutionEngine::execute_select`. The internal `engine_select.rs` helpers
may be called only from this entry point.

### 3. Result Format

Every successful execution returns `ExecutorResult { rows, affected_rows }`
where:

- `rows: Vec<Vec<Value>>` — the result set
- `affected_rows: usize` — for DML, the number of rows touched
- For DDL: `affected_rows = 0` or `1` (depending on operation); `rows = []`
- For SELECT: `rows` contains the data; `affected_rows = rows.len()`
- For ANALYZE: single-row result with the row count

### 4. Error Format

All errors flow as `SqlResult<T> = Result<T, SqlError>`. `SqlError`
variants:

- `ParseError(String)` — produced by parser
- `ExecutionError(String)` — produced by any executor
- `StorageError(String)` — produced by storage
- `TransactionError(String)` — produced by TM
- `TypeError(String)` — produced by type checker

**Rule**: error messages MUST be human-readable, lowercase, and start with
the failing entity (table name, column name, etc.) when applicable.

### 5. Transaction Lifecycle

Every DML statement that is not in an explicit transaction is automatically
wrapped in an implicit transaction by `ExecutionEngine`. The transaction
follows the TX lifecycle:

```
BEGIN → DML... → COMMIT (on success) | ROLLBACK (on error)
```

The lifecycle is enforced at `ExecutionEngine::execute` boundary; lower
layers MUST NOT start or commit transactions.

### 6. WAL Recording

All DML writes MUST emit WAL entries with a non-zero `tx_id`. The
transaction ID is set by `ExecutionEngine` at `BEGIN` time and propagated
to `WalStorage` via `set_tx_id`.

**Enforcement**: VtuGuard verifies tx_id is non-zero at commit.

### 7. NULL Semantics

NULL is a distinct value, not equal to anything (including itself). The
3-value logic (TRUE / FALSE / UNKNOWN) applies to all comparisons.
Per-aggregate NULL behavior:

- `COUNT(*)` counts all rows, including those with NULL
- `COUNT(col)` skips NULLs
- `SUM`, `AVG`, `MIN`, `MAX` skip NULLs; return NULL on empty
- `STDDEV`, `VARIANCE`, `MEDIAN` skip NULLs; return NULL on empty

### 8. Identifier Resolution

Table and column names are case-sensitive in storage layer, but parser
normalizes them to lowercase before storage lookup. Quoted identifiers
preserve case.

### 9. Authorization

DML and DDL operations check `current_user` against `Privilege` table.
Authorization is enforced at the start of each statement, before any work
is done.

## Compliance Verification

Run:

```bash
bash scripts/gate/check_execution_semantics.sh
```

This script (to be added in v3.8.0+) greps for forbidden patterns:

- Direct `storage.write()` / `storage.update()` / `storage.delete()` calls
  outside `ExecutionEngine`
- Direct `TransactionManager::begin` outside `ExecutionEngine`
- Use of `LocalExecutor` from `crates/executor/src/local_executor.rs` from
  any other module (only `local_executor_dml.rs` may be used)

Any non-empty output is a release blocker.

## Migration Notes

- v3.7.0 had three execution paths: `local_executor.rs`, `parallel_executor.rs`,
  and `local_executor_dml.rs`. v3.8.0 unifies on `local_executor_dml.rs` +
  `ExecutionEngine`.
- The legacy `local_executor.rs` and `parallel_executor.rs` files are
  retained for archaeology but excluded from `lib.rs` mod tree. Tests
  inside those files are not compiled.
- New execution code MUST be added to `local_executor_dml.rs` or
  `ExecutionEngine`. Adding new files requires updating this contract.

## Related

- ADR-006: TX-WAL contract deferral
- ADR-009: G-01 validation chain enforcement
- Issue #2975 (SEM-1): this document
- Issue #2976 (SEM-2): error code standardization (follow-up)
