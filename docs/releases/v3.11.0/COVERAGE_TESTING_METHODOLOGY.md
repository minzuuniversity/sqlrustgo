# V311 Coverage Testing Methodology

**Date**: 2026-07-15
**Tool**: `cargo-llvm-cov` v0.8.5
**Target**: ≥85% lines per crate (SEM-4), ≥80% GA gate
**Scope**: All 37 workspace members + root `tests/` integration suite

---

## 1. Measurement Commands

### Per-Crate (Primary Method)
```bash
# Each crate independently — avoids workspace test build failures
cargo llvm-cov test --no-clean --ignore-run-fail -p <crate> --all-features

# With coverage delta
rm -rf target/llvm-cov
BEFORE=$(cargo llvm-cov test -p <crate> 2>/dev/null | grep "^TOTAL" | awk '{print $10}')
# ... add tests ...
AFTER=$(cargo llvm-cov test -p <crate> 2>/dev/null | grep "^TOTAL" | awk '{print $10}')
```

**Critical flags**:
- `--no-clean`: Reuses prior coverage data — required for cumulative measurement
- `--ignore-run-fail`: Returns coverage data even when tests fail (important for pre-existing failures)
- `--all-features`: Ensures all code paths are compiled

### Batch Measurement (All Crates)
```bash
rm -rf target/llvm-cov
for crate in $(cargo metadata --format-version 1 --no-deps 2>/dev/null | \
  jq -r '.workspace_members[]' | grep "sqlrustgo" | sed 's/.*sqlrustgo-//' | sed 's/@.*//'); do
  cargo llvm-cov test --no-clean --ignore-run-fail -p "sqlrustgo-$crate" 2>/dev/null | \
    grep "^TOTAL" | awk -v c="$crate" '{print c "|" $10 "|" $5}'
done
```

### Workspace Integration Tests
```bash
# Only when root tests build cleanly (currently blocked by pre-existing errors)
cargo llvm-cov test --no-clean --ignore-run-fail --workspace
```

---

## 2. Coverage Snapshot (2026-07-15 FINAL)

**Overall workspace: 79.00% (84,613 / 106,464 lines)**

### ≥85% SEM-4 Target (13/21 packages) ✅

| Package | Line % | Missed | Status |
|---------|--------|-------:|:------:|
| sqlrustgo-network | 100.00% | 0 | ✅ |
| sqlrustgo-cache | 99.47% | 2 | ✅ |
| sqlrustgo-wal-verification | 97.20% | 28 | ✅ |
| sqlrustgo-telemetry | 96.67% | 24 | ✅ |
| sqlrustgo-types | 92.57% | 92 | ✅ |
| sqlrustgo-common | 89.17% | 230 | ✅ |
| sqlrustgo-optimizer | 88.23% | 644 | ✅ |
| sqlrustgo-planner | 87.20% | 325 | ✅ |
| sqlrustgo-transaction | 85.41% | 590 | ✅ |
| sqlrustgo-storage | 85.92% | 3442 | ✅ |
| sqlrustgo-security | 85.15% | 438 | ✅ |
| sqlrustgo-catalog | 85.19% | 897 | ✅ |
| sqlrustgo-server | 85.00% | 286 | ✅ |

### 80–84% (1 package — GA gate OK, SEM-4 gap)

| Package | Line % | Missed | GA | Gap to 85% |
|---------|--------|-------:|:---:|----------:|
| sqlrustgo-executor | 82.15% | 4109 | ✅ | ~655 lines |

### <80% (7 packages — structural blockers)

| Package | Line % | Missed | Blocker |
|---------|--------|-------:|---------|
| sqlrustgo-sql-corpus | 75.16% | 377 | Rust `#[cfg(test)]` cannot nest inside trait impl |
| sqlrustgo-parser | 75.56% | 3953 | Pre-existing compile failure: `test_parse_create_procedure_inout_params` |
| sqlrustgo-admin | 65.41% | 823 | `wire_client.rs` needs live MySQL connection |
| sqlrustgo-mysql-server | 40.28% | 3540 | `do_command_loop` needs live MySQL client |
| sqlrustgo-tools | 59.94% | 1108 | `upgrade.rs` (799 lines) API complexity |
| sqlrustgo-mysql-client | 31.56% | 619 | Protocol-level code needs integration tests |
| sqlrustgo-cli | 0.00% | 318 | Binary smoke tests don't instrument |

### Build Errors (Not Measured)

| Package | Error | Status |
|---------|-------|--------|
| `sqlrustgo` (root) | 33 files missing `compression: None` in `TableInfo`; `Value::Point` exhaustive match | Partially fixed in PR #3555, residual errors remain |
| `sqlrustgo-gis` | Missing `Value::Point` match arms | Pre-existing F-03 GIS issue |

---

## 3. Integration / E2E Test Guidelines

### `start_ephemeral` Pattern
Use `start_ephemeral()` from `sqlrustgo::test_utils` for integration tests:
```rust
#[test]
fn test_integration_backup_restore() {
    let (db, tmp) = sqlrustgo::test_utils::start_ephemeral();
    // db is a running in-memory database
    db.execute("CREATE TABLE t (id INT)").unwrap();
    db.execute("INSERT INTO t VALUES (1)").unwrap();
    // ...
}
```

### Adding Tests to Covered Code

1. **Find uncovered function**: `cargo llvm-cov report -p <crate> --show-missing`
2. **Check visibility**: `pub(crate)` functions in non-test modules need tests in sibling `#[cfg(test)] mod tests`
3. **Match the exact API**: Inspect function signatures before writing tests

```bash
# Find exact line ranges of uncovered code
cargo llvm-cov report -p <crate> --show-missing 2>/dev/null | grep "src/foo.rs" | head -20
```

---

## 4. Known Obstacles & Workarounds

### `pub(crate)` Visibility
**Problem**: `pub(crate)` functions in non-test modules can't be tested from `tests/` directory.
**Workaround**: Add `#[cfg(test)] mod tests { ... }` inside the source module itself.

### Pre-existing Compile Failures (Block `cargo test --workspace`)
**Problem**: 33 root `tests/` files have missing `compression: None` in `TableInfo` initializers.
**Status**: Partially fixed in `fix/v311-14-sem4-common-coverage` (PR #3555). Remaining have `ExecutionEngine` generic parameter errors.
**Workaround**: Measure per-crate with `cargo llvm-cov test -p <crate>`, not workspace-wide.

### `Value::Point` Exhaustive Match
**Problem**: `match v { Value::Point(..) => ... }` missing arm in several test files.
**Fix**: Add `Value::Point(x, y) => format!("({}, {})", x, y)` arm.

### Parser Pre-existing Failures
**Problem**: `test_parse_create_procedure_inout_params` fails to compile (procedure parameter handling).
**Impact**: Blocks `cargo test -p sqlrustgo-parser` from running all tests.
**Workaround**: Use `cargo llvm-cov test --ignore-run-fail -p sqlrustgo-parser` to get full coverage despite failures.

### MySQL Protocol Tests (wire_client, mysql-server)
**Problem**: `wire_client.rs` and `do_command_loop` in mysql-server require live MySQL connections.
**Workaround**: Either mock the TCP layer or accept these as integration-level gaps.
