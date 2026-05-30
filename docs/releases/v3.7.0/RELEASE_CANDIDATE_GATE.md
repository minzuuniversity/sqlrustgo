# v3.7.0 Release Candidate Gate

| Field | Value |
|-------|-------|
| Document | v3.7.0 Release Candidate Gate |
| Status | PROPOSED FREEZE |
| Branch | develop/v3.7.0 |
| Author | Hermes Agent (drafted by user input) |
| Date | 2026-05-30 |

---

## 1. Version Definition

v3.7.0 is defined as:

> **SQLRustGo Core SQL Execution Engine (Non-Transactional Edition)**

### Core Capabilities

- MySQL Wire Protocol Server
- SQL Parser → AST
- LocalExecutor execution pipeline
- StorageEngine CRUD operations
- DDL support (CREATE / DROP TABLE)
- DML support (INSERT / UPDATE / DELETE)
- Prepared Statement framework
- Basic error handling & result encoding

### Explicit Non-Goals

- Transaction consistency (BEGIN/COMMIT/ROLLBACK not functional)
- MVCC / isolation levels
- SHOW / metadata SQL completeness
- Full MySQL compatibility (subset only)
- Distributed / replication

---

## 2. Architecture Boundary

### Current Execution Model

```
MySQL Client
    ↓
COM_QUERY
    ↓
parse(sql) → Statement AST
    ↓
eng.execute(sql)
    ↓
LocalExecutor
    ↓
StorageEngine
```

### Frozen Facts

#### TransactionManager

- Exists in codebase (`crates/transaction/`)
- **NOT connected** to mysql-server
- **NOT part of** execution path
- BEGIN/COMMIT/ROLLBACK parsed but not routed

#### Execution Model

- Stateless executor model
- No session transaction state
- Direct storage execution (no buffering)

#### AST Usage

- Parsed successfully
- **NOT used** for routing decisions in COM_QUERY
- `eng.execute(&q)` receives raw SQL string, not Statement

---

## 3. Known Limitations

### 3.1 Transaction System (CRITICAL)

BEGIN / COMMIT / ROLLBACK:
- Parsed by parser
- NOT routed to TransactionManager
- Results in no-op or "No transaction in progress" error

### 3.2 SHOW Statements

SHOW TABLES / SHOW COLUMNS / etc.:
- Not implemented in executor dispatch
- Returns "Unsupported statement type" error

### 3.3 Protocol Limitations

- Basic OK packet ✅
- Result set encoding ✅
- Limited metadata richness ⚠️

---

## 4. Readiness Assessment

| Category | Status | Notes |
|----------|--------|-------|
| SQL Execution | ✅ Stable | 93/93 tests pass |
| DDL | ✅ Complete | CREATE TABLE works |
| DML | ✅ Correct | INSERT/UPDATE/DELETE verified via mysql CLI |
| Transactions | ❌ Not Wired | TransactionManager disconnected |
| Metadata SQL | ⚠️ Partial | SHOW not supported |
| Protocol | ✅ Usable | MySQL wire protocol compatible |

---

## 5. RC Gate Test Set

The following must pass:

```sql
CREATE TABLE t1(id INT, name TEXT);
INSERT INTO t1 VALUES(1,'alice');
SELECT * FROM t1;
UPDATE t1 SET name='bob' WHERE id=1;
DELETE FROM t1 WHERE id=1;
```

**Expected results:**

- affected_rows correct
- result set correct
- no crash
- storage consistency correct

**Status:** ✅ All verified (2026-05-30 via mysql CLI integration test)

---

## 6. Freeze Rules (Critical)

### From RC1 onward — PROHIBITED:

- No structural changes to LocalExecutor
- No new execution models
- No TransactionManager integration work on this branch
- No planner rewrite

### Allowed changes only:

- Bug fixes
- Protocol fixes
- Test fixes
- Minor performance tuning

---

## 7. Production Readiness Statement

v3.7.0 is:

> A stable single-node SQL execution engine **WITHOUT transactional guarantees.**

Suitable for: read-heavy workloads, in-memory storage, single-node evaluation, protocol compatibility testing.

Not suitable for: multi-statement transactions, data integrity workloads requiring ACID, production systems needing MySQL-compatible transaction semantics.

---

## 8. Risk Summary

| Risk | Impact | Mitigation |
|------|--------|------------|
| No transactions | HIGH — correctness workloads fail | v3.8 Phase 1 |
| Partial metadata | MEDIUM | Document as known limitation |
| No SHOW support | LOW | Document as known limitation |
| No MVCC | EXPECTED | v3.9 candidate |

---

## 9. Release Criteria (Gate Closure)

v3.7.0 is considered **release-ready** if:

- ✅ All DDL/DML tests pass (confirmed: 93/93 lib tests)
- ✅ MySQL client integration validated (CREATE/INSERT/SELECT/UPDATE/DELETE verified)
- ✅ No crash under basic workload
- ✅ No regression in protocol layer

---

## 10. Explicit Non-Regression Rule

v3.7.0 must NOT evolve into:

- ❌ Transactional engine
- ❌ Planner-driven execution system
- ❌ MVCC system

These capabilities explicitly belong to v3.8+.

---

## 11. Next Version Boundary

v3.8.0 is explicitly defined as:

```
Transaction-aware execution system
+ COM_QUERY dispatch rewrite
+ TransactionManager integration
+ WriteBuffer introduction
+ Phase 1: BEGIN/COMMIT/ROLLBACK routing
+ Phase 2: WriteBuffer staging
+ Phase 3: Commit engine
+ Phase 4: Rollback engine
+ Phase 5: Read consistency (snapshot)
```

---

## 12. Document History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-30 | Initial draft — proposed RC freeze | Hermes Agent |

---

## 13. Sign-Off

> **v3.7.0 = Stable SQL execution engine (non-transactional)**
>
> **v3.8.0 = Transactional architecture evolution layer**
>
> **Freeze boundary set. No rollback permitted without version bump.**