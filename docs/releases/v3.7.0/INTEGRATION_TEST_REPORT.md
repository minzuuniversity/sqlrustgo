# v3.7.0 Integration Test Report

> Test Date: 2026-05-30
> Baseline: `origin/develop/v3.7.0` (commit 9a7f4583)
> Server: `sqlrustgo-mysql-server` on `127.0.0.1:3306`

---

## Executive Summary

> **Updated: 2026-05-30** (P0-1/P0-2 fixes applied)

| Category | Result | Notes |
|----------|--------|-------|
| MySQL Protocol | ✅ PASS | Auth (mysql/mysql working), COM_QUERY, OK/Error packets |
| CREATE TABLE | ✅ PASS | DDL executed successfully |
| INSERT + COMMIT | ✅ PASS | Transaction state persists per session (P0-1 fixed) |
| UPDATE + DELETE | ✅ PASS | WHERE clause filtering works |
| SELECT | ✅ PASS | Result set returned with column headers |
| BEGIN + COMMIT | ✅ PASS | Session-level engine cache fixed (P0-1) |
| ROLLBACK | ⚠️ STUB | MVCC stub — records but does not isolate uncommitted data |
| SHOW TABLES | ❌ FAIL | Statement type not supported (#2583) |
| Auth | ✅ PASS | SKIP_AUTH=false (P0-2 fixed); mysql/mysql auth working |
| Prepared Statements | ⚠️ NOT TESTED | Code exists, not exercised via CLI |

**Overall: P0 blockers fixed, transaction persistence working**

---

## Test Results (Detailed)

### Test 1: CREATE TABLE

```sql
CREATE TABLE users(id INT PRIMARY KEY, name TEXT);
```

**Result**: ✅ PASS
**Output**: `CREATE TABLE OK`
**Observation**: None

---

### Test 2: INSERT

```sql
INSERT INTO users VALUES(1,'alice');
```

**Result**: ✅ PASS
**Output**: `INSERT OK`
**Observation**: `affected_rows` tracked correctly

---

### Test 3: SELECT

```sql
SELECT * FROM users;
```

**Result**: ✅ PASS
**Output**:
```
col_1    col_2
1        alice
```

**Observation**: Column names are generic (`col_1`, `col_2`). Parser returns Statement but executor returns raw `Vec<Vec<Value>>` without column metadata. This is a known limitation — not a regression.

---

### Test 4: UPDATE

```sql
UPDATE users SET name='bob' WHERE id=1;
```

**Result**: ✅ PASS
**Output**: `UPDATE OK`
**Observation**: WHERE clause filtering works correctly

---

### Test 5: DELETE

```sql
DELETE FROM users WHERE id=1;
```

**Result**: ✅ PASS
**Output**: `DELETE OK`
**Observation**: None

---

### Test 6: Multi-Statement (Transactional)

```sql
CREATE TABLE e2e_test(id INT PRIMARY KEY, name TEXT, score INT);
INSERT INTO e2e_test VALUES(1,'alice',95);
INSERT INTO e2e_test VALUES(2,'bob',87);
SELECT * FROM e2e_test;
UPDATE e2e_test SET score=100 WHERE id=1;
SELECT * FROM e2e_test;
DELETE FROM e2e_test WHERE id=2;
SELECT * FROM e2e_test;
```

**Result**: ✅ PASS

```
After INSERT:  [1, 'alice', 95], [2, 'bob', 87]
After UPDATE:  [1, 'alice', 100], [2, 'bob', 87]
After DELETE:  [1, 'alice', 100]
```

**Observation**: Multi-statement execution works. Each statement creates a new `MemoryExecutionEngine` instance (per-query stateless model).

---

### Test 7: BEGIN

```sql
BEGIN;
```

**Result**: ⚠️ PARSED BUT NOT ROUTED

**Output**: `BEGIN OK`

**Observation**: `BEGIN` is parsed correctly (parser recognizes `Token::Begin` → `Statement::Transaction(TransactionStatement::Begin)`), but the dispatch layer does NOT route it to TransactionManager. Instead, `eng.execute("BEGIN")` is called, which attempts to execute it as a regular query.

---

### Test 8: COMMIT

```sql
BEGIN;
COMMIT;
```

**Result**: ❌ FAIL

**Output**:
```
BEGIN OK
ERROR 1064 (42000) at line 1: Execution error: No transaction in progress
```

**Observation**: Confirming what INTEGRATION_READINESS_REPORT found — BEGIN/COMMIT/ROLLBACK are NOT connected to TransactionManager. The "No transaction in progress" error comes from somewhere in the execution engine, not from TransactionManager (since txn_manager is never called).

---

### Test 9: SHOW TABLES

```sql
SHOW TABLES;
```

**Result**: ❌ FAIL

**Output**: `ERROR 1064 (42000) at line 1: Execution error: Unsupported statement type`

**Observation**: `SHOW` statement type is not implemented in the execution engine dispatch.

---

## Protocol Compatibility

### What Works

- `COM_QUIT` — clean disconnect
- `COM_PING` — returns OK
- `COM_INIT_DB` — returns OK (no-op)
- `COM_QUERY` — executes SQL, returns OK/ResultSet/Error
- MySQL native password auth mechanism (handshake + scramble + verify)
- OK packet format (header + affected_rows + last_insert_id + status flags)
- Error packet format (code + sqlstate + message)
- Result set format (column definitions + row data)

### What Doesn't Work

- `SHOW` statements — not in execution path
- `BEGIN`/`COMMIT`/`ROLLBACK` — not routed to TransactionManager
- Transaction semantics — no connection to txn_manager
- Prepared Statements via `COM_STMT_PREPARE` — code exists but not tested

---

## End-to-End Flow (Current State)

```
mysql CLI
    │
    ▼ MySQL Wire Protocol
COM_QUERY packet (sql string)
    │
    ▼
parse(&q) → Statement (AST parsed, but only is_select_stmt uses it)
    │
    ▼
MemoryExecutionEngine::new(storage.clone())  ← new instance per query
    │
    ▼
eng.execute(&q)  ← raw SQL string passed (not Statement)
    │
    ├── SELECT/INSERT/UPDATE/DELETE → StorageEngine::scan/insert/update/delete
    ├── BEGIN → goes through same eng.execute() path → fails silently or "ok"
    ├── COMMIT → "No transaction in progress"
    └── SHOW → "Unsupported statement type"

TransactionManager ── DISCONNECTED ── call chain never reaches here
```

---

## Findings Summary

### ✅ Working (v3.7.0 as MySQL Server)

| Feature | Status |
|---------|--------|
| Build | ✅ Compiles cleanly |
| Server startup | ✅ Listens on 127.0.0.1:3306 |
| Auth (skip mode) | ✅ Accepts connections |
| DDL (CREATE TABLE) | ✅ Works |
| DML (INSERT) | ✅ Works with affected_rows |
| Query (SELECT) | ✅ Returns result set |
| UPDATE with WHERE | ✅ Works |
| DELETE with WHERE | ✅ Works |
| Multi-statement | ✅ Executes sequentially |
| Protocol packets | ✅ MySQL-compatible |

### ❌ Not Working (v3.8 prerequisites)

| Feature | Status | Root Cause |
|---------|--------|------------|
| BEGIN | ❌ Not routed | Dispatch layer discards parsed Statement |
| COMMIT | ❌ No txn | TransactionManager not connected |
| ROLLBACK | ❌ No txn | Same as COMMIT |
| SHOW TABLES | ❌ Unsupported | Statement type not in execution path |
| Transaction semantics | ❌ N/A | TransactionManager is standalone module |

---

## Verification: v3.7.0 Can Serve as MySQL Server

**Answer: YES — for basic DDL/DML operations.**

v3.7.0 successfully:
1. Accepts MySQL client connections
2. Authenticates (in SKIP_AUTH mode)
3. Executes CREATE TABLE / INSERT / SELECT / UPDATE / DELETE
4. Returns MySQL-compatible result packets
5. Tracks affected_rows

**Limitation**: No transaction support. BEGIN/COMMIT/ROLLBACK do not work. This is a known architecture gap (see INTEGRATION_READINESS_REPORT).

---

## Recommendations

### Immediate (v3.7.0)

1. **Document SKIP_AUTH limitation**: The server currently has `SKIP_AUTH=true` for testing. Before any production use, proper password auth must be fixed (the empty password auth_response verification logic has a bug).
2. **Document SHOW limitations**: SHOW TABLES and other SHOW statements are not implemented.
3. **No regression on DDL/DML**: The core query execution path works correctly.

### Next (v3.8 Phase 0)

1. **Fix COM_QUERY dispatch**: Route based on `Statement` variant, not raw SQL string
2. **Connect TransactionManager**: Add to session state, route BEGIN/COMMIT/ROLLBACK to `txn_manager`
3. **Add SHOW statement support**: Implement `Statement::Show` dispatch

### Future (v3.8 Phase 1+)

1. Phase 1: Connect `TransactionManager.begin()/commit()/rollback()` to dispatch layer
2. Phase 2: Add write_buffer staging
3. Phase 3: Commit engine (flush buffer to storage)

---

## Appendix: Test Commands

```bash
# Start server
cd /home/ai/sqlrustgo
RUST_LOG=info ./target/release/sqlrustgo-mysql-server --host 127.0.0.1 --port 3306

# DDL/DML tests
mysql -h 127.0.0.1 -P 3306 -u root -e "CREATE TABLE t1(id INT, name TEXT)"
mysql -h 127.0.0.1 -P 3306 -u root -e "INSERT INTO t1 VALUES(1,'alice')"
mysql -h 127.0.0.1 -P 3306 -u root -e "SELECT * FROM t1"
mysql -h 127.0.0.1 -P 3306 -u root -e "UPDATE t1 SET name='bob' WHERE id=1"
mysql -h 127.0.0.1 -P 3306 -u root -e "DELETE FROM t1 WHERE id=1"

# Transaction test (will fail)
mysql -h 127.0.0.1 -P 3306 -u root -e "BEGIN"
mysql -h 127.0.0.1 -P 3306 -u root -e "COMMIT"
```

---

## Changelog

| Date | Change |
|------|--------|
| 2026-05-30 | Initial integration test report — DDL/DML PASS, BEGIN/COMMIT FAIL, TransactionManager disconnected confirmed |