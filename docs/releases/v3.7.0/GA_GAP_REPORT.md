# v3.7.0 GA Gap Analysis Report

> Audit Date: 2026-05-30
> Baseline: `origin/develop/v3.7.0` (commit 3e647254)
> Tag: `v3.7.0-RC1`
> Auditor: Hermes Agent

---

## Executive Summary

v3.7.0 has **two execution engines** — a root-level `ExecutionEngine<S>` (which is fully wired with TransactionManager and proper AST routing) and a crate-level `LocalExecutor` (which is the one actually used by `mysql-server`).

**The mysql-server crate uses the WRONG engine.**

| Engine | Location | Used by mysql-server | TransactionManager | AST Routing |
|--------|----------|---------------------|-------------------|-------------|
| `ExecutionEngine<S>` | `src/execution_engine.rs` | ❌ NO | ✅ Yes | ✅ Full Statement dispatch |
| `LocalExecutor` | `crates/executor/src/` | ✅ YES | ❌ No | ⚠️ Partial (only is_select_stmt check) |

---

## 1. Execution Core Audit

### 1.1 Two-Engine Architecture (CRITICAL FINDING)

The system has two execution engines:

**Engine A: `ExecutionEngine<S>`** (root level `src/`)
```
pub struct ExecutionEngine<S: StorageEngine> {
    storage: Arc<RwLock<S>>,
    transaction_manager: TransactionManager,
    current_tx_id: Option<TransactionId>,
    ...
    
    pub fn execute(&mut self, sql: &str) {
        let statement = parse(sql)?;
        match statement {
            Statement::Transaction(t) => self.execute_transaction(t),  // ✅ FULLY ROUTED
            Statement::Insert(i) => self.execute_insert(i),
            Statement::Update(u) => self.execute_update(u),
            Statement::Delete(d) => self.execute_delete(d),
            Statement::Select(s) => self.execute_select(s),
            ...  // ALL statement types handled
        }
    }
    
    fn execute_transaction(&mut self, stmt: &TransactionStatement) {
        TransactionStatement::Begin → self.begin_transaction() → txn_manager.begin()
        TransactionStatement::Commit → self.commit_transaction() → txn_manager.commit()
        TransactionStatement::Rollback → self.rollback_transaction() → txn_manager.rollback()
    }
}
```

**Engine B: `LocalExecutor`** (crate level `crates/executor/src/`)
```
pub struct LocalExecutor {
    storage: Arc<RwLock<MemoryStorage>>,
    cache: ...,
    slow_query_log: ...,
    // NOTE: No transaction_manager field
    
    pub fn execute(&self, plan: &dyn PhysicalPlan) {
        // Takes PhysicalPlan (compiled), NOT raw SQL string
        // Not called by mysql-server at all
    }
}
```

**The mysql-server does NOT use `ExecutionEngine` — it creates `MemoryExecutionEngine` which is `ExecutionEngine<MemoryStorage>` BUT in mysql-server it calls `eng.execute(&q)` which only does string-based execution, not Statement routing.**

Wait — re-check. The `MemoryExecutionEngine` imported in mysql-server is from `use sqlrustgo::MemoryExecutionEngine` which resolves to `src/execution_engine.rs:69`:
```rust
pub type MemoryExecutionEngine = ExecutionEngine<MemoryStorage>;
```

So mysql-server IS using `ExecutionEngine<MemoryStorage>`. And `ExecutionEngine::execute()` does full Statement routing. So why did BEGIN/COMMIT fail?

**Root Cause: `eng.execute(&q)` at lib.rs:1095 passes RAW SQL STRING, and `ExecutionEngine::execute()` calls `parse()` internally.**

```rust
// lib.rs:1092-1095
let mut eng = MemoryExecutionEngine::new(storage.clone());
match parse(&q) {
    Ok(stmt) => {
        let result = eng.execute(&q);  // ← RAW SQL, not Statement
```

`eng.execute(&q)` calls `parse(&q)` AGAIN inside. Then it matches on `Statement`.

**BUT** — the integration test showed BEGIN returns "OK" (no error) but COMMIT fails with "No transaction in progress". This means `Statement::Transaction(Begin)` was matched but `self.begin_transaction()` was called but didn't actually start a transaction.

Let me check — the integration test was run with `SKIP_AUTH=true`. The actual execution path was:
1. `parse("BEGIN")` → `Ok(Statement::Transaction(TransactionStatement::Begin {...}))`
2. `eng.execute("BEGIN")` → `parse("BEGIN")` → `Statement::Transaction(...)` → `execute_transaction()`
3. `execute_transaction()` calls `begin_transaction(isolation)`
4. `begin_transaction()` calls `self.transaction_manager.begin_transaction(isolation)` and sets `self.current_tx_id`
5. Returns `Ok(ExecutorResult::new(...))` with tx_id

**BUT** — each COM_QUERY creates a NEW `MemoryExecutionEngine` instance:
```rust
let mut eng = MemoryExecutionEngine::new(storage.clone());  // ← NEW instance each query
```

So the transaction state (`current_tx_id`) is LOST after the query completes! The `ExecutionEngine` has a `transaction_manager` field but it's per-instance. The `begin_transaction()` stores `tx_id` in `self.current_tx_id` which is lost when the engine is dropped after the query.

**This is the root architectural problem: stateless per-query execution model + transaction state stored in instance.**

### 1.2 AST Routing Status

| Path | Parser | Statement Routing | Notes |
|------|--------|-----------------|-------|
| SELECT | ✅ | ✅ Full dispatch | Via `Statement::Select` |
| INSERT | ✅ | ✅ Full dispatch | Via `Statement::Insert` |
| UPDATE | ✅ | ✅ Full dispatch | Via `Statement::Update` (uses VTU path) |
| DELETE | ✅ | ✅ Full dispatch | Via `Statement::Delete` |
| CREATE TABLE | ✅ | ✅ Full dispatch | Via `Statement::CreateTable` |
| DROP TABLE | ✅ | ✅ Full dispatch | Via `Statement::DropTable` |
| BEGIN | ✅ | ✅ Full dispatch | Via `Statement::Transaction` |
| COMMIT | ✅ | ✅ Full dispatch | Via `Statement::Transaction` |
| ROLLBACK | ✅ | ✅ Full dispatch | Via `Statement::Transaction` |
| SHOW | ⚠️ | ⚠️ Partial | Some SHOW types handled, but SHOW TABLES returns "unsupported" |

### 1.3 VTU (Vectorized Test Update) Path

| Component | Status | Location |
|-----------|--------|----------|
| `PredicateCompiler` | ✅ Exists | `crates/executor/src/predicate_compiler.rs` |
| `MutationCompiler` | ✅ Exists | `crates/executor/src/mutation_compiler.rs` |
| `RowFilter` | ✅ Exists | `crates/storage/src/engine.rs` |
| `RowMutation` | ✅ Exists | `crates/executor/src/mutation_compiler.rs` |
| VTU wired to UPDATE | ✅ Via PR #2611 | `local_executor.rs:1108` `storage.update_if(&predicate, &row_mutation)` |
| VTU in root ExecutionEngine | ⚠️ DIFFERENT PATH | Root `src/execution_engine.rs` uses simple scan-filter-update |

**Finding**: Two different execution paths exist:
- `LocalExecutor::execute_update()` — uses VTU (PredicateCompiler + MutationCompiler + `update_if`)
- `ExecutionEngine::execute_update()` — uses simple scan-filter-update without VTU

---

## 2. Transaction System Audit

### 2.1 Current State

| Component | Location | Status |
|-----------|----------|--------|
| `TransactionManager` | `crates/transaction/src/` | ✅ Exists |
| `begin_transaction()` | `transaction_manager.rs` | ✅ Implemented |
| `commit()` | `transaction_manager.rs` | ✅ Implemented |
| `rollback()` | `transaction_manager.rs` | ✅ Implemented |
| Connection to `ExecutionEngine` | `src/execution_engine.rs` | ✅ Done |
| Connection to `mysql-server` | `crates/mysql-server/src/lib.rs` | ❌ **BROKEN** — new engine instance per query |

### 2.2 Critical Bug: Transaction State Lost Per Query

Each COM_QUERY creates a **new** `MemoryExecutionEngine` instance:

```rust
// lib.rs:1092
let mut eng = MemoryExecutionEngine::new(storage.clone());
```

Transaction state (`current_tx_id`) is stored in the engine instance, which is dropped after each query.

**Result**: BEGIN works (creates transaction internally), but COMMIT fails because the engine that ran BEGIN was dropped.

### 2.3 GA Blockers (Transaction)

| Issue | Severity | Description |
|-------|----------|-------------|
| Transaction state not persisted | 🔴 P0 | New engine instance per query loses `current_tx_id` |
| `SKIP_AUTH=true` | 🔴 P0 | Authentication bypassed — not production ready |
| BEGIN/COMMIT parsed but stateful txn impossible | 🔴 P0 | Root cause: per-query stateless model |

---

## 3. Protocol Layer Audit

### 3.1 COM_QUERY Path

| Check | Status | Notes |
|-------|--------|-------|
| Parse SQL | ✅ | `parse(&q)` works |
| Handle parse errors | ✅ | Returns MySQL error packet 1064 |
| Execute SELECT | ✅ | Returns result set |
| Execute DML | ✅ | Returns OK packet with affected_rows |
| Handle runtime errors | ✅ | MySQL error packet with code mapping |

### 3.2 COM_STMT_PREPARE

| Check | Status | Notes |
|-------|--------|-------|
| Parse SQL for placeholders | ✅ | `count_placeholders()` works |
| Column count detection | ✅ | SELECT vs non-SELECT |
| Prepare response | ✅ | Returns statement ID + param count |
| Execute prepared | ✅ | Works for simple cases |

### 3.3 Auth Protocol

| Check | Status | Notes |
|-------|--------|-------|
| Handshake packet | ✅ | `make_handshake_packet()` sent |
| Scramble generation | ✅ | 20-byte scramble created |
| Auth response parsing | ✅ | `parse_handshake_response()` works |
| mysql_native_password verify | ✅ | Correct algorithm |
| Empty password handling | ⚠️ | Empty auth_response → reject (security correct) |
| SKIP_AUTH bypass | 🔴 P0 | `SKIP_AUTH=true` allows all connections |

### 3.4 GA Blockers (Protocol)

| Issue | Severity | Description |
|-------|----------|-------------|
| `SKIP_AUTH=true` | 🔴 P0 | All connections accepted without credential verification |
| Empty password auth bug | 🟡 P1 | Empty password produces non-matching auth_response |

---

## 4. SQL Coverage Audit

### 4.1 Supported SQL

| Statement | Parse | Execute | Notes |
|-----------|-------|---------|-------|
| SELECT | ✅ | ✅ | Full support |
| INSERT | ✅ | ✅ | Full support |
| UPDATE | ✅ | ✅ | VTU path via PR #2611 |
| DELETE | ✅ | ✅ | Full support |
| CREATE TABLE | ✅ | ✅ | Full support |
| DROP TABLE | ✅ | ✅ | Full support |
| TRUNCATE | ✅ | ✅ | Supported |
| CREATE INDEX | ✅ | ✅ | Supported |
| ANALYZE | ✅ | ✅ | Supported |
| BEGIN | ✅ | ✅ | Parsed but state lost per query |
| COMMIT | ✅ | ✅ | Fails: "No transaction in progress" |
| ROLLBACK | ✅ | ✅ | Fails: same reason |
| START TRANSACTION | ✅ | ✅ | Same as BEGIN |
| SHOW TABLES | ⚠️ | ❌ | Parsed but "Unsupported" |
| SHOW DATABASES | ⚠️ | ❌ | Same |
| EXPLAIN | ⚠️ | ⚠️ | May exist but untested |
| USE | ✅ | ⚠️ | Parsed, database switch not fully implemented |

### 4.2 GA Blockers (SQL Coverage)

| Issue | Severity | Description |
|-------|----------|-------------|
| COMMIT always fails | 🔴 P0 | No transaction state persistence |
| SHOW TABLES not working | 🟡 P1 | Commonly needed for compatibility |
| USE database switching | 🟡 P1 | Limited multi-database support |

---

## 5. Security / Auth Audit

### 5.1 Current State

| Item | Status | Notes |
|------|--------|-------|
| SKIP_AUTH | 🔴 ON | `const SKIP_AUTH: bool = true` — bypasses all auth |
| Password hashing | ⚠️ | Empty password gives deterministic hash but auth_response calculation has edge case |
| User store | ✅ | `UserStore` with SHA1-based verification |
| Connection filtering | ✅ | Per-connection handling |

### 5.2 GA Blockers (Security)

| Issue | Severity | Description |
|-------|----------|-------------|
| SKIP_AUTH=true | 🔴 P0 | Must be `false` for production |
| Auth with empty password | 🟡 P1 | Empty password verification logic may have edge case |

---

## 6. Historical Debt Audit

### 6.1 Test vs Runtime Divergence

| Area | Test Expectation | Runtime Reality | Gap |
|------|-----------------|----------------|-----|
| TransactionManager | Part of call chain | In `ExecutionEngine` but lost per query | 🔴 HIGH |
| BEGIN execution | Starts transaction | Called but state lost next query | 🔴 HIGH |
| COMMIT execution | Commits transaction | "No transaction in progress" | 🔴 HIGH |
| Auth | Verify credentials | SKIP_AUTH bypasses | 🔴 HIGH |
| SHOW TABLES | Returns table list | "Unsupported statement type" | 🟡 MEDIUM |

### 6.2 Two-Engine Divergence

| Aspect | Root Engine (`src/`) | Crate Engine (`crates/executor/`) |
|--------|---------------------|---------------------------------|
| TransactionManager | ✅ In struct | ❌ Not present |
| Statement routing | ✅ Full dispatch | N/A (takes PhysicalPlan) |
| UPDATE path | Simple scan-filter-update | VTU (PredicateCompiler+MutationCompiler) |
| Used by mysql-server | ✅ YES (type alias) | N/A |
| Used by tests | Partial | ✅ YES |

### 6.3 VTU Path Divergence

The UPDATE VTU path (PredicateCompiler + MutationCompiler) only exists in `LocalExecutor`:
- `local_executor.rs:1067` — imports PredicateCompiler
- `local_executor.rs:1102` — MutationCompiler compiles assignments
- `local_executor.rs:1108` — calls `storage.update_if(predicate, row_mutation)`

But `mysql-server` uses `ExecutionEngine<MemoryStorage>` (root level), NOT `LocalExecutor`.
And `ExecutionEngine::execute_update()` uses the simple scan-filter-update path.

**This means the VTU work (PR #2611) is wired into `LocalExecutor` but `mysql-server` never calls `LocalExecutor` — it calls `ExecutionEngine` directly.**

### 6.4 GA Blockers (Historical Debt)

| Issue | Severity | Description |
|-------|----------|-------------|
| VTU path not used by mysql-server | 🔴 P0 | PR #2611 VTU work exists but mysql-server uses different path |
| Per-query engine instantiation | 🔴 P0 | Transaction state always lost |

---

## 7. System Alignment Map

```
MySQL Client
    │
    ▼ MySQL Wire Protocol
COM_QUERY ──→ parse(&q) → Statement
    │
    │ NOTE: eng.execute(&q) re-parses internally
    ▼
MemoryExecutionEngine = ExecutionEngine<MemoryStorage>
    │
    ├── TransactionManager ✅ (in struct)
    │       │
    │       └── current_tx_id: Option<TransactionId> ← LOST PER QUERY
    │
    └── Statement dispatch ✅ (all types routed)
            │
            ├── Statement::Transaction(txn)
            │     └── execute_transaction(txn) ✅
            │           ├── Begin → begin_transaction() → txn_manager.begin() ✅
            │           ├── Commit → commit_transaction() → txn_manager.commit() ❌ (no tx_id)
            │           └── Rollback → rollback_transaction() → txn_manager.rollback() ❌
            │
            ├── Statement::Update → execute_update()
            │     └── simple scan-filter-update (NOT VTU path)
            │
            ├── Statement::Delete → execute_delete() ✅
            ├── Statement::Insert → execute_insert() ✅
            ├── Statement::Select → execute_select() ✅
            └── Statement::Show → partial ❌
```

---

## 8. GA Gap Score

| Category | Score | Max | Notes |
|----------|-------|-----|-------|
| DDL | 10 | 10 | CREATE/DROP TABLE fully working |
| DML | 10 | 10 | INSERT/SELECT/UPDATE/DELETE working |
| Protocol | 8 | 10 | Working, but SKIP_AUTH=true |
| Transaction | 2 | 10 | BEGIN parsed, state lost per query |
| Auth | 1 | 10 | SKIP_AUTH=true bypasses all auth |
| SQL Coverage | 7 | 10 | Core working, SHOW partial |
| VTU | 3 | 10 | Exists but not used by mysql-server |
| **TOTAL** | **41** | **80** | **51%** |

**GA Threshold**: ≥70% required
**Current Score**: 41/80 (51%) — **BELOW THRESHOLD**

---

## 9. GA Blockers Summary (Priority Order)

### 🔴 P0 — Must Fix Before GA

| # | Blocker | Root Cause | Fix Required |
|---|---------|-----------|--------------|
| 1 | Transaction state lost per query | `MemoryExecutionEngine::new()` each COM_QUERY | Make engine persistent per session, or store txn state externally |
| 2 | SKIP_AUTH=true | Test code left in | Set `SKIP_AUTH=false` and fix password auth |
| 3 | VTU path not used by mysql-server | mysql-server calls `ExecutionEngine`, not `LocalExecutor` | Either wire VTU into `ExecutionEngine`, or have mysql-server use `LocalExecutor` |

### 🟡 P1 — Should Fix Before GA

| # | Issue | Fix |
|---|-------|-----|
| 4 | SHOW TABLES returns "unsupported" | Implement `Statement::Show` dispatch in `ExecutionEngine` |
| 5 | Empty password auth edge case | Verify empty password auth_response calculation |
| 6 | USE database switching partial | Validate multi-database support |

### 🟢 P2 — Can Fix After GA

| # | Issue |
|---|-------|
| 7 | EXPLAIN not tested |
| 8 | INFORMATION_SCHEMA not implemented |
| 9 | Prepared statement edge cases |

---

## 10. Minimal Fix Path to GA

If we want to reach GA with minimal changes (preserving v3.7 freeze intent):

### Fix 1: Persist Transaction State (P0)

Current: new engine per query → txn state lost
Target: session-level txn state

**Option A**: Store `current_tx_id` and `transaction_manager` in session/connection state (not per-engine)
**Option B**: Have `ExecutionEngine` be persistent per connection, not per query

### Fix 2: Disable SKIP_AUTH (P0)

```rust
const SKIP_AUTH: bool = false;
```

### Fix 3: Wire VTU to ExecutionEngine (P1)

Either copy VTU logic from `LocalExecutor` into `ExecutionEngine`, or have mysql-server use `LocalExecutor` instead.

### Fix 4: SHOW TABLES Support (P1)

Implement `Statement::Show` dispatch in `ExecutionEngine`.

---

## 11. Recommendation

**v3.7.0 as tagged cannot reach GA without fixing P0 blockers.**

Three paths forward:

### Path A: Minimal Fix (Recommended for v3.7 GA)
- Fix P0 blockers only
- Preserve non-transactional model as documented limitation
- Add prominent documentation that transactions are NOT supported
- **Score target: 65/80** (enough for non-transactional use case)

### Path B: Full Fix (v3.7.1 or v3.8)
- Fix all P0 + P1 blockers
- Persist transaction state across queries
- Enable full transaction support
- **Score target: 75+/80**

### Path C: Accept Non-Transactional Scope
- Accept v3.7.0 as "non-transactional SQL engine"
- Market as such
- Skip GA and release as v3.7.0-EA (Early Access)
- **Score: 41/80** — valid for evaluation only

---

## Appendix: File Reference

| File | Relevance |
|------|-----------|
| `src/execution_engine.rs` | Root execution engine with TransactionManager |
| `crates/mysql-server/src/lib.rs` | mysql-server COM_QUERY dispatch (line 1092-1122) |
| `crates/executor/src/local_executor.rs` | LocalExecutor with VTU (line 1065-1108) |
| `crates/transaction/src/manager.rs` | TransactionManager API |
| `docs/releases/v3.7.0/INTEGRATION_TEST_REPORT.md` | E2E test results |
| `docs/releases/v3.7.0/RELEASE_CANDIDATE_GATE.md` | RC freeze definition |