# v3.8.0 Write Path Verification Report

**Generated**: 2026-05-31
**Status**: Architecture Complete, Behavior PARTIALLY VERIFIED
**Branch**: develop/v3.8.0 @ aa0bc757

---

## 1. Executive Summary

| Domain | Status | Notes |
|--------|--------|-------|
| Execution (P1) | ✅ Verified | WalStorage enforced via local_executor |
| HTTP Endpoint (P2) | ⚠️ GAP | Direct storage access, no VTU enforcement |
| Trigger (P3) | ⚠️ GAP | Triggers fire but endpoint bypasses ExecutionEngine |
| Stored Procedure (P4) | ⚠️ GAP | CALL handled separately, no WAL guarantee |
| WAL Persistence | ❌ Unverified | crash consistency not tested |
| TX Enforcement | ⚠️ Partial | Only ExecutionEngine path has TX gate |

---

## 2. Runtime Storage Topology

### Where Storage is Created

| Module | Storage Type | WAL Enabled | Risk |
|--------|-------------|-------------|------|
| `connection_pool.rs:39,76` | `MemoryStorage::new()` | ❌ No | HIGH - pool sessions bypass WAL |
| `openclaw_endpoints.rs` | Injected via constructor | ❓ Unknown | MEDIUM - depends on caller |
| `mysql-server/lib.rs:1530` | `MemoryStorage::new()` | ❌ No | HIGH - mysql server bypasses WAL |
| `gmp/sql_api.rs:250,274,304` | `MemoryStorage::new()` | ❌ No | MEDIUM - GMP internal |
| `local_executor.rs:53` | `WalStorage::new()` | ✅ Yes | LOW - proper path |
| `trigger.rs:101` | Assert check only | ⚠️ Assert only | MEDIUM - runtime only |

### Critical Finding

```text
OpenClawHttpServer::new() has WAL assert (line 463)
BUT the actual write path (execute_sql) goes directly:
  storage.write().map_err(...) -> storage.insert()/update()/delete()

The storage passed to OpenClawHttpServer is caller-controlled.
If caller passes MemoryStorage → no WAL, no assertion until runtime write.
```

---

## 3. Write Path Coverage Matrix

| Path | Code Location | Direct Storage | Via ExecutionEngine | WAL Enforced |
|------|--------------|----------------|---------------------|--------------|
| P1: LocalExecutor | `local_executor.rs:53` | ❌ No | ✅ Yes | ✅ Yes |
| P2: HTTP Endpoint Insert | `openclaw_endpoints.rs:2110` | ✅ Yes | ❌ No | ❌ No guarantee |
| P3: HTTP Endpoint Update | `openclaw_endpoints.rs:2263` | ✅ Yes | ❌ No | ❌ No guarantee |
| P4: HTTP Endpoint Delete | `openclaw_endpoints.rs:2203` | ✅ Yes | ❌ No | ❌ No guarantee |
| P5: Trigger in Endpoint | `openclaw_endpoints.rs:2108` | ✅ Yes | ❌ No | ❌ No guarantee |
| P6: StoredProc CALL | `openclaw_endpoints.rs:2291` | N/A | ❌ No | ❌ No guarantee |

---

## 4. WAL Enforcement Escape Analysis

### Direct Storage Write Patterns (Non-Test)

```
crates/server/src/openclaw_endpoints.rs:
  2203: let _ = storage.delete(&delete.table, &[]);
  2263: let _ = storage.delete(&update.table, &[]);
  2110: storage.insert(table_name, records)
  2264: storage.insert(&update.table, final_rows)

crates/server/src/teaching_endpoints.rs:
  660: let _ = storage.delete(&delete.table, &[]);
  718: let _ = storage.delete(&update.table, &[]);

crates/mysql-server/src/lib.rs:
  (all writes go through MemoryStorage)
```

### MemoryStorage Injection Points (Outside Tests)

| File | Line | Risk |
|------|------|------|
| `connection_pool.rs` | 39, 76 | HIGH - pooled sessions use MemoryStorage |
| `mysql-server/lib.rs` | 1530 | HIGH - production MySQL server |
| `gmp/sql_api.rs` | 250, 274, 304 | MEDIUM - internal tools |
| `gmp/audit.rs` | 514, 521, 548, 570, 633 | MEDIUM - audit subsystem |
| `gmp/document.rs` | 542, 551 | MEDIUM - document storage |
| `gmp/vector_search.rs` | 288, 358 | MEDIUM - vector index |

---

## 5. TX Enforcement Analysis

### TX Enforcement Points

| Component | TX Gate | Implementation |
|-----------|---------|---------------|
| `ExecutionEngine::execute()` | ✅ Yes | Checks active transaction |
| `local_executor_dml` | ✅ Yes | VTU path through ExecutionEngine |
| `MergeExecutor` | ✅ Yes | VTU path through ExecutionEngine |
| `openclaw_endpoints` INSERT/UPDATE/DELETE | ❌ No | Direct storage, no TX check |
| `trigger.rs` | ✅ Assert | Checks is_wal_enabled at construction |
| `stored_proc.rs` | ✅ Assert | Checks is_wal_enabled at construction |

---

## 6. Gap Summary

### 🔴 CRITICAL GAPS

1. **G-1: HTTP Endpoint bypasses VTU path**
   - All INSERT/UPDATE/DELETE go directly to `storage.insert()/update()/delete()`
   - No ExecutionEngine routing
   - No TX enforcement
   - WAL depends on caller-provided storage

2. **G-2: MemoryStorage in MySQL Server**
   - `mysql-server/lib.rs:1530` creates `MemoryStorage::new()`
   - All MySQL protocol writes bypass WAL entirely
   - No assert in MySQL server startup

3. **G-3: ConnectionPool uses MemoryStorage**
   - Pooled sessions created with `MemoryStorage::new()`
   - WAL enforcement completely absent in pooled path

### 🟡 MEDIUM GAPS

4. **G-4: Trigger execution via endpoint**
   - Triggers fire but execution happens in endpoint context
   - Nested trigger → direct storage write → no WAL guarantee

5. **G-5: Stored Procedure CALL handler**
   - Handles CALL but no VTU routing
   - No TX context propagation

---

## 7. Recommendations

### Immediate (Must Fix for v3.8.0 GA)

- [ ] **R-1**: Add `is_wal_enabled()` assert to all HTTP write paths
- [ ] **R-2**: Make `openclaw_endpoints.rs` require WalStorage via trait bound
- [ ] **R-3**: Remove or wrap MemoryStorage in connection_pool

### Validation Tests Needed

- [ ] **V-1**: Verify `OpenClawHttpServer` panics when given MemoryStorage
- [ ] **V-2**: Verify MySQL server with MemoryStorage → panic
- [ ] **V-3**: Verify pooled session with MemoryStorage → panic
- [ ] **V-4**: WAL replay consistency test (crash mid-write)

---

## 8. Behavioral Verification Status

| Test | Status | Notes |
|------|--------|-------|
| TX blocks writes without active tx | ⚠️ Partial | Only ExecutionEngine path |
| WAL logs all writes | ❌ Unverified | No comprehensive test |
| MemoryStorage → panic | ❌ Unverified | Only constructor assert |
| Trigger recursion safety | ❌ Unverified | |
| Stored procedure multi-step | ❌ Unverified | |
| Crash recovery | ❌ Unverified | |

---

## 9. Conclusion

> **System is NOT production-ready for WAL enforcement**
> All write paths have structural VTU enforcement ONLY in the local_executor path.
> HTTP endpoint, MySQL server, and connection pool all bypass VTU and use MemoryStorage directly.
> This is a CRITICAL architectural gap.

**Required Actions**:
1. Fix HTTP endpoint to route through ExecutionEngine (or enforce WalStorage at construction)
2. Fix MySQL server to use WalStorage
3. Fix connection pool to use WalStorage or remove pooled sessions
4. Add comprehensive WAL verification tests
5. Add crash consistency tests

---

*Generated by Hermes verification analysis*