# v4.1.0 / Issue #4910 §3.1 Phase 3 Plan

> **Date**: 2026-10-03
> **Status**: PLAN (no code changes)
> **Author**: omp-z440
> **Goal**: Make `execute()` on `ExecutionEngine<S>` accept `&self` (not just
> `execute_read_only`), so the engine's routing layer for **DDL/DML**
> (not just SELECT-family) doesn't serialize on `&mut self` at the routing
> layer. Combined with PR #4918 (Phase 1: atomics) and PR #4919 (Phase 2:
> `execute_read_only`), this completes the engine-side half of the §3.1
> BETA acceptance for Issue #4910.

## Background

Issue #4910 §3.1 acceptance criteria (full text):

> 1. **TPS metric**: `bench-v4.1.0-sysbench-oltp_read_write` 8 threads, 60s — Target ≥ 400 TPS
> 2. **Concurrency**: 16-thread sysbench reads — Effective concurrent SQL: 1 → ≥ 8
> 3. **Lock profile**: `lock_shared_slow` / `execute_select` ratio — 0.975 → ≤ 0.10
> 4. No regression: 145-test lib suite + alpha/beta composite gates stay PASS
> 5. 8h SOAK: per RSS-ceiling constraint

PR #4918 + #4919 + #4924 cover the **engine-layer §3.1 §3** (read-only
path: SELECT / EXPLAIN / SHOW / DESCRIBE / PRAGMA / VACUUM / REINDEX). What
remains for **DDL/DML** is the engine-layer `&mut self` serialisation that
PR #4919 deliberately left in place (it returns "use execute_mut() for
DDL/DML" for those arms).

This plan implements the rest of §3.1 by wrapping tx-state in
`Arc<parking_lot::Mutex<TxSession>>`, making DDL/DML methods callable
via `&self` with brief mutex locks instead of `&mut self`.

## Approach

### Step 1: introduce `TxSession` struct

Currently these mutable fields on `ExecutionEngine` require `&mut self`:

```rust
pub(crate) current_tx_id: Option<TxId>,
pub(crate) tx_status: TxStatus,
pub(crate) is_explicit_transaction: bool,
pub(crate) tx_readonly: bool,
pub(crate) default_isolation: TmIsolationLevel,
pub(crate) current_role: Option<String>,
```

Replace with a single field:

```rust
pub(crate) tx_session: Arc<parking_lot::Mutex<TxSession>>,
```

where:

```rust
pub(crate) struct TxSession {
    pub(crate) current_tx_id: Option<TxId>,
    pub(crate) tx_status: TxStatus,
    pub(crate) tx_readonly: bool,
    pub(crate) is_explicit_transaction: bool,
    pub(crate) default_isolation: TmIsolationLevel,
    pub(crate) current_role: Option<String>,
}
```

**Why these 6 and not the others?** `current_user` is a `Copy`-able small
struct that's only read on the auth check path — no contention, leave
direct. `session_null_order_first` has only 5 read sites — leave direct.
`session_vars` is already `Arc<RwLock<HashMap>>`. `trigger_undo_sink`
is already `Arc<Mutex<Vec<_>>>`. The 6 fields above are the ones
`execute_insert/update/delete` actually need to mutate during the hot
path.

### Step 2: update call sites (~22 in `execution_engine_methods.rs`)

Pattern translations:

```rust
// Read
if self.current_tx_id.is_some() { ... }
// becomes
if self.tx_session.lock().current_tx_id.is_some() { ... }

// Write
self.tx_status = TxStatus::Idle;
// becomes
self.tx_session.lock().tx_status = TxStatus::Idle;
```

For the **frequent** `begin_implicit_dml_tx` / `commit_implicit_dml_tx` /
`begin_transaction` / `commit_transaction` / `rollback_transaction` /
`savepoint` methods: take a `let mut session = self.tx_session.lock();` at
the top and use `session.field` throughout.

### Step 3: convert `begin_implicit_dml_tx` to `&self`

```rust
pub(crate) fn begin_implicit_dml_tx(
    &self,  // was &mut self
    op: &'static str,
    _table: &str,
) -> SqlResult<(Option<TxId>, bool)> {
    let mut session = self.tx_session.lock();
    if session.tx_readonly {
        return Err(SqlError::ExecutionError(
            "Cannot execute DML in READONLY transaction".to_string(),
        ));
    }
    match session.tx_status {
        TxStatus::Committed => return Err(...),
        TxStatus::Aborted => return Err(...),
        TxStatus::Idle | TxStatus::Active => {}
    }
    if session.current_tx_id.is_none() {
        let tx_id = self.transaction_manager.begin_transaction(session.default_isolation)?;
        session.current_tx_id = Some(tx_id);
        session.tx_status = TxStatus::Active;
        let mut storage = self.storage.write();
        storage.set_current_tx_id(tx_id.as_u64());
        Ok((Some(tx_id), true))
    } else {
        Ok((session.current_tx_id, false))
    }
}
```

### Step 4: convert `execute_insert` to `&self`

```rust
pub fn execute_insert<S: StorageEngine + 'static>(
    engine: &ExecutionEngine<S>,  // was &mut ExecutionEngine<S>
    insert: &InsertStatement,
) -> SqlResult<ExecutorResult> {
    // body unchanged — calls engine.begin_implicit_dml_tx (now &self) and
    // engine.commit_implicit_dml_tx (now &self)
}
```

This unlocks `engine.read().execute_insert(insert)` for the engine-level
INSERT hot path. The server (`crates/mysql-server/src/lib.rs:5314`) can
then route INSERT statements through `engine.read()` when no other
transaction state needs `&mut`.

### Step 5: make `execute()` itself `&self`

After step 4, all routing arms (DDL/DML/Transaction) in `execute()`
either already take `&self` (read-only) or now take `&self` (DDL/DML/Tx
after step 3-4). Convert:

```rust
pub fn execute(&self, sql: &str) -> SqlResult<ExecutorResult> {
    // match arms that previously dispatched via &mut self can now call
    // &self methods. The borrow checker enforces the §3.1 contract.
}
```

The existing `pub fn execute(&mut self, sql: &str)` is renamed to
`execute_mut(&mut self, sql: &str)` for callers that already hold
`&mut self` (e.g. tests that want to mix DDL/DML with internal state
inspection). `execute()` and `execute_mut()` are the entry points for
read-mostly and write-mostly server-side dispatch respectively.

### Step 6: server integration

`crates/mysql-server/src/lib.rs:5314` currently does:

```rust
let result = if let Some(stmt) = is_read_only {
    let rstmt = read_only_stmt(stmt);
    let eng = engine.read();
    match rstmt {
        Some(ReadOnlyStmt::Select(s)) => eng.execute_select(s),
        ...
    }
} else {
    let mut eng = engine.write();
    eng.execute(stmt_sql)
};
```

After Phase 3, both branches can use `engine.read()`:

```rust
let result = match is_read_only {
    Some(ReadOnlyStmt::Select(s)) => engine.read().execute_select(s),
    Some(ReadOnlyStmt::Show(s)) => engine.read().execute_show(s),
    Some(ReadOnlyStmt::Describe(s)) => engine.read().execute_describe(s),
    None => engine.read().execute(stmt_sql),  // routes to DDL/DML/Tx via &self
};
```

The single `engine.read()` lock is now the only contention point for
SELECT, INSERT, UPDATE, DELETE, COMMIT, ROLLBACK, etc. **No more
`engine.write()` anywhere in the per-statement hot path.**

## Why this is bounded

- 22 sites in `execution_engine_methods.rs` (TX lifecycle methods)
- 11 sites in `engine_dml.rs` (insert/update/delete)
- `begin_implicit_dml_tx`, `commit_implicit_dml_tx` signatures change to
  `&self` — the 22-site mechanical refactor
- `execute_insert/update/delete` signatures change to take `engine: &ExecutionEngine<S>` (was `&mut`) — the 11-site mechanical refactor
- No semantic changes; only borrow-checker relaxation
- New `pub fn execute(&self, sql)` entry point — pure routing logic, no behaviour change

## Estimated scope

- ~35 site updates (mechanical, low risk if patterns are uniform)
- 1 entry-point addition (`pub fn execute(&self, sql: &str)`)
- 1 entry-point rename (`execute` → `execute_mut`)
- New regression tests:
  - `test_v410_concurrent_inserts_via_ref_engine` — 4 threads × 100 INSERTs via `&engine`
  - Update `execute_insert/update/delete` test sites to pass `&engine` instead of `&mut engine`
- 1 server-side integration in `crates/mysql-server/src/lib.rs`

Estimated time: 1-2 days for the mechanical refactor + integration test,
half a day for the new bench, half a day for review and edge cases
(trigger_undu_sink concurrency, error paths).

## Risk register

| Risk | Mitigation |
|---|---|
| Lock-poisoning panic if a DML panics while holding `tx_session.lock()` | Recover via `parking_lot::Mutex::lock()`'s built-in poisoning recovery; or split into finer-grained locks per-method |
| `commit_implicit_dml_tx` holding the lock while calling `self.storage.write()` causes deadlock | Re-order: take storage lock first (fine), then session lock for the tx-id bookkeeping |
| Tx state writes race with the existing `tx_status = Committed; tx_id = None` ordering in `commit_transaction` | The mutex serialises the 6-field updates atomically (Rust's `parking_lot::Mutex` doesn't observe field-level atomicity). Re-check ordering in test pass/fail after the refactor |
| `current_user` (Copy, not in TxSession) and `session_null_order_first` (not in TxSession) — concurrent SELECTs read them via `&self`; tx-state reads must be consistent with them | Acceptable since they're never mutated on the DML hot path (only at init or via rare SET statements); if they ever are, move them into TxSession too |

## Acceptance for this PR

- All 145+ tests pass (current main lib count is 153 with Phase 1+2 tests)
- The new `test_v410_concurrent_inserts_via_ref_engine` runs to completion
- `cargo build --workspace` clean (no warnings from the refactor itself)
- `cargo test --package sqlrustgo-storage --lib` clean (763 tests)

## What this PR does NOT do

- **§3.1 §3 TPS metric #1** (`bench-v4.1.0-sysbench-oltp_read_write ≥ 400 TPS`) — needs the server-integration sysbench runner, deferred
- **§3.1 §3 lock_profile ratio** — needs `perf` + `cargo flamegraph` instrumentation on `FileStorage` under concurrent load, deferred
- **§3.1 §3 8h SOAK** — deferral per STAGE.yaml

## References

- Issue #4910 — full BETA acceptance gate
- PR #4918 — Phase 1 (atomics)
- PR #4919 — Phase 2 (`execute_read_only`)
- PR #4924 — TPS micro-bench (Phase 1+2 validated at 13k TPS)
- PR #4916 — storage-side `lockfree` forwarding (complementary)
- `docs/releases/v4.1.0/PHASE_1_SCOPE.md` §3.1 — covered scope
- `docs/releases/v4.0.0/V400_04_20MIN_SOAK.md` §3 — origin profile