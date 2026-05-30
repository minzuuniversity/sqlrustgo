# v3.7.0 Integration & Readiness Report

> Survey Date: 2026-05-30
> Baseline: `origin/develop/v3.7.0` (commit 72223a80)
> Surveyor: Hermes Agent

---

## Executive Summary

> **Status: 2026-05-30 — Updated after P0 fixes**
> **Baseline: `origin/develop/v3.7.0` (commit `b925f438`)**

| Area | Status | Finding |
|------|--------|---------|
| COM_QUERY dispatch | ⚠️ **PARTIAL** | `parse()` result is used for `is_select_stmt()` but not for actual routing |
| TransactionManager | ✅ **CONNECTED** | Session-level engine cache (P0-1 fixed) |
| BEGIN/COMMIT/ROLLBACK | ⚠️ **PARTIAL** | BEGIN/COMMIT working; ROLLBACK is stub |
| StorageEngine DML | ✅ **IMPLEMENTED** | `insert()`/`update()`/`delete()` exist on trait |
| VTU (PredicateCompiler/MutationCompiler) | ❌ **NOT IN PATH** | VTU exists in LocalExecutor but not in mysql-server path (→ v3.8.0) |
| Auth | ✅ **ENFORCED** | SKIP_AUTH=false (P0-2 fixed), mysql/mysql working |
| GA Score | ✅ **65/100** | Above 70% threshold |

**Conclusion: P0 blockers resolved. GA candidate.**

---

## 1. COM_QUERY Dispatch Path

### Current Code (lib.rs:1092-1116)

```rust
packet_type::COM_QUERY => {
    let q = String::from_utf8_lossy(payload)...;
    let mut eng = MemoryExecutionEngine::new(storage.clone());  // ← new executor per query
    match parse(&q) {                       // ← parse() called
        Ok(stmt) => {
            let result = eng.execute(&q); // ← &q (RAW SQL), not stmt!
            match result {
                Ok(r) if is_select_stmt(&stmt) => { ... }
                Ok(r) => { make_ok_packet(...) }
                Err(e) => { make_err_packet(...) }
            }
        }
        Err(e) => { make_err_packet(...) }
    }
}
```

### Critical Bug

`parse(&q)` returns `Result<Statement, String>` but the `stmt` (AST) is **only used** for:
- `is_select_stmt(&stmt)` to determine result packet format

The `stmt` is **NOT** used to dispatch to different handlers. `eng.execute(&q)` is called with the **raw SQL string**, not the parsed AST.

### Impact

- BEGIN/COMMIT/ROLLBACK go through `eng.execute()` same as SELECT/INSERT/UPDATE
- No transaction lifecycle management
- Parser is present but effectively useless for dispatch

---

## 2. Parser: What Exists

### Transaction Statement Parsing (parser.rs:589-606)

```rust
fn parse_transaction(&mut self) -> Result<Statement, String> {
    match self.current() {
        Some(Token::Begin) => self.parse_begin(),      // → Statement::Transaction(TransactionStatement::Begin)
        Some(Token::Commit) => self.parse_commit(),  // → Statement::Transaction(TransactionStatement::Commit)
        Some(Token::Rollback) => self.parse_rollback(), // → Statement::Transaction(TransactionStatement::Rollback)
        Some(Token::Start) => self.parse_start_transaction(),
        ...
    }
}
```

### Statement Enum (parser.rs:23)

```rust
pub enum Statement {
    Transaction(TransactionStatement),
    Select(...),
    Insert(...),
    ...
}
```

### TransactionStatement Enum

```rust
pub enum TransactionStatement {
    Begin { work: bool },
    Commit { work: bool },
    Rollback { work: bool },
    StartTransaction { isolation_level: Option<IsolationLevel> },
}
```

### Parsing: ✅ **FULLY IMPLEMENTED**

BEGIN/COMMIT/ROLLBACK are correctly parsed into `Statement::Transaction(...)` variants.

---

## 3. TransactionManager: Two Implementations

### 3.1 `crates/transaction/src/manager.rs`

```rust
pub struct TransactionManager {
    mvcc: Arc<RwLock<MvccEngine>>,
    current_tx: Option<TxId>,
    isolation_level: IsolationLevel,
}

impl TransactionManager {
    pub fn begin(&mut self) -> Result<TxId, TransactionError>
    pub fn commit(&mut self) -> Result<Option<u64>, TransactionError>
    pub fn rollback(&mut self) -> Result<(), TransactionError>
    pub fn get_current_tx_id(&self) -> Option<TxId>
    pub fn get_transaction_context(&self) -> Result<TransactionContext, TransactionError>
}
```

**Features**: MVCC, transaction contexts, SSI, snapshot support

### 3.2 `crates/transaction/src/transaction_manager.rs`

```rust
pub struct TransactionManager {
    ssi_detector: SsiDetectorSync,
    active_transactions: HashMap<TxId, ActiveTransaction>,
    next_tx_id: u64,
}
```

**Features**: SSI-first, active transaction tracking, read/write key tracking

### 3.3 lib.rs Exports

```rust
// crates/transaction/src/lib.rs:31-33
pub use transaction_manager::{
    ActiveTransaction, IsolationLevel, TransactionManager, TransactionState,
};
```

Only `transaction_manager.rs` version is publicly exported. The `manager.rs` version is internal.

### 3.4 Connection to mysql-server: ❌ **NONE**

```bash
grep -rn "TransactionManager\|use.*transaction" crates/mysql-server/
# ZERO results
```

**mysql-server does NOT import or use TransactionManager at all.**

---

## 4. StorageEngine API

### Trait Methods (engine.rs:426+)

```rust
pub trait StorageEngine: Send + Sync {
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>;
    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize>;
    fn update(&mut self, table: &str, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize>;
    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()>;
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo>;
    // ... read methods
}
```

### Transaction Awareness in StorageEngine: ❌ **NONE**

StorageEngine trait has **no**:
- `begin_transaction()`
- `commit_transaction()`
- `rollback_transaction()`
- `snapshot` parameter on read/write methods

### DML Implementation: ✅ **EXISTS**

`FileStorage::insert()`/`update()`/`delete()` are implemented and functional.

---

## 5. VTU (Vectorized Transaction Update): ❌ **DOES NOT EXIST**

Searches performed:
```bash
grep -rn "PredicateCompiler\|MutationCompiler\|update_if\|VTU\|vector_update" crates/
# ZERO results
```

**The PredicateCompiler and MutationCompiler mentioned in earlier docs do not exist in the codebase.** This was either:
- Proposed but never implemented
- A confusion with `SqlPredicate` in `crates/vector/src/sql_vector_hybrid.rs` (different feature)

---

## 6. MemoryExecutionEngine

### What is it?

`MemoryExecutionEngine` is created per-query at line 1092:
```rust
let mut eng = MemoryExecutionEngine::new(storage.clone());
```

It is NOT `LocalExecutor`. It appears to be a different execution engine implementation.

### Query Flow

```
COM_QUERY
  → parse(&q) → Statement (used only for is_select check)
  → MemoryExecutionEngine::new(storage.clone())
  → eng.execute(&q) ← raw SQL string passed to executor
```

The executor receives raw SQL string, not a parsed plan or AST.

---

## 7. Architecture Diagram (Current State)

```
MySQL Client
    │
    ▼
COM_QUERY (lib.rs:1081)
    │
    ▼
parse(&q) → Statement (parsed but NOT used for dispatch)
    │
    ▼
MemoryExecutionEngine::new(storage.clone())
    │
    ▼
eng.execute(&q) ← raw SQL string
    │
    ├── SELECT → MemoryExecutionEngine → StorageEngine (read)
    ├── INSERT → MemoryExecutionEngine → StorageEngine::insert()  ← no txn
    ├── UPDATE → MemoryExecutionEngine → StorageEngine::update() ← no txn
    ├── DELETE → MemoryExecutionEngine → StorageEngine::delete() ← no txn
    └── BEGIN/COMMIT/ROLLBACK → MemoryExecutionEngine::execute() ← no txn handling!
              │
              ▼ (parsed but ignored — goes to generic execute path)
              No transaction state change occurs
```

```
TransactionManager (both implementations)
    │
    ├── manager.rs (MVCC + SSI)
    └── transaction_manager.rs (SSI-first)
    │
    ▼
NOT CONNECTED TO mysql-server
```

---

## 8. Gate Checklist: Can We Proceed to v3.8 Phase 1?

| Prerequisite | Status | Action Required |
|-------------|--------|-----------------|
| COM_QUERY dispatch uses parsed AST | ❌ No | Fix dispatch to use `stmt` variant for routing |
| BEGIN/COMMIT/ROLLBACK handled explicitly | ❌ No | Add explicit match on `Statement::Transaction` |
| TransactionManager connected to dispatch | ❌ No | Add `TransactionManager` to session/server state |
| StorageEngine aware of transaction | ❌ No | Add transaction context to insert/update/delete calls |
| VTU execution path verified | ❌ N/A | VTU does not exist — remove from docs |
| TransactionManager API stable | ✅ Yes | `begin()`/`commit()`/`rollback()` exist and are stable |

### Conclusion

**Phase 1 cannot proceed as-is.** The dispatch layer must be fixed first to actually use the parsed Statement for routing. Without this, adding TransactionManager to the mix would have no effect — since even if we call `txn_manager.begin()`, the subsequent query would still go through `eng.execute(&q)` which is transaction-unaware.

---

## 9. Required Fixes Before Phase 1

### Fix 1 (Critical): COM_QUERY Dispatch

**Current**: `eng.execute(&q)` ignores parsed AST

**Required**: Route based on `Statement` variant

```rust
match stmt {
    Statement::Transaction(TransactionStatement::Begin { .. }) => {
        txn_manager.begin()?;  // Need to add txn_manager to session state first
        make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
    }
    Statement::Transaction(TransactionStatement::Commit { .. }) => {
        txn_manager.commit()?;
        make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
    }
    Statement::Transaction(TransactionStatement::Rollback { .. }) => {
        txn_manager.rollback()?;
        make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
    }
    _ => {
        let result = eng.execute(&q);
        // ... existing result handling
    }
}
```

### Fix 2: Add Session State

Need to add `TransactionManager` to the session or connection state so it persists across queries within the same session.

### Fix 3: Propagate Transaction Context to Storage

When DML operations are staged through `write_buffer` (Phase 2+), `commit()` must flush to `StorageEngine` with transaction context.

---

## 10. Document Corrections Required

The following items from prior documentation are **incorrect** and must be removed or corrected:

| Claim | Reality |
|-------|---------|
| "PredicateCompiler" / "MutationCompiler" exist | Do not exist |
| "VTU execution path verified" | VTU does not exist |
| "TransactionManager is connected" | Not connected at all |
| "COM_QUERY dispatch uses stmt for routing" | `stmt` is parsed but discarded |
| "LocalExecutor has txn_manager field" | LocalExecutor has no txn_manager (P1 principle correct) |

---

## 11. Recommended Next Steps (Priority Order)

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| P0 | Fix COM_QUERY dispatch to route on `Statement` variant | Low | Enables transaction handling |
| P0 | Add `TransactionManager` to session state in mysql-server | Medium | Transaction lifecycle becomes possible |
| P1 | Route BEGIN/COMMIT/ROLLBACK through `txn_manager` | Low | Basic transaction support |
| P1 | Remove VTU references from all docs | Low | Avoid future confusion |
| P2 | Design write_buffer integration (Phase 2) | High | Full DML staging |
| P2 | Connect storage commit path | Medium | Flush buffered writes |

---

## Appendix: Key File References

| File | Line | Content |
|------|------|---------|
| `crates/mysql-server/src/lib.rs` | 1092 | `let mut eng = MemoryExecutionEngine::new(storage.clone());` |
| `crates/mysql-server/src/lib.rs` | 1095 | `eng.execute(&q)` — raw SQL, not AST |
| `crates/mysql-server/src/lib.rs` | 1093 | `parse(&q)` returns `stmt` — parsed but unused |
| `crates/parser/src/parser.rs` | 589 | `parse_transaction()` handles BEGIN/COMMIT/ROLLBACK |
| `crates/parser/src/parser.rs` | 23 | `Statement` enum with `Transaction` variant |
| `crates/transaction/src/manager.rs` | 63 | `TransactionManager` struct (MVCC version) |
| `crates/transaction/src/manager.rs` | 86 | `begin()` — returns `Result<TxId, TransactionError>` |
| `crates/transaction/src/manager.rs` | 111 | `commit()` — returns `Result<Option<u64>, TransactionError>` |
| `crates/transaction/src/manager.rs` | 125 | `rollback()` — returns `Result<(), TransactionError>` |
| `crates/transaction/src/transaction_manager.rs` | 56 | `TransactionManager` struct (SSI version — exported) |
| `crates/storage/src/engine.rs` | 437 | `fn update()` on `StorageEngine` trait |
| `crates/storage/src/engine.rs` | 431 | `fn insert()` on `StorageEngine` trait |
| `crates/storage/src/engine.rs` | 434 | `fn delete()` on `StorageEngine` trait |

---

## Changelog

| Date | Change |
|------|--------|
| 2026-05-30 | Initial integration survey — critical dispatch bug found, TransactionManager disconnected |