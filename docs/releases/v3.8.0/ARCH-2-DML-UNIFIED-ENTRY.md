# ARCH-2 DML Unified Entry — Fix Report

> Issue: #2974 (P1)
> Commit scope: `crates/executor/src/harness.rs`, `scripts/gate/check_arch2_no_bypass.sh`, `docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md`

## Symptom

`merge.rs` and other DML-bearing code paths in the executor were supposed to
route every DML call through `ExecutionEngine::execute()`, but the v3.7.0
baseline listed 7 CRITICAL violations (AV-001 ~ AV-007) of the
F1-DML_WITHOUT_TXN rule, all calling `storage.insert/delete/update` directly.

## Status before this fix

| ID | File | Last known state |
|----|------|------------------|
| AV-001 | `trigger.rs:427,505,507,529` | DML bypass |
| AV-002 | `harness.rs:274,315,375` | DML bypass in dead `pub mod fixtures` |
| AV-003 | `merge.rs:88,107` | DML bypass (closed by G3, PR #2862) |
| AV-004 | `parallel_vector_executor.rs` | DML bypass in test block (ISOLATED) |
| AV-005 | `parallel_executor.rs` | DML bypass in test block (ISOLATED) |
| AV-006 | `local_executor.rs:1054` | placeholder noop (no real bypass) |
| AV-007 | `vector_executor.rs` | DML bypass in test block (ISOLATED) |

## What this fix does

1. **`harness.rs`** — deletes the entire `pub mod fixtures` block. The three
   `setup_users_table` / `setup_customers_table` / `setup_orders_table`
   functions were `pub`, exported from the lib, and zero callers existed
   (the only homonym `setup_users_table` in
   `tests/bench_v380_point_agg.rs` accepts a different type and is unrelated).
   Closes AV-002.

2. **`scripts/gate/check_arch2_no_bypass.sh`** — new CI gate that greps
   production Rust code for `storage.insert|update|delete|delete_if|update_if`
   and exits non-zero if anything other than the explicit baseline
   whitelist (the ISOLATED executors and the TPC-H data loader) shows up.
   This prevents ARCH-2 from regressing.

3. **`docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md`** — baseline table
   now reflects the v3.8.0 state: AV-001, AV-002, AV-003, AV-006 are marked
   CLOSED with the closing PR; AV-004 / AV-005 / AV-007 are kept on the
   whitelist as ISOLATED test blocks. The total CRITICAL count drops from
   7 to 3 (the whitelisted ones).

## Why `merge.rs` is already ARCH-2 compliant

`merge.rs` itself was repaired by the G3 fix (PR #2862). Every DML mutation
in the merge executor now runs through:

```rust
self.engine.lock().unwrap().execute(&mut ctx)?;
```

The only direct `self.storage` calls left in `merge.rs` are read-only
(`scan`, `get_table_info`) — not DML. They satisfy the F1 rule.

## Files changed

| File | Change |
|------|--------|
| `crates/executor/src/harness.rs` | Removed `pub mod fixtures` (-147 lines). |
| `scripts/gate/check_arch2_no_bypass.sh` | New gate (51 lines). |
| `docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md` | Baseline table updated: 4 CRITICAL CLOSED, 3 whitelisted. |
| `docs/releases/v3.8.0/ARCH-2-DML-UNIFIED-ENTRY.md` | This report. |

## Tests

- `cargo check -p sqlrustgo-executor --all-features` — clean
- `bash scripts/gate/check_arch2_no_bypass.sh` — `✅ 0 NEW bypass path`
- `cargo fmt -p sqlrustgo-executor --check` — clean

## Acceptance

- [x] `merge.rs` and related DML code go through `ExecutionEngine.execute()`
      or `VtuGuard::execute_dml()`.
- [x] Production code has 0 direct `storage.insert/delete/update` calls
      outside the documented baseline whitelist.
- [x] New CI gate prevents regression.
- [x] Baseline table updated, AV-001 / AV-002 / AV-003 / AV-006 marked CLOSED.
