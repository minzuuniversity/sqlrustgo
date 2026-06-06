# UPDATE Replay Contract

> **Last updated**: 2026-06-06
> **Status**: Active — VTU (Visionary Transaction Unit) integrated
> **Tracking issue**: #3236

## P0 Correctness Requirement

> UPDATE replay must survive restart. This is not a feature - it is an ACID correctness requirement.

---

## Given/When/Then

### Scenario: UPDATE survives crash

**Given:**
```sql
CREATE TABLE t (id INTEGER PRIMARY KEY, value TEXT);
INSERT INTO t VALUES (1, 'original');
```

**When:**
```sql
UPDATE t SET value = 'updated' WHERE id = 1;
```
(followed by crash/restart)

**Then:**
```sql
SELECT value FROM t WHERE id = 1;
```
**Returns:** `'updated'`

---

## Architecture: VTU (Visionary Transaction Unit) — Current Path

As of 2026-06, all UPDATE operations MUST go through the VTU layer:

```
Parser AST (UpdateStatement)
       │
       ▼  AstAdapter::to_update_plan()
VTU IR (UpdatePlan: PredicateIR + MutationIR + AssignmentIR + ExprIR)
       │
       ▼  UpdateCompiler::compile()
VtuGuard::execute_dml(closure)     ← single chokepoint
       │
       ▼
StorageEngine::update()            ← unified UPDATE entry
       │
       ├── WalStorage::update()    ← WAL record (structured)
       ├── FileStorage::update()   ← persistent
       ├── ColumnarStorage::update()
       └── MemoryStorage::update()
```

### Why VTU?

- **Single chokepoint**: `VtuGuard::execute_dml` enforces all DML passes through TransactionManager (INT-4, #2973)
- **Type-safe IR**: `UpdatePlan` is a serializable intermediate representation
- **Panic on direct calls**: bypassing VtuGuard panics with `VTU VIOLATION`

### Key Components

| Component | Path | Purpose |
|-----------|------|---------|
| VtuGuard wrapper | `crates/storage/src/vtu_guard.rs` | DML chokepoint |
| UpdatePlan IR | `crates/storage/src/vtu_ir.rs` | Serializable UPDATE plan |
| AstAdapter | `crates/executor/src/ast_adapter.rs` | Parser AST → VTU IR |
| UpdateCompiler | `crates/executor/src/update_compiler.rs` | Compile UpdatePlan to RowMutation |
| vtu_ir_test | `crates/storage/tests/vtu_ir_test.rs` | Unit tests |
| vtu_ir_pipeline_test | `crates/storage/tests/vtu_ir_pipeline_test.rs` | E2E tests |

---

## WAL UPDATE Record Contract

### Structure

A WAL UPDATE entry MUST contain:

| Field | Type | Description |
|-------|------|-------------|
| `table_id` | `u64` | Hash of table name |
| `tx_id` | `u64` | Transaction ID |
| `key` | `Vec<u8>` | Primary key filter values (binary serialized) |
| `data` | `Vec<u8>` | Update pairs: `(col_idx, value)` serialized |

### Serialization Format

```
filters_to_bytes(filters: &[Value]) -> Vec<u8>
  └── Uses record_to_bytes (same as INSERT/DELETE)

updates_to_bytes(updates: &[(usize, Value)]) -> Vec<u8>
  ├── 4 bytes LE: number of update pairs (u32)
  └── Per pair:
      ├── 4 bytes LE: column index (u32)
      └── binary: Value serialized via record_to_bytes
```

### Deserialization (Replay)

```
bytes_to_filters(bytes: &[u8]) -> Vec<Value>
bytes_to_updates(bytes: &[u8]) -> Vec<(usize, Value)>
```

**Contract:** `bytes_to_X` must be inverse of `X_to_bytes`:
```rust
let original = updates.clone();
let bytes = updates_to_bytes(&original);
let recovered = bytes_to_updates(&bytes);
assert_eq!(original, recovered);
```

---

## Known Limitations

### `update_if` with RowFilter (closure)

**Status:** Cannot fully recover - requires expression tree AST (not closures).

**Reason:** `RowFilter` stores `Box<dyn Fn(&Row) -> bool>` which cannot be serialized.

**Impact:** Only unconditional UPDATEs survive crash.

**Workaround:** Use trigger-based conditional updates instead.

### Two execution paths (legacy coexistence) — to be unified

Two UPDATE execution paths currently coexist:

1. **`local_executor.rs:1371`** (new, VTU-compliant via `UpdateExec` plan node)
2. **`execution_engine.rs:527`** (legacy, DELETE+INSERT + PR-842 Option A)

The legacy path is still the default entry for `sqlrustgo-mysql-server` (main bin).
Target: migrate legacy to VTU (Phase B2 of #3236), then deprecate legacy entirely (Phase B4).

### Discovered bugs (2026-06-06) — Tracked in #3236

| Bug | Symptom | Test | Status |
|-----|---------|------|--------|
| UPDATE without WHERE doesn't apply | `UPDATE t SET value = 0` preserves original values | `cross_path_update_no_where_consistency` | `#[ignore]`, pending Phase B2 fix |
| UPDATE self-reference uses first row only | `UPDATE t SET value = value + 100` sets all rows to first row's value + 100 | `cross_path_update_self_reference_consistency` | `#[ignore]`, pending Phase B2 fix |

These bugs affect the **legacy execution_engine.rs path**. The VTU path (`local_executor.rs`) is expected to handle these correctly post-migration.

---

## Implementation Roadmap

| PR/Issue | Goal | Status |
|----------|------|--------|
| PR-2755 / #2741 | PR-842 UPDATE replay (early) | ✅ Merged |
| PR-2758 / SPEC-003 | Update WAL replay encoding | ✅ Merged |
| PR-2761 / F-09 | F-09 UPDATE replay recovery | ✅ Merged |
| 5ed7a1d75 / #3129 | ARCH-3 VTU main path (MemoryStorage TX + autocommit) | ✅ Merged |
| c919ca024 | VtuGuard wrapper | ✅ Merged |
| 131f466bf / #2973 | INT-4 VtuGuard enforcement on trigger DML | ✅ Merged |
| #3236 Phase B1 | Cross-path UPDATE consistency tests | ✅ Done (2026-06-06, commit 060bc38ed) |
| **#3236 Phase B2** | **Migrate execution_engine.rs:527 to VTU path** | ⏳ Next |
| #3236 Phase B3 | UPDATE_REPLAY_CONTRACT rewrite (this doc) | ✅ Done (2026-06-06) |
| #3236 Phase B4 | Deprecate execution_engine.rs legacy path | ⏳ After B2 |

---

## Cross-path Consistency Tests

`tests/cross_path_consistency_test.rs` validates UPDATE consistency across three storage backends:

1. **L1**: `MemoryStorage` (in-memory)
2. **L2**: `WalStorage<MemoryStorage, MemoryWalManager>` (WAL stub)
3. **L3**: `WalStorage<FileStorage, FileBackedWalManager>` (production, wire protocol)

Test scenarios (as of 2026-06-06):

| Scenario | Status |
|----------|--------|
| Single SET with WHERE | ✅ pass |
| Multi-column SET | ✅ pass |
| WHERE no match | ✅ pass |
| WHERE matches multiple rows | ✅ pass |
| **No WHERE (full table)** | 🐛 ignored (bug, see Known Limitations) |
| **Self-reference** | 🐛 ignored (bug, see Known Limitations) |
| DELETE | ✅ pass |
| Error handling | ✅ pass |

---

## Related

- **PR #3239**: Closed as superseded (PR-841+PR-842 merged earlier via PR-2755/2758/2761)
- **Issue #3236**: Active tracking of UPDATE path refactor
- **Issue #3235**: Storage visibility bug (closed after SPEC-003 + F-09)
- **Issue #2741**: Original architectural debt (closed)
- **Issue #2973**: INT-4 VtuGuard enforcement (closed)
- **Issue #3129**: ARCH-3 VTU main path (closed)
- **SPEC-002 / SPEC-003**: Storage WAL refactor (merged)
- **PR-2755, PR-2758, PR-2761**: VTU core implementation PRs
