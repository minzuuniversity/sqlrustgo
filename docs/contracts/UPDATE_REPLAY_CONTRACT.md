# UPDATE Replay Contract

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

---

## Implementation Roadmap

| PR | Goal | Gate |
|----|------|------|
| PR-841 | Structured Serialization (`updates_to_bytes`, `bytes_to_filters`) | Roundtrip test |
| PR-842 | UPDATE Replay in Recovery | Crash/restart test |
| PR-843 | Checkpoint Integration | Checkpoint + replay |

---

## Related

- PR #2715: Closed (branch drift, same contract extracted)
- Issue #2733: Workflow tracking
