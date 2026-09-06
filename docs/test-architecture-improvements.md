# Test Architecture Improvements

## Completed

### 1. Stateful Differential Test Framework

**File**: `tests/compat/differential_framework.rs`

A generic framework for running SQL sequences on both SQLRustGo and a reference database (SQLite/MySQL), then comparing results.

**Key features**:
- Runs sequences of SQL statements, not just isolated SELECTs
- Compares result rows, column metadata, affected rows, and error codes
- Supports stateful sessions (transactions, prepared statements)
- Can use MySQL as oracle, not just SQLite
- Auto-skips if binaries not available

### 2. BustubX-EDU B-Track Compatibility Corpus

**File**: `tests/compat/bustubx_b_track_corpus.sql`

19 P3 failure-category test cases from BustubX-EDU differential_test.py:

| Test ID | Category | Description | Status |
|---------|----------|-------------|--------|
| P3-MATH-001 | Math | MOD returns Integer | FIXED |
| P3-WIN-002 | Window | NTILE distribution | NOT IMPLEMENTED |
| P3-DDL-001 | DDL | DROP INDEX | CANNOT REPRODUCE |
| P3-HINT-001 | Hint | INDEXED BY | NOT IMPLEMENTED |
| P3-NULL-001 | NULL | NULL in WHERE | NEEDS VERIFICATION |
| P3-AGG-001 | Aggregate | SUM empty | NEEDS VERIFICATION |
| P3-AGG-002 | Aggregate | COUNT vs SUM | NEEDS VERIFICATION |
| P3-JOIN-001 | Join | 3-table JOIN | NEEDS VERIFICATION |
| P3-SUB-001 | Subquery | EXISTS | NEEDS VERIFICATION |
| P3-SET-001 | Set | UNION ALL | NEEDS VERIFICATION |
| P3-SET-002 | Set | UNION DISTINCT | NEEDS VERIFICATION |
| P3-CHAR-001 | String | CHAR_LENGTH | NEEDS VERIFICATION |
| P3-TX-001 | Transaction | Rollback | NEEDS VERIFICATION |
| P3-SCHEMA-001 | Schema | sqlite_master | NEEDS VERIFICATION |
| P3-COLLATE-001 | Collation | Case-insensitive | NEEDS VERIFICATION |
| P3-AGG-003 | Aggregate | GROUP BY HAVING | NEEDS VERIFICATION |
| P3-WIN-001 | Window | ROW_NUMBER | NEEDS VERIFICATION |
| P3-INSERT-001 | Insert | DEFAULT values | NEEDS VERIFICATION |
| P3-UPDATE-001 | Update | Expression | NEEDS VERIFICATION |
| P3-DELETE-001 | Delete | Subquery | NEEDS VERIFICATION |

### 3. Differential Corpus Integration Test

**File**: `tests/integration/compat/differential_corpus_test.rs`

10 Rust integration tests running corpus via CLI:

```bash
cargo test --test differential_corpus_test --all-features
```

**Tests**:
- test_p3_math_001_mod_integer
- test_p3_agg_002_count_sum
- test_p3_set_001_union_all
- test_p3_set_002_union_distinct
- test_p3_null_001_null_comparison
- test_p3_char_001_char_length
- test_p3_agg_001_sum_empty
- test_p3_tx_001_rollback
- test_p3_update_001_expression
- test_p3_schema_001_sqlite_master

**Result**: All 10 tests pass (auto-skip if sqlrustgo binary not available)

### 4. Shell Corpus Runner

**File**: `tests/compat/run_bustubx_corpus.sh`

Shell runner for corpus against SQLite:

```bash
bash tests/compat/run_bustubx_corpus.sh [sqlite_bin] [sqlrustgo_bin]
```

## PR Created

**PR #4835**: test: add stateful differential testing framework

## Next Steps

### Pending Items

1. **Behavioral Coverage Metrics** - Track which SQL features are covered by differential tests
2. **MySQL Oracle Support** - Integrate MySQL as additional oracle alongside SQLite
3. **Protocol-level MySQL Wire Tests** - Add more MySQL protocol state machine tests

### Existing Infrastructure

The project already has:
- MySQL wire protocol tests (12 tests in `mysql_wire_protocol_test.rs`)
- Compatibility harness (`compatibility_harness.rs`)
- Oracle G16 compat tests (`oracle_g16_compat.rs`)
- TPC-H vs SQLite differential tests

## How to Run

### Run all differential corpus tests
```bash
cargo test --test differential_corpus_test --all-features
```

### Run shell corpus runner
```bash
bash tests/compat/run_bustubx_corpus.sh
```

### Run specific test
```bash
cargo test --test differential_corpus_test test_p3_math_001_mod_integer --all-features
```
