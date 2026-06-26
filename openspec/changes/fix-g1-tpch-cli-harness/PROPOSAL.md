# SPEC: fix-g1-tpch-cli-harness — Fix `tpch_soak_test` compilation

## Summary

`tests/tpch_soak_test.rs` imports `common::tpch_cli_harness::{mysql_query, start_sf001_cli}`, but `tests/common/mod.rs` never declared `pub mod tpch_cli_harness`. The file `tests/common/tpch_cli_harness.rs` exists and exports both symbols correctly — only the module-tree wiring was missing. Adding one line to `mod.rs` resolves the unresolved-import error; the secondary type-inference error at line 130 is a cascading consequence that clears once the import succeeds.

## Root Cause

| File | Status |
|------|--------|
| `tests/common/tpch_cli_harness.rs` | EXISTS — exports `mysql_query`, `start_sf001_cli`, etc. |
| `tests/common/mod.rs` | MISSING `pub mod tpch_cli_harness;` declaration |
| `tests/tpch_soak_test.rs:21` | `use common::tpch_cli_harness::{mysql_query, start_sf001_cli};` |

The module was authored but never wired into the `common` module namespace.

## Changes

### `tests/common/mod.rs`

Add one public module declaration after the existing `tpch_wire_harness` line:

```rust
pub mod oracle_framework;
pub mod tpch_cli_harness;   // <— ADD THIS LINE
pub mod tpch_wire_harness;
pub mod wire_proto { … }
```

No other files need changes. The type annotation error on `rows.len()` (line 130) is a cascading compiler consequence of the unresolved import — fixing the import resolves it without any extra code.

## Secondary Finding

`tests/small_executor_modules_test.rs` has a `MockEngine` struct (line 48) that implements `ExecutionEngine` but is missing a `flush()` method. This is a **pre-existing separate bug** (not blocking `tpch_soak_test` compilation) and is tracked independently.

## Verification

```bash
cargo check --tests --test tpch_soak_test
```

Expected: clean compile (warnings acceptable, zero errors).
