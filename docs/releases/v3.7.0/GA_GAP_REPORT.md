# SQLRustGo v3.7.0 GA Gap Analysis Report

> Audit Date: 2026-05-30
> Baseline: `origin/develop/v3.7.0` (commit d7d5cfdc, tag v3.7.0-RC1)
> Auditor: Hermes Agent

---

## 1. Executive Summary

| Field | Value |
|-------|-------|
| GA Readiness Score | 41 / 100 |
| Blockers (P0) | 3 |
| High Risk (P1) | 3 |
| Backlog (P2) | 3 |
| GA Threshold | 70 / 100 |

**Verdict: ❌ NOT GA READY — Below 70% threshold**

---

## 2. P0 — GA Blockers (Must Fix Before GA)

### 2.1 Transaction State Lost Per Query

| Field | Value |
|-------|-------|
| File | `crates/mysql-server/src/lib.rs:1092` |
| Location | COM_QUERY dispatch loop |
| Issue | Each COM_QUERY creates a **new** `MemoryExecutionEngine` instance. Transaction state (`current_tx_id`) is stored in the engine instance, which is dropped after each query completes. BEGIN parses and calls `begin_transaction()` but COMMIT fails because the engine that started the transaction no longer exists. |
| Impact | BEGIN/COMMIT/ROLLBACK all fail. No ACID guarantee. COMMIT returns "No transaction in progress" every time. |
| Severity | 🔴 BLOCKER |
| Fix | Make `MemoryExecutionEngine` persistent per session (connection-level, not query-level), OR store `current_tx_id` and `transaction_manager` outside the engine instance in session/connection state. |

**Code reference:**
```rust
// lib.rs:1092 — new engine each query
let mut eng = MemoryExecutionEngine::new(storage.clone());
// ...
let result = eng.execute(&q);  // engine dropped after this
```

**Root cause**: `ExecutionEngine<S>` has `transaction_manager: TransactionManager` and `current_tx_id: Option<TransactionId>` as instance fields. But the instance is recreated for every query.

---

### 2.2 Authentication Bypass (`SKIP_AUTH=true`)

| Field | Value |
|-------|-------|
| File | `crates/mysql-server/src/lib.rs:22` |
| Issue | `const SKIP_AUTH: bool = true` — all connections accepted without credential verification. Auth flow is completely bypassed in both COM_AUTH and connection acceptance paths. |
| Impact | Any client can connect without valid credentials. Not production-safe. |
| Severity | 🔴 BLOCKER |
| Fix | Set `SKIP_AUTH = false`. Requires fixing empty password auth edge case first. |

**Code reference:**
```rust
// lib.rs:22
const SKIP_AUTH: bool = true;

// lib.rs:1396 (COM_AUTH handler)
let auth_ok = if SKIP_AUTH { true } else { ... verify ... }

// lib.rs:1443 (older auth path)
let auth_ok = if SKIP_AUTH { true } else { ... verify ... }
```

---

### 2.3 VTU Path Exists But Not Used By mysql-server

| Field | Value |
|-------|-------|
| File | `crates/executor/src/local_executor.rs:1065-1108` |
| Issue | VTU (Vectorized Test Update) work from PR #2611 (`PredicateCompiler` + `MutationCompiler` + `update_if`) is wired into `LocalExecutor`. But `mysql-server` uses `ExecutionEngine<MemoryStorage>` (root level `src/execution_engine.rs`), NOT `LocalExecutor`. The VTU path is never exercised by the mysql-server. |
| Impact | PR #2611 VTU investment not delivered to production. UPDATE operations use simple scan-filter-update instead of vectorized path. |
| Severity | 🔴 BLOCKER (investment not delivered) |
| Fix | Either (A) wire VTU into `ExecutionEngine::execute_update()` similar to `LocalExecutor`, OR (B) change mysql-server to use `LocalExecutor` instead of `ExecutionEngine` |

**Code reference — VTU exists but disconnected:**
```rust
// local_executor.rs:1065-1108 — VTU path (wired)
let predicate = PredicateCompiler::compile(where_clause, &table_info)?;
let row_mutation = MutationCompiler::compile_assignments(&update.set_clauses, &table_info)?;
storage.update_if(&predicate, &row_mutation)?;

// mysql-server uses ExecutionEngine, NOT LocalExecutor
// execution_engine.rs:924 — simple scan-filter-update (NOT VTU)
fn execute_update(&self, update: &UpdateStatement) -> SqlResult<ExecutorResult> {
    // ... simple scan-filter-update without VTU
}
```

---

## 3. P1 — High Risk Issues

### 3.1 SHOW Statement Not Supported

| Field | Value |
|-------|-------|
| File | `src/execution_engine.rs` |
| Issue | `Statement::Show` types (`SHOW TABLES`, `SHOW DATABASES`, etc.) are parsed but execution falls through to "Unsupported statement type". `ExecutionEngine::execute()` has no `Statement::Show` match arm. |
| Impact | Client compatibility significantly reduced. `mysql` CLI users expect `SHOW TABLES` to work. |
| Severity | 🟡 HIGH RISK |
| Fix | Add `Statement::Show` match arm to `ExecutionEngine::execute()` with handler methods |

---

### 3.2 AST Parsed But Not Used for Routing in COM_QUERY

| Field | Value |
|-------|-------|
| File | `crates/mysql-server/src/lib.rs:1092-1095` |
| Issue | `parse(&q)` is called, resulting `stmt` is used only for `is_select_stmt()` check. Then `eng.execute(&q)` is called with the **raw SQL string**, which re-parses internally. The parsed AST (`stmt`) is not passed to the execution engine for routing decisions. |
| Impact | Design smell — double parsing overhead. Also means any future AST-based routing (like routing `BEGIN` to TransactionManager) would require re-parsing. |
| Severity | 🟡 HIGH RISK (performance + design) |
| Fix | After `parse(&q)`, match on `stmt` directly for known statement types (Transaction, DML) and call the appropriate method, avoiding re-parsing. |

---

### 3.3 Empty Password Auth Edge Case

| Field | Value |
|-------|-------|
| File | `crates/mysql-server/src/lib.rs:148-165` |
| Issue | When password is empty, `stored = SHA1(SHA1("")) = be1bdec0aa74b4dcb079943e70528096cca985f8`. The `verify_mysql_native_password()` function computes `expected_hash = SHA1(scramble + stored_password_hash)` and XORs with `auth_response`. For empty auth_response `[]`, this never matches because the algorithm requires a non-empty auth_response. The connection test with `mysql -u root -p''` fails. |
| Impact | Users cannot authenticate with empty password, which is a common development setup. |
| Severity | 🟡 HIGH RISK |
| Fix | Add special case: if `auth_response.len() == 0` and `stored_password_hash == be1bdec0aa74b4dcb079943e70528096cca985f8`, accept connection |

---

## 4. P2 — Technical Debt

### 4.1 col_type Enum Mismatch

| Field | Value |
|-------|-------|
| Area | `crates/types/` vs legacy test expectations |
| Issue | `col_type` enum values may have changed between versions. Legacy tests may expect old numeric values. Not confirmed but flagged in historical debt. |
| Severity | 🟢 P2 |

---

### 4.2 Packet API Divergence

| Field | Value |
|-------|-------|
| Area | `mysql-server/tests/mysql_server_tests.rs:63-92` |
| Issue | Test `test_packet_struct` expects `Packet::read_from()` / `Packet::write_to()`. This test compiles in lib but integration tests fail due to `MySqlError: From<String>` not implemented. |
| Severity | 🟢 P2 (test fix needed, not runtime) |

---

### 4.3 old_password_hash Type Mismatch

| Field | Value |
|-------|-------|
| Area | UserStore password verification |
| Issue | `verify_old_password_response()` at lib.rs:1967 exists but may have type/return mismatches vs old test expectations. Not tested in current integration run. |
| Severity | 🟢 P2 |

---

## 5. Legacy Mismatch Matrix

| Test / Expectation | Runtime Behavior | Status |
|--------------------|-----------------|--------|
| BEGIN starts a transaction | BEGIN parses, calls `begin_transaction()`, but state lost next query | ❌ FAIL |
| COMMIT commits the transaction | "No transaction in progress" — engine dropped | ❌ FAIL |
| ROLLBACK rolls back | Same — no persistent state | ❌ FAIL |
| SHOW TABLES returns table list | "Unsupported statement type" | ❌ FAIL |
| mysql CLI connects with `-u root -p''` | Access denied — empty password edge case | ❌ FAIL |
| All 93 lib tests pass | 93/93 ✅ PASS | ✅ OK |
| VTU UPDATE uses vectorized path | mysql-server uses simple path | ❌ FAIL |
| Packet roundtrip (test) | Compiles but `MySqlError: From<String>` not impl | ❌ FAIL |

---

## 6. Architecture Consistency Check

| Component | Status | Notes |
|-----------|--------|-------|
| Parser → AST | ✅ OK | `sqlrustgo-parser` fully working |
| AST → Statement dispatch | ✅ OK | `ExecutionEngine::execute()` routes all statement types |
| Statement::Transaction routing | ✅ OK | `Statement::Transaction` → `execute_transaction()` |
| TransactionManager API | ✅ OK | `begin/commit/rollback()` all implemented |
| TransactionManager wired to mysql-server | ❌ DISCONNECTED | New engine per query, state lost |
| VTU PredicateCompiler | ✅ EXISTS | `crates/executor/src/predicate_compiler.rs` |
| VTU MutationCompiler | ✅ EXISTS | `crates/executor/src/mutation_compiler.rs` |
| VTU used by mysql-server | ❌ NOT USED | mysql-server uses `ExecutionEngine` not `LocalExecutor` |
| AST used for dispatch decision | ⚠️ PARTIAL | Only `is_select_stmt()` uses AST |
| Auth flow | ⚠️ BYPASSED | `SKIP_AUTH=true` |
| COM_STMT_PREPARE | ✅ OK | Placeholder counting + prepare works |
| COM_QUERY error handling | ✅ OK | MySQL error codes returned |

---

## 7. Cross-Check: Tests vs Runtime

| Test Suite | Result | Runtime Match |
|------------|--------|--------------|
| `cargo test --lib -p sqlrustgo-mysql-server` | ✅ 93/93 PASS | Yes |
| `cargo test --lib -p sqlrustgo` | ✅ 12/12 PASS | Yes |
| `cargo test --workspace` | ❌ FAILED | `MySqlError: From<String>` not impl (test file issue) |
| E2E: CREATE TABLE | ✅ PASS | Yes |
| E2E: INSERT | ✅ PASS | Yes |
| E2E: SELECT | ✅ PASS | Yes |
| E2E: UPDATE | ✅ PASS | Yes |
| E2E: DELETE | ✅ PASS | Yes |
| E2E: BEGIN | ⚠️ PARSED | Transaction started but state lost |
| E2E: COMMIT | ❌ FAIL | "No transaction in progress" |
| E2E: SHOW TABLES | ❌ FAIL | "Unsupported statement type" |
| E2E: `mysql -u root -p''` | ❌ FAIL | Access denied |

---

## 8. GA Readiness Score

**Total: 41 / 100**

| Category | Score | Max | Notes |
|----------|-------|-----|-------|
| Execution Core (DDL/DML) | 10 | 10 | All DDL/DML working correctly |
| Protocol Layer | 8 | 10 | COM_QUERY/COM_STMT working; SKIP_AUTH is P0 |
| Transaction System | 2 | 10 | BEGIN parsed, state lost; COMMIT always fails |
| Authentication | 1 | 10 | Completely bypassed via SKIP_AUTH=true |
| SQL Coverage | 7 | 10 | Core SELECT/INSERT/UPDATE/DELETE working; SHOW partial |
| VTU Investment | 3 | 10 | Exists but not used by mysql-server |
| Error Handling | 8 | 10 | MySQL error codes properly returned |
| **TOTAL** | **41** | **80** | **51% — Below 70% threshold** |

---

## 9. Recommendation

### ❌ NOT GA READY

v3.7.0 cannot be released as GA without fixing P0 blockers.

### Required Actions (in priority order):

| Priority | Action | Effort |
|----------|--------|--------|
| P0-1 | Persist transaction state across queries (session-level) | Medium |
| P0-2 | Set `SKIP_AUTH = false` | Low |
| P0-3 | Wire VTU into `ExecutionEngine::execute_update()` | Medium |
| P1-1 | Implement `Statement::Show` dispatch | Low |
| P1-2 | Fix empty password auth edge case | Low |
| P2-1 | Fix `MySqlError: From<String>` test | Low |

### Three Paths Forward:

| Path | Description | Target Score |
|------|-------------|--------------|
| **A: Minimal Fix** | Fix P0-1 (txn state) + P0-2 (SKIP_AUTH) only | 60/80 |
| **B: Full Fix** | Fix all P0 + P1 items | 75+/80 |
| **C: Accept Non-Transactional Scope** | Document v3.7.0 as "Non-Transactional SQL Engine" and release as EA | 41/80 |

**Recommended**: Path A (Minimal Fix) — preserves v3.7 "frozen" intent while fixing critical blockers.

---

## Appendix: Key File References

| File | Line | Relevance |
|------|------|-----------|
| `crates/mysql-server/src/lib.rs` | 22 | SKIP_AUTH constant |
| `crates/mysql-server/src/lib.rs` | 1092-1122 | COM_QUERY dispatch (new engine per query) |
| `src/execution_engine.rs` | 35-93 | ExecutionEngine struct (has TransactionManager) |
| `src/execution_engine.rs` | 336-413 | ExecutionEngine::execute() full Statement dispatch |
| `src/execution_engine.rs` | 1327-1376 | execute_transaction() (Begin/Commit/Rollback) |
| `crates/executor/src/local_executor.rs` | 1065-1108 | VTU UPDATE path (NOT used by mysql-server) |
| `crates/transaction/src/manager.rs` | — | TransactionManager API |
| `docs/releases/v3.7.0/RELEASE_CANDIDATE_GATE.md` | — | RC1 freeze definition |
| `docs/releases/v3.7.0/INTEGRATION_TEST_REPORT.md` | — | E2E test results |